# Adding a New Agent to the Fleet

Complete workflow from the Quill addition (2026-05-04). Follow these steps when adding a new agent profile.

## Prerequisites

- Discord Developer Portal access (to create bot app and get token)
- Server admin permissions (to create channel and invite bot)
- OpenRouter API key or other provider credentials in auth pool

## Step-by-Step

### 1. Create the Hermes Profile

```bash
hermes profile create <name>
```

This creates `~/.hermes/profiles/<name>/` with SOUL.md, sessions/, cron/, skills/, etc. A wrapper script is also created at `~/.local/bin/<name>`.

### 2. Write SOUL.md

The auto-generated SOUL.md is minimal. Replace it with one that defines:

- **Role**: one clear sentence about what this agent does
- **Personality**: tone, voice, quirks
- **Core principles**: behavioral rules (e.g., "read source before writing", "professional not academic")
- **Workflow**: how to approach assigned tasks
- **Team context**: who Tuck is, who Drey is, how this agent fits in

⚠️ **Personality override bug**: Cloned profiles may have `personality: kawaii` or similar in config.yaml, which OVERRIDES SOUL.md. If the agent ignores its SOUL identity, check config.yaml for a `personality:` key and remove it.

### 3. Write Config.yaml

The auto-generated profile has NO config.yaml. Write one modeled after an existing agent:

```yaml
model:
  default: <model_id>
  provider: <provider>  # e.g., openrouter
agent:
  max_turns: 90
terminal:
  backend: local
  cwd: .
memory:
  provider: mnemosyne
  memory_enabled: true
discord:
  require_mention: true
  free_response_channels: '<channel_id>'
  allowed_channels: '<channel_id>'
  auto_thread: false
  reactions: false
skills:
  config:
    <custom-skill-name>: {}
platform_toolsets:
  discord:
  - hermes-discord
```

Key config values to copy from an existing working profile (e.g., Scout):

- `toolsets`, `compression`, `display`, `security`, `tts`, `stt`, `auxiliary`
- `approvals`, `delegation`, `cron`, `logging`, `session_reset`
- `_config_version: 16` (must match current Hermes version)

### 4. Set Up Auth (API Keys + Credential Pool)

The new profile needs an `auth.json` with credential pools. The simplest approach:

**Option A — Copy from working profile:**
```bash
cp ~/.hermes/profiles/scout/auth.json ~/.hermes/profiles/<name>/auth.json
```
Then add the Discord bot token to the credential pool (see below).

**Option B — Build minimal auth.json:**
```json
{
  "version": 1,
  "updated_at": "<timestamp>",
  "active_provider": "openrouter",
  "credential_pool": {
    "openrouter": [
      {
        "id": "<name>_or_1",
        "label": "OPENROUTER_API_KEY",
        "auth_type": "api_key",
        "priority": 0,
        "source": "env:OPENROUTER_API_KEY",
        "access_token": "<actual_key>",
        "last_status": "ok",
        "last_status_at": "<timestamp>"
      }
    ],
    "discord": [
      {
        "id": "<name>_discord_1",
        "label": "DISCORD_BOT_TOKEN",
        "auth_type": "bot_token",
        "priority": 0,
        "source": "env:DISCORD_BOT_TOKEN",
        "access_token": "<bot_token>",
        "last_status": "ok",
        "last_status_at": "<timestamp>"
      }
    ]
  },
  "providers": {
    "openrouter": {}
  }
}
```

### 5. Set Up .env

Minimal .env for the new profile:

```
DISCORD_BOT_TOKEN=<bot_token>
```

Other keys (OpenRouter, etc.) are inherited from the credential pool in auth.json.

### 6. Create Custom Skill

Create a skill for the agent's specialty (see `skill_manage(action='create')`). The skill should contain:

- **Templates** for the agent's output format (README templates, doc templates, etc.)
- **Style guide** specifying tone, conventions, and quality standards
- **Verification checklist** the agent should run before marking work complete

Reference the skill in config.yaml:
```yaml
skills:
  config:
    <custom-skill-name>: {}
```

### 7. Create Discord Channel

In the Discord server, create a channel named `#<name>` matching the agent's name. Note the channel ID (right-click → Copy ID with Developer Mode enabled).

### 8. Create Systemd Service

Copy an existing gateway service file and modify the profile name:

Service files live at `~/.config/systemd/user/hermes-gateway-<name>.service`.

Key differences per service file:
- `Description`: Include the agent's role
- `ExecStart`: `python -m hermes_cli.main --profile <name> gateway run --replace`
- `HERMES_HOME`: points to `~/.hermes/profiles/<name>`

```bash
# Enable and start
systemctl --user enable hermes-gateway-<name>.service
systemctl --user start hermes-gateway-<name>.service

# Verify
systemctl --user status hermes-gateway-<name>.service
```

### 9. Invite Bot to Server

The bot token authenticates to Discord's API, but the bot must also be **invited to the server** via OAuth URL:

1. Go to Discord Developer Portal → your bot app → OAuth2 → URL Generator
2. Scopes: `bot` + `applications.commands`
3. Permissions: `Send Messages`, `Read Message History`, `Use Slash Commands`
4. Visit the generated URL → pick your server → Authorize

### 10. Configure Channel

Update the profile's config.yaml with the channel ID:

```yaml
discord:
  free_response_channels: '<channel_id>'
  allowed_channels: '<channel_id>'
```

Then restart the gateway:
```bash
systemctl --user restart hermes-gateway-<name>.service
```

### 11. Test

Send a message in the agent's channel. If the channel is set to `free_response_channels`, the agent should respond without a mention. Otherwise, use `<@BOT_ID>` mention format.

## Pitfalls

- **Bot mention format matters**: Use `<@BOT_ID>` (e.g., `<@1493042656484397116>`), not `@Name`. Plain text `@Name` does NOT trigger the gateway's mention detection. The `<@ID>` format works — Jeremy confirmed the agent sees it.
- **Gateway starts then immediately exits**: Check systemd status. Common causes: missing Discord token, missing API key in credential pool, or the bot hasn't been invited to the server yet.
- **Agent silent despite running**: Check `DISCORD_ALLOWED_USERS` in .env — Tuck's bot ID (`1493038094105055313`) must be in the list for Tuck's messages to trigger a response.
- **Personality override**: As noted above, `personality:` in config.yaml overrides SOUL.md. Remove it.
- **Hindsight memory bank**: The `bank_id_template: "{profile}"` config auto-creates `bank:<name>` for new profiles. No extra setup needed.
- **Discord token in .env vs auth.json**: Token must be in BOTH. The gateway reads from .env on startup, and the credential pool in auth.json is used for runtime auth checks.
