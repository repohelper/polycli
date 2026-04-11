use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

use crate::utils::files::{copy_dir_recursive, copy_profile_files};

/// Atomic profile switcher using a stage-then-rename strategy.
///
/// The staging directory is created in the same parent as the target so that
/// `std::fs::rename` is always a same-filesystem, atomic operation.
///
/// # Lifecycle
///
/// 1. [`ProfileTransaction::new`] – allocate staging space.
/// 2. [`stage_profile`] – populate staging with profile files.
/// 3. [`commit`] – atomically swap staging → target (old target saved internally).
/// 4. [`cleanup_original`] – drop the pre-commit snapshot when no longer needed.
/// 5. [`rollback`] – restore the pre-commit snapshot (can be called after commit too).
///
/// On `Drop`, any un-committed staging directory is removed automatically.
pub struct ProfileTransaction {
    target_dir: PathBuf,
    /// Temp directory for the incoming profile (same filesystem as target).
    staging_dir: PathBuf,
    /// Where the original `target_dir` was moved to during `commit()` (enables rollback).
    original_dir: Option<PathBuf>,
    staged: bool,
    committed: bool,
}

impl ProfileTransaction {
    /// Create a new transaction targeting `target_dir`.
    ///
    /// The staging directory is placed in the same parent directory as `target_dir`
    /// to guarantee a same-filesystem rename on commit.
    ///
    /// # Errors
    ///
    /// Returns an error if the staging directory cannot be created.
    pub fn new(target_dir: impl Into<PathBuf>) -> Result<Self> {
        let target_dir = target_dir.into();
        let parent = target_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        std::fs::create_dir_all(&parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;

        let staging_dir = unique_sibling_path(&parent, ".codex_txn_staging")?;
        std::fs::create_dir_all(&staging_dir).with_context(|| {
            format!(
                "Failed to create staging directory: {}",
                staging_dir.display()
            )
        })?;

        Ok(Self {
            target_dir,
            staging_dir,
            original_dir: None,
            staged: false,
            committed: false,
        })
    }

    /// Copy profile files from `src` into the staging directory.
    ///
    /// Only the listed `files` patterns are copied; the staging directory starts
    /// empty, so any file absent from the profile is absent after commit.
    ///
    /// # Errors
    ///
    /// Returns an error if any file copy fails.
    pub fn stage_profile(&mut self, src: &Path, files: &[&str]) -> Result<Vec<String>> {
        let copied = copy_profile_files(src, &self.staging_dir, files)
            .context("Failed to stage profile files")?;
        self.staged = true;
        Ok(copied)
    }

    /// Stage an entire directory tree into the staging directory.
    ///
    /// Used by `run` to restore the original codex state after a command.
    ///
    /// # Errors
    ///
    /// Returns an error if any file copy fails.
    #[allow(dead_code)]
    pub fn stage_dir(&mut self, src: &Path) -> Result<()> {
        copy_dir_recursive(src, &self.staging_dir).context("Failed to stage directory")?;
        self.staged = true;
        Ok(())
    }

    /// Atomically commit the staged profile to the target directory.
    ///
    /// The sequence is:
    /// 1. Rename target → `original_dir` (saves current state for rollback).
    /// 2. Rename staging → target (installs new profile atomically).
    ///
    /// Both renames are same-filesystem operations and therefore atomic on
    /// POSIX systems.
    ///
    /// # Errors
    ///
    /// Returns an error if either rename fails.
    pub fn commit(&mut self) -> Result<()> {
        if !self.staged {
            anyhow::bail!("Cannot commit: no profile has been staged");
        }

        let parent = self.target_dir.parent().unwrap_or_else(|| Path::new("."));

        // Move existing target out of the way so we can rename staging into its place.
        if self.target_dir.exists() {
            let orig_path = unique_sibling_path(parent, ".codex_txn_orig")?;
            std::fs::rename(&self.target_dir, &orig_path).with_context(|| {
                format!(
                    "Failed to move current profile aside: {}",
                    self.target_dir.display()
                )
            })?;
            self.original_dir = Some(orig_path);
        }

        // Atomic rename: staging → target.
        std::fs::rename(&self.staging_dir, &self.target_dir).with_context(|| {
            format!(
                "Failed to rename staging dir to target: {}",
                self.target_dir.display()
            )
        })?;

        self.committed = true;
        Ok(())
    }

    /// Rollback to the state before `commit()`.
    ///
    /// - If called before `commit()`: removes the staging directory.
    /// - If called after `commit()`: removes the committed state and restores
    ///   the original, or just removes the target when there was no original.
    ///
    /// # Errors
    ///
    /// Returns an error if any filesystem operation fails.
    pub fn rollback(&mut self) -> Result<()> {
        if self.committed {
            // Remove what we committed.
            if self.target_dir.exists() {
                std::fs::remove_dir_all(&self.target_dir)
                    .context("Failed to remove committed profile during rollback")?;
            }

            // Restore original if there was one.
            if let Some(ref orig) = self.original_dir.take()
                && orig.exists()
            {
                std::fs::rename(orig, &self.target_dir)
                    .context("Failed to restore original profile during rollback")?;
            }

            self.committed = false;
        } else {
            // Not yet committed; just wipe the staging directory.
            if self.staging_dir.exists() {
                std::fs::remove_dir_all(&self.staging_dir)
                    .context("Failed to clean up staging directory during rollback")?;
            }
            self.staged = false;
        }

        Ok(())
    }

    /// Remove the pre-commit snapshot saved during `commit()`.
    ///
    /// Call this once the committed state is confirmed good and the original
    /// is no longer needed for rollback.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be removed.
    pub fn cleanup_original(&self) -> Result<()> {
        if let Some(ref orig) = self.original_dir
            && orig.exists()
        {
            std::fs::remove_dir_all(orig)
                .context("Failed to remove original backup after commit")?;
        }
        Ok(())
    }

    /// Path where the original target was saved after `commit()`, if any.
    #[must_use]
    #[cfg(test)]
    pub fn original_backup_path(&self) -> Option<&Path> {
        self.original_dir.as_deref()
    }

    /// The staging directory path.
    #[must_use]
    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }
}

impl Drop for ProfileTransaction {
    /// Ensure staging leftovers are cleaned up if the transaction is dropped
    /// without being committed (e.g. on early error return).
    fn drop(&mut self) {
        if !self.committed && self.staging_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.staging_dir);
        }
    }
}

/// Generate a path with a unique suffix so it does not collide with
/// any existing entry in `parent`.
///
/// Uses a combination of timestamp and random suffix to avoid collisions
/// under concurrent access.
#[allow(clippy::unnecessary_wraps)]
fn unique_sibling_path(parent: &Path, prefix: &str) -> Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());

    let rand_suffix: u64 = rand::random::<u64>();

    let path = parent.join(format!("{prefix}_{ts}_{rand_suffix:016x}"));
    Ok(path)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn make_profile(dir: &TempDir) -> PathBuf {
        let profile = dir.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join("auth.json"), r#"{"token":"test"}"#).unwrap();
        std::fs::write(profile.join("config.toml"), "model = \"o4-mini\"").unwrap();
        profile
    }

    #[test]
    fn test_stage_and_commit_creates_target() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");
        let profile_src = make_profile(&tmp);

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_profile(&profile_src, &["auth.json", "config.toml"])
            .unwrap();
        txn.commit().unwrap();

        assert!(target.exists(), "target should exist after commit");
        assert!(target.join("auth.json").exists());
        assert!(target.join("config.toml").exists());
    }

    #[test]
    fn test_commit_clears_stale_files() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");
        std::fs::create_dir_all(&target).unwrap();
        // Write a stale file that is NOT in the new profile.
        std::fs::write(target.join("stale.json"), "old").unwrap();

        let profile_src = make_profile(&tmp);

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_profile(&profile_src, &["auth.json"]).unwrap();
        txn.commit().unwrap();

        // The committed target should only contain what was staged.
        assert!(target.join("auth.json").exists());
        assert!(
            !target.join("stale.json").exists(),
            "stale file must be gone after atomic switch"
        );
    }

    #[test]
    fn test_rollback_before_commit_removes_staging() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");
        let profile_src = make_profile(&tmp);

        let mut txn = ProfileTransaction::new(&target).unwrap();
        let staging = txn.staging_dir().to_path_buf();
        txn.stage_profile(&profile_src, &["auth.json"]).unwrap();

        assert!(staging.exists(), "staging should exist before rollback");
        txn.rollback().unwrap();

        assert!(!staging.exists(), "staging should be gone after rollback");
        assert!(!target.exists(), "target should not have been created");
    }

    #[test]
    fn test_rollback_after_commit_restores_original() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");

        // Pre-existing state.
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("auth.json"), r#"{"token":"original"}"#).unwrap();

        let profile_src = make_profile(&tmp); // has auth.json with "test" token

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_profile(&profile_src, &["auth.json"]).unwrap();
        txn.commit().unwrap();

        // Confirm new profile is live.
        let committed = std::fs::read_to_string(target.join("auth.json")).unwrap();
        assert!(committed.contains("test"));

        // Rollback should restore original.
        txn.rollback().unwrap();

        let restored = std::fs::read_to_string(target.join("auth.json")).unwrap();
        assert!(
            restored.contains("original"),
            "original content should be restored after rollback"
        );
    }

    #[test]
    fn test_rollback_after_commit_no_original() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex"); // does not exist initially

        let profile_src = make_profile(&tmp);

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_profile(&profile_src, &["auth.json"]).unwrap();
        txn.commit().unwrap();

        assert!(target.exists());

        txn.rollback().unwrap();

        assert!(
            !target.exists(),
            "target should be removed when there was no original"
        );
    }

    #[test]
    fn test_cleanup_original_removes_backup() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("auth.json"), "old").unwrap();

        let profile_src = make_profile(&tmp);

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_profile(&profile_src, &["auth.json"]).unwrap();
        txn.commit().unwrap();

        let orig_path = txn.original_backup_path().map(PathBuf::from);
        assert!(orig_path.is_some());
        let orig_path = orig_path.unwrap();
        assert!(
            orig_path.exists(),
            "original backup should exist after commit"
        );

        txn.cleanup_original().unwrap();
        assert!(
            !orig_path.exists(),
            "original backup should be gone after cleanup"
        );
    }

    #[test]
    fn test_drop_cleans_up_uncommitted_staging() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");
        let profile_src = make_profile(&tmp);

        let staging_path = {
            let mut txn = ProfileTransaction::new(&target).unwrap();
            txn.stage_profile(&profile_src, &["auth.json"]).unwrap();
            txn.staging_dir().to_path_buf()
            // txn dropped here without commit
        };

        assert!(
            !staging_path.exists(),
            "staging dir should be removed on drop"
        );
    }

    #[test]
    fn test_commit_without_stage_returns_error() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");

        let mut txn = ProfileTransaction::new(&target).unwrap();
        let result = txn.commit();
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_dir_stages_entire_tree() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("codex");

        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("auth.json"), "a").unwrap();
        std::fs::create_dir_all(src.join("sessions")).unwrap();
        std::fs::write(src.join("sessions").join("s1.json"), "s").unwrap();

        let mut txn = ProfileTransaction::new(&target).unwrap();
        txn.stage_dir(&src).unwrap();
        txn.commit().unwrap();

        assert!(target.join("auth.json").exists());
        assert!(target.join("sessions").join("s1.json").exists());
    }
}
