# Curator

> Autonomous skill curator with 7-day grading cycles and rubric-based recommendations.

## Overview

The `curator` module is Praxis's self-maintenance system for its skill library. Running on a configurable cycle (default 7 days), it grades every installed skill against a weighted rubric, identifies candidates for pruning, consolidation, or promotion, and writes a structured report.

The grading rubric balances four dimensions: usage frequency (40%), age freshness (20%), quality score from prior evals (20%), and dependency count (20% — skills referenced by other skills score higher). Based on the composite score, the curator issues one of four recommendations: `Keep`, `RecommendPrune` (score < 0.3), `RecommendConsolidate` (similar skills detected), or `Promote` (score > 0.8).

## Architecture

### Types

| Type | Description |
|------|-------------|
| `CuratorConfig` | Configuration: `cycle_days` (default 7), `prune_threshold` (default 0.3), `auto_prune` (default false), `report_path`. |
| `SkillGrade` | Per-skill grading result: score, usage count, age in days, quality score, dependency count, recommendation, and reason string. |
| `GradeRecommendation` | Enum: `Keep`, `RecommendPrune`, `RecommendConsolidate { similar_to }`, `Promote`. |
| `CuratorReport` | Full cycle output: timestamp, total skills, all grades, and lists of prune/promote/consolidate candidates. |
| `Curator` | The main curator struct, holding config and `PraxisPaths`. |

### Curator Methods

| Method | Description |
|--------|-------------|
| `new()` | Construct from config and paths. |
| `run_cycle()` | Execute a full grading cycle: load catalog, grade each skill, save report. |
| `latest_report()` | Load the most recent report from disk. |
| `is_cycle_due()` | Check whether enough days have elapsed since the last run. |
| `mark_cycle_run()` | Write the current timestamp to mark a completed cycle. |

### Scoring Formula

```
score = min(usage_count * 0.1, 1.0) * 0.40  // Usage frequency (40%)
      + age_score                * 0.20       // Age freshness (20%)
      + quality_score            * 0.20       // Quality from evals (20%)
      + min(deps * 0.1, 1.0)    * 0.20       // Dependencies (20%)
```

Age score: `< 7 days = 1.0`, `< 30 days = 0.7`, `≥ 30 days = 0.4`.

## Public API

```rust
pub struct Curator { /* ... */ }
impl Curator {
    pub fn new(config: CuratorConfig, paths: &PraxisPaths) -> Self;
    pub fn run_cycle(&self) -> Result<CuratorReport>;
    pub fn latest_report(&self) -> Result<Option<CuratorReport>>;
    pub fn is_cycle_due(&self) -> Result<bool>;
    pub fn mark_cycle_run(&self) -> Result<()>;
}

pub struct CuratorConfig {
    pub cycle_days: u32,
    pub prune_threshold: f64,
    pub auto_prune: bool,
    pub report_path: PathBuf,
}
```

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `cycle_days` | 7 | Days between automatic curation cycles. |
| `prune_threshold` | 0.3 | Score below which a skill is recommended for pruning. |
| `auto_prune` | false | Whether to automatically delete prune candidates without operator approval. |

Configuration is typically embedded in `praxis.toml` or managed via the CLI.

## Usage

```bash
# Run the curation cycle manually
praxis curator run

# Check curation status and see ranked skills
praxis curator status

# View the latest report
cat data_dir/curator_report.json | jq .
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `curator_report.json` | JSON | Full grading report from the last cycle. |
| `curator_last_run.txt` | Plain text (Unix timestamp) | Tracks when the last cycle ran. |

## Dependencies

- `skills` — `load_catalog()` to enumerate installed skills
- `paths` — `PraxisPaths` for file locations

## Source

`src/curator/mod.rs`
