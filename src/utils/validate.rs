use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context as _, Result};
use serde::Serialize;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
    TimedOut,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCheckResult {
    pub id: String,
    pub kind: String,
    pub command: String,
    pub cwd: String,
    pub timeout_seconds: u64,
    pub status: ValidationStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub errors: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationRunResult {
    pub status: ValidationStatus,
    pub summary: ValidationSummary,
    pub checks: Vec<ValidationCheckResult>,
}

pub async fn run_shell_checks(
    checks: &[String],
    cwd: &Path,
    timeout_seconds: u64,
    fail_fast: bool,
) -> Result<ValidationRunResult> {
    let started = std::time::Instant::now();
    let mut results = Vec::with_capacity(checks.len());

    for (index, check) in checks.iter().enumerate() {
        let result = run_single_shell_check(index + 1, check, cwd, timeout_seconds).await?;
        let stop_early = fail_fast
            && matches!(
                result.status,
                ValidationStatus::Failed | ValidationStatus::TimedOut | ValidationStatus::Error
            );
        results.push(result);
        if stop_early {
            break;
        }
    }

    let summary = summarize(&results, started.elapsed().as_millis());
    let status = overall_status(&summary);
    Ok(ValidationRunResult {
        status,
        summary,
        checks: results,
    })
}

async fn run_single_shell_check(
    index: usize,
    check: &str,
    cwd: &Path,
    timeout_seconds: u64,
) -> Result<ValidationCheckResult> {
    let started = std::time::Instant::now();
    let mut command = build_shell_command(check, cwd);
    command.stdout(Stdio::null()).stderr(Stdio::null());

    let status = match timeout(Duration::from_secs(timeout_seconds), command.status()).await {
        Ok(Ok(status)) if status.success() => ValidationStatus::Passed,
        Ok(Ok(status)) => {
            return Ok(ValidationCheckResult {
                id: format!("check_{index:02}"),
                kind: "shell".to_string(),
                command: check.to_string(),
                cwd: cwd.display().to_string(),
                timeout_seconds,
                status: ValidationStatus::Failed,
                exit_code: status.code(),
                duration_ms: started.elapsed().as_millis(),
                stdout_path: None,
                stderr_path: None,
            });
        }
        Ok(Err(_)) => ValidationStatus::Error,
        Err(_) => ValidationStatus::TimedOut,
    };

    Ok(ValidationCheckResult {
        id: format!("check_{index:02}"),
        kind: "shell".to_string(),
        command: check.to_string(),
        cwd: cwd.display().to_string(),
        timeout_seconds,
        status,
        exit_code: None,
        duration_ms: started.elapsed().as_millis(),
        stdout_path: None,
        stderr_path: None,
    })
}

fn build_shell_command(command: &str, cwd: &Path) -> Command {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut cmd = Command::new("bash");
        cmd.arg("-lc").arg(command);
        cmd
    };

    cmd.current_dir(cwd);
    cmd
}

fn summarize(results: &[ValidationCheckResult], duration_ms: u128) -> ValidationSummary {
    let mut summary = ValidationSummary {
        total_checks: results.len(),
        passed: 0,
        failed: 0,
        timed_out: 0,
        errors: 0,
        duration_ms,
    };

    for result in results {
        match result.status {
            ValidationStatus::Passed => summary.passed += 1,
            ValidationStatus::Failed => summary.failed += 1,
            ValidationStatus::TimedOut => summary.timed_out += 1,
            ValidationStatus::Error => summary.errors += 1,
        }
    }

    summary
}

fn overall_status(summary: &ValidationSummary) -> ValidationStatus {
    if summary.errors > 0 {
        ValidationStatus::Error
    } else if summary.timed_out > 0 {
        ValidationStatus::TimedOut
    } else if summary.failed > 0 {
        ValidationStatus::Failed
    } else {
        ValidationStatus::Passed
    }
}

pub fn resolve_validation_cwd(
    task_path: Option<&Path>,
    override_cwd: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(cwd) = override_cwd {
        return std::fs::canonicalize(cwd)
            .with_context(|| format!("Failed to resolve validation cwd: {}", cwd.display()));
    }

    if let Some(task_path) = task_path {
        let parent = task_path.parent().unwrap_or_else(|| Path::new("."));
        return std::fs::canonicalize(parent).with_context(|| {
            format!("Failed to resolve bet spec directory: {}", parent.display())
        });
    }

    std::env::current_dir().context("Failed to resolve current working directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn shell_checks_report_pass_and_fail() {
        let cwd = TempDir::new().unwrap();
        let result = run_shell_checks(
            &["true".to_string(), "false".to_string()],
            cwd.path(),
            5,
            false,
        )
        .await
        .unwrap();

        assert_eq!(result.summary.total_checks, 2);
        assert_eq!(result.summary.passed, 1);
        assert_eq!(result.summary.failed, 1);
        assert_eq!(result.status, ValidationStatus::Failed);
    }

    #[tokio::test]
    async fn shell_check_timeout_sets_timed_out_status() {
        let cwd = TempDir::new().unwrap();
        let result = run_shell_checks(&["sleep 2".to_string()], cwd.path(), 1, false)
            .await
            .unwrap();

        assert_eq!(result.summary.timed_out, 1);
        assert_eq!(result.status, ValidationStatus::TimedOut);
    }

    #[tokio::test]
    async fn fail_fast_stops_after_first_failure() {
        let cwd = TempDir::new().unwrap();
        let result = run_shell_checks(
            &["false".to_string(), "true".to_string()],
            cwd.path(),
            5,
            true,
        )
        .await
        .unwrap();

        assert_eq!(result.summary.total_checks, 1);
        assert_eq!(result.checks.len(), 1);
    }
}
