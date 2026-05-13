---
name: rust-auth-security-review
description: Manual security-focused code review for Rust web auth systems (token generation, storage, verification, middleware, scope enforcement).
version: 0.1.0
---

# Rust Auth Security Review

Manual security-focused code review methodology for Rust web services (axum/actix) with token-based authentication.

## When to Use
- Reviewing new auth middleware, token generation, or session management code
- Auditing service-token or API-key systems
- Checking a monorepo rollout of shared auth across multiple products

## Tooling Notes
- `/mnt/docker/code/` paths: Hermes `terminal` tool returns empty output. Use `execute_code` with `subprocess.run()` directly.
- `delegate_task` subagents cannot access `/mnt/docker/` filesystem. Do reviews in the main agent.
- Read large files in chunks via `subprocess.run(["cat", path], capture_output=True, text=True)` then slice the output.

## Review Checklist

### Token Generation
- [ ] Tokens generated from cryptographically secure RNG (UUID v4 is fine, `getrandom` backed)
- [ ] Token format includes a product-specific prefix for visual identification
- [ ] Tokens are hashed (SHA256 minimum) before storage — never stored raw in the consuming product

### Token Storage
- [ ] Hashes stored in `.env` or config files, not hardcoded
- [ ] **Check the orchestration layer**: Control planes may store raw tokens in SQLite to forward as headers. This is the biggest risk surface — flag for at-rest encryption.
- [ ] Fingerprint/display-only identifiers should use partial hash, not the full value

### Token Verification
- [ ] Constant-time comparison (fold XOR pattern, NOT `==` on strings/bytes)
- [ ] Empty stored hash returns false (rejects unconfigured auth)
- [ ] Token expiration is enforced, not just stored

### Middleware / Route Protection
- [ ] Public paths explicitly whitelisted
- [ ] Auth middleware applies to ALL routes unless in public list
- [ ] Bootstrap/generate-key endpoints restricted to localhost by default
- [ ] Remote bootstrap requires explicit opt-in env var
- [ ] Error messages don't leak whether auth is configured vs just wrong token

### Scope / Authorization
- [ ] Service tokens have defined scopes (e.g., `runs:read`, `actions:dispatch`)
- [ ] Scope enforcement happens per-route, not just at middleware level
- [ ] Legacy/unscoped tokens have restricted access until rotated

### Monorepo Rollout Consistency
- [ ] All products use the same shared auth crate — no copy-paste drift
- [ ] Each product has unique env var names and token prefixes
- [ ] Dispatch paths are product-specific and correct
- [ ] Template/starter is updated to match

### Frontend
- [ ] Token inputs use `type="password"`
- [ ] No `dangerouslySetInnerHTML` or raw HTML injection near token display
- [ ] Provision flow clears operator API key from state after use

## Common Finding Patterns
| Severity | Pattern |
|----------|---------|
| CRITICAL | Raw tokens stored in orchestrator DB without encryption |
| HIGH | Non-constant-time comparison of secrets |
| HIGH | Token expiration field exists but isn't enforced |
| MEDIUM | Bootstrap endpoints accessible from non-localhost without opt-in |
| MEDIUM | Scope checks bypassed for legacy/unscoped tokens |
| LOW | Short fingerprints (collision risk at scale) |
| LOW | UUID v4 instead of raw CSPRNG bytes (cosmetic) |
