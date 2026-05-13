---
name: github-social-poster
description: Automatically posts GitHub activity (commits, PRs, issues, releases) to X/Twitter with human-readable formatting and deduplication.
category: social-media
---

# GitHub → Social Media Poster

Fetches new GitHub activity (commits, PRs, issues, releases) and posts to X with human-readable formatting. Tracks state per-repo to prevent duplicates. Supports multiple repos/configurable tweet templates.

## When to Use

- You want your X to automatically announce code changes
- You maintain open-source projects and want to auto-tweet releases
- You want commit → tweet without manual copy-paste
- You need deduplication (no re-tweeting old commits)
- You want separate X accounts per project or personal vs org

## Setup

### 1. Install dependencies
```bash
pip install requests
```

### 2. Get GitHub token
GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
- Create token with `repo` scope (full control of private repos) or `public_repo` (public only)
- Copy the token

### 3. Configure repos
Edit `~/.echo_github_x/config.json` (auto-created on first run):

```json
{
  "github_token": "ghp_XXXXXXXXXXXXXXXXXXXX",
  "x_post_script": "/home/coemedia/patchhive_x_setup/post_x.py",
  "repos": {
    "patchhive": {
      "owner": "coemedia",
      "repo": "PatchHive",
      "post_to": "patchhive",
      "enabled": true,
      "tweet_format": {
        "commit": "🚀 {repo}: {short_msg} — {url}",
        "pr": "🔀 {repo}: PR #{num} opened — {title} {url}",
        "issue": "📝 {repo}: Issue #{num} — {title} {url}",
        "release": "🎉 {repo}: v{tag} released! {url}"
      }
    }
  },
  "schedule": "every 6h",
  "max_tweets_per_run": 3,
  "humanize": true
}
```

Variables for templates:
- `{repo}` — repository name
- `{short_msg}` — first line of commit message, cleaned up
- `{num}` — PR/issue number
- `{title}` — PR/issue title
- `{url}` — GitHub URL (already shortened)

### 4. Test
```bash
python3 ~/.echo_skills/github_x_poster/github_x_poster.py
```

First run may post recent activity; later runs only post new items.

### 5. Schedule (every 6 hours)
```bash
crontab -e
# Add:
0 */6 * * * /home/coemedia/.echo_skills/github_x_poster/github_x_poster.py >> /home/coemedia/.echo_skills/github_x_poster/cron.log 2>&1
```

## How It Works

1. Loads `state.json` → remembers last checked timestamp + per-repo checkpoint
2. For each enabled repo:
   - Fetches last 10 commits via GitHub API
   - Filters: commit date > last_check (first run uses last 3 only)
   - Fetches open PRs (created since last check)
   - Fetches open Issues (excludes PRs, created since last check)
3. Deduplicates within run (same PR won't post twice)
4. Formats each item using repo's `tweet_format` templates
5. Posts via Playwright X script (`post_x.py`) — cookie-based, no API
6. Updates `state.json` with current timestamp → next run only sees new items

## State Management

State file: `~/.echo_github_x/state.json`
```json
{
  "last_check": "2026-04-23T12:00:00+00:00",
  "repos": {
    "patchhive": {
      "last_check": "2026-04-23T12:00:00+00:00"
    }
  }
}
```

- Safe to delete → next run re-fetches recent items (bounded by `max_tweets_per_run`)
- Checkpoints per-repo → you can adjust schedules independently

## Multiple Repos / X Accounts

Add more repos under `"repos"`:
```json
"axonix": {
  "owner": "coemedia",
  "repo": "Axonix",
  "post_to": "patchhive",
  "enabled": true
},
"personal-site": {
  "owner": "yourname",
  "repo": "your-site",
  "post_to": "your_personal_x_handle",
  "enabled": false
}
```

Each repo can post to a different X account (via different `post_x.py` config or separate X script paths). Currently all use the same `x_post_script`.

## Humanization

The `humanize: true` flag (future) will:
- Replace "Issue #123 opened" → "Hey, found a bug! #123"
- Add variety: "Just shipped", "Hot off the press", "Quick fix" for commits
- Emojis already in templates; customize to your voice

Customize templates in config to match your brand tone.

## Rate Limits & Quotas

- GitHub API: 5,000 req/hr authenticated → ~10 repos every 6h is ~0.4 req/hr, negligible
- X posting: Limited by `max_posts_per_run` and daily limit in `post_x.py` config (default 4/day)
- Playwright X script: ~20–30 seconds per post (browser launch overhead)

## Pitfalls

### Same tweet posted every run (state not advancing)

The most common bug: the posting script defines a `since` parameter in its fetch function but **never passes it** in `main()`. Every run fetches the latest N commits regardless of what was already posted. If boring commits (chore, lockfile, CI-only) get filtered out by an interest filter, the same interesting commits get posted every cycle.

**Symptoms:**
- Identical tweets posted every 6 hours
- Cron log shows `Built 0 tweet(s)` followed hours later by the same successful post again
- `state.json` shows a stale `last_global_check` (or it jumps forward and back)

**Root cause check — look for this pattern in the script's `main()`:**
```python
# BUG: no since parameter passed
commits = fetch_commits(repo)

# FIX: pass the last check timestamp
last_check = state.get("last_global_check")
commits = fetch_commits(repo, since=last_check)
```

**Fix:**
1. Ensure `fetch_commits(repo, since=last_check)` is actually called with the `since` arg
2. Also deduplicate by commit message across repos — same message pushed to multiple product repos (e.g., the same fix being backported) should only be tweeted once per run:
   ```python
   seen_messages = set()
   for repo in repos:
       for c in commits:
           if c["message"] not in seen_messages:
               seen_messages.add(c["message"])
               interesting.append((repo, c))
   ```
3. State only saves when posting succeeds — if the X posting fails, the state timestamp doesn't advance and the next run will find and retry the same commits. Either save state optimistically (before posting) or log the failure clearly.

### Duplicate posts from force-push
If Drey/Codex force-pushes the same work (same message, new hash), GitHub sees a new commit object. The script sees a new SHA and tries to post again. Mitigate with message-level deduplication (above).

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| 401/403 from GitHub | Token missing/expired. Regenerate and update config |
| No tweets posted | Check GitHub API rate limit: `curl -H "Authorization: token <token>" https://api.github.com/rate_limit` |
| "Tweet failed" | Run `python3 ~/patchhive_x_setup/post_x.py --check-limit` to verify X cookies work |
| Duplicate posts | State file corrupted or not saving. Delete `~/.echo_github_x/state.json` and re-run |
| Same tweet every run | `since` parameter not passed to commit fetch (see Pitfalls above) |

## Extending

To add more activity types (discussions, deployments, stars):
- Add fetch function using GitHub API
- Add template key in config
- Append to `all_tweets` list in `main()` with appropriate dedup key

Example for discussions:
```python
def get_new_discussions(owner, repo, token, last_check):
    url = f"https://api.github.com/repos/{owner}/{repo}/discussions?per_page=10"
    # filter by created_at > last_check
    ...
```

## Files

- Main script: `~/.echo_skills/github_x_poster/github_x_poster.py`
- Config: `~/.echo_github_x/config.json`
- State: `~/.echo_github_x/state.json`
- Requirements: `~/.echo_skills/github_x_poster/requirements.txt`
