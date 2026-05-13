# GitHub Packaging Pattern — Tool Scripts

When packaging a local tool script as a standalone GitHub repo, strike this balance: **clean enough to share, real enough to actually work.**

## Repo Structure

```
twitter-setup/
├── post_x.py              # The actual working script
├── config.json.example    # Template with placeholders, no secrets
├── .gitignore             # Keep creds out
└── README.md              # Full docs, generic paths, no insider refs
```

## The Files

### `config.json.example`

Placeholder values only — never real credentials:

```json
{
  "auth_token": "paste-your-auth-token-here",
  "ct0": "paste-your-ct0-here",
  "max_posts_per_day": 4,
  "headless": true
}
```

### `.gitignore`

Block config files, auto-generated state, post logs, Python artifacts:

```
# Config with real credentials
config.json

# Auto-generated Playwright storage state
x_state.json

# Local post log (daily counter)
post_log.json

# Python
__pycache__/
*.pyc
*.pyo
*.egg-info/
dist/
build/

# OS
.DS_Store
Thumbs.db
```

### `README.md` Conventions

When Jeremy says "make it a good .md file, I'm posting it on my GitHub," here's the checklist:

1. **Strip all personal paths** — no `/home/coemedia/`, no `~/.patchhive_x` that references YOUR machine. Use generic examples like `/path/to/post_x.py` or explain the config dir gets created automatically.
2. **No insider references** — don't mention Echo, PatchHive, your homelab, or other projects. Anyone should be able to use it without your context.
3. **Copy-paste commands** — every step should be runnable by pasting into a terminal. No "then modify the path" steps.
4. **Strong opening hook** — lead with the problem it solves (API costs money, this is free).
5. **Honest trade-offs** — include a "Fragility" section. This isn't a polished product, and pretending otherwise erodes trust.
6. **Troubleshooting table** — the most common failures mapped to fixes. This is what saves people time.
7. **License** — "Do whatever you want with this" is fine for a technique repo.

### Commit Flow

```bash
git clone https://github.com/owner/twitter-setup.git
cd twitter-setup
cp /path/to/post_x.py .
# create config.json.example, .gitignore
git add -A
git commit -m "Full repo: post_x.py, config template, gitignore, polished README"
git push
```

## When to Use This Pattern

Use when Jeremy says any of:
- "I'm gonna post this on my GitHub"
- "Make it a good .md file"
- "Can you package this up?"
- "I already created a repo for it"

The script file itself should come from the homelab (it's battle-tested), but the rest of the repo is built fresh.
