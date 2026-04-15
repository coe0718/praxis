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
