use crate::utils::auth::read_email_from_codex_dir;
use crate::utils::config::Config;
use crate::utils::files::{copy_profile_files, get_critical_files};
use crate::utils::profile::Profile;
use crate::utils::validation::ProfileName;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn execute(
    config: Config,
    name: String,
    description: Option<String>,
    force: bool,
    quiet: bool,
    passphrase: Option<String>,
) -> Result<()> {
    let profile_name = ProfileName::try_from(name.as_str())
        .with_context(|| format!("Invalid profile name '{name}'"))?;
    let codex_dir = config.codex_dir();
    let profile_dir = config.profile_path_validated(&profile_name)?;

    if !codex_dir.exists() {
        anyhow::bail!(
            "Codex directory not found at {}. Is Codex CLI installed?",
            codex_dir.display()
        );
    }

    if profile_dir.exists() && !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Profile '{}' already exists. Overwrite?",
                name.yellow()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            if !quiet {
                println!("Cancelled");
            }
            return Ok(());
        }
    }

    // Create progress bar with modern styling (unless quiet)
    let pb = if quiet {
        None
    } else {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Valid template"),
        );
        bar.set_message("Saving profile...");
        bar.enable_steady_tick(Duration::from_millis(100));
        Some(bar)
    };

    // Extract email from existing auth.json if present
    let email = read_email_from_codex_dir(codex_dir).await;

    // Create profile
    let mut profile = Profile::new(name.clone(), email.clone(), description.clone());

    // Copy critical files
    let files_to_copy = get_critical_files();
    let copied = copy_profile_files(codex_dir, &profile_dir, files_to_copy)
        .with_context(|| "Failed to copy profile files")?;

    // Add files to profile object
    for file in &copied {
        let file_path = profile_dir.join(file.split('/').next().unwrap_or(file));
        if file_path.exists() && file_path.is_file() {
            let Ok(content) = tokio::fs::read(&file_path).await else {
                continue;
            };
            profile.add_file(&file_path, content);
        }
    }

    // Handle passphrase
    let secret_passphrase = passphrase.filter(|p| !p.is_empty());
    let is_encrypted = secret_passphrase.is_some();

    // Save profile with optional encryption
    profile.save_to_disk_encrypted(&profile_dir, secret_passphrase.as_ref())
        .with_context(|| "Failed to save profile")?;

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    // Success message
    if !quiet {
        let encryption_status = if is_encrypted {
            format!(" {}", "[encrypted]".cyan())
        } else {
            String::new()
        };
        println!(
            "{} Profile {} saved successfully{}",
            "✓".green().bold(),
            name.cyan(),
            encryption_status
        );

        if let Some(e) = email {
            println!("  {}: {}", "Email".dimmed(), e);
        }
        println!("  {}: {}", "Location".dimmed(), profile_dir.display());
        println!("  {}: {} files", "Files".dimmed(), copied.len());
    }

    Ok(())
}

