use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};

async fn run_health_checks(config: &Config) -> Vec<(String, String, bool, String)> {
    let mut checks: Vec<(String, String, bool, String)> = Vec::new();
    let codex_installed = which::which("codex").is_ok();
    checks.push((
        "`Codex` CLI".to_string(),
        if codex_installed {
            "✓ Installed".to_string()
        } else {
            "✗ Not found".to_string()
        },
        codex_installed,
        if codex_installed {
            "none".to_string()
        } else {
            "Install with: npm install -g @openai/codex_cli".to_string()
        },
    ));
    let codex_dir = config.codex_dir();
    let codex_dir_exists = codex_dir.exists();
    checks.push((
        "`Codex` Directory".to_string(),
        if codex_dir_exists {
            "✓ Exists".to_string()
        } else {
            "✗ Missing".to_string()
        },
        codex_dir_exists,
        if codex_dir_exists {
            "none".to_string()
        } else {
            format!("Directory: {}", codex_dir.display())
        },
    ));
    let auth_file = codex_dir.join("auth.json");
    let auth_exists = auth_file.exists();
    checks.push((
        "Auth File".to_string(),
        if auth_exists {
            "✓ Exists".to_string()
        } else {
            "✗ Missing".to_string()
        },
        auth_exists,
        if auth_exists {
            "none".to_string()
        } else {
            "Run: codex login".to_string()
        },
    ));
    let mut auth_valid = false;
    let mut auth_error = String::new();
    if auth_exists {
        match tokio::fs::read_to_string(&auth_file).await {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(_) => auth_valid = true,
                Err(e) => auth_error = format!("Invalid JSON: {e}"),
            },
            Err(e) => auth_error = format!("Cannot read: {e}"),
        }
    }
    checks.push((
        "Auth Valid".to_string(),
        if auth_valid {
            "✓ Valid".to_string()
        } else {
            "✗ Invalid".to_string()
        },
        auth_valid,
        if auth_valid {
            "none".to_string()
        } else {
            auth_error
        },
    ));
    let profiles_dir = config.profiles_dir();
    let profiles_dir_exists = profiles_dir.exists();
    checks.push((
        "Profiles Directory".to_string(),
        if profiles_dir_exists {
            "✓ Exists".to_string()
        } else {
            "✗ Missing".to_string()
        },
        profiles_dir_exists,
        if profiles_dir_exists {
            "none".to_string()
        } else {
            format!("Will be created at: {}", profiles_dir.display())
        },
    ));
    let mut profile_count = 0;
    if profiles_dir_exists && let Ok(mut entries) = tokio::fs::read_dir(&profiles_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_dir() {
                profile_count += 1;
            }
        }
    }
    let has_profiles = profile_count > 0;
    checks.push((
        "Saved Profiles".to_string(),
        format!("✓ {profile_count} profile(s)"),
        has_profiles,
        if has_profiles {
            "none".to_string()
        } else {
            "Create one with: poly save <name>".to_string()
        },
    ));
    checks
}

pub async fn execute(config: Config, quiet: bool) -> Result<()> {
    if !quiet {
        println!("\n{} Running health check...\n", "🏥".cyan());
    }
    let checks = run_health_checks(&config).await;
    let mut issues = 0;
    for (_, _, ok, _) in &checks {
        if !ok {
            issues += 1;
        }
    }
    if !quiet {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.add_row(Row::new(vec![
            Cell::new("Check").style_spec("Fb"),
            Cell::new("Status").style_spec("Fb"),
            Cell::new("Fix").style_spec("Fb"),
        ]));
        for (check, status, _, fix) in &checks {
            let status_cell = if status.starts_with('✓') {
                Cell::new(status).style_spec("Fg")
            } else {
                Cell::new(status).style_spec("Fr")
            };
            let fix_str = if fix == "none" {
                "-".to_string()
            } else {
                fix.clone()
            };
            table.add_row(Row::new(vec![
                Cell::new(check),
                status_cell,
                Cell::new(&fix_str),
            ]));
        }
        table.printstd();
        println!();
        if issues == 0 {
            println!(
                "{} All checks passed! System is healthy.",
                "✓".green().bold()
            );
        } else {
            println!(
                "{} {issues} issue(s) found. See 'Fix' column above.",
                "!".yellow().bold()
            );
        }
    }
    if issues > 0 {
        anyhow::bail!("{issues} issue(s) found. See 'Fix' column above.");
    }
    Ok(())
}
