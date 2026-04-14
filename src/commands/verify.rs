//! Verify command - Validate all profiles' authentication without switching

use crate::utils::auth::{
    auth_mode_has_api_key, auth_mode_has_chatgpt, auth_mode_label, detect_auth_mode,
    extract_usage_info,
};
use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};
use serde::Serialize;

/// Verify all profiles' authentication status
pub async fn execute(config: Config, json: bool, quiet: bool) -> Result<()> {
    let profiles_dir = config.profiles_dir();

    if !profiles_dir.exists() {
        anyhow::bail!(
            "No profiles directory found. Create profiles first with: codexctl save <name>"
        );
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

        if Config::is_reserved_entry_name(&name) {
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
            ProfileStatus::Locked { .. } => 1,
            ProfileStatus::Invalid(_) => 2,
        };
        score(&a.1).cmp(&score(&b.1))
    });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_verify_summary(&results))?
        );
    } else if !quiet {
        display_results(&results);
    }

    // Return error if any profile is invalid
    let invalid_count = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Invalid(_)))
        .count();
    if invalid_count > 0 {
        anyhow::bail!("{invalid_count} profile(s) have invalid authentication");
    }

    Ok(())
}

#[derive(Debug)]
enum ProfileStatus {
    Valid {
        auth_mode: String,
        identity: String,
        access: String,
    },
    Locked {
        auth_mode: String,
    },
    Invalid(String),
}

async fn verify_profile(profile_dir: &std::path::Path) -> ProfileStatus {
    let auth_path = profile_dir.join("auth.json");

    if !auth_path.exists() {
        return ProfileStatus::Invalid("No auth.json found".to_string());
    }

    let auth_content = match tokio::fs::read(&auth_path).await {
        Ok(c) => c,
        Err(e) => return ProfileStatus::Invalid(format!("Cannot read auth.json: {e}")),
    };
    let fallback_auth_mode = read_profile_auth_mode(profile_dir)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    // Check if encrypted
    if crate::utils::crypto::is_encrypted(&auth_content) {
        return ProfileStatus::Locked {
            auth_mode: fallback_auth_mode,
        };
    }

    let auth_json: serde_json::Value = match serde_json::from_slice(&auth_content) {
        Ok(j) => j,
        Err(e) => return ProfileStatus::Invalid(format!("Invalid auth.json: {e}")),
    };
    let auth_mode = detect_auth_mode(&auth_json);

    if auth_mode_has_chatgpt(&auth_mode) {
        match extract_usage_info(&auth_json) {
            Ok(usage) => {
                if let Some(ref end) = usage.subscription_end
                    && let Ok(days) = calculate_days_remaining(end)
                    && days < 0
                {
                    return ProfileStatus::Invalid(format!(
                        "Subscription expired {} days ago",
                        days.unsigned_abs()
                    ));
                }

                let access = if auth_mode_has_api_key(&auth_mode) {
                    format!("{} + API key", usage.plan_type.to_uppercase())
                } else {
                    usage.plan_type.to_uppercase()
                };
                return ProfileStatus::Valid {
                    auth_mode,
                    identity: usage.email,
                    access,
                };
            }
            Err(e) if auth_mode_has_api_key(&auth_mode) => {
                return ProfileStatus::Valid {
                    auth_mode,
                    identity: "API key".to_string(),
                    access: format!("OpenAI API billing/quota (ChatGPT claims unavailable: {e})"),
                };
            }
            Err(e) => return ProfileStatus::Invalid(format!("Cannot parse token: {e}")),
        }
    }

    if auth_mode_has_api_key(&auth_mode) {
        return ProfileStatus::Valid {
            auth_mode,
            identity: "API key".to_string(),
            access: "OpenAI API billing/quota".to_string(),
        };
    }

    ProfileStatus::Invalid("Unknown auth mode".to_string())
}

fn display_results(results: &[(String, ProfileStatus)]) {
    println!("\n{}", "🔍 Profile Verification Results".bold().cyan());
    println!();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(Row::new(vec![
        Cell::new("Profile").style_spec("Fb"),
        Cell::new("Status").style_spec("Fb"),
        Cell::new("Auth Mode").style_spec("Fb"),
        Cell::new("Details").style_spec("Fb"),
    ]));

    for (name, status) in results {
        match status {
            ProfileStatus::Valid {
                auth_mode,
                identity,
                access,
            } => {
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("✓ Valid").style_spec("Fg"),
                    Cell::new(auth_mode_label(auth_mode)),
                    Cell::new(&format!("{identity} ({access})")),
                ]));
            }
            ProfileStatus::Locked { auth_mode } => {
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("🔒 Locked").style_spec("Fy"),
                    Cell::new(auth_mode_label(auth_mode)),
                    Cell::new("Encrypted profile; decrypt on load"),
                ]));
            }
            ProfileStatus::Invalid(reason) => {
                table.add_row(Row::new(vec![
                    Cell::new(name).style_spec("Fg"),
                    Cell::new("✗ Invalid").style_spec("Fr"),
                    Cell::new("-"),
                    Cell::new(reason).style_spec("Fy"),
                ]));
            }
        }
    }

    table.printstd();
    println!();

    let valid = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Valid { .. }))
        .count();
    let invalid = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Invalid(_)))
        .count();
    let locked = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Locked { .. }))
        .count();

    println!(
        "{}: {} valid, {} locked, {} invalid",
        "Summary".bold(),
        valid.to_string().green(),
        locked.to_string().yellow(),
        invalid.to_string().red()
    );
    println!();
}

fn build_verify_summary(results: &[(String, ProfileStatus)]) -> VerifySummary {
    let valid = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Valid { .. }))
        .count();
    let invalid = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Invalid(_)))
        .count();
    let locked = results
        .iter()
        .filter(|(_, r)| matches!(r, ProfileStatus::Locked { .. }))
        .count();

    VerifySummary {
        valid,
        locked,
        invalid,
        profiles: results
            .iter()
            .map(|(name, status)| match status {
                ProfileStatus::Valid {
                    auth_mode,
                    identity,
                    access,
                } => VerifyProfileRow {
                    profile: name.clone(),
                    status: "valid".to_string(),
                    auth_mode: auth_mode.clone(),
                    auth_mode_label: auth_mode_label(auth_mode).to_string(),
                    details: format!("{identity} ({access})"),
                },
                ProfileStatus::Locked { auth_mode } => VerifyProfileRow {
                    profile: name.clone(),
                    status: "locked".to_string(),
                    auth_mode: auth_mode.clone(),
                    auth_mode_label: auth_mode_label(auth_mode).to_string(),
                    details: "Encrypted profile; decrypt on load".to_string(),
                },
                ProfileStatus::Invalid(reason) => VerifyProfileRow {
                    profile: name.clone(),
                    status: "invalid".to_string(),
                    auth_mode: "unknown".to_string(),
                    auth_mode_label: "Unknown".to_string(),
                    details: reason.clone(),
                },
            })
            .collect(),
    }
}

async fn read_profile_auth_mode(profile_dir: &std::path::Path) -> Option<String> {
    let meta_path = profile_dir.join("profile.json");
    let content = tokio::fs::read_to_string(meta_path).await.ok()?;
    let meta: crate::utils::profile::ProfileMeta = serde_json::from_str(&content).ok()?;
    Some(meta.auth_mode)
}

#[derive(Debug, Serialize)]
struct VerifySummary {
    valid: usize,
    locked: usize,
    invalid: usize,
    profiles: Vec<VerifyProfileRow>,
}

#[derive(Debug, Serialize)]
struct VerifyProfileRow {
    profile: String,
    status: String,
    auth_mode: String,
    auth_mode_label: String,
    details: String,
}

fn calculate_days_remaining(iso_date: &str) -> anyhow::Result<i64> {
    use chrono::{DateTime, Utc};

    let end_date = DateTime::parse_from_rfc3339(iso_date)
        .map_err(|e| anyhow::anyhow!("Failed to parse date: {e}"))?;

    let now = Utc::now();
    let duration = end_date.with_timezone(&Utc) - now;

    Ok(duration.num_days())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::Duration;
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

    #[tokio::test]
    async fn test_verify_profile_api_key_only_is_valid() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(
            temp_dir.path().join("auth.json"),
            r#"{"api_key":"sk-test"}"#,
        )
        .await
        .unwrap();

        let status = verify_profile(temp_dir.path()).await;

        match status {
            ProfileStatus::Valid {
                auth_mode,
                identity,
                access,
            } => {
                assert_eq!(auth_mode, "api_key");
                assert_eq!(identity, "API key");
                assert_eq!(access, "OpenAI API billing/quota");
            }
            _ => panic!("Expected Valid status"),
        }
    }

    #[tokio::test]
    async fn test_verify_profile_encrypted_is_locked() {
        let temp_dir = TempDir::new().unwrap();
        let mut profile = crate::utils::profile::Profile::new(
            "locked".to_string(),
            Some("locked@example.com".to_string()),
            None,
        );
        profile.meta.auth_mode = "chatgpt".to_string();
        profile.add_file(
            "auth.json",
            br#"{"tokens":{"id_token":"header.payload.signature"}}"#.to_vec(),
        );
        profile
            .save_to_disk_encrypted(temp_dir.path(), Some(&"secret".to_string()))
            .unwrap();

        let status = verify_profile(temp_dir.path()).await;

        match status {
            ProfileStatus::Locked { auth_mode } => assert_eq!(auth_mode, "chatgpt"),
            _ => panic!("Expected Locked status"),
        }
    }

    #[tokio::test]
    async fn test_verify_profile_hybrid_is_valid() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(
            temp_dir.path().join("auth.json"),
            make_chatgpt_auth("enterprise", 30, true).to_string(),
        )
        .await
        .unwrap();

        let status = verify_profile(temp_dir.path()).await;

        match status {
            ProfileStatus::Valid {
                auth_mode,
                identity,
                access,
            } => {
                assert_eq!(auth_mode, "chatgpt+api_key");
                assert_eq!(identity, "user@example.com");
                assert_eq!(access, "ENTERPRISE + API key");
            }
            _ => panic!("Expected Valid status"),
        }
    }

    fn make_chatgpt_auth(
        plan: &str,
        days_until_expiry: i64,
        with_api_key: bool,
    ) -> serde_json::Value {
        let now = chrono::Utc::now();
        let payload = serde_json::json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": plan,
                "chatgpt_subscription_active_start": now.to_rfc3339(),
                "chatgpt_subscription_active_until": (now + Duration::days(days_until_expiry)).to_rfc3339(),
                "chatgpt_account_id": "acct_test",
                "organizations": []
            }
        });
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
        let token = format!("{header}.{payload}.sig");

        if with_api_key {
            serde_json::json!({
                "tokens": { "id_token": token },
                "api_key": "sk-test"
            })
        } else {
            serde_json::json!({
                "tokens": { "id_token": token }
            })
        }
    }
}
