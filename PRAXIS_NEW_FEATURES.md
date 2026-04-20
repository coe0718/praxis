# Praxis – Untracked Feature Gaps from OpenClaw Ecosystem
*Compiled from clawcharts.com and shelldex.com (2026). Items already implemented or tracked in PRAXIS_DESIGN.md have been removed.*

| Project (Source) | Gap | Why it matters |
|------------------|-----|----------------|
| **Moltworker** (shelldex) | Serverless Cloudflare-Workers deployment — zero-ops hosting with no persistent process | Lets operators run Praxis on demand without a VPS or Docker; cold-start on webhook triggers. |
| **OpenFang** (clawcharts) | Native MCP (Model Context Protocol) support — tool registration and context sharing via the emerging open standard | Interop with the growing MCP ecosystem; agents could expose or consume MCP-native tools without custom adapters. |
| **SafeClaw** (shelldex) | Zero-LLM deterministic mode — rule-based fallback path that skips all LLM calls | Offline operation, cost-free maintenance passes, and a testable baseline with no token spend. |
| **Gitclaw** (shelldex) | Agent-as-repo — all state (SQLite, markdown files) committed to git automatically | Immutable audit trail, easy CI/CD integration, and trivial rollback of any state change. |
| **AstrBot / LangBot** (shelldex) | Discord and Slack adapters alongside Telegram | Reaches operators on their primary platforms; Praxis currently ships only Telegram. |
| **ClawGo** (shelldex) | Voice transcript streaming — real-time speech-to-text piped into the agent loop | Hands-free interaction and Raspberry Pi voice interface without a separate transcription service. |

## Why the others were removed

| Removed | Reason |
|---------|--------|
| ZeroClaw, NullClaw (Zig), NanoClaw (Python), PicoClaw, MimicLaw/ZClaw (C/ESP-32), GoGogot (Go), BashoBot (Bash) | Alternative-language implementations; Praxis is Rust-native. Lite mode and Cargo feature modularity already track the size/edge angle. |
| IronClaw, Secure-OpenClaw | Already tracked as "Security fuzzing" and "Plugin / extension architecture" in PRAXIS_DESIGN.md. |
| Carapace (WASM plugins) | Already tracked as "WASM tool runtime" in PRAXIS_DESIGN.md. |
| Hermes Agent | **Implemented.** `SkillSynthesizer` already auto-drafts `SKILL.md` files after complex successful sessions. |
| memU, MemOS | **Implemented.** Hot/cold/linked memory with FTS5, decay, and reinforcement is the current memory architecture. |
| OpenBrowserClaw | Browser-only, no persistent state — not compatible with Praxis's architecture. |
| ClawWork | Revenue/marketplace focus — out of scope for a personal agent framework. |
| Moltis (zero-unsafe Rust) | Praxis already uses safe Rust throughout. Decision receipts + forensics snapshots cover the audit-trail angle. |
| ApexClaw (94 built-in tools) | Tool quantity; "Community tool registry improvements" in PRAXIS_DESIGN.md covers discoverability. |
| AionUi | Multi-provider graphical dashboard — the web dashboard and SSE streaming in PRAXIS_DESIGN.md cover this direction. |

*Sources: ClawCharts (clawcharts.com) and Shelldex "OpenClaw Alternatives & Forks Compared (2026)" (shelldex.com).*

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
