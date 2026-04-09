use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use dialoguer::{Select, theme::ColorfulTheme};

pub async fn execute(config: Config, _quiet: bool) -> Result<()> {
    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║         Codexo - Setup Wizard                 ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    // Check codex installation
    let codex_installed = which::which("codex").is_ok();

    if !codex_installed {
        println!("{}", "⚠ Codex CLI not found in PATH".yellow());
        println!(
            "  Install it first: {}",
            "npm install -g @openai/codex-cli".cyan()
        );
        return Ok(());
    }

    println!("{} Codex CLI found", "✓".green());
    println!();

    // Main menu
    loop {
        let options = vec![
            "Save current Codex auth as a profile",
            "List existing profiles",
            "Load/switch to a profile",
            "Delete a profile",
            "Show status",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let name = dialoguer::Input::<String>::new()
                    .with_prompt("Profile name")
                    .interact()?;
                let desc = dialoguer::Input::<String>::new()
                    .with_prompt("Description (optional)")
                    .allow_empty(true)
                    .interact()
                    .ok();

                crate::commands::save::execute(
                    config.clone(),
                    name,
                    desc.filter(|s| !s.is_empty()),
                    true,
                    false,
                    None,
                )
                .await?;
            }
            1 => {
                crate::commands::list::execute(config.clone(), false, false).await?;
            }
            2 => {
                let name = dialoguer::Input::<String>::new()
                    .with_prompt("Profile name to load")
                    .interact()?;
                crate::commands::load::execute(config.clone(), name, true, false, false, None).await?;
            }
            3 => {
                let name = dialoguer::Input::<String>::new()
                    .with_prompt("Profile name to delete")
                    .interact()?;
                crate::commands::delete::execute(config.clone(), name, true, false).await?;
            }
            4 => {
                crate::commands::status::execute(config.clone(), false).await?;
            }
            _ => break,
        }

        println!();
        let continue_setup = dialoguer::Confirm::new()
            .with_prompt("Continue with setup?")
            .default(true)
            .interact()?;

        if !continue_setup {
            break;
        }

        println!();
    }

    println!("\n{} Setup complete!", "✓".green().bold());
    println!("  Run {} for more commands", "codexo --help".cyan());

    Ok(())
}
