use anyhow::Result;
use colored::Colorize as _;
use serde::Serialize;

use crate::utils::auth::{
    auth_mode_has_api_key, auth_mode_has_chatgpt, auth_mode_label, detect_auth_mode,
    read_email_from_codex_dir,
};
use crate::utils::config::Config;

pub async fn execute(config: Config, json: bool, quiet: bool) -> Result<()> {
    let codex_dir = config.codex_dir();
    let summary = build_status_summary(&config).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

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
            let current_profile = summary.current_profile.clone();
            let previous_profile = summary.previous_profile.clone();

            println!(
                "  {}: {}",
                "Current Profile".dimmed(),
                format_profile_marker(&current_profile)
            );
            if let Some(previous) = previous_profile {
                println!("  {}: {}", "Previous Profile".dimmed(), previous.cyan());
            }

            if !summary.auth_file_present {
                println!("  {}: {}", "Auth Mode".dimmed(), "not logged in".yellow());
                show_profiles_info(config, quiet);
                return Ok(());
            }

            println!(
                "  {}: {}",
                "Auth Mode".dimmed(),
                auth_mode_label(&summary.auth_mode).cyan()
            );

            if let Some(email) = summary.current_email.as_ref() {
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
                capability_status(summary.plan_claims_available)
            );
            println!(
                "  {}: {}",
                "API Realtime".dimmed(),
                capability_status(summary.api_realtime_available)
            );

            println!("\n  {} Critical Files:", "•".cyan());
            for file in &summary.critical_files {
                let exists = file.exists;
                let status = if exists { "✓".green() } else { "✗".red() };
                let missing_msg = if exists { "" } else { "(missing)" };
                println!(
                    "    {status} {} {missing_msg}",
                    file.path,
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

#[derive(Debug, Serialize)]
struct StatusSummary {
    version: String,
    codex_installed: bool,
    codex_dir: String,
    profiles_dir: String,
    profile_count: usize,
    current_profile: Option<String>,
    previous_profile: Option<String>,
    auth_file_present: bool,
    auth_mode: String,
    auth_mode_label: String,
    current_email: Option<String>,
    plan_claims_available: bool,
    api_realtime_available: bool,
    critical_files: Vec<CriticalFileStatus>,
}

#[derive(Debug, Serialize)]
struct CriticalFileStatus {
    path: String,
    exists: bool,
}

async fn build_status_summary(config: &Config) -> StatusSummary {
    let codex_dir = config.codex_dir();
    let current_profile = read_profile_marker(config.profiles_dir(), ".current_profile").await;
    let previous_profile = read_profile_marker(config.profiles_dir(), ".previous_profile").await;
    let auth_path = codex_dir.join("auth.json");
    let auth_file_present = auth_path.exists();

    let auth_mode = if auth_file_present {
        tokio::fs::read_to_string(&auth_path)
            .await
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .map(|auth_json| detect_auth_mode(&auth_json))
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    let critical_files = Config::critical_files()
        .iter()
        .map(|file| CriticalFileStatus {
            path: (*file).to_string(),
            exists: codex_dir.join(file).exists(),
        })
        .collect();

    StatusSummary {
        version: env!("CARGO_PKG_VERSION").to_string(),
        codex_installed: which::which("codex").is_ok(),
        codex_dir: codex_dir.display().to_string(),
        profiles_dir: config.profiles_dir().display().to_string(),
        profile_count: count_profiles(config.profiles_dir()),
        current_profile,
        previous_profile,
        auth_file_present,
        auth_mode: auth_mode.clone(),
        auth_mode_label: auth_mode_label(&auth_mode).to_string(),
        current_email: read_email_from_codex_dir(codex_dir).await,
        plan_claims_available: auth_mode_has_chatgpt(&auth_mode),
        api_realtime_available: auth_mode_has_api_key(&auth_mode),
        critical_files,
    }
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
    let profiles_dir = config.profiles_dir();
    let profile_count = count_profiles(config.profiles_dir());

    if !quiet {
        println!(
            "\n  {}: {} saved profiles",
            "Profiles".dimmed(),
            profile_count.to_string().cyan()
        );
        println!("  {}: {}", "Profile Dir".dimmed(), profiles_dir.display());
    }
}

fn count_profiles(profiles_dir: &std::path::Path) -> usize {
    if !profiles_dir.exists() {
        return 0;
    }

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
                            !Config::is_reserved_entry_name(&name)
                        })
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}
