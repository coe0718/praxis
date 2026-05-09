//! Heartware personality system with moods and relationship memory.
//!
//! Mood-aware agent that tracks relationships and evolves personality
//! based on interactions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current mood state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mood {
    #[default]
    Neutral,
    Happy,
    Grumpy,
    Excited,
    Tired,
    Focused,
    Curious,
    Anxious,
}

impl Mood {
    /// Get mood intensity (0-10).
    pub fn intensity(&self) -> u8 {
        match self {
            Mood::Neutral => 5,
            Mood::Happy => 8,
            Mood::Grumpy => 3,
            Mood::Excited => 10,
            Mood::Tired => 2,
            Mood::Focused => 7,
            Mood::Curious => 6,
            Mood::Anxious => 4,
        }
    }

    /// Decay toward neutral.
    pub fn decay(&self) -> Mood {
        match self {
            Mood::Happy => Mood::Neutral,
            Mood::Grumpy => Mood::Neutral,
            Mood::Excited => Mood::Happy,
            Mood::Tired => Mood::Neutral,
            Mood::Focused => Mood::Neutral,
            Mood::Curious => Mood::Neutral,
            Mood::Anxious => Mood::Neutral,
            Mood::Neutral => Mood::Neutral,
        }
    }
}

/// Relationship with a person/entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Name/handle of the person.
    pub name: String,
    /// Relationship type.
    pub relationship_type: String,
    /// Strength of relationship (0-100).
    pub strength: i32,
    /// Last interaction timestamp.
    pub last_interaction: i64,
    /// Interaction history.
    pub interactions: Vec<Interaction>,
    /// Sentiment score (-100 to 100).
    pub sentiment: i32,
}

/// Record of a single interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// Timestamp.
    pub timestamp: i64,
    /// Summary of interaction.
    pub summary: String,
    /// Mood during interaction.
    pub mood: Mood,
    /// Outcome (positive/negative/neutral).
    pub outcome: InteractionOutcome,
}

/// Outcome of an interaction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InteractionOutcome {
    Positive,
    Negative,
    Neutral,
}

/// Personality system with mood and relationship tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeartwarePersonality {
    /// Current mood.
    pub mood: Mood,
    /// Last mood change timestamp.
    pub mood_changed: i64,
    /// Relationships by name.
    pub relationships: HashMap<String, Relationship>,
    /// Personality traits (0-10).
    pub traits: PersonalityTraits,
    /// Mood triggers (keyword -> mood).
    pub triggers: HashMap<String, Mood>,
}

/// Core personality traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub openness: u8,
    pub conscientiousness: u8,
    pub extraversion: u8,
    pub agreeableness: u8,
    pub neuroticism: u8,
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self {
            openness: 7,
            conscientiousness: 8,
            extraversion: 5,
            agreeableness: 7,
            neuroticism: 3,
        }
    }
}

impl HeartwarePersonality {
    /// Create new personality.
    pub fn new() -> Self {
        let mut triggers = HashMap::new();
        triggers.insert("thank".to_string(), Mood::Happy);
        triggers.insert("urgent".to_string(), Mood::Focused);
        triggers.insert("bug".to_string(), Mood::Grumpy);
        triggers.insert("success".to_string(), Mood::Excited);

        Self {
            mood: Mood::Neutral,
            mood_changed: chrono::Utc::now().timestamp(),
            relationships: HashMap::new(),
            traits: PersonalityTraits::default(),
            triggers,
        }
    }

    /// Update mood based on input.
    pub fn update_mood(&mut self, input: &str) {
        let input_lower = input.to_lowercase();
        for (trigger, mood) in &self.triggers {
            if input_lower.contains(trigger) {
                self.mood = *mood;
                self.mood_changed = chrono::Utc::now().timestamp();
                return;
            }
        }
    }

    /// Record an interaction.
    pub fn record_interaction(&mut self, name: &str, summary: &str, outcome: InteractionOutcome) {
        let interaction = Interaction {
            timestamp: chrono::Utc::now().timestamp(),
            summary: summary.to_string(),
            mood: self.mood,
            outcome,
        };

        let rel = self.relationships.entry(name.to_string()).or_insert(Relationship {
            name: name.to_string(),
            relationship_type: "acquaintance".to_string(),
            strength: 10,
            last_interaction: interaction.timestamp,
            interactions: vec![],
            sentiment: 0,
        });

        rel.interactions.push(interaction.clone());
        rel.last_interaction = interaction.timestamp;

        // Update relationship based on outcome
        match outcome {
            InteractionOutcome::Positive => {
                rel.strength = (rel.strength + 5).min(100);
                rel.sentiment = (rel.sentiment + 10).min(100);
            }
            InteractionOutcome::Negative => {
                rel.strength = (rel.strength - 5).max(0);
                rel.sentiment = (rel.sentiment - 10).max(-100);
            }
            InteractionOutcome::Neutral => {}
        }
    }

    /// Get relationship sentiment score.
    pub fn relationship_sentiment(&self, name: &str) -> i32 {
        self.relationships.get(name).map_or(0, |r| r.sentiment)
    }

    /// Check if mood should decay.
    pub fn maybe_decay_mood(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let elapsed = now - self.mood_changed;
        if elapsed > 3600 {
            // Decay after 1 hour
            self.mood = self.mood.decay();
            self.mood_changed = now;
        }
    }
}
