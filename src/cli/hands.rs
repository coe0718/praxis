use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    hands::{HandStore, remove_hand},
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct HandsArgs {
    #[command(subcommand)]
    command: HandsCommands,
}

#[derive(Debug, Subcommand)]
enum HandsCommands {
    /// List all installed hand manifests.
    List,
    /// Show details for a specific hand.
    Show(ShowHandArgs),
    /// Remove an installed hand by name.
    Remove(RemoveHandArgs),
}

#[derive(Debug, Args)]
struct ShowHandArgs {
    /// Name of the hand to show.
    name: String,
}

#[derive(Debug, Args)]
struct RemoveHandArgs {
    /// Name of the hand to remove.
    name: String,
}

pub(super) fn handle_hands(
    data_dir_override: Option<PathBuf>,
    args: HandsArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        HandsCommands::List => {
            let store = HandStore::load(&paths.hands_dir)?;
            Ok(store.summary())
        }
        HandsCommands::Show(a) => {
            let store = HandStore::load(&paths.hands_dir)?;
            match store.get(&a.name) {
                Some(hand) => {
                    let mut lines = vec![
                        format!("name: {}", hand.name),
                        format!("version: {}", hand.version),
                        format!("description: {}", hand.description),
                    ];
                    if !hand.tools.required.is_empty() {
                        lines.push(format!("tools.required: {}", hand.tools.required.join(", ")));
                    }
                    if !hand.tools.optional.is_empty() {
                        lines.push(format!("tools.optional: {}", hand.tools.optional.join(", ")));
                    }
                    if !hand.skills.load.is_empty() {
                        lines.push(format!("skills: {}", hand.skills.load.join(", ")));
                    }
                    if !hand.schedule.quiet_hours.is_empty() {
                        let hours: Vec<String> =
                            hand.schedule.quiet_hours.iter().map(|h| h.to_string()).collect();
                        lines.push(format!("quiet_hours: {}", hours.join(", ")));
                    }
                    if !hand.metadata.tags.is_empty() {
                        lines.push(format!("tags: {}", hand.metadata.tags.join(", ")));
                    }
                    Ok(lines.join("\n"))
                }
                None => Ok(format!("hands: '{}' not found", a.name)),
            }
        }
        HandsCommands::Remove(a) => {
            if remove_hand(&paths, &a.name)? {
                Ok(format!("hands: '{}' removed", a.name))
            } else {
                Ok(format!("hands: '{}' not found", a.name))
            }
        }
    }
}
