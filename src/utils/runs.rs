use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::command_exit::fail;
use crate::utils::config::Config;
use crate::utils::files::write_bytes_preserve_permissions;

const RUN_STATE_EXIT_CODE: u8 = 23;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Blocked,
    Cancelled,
    BudgetExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestValidationSummary {
    pub status: Option<String>,
    pub passed: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub schema_version: String,
    pub run_id: String,
    pub status: RunStatus,
    pub stop_reason: Option<String>,
    pub task_name: String,
    pub task_path: String,
    pub repo_root: String,
    pub profile: Option<String>,
    pub auth_mode: Option<String>,
    pub iteration_count: u32,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub latest_validation: LatestValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub iteration: Option<u32>,
    pub message: String,
    pub payload: Option<serde_json::Value>,
}

pub fn create_run_id() -> String {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    format!("{timestamp}-{:08x}", rand::random::<u32>())
}

pub fn run_dir(config: &Config, run_id: &str) -> PathBuf {
    config.runs_dir().join(run_id)
}

pub fn run_file(run_dir: &Path) -> PathBuf {
    run_dir.join("run.json")
}

pub fn events_file(run_dir: &Path) -> PathBuf {
    run_dir.join("events.jsonl")
}

pub fn iterations_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("iterations")
}

pub fn logs_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("logs")
}

pub fn prompt_file(run_dir: &Path, iteration: u32) -> PathBuf {
    iterations_dir(run_dir).join(format!("{iteration:03}.prompt.md"))
}

pub fn summary_file(run_dir: &Path, iteration: u32) -> PathBuf {
    iterations_dir(run_dir).join(format!("{iteration:03}.summary.md"))
}

pub fn validation_file(run_dir: &Path, iteration: u32) -> PathBuf {
    iterations_dir(run_dir).join(format!("{iteration:03}.validation.json"))
}

pub fn task_snapshot_file(run_dir: &Path) -> PathBuf {
    run_dir.join("task.snapshot.yaml")
}

pub fn final_report_file(run_dir: &Path) -> PathBuf {
    run_dir.join("final-report.md")
}

pub fn stop_file(run_dir: &Path) -> PathBuf {
    run_dir.join(".stop")
}

pub fn agent_stdout_log(run_dir: &Path) -> PathBuf {
    logs_dir(run_dir).join("agent.stdout.log")
}

pub fn agent_stderr_log(run_dir: &Path) -> PathBuf {
    logs_dir(run_dir).join("agent.stderr.log")
}

pub fn initialize_run_dir(run_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(iterations_dir(run_dir))
        .with_context(|| format!("Failed to create iterations dir: {}", run_dir.display()))?;
    std::fs::create_dir_all(logs_dir(run_dir))
        .with_context(|| format!("Failed to create logs dir: {}", run_dir.display()))?;
    Ok(())
}

pub fn save_run_record(run_dir: &Path, record: &RunRecord) -> Result<()> {
    let data = serde_json::to_vec_pretty(record).context("Failed to serialize run record")?;
    write_bytes_preserve_permissions(&run_file(run_dir), &data)
        .with_context(|| format!("Failed to write run record in {}", run_dir.display()))
}

pub async fn load_run_record(config: &Config, run_id: &str) -> Result<RunRecord> {
    let path = run_file(&run_dir(config, run_id));
    if !path.exists() {
        return fail(RUN_STATE_EXIT_CODE, format!("Run '{}' not found", run_id));
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read {}", path.display()))
        .or_else(|_| {
            fail(
                RUN_STATE_EXIT_CODE,
                format!(
                    "Failed to read run record for '{}': {}",
                    run_id,
                    path.display()
                ),
            )
        })?;

    serde_json::from_str::<RunRecord>(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))
        .or_else(|_| {
            fail(
                RUN_STATE_EXIT_CODE,
                format!("Run state for '{}' is corrupt", run_id),
            )
        })
}

pub fn append_event(run_dir: &Path, event: &RunEvent) -> Result<()> {
    let path = events_file(run_dir);
    let serialized = serde_json::to_string(event).context("Failed to serialize run event")?;
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    writeln!(file, "{serialized}").with_context(|| format!("Failed to append {}", path.display()))
}

pub async fn list_run_records(config: &Config) -> Result<Vec<RunRecord>> {
    let mut entries = tokio::fs::read_dir(config.runs_dir())
        .await
        .with_context(|| {
            format!(
                "Failed to read runs directory: {}",
                config.runs_dir().display()
            )
        })?;
    let mut records = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let run_json = path.join("run.json");
        if !run_json.exists() {
            continue;
        }

        let content = tokio::fs::read_to_string(&run_json)
            .await
            .with_context(|| format!("Failed to read {}", run_json.display()))
            .or_else(|_| {
                fail(
                    RUN_STATE_EXIT_CODE,
                    format!("Failed to read run record: {}", run_json.display()),
                )
            })?;
        let record = serde_json::from_str::<RunRecord>(&content)
            .with_context(|| format!("Failed to parse {}", run_json.display()))
            .or_else(|_| {
                fail(
                    RUN_STATE_EXIT_CODE,
                    format!("Run state is corrupt: {}", run_json.display()),
                )
            })?;
        records.push(record);
    }

    records.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(records)
}

pub fn append_log(path: &Path, content: &[u8]) -> Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    file.write_all(content)
        .with_context(|| format!("Failed to append {}", path.display()))?;
    if !content.ends_with(b"\n") {
        file.write_all(b"\n")
            .with_context(|| format!("Failed to append newline to {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn initialize_run_dir_creates_expected_structure() {
        let temp_dir = TempDir::new().unwrap();
        initialize_run_dir(temp_dir.path()).unwrap();
        assert!(iterations_dir(temp_dir.path()).exists());
        assert!(logs_dir(temp_dir.path()).exists());
    }

    #[test]
    fn save_and_append_event_work() {
        let temp_dir = TempDir::new().unwrap();
        initialize_run_dir(temp_dir.path()).unwrap();
        let record = RunRecord {
            schema_version: "runs/v1".to_string(),
            run_id: "run-1".to_string(),
            status: RunStatus::Queued,
            stop_reason: None,
            task_name: "test".to_string(),
            task_path: "task.yaml".to_string(),
            repo_root: "/tmp/repo".to_string(),
            profile: None,
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
        save_run_record(temp_dir.path(), &record).unwrap();
        append_event(
            temp_dir.path(),
            &RunEvent {
                timestamp: Utc::now(),
                event_type: "started".to_string(),
                iteration: Some(1),
                message: "started".to_string(),
                payload: None,
            },
        )
        .unwrap();

        assert!(run_file(temp_dir.path()).exists());
        assert!(events_file(temp_dir.path()).exists());
    }
}
