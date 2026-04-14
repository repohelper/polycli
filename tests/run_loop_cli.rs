#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::{Value, json};
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn run_loop_succeeds_and_persists_run_state() {
    let fixture = CliFixture::new();
    let task_path = fixture.write_task(
        "success.yaml",
        r#"
type: codexctl-bet/v1
name: success-bet
appetite: 1_week
objective: Produce a file and validate it
bounded_contexts:
  - Run Orchestration
success_signal: The loop creates loop.txt and validation passes
no_gos:
  - Do not add queueing.
acceptance_checks:
  - test -f loop.txt
agent:
  command:
    - bash
    - -lc
    - printf 'done\n' > loop.txt
"#,
    );

    let output = fixture
        .command()
        .args(["run-loop", "--task", task_path.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).unwrap();
    let run_id = payload["run_id"].as_str().unwrap();
    assert_eq!(payload["schema_version"], "run_loop/v1");
    assert_eq!(payload["status"], "succeeded");
    assert_eq!(payload["iteration_count"], 1);

    let run_dir = fixture.config_dir.join("runs").join(run_id);
    assert!(run_dir.join("run.json").exists());
    assert!(run_dir.join("final-report.md").exists());
    assert!(run_dir.join("task.snapshot.yaml").exists());
    assert!(run_dir.join("iterations").join("001.prompt.md").exists());
    assert!(run_dir.join("iterations").join("001.summary.md").exists());
    assert!(
        run_dir
            .join("iterations")
            .join("001.validation.json")
            .exists()
    );

    let latest_output = fixture
        .command()
        .args(["runs", "--latest", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let latest_payload: Value = serde_json::from_slice(&latest_output).unwrap();
    assert_eq!(latest_payload["run"]["run_id"], run_id);
    assert_eq!(latest_payload["run"]["status"], "succeeded");

    let list_output = fixture
        .command()
        .args(["runs", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_payload: Value = serde_json::from_slice(&list_output).unwrap();
    assert_eq!(list_payload["items"].as_array().unwrap().len(), 1);
}

#[test]
#[serial]
fn run_loop_returns_exit_21_when_budget_is_exhausted() {
    let fixture = CliFixture::new();
    let task_path = fixture.write_task(
        "budget.yaml",
        r#"
type: codexctl-bet/v1
name: failing-bet
appetite: 1_week
objective: Demonstrate a failing run
bounded_contexts:
  - Validation
success_signal: Acceptance checks stay red
no_gos:
  - Do not mutate the budget logic.
acceptance_checks:
  - false
agent:
  command:
    - bash
    - -lc
    - printf 'ran\n' > ran.txt
budgets:
  max_iterations: 1
  max_consecutive_failures: 1
"#,
    );

    let output = fixture
        .command()
        .args(["run-loop", "--task", task_path.to_str().unwrap(), "--json"])
        .assert()
        .failure()
        .code(21)
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["status"], "budget_exhausted");
    assert_eq!(payload["stop_reason"], "max_consecutive_failures_reached");
    assert_eq!(payload["latest_validation"]["failed"], 1);
}

#[test]
#[serial]
fn run_loop_resume_executes_from_persisted_repo_root() {
    let fixture = CliFixture::new();
    let run_id = "20260414T000000Z-deadbeef";
    let run_dir = fixture.config_dir.join("runs").join(run_id);
    fs::create_dir_all(run_dir.join("iterations")).unwrap();
    fs::create_dir_all(run_dir.join("logs")).unwrap();

    fs::write(
        run_dir.join("task.snapshot.yaml"),
        r#"
type: codexctl-bet/v1
name: resumed-bet
appetite: 1_week
objective: Resume from a persisted run record
bounded_contexts:
  - Run Ledger
success_signal: The resumed run creates resumed.txt
no_gos:
  - Do not reinitialize the run.
acceptance_checks:
  - test -f resumed.txt
agent:
  command:
    - bash
    - -lc
    - printf 'resumed\n' > resumed.txt
"#,
    )
    .unwrap();

    fs::write(
        run_dir.join("run.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": "runs/v1",
            "run_id": run_id,
            "status": "queued",
            "stop_reason": null,
            "task_name": "resumed-bet",
            "task_path": fixture.repo_dir.join(".codexctl/tasks/resume.yaml").display().to_string(),
            "repo_root": fixture.repo_dir.display().to_string(),
            "profile": null,
            "auth_mode": null,
            "iteration_count": 0,
            "started_at": "2026-04-14T00:00:00Z",
            "updated_at": "2026-04-14T00:00:00Z",
            "finished_at": null,
            "latest_validation": {
                "status": null,
                "passed": 0,
                "failed": 0,
                "timed_out": 0,
                "errors": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let alternate_dir = fixture.temp.path().join("elsewhere");
    fs::create_dir_all(&alternate_dir).unwrap();

    let output = fixture
        .command_in(&alternate_dir)
        .args(["run-loop", "--resume", run_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["status"], "succeeded");
    assert!(fixture.repo_dir.join("resumed.txt").exists());
}

#[test]
#[serial]
fn run_loop_uses_profile_auth_and_restores_original_auth() {
    let fixture = CliFixture::new();
    fixture.write_profile("builder", br#"{"api_key":"profile-key"}"#);
    fixture.write_live_auth(br#"{"api_key":"original-key"}"#);
    let task_path = fixture.write_task(
        "profile.yaml",
        r#"
type: codexctl-bet/v1
name: profile-bet
appetite: 1_week
objective: Confirm profile auth is active during the agent step
bounded_contexts:
  - Auth Switching
success_signal: The agent observes profile auth and local auth is restored after the run
no_gos:
  - Do not mutate non-auth Codex state.
acceptance_checks:
  - grep -q 'profile-key' profile_seen.json
agent:
  command:
    - bash
    - -lc
    - cat "$HOME/.codex/auth.json" > profile_seen.json
"#,
    );

    fixture
        .command()
        .args([
            "run-loop",
            "--task",
            task_path.to_str().unwrap(),
            "--profile",
            "builder",
            "--json",
        ])
        .assert()
        .success();

    assert!(fixture.repo_dir.join("profile_seen.json").exists());
    let seen_auth = fs::read_to_string(fixture.repo_dir.join("profile_seen.json")).unwrap();
    assert!(seen_auth.contains("profile-key"));

    let live_auth = fs::read_to_string(fixture.home_dir.join(".codex").join("auth.json")).unwrap();
    assert!(live_auth.contains("original-key"));
}

struct CliFixture {
    temp: TempDir,
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
        fs::create_dir_all(config_dir.join("runs")).unwrap();
        fs::create_dir_all(&repo_dir).unwrap();

        Self {
            temp,
            home_dir,
            config_dir,
            repo_dir,
        }
    }

    fn command(&self) -> Command {
        self.command_in(&self.repo_dir)
    }

    fn command_in(&self, cwd: &PathBuf) -> Command {
        let mut cmd = Command::cargo_bin("codexctl").unwrap();
        cmd.current_dir(cwd)
            .env("HOME", &self.home_dir)
            .env("CODEXCTL_DIR", &self.config_dir)
            .env("NO_COLOR", "1");
        cmd
    }

    fn write_task(&self, file_name: &str, content: &str) -> PathBuf {
        let tasks_dir = self.repo_dir.join(".codexctl").join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        let path = tasks_dir.join(file_name);
        fs::write(&path, content).unwrap();
        path
    }

    fn write_profile(&self, name: &str, auth_json: &[u8]) {
        let profile_dir = self.config_dir.join(name);
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(profile_dir.join("auth.json"), auth_json).unwrap();
    }

    fn write_live_auth(&self, auth_json: &[u8]) {
        fs::write(self.home_dir.join(".codex").join("auth.json"), auth_json).unwrap();
    }
}
