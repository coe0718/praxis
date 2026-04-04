# Agent Runtime Boundary

Praxis owns the agent runtime contract through the `AgentBackend` trait in
`src/backend/mod.rs`.

That contract is intentionally narrower than any specific provider SDK, agent
framework crate, or orchestration runtime:

- `answer_prompt(...)` handles lightweight stateless asks
- `plan_action(...)` handles Decide-phase planning
- `finalize_action(...)` handles Act-phase refinement

## Why This Boundary Exists

Praxis is meant to be long-lived. Any external dependency that tries to become
the "real" runtime is a risk if it becomes abandoned, changes semantics, or
starts shaping the operator relationship in ways Praxis cannot control.

The replacement rule is simple:

- external runtimes are adapters
- Praxis owns the session model
- Praxis owns state, memory, tools, approvals, quality gates, and loop policy
- Praxis can replace any backend without changing those core systems

## Current Implementations

- `StubBackend` for deterministic offline runs and tests
- `ConfiguredBackend` for routed provider-backed execution
- provider-specific adapters under `src/backend/claude.rs`,
  `src/backend/openai.rs`, and `src/backend/ollama.rs`

## Replacement Plan

If a future dependency such as `yoagent` or another orchestration crate is
introduced, it must remain behind `AgentBackend` or a Praxis-owned successor
trait with the same property: the rest of the runtime must not need to know.

If a backend dependency is abandoned:

1. freeze the affected adapter behind a feature flag or config guard
2. keep `stub` and other adapters operational
3. replace only the adapter layer, not the session loop
4. preserve compatibility for stored state and operator-facing commands

## Non-Negotiable Ownership

The following stay Praxis-owned even if adapters change:

- loop phases and resumable session state
- tool approval and execution policy
- memory and operational learning
- eval and review gates
- analytics, forensics, and audit exports
- operator command surfaces
