//! Post-session memory extraction via LLM.
//!
//! At session end, calls a lightweight model to extract structured memories
//! from the full session context. One API call, full context, much better
//! signal than evaluating individual turns.
//!
//! Only memories scoring >= 0.5 importance are persisted.
//! If extraction fails or returns nothing, gracefully degrades.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    backend::AgentBackend,
    memory::{MemoryLinkStore, MemoryLinkType, MemoryStore, NewHotMemory},
    storage::StoredSession,
};

/// A single memory extracted from a session by the LLM.
#[derive(Debug, Deserialize)]
pub struct ExtractedMemory {
    /// The durable fact or insight (1-3 sentences).
    pub content: String,
    /// One-line summary for quick scanning.
    pub summary: Option<String>,
    /// Importance 0.0-1.0. Only >= 0.5 is persisted.
    pub importance: f32,
    /// Tags for categorization and clustering.
    pub tags: Vec<String>,
    /// Memory type: "episodic", "semantic", or "procedural".
    pub memory_type: String,
    /// Links to existing memories by ID.
    pub links: Vec<ExtractedLink>,
}

/// A relational link between this memory and an existing one.
#[derive(Debug, Deserialize)]
pub struct ExtractedLink {
    /// ID of the existing memory this connects to.
    pub target_memory_id: i64,
    /// Link type: "related_to", "contradicts", "caused_by", "user_preference", "follow_up"
    pub link_type: String,
}

/// Statistics from an extraction run.
#[derive(Debug, Default)]
pub struct ExtractionSummary {
    pub memories_extracted: usize,
    pub memories_persisted: usize,
    pub links_created: usize,
}

/// Run the extraction pipeline at session end.
///
/// Calls a lightweight model with the session context, parses the structured
/// output, and persists qualifying memories (importance >= 0.5) to the hot store.
///
/// Gracefully handles failures — returns `Ok(ExtractionSummary::default())`
/// on any error so the reflect phase can continue.
pub fn extract<S, B>(
    store: &S,
    backend: &B,
    stored: &StoredSession,
    outcome: &str,
) -> Result<ExtractionSummary>
where
    S: MemoryStore + MemoryLinkStore,
    B: AgentBackend,
{
    // Skip extraction for non-sessions (idle/skipped/idle outcomes)
    if matches!(outcome, "idle" | "skipped" | "deferred") {
        return Ok(ExtractionSummary::default());
    }

    let session_summary = &stored.action_summary;
    if session_summary.is_empty() {
        return Ok(ExtractionSummary::default());
    }

    let prompt =
        build_extraction_prompt(session_summary, outcome, stored.selected_goal_title.as_deref());

    let result = match backend.answer_prompt(&prompt) {
        Ok(output) => output.summary,
        Err(e) => {
            log::warn!("memory capture: LLM extraction failed: {e}");
            return Ok(ExtractionSummary::default());
        }
    };

    let extracted = match parse_extraction_result(&result) {
        Ok(memories) => memories,
        Err(e) => {
            log::warn!("memory capture: failed to parse extraction result: {e}");
            return Ok(ExtractionSummary::default());
        }
    };

    let mut summary = ExtractionSummary::default();
    summary.memories_extracted = extracted.len();

    for memory in &extracted {
        if memory.importance < 0.5 || memory.content.trim().is_empty() {
            continue;
        }

        let memory_type = parse_memory_type(&memory.memory_type);
        let tags = normalize_tags(&memory.tags, outcome);

        let stored_memory = match store.insert_hot_memory(NewHotMemory {
            content: memory.content.clone(),
            summary: memory.summary.clone(),
            importance: memory.importance,
            tags,
            expires_at: None,
            memory_type,
        }) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("memory capture: failed to insert hot memory: {e}");
                continue;
            }
        };

        summary.memories_persisted += 1;

        // Create memory links
        for link in &memory.links {
            let link_type = parse_link_type(&link.link_type);
            if let Err(e) =
                store.add_memory_link(stored_memory.id, link.target_memory_id, link_type)
            {
                log::warn!("memory capture: failed to create link: {e}");
                continue;
            }
            summary.links_created += 1;
        }
    }

    log::info!(
        "memory capture: extracted {}, persisted {} ({} links)",
        summary.memories_extracted,
        summary.memories_persisted,
        summary.links_created,
    );

    Ok(summary)
}

/// Build the structured extraction prompt for the LLM.
fn build_extraction_prompt(
    session_summary: &str,
    outcome: &str,
    goal_title: Option<&str>,
) -> String {
    let goal_context = match goal_title {
        Some(title) => format!("Goal: {title}"),
        None => "No specific goal was selected.".to_string(),
    };

    format!(
        r#"Extract the MOST important durable facts from this session as JSON.
Return ONLY a JSON array of objects. No markdown, no explanation, no code fences.

For each fact include:
- "content": the fact/insight (1-3 sentences, self-contained)
- "summary": one-line summary (optional)
- "importance": 0.0 to 1.0 (0.5+ = worth keeping, 0.7+ = important insight, 0.9+ = irreplaceable)
- "tags": ["category1", "category2"] (e.g. user-preference, bug, architecture, workflow, decision)
- "memory_type": "episodic" (event, happened once), "semantic" (general knowledge), or "procedural" (how-to)
- "links": [] (connections to other memories — can be empty)

Focus on: preferences, decisions, bugs found, patterns, architecture insights, gotchas, user corrections.
Exclude: transient status, trivial test output, session boilerplate, timestamps.

{goal_context}
Session outcome: {outcome}
Session summary: {session_summary}

JSON:>"#,
        goal_context = goal_context,
        outcome = outcome,
        session_summary = session_summary,
    )
}

/// Parse the LLM's response into extracted memories.
///
/// Attempts to extract JSON from the response, handling common LLM output
/// patterns like markdown code fences, trailing text, etc.
fn parse_extraction_result(raw: &str) -> Result<Vec<ExtractedMemory>> {
    let trimmed = raw.trim();

    // Strip markdown code fences if present
    let json_str = if trimmed.starts_with("```") {
        let without_fence = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_fence.strip_suffix("```").unwrap_or(without_fence).trim()
    } else {
        trimmed
    };

    // Find the JSON array — look for '[' or '{'
    let start = json_str.find(|c| c == '[' || c == '{');
    let end = json_str.rfind(|c| c == ']' || c == '}');

    let extracted = match (start, end) {
        (Some(s), Some(e)) if s < e => {
            let potential_json = &json_str[s..=e];
            serde_json::from_str::<Vec<ExtractedMemory>>(potential_json)
                .or_else(|_| {
                    // Maybe it's a single object wrapped in an array
                    serde_json::from_str::<ExtractedMemory>(potential_json).map(|m| vec![m])
                })
                .context("failed to parse extraction JSON")?
        }
        _ => {
            // Try parsing the entire response as JSON
            serde_json::from_str::<Vec<ExtractedMemory>>(json_str)
                .or_else(|_| serde_json::from_str::<ExtractedMemory>(json_str).map(|m| vec![m]))
                .context("no JSON found in extraction response")?
        }
    };

    Ok(extracted)
}

/// Parse a memory type string, defaulting to episodic.
fn parse_memory_type(s: &str) -> crate::memory::MemoryType {
    crate::memory::MemoryType::parse(s)
}

/// Parse a link type string, defaulting to RelatedTo.
fn parse_link_type(s: &str) -> MemoryLinkType {
    match s.to_lowercase().replace(' ', "_").as_str() {
        "caused_by" => MemoryLinkType::CausedBy,
        "contradicts" => MemoryLinkType::Contradicts,
        "user_preference" => MemoryLinkType::UserPreference,
        "follow_up" => MemoryLinkType::FollowUp,
        _ => MemoryLinkType::RelatedTo,
    }
}

/// Ensure consistent tags across extraction runs.
fn normalize_tags(tags: &[String], outcome: &str) -> Vec<String> {
    let mut normalized: Vec<String> =
        tags.iter().map(|t| t.to_lowercase().replace(' ', "-")).collect();

    // Always tag with the outcome
    normalized.push(format!("outcome:{outcome}"));

    // Deduplicate
    normalized.sort();
    normalized.dedup();
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extraction_json_array() {
        let input = r#"[
            {"content": "User prefers concise responses", "importance": 0.8, "tags": ["preference"], "memory_type": "semantic", "links": []}
        ]"#;
        let result = parse_extraction_result(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "User prefers concise responses");
        assert!((result[0].importance - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_parse_extraction_with_fences() {
        let input = "```json\n[{\"content\": \"test\", \"importance\": 0.6, \"tags\": [], \"memory_type\": \"episodic\", \"links\": []}]\n```";
        let result = parse_extraction_result(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "test");
    }

    #[test]
    fn test_parse_single_object() {
        let input = r#"{"content": "single fact", "importance": 0.9, "tags": ["critical"], "memory_type": "semantic", "links": []}"#;
        let result = parse_extraction_result(input).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_normalize_tags() {
        let tags = vec!["User Preference".to_string()];
        let result = normalize_tags(&tags, "success");
        assert!(result.contains(&"user-preference".to_string()));
        assert!(result.contains(&"outcome:success".to_string()));
    }

    #[test]
    fn test_parse_empty_tags_uses_default() {
        let input = r#"[]"#;
        let result = parse_extraction_result(input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_low_importance_excluded() {
        // Low importance memories should be filtered in `extract()`, not in parsing
        let input = r#"[{"content": "low value", "importance": 0.2, "tags": [], "memory_type": "episodic", "links": []}]"#;
        let result = parse_extraction_result(input).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].importance < 0.5);
    }

    #[test]
    fn test_parse_link_types() {
        assert!(matches!(parse_link_type("contradicts"), MemoryLinkType::Contradicts));
        assert!(matches!(parse_link_type("caused_by"), MemoryLinkType::CausedBy));
        assert!(matches!(parse_link_type("user_preference"), MemoryLinkType::UserPreference));
        assert!(matches!(parse_link_type("follow_up"), MemoryLinkType::FollowUp));
        assert!(matches!(parse_link_type("random"), MemoryLinkType::RelatedTo));
    }
}
