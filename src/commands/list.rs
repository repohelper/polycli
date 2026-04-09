use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};

use crate::utils::config::Config;

pub async fn execute(config: Config, detailed: bool, quiet: bool) -> Result<()> {
    let profiles_dir = config.profiles_dir();

    if !profiles_dir.exists() {
        if !quiet {
            println!("{} No profiles found.", "ℹ".blue());
            println!(
                "  Create your first profile with: {}",
                "poly save <name>".cyan()
            );
        }
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(profiles_dir).await?;
    let mut profiles = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip internal directories
        if name == "backups" {
            continue;
        }

        let meta_path = path.join("profile.json");

        let meta = if meta_path.exists() {
            let content = tokio::fs::read_to_string(&meta_path).await.ok();
            content
                .and_then(|c| serde_json::from_str::<crate::utils::profile::ProfileMeta>(&c).ok())
        } else {
            None
        };

        profiles.push((name, meta));
    }

    if profiles.is_empty() {
        if !quiet {
            println!("{} No profiles found.", "ℹ".blue());
            println!(
                "  Create your first profile with: {}",
                "poly save <name>".cyan()
            );
        }
        return Ok(());
    }

    // Sort by name
    profiles.sort_by(|a, b| a.0.cmp(&b.0));

    if detailed {
        // Detailed table view
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        // Header
        table.add_row(Row::new(vec![
            Cell::new("Profile").style_spec("Fb"),
            Cell::new("Email").style_spec("Fb"),
            Cell::new("Last Updated").style_spec("Fb"),
            Cell::new("Description").style_spec("Fb"),
        ]));

        for (name, meta) in profiles {
            let email = meta
                .as_ref()
                .and_then(|m| m.email.clone())
                .unwrap_or_else(|| "-".to_string());
            let updated = meta
                .as_ref()
                .map(|m| m.updated_at.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let desc = meta
                .as_ref()
                .and_then(|m| m.description.clone())
                .unwrap_or_else(|| "-".to_string());

            table.add_row(Row::new(vec![
                Cell::new(&name).style_spec("Fg"),
                Cell::new(&email),
                Cell::new(&updated),
                Cell::new(&desc),
            ]));
        }

        table.printstd();
    } else if !quiet {
        // Simple list view
        println!("{} Saved Profiles:\n", "✓".green().bold());

        for (name, meta) in &profiles {
            let email_str = meta
                .as_ref()
                .and_then(|m| m.email.as_ref())
                .map(|e| format!(" ({})", e.dimmed()))
                .unwrap_or_default();

            println!("  {} {}{}", "•".cyan(), name.bold(), email_str);
        }

        println!("\n{} profiles total", profiles.len());
    }

    Ok(())
}
