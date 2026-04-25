# Phase 2A Code Review — Vex

**Reviewer:** Vex  
**Date:** 2026-04-23  
**Scope:** Session Timeline + Approval Search (Phase 2A)  
**Verdict:** Build clean, 212 tests pass. **2 CRITICAL, 4 WARNING, 5 SUGGESTION**

---

## CRITICAL

### C1 — `search_approvals`: Dynamic SQL with hardcoded parameter indexes breaks when conditions are missing
**File:** `src/storage/sqlite/approvals.rs:50-111`  
**Severity:** CRITICAL

The query builds `WHERE` clauses with hardcoded `?1`, `?2`, `?3` placeholders, then uses an 8-branch `if/else` to decide which params to bind. The SQL string always contains `?1` when `q` is present, `?2` when `tool` is present, and `?3` when `status` is present — but the **actual bound parameters shift position** depending on which branches execute.

Wait — re-reading: the SQL placeholders are always `?1`, `?2`, `?3` and the branches bind them in the right combos. **BUT**: when only `status` is provided (no `q`, no `tool`), the SQL has `WHERE status = ?3` — but rusqlite's `query_map` with `[p3]` binds `p3` as `?1`, not `?3`. **The placeholder numbering in the SQL doesn't match the positional binding.**

Example failure path:
- User passes only `?status=pending`
- SQL: `WHERE status = ?3`
- Bind: `[p3]` → rusqlite binds as `?1`
- Result: **runtime error** — `?3` has no value, or wrong value bound to `?1`

**Fix:** Either:
1. Use `rusqlite::params_from_iter` with a `Vec<Box<dyn ToSql>>` built in the same order as conditions, OR
2. Use named parameters (`:q`, `:tool`, `:status`) instead of positional, OR
3. Rewrite to use a single `query_map` call with a `Vec<Box<dyn ToSql>>` and dynamically numbered `?` placeholders that match the vec order.

### C2 — `search_approvals`: Unbounded query — no LIMIT clause
**File:** `src/storage/sqlite/approvals.rs:66-70`  
**Severity:** CRITICAL

`search_approvals` has no `LIMIT`. On a system with thousands of approval records, a broad search (or no filters) returns the entire table. This is a DoS vector — a single empty-query request serializes every row.

The existing `list_approvals` also lacks LIMIT, so this is a pre-existing issue, but `search_approvals` is now exposed directly to frontend user input via query params.

**Fix:** Add `ORDER BY id DESC LIMIT 500` (or accept a `limit` param).

---

## WARNING

### W1 — SQL injection via LIKE wildcards in search query
**File:** `src/storage/sqlite/approvals.rs:78`  
**Severity:** WARNING

```rust
let q_pattern = q.map(|s| format!("%{}%", s));
```

User input `s` is interpolated directly into the LIKE pattern. If a user searches for `%` or `_`, these are LIKE metacharacters and will match unexpectedly. This is **not** a classical SQL injection (it's parameterized), but it's a LIKE injection that allows wildcard abuse.

**Fix:** Escape LIKE metacharacters:
```rust
let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\");
let q_pattern = format!("%{}%", escaped);
```

### W2 — `.unwrap_or(json!({}))` on malformed JSON in production path
**File:** `src/dashboard/helpers.rs:22-23`  
**Severity:** WARNING

```rust
let phase_durations: Value =
    serde_json::from_str(&phase_durations_raw).unwrap_or(json!({}));
```

If `phase_durations` column contains malformed JSON, this silently swallows the error and returns `{}`. For a dashboard displaying timing data, silent data loss is worse than an error — the user sees "no data" and has no indication that parsing failed.

**Fix:** Log a warning on parse failure:
```rust
let phase_durations: Value = serde_json::from_str(&phase_durations_raw)
    .unwrap_or_else(|e| {
        log::warn!("Invalid phase_durations JSON for session: {e}");
        json!({})
    });
```

### W3 — Tool filter dropdown derived from filtered results, not all tools
**File:** `frontend/src/pages/Approvals.tsx:171-172`  
**Severity:** WARNING

```tsx
const uniqueTools = Array.from(new Set(approvals.map((a) => a.tool_name))).sort()
```

The tool dropdown is populated from the **current filtered result set**. When a status filter is active (e.g., "pending"), the dropdown only shows tools that have pending approvals. If you select a tool and it filters down to 0 results, that tool disappears from the dropdown — the user can't select it again.

Worse, the tool filter and status filter interact in a confusing way: selecting a tool may remove other tools from the dropdown, making it impossible to switch to them without clearing all filters first.

**Fix:** Fetch a separate unfiltered list for the tool dropdown, or compute `uniqueTools` from a dedicated `useQuery` with no filters.

### W4 — `statusParam` silently ignores `claiming` status
**File:** `frontend/src/pages/Approvals.tsx:159`, `src/storage/mod.rs:216-234`  
**Severity:** WARNING

The frontend status tabs are `['all', 'pending', 'approved', 'rejected', 'executed']`. The backend `ApprovalStatus` enum also has `Claiming`. If any approval has `status=claiming`, it will never appear when filtering by status (it's not a tab option), and when `filter='all'`, it's included but the user has no way to isolate claiming records.

This may be intentional, but it's worth documenting — claiming is a transient state that probably needs UI visibility for debugging.

---

## SUGGESTION

### S1 — SessionTimeline doesn't handle `sleep` phase in the bar chart
**File:** `frontend/src/components/SessionTimeline.tsx:41`  
**Severity:** SUGGESTION

`PHASE_ORDER` only includes `['orient', 'decide', 'act', 'reflect']`. The `PHASE_COLORS` map includes `sleep`, but it's never rendered in the bar. If the backend ever sends `sleep` durations, they'll appear in the legend (lines 56-66 iterate over `PHASE_ORDER`) but NOT in the bar.

**Fix:** Either add `sleep` to `PHASE_ORDER` if it's a valid phase, or remove it from `PHASE_COLORS`/`PHASE_LABELS`.

### S2 — Floating point precision in percentage widths
**File:** `frontend/src/components/SessionTimeline.tsx:43`  
**Severity:** SUGGESTION

```tsx
const pct = total > 0 ? (value / total) * 100 : 0
```

Tiny durations (e.g., 0.001s) may round to 0% and be skipped, while still showing in the legend as "0.0s". Consider a minimum width (e.g., `Math.max(pct, 1)`) for non-zero phases to make them visible.

### S3 — Debounce implementation is correct but could use `useDeferredValue` or `useMemo`
**File:** `frontend/src/pages/Approvals.tsx:155-157`  
**Severity:** SUGGESTION

The manual debounce is fine, but React 18+ has `useDeferredValue` which is designed for exactly this pattern and integrates better with concurrent rendering.

### S4 — Layout.tsx fetches all approvals every 30s just for a badge count
**File:** `frontend/src/components/layout/Layout.tsx:16`  
**Severity:** SUGGESTION

The Layout sidebar fetches ALL approvals every 30 seconds just to count pending ones. Now that `fetchApprovals` supports server-side filtering, this should be:
```tsx
queryFn: () => fetchApprovals({ status: 'pending' }),
```
This would reduce payload size significantly.

### S5 — Dashboard.tsx fetches `fetchTokenSessions` without using most of the data
**File:** `frontend/src/pages/Dashboard.tsx:73-76`  
**Severity:** SUGGESTION

`fetchTokenSessions` fetches all session token usage, but the dashboard only renders a bar chart. Consider adding a `limit` param to avoid fetching hundreds of rows.

---

## GOOD CODE

- **SQL parameterization**: All queries use `?` placeholders — no string interpolation into SQL. ✅
- **Type safety**: `ApprovalQuery` struct with `Deserialize` for query params — proper Axum pattern. ✅
- **Debounced search**: Clean `useEffect` + `setTimeout` pattern with proper cleanup. ✅
- **SessionTimeline**: Graceful null handling, clean prop interface, zero external dependencies. ✅
- **Backend/frontend API contract**: `phase_durations` typed as `Record<string, number>` matches the JSON structure. ✅
- **Backward compatible**: `fetchApprovals` accepts optional params, existing call sites updated to `() => fetchApprovals()`. ✅

---

**Bottom line:** C1 is a real runtime bug that will cause errors when filtering by only status or only tool. C2 is an unbounded query exposed to user input. Fix those two and this ships clean.
