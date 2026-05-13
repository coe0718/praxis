# Code Review → Codex Fixes

A workflow for taking structured code review feedback (from Claude, humans, or other agents) and having Codex implement all fixes atomically.

## When to Use

- Got a code review from an external source (Claude, colleague, another agent)
- Feedback has multiple specific, actionable items
- Each item is scoped enough for automated implementation
- You want a single Codex pass rather than fixing each issue manually

## Workflow

### 1. Format the Review as a Prompt

Structure each feedback item with enough context for Codex to act without guessing:

```markdown
Read all source files first, then fix the following issues:

1. **Clipboard polling at 200ms is expensive** — It spawns `wl-paste` as a subprocess every 200ms forever. Use `wl-paste --watch` instead, which blocks until clipboard changes. See `src/clipboard.rs` line ~XX.

2. **Duplicated keycode table** — `key_sequence_for_char` and `keycode_for_name` both have the same keycode table in `src/backend/gnome.rs`. Consolidate into one function.

3. **Blocking Command in async code** — `detect_desktop` uses `Command::new` in `src/backend/detect.rs`. Move to `tokio::task::spawn_blocking`.

4. **Socket permissions** — After `UnixListener::bind`, set `chmod 0600` on the socket so only the daemon's user can connect. In `src/lib.rs`.

5. **CI missing lints** — `.github/workflows/ci.yml` only runs `cargo check + cargo test`. Add `cargo clippy` and `cargo fmt --check`.

6. **Python client listen loop** — `clients/python/deskbrid/client.py` has a `sleep(3600)` in the listen loop. Clean it up with better reconnection handling.

Build rules:
- cargo check must pass
- cargo test must pass
- All 6 items must be addressed
```

### 2. Write Prompt to File (Avoid $() Escaping)

```bash
cat > .codex-prompt.md << 'EOF'
[prompt content here — EOF is quoted, so $() is safe]
EOF
```

### 3. Execute in Background

```bash
codex exec --full-auto "$(cat .codex-prompt.md)"
```

Use `--full-auto` for a trusted repo where Codex can auto-approve changes. Use `background=true` if the fix list is long.

### 4. Verify All Items

After Codex finishes:

```bash
# Build
cargo check
cargo test

# Scope check
git diff --stat HEAD

# Review specific fixes
git diff HEAD -- src/clipboard.rs      # clipboard fix
git diff HEAD -- src/backend/gnome.rs  # keycode dedup
git diff HEAD -- src/lib.rs            # socket perms
git diff HEAD -- .github/workflows/    # CI changes
```

### 5. Commit with Itemized Message

Structure the commit to reference each fix — makes future reviews easier:

```
Address code review feedback: [brief summary]

- Clipboard: replace polling with wl-paste --watch (event-driven)
- Keycode: deduplicate keycode tables in gnome.rs
- Async: move blocking Command to spawn_blocking
- Security: set socket permissions to 0600 after bind
- CI: add clippy and cargo fmt --check
- Python: clean up listen loop, better reconnection
```

## Why This Works

**Single-pass atomicity** — Codex sees all issues at once and can make holistic decisions. One keycode table refactor doesn't conflict with itself. One prompt avoids the risk of Codex reverting or conflicting across multiple invocations.

**Transparency** — The commit message documents exactly what third-party feedback was addressed. Future developers (or Claude reading the repo) can see the provenance of each change.

**Separation of concerns** — The reviewer critiques, Codex implements, you verify. No one is both critic and executor. Keeps the review objective and the implementation mechanical.

## Pitfalls

1. **Don't include subjective opinions** — "This function is ugly" won't produce a fix. Convert opinions to specific actions: "Rename `foo_bar` to `foo_baz`" or "Extract the duplicated logic into a helper function."

2. **Don't include multiple approaches** — "You could use either wl-paste --watch or inotify" confuses Codex. Pick one approach per item and specify it clearly.

3. **Verify scope** — After fixing, check `git diff --stat HEAD` to confirm Codex didn't touch unrelated files. It sometimes refactors things you didn't ask it to.

4. **Codex may remove code you need later** — If the review says "this function is duplicated" and you have a future feature depending on one copy, note that explicitly: "Consolidate but keep both function signatures for now — the separate functions will diverge in the next feature."
