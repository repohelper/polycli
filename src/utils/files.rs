use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use walkdir::WalkDir;

use crate::utils::config::Config;

/// Copy files from source to destination, preserving directory structure
///
/// # Errors
///
/// Returns an error if any file operation fails
pub fn copy_profile_files(src: &Path, dst: &Path, files: &[&str]) -> Result<Vec<String>> {
    let mut copied = Vec::new();

    for pattern in files {
        let src_path = src.join(pattern);

        if src_path.is_file() {
            let dst_path = dst.join(pattern);
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
            copied.push((*pattern).to_string());
        } else if src_path.is_dir() {
            // Copy entire directory
            copy_dir_recursive(&src_path, &dst.join(pattern))?;
            copied.push(format!("{pattern}/ (directory)"));
        }
    }

    Ok(copied)
}

/// Recursively copy directory
///
/// # Errors
///
/// Returns an error if any directory creation or file copy fails
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory: {}", dst.display()))?;

    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(src) else {
            continue;
        };
        let destination = dst.join(relative);

        if path.is_file() {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, destination)?;
        } else if path.is_dir() && path != src {
            std::fs::create_dir_all(destination)?;
        }
    }

    Ok(())
}

/// Create a backup of the current codex config
///
/// # Errors
///
/// Returns an error if the source directory doesn't exist or copy fails
pub fn create_backup(codex_dir: &Path, backup_dir: &Path) -> Result<PathBuf> {
    use chrono::Local;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("codex_backup_{timestamp}");
    let backup_path = backup_dir.join(&backup_name);

    if !codex_dir.exists() {
        anyhow::bail!("Codex directory does not exist: {}", codex_dir.display());
    }

    copy_dir_recursive(codex_dir, &backup_path)?;

    Ok(backup_path)
}

/// Check if codex CLI is installed
#[must_use]
#[allow(dead_code)]
pub fn check_codex_installed() -> bool {
    which::which("codex").is_ok()
}

/// Get critical files to sync.
///
/// Delegates to [`Config::critical_files`] to keep the list in one place.
#[must_use]
pub fn get_critical_files() -> &'static [&'static str] {
    Config::critical_files()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_copy_dir_recursive() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create source structure
        let sub_dir = src_dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        std::fs::write(src_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(sub_dir.join("file2.txt"), "content2").unwrap();

        // Copy
        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        // Verify
        assert!(dst_dir.path().join("file1.txt").exists());
        assert!(dst_dir.path().join("subdir/file2.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dst_dir.path().join("file1.txt")).unwrap(),
            "content1"
        );
    }

    #[test]
    fn test_copy_profile_files() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create source files
        std::fs::write(src_dir.path().join("auth.json"), "{\"token\": \"test\"}").unwrap();
        std::fs::write(src_dir.path().join("config.toml"), "model = \"gpt-4\"").unwrap();

        // Create sessions directory
        let sessions_dir = src_dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::write(sessions_dir.join("session1.json"), "{}").unwrap();

        // Copy
        let files = vec!["auth.json", "config.toml", "sessions/"];
        let copied = copy_profile_files(src_dir.path(), dst_dir.path(), &files).unwrap();

        // Verify
        assert!(copied.contains(&"auth.json".to_string()));
        assert!(copied.contains(&"config.toml".to_string()));
        // Check for directory (with trailing slash stripped)
        assert!(copied.iter().any(|c| c.starts_with("sessions")));
        assert!(dst_dir.path().join("auth.json").exists());
        assert!(dst_dir.path().join("sessions/session1.json").exists());
    }

    #[test]
    fn test_create_backup() {
        let codex_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        // Create codex content
        std::fs::write(
            codex_dir.path().join("auth.json"),
            "{\"token\": \"secret\"}",
        )
        .unwrap();

        // Create backup
        let backup_path = create_backup(codex_dir.path(), backup_dir.path()).unwrap();

        // Verify
        assert!(backup_path.exists());
        assert!(backup_path.join("auth.json").exists());
        assert_eq!(
            std::fs::read_to_string(backup_path.join("auth.json")).unwrap(),
            "{\"token\": \"secret\"}"
        );
    }

    #[test]
    fn test_create_backup_nonexistent() {
        let backup_dir = TempDir::new().unwrap();
        let nonexistent = std::path::Path::new("/nonexistent/path/that/does/not/exist");

        let result = create_backup(nonexistent, backup_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_critical_files() {
        let files = get_critical_files();
        assert!(!files.is_empty());
        assert!(files.contains(&"auth.json"));
        assert!(files.contains(&"config.toml"));
    }

    #[test]
    fn test_check_codex_installed() {
        // This is a simple smoke test - can't guarantee codex is/isn't installed
        let _result = check_codex_installed();
        // Result will be true or false depending on system
    }
}
