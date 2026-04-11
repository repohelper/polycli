use crate::utils::config::Config;
use crate::utils::validation::ProfileName;
use anyhow::{Context as _, Result};
use colored::Colorize as _;

pub async fn execute(config: Config, name: String, force: bool, quiet: bool) -> Result<()> {
    let profile_name = ProfileName::try_from(name.as_str())
        .with_context(|| format!("Invalid profile name '{name}'"))?;
    let profile_dir = config.profile_path_validated(&profile_name)?;

    if !profile_dir.exists() {
        anyhow::bail!("Profile '{name}' not found");
    }

    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete profile '{}' permanently?", name.yellow()))
            .default(false)
            .interact()?;

        if !confirm {
            if !quiet {
                println!("Cancelled");
            }
            return Ok(());
        }
    }

    tokio::fs::remove_dir_all(&profile_dir).await?;

    if !quiet {
        println!("{} Profile {} deleted", "✓".green().bold(), name.cyan());
    }

    Ok(())
}
