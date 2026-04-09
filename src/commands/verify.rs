//! Verify command - Validate all profiles' authentication without switching

use crate::utils::auth::extract_usage_info;
use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};

/// Verify all profiles' authentication status
pub async fn execute(config: Config, quiet: bool) -> Result<()> {
    let profiles_dir = config.profiles_dir();
    
    if !profiles_dir.exists() {
        anyhow::bail!("No profiles directory found. Create profiles first with: codexo save <name>");
    }

    let mut entries = tokio::fs::read_dir(profiles_dir).await?;
    let mut results = Vec::new();

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

        if name == "backups" || name.starts_with('.') {
            continue;
        }

        let result = verify_profile(&path).await;
        results.push((name, result));
    }

    if results.is_empty() {
        anyhow::bail!("No profiles found to verify");
    }

    // Sort by status (valid first, then invalid, then unknown)
    results.sort_by(|a, b| {
        let score = |r: &ProfileStatus| match r {
            ProfileStatus::Valid { .. } => 0,
            ProfileStatus::Invalid(_) => 1,
            ProfileStatus::Unknown => 2,
        };
        score(&a.1).cmp(&score(&b.1))
    });

    if !quiet {
        display_results(&results);
    }

    // Return error if any profile is invalid
    let invalid_count = results.iter().filter(|(_, r)| matches!(r, ProfileStatus::Invalid(_))).count();
    if invalid_count > 0 {
        anyhow::bail!("{} profile(s) have invalid authentication", invalid_count);
    }

    Ok(())
}

#[derive(Debug)]
enum ProfileStatus {
    Valid { email: String, plan: String },
    Invalid(String),
    Unknown,
}

async fn verify_profile(profile_dir: &std::path::Path) -> ProfileStatus {
    let auth_path = profile_dir.join("auth.json");
    
    if !auth_path.exists() {
        return ProfileStatus::Invalid("No auth.json found".to_string());
    }

    let auth_content = match tokio::fs::read_to_string(&auth_path).await {
        Ok(c) => c,
        Err(e) => return ProfileStatus::Invalid(format!("Cannot read auth.json: {}", e)),
    };

    // Check if encrypted
    if auth_content.trim().starts_with("age-encrypted:v1") {
        return ProfileStatus::Invalid("Profile is encrypted (passphrase required)".to_string());
    }

    let auth_json: serde_json::Value = match serde_json::from_str(&auth_content) {
        Ok(j) => j,
        Err(e) => return ProfileStatus::Invalid(format!("Invalid auth.json: {}", e)),
    };

    // Try to extract usage info (validates JWT structure)
    match extract_usage_info(&auth_json) {
        Ok(usage) => {
            // Check if token is expired
            if let Some(ref end) = usage.subscription_end {
                match calculate_days_remaining(end) {
                    Ok(days) if days < 0 => {
                        ProfileStatus::Invalid(format!("Subscription expired {} days ago", days.abs()))
                    }
                    _ => ProfileStatus::Valid {
                        email: usage.email,
                        plan: usage.plan_type,
                    }
                }
            } else {
                ProfileStatus::Valid {
                    email: usage.email,
                    plan: usage.plan_type,
                }
            }
        }
        Err(e) => ProfileStatus::Invalid(format!("Cannot parse token: {}", e)),
    }
}

fn display_results(results: &[(String, ProfileStatus)]) {
    println!("\n{}", "🔍 Profile Verification Results".bold().cyan());
    println!();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(Row::new(vec![
        Cell::new("Profile").style_spec("Fb"),
        Cell::new("Status").style_spec("Fb"),
        Cell::new("Details").style_spec("Fb"),
    ]));

    for (name, status) in results {
        match status {
            ProfileStatus::Valid { email, plan } => {
                let plan_badge = match plan.as_str() {
                    "team" => "👥 Team".cyan(),
                    "enterprise" => "🏢 Enterprise".magenta(),
                    _ => "👤 Personal".yellow(),
                };
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("✓ Valid").style_spec("Fg"),
                    Cell::new(&format!("{} ({})", email, plan_badge)),
                ]));
            }
            ProfileStatus::Invalid(reason) => {
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("✗ Invalid").style_spec("Fr"),
                    Cell::new(reason).style_spec("Fy"),
                ]));
            }
            ProfileStatus::Unknown => {
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("? Unknown"),
                    Cell::new("Could not verify"),
                ]));
            }
        }
    }

    table.printstd();
    println!();

    let valid = results.iter().filter(|(_, r)| matches!(r, ProfileStatus::Valid { .. })).count();
    let invalid = results.iter().filter(|(_, r)| matches!(r, ProfileStatus::Invalid(_))).count();
    
    println!("{}: {} valid, {} invalid", "Summary".bold(), valid.to_string().green(), invalid.to_string().red());
    println!();
}

fn calculate_days_remaining(iso_date: &str) -> anyhow::Result<i64> {
    use chrono::{DateTime, Utc};

    let end_date = DateTime::parse_from_rfc3339(iso_date)
        .map_err(|e| anyhow::anyhow!("Failed to parse date: {}", e))?;

    let now = Utc::now();
    let duration = end_date.with_timezone(&Utc) - now;

    Ok(duration.num_days())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_profile_missing_auth() {
        let temp_dir = TempDir::new().unwrap();
        let status = verify_profile(temp_dir.path()).await;
        
        match status {
            ProfileStatus::Invalid(msg) => assert!(msg.contains("No auth.json")),
            _ => panic!("Expected Invalid status"),
        }
    }
}
