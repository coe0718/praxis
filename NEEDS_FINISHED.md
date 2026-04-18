# NEEDS FINISHED

Features that are either fully stubbed, partially wired, or implemented but disconnected from the runtime. Ordered by priority.

---

## STUB — Zero implementation behind the claim

### Lite Mode
Nothing actually changes when lite mode is referenced. No reduced token budgets, no simplified loop, no gating on Pi/low-power hardware. The README lists it as unfinished; confirm before deleting from roadmap.

### Zero-LLM Deterministic Mode
`StubBackend` returns hardcoded strings but there is no actual rule-based path that avoids LLM calls while producing real decisions. If this is a real goal, the orient/decide phases need a pure-logic branch.

### Voice Transcript Streaming
No Whisper integration, no audio input handler, no pipe into `ask`/`run`. Nothing exists.

### Serverless / Edge Entry Point
No Cloudflare Workers or AWS Lambda entry point. Nothing exists.

---

## PARTIAL — Some logic exists but the feature is incomplete

### ~~Delegation (agent-to-agent work distribution)~~ ✓ DONE
`src/delegation.rs` now includes `send_over_link()` (file-based: writes a `WakeIntent` to the remote agent's data_dir), `DelegatedTask`, and `drain_inbound_delegation()` (reads `delegation_queue.jsonl`). Act phase calls `try_delegate()` before the LLM finalize call — if an enabled outbound link permits the current task, it's sent and the session records outcome `delegated`. Orient drains `delegation_queue.jsonl` and converts each inbound task into a pending approval request. HTTP endpoints are not yet supported.

### Discord / Slack — Inbound Routing
`src/messaging/discord.rs` and `src/messaging/slack.rs` handle outbound send/webhook. Neither has an inbound polling loop or command router equivalent to Telegram's. The dashboard webhook stubs (`/src/dashboard/server.rs` `:139`, `:185`) parse nothing.

### ~~MCP Integration~~ ✓ DONE
`src/mcp/server.rs` is fully wired. `tools/list` returns all registered Praxis tools with per-kind input schemas (shell tools expose `args`, HTTP tools expose `params`). `tools/call` creates a real `NewApprovalRequest` in SQLite and returns the approval ID — operators approve via `praxis approve <id>` before the next agent cycle executes it. `resources/list` now uses `PraxisPaths` fields with correct filenames. MCP client (`src/mcp/client.rs`) can consume external MCP servers over HTTP.

### Full Rollout Canaries
`src/canary.rs` now tracks `consecutive_passes` per record and runs `apply_automation()` after every `run_canaries()` call. Routes frozen after a fresh failure (rollback) and unfrozen after `PROMOTION_THRESHOLD=3` consecutive passes (promotion). Freeze state persisted in `canary_frozen.json`. `praxis watchdog check` also freezes failing routes when a heartbeat stall is detected. Missing: gradual traffic shifting (router weight adjustment) and HTTP-endpoint delegation.

### ~~Dashboard UI~~ ✓ DONE
Full SPA rewrite in `src/dashboard/server.rs`. 24 API routes covering sessions, approvals (inline approve/reject), hot/cold memories (reinforce/forget/consolidate), tools, goals (add), identity file editor (8-file picker), canary status, heartbeat, scores, evolution proposals (inline approve), delegation links, config viewer, wake trigger, and manual run. Dark-themed frontend with sidebar nav, approval badge, SSE live event feed, toast notifications, auto-refresh, and hash-based routing. SSE stream and Prometheus endpoint preserved.

### ~~Memory Consolidation~~ ✓ DONE
Implemented in `src/storage/sqlite/memory_consolidation.rs` and wired into `execute_reflect()`. Every session, hot memories older than 7 days are clustered by shared tag; clusters of 3+ are promoted to a single cold memory and the originals deleted. Cold memories at the weight floor (0.25) beyond 2× their type-specific decay window are pruned. Also available on demand via `praxis memory consolidate`.

### ~~Evolution — Agent-Driven Proposals~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `maybe_propose_evolution()` is now called after every non-trivial Reflect phase. Generates `Config` proposals for `review_failed`/`eval_failed` outcomes and `Identity` proposals when composite+follow-through scores are both below 0.5. Capped at 3 pending proposals, deduplicated by title prefix, and triggers `render_self_evolution_doc()` after each new proposal.

### Watchdog / Auto-Update
`src/cli/watchdog.rs` installs a systemd/launchd timer that calls `praxis run --once`. `praxis watchdog check` now reads `heartbeat.json`, reports stale/ok status, and spawns `praxis run --once` when `--restart` is passed. Missing: binary self-update and canary-driven binary promotion/rollback.

### OAuth — Integration into Agent Operations
Device-flow auth for GitHub and Google works. Tokens are injected into shell and HTTP tool env vars. But no high-level agent-facing integrations exist (no Gmail reader, no GitHub API client in the loop). OAuth is infrastructure with no consumers beyond raw env injection.

### ~~Scoring / Irreplaceability~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `SessionScore::compute()` and `record_score()` are now called at the end of every Reflect phase. Score inputs are derived from session state (proactive wake, goal completion, tool invocations, resume count). Composite score is also attached to synthetic examples as `quality_score`.

---

## WIRED — Implemented but disconnected from the runtime

These have real implementations but no call site in the main loop. They are effectively dead code until wired in.

### ~~Hand Manifests~~ ✓ DONE
Wired into `src/loop/phases.rs::orient()`. `enforce_active_hand()` reads the active hand name from `hands/active.txt`, loads the manifest from `HandStore`, and validates that all required tools are registered. Missing required tools abort the session with a clear error. Missing optional tools log a warning. Write the hand name to `hands/active.txt` (or leave it empty/absent for no active hand).

### ~~Synthetic Example Generation~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `record_example()` is now called at the end of every Reflect phase for non-idle outcomes. Quality score is attached from the composite irreplaceability score computed in the same pass.

### ~~System Anomaly Correlation~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `SystemSnapshot::capture()` and `record_snapshot()` are now called at the end of every Reflect phase, tagged with the final session outcome.

### ~~Vault (Credential Store)~~ ✓ DONE
Wired into `src/tools/execute.rs`. The vault is loaded at the start of every `execute_request()`. Shell tools (including `shell-exec`) receive all vault secrets as `VAULT_<NAME>` env vars. HTTP tool headers and endpoint URLs have `$VAULT{name}` placeholders substituted with resolved values. Vault load failure is non-fatal (falls back to empty vault so env-var-only setups are unaffected).

### Speculative Execution
`src/speculative/mod.rs` stores trial execution records. The Act phase has no branching logic. No rehearsal automation exists. Nothing calls into speculative at runtime.

### ~~Learning / Opportunity Mining~~ ✓ DONE
Wired into `src/loop/runtime.rs::execute_reflect()`. `learning::run_once()` is now called automatically after every session, with daily/weekly throttles enforced internally. New opportunities emit `agent:learning_opportunities_found` events.

### ~~Anatomy Refresh~~ ✓ DONE
Wired into `src/loop/phases.rs::orient()`. `refresh_stale_anatomy()` is now called automatically on every Orient phase, keeping CAPABILITIES.md in sync without operator intervention.

### ~~Brief Generation~~ ✓ DONE
Wired into `src/loop/runtime.rs::execute_reflect()`. `try_send_morning_brief()` runs after every session, delivering a Telegram brief to the primary configured chat if the brief hasn't been sent today. Gated by `PRAXIS_TELEGRAM_BOT_TOKEN` — silently skipped when Telegram is not configured.

### ~~Sub-Agent Steering~~ ✓ DONE
Act phase now checks for a newly-arrived `wake_intent.json` before calling the LLM. If a wake intent with a `task` arrives mid-session, the action is redirected without an LLM call and the session records outcome `steered` with an `agent:steered` event.

---

## NOT BROKEN, BUT NOTED

- **Vault** credentials are stored at `vault.toml` with `600` permissions but are not encrypted at rest.
- **OAuth tokens** in `oauth_tokens.json` are also plaintext on disk.
- **Git-backed state sync** (`src/cli/git.rs`) is a generic git CLI wrapper, not automatic commit-on-session-end. Labelled as a future feature in design docs; no confusion, just documenting.
