use anyhow::Result;
use colored::Colorize as _;

use crate::utils::auth::{
    auth_mode_has_api_key, auth_mode_has_chatgpt, auth_mode_label, detect_auth_mode,
    read_email_from_codex_dir,
};
use crate::utils::config::Config;

pub async fn execute(config: Config, quiet: bool) -> Result<()> {
    let codex_dir = config.codex_dir();

    if !quiet {
        println!(
            "{} {}",
            "Codex Controller".bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!();

        // Check if codex is installed
        let codex_installed = which::which("codex").is_ok();
        println!(
            "  {}: {}",
            "(`Codex` CLI)".dimmed(),
            if codex_installed {
                "installed ✓".green()
            } else {
                "not found".red()
            }
        );

        // Check codex directory
        println!("  {}: {}", "`Codex` Dir".dimmed(), codex_dir.display());

        if codex_dir.exists() {
            let current_profile =
                read_profile_marker(config.profiles_dir(), ".current_profile").await;
            let previous_profile =
                read_profile_marker(config.profiles_dir(), ".previous_profile").await;

            println!(
                "  {}: {}",
                "Current Profile".dimmed(),
                format_profile_marker(&current_profile)
            );
            if let Some(previous) = previous_profile {
                println!("  {}: {}", "Previous Profile".dimmed(), previous.cyan());
            }

            let auth_path = codex_dir.join("auth.json");
            if !auth_path.exists() {
                println!("  {}: {}", "Auth Mode".dimmed(), "not logged in".yellow());
                show_profiles_info(config, quiet);
                return Ok(());
            }

            match tokio::fs::read_to_string(&auth_path).await {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(auth_json) => {
                        let auth_mode = detect_auth_mode(&auth_json);
                        println!(
                            "  {}: {}",
                            "Auth Mode".dimmed(),
                            auth_mode_label(&auth_mode).cyan()
                        );

                        if let Some(email) = read_email_from_codex_dir(codex_dir).await {
                            println!("  {}: {}", "Current Email".dimmed(), email.green());
                        } else {
                            println!(
                                "  {}: {}",
                                "Current Email".dimmed(),
                                "not available in this auth mode".yellow()
                            );
                        }

                        println!(
                            "  {}: {}",
                            "Plan Claims".dimmed(),
                            capability_status(auth_mode_has_chatgpt(&auth_mode))
                        );
                        println!(
                            "  {}: {}",
                            "API Realtime".dimmed(),
                            capability_status(auth_mode_has_api_key(&auth_mode))
                        );
                    }
                    Err(_) => {
                        println!("  {}: {}", "Auth Mode".dimmed(), "invalid auth.json".red());
                    }
                },
                Err(_) => {
                    println!(
                        "  {}: {}",
                        "Auth Mode".dimmed(),
                        "unreadable auth.json".red()
                    );
                }
            }

            let files = Config::critical_files();
            println!("\n  {} Critical Files:", "•".cyan());
            for file in files {
                let file_path = codex_dir.join(file);
                let exists = file_path.exists();
                let status = if exists { "✓".green() } else { "✗".red() };
                let missing_msg = if exists { "" } else { "(missing)" };
                println!(
                    "    {status} {file} {missing_msg}",
                    missing_msg = missing_msg.dimmed()
                );
            }
        } else {
            println!("  {}: {}", "Status".dimmed(), "Not configured".yellow());
        }

        show_profiles_info(config, quiet);
    }

    Ok(())
}

async fn read_profile_marker(profiles_dir: &std::path::Path, marker_name: &str) -> Option<String> {
    let marker = profiles_dir.join(marker_name);
    let content = tokio::fs::read_to_string(marker).await.ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn format_profile_marker(marker: &Option<String>) -> colored::ColoredString {
    match marker {
        Some(name) => name.cyan(),
        None => "ad hoc / untracked".yellow(),
    }
}

fn capability_status(available: bool) -> colored::ColoredString {
    if available {
        "available".green()
    } else {
        "not available".yellow()
    }
}

#[allow(clippy::needless_pass_by_value)]
fn show_profiles_info(config: Config, quiet: bool) {
    // Show profiles count
    let profiles_dir = config.profiles_dir();
    let profile_count = if profiles_dir.exists() {
        std::fs::read_dir(profiles_dir)
            .map(|entries| {
                entries
                    .filter(|e| {
                        e.as_ref()
                            .map(|entry| {
                                let path = entry.path();
                                if !path.is_dir() {
                                    return false;
                                }
                                let name = entry.file_name().to_string_lossy().to_string();
                                name != "backups" && !name.starts_with('.')
                            })
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    if !quiet {
        println!(
            "\n  {}: {} saved profiles",
            "Profiles".dimmed(),
            profile_count.to_string().cyan()
        );
        println!("  {}: {}", "Profile Dir".dimmed(), profiles_dir.display());
    }
}
