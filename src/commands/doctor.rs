use crate::utils::auth::{
    auth_mode_has_api_key, auth_mode_has_chatgpt, auth_mode_label, detect_auth_mode,
};
use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};
use serde::Serialize;

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
            "Install: npm install -g @openai/codex then npm install -g codexctl".to_string()
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
            "Run: codex (then sign in with ChatGPT or API key)".to_string()
        },
    ));
    let mut auth_valid = false;
    let mut auth_error = String::new();
    let mut auth_mode = String::from("unknown");
    if auth_exists {
        match tokio::fs::read_to_string(&auth_file).await {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(auth_json) => {
                    auth_valid = true;
                    auth_mode = detect_auth_mode(&auth_json);
                }
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
    let auth_mode_known = auth_valid && auth_mode != "unknown";
    checks.push((
        "Auth Mode".to_string(),
        if auth_mode_known {
            format!("✓ {}", auth_mode_label(&auth_mode))
        } else if auth_valid {
            "✗ Unknown".to_string()
        } else {
            "✗ Unavailable".to_string()
        },
        auth_mode_known,
        if auth_mode_known {
            "none".to_string()
        } else if auth_valid {
            "Re-authenticate with: codex".to_string()
        } else {
            "Run: codex (then sign in with ChatGPT or API key)".to_string()
        },
    ));
    let usage_surface = if auth_mode_known {
        match (
            auth_mode_has_chatgpt(&auth_mode),
            auth_mode_has_api_key(&auth_mode),
        ) {
            (true, true) => "✓ Local plan claims + API quota".to_string(),
            (true, false) => "✓ Local plan claims".to_string(),
            (false, true) => "✓ API billing/quota".to_string(),
            (false, false) => "✗ Unknown".to_string(),
        }
    } else if auth_valid {
        "✗ Unknown".to_string()
    } else {
        "✗ Unavailable".to_string()
    };
    checks.push((
        "Usage Surface".to_string(),
        usage_surface,
        auth_mode_known,
        if auth_mode_known {
            "none".to_string()
        } else {
            "Run `codexctl status` after signing in to inspect capabilities".to_string()
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
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if Config::is_reserved_entry_name(&name) {
                    continue;
                }
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
            "Create one with: codexctl save <name>".to_string()
        },
    ));
    checks
}

pub async fn execute(config: Config, json: bool, quiet: bool) -> Result<()> {
    if !json && !quiet {
        println!("\n{} Running health check...\n", "🏥".cyan());
    }
    let checks = run_health_checks(&config).await;
    let mut issues = 0;
    for (_, _, ok, _) in &checks {
        if !ok {
            issues += 1;
        }
    }
    if json {
        let payload = DoctorSummary {
            issues,
            healthy: issues == 0,
            checks: checks
                .iter()
                .map(|(check, status, ok, fix)| DoctorCheck {
                    name: check.clone(),
                    status: status.clone(),
                    ok: *ok,
                    fix: if fix == "none" {
                        None
                    } else {
                        Some(fix.clone())
                    },
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if !quiet {
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

#[derive(Debug, Serialize)]
struct DoctorSummary {
    healthy: bool,
    issues: usize,
    checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: String,
    status: String,
    ok: bool,
    fix: Option<String>,
}
