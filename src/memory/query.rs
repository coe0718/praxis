use crate::identity::Goal;

const MAX_QUERY_TOKENS: usize = 12;

pub fn build_lookup_query(requested_task: Option<&str>, open_goals: &[Goal]) -> Option<String> {
    if let Some(task) = requested_task {
        return normalize_query(task);
    }

    normalize_query(
        &open_goals
            .iter()
            .take(2)
            .map(|goal| goal.title.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub fn to_fts_query(query: &str) -> Option<String> {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        None
    } else {
        Some(
            tokens
                .into_iter()
                .map(|token| format!("\"{token}\""))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

fn normalize_query(input: &str) -> Option<String> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .take(MAX_QUERY_TOKENS)
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_lookup_query, to_fts_query};
    use crate::identity::Goal;

    #[test]
    fn strips_punctuation_from_lookup_queries() {
        let query = build_lookup_query(Some("Automate recurring work: task: clean notes"), &[]);

        assert_eq!(
            query.as_deref(),
            Some("Automate recurring work task clean notes")
        );
    }

    #[test]
    fn turns_free_text_into_safe_fts_terms() {
        let query = to_fts_query("Automate recurring work: task: clean notes");
        assert_eq!(
            query.as_deref(),
            Some("\"Automate\" \"recurring\" \"work\" \"task\" \"clean\" \"notes\"")
        );
    }

    #[test]
    fn derives_lookup_query_from_goal_titles() {
        let query = build_lookup_query(
            None,
            &[Goal {
                id: "G-002".to_string(),
                title: "Ship the dependent feature".to_string(),
                completed: false,
                line_number: 1,
                blocked_by: Vec::new(),
                wake_when: None,
            }],
        );

        assert_eq!(query.as_deref(), Some("Ship the dependent feature"));
    }
}
