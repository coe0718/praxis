# Vex — Deep Code Review: Praxis

**Reviewed by:** Vex  
**Date:** May 12, 2026  
**Codebase:** `/mnt/docker/code/praxis-clean` — Rust autonomous AI agent daemon  
**Scope:** Full security + quality review, ~108 modules, ~50k+ lines

---

## Severity Key

| Tag | Meaning |
|-----|---------|
| 🔴 CRITICAL | Must fix before production use — security vulnerability, data loss, or crash |
| 🟡 WARNING | Should fix — correctness issue, race condition, or defense-in-depth gap |
| 💡 SUGGESTION | Consider — performance, clarity, or ergonomic improvement |

---

## 🔴 CRITICAL (12)

### C1. Shell metacharacter validation is bypassable
**File:** `src/tools/execute.rs:719-720`  
**Issue:** `shell-exec` passes raw `command` string to `/bin/bash -c`. `validate_shell_command` blocks `; | & \ $ ( ) < > \n \r \\` but misses tab, null bytes, and Unicode homoglyphs. The denylist is explicitly advisory (line 676 comments). **If approval bypass occurs (see C4), this is RCE.**  
**Fix:** Use `Command::new()` with explicit args instead of `bash -c`. If `bash -c` is required, shell-escape all arguments or use a proper argument parser.

### C2. Docker mount path injection
**File:** `src/docker_isolation.rs:90-96`  
**Issue:** `MountSpec.source` and `target` are interpolated directly into `docker run -v` with zero sanitization. `source: "/:/hostfs"` would mount the entire root. `source: "/var/run/docker.sock"` would expose the Docker socket.  
**Fix:** Validate mount sources against an allowlist of permitted paths. Reject absolute source paths unless explicitly whitelisted.

### C3. DockerIsolation `Default` impl panics
**File:** `src/docker_isolation.rs:138-141`  
**Issue:** `impl Default for DockerIsolation` calls `Self::new().unwrap()`. If Docker isn't installed or the socket is unreachable, this panics.  
**Fix:** Return a no-op stub, or return `Result` instead of using `Default`.

### C4. MCP HTTP endpoint bypasses init handshake
**File:** `src/mcp/server.rs:34`  
**Issue:** Dashboard `/mcp` dispatch sets `initialized = true` immediately. Any HTTP client can call `tools/call` with zero auth. Combined with `file-read` being auto-approved (level 1), this is **unauthenticated arbitrary file read** within the sandbox.  
**Fix:** Require the MCP initialization handshake or an auth token on the HTTP endpoint.

### C5. Bus TOCTOU race causes message loss
**File:** `src/bus/mod.rs:110-117`  
**Issue:** `drain()` reads events then truncates. Between `peek()` and `fs::write(&self.path, "")`, concurrent `publish()` writes are silently destroyed. No file locking or atomic swap.  
**Fix:** Write to a temp file, then `fs::rename` for atomic swap.

### C6. Matrix room_id URL path injection
**File:** `src/channels.rs:67-71`  
**Issue:** `self.room_id` interpolated directly into URL — no validation or URL-encoding. `../../` or `?` chars enable path traversal / request forgery.  
**Fix:** URL-encode room_id and validate it against an expected format.

### C7. Circuit breaker registry returns detached instances
**File:** `src/circuit_breaker.rs:175-191`  
**Issue:** `CircuitBreakerRegistry::get()` constructs a brand-new `CircuitBreaker` with zeroed state. Every call gets a fresh instance. **The entire circuit breaker system tracks nothing — repeated failures are invisible.**  
**Fix:** Store `Arc<CircuitBreaker>` in the registry; return a clone of the Arc.

### C8. Circuit breaker deadlock from inconsistent lock ordering
**File:** `src/circuit_breaker.rs:93-117`  
**Issue:** `record_success()` acquires `state` → `last_failure`. `record_failure()` acquires `last_failure` → `state`. Textbook deadlock under concurrent load.  
**Fix:** Consolidate all mutable state into a single `Mutex<InnerState>`.

### C9. Non-atomic memory hot→cold promotion
**File:** `src/storage/sqlite/memory_consolidation.rs:126-151`  
**Issue:** Cold insertion and hot deletion use separate connections and transactions. A crash between them duplicates memories in both tiers.  
**Fix:** Perform entire insert-cold + delete-hot in a single SQLite transaction on one connection.

### C10. Infinite loop with no iteration cap
**File:** `src/loop/runtime.rs:144-152`  
**Issue:** `while state.current_phase != SessionPhase::Sleep` has no max iterations. If a phase handler returns `Ok(())` without advancing state, the loop runs forever.  
**Fix:** Add `max_iterations = 100` hard cap with bail-on-exceed.

### C11. API key potential leakage through error chains
**File:** `src/backend/openai.rs:82`  
**Issue:** `bearer_auth(api_key.clone())` — connection-level errors in `with_context` chains may include the key in debug-formatted output.  
**Fix:** Use a redacted `Debug` wrapper around the API key, or sanitize error output before logging.

### C12. Docker container name collision
**File:** `src/docker_isolation.rs:63-67`  
**Issue:** Container name uses `timestamp_nanos_opt()`. Two executions in the same nanosecond (or clock resolution limits) cause Docker to refuse the second start.  
**Fix:** Append a random suffix: `format!("praxis-{}-{}-{}", tool_name, ts, rand::random::<u32>())`.

---

## 🟡 WARNING (18)

### W1. Corrupt vault file silently ignored
**File:** `src/tools/execute.rs:46`  
**Issue:** `Vault::load().unwrap_or_default()` — if vault file is corrupt, all secrets become unavailable with zero logging.  
**Fix:** Log a warning on corrupt vault.

### W2. Shell tool args whitespace splitting destroys quoting
**File:** `src/tools/execute.rs:253-263`  
**Issue:** `split_whitespace()` on `args` destroys quoting. `--msg "hello world"` becomes two args.  
**Fix:** Use `shlex::split()` or pass args as a JSON array.

### W3. Hardline blocklist uses substring matching, not regex
**File:** `src/tools/policy.rs:183-196`  
**Issue:** Patterns like `"curl .* | sh"` are treated as literal substrings. The `.*` is NOT regex.  
**Fix:** Use `regex::Regex` for actual regex matching, or rename patterns.

### W4. Sanitize function provides false sense of security
**File:** `src/injection.rs:59-63`  
**Issue:** `sanitize_input` only strips `<|`, `|>`, `<?`, `?>` chars.  
**Fix:** Rename to `strip_delimiter_markers()` or expand.

### W5. Rate limiter consumes tool token on global rejection
**File:** `src/rate_limit.rs:139-161`  
**Issue:** Tool-specific token consumed first, then global check. If global fails, the tool token is lost.  
**Fix:** Check global first, or refund the tool token on global rejection.

### W6. Plugin verification errors silently swallowed
**File:** `src/plugin_signing/mod.rs:127`  
**Issue:** `verify().unwrap_or(false)` — malformed signatures produce no error log.  
**Fix:** Log the error for verification failure and errors.

### W7. ProcessManager::new() creates channels with no receivers
**File:** `src/process_manager.rs:200-212`  
**Issue:** `mpsc::channel` handles created but `_rx` immediately dropped. Messages silently lost.  
**Fix:** Remove `new()` and force callers through `with_tool_executor()`.

### W8. execute_tool_with_result always returns success
**File:** `src/process_manager.rs:89-93`  
**Issue:** Sends then immediately returns `ToolResult { success: true, .. }`.  
**Fix:** Implement proper response channels or rename to `submit_tool`.

### W9. allowed_read_paths canonicalize follows symlinks
**File:** `src/tools/execute.rs:620-634`  
**Issue:** `fs::canonicalize` resolves symlinks. A symlink inside allowed root pointing outside can be exploited.  
**Fix:** Check for symlinks before canonicalization.

### W10. Compaction errors silently swallowed
**File:** `src/loop/runtime.rs:283-287`  
**Issue:** `.compact(...).await.ok()` — zero logging on failure.  
**Fix:** Log a warning on compaction failure.

### W11. Inactivity timeout leaves stale state on disk
**File:** `src/loop/runtime.rs:146-148`  
**Issue:** When timeout fires, in-memory phase becomes Sleep but `SessionState` is never persisted.  
**Fix:** Save state before returning from `run_once` on timeout.

### W12. PID file TOCTOU race
**File:** `src/daemon/mod.rs:153-161`  
**Issue:** `path.exists()` then `fs::write()` is not atomic.  
**Fix:** Use `fs::OpenOptions::new().write(true).create_new(true)` for atomic creation.

### W13. Regex compiled on every evaluation — ReDoS vector
**File:** `src/backend/rule.rs:42`  
**Issue:** `regex::Regex::new(pattern)` compiles on every eval.  
**Fix:** Compile and cache at load time.

### W14. Predictable UUID from nanosecond timestamp
**File:** `src/federation/mod.rs:264-268`  
**Issue:** Uses only `SystemTime::now()` nanoseconds as hex string.  
**Fix:** Use `uuid::Uuid::new_v4()`.

### W15. No bounds check on marketplace rating
**File:** `src/marketplace/reputation.rs:32`  
**Issue:** `rating: 99999.0` inflates `avg_rating`.  
**Fix:** Clamp rating to 0.0–5.0.

### W16. App secrets in serializable struct with no redaction
**File:** `src/zh_channels.rs:9-14`  
**Issue:** `ChinesePlatformConfig` holds `app_secret` with `#[derive(Serialize)]` and no `#[serde(skip)]`.  
**Fix:** Add `#[serde(skip)]` and redacted `Debug`.

### W17. Non-atomic wake_intent.json write
**File:** `src/session/spawn.rs:111-113`  
**Issue:** `fs::write` is not atomic. Crash mid-write leaves corrupt JSON.  
**Fix:** Write to temp file then `fs::rename`.

### W18. User memory load-modify-save race
**File:** `src/memory/user.rs:120-189`  
**Issue:** Loads from disk, mutates, saves. Two concurrent calls = last-writer-wins data loss.  
**Fix:** Use file locking or move into SQLite.

---

## 💡 SUGGESTION (10)

### S1. Streaming backend loses multi-modal content
**Suggestion:** Support proper multi-modal in `StreamChatMessage`.

### S2. Dead code: check_spawn_depth
**Suggestion:** Wire it up or remove it.

### S3. RuleBackend doesn't implement AgentBackend
**Suggestion:** Implement `AgentBackend` trait or remove.

### S4. Double-Mutex wrapping in response cache
**Suggestion:** Consolidate to single mutex.

### S5. Expired cache entries never evicted
**Suggestion:** Lazily remove expired entries during `get()`.

### S6. known_pricing allocates HashMap on every record()
**Suggestion:** Use `once_cell::sync::Lazy<HashMap>`.

### S7. Opens new SQLite connection per operation
**Suggestion:** Use connection pooling or shared `Mutex<Connection>`.

### S8. ephemeral_prompt is a prompt injection vector
**Suggestion:** Validate or sanitize ephemeral prompts.

### S9. Stub env vars allow LLM response injection
**Suggestion:** Guard behind `#[cfg(test)]`.

### S10. No dead-letter logging for malformed bus events
**Suggestion:** Log a warning and write to `bus_dead.jsonl`.

---

## ✅ Good Code Worth Noting

| Module | What's right |
|--------|-------------|
| `tools/policy.rs` | `normalize_relative()` — textbook path traversal prevention |
| `tools/execute.rs:641-645` | Symlink rejection in `file-read` |
| `tools/guard.rs` | Loop guard with 1/2/3-length pattern detection |
| `tools/tool_policy.rs` | Multi-level tool access control |
| `plugin_signing/mod.rs` | Ed25519 signature verification |
| `tools/execute.rs:512-550` | SSRF protection |

---

## Priority Fix Order

1. **C4** — MCP unauthenticated access
2. **C7+C8** — Circuit breaker completely broken
3. **C5** — Bus message loss
4. **C2** — Docker mount injection
5. **C3** — Default panic
6. **C6** — Matrix URL injection
7. **W3** — Hardline blocklist
8. **W7+W8** — Process manager silent loss
9. **W1** — Corrupt vault silence
10. **W17** — Non-atomic wake_intent write