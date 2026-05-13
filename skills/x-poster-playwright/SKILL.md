---
name: x-poster-playwright
description: Reliable cookie-based X posting using Playwright (no API keys). Handles auth via stored cookies, survives bot detection, supports images.
category: social-media
---

# X Poster via Playwright

Cookie-based X posting without API. Uses Playwright for reliable browser automation that survives X's bot detection. Saves session state for reuse.

## When to Use

- You need to post to X programmatically but don't want to pay for API access
- You have a browser-authenticated X session (cookies) available
- Scweet/Selenium keep breaking due to X DOM changes
- You want persistent login across runs (no re-auth every time)

## Setup

### 1. Install dependencies
```bash
pip install playwright
playwright install chromium
```

### 2. Get X cookies (one-time)
**Option A — Direct cookie values (fastest):**
```bash
python3 ~/patchhive_x_setup/post_x.py --setup-manual
# paste auth_token and ct0 from browser
```

**Option B — Export all cookies:**
- Chrome/Firefox: DevTools → Application → Storage → Cookies → https://x.com
- Copy all as JSON or use Cookie Export extension
- Save to `~/.patchhive_x/cookies_chrome.json` (or `cookies_firefox.json`)
- Run: `python3 ~/patchhive_x_setup/post_x.py --import-chrome` (or `--import-firefox`)

**Option C — Interactive login (first run only):**
```bash
python3 ~/patchhive_x_setup/post_x.py "test"
# Opens browser, log in manually, saves state
```

### 3. Test
```bash
python3 ~/patchhive_x_setup/post_x.py "Test from Playwright"
```

## Usage

```bash
# Post text
python3 ~/patchhive_x_setup/post_x.py "Your tweet here"

# Post with image
python3 ~/patchhive_x_setup/post_x.py "Check this out" /path/to/image.png

# Check daily limit
python3 ~/patchhive_x_setup/post_x.py --check-limit

# Reset cookies (if session expires)
python3 ~/patchhive_x_setup/post_x.py --reset-cookies
```

## Configuration

File: `~/.patchhive_x/config.json`
```json
{
  "auth_token": "long-jwt-token-here",
  "ct0": "csrf-token-here",
  "max_posts_per_day": 4,
  "headless": true
}
```

## How It Works

1. **First run**: Opens Chromium, navigates to x.com, you log in manually → saves full storage state (cookies + localStorage) to `~/.patchhive_x/x_state.json`
2. **Subsequent runs**: Loads saved state, navigates directly to home, clicks compose, types text, posts
3. **Cookie injection** (fallback): If state file missing, uses `auth_token` + `ct0` from config to set cookies before navigation
4. **Bot evasion**: Spoofs `navigator.webdriver`, uses real browser context
5. **State persistence**: Cookies saved after each successful run → no re-login needed
6. **Rate limiting**: Tracks daily post count in `post_log.json`

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| "Login failed — cookies invalid" | Cookies expired. Delete `~/.patchhive_x/x_state.json`, run `--setup-manual` again with fresh cookies |
| "Could not find textarea" | X's DOM changed. Update selectors in `post_x.py` → `post_tweet()` function |
| Posting fails silently | Run with `headless: false` in config to see browser UI |
| 403/Account locked | X rate-limited you. Wait 24h, reduce `max_posts_per_day` |
| Chrome download fails | `playwright install chromium` (may need `--with-deps` on Linux) |

## Why Not Scweet?

- Scweet uses Selenium + ChromeDriver → brittle, unmaintained
- X changes DOM constantly; Scweet breaks every 2–3 months
- Playwright is actively maintained, handles dynamic content better
- Storage state API persists full session (cookies + localStorage) → single login lasts months

## Integration

Call from other scripts:
```python
import subprocess
result = subprocess.run(
    ['python3', os.path.expanduser('~/patchhive_x_setup/post_x.py'), 'Your tweet'],
    capture_output=True, text=True
)
if result.returncode == 0:
    print("Posted!")
```

## Published Repo

Now available as a standalone project: **coe0718/twitter-setup** — https://github.com/coe0718/twitter-setup

Contains `post_x.py`, `config.json.example`, `.gitignore`, and a polished README. Clone it, run `--setup-manual`, and you're posting. The repo is the canonical reference for anyone wanting to use this without digging through homelab paths.

## Files

- Script: `~/patchhive_x_setup/post_x.py`
- Config: `~/.patchhive_x/config.json`
- State: `~/.patchhive_x/x_state.json` (auto-generated)
- Log: `~/.patchhive_x/post_log.json`
- Reference: `references/github-packaging-pattern.md` — how to package tool scripts as standalone GitHub repos
