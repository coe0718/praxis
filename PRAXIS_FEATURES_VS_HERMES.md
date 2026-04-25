# Praxis Features Missing From Hermes Agent

*Compiled by Drey + Tuck, 2026-04-25. Features Praxis has that Hermes Agent does not, ranked by porting value.*

---

## Tier 1 — Actually Worth Porting

| # | Feature | Description | Porting Effort |
|---|---------|-------------|---------------|
| 1 | **Session Scoring** | 4-dimension quality metric per session (anticipation, follow-through, reliability, independence). Rolling averages show if the agent is improving or degrading over time. Neutral-on-absence defaults prevent idle sessions from dragging scores down. | Low |
| 2 | **Self-Evolution** | Agent proposes config/identity changes after bad sessions (composite < 0.5, review_failed, eval_failed). Tiered autonomy: Config auto-applicable after approval; Identity always requires human judgment. Evidence-backed (`evidence_session_ids`). | Medium-High |
| 3 | **Argus Observability** | Drift detection (is quality regressing?), pattern detection (repeated work, failure clusters), automated directives ("tighten completion discipline"). Actual analytics, not just logs. | Medium |
| 4 | **Opportunity Mining** | Feeds Argus results into auto-generated improvement candidates. Accept one and it creates a goal in GOALS.md automatically. Throttled at 2/day, 5/week. Deduplicated by content signature. | Medium |
| 5 | **Model Canary** | Automated health probes per provider. Sends a deterministic probe, runs evals on the response. Failed provider → auto-frozen (0% traffic). Gradual recovery: 0.125 weight gain per passing cycle, 3 consecutive passes to unfreeze. Auto-rollback on failure. | Medium |

**The killer combo:** #1 → #2 → #3 → #4 forms a closed loop. Score sessions → detect drift → mine opportunities → propose evolution. The agent gets better over time without operator intervention.

---

## Tier 2 — Would Use Regularly

| # | Feature | Description | Porting Effort |
|---|---------|-------------|---------------|
| 6 | **Tool Cooldowns** | "Trust this write to JOURNAL.md for 30 min." No more approving the same tool on the same file every turn. Session-scoped with configurable duration. | Low |
| 7 | **Progressive Context** | Walks directory tree from CWD to git root, discovers `.praxis.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules` per directory. Agent behaves differently in `src/` vs `tests/` automatically. | ✅ DONE (in Praxis) |
| 8 | **Tool Policy Hardening** | Write-path circuit breakers, locked files (SOUL.md, .env), path traversal detection, payload size limits. Production safety net for tool execution. | Low-Medium |
| 9 | **Adaptive Context Budget** | Tracks which context sources actually correlate with success and allocates more budget to them, less to the others. Learns which sources matter. | Medium |
| 10 | **Snapshot / Bundle** | Daily auto-snapshots with retention, full export/import with validation. Actual disaster recovery for agent state. | Medium |

---

## Tier 3 — Nice, Not Urgent

| # | Feature | Description | Porting Effort |
|---|---------|-------------|---------------|
| 11 | **Injection Protection Scanner** | Scans AGENTS.md, SOUL.md, .cursorrules, and tool manifests for prompt injection patterns (18 known patterns) before loading into context. Blocks loading flagged files. | ✅ DONE (in Praxis) |
| 12 | **Session Postmortems** | Auto-writes postmortem analysis after failures. Records what went wrong, what pattern was detected, and recommended fixes. | Low |
| 13 | **Quality Gates** | Deterministic output filters — no LLM call needed. `CredentialScrubGate` (redacts `sk-...`, `sk-ant-...`, `Bearer ...`), `NonEmptyGate`, `ForbiddenPhraseGate`, `MaxLengthGate`. First non-Pass decision wins. | Low |
| 14 | **Credential Vault** | Centralized TOML secret store with encryption-at-rest (`master.key`). Supports literal values (dev-only, warned) and env-var references with fallbacks. `audit_literals()` runs at startup. | Low |
| 15 | **Watchdog Supervisor** | Separate process (`praxis-watchdog`). Auto-restart on crash, canary-gated binary updates, rollback on failure. Keeps the daemon alive. | High |
| 16 | **Dual-Mode Usage Budgets** | Separate `run` (autonomous) vs `ask` (operator-requested) token+cost tracking. Prevents autonomous sessions from burning through all budget. | Low-Medium |
| 17 | **Boundary Review** | 7-day forced alignment heartbeat. Parses SOUL.md for `## Boundaries` section, tracks `last_confirmed_at`, prompts operator when review is due. Prevents silent constraint drift. | Low |
| 18 | **Wave Execution** | Dependency-aware parallel tool scheduling. Tools with no interdependencies run concurrently. Respects approval gates. | Medium-High |
| 19 | **Speculative Execution** | Rehearsal tournament comparing candidate action plans before committing. Scores branches against keyword-based success criteria and trust-constraint penalties (e.g. penalizing "force push"). Currently store-only in Praxis. | Medium |
| 20 | **System Anomaly Snapshots** | Correlates resource pressure (CPU, memory, disk) with session failures. Stored in `system_anomalies.jsonl`. Builds a failure correlation dataset over time. | Low-Medium |
| 21 | **Cross-Session Compaction** | Signals next session to start fresh with a compaction request. Prevents context window bloat across session boundaries. | Low |
| 22 | **Evaluate Loop** | Generate→review→refine iteration pattern. Generator produces content, evaluator checks criteria, failures return structured feedback, generator retries. Capped at `max_rounds` (default 3). Reusable trait. | Low-Medium |

---

## Drey's Independent Findings (Cross-Validated)

Independently identified these as unique and worth porting before seeing Tuck's list:

- **Synthetic Example Flywheel** (`examples.rs`) — Auto-generates `(context, action, outcome)` training triples after every session, tagged with quality score. Self-curating few-shot dataset. Max 500, oldest pruned. Used during Orient for few-shot prompting and as offline fine-tuning dataset.
- **Merkle Audit Trail** (`merkle.rs`) — Append-only JSONL with SHA-256 hash chaining. `verify()` recomputes every hash end-to-end. Tamper-evident action history — any past modification breaks the chain.
- **Irreplaceability Scoring Philosophy** — The score measures "how much the operator would lose if the agent were replaced with a naive assistant" — not generic task-completion. The four dimensions (anticipation, follow-through, reliability, independence) capture agent-specific accumulated value.
- **Three-Layer Quality Architecture** — Reviewer (goal-level validation via shell commands) → Evals (behavioral correctness, severity-tiered) → Gates (deterministic output scrubbing). All execute during Reflect phase.

---

## Summary

- **22 total features** Praxis has that Hermes lacks
- **5 in Tier 1** (closed-loop self-improvement: score → drift → mine → evolve)
- **5 in Tier 2** (operational quality of life)
- **12 in Tier 3** (cherry-pick as needed)
- **2 already closed** in Praxis this session (#7 progressive context, #11 injection scanner)
- **Killer combo:** Tier 1 #1–4 — that's the differentiation
