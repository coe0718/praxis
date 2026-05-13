# Deskbrid Build — Multi-Phase Codex Case Study

A real example of building a ~7,500 line project across 7 Codex invocations.

## Project

[deskbrid](https://github.com/coe0718/deskbrid) — JSON-over-Unix-socket daemon for desktop agent control (Rust + Python client).

## Phase Sequence

| Phase | Prompt scope | Codex output | Key lesson |
|---|---|---|---|
| **Phase 1** | "Implement Phase 1 deskbrid daemon. Read PROTOCOL.md and all src/*.rs. Cargo check must pass." | 2,484 lines across 6 files (input.rs, dbus.rs, clipboard.rs, capture.rs, lib.rs, tests) | Give broad architecture + specific DBus APIs (Shell.Eval, RemoteDesktop). Codex needs system interface details in prompt — its sandbox can't probe DBus. |
| **Phase 1.1** | "Add display:list, graceful shutdown, startup resilience, systemd service, README prerequisites" | 486 lines added, startup resilience refactored entire state model | Codex may restructure architecture (DaemonState, DaemonCapabilities). Verify intent. |
| **Phase 2** | "Build Python client library + Rust CLI tool. Read existing source first." | 1,816 lines across 21 files (cli.rs, Python client 4 files, Cargo.toml deps cleanup) | Prompt said "Don't touch existing Rust code" but Codex still refactored main.rs → cli.rs. Use explicit constraints. |
| **Phase 2.1** | "Add CI, demo script, Python README, Hermes skill, Praxis tool manifest. No Rust/Python changes." | 744 lines of docs/config (ci.yml, demo.sh, README.md, hermes/) | Cleanup phase is lowest risk. No build verification needed for docs. |
| **Backend refactor** | "Refactor to support multiple desktop backends. Create DesktopBackend + InputBackend traits." | ~1,000 lines moved into src/backend/ (gnome.rs, kde.rs, detect.rs, types.rs, mod.rs) | Codex gutted dbus.rs and input.rs (moved code to gnome.rs). Verify intent. |
| **Wlroots backend** | "Add wlroots detection stub for Sway/Hyprland/River/Wayfire." | 157 lines of wlroots.rs + detect.rs update | Smallest phase. Detection checks for binary existence (sway/hyprctl/riverctl/wayfire). |
| **PipeWire + Audio** | "Implement screencast via Mutter.ScreenCast DBus + PipeWire stream; audio monitoring via PipeWire registry." | 707 lines across screencast.rs (452) + audio.rs (255) | Codex gated behind #[cfg(feature = "pipewire")] automatically. Feature flags keep compilation clean without installed deps. |

## Prompt Patterns That Worked

### Phase 1 (big build)
```
Implement Phase 1 deskbrid daemon. This is a Rust project at /path/to/project.
Read PROTOCOL.md first. Read all src/*.rs files.

## What to build
### 1. Feature X
- Specific API: method signatures, endpoint paths, key mappings
- Approach: use Z crate's Y method

### N. Feature N
...

## Build rules
- cargo check must pass
- No unwrap() in production paths
- Write tests in tests/
- Keep PROTOCOL.md in sync

Start by reading PROTOCOL.md and all src/*.rs, then implement. Run cargo check at end.
```

### Phase 1.1 (refinement)
```
Add the following to the project. Read all existing src/*.rs files first.

1. Action X — Add to dispatch_action in src/lib.rs

2. Feature Y — In src/module.rs, add...

Build rules:
- cargo check must pass
```

### Phase 2 (new subsystem)
```
Build a Python client library at clients/python/.
Read all existing source files first.

The client API should look like:
```python
from deskbrid import Deskbrid
client = Deskbrid()
client.type_text("hello")
```

Build rules:
- pip install ./clients/python/ must work
- cargo check must still pass (don't break Rust code)
```

## Pitfalls Encountered

1. **Model errors block everything** — `codex exec` fails immediately if the model in `config.toml` isn't compatible with the auth mode. Fix: remove `model` line from config.

2. **Prompts with `$()` break bash** — DBus type syntax like `(o session_path)` gets interpreted as command substitution. Fix: write prompt to file first, `$(cat file)` to pass.

3. **Codex removes unused deps** — In Phase 2, Codex stripped `pipewire`, `wayland-client`, `wayland-protocols` from Cargo.toml because they had TODO-only imports. It was correct, but surprising. Verify dependency changes.

4. **Codex restructures without asking** — Phase 1.1 refactored `main.rs` into `cli.rs` + `lib.rs` with a new `DaemonState` struct and `shutdown_rx` parameter. The skill should note this can happen.

5. **Build artifacts accumulate** — After 4 phases: `.codex`, `.codex-prompt.md`, `build/`, `*.egg-info/`, `__pycache__/`. Track in `.gitignore` after each phase.

6. **Restore user's Codex config** — Every invocation required removing the `model` line from `~/.codex/config.toml`. Must restore it after the final phase.

7. **Codex generates feature flags automatically** — For platform-specific code (PipeWire screencast, audio), Codex used `#[cfg(feature = "pipewire")]` with stub alternatives behind `#[cfg(not(feature = "pipewire"))]`. This is a smart pattern worth keeping: gate platform-dependent code behind Cargo features so the project compiles cleanly without exotic system dependencies.

8. **Parallel delegation pattern** — Jeremy explicitly prefers "send the work to Codex so we can keep talking." Don't wait around for Codex to finish. Fire it in background, then continue the conversation about architecture, strategy, investor pitches, or next phases. The `notify_on_complete` flag signals when to review, not when to stop talking.
