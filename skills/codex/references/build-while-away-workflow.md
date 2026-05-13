# Build-While-Away Workflow

When the user is not at the keyboard (at work, asleep, etc.) but wants progress made, use this pattern to maximize output.

## Parallel Delegation Pattern

Jeremy's explicit workflow preference: **"Send the work to Codex so we can keep talking."**

Do NOT wait for Codex to finish before discussing next steps. The pattern is:

1. Fire Codex in background with `notify_on_complete=true`
2. Immediately continue talking about strategy, architecture, next phases
3. When Codex finishes notification arrives, review and commit
4. Fire next Codex phase from the conversation you've been having
5. Repeat

This is not "background processing" — it's **parallel streams**. One thread builds, the other plans. The user sees progress on both simultaneously.

### How to structure parallel conversation while Codex builds

- Use the open time to discuss strategic questions you'd normally save for later
- Lay out architecture options (traits vs enums, monorepo vs multi-crate, feature flags)
- Talk about market positioning, investors, documentation improvements
- When Codex finishes, switch quickly to review/commit/fire-next

The `notify_on_complete` flag is critical here — it means you don't poll. The system tells you when to switch contexts.

1. Assess what can be done without user interaction
   - Codex builds, docs, CI, demos — anything that compiles/tests/installs
   - NOT: things needing live system access (DBus, Wayland, GNOME sessions)

2. Fire Codex in background with notify_on_complete=true
   - Frees you to think, plan, talk to user about strategy

3. While Codex runs, talk to user about:
   - Architecture decisions ("should we make it a trait?")
   - Next phase scope ("what about KDE support?")
   - Strategic direction ("this could be a platform play")

4. When Codex finishes:
   - Verify build + tests
   - Review diff for unintended changes
   - Clean artifacts (.codex, .codex-prompt.md, build/)
   - Commit and push
   - Fire next Codex phase with lessons from the last

5. Repeat until the user's keyboard is available

## What You Can Build From Here

From a headless Hermes session with no desktop access:

| Can build | Can't build/test |
|---|---|
| Rust code (compiles, tests pass) | PipeWire screencast (needs compositor) |
| Python libraries (pip install works) | DBus calls (needs session bus) |
| CI/CD configs (runs on GitHub) | Input injection (needs GNOME session) |
| Documentation and READMEs | Desktop detection (needs env vars) |
| Protocol specs and architecture | Portal permission flows |
| Systemd services, shell scripts | Real window focus tracking |
| Hermes skills, Praxis tools | Audio node monitoring |

## Demo-Ready Handoff

When the user gets back, they should be able to run ONE command:

```bash
# A good demo.sh
git pull && cargo build && bash demo.sh
```

Always leave a one-liner ready:
```
"When you're home: cd ~/projects/deskbrid && bash demo.sh"
```
