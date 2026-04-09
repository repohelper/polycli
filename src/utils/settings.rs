//! Global configuration management for codexo
//!
//! Configuration is stored at `~/.config/codexo/config.toml`

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Global application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default profile to load on startup
    #[serde(default)]
    pub default_profile: Option<String>,

    /// Automatically create backups before operations
    #[serde(default = "default_auto_backup")]
    pub auto_backup: bool,

    /// Number of days to retain backups (0 = forever)
    #[serde(default = "default_backup_retention_days")]
    pub backup_retention_days: u32,

    /// Default quiet mode
    #[serde(default)]
    pub quiet_mode: bool,

    /// Default editor for interactive operations
    #[serde(default)]
    pub editor: Option<String>,

    /// Auto-switch profiles based on directory
    #[serde(default)]
    pub auto_switch: HashMap<String, String>,

    /// Notification settings
    #[serde(default)]
    pub notifications: NotificationSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_profile: None,
            auto_backup: default_auto_backup(),
            backup_retention_days: default_backup_retention_days(),
            quiet_mode: false,
            editor: None,
            auto_switch: HashMap::new(),
            notifications: NotificationSettings::default(),
        }
    }
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Weekly usage threshold percentage for alerts
    #[serde(default = "default_weekly_threshold")]
    pub weekly_threshold: u8,

    /// Email for notifications
    #[serde(default)]
    pub email: Option<String>,

    /// Slack webhook URL
    #[serde(default)]
    pub slack_webhook: Option<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            weekly_threshold: default_weekly_threshold(),
            email: None,
            slack_webhook: None,
        }
    }
}

fn default_auto_backup() -> bool {
    true
}

fn default_backup_retention_days() -> u32 {
    30
}

fn default_weekly_threshold() -> u8 {
    80
}

impl Settings {
    /// Load settings from the default config file location
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be read or parsed.
    /// If the file doesn't exist, returns `Ok(Settings::default())`.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(settings)
    }

    /// Load settings from a specific path
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(settings)
    }

    /// Save settings to the default config file location
    ///
    /// # Errors
    ///
    /// Returns an error if the config directory cannot be created or the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        let config_dir = config_path
            .parent()
            .expect("Config path must have a parent directory");

        std::fs::create_dir_all(config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        let content =
            toml::to_string_pretty(self).context("Failed to serialize settings to TOML")?;

        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&config_path)?.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&config_path, permissions)?;
        }

        Ok(())
    }

    /// Get the default config file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join("codexo").join("config.toml"))
            .unwrap_or_else(|| {
                PathBuf::from(".")
                    .join(".codexo")
                    .join("config.toml")
            })
    }

    /// Get the profile for a given directory path (for auto-switch feature)
    ///
    /// Returns `None` if no matching auto-switch rule is found.
    #[must_use]
    pub fn get_profile_for_directory(&self, path: &std::path::Path) -> Option<String> {
        let path_str = path.to_string_lossy();
        let home_dir = dirs::home_dir()?;
        let home_str = home_dir.to_string_lossy();

        // Normalize the path
        let normalized_path = if path_str.starts_with(&*home_str) {
            format!("~{}", &path_str[home_str.len()..])
        } else {
            path_str.to_string()
        };

        // Check for exact match first
        if let Some(profile) = self.auto_switch.get(&normalized_path) {
            return Some(profile.clone());
        }

        // Check for parent directory matches (most specific first)
        let mut longest_match: Option<(&String, &String)> = None;

        for (dir_pattern, profile) in &self.auto_switch {
            let expanded_pattern = if dir_pattern.starts_with("~/") {
                format!("{}{}", home_str, &dir_pattern[1..])
            } else {
                dir_pattern.clone()
            };

            if (normalized_path.starts_with(&expanded_pattern)
                || path_str.starts_with(&expanded_pattern))
                && (longest_match.is_none()
                    || expanded_pattern.len() > longest_match.unwrap().0.len())
            {
                longest_match = Some((dir_pattern, profile));
            }
        }

        longest_match.map(|(_, profile)| profile.clone())
    }

    /// Merge another settings object into this one (overriding values)
    pub fn merge(&mut self, other: Settings) {
        if other.default_profile.is_some() {
            self.default_profile = other.default_profile;
        }
        if other.editor.is_some() {
            self.editor = other.editor;
        }
        self.auto_backup = other.auto_backup;
        self.backup_retention_days = other.backup_retention_days;
        self.quiet_mode = other.quiet_mode;
        self.auto_switch.extend(other.auto_switch);
    }
}

/// Initialize a new config file with defaults and examples
///
/// # Errors
///
/// Returns an error if the config directory cannot be created or the file cannot be written.
pub fn init_config() -> Result<()> {
    let config_path = Settings::config_path();

    if config_path.exists() {
        anyhow::bail!("Config file already exists at {}", config_path.display());
    }

    let example_config = r#"# Codexo Configuration
# Location: ~/.config/codexo/config.toml

# Default profile to load when starting a new shell
# default_profile = "work"

# Automatically create backups before switching profiles
auto_backup = true

# Number of days to keep backups (0 = keep forever)
backup_retention_days = 30

# Default quiet mode for all commands
quiet_mode = false

# Default editor for interactive prompts (defaults to $EDITOR or vi)
# editor = "vim"

# Auto-switch profiles based on current directory
# When you cd into a directory, the matching profile is automatically loaded
[auto_switch]
# "~/work" = "work"
# "~/personal" = "personal"
# "~/projects/client-a" = "client-a"

# Notification settings for usage alerts
[notifications]
# Weekly usage threshold percentage (0-100)
weekly_threshold = 80
# Email address for notifications (optional)
# email = "user@example.com"
# Slack webhook URL for notifications (optional)
# slack_webhook = "https://hooks.slack.com/services/..."
"#;

    let config_dir = config_path
        .parent()
        .expect("Config path must have a parent directory");
    std::fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    std::fs::write(&config_path, example_config)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("✓ Created config file at {}", config_path.display());
    println!("  Edit it to customize your codexo settings");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.default_profile.is_none());
        assert!(settings.auto_backup);
        assert_eq!(settings.backup_retention_days, 30);
        assert!(!settings.quiet_mode);
        assert!(settings.editor.is_none());
        assert!(settings.auto_switch.is_empty());
        assert_eq!(settings.notifications.weekly_threshold, 80);
    }

    #[test]
    fn test_load_from_valid_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"
default_profile = "work"
auto_backup = false
[auto_switch]
"~/work" = "work"
"#
        )
        .unwrap();

        let settings = Settings::load_from(file.path()).unwrap();
        assert_eq!(settings.default_profile, Some("work".to_string()));
        assert!(!settings.auto_backup);
        assert_eq!(
            settings.auto_switch.get("~/work"),
            Some(&"work".to_string())
        );
    }

    #[test]
    fn test_get_profile_for_directory() {
        let settings = Settings {
            auto_switch: [
                ("~/work".to_string(), "work".to_string()),
                ("~/personal".to_string(), "personal".to_string()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let home = dirs::home_dir().unwrap();
        let work_dir = home.join("work").join("project-a");
        let personal_dir = home.join("personal");
        let other_dir = home.join("other");

        assert_eq!(
            settings.get_profile_for_directory(&work_dir),
            Some("work".to_string())
        );
        assert_eq!(
            settings.get_profile_for_directory(&personal_dir),
            Some("personal".to_string())
        );
        assert_eq!(settings.get_profile_for_directory(&other_dir), None);
    }

    #[test]
    fn test_config_path() {
        let path = Settings::config_path();
        assert!(path.to_string_lossy().contains("codexo"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }
}
