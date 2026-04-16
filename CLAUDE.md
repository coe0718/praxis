# Praxis — Claude Code Working Guide

## What this project is

Praxis is a self-hosted personal AI agent written in Rust. It runs a four-phase loop (Orient → Decide → Act → Reflect → Sleep), wakes on a cron schedule, pursues goals autonomously, stores memories, and sends Telegram briefs to its operator. It is not a library; it is an always-on daemon with a CLI surface.

The reference live instance is **Axonix** (`github.com/coe0718/axonix`). Praxis is the generalized framework extracted from it.

---

## Repository layout

```
src/
  loop/           — runtime orchestration
    runtime.rs    — PraxisRuntime, run_once(), execute_reflect()
    phases.rs     — orient(), decide(), act()
    reflect.rs    — reflect(), capture_session_memory(), maybe_propose_evolution()
    planner.rs    — choose_goal(), GoalDecision
  identity/       — SOUL.md / IDENTITY.md loading and validation
  memory/         — hot/cold/link memory store traits
  storage/        — SQLite trait impls (SqliteSessionStore is the concrete type)
  tools/          — manifest loading, approval queue, execute_request(), policy, cooldowns
  context/        — context assembly, compaction, handoff notes
  messaging/      — Telegram polling/sending, Discord stub, Slack stub, bus
  evolution.rs    — EvolutionStore, EvolutionProposal, append-only JSONL + approval lifecycle
  score.rs        — SessionScore, four-dimension composite (anticipation/follow-through/reliability/independence)
  examples.rs     — SyntheticExample training triple (context/action/outcome) → evals/examples.jsonl
  anomaly.rs      — SystemSnapshot capture → system_anomalies.jsonl
  anatomy.rs      — refresh_stale_anatomy() — keeps CAPABILITIES.md in sync with identity/tool mtimes
  brief/          — generate_brief() — aggregates goals/memories/approvals/events for Telegram delivery
  learning/       — run_once() — mines argus report for opportunities, daily throttled
  paths.rs        — PraxisPaths — single struct holds every file path used in the runtime
  config/         — praxis.toml model (AppConfig)
  cli/            — all `praxis <sub>` commands
  hooks.rs        — HookRunner: interceptor (can abort phase) + observer (fire-and-forget)
  sandbox.rs      — per-channel filesystem isolation policy
  delegation.rs   — agent-to-agent delegation links (store only; Act phase does not use them yet)
  speculative/    — trial execution records (store only; Act phase does not use them yet)
tests/            — integration tests; most spin up a tmp data dir and run CLI commands
NEEDS_FINISHED.md — authoritative list of STUB / PARTIAL / WIRED-but-disconnected features
PRAXIS_DESIGN.md  — canonical architecture and philosophy document (1400 lines)
```

---

## The agent loop

```
run_once()
  → orient()     load context, goals, tools, anatomy; compute context pressure; write handoff/compact if needed
  → decide()     pick task/approval-queue/goal; plan_action() → LLM; write decision receipt
  → act()        finalize_action() → LLM; or execute approved tool request
  → reflect()    record session, run reviewer + evals, compute score, record example, snapshot,
                 propose evolution, append postmortem, capture memory, synthesize skill
  (post-reflect) decay cold memories, run daily learning, send morning brief, fire session.end hooks
```

All phases are methods on `PraxisRuntime<B,C,E,G,I,S,T>`. The seven generics are:
- **B** — `AgentBackend` (LLM driver: plan_action / finalize_action)
- **C** — `Clock`
- **E** — `EventSink`
- **G** — `GoalParser`
- **I** — `IdentityPolicy`
- **S** — compound store trait (SessionStore + MemoryStore + MemoryLinkStore + ApprovalStore + QualityStore + ProviderUsageStore + OperationalMemoryStore + AnatomyStore + DecisionReceiptStore)
- **T** — `ToolRegistry`

The concrete `S` is always `SqliteSessionStore`. When reflect-phase code needs `SqliteSessionStore` directly (e.g. `learning::run_once`), construct a transient one: `SqliteSessionStore::new(self.paths.database_file.clone())`.

---

## Key data files (all under `PraxisPaths`)

| File | Purpose |
|---|---|
| `praxis.toml` | Main config (AppConfig) |
| `SOUL.md` | Immutable core identity — operator-only |
| `IDENTITY.md` | Evolving working identity — agent-writable |
| `GOALS.md` | Active goal list, parsed by GoalParser |
| `AGENTS.md` | Conventions and quality gate guidance |
| `CAPABILITIES.md` | Auto-generated tool/identity index (anatomy) |
| `score.jsonl` | Per-session irreplaceability scores |
| `evolution.jsonl` | Agent self-proposals, append-only |
| `SELF_EVOLUTION.md` | Human-readable render of evolution.jsonl |
| `system_anomalies.jsonl` | SystemSnapshot records |
| `evals/examples.jsonl` | SyntheticExample training triples |
| `brief_sent.txt` | Date of last Telegram morning brief |
| `hooks.toml` | HookRunner definitions |
| `tools/` | Tool manifest TOML files |
| `skills/` | Installed skill SKILL.md docs |
| `session_state.json` | Live session state across phase boundaries |

---

## Tool system

Tools are TOML manifests in `data_dir/tools/`. Each has a `kind` (Internal / Shell / Http), `required_level` (1–3), and approval flags. The approval queue lives in SQLite.

Four built-in tools added in the current codebase:
- `file-read` — Internal, level 1, no approval, sandboxed to `data_dir` + `allowed_read_paths`
- `git-query` — Shell, level 2, requires approval, `/usr/bin/git`
- `shell-exec` — Shell, level 3, requires approval + rehearsal, `/bin/bash -c`
- `web-fetch` — Http, level 2, requires approval, endpoint resolved from params

`execute_request()` in `src/tools/execute.rs` dispatches by tool name first, then by kind.

---

## Telegram

`TelegramBot::from_env()` reads `PRAXIS_TELEGRAM_BOT_TOKEN` + `PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS`. The module is `src/messaging/telegram.rs` but re-exported as `crate::messaging::TelegramBot` (the inner module is private — always use the re-export path).

Morning brief is sent once per calendar day via `try_send_morning_brief()` in `runtime.rs`. Gated by `data_dir/brief_sent.txt`. Silently skipped if env vars absent.

---

## Evolution proposals

`maybe_propose_evolution()` in `src/loop/reflect.rs` generates proposals after non-trivial sessions:
- `review_failed` → Config proposal
- `eval_failed` → Config proposal
- composite < 0.5 AND follow_through < 0.5 → Identity proposal

Capped at 3 pending proposals; deduplicated by title prefix. Calls `render_self_evolution_doc()` after each new proposal.

---

## What is NOT yet wired (see NEEDS_FINISHED.md for full list)

- **Delegation** — store exists, Act phase never sends work over a link
- **Hands** — manifests exist, Orient never loads an active hand
- **Speculative execution** — store exists, Act has no branching logic
- **Vault** — store exists, tools bypass it and use env vars directly
- **MCP** — dispatch stub exists, never touches tool registry
- **Discord/Slack inbound** — outbound only, no polling loop
- **Dashboard UI** — SSE and Prometheus work; HTML is a skeleton

---

## Coding conventions (project-specific)

- All errors use `anyhow::Result` with `.context()`; no `unwrap()` in production paths
- Warn-and-continue pattern for non-fatal reflect-phase side effects: `if let Err(e) = ... { log::warn!(...) }`
- `cargo fmt` + `cargo clippy` must be clean before committing
- Tests live in `tests/` as integration tests using tmp data dirs; unit tests use `#[cfg(test)]` in-file
- When adding a field to `ToolManifest`, update every struct literal in tests and source (grep for `ToolManifest {`)
- `PraxisRuntime` is generic; it cannot name `SqliteSessionStore` directly — construct a transient instance when needed
