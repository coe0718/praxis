# Personality

> Heartware personality system with moods and relationship memory. A mood-aware agent that tracks relationships and evolves personality based on interactions.

## Overview

The `personality` module gives Praxis an emotional layer — the `HeartwarePersonality` system. It implements eight mood states (Neutral, Happy, Grumpy, Excited, Tired, Focused, Curious, Anxious) with configurable intensity levels and a decay mechanism that drifts strong moods back toward Neutral after one hour without reinforcing stimuli.

The module also maintains relationship memory per person/entity: relationship strength (0–100), sentiment score (−100 to +100), interaction history, and outcome-based updates (positive interactions boost strength/sentiment, negative ones reduce them). Mood can be triggered by keyword matching in input text.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `Mood` | Enum of eight mood states with `intensity()` (0–10) and `decay()` methods. Default: `Neutral`. |
| `Relationship` | Per-person record: `name`, `relationship_type`, `strength` (0–100), `last_interaction`, `interactions` history, `sentiment` (−100 to +100). |
| `Interaction` | Single interaction record: `timestamp`, `summary`, `mood`, `outcome`. |
| `InteractionOutcome` | Enum: `Positive`, `Negative`, `Neutral`. |
| `PersonalityTraits` | OCEAN model: `openness`, `conscientiousness`, `extraversion`, `agreeableness`, `neuroticism` (0–10 each). Defaults: 7, 8, 5, 7, 3. |
| `HeartwarePersonality` | Main system: `mood`, `mood_changed`, `relationships` map, `traits`, `triggers` (keyword→Mood). |

### Mood Triggers (Built-in)

| Keyword | Mood |
|---------|------|
| "thank" | Happy |
| "urgent" | Focused |
| "bug" | Grumpy |
| "success" | Excited |

### Mood Decay

After 1 hour (3600 seconds) of no mood-reinforcing input, the mood decays toward Neutral:
- Happy → Neutral
- Grumpy → Neutral
- Excited → Happy
- Tired → Neutral
- Focused → Neutral
- Curious → Neutral
- Anxious → Neutral
- Neutral → Neutral

## Public API

```rust
// Mood system
pub enum Mood {
    Neutral, Happy, Grumpy, Excited, Tired, Focused, Curious, Anxious,
}
impl Mood {
    pub fn intensity(&self) -> u8;
    pub fn decay(&self) -> Mood;
}

// Relationship tracking
pub struct Relationship {
    pub name: String,
    pub relationship_type: String,
    pub strength: i32,
    pub last_interaction: i64,
    pub interactions: Vec<Interaction>,
    pub sentiment: i32,
}

pub struct Interaction {
    pub timestamp: i64,
    pub summary: String,
    pub mood: Mood,
    pub outcome: InteractionOutcome,
}

pub enum InteractionOutcome {
    Positive, Negative, Neutral,
}

// Personality traits (OCEAN model)
pub struct PersonalityTraits {
    pub openness: u8,
    pub conscientiousness: u8,
    pub extraversion: u8,
    pub agreeableness: u8,
    pub neuroticism: u8,
}

// Main personality system
pub struct HeartwarePersonality {
    pub mood: Mood,
    pub mood_changed: i64,
    pub relationships: HashMap<String, Relationship>,
    pub traits: PersonalityTraits,
    pub triggers: HashMap<String, Mood>,
}
impl HeartwarePersonality {
    pub fn new() -> Self;
    pub fn update_mood(&mut self, input: &str);
    pub fn record_interaction(&mut self, name: &str, summary: &str, outcome: InteractionOutcome);
    pub fn relationship_sentiment(&self, name: &str) -> i32;
    pub fn maybe_decay_mood(&mut self);
}
```

## Configuration

No `praxis.toml` section. The personality is initialized in code with built-in mood triggers. Custom triggers can be added to `HeartwarePersonality::triggers` at runtime.

### Example

```rust
let mut personality = HeartwarePersonality::new();

// Mood reacts to input
personality.update_mood("There's a critical bug in production");
assert_eq!(personality.mood, Mood::Grumpy);

// Record interactions
personality.record_interaction("alice", "Deployed fix", InteractionOutcome::Positive);
personality.record_interaction("bob", "Missed deadline", InteractionOutcome::Negative);

// Check sentiment
let sentiment = personality.relationship_sentiment("alice");
// sentiment == 10 (started at 0, +10 for positive outcome)
```

## Traits (OCEAN Defaults)

| Trait | Default | Range |
|-------|---------|-------|
| Openness | 7 | 0–10 |
| Conscientiousness | 8 | 0–10 |
| Extraversion | 5 | 0–10 |
| Agreeableness | 7 | 0–10 |
| Neuroticism | 3 | 0–10 |

## Dependencies

- `chrono` — timestamps for mood change and interactions
- `serde` / `serde_json` — serialization

## Source

`src/personality.rs`