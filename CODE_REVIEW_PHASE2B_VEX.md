# Phase 2B Code Review — Vex

**Reviewer:** Vex  
**Date:** 2026-04-23  
**Scope:** Token Spend Tracking + Health Dashboard  
**Verdict:** Build clean, 212 tests pass. **0 CRITICAL, 3 WARNING, 4 SUGGESTION**

---

## CRITICAL

None. Clean implementation.

---

## WARNING

### W1 — Health endpoint fetches 10,000 hot + 10,000 cold memories just to count them
**File:** `src/dashboard/routes_core.rs:286-295`  
**Severity:** WARNING

```rust
let hot_count = store.recent_hot_memories(10_000).map(|v| v.len()).unwrap_or(0);
let cold_count = store.strongest_cold_memories(10_000).map(|v| v.len()).unwrap_or(0);
```

This deserializes up to 20,000 full `StoredMemory` structs (with content, tags, summaries) from SQLite, allocates `Vec`s for them, then throws everything away except `.len()`. On a health endpoint that polls every 30s from the dashboard, this is unnecessary allocation pressure and I/O.

**Fix:** Use `SELECT COUNT(*)` queries instead:
```rust
// Add count_hot_memories() and count_cold_memories() to the store
let hot_count = store.count_hot_memories().unwrap_or(0);
let cold_count = store.count_cold_memories().unwrap_or(0);
```
One-liner SQL each, zero allocation.

### W2 — `hb_status == "unknown"` doesn't affect overall status
**File:** `src/dashboard/routes_core.rs:270-277`  
**Severity:** WARNING

When `hb_age` is -1 (heartbeat file missing or unparseable), `hb_status` is set to `"unknown"`. But the overall status downgrade logic only checks for `"error"` and `"warn"`:

```rust
if hb_status == "error" {
    overall = "error";
} else if hb_status == "warn" && overall == "ok" {
    overall = "warn";
}
```

So a missing heartbeat silently shows `"unknown"` on the heartbeat check while overall remains `"ok"`. The green health indicator in the dashboard is misleading if the agent is actually dead.

**Fix:** Either downgrade overall to `"warn"` when heartbeat is unknown, or add explicit handling:
```rust
} else if hb_status == "unknown" && overall == "ok" {
    overall = "warn";
}
```

### W3 — Health endpoint hits the database 4 times on every call
**File:** `src/dashboard/routes_core.rs:229-300`  
**Severity:** WARNING

The `/api/health` endpoint opens a new `SqliteSessionStore` (which creates a new `Connection`) then calls:
1. `validate_schema()` — 1 query
2. `list_approvals(Some(Pending))` — 1 query (returns full rows, not COUNT)
3. `recent_hot_memories(10_000)` — 1 query (deserializes up to 10k rows)
4. `strongest_cold_memories(10_000)` — 1 query (deserializes up to 10k rows)

That's 4 separate connections to SQLite with 2 of them doing full deserialization. Combined with W1, this is heavy for a 30-second poll endpoint.

**Fix:** 
1. Use a single connection for all checks.
2. Replace `list_approvals(Some(Pending))` with a `COUNT(*)` query.
3. Replace `recent_hot_memories`/`strongest_cold_memories` with `COUNT(*)` queries (per W1).

---

## SUGGESTION

### S1 — `hb_age` is `i64` but serialized as `-1` in JSON for unknown status
**File:** `src/dashboard/routes_core.rs:278`  
**Severity:** SUGGESTION

The JSON output is `"age_seconds": -1` when heartbeat is unknown. The frontend `HealthCheck` interface types this as `age_seconds?: number` which is fine, but `-1` is a magic number. Consider `null` or omitting the field when unknown — it's cleaner semantically.

### S2 — Token bar chart renders `#123` labels — gets noisy with many sessions
**File:** `frontend/src/pages/Dashboard.tsx:94-100`  
**Severity:** SUGGESTION

```tsx
session: `#${r.session_id}`,
```

With 50 sessions, the X-axis becomes a wall of `#1`, `#2`, ..., `#50` labels. Consider showing every 5th or 10th label, or using `Day {r.day}` for better context.

### S3 — `cost` field computed in `tokenChartData` but never used
**File:** `frontend/src/pages/Dashboard.tsx:98`  
**Severity:** SUGGESTION

```tsx
cost: r.estimated_cost_micros / 1_000_000,
```

The `cost` field is computed for every session but the `BarChart` only renders `tokens`. Dead computation. Either add a second bar (dual-axis chart) or remove it.

### S4 — `api_tokens` silently returns zeros on DB error
**File:** `src/dashboard/routes_core.rs:181-186`  
**Severity:** SUGGESTION

```rust
let summary = store
    .token_summary_all_time()
    .unwrap_or(TokenSummaryAllTime { total_tokens: 0, total_cost_micros: 0, total_sessions: 0 });
```

If the DB query fails (corrupt, locked, missing table), the endpoint returns `200 OK` with all zeros. The dashboard shows "Total Tokens: 0" with no indication that something is wrong. Consider logging the error at minimum:

```rust
.unwrap_or_else(|e| {
    log::warn!("token summary query failed: {e:#}");
    TokenSummaryAllTime { total_tokens: 0, total_cost_micros: 0, total_sessions: 0 }
})
```

---

## GOOD CODE

- **SQL queries are well-written.** All parameterized, proper `COALESCE` for null safety, `LEFT JOIN` for session token aggregation. ✅
- **Type definitions are clean.** `TokenSummaryAllTime`, `SessionTokenUsage`, `ProviderTokenSummary` — simple, no unnecessary derives. ✅
- **Trait extension is clean.** `ProviderUsageStore` gets 3 new methods, `SqliteSessionStore` implements them by delegating to module functions. ✅
- **Health check logic is sound.** The tiered heartbeat status (ok <5min, warn <15min, error) with proper overall downgrade is a good pattern. ✅
- **Frontend type safety.** `HealthCheck`, `HealthReport`, `TokenSummary` all properly typed. `URLSearchParams` for query building. ✅
- **No security issues.** No user input reaches SQL unsanitized. Token/cost data is read-only from the dashboard. ✅
- **Graceful degradation.** Every store call in health has `.unwrap_or()` fallback — the endpoint always returns valid JSON. ✅

---

## Summary

No criticals. The main theme is **efficiency** — the health endpoint does too much work per request (W1/W3), and the "unknown" heartbeat status should degrade the overall indicator (W2). Fix those three and this is solid.

Compared to Phase 2A (which had a runtime-breaking parameter binding bug), this phase is significantly cleaner. The SQL is correct, the types are right, and the architecture follows established patterns.

— Vex
