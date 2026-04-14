use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context as _, Result};
use chrono::Utc;
use colored::Colorize as _;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;
use tokio::time::timeout;

use crate::commands::run::{apply_profile_auth, load_profile_auth, restore_original_auth};
use crate::utils::auth::detect_auth_mode;
use crate::utils::command_exit::fail;
use crate::utils::config::Config;
use crate::utils::runs::{
    LatestValidationSummary, RunEvent, RunRecord, RunStatus, agent_stderr_log, agent_stdout_log,
    append_event, append_log, create_run_id, final_report_file, initialize_run_dir,
    load_run_record, prompt_file, run_dir, save_run_record, stop_file, summary_file,
    task_snapshot_file, validation_file,
};
use crate::utils::task::BetSpec;
use crate::utils::validate::{ValidationStatus, resolve_validation_cwd, run_shell_checks};
use crate::utils::validation::ProfileName;

const RUN_BUDGET_EXIT_CODE: u8 = 21;
const RUN_CANCELLED_EXIT_CODE: u8 = 22;
const RUN_STATE_EXIT_CODE: u8 = 23;
const RUN_PROFILE_EXIT_CODE: u8 = 24;
const RUN_AGENT_EXIT_CODE: u8 = 25;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    config: Config,
    task: Option<PathBuf>,
    resume: Option<String>,
    max_iterations: Option<u32>,
    timeout_minutes: Option<u32>,
    max_consecutive_failures: Option<u32>,
    profile: Option<String>,
    passphrase: Option<String>,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let current_dir =
        std::env::current_dir().context("Failed to resolve current working directory")?;

    if task.is_some() == resume.is_some() {
        return fail(2, "run-loop requires exactly one of --task or --resume");
    }

    if let Some(run_id) = resume {
        return resume_run(
            config,
            &current_dir,
            &run_id,
            max_iterations,
            timeout_minutes,
            max_consecutive_failures,
            profile,
            passphrase,
            dry_run,
            json,
            quiet,
        )
        .await;
    }

    let task_path = task.expect("task path checked above");
    let bet_spec = BetSpec::load_from_path(&task_path).await?;
    let run_id = create_run_id();
    let repo_state = inspect_repo_state(&current_dir).await;

    if dry_run {
        let payload = RunLoopJson {
            schema_version: "run_loop/v1".to_string(),
            command: "run-loop".to_string(),
            run_id,
            status: "dry_run".to_string(),
            stop_reason: "dry_run".to_string(),
            task_name: bet_spec.name.clone(),
            task_path: task_path.display().to_string(),
            repo_root: current_dir.display().to_string(),
            profile,
            auth_mode: None,
            iteration_count: 0,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            latest_validation: LatestValidationSummary {
                status: None,
                passed: 0,
                failed: 0,
                timed_out: 0,
                errors: 0,
            },
        };
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else if !quiet {
            print_repo_warnings(&repo_state);
            println!("{}", "Run Loop".bold().cyan());
            println!("  Run ID: {}", payload.run_id.cyan());
            println!("  Task: {}", payload.task_name.green());
            println!("  Status: {}", "dry_run".yellow());
        }
        return Ok(());
    }

    let run_dir = run_dir(&config, &run_id);
    initialize_run_dir(&run_dir)?;
    tokio::fs::copy(&task_path, task_snapshot_file(&run_dir))
        .await
        .with_context(|| format!("Failed to snapshot {}", task_path.display()))?;

    let mut record = RunRecord {
        schema_version: "runs/v1".to_string(),
        run_id: run_id.clone(),
        status: RunStatus::Queued,
        stop_reason: None,
        task_name: bet_spec.name.clone(),
        task_path: task_path.display().to_string(),
        repo_root: current_dir.display().to_string(),
        profile: profile.clone(),
        auth_mode: None,
        iteration_count: 0,
        started_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        latest_validation: LatestValidationSummary {
            status: None,
            passed: 0,
            failed: 0,
            timed_out: 0,
            errors: 0,
        },
    };
    save_run_record(&run_dir, &record)?;
    append_event(
        &run_dir,
        &RunEvent {
            timestamp: Utc::now(),
            event_type: "started".to_string(),
            iteration: None,
            message: format!("Run started for {}", bet_spec.name),
            payload: None,
        },
    )?;

    execute_loop(
        &config,
        &current_dir,
        &run_dir,
        &mut record,
        &bet_spec,
        max_iterations,
        timeout_minutes,
        max_consecutive_failures,
        passphrase,
        repo_state,
        json,
        quiet,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn resume_run(
    config: Config,
    _current_dir: &Path,
    run_id: &str,
    max_iterations: Option<u32>,
    timeout_minutes: Option<u32>,
    max_consecutive_failures: Option<u32>,
    profile: Option<String>,
    passphrase: Option<String>,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if dry_run {
        return fail(2, "--dry-run cannot be used with --resume");
    }

    let mut record = load_run_record(&config, run_id).await?;
    if matches!(
        record.status,
        RunStatus::Succeeded
            | RunStatus::Failed
            | RunStatus::Cancelled
            | RunStatus::BudgetExhausted
    ) {
        return fail(
            RUN_STATE_EXIT_CODE,
            format!(
                "Run '{}' is already in a terminal state and cannot be resumed",
                run_id
            ),
        );
    }

    let repo_root = PathBuf::from(&record.repo_root);
    if !repo_root.exists() {
        return fail(
            RUN_STATE_EXIT_CODE,
            format!(
                "Run '{}' cannot be resumed because repo root '{}' no longer exists",
                run_id,
                repo_root.display()
            ),
        );
    }

    let run_root = run_dir(&config, run_id);
    initialize_run_dir(&run_root)?;

    let task_snapshot = task_snapshot_file(&run_root);
    let bet_spec = BetSpec::load_from_path(&task_snapshot).await?;
    let repo_state = inspect_repo_state(&repo_root).await;

    if let Some(profile) = profile {
        record.profile = Some(profile);
    }

    execute_loop(
        &config,
        &repo_root,
        &run_root,
        &mut record,
        &bet_spec,
        max_iterations,
        timeout_minutes,
        max_consecutive_failures,
        passphrase,
        repo_state,
        json,
        quiet,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn execute_loop(
    config: &Config,
    current_dir: &Path,
    run_dir: &Path,
    record: &mut RunRecord,
    bet_spec: &BetSpec,
    max_iterations_override: Option<u32>,
    timeout_minutes_override: Option<u32>,
    max_consecutive_failures_override: Option<u32>,
    passphrase: Option<String>,
    repo_state: RepoState,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let max_iterations = max_iterations_override
        .or(bet_spec
            .budgets
            .as_ref()
            .and_then(|budgets| budgets.max_iterations))
        .unwrap_or(8);
    let max_runtime_minutes = timeout_minutes_override
        .or(bet_spec
            .budgets
            .as_ref()
            .and_then(|budgets| budgets.max_runtime_minutes))
        .unwrap_or(90);
    let max_consecutive_failures = max_consecutive_failures_override
        .or(bet_spec
            .budgets
            .as_ref()
            .and_then(|budgets| budgets.max_consecutive_failures))
        .unwrap_or(3);

    let started = std::time::Instant::now();
    let mut consecutive_failures = 0u32;
    let mut previous_summary: Option<String> = None;

    record.status = RunStatus::Running;
    record.updated_at = Utc::now();
    save_run_record(run_dir, record)?;

    if !json && !quiet {
        print_repo_warnings(&repo_state);
        println!("{}", "Run Loop".bold().cyan());
        println!("  Run ID: {}", record.run_id.cyan());
        println!("  Task: {}", record.task_name.green());
    }

    loop {
        if stop_file(run_dir).exists() {
            record.status = RunStatus::Cancelled;
            record.stop_reason = Some("stop_file_detected".to_string());
            record.finished_at = Some(Utc::now());
            record.updated_at = Utc::now();
            save_run_record(run_dir, record)?;
            return finish_run(run_dir, record, json, quiet, RUN_CANCELLED_EXIT_CODE);
        }

        if record.iteration_count >= max_iterations {
            record.status = RunStatus::BudgetExhausted;
            record.stop_reason = Some("max_iterations_reached".to_string());
            record.finished_at = Some(Utc::now());
            record.updated_at = Utc::now();
            save_run_record(run_dir, record)?;
            return finish_run(run_dir, record, json, quiet, RUN_BUDGET_EXIT_CODE);
        }

        if started.elapsed() > Duration::from_secs(u64::from(max_runtime_minutes) * 60) {
            record.status = RunStatus::BudgetExhausted;
            record.stop_reason = Some("max_runtime_exceeded".to_string());
            record.finished_at = Some(Utc::now());
            record.updated_at = Utc::now();
            save_run_record(run_dir, record)?;
            return finish_run(run_dir, record, json, quiet, RUN_BUDGET_EXIT_CODE);
        }

        record.iteration_count += 1;
        record.updated_at = Utc::now();
        save_run_record(run_dir, record)?;

        let prompt = build_iteration_prompt(
            bet_spec,
            record.iteration_count,
            previous_summary.as_deref(),
            current_dir,
        );
        tokio::fs::write(prompt_file(run_dir, record.iteration_count), &prompt)
            .await
            .with_context(|| format!("Failed to write prompt for run {}", record.run_id))?;

        append_event(
            run_dir,
            &RunEvent {
                timestamp: Utc::now(),
                event_type: "iteration_started".to_string(),
                iteration: Some(record.iteration_count),
                message: format!("Starting iteration {}", record.iteration_count),
                payload: None,
            },
        )?;

        let agent_result = invoke_agent(AgentInvocation {
            config,
            repo_root: current_dir,
            run_dir,
            bet_spec,
            record,
            passphrase: passphrase.as_ref(),
            elapsed: started.elapsed(),
            max_runtime: Duration::from_secs(u64::from(max_runtime_minutes) * 60),
            is_git_repo: repo_state.is_git_repo,
        })
        .await;

        let (stdout, stderr) = match agent_result {
            Ok(output) => output,
            Err(error) => {
                record.status = RunStatus::Failed;
                record.stop_reason = Some("agent_invocation_failed".to_string());
                record.finished_at = Some(Utc::now());
                record.updated_at = Utc::now();
                save_run_record(run_dir, record)?;
                append_event(
                    run_dir,
                    &RunEvent {
                        timestamp: Utc::now(),
                        event_type: "agent_failed".to_string(),
                        iteration: Some(record.iteration_count),
                        message: error.to_string(),
                        payload: None,
                    },
                )?;
                return fail(
                    RUN_AGENT_EXIT_CODE,
                    format!("Agent invocation failed: {error}"),
                );
            }
        };

        append_log(&agent_stdout_log(run_dir), &stdout)?;
        append_log(&agent_stderr_log(run_dir), &stderr)?;

        let summary = summarize_agent_output(&stdout, &stderr);
        tokio::fs::write(summary_file(run_dir, record.iteration_count), &summary)
            .await
            .with_context(|| {
                format!(
                    "Failed to write iteration summary for run {}",
                    record.run_id
                )
            })?;

        let validation_cwd = resolve_validation_cwd(None, Some(current_dir))?;
        let validation =
            run_shell_checks(&bet_spec.acceptance_checks, &validation_cwd, 300, false).await?;
        tokio::fs::write(
            validation_file(run_dir, record.iteration_count),
            serde_json::to_vec_pretty(&validation)
                .context("Failed to serialize validation output")?,
        )
        .await
        .with_context(|| {
            format!(
                "Failed to write validation output for run {}",
                record.run_id
            )
        })?;

        record.latest_validation = LatestValidationSummary {
            status: Some(status_label(&validation.status).to_string()),
            passed: validation.summary.passed,
            failed: validation.summary.failed,
            timed_out: validation.summary.timed_out,
            errors: validation.summary.errors,
        };
        record.updated_at = Utc::now();
        save_run_record(run_dir, record)?;

        match validation.status {
            ValidationStatus::Passed => {
                record.status = RunStatus::Succeeded;
                record.stop_reason = Some("acceptance_checks_passed".to_string());
                record.finished_at = Some(Utc::now());
                record.updated_at = Utc::now();
                save_run_record(run_dir, record)?;
                tokio::fs::write(
                    final_report_file(run_dir),
                    build_final_report(record, bet_spec, Some(&summary)),
                )
                .await
                .with_context(|| {
                    format!("Failed to write final report for run {}", record.run_id)
                })?;
                append_event(
                    run_dir,
                    &RunEvent {
                        timestamp: Utc::now(),
                        event_type: "succeeded".to_string(),
                        iteration: Some(record.iteration_count),
                        message: "Acceptance checks passed".to_string(),
                        payload: None,
                    },
                )?;
                return finish_run(run_dir, record, json, quiet, 0);
            }
            ValidationStatus::Failed | ValidationStatus::TimedOut | ValidationStatus::Error => {
                consecutive_failures += 1;
                previous_summary = Some(build_validation_feedback(&validation, &summary));
                append_event(
                    run_dir,
                    &RunEvent {
                        timestamp: Utc::now(),
                        event_type: "validation_failed".to_string(),
                        iteration: Some(record.iteration_count),
                        message: format!(
                            "Validation {} after iteration {}",
                            status_label(&validation.status),
                            record.iteration_count
                        ),
                        payload: None,
                    },
                )?;

                if consecutive_failures >= max_consecutive_failures {
                    record.status = RunStatus::BudgetExhausted;
                    record.stop_reason = Some("max_consecutive_failures_reached".to_string());
                    record.finished_at = Some(Utc::now());
                    record.updated_at = Utc::now();
                    save_run_record(run_dir, record)?;
                    tokio::fs::write(
                        final_report_file(run_dir),
                        build_final_report(record, bet_spec, previous_summary.as_deref()),
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to write final report for run {}", record.run_id)
                    })?;
                    return finish_run(run_dir, record, json, quiet, RUN_BUDGET_EXIT_CODE);
                }
            }
        }
    }
}

struct AgentInvocation<'a> {
    config: &'a Config,
    repo_root: &'a Path,
    run_dir: &'a Path,
    bet_spec: &'a BetSpec,
    record: &'a mut RunRecord,
    passphrase: Option<&'a String>,
    elapsed: Duration,
    max_runtime: Duration,
    is_git_repo: bool,
}

async fn invoke_agent(invocation: AgentInvocation<'_>) -> Result<(Vec<u8>, Vec<u8>)> {
    let prompt = tokio::fs::read(prompt_file(
        invocation.run_dir,
        invocation.record.iteration_count,
    ))
    .await
    .with_context(|| format!("Failed to read prompt for run {}", invocation.record.run_id))?;

    let mut command = if let Some(agent) = &invocation.bet_spec.agent {
        if let Some(argv) = &agent.command {
            let mut iter = argv.iter();
            let Some(program) = iter.next() else {
                return fail(RUN_AGENT_EXIT_CODE, "Bet spec agent.command is empty");
            };
            let mut command = Command::new(program);
            command.args(iter);
            command
        } else {
            default_codex_command(invocation.repo_root, invocation.is_git_repo)
        }
    } else {
        default_codex_command(invocation.repo_root, invocation.is_git_repo)
    };

    let mut original_auth = None;
    if let Some(profile_name) = &invocation.record.profile {
        let validated_name = ProfileName::try_from(profile_name.as_str()).or_else(|_| {
            fail(
                RUN_PROFILE_EXIT_CODE,
                format!("Invalid profile name '{}'", profile_name),
            )
        })?;
        let profile_path = invocation
            .config
            .profile_path_validated(&validated_name)
            .or_else(|_| {
                fail(
                    RUN_PROFILE_EXIT_CODE,
                    format!("Failed to resolve profile '{}'", profile_name),
                )
            })?;
        let profile_auth = load_profile_auth(&profile_path, profile_name, invocation.passphrase)
            .await
            .or_else(|error| {
                fail(
                    RUN_PROFILE_EXIT_CODE,
                    format!("Failed to load profile '{}': {error}", profile_name),
                )
            })?;
        original_auth =
            Some(activate_profile_auth(invocation.config, &profile_auth, profile_name).await?);
        invocation.record.auth_mode = serde_json::from_slice::<serde_json::Value>(&profile_auth)
            .ok()
            .map(|json| detect_auth_mode(&json));
    }

    command.current_dir(invocation.repo_root);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());

    let result = async {
        let mut child = command.spawn().context("Failed to spawn agent process")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&prompt)
                .await
                .context("Failed to send prompt to agent")?;
        }

        let remaining = invocation.max_runtime.saturating_sub(invocation.elapsed);
        let output_result = timeout(remaining, child.wait_with_output()).await;

        match output_result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    return fail(
                        RUN_AGENT_EXIT_CODE,
                        format!("Agent exited with code {:?}", output.status.code()),
                    );
                }
                Ok((output.stdout, output.stderr))
            }
            Ok(Err(error)) => Err(error).context("Agent execution failed"),
            Err(_) => fail(
                RUN_AGENT_EXIT_CODE,
                "Agent execution exceeded the remaining runtime budget",
            ),
        }
    }
    .await;

    if let Some(original_auth) = original_auth {
        let _ = restore_original_auth(invocation.config.codex_dir(), original_auth).await;
    }

    result
}

async fn activate_profile_auth(
    config: &Config,
    profile_auth: &[u8],
    profile_name: &str,
) -> Result<Option<Vec<u8>>> {
    apply_profile_auth(config.codex_dir(), profile_auth)
        .await
        .or_else(|error| {
            fail(
                RUN_PROFILE_EXIT_CODE,
                format!("Failed to activate profile '{}': {error}", profile_name),
            )
        })
}

fn default_codex_command(repo_root: &Path, is_git_repo: bool) -> Command {
    let mut command = Command::new("codex");
    command.arg("exec").arg("-").arg("--cd").arg(repo_root);
    if !is_git_repo {
        command.arg("--skip-git-repo-check");
    }
    command
}

fn build_iteration_prompt(
    bet_spec: &BetSpec,
    iteration: u32,
    previous_summary: Option<&str>,
    repo_root: &Path,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are executing a shaped product bet.\n\n");
    prompt.push_str(&format!("Bet: {}\n", bet_spec.name));
    prompt.push_str(&format!("Appetite: {}\n", bet_spec.appetite));
    prompt.push_str(&format!("Repository: {}\n\n", repo_root.display()));
    prompt.push_str("Objective:\n");
    prompt.push_str(&bet_spec.objective);
    prompt.push_str("\n\nBounded contexts:\n");
    for context in &bet_spec.bounded_contexts {
        prompt.push_str(&format!("- {context}\n"));
    }
    prompt.push_str("\nSuccess signal:\n");
    prompt.push_str(&bet_spec.success_signal);
    prompt.push_str("\n\nNo-gos:\n");
    for item in &bet_spec.no_gos {
        prompt.push_str(&format!("- {item}\n"));
    }
    if !bet_spec.constraints.is_empty() {
        prompt.push_str("\nConstraints:\n");
        for item in &bet_spec.constraints {
            prompt.push_str(&format!("- {item}\n"));
        }
    }
    if !bet_spec.context_files.is_empty() {
        prompt.push_str("\nContext files:\n");
        for item in &bet_spec.context_files {
            prompt.push_str(&format!("- {item}\n"));
        }
    }
    prompt.push_str(&format!("\nCurrent iteration: {}\n", iteration));
    if let Some(summary) = previous_summary {
        prompt.push_str("\nFeedback from previous iteration:\n");
        prompt.push_str(summary);
        prompt.push('\n');
    }
    if let Some(agent) = &bet_spec.agent
        && let Some(preamble) = &agent.prompt_preamble
    {
        prompt.push_str("\nPrompt preamble:\n");
        prompt.push_str(preamble);
        prompt.push('\n');
    }
    prompt
}

fn summarize_agent_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let stdout_lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(10);
    let stderr_lines = stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(10);

    let mut summary = String::new();
    summary.push_str("Recent stdout:\n");
    for line in stdout_lines.collect::<Vec<_>>().into_iter().rev() {
        summary.push_str(&format!("- {line}\n"));
    }
    if !stderr.trim().is_empty() {
        summary.push_str("Recent stderr:\n");
        for line in stderr_lines.collect::<Vec<_>>().into_iter().rev() {
            summary.push_str(&format!("- {line}\n"));
        }
    }
    summary
}

fn build_validation_feedback(
    validation: &crate::utils::validate::ValidationRunResult,
    agent_summary: &str,
) -> String {
    let mut feedback = String::new();
    feedback.push_str(agent_summary);
    feedback.push_str("\nValidation summary:\n");
    feedback.push_str(&format!(
        "- status: {}\n- passed: {}\n- failed: {}\n- timed_out: {}\n- errors: {}\n",
        status_label(&validation.status),
        validation.summary.passed,
        validation.summary.failed,
        validation.summary.timed_out,
        validation.summary.errors
    ));

    for check in validation
        .checks
        .iter()
        .filter(|check| check.status != ValidationStatus::Passed)
    {
        feedback.push_str(&format!(
            "- failing_check {}: {} ({})\n",
            check.id,
            check.command,
            status_label(&check.status)
        ));
    }

    feedback
}

fn build_final_report(
    record: &RunRecord,
    bet_spec: &BetSpec,
    last_summary: Option<&str>,
) -> String {
    let mut report = String::new();
    report.push_str(&format!("# Final Report\n\nRun ID: {}\n", record.run_id));
    report.push_str(&format!("Bet: {}\n", record.task_name));
    report.push_str(&format!("Status: {:?}\n", record.status));
    report.push_str(&format!(
        "Stop reason: {}\n",
        record.stop_reason.as_deref().unwrap_or("none")
    ));
    report.push_str(&format!("Iterations: {}\n\n", record.iteration_count));
    report.push_str("Success signal:\n");
    report.push_str(&bet_spec.success_signal);
    report.push_str("\n\n");
    if let Some(summary) = last_summary {
        report.push_str("Last iteration summary:\n");
        report.push_str(summary);
        report.push('\n');
    }
    report
}

fn status_label(status: &ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::Passed => "passed",
        ValidationStatus::Failed => "failed",
        ValidationStatus::TimedOut => "timed_out",
        ValidationStatus::Error => "error",
    }
}

fn finish_run(
    run_dir: &Path,
    record: &RunRecord,
    json: bool,
    quiet: bool,
    exit_code: u8,
) -> Result<()> {
    let payload = RunLoopJson {
        schema_version: "run_loop/v1".to_string(),
        command: "run-loop".to_string(),
        run_id: record.run_id.clone(),
        status: run_status_text(&record.status).to_string(),
        stop_reason: record
            .stop_reason
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        task_name: record.task_name.clone(),
        task_path: record.task_path.clone(),
        repo_root: record.repo_root.clone(),
        profile: record.profile.clone(),
        auth_mode: record.auth_mode.clone(),
        iteration_count: record.iteration_count,
        started_at: record.started_at,
        updated_at: record.updated_at,
        finished_at: record.finished_at,
        latest_validation: record.latest_validation.clone(),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if !quiet {
        println!("\n{}", "Run Result".bold().cyan());
        println!("  Run ID: {}", record.run_id.cyan());
        println!("  Status: {}", run_status_label(&record.status));
        println!("  Stop reason: {}", payload.stop_reason);
        println!("  Iterations: {}", record.iteration_count);
        println!("  Report: {}", final_report_file(run_dir).display());
    }

    if exit_code == 0 {
        Ok(())
    } else {
        fail(
            exit_code,
            format!("Run ended with status {}", payload.status),
        )
    }
}

fn run_status_text(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Blocked => "blocked",
        RunStatus::Cancelled => "cancelled",
        RunStatus::BudgetExhausted => "budget_exhausted",
    }
}

fn run_status_label(status: &RunStatus) -> colored::ColoredString {
    match status {
        RunStatus::Queued => run_status_text(status).yellow(),
        RunStatus::Running => run_status_text(status).cyan(),
        RunStatus::Succeeded => run_status_text(status).green(),
        RunStatus::Failed => run_status_text(status).red(),
        RunStatus::Blocked => run_status_text(status).yellow(),
        RunStatus::Cancelled => run_status_text(status).yellow(),
        RunStatus::BudgetExhausted => run_status_text(status).yellow(),
    }
}

#[derive(Debug, Serialize)]
struct RunLoopJson {
    schema_version: String,
    command: String,
    run_id: String,
    status: String,
    stop_reason: String,
    task_name: String,
    task_path: String,
    repo_root: String,
    profile: Option<String>,
    auth_mode: Option<String>,
    iteration_count: u32,
    started_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    finished_at: Option<chrono::DateTime<Utc>>,
    latest_validation: LatestValidationSummary,
}

#[derive(Debug, Clone)]
struct RepoState {
    is_git_repo: bool,
    is_dirty: bool,
    is_detached_head: bool,
}

async fn inspect_repo_state(repo_root: &Path) -> RepoState {
    let git_toplevel = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .ok();
    let is_git_repo = git_toplevel.is_some_and(|status| status.success());

    if !is_git_repo {
        return RepoState {
            is_git_repo,
            is_dirty: false,
            is_detached_head: false,
        };
    }

    let dirty_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .await
        .ok();
    let is_dirty = dirty_output
        .as_ref()
        .is_some_and(|output| !String::from_utf8_lossy(&output.stdout).trim().is_empty());

    let detached_status = Command::new("git")
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .ok();
    let is_detached_head = detached_status.is_some_and(|status| !status.success());

    RepoState {
        is_git_repo,
        is_dirty,
        is_detached_head,
    }
}

fn print_repo_warnings(repo_state: &RepoState) {
    if !repo_state.is_git_repo {
        eprintln!(
            "{} Running outside a git repository; traceability is reduced",
            "⚠".yellow()
        );
        return;
    }

    if repo_state.is_dirty {
        eprintln!(
            "{} Working tree is dirty; run traceability is reduced",
            "⚠".yellow()
        );
    }
    if repo_state.is_detached_head {
        eprintln!("{} Repository is in detached HEAD state", "⚠".yellow());
    }
}
