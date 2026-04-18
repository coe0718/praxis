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

### ~~Discord / Slack — Inbound Routing~~ ✓ DONE
`DiscordClient::poll_once()` polls watched channels via `PRAXIS_DISCORD_CHANNEL_IDS` (REST `/messages?after=`), tracks offset in `discord_state.json`, and routes messages through `handle_discord_command()`. `SlackClient::poll_once()` uses `conversations.history` with `PRAXIS_SLACK_CHANNEL_IDS`, tracks timestamps in `slack_state.json`. Both feed into the same command vocabulary as Telegram (`/ask`, `/run`, `/approve`, `/goal`, etc.). `praxis discord poll-once` / `praxis discord run` and `praxis slack poll-once` / `praxis slack run` mirror the Telegram CLI. Webhook handlers in the dashboard already wired. Both gated by `PRAXIS_DISCORD_ALLOWED_USER_IDS` / `PRAXIS_SLACK_ALLOWED_USER_IDS`.

### ~~MCP Integration~~ ✓ DONE
`src/mcp/server.rs` is fully wired. `tools/list` returns all registered Praxis tools with per-kind input schemas (shell tools expose `args`, HTTP tools expose `params`). `tools/call` creates a real `NewApprovalRequest` in SQLite and returns the approval ID — operators approve via `praxis approve <id>` before the next agent cycle executes it. `resources/list` now uses `PraxisPaths` fields with correct filenames. MCP client (`src/mcp/client.rs`) can consume external MCP servers over HTTP.

### ~~Full Rollout Canaries~~ ✓ DONE
`RouteWeightStore` (`canary_weights.json`) tracks per-route weights (0.0–1.0). `apply_automation()` decrements weight by 0.25 per failing run, increments by 0.125 per passing run, auto-freezes at 0.0, promotes and restores to 1.0 after 3 consecutive passes. `RouterBackend` loads weights at construction, sorts routes within each class bucket by effective weight (static × dynamic) so degraded routes become fallbacks before full freeze. `praxis canary status` shows per-route weight. HTTP-endpoint delegation still pending.

### ~~Dashboard UI~~ ✓ DONE
Full SPA rewrite in `src/dashboard/server.rs`. 24 API routes covering sessions, approvals (inline approve/reject), hot/cold memories (reinforce/forget/consolidate), tools, goals (add), identity file editor (8-file picker), canary status, heartbeat, scores, evolution proposals (inline approve), delegation links, config viewer, wake trigger, and manual run. Dark-themed frontend with sidebar nav, approval badge, SSE live event feed, toast notifications, auto-refresh, and hash-based routing. SSE stream and Prometheus endpoint preserved.

### ~~Memory Consolidation~~ ✓ DONE
Implemented in `src/storage/sqlite/memory_consolidation.rs` and wired into `execute_reflect()`. Every session, hot memories older than 7 days are clustered by shared tag; clusters of 3+ are promoted to a single cold memory and the originals deleted. Cold memories at the weight floor (0.25) beyond 2× their type-specific decay window are pruned. Also available on demand via `praxis memory consolidate`.

### ~~Evolution — Agent-Driven Proposals~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `maybe_propose_evolution()` is now called after every non-trivial Reflect phase. Generates `Config` proposals for `review_failed`/`eval_failed` outcomes and `Identity` proposals when composite+follow-through scores are both below 0.5. Capped at 3 pending proposals, deduplicated by title prefix, and triggers `render_self_evolution_doc()` after each new proposal.

### ~~Watchdog / Auto-Update~~ ✓ DONE
`praxis watchdog update [--repo owner/repo] [--apply]` fetches the latest GitHub release, detects platform asset, downloads binary to a temp path, backs up current binary to `backups/praxis.prev`, and replaces in-place when `--apply` is given. Update record written to `watchdog_update.json`. `praxis watchdog rollback` restores the backup and removes the record. Without `--apply`, reports the latest version and download path for operator review.

### ~~OAuth — Integration into Agent Operations~~ ✓ DONE
`GmailClient` (`src/oauth/gmail.rs`) reads unread inbox via Google token and injects up to 5 email summaries into the morning brief when a valid Google OAuth token is present. `GitHubClient` (`src/oauth/github_client.rs`) lists open issues and PRs via GitHub token. `github-issues` and `github-prs` built-in tools wired into `execute_request()` — take a `repo` param (`owner/repo`) and return formatted lists. Both clients load from `OAuthTokenStore` and are silently skipped when no token is available.

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
