//! Autonomous Curator — skill self-maintenance on a 7-day cycle.
//!
//! #6 Autonomous Curator (Hermes v0.12 headline feature).
//!
//! Background agent that grades, prunes, and consolidates the skill
//! library on a 7-day cycle. Per-run reports. `praxis curator status`
//! ranks skills by usage. Rubric-based grading. Scoped toolsets (memory + skills only).
//!
//! Grading rubric:
//!   - Usage frequency (40%) — how often invoked in recent sessions
//!   - Age (20%) — newer skills score higher (freshness)
//!   - Quality score (20%) — from previous eval results
//!   - Dependencies (20%) — skills referenced by other skills score higher
//!
//! Actions: recommend_prune (score < 0.3), recommend_consolidate (similar), promote (score > 0.8)

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// Configuration for the autonomous curator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorConfig {
    /// Cycle interval in days (default 7).
    #[serde(default = "default_cycle_days")]
    pub cycle_days: u32,
    /// Minimum score below which a skill is recommended for pruning.
    #[serde(default = "default_prune_threshold")]
    pub prune_threshold: f64,
    /// Whether to auto-prune without approval.
    #[serde(default)]
    pub auto_prune: bool,
    /// Path to the curator report.
    #[serde(skip)]
    pub report_path: PathBuf,
}

fn default_cycle_days() -> u32 {
    7
}
fn default_prune_threshold() -> f64 {
    0.3
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            cycle_days: 7,
            prune_threshold: 0.3,
            auto_prune: false,
            report_path: PathBuf::new(),
        }
    }
}

/// A single skill graded by the curator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillGrade {
    pub skill_name: String,
    pub score: f64,
    pub usage_count: u32,
    pub age_days: u32,
    pub quality_score: f64,
    pub dependencies: u32,
    pub recommendation: GradeRecommendation,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeRecommendation {
    /// Skill is healthy — keep as-is.
    Keep,
    /// Score is low — recommend pruning.
    RecommendPrune,
    /// Similar to another skill — recommend consolidation.
    RecommendConsolidate { similar_to: String },
    /// High performer — candidate for promotion.
    Promote,
}

impl GradeRecommendation {
    fn from_score(score: f64) -> Self {
        if score > 0.8 {
            Self::Promote
        } else if score < 0.3 {
            Self::RecommendPrune
        } else {
            Self::Keep
        }
    }
}

/// Curated skill report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorReport {
    pub generated_at: i64,
    pub total_skills: usize,
    pub grades: Vec<SkillGrade>,
    pub prune_candidates: Vec<String>,
    pub promote_candidates: Vec<String>,
    pub consolidate_candidates: Vec<(String, String)>,
}

impl Default for CuratorReport {
    fn default() -> Self {
        Self::new()
    }
}

impl CuratorReport {
    pub fn new() -> Self {
        Self {
            generated_at: now_secs(),
            total_skills: 0,
            grades: Vec::new(),
            prune_candidates: Vec::new(),
            promote_candidates: Vec::new(),
            consolidate_candidates: Vec::new(),
        }
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The autonomous curator.
pub struct Curator {
    config: CuratorConfig,
    paths: PraxisPaths,
}

impl Curator {
    pub fn new(config: CuratorConfig, paths: &PraxisPaths) -> Self {
        Self { config, paths: paths.clone() }
    }

    /// Run the curation cycle.
    pub fn run_cycle(&self) -> Result<CuratorReport> {
        use crate::skills::load_catalog;
        let mut report = CuratorReport::new();
        let skills_dir = self.paths.data_dir.join("skills");
        let skills = load_catalog(&skills_dir);
        report.total_skills = skills.len();

        for skill in skills {
            let grade = self.grade_skill(skill.name.as_str())?;
            let recommendation = GradeRecommendation::from_score(grade.score);

            match &recommendation {
                GradeRecommendation::RecommendPrune => {
                    report.prune_candidates.push(skill.name.clone())
                }
                GradeRecommendation::Promote => report.promote_candidates.push(skill.name.clone()),
                _ => {}
            }

            report.grades.push(SkillGrade {
                skill_name: skill.name.clone(),
                score: grade.score,
                usage_count: grade.usage_count,
                age_days: grade.age_days,
                quality_score: grade.quality_score,
                dependencies: grade.dependencies,
                recommendation,
                reason: grade.reason,
            });
        }

        // Sort grades by score descending
        report
            .grades
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Save report
        self.save_report(&report)?;

        Ok(report)
    }

    fn grade_skill(&self, name: &str) -> Result<SkillGrade> {
        // Usage frequency from skill file metadata
        let skill_path = self.paths.data_dir.join("skills").join(format!("{}.md", name));
        let age_days = if skill_path.exists() {
            let meta = fs::metadata(&skill_path)?;
            let created = meta.created().ok();
            let modified = meta.modified().ok();
            let newest = created.or(modified);
            newest
                .map(|t| {
                    let diff =
                        SystemTime::now().duration_since(t).ok().map(|d| d.as_secs() / 86400);
                    diff.unwrap_or(0) as u32
                })
                .unwrap_or(0)
        } else {
            0
        };

        // Placeholder scoring — real implementation would query session history
        let usage_count: u32 = 0;
        let quality_score: f64 = 0.7;
        let dependencies: u32 = 0;

        // Weighted score: usage(40%) + age_freshness(20%) + quality(20%) + deps(20%)
        let age_score = if age_days < 7 {
            1.0
        } else if age_days < 30 {
            0.7
        } else {
            0.4
        };
        let score = (usage_count as f64 * 0.1).min(1.0) * 0.4
            + age_score * 0.2
            + quality_score * 0.2
            + (dependencies as f64 * 0.1).min(1.0) * 0.2;

        Ok(SkillGrade {
            skill_name: name.to_string(),
            score,
            usage_count,
            age_days,
            quality_score,
            dependencies,
            recommendation: GradeRecommendation::from_score(score),
            reason: format!(
                "usage={}, age={}d, quality={:.1}, deps={}",
                usage_count, age_days, quality_score, dependencies
            ),
        })
    }

    fn save_report(&self, report: &CuratorReport) -> Result<()> {
        let path = self.paths.data_dir.join("curator_report.json");
        let json = serde_json::to_string_pretty(report).context("serialize curator report")?;
        fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
        log::info!("curator report saved to {}", path.display());
        Ok(())
    }

    /// Get the current curator report.
    pub fn latest_report(&self) -> Result<Option<CuratorReport>> {
        let path = self.paths.data_dir.join("curator_report.json");
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path)?;
        let report: CuratorReport =
            serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        Ok(Some(report))
    }

    /// Check if a cycle is due.
    pub fn is_cycle_due(&self) -> Result<bool> {
        let path = self.paths.data_dir.join("curator_last_run.txt");
        if !path.exists() {
            return Ok(true);
        }
        let raw = fs::read_to_string(&path)?;
        let last_run: i64 = raw.trim().parse().unwrap_or(0);
        let days_since = (now_secs() - last_run) / 86400;
        Ok(days_since >= self.config.cycle_days as i64)
    }

    /// Mark that a cycle has run.
    pub fn mark_cycle_run(&self) -> Result<()> {
        let path = self.paths.data_dir.join("curator_last_run.txt");
        fs::write(&path, now_secs().to_string())?;
        Ok(())
    }
}
