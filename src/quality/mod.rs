mod evals;
mod reviewer;

use std::path::Path;

use anyhow::Result;

pub use evals::{EvalDefinition, EvalOutcome, EvalRunner, EvalSummary, LocalEvalSuite, summarize};
pub use reviewer::{GoalCriteria, LocalReviewer, ReviewOutcome, Reviewer};

pub(crate) fn json_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}
