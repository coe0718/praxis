# Praxis Codebase Security & Quality Review

**Reviewer:** Vex
**Date:** 2026-04-19
**Scope:** Full codebase review — security, SQL safety, tool system, Telegram, error handling, concurrency, data leaks

---

## Executive Summary

Praxis is a well-architected self-hosted AI agent framework with solid separation of concerns, a thoughtfully layered tool approval system, and proper use of parameterized SQL throughout. The codebase follows Rust best practices (`anyhow::Result` with `.context()`, warn-and-continue for non-fatal side effects).

However, there are **3 critical**, **4 high**, and **6 medium** severity issues that should be addressed before production deployment.

### Severity Overview

| Severity | Count | Categories |
|----------|-------|------------|
| 🔴 Critical | 3 | Dashboard auth, approval hook timeout, shell argument injection |
| 🟠 High | 4 | Fail-open sandbox, TOCTOU in approval, token env-var exposure, hook script path validation |
| 🟡 Medium | 6 | Telegram bot token in URL, master key file perms, no TLS for dashboard, vault literal in prod, error message leakage, concurrent SQL connections |

---

## 🔴 CRITICAL Issues

### C1: Dashboard HTTP Server Has Zero Authentication

**File:** `src/dashboard/server.rs` (lines 93–141)
**Severity:** CRITICAL — Remote exploit if dashboard port is network-reachable

The axum dashboard server binds to `host:port` with no authentication middleware whatsoever:

```rust
let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
axum::serve(listener, app).await?;
```

**Every endpoint is fully open to any network peer that can reach the port**, including:

- `POST /api/approvals/:id/approve` — approve any pending tool request
- `POST /api/approvals/:id/reject` — reject approvals
- `PUT /api/identity/:file` — write to IDENTITY.md, GOALS.md, etc.
- `POST /api/run` — force-execute agent sessions
- `POST /api/wake` — inject wake intents
- `POST /api/goals` — add arbitrary goals
- `POST /api/evolution/:id/approve` — approve self-modification proposals
- `GET /api/config` — read praxis.toml, providers.toml, budgets.toml (may contain secrets)

**Impact:** Any attacker with network access to the dashboard can:
1. Approve arbitrary tool requests (including shell-exec at security level 3)
2. Modify agent identity/goals
3. Read configuration files that may contain provider API keys
4. Force the agent to execute arbitrary tasks

**Recommendation:** Add bearer token auth middleware or API key validation. At minimum, read `PRAXIS_DASHBOARD_TOKEN` from env and reject requests without a matching `Authorization: Bearer <token>` header. Bind to `127.0.0.1` by default instead of `0.0.0.0`.

---

### C2: Approval Hook Timeout Is Never Enforced

**File:** `src/hooks.rs` (line 334)
**Severity:** CRITICAL — Denial of service / hook hang

```rust
let timeout = Duration::from_secs(hook.timeout_secs);
// ...
let _ = timeout; // timeout enforcement for approval hooks via wait_with_output
```

The timeout `Duration` is constructed but immediately discarded. `wait_with_output()` has **no built-in timeout**. A hung approval hook script will block the entire agent loop indefinitely, effectively creating a denial-of-service condition.

**Impact:**
- A misbehaving or malicious hook script can freeze the agent permanently
- The 10-second default timeout is not enforced
- No watchdog mechanism exists to recover

**Recommendation:** Implement the same `wait_with_timeout` + `child.kill()` pattern used by `fire_interceptor` and the shell tool execution paths. The codebase already has the correct pattern in `hooks.rs` lines 376–393.

---

### C3: shell-exec Command Injection via Approval Bypass

**File:** `src/tools/execute.rs` (lines 428–517)
**Severity:** CRITICAL — Arbitrary code execution

The `shell-exec` tool passes user-supplied strings directly to `/bin/bash -c`:

```rust
let mut cmd = Command::new("/bin/bash");
cmd.args(["-c", command]);
```

While this is gated by the approval system (level 3 + requires approval + rehearsal), the combination with C1 (no dashboard auth) means any network-adjacent attacker can approve and execute arbitrary shell commands.

Additionally, the `run_shell` function (lines 70–197) splits user-supplied payload arguments on whitespace:

```rust
let extra_args: Vec<String> = payload
    .params
    .get("args")
    .map(|s| s.split_whitespace().map(str::to_string).collect())
```

This means an argument like `"foo; rm -rf /"` would be split into `["foo;", "rm", "-rf", "/"]` and passed as individual arguments to the shell tool's executable, which may still be dangerous depending on the tool.

**Recommendation:**
1. Validate that `command` for `shell-exec` doesn't contain shell metacharacters after approval, or at minimum log a high-severity warning
2. Consider using `Command::new()` with argument vector instead of `bash -c` where possible
3. Ensure the dashboard auth (C1) is fixed first, as it directly enables this attack

---

## 🟠 HIGH Issues

### H1: Sandbox Defaults to Fail-Open on Error

**File:** `src/sandbox.rs` (lines 264–275)
**Severity:** HIGH — Security policy bypass

```rust
let store = match ChannelSandboxStore::load(&paths.sandbox_file) {
    Ok(s) => s,
    Err(e) => {
        log::warn!("sandbox: failed to load store: {e}");
        return SandboxVerdict::Allow;  // FAIL-OPEN
    }
};
```

If the sandbox configuration file is corrupt, unreadable, or has invalid JSON, all channel restrictions are silently bypassed. The default behavior is `SandboxVerdict::Allow`.

**Recommendation:** Default to `SandboxVerdict::Block` on parse errors, or at minimum `SandboxVerdict::RequireApproval`. The operator should be forced to fix a corrupt security policy rather than having it silently disabled.

---

### H2: Approval Race Condition (TOCTOU)

**File:** `src/storage/sqlite/approvals.rs` (lines 113–146)
**File:** `src/loop/phases.rs` (line 176, 449–450)
**Severity:** HIGH — Double execution of approved tool requests

The approval consumption flow is not atomic:

1. `decide()` calls `next_approved_request()` — reads the first approved request
2. `execute_tool_request()` executes it
3. `mark_approval_consumed()` — marks it as executed

Between steps 1 and 3, another concurrent session (or dashboard API call) could read the same approved request and also execute it. While the typical deployment runs a single daemon, the dashboard API can approve/reject while the daemon is running.

**Recommendation:** Wrap `next_approved_request()` + status update in a single SQL transaction, or add a `claimed_at` timestamp with an atomic `UPDATE ... WHERE status = 'approved' AND claimed_at IS NULL` pattern.

---

### H3: OAuth Tokens and Vault Secrets Injected as Environment Variables

**File:** `src/tools/execute.rs` (lines 124–141, 459–475)
**Severity:** HIGH — Credential leakage to child processes

Every shell tool execution receives:
1. All vault secrets as `VAULT_<NAME>` environment variables
2. All OAuth tokens as `PRAXIS_OAUTH_<PROVIDER>_TOKEN` environment variables

```rust
for (name, entry) in &vault.secrets {
    if let Some(value) = entry.resolve().ok().flatten() {
        let key = format!("VAULT_{}", name.to_ascii_uppercase().replace('-', "_"));
        cmd.env(key, value);
    }
}
```

**Impact:**
- A tool manifest pointing to an arbitrary executable can harvest all secrets
- The `shell-exec` tool (which runs `/bin/bash -c`) exposes all vault secrets to whatever command the agent runs
- There's no per-tool secret scoping mechanism

**Recommendation:**
1. Add an optional `exposed_secrets` field to `ToolManifest` to limit which secrets a tool receives
2. Don't inject vault secrets into `shell-exec` by default — require explicit opt-in per manifest
3. Document the risk clearly in operator-facing docs

---

### H4: Hook Scripts Executed Without Path Validation

**File:** `src/hooks.rs` (lines 245, 272, 311)
**Severity:** HIGH — Arbitrary code execution via hooks.toml

Hook scripts are executed directly without validating:
- That the path is absolute (no `path.is_absolute()` check)
- That the file is not a symlink
- That the file has appropriate permissions
- That the script is owned by the expected user

```rust
match Command::new(&script)  // script is user-provided PathBuf
    .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    .spawn()
```

If an attacker can modify `hooks.toml` or the script files, they achieve arbitrary code execution with the daemon's privileges. While this is a local-only attack vector, the `hooks.toml` is not listed in `is_locked_path()` (policy.rs line 136), so the agent itself could potentially modify it.

**Recommendation:**
1. Validate that script paths are absolute
2. Check that scripts are not symlinks before execution
3. Add `hooks.toml` to the locked path list so the agent cannot modify it

---

## 🟡 MEDIUM Issues

### M1: Telegram Bot Token in URL Path

**File:** `src/messaging/telegram.rs` (line 200–202)
**Severity:** MEDIUM — Token leakage via logs/referer

```rust
fn api_url(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}
```

The bot token is embedded in the URL path. While Telegram's API design mandates this, the token can leak through:
- HTTP client debug logs (reqwest logs URLs at debug level)
- Proxy/CDN access logs
- TLS inspection tools

**Recommendation:** Ensure log level is set to `info` or higher in production. Consider wrapping the token in a type that implements `Display` with redaction (e.g., `bot***redacted`).

---

### M2: Master Key File Permission Not Verified on Load

**File:** `src/crypto.rs` (lines 27–54)
**Severity:** MEDIUM — Key file may have been tampered with

When generating a new key, permissions are correctly set to 0600:

```rust
fs::write(path, key.as_slice())?;
set_permissions_600(path);
```

But when loading an existing key, there is **no permission check**:

```rust
if path.exists() {
    let raw = fs::read(path)?;  // No permission validation
    // ...
}
```

**Impact:** If the key file is world-readable (e.g., due to a backup restore with incorrect permissions), the encryption key is exposed.

**Recommendation:** Check `metadata.permissions().mode() & 0o777 == 0o600` before loading the key. Warn or refuse to load if permissions are too broad.

---

### M3: No TLS on Dashboard Server

**File:** `src/dashboard/server.rs` (line 137)
**Severity:** MEDIUM — All traffic including approvals is plaintext

```rust
let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
```

The dashboard uses plain HTTP. Combined with C1 (no auth), all API calls including approval decisions travel in cleartext.

**Recommendation:** Document that operators must use a reverse proxy (nginx/caddy) with TLS. Consider adding native TLS support via axum-server.

---

### M4: Vault Literal Secrets Warning Not Enforced

**File:** `src/vault.rs` (lines 200–210)
**Severity:** MEDIUM — Production secrets in config files

The `audit_literals()` function warns about literal vault entries, but it's never called during startup in a blocking fashion. Operators may run with literal secrets in `vault.toml` indefinitely.

**Recommendation:** Call `audit_literals()` during `praxis init` and the daemon startup path. Emit a prominent warning or refuse to start in non-dev mode with literal secrets present.

---

### M5: Error Messages Leak Internal State

**File:** `src/dashboard/server.rs` (throughout)
**Severity:** MEDIUM — Information disclosure

Error responses from the dashboard API include full error messages with internal context:

```rust
Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
```

This can reveal:
- Database schema details
- File system paths
- Internal configuration values

**Recommendation:** Return sanitized error messages to API clients. Log the detailed error server-side. Use a structured error type with a public message and a private detail.

---

### M6: Concurrent SQLite Access Without WAL Mode

**File:** `src/storage/sqlite/mod.rs` (lines 45–53)
**Severity:** MEDIUM — Potential lock contention

```rust
fn connect(&self) -> Result<Connection> {
    // Opens a new connection each time
    Connection::open(&self.path)
}
```

Each store operation opens a new connection. The dashboard API and the daemon loop can both access the database simultaneously. Without explicit WAL mode configuration, concurrent reads+writes may encounter `SQLITE_BUSY` errors.

**Recommendation:** Enable WAL mode during schema initialization (`PRAGMA journal_mode=WAL`) and set a busy timeout (`PRAGMA busy_timeout=5000`).

---

## ✅ Positive Findings

The following areas were reviewed and found to be well-implemented:

### SQL Safety
- **All SQL queries use parameterized statements** with `params![]` — no string interpolation in queries found anywhere in the codebase
- Query preparation via `connection.prepare()` with `?N` placeholders is consistent across all 18 SQLite module files
- No dynamic table/column name construction found

### Tool System Security
- Path normalization (`normalize_relative`) correctly handles `..`, absolute paths, and prefix components
- Symlink detection in `file-read` and `praxis-data-write` paths
- Circuit breaker limits on write paths (8 max), protected files (2 max), and append size (4KB)
- Locked paths prevent agent from modifying `SOUL.md`, `praxis.toml`, `.env`, and `tools/` directory
- The `SecurityPolicy.validate_request()` provides defense-in-depth

### Encryption
- AES-256-GCM with proper random nonces (12 bytes via `OsRng`)
- Nonce-per-encryption (no reuse)
- `PRAXISENC1:` prefix enables transparent detection
- Key file created with 0600 permissions on Unix

### Telegram Security
- Chat ID allowlist enforced before message processing
- Unknown chat pairing flow requires operator approval
- Private vs. group chat distinction respected
- `sender_id` tracked for accountability

### Code Quality
- Consistent error handling with `anyhow::Result` + `.context()`
- `unwrap()` in production code is limited to:
  - `Vault::load().unwrap_or_default()` (execute.rs:34) — safe, defaults to empty vault
  - `HookRunner::load().unwrap_or_default()` (hooks.rs:228) — safe, defaults to no hooks
  - `load_offset().unwrap_or(0)` (telegram.rs:101) — safe, defaults to offset 0
- No `unsafe` blocks in production code (only in test code for `env::set_var`)
- Clean separation of concerns via trait-based generics

---

## 📋 Detailed File-Level Notes

### `src/tools/execute.rs`
- Line 34: `Vault::load(&paths.vault_file).unwrap_or_default()` — acceptable but logs should note vault load failure
- Lines 95-106: `split_whitespace()` for argument parsing may split quoted strings incorrectly
- Line 454: `/bin/bash -c` hardcoded — consider making shell configurable

### `src/hooks.rs`
- Line 334: `let _ = timeout;` — **THE critical approval hook bug**
- Line 228: `unwrap_or_default()` on hook load — silently swallows TOML parse errors

### `src/sandbox.rs`
- Line 268: Fail-open on error — should be fail-closed
- The `glob_match` function is duplicated in both `sandbox.rs` and `hooks.rs` — should be shared

### `src/loop/phases.rs`
- Lines 449-450: `execute_request` + `mark_approval_consumed` are not atomic
- Lines 125-130: Vault/OAuth injection into shell commands happens in execute.rs but is triggered from here

### `src/dashboard/server.rs`
- **No authentication middleware anywhere in the router**
- Line 407: `api_identity_read` returns SOUL.md content to unauthenticated callers
- Line 430: `api_identity_write` has no rate limiting
- Error responses leak internal details

### `src/storage/sqlite/approvals.rs`
- All queries properly parameterized ✅
- `next_approved_request` + `mark_approval_consumed` not in a transaction

### `src/crypto.rs`
- Line 156: `let _ = fs::set_permissions(...)` — silently ignores permission errors (acceptable on non-Unix)

### `src/messaging/telegram.rs`
- Line 201: Bot token in URL — inherent to Telegram API design
- Line 288: `sender_id` defaults to 0 when `from` is None — should this be treated as suspicious?

---

## 📝 Recommended Action Items (Priority Order)

1. **[CRITICAL]** Add authentication to dashboard HTTP server — bearer token or API key
2. **[CRITICAL]** Fix approval hook timeout enforcement in `fire_approval_hooks`
3. **[CRITICAL]** Audit shell-exec command injection surface given dashboard auth gap
4. **[HIGH]** Change sandbox to fail-closed on error
5. **[HIGH]** Make approval consumption atomic (transaction or claimed_at pattern)
6. **[HIGH]** Add per-tool secret scoping for vault/OAuth injection
7. **[HIGH]** Validate hook script paths (absolute, not symlink)
8. **[MEDIUM]** Enable SQLite WAL mode and busy timeout
9. **[MEDIUM]** Verify master key file permissions on load
10. **[MEDIUM]** Add `hooks.toml` to locked path list
11. **[MEDIUM]** Sanitize API error messages
12. **[MEDIUM]** Enforce vault literal warnings at startup

---

## Appendix: Search Patterns Used

```
unwrap() in production paths          → 3 acceptable uses found (all with fallback)
SQL string interpolation              → 0 found (all parameterized)
unsafe blocks                         → 2 found (both in test code only)
Shell command construction            → 1 critical: bash -c with user input
Auth middleware on dashboard          → None found
Token/secret in logs                  → 0 found
Fail-open patterns                    → 1 found (sandbox)
```
