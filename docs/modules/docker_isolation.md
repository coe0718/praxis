# Docker Isolation

> Docker isolation mode — per-tool container isolation. Tools execute in isolated Docker containers with resource limits, each getting its own container with specific capabilities and mounts.

## Overview

The `docker_isolation` module provides secure, containerized execution for Praxis tools. A `ContainerConfig` defines the image, memory/CPU limits, network mode (`Isolated` — no network, `Bridge` — shared, `None` — default), mount specifications (with security validation against path traversal), and environment variables.

The `DockerIsolation` manager builds and runs `docker run` commands with configurable resource constraints. It includes mount source validation that blocks sensitive paths (`/:/hostfs`, `/var/run/docker.sock`, `/etc/passwd`, `/root/.ssh`, `/home`) and canonicalization checks against `docker.sock` and `ssh` references.

A `presets` module provides pre-defined container configurations for shell, file, and web tools. The `Default` implementation returns a no-op stub with `docker_host: "disabled://no-docker"` (C3 fix) instead of panicking on Docker unreachable.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `ContainerConfig` | Container settings: `image`, `memory_limit`, `cpu_limit` (optional), `network_mode`, `mounts`, `env_vars`. |
| `NetworkMode` | Enum: `Isolated` (no network), `Bridge` (shared), `None` (default). |
| `MountSpec` | Bind mount: `source`, `target`, `readonly`. |
| `DockerIsolation` | Manager: `docker_host`, `containers` map. Methods: `execute_isolated()`, `build_image()`, `cleanup()`. |
| `ContainerInfo` | Runtime container record: `container_id`, `image`, `created_at`, `exited`. |

### Mount Security Validation

The `validate_mount_source()` method enforces:
- Blocks paths starting with or matching sensitive prefixes: `/:/hostfs`, `/var/run/docker.sock`, `/etc/passwd`, `/root/.ssh`, `/home`
- Only allows mounts under `/mnt/docker`, `/home`, or `/tmp`
- Canonicalizes the path and checks for `docker.sock` or `ssh` in the resolved path

## Public API

```rust
// Container config
pub struct ContainerConfig {
    pub image: String,
    pub memory_limit: String,
    pub cpu_limit: Option<String>,
    pub network_mode: NetworkMode,
    pub mounts: Vec<MountSpec>,
    pub env_vars: Vec<String>,
}

pub enum NetworkMode { Isolated, Bridge, None }

pub struct MountSpec {
    pub source: String,
    pub target: String,
    pub readonly: bool,
}

// Docker isolation manager
pub struct DockerIsolation;
impl DockerIsolation {
    pub fn new() -> Result<Self, anyhow::Error>;
    pub async fn execute_isolated(&mut self, tool_name: &str, config: &ContainerConfig, args: &[String]) -> Result<String>;
    pub async fn build_image(&self, _dockerfile: &str, tag: &str) -> Result<(), anyhow::Error>;
    pub fn cleanup(&mut self) -> Result<(), anyhow::Error>;
}
impl Default for DockerIsolation;

// Pre-defined configurations
pub mod presets {
    pub fn shell_config() -> ContainerConfig;
    pub fn file_config() -> ContainerConfig;
    pub fn web_config() -> ContainerConfig;
}
```

## Configuration

No `praxis.toml` section. Containers are configured programmatically via `ContainerConfig`.

### Example

```rust
let mut docker = DockerIsolation::new()?;

// Execute a shell command in an isolated container
let output = docker.execute_isolated(
    "shell-exec",
    &presets::shell_config(),
    &["echo".into(), "hello".into()],
).await?;
println!("Output: {}", output);

// Execute a web fetch in bridged network mode
let output = docker.execute_isolated(
    "web-fetch",
    &presets::web_config(),
    &["curl".into(), "https://example.com".into()],
).await?;

// Clean up unused containers
docker.cleanup()?;
```

### Preset Configurations

| Preset | Image | Memory | CPU | Network |
|--------|-------|--------|-----|---------|
| `shell_config` | praxis/shell:latest | 128m | 0.5 | Isolated |
| `file_config` | praxis/file:latest | 64m | None | None |
| `web_config` | praxis/web:latest | 256m | 1.0 | Bridge |

## Security

- **Mount validation** prevents path traversal and Docker socket exposure.
- **Container naming** uses a random suffix to prevent name collisions in the same nanosecond (C12 fix).
- **Network isolation** via `NetworkMode::Isolated` for untrusted tools.
- **Default stub** returns a no-op manager when Docker is not available (C3 fix), avoiding panics.

## Dependencies

- `rand` — random container name suffix
- `chrono` — container creation timestamps
- `serde` / `serde_json` — serialization
- `anyhow` — error handling
- `std::process::Command` — Docker CLI invocation

## Source

`src/docker_isolation.rs`