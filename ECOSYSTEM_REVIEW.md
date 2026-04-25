# Praxis Ecosystem Review — ClawCharts + Shelldex Cross-Reference

**Date:** 2026-04-20 | **Updated:** 2026-04-25 (stale entries fixed)
**Reviewed:** 10 ClawCharts repos, 36 Shelldex projects, cross-referenced against Praxis codebase
**Note:** Open action items folded into STATUS.md. This doc is historical benchmarking.

---

## 🔴 HIGH PRIORITY — Security Hardening (Vex's 3 criticals)

Praxis has a `sandbox.rs` but it's just per-channel policy gating — no actual code execution sandboxing.

### OpenFang (Rust, 137K LOC) — 16-layer security stack

- **WASM Dual-Metered Sandbox** — tools run with fuel metering + epoch interruption. Praxis's shell-exec runs raw bash with no isolation beyond approval queue. ❌ *Not implemented.*
- **Secret Zeroization** — auto-wipes API keys from memory when no longer needed. Praxis injects secrets into env without cleanup. ❌ *Not implemented.*
- **Loop Guard** — SHA256-based tool call loop detection. If the agent calls the same tool with same args 3x, it stops. ✅ *Implemented* in `src/tools/guard.rs` with 1–3 step pattern detection.
- **Prompt Injection Scanner** — detects override attempts in tool outputs and user messages. ✅ *Implemented* in `src/context/injection.rs` with 18 patterns.

### IronClaw (Rust, 11.6k★)

- **Merkle Hash-Chain Audit Trail** — cryptographically links all actions. Praxis has `anomaly.rs` but no tamper-evident chain. ❌ *Not implemented.*
- **CodeAct mode** — agent writes code blocks, Praxis executes them with state persistence between steps. More flexible than the current tool-dispatch model. ❌ *Not implemented.*

### NullClaw (Zig, 678KB)

- **ChaCha20-Poly1305 encrypted secrets** — Praxis stores API keys in plain env vars. ❌ *Not implemented; at-rest encryption exists for vault/oauth via AES-256-GCM but not in-memory zeroization.*
- **Autonomy levels (ReadOnly / Supervised / Full)** — cleaner than Praxis's per-tool approval flags. One global policy knob. ⚠️ *Partially implemented* — `ChannelSandbox` in `src/sandbox.rs` has `strict()` (read-only, force-approval) and `read_only()` presets, plus per-channel `max_security_level` caps, but no single global ReadOnly/Supervised/Full toggle.

---

## 🟡 MEDIUM PRIORITY — Memory Architecture Upgrade

Praxis has hot/cold/links with FTS5 + decay. But it's keyword-only.

### Hybrid Vector + Keyword

- **NullClaw** — Hybrid memory: SQLite BLOB embeddings + FTS5 keyword search + weighted merge. Same SQLite Praxis already uses, just add an embedding column and cosine similarity. No new infrastructure.
- **memU** (Python, 13.3k★) — memory-first proactive agent pattern. Instead of Orient→Decide→Act with memory as context, the agent's memory IS the driver. Worth studying the retrieval-first architecture.
- **MemOS** (Python, 8.4k★) — 72% lower token usage through intelligent memory retrieval vs loading full chat history. Praxis loads context aggressively; this approach could cut costs significantly.

### Implementation

Add an optional embeddings column to the existing hot/cold memory tables in SQLite. Use a local embedding model (or OpenAI embeddings API). Implement weighted hybrid retrieval (0.6 vector + 0.4 keyword). Same schema, additive column, backward compatible. ❌ *Not implemented — memory is keyword-only FTS5 today.*

---

## 🟢 NICE TO HAVE — Feature Ideas

### From OpenFang — "Hands" system

Praxis already has `hands.rs` but OpenFang's are fully autonomous — they run on schedules without prompting. The **Predictor** (superforecasting with confidence intervals) and **Researcher** (deep research with CRAAP credibility scoring) hands are genuinely useful. Praxis could add hand-level scheduling so a "researcher" hand runs nightly.

### From IronClaw — Mission system

Replaces "routines" with evolving, self-improving missions. They accumulate knowledge over time via meta-prompts built from Project knowledge. Praxis has `evolution.rs` for proposals but doesn't self-improve its own scheduled tasks.

### From PicoClaw — Rule-based model routing

Smart routing that picks the cheapest model capable of handling a task. Praxis uses one model; routing based on task complexity would save money.

### From ZeroClaw — Emergency stop

`zeroclaw stop` immediately halts all agent activity. Praxis has no kill switch beyond `kill`.

---

## 🗑️ NOT WORTH STEALING

- **OpenClaw** (360K★ TypeScript) — completely different stack. Architectural patterns don't translate to Rust.
- **Nanobot** (Python, research-focused) — too academic, "dream skill discovery" is interesting but unproven.
- **Moltworker** (Cloudflare Workers) — serverless doesn't fit Praxis's always-on daemon model.
- **MimicLaw, ApexClaw, LangBot** — too niche or too new (2★ repos).

---

## 📋 Specific Action Items for Praxis

1. **WASM sandbox for shell-exec** — biggest security gap. OpenFang's approach with fuel metering is the gold standard. ❌ *Open*
2. **Loop guard (SHA256 tool-call dedup)** — trivial to implement, prevents runaway agents. ✅ *Done* — `src/tools/guard.rs` with multi-step pattern detection.
3. **Secret zeroization** — Drop impl that wipes key bytes from memory. ❌ *Open*
4. **Hybrid memory retrieval** — add embedding column to SQLite, weighted merge with existing FTS5. ❌ *Open*
5. **Autonomy levels** — replace per-tool approval with global ReadOnly/Supervised/Full. ⚠️ *Partial* — channel sandboxing exists (`src/sandbox.rs`) but no global toggle.
6. **Kill switch** — `praxis stop` that signals the daemon to halt immediately. ✅ *Done* — `praxis daemon stop` sends SIGTERM via `src/cli/daemon.rs`.
7. **Rule-based model routing** — classify task complexity, route to cheap/fast or expensive/thorough model. ⚠️ *Partial* — `ProviderRoute` has `class` (`fast`/`reliable`/`local`) and router selects by class, but no automatic task-complexity classification.

---

## Priority Summary

- **Security stuff is most urgent** (especially given Vex's 3 criticals)
- **Memory upgrade is most impactful for daily use**
- **Everything else is polish**
