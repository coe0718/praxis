//! Capability benchmarking — recurring operator-specific capability tests that
//! track agent usefulness over time.
//!
//! Benchmark cases live in `benchmarks/*.json` inside the data directory.
//! Each case is a named, versioned scenario with shell verification commands.
//! Results are appended to `benchmark_log.jsonl` so trends can be analysed
//! across sessions.
//!
//! Benchmark cases are distinct from evals:
//! - **Evals** — correctness checks for identity and trust (run on every session).
//! - **Benchmarks** — longitudinal capability tracking (run on demand or per schedule).

use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Case format ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCase {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Semantic version of this case definition.
    #[serde(default = "default_version")]
    pub version: String,
    /// Shell commands that must all exit 0 for the benchmark to pass.
    pub commands: Vec<String>,
    /// Tags for grouping (e.g. "memory", "reasoning", "tool-use").
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

// ── Result format ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BenchmarkStatus {
    Passed,
    Failed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub case_id: String,
    pub case_name: String,
    pub version: String,
    pub status: BenchmarkStatus,
    pub summary: String,
    pub ran_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
}

// ── Suite ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy)]
pub struct BenchmarkSuite;

impl BenchmarkSuite {
    /// Validate all case files in `benchmarks_dir`. Returns the count.
    pub fn validate(&self, paths: &PraxisPaths) -> Result<usize> {
        let files = json_files(&paths.benchmarks_dir)?;
        for path in &files {
            load_case(path)?;
        }
        Ok(files.len())
    }

    /// Run all cases and append results to the benchmark log.
    pub fn run(&self, paths: &PraxisPaths) -> Result<Vec<BenchmarkResult>> {
        let now = Utc::now();
        let mut results = Vec::new();

        for path in json_files(&paths.benchmarks_dir)? {
            let case = load_case(&path)?;
            let result = run_case(paths, &case, now);
            results.push(result);
        }

        append_results(&paths.benchmark_log_file, &results)?;
        Ok(results)
    }

    /// Load previously recorded results from the log.
    pub fn load_log(paths: &PraxisPaths) -> Result<Vec<BenchmarkResult>> {
        if !paths.benchmark_log_file.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&paths.benchmark_log_file)
            .context("failed to read benchmark log")?;
        let mut records = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<BenchmarkResult>(line) {
                records.push(record);
            }
        }
        Ok(records)
    }
}

// ── Running ───────────────────────────────────────────────────────────────────

fn run_case(paths: &PraxisPaths, case: &BenchmarkCase, now: DateTime<Utc>) -> BenchmarkResult {
    for command in &case.commands {
        match Command::new("/bin/sh")
            .arg("-lc")
            .arg(command)
            .current_dir(&paths.data_dir)
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(_) => {
                return BenchmarkResult {
                    case_id: case.id.clone(),
                    case_name: case.name.clone(),
                    version: case.version.clone(),
                    status: BenchmarkStatus::Failed,
                    summary: format!("Benchmark failed: {} (command exited non-zero)", case.name),
                    ran_at: now,
                    tags: case.tags.clone(),
                };
            }
            Err(e) => {
                return BenchmarkResult {
                    case_id: case.id.clone(),
                    case_name: case.name.clone(),
                    version: case.version.clone(),
                    status: BenchmarkStatus::Error,
                    summary: format!("Benchmark error: {} — {e}", case.name),
                    ran_at: now,
                    tags: case.tags.clone(),
                };
            }
        }
    }

    BenchmarkResult {
        case_id: case.id.clone(),
        case_name: case.name.clone(),
        version: case.version.clone(),
        status: BenchmarkStatus::Passed,
        summary: format!("Benchmark passed: {}", case.name),
        ran_at: now,
        tags: case.tags.clone(),
    }
}

fn append_results(log_file: &std::path::Path, results: &[BenchmarkResult]) -> Result<()> {
    if results.is_empty() {
        return Ok(());
    }
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .with_context(|| format!("failed to open {}", log_file.display()))?;
    for result in results {
        let line = serde_json::to_string(result).context("failed to serialize benchmark result")?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to write to {}", log_file.display()))?;
    }
    Ok(())
}

// ── Loading ───────────────────────────────────────────────────────────────────

fn load_case(path: &PathBuf) -> Result<BenchmarkCase> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let case: BenchmarkCase = serde_json::from_str(&raw)
        .with_context(|| format!("invalid benchmark JSON in {}", path.display()))?;

    if case.id.trim().is_empty() || case.name.trim().is_empty() {
        bail!("benchmark in {} must include id and name", path.display());
    }
    if case.commands.is_empty() {
        bail!("benchmark {} must include at least one command", case.id);
    }

    Ok(case)
}

fn json_files(dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

impl BenchmarkResult {
    pub fn status_label(&self) -> &'static str {
        match self.status {
            BenchmarkStatus::Passed => "pass",
            BenchmarkStatus::Failed => "FAIL",
            BenchmarkStatus::Error => "ERROR",
        }
    }
}

// ── Summary helpers ───────────────────────────────────────────────────────────

pub fn summarize_results(results: &[BenchmarkResult]) -> String {
    let passed = results.iter().filter(|r| r.status == BenchmarkStatus::Passed).count();
    let failed = results.iter().filter(|r| r.status == BenchmarkStatus::Failed).count();
    let errors = results.iter().filter(|r| r.status == BenchmarkStatus::Error).count();
    format!(
        "Benchmarks: {}/{} passed, {} failed, {} errors",
        passed,
        results.len(),
        failed,
        errors
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::paths::PraxisPaths;

    use super::{BenchmarkCase, BenchmarkStatus, BenchmarkSuite, summarize_results};

    fn paths_for(dir: &std::path::Path) -> PraxisPaths {
        PraxisPaths::for_data_dir(dir.to_path_buf())
    }

    fn write_case(dir: &std::path::Path, id: &str, commands: &[&str]) {
        let case = BenchmarkCase {
            id: id.to_string(),
            name: format!("Test case {id}"),
            description: "A test benchmark case.".to_string(),
            version: "1.0.0".to_string(),
            commands: commands.iter().map(|c| c.to_string()).collect(),
            tags: vec!["test".to_string()],
        };
        let path = dir.join(format!("{id}.json"));
        fs::write(path, serde_json::to_string(&case).unwrap()).unwrap();
    }

    #[test]
    fn runs_passing_benchmarks_and_appends_log() {
        let tmp = tempdir().unwrap();
        let paths = paths_for(tmp.path());
        fs::create_dir_all(&paths.benchmarks_dir).unwrap();

        write_case(&paths.benchmarks_dir, "b-001", &["true"]);

        let results = BenchmarkSuite.run(&paths).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, BenchmarkStatus::Passed);

        // Log should now exist and be parseable.
        let log = BenchmarkSuite::load_log(&paths).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].case_id, "b-001");
    }

    #[test]
    fn records_failing_benchmark() {
        let tmp = tempdir().unwrap();
        let paths = paths_for(tmp.path());
        fs::create_dir_all(&paths.benchmarks_dir).unwrap();

        write_case(&paths.benchmarks_dir, "b-002", &["false"]);

        let results = BenchmarkSuite.run(&paths).unwrap();
        assert_eq!(results[0].status, BenchmarkStatus::Failed);
    }

    #[test]
    fn summarize_gives_correct_counts() {
        let tmp = tempdir().unwrap();
        let paths = paths_for(tmp.path());
        fs::create_dir_all(&paths.benchmarks_dir).unwrap();

        write_case(&paths.benchmarks_dir, "b-003", &["true"]);
        write_case(&paths.benchmarks_dir, "b-004", &["false"]);

        let results = BenchmarkSuite.run(&paths).unwrap();
        let summary = summarize_results(&results);
        assert!(summary.contains("1/2 passed"));
        assert!(summary.contains("1 failed"));
    }

    #[test]
    fn no_benchmarks_directory_returns_empty() {
        let tmp = tempdir().unwrap();
        let paths = paths_for(tmp.path());
        // benchmarks_dir does not exist
        let results = BenchmarkSuite.run(&paths).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn log_accumulates_across_runs() {
        let tmp = tempdir().unwrap();
        let paths = paths_for(tmp.path());
        fs::create_dir_all(&paths.benchmarks_dir).unwrap();
        write_case(&paths.benchmarks_dir, "b-005", &["true"]);

        BenchmarkSuite.run(&paths).unwrap();
        BenchmarkSuite.run(&paths).unwrap();

        let log = BenchmarkSuite::load_log(&paths).unwrap();
        assert_eq!(log.len(), 2);
    }
}
