use std::{convert::Infallible, path::PathBuf, time::Duration};

use anyhow::Result;
use async_stream::stream;
use axum::{
    Json, Router,
    extract::State,
    response::{Html, IntoResponse, Sse, sse::Event},
    routing::get,
};

use crate::{
    cli::core,
    events::{Event as PraxisEvent, read_events_since},
    report::build_status_report,
};

#[derive(Clone)]
struct DashboardState {
    data_dir: PathBuf,
}

pub async fn serve_dashboard(data_dir: PathBuf, host: String, port: u16) -> Result<()> {
    let state = DashboardState { data_dir };
    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(status))
        .route("/summary", get(summary))
        .route("/health", get(health))
        .route("/events/recent", get(recent_events))
        .route("/events", get(events))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Praxis</title></head>
<body style="font-family: sans-serif; margin: 2rem;">
<h1>Praxis Dashboard</h1>
<pre id="status">loading...</pre>
<pre id="events" style="height: 20rem; overflow: auto; border: 1px solid #ccc; padding: 1rem;"></pre>
<script>
async function refreshStatus() {
  const summary = await fetch('/summary').then(r => r.json());
  document.getElementById('status').textContent = JSON.stringify(summary, null, 2);
}
refreshStatus();
setInterval(refreshStatus, 3000);
const events = document.getElementById('events');
fetch('/events/recent').then(r => r.json()).then(items => {
  items.forEach(item => {
    events.textContent += item.kind + ": " + item.detail + "\n";
  });
});
const source = new EventSource('/events');
source.onmessage = (event) => {
  events.textContent += event.data + "\n";
  events.scrollTop = events.scrollHeight;
};
</script></body></html>"#,
    )
}

async fn status(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_status(Some(state.data_dir))
        .unwrap_or_else(|error| format!("status error: {error}"))
}

async fn summary(State(state): State<DashboardState>) -> impl IntoResponse {
    match core::load_initialized_config(Some(state.data_dir.clone()))
        .and_then(|(config, paths)| build_status_report(&config, &paths))
    {
        Ok(report) => Json(report).into_response(),
        Err(error) => format!("summary error: {error}").into_response(),
    }
}

async fn health(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_doctor(Some(state.data_dir))
        .unwrap_or_else(|error| format!("health error: {error}"))
}

async fn recent_events(State(state): State<DashboardState>) -> impl IntoResponse {
    let path = state.data_dir.join("events.jsonl");
    let events = read_events_since(&path, 0)
        .map(|(items, _)| {
            items
                .into_iter()
                .rev()
                .take(20)
                .collect::<Vec<PraxisEvent>>()
        })
        .unwrap_or_default();
    Json(events.into_iter().rev().collect::<Vec<_>>())
}

async fn events(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let path = state.data_dir.join("events.jsonl");
    let stream = stream! {
        let mut offset = 0;
        loop {
            let (events, next_offset) = read_events_since(&path, offset)
                .unwrap_or_else(|_| (Vec::new(), offset));
            offset = next_offset;
            for item in events {
                yield Ok(Event::default().event(item.kind).data(item.detail));
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    };
    Sse::new(stream)
}
