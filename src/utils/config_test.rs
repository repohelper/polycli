#[cfg(test)]
mod tests {
    use crate::utils::config::Config;
    use crate::utils::validation::ProfileName;
    use tempfile::TempDir;

    #[test]
    fn test_config_new_with_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::new(Some(temp_dir.path().to_path_buf())).unwrap();

        assert!(config.profiles_dir().exists());
        assert!(config.backup_dir().exists());
        assert_eq!(config.profiles_dir(), temp_dir.path());
    }

    #[test]
    fn test_profile_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let name = ProfileName::try_from("test-profile").unwrap();
        let profile_path = config.profile_path_validated(&name).unwrap();
        assert_eq!(profile_path, temp_dir.path().join("test-profile"));
    }

    #[test]
    fn test_critical_files_list() {
        let files = Config::critical_files();
        assert!(files.contains(&"auth.json"));
        assert!(files.contains(&"config.toml"));
        assert!(files.contains(&"history.jsonl"));
        assert!(files.contains(&"sessions/"));
        assert!(files.contains(&"memories/"));
    }

    #[test]
    fn test_config_clone() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let _ = config;
    }
}
