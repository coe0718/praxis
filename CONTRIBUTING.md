# Contributing

## Code Shape

- Keep Rust code files under 250 lines whenever practical.
- Split modules early instead of letting core files grow past the limit.
- Prefer small focused modules over large multi-responsibility files.

## Foundation Defaults

- Keep the first milestone fully offline and deterministic in tests.
- Preserve Docker support when adding new runtime behavior.
