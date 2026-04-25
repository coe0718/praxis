# Praxis — Current Status
**Updated:** 2026-04-25 | **Replaces:** PLAN_TUCK.md + PLAN_SCOUT.md (both 2026-04-22)

---

## Complete ✅

### Security Fixes (Phase 1)
All 12 CRITICAL + 23 WARNING from Vex's review resolved:
- Shell injection hardening (newline/CR/backslash + 1MB output cap)
- OAuth token filtering by provider + SSRF blocking
- Daemon urgency fix (force:true on wake intent)
- SQL identifier validation against `^[a-z_][a-z0-9_]*$`
- Atomic memory inserts (transaction wrapping)
- Webhook signature verification (Discord ED25519 + Slack HMAC)
- MCP resource restriction + tool validation
- Cryptographic pairing codes (rand::thread_rng)
- Prompt caching fix (input token estimation)
- Error body truncation (200 chars)
- LIKE pattern escaping
- Atomic multi-table operations
- Config level validation (1-4 in core.rs, 1-3 in security.toml)
- CSP unsafe-inline removal

### Tooling
- Shell completions (bash/zsh/fish via clap_complete)
- Prompt injection protection (18 patterns, context/injection.rs)
- Clarify tool (pause agent, ask operator)
- Todo tool (JSON persistence)
- Memory tool (upsert/search/forget/list with tags)

### Operator Experience
- Progressive context files (walk tree to git root, discover .praxis.md/AGENTS.md/CLAUDE.md/.cursorrules)
- Persistent user memory (user_memory.json key-value store)
- Session search (`praxis sessions search <query>` — SQLite LIKE)
- Usage insights (`praxis insights [--days N]` — tokens, cost, provider breakdown)
- Health endpoint batch COUNT queries (single connection)

### Frontend (Phase 2A/2B)
- Session Timeline View (Orient→Decide→Act→Reflect bar chart)
- Approval Queue Search (filter by tool, status, text; debounced)
- Token Spend Tracking (bar chart from provider_usage)
- Agent Health Dashboard (heartbeat, DB, memory, approvals)

---

## In Progress / Recent

- CI: `cargo fmt`, `clippy -D warnings`, `cargo test --locked`, `cargo audit` all green
- 204 tests passing, zero warnings
- Cargo audit: 3 transitive warnings suppressed (paste, lru, rand)

---

## Not Done (from original plans)

### Phase 2A — Memory Architecture
- Vector search / embeddings (hybrid FTS5 + cosine similarity)
- Memory consolidation via embedding dedup

### Phase 2B — WASM Sandbox
- wasmtime fuel-metered tool execution
- Secret zeroization

### Phase 2C — Frontend
- Mobile-responsive layout
- Keyboard shortcuts

### Phase 3 — Advanced
- Goal dependency graph visualization
- Config diff on evolution proposals
- Session replay (step-by-step)
- Browser notifications (Service Worker + SSE)
- Autonomy levels (ReadOnly/Supervised/Full toggle)
- Rule-based model routing
- CodeAct mode
- Merkle audit trail
- Deterministic/offline mode

### STUB — Aspirational (zero implementation)
- Voice transcript streaming (Whisper → agent loop)
- Serverless/edge entry point (Cloudflare Workers / AWS Lambda)

---

## Backlog (SUGGESTION items from reviews)

**Praxis main (14):**
- `expect()` in daemon signal handler
- `file_exists:` / `env:` planner probes leak info
- Triple duplicate `glob_match` (cooldown/sandbox/hooks)
- Sandbox allow-by-default for unknown channels
- `read_jsonl_tail` O(n) `Vec::remove(0)`
- Silent JSONL parse failures
- No FOREIGN KEY enforcement in SQLite
- MCP client blocking HTTP in async context
- Health/metrics behind auth
- Skill path traversal potential
- No confirmation dialogs on destructive frontend actions
- Duplicate ErrorBoundary components
- `after:` timestamp parse silently ignored

**Phase 2A (5):** sleep phase in timeline, floating-point precision, useDeferredValue, layout fetches all approvals, dashboard fetches unused data

**Phase 2B (4):** hb_age magic -1, bar chart #123 labels, dead cost computation, token summary silent zeros

**Memory Search (3):** over-fetch limit/2, score conflates signals, template literal vs URLSearchParams

---

## SUGGESTION Backlog (from PatchHive review)
- AgentConfig Clone leaks keys on Debug
- max_tokens: 2000 hardcoded
- Raw LLM output in error messages
- /tmp work dir for git clones
- Frontend plain JS, no TypeScript
- Binary file reads in collect_files_all_sync
- Webhook secret not cached
- Contract version hardcoded "0.1.0"

---

## Environment
- **Repo:** `/mnt/docker/code/praxis` (Rust)
- **Branch:** `main`
- **Tests:** 204 passing, 0 warnings
- **CI:** fmt + clippy + test + audit + build all green
