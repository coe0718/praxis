use std::collections::HashSet;

use super::budget::estimate_tokens;

pub(super) fn summarize_to_tokens(content: &str, max_tokens: usize) -> String {
    if estimate_tokens(content) <= max_tokens {
        return content.to_string();
    }

    let lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return String::new();
    }

    let max_chars = max_tokens.saturating_mul(4);
    let mut summary = "Summarized excerpt preserving anchors:".to_string();
    let mut seen = HashSet::new();
    let anchors = anchor_lines(&lines);
    let anchor_line_limit = (max_chars.saturating_sub(40) / anchors.len().max(1)).clamp(16, 48);
    append_section(
        &mut summary,
        &mut seen,
        "Anchors",
        anchors,
        max_chars,
        Some(anchor_line_limit),
    );
    append_section(
        &mut summary,
        &mut seen,
        "Start",
        lines.iter().take(2).copied().collect(),
        max_chars,
        None,
    );
    append_section(
        &mut summary,
        &mut seen,
        "Tail",
        lines.iter().rev().take(3).copied().collect(),
        max_chars,
        None,
    );

    if summary == "Summarized excerpt preserving anchors:" {
        fit_to_budget(content, max_tokens)
    } else {
        fit_to_budget(&summary, max_tokens)
    }
}

fn append_section(
    summary: &mut String,
    seen: &mut HashSet<String>,
    label: &str,
    lines: Vec<&str>,
    max_chars: usize,
    line_limit: Option<usize>,
) {
    let mut added_any = false;
    for line in lines {
        let normalized = compact_line(line);
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        let rendered = line_limit
            .map(|limit| truncate_chars(&normalized, limit))
            .unwrap_or(normalized);

        let prefix = if added_any {
            "\n- ".to_string()
        } else {
            format!("\n{label}:\n- ")
        };
        let candidate = format!("{prefix}{rendered}");
        if summary.chars().count() + candidate.chars().count() <= max_chars {
            summary.push_str(&candidate);
            added_any = true;
            continue;
        }

        if added_any {
            continue;
        }

        let remaining = max_chars.saturating_sub(summary.chars().count() + prefix.chars().count());
        if remaining <= 3 {
            return;
        }

        let truncated = truncate_chars(&rendered, remaining);
        if truncated.is_empty() {
            return;
        }
        summary.push_str(&prefix);
        summary.push_str(&truncated);
        return;
    }
}

fn anchor_lines<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    let mut anchors = lines
        .iter()
        .copied()
        .filter(|line| is_anchor(line))
        .collect::<Vec<_>>();
    anchors.sort_by_key(|line| std::cmp::Reverse(anchor_score(line)));
    anchors.truncate(8);
    anchors
}

fn is_anchor(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.starts_with('#')
        || lower.starts_with("- [")
        || contains_goal_id(line)
        || looks_like_date(line)
        || ["boundary", "never", "blocked", "wait", "review", "operator"]
            .iter()
            .any(|keyword| lower.contains(keyword))
}

fn anchor_score(line: &str) -> usize {
    let lower = line.to_ascii_lowercase();
    usize::from(line.starts_with('#'))
        + usize::from(lower.starts_with("- [")) * 3
        + usize::from(contains_goal_id(line)) * 3
        + usize::from(looks_like_date(line)) * 2
        + usize::from(lower.contains("boundary") || lower.contains("never")) * 3
        + usize::from(lower.contains("operator") || lower.contains("review"))
}

fn contains_goal_id(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.windows(2).enumerate().any(|(index, pair)| {
        pair == b"G-"
            && bytes
                .iter()
                .skip(index + 2)
                .take_while(|value| value.is_ascii_digit())
                .count()
                >= 3
    })
}

fn looks_like_date(line: &str) -> bool {
    line.contains("20")
        && (line.contains('-') || line.contains('/'))
        && line.chars().filter(|ch| ch.is_ascii_digit()).count() >= 6
}

fn compact_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fit_to_budget(content: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    truncate_chars(content, max_chars)
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    let truncated = content
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{}...", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::summarize_to_tokens;

    #[test]
    fn preserves_goal_boundary_and_date_anchors() {
        let content = [
            "# Journal",
            "This line is filler and should be compressible.",
            "- [ ] G-042: Ship the next tool release",
            "Boundary: never message the operator after quiet hours.",
            "2026-04-05 review noted repeated context thrash.",
            "Another long filler line that should not dominate the summary output.",
        ]
        .join("\n");

        let summary = summarize_to_tokens(&content, 30);
        assert!(summary.contains("G-042"));
        assert!(summary.contains("Boundary:"));
        assert!(summary.contains("2026-04-05"));
    }

    #[test]
    fn truncates_when_anchor_summary_still_exceeds_budget() {
        let summary = summarize_to_tokens(&"A".repeat(500), 10);
        assert!(summary.ends_with("..."));
    }
}
