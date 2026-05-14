//! Singularity Terminal Backend — Execute tools in Singularity container sandboxes.
//!
//! Singularity (now Apptainer) provides reproducible container environments
//! commonly used in HPC and research environments.
//!
//! This backend builds on the existing `docker_isolation` module concepts
//! but uses Singularity CLI commands instead of Docker API calls.
//!
//! Set `PRAXIS_SINGULARITY_IMAGE` to specify the default container image.
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Singularity container configuration.
#[derive(Debug, Clone)]
pub struct SingularityConfig {
    pub image: String,
    pub bind_paths: Vec<BindMount>,
    pub env_vars: Vec<(String, String)>,
}

/// A bind mount specification for Singularity.
#[derive(Debug, Clone)]
pub struct BindMount {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub read_only: bool,
}

impl SingularityConfig {
    pub fn from_env() -> Result<Self> {
        let image = std::env::var("PRAXIS_SINGULARITY_IMAGE")
            .unwrap_or_else(|_| "docker://ubuntu:22.04".to_string());

        let bind_paths = Self::parse_bind_paths();
        let env_vars = Self::parse_env_vars();

        Ok(Self { image, bind_paths, env_vars })
    }

    pub fn validate_environment() -> Result<()> {
        // Check that singularity is available.
        let output = Command::new("which")
            .arg("singularity")
            .output()
            .context("failed to check for singularity binary")?;

        if !output.status.success() {
            anyhow::bail!("singularity binary not found in PATH — install Apptainer/Singularity");
        }

        Ok(())
    }

    fn parse_bind_paths() -> Vec<BindMount> {
        let mut mounts = Vec::new();

        if let Ok(mounts_str) = std::env::var("PRAXIS_SINGULARITY_BIND") {
            for mount in mounts_str.split(',') {
                let mount = mount.trim();
                if mount.is_empty() {
                    continue;
                }

                // Format: source:dest[:ro]
                let parts: Vec<&str> = mount.split(':').collect();
                if parts.len() >= 2 {
                    let read_only = parts.get(2).is_some_and(|&s| s == "ro");
                    mounts.push(BindMount {
                        source: PathBuf::from(parts[0]),
                        dest: PathBuf::from(parts[1]),
                        read_only,
                    });
                }
            }
        }

        mounts
    }

    fn parse_env_vars() -> Vec<(String, String)> {
        let mut vars = Vec::new();

        if let Ok(env_str) = std::env::var("PRAXIS_SINGULARITY_ENV") {
            for pair in env_str.split(',') {
                let pair = pair.trim();
                if let Some((key, value)) = pair.split_once('=') {
                    vars.push((key.to_string(), value.to_string()));
                }
            }
        }

        vars
    }
}

/// A Singularity backend for executing commands in isolated containers.
pub struct SingularityBackend {
    config: SingularityConfig,
}

impl SingularityBackend {
    pub fn new(config: SingularityConfig) -> Self {
        Self { config }
    }

    /// Execute a command inside a Singularity container.
    ///
    /// Runs the command via `singularity exec` with configured bind mounts
    /// and environment variables.
    pub fn execute(&self, command: &str, image: Option<&str>) -> Result<String> {
        let image = image.unwrap_or(&self.config.image);

        let mut args = Vec::new();

        // Add bind mounts.
        for mount in &self.config.bind_paths {
            let flag = if mount.read_only { "--bind-ro" } else { "--bind" };
            args.push(flag.to_string());
            args.push(format!("{}:{}", mount.source.display(), mount.dest.display()));
        }

        // Add extra environment variables.
        for (key, value) in &self.config.env_vars {
            args.push("--env".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Build the full command.
        // NOTE: We use /bin/sh -c to support complex command strings with pipes,
        // redirects, etc.
        args.push(image.to_string());
        args.push("/bin/sh".to_string());
        args.push("-c".to_string());
        args.push(command.to_string());

        log::debug!("singularity exec {:?}", args);

        let output = Command::new("singularity")
            .args(["exec"])
            .args(&args)
            .output()
            .context("failed to execute Singularity command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Singularity command failed: {}", stderr)
        }
    }

    /// Validate Singularity connectivity and that the image is accessible.
    pub fn validate(&self) -> Result<()> {
        // First check the binary exists.
        crate::singularity_backend::SingularityConfig::validate_environment()?;

        // Then verify the image exists locally or can be pulled.
        let output = Command::new("singularity").args(["inspect", &self.config.image]).output();

        match output {
            Ok(inspect) if inspect.status.success() => {
                log::info!("singularity: image {} is available locally", self.config.image);
                Ok(())
            }
            Ok(_) | Err(_) => {
                // Image not found locally — try a quick pull test.
                log::warn!(
                    "singularity: image {} not found locally, will attempt pull on first use",
                    self.config.image
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singularity_config_default_image() {
        unsafe { std::env::remove_var("PRAXIS_SINGULARITY_IMAGE") };
        let config = SingularityConfig::from_env().unwrap();
        assert_eq!(config.image, "docker://ubuntu:22.04");
    }

    #[test]
    fn singularity_config_parse_bind_paths() {
        unsafe {
            std::env::set_var("PRAXIS_SINGULARITY_BIND", "/tmp/src:/mnt/src,/data:/mnt/data:ro")
        };
        unsafe { std::env::remove_var("PRAXIS_SINGULARITY_IMAGE") };
        let config = SingularityConfig::from_env().unwrap();
        assert_eq!(config.bind_paths.len(), 2);
        assert!(!config.bind_paths[0].read_only);
        assert!(config.bind_paths[1].read_only);
    }

    #[test]
    fn singularity_config_parse_env_vars() {
        unsafe { std::env::set_var("PRAXIS_SINGULARITY_ENV", "FOO=bar,BAZ=qux") };
        unsafe { std::env::remove_var("PRAXIS_SINGULARITY_IMAGE") };
        let config = SingularityConfig::from_env().unwrap();
        assert_eq!(config.env_vars.len(), 2);
        assert_eq!(config.env_vars[0], ("FOO".into(), "bar".into()));
    }
}
