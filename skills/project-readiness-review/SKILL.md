---
name: project-readiness-review
category: software-development
description: Assess a project's completion state and submission readiness by surveying structure, checking status docs, reviewing key files, and identifying blockers.
tags: [review, assessment, status, completion, blockers, submission, milestone]
---

# Project Readiness Review

Assess a project's completion state and readiness for submission, merge, or milestone delivery.

## Trigger Conditions

Use when:
- User asks "check this out" / "what do you think" about a project they've built
- Need to assess hackathon submission readiness
- Evaluating PR/milestone completion
- Reviewing deliverables against a deadline
- Identifying what's actually blocking ship

## Class Pattern (Not Project-Specific)

This skill applies to ANY project review — Web apps, Rust crates, ML pipelines, creative coding, etc. The pattern is:
1. **Survey structure** → understand layout and components
2. **Check status docs** → STATUS.md, TODO, CHANGELOG, or project tracking
3. **Review key files** → README, main entry points, configuration
4. **Assess completion** → what exists vs. what was promised
5. **Identify blockers** → actual remaining work vs. polish
6. **Deliver verdict** → honest assessment of readiness

## Workflow

### Step 1: Locate Project Files

```bash
# Quick directory survey
ls -la <project-path>/

# Find status documents
find <project-path> -maxdepth 2 -name "STATUS*" -o -name "TODO*" -o -name "ROADMAP*" 2>/dev/null
```

### Step 2: Read Status/TODO Files First

Priority order:
- `STATUS.md` — explicit completion checklist
- `TODO.md` / `TODO` — planned vs done
- `README.md` — stated features/goals
- `package.json`, `Cargo.toml`, `pyproject.toml` — dependencies and scripts

### Step 3: Survey Key Implementation Files

Read 2-3 representative files:
- Main entry point (server, app, lib)
- Core generator/module (the actual workhorse)
- Main UI/presentation file

### Step 4: Assess Against Stated Goals

Cross-reference:
| Source | Claims | Actual |
|--------|--------|--------|
| README features list | 10 items | Checkmarks |
| STATUS.md components | 25 items | Done/Remaining |
| Architecture diagram | 4 layers | Files match? |

### Step 5: Identify Real Blockers

Distinguish:
- **Hard blockers** — feature incomplete, broken, won't run
- **Soft blockers** — missing polish, needs deployment, expired keys
- **Nice-to-have** — optional enhancements

### Step 6: Deliver Verdict

Structure the response:
1. **Quick summary** — 1-2 sentences on overall state
2. **Architecture** — clean/messy, dependencies, structure
3. **Completion assessment** — X of Y components done
4. **Real blockers** — what's actually stopping ship
5. **Recommendation** — Proceed / Fix X first / Decline

## Common Patterns

### Web App / Creative Coding Project
Check:
- `README.md` → features list, quick start
- `STATUS.md` → explicit done/pending
- Entry point (`server.js`, `index.html`) → runs?
- Dependencies → in `package.json`, `requirements.txt`, or zero-dep?
- Docker → `Dockerfile` exists and builds?
- Landing/demo page → exists and renders?

### Rust/Python/Node Library
Check:
- Cargo.toml / setup.py / package.json → metadata correct?
- Tests → `tests/` or inline, do they pass?
- Documentation → examples in README?
- CI → `.github/workflows/` present?

### Multi-Agent Project
Check:
- Agent roster → all agents have profiles?
- Communication → can agents @mention each other?
- State → persistence working?
- Deployment → systemd services / cron jobs active?

## Pitfalls

1. **Don't trust README claims** — verify with file inspection
2. **Check .env.example vs actual** — expired keys are silent killers
3. **Distinguish "built" from "working"** — code-complete ≠ functional
4. **Deployment is a real task** — "just needs to deploy" is still blocked
5. **Assume expiry** — API keys, tokens, certs are probably stale

## When to Stop

Review is complete when you can answer:
- "Is this shippable today?" (Yes / No / With X fix)
- "What's the one blocker if stopped now?"
- "What would make this a "no" for submission?"

Beyond that, you're polishing. Stop.