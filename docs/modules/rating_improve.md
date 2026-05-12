# Rating Improvement (rating_improve)

> Self-improvement from user ratings — the agent collects star ratings and feedback to adjust behaviour and improve performance.

## Overview

The rating_improve module provides a `RatingProcessor` that collects `UserRating` submissions (1–5 stars with optional feedback and session context) and derives behavioural adjustments. It computes per-task-type averages, an overall average, an adjustment factor (mapped from the 1–5 rating scale to a 0.5–1.5 multiplier), and identifies low-performing task types that need improvement.

This enables a closed feedback loop: the operator rates agent interactions, and the agent adjusts its behaviour (e.g., confidence thresholds, verbosity, tool selection) based on the aggregate ratings.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `UserRating` | A single rating (1–5 stars) with optional feedback, timestamp, and context. |
| `RatingContext` | Context metadata: session ID, task type, and tools used. |
| `RatingProcessor` | Collects ratings and computes averages, adjustment factors, and improvement targets. |

### Relationships

`RatingProcessor` owns a `Vec<UserRating>` and an internal `HashMap<String, f32>` of adjustment rules. Ratings are appended and analysed on demand.

## Public API

### `UserRating`

```rust
pub struct UserRating {
    pub rating: i32,
    pub feedback: Option<String>,
    pub timestamp: i64,
    pub context: RatingContext,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `rating` | `i32` | Star rating 1–5. |
| `feedback` | `Option<String>` | Optional free-text feedback. |
| `timestamp` | `i64` | Unix timestamp of the rating. |
| `context` | `RatingContext` | Session and task metadata. |

### `RatingContext`

```rust
pub struct RatingContext {
    pub session_id: String,
    pub task_type: String,
    pub tools_used: Vec<String>,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `String` | The session this rating applies to. |
| `task_type` | `String` | The type/category of task being rated. |
| `tools_used` | `Vec<String>` | Tools that were invoked during the session. |

### `RatingProcessor`

```rust
impl RatingProcessor {
    pub fn new() -> Self
    pub fn record(&mut self, rating: UserRating)
    pub fn avg_for_task(&self, task_type: &str) -> Option<f32>
    pub fn overall_average(&self) -> f32
    pub fn adjustment_factor(&self) -> f32
    pub fn needs_improvement(&self, threshold: f32) -> Vec<String>
}
```

- **`new`** — Creates an empty `RatingProcessor`.
- **`record`** — Appends a new rating to the collection.
- **`avg_for_task`** — Returns the average rating for a specific task type, or `None` if none exist.
- **`overall_average`** — Returns the average across all ratings (0.0 if none recorded).
- **`adjustment_factor`** — Maps the overall average (1–5) to a behavioural adjustment factor (0.5–1.5) using the formula: `0.5 + (avg - 1.0) / 4.0`.
- **`needs_improvement`** — Returns a list of task types whose average rating falls below `threshold`. Deduplicates by task type name.

### Adjustment Factor Mapping

| Avg Rating | Adjustment Factor |
|------------|------------------|
| 1.0 | 0.50 |
| 2.0 | 0.75 |
| 3.0 | 1.00 (neutral) |
| 4.0 | 1.25 |
| 5.0 | 1.50 |

## Configuration

No configuration fields. The `RatingProcessor` is instantiated in-memory and populated programmatically.

## Usage

```rust
use praxis::rating_improve::{RatingProcessor, UserRating, RatingContext};

let mut processor = RatingProcessor::new();

processor.record(UserRating {
    rating: 4,
    feedback: Some("Good response time".into()),
    timestamp: 1747000000,
    context: RatingContext {
        session_id: "session_001".into(),
        task_type: "code_review".into(),
        tools_used: vec!["file-read".into(), "git-query".into()],
    },
});

let avg = processor.overall_average();
let factor = processor.adjustment_factor();
let needs_work = processor.needs_improvement(3.0);

println!("Overall: {avg:.1}, Factor: {factor:.2}");
```

## Data Files

None. Ratings are held in-memory. For persistence, the `Vec<UserRating>` should be serialised to a file (e.g., `ratings.jsonl`) by the caller.

## Dependencies

- **`serde`** — Serialization for `UserRating` and `RatingContext`.

## Source

`src/rating_improve.rs`