//! Skill synthesis — automatic draft generation in Reflect.
//!
//! After a sufficiently complex successful session, Reflect checks whether the
//! work warrants a new reusable skill draft.  Complexity is currently measured
//! by a combination of:
//!
//! - session outcome (must be "completed" or similar success)
//! - whether a named goal was worked on (not just idle maintenance)
//! - the number of tool calls that were reviewed (proxy for multi-step work)
//! - whether the goal title suggests a repeatable workflow pattern
//!
//! When all thresholds are met a `SKILL.md` draft is written to
//! `skills/drafts/<slug>.md`.  Drafts are never activated automatically; they
//! enter the same proposal/review path as other capability changes.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

/// Minimum number of tool invocations before we consider a session complex
/// enough to potentially produce a reusable skill.
const MIN_TOOL_CALLS_FOR_SKILL: usize = 3;

/// Criteria for deciding whether to draft a skill from a session.
#[derive(Debug)]
pub struct SkillSynthesisInput<'a> {
    pub outcome: &'a str,
    pub goal_title: Option<&'a str>,
    pub tool_call_count: usize,
    pub action_summary: &'a str,
    pub session_ended_at: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SkillSynthesizer;

impl SkillSynthesizer {
    /// Evaluate the session and write a skill draft to `drafts_dir` if the
    /// complexity thresholds are met.  Returns the path of the drafted file, or
    /// `None` if no draft was produced.
    pub fn maybe_draft(
        &self,
        drafts_dir: &Path,
        input: &SkillSynthesisInput<'_>,
    ) -> Result<Option<std::path::PathBuf>> {
        if !self.should_draft(input) {
            return Ok(None);
        }

        let goal_title = input.goal_title.unwrap_or("unnamed task");
        let slug = slugify(goal_title);
        let timestamp = input.session_ended_at.format("%Y%m%dT%H%M%SZ");
        let filename = format!("{slug}-{timestamp}.md");

        fs::create_dir_all(drafts_dir)
            .with_context(|| format!("failed to create {}", drafts_dir.display()))?;

        let path = drafts_dir.join(&filename);

        // Only create the draft if a file with this slug doesn't already exist
        // from a prior session.
        if path.exists() {
            return Ok(None);
        }

        let content = render_draft(goal_title, input);
        fs::write(&path, content)
            .with_context(|| format!("failed to write skill draft {}", path.display()))?;

        Ok(Some(path))
    }

    fn should_draft(&self, input: &SkillSynthesisInput<'_>) -> bool {
        // Must be a successful outcome.
        let success = matches!(input.outcome, "completed" | "success" | "done" | "reviewer_passed");
        if !success {
            return false;
        }

        // Must have worked on a named goal (not idle maintenance).
        if input.goal_title.is_none() {
            return false;
        }

        // Must have enough tool calls to suggest multi-step work.
        if input.tool_call_count < MIN_TOOL_CALLS_FOR_SKILL {
            return false;
        }

        true
    }
}

fn render_draft(goal_title: &str, input: &SkillSynthesisInput<'_>) -> String {
    let tags = infer_tags(goal_title, input.action_summary);
    let tags_toml = tags.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(", ");

    format!(
        r#"+++
name = "{goal_title}"
description = "Auto-drafted skill from session on {date}. Review and edit before activating."
tags = [{tags_toml}]
token_estimate = 500
provenance = "auto-synthesized"
status = "draft"
+++

## Overview

This skill was drafted automatically after a successful multi-step session.
Review, edit, and move it to `skills/` when it is ready to be used in Orient.

## Context

- Goal: {goal_title}
- Session ended: {date}
- Tool call count: {tool_calls}

## Session Summary

{summary}

## Steps (inferred — please verify and refine)

<!-- Add the actual step-by-step workflow here. -->

1. (Step 1 from session)
2. (Step 2 from session)
3. (Verify outcome)

## When to Use

<!-- Describe the conditions under which this skill should be selected. -->

## Known Limitations

<!-- Document any edge cases or failure modes observed during the session. -->
"#,
        date = input.session_ended_at.format("%Y-%m-%d"),
        tags_toml = tags_toml,
        tool_calls = input.tool_call_count,
        summary = input.action_summary,
        goal_title = goal_title,
    )
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn infer_tags(title: &str, summary: &str) -> Vec<String> {
    let combined = format!("{title} {summary}").to_lowercase();
    let keywords = [
        ("deploy", "ops"),
        ("build", "build"),
        ("test", "testing"),
        ("review", "review"),
        ("brief", "brief"),
        ("telegram", "telegram"),
        ("memory", "memory"),
        ("goal", "goals"),
        ("mail", "email"),
        ("git", "git"),
        ("database", "db"),
        ("monitor", "monitoring"),
    ];
    let mut tags: Vec<String> = keywords
        .iter()
        .filter(|(kw, _)| combined.contains(kw))
        .map(|(_, tag)| tag.to_string())
        .collect();
    tags.push("workflow".to_string());
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    fn input<'a>(
        outcome: &'a str,
        goal: Option<&'a str>,
        calls: usize,
        summary: &'a str,
    ) -> SkillSynthesisInput<'a> {
        SkillSynthesisInput {
            outcome,
            goal_title: goal,
            tool_call_count: calls,
            action_summary: summary,
            session_ended_at: chrono::Utc.with_ymd_and_hms(2026, 4, 14, 8, 0, 0).unwrap(),
        }
    }

    #[test]
    fn drafts_skill_for_complex_successful_session() {
        let tmp = tempdir().unwrap();
        let drafts = tmp.path().join("drafts");
        let synth = SkillSynthesizer;

        let result = synth
            .maybe_draft(
                &drafts,
                &input("completed", Some("morning brief delivery"), 5, "Pushed brief to Telegram."),
            )
            .unwrap();

        assert!(result.is_some());
        let path = result.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("morning brief delivery"));
        assert!(content.contains("provenance = \"auto-synthesized\""));
    }

    #[test]
    fn skips_draft_for_idle_session() {
        let tmp = tempdir().unwrap();
        let synth = SkillSynthesizer;
        let result = synth
            .maybe_draft(&tmp.path().join("drafts"), &input("idle", None, 0, "Idle."))
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn skips_draft_for_failed_outcome() {
        let tmp = tempdir().unwrap();
        let synth = SkillSynthesizer;
        let result = synth
            .maybe_draft(
                &tmp.path().join("drafts"),
                &input("failed", Some("deploy production"), 6, "Deploy failed."),
            )
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn skips_draft_when_too_few_tool_calls() {
        let tmp = tempdir().unwrap();
        let synth = SkillSynthesizer;
        let result = synth
            .maybe_draft(
                &tmp.path().join("drafts"),
                &input("completed", Some("quick check"), 2, "Done."),
            )
            .unwrap();
        assert!(result.is_none());
    }
}
