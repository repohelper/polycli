#[cfg(test)]
mod tests {
    use crate::utils::profile::{Profile, ProfileMeta};
    use tempfile::TempDir;

    #[test]
    fn test_profile_meta_new() {
        let meta = ProfileMeta::new(
            "test-profile".to_string(),
            Some("test@example.com".to_string()),
            Some("Test description".to_string()),
        );

        assert_eq!(meta.name, "test-profile");
        assert_eq!(meta.email, Some("test@example.com".to_string()));
        assert_eq!(meta.description, Some("Test description".to_string()));
        assert_eq!(meta.auth_mode, "chatgpt");
        assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_profile_meta_update() {
        let mut meta = ProfileMeta::new("test".to_string(), None, None);
        let original_updated = meta.updated_at;

        // Small delay to ensure time changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        meta.update();

        assert!(meta.updated_at > original_updated);
    }

    #[test]
    fn test_profile_new() {
        let profile = Profile::new(
            "my-profile".to_string(),
            Some("user@example.com".to_string()),
            Some("My description".to_string()),
        );

        assert_eq!(profile.meta.name, "my-profile");
        assert!(profile.files.is_empty());
    }

    #[test]
    fn test_profile_add_file() {
        let mut profile = Profile::new("test".to_string(), None, None);
        let content = b"test content".to_vec();

        profile.add_file("test.txt", content.clone());

        assert_eq!(profile.files.get("test.txt"), Some(&content));
    }

    #[test]
    fn test_profile_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let profile_dir = temp_dir.path().join("test-profile");

        // Create and save profile
        let mut profile = Profile::new(
            "test-profile".to_string(),
            Some("test@example.com".to_string()),
            Some("Description".to_string()),
        );
        profile.add_file("auth.json", b"{\"token\": \"test\"}".to_vec());
        profile.add_file("config.toml", b"model = \"gpt-4\"".to_vec());

        profile.save_to_disk(&profile_dir).unwrap();

        // Verify files exist
        assert!(profile_dir.exists());
        assert!(profile_dir.join("profile.json").exists());
        assert!(profile_dir.join("auth.json").exists());
        assert!(profile_dir.join("config.toml").exists());

        // Load profile back
        let loaded = Profile::load_from_disk(&profile_dir).unwrap();
        assert_eq!(loaded.meta.name, "test-profile");
        assert_eq!(
            loaded.files.get("auth.json"),
            Some(&b"{\"token\": \"test\"}".to_vec())
        );
    }

    #[test]
    fn test_extract_email_from_jwt() {
        // Create a mock JWT token with email in payload
        // Header: {"alg":"none"}
        // Payload: {"email":"test@example.com"}
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = URL_SAFE_NO_PAD.encode(b"{\"email\":\"jwt-test@example.com\"}");
        let signature = "dummy-signature";

        let mock_jwt = format!("{}.{}.{}", header, payload, signature);

        let mock_auth = format!("{{\"tokens\": {{\"id_token\": \"{}\"}}}}", mock_jwt);

        let mut profile = Profile::new("test".to_string(), None, None);
        profile.add_file("auth.json", mock_auth.into_bytes());

        let email = profile.extract_email();
        assert_eq!(email, Some("jwt-test@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_no_auth() {
        let profile = Profile::new("test".to_string(), None, None);
        assert!(profile.extract_email().is_none());
    }

    #[test]
    fn test_extract_email_invalid_jwt() {
        let mock_auth = r#"{"tokens": {"id_token": "invalid.jwt.format"}}"#;

        let mut profile = Profile::new("test".to_string(), None, None);
        profile.add_file("auth.json", mock_auth.as_bytes().to_vec());

        // Should return None for invalid JWT
        assert!(profile.extract_email().is_none());
    }
}
