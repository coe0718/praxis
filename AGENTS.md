# Praxis — Agent Working Conventions

This file is written for AI agents (Drey, Codex, etc.) contributing to Praxis.
CLAUDE.md has the full architecture — this is your commit-ready ruleset.

---

## 1. File Size Cap

**No source file over 250 lines unless absolutely unavoidable.**

- If a file hits 250 lines, split it.
- Modules with clearly separable concerns get their own file.
- Utility blocks, enum variants, and test helpers are the easiest extraction targets.
- If you genuinely can't split (e.g., a large match on a core enum), add a `// reason: ...` comment at the top justifying it.
- `#[allow(dead_code)]` is NOT a valid reason to keep a bloated file. Remove dead code or mark it for removal.

*Violations will be caught in review. Don't push 400-line files.*

---

## 2. Pre-Commit Checklist

**Run ALL of these before every commit:**

```bash
# 1. Format
cargo fmt

# 2. Lint — zero warnings, zero errors
cargo clippy -- -D warnings

# 3. Build — release if applicable
cargo check

# 4. Test — full suite
cargo test
```

- **Never** skip steps. "I'll fix clippy in the next PR" is not acceptable.
- If tests fail, fix them before committing. Don't push failing tests with "WIP" as an excuse.
- If a test takes too long, ask before skipping it in a commit.

---

## 3. Dead Code

- **No `#[allow(dead_code)]`.** Dead code is debt. Delete it or add a todo to wire it.
  *Exception: feature stubs planned for the next sprint, with a tracking issue/comment.*
- Each `#[allow(dead_code)]` remaining in the codebase is a review flag. Be prepared to justify it.

---

## 4. Error Handling

- `anyhow::Result` + `.context()` everywhere. No bare `unwrap()` in production paths.
- Use `if let Err(e) = ... { log::warn!(...) }` for non-fatal side effects in reflect/hooks.

---

## 5. Architecture Rules

- `PraxisRuntime` is generic — don't call `SqliteSessionStore` methods on it directly. Construct a transient `SqliteSessionStore::new(self.paths.database_file.clone())` when needed.
- Adding a field to `ToolManifest`? Grep every struct literal and update them. Miss one, test fails.
- Tests go in `tests/` as integration tests (tmp data dirs). In-file `#[cfg(test)]` for unit tests only.

---

## 6. Agent Conduct

- Read CLAUDE.md before touching unfamiliar modules — it has the full architecture map.
- If unsure about an approach, ask Tuck before writing code. Changes are cheaper as text than as commits.
- Prefer small focused PRs over one massive refactor.
- Migrate deprecated patterns (Rust edition bumps, renamed APIs) when you touch a file — don't leave the file worse than you found it.

---

*This file is the source of truth for agent behavior. If CLAUDE.md contradicts AGENTS.md, AGENTS.md wins for conventions.*
