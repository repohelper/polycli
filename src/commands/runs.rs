use anyhow::Result;
use colored::Colorize as _;
use serde::Serialize;

use crate::utils::command_exit::fail;
use crate::utils::config::Config;
use crate::utils::runs::{events_file, list_run_records, load_run_record, run_dir};

pub async fn execute(
    config: Config,
    latest: bool,
    run_id: Option<String>,
    json: bool,
    tail: bool,
    quiet: bool,
) -> Result<()> {
    if latest && run_id.is_some() {
        return fail(2, "runs accepts only one of --latest or --id");
    }
    if json && tail {
        return fail(2, "runs --tail cannot be combined with --json in Bet 01");
    }

    if latest || run_id.is_some() {
        let record = if let Some(run_id) = run_id {
            load_run_record(&config, &run_id).await?
        } else {
            let mut records = list_run_records(&config).await?;
            if records.is_empty() {
                return fail(23, "No runs found");
            }
            records.remove(0)
        };

        if json {
            let payload = RunsDetailJson {
                schema_version: "runs/v1".to_string(),
                command: "runs".to_string(),
                run: record,
            };
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else if !quiet {
            println!("{}", "Run Details".bold().cyan());
            println!("  Run ID: {}", record.run_id.cyan());
            println!("  Task: {}", record.task_name.green());
            println!("  Status: {}", run_status_text(&record.status));
            println!(
                "  Stop reason: {}",
                record.stop_reason.as_deref().unwrap_or("none")
            );
            println!("  Iterations: {}", record.iteration_count);
            println!("  Started: {}", record.started_at);
            if let Some(finished_at) = record.finished_at {
                println!("  Finished: {}", finished_at);
            }
            if tail {
                let events =
                    tokio::fs::read_to_string(events_file(&run_dir(&config, &record.run_id)))
                        .await
                        .unwrap_or_default();
                println!("\n{}", "Recent Events".bold().cyan());
                for line in events
                    .lines()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    println!("  {line}");
                }
            }
        }

        return Ok(());
    }

    let records = list_run_records(&config).await?;
    if json {
        let payload = RunsListJson {
            schema_version: "runs/v1".to_string(),
            command: "runs".to_string(),
            items: records,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if !quiet {
        println!("{}", "Runs".bold().cyan());
        for record in records {
            println!(
                "  {} {} {} ({})",
                record.run_id.cyan(),
                record.task_name.green(),
                run_status_text(&record.status).yellow(),
                record.iteration_count
            );
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct RunsListJson {
    schema_version: String,
    command: String,
    items: Vec<crate::utils::runs::RunRecord>,
}

#[derive(Debug, Serialize)]
struct RunsDetailJson {
    schema_version: String,
    command: String,
    run: crate::utils::runs::RunRecord,
}

fn run_status_text(status: &crate::utils::runs::RunStatus) -> &'static str {
    match status {
        crate::utils::runs::RunStatus::Queued => "queued",
        crate::utils::runs::RunStatus::Running => "running",
        crate::utils::runs::RunStatus::Succeeded => "succeeded",
        crate::utils::runs::RunStatus::Failed => "failed",
        crate::utils::runs::RunStatus::Blocked => "blocked",
        crate::utils::runs::RunStatus::Cancelled => "cancelled",
        crate::utils::runs::RunStatus::BudgetExhausted => "budget_exhausted",
    }
}
