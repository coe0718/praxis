//! Formal generator/evaluator loop — a named orchestration primitive.
//!
//! For high-stakes generation or design work, Praxis supports an explicit loop:
//!
//! 1. A **generator** produces output against explicit pass criteria.
//! 2. An **evaluator** reviews it against those criteria.
//! 3. Failures return with structured feedback.
//! 4. The loop stops after a configurable max round count or a clean pass.
//!
//! This is deliberately separate from the `Reviewer` (which runs shell
//! commands against success criteria files) and `EvalRunner` (which runs the
//! operator-specific eval suite).  The evaluate loop is for in-flight
//! generation tasks where the content itself needs iterative refinement before
//! it is stored or delivered.

use anyhow::Result;

/// Configuration for a single evaluate loop run.
#[derive(Debug, Clone)]
pub struct EvaluateConfig {
    /// Maximum number of generate→evaluate rounds before giving up.
    pub max_rounds: usize,
    /// Name of the task being evaluated — used in diagnostics.
    pub task_name: String,
}

impl Default for EvaluateConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            task_name: "unnamed task".to_string(),
        }
    }
}

/// The outcome of a single generator call.
#[derive(Debug, Clone)]
pub struct GeneratorOutput {
    /// The generated content.
    pub content: String,
    /// Optional metadata the evaluator can use (e.g. which criteria were applied).
    pub metadata: Option<String>,
}

/// The outcome of a single evaluator call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluatorVerdict {
    /// Content meets all criteria.
    Pass,
    /// Content has specific failures.  The strings are feedback for the next
    /// generator round.
    Fail(Vec<String>),
}

/// The result of the full evaluate loop.
#[derive(Debug, Clone)]
pub struct EvaluateResult {
    /// The final content (from the last successful or last attempted round).
    pub content: String,
    /// Whether the loop ended with a clean pass.
    pub passed: bool,
    /// How many rounds were attempted.
    pub rounds: usize,
    /// Feedback from the last evaluator call (empty when passed).
    pub final_feedback: Vec<String>,
}

/// Run the evaluate loop.
///
/// `generator` is called with the prior feedback (empty on the first round)
/// and returns a `GeneratorOutput`.
///
/// `evaluator` is called with the output and returns a `EvaluatorVerdict`.
///
/// The loop terminates when the evaluator returns `Pass`, or when
/// `config.max_rounds` is reached.
pub fn run_evaluate_loop<G, E>(
    config: &EvaluateConfig,
    mut generator: G,
    mut evaluator: E,
) -> Result<EvaluateResult>
where
    G: FnMut(&[String]) -> Result<GeneratorOutput>,
    E: FnMut(&GeneratorOutput) -> Result<EvaluatorVerdict>,
{
    let mut feedback: Vec<String> = Vec::new();
    let mut last_content = String::new();
    let mut rounds = 0;

    for _ in 0..config.max_rounds.max(1) {
        rounds += 1;
        let output = generator(&feedback)?;
        last_content = output.content.clone();

        match evaluator(&output)? {
            EvaluatorVerdict::Pass => {
                return Ok(EvaluateResult {
                    content: last_content,
                    passed: true,
                    rounds,
                    final_feedback: Vec::new(),
                });
            }
            EvaluatorVerdict::Fail(findings) => {
                feedback = findings;
            }
        }
    }

    Ok(EvaluateResult {
        content: last_content,
        passed: false,
        rounds,
        final_feedback: feedback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_immediately_when_output_is_acceptable() {
        let config = EvaluateConfig {
            max_rounds: 3,
            task_name: "test".to_string(),
        };
        let result = run_evaluate_loop(
            &config,
            |_feedback| {
                Ok(GeneratorOutput {
                    content: "clean output".to_string(),
                    metadata: None,
                })
            },
            |output| {
                if output.content == "clean output" {
                    Ok(EvaluatorVerdict::Pass)
                } else {
                    Ok(EvaluatorVerdict::Fail(vec!["not clean".to_string()]))
                }
            },
        )
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.rounds, 1);
        assert_eq!(result.content, "clean output");
    }

    #[test]
    fn improves_across_rounds_using_feedback() {
        let config = EvaluateConfig {
            max_rounds: 3,
            task_name: "iterative".to_string(),
        };
        let mut attempt = 0usize;
        let result = run_evaluate_loop(
            &config,
            |_feedback| {
                attempt += 1;
                Ok(GeneratorOutput {
                    content: format!("output-{attempt}"),
                    metadata: None,
                })
            },
            |output| {
                if output.content == "output-2" {
                    Ok(EvaluatorVerdict::Pass)
                } else {
                    Ok(EvaluatorVerdict::Fail(vec!["not ready yet".to_string()]))
                }
            },
        )
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.rounds, 2);
    }

    #[test]
    fn exhausts_max_rounds_and_reports_failure() {
        let config = EvaluateConfig {
            max_rounds: 2,
            task_name: "always-fails".to_string(),
        };
        let result = run_evaluate_loop(
            &config,
            |_| {
                Ok(GeneratorOutput {
                    content: "bad".to_string(),
                    metadata: None,
                })
            },
            |_| Ok(EvaluatorVerdict::Fail(vec!["still wrong".to_string()])),
        )
        .unwrap();

        assert!(!result.passed);
        assert_eq!(result.rounds, 2);
        assert!(!result.final_feedback.is_empty());
    }

    #[test]
    fn passes_feedback_from_previous_round_to_generator() {
        let config = EvaluateConfig {
            max_rounds: 3,
            task_name: "feedback-passing".to_string(),
        };
        let mut received_feedback: Vec<String> = Vec::new();
        run_evaluate_loop(
            &config,
            |feedback| {
                received_feedback = feedback.to_vec();
                Ok(GeneratorOutput {
                    content: "output".to_string(),
                    metadata: None,
                })
            },
            |_| Ok(EvaluatorVerdict::Fail(vec!["specific finding".to_string()])),
        )
        .unwrap();

        // By round 2+, generator should have received the evaluator's feedback.
        assert!(received_feedback.contains(&"specific finding".to_string()));
    }
}
