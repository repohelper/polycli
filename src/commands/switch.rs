use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use dialoguer::Select;

pub async fn execute(config: Config, quiet: bool) -> Result<()> {
    let profiles_dir = config.profiles_dir();

    if !profiles_dir.exists() {
        anyhow::bail!("No profiles directory found. Create a profile first with: poly save <name>");
    }

    // Collect profiles
    let mut entries = tokio::fs::read_dir(&profiles_dir).await?;
    let mut profiles = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip hidden/system directories
            if name.starts_with('.') || name == "backups" {
                continue;
            }

            // Load metadata if available
            let meta_path = path.join("profile.json");
            let description = if meta_path.exists() {
                let content = tokio::fs::read_to_string(&meta_path).await.ok();
                content.and_then(|c| {
                    serde_json::from_str::<serde_json::Value>(&c)
                        .ok()
                        .and_then(|v| {
                            v.get("description")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string())
                        })
                })
            } else {
                None
            };

            profiles.push((name, description));
        }
    }

    if profiles.is_empty() {
        anyhow::bail!("No profiles found. Create one with: poly save <name>");
    }

    // Sort profiles
    profiles.sort_by(|a, b| a.0.cmp(&b.0));

    // Create display strings
    let display_items: Vec<String> = profiles
        .iter()
        .map(|(name, desc)| {
            if let Some(d) = desc {
                format!("{} - {}", name.bold(), d.dimmed())
            } else {
                name.bold().to_string()
            }
        })
        .collect();

    // Use dialoguer for selection
    let selection = Select::new()
        .with_prompt("Select profile to switch to")
        .items(&display_items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(index) => {
            let (profile_name, _) = &profiles[index];

            if !quiet {
                println!(
                    "{} Switching to profile: {}",
                    "→".cyan(),
                    profile_name.green()
                );
            }

            // Load the profile
            crate::commands::load::execute(config, profile_name.clone(), true, false, quiet, None)
                .await?;
        }
        None => {
            println!("Cancelled");
        }
    }

    Ok(())
}
