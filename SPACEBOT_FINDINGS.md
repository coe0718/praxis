# Spacebot Assessment

**Source:** https://github.com/spacedriveapp/spacebot (237 Rust files, 140K lines)
**License:** FSL-1.1-ALv2 (converts to Apache 2.0 after 2 years)

## Architecture — Genuinely Different from Praxis

Spacebot replaces the monolithic session model with **five specialized process types** running concurrently on tokio:

| Process | Role | LLM? | Tools |
|---------|------|------|-------|
| **Channel** | User-facing conversation. Always responsive, never blocked. Delegates everything else. | Yes | reply, branch, spawn_worker, route, cancel, skip, react |
| **Branch** | Fork of channel context for independent thinking. Gets channel's full history, operates independently, injects conclusion back. | Yes | memory_recall, memory_save, task tools, spawn_worker |
| **Worker** | Focused task execution. Fresh context + task prompt. Two kinds: fire-and-forget and interactive (long-running). | Yes | shell, file, set_status |
| **Compactor** | Programmatic context monitor per channel. Tiered thresholds (80%/85%/95%). Runs async alongside channel, never blocks it. | No | — |
| **Cortex** | System-level observer. Generates **memory bulletin** every 60 min — LLM-curated summary of current knowledge across identity/events/decisions/preferences. Every channel reads it on every turn via `ArcSwap`. | Yes | memory_recall, memory_save |

## Key Concepts Not in Praxis

1. **Branch pattern:** Fork channel context → think independently → inject conclusion. Multiple branches run concurrently per channel. First done, first incorporated. Solves "agent stops responding while thinking" problem.

2. **Programmatic compactor:** Tiered compaction thresholds with escalating urgency. At 80% → background LLM summary. At 95% → emergency drop-oldest-turns (no LLM). All runs async without blocking the channel.

3. **Cortex/memory bulletin:** A periodically refreshed, LLM-curated summary of ALL current knowledge (not per-session). Every channel reads it on every turn. Cached via `ArcSwap`. Different from per-session context.

4. **Presets directory:** `presets/` with pre-built agent configurations (community-manager, content-writer, customer-support, executive-assistant, research-analyst, sales-bdr, etc.) as TOML files.

5. **No monolithic session loop.** Contrasts with Praxis's `orient → decide → act → reflect` single-threaded loop. Spacebot's processes are fully concurrent and communicate via delegation.

## Stack
- Rust (edition 2024), tokio, Rig (v0.30.0 agentic framework)
- SQLite (sqlx), LanceDB (embedded vector + FTS), redb (embedded key-value)
- Tauri desktop app

## Verdict
Genuinely novel architecture. The **branch pattern** and **cortex/memory bulletin** are interaction models Praxis doesn't have. The **compactor** is more sophisticated than Praxis's context management. Worth studying the multi-process delegation architecture if Praxis moves beyond its single-loop model.

*Clone deleted.*
