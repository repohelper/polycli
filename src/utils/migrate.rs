//! Auto-migration system for seamless version upgrades
//!
//! This module handles automatic migration of profiles and config
//! when upgrading between versions. No user intervention required.

use crate::utils::config::Config;
use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Current schema version of codexctl
const CURRENT_SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Migration metadata stored in profiles directory
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationMeta {
    schema_version: String,
    last_migration: chrono::DateTime<chrono::Utc>,
    migrations_applied: Vec<String>,
}

impl Default for MigrationMeta {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            last_migration: chrono::Utc::now(),
            migrations_applied: Vec::new(),
        }
    }
}

/// Check and apply any pending migrations
pub async fn auto_migrate(config: &Config) -> Result<()> {
    let meta_path = config.profiles_dir().join(".migration_meta.json");

    // Ensure profiles directory exists
    tokio::fs::create_dir_all(config.profiles_dir()).await.ok();

    // Read current migration state
    let mut meta = if meta_path.exists() {
        match tokio::fs::read_to_string(&meta_path).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => MigrationMeta::default(),
        }
    } else {
        MigrationMeta::default()
    };

    let current = Version::parse(CURRENT_SCHEMA_VERSION)?;
    let stored =
        Version::parse(&meta.schema_version).unwrap_or_else(|_| Version::parse("0.0.0").unwrap());

    // No migration needed, but still update the meta file to track we're on current version
    if stored >= current && meta_path.exists() {
        return Ok(());
    }

    tracing::info!("Migrating from {} to {}", stored, current);

    // Apply migrations in order
    if stored < Version::parse("0.4.0")? {
        migrate_to_v0_4_0(config, &mut meta).await?;
    }

    // Future migrations:
    // if stored < Version::parse("0.5.0")? {
    //     migrate_to_v0_5_0(config, &mut meta).await?;
    // }

    // Update migration metadata
    meta.schema_version = CURRENT_SCHEMA_VERSION.to_string();
    meta.last_migration = chrono::Utc::now();

    let meta_json = serde_json::to_string_pretty(&meta)?;
    tokio::fs::write(&meta_path, meta_json).await?;

    tracing::info!("Migration complete");
    Ok(())
}

/// Migration to v0.4.0
/// - Adds encrypted field to profile metadata if missing
/// - Creates marker files for quick-switch if not present
async fn migrate_to_v0_4_0(config: &Config, meta: &mut MigrationMeta) -> Result<()> {
    tracing::info!("Applying migration: v0.4.0");

    let profiles_dir = config.profiles_dir();
    if !profiles_dir.exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(profiles_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden directories and special files
        if name.starts_with('.') || name == "backups" {
            continue;
        }

        // Check and update profile.json
        let profile_json_path = path.join("profile.json");
        if profile_json_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&profile_json_path).await {
                if let Ok(mut profile_meta) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Add encrypted field if missing
                    if profile_meta.get("encrypted").is_none() {
                        profile_meta["encrypted"] = serde_json::json!(false);

                        if let Ok(updated) = serde_json::to_string_pretty(&profile_meta) {
                            let _ = tokio::fs::write(&profile_json_path, updated).await;
                        }
                    }
                }
            }
        }
    }

    // Create marker files if they don't exist
    let current_marker = profiles_dir.join(".current_profile");
    let previous_marker = profiles_dir.join(".previous_profile");

    if !current_marker.exists() {
        let _ = tokio::fs::write(&current_marker, "").await;
    }
    if !previous_marker.exists() {
        let _ = tokio::fs::write(&previous_marker, "").await;
    }

    meta.migrations_applied.push("v0.4.0".to_string());
    Ok(())
}

/// Force a full migration check and repair
#[allow(dead_code)]
pub async fn repair_profiles(config: &Config) -> Result<()> {
    tracing::info!("Running profile repair...");

    let profiles_dir = config.profiles_dir();
    if !profiles_dir.exists() {
        tokio::fs::create_dir_all(profiles_dir).await?;
    }

    // Reset migration state and re-run all migrations
    let meta_path = profiles_dir.join(".migration_meta.json");
    let _ = tokio::fs::remove_file(&meta_path).await;

    auto_migrate(config).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_migration_meta_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::new(Some(temp_dir.path().to_path_buf())).unwrap();

        // Run migration
        auto_migrate(&config).await.unwrap();

        // Check meta file was created
        let meta_path = config.profiles_dir().join(".migration_meta.json");
        assert!(meta_path.exists());

        // Verify content
        let content = tokio::fs::read_to_string(&meta_path).await.unwrap();
        let meta: MigrationMeta = serde_json::from_str(&content).unwrap();
        assert_eq!(meta.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
