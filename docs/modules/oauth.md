# OAuth

> OAuth 2.0 device-flow authorization for GitHub, Google, Gmail, and GitHub Copilot integrations.

## Overview

The OAuth module lets Praxis act on the operator's behalf across external services — reading Gmail, querying GitHub issues and pull requests, accessing Google Calendar and Drive, and authenticating with the GitHub Copilot Chat API. All providers use the **device authorization flow** (RFC 8628), which means no local HTTP server or browser redirect is needed. The operator opens a URL in their browser, enters a one-time code, and Praxis polls until authorization completes.

Tokens are stored in a single JSON file within the Praxis data directory. The store supports optional encryption via Praxis's `crypto` module, and on Unix the file is written with owner-only (`600`) permissions. Tokens that support refresh (Google) are refreshed automatically by API clients; tokens that do not expire by default (GitHub, Copilot) require re-login only if revoked.

## Architecture

### Provider structs

| Struct | Purpose |
|--------|---------|
| `GitHubOAuth` | Device-flow login against GitHub's `/login/device/code` endpoint. No client secret required. |
| `GoogleOAuth` | Device-flow login against Google's `/o/oauth2/device/code` endpoint. Requires client ID and secret. Supports token refresh. |
| `CopilotOAuth` | Device-flow login for the GitHub Copilot Chat API. Reuses GitHub's OAuth endpoints with `copilot` and `read:org` scopes. |
| `GmailClient` | REST client for Gmail using a stored Google token. Auto-refreshes expired tokens on construction. |
| `GitHubClient` | REST client for the GitHub API using a stored GitHub token. Provides issue and PR listing. |

### Core infrastructure

| Struct | Purpose |
|--------|---------|
| `OAuthToken` | Serializable token record: provider, access/refresh tokens, scopes, expiry, authorized timestamp. |
| `OAuthTokenStore` | JSON-backed key-value store keyed by provider name. Handles load/save/remove with optional encryption. |
| `DeviceFlowConfig` | Parameterizes the two-step device flow (code URL, token URL, client ID/secret, scope, grant type). |

### Device flow internals

The `device_flow` submodule implements the generic two-step flow:

1. **`request_device_code()`** — POSTs to the provider's code URL with client ID and scopes. Returns a `DeviceCodeResponse` containing the `device_code`, `user_code`, `verification_uri`, polling interval, and expiry.
2. **`poll_for_token()`** — Loops at the provider-suggested interval, POSTing to the token endpoint. Handles `authorization_pending` (keep polling), `slow_down` (increase interval by 5 s), `access_denied` (bail), and `expired_token` (bail).

## Public API

### Provider login

```rust
// GitHub
let gh = GitHubOAuth::from_env()?;          // reads PRAXIS_GITHUB_OAUTH_CLIENT_ID
let token = gh.login("repo user read:org")?; // interactive device flow

// Google
let goog = GoogleOAuth::from_env()?;         // reads PRAXIS_GOOGLE_OAUTH_CLIENT_ID + _SECRET
let token = goog.login(scopes)?;

// Google token refresh
let refreshed = goog.refresh(&token)?;
```

### API clients (auto-refresh on construction)

```rust
let store = OAuthTokenStore::new(&data_dir);

// Gmail — auto-refreshes if token is near expiry
let gmail = GmailClient::from_store(&store)?; // -> Option<GmailClient>
if let Some(g) = gmail {
    let emails = g.list_recent(10)?;  // Vec<EmailSummary>
}

// GitHub — checks expiry, warns if expired
let gh = GitHubClient::from_store(&store)?; // -> Option<GitHubClient>
if let Some(g) = gh {
    let issues = g.list_open_issues("owner", "repo")?; // Vec<IssueSummary>
    let prs = g.list_open_prs("owner", "repo")?;       // Vec<PrSummary>
}
```

### Token store

```rust
let store = OAuthTokenStore::new(&data_dir);
store.save(&token)?;
let loaded: Option<OAuthToken> = store.get("github")?;
let all: HashMap<String, OAuthToken> = store.load()?;
store.remove("github")?; // -> bool
```

### Token introspection

```rust
token.is_expired()    // true if past expires_at
token.needs_refresh() // true if expires within 5 minutes
```

## Configuration

### Environment variables

| Variable | Provider | Required | Description |
|----------|----------|----------|-------------|
| `PRAXIS_GITHUB_OAUTH_CLIENT_ID` | GitHub | Yes | OAuth App client ID from [GitHub Developer Settings](https://github.com/settings/developers) |
| `PRAXIS_GOOGLE_OAUTH_CLIENT_ID` | Google | Yes | "TV and limited input devices" OAuth client ID from [Google Cloud Console](https://console.cloud.google.com/apis/credentials) |
| `PRAXIS_GOOGLE_OAUTH_CLIENT_SECRET` | Google | Yes | OAuth client secret (Google device flow requires it) |
| `PRAXIS_COPILOT_OAUTH_CLIENT_ID` | Copilot | Yes | OAuth App client ID for Copilot Chat access |

### Default scopes

| Provider | Default scopes |
|----------|---------------|
| GitHub | `repo user read:org` |
| Google | `gmail.readonly calendar drive.readonly userinfo.email userinfo.profile` |
| Copilot | `read:org copilot` |

### Feature flags

No feature flag required — the OAuth module is always compiled.

## Usage

### CLI commands

```bash
# Authorize a provider (interactive — opens browser)
praxis oauth login github
praxis oauth login google
praxis oauth login copilot

# Override default scopes
praxis oauth login google --scopes "https://www.googleapis.com/auth/gmail.readonly"

# Check status of all providers
praxis oauth status

# Print raw access token (useful for shell scripting)
praxis oauth token github

# Force a token refresh (Google only; GitHub tokens don't expire)
praxis oauth refresh google

# Remove stored authorization
praxis oauth revoke github
```

## Data Files

| File | Location | Description |
|------|----------|-------------|
| `oauth_tokens.json` | `{data_dir}/oauth_tokens.json` | JSON map of provider → `OAuthToken`. Auto-encrypted if Praxis encryption is configured. |

## Dependencies

- **`crypto`** — optional encryption/decryption of the token store file via `maybe_encrypt` / `maybe_decrypt`.
- **`reqwest`** (blocking) — HTTP client for OAuth endpoints and API calls.
- **`chrono`** — timestamp handling for token expiry.
- **`serde` / `serde_json`** — serialization of tokens and API responses.

## Source

`src/oauth/`
