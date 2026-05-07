//! Self-Improvement from User Ratings.
//!
//! Agent collects star ratings and feedback to adjust behavior and improve performance.

use serde::{Deserialize, Serialize};

/// A user rating for an agent interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRating {
    pub rating: i32, // 1-5 stars
    pub feedback: Option<String>,
    pub timestamp: i64,
    pub context: RatingContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingContext {
    pub session_id: String,
    pub task_type: String,
    pub tools_used: Vec<String>,
}

/// Rating processor that adjusts agent behavior based on feedback.
pub struct RatingProcessor {
    ratings: Vec<UserRating>,
    adjustment_rules: std::collections::HashMap<String, f32>,
}

impl RatingProcessor {
    pub fn new() -> Self {
        Self {
            ratings: Vec::new(),
            adjustment_rules: std::collections::HashMap::new(),
        }
    }

    /// Record a new user rating.
    pub fn record(&mut self, rating: UserRating) {
        self.ratings.push(rating);
    }

    /// Calculate average rating for a task type.
    pub fn avg_for_task(&self, task_type: &str) -> Option<f32> {
        let relevant: Vec<&UserRating> = self
            .ratings
            .iter()
            .filter(|r| r.context.task_type == task_type)
            .collect();

        if relevant.is_empty() {
            return None;
        }

        let sum: i32 = relevant.iter().map(|r| r.rating).sum();
        Some(sum as f32 / relevant.len() as f32)
    }

    /// Get overall average rating.
    pub fn overall_average(&self) -> f32 {
        if self.ratings.is_empty() {
            return 0.0;
        }
        let sum: i32 = self.ratings.iter().map(|r| r.rating).sum();
        sum as f32 / self.ratings.len() as f32
    }

    /// Get behavioral adjustment factor based on ratings.
    pub fn adjustment_factor(&self) -> f32 {
        let avg = self.overall_average();
        // Map 1-5 stars to adjustment factor 0.5-1.5
        0.5 + (avg - 1.0) / 4.0
    }

    /// Get low-performing task types that need improvement.
    pub fn needs_improvement(&self, threshold: f32) -> Vec<String> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for r in &self.ratings {
            if !seen.insert(&r.context.task_type) {
                continue;
            }
            if let Some(avg) = self.avg_for_task(&r.context.task_type) {
                if avg < threshold {
                    result.push(r.context.task_type.clone());
                }
            }
        }
        result
    }
}

impl Default for RatingProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_average() {
        let mut processor = RatingProcessor::new();
        processor.record(UserRating {
            rating: 4,
            feedback: None,
            timestamp: 0,
            context: RatingContext {
                session_id: "s1".into(),
                task_type: "task_a".into(),
                tools_used: vec![],
            },
        });
        processor.record(UserRating {
            rating: 5,
            feedback: None,
            timestamp: 0,
            context: RatingContext {
                session_id: "s2".into(),
                task_type: "task_a".into(),
                tools_used: vec![],
            },
        });

        assert!((processor.overall_average() - 4.5).abs() < 0.01);
        assert!((processor.avg_for_task("task_a").unwrap() - 4.5).abs() < 0.01);
    }

    #[test]
    fn test_adjustment_factor() {
        let mut processor = RatingProcessor::new();
        processor.record(UserRating {
            rating: 4,
            feedback: None,
            timestamp: 0,
            context: RatingContext {
                session_id: "s1".into(),
                task_type: "task".into(),
                tools_used: vec![],
            },
        });
        let factor = processor.adjustment_factor();
        assert!(factor > 0.7 && factor < 1.3);
    }
}