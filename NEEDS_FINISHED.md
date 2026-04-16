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

### Delegation (agent-to-agent work distribution)
`src/delegation.rs` and `src/cli/delegation.rs` store and manage delegation links. The runtime never uses them. The Act phase needs code to send work over an active link and the messaging layer needs an inbound handler for delegated tasks.

### Discord / Slack — Inbound Routing
`src/messaging/discord.rs` and `src/messaging/slack.rs` handle outbound send/webhook. Neither has an inbound polling loop or command router equivalent to Telegram's. The dashboard webhook stubs (`/src/dashboard/server.rs` `:139`, `:185`) parse nothing.

### MCP Integration
`src/mcp/server.rs` has a `dispatch()` function that accepts JSON-RPC but never touches the tool registry. No tool listing, no resource definitions, no MCP client for consuming external servers. The `/mcp` route is registered in the dashboard but effectively does nothing useful.

### Full Rollout Canaries
`src/canary.rs` records pass/fail per model and can freeze remote routes. Missing: automated promotion on sustained success, automated rollback trigger from heartbeat or watchdog, and gradual traffic shifting. All canary actions are currently manual.

### Dashboard UI
SSE stream and Prometheus endpoint work. The HTML served (`src/dashboard/server.rs` ~line 57) is a bare skeleton with no widgets, no controls, and no reactive state. Needs real frontend work.

### Memory Consolidation
Decay (`src/storage/sqlite/memory_decay.rs`) multiplies weight by 0.97 every N days. There is no synthesis pass that clusters related hot memories into a single cold memory or prunes redundant entries. Memory grows without bound beyond decay weight reduction.

### Evolution — Agent-Driven Proposals
`src/evolution.rs` is complete for the store and approval lifecycle. `src/loop/reflect.rs` never calls any proposal generation. The agent cannot propose changes to its own config, identity, or roadmap — only the operator can via CLI. The Reflect phase needs logic that inspects session outcomes and writes evolution proposals.

### Watchdog / Auto-Update
`src/cli/watchdog.rs` installs a systemd/launchd timer that calls `praxis run --once`. Missing: binary self-update, canary-driven binary promotion/rollback, and automatic heartbeat verification with restart on stall.

### OAuth — Integration into Agent Operations
Device-flow auth for GitHub and Google works. Tokens are injected into shell and HTTP tool env vars. But no high-level agent-facing integrations exist (no Gmail reader, no GitHub API client in the loop). OAuth is infrastructure with no consumers beyond raw env injection.

### ~~Scoring / Irreplaceability~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `SessionScore::compute()` and `record_score()` are now called at the end of every Reflect phase. Score inputs are derived from session state (proactive wake, goal completion, tool invocations, resume count). Composite score is also attached to synthetic examples as `quality_score`.

---

## WIRED — Implemented but disconnected from the runtime

These have real implementations but no call site in the main loop. They are effectively dead code until wired in.

### Hand Manifests
`src/hands.rs` and CLI (`src/cli/hands.rs`) are complete. Orient phase never loads an active hand. No tool activation, no role enforcement. The feature is invisible at runtime.

### ~~Synthetic Example Generation~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `record_example()` is now called at the end of every Reflect phase for non-idle outcomes. Quality score is attached from the composite irreplaceability score computed in the same pass.

### ~~System Anomaly Correlation~~ ✓ DONE
Wired into `src/loop/reflect.rs`. `SystemSnapshot::capture()` and `record_snapshot()` are now called at the end of every Reflect phase, tagged with the final session outcome.

### Vault (Credential Store)
`src/vault.rs` and `src/cli/vault.rs` are complete. Provider auth and tool injection both bypass the vault and use direct env vars. The vault is never queried at runtime.

### Speculative Execution
`src/speculative/mod.rs` stores trial execution records. The Act phase has no branching logic. No rehearsal automation exists. Nothing calls into speculative at runtime.

### ~~Learning / Opportunity Mining~~ ✓ DONE
Wired into `src/loop/runtime.rs::execute_reflect()`. `learning::run_once()` is now called automatically after every session, with daily/weekly throttles enforced internally. New opportunities emit `agent:learning_opportunities_found` events.

### Anatomy Refresh
`src/anatomy.rs` has `refresh_stale_anatomy()`. The main loop never calls it automatically. It runs on `praxis doctor` only. CAPABILITIES.md drifts unless the operator manually refreshes.

### Brief Generation
`src/brief/mod.rs` and `src/cli/brief.rs` exist. No scheduler triggers it. No Telegram delivery is wired. It runs on `praxis brief` only.

### Sub-Agent Steering
`src/wakeup/mod.rs` parses a steering flag on wakeup intent. The Act phase has no mid-session message injection or branch redirection handler. The flag is parsed and ignored.

---

## NOT BROKEN, BUT NOTED

- **Vault** credentials are stored at `vault.toml` with `600` permissions but are not encrypted at rest.
- **OAuth tokens** in `oauth_tokens.json` are also plaintext on disk.
- **Git-backed state sync** (`src/cli/git.rs`) is a generic git CLI wrapper, not automatic commit-on-session-end. Labelled as a future feature in design docs; no confusion, just documenting.
