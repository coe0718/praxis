use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    paths::{PraxisPaths, default_data_dir},
    vault::{Vault, VaultEntry, audit_literals},
};

#[derive(Debug, Args)]
pub struct VaultArgs {
    #[command(subcommand)]
    command: VaultCommands,
}

#[derive(Debug, Subcommand)]
enum VaultCommands {
    /// List all secret names in the vault (values are never shown).
    List,
    /// Add or replace a secret entry backed by an environment variable.
    SetEnv(SetEnvArgs),
    /// Add or replace a secret entry with a literal value (dev use only).
    SetLiteral(SetLiteralArgs),
    /// Remove a named secret.
    Remove(RemoveSecretArgs),
    /// Resolve and print the current value of a named secret.
    Resolve(ResolveSecretArgs),
    /// Audit the vault for literal-value entries.
    Audit,
}

#[derive(Debug, Args)]
struct SetEnvArgs {
    /// Alias name for the secret.
    name: String,
    /// Environment variable to read the secret from at runtime.
    #[arg(long)]
    env: String,
    /// Optional fallback literal if the env var is unset.
    #[arg(long)]
    fallback: Option<String>,
}

#[derive(Debug, Args)]
struct SetLiteralArgs {
    /// Alias name for the secret.
    name: String,
    /// Literal secret value (prefer env-var references in production).
    #[arg(long)]
    value: String,
}

#[derive(Debug, Args)]
struct RemoveSecretArgs {
    /// Name of the secret to remove.
    name: String,
}

#[derive(Debug, Args)]
struct ResolveSecretArgs {
    /// Name of the secret to resolve.
    name: String,
}

pub(super) fn handle_vault(
    data_dir_override: Option<PathBuf>,
    args: VaultArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        VaultCommands::List => {
            let vault = Vault::load(&paths.vault_file)?;
            Ok(vault.summary())
        }
        VaultCommands::SetEnv(a) => {
            let mut vault = Vault::load(&paths.vault_file)?;
            vault.set(
                &a.name,
                VaultEntry::EnvVar {
                    env: a.env.clone(),
                    fallback: a.fallback,
                },
            );
            vault.save(&paths.vault_file)?;
            Ok(format!("vault: '{}' set to env var '{}'", a.name, a.env))
        }
        VaultCommands::SetLiteral(a) => {
            let mut vault = Vault::load(&paths.vault_file)?;
            vault.set(&a.name, VaultEntry::Literal { value: a.value });
            vault.save(&paths.vault_file)?;
            Ok(format!(
                "vault: '{}' set as literal value (consider using --env in production)",
                a.name
            ))
        }
        VaultCommands::Remove(a) => {
            let mut vault = Vault::load(&paths.vault_file)?;
            if vault.remove(&a.name) {
                vault.save(&paths.vault_file)?;
                Ok(format!("vault: '{}' removed", a.name))
            } else {
                Ok(format!("vault: '{}' not found", a.name))
            }
        }
        VaultCommands::Resolve(a) => {
            let vault = Vault::load(&paths.vault_file)?;
            let value = vault.resolve(&a.name)?;
            Ok(value)
        }
        VaultCommands::Audit => {
            let vault = Vault::load(&paths.vault_file)?;
            let warnings = audit_literals(&vault);
            if warnings.is_empty() {
                Ok("vault: no literal entries found — all secrets use env-var references".to_string())
            } else {
                Ok(warnings.join("\n"))
            }
        }
    }
}
