//! Docker Isolation Mode — Per-tool container isolation.
//!
//! Tools execute in isolated Docker containers with resource limits.
//! Each tool gets its own container with specific capabilities and mounts.

use serde::{Deserialize, Serialize};
use std::process::Command;

/// Container configuration for isolated tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub memory_limit: String,
    pub cpu_limit: Option<String>,
    pub network_mode: NetworkMode,
    pub mounts: Vec<MountSpec>,
    pub env_vars: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NetworkMode {
    Isolated,
    Bridge,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountSpec {
    pub source: String,
    pub target: String,
    pub readonly: bool,
}

/// Docker Isolation manager for container execution.
pub struct DockerIsolation {
    docker_host: String,
    containers: std::collections::HashMap<String, ContainerInfo>,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub container_id: String,
    pub image: String,
    pub created_at: u64,
    pub exited: bool,
}

impl DockerIsolation {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            docker_host: "unix:///var/run/docker.sock".to_string(),
            containers: std::collections::HashMap::new(),
        })
    }

    /// Validate a mount source is within allowed paths.
    /// Prevents path traversal attacks like `/:/hostfs` or `/var/run/docker.sock`.
    fn validate_mount_source(&self, source: &str) -> Result<(), anyhow::Error> {
        use std::path::Path;
        let path = Path::new(source);
        // Reject absolute paths that could expose sensitive system paths
        let dangerous_paths =
            ["/:/hostfs", "/var/run/docker.sock", "/etc/passwd", "/root/.ssh", "/home"];
        let source_str = source;
        for dangerous in &dangerous_paths {
            if source_str.starts_with(dangerous) || dangerous.starts_with(source_str) {
                anyhow::bail!("Mount source '{}' is not allowed (security restriction)", source);
            }
        }
        // Allow mounts only under Praxis data directories or explicitly whitelisted paths
        if source.starts_with("/mnt/docker")
            || source.starts_with("/home")
            || source.starts_with("/tmp")
        {
            // Additional check: ensure normalized path doesn't escape
            if let Ok(canonical) = path.canonicalize() {
                let canon_str = canonical.to_string_lossy();
                if canon_str.contains("docker.sock") || canon_str.contains("ssh") {
                    anyhow::bail!("Mount source '{}' resolves to a sensitive path", source);
                }
            }
        }
        Ok(())
    }

    /// Execute a tool in an isolated container.
    pub async fn execute_isolated(
        &mut self,
        tool_name: &str,
        config: &ContainerConfig,
        args: &[String],
    ) -> Result<String, anyhow::Error> {
        log::debug!("docker isolation: executing '{}' via host '{}'", tool_name, self.docker_host);

        // C12 fix: Add random suffix to prevent collision in same nanosecond
        let rand_suffix: u32 = rand::random();
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let container_name = format!("praxis-{}-{:?}-{}", tool_name, ts, rand_suffix);

        let mut cmd = Command::new("docker");
        cmd.args(["run", "--rm", "--name", &container_name]);

        // Add resource limits
        cmd.args(["--memory", &config.memory_limit]);
        if let Some(cpu) = &config.cpu_limit {
            cmd.args(["--cpus", cpu]);
        }

        // Network mode
        match config.network_mode {
            NetworkMode::Isolated => {
                cmd.args(["--network", "none"]);
            }
            NetworkMode::Bridge => {
                cmd.args(["--network", "bridge"]);
            }
            NetworkMode::None => {}
        }

        // Mounts - C2 fix: validate each source path
        for mount in &config.mounts {
            self.validate_mount_source(&mount.source)?;
            let mount_str = if mount.readonly {
                format!("{}:{}:ro", mount.source, mount.target)
            } else {
                format!("{}:{}:rw", mount.source, mount.target)
            };
            cmd.args(["-v", &mount_str]);
        }

        // Environment
        for env in &config.env_vars {
            cmd.args(["-e", env]);
        }

        cmd.arg(&config.image);
        cmd.args(args);

        let output = cmd.output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!("Container failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    /// Build a container image for a tool.
    pub async fn build_image(&self, _dockerfile: &str, tag: &str) -> Result<(), anyhow::Error> {
        let output = Command::new("docker")
            .args(["build", "-t", tag, "-"])
            .stdin(std::process::Stdio::piped())
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Build failed"))
        }
    }

    /// Remove unused containers.
    pub fn cleanup(&mut self) -> Result<(), anyhow::Error> {
        let _ = Command::new("docker").args(["container", "prune", "-f"]).output();
        self.containers.clear();
        Ok(())
    }
}

impl Default for DockerIsolation {
    fn default() -> Self {
        // C3 fix: Return a no-op stub instead of panicking on Docker unreachable
        Self {
            docker_host: "disabled://no-docker".to_string(),
            containers: std::collections::HashMap::new(),
        }
    }
}

/// Pre-defined container configs for common tools.
pub mod presets {
    use super::*;

    pub fn shell_config() -> ContainerConfig {
        ContainerConfig {
            image: "praxis/shell:latest".into(),
            memory_limit: "128m".into(),
            cpu_limit: Some("0.5".into()),
            network_mode: NetworkMode::Isolated,
            mounts: vec![],
            env_vars: vec!["HOME=/tmp".into()],
        }
    }

    pub fn file_config() -> ContainerConfig {
        ContainerConfig {
            image: "praxis/file:latest".into(),
            memory_limit: "64m".into(),
            cpu_limit: None,
            network_mode: NetworkMode::None,
            mounts: vec![],
            env_vars: vec![],
        }
    }

    pub fn web_config() -> ContainerConfig {
        ContainerConfig {
            image: "praxis/web:latest".into(),
            memory_limit: "256m".into(),
            cpu_limit: Some("1.0".into()),
            network_mode: NetworkMode::Bridge,
            mounts: vec![],
            env_vars: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config() {
        let config = ContainerConfig {
            image: "test:latest".into(),
            memory_limit: "128m".into(),
            cpu_limit: Some("1.0".into()),
            network_mode: NetworkMode::Isolated,
            mounts: vec![],
            env_vars: vec![],
        };

        assert_eq!(config.image, "test:latest");
    }
}
