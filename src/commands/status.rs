use anyhow::Result;
use colored::Colorize as _;

use crate::utils::auth::read_email_from_codex_dir;
use crate::utils::config::Config;

pub async fn execute(config: Config, quiet: bool) -> Result<()> {
    let codex_dir = config.codex_dir();

    if !quiet {
        println!("{} {}", "Codexo".bold(), env!("CARGO_PKG_VERSION"));
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
            // Extract current email
            let Some(email) = read_email_from_codex_dir(codex_dir).await else {
                println!(
                    "  {}: {}",
                    "Current Email".dimmed(),
                    "not logged in".yellow()
                );
                show_profiles_info(config, quiet);
                return Ok(());
            };

            println!("  {}: {}", "Current Email".dimmed(), email.green());

            // List critical files
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
                            .map(|entry| entry.path().is_dir())
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
