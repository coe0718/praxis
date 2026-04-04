use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use predicates::prelude::*;
use serde::Deserialize;
use tempfile::tempdir;

use crate::common::praxis_command;

#[derive(Debug, Deserialize)]
struct ReplayFixture {
    steps: Vec<ReplayStep>,
}

#[derive(Debug, Deserialize)]
struct ReplayStep {
    #[serde(default)]
    at: Option<String>,
    args: Vec<String>,
    #[serde(default)]
    env: Vec<ReplayEnvVar>,
    #[serde(default)]
    stdout: Vec<String>,
    #[serde(default)]
    stderr: Vec<String>,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    files: Vec<ReplayFileCheck>,
}

#[derive(Debug, Deserialize)]
struct ReplayEnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct ReplayFileCheck {
    path: String,
    contains: String,
}

fn default_status() -> String {
    "success".to_string()
}

pub fn run_fixture(name: &str) -> Result<()> {
    let temp = tempdir().context("failed to create replay tempdir")?;
    let data_dir = temp.path().join("praxis");
    let fixture = load_fixture(name)?;

    for step in fixture.steps {
        run_step(&data_dir, step)?;
    }
    Ok(())
}

fn load_fixture(name: &str) -> Result<ReplayFixture> {
    let path = fixture_path(name);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read replay fixture {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("invalid replay fixture {}", path.display()))
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/replay")
        .join(format!("{name}.toml"))
}

fn run_step(data_dir: &PathBuf, step: ReplayStep) -> Result<()> {
    let mut command = praxis_command();
    command.arg("--data-dir").arg(data_dir);
    if let Some(at) = &step.at {
        command.env("PRAXIS_FIXED_NOW", at);
    }
    for env in &step.env {
        command.env(&env.key, &env.value);
    }
    for arg in &step.args {
        command.arg(arg);
    }

    let mut assert = command.assert();
    assert = match step.status.as_str() {
        "success" => assert.success(),
        "failure" => assert.failure(),
        other => bail!("unsupported replay step status {other}"),
    };

    for needle in &step.stdout {
        assert = assert.stdout(predicate::str::contains(needle));
    }
    for needle in &step.stderr {
        assert = assert.stderr(predicate::str::contains(needle));
    }

    for check in &step.files {
        let content = fs::read_to_string(data_dir.join(&check.path))
            .with_context(|| format!("failed to read replay file {}", check.path))?;
        if !content.contains(&check.contains) {
            bail!(
                "expected {} to contain {:?}, but it did not",
                check.path,
                check.contains
            );
        }
    }
    Ok(())
}
