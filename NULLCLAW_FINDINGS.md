# NullClaw Assessment

**Quick assessment only.** NullClaw (283 Zig files, 268K lines) is a direct Zig port of ZeroClaw's architecture — explicitly stated in AGENTS.md as "full feature parity with ZeroClaw (Rust reference implementation)." Same vtable-driven modular design, same 10 memory engine backends, same 17+ channels, same security subsystems.

**Unique features (not already covered by ZeroClaw analysis):**
- Zig language — 678KB static binary, <2ms startup — but irrelevant since Praxis is Rust
- ClickHouse memory backend — unusual, for analytics workloads
- None of the interaction models are novel vs what Praxis lacks

**Verdict:** No net-new value for Praxis beyond what ZeroClaw's analysis already covered. ZeroClaw's features (SOP engine, Hands, Knowledge Graph, Approval System, Verifiable Intent, WebAuthn, Swarm tool, AIEOS identity, Tunnel providers) remain the relevant reference.

*Clone deleted.*
