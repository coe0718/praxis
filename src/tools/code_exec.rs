//! Code Execution tool — sandboxed Python/script execution.
//!
//! Provides a tool for the agent to write and execute code in a sandboxed
//! environment. Supports Python scripts via subprocess, with optional
//! Docker isolation when available.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Configuration for code execution sandboxing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecConfig {
    /// Maximum execution time in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum output size in bytes before truncation.
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,
    /// Working directory for execution.
    #[serde(default)]
    pub workdir: Option<PathBuf>,
    /// Use Docker isolation if available.
    #[serde(default)]
    pub use_docker: bool,
    /// Docker image to use.
    #[serde(default = "default_docker_image")]
    pub docker_image: String,
    /// Allowed languages.
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
}

fn default_timeout() -> u64 {
    30
}
fn default_max_output() -> usize {
    50_000
}
fn default_docker_image() -> String {
    "python:3.12-slim".to_string()
}
fn default_languages() -> Vec<String> {
    vec!["python".to_string(), "bash".to_string(), "javascript".to_string()]
}

impl Default for CodeExecConfig {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            max_output_bytes: default_max_output(),
            workdir: None,
            use_docker: false,
            docker_image: default_docker_image(),
            languages: default_languages(),
        }
    }
}

/// Result of a code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecResult {
    pub language: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_ms: u64,
}

/// Execute code in a sandboxed environment.
pub fn execute_code(code: &str, language: &str, config: &CodeExecConfig) -> Result<CodeExecResult> {
    if !config.languages.contains(&language.to_string()) {
        bail!("language '{}' not allowed. Allowed: {:?}", language, config.languages);
    }

    let workdir = config
        .workdir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("praxis-code-exec"));
    fs::create_dir_all(&workdir)
        .with_context(|| format!("creating workdir {}", workdir.display()))?;

    let start = std::time::Instant::now();

    if config.use_docker {
        execute_docker(code, language, config, &workdir, start)
    } else {
        execute_local(code, language, config, &workdir, start)
    }
}

fn execute_local(
    code: &str,
    language: &str,
    config: &CodeExecConfig,
    workdir: &Path,
    start: std::time::Instant,
) -> Result<CodeExecResult> {
    let (file_name, mut cmd) = match language {
        "python" | "python3" => {
            let path = workdir.join("script.py");
            fs::write(&path, code)?;
            (path, Command::new("python3"))
        }
        "bash" | "shell" => {
            let path = workdir.join("script.sh");
            fs::write(&path, code)?;
            (path, Command::new("/bin/bash"))
        }
        "javascript" | "node" => {
            let path = workdir.join("script.js");
            fs::write(&path, code)?;
            (path, Command::new("node"))
        }
        _ => bail!("unsupported language: {}", language),
    };

    cmd.arg(&file_name).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().with_context(|| format!("spawning {} process", language))?;
    let timed_out = match child.try_wait() {
        Ok(Some(_)) => false,
        _ => {
            // Wait with timeout
            let timeout = std::time::Duration::from_secs(config.timeout_secs);
            let deadline = start + timeout;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break false,
                    Ok(None) => {
                        if std::time::Instant::now() > deadline {
                            let _ = child.kill();
                            let _ = child.wait();
                            break true;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => break false,
                }
            }
        }
    };

    let output = child.wait_with_output().context("reading process output")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let stdout = truncate_output(&output.stdout, config.max_output_bytes);
    let stderr = truncate_output(&output.stderr, config.max_output_bytes);

    // Cleanup temp file
    let _ = fs::remove_file(&file_name);

    Ok(CodeExecResult {
        language: language.to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        timed_out,
        duration_ms,
    })
}

fn execute_docker(
    code: &str,
    language: &str,
    config: &CodeExecConfig,
    workdir: &Path,
    start: std::time::Instant,
) -> Result<CodeExecResult> {
    let ext = match language {
        "python" | "python3" => "py",
        "bash" | "shell" => "sh",
        "javascript" | "node" => "js",
        _ => bail!("unsupported language for Docker: {}", language),
    };

    let script_path = workdir.join(format!("script.{ext}"));
    fs::write(&script_path, code)?;

    let cmd = match language {
        "python" | "python3" => vec!["python3", "/tmp/script.py"],
        "bash" | "shell" => vec!["bash", "/tmp/script.sh"],
        "javascript" | "node" => vec!["node", "/tmp/script.js"],
        _ => unreachable!(),
    };

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/tmp", workdir.display()),
            "--network",
            "none",
            "--memory",
            "512m",
            "--cpus",
            "1",
            &config.docker_image,
        ])
        .args(&cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("running docker container")?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = truncate_output(&output.stdout, config.max_output_bytes);
    let stderr = truncate_output(&output.stderr, config.max_output_bytes);

    let _ = fs::remove_file(&script_path);

    Ok(CodeExecResult {
        language: language.to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        timed_out: false,
        duration_ms,
    })
}

fn truncate_output(data: &[u8], max_bytes: usize) -> String {
    let s = String::from_utf8_lossy(data);
    if s.len() > max_bytes {
        format!("{}...\n[truncated at {} bytes]", &s[..max_bytes], max_bytes)
    } else {
        s.to_string()
    }
}

/// Execute the code-execution tool from a tool call.
pub fn execute_code_tool(params: &serde_json::Value) -> Result<String> {
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing 'code' parameter"))?;
    let language = params.get("language").and_then(|v| v.as_str()).unwrap_or("python");
    let timeout = params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

    let config = CodeExecConfig {
        timeout_secs: timeout,
        ..Default::default()
    };

    let result = execute_code(code, language, &config)?;
    let mut lines = vec![format!("```{}", result.language)];
    if !result.stdout.is_empty() {
        lines.push(result.stdout.clone());
    }
    if !result.stderr.is_empty() {
        lines.push(format!("--- stderr ---\n{}", result.stderr));
    }
    lines.push("```".to_string());
    lines.push(format!(
        "exit: {} | {}ms{}",
        result.exit_code,
        result.duration_ms,
        if result.timed_out { " (TIMED OUT)" } else { "" }
    ));

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_python() {
        let config = CodeExecConfig::default();
        let result = execute_code("print('hello from praxis')", "python", &config).unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello from praxis"));
    }

    #[test]
    fn test_execute_bash() {
        let config = CodeExecConfig::default();
        let result = execute_code("echo 'bash works'", "bash", &config).unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("bash works"));
    }

    #[test]
    fn test_disallowed_language() {
        let config = CodeExecConfig::default();
        assert!(execute_code("code", "ruby", &config).is_err());
    }

    #[test]
    fn test_timeout() {
        let config = CodeExecConfig {
            timeout_secs: 1,
            ..Default::default()
        };
        let result = execute_code("import time; time.sleep(10)", "python", &config).unwrap();
        assert!(result.timed_out);
    }
}
