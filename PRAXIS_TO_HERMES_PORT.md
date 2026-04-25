# Praxis → Hermes: Full Port Candidates

*Compiled by Tuck + Drey, April 25, 2026*
*Combined analysis — features Praxis has that Hermes lacks*

---

## Caveats & Corrections

*After a second pass on the Hermes codebase, some of my initial assertions were wrong. Here's what Hermes already has:*

- **Credential redaction** — Hermes has `agent/redact.py` (340 lines) that redacts API keys, tokens, and secrets from logs, terminal output, tool output, and compressed context. This covers what Praxis calls "CredentialScrubGate" in its quality gates. ✅ Already handled.
- **Session-scoped approvals** — Hermes has `tools/approval.py` with session-scoped (`/approve`) and permanent (`/approve --permanent`) approval caching. This covers the same use case as Praxis's tool cooldowns (albeit based on approval caching rather than time windows). ✅ Already handled.
- **Provider fallback/circuit breakers** — Hermes has rate-limit-based provider fallback and cooldown via `agent/nous_rate_guard.py`. It lacks Praxis's health-probe-based canary routing with freeze/promotion, but the fallback foundation exists. ⚠️ Partial overlap.

The items below were verified as genuinely absent from Hermes.

---

## Tier 1 — High Impact, Worth Porting

| # | Feature | Source | Effort | Dependencies |
|---|---------|--------|--------|--------------|
| 1 | **Output Quality Gates** | `src/quality/gates.rs` | Trivial (< 1hr) | None |
| 2 | **Boundary Review System** | `src/boundaries.rs` | Trivial (~170 lines) | None |
| 3 | **Session Scoring (Irreplaceability Score)** | `src/score.rs` | Low | None (feeds #4) |
| 4 | **Self-Evolution Proposals** | `src/evolution.rs` | Medium-High | Requires #3 (Scoring) |
| 5 | **Argus Observability (Drift + Patterns)** | `src/argus/` | Medium | Requires #3 (Scoring) |
| 6 | **Model Canary (Health Probe Routing)** | `src/canary.rs` | Medium | Provider routing infra |
| 7 | **Progressive Context Loading** | `src/context/progressive.rs` | Medium | None (just landed in Praxis) |

**Killer combo:** #3 → #4 → #5 forms a closed loop — measure quality → detect drift → propose evolution.

### Detail: Output Quality Gates
Three deterministic checks on every output, no LLM call needed:
- `NonEmptyGate` — blocks whitespace-only responses
- `ForbiddenPhraseGate` — blocks banned substrings
- `MaxLengthGate` — retry-with-feedback if output exceeds limit

Trait-based: `fn check(content) → Pass | Block | RetryWithFeedback`. First non-Pass wins.

**Note:** Hermes already has credential scrubbing via `agent/redact.py` — that gate isn't needed. The remaining three gates (empty output, forbidden phrases, max length) are easy wins with no overlap.

### Detail: Boundary Review System
Parses SOUL.md for a `## Boundaries` section, tracks `last_confirmed_at`, prompts weekly: *"Review: have any hard limits changed?"* Boundaries can be programmatically added via CLI. Prevents silent constraint drift over months of operation.

### Detail: Session Scoring
4-dimension composite:
- **Anticipation** — proactive wake accuracy (0.20 weight)
- **Follow-through** — goals completed / selected (0.40)
- **Reliability** — approvals passed / total (0.25)
- **Operator Independence** — inverse of intervention count (0.15)

All normalized to [0,1]. Rolling averages enable trend detection. Neutral-on-absence defaults — idle sessions don't drag the score down.

**Framing:** The score measures "how much the operator would lose if the agent were replaced with a naive assistant" — not generic task-completion. The four dimensions capture agent-specific accumulated value.

### Detail: Self-Evolution Proposals
When quality drops (composite < 0.5, review_failed, eval_failed), agent proposes changes with `evidence_session_ids`:
- **Config** — auto-applicable after approval
- **Profile** — auto-applicable after approval
- **Identity** — always requires human judgment
- **Code** — never auto-applied (manual apply only)

Append-only JSONL. Lifecycle: `Proposed → Approved → Applied | Rejected`. Capped at 3 pending, deduplicated by title prefix. Auto-generates `SELF_EVOLUTION.md`.

### Detail: Argus Observability
Full session analysis producing an `ArgusReport`:
- **Drift detection** — compares recent N sessions vs baseline N for quality regression. Reports `Regressed`, `Stable`, `Improving`, `InsufficientData`.
- **Pattern detection** — identifies repeated work (same goal/task across days), failure clusters (grouped by outcome type), token hotspots (phase/provider consumption).
- **Directives** — actionable operator recommendations: "Tighten completion discipline", "Reduce retry thrash", "Trim the Reflect phase first."

Without Argus, scores are just numbers going up and down. This makes them actionable.

### Detail: Model Canary (Health Probe Routing)
Automated health probes per provider. Sends `"Reply: PraxisCanaryReady"` + runs evals on response:
- **Freeze on failure** — problematic provider gets 0% traffic immediately
- **Gradual recovery** — 0.125 weight gain per passing cycle, needs 3 consecutive passes to unfreeze
- **Persisted state** — `canary_frozen.json` + `route_weights.json`

**Note:** Hermes has rate-limit-based provider fallback and circuit breakers, but no proactive health probing. This is more sophisticated — detects degradation *before* a real session fails.

### Detail: Progressive Context Loading
Walks from CWD up to git root, discovers `.praxis.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules` per directory. Files from deeper directories override parent dirs. Injected as progressive context in Orient phase. **✅ Already done in Praxis** (commit 5f40ea5).

---

## Tier 2 — Strong Utility, Port When Needed

| # | Feature | Source | Effort | Dependencies |
|---|---------|--------|--------|--------------|
| 8 | **Tool Policy Hardening** | `src/tools/policy.rs` | Low-Medium | Tool system |
| 9 | **Synthetic Example Flywheel** | `src/examples.rs` | Low/Medium | Ideally #3 (Scoring) |
| 10 | **Adaptive Context Budgeting** | `src/context/adaptive.rs` | Medium | Context engine |
| 11 | **Snapshot & Bundle System** | `src/archive/` | Medium | None |
| 12 | **Injection Protection Scanner** | `src/context/injection.rs` | Low | Context loading |
| 13 | **Opportunity Mining & Auto-Goals** | `src/learning/` | Medium | Requires #5 (Argus) |
| 14 | **Merkle Audit Trail** | `src/merkle.rs` | Low | None |

### Detail: Tool Policy Hardening
Security policy validation for tool requests:
- Write-path circuit breaker (max 8 paths per request)
- Protected file limits (max 2 protected files per request)
- Locked path prevention (SOUL.md, praxis.toml, .env, tools/)
- Data-directory escape detection with path normalization
- Payload size limits (4KB max append)

**Note:** This is different from the secret redaction Hermes already has. This prevents the agent from *writing* to locked control-plane files — a distinct security concern.

### Detail: Synthetic Example Flywheel
Every session auto-generates a (context, action, outcome) training triple tagged with quality score. Stored in `evals/examples.jsonl` (500 max, oldest pruned). Creates a self-curating few-shot dataset — filter by `quality_score > 0.7` for high-signal data with zero manual annotation. Used during Orient for few-shot prompting.

**Caveat:** Filtering by score requires the scoring system (#3) first. Without it, it's just logging.

### Detail: Adaptive Context Budgeting
Tracks which context sources (identity, task, journal, etc.) correlate with success vs. failure. Successful sources get up to 1.2x budget, failing sources down to 0.8x. Total preserved via re-normalization. Persisted as `adaptive.json`. Self-optimizing — no operator tuning needed.

### Detail: Snapshot & Bundle System
Four sub-systems:
- **Audit** — exports session/approval/review/eval summaries as markdown reports
- **Bundle** — full data directory export/import with manifest, schema validation, path rebasing
- **Snapshots** — daily automatic snapshots with configurable retention, pruning
- **Tree** — safe directory copy/restore with symlink rejection, path normalization, escape detection

### Detail: Injection Protection Scanner
Scans context files (SOUL.md, AGENTS.md, tool manifests, .cursorrules) for 20+ known prompt injection patterns before loading: "ignore previous instructions", token boundary escapes (`<|im_start|>`), system role escapes, identity overrides. False-positive mitigation for legitimate phrases like "you are a helpful". Returns threat category, line number, and context snippet. **✅ Already done in Praxis.**

### Detail: Opportunity Mining & Auto-Goals
Feeds Argus reports into candidate generation:
- Drift regression → "Stabilize runtime quality drift" opportunity
- Repeated work → "Automate recurring work" opportunity
- Accepted opportunities auto-create a goal in GOALS.md via `ensure_goal()`
- Throttled at 2/day, 5/week to prevent spam

### Detail: Merkle Audit Trail
Append-only JSONL with SHA-256 hash chaining. Each entry includes the hash of the previous entry. `verify()` recomputes every hash end-to-end. Tamper-evident action history — any past modification breaks the chain. Unique to Praxis — no equivalent in any agent framework.

---

## Tier 3 — Nice to Have

| # | Feature | Source | Effort | Dependencies |
|---|---------|--------|--------|--------------|
| 15 | **Credential Vault** | `src/vault.rs` | Low | None |
| 16 | **Session Postmortems** | `src/postmortem.rs` | Trivial | Session data |
| 17 | **Dual-Mode Usage Budgets** | `src/usage/budget.rs` | Low | LLM provider layer |
| 18 | **Watchdog Supervisor** | `src/watchdog/` | High | Binary deployment |
| 19 | **Wave Execution** | `src/wave/mod.rs` | Medium | Tool dispatch |
| 20 | **Speculative Execution** | `src/speculative/mod.rs` | Low | LLM planner |
| 21 | **System Anomaly Snapshots** | `src/anomaly.rs` | Trivial | None |
| 22 | **Evaluator Loop** | `src/quality/evaluate.rs` | Low | None |
| 23 | **Cross-Session Compaction** | `src/context/compaction.rs` | Low | Session management |

### Detail: Credential Vault
TOML-based credential store with transparent encryption via `master.key`. Supports env-var references with fallbacks, literal auditing. More structured than raw `.env` but Hermes's existing `agent/redact.py` covers the leak-prevention use case. Lower priority.

### Detail: Session Postmortems
Auto-appends to `POSTMORTEMS.md` when session has bad outcome ("review_failed", "eval_failed", "blocked_loop_guard", or any eval failures). Records session ID, outcome, goal, task, summary, reviewer findings, failed eval results.

### Detail: Dual-Mode Usage Budgets
Two modes: `Run` (autonomous sessions) and `Ask` (quick queries). Each tracks: max_attempts, max_tokens, max_cost_usd. Defaults: Run=6/3000/$0.25, Ask=1/600/$0.05. Cost-aware prevents bill shock.

### Detail: Watchdog Supervisor
Separate process owning cron schedule, spawning agent sessions, writing heartbeats, managing binary updates with canary-gated rollback. Heavy lift — infrastructure-level reliability.

### Detail: Wave Execution
Dependency-aware parallel work scheduling via topological sort. Runs independent tasks in parallel waves. Useful for multi-step pipelines.

### Detail: Speculative Execution
Compares multiple plan branches against success criteria before committing. Prevents costly wrong-first-try patterns. Lightweight — keyword-matching based.

### Detail: System Anomaly Snapshots
CPU load, process RSS, disk usage captured at session boundaries. Flagged on high load/memory/failure. Correlates resource pressure with degraded performance.

### Detail: Evaluator Loop
Generator → evaluator iterative refinement. Configurable max rounds (default 3). For high-stakes content refinement.

### Detail: Cross-Session Compaction
Signals next session to start fresh via compaction request file. Different from Hermes's intra-session compressor — this is about session boundaries, not mid-session summarization.

---

## Drey's Independent Findings

### Three-Layer Quality Architecture
All execute during Reflect phase:
1. **Reviewer** (goal-level validation via shell commands against `GoalCriteria` JSON)
2. **Evaluator** (behavioral correctness, severity-tiered: Normal / TrustDamaging)
3. **Gates** (deterministic output filtering)

This layered approach catches different failure modes without relying on a single pass/fail signal.

### Scoring Philosophy
The Irreplaceability Score measures "how much the operator would lose if the agent were replaced with a naive assistant" — not generic task-completion. The four dimensions (anticipation, follow-through, reliability, independence) capture agent-specific accumulated value that a fresh instance wouldn't have.

---

## Execution Priority (Tuck's Recommendation)

If porting in practical order:

1. **Output Quality Gates** — NonEmpty + ForbiddenPhrase + MaxLength, no credential scrub needed
2. **Boundary Review** — alignment check, ~170 line port
3. **Session Scoring** — measurement foundation
4. **Self-Evolution** — feedback loop
5. **Argus** — makes scores actionable
6. **Model Canary** — provider reliability
7. **Progressive Context** — behavior per directory

Everything else (#8–23): cherry-pick as needed.

---

## Summary

- **23 total features** Praxis has that Hermes genuinely lacks (after corrections)
- **7 in Tier 1** (closed-loop self-improvement + safety)
- **7 in Tier 2** (operational quality of life)
- **9 in Tier 3** (cherry-pick as needed)
- **2 already done** in Praxis: progressive context (#7), injection scanner (#12)
- **Killer combo:** Tier 1 #3–5 (Scoring → Argus → Evolution) — that's the differentiation. Without it, Hermes never learns from its own sessions.

---

*Compiled by Tuck + Drey, April 25, 2026*
*Corrected: Hermes already has redact.py (credential scrubbing), tools/approval.py (session-scoped approvals), and nous_rate_guard.py (provider fallback). Remaining features are genuinely absent.*
