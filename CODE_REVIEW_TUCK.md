# Praxis Code Review — Tuck's Pass

**Reviewer:** Tuck
**Date:** 2026-04-19
**Scope:** Follow-up review after Vex's April 19 security audit. Verified which issues were fixed, identified new issues in frontend + SSE, and proposed new features.

---

## Status of Vex's Original Findings

### ✅ FIXED

| ID | Severity | Issue | Fix |
|----|----------|-------|-----|
| C1 | Critical | Dashboard HTTP server has zero authentication | `require_auth` middleware with `PRAXIS_DASHBOARD_TOKEN` env var |
| C2 | Critical | Approval hook timeout never enforced | `wait_with_timeout()` now used in all approval hook paths |
| H1 | High | Sandbox defaults to fail-open on error | Now returns `SandboxVerdict::Block` on parse/load errors |
| H2 | High | Approval race condition (TOCTOU) | `IMMEDIATE` transaction in `next_approved_request()` |
| M2 | Medium | Master key file permission not verified on load | Checks `metadata.permissions().mode() & 0o777 == 0o600` |
| M6 | Medium | Concurrent SQLite access without WAL mode | `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;` |

### ❌ STILL UNFIXED

| ID | Severity | Issue | Status |
|----|----------|-------|--------|
| C3 | Critical | `shell-exec` command injection via `bash -c` | Dashboard auth mitigates, but tool itself still passes raw strings to bash |
| H3 | High | Vault/OAuth secrets injected into all tools | No per-tool scoping — every shell tool gets all `VAULT_*` env vars |
| H4 | High | Hook scripts executed without path validation | No absolute path check, no symlink check, `hooks.toml` not in locked path list |
| M4 | Medium | Vault literal secrets warning not enforced at startup | `audit_literals()` exists but not called during daemon startup |
| M5 | Medium | Error messages leak internal state in dashboard API | `e.to_string()` still returned to API clients |

---

## 🆕 New Issues Found

### 🔴 N1: SSE Token Leaked in URL Query Parameter

**File:** `frontend/src/hooks/useSSE.ts` (line 15)
**Severity:** HIGH — Token exposure via logs

```typescript
const url = token
  ? `${base}/events?token=${encodeURIComponent(token)}`
  : `${base}/events`
```

The dashboard auth token is passed as a query parameter because `EventSource` doesn't support custom headers. This means the token appears in:
- Server access logs
- Browser history
- Proxy/CDN logs
- TLS inspection tools

**Impact:** Anyone with access to server logs can extract the dashboard token and approve arbitrary tool requests.

**Recommendation:**
- Option A: Switch SSE to WebSocket, which supports auth headers
- Option B: Use a short-lived session cookie set via a one-time `/api/login` endpoint
- Option C: At minimum, rotate the token periodically and document the risk

---

### 🟠 N2: Double SSE Connection

**File:** `frontend/src/components/layout/Layout.tsx` (line 12)
**File:** `frontend/src/pages/Dashboard.tsx` (line 32)
**Severity:** MEDIUM — Resource waste

`Layout` creates `useSSE()` for the header "Live" indicator. `Dashboard` creates `useSSE(30)` for the event feed. Both create separate `EventSource` connections to `/events`. Every dashboard visit opens two SSE connections.

**Recommendation:** Lift the SSE connection to Layout and pass events/connected state down via context or props. One connection, shared state.

---

### 🟡 N3: No Content-Security-Policy Header

**File:** `src/dashboard/server.rs`
**Severity:** MEDIUM — Defense-in-depth missing

The dashboard server doesn't set any `Content-Security-Policy` headers. This means:
- Inline scripts could execute if an attacker finds an XSS vector
- No restrictions on script sources, frame ancestors, or connect origins

**Recommendation:** Add CSP middleware. At minimum:
```
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'
```

---

### 🟡 N4: Frontend Token Stored in Plain localStorage

**File:** `frontend/src/lib/api.ts` (line 2, 5)
**Severity:** LOW — Self-hosted mitigates, but worth noting

```typescript
const getBaseUrl = (): string =>
  localStorage.getItem('praxis_base_url') ?? ''
const getToken = (): string | null =>
  localStorage.getItem('praxis_token')
```

The dashboard token is stored in `localStorage` with no expiry or encryption. A single XSS vulnerability would expose it.

**Recommendation:** Use `sessionStorage` (cleared on tab close) or a secure cookie with `HttpOnly` flag.

---

### 🟡 N5: Approvals Page Payload Parsing Can Crash

**File:** `frontend/src/pages/Approvals.tsx` (line 116)
**Severity:** LOW — UX bug

```tsx
<pre>{JSON.stringify(JSON.parse(a.payload_json), null, 2)}</pre>
```

If `payload_json` is malformed JSON, `JSON.parse()` throws and the entire approvals list breaks. Should wrap in try/catch.

---

### 🟡 N6: No Error Boundary in React App

**File:** `frontend/src/App.tsx`
**Severity:** MEDIUM — One component crash kills the entire app

There's no React error boundary. If any page component throws during render, the entire SPA shows a blank screen. The user has to manually refresh.

**Recommendation:** Wrap the `<Routes>` in an error boundary that shows a fallback UI and a "Reload" button.

---

### 🟡 N7: Header Actions Have No Loading Feedback

**File:** `frontend/src/components/layout/Header.tsx` (lines 17-35)
**Severity:** LOW — UX

`handleWake()` silently catches errors and shows no loading state. `handleRun()` has loading state but no success/error feedback. Users don't know if the action worked.

**Recommendation:** Add toast notifications for success/failure.

---

## 🔧 Remaining Stubs (from NEEDS_FINISHED.md)

| Feature | Status | Notes |
|---------|--------|-------|
| Lite Mode | STUB | No reduced token budgets, no simplified loop |
| Zero-LLM Deterministic Mode | STUB | `StubBackend` returns hardcoded strings, no real rule-based decisions |
| Voice Transcript Streaming | STUB | No Whisper integration, no audio input |
| Serverless/Edge Entry Point | STUB | No Cloudflare Workers or Lambda entry point |

---

## 💡 New Feature Proposals

### 1. Session Timeline View
Visualize the Orient → Decide → Act → Reflect cycle with timestamps, tool calls, and decision points. Currently you only see the outcome — a timeline would show the journey.

**Value:** Debugging agent behavior, understanding why decisions were made, identifying bottlenecks in the loop.

### 2. Approval Queue Search & Sort
The Approvals page has status tabs but no search. With long-running agents, filter by tool name, write path, date range, or requested_by.

**Value:** Faster triage when the agent generates many approval requests.

### 3. Goal Dependency Graph
Goals are a flat list. Show dependencies and blockers visually — which goals depend on which, which are blocked by pending approvals or missing capabilities.

**Value:** Better goal prioritization, understanding why certain goals stall.

### 4. Agent Health Dashboard
Combine memory usage, database size, token spend rate, error rate, and uptime into one view. Forensics has snapshots but no trend chart.

**Value:** Early warning for resource exhaustion, cost tracking.

### 5. Config Diff on Evolution Proposals
When the agent proposes an evolution, show a side-by-side diff of what would change in config/identity. Currently just shows the new text.

**Value:** Faster operator review, fewer rejected proposals due to confusion.

### 6. Keyboard Shortcuts
Power-user hotkeys: `g` for goals, `a` for approvals, `r` for run, `w` for wake, `/` for search, `Esc` to close modals.

**Value:** Faster navigation, less mouse dependency.

### 7. Browser Notifications for Approvals
Push notification or toast when a new approval arrives via SSE. Currently requires watching the sidebar badge.

**Value:** Faster response to approval requests, especially for level 3 tools.

### 8. Session Replay
Not just the outcome — replay the actual decisions, tool calls, and context assembly step by step with timestamps.

**Value:** Deep debugging, training data review, trust building.

### 9. Mobile-Responsive Layout
The sidebar collapses on mobile but the content area doesn't reflow well. Tables overflow, charts are cramped.

**Value:** Monitor the agent from a phone.

### 10. Token Spend Tracking
Real-time token usage per provider with cost estimates. The data exists in `provider_usage` but has no frontend visualization.

**Value:** Cost control, budget alerts, identifying expensive sessions.

---

## 📋 Recommended Action Items (Priority Order)

1. **[HIGH]** Fix SSE token leak — switch to WebSocket or session cookie
2. **[HIGH]** Fix double SSE connection — lift to shared context
3. **[HIGH]** Add per-tool vault secret scoping (H3)
4. **[MEDIUM]** Add hook script path validation (H4)
5. **[MEDIUM]** Add Content-Security-Policy header (N3)
6. **[MEDIUM]** Add React error boundary (N6)
7. **[MEDIUM]** Wrap Approvals payload parsing in try/catch (N5)
8. **[LOW]** Add toast notifications for header actions (N7)
9. **[LOW]** Switch localStorage to sessionStorage for token (N4)
10. **[LOW]** Enforce vault literal warnings at startup (M4)
11. **[LOW]** Sanitize dashboard API error messages (M5)

---

## Appendix: Files Reviewed

```
Backend (src/):
  lib.rs, main.rs, hooks.rs, sandbox.rs, crypto.rs, paths.rs,
  vault.rs, score.rs, evolution.rs, daemon.rs, canary.rs,
  delegation.rs, anomaly.rs, anatomy.rs, heartbeat.rs,
  postmortem.rs, profiles.rs, report.rs, state.rs, time.rs,
  attachments.rs, boundaries.rs, examples.rs, hands.rs

Backend (src/loop/):
  runtime.rs, phases.rs, reflect.rs, planner.rs

Backend (src/dashboard/):
  server.rs, mod.rs

Backend (src/storage/sqlite/):
  mod.rs, approvals.rs, memory*.rs, sessions*.rs

Backend (src/tools/):
  execute.rs, manifest.rs

Backend (src/messaging/):
  telegram.rs

Frontend (frontend/src/):
  App.tsx, main.tsx, index.css
  lib/api.ts, lib/utils.ts
  hooks/useSSE.ts
  contexts/ThemeContext.tsx
  components/layout/Layout.tsx, Sidebar.tsx, Header.tsx
  components/ui/Badge.tsx, Card.tsx, Empty.tsx, Input.tsx, Modal.tsx, Spinner.tsx
  pages/Dashboard.tsx, Chat.tsx, Approvals.tsx, Config.tsx, Sessions.tsx,
  Goals.tsx, Memories.tsx, Identity.tsx, Tools.tsx, Evolution.tsx,
  Score.tsx, Learning.tsx, Canary.tsx, Boundaries.tsx, Delegation.tsx,
  Forensics.tsx, Agents.tsx, Vault.tsx, Argus.tsx

Docs:
  CODE_REVIEW_HERMES.md, NEEDS_FINISHED.md, PRAXIS_DESIGN.md,
  README.md, CLAUDE.md, Cargo.toml, package.json
```
