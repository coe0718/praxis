# Nanobot Analysis — Features of Interest for Praxis

**Date:** 2026-05-14
**Source:** https://github.com/HKUDS/nanobot (4K lines Python core, ~115K total with tests/WebUI)
**License:** MIT
**Size:** ~400 Python source files across nanobot/ package, plus React/TS WebUI

---

## Overview

Nanobot is a 4,000-line Python AI agent framework styled after OpenClaw/Claude Code. It keeps the core agent loop small while supporting chat channels, memory, MCP, and practical deployment. HKU DS lab project with ~2,000+ GitHub stars.

---

## High-Value Features Praxis Should Consider Absorbing

### 1. Dream — Two-Phase Memory Consolidation (CRITICAL)

`nanobot/agent/memory.py` — This is Nanobot's signature feature. A cron-scheduled cron agent that:

- **Phase 1:** Uses an LLM to analyze history.jsonl entries and produce an analysis summary about what the agent learned, what facts changed, what skills should be created, etc.
- **Phase 2:** Delegates to AgentRunner with `read_file`/`edit_file`/`write_file` tools to make targeted edits to MEMORY.md, SOUL.md, USER.md, and create/update skills.

Key mechanisms:
- **Cursor-based processing** — Tracks a `.dream_cursor` to only process unread history entries.
- **Per-line age annotation** — Uses `git blame` (via dulwich) to annotate MEMORY.md lines with age (e.g. `← 30d`) so the LLM spots stale entries — 14-day threshold.
- **Git-backed versioning** — Auto-commits all changes with the Dream analysis as the commit message. Supports revert rollback.
- **Config:** `2h` default interval, max 20 entries/batch, 15 max iterations.

**Why for Praxis:** Praxis has no equivalent. Dream gives every agent session durable long-term memory that evolves with use. The two-phase pattern (analysis → agent-executed edits) is elegant and avoids replacing entire files. The git-backed versioning and line-age annotation are novel.

---

### 2. Channel Ecosystem (HIGH)

18 platform adapters auto-discovered via pkgutil scan + entry-point plugins:

| Channel | File | Notes |
|---------|------|-------|
| Telegram | `channels/telegram.py` | Inline buttons, long-message split, streaming |
| Discord | `channels/discord.py` | Thread sessions, allow-list, typing indicator |
| Slack | `channels/slack.py` | Thread isolation, mrkdwn, file sends |
| Feishu/Lark | `channels/feishu.py` | Streaming, CardKit, rich text, global domain |
| Matrix | `channels/matrix.py` | Basic messaging |
| WhatsApp | `channels/whatsapp.py` | Media, dedup |
| QQ | `channels/qq.py` | Group chats, media, ACK |
| WeChat (Weixin) | `channels/weixin.py` | Voice, typing, QR, multimodal |
| WeCom | `channels/wecom.py` | WeChat Work |
| DingTalk | `channels/dingtalk.py` | Rich media |
| Email | `channels/email.py` | IMAP/SMTP, attachments |
| WebSocket | `channels/websocket.py` | Cross-channel unified session, multiplexing |
| MS Teams | `channels/msteams.py` | Stale-reference cleanup |

**Why for Praxis:** Praxis has Discord only. These channel implementations are battle-tested (daily releases since Feb 2026). Each has ~300-800 lines of platform-specific logic. Could be adapted as Rust async channel crates.

---

### 3. Message Bus Pattern (MEDIUM)

`nanobot/bus/queue.py` — Simple asyncio.Queue-based pub/sub:

```python
class MessageBus:
    inbound: asyncio.Queue[InboundMessage]
    outbound: asyncio.Queue[OutboundMessage]
```

Events defined as dataclasses:
- `InboundMessage`: channel, sender_id, chat_id, content, timestamp, media[], metadata, session_key_override
- `OutboundMessage`: channel, chat_id, content, reply_to, media[], metadata, buttons[][]

**Why for Praxis:** Praxis's Discord integration is tightly coupled. A message bus decouples channel I/O from agent processing, making it trivial to add channels. Could be a tokio broadcast channel or crossbeam channel in Rust.

---

### 4. MyTool — Runtime Self-Inspection (MEDIUM)

`nanobot/agent/tools/self.py` (475 lines) — A tool the agent can call to inspect/modify its own runtime:

- `check key` — Inspect any runtime attribute via dot-path (model, max_iterations, context_window, scratchpad)
- `set key value` — Modify bounded config values with type/range validation
- **Blocked attributes** — Can't access provider, runner, sessions, credentials, MCP servers
- **Sensitive field masking** — Blocks access to fields containing api_key, secret, password, token, etc.
- **Scratchpad** — Per-session key-value store for ephemeral notes
- **Restricted mutations** — Type-validated, range-bounded: max_tool_result, context_window, model, max_iterations

**Why for Praxis:** Agents that can observe and tune themselves is a powerful pattern. Provides debugging capability without privileged access. The blocked/sensitive attribute protection is a good security model to replicate.

---

### 5. Auto-Compact + Provider Fallbacks (MEDIUM)

`nanobot/agent/autocompact.py` — TTL-based session compaction:
- Detects idle sessions and archives old messages via LLM summarization
- Preserves last 8 messages as recent context
- Ratio-based token budget: compacts until prompt fits within `consolidation_ratio` (default 50%)
- Summary injected as `_last_summary` in session metadata on next access

`nanobot/config/schema.py` — Fallback model chain:
```python
fallback_models: list[str | InlineFallbackConfig]
```
Each fallback can specify provider + model + max_tokens + temperature + reasoning_effort.

**Why for Praxis:** Auto-compact is more sophisticated than simple TTL — it preserves semantic context via summarization. Provider fallbacks add reliability (if Anthropic is down, try OpenRouter).

---

### 6. Git-Backed Memory Store (LOW-MEDIUM)

`nanobot/utils/gitstore.py` (390 lines) — Pure-Python (dulwich) git wrapper:
- Automatic `.gitignore` that only tracks memory files
- Initializes hidden repo in workspace
- Auto-commits on every Dream change
- `line_ages()` — git blame per-line age for annotation
- `log()`, `diff_commits()`, `find_commit()`, `revert()` — full version control
- Handles nested git repo detection (won't initialize inside existing repo)

**Why for Praxis:** Git-backed memory is novel in agent frameworks. Enables undo/rollback of memory changes, auditing agent behavior over time, and per-line staleness detection.

---

### 7. MCP Client Implementation (LOW)

`nanobot/agent/tools/mcp.py` (664 lines) — Full MCP protocol client:
- stdio, SSE, streamable HTTP transports
- Tool name sanitization (`mcp_server_tool` format for provider compatibility)
- Transient error detection + single retry (BrokenResourceError, ConnectionResetError, etc.)
- TCP probe before HTTP connect (avoids exception groups from SSE clients on closed ports)
- Windows path normalization for stdio commands
- Per-server `enabled_tools: list[str]` filter
- Tool timeout config (default 30s)

**Why for Praxis:** If Praxis adds MCP support, this is a well-tested reference implementation with battle-hardened edge case handling.

---

### 8. WebUI (LOW)

`webui/` — React/TypeScript SPA (~40 components):
- Chat interface with streaming
- Multi-session sidebar with WebSocket multiplexing
- i18n locale switcher (LanguageSwitcher component)
- Dark mode, tool traces, code blocks with syntax highlighting
- Settings view
- Image upload and lightbox
- Connection status badge

**Why for Praxis:** Would be nice to have, but low priority — Discord is the primary interface. That said, the WebSocket multiplexing protocol and component architecture are reusable.

---

### 9. Progress Hook System (LOW)

`nanobot/agent/progress_hook.py` (178 lines) — Agent lifecycle events adapted to channel UI:
- `on_stream` — Incremental text delta with reasoning extraction
- `on_stream_end` — Finalize stream
- `before_iteration` — Per-iteration callback
- `before_execute_tools` — Tool hint formatting, progress events
- `after_iteration` — Tool event finish payloads, usage logging
- `emit_reasoning` / `emit_reasoning_end` — Reasoning stream management
- `finalize_content` — strip_think clean up

Uses `IncrementalThinkExtractor` to pull reasoning (think blocks) out of streaming text.

**Why for Praxis:** High-quality UX pattern for showing agent progress in Discord. Separates reasoning, tool calls, and answer text cleanly.

---

### 10. Misc. Notable Features

- **Sandbox shell** (`nanobot/agent/tools/sandbox.py`) — bubblewrap sandbox wrapping for exec tool. Only workspace is RW; config dir is masked with tmpfs; media dir is RO.
- **Notebook editing** (`nanobot/agent/tools/notebook.py`) — Jupyter .ipynb cell replace/insert/delete tool.
- **Cron** (`nanobot/agent/tools/cron.py`) — Full schedule service with natural language agent tool. Protected system jobs (Dream can't be deleted).
- **SSRF protection** — CIDR-based whitelist for web fetch tool.
- **Image generation tool** — Configurable provider support for image gen.
- **Subagent spawning** — Background task execution with concurrency limits and status reporting.
- **Skills system** — Built-in and user-defined skills loaded as system prompts. Dream can auto-create skills from conversation patterns.
- **OpenAI-compatible API** — Serve as an API endpoint, enabling integration from external tools.
- **Prompt templates** — Jinja2-based template system for system prompts.

---

## Architecture for Praxis Consideration

```
Channel (Telegram/Discord/Slack/...) ──→ InboundMessage ──→ MessageBus
                                                              │
                                                         AgentLoop
                                                              │
                                                         AgentRunner
                                                         ┌──┬──┬──┐
                                                         LLM Tools MCP
                                                              │
                                                         memory.py
                                                        (Dream + GitStore)
                                                              │
MessageBus ←── OutboundMessage ←─── Channel (back to user)
```

The bus decoupling is the key architectural insight. Channels publish `InboundMessage` to the bus; AgentLoop consumes from the bus and publishes `OutboundMessage` back. This means adding a channel is just: implement the platform adapter, call `bus.publish_inbound()`, done.

---

## What Nanobot Does NOT Have (Already in Praxis)

- Multi-agent orchestration / team working (Praxis has subagent)
- Kanban / task decomposition
- Curator system
- Scoring/evaluation
- Tool WASM sandbox (Praxis could learn from IronClaw's approach instead)
- Self-evolution/reflection loop
- Plugin/system
- Skills hub/marketplace
- Agent vision tools
- Voice I/O

---

## Recommendation

**Highest ROI for Praxis:** The **Dream memory system** (#1) and **channel ecosystem** (#2). Dream is a genuinely differentiated feature — no other Rust agent has git-backed two-phase memory consolidation. The channel adapters unlock Telegram, Slack, Feishu, etc. with battle-tested production code.

The **message bus pattern** (#3) should be adopted architecturally even if no new channels are added immediately — it makes adding channels trivial when the time comes.

---

*Clone deleted.*
