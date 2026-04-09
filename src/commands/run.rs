use crate::utils::config::Config;
use crate::utils::transaction::ProfileTransaction;
use crate::utils::validation::ProfileName;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use std::process::Stdio;
use tokio::process::Command;

pub async fn execute(
    config: Config,
    profile: String,
    command: Vec<String>,
    quiet: bool,
) -> Result<()> {
    let profile_name = ProfileName::try_from(profile.as_str())
        .with_context(|| format!("Invalid profile name '{profile}'"))?;
    let profile_dir = config.profile_path_validated(&profile_name)?;
    let codex_dir = config.codex_dir();

    if !profile_dir.exists() {
        anyhow::bail!("Profile '{}' not found", profile);
    }

    if command.is_empty() {
        anyhow::bail!("No command specified to run");
    }

    // Atomically switch to the target profile.
    // The transaction saves the original codex dir internally so it can be
    // restored atomically after the command finishes.
    let mut txn = ProfileTransaction::new(codex_dir)
        .context("Failed to initialise profile transaction")?;
    txn.stage_profile(&profile_dir, crate::utils::files::get_critical_files())
        .context("Failed to stage profile")?;
    txn.commit()
        .context("Failed to atomically load profile for run")?;

    // Execute command
    let cmd = &command[0];
    let args = &command[1..];

    if !quiet {
        println!(
            "{} Running with profile {}: {}",
            "▶".cyan(),
            profile.green(),
            command.join(" ").dimmed()
        );
    }

    // Log to history
    let _ = crate::commands::history::log_command(&config, &profile, &command.join(" ")).await;

    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("Failed to execute command: {}", cmd))?;

    // Atomically restore the original profile.
    // rollback() renames the saved original back into place — no partial state.
    if let Err(e) = txn.rollback() {
        if !quiet {
            eprintln!(
                "{} Warning: Could not fully restore original profile: {}",
                "⚠".yellow(),
                e
            );
        }
    }

    if !quiet {
        if status.success() {
            println!(
                "\n{} Command completed, restored original profile",
                "✓".green()
            );
        } else {
            println!(
                "\n{} Command exited with code {:?}",
                "!".yellow(),
                status.code()
            );
        }
    }

    Ok(())
}
