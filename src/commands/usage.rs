use crate::utils::auth::{UsageInfo, extract_usage_info};
use crate::utils::config::Config;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};

pub async fn execute(config: Config, all: bool, realtime: bool, quiet: bool) -> Result<()> {
    if all {
        return show_all_profiles_usage(config, quiet).await;
    }

    let codex_dir = config.codex_dir();
    let auth_path = codex_dir.join("auth.json");

    if !auth_path.exists() {
        anyhow::bail!("No `Codex` authentication found. Please login with: codex login");
    }

    let Ok(content) = tokio::fs::read_to_string(&auth_path).await else {
        anyhow::bail!("Failed to read auth.json");
    };

    let Ok(auth_json) = serde_json::from_str::<serde_json::Value>(&content) else {
        anyhow::bail!("Failed to parse auth.json");
    };

    // Extract usage info from the `JWT` tokens
    let usage_info = extract_usage_info(&auth_json)?;

    // Display usage info
    display_usage_table(&usage_info);

    // Fetch real-time quota if requested
    if realtime {
        match fetch_realtime_quota(&auth_json).await {
            Ok(quota) => display_realtime_quota(&quota),
            Err(e) => {
                if !quiet {
                    eprintln!("\n{} Could not fetch real-time quota: {}", "⚠".yellow(), e);
                }
            }
        }
    }

    // Display subscription status
    display_subscription_status(&usage_info);

    // Display helpful info about limits
    display_limits_info(&usage_info);

    Ok(())
}

/// Fetch real-time quota from `OpenAI` API
async fn fetch_realtime_quota(
    auth_json: &serde_json::Value,
) -> anyhow::Result<crate::utils::api::RealTimeQuota> {
    use crate::utils::api::{extract_api_key, fetch_quota};

    let api_key = extract_api_key(auth_json).context("No API key found in auth.json")?;

    fetch_quota(&api_key).await
}

/// Display real-time quota information
#[allow(clippy::cast_precision_loss)]
fn display_realtime_quota(quota: &crate::utils::api::RealTimeQuota) {
    println!("\n{}", "📈 Real-Time Quota (from OpenAI API)".bold().cyan());
    println!();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(Row::new(vec![
        Cell::new("Account ID"),
        Cell::new(&quota.account_id),
    ]));

    table.add_row(Row::new(vec![
        Cell::new("Plan"),
        Cell::new(&quota.plan.to_uppercase()),
    ]));

    // Format as dollars
    let limit_dollars = format!("${:.2}", quota.quota_limit as f64 / 100.0);
    let usage_dollars = format!("${:.2}", quota.usage_this_month as f64 / 100.0);
    let remaining_dollars = format!("${:.2}", quota.remaining_quota as f64 / 100.0);

    table.add_row(Row::new(vec![
        Cell::new("Monthly Limit"),
        Cell::new(&limit_dollars),
    ]));

    table.add_row(Row::new(vec![
        Cell::new("Used This Month"),
        Cell::new(&usage_dollars),
    ]));

    let remaining_style = if quota.is_critical() {
        "Fr"
    } else if quota.is_low() {
        "Fy"
    } else {
        "Fg"
    };
    table.add_row(Row::new(vec![
        Cell::new("Remaining"),
        Cell::new(&remaining_dollars).style_spec(remaining_style),
    ]));

    let percent_style = if quota.is_critical() {
        "Fr"
    } else if quota.is_low() {
        "Fy"
    } else {
        "Fg"
    };
    table.add_row(Row::new(vec![
        Cell::new("Percent Used"),
        Cell::new(&format!("{:.1}%", quota.percent_used)).style_spec(percent_style),
    ]));

    if let Some(days) = quota.days_until_reset() {
        let days_text = if days > 0 {
            format!("{days} days until reset")
        } else {
            "Resets today".to_string()
        };
        table.add_row(Row::new(vec![Cell::new("Reset"), Cell::new(&days_text)]));
    }

    table.printstd();

    // Warning messages
    if quota.is_critical() {
        println!("\n{}", "⚠️  WARNING: Quota critically low!".red().bold());
        println!(
            "   Only {:.1}% remaining. Consider switching profiles.",
            100.0 - quota.percent_used
        );
    } else if quota.is_low() {
        println!("\n{}", "⚠️  Quota running low".yellow());
        println!("   {:.1}% used. Monitor usage closely.", quota.percent_used);
    }
}

/// Show usage information for all profiles
#[allow(clippy::too_many_lines)]
async fn show_all_profiles_usage(config: Config, quiet: bool) -> Result<()> {
    use chrono::{DateTime, Utc};

    let profiles_dir = config.profiles_dir();
    if !profiles_dir.exists() {
        anyhow::bail!("No profiles directory found");
    }

    let mut entries = tokio::fs::read_dir(profiles_dir).await?;
    let mut profiles_with_usage = Vec::new();

    // Scan all profiles
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

        // Read auth.json from profile
        let auth_path = path.join("auth.json");
        if !auth_path.exists() {
            profiles_with_usage.push((name, None));
            continue;
        }

        let auth_content = tokio::fs::read_to_string(&auth_path).await.ok();
        let auth_json: Option<serde_json::Value> = auth_content
            .as_ref()
            .and_then(|c| serde_json::from_str(c).ok());

        if let Some(auth) = auth_json {
            if let Ok(usage) = extract_usage_info(&auth) {
                profiles_with_usage.push((name, Some(usage)));
            } else {
                profiles_with_usage.push((name, None));
            }
        } else {
            profiles_with_usage.push((name, None));
        }
    }

    if profiles_with_usage.is_empty() {
        anyhow::bail!("No profiles found");
    }

    // Sort by plan type (enterprise > team > personal)
    profiles_with_usage.sort_by(|a, b| {
        let score_a = a.1.as_ref().map_or(0, |u| match u.plan_type.as_str() {
            "enterprise" => 3,
            "team" => 2,
            "personal" => 1,
            _ => 0,
        });
        let score_b = b.1.as_ref().map_or(0, |u| match u.plan_type.as_str() {
            "enterprise" => 3,
            "team" => 2,
            "personal" => 1,
            _ => 0,
        });
        score_b.cmp(&score_a)
    });

    if !quiet {
        println!("\n{}", "📊 Usage Across All Profiles".bold().cyan());
        println!();

        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        // Header
        table.add_row(Row::new(vec![
            Cell::new("Profile").style_spec("Fb"),
            Cell::new("Email").style_spec("Fb"),
            Cell::new("Plan").style_spec("Fb"),
            Cell::new("Days Left").style_spec("Fb"),
            Cell::new("Status").style_spec("Fb"),
        ]));

        for (name, usage_opt) in profiles_with_usage {
            match usage_opt {
                Some(usage) => {
                    let plan_badge = match usage.plan_type.as_str() {
                        "team" => "👥 Team".cyan(),
                        "enterprise" => "🏢 Enterprise".magenta(),
                        _ => "👤 Personal".yellow(),
                    };

                    let days_left = usage
                        .subscription_end
                        .as_ref()
                        .and_then(|end| {
                            DateTime::parse_from_rfc3339(end)
                                .ok()
                                .map(|d| (d.with_timezone(&Utc) - Utc::now()).num_days())
                        })
                        .unwrap_or(0);

                    let status = if days_left > 7 {
                        "✓ Active".green()
                    } else if days_left > 0 {
                        "⚠ Expiring Soon".yellow()
                    } else {
                        "✗ Expired".red()
                    };

                    table.add_row(Row::new(vec![
                        Cell::new(&name).style_spec("Fg"),
                        Cell::new(&usage.email),
                        Cell::new(&plan_badge.to_string()),
                        Cell::new(&days_left.to_string()),
                        Cell::new(&status.to_string()),
                    ]));
                }
                None => {
                    table.add_row(Row::new(vec![
                        Cell::new(&name).style_spec("Fg"),
                        Cell::new("-"),
                        Cell::new("?"),
                        Cell::new("-"),
                        Cell::new("⚠ No auth data"),
                    ]));
                }
            }
        }

        table.printstd();
        println!();
        println!(
            "{}",
            "💡 Tip: Use 'poly load auto' to switch to the best available profile".dimmed()
        );
    }

    Ok(())
}

fn display_usage_table(info: &UsageInfo) {
    println!("\n{}", "`Codex` Usage & Subscription Info".bold().cyan());
    println!();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    // Header
    table.add_row(Row::new(vec![
        Cell::new("Property").style_spec("Fb"),
        Cell::new("Value").style_spec("Fb"),
    ]));

    // Email
    table.add_row(Row::new(vec![
        Cell::new("Email"),
        Cell::new(&info.email).style_spec("Fg"),
    ]));

    // Plan Type with color coding
    let plan_style = match info.plan_type.as_str() {
        "team" => "Fb",
        "enterprise" => "Fm",
        "personal" => "Fy",
        _ => "",
    };
    table.add_row(Row::new(vec![
        Cell::new("Plan Type"),
        Cell::new(&info.plan_type.to_uppercase()).style_spec(plan_style),
    ]));

    // Account ID (truncated)
    let short_account_id = if info.account_id.len() > 12 {
        format!("{}...", &info.account_id[..12])
    } else {
        info.account_id.clone()
    };
    table.add_row(Row::new(vec![
        Cell::new("Account ID"),
        Cell::new(&short_account_id),
    ]));

    // Organizations
    if !info.organizations.is_empty() {
        let orgs = info.organizations.join(", ");
        table.add_row(Row::new(vec![Cell::new("Organizations"), Cell::new(&orgs)]));
    }

    // Subscription period
    if let (Some(start), Some(end)) = (&info.subscription_start, &info.subscription_end) {
        let start_formatted = format_date(start);
        let end_formatted = format_date(end);

        table.add_row(Row::new(vec![
            Cell::new("Subscription"),
            Cell::new(&format!("{start_formatted} to {end_formatted}")),
        ]));

        // Calculate days remaining
        if let Ok(days) = calculate_days_remaining(end) {
            let (days_text, color) = if days > 0 {
                (format!("{days} days remaining"), "Fg")
            } else {
                (format!("Expired {} days ago", days.abs()), "Fr")
            };
            table.add_row(Row::new(vec![
                Cell::new("Status"),
                Cell::new(&days_text).style_spec(color),
            ]));
        }
    }

    table.printstd();
}

fn display_subscription_status(info: &UsageInfo) {
    println!();

    match info.plan_type.as_str() {
        "team" => {
            println!("{}", "✓ Team Plan Active".green().bold());
            println!("  • Unlimited GPT-4 requests (rate limited)");
            println!("  • Higher rate limits than personal plan");
            println!("  • Shared workspace features");
        }
        "enterprise" => {
            println!("{}", "✓ Enterprise Plan Active".magenta().bold());
            println!("  • Unlimited GPT-4 requests");
            println!("  • Highest rate limits");
            println!("  • Admin controls & audit logs");
            println!("  • SSO integration");
        }
        "personal" => {
            println!("{}", "✓ Personal Plan Active".yellow().bold());
            println!("  • Limited GPT-4 requests per week");
            println!("  • Standard rate limits");
        }
        _ => {
            println!("{}", "⚠ Unknown Plan Type".yellow().bold());
        }
    }
}

fn display_limits_info(_info: &UsageInfo) {
    println!();
    println!("{}", "📋 Usage Limits".bold());
    println!("  To view real-time usage limits and remaining quota,");
    println!(
        "  visit: {}",
        "https://platform.openai.com/settings/organization/limits"
            .cyan()
            .underline()
    );
    println!();
    println!("{}", "💡 Tips:".dimmed());
    println!(
        "  {}",
        "• `Codex` CLI shows usage warnings when approaching limits".dimmed()
    );
    println!(
        "  {}",
        "• Run 'poly status' to check current profile".dimmed()
    );
    println!(
        "  {}",
        "• Use 'poly backup' before switching profiles".dimmed()
    );
}

fn format_date(iso_date: &str) -> String {
    // Parse ISO 8601 date and format nicely
    let parts: Vec<&str> = iso_date.split('T').collect();
    if let Some(date_part) = parts.first() {
        let date_parts: Vec<&str> = date_part.split('-').collect();
        if date_parts.len() == 3 {
            let year = date_parts[0];
            let month = match date_parts[1] {
                "01" => "Jan",
                "02" => "Feb",
                "03" => "Mar",
                "04" => "Apr",
                "05" => "May",
                "06" => "Jun",
                "07" => "Jul",
                "08" => "Aug",
                "09" => "Sep",
                "10" => "Oct",
                "11" => "Nov",
                "12" => "Dec",
                _ => date_parts[1],
            };
            let day = date_parts[2];
            return format!("{month} {day}, {year}");
        }
    }

    iso_date.to_string()
}

fn calculate_days_remaining(iso_date: &str) -> Result<i64> {
    use chrono::{DateTime, Utc};

    let end_date = DateTime::parse_from_rfc3339(iso_date)
        .map_err(|e| anyhow::anyhow!("Failed to parse date: {e}"))?;

    let now = Utc::now();
    let duration = end_date.with_timezone(&Utc) - now;

    Ok(duration.num_days())
}
