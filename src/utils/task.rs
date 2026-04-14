use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::utils::command_exit::fail;

const REQUIRED_FIELDS_EXIT_CODE: u8 = 13;
const INVALID_SPEC_EXIT_CODE: u8 = 12;
const BET_SCHEMA_TYPE: &str = "codexctl-bet/v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BetSpec {
    pub schema_type: String,
    pub name: String,
    pub appetite: String,
    pub objective: String,
    pub bounded_contexts: Vec<String>,
    pub success_signal: String,
    pub no_gos: Vec<String>,
    pub context_files: Vec<String>,
    pub constraints: Vec<String>,
    pub acceptance_checks: Vec<String>,
    pub review_checks: Vec<String>,
    pub agent: Option<BetAgent>,
    pub budgets: Option<BetBudgets>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BetAgent {
    pub prompt_preamble: Option<String>,
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BetBudgets {
    pub max_iterations: Option<u32>,
    pub max_runtime_minutes: Option<u32>,
    pub max_consecutive_failures: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawBetSpec {
    #[serde(rename = "type")]
    schema_type: Option<String>,
    name: Option<String>,
    appetite: Option<String>,
    objective: Option<String>,
    bounded_contexts: Option<Vec<String>>,
    success_signal: Option<String>,
    no_gos: Option<Vec<String>>,
    context_files: Option<Vec<String>>,
    constraints: Option<Vec<String>>,
    acceptance_checks: Option<Vec<String>>,
    review_checks: Option<Vec<String>>,
    agent: Option<BetAgent>,
    budgets: Option<BetBudgets>,
    notes: Option<String>,
}

impl BetSpec {
    pub async fn load_from_path(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read bet spec: {}", path.display()))
            .or_else(|_| {
                fail(
                    INVALID_SPEC_EXIT_CODE,
                    format!("Failed to read bet spec: {}", path.display()),
                )
            })?;

        let raw: RawBetSpec = serde_yaml::from_str(&content).map_err(|e| {
            crate::utils::command_exit::CommandExitError::new(
                INVALID_SPEC_EXIT_CODE,
                format!("Invalid bet spec YAML in {}: {e}", path.display()),
            )
        })?;

        raw.validate(path)
    }
}

impl RawBetSpec {
    fn validate(self, path: &Path) -> Result<BetSpec> {
        let schema_type = required_string(self.schema_type, "type", path)?;
        if schema_type != BET_SCHEMA_TYPE {
            return fail(
                INVALID_SPEC_EXIT_CODE,
                format!(
                    "Unsupported bet spec type in {}: expected '{}', got '{}'",
                    path.display(),
                    BET_SCHEMA_TYPE,
                    schema_type
                ),
            );
        }

        let name = required_string(self.name, "name", path)?;
        let appetite = required_string(self.appetite, "appetite", path)?;
        validate_appetite(&appetite, path)?;

        let objective = required_string(self.objective, "objective", path)?;
        let bounded_contexts =
            required_non_empty_strings(self.bounded_contexts, "bounded_contexts", path)?;
        let success_signal = required_string(self.success_signal, "success_signal", path)?;
        let no_gos = required_non_empty_strings(self.no_gos, "no_gos", path)?;
        let acceptance_checks =
            required_non_empty_strings(self.acceptance_checks, "acceptance_checks", path)?;

        let context_files = optional_non_empty_strings(self.context_files, "context_files", path)?;
        for context_file in &context_files {
            validate_repo_relative_path(context_file, path)?;
        }

        let constraints = optional_non_empty_strings(self.constraints, "constraints", path)?;
        let review_checks = optional_non_empty_strings(self.review_checks, "review_checks", path)?;
        validate_agent(self.agent.as_ref(), path)?;
        validate_budgets(self.budgets.as_ref(), path)?;

        Ok(BetSpec {
            schema_type,
            name,
            appetite,
            objective,
            bounded_contexts,
            success_signal,
            no_gos,
            context_files,
            constraints,
            acceptance_checks,
            review_checks,
            agent: self.agent,
            budgets: self.budgets,
            notes: self.notes,
        })
    }
}

fn required_string(value: Option<String>, field: &str, path: &Path) -> Result<String> {
    match value.map(|v| v.trim().to_string()) {
        Some(value) if !value.is_empty() => Ok(value),
        _ => fail(
            REQUIRED_FIELDS_EXIT_CODE,
            format!(
                "Missing required bet field '{}' in {}",
                field,
                path.display()
            ),
        ),
    }
}

fn required_non_empty_strings(
    value: Option<Vec<String>>,
    field: &str,
    path: &Path,
) -> Result<Vec<String>> {
    let values = value.ok_or_else(|| {
        crate::utils::command_exit::CommandExitError::new(
            REQUIRED_FIELDS_EXIT_CODE,
            format!(
                "Missing required bet field '{}' in {}",
                field,
                path.display()
            ),
        )
    })?;

    normalize_string_list(values, field, path, true)
}

fn optional_non_empty_strings(
    value: Option<Vec<String>>,
    field: &str,
    path: &Path,
) -> Result<Vec<String>> {
    match value {
        Some(values) => normalize_string_list(values, field, path, false),
        None => Ok(Vec::new()),
    }
}

fn normalize_string_list(
    values: Vec<String>,
    field: &str,
    path: &Path,
    required_non_empty: bool,
) -> Result<Vec<String>> {
    let values: Vec<String> = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();

    if required_non_empty && values.is_empty() {
        return fail(
            REQUIRED_FIELDS_EXIT_CODE,
            format!(
                "Bet field '{}' must contain at least one value in {}",
                field,
                path.display()
            ),
        );
    }

    Ok(values)
}

fn validate_appetite(appetite: &str, path: &Path) -> Result<()> {
    let Some((count, unit)) = appetite.split_once('_') else {
        return fail(
            INVALID_SPEC_EXIT_CODE,
            format!(
                "Invalid appetite '{}' in {}: expected '<number>_<unit>'",
                appetite,
                path.display()
            ),
        );
    };

    if count
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .is_none()
    {
        return fail(
            INVALID_SPEC_EXIT_CODE,
            format!(
                "Invalid appetite '{}' in {}: count must be a positive integer",
                appetite,
                path.display()
            ),
        );
    }

    let valid_unit = matches!(unit, "day" | "days" | "week" | "weeks");
    if !valid_unit {
        return fail(
            INVALID_SPEC_EXIT_CODE,
            format!(
                "Invalid appetite '{}' in {}: unit must be day(s) or week(s)",
                appetite,
                path.display()
            ),
        );
    }

    Ok(())
}

fn validate_repo_relative_path(value: &str, path: &Path) -> Result<()> {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        return fail(
            INVALID_SPEC_EXIT_CODE,
            format!(
                "Context file '{}' in {} must be a relative path",
                value,
                path.display()
            ),
        );
    }

    if candidate.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return fail(
            INVALID_SPEC_EXIT_CODE,
            format!(
                "Context file '{}' in {} must stay within the repository",
                value,
                path.display()
            ),
        );
    }

    Ok(())
}

fn validate_agent(agent: Option<&BetAgent>, path: &Path) -> Result<()> {
    let Some(agent) = agent else {
        return Ok(());
    };

    if let Some(command) = &agent.command {
        if command.is_empty() || command.iter().all(|item| item.trim().is_empty()) {
            return fail(
                INVALID_SPEC_EXIT_CODE,
                format!(
                    "Bet field 'agent.command' must not be empty in {}",
                    path.display()
                ),
            );
        }
    }

    Ok(())
}

fn validate_budgets(budgets: Option<&BetBudgets>, path: &Path) -> Result<()> {
    let Some(budgets) = budgets else {
        return Ok(());
    };

    for (field, value) in [
        ("max_iterations", budgets.max_iterations),
        ("max_runtime_minutes", budgets.max_runtime_minutes),
        ("max_consecutive_failures", budgets.max_consecutive_failures),
    ] {
        if let Some(0) = value {
            return fail(
                INVALID_SPEC_EXIT_CODE,
                format!(
                    "Bet budget '{}' must be greater than zero in {}",
                    field,
                    path.display()
                ),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn loads_minimal_valid_bet_spec() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("bet.yaml");
        tokio::fs::write(
            &spec_path,
            r#"
type: codexctl-bet/v1
name: test-bet
appetite: 2_weeks
objective: Ship one thing
bounded_contexts:
  - Validation
success_signal: It works
no_gos:
  - Do not add queueing.
acceptance_checks:
  - true
"#,
        )
        .await
        .unwrap();

        let spec = BetSpec::load_from_path(&spec_path).await.unwrap();
        assert_eq!(spec.name, "test-bet");
        assert_eq!(spec.schema_type, BET_SCHEMA_TYPE);
        assert_eq!(spec.acceptance_checks, vec!["true"]);
    }

    #[tokio::test]
    async fn missing_required_shape_fields_returns_required_fields_code() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("bet.yaml");
        tokio::fs::write(
            &spec_path,
            r#"
type: codexctl-bet/v1
name: test-bet
objective: Ship one thing
bounded_contexts:
  - Validation
success_signal: It works
no_gos:
  - Do not add queueing.
acceptance_checks:
  - true
"#,
        )
        .await
        .unwrap();

        let error = BetSpec::load_from_path(&spec_path).await.unwrap_err();
        let exit_error = error
            .downcast_ref::<crate::utils::command_exit::CommandExitError>()
            .unwrap();
        assert_eq!(exit_error.code(), REQUIRED_FIELDS_EXIT_CODE);
    }

    #[tokio::test]
    async fn invalid_context_file_path_returns_invalid_spec_code() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("bet.yaml");
        tokio::fs::write(
            &spec_path,
            r#"
type: codexctl-bet/v1
name: test-bet
appetite: 2_weeks
objective: Ship one thing
bounded_contexts:
  - Validation
success_signal: It works
no_gos:
  - Do not add queueing.
context_files:
  - ../secret
acceptance_checks:
  - true
"#,
        )
        .await
        .unwrap();

        let error = BetSpec::load_from_path(&spec_path).await.unwrap_err();
        let exit_error = error
            .downcast_ref::<crate::utils::command_exit::CommandExitError>()
            .unwrap();
        assert_eq!(exit_error.code(), INVALID_SPEC_EXIT_CODE);
    }
}
