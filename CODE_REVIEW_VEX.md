# Praxis — Full Code Review by Vex
**Date:** 2026-04-22  
**Scope:** Complete codebase (~32k lines Rust, ~38 TypeScript/React files)  
**Reviewer:** Vex  

---

## Executive Summary

Praxis is a self-hosted personal AI agent daemon in Rust (~32k lines) with a React frontend. The codebase shows strong engineering discipline — no `unwrap()` in production paths, consistent `anyhow::Result` with `.context()`, good test coverage with integration tests. However, the security boundary enforcement has significant gaps that need immediate attention.

**Total findings: 12 CRITICAL, 23 WARNING, 30+ SUGGESTION**

The two most urgent issues are:
1. **Shell command injection via newline bypass** in `execute.rs` — allows arbitrary command execution
2. **OAuth token exfiltration** via HTTP tools — all OAuth tokens sent to arbitrary URLs

---

## CRITICAL Findings

### C1. Shell Command Injection via Newline Bypass
**File:** `src/tools/execute.rs:25`  
`DANGEROUS_SHELL_CHARS` blocks `;`, `|`, `&`, `` ` ``, `$`, `(`, `)`, `<`, `>` — but **not newline (`\n`), carriage return (`\r`), or backslash (`\`)**. In `exec_shell_command`, user-supplied `command` is passed to `bash -c`. A payload like `{"command": "echo hello\nrm -rf /"}` passes validation and bash executes both commands.  
**Fix:** Add `'\n'`, `'\r'`, `'\\'` to `DANGEROUS_SHELL_CHARS`, or switch from `bash -c` to direct `Command::new(cmd).args(...)`.

### C2. OAuth Token Exfiltration via HTTP Tools — Missing Provider Filter
**File:** `src/tools/execute.rs:276-284`  
`run_http` injects **ALL** non-expired OAuth tokens as headers into **every** HTTP request, with no filtering by `manifest.allowed_oauth_providers`. Combined with no SSRF protection, an approved request to `http://evil.com/capture` leaks every OAuth access token.  
**Fix:** Apply the same `allowed_oauth_providers` filter used in `run_shell`/`exec_shell_command` to the `run_http` OAuth loop.

### C3. Daemon + `run_once` Double-Consume Drops Urgency
**File:** `src/daemon.rs:297` + `src/loop/runtime.rs:61`  
The daemon calls `consume_intent` (which **deletes** the file) at line 297, then passes the task to `run_once`, which calls `consume_intent` again at line 61 and gets `None`. Urgent wake intents during quiet hours are silently deferred.  
**Fix:** Set `force: true` in `RunOptions` when daemon detects a wake intent.

### C4. SQL Injection Surface in Schema Migration
**File:** `src/storage/sqlite/schema.rs:101,119`  
Table/column names interpolated via `format!()` into DDL. All current callers use hardcoded literals, but the function is `pub(super)` — any future caller with derived strings introduces injection.  
**Fix:** Validate identifiers against `^[a-z_][a-z0-9_]*$`.

### C5. Non-Atomic Hot/Cold Memory Insert (No Transaction)
**File:** `src/storage/sqlite/memory.rs:13-41`  
Two dependent INSERTs (main table + FTS index) run without a transaction. If FTS insert fails, the row is missing from the search index silently.  
**Fix:** Wrap both inserts in `connection.transaction()`.

### C6. Discord/Slack Webhooks Behind Auth — Deadlock
**File:** `src/dashboard/server.rs:175-184`  
Webhook routes are behind `require_auth` middleware. Discord/Slack send webhooks without bearer tokens. If `PRAXIS_DASHBOARD_TOKEN` is set, webhooks are rejected (401). If unset, the dashboard is unauthenticated.  
**Fix:** Move webhook routes to public routes with platform-specific signature verification.

### C7. MCP `resources/read` — Arbitrary File Read in Data Directory
**File:** `src/mcp/server.rs:175-233`  
Strips `praxis://` from URI and joins to `data_dir`. Non-`praxis://` URIs pass through. An authenticated user can read `praxis.db`, vault file, config with secrets.  
**Fix:** Restrict to only files enumerated in `collect_resources()`.

### C8. MCP `tools/call` Queues Approvals Without Tool Validation
**File:** `src/mcp/server.rs:83-168`  
Tool name used directly without verifying it corresponds to a registered manifest. MCP client can queue phantom approvals.  
**Fix:** Validate `tool_name` against registry before queueing.

### C9. Shell Command Execution from Data-Driven Config
**Files:** `src/quality/evals.rs:182-196`, `src/quality/reviewer.rs:187-229`  
Eval/reviewer execute shell commands from JSON files in data directory. If an attacker can write to these files (via agent manipulation), they achieve arbitrary code execution.  
**Fix:** Use command allowlist or sandboxed execution.

### C10. Predictable Pairing Code — Auth Bypass
**File:** `src/messaging/pairing.rs:101-105`  
`generate_code()` derives 6-digit code from `SystemTime::now().subsec_nanos()` — only ~20 bits of entropy. Attacker who observes timing can brute-force ~1M code space.  
**Fix:** Use `rand::random::<u32>() % 1_000_000` + rate limiting + code expiry.

### C11. Prompt Caching Dead Code
**File:** `src/backend/claude.rs:40`  
Checks `max_output_tokens >= CACHE_MIN_TOKENS(1024)` — output tokens are never ≥1024. Feature silently never activates.  
**Fix:** Compare against input text token count, not output limit.

### C12. SSE Token in Vault Transmitted Plaintext / No CSRF
**Files:** `src/contexts/SSEContext.tsx:30`, `src/lib/api.ts` (all POST endpoints)  
Auth token in URL query param (logs, history, referrer). Vault secrets in plaintext API calls. No CSRF protection on state-changing requests.

---

## WARNING Findings

### W1. No SSRF Protection for HTTP Tools
**File:** `src/tools/execute.rs:246-264`  
No blocklist for `localhost`, `127.0.0.1`, `169.254.169.254`, private ranges.

### W2. Unbounded Command Output — Memory Exhaustion
**File:** `src/tools/execute.rs:159-183, 525-542`  
`wait_with_output()` buffers all stdout/stderr. `cat /dev/urandom` exhausts memory.

### W3. Hook Scripts Enable Privilege Escalation
**File:** `src/hooks.rs:306-374`  
Hook scripts can auto-approve any tool request. No content/integrity verification.

### W4. URL Parameter Injection in HTTP Tools
**File:** `src/tools/execute.rs:328-334`  
`substitute_params` does raw string replacement — param values can manipulate URL target.

### W5. TOCTOU Race in file-read Symlink Check
**File:** `src/tools/execute.rs:412-422`  
Symlink check and actual read are not atomic.

### W6. Non-Atomic Multi-Table Provider Recording
**File:** `src/storage/sqlite/providers.rs:16-61`  
`record_attempts()` inserts into two tables without transaction — inconsistent billing data.

### W7. TOCTOU Race on Session Number Assignment
**File:** `src/storage/sqlite/sessions.rs:12-13`  
`SELECT MAX + 1` and `INSERT` not in transaction — duplicate session numbers.

### W8. Approval Status Update + Read on Separate Connections
**File:** `src/storage/sqlite/approvals.rs:92-111`  
`get_approval()` opens a new connection — stale data between UPDATE and SELECT.

### W9. Non-Atomic Memory Consolidation
**File:** `src/storage/sqlite/memory_consolidation.rs:131-153`  
Cold memory created but source hot memories may not be cleaned up on failure.

### W10. Non-Atomic Memory Decay
**File:** `src/storage/sqlite/memory_decay.rs:12-61`  
Batch of individual UPDATEs without transaction — inconsistent decay states.

### W11. Per-Operation Connection Creation (No Pooling)
**File:** `src/storage/sqlite/mod.rs:45-56`  
New connection per operation. Multi-statement atomicity impossible for most operations.

### W12. LIKE Pattern Injection
**File:** `src/storage/sqlite/ops.rs:54,111`  
User-supplied `query` not escaped for `%`/`_` wildcards — unintended matches.

### W13. Error Bodies May Contain API Keys
**Files:** `src/backend/claude.rs:76`, `openai.rs:61`, `ollama.rs:38`, `discord.rs:110,139,229`, `slack.rs:101,215`  
Provider error responses included verbatim in `bail!()` — may leak API keys into logs/DB.

### W14. OAuth Token Loss on Load Error
**File:** `src/oauth/store.rs:67`  
`load().unwrap_or_default()` — transient error causes all other tokens to be overwritten.

### W15. No Token Refresh on Expiry
**Files:** `src/oauth/github_client.rs:40-42`, `src/oauth/gmail.rs:33-35`  
Returns `Ok(None)` when expired instead of attempting refresh. Gmail becomes non-functional after ~1 hour.

### W16. Race Condition in Messaging Offset Persistence
**File:** `src/messaging/telegram.rs:101-119`  
Overlapping `poll_once` calls can re-process messages or lose them.

### W17. Discord Channel ID Parsing Overflow
**File:** `src/messaging/router.rs:400`  
`channel_id.parse().unwrap_or(0)` — snowflake IDs don't fit i64 consistently.

### W18. Slack Channel ID Stripping Creates Collisions
**File:** `src/messaging/router.rs:412-417`  
Stripping non-digit chars from `C01234ABCD` creates ambiguous numeric IDs.

### W19. SecurityOverrides Level Bypass
**File:** `src/config/security.rs:14-15`  
Override applied after validation — `level = 0` or `level = 99` bypasses bounds check.

### W20. CSP Allows `unsafe-inline`
**File:** `src/dashboard/server.rs:62-63`  
Weakens XSS protection significantly.

### W21. `api_config` Exposes Full Config with Potential Secrets
**File:** `src/dashboard/routes_core.rs:109-122`  
Returns raw `praxis.toml`, `providers.toml`, `budgets.toml`.

### W22. Evolution Store Not Truly Append-Only
**File:** `src/evolution.rs:221,238,258`  
`rewrite_all()` truncates — crash mid-rewrite corrupts the log.

### W23. Race Conditions in Goal ID Generation
**Files:** `src/dashboard/helpers.rs:75-90`, `src/identity/goals.rs:151-159`  
Concurrent requests produce duplicate IDs.

---

## SUGGESTION Findings (Highlights)

- **S1:** `expect()` in daemon signal handler (`daemon.rs:510-511`) — could panic production daemon
- **S2:** `file_exists:` allows arbitrary filesystem probing (`planner.rs:96`)
- **S3:** `env:` leaks environment variable existence (`planner.rs:99`)
- **S4:** Triple duplicate `glob_match` implementations (`cooldown.rs`, `sandbox.rs`, `hooks.rs`)
- **S5:** Sandbox allow-by-default for unknown channels (`sandbox.rs:259`)
- **S6:** No HTTPS enforcement for HTTP tool endpoints (`execute.rs:258`)
- **S7:** `read_jsonl_tail` uses O(n) `Vec::remove(0)` (`helpers.rs:106`)
- **S8:** Silent JSONL parse failures mask data corruption (multiple files)
- **S9:** No `FOREIGN KEY` enforcement in SQLite schema
- **S10:** MCP client uses blocking HTTP in async context (`client.rs:4`)
- **S11:** Health/metrics endpoints behind auth — incompatible with monitoring
- **S12:** `skills::read_skill_content` potential path traversal (`skills/mod.rs:123`)
- **S13:** No confirmation dialogs on destructive frontend actions (Vault delete, etc.)
- **S14:** Duplicate ErrorBoundary components (`components/` and `components/ui/`)
- **S15:** `after:` timestamp parse failure silently ignored (`planner.rs:103`)
- **S16:** `wake_when: env:` allows GOALS.md to probe any env var

---

## Positive Observations

- **Zero `unwrap()` in production paths** — all confined to `#[cfg(test)]` blocks
- **Consistent error handling** — `anyhow::Result` with `.context()` throughout
- **Parameterized SQL** — all data-value queries use `params![]`, no string interpolation
- **`next_approved_request()`** in approvals is the gold standard (IMMEDIATE transaction, claim+commit)
- **WAL mode + busy_timeout** correctly configured
- **No `dangerouslySetInnerHTML`** in React frontend — zero XSS surface from rendering
- **No hardcoded secrets** in frontend
- **Good test structure** — integration tests in `tests/` with tmp data dirs

---

## Priority Remediation

| Priority | Issue | Effort |
|----------|-------|--------|
| **P0 — Immediate** | C1: Newline injection in shell-exec | Small |
| **P0 — Immediate** | C2: OAuth token exfiltration | Small |
| **P0 — Immediate** | C3: Daemon urgency loss | Small |
| **P1 — This Week** | C5: Memory insert transactions | Small |
| **P1 — This Week** | C6: Webhook auth deadlock | Medium |
| **P1 — This Week** | W1: SSRF protection | Medium |
| **P1 — This Week** | W2: Bounded command output | Small |
| **P2 — Soon** | C4: Schema migration validation | Small |
| **P2 — Soon** | C7-C8: MCP validation | Small |
| **P2 — Soon** | C10: Secure pairing codes | Small |
| **P2 — Soon** | W6-W12: Storage transactions | Medium |
| **P3 — Backlog** | All SUGGESTION items | Varies |

---

*Review completed by Vex. Full codebase coverage: 170+ Rust source files, 38 TypeScript/React files, 28 integration tests.*
