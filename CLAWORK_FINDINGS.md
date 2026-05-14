# ClawWork Feature Analysis — Features Praxis Might Not Have

**Source**: https://github.com/clawwork-ai/ClawWork (cloned & explored)
**Date**: 2026-05-14
**Praxis comparison**: Praxis is a Rust-based agent runtime (uses `cargo`, has `PraxisRuntime`, `SqliteSessionStore`, `ToolManifest`). ClawWork is an Electron/React/TypeScript desktop client for OpenClaw Gateway.

---

## 1. Multi-Client Architecture: Desktop App + PWA + Website

ClawWork ships **three separate client surfaces** from one monorepo:

| Package | Purpose |
|---------|---------|
| `packages/desktop/` | Full Electron 34 desktop app (React 19, Zustand, SQLite/Drizzle) |
| `packages/pwa/` | Progressive Web App at `cpwa.pages.dev` — mobile-friendly, installable to home screen, offline-capable |
| `packages/shared/` | Shared protocol types, constants, gateway protocol definitions (zero-dependency bridge) |
| `packages/core/` | Shared business logic used by both desktop and PWA (stores, services, ports) |

**Key insight**: ClawWork factorizes business logic into `packages/core/` so both Electron and PWA share the same stores, session sync, gateway dispatcher, and protocol handling. The PWA has its own platform adapters for IndexedDB persistence, browser WebSocket, and browser notifications.

**Relevance to Praxis**: If Praxis is a CLI-only or single-surface project, the multi-client architecture patterns in ClawWork may be instructive.

---

## 2. Structured Desktop GUI with Three-Panel Layout

ClawWork is a **full graphical desktop application** with:

- **Left Nav** (260px): Task list, search, file browser entry, settings
- **Main Area** (flex): Chat conversation, file browser, cron panel, team management
- **Right Panel** (320px, collapsible): Progress tracker + Artifacts list

This is fundamentally different from a CLI or terminal-based agent runtime. It includes:
- Real-time streaming message rendering (delta accumulation + blinking cursor)
- Inline collapsible tool-call cards with status/progress
- Markdown rendering with syntax highlighting (react-markdown + rehype-highlight)
- Framer Motion animations throughout
- shadcn/ui component system (Radix UI + Tailwind CSS v4)
- Dark/light theme with 8 languages (i18n)
- Design token system (`theme.css` CSS variables + `design-tokens.ts`)

**Likely absent in Praxis** unless Praxis also has a GUI.

---

## 3. Scheduled (Cron) Tasks with Full UI

A first-class cron scheduling system with 7 full RPC methods (`cron.list`, `cron.add`, `cron.update`, `cron.remove`, `cron.run`, `cron.runs`, `cron.status`):

- **Schedule types**: `at` (one-shot ISO), `every` (interval ms), `cron` (expr + timezone + stagger)
- **Payload types**: `systemEvent` (text notification) or `agentTurn` (message + model + thinking + timeout)
- **Session targets**: `main`, `isolated`, `current`, `session:<specific-key>`
- **Delivery**: `none`, `announce` (to channel), `webhook` (HTTP POST)
- **Failure alerts**: Exponential backoff, consecutive error tracking, alert destinations
- **Run history**: Paginated log with per-run status, usage, duration, delivery status
- **UI**: Dedicated CronPanel with job cards, create/edit dialog (simple + advanced mode), run history viewer, scheduler status indicator, pagination
- **Simple mode** (90% use cases): name + cron expression + agent message + session target
- **Advanced mode**: timezone, description, delivery, model override, thinking mode, failure alerts, deleteAfterRun

**Highly likely absent in Praxis** — this requires both a cron scheduling engine and a management UI.

---

## 4. Teams System (Multi-Agent Orchestration)

ClawWork supports **composable multi-agent teams** with a coordinator/worker pattern:

- **Team structure**: `TEAM.md` metadata + individual agent directories with `IDENTITY.md`, `SOUL.md`, `skills.json`
- **Coordinator agent**: Breaks down tasks and delegates to worker agents
- **Worker agents**: Each runs in its own sub-session
- **Real-time orchestration**: UI shows the full delegation tree and per-agent progress
- **Three ways to create teams**:
  1. **TeamsHub** — Git-native registry of community-contributed teams (browse, discover, install)
  2. **Manual wizard** — Step-by-step UI to define agents, roles, identities, and skills
  3. **AI Builder** — Describe what you need; an LLM designs the team structure, roles, and prompts
- **Task rooms**: `roomStore` tracks ensemble task sessions with conductor/performer orchestration
- **EnsembleAgentBar**: UI component showing active agents in a multi-agent task with tooltips

**Checklist of sub-features**:
- `TEAM.md` file format with YAML frontmatter (parsed by `team-parser.ts`)
- Agent-level skill dependencies (`skills.json` references skills by slug)
- `team-installer.ts` with async generator for progress reporting (creates agents, sets files, installs skills, persists team metadata, rollback on failure)
- TeamsHub registry management (multiple registries, refresh, search, category filtering)
- Team card UI with install state indicators

**Likely absent in Praxis** — this is a full multi-agent orchestration + registry + UI system.

---

## 5. Gateway Pairing Protocol (QR Code / Device Auth)

ClawWork implements a sophisticated **device pairing protocol** for OpenClaw Gateway authentication:

- **Device identity**: Ed25519 keypair generated per device, stored in `device-identity.json`
- **Fingerprint**: SHA-256 of the raw Ed25519 public key serves as device ID
- **Challenge-response auth**: Gateway sends `connect.challenge` with nonce; client signs the nonce + metadata with device key
- **QR code pairing**: PWA scans a QR code containing gateway URL, auth token/password/pairingCode, and scope ID
- **Auth modes**: token, password, pairing code, bootstrap token
- **Scopes**: `operator.admin`, `operator.write`, `operator.read`, `operator.approvals`, `operator.pairing`
- **Device token exchange**: On first successful connect, server issues a device token for future reconnects (persisted locally)

The PWA's `PairingView.tsx` provides a full mobile-friendly scanning experience with camera access, QR decode, and automatic gateway configuration.

**Likely absent in Praxis** unless it also has a gateway-centric architecture.

---

## 6. Desktop Operating System Integration

Electron-specific deep OS integration:

- **System tray**: Background process, tray icon, context menu
- **Quick-launch window**: Global shortcut (`Alt+Space`, customizable), minimal UI for quick task creation
- **Auto-updater**: Background download, install-on-quit, progress in Settings
- **Workspace directory**: Configurable user-chosen workspace root with `.clawwork.db` (SQLite) + per-task artifact directories
- **Keyboard shortcuts**: Throughout the app
- **Zoom control**: Remembers preference
- **Native notifications**: Task completions, approval requests, gateway disconnects — per-event toggles
- **macOS entitlements**: Code signing, notarization, Universal Binary

---

## 7. Local-First Artifact Management with Full-Text Search

- **Workspace structure**: `<workspace>/tasks/<task-name>/artifacts/...` on filesystem
- **SQLite as index only**: File blobs never go in DB; SQLite holds metadata + FTS5 full-text index
- **Auto-extraction**: Code blocks, images, and remote files in assistant replies auto-extract to local workspace
- **Context folder watcher**: Watch up to 10 directories (4 levels deep, files up to 10 MB), auto-re-index on changes
- **File browser UI**: Grid view with type filter tabs (All/Docs/Code/Images), search, reverse-chronological sort, click-to-navigate-back-to-originating-task
- **Full-text search**: Search across tasks, messages, and artifacts simultaneously (FTS5)
- **SSRF guard**: `net/ssrf-guard.ts` prevents request forgery in file fetching
- **Safe-fetch**: `net/safe-fetch.ts` with timeout and size limits for remote content

---

## 8. Voice Input (Local Whisper.cpp)

- Hold-Space-to-dictate via local `whisper.cpp` sidecar
- **No cloud API**: All transcription runs locally
- **Model search paths**: Multiple directories searched automatically
- **Timer-based**: Short press = space character; hold = record; release = transcribe
- **Cursor insertion**: Transcription goes to cursor position, never sent automatically

---

## 9. Approval Workflow for Sensitive Actions

- Gateway emits `exec.approval.requested` events
- Client shows a modal `ApprovalDialog` with details of the requested action
- User approves/rejects via UI
- Result sent back via `exec.approval.resolve` RPC
- Per-event notification toggles control which approvals notify

This is a design pattern: treat approval as a first-class event in the gateway protocol, not a hidden feature.

---

## 10. Message Persistence State Machine

A well-documented, proven message state machine:

```
idle → streaming → pending → canonical (with retry loop)
```

- Three disjoint storage buckets guarantee no duplicates
- Single-writer invariant: sync path is the sole DB writer for assistant messages
- Unique DB index `(task_id, role, timestamp)` as idempotency guard
- Past bug patterns documented (dual-write bugs, race conditions) with fixes
- Startup sync, runtime sync, reconnect sync paths all documented

This level of architectural documentation for a specific hard problem is valuable.

---

## 11. Debugging & Observability Infrastructure

- **Ring buffer**: In-memory ring buffer of debug events
- **NDJSON logs**: `.clawwork-debug/` directory with daily NDJSON log files
- **Export bundle**: Debug bundle (logs + gateway status + sanitized config) exportable via UI — designed as the primary support artifact
- **Structured debug events**: `domain.noun.verb` naming convention
- **Rate-limited debug handlers**: Prevent debug event floods
- **Architecture checks**: Automated `pnpm check:architecture.mjs` enforces layer boundaries
- **UI contract checks**: Automated `pnpm check:ui-contract.mjs` enforces design system adherence

---

## 12. Diverse IPC Surface (Electron-specific Architecture Pattern)

The Electron main process exposes a wide range of IPC handlers to the renderer:

| Handler File | Purpose |
|---|---|
| `artifact-handlers.ts` | File save, extract, search |
| `avatar-handlers.ts` | Agent avatar management |
| `context-handlers.ts` | File context scanning |
| `data-handlers.ts` | Data operations |
| `debug-handlers.ts` | Debug ring buffer, export |
| `hub-handlers.ts` | TeamsHub registry management |
| `inbox-handlers.ts` | Inbox message resolution |
| `media-handlers.ts` | Media file resolution |
| `notification-handlers.ts` | Notification management |
| `quick-launch-handlers.ts` | Quick-launch window management |
| `search-handlers.ts` | Full-text search |
| `settings-handlers.ts` | Settings persistence |
| `stats-handlers.ts` | Usage/cost dashboard stats |
| `tray-handlers.ts` | System tray management |
| `update-handlers.ts` | Auto-updater control |
| `voice-handlers.ts` | Voice input management |
| `workspace-handlers.ts` | Workspace config/init |
| `ws-handlers.ts` | Gateway WebSocket interaction |

This is a pattern for how to structure a complex desktop app's process-boundary API.

---

## Summary: Top Features Most Likely Absent in Praxis

| # | Feature | ClawWork Complexity |
|---|---------|---------------------|
| 1 | **Desktop GUI (Electron + React)** | Full app with 3-panel layout, animations, i18n, themes |
| 2 | **Cron/Scheduled Tasks** | 7 RPCs, full UI, run history, delivery config, failure alerts |
| 3 | **Multi-Agent Teams** | Coordinator/worker pattern, registry, AI builder, install wizard |
| 4 | **Gateway Pairing Protocol** | Ed25519 device identity, QR scan, challenge-response, device tokens |
| 5 | **PWA Mobile Client** | Separate package sharing core logic with desktop |
| 6 | **OS Integration** | System tray, quick-launch, auto-update, native notifications, shortcuts |
| 7 | **Local Whisper Voice Input** | Hold-to-dictate, local-only, multiple model paths |
| 8 | **Artifact Auto-Extraction** | Code blocks/images auto-saved to filesystem workspace |
| 9 | **Approval Workflow UI** | Modal dialog for sensitive exec actions |
| 10 | **Message State Machine** | Proven architecture with documented bug patterns |

---

*End of findings. Clone of https://github.com/clawwork-ai/ClawWork has been deleted from /tmp.*
