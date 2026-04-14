use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize as _;
use serde::Serialize;

use crate::utils::command_exit::fail;
use crate::utils::task::BetSpec;
use crate::utils::validate::{
    ValidationRunResult, ValidationStatus, resolve_validation_cwd, run_shell_checks,
};

const CHECKS_FAILED_EXIT_CODE: u8 = 10;
const CHECKS_TIMED_OUT_EXIT_CODE: u8 = 11;

pub async fn execute(
    task: Option<PathBuf>,
    checks: Vec<String>,
    timeout_seconds: u64,
    cwd: Option<PathBuf>,
    fail_fast: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if task.is_none() && checks.is_empty() {
        return fail(2, "validate requires --task or at least one --check");
    }

    let bet_spec = if let Some(task_path) = task.as_ref() {
        Some(BetSpec::load_from_path(task_path).await?)
    } else {
        None
    };

    let mut merged_checks = bet_spec
        .as_ref()
        .map(|spec| spec.acceptance_checks.clone())
        .unwrap_or_default();
    merged_checks.extend(checks);

    if merged_checks.is_empty() {
        return fail(
            13,
            "Bet spec does not contain any acceptance_checks and no CLI checks were provided",
        );
    }

    let resolved_cwd = resolve_validation_cwd(task.as_deref(), cwd.as_deref())?;
    let result =
        run_shell_checks(&merged_checks, &resolved_cwd, timeout_seconds, fail_fast).await?;

    if json {
        let payload = ValidateJson {
            schema_version: "validate/v1".to_string(),
            command: "validate".to_string(),
            status: result.status.clone(),
            task_path: task.map(|path| path.display().to_string()),
            summary: result.summary,
            checks: result.checks,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if !quiet {
        print_human_summary(&result, bet_spec.as_ref().map(|spec| spec.name.as_str()));
    }

    match result.status {
        ValidationStatus::Passed => Ok(()),
        ValidationStatus::Failed => fail(
            CHECKS_FAILED_EXIT_CODE,
            "One or more validation checks failed",
        ),
        ValidationStatus::TimedOut => fail(
            CHECKS_TIMED_OUT_EXIT_CODE,
            "One or more validation checks timed out",
        ),
        ValidationStatus::Error => fail(
            CHECKS_TIMED_OUT_EXIT_CODE,
            "One or more validation checks failed to execute",
        ),
    }
}

#[derive(Debug, Serialize)]
struct ValidateJson {
    schema_version: String,
    command: String,
    status: ValidationStatus,
    task_path: Option<String>,
    summary: crate::utils::validate::ValidationSummary,
    checks: Vec<crate::utils::validate::ValidationCheckResult>,
}

fn print_human_summary(result: &ValidationRunResult, bet_name: Option<&str>) {
    println!("{}", "Validation".bold().cyan());
    if let Some(bet_name) = bet_name {
        println!("  Bet: {}", bet_name.cyan());
    }
    println!("  Status: {}", format_status(&result.status));
    println!(
        "  Checks: {} total, {} passed, {} failed, {} timed out, {} errors",
        result.summary.total_checks,
        result.summary.passed.to_string().green(),
        result.summary.failed.to_string().red(),
        result.summary.timed_out.to_string().yellow(),
        result.summary.errors.to_string().yellow(),
    );

    for check in result
        .checks
        .iter()
        .filter(|check| check.status != ValidationStatus::Passed)
    {
        println!("  - {} {}", format_status(&check.status), check.command);
    }
}

fn format_status(status: &ValidationStatus) -> colored::ColoredString {
    match status {
        ValidationStatus::Passed => "passed".green(),
        ValidationStatus::Failed => "failed".red(),
        ValidationStatus::TimedOut => "timed_out".yellow(),
        ValidationStatus::Error => "error".yellow(),
    }
}
