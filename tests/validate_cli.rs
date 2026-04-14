#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn validate_json_passes_for_valid_bet_spec() {
    let fixture = CliFixture::new();
    let task_path = fixture.write_task(
        "valid.yaml",
        r#"
type: codexctl-bet/v1
name: valid-bet
appetite: 2_weeks
objective: Confirm validation succeeds
bounded_contexts:
  - Validation
success_signal: Validation passes
no_gos:
  - Do not add queueing.
acceptance_checks:
  - true
"#,
    );

    let output = fixture
        .command()
        .args(["validate", "--task", task_path.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["schema_version"], "validate/v1");
    assert_eq!(payload["status"], "passed");
    assert_eq!(payload["summary"]["passed"], 1);
}

#[test]
#[serial]
fn validate_returns_exit_10_for_failing_check() {
    let fixture = CliFixture::new();
    fixture
        .command()
        .args(["validate", "--check", "false", "--json"])
        .assert()
        .failure()
        .code(10);
}

#[test]
#[serial]
fn validate_returns_exit_11_for_timed_out_check() {
    let fixture = CliFixture::new();
    fixture
        .command()
        .args([
            "validate",
            "--check",
            "sleep 2",
            "--timeout-seconds",
            "1",
            "--json",
        ])
        .assert()
        .failure()
        .code(11);
}

#[test]
#[serial]
fn validate_returns_exit_13_for_missing_shapeup_fields() {
    let fixture = CliFixture::new();
    let task_path = fixture.write_task(
        "missing-fields.yaml",
        r#"
type: codexctl-bet/v1
name: invalid-bet
objective: Missing appetite and no_gos
bounded_contexts:
  - Validation
success_signal: It should fail
acceptance_checks:
  - true
"#,
    );

    fixture
        .command()
        .args(["validate", "--task", task_path.to_str().unwrap(), "--json"])
        .assert()
        .failure()
        .code(13);
}

struct CliFixture {
    _temp: TempDir,
    home_dir: PathBuf,
    config_dir: PathBuf,
    repo_dir: PathBuf,
}

impl CliFixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let home_dir = temp.path().join("home");
        let config_dir = temp.path().join("profiles");
        let repo_dir = temp.path().join("repo");
        fs::create_dir_all(home_dir.join(".codex")).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&repo_dir).unwrap();

        Self {
            _temp: temp,
            home_dir,
            config_dir,
            repo_dir,
        }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("codexctl").unwrap();
        cmd.current_dir(&self.repo_dir)
            .env("HOME", &self.home_dir)
            .env("CODEXCTL_DIR", &self.config_dir);
        cmd
    }

    fn write_task(&self, file_name: &str, content: &str) -> PathBuf {
        let tasks_dir = self.repo_dir.join(".codexctl").join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        let path = tasks_dir.join(file_name);
        fs::write(&path, content).unwrap();
        path
    }
}

#[allow(dead_code)]
fn _assert_exists(path: &Path) {
    assert!(path.exists(), "{} should exist", path.display());
}
