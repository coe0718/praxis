# Memory Search Code Review — Vex

**Reviewer:** Vex  
**Date:** 2026-04-23  
**Scope:** Memory Search feature  
**Verdict:** Clean. **0 CRITICAL, 1 WARNING, 3 SUGGESTION**

---

## CRITICAL

None.

---

## WARNING

### W1 — `to_fts_query("---")` returns `Some("")` which passes the guard but hits FTS5 with an empty query
**File:** `src/memory/query.rs:20-28`  
**Severity:** WARNING

```rust
pub fn to_fts_query(query: &str) -> Option<String> {
 let tokens = tokenize(query);
 if tokens.is_empty() {
  None
 } else {
  Some(tokens
   .into_iter()
   .map(|token| format!("\"{token}\""))
   .join(" "))
 }
}
```

`to_fts_query` returns `None` only when `tokens` is empty. If `query` is all punctuation (e.g., `"---"`), `tokenize` splits on non-alphanumeric — but the split segments are empty strings. `filter(|token| !token.is_empty())` removes them, leaving `tokens` empty, so `to_fts_query` returns `None` and `search_memories` returns `Vec::new()` early. **This path is actually fine.**

BUT: `tokenize` strips non-alphanumeric characters from valid tokens before wrapping them in double quotes for phrase matching. This means punctuation-based queries fail gracefully, which is good. No actual injection risk here.

What IS worth noting: the `.take(12)` token limit in `tokenize` silently truncates long queries. A 20-word search phrase becomes a 12-word phrase. For a memory search this is usually fine, but it's not obvious to the user that their query was truncated.

---

## SUGGESTION

### S1 — Hot/cold `search_*` functions each have independent `limit` — up to 100 rows fetched for a `limit=50` search
**File:** `src/storage/sqlite/memory.rs:161-174`  
**Severity:** SUGGESTION

```rust
let hot = search_hot_memories(&connection, &query, limit)?;
let cold = search_cold_memories(&connection, &query, limit)?;
let mut combined = hot.into_iter().chain(cold).collect::<Vec<_>>();
combined.sort_by(|left, right| right.score.total_cmp(&left.score));
combined.truncate(limit);
```

Each branch fetches `limit` results independently. If both tiers have relevant matches, you fetch 100 rows, sort 100, then truncate to 50. This is fine for `limit=50`, but wasteful if `limit` grows. Consider fetching `limit / 2` from each branch to avoid over-fetching.

### S2 — `Memory.score` field conflates two different signals for display
**Files:** `src/storage/sqlite/memory.rs:324, 361`  
**Severity:** SUGGESTION

The `score` field displayed in the UI as `score.toFixed(3)` is actually `importance + relevance_score(rank)` for hot memories, and `weight + relevance_score(rank)` for cold. The user sees a single number that mixes the memory's intrinsic quality with how well it matched the search query.

When NOT searching (hot/cold tabs), `score` is purely `importance` or `weight`. When searching, it becomes `importance + relevance`. A memory with low importance but high BM25 match can score the same as a high-importance memory with a weak match.

Consider either: (a) returning a separate `relevance` field alongside `score`, or (b) having the search path return raw `importance`/`weight` and letting the FTS `rank` be a separate field.

### S3 — `searchMemories` in api.ts uses template literal instead of `URLSearchParams`
**File:** `frontend/src/lib/api.ts`  
**Severity:** SUGGESTION

```typescript
export const searchMemories = (q: string): Promise<Memory[]> =>
 request(`/api/memories/search?q=${encodeURIComponent(q)}`)
```

This is fine and `encodeURIComponent` is correct. But `fetchApprovals` in the same file was updated to use `URLSearchParams` for consistency. Consider being consistent here too — either both use `URLSearchParams` or both use template literals.

---

## GOOD CODE

- **FTS5 is used correctly.** Phrase matching via `"token"` wrapping is proper. Punctuation is stripped before reaching FTS5 — no injection risk. ✅
- **`to_fts_query` handles empty/invalid input gracefully.** Returns `None` for whitespace-only input, which causes `search_memories` to return `Vec::new()`. ✅
- **Search query is validated at the route layer.** `query.q.trim().is_empty()` returns `400 BAD_REQUEST`. ✅
- **Debounce is correctly implemented.** 300ms in both `Memories.tsx` and `Approvals.tsx` with proper cleanup. ✅
- **Frontend hides tabs during search** — correct UX, search crosses both tiers. ✅
- **`enabled` on `useQuery` prevents unnecessary fetches.** Hot/cold queries skip when `isSearching`. ✅
- **`encodeURIComponent` is used correctly** for the query param. ✅
- **Score compositing is mostly sound.** `importance + relevance_score(rank)` keeps BM25 as the primary signal (since `relevance_score` is bounded 0–1 and `importance` is 0–1, BM25 rank dominates when rank > 1). ✅
- **No N+1.** Single JOIN query per tier, no loops over results. ✅
- **No hardcoded secrets or SQL injection.** ✅

---

**Bottom line:** This is a solid implementation. The FTS5 usage is correct, the routing layer validates input, the frontend debounces properly and hides tabs during cross-tier search. The only meaningful concern is the token truncation on long queries (W1), but it's minor. Ship it.

— Vex