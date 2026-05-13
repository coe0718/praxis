---
name: codex
description: "Delegate coding to OpenAI Codex CLI (features, PRs)."
version: 1.0.0
author: Hermes Agent
license: MIT
metadata:
  hermes:
    tags: [Coding-Agent, Codex, OpenAI, Code-Review, Refactoring]
    related_skills: [claude-code, hermes-agent]
---

# Codex CLI

Delegate coding tasks to [Codex](https://github.com/openai/codex) via the Hermes terminal. Codex is OpenAI's autonomous coding agent CLI.

## When to use

- Building features
- Refactoring
- PR reviews
- Batch issue fixing

Requires the codex CLI and a git repository.

## Prerequisites

- Codex installed: `npm install -g @openai/codex`
- Authenticated (either ChatGPT OAuth or OpenAI API key — check `~/.codex/auth.json` for `auth_mode`)
- **Must run inside a git repository** — Codex refuses to run outside one
- Use `pty=true` in terminal calls — Codex is an interactive terminal app

## One-Shot Tasks

```
terminal(command="codex exec 'Add dark mode toggle to settings'", workdir="~/project", pty=true)
```

For scratch work (Codex needs a git repo):
```
terminal(command="cd $(mktemp -d) && git init && codex exec 'Build a snake game in Python'", pty=true)
```

## Background Mode (Long Tasks)

```
# Start in background with PTY — use notify_on_complete to get pinged
terminal(command="codex exec --full-auto 'Refactor the auth module'", workdir="~/project", background=true, pty=true, notify_on_complete=true)
# Returns session_id

# Monitor progress
process(action="poll", session_id="<id>")
process(action="log", session_id="<id>")

# Send input if Codex asks a question
process(action="submit", session_id="<id>", data="yes")

# Kill if needed
process(action="kill", session_id="<id>")
```

The `notify_on_complete=true` flag sends a system notification when the background process exits. Use this for long Codex builds so you don't have to poll manually.

## Key Flags

| Flag | Effect |
|------|--------|
| `exec "prompt"` | One-shot execution, exits when done |
| `--full-auto` | Sandboxed but auto-approves file changes in workspace |
| `--yolo` | No sandbox, no approvals (fastest, most dangerous) |

## Model & Auth

Codex CLI has two auth modes (check `~/.codex/auth.json` for `auth_mode`):

| Mode | Source | Model Flexibility |
|---|---|---|
| `chatgpt` | OAuth via ChatGPT login | Restricted to ChatGPT account tier |
| `api_key` | `OPENAI_API_KEY` set | Full OpenAI API model access |

### ChatGPT OAuth Restrictions

- **Default model** varies by CLI version. CLI 0.118.0 uses the ChatGPT account's default.
- **gpt-5.x** requires a newer Codex CLI version. Error: `The 'gpt-5.5' model requires a newer version of Codex.`
- **o4-mini / gpt-4o** are NOT supported with ChatGPT auth. Error: `model is not supported when using Codex with a ChatGPT account.`
- **Fix**: remove the `model = "..."` line from `~/.codex/config.toml`. The CLI falls back to a compatible default.

### API Key Mode

- Full model access based on API key tier.
- Can use any model available via OpenAI API.

### Troubleshooting Model Errors

If `codex exec` fails with a model error:

1. Check `codex --version` — is the CLI new enough for the configured model?
2. Try `codex exec -m gpt-4o "..."` (note: gpt-4o may also fail with ChatGPT auth)
3. **Fallback**: remove the `model` line from `config.toml`. The CLI picks its own default, guaranteed compatible.
4. Can't run `codex models` from Hermes (needs TTY) — check config directly: `cat ~/.codex/config.toml | grep model`

### How to Check What Model Is Actually Running

When no model line is in `config.toml` and you want to know what Codex picked:
```bash
ps aux | grep -E "codex.*exec" | grep -v grep
```
The process args or binary name may reveal the model. Alternatively, check `~/.codex/sessions/` for session metadata.

See `references/model-auth-compatibility.md` for the full error message reference and model compatibility table.

## PR Reviews

Clone to a temp directory for safe review:

```
terminal(command="REVIEW=$(mktemp -d) && git clone https://github.com/user/repo.git $REVIEW && cd $REVIEW && gh pr checkout 42 && codex review --base origin/main", pty=true)
```

## Parallel Issue Fixing with Worktrees

```
# Create worktrees
terminal(command="git worktree add -b fix/issue-78 /tmp/issue-78 main", workdir="~/project")
terminal(command="git worktree add -b fix/issue-99 /tmp/issue-99 main", workdir="~/project")

# Launch Codex in each
terminal(command="codex --yolo exec 'Fix issue #78: <description>. Commit when done.'", workdir="/tmp/issue-78", background=true, pty=true)
terminal(command="codex --yolo exec 'Fix issue #99: <description>. Commit when done.'", workdir="/tmp/issue-99", background=true, pty=true)

# Monitor
process(action="list")

# After completion, push and create PRs
terminal(command="cd /tmp/issue-78 && git push -u origin fix/issue-78")
terminal(command="gh pr create --repo user/repo --head fix/issue-78 --title 'fix: ...' --body '...'")

# Cleanup
terminal(command="git worktree remove /tmp/issue-78", workdir="~/project")
```

## Batch PR Reviews

```
# Fetch all PR refs
terminal(command="git fetch origin '+refs/pull/*/head:refs/remotes/origin/pr/*'", workdir="~/project")

# Review multiple PRs in parallel
terminal(command="codex exec 'Review PR #86. git diff origin/main...origin/pr/86'", workdir="~/project", background=true, pty=true)
terminal(command="codex exec 'Review PR #87. git diff origin/main...origin/pr/87'", workdir="~/project", background=true, pty=true)

# Post results
terminal(command="gh pr comment 86 --body '<review>'", workdir="~/project")
```

## Post-Run Cleanup

After a `codex exec` that produced changes, always do the following:

1. **Verify compilation/tests** — `cargo check` or the project's equivalent build step. Don't trust exit code 0 alone — Codex reports success even if it couldn't compile.

2. **Restore user's config** — If you modified `~/.codex/config.toml` (e.g., removed `model` line), restore it to the original. The user may have a model set that only works on a newer CLI version — don't leave their config broken.

3. **Remove artifacts every phase** — Codex creates files every run that shouldn't be committed. **These get regenerated on every Codex invocation**, so this isn't a one-time fix:
   - `.codex` — Codex writes its own copy of the prompt here (regenerates on each run)
   - `.codex-prompt.md` — your prompt file
   - `build/`, `*.egg-info/`, `__pycache__/` — Python build artifacts
   
   Run this after EVERY Codex phase, not just the first one:
   ```bash
   git rm --cached .codex .codex-prompt.md 2>/dev/null; git add -A
   ```
   
   Keep `.gitignore` up to date as new artifact types appear. The `.codex` file will show as tracked after every Codex run if you don't gitignore it early.

4. **Check git diff stats** — `git diff --stat HEAD` shows what changed. Confirm the scope matches what was asked. Watch for:
   - **Deleted code** — Codex may gut modules it thinks are unused (e.g., stripping `pipewire` deps that were TODO-only). Re-add if the next phase needs them.
   - **Architecture refactors** — Codex may move code between files, extract structs, change function signatures. Verify the refactor matches intent before subsequent phases build on the new structure.
   - **Cargo.toml removals** — Codex strips unused deps aggressively. If a dep was placeholder for a future phase, restore it.

5. **Add project to .gitignore** if new artifact types appeared. Common additions for Codex-generated projects:
   ```
   __pycache__/
   *.pyc
   build/
   *.egg-info/
   .codex
   .codex-prompt.md
   ```

6. **Commit and push** — structure the commit message as `Phase N: short description` for multi-phase builds.

## Shell Escaping (Critical Pitfall)

When passing long prompts via `codex exec "..."`, bash interprets special characters in the prompt string before it reaches Codex:

- **`$()`** — bash tries to execute as command substitution. If your prompt mentions DBus type syntax like `(usss)` or similar, bash treats `()` as subshell syntax and errors out. **Never** put unescaped `$()` in a prompt string passed via `$()`.

**Safer approach for complex prompts:**
```bash
# Write prompt to a file, then pass it
cat > .codex-prompt.md << 'EOF'
Your multi-line prompt here with $() syntax or any special chars
EOF
codex exec --full-auto "$(cat .codex-prompt.md)"
```

This works because `$(cat file)` reads the file content as a single argument, and since the prompt was written via heredoc (quoted 'EOF' prevents expansion), special chars are preserved.

## Sandbox Limitations

Codex runs commands inside a sandbox container. This sandbox has restricted access to system services:

- **No DBus access** — `gdbus introspect --session` fails with "Operation not permitted." Codex cannot probe live DBus interfaces, Wayland connections, or PulseAudio/PipeWire sockets.
- **No Wayland/Windowing** — Can't access the display server, get window lists, or interact with GNOME/KDE APIs.
- **Workaround**: If your task requires system service knowledge (DBus interface signatures, Wayland protocol details, etc.), **include the interface details explicitly in the prompt**. Don't expect Codex to discover them at runtime.
- **Filesystem is shared** — the sandbox can read/write the project directory. This is how changes get persisted. Use this: Codex edits files, you verify outside the sandbox.
- **Network is available** — the sandbox can install crates, fetch docs, etc.

## Phased Build Workflow

For large projects, break the work into successive Codex invocations. Each phase builds on the last with a narrower scope:

```
Phase 1:  "Implement the core daemon. Cargo check must pass."
Phase 1.1: "Add feature X. Add graceful shutdown."
Phase 2:   "Build the Python client library. Don't touch existing Rust code."
Phase 2.1: "Add CI, demo script, documentation."
```

### Prompt strategy per phase

| Phase | Prompt style | What to include |
|---|---|---|
| **Phase 1** | Broad architecture | Key APIs, DBus interfaces, build rules, constraints. Let Codex make structural decisions. |
| **Mid phases** | Specific additions | "Add X to existing module Y." Reference existing code structures. |
| **Cleanup** | Docs + config | CI, README, demo scripts, integrations. Lowest risk, highest polish. |

### What Codex may do beyond what you asked

- **Remove unused dependencies** — Codex may strip Cargo.toml deps not actively imported. Verify with build step.
- **Refactor architecture** — Codex may create new modules, restructure state into structs, move code between files. Verify intent after each phase.
- **Create build artifacts** — Python projects get `build/`, `*.egg-info/`, `__pycache__/`. Codex writes `.codex` and `.codex-prompt.md`. Update `.gitignore` per phase.

### Verification cadence

1. `cargo check` or equivalent build step
2. `cargo test` if tests exist
3. `git diff --stat HEAD` — confirm scope matches intent
4. Check for deleted code or unintended dependency changes
5. Remove Codex artifacts, update `.gitignore`
6. Restore user's `~/.codex/config.toml` if you modified it
7. Commit with `Phase N: description` message

## Pitfalls

1. **Always use `pty=true`** — Codex is an interactive terminal app and hangs without a PTY
2. **Git repo required** — Codex won't run outside a git directory. Use `mktemp -d && git init` for scratch
3. **Use `exec` for one-shots** — `codex exec "prompt"` runs and exits cleanly
4. **`--full-auto` for building** — auto-approves changes within the sandbox in a trusted project
5. **`--yolo` for trusted repos** — no sandbox, no approvals (fastest). Only use with projects already in config.toml's trusted list
6. **Background for long tasks** — use `background=true` and monitor with `process` tool
7. **Don't interfere** — monitor with `poll`/`log`, be patient with long-running tasks
8. **Parallel is fine** — run multiple Codex processes at once for batch work
9. **Trust the project first** — add `[projects."/path/to/project"]` with `trust_level = "trusted"` to `~/.codex/config.toml` before running `--full-auto`, otherwise Codex prompts for approval on every file write

## Code Review → Codex Fixes

See `references/code-review-driven-fixes.md` for the workflow of taking third-party code review feedback (from Claude, another agent, or a human) and having Codex implement all fixes atomically in a single pass. Covers prompt formatting, verification, and commit message structure.

## Context Isolation (Critical)

Codex CLI and Codex VS Code Extension run in **separate session contexts**. A conversation you started in VS Code's extension panel is invisible to the CLI and vice versa. There is no shared history, no shared AGENTS.md loading, no shared state.

If Jeremy was working in VS Code with Codex and you want to continue that work from the CLI:
- **You can't just pick up the conversation** — the CLI starts fresh with no memory of what the extension discussed.
- What you CAN do: read the project files (he was working on PatchHive launcher, for example), inspect recent file changes with `find ... -mtime`, and feed the CLI a self-contained prompt with enough context to continue.
- **AGENTS.md / CLAUDE.md** files are loaded by the VS Code extension on project open but NOT by the CLI. If the task depends on context in those files, you must include it in your prompt to the CLI.
- **Filesystem is shared** — changes made by one are visible to the other. Use this. Codex CLI edits files, VS Code picks them up automatically. No need for the CLI session to "know" what VS Code was doing.
