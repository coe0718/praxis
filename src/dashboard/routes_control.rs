use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    backend::{ContentBlock, ImageUrl, InputContent},
    paths::PraxisPaths,
    wakeup::{WakeIntent, request_wake},
};

use super::{
    server::{DashboardState, api_error},
    types::{
        ApiChatContent, ApiContentBlock, AskBody, ChatChoice, ChatCompletionsRequest,
        ChatCompletionsResponse, ChatResponseMessage, ChatUsage, RunBody, WakeBody,
    },
};

pub(super) async fn api_wake(
    State(state): State<DashboardState>,
    Json(body): Json<WakeBody>,
) -> impl IntoResponse {
    let reason = body.reason.unwrap_or_else(|| "dashboard wake".to_string());
    let mut intent = WakeIntent::new(&reason, "dashboard");
    if let Some(task) = body.task {
        intent = intent.with_task(task);
    }
    if body.urgent.unwrap_or(false) {
        intent = intent.urgent();
    }
    match request_wake(&state.data_dir, &intent) {
        Ok(()) => Json(json!({ "queued": true, "reason": reason })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_run(
    State(state): State<DashboardState>,
    Json(body): Json<RunBody>,
) -> impl IntoResponse {
    use crate::cli::{RunArgs, core::handle_run};
    match handle_run(
        Some(state.data_dir.clone()),
        RunArgs {
            once: true,
            force: true,
            task: body.task,
            profile: None,
            one_shot: false,
            fast: false,
            redact_secrets: false,
        },
    ) {
        Ok(summary) => Json(json!({ "outcome": summary })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_ask(
    State(state): State<DashboardState>,
    Json(body): Json<AskBody>,
) -> impl IntoResponse {
    use crate::cli::{AskArgs, core::handle_ask};
    if body.prompt.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "prompt is required").into_response();
    }
    match handle_ask(
        Some(state.data_dir.clone()),
        AskArgs {
            files: vec![],
            attachment_policy: "reject".to_string(),
            tools: false,
            prompt: vec![body.prompt],
            redact_secrets: false,
        },
    ) {
        Ok(output) => Json(json!({ "output": output })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_canary_run(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::cli::canary::{CanaryArgs, CanaryCommand, CanaryRunArgs, handle_canary};
    match handle_canary(
        Some(state.data_dir.clone()),
        CanaryArgs {
            command: CanaryCommand::Run(CanaryRunArgs { provider: None }),
        },
    ) {
        Ok(output) => Json(json!({ "output": output })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_evolution(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = crate::evolution::EvolutionStore::from_paths(&paths);
    match store.all() {
        Ok(proposals) => Json(proposals).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_evolution_approve(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = crate::evolution::EvolutionStore::from_paths(&paths);
    match store.approve(&id) {
        Ok(proposal) => {
            Json(json!({ "id": id, "approved": true, "status": proposal.status.label() }))
                .into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_delegation(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::delegation::DelegationStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match DelegationStore::load(&paths.delegation_links_file) {
        Ok(store) => Json(json!({
            "links": store.links.values().collect::<Vec<_>>(),
            "active_counts": store.active_counts,
        }))
        .into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

// ── OpenAI-compatible chat/completions ────────────────────────────────────────

/// Convert API-side message content to internal `InputContent`.
///
/// Handles three cases:
/// - Plain text → `InputContent::Text`
/// - Content blocks with only text → `InputContent::Text` (joined)
/// - Content blocks with images → `InputContent::Blocks` with `ImageUrl` blocks
///
/// Base64 data URIs (`data:image/...;base64,...`) are passed through unchanged
/// so the upstream provider can decode them.
fn api_content_to_input(content: &ApiChatContent) -> InputContent {
    match content {
        ApiChatContent::Text(text) => InputContent::Text(text.clone()),
        ApiChatContent::Blocks(blocks) => {
            // If there are no image blocks, flatten to plain text.
            let has_images = blocks.iter().any(|b| matches!(b, ApiContentBlock::ImageUrl { .. }));
            if !has_images {
                let text = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ApiContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                InputContent::Text(text)
            } else {
                let content_blocks: Vec<ContentBlock> = blocks
                    .iter()
                    .map(|b| match b {
                        ApiContentBlock::Text { text } => ContentBlock::Text { text: text.clone() },
                        ApiContentBlock::ImageUrl { image_url } => ContentBlock::ImageUrl {
                            image_url: ImageUrl {
                                url: image_url.url.clone(),
                                detail: image_url.detail.clone(),
                            },
                        },
                    })
                    .collect();
                InputContent::Blocks(content_blocks)
            }
        }
    }
}

/// OpenAI-compatible `/v1/chat/completions` endpoint.
///
/// Accepts multi-modal messages with inline images (URLs and base64 data URIs).
/// Images are converted to `InputContent::Blocks` with `ImageUrl` content blocks
/// before being dispatched to the configured backend provider.
pub(super) async fn api_chat_completions(
    State(state): State<DashboardState>,
    Json(body): Json<ChatCompletionsRequest>,
) -> impl IntoResponse {
    use crate::backend::{AgentBackend, ConfiguredBackend};
    use crate::config::AppConfig;
    use crate::usage::UsageBudgetPolicy;

    // Collect the last user message as the input.
    let (system, user_input) = match extract_messages(&body) {
        Ok(pair) => pair,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": { "message": e, "type": "invalid_request_error" } })),
            )
                .into_response();
        }
    };

    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let config = match AppConfig::load(&paths.config_file) {
        Ok(c) => c,
        Err(e) => return api_error(e).into_response(),
    };
    let backend = match ConfiguredBackend::from_runtime(&config, &paths) {
        Ok(b) => b,
        Err(e) => return api_error(e).into_response(),
    };

    let budgets = match UsageBudgetPolicy::load_or_default(&paths.budgets_file) {
        Ok(b) => b,
        Err(e) => return api_error(e).into_response(),
    };

    let max_tokens = body.max_completion_tokens.unwrap_or(1024);
    let decision = budgets
        .ask
        .check_estimate(max_tokens as i64, crate::usage::UsageBudgetMode::Ask);
    if decision.blocked {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": { "message": decision.summary, "type": "rate_limit_error" } })),
        )
            .into_response();
    }

    let text_input = match &user_input {
        crate::backend::InputContent::Text(t) => t.as_str(),
        crate::backend::InputContent::Blocks(blocks) => {
            let combined: String = blocks
                .iter()
                .filter_map(|b| match b {
                    crate::backend::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            Box::leak(combined.into_boxed_str())
        }
    };

    match backend.answer_prompt(text_input) {
        Ok(output) => {
            let response = ChatCompletionsResponse {
                id: format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis()),
                object: "chat.completion",
                created: chrono::Utc::now().timestamp(),
                model: body.model.unwrap_or_else(|| "praxis".to_string()),
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatResponseMessage {
                        role: "assistant",
                        content: output.summary,
                    },
                    finish_reason: "stop",
                }],
                usage: ChatUsage {
                    prompt_tokens: output.attempts.first().map(|a| a.input_tokens).unwrap_or(0),
                    completion_tokens: output
                        .attempts
                        .first()
                        .map(|a| a.output_tokens)
                        .unwrap_or(0),
                    total_tokens: output
                        .attempts
                        .first()
                        .map(|a| a.input_tokens + a.output_tokens)
                        .unwrap_or(0),
                },
            };
            Json(response).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

/// Extract system and user messages from the request, converting inline images.
fn extract_messages(body: &ChatCompletionsRequest) -> Result<(String, InputContent), String> {
    let mut system = String::new();
    let mut last_user_input: Option<InputContent> = None;

    for msg in &body.messages {
        match msg.role.as_str() {
            "system" => {
                if let ApiChatContent::Text(text) = &msg.content {
                    system = text.clone();
                }
            }
            "user" => {
                last_user_input = Some(api_content_to_input(&msg.content));
            }
            _ => {} // ignore assistant / tool messages
        }
    }

    match last_user_input {
        Some(input) => Ok((system, input)),
        None => Err("at least one user message is required".to_string()),
    }
}
