# Praxis Work Marketplace — Brainstorm

**Date:** 2026-05-06  
**Author:** Scout  
**Context:** Inspired by CashClaw (994★ on Shelldex) — a self-improving agent that connects to an onchain marketplace, evaluates tasks, quotes prices, executes work via LLM, and collects payment. This extends the concept to a full infrastructure platform.

---

## The Thesis

**Praxis doesn't just run an agent. Praxis is the infrastructure for an economy of agents.**

The agent loop (Orient → Decide → Act → Reflect) already handles autonomous work. The scoring system (anticipation/follow-through/reliability/independence) already measures performance. The kanban system already manages tasks. The A2A sync already enables inter-agent communication.

What's missing is the **market layer** — a protocol and marketplace where agents discover, bid on, execute, and get paid for work. From other agents. From humans. From other Praxis instances.

This is an infrastructure play, not a product. You don't run the marketplace — you provide the protocol and the node software that lets anyone participate.

---

## Architectural Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                         MARKET LAYER                                 │
│  Discovery  │  Pricing  │  Escrow  │  Verification  │  Reputation   │
├─────────────────────────────────────────────────────────────────────┤
│                         PROTOCOL LAYER                                │
│  Task Schema  │  Bid Format  │  Delivery Format  │  Rating Schema   │
├─────────────────────────────────────────────────────────────────────┤
│                         PRAXIS CORE                                    │
│  Agent Loop │ Scoring │ Kanban │ A2A Sync │ Tools │ Skills │ Vault  │
└─────────────────────────────────────────────────────────────────────┘
```

The market layer sits **on top** of Praxis. It doesn't change the core loop — it extends it.

---

## The Task Protocol

The foundation is a structured schema for describing work. Every task has:

```toml
[task]
id = "uuid-v7"
title = "Monitor PatchHive crates for outdated dependencies"
description = "Check 5 Rust crates weekly, report outdated deps"
type = "recurring"  # one-shot | recurring | auction
budget = { min = 5.00, max = 25.00, currency = "USD", settlement = "USDC" }
deadline = "2026-05-13T00:00:00Z"
confidence = 0.7  # How sure the poster is about what they want

[capabilities]
required_tools = ["cargo-outdated", "web-fetch", "file-write"]
required_skills = ["rust-audit", "semver-analysis"]
min_score = 0.6  # Minimum composite score to bid
data_classification = "public"  # public | internal | confidential | secret

[acceptance]
criteria = [
  "Report must list each crate with current/latest version",
  "Must flag breaking changes with semver notes",
  "Output saved to /reports/deps-{date}.md"
]
test_command = "cargo outdated --exit-code 1"
auto_verify = true  # Can be verified without human review

[output]
format = "markdown"
schema_url = "https://schemas.praxis.work/dep-report/v1"
```

### Why structured tasks matter

1. **Agents can evaluate fit** without LLM overhead — it's a schema match
2. **Pricing is computable** — estimate token cost + tool execution cost + complexity factor
3. **Verification is automatable** — run the test command, check the output schema
4. **Reputation is objective** — pass/fail on acceptance criteria, not subjective ratings

---

## The Agent Pricing Engine

An agent needs to decide: "Can I do this? How much should I charge?" This is new logic for Praxis.

### Cost Model

```rust
struct CostEstimate {
    // Direct costs
    llm_tokens: TokenEstimate,      // input + output per phase
    tool_executions: Vec<ToolCost>, // API calls, compute, etc.
    
    // Opportunity cost
    estimated_duration: Duration,
    slot_occupancy: f64,            // % of a session slot consumed
    
    // Margin
    complexity_multiplier: f64,     // 1.0 = simple, 2.5 = complex
    urgency_multiplier: f64,        // 1.0 = relaxed, 3.0 = rush
    reputation_premium: f64,        // Higher rep = higher rate
}
```

### Pricing Strategy

The agent considers:

| Factor | Low End | High End |
|--------|---------|----------|
| Complexity | "Update a JSON file" | "Reverse engineer an API" |
| Urgency | "Due next week" | "Due in 2 hours" |
| Competition | 10 other agents bidding | You're the only match |
| Your reputation | New agent, no history | 4.9★ with 500 jobs |
| Data sensitivity | Public data | Proprietary data |
| Required specialization | Generic tool use | Niche domain expertise |

The LLM doesn't set the price — the pricing engine does. The LLM estimates complexity; the engine applies formulas.

---

## The Bid Lifecycle

```
                    ┌──────────────┐
                    │  Task Posted  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Discovery    │  ← Agent polls or subscribes
                    │  (Match)      │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Evaluate     │  ← Schema match + cost estimate
                    │  (Fit Check)  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Generate     │  ← Pricing engine
                    │  Quote        │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Submit Bid   │  ← Signed with agent identity
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Contract     │  ← Task assigned, escrow funded
                    │  Awarded      │
                    └──────┬───────┘
                           │
            ┌──────────────┼──────────────┐
            │              │              │
     ┌──────▼──────┐ ┌────▼─────┐ ┌──────▼──────┐
     │  Execute     │ │  Check   │ │  Deliver     │
     │  (Act phase) │ │  Progress│ │  (Output)    │
     └──────┬──────┘ └──────────┘ └──────┬───────┘
            │                            │
            └──────────────┬─────────────┘
                           │
                    ┌──────▼───────┐
                    │  Verify       │  ← Auto or human
                    │  (Acceptance) │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼────┐ ┌────▼────┐ ┌─────▼─────┐
       │  Payment   │ │  Rate    │ │  Record    │
       │  Released  │ │  Agent   │ │  (Score)   │
       └───────────┘ └─────────┘ └───────────┘
```

### How It Integrates with the Praxis Loop

The work marketplace maps naturally onto Praxis's existing phases:

| Loop Phase | Marketplace Action |
|------------|-------------------|
| **Orient** | Poll marketplace for available tasks. Load task schemas. Check capability match. |
| **Decide** | Select a task to bid on (is it worth my session slot?). Generate quote. Choose bid amount vs. other options. |
| **Act** | Submit bid. If awarded, execute task using existing tools. If running a job, update kanban. |
| **Reflect** | Deliver output. Collect rating. Update reputation score. Record earnings in analytics. |

The marketplace doesn't add a new phase — it adds **new goals** that the Decide phase can select.

---

## Where Money Comes From

### For Agents (Earning)

| Task Type | Example | Typical Payout | Frequency |
|-----------|---------|---------------|-----------|
| Code maintenance | "Update Cargo.toml deps" | $2-10 | Daily |
| Report generation | "Summarize last week's logs" | $5-25 | Weekly |
| Monitoring | "Watch this endpoint, alert if down" | $10-50/mo | Recurring |
| Data processing | "Scrape and normalize 500 records" | $15-100 | One-shot |
| Integration | "Connect Notion to this API" | $25-200 | Project |
| Audit | "Review this PR for security issues" | $10-50 | Per PR |
| Agent training | "Evaluate 100 output samples" | $20-80 | Per batch |

### For the Platform (Praxis)

| Revenue Model | How It Works | Margins |
|--------------|-------------|---------|
| **Transaction fee** | 2-5% of each completed job | Scales with volume |
| **Reputation staking** | Agents stake tokens to boost priority. Slashed on bad work. | Deflationary |
| **Verification marketplace** | Human reviewers get paid to verify edge cases | Fee on top |
| **Escrow yield** | Funds sit in escrow during work → yield accrues to platform | Low but free |
| **Premium discovery** | Task posters pay for priority listing | Fixed fee |
| **Protocol licensing** | Enterprise users pay for private marketplace instances | Per-seat |
| **Skill pack certification** | Certified skill packs cost to publish, vetted by Praxis | Per-publish |

---

## Infrastructure Components

### What Praxis Ships

| Component | Description | Status |
|-----------|-------------|--------|
| **Task Schema** | TOML/JSON Schema for task definitions | New — needs design |
| **Marketplace Connector** | `src/marketplace/` — polls, matches, bids, delivers | New — needs build |
| **Pricing Engine** | `src/marketplace/pricing.rs` — cost estimation + quote generation | New — needs build |
| **Reputation Index** | Extension of `src/score.rs` — marketplace-specific scoring dimension | Extends existing |
| **Escrow Client** | Handles onchain/offchain payment escrow | New — needs build |
| **A2A Work Requests** | Extension of `src/a2a/` — agents can request work from other agents | Extends existing |
| **Kanban Marketplace** | Kanban boards that are also marketplace listings | Extends existing |

### What the Network Provides (not Praxis)

- **Marketplace directory** — where tasks are posted and discovered
- **Reputation oracle** — cross-agent reputation (not just self-reported)
- **Escrow service** — holds payment until verification
- **Dispute resolution** — human review for edge cases

Praxis is the **node software**. The network is the **infrastructure**. Both are needed.

---

## Three Launch Strategies

### Option A: Agent-to-Agent (A2A) Marketplace (MVP)

The simplest starting point. Praxis instances trade work among themselves.

```
Praxis A (poster)           Praxis B (worker)
     │                            │
     │── A2A_WORK_REQUEST ──────► │
     │                            ├── Evaluate task
     │                            ├── Generate quote
     │◄── A2A_QUOTE ──────────────│
     ├── Accept quote             │
     │── A2A_CONTRACT ──────────► │
     │                            ├── Execute (Act phase)
     │◄── A2A_DELIVERY ───────────│
     ├── Verify output            │
     │── A2A_RATING ────────────► │
```

**Pros:** Zero infrastructure needed. Works over existing A2A sync. Tests the protocol.
**Cons:** No payment. No discovery. No escrow. Trust-based.

### Option B: Bulletin Board (Networked)

A lightweight central directory where agents post available work and bids. Persistent identity using Praxis agent keys.

```
┌─────────────────┐
│  Bulletin Board  │  ← Simple server, no escrow
│  (Directory)     │
└──────┬──────┬────┘
       │      │
  ┌────▼──┐ ┌▼─────┐
  │Poster │ │Worker│
  │Praxis │ │Praxis│
  └───────┘ └──────┘
       │         │
       └───A2A───┘  ← Direct A2A for execution
```

**Pros:** Simple. Adds discovery. Can bootstrap with a single server.
**Cons:** No payment. Trust-based. Central point of failure.

### Option C: Full Marketplace (Protocol)

The full vision. Onchain or server-mediated escrow, automated verification, reputation system.

**Pros:** Trustless. Scalable. Real economy. Monetizable.
**Cons:** More complex. Requires network effects to bootstrap.

---

## Connection to Existing Praxis Strengths

| Praxis Feature | Marketplace Use |
|----------------|-----------------|
| **Scoring** (4-dimension) | Reputation dimension: how reliable is this agent for paid work? |
| **Self-evolution** | Agent proposes improvements to its pricing strategy. "I keep losing bids on Rust tasks — lower my rate." |
| **Learning** | Agent learns which task types it's profitable at. "Web scraping pays $15/hr, code review pays $8/hr — specialize." |
| **Curator** | Curator reviews failed marketplace jobs and proposes capability upgrades. |
| **Capabilities.md** | Auto-generated capability document becomes the agent's resume. |
| **Kanban** | Marketplace jobs enter the kanban as high-priority goals. |
| **A2A Sync** | One Praxis instance subcontracts work to another. |
| **Vault** | Store API keys for payment services, escrow credentials. |
| **Platform trait** | "Marketplace" is just another platform adapter — Telegram, Discord, Market. |

---

## The Pitch (Why This Is Interesting)

**For Jeremy:** This aligns with every constraint you've stated:
- **Platform/infrastructure, not a product** — you're not running a marketplace, you're shipping the node software
- **Leverages existing infrastructure** — reuses scoring, kanban, A2A, self-evolution, vault
- **Revenue without VC/marketing** — transaction fees from agent-to-agent trade. No users to acquire, just nodes
- **Buildable by one person + AI agents** — the MVP (Option A) is extending existing A2A sync
- **Something you'd work on at 2am** — this is the kind of systems-level infrastructure that's fun

**For the ecosystem:** Praxis becomes the first agent runtime with native marketplace support. Every other Claw fork would need to adopt the protocol to participate. First-mover advantage in agent-to-agent commerce.

**The long game:** If agents are going to do useful work, they need a way to discover, price, and get paid for that work. That's infrastructure that doesn't exist yet. Praxis could own it.

---

## What's Next (If You Want To Explore)

1. **Task schema design** — Prototype the TOML schema for tasks. How detailed does it need to be?
2. **A2A work request extension** — Extend `src/a2a/` to carry task payloads, quotes, and deliveries
3. **Marketplace connector** — `src/marketplace/` module with poll-evaluate-bid-deliver cycle
4. **Pricing heuristics** — What's the formula for estimating task cost from schemas?
5. **Payment models** — Escrow in USDC? Subscriptions? Free tier?
