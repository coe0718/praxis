# Score

> Per-session irreplaceability score — a four-dimension composite metric quantifying how much value the agent delivers.

## Overview

The `score` module computes a per-session "irreplaceability score" that measures how much the operator would lose if the agent were replaced with a naive assistant. The score is a weighted average of four dimensions: **anticipation** (proactive accuracy), **follow-through** (goal completion), **reliability** (tool approval pass rate), and **operator independence** (absence of manual intervention).

Scores are appended as JSONL to `score.jsonl`, auto-pruned to 365 records, and exposed via `praxis status`, `praxis insights`, and the morning brief. A rolling composite average is available for trend analysis.

## Architecture

### Types

| Type | Description |
|------|-------------|
| `ScoreWeights` | Weight per dimension: `anticipation` (0.20), `follow_through` (0.40), `reliability` (0.25), `operator_independence` (0.15). |
| `SessionScoreInput` | Raw counts collected during a session before normalisation. |
| `SessionScore` | Fully computed score with per-dimension values [0.0, 1.0] and a weighted composite. |

### Dimension Details

| Dimension | Input | Normalisation | Neutral default |
|-----------|-------|---------------|-----------------|
| Anticipation | `proactive_wake_hits / proactive_wakes_total` | Ratio | 0.5 (no proactive wakes) |
| Follow-through | `goal_completed / goal_was_selected` | Binary (1.0 or 0.0) | 0.5 (no goal selected = maintenance session) |
| Reliability | `approvals_passed / approvals_total` | Ratio | 1.0 (no approvals needed = fully autonomous) |
| Operator independence | `!operator_intervened` | Binary | — |

### Composite Formula

```
composite = (anticipation * w₁ + follow_through * w₂ + reliability * w₃ + independence * w₄) / total_weight
```

Default weights prioritize follow-through (40%) as the most important signal.

## Public API

```rust
pub struct SessionScore {
    pub session_id: Option<i64>,
    pub recorded_at: DateTime<Utc>,
    pub anticipation: f64,
    pub follow_through: f64,
    pub reliability: f64,
    pub operator_independence: f64,
    pub composite: f64,
    pub weights: ScoreWeights,
}

impl SessionScore {
    pub fn compute(input: &SessionScoreInput, weights: &ScoreWeights) -> Self;
    pub fn with_session_id(mut self, id: i64) -> Self;
    pub fn summary_line(&self) -> String;
}

// I/O
pub fn record_score(path: &Path, score: &SessionScore) -> Result<()>;
pub fn load_recent_scores(path: &Path, limit: usize) -> Result<Vec<SessionScore>>;
pub fn rolling_composite(path: &Path, limit: usize) -> Option<f64>;
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `anticipation` weight | 0.20 | Weight of proactive accuracy in composite. |
| `follow_through` weight | 0.40 | Weight of goal completion (highest priority). |
| `reliability` weight | 0.25 | Weight of tool approval pass rate. |
| `operator_independence` weight | 0.15 | Weight of operator non-intervention. |
| `MAX_RECORDS` | 365 | Maximum score records retained in the JSONL file. |

Weights are defined in `ScoreWeights::default()`. Custom weights can be provided when calling `SessionScore::compute()`.

## Usage

Scores are computed automatically during the Reflect phase. The operator can view trends:

```bash
# See the latest score in status output
praxis status

# View rolling trends and token usage
praxis insights

# The morning brief includes the rolling composite
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `score.jsonl` | JSONL | Per-session score records, up to 365 entries, auto-pruned. |

## Dependencies

- `paths` — `PraxisPaths` for the score file location
- `chrono`, `serde`, `anyhow`

## Source

`src/score.rs`
