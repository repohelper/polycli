use crate::utils::config::Config;
use anyhow::Result;
use chrono::Local;
use colored::Colorize as _;

#[allow(clippy::needless_pass_by_value)]
pub fn execute(config: Config, name: Option<String>, quiet: bool) -> Result<()> {
    let codex_dir = config.codex_dir();
    let backup_dir = config.backup_dir();

    if !codex_dir.exists() {
        anyhow::bail!("Codex directory not found at {}", codex_dir.display());
    }

    let backup_name =
        name.unwrap_or_else(|| Local::now().format("backup_%Y%m%d_%H%M%S").to_string());

    let backup_path = backup_dir.join(&backup_name);

    // Create backup
    crate::utils::files::copy_dir_recursive(codex_dir, &backup_path)?;

    if !quiet {
        println!(
            "{} Backup created: {}",
            "✓".green().bold(),
            backup_name.cyan()
        );
        println!("  {}: {}", "Location".dimmed(), backup_path.display());
    }

    Ok(())
}
