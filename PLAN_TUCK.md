# Praxis — Implementation Roadmap
**Author:** Tuck | **Date:** 2026-04-22
**Inputs:** PRAXIS_NEW_FEATURES.md, ECOSYSTEM_REVIEW.md, CODE_REVIEW_VEX.md, CLAUDE.md

---

## Guiding Principles

1. **Security first.** Vex found 12 criticals. Fix those before adding anything new.
2. **Drey writes, Vex reviews.** That's the pipeline. Drey on GLM 5.1 does the coding, Vex on MiniMax M2.7 reviews.
3. **No new infrastructure.** Everything runs on SQLite + Rust. No new databases, no new runtimes.
4. **Backward compatible.** Additive columns, feature flags, Cargo feature gates. No breaking schema changes.

---

## Phase 1 — Security Fixes (Drey, now → this week)

Drey is already working on these. Priority order:

### P0 — Immediate (Drey, in progress)

| # | Issue | File | Fix | Agent |
|---|-------|------|-----|-------|
| C1 | Shell injection via newline/CR/backslash | `execute.rs:25` | Add `\n`, `\r`, `\\` to `DANGEROUS_SHELL_CHARS` | Drey |
| C2 | OAuth token exfiltration via HTTP tools | `execute.rs:276-284` | Filter tokens by `allowed_oauth_providers` before injecting | Drey |
| C3 | Daemon urgency loss (double consume) | `daemon.rs:297` + `runtime.rs:61` | Set `force: true` in RunOptions when daemon detects wake intent | Drey |

### P1 — This Week (Drey)

| # | Issue | Fix | Agent |
|---|-------|-----|-------|
| C5 | Non-atomic hot/cold memory insert | Wrap INSERT + FTS INSERT in `connection.transaction()` | Drey |
| C6 | Webhook auth deadlock | Move webhook routes to public router with platform signature verification | Drey |
| W1 | No SSRF protection | Block localhost, 127.0.0.1, 169.254.169.254, private ranges in `run_http` | Drey |
| W2 | Unbounded command output | Add `take()` limit on stdout/stderr (e.g., 1MB cap) | Drey |
| W11 | Per-operation DB connections (no pooling) | Add connection pooling or at minimum transaction wrapping for multi-step ops | Drey |

### P1.5 — After P1 clears (Drey)

| # | Issue | Fix | Agent |
|---|-------|-----|-------|
| C4 | SQL injection surface in schema migration | Validate identifiers against `^[a-z_][a-z0-9_]*$` regex | Drey |
| C7 | MCP resources/read arbitrary file read | Restrict to files enumerated in `collect_resources()` | Drey |
| C8 | MCP tools/call no tool validation | Validate `tool_name` against registry before queueing | Drey |
| C10 | Predictable pairing code | Use `rand::random::<u32>() % 1_000_000` + rate limit + expiry | Drey |
| W6-W10 | Non-atomic multi-table operations | Wrap in transactions across `providers.rs`, `sessions.rs`, `approvals.rs`, `memory_consolidation.rs`, `memory_decay.rs` | Drey |
| W12 | LIKE pattern injection | Escape `%` and `_` wildcards in user-supplied queries | Drey |
| W13 | Error bodies leaking API keys | Truncate/sanitize provider error responses before logging | Drey |

**Gate:** Vex reviews all Phase 1 fixes. CI must pass. No Phase 2 work until Phase 1 is green.

---

## Phase 2 — Core Feature Upgrades (Drey, weeks 2-3)

### 2A — Memory Architecture Upgrade (HIGH IMPACT)

**Why:** Current FTS5 keyword-only search is the biggest daily-use limitation. Adding vector search is the single most impactful change.

**Approach:** Hybrid retrieval (0.6 vector + 0.4 keyword), same SQLite database.

| Task | Details | Agent |
|------|---------|-------|
| Add embedding column to hot/cold tables | `ALTER TABLE` with backward-compatible migration, `Vec<f32>` stored as BLOB | Drey |
| Embedding generation | Call OpenAI embeddings API or local model. Batch-generate for existing memories. | Drey |
| Hybrid retrieval function | Cosine similarity on vector column + FTS5 rank, weighted merge | Drey |
| Memory consolidation upgrade | Use embeddings to detect near-duplicates before consolidating | Drey |
| Frontend: memory search | Show relevance scores, toggle between keyword/semantic/hybrid | Drey |

**Dependencies:** Phase 1 complete (especially C5, W11 — we need transactions before adding embedding writes)

### 2B — WASM Sandbox for shell-exec (HIGH SECURITY)

**Why:** OpenFang's approach is the gold standard. Raw bash execution with approval queue is not enough.

| Task | Details | Agent |
|------|---------|-------|
| Add `wasmtime` dependency | Cargo feature gate behind `wasm-sandbox` | Drey |
| WASM tool runtime | Tools run with fuel metering + epoch interruption | Drey |
| Migrate shell-exec to WASM | Keep bash as fallback, WASM as primary | Drey |
| Secret zeroization | Zero key bytes from memory after use in `execute.rs` | Drey |

**Dependencies:** Phase 1 C1 fix (newline injection) — fix the immediate hole first, then add the sandbox

### 2C — Frontend Improvements

| Task | Details | Agent |
|------|---------|-------|
| Session Timeline View | Visualize Orient→Decide→Act→Reflect with timestamps and tool calls (Feature #1) | Drey |
| Approval Queue Search | Filter by tool name, path, date range, requested_by (Feature #2) | Drey |
| Token Spend Tracking | Frontend chart from `provider_usage` table data (Feature #10) | Drey |
| Mobile-Responsive Layout | Fix sidebar collapse, table overflow, chart sizing (Feature #9) | Drey |
| Keyboard Shortcuts | Hotkeys: `g` goals, `a` approvals, `r` run, `/` search (Feature #6) | Drey |

---

## Phase 3 — Polish & Advanced Features (Drey, weeks 4+)

### 3A — Goal & Agent Intelligence

| Task | Details | Agent |
|------|---------|-------|
| Goal Dependency Graph | Visualize goal dependencies and blockers (Feature #3) | Drey |
| Agent Health Dashboard | Memory usage, DB size, token spend rate, error rate, uptime trends (Feature #4) | Drey |
| Config Diff on Evolution | Side-by-side diff when agent proposes evolution (Feature #5) | Drey |
| Session Replay | Step-by-step replay of decisions and tool calls (Feature #8) | Drey |
| Browser Notifications | Push notification for new approvals via SSE (Feature #7) | Drey |

### 3B — Advanced Features from Ecosystem Review

| Task | Details | Agent |
|------|---------|-------|
| Autonomy levels (global toggle) | ReadOnly / Supervised / Full — replace per-tool approval | Drey |
| Rule-based model routing | Classify task complexity → route to cheap or expensive model | Drey |
| CodeAct mode | Agent writes code blocks, Praxis executes with state persistence | Drey |
| Merkle audit trail | Cryptographically link all actions for tamper evidence | Drey |
| Prompt injection scanner | Detect override attempts in tool outputs and user messages | Drey |
| Deterministic mode | Zero-LLM rule-based fallback path for offline/cost-free passes | Drey |

---

## Agent Assignments

| Agent | Role | Model | Tasks |
|-------|------|-------|-------|
| **Drey** | Primary coder | GLM 5.1 (Z.AI) | All implementation work across all phases |
| **Vex** | Code reviewer | MiniMax M2.7 (NVIDIA NIM) | Review every PR, verify fixes, catch regressions |
| **Tuck** | Planner / orchestrator | GLM 5.1 (Z.AI) | Planning, research, coordination, Jeremy's interface |
| **Scout** | Research / alt plans | GLM 5.1 (Z.AI) | Second opinion on architecture, ecosystem research |

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| GLM 5.1 language bleeding | Medium | Vex catches it in review; Drey re-edits |
| Drey introduces new bugs while fixing | Medium | Vex reviews every change; CI gates |
| WASM sandbox too complex for GLM | High | Start with simpler fixes, build up to WASM |
| Memory upgrade breaks existing data | Low | Backward-compatible migration, feature-flagged |
| Kimi's broken CI fixes linger | Medium | Drey reverts Kimi's changes, starts from Vex-verified baseline |

---

## Milestones

| Date | Milestone |
|------|-----------|
| End of week 1 | Phase 1 P0 + P1 complete, CI green, Vex sign-off |
| End of week 2 | Phase 1 complete, memory architecture upgrade started |
| End of week 3 | Memory upgrade + WASM sandbox, frontend improvements started |
| End of week 4+ | Phase 3 features rolling in, polish pass |

---

*Plan by Tuck. Compare against Scout's PLAN_SCOUT.md for Jeremy's review.*
