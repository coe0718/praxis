//! SSH Terminal Backend — Execute tools on remote SSH hosts.
//!
//! Tools can run in an SSH session with configurable connection parameters.
//! Uses the `ssh2` crate for native SSH client support.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// SSH connection configuration.
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: Option<PathBuf>,
}

impl SshConfig {
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("PRAXIS_SSH_HOST")
            .context("PRAXIS_SSH_HOST is required for SSH backend")?;
        let port = std::env::var("PRAXIS_SSH_PORT")
            .unwrap_or_else(|_| "22".to_string())
            .parse()
            .context("PRAXIS_SSH_PORT must be a valid port number")?;
        let username = std::env::var("PRAXIS_SSH_USER")
            .unwrap_or_else(|_| whoami::username());
        let private_key_path = std::env::var("PRAXIS_SSH_KEY").ok().map(PathBuf::from);

        Ok(Self {
            host,
            port,
            username,
            private_key_path,
        })
    }
}

/// SSH Backend for remote tool execution.
pub struct SshBackend {
    config: SshConfig,
}

impl SshBackend {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }

    /// Execute a command on the remote SSH host.
    pub fn execute(&self, command: &str, _timeout_secs: Option<u64>) -> Result<String> {
        let ssh_cmd = format!(
            "ssh -p {} {}@{} '{}'",
            self.config.port, self.config.username, self.config.host, command
        );

        let output = if let Some(key_path) = &self.config.private_key_path {
            Command::new("ssh")
                .args([
                    "-p",
                    &self.config.port.to_string(),
                    "-i",
                    key_path.to_str().unwrap_or(""),
                    &format!("{}@{}", self.config.username, self.config.host),
                    command,
                ])
                .output()
                .context("failed to execute SSH command")?
        } else {
            Command::new("ssh")
                .args([
                    "-p",
                    &self.config.port.to_string(),
                    &format!("{}@{}", self.config.username, self.config.host),
                    command,
                ])
                .output()
                .context("failed to execute SSH command")?
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SSH command failed: {}", stderr)
        }
    }

    /// Validate SSH connectivity.
    pub fn validate(&self) -> Result<()> {
        let _ = self.execute("echo 'praxis-ssh-ok'", None)?;
        Ok(())
    }
}
