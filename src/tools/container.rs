//! Container runtime detection — Docker or Podman.
//!
//! Provides runtime detection and command builder for container-based
//! code execution. Prefers Podman if available (rootless by default).

use std::process::Command;

/// Detected container runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ContainerRuntime {
    /// Docker.
    Docker,
    /// Podman (preferred for rootless execution).
    Podman,
    /// No container runtime available.
    None,
}

impl ContainerRuntime {
    /// Detect available container runtime.
    /// Prefers Podman over Docker (rootless by default, no daemon required).
    #[allow(dead_code)]
    pub fn detect() -> Self {
        // Check for podman first (rootless, no daemon)
        if Self::check_podman() {
            return Self::Podman;
        }
        // Fall back to docker
        if Self::check_docker() {
            return Self::Docker;
        }
        Self::None
    }

    fn check_podman() -> bool {
        Command::new("podman")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn check_docker() -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get the command name for this runtime.
    #[allow(dead_code)]
    pub fn command(&self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::None => "",
        }
    }

    /// Check if this runtime is available.
    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        *self != Self::None
    }
}

/// Detect Nix environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct NixEnv {
    /// Running inside nix-shell.
    pub in_nix_shell: bool,
    /// nix-store path available.
    pub has_nix_store: bool,
}

impl NixEnv {
    /// Detect Nix environment.
    #[allow(dead_code)]
    pub fn detect() -> Self {
        let in_nix_shell = std::env::var("IN_NIX_SHELL").is_ok()
            || std::env::var("NIX_SHELL_STARTED_ONCE").is_ok();
        let has_nix_store = std::path::Path::new("/nix/store").exists();
        Self { in_nix_shell, has_nix_store }
    }

    /// Check if running in a Nix environment.
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.in_nix_shell || self.has_nix_store
    }
}
