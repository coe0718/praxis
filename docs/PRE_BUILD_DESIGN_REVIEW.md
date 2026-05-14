# Praxis — Pre-Build Design Review

> Written May 14, 2026. Review of architecture decisions before building begins.
> Questions whether all of these are already addressed in the current codebase.

---

## 1. The agent loop is too Git-centric

Axonix inherited its loop from yoagent: wake → read context → prompt Claude → act → commit → sleep. That works for a self-evolving coding agent, but Praxis is meant for everyone — not just developers. Some users won't have a Git repo. Some won't want commits.

The loop needs to be more abstract:

```
wake → orient → decide → act → reflect → sleep
```

Where "act" is configurable — committing code, sending a message, calling an API, updating a file, or just logging. The loop shouldn't assume Git.

---

## 2. The context window problem

Right now Axonix solves this crudely: inject the last 3 journal entries, filter test output, hope for the best. But as Praxis instances get older and accumulate memories, goals, journal entries, and observations, the session prompt gets heavier and heavier. Nobody has really solved "what do you inject at session start?" well yet.

Worth designing a proper context budget system upfront:

- Hard token ceiling per session (configurable)
- Priority queue: identity > active goals > recent memory > predictions > journal
- Automatic truncation with a summary when budget is exceeded
- The agent knows its own budget and can request more context mid-session

---

## 3. Skills / tools system

ClaudeClaw has a skills directory — reusable prompt templates the agent can invoke. Axonix has something similar. But neither has a proper tool system where the agent can call external APIs, run scripts, or interact with services as structured function calls.

Praxis should have a first-class plugin/tool system from day one:

```toml
[[tool]]
name = "weather"
type = "http"
endpoint = "https://wttr.in/{location}?format=j1"

[[tool]]
name = "run_script"
type = "shell"
path = "./scripts/my_tool.sh"
security_level = 3
```

The agent discovers available tools from config, knows what they do, and calls them. This is what makes Praxis genuinely extensible without touching Rust code.

---

## 4. Multi-agent / sub-agent support

Axonix already does this crudely with `/run` spawning a mini-session. But Praxis should think about this properly: a primary agent that can spawn specialized sub-agents for specific tasks — a research agent, a coding agent, a summarization agent — each with their own context budget and tool access. The orchestrator pattern. This is how you get past the single-agent context ceiling on complex tasks.

---

## 5. The identity bootstrapping problem

We talked about the install interview. But there's a deeper question: when does the agent's identity stop being what you gave it and start being what it made for itself?

Praxis needs a clear policy:

- Which files can the agent rewrite freely? (GOALS.md, JOURNAL.md — yes)
- Which files need operator approval before rewriting? (IDENTITY.md, ROADMAP.md — maybe)
- Which files are locked entirely? (praxis.toml, security config — yes)

Without this, a Praxis instance could gradually drift away from what the user actually wanted — and the user might not notice until it's very different.

---

## 6. Observability that isn't just a dashboard

The dashboard is great for humans watching. But Praxis needs structured logging that the *agent itself* can query — "how have my session lengths trended?" "which goals take the most sessions to complete?" "when do I tend to get stuck?"

This is meta-cognition — the agent understanding its own performance patterns, not just recording them. Axonix has METRICS.md, but the agent can't reason over it systematically.

---

## 7. The upgrade problem

If Praxis ships as a single binary and someone is running it, how do they get updates? The agent can't upgrade itself without risking breaking the very process managing it. Need a proper update strategy — a separate watchdog process, a blue/green swap, or pinned versions with explicit user-triggered upgrades. Sounds boring, but it's a real problem nobody thinks about until v0.2 ships.

---

## 8. Privacy by default

The dashboard is public-facing. Axonix's is intentionally public because you want people watching. But most Praxis users won't want that. The framework should default to private (localhost only, auth required) with opt-in public visibility. The install script should ask: "Do you want your dashboard to be public?" and explain what that means before enabling it.

---

## Priority ranking

Of those eight, items **3 (tools system)**, **5 (identity policy)**, and **2 (context budget)** are the hardest to retrofit later. Get those wrong in the design and you're rewriting core systems at v0.5 when people are already running instances.
