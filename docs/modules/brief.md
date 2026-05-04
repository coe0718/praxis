# Brief

> Morning brief generation for Telegram delivery

## Overview

The `brief` module generates a concise, human-readable morning brief that aggregates the most important information the operator needs to start their day. It is designed to fit within a single Telegram message and covers active goals, recent memories, pending approvals, boundary review status, recent events, Gmail inbox (if configured), and a journal excerpt.

Briefs are sent once per calendar day, gated by `data_dir/brief_sent.txt`. The runtime's `try_send_morning_brief()` function checks this file and silently skips sending if the env vars for Telegram are not set, making the system safe for installations without messaging configured.

The greeting is time-of-day aware: "Good morning" (5–11), "Good afternoon" (12–17), "Good evening" (18–21), or "Hello" (otherwise), based on the UTC hour.

## Architecture

### Key Functions

| Function | Purpose |
|---|---|
| `generate_brief(paths, now)` | Assemble all sections into a single brief string |
| `active_goals_section(paths)` | Read `GOALS.md` and extract up to 5 unchecked (`- [ ]`) goals |
| `hot_memories_section(paths)` | Query the 3 most recent hot memories from SQLite |
| `pending_approvals_section(paths)` | List up to 3 pending tool approval requests |
| `recent_events_section(paths)` | Show the last 5 events from the event log |
| `gmail_section(paths)` | List up to 5 unread emails (if OAuth is configured) |
| `journal_excerpt_section(paths)` | Show the last 4 non-empty journal lines |
| `greeting_for_hour(hour)` | Return the appropriate greeting for the time of day |

### Brief Sections

Each section is independently generated and only included if it has content. The sections are joined with double newlines:

1. **🌅 Greeting** — Time-aware greeting + "Morning Brief" title
2. **📋 Active goals** — Up to 5 open goals from `GOALS.md`
3. **🧠 Top memories** — 3 most recent hot memories (truncated to 100 chars)
4. **⏳ Pending approvals** — Up to 3 pending tool approval requests (truncated to 60 chars)
5. **⚠️ Boundary review** — Warning if a weekly boundary review is overdue
6. **📡 Recent events** — Last 5 events from the event log
7. **✉️ Gmail** — Up to 5 unread emails (requires OAuth setup)
8. **📓 Journal** — Last 4 non-empty lines from the journal

## Public API

```rust
pub fn generate_brief(paths: &PraxisPaths, now: DateTime<Utc>) -> Result<String>
```

## Configuration

| Config | Location | Description |
|---|---|---|
| `PRAXIS_TELEGRAM_BOT_TOKEN` | Environment variable | Telegram bot API token |
| `PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS` | Environment variable | Comma-separated allowed chat IDs |
| Gmail OAuth | `data_dir/oauth_tokens/` | Required for email section |

The brief itself is configured by the data it aggregates — goals, memories, approvals, and events come from the standard data files and SQLite database.

## Usage

### Automatic

Briefs are sent automatically by the runtime's `try_send_morning_brief()` function, called during the post-reflect phase. It checks `brief_sent.txt` and sends at most once per calendar day.

### CLI

```bash
# Generate and preview a brief without sending
praxis brief preview

# Force-send a brief (updates brief_sent.txt)
praxis brief send
```

## Data Files

| File | Purpose |
|---|---|
| `data_dir/brief_sent.txt` | Date string of last sent brief (prevents duplicates) |
| `data_dir/GOALS.md` | Active goals (parsed for `- [ ]` items) |
| `data_dir/journal.md` | Journal (last 4 lines shown) |
| `data_dir/events.jsonl` | Event log (last 5 events) |
| `data_dir/boundary_review.json` | Boundary review state |
| SQLite database | Hot memories, approval requests |

## Dependencies

- `storage` — `SqliteSessionStore`, `ApprovalStore`
- `boundaries` — `BoundaryReviewState`, `review_prompt`
- `events` — `read_events_since`
- `oauth` — `GmailClient`, `OAuthTokenStore` (optional)
- `memory` — `MemoryStore`
- `paths` — `PraxisPaths`

## Source

`src/brief/`
