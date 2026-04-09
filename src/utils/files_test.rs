#[cfg(test)]
mod tests {
    use crate::utils::files::{
        check_codex_installed, copy_dir_recursive, copy_profile_files, create_backup,
        get_critical_files,
    };
    use tempfile::TempDir;

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
