//! Configuration handling for deadbranch

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default number of days before a branch is considered stale
const DEFAULT_DAYS: u32 = 30;

/// Default protected branches
const DEFAULT_PROTECTED: &[&str] = &[
    "main",
    "master",
    "trunk",
    "development",
    "develop",
    "staging",
    "production",
];

/// Default exclude patterns (WIP/draft branches)
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &["wip/*", "draft/*", "*/wip", "*/draft"];

/// General settings section
#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    /// Default age threshold (days)
    #[serde(default = "default_days")]
    pub default_days: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_days: default_days(),
        }
    }
}

/// Branch-related settings section
#[derive(Debug, Deserialize, Serialize)]
pub struct BranchesConfig {
    /// The default branch to check merges against (auto-detected if not set)
    #[serde(default)]
    pub default_branch: Option<String>,

    /// Protected branches (never deleted)
    #[serde(default = "default_protected_branches")]
    pub protected: Vec<String>,

    /// Branch name patterns to exclude (glob-style: wip/*, */draft, etc.)
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

impl Default for BranchesConfig {
    fn default() -> Self {
        Self {
            default_branch: None,
            protected: default_protected_branches(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

/// Configuration for deadbranch
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub branches: BranchesConfig,
}

fn default_days() -> u32 {
    DEFAULT_DAYS
}

fn default_protected_branches() -> Vec<String> {
    DEFAULT_PROTECTED.iter().map(|s| s.to_string()).collect()
}

fn default_exclude_patterns() -> Vec<String> {
    DEFAULT_EXCLUDE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Config {
    /// Get the main deadbranch directory (~/.deadbranch)
    pub fn deadbranch_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".deadbranch"))
    }

    /// Get the path to the config file (~/.deadbranch/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::deadbranch_dir()?.join("config.toml"))
    }

    /// Get the backups directory (~/.deadbranch/backups)
    pub fn backups_dir() -> Result<PathBuf> {
        Ok(Self::deadbranch_dir()?.join("backups"))
    }

    /// Get the backup directory for a specific repository
    pub fn repo_backup_dir(repo_name: &str) -> Result<PathBuf> {
        Ok(Self::backups_dir()?.join(repo_name))
    }

    /// Get the current repository name (uses directory name)
    pub fn get_repo_name() -> String {
        std::env::current_dir()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown-repo".to_string())
    }

    /// Load config from file, or create default config if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        } else {
            // Auto-create config file with defaults on first use
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Set a configuration value by key (accepts multiple values for list types)
    /// Supports both flat keys (default-days) and dotted keys (general.default-days)
    pub fn set(&mut self, key: &str, values: &[String]) -> Result<()> {
        match key {
            // General section
            "general.default-days" | "default-days" | "days" => {
                if values.len() != 1 {
                    anyhow::bail!("default-days expects a single value");
                }
                self.general.default_days = values[0]
                    .parse()
                    .with_context(|| format!("Invalid number: {}", values[0]))?;
            }

            // Branches section
            "branches.protected" | "protected-branches" => {
                // Filter out empty strings to allow clearing with ""
                self.branches.protected =
                    values.iter().filter(|s| !s.is_empty()).cloned().collect();
            }
            "branches.default-branch" | "default-branch" => {
                if values.len() != 1 {
                    anyhow::bail!("default-branch expects a single value");
                }
                self.branches.default_branch = if values[0].is_empty() {
                    None
                } else {
                    Some(values[0].clone())
                };
            }
            "branches.exclude-patterns" | "exclude-patterns" => {
                // Filter out empty strings to allow clearing with ""
                self.branches.exclude_patterns =
                    values.iter().filter(|s| !s.is_empty()).cloned().collect();
            }

            _ => {
                anyhow::bail!(
                    "Unknown config key: {}. Valid keys: general.default-days, branches.protected, branches.default-branch, branches.exclude-patterns",
                    key
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a temporary config in a temp directory
    fn with_temp_config<F>(test: F)
    where
        F: FnOnce(PathBuf),
    {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        test(config_path);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.default_days, 30);
        assert_eq!(
            config.branches.protected,
            vec![
                "main",
                "master",
                "trunk",
                "development",
                "develop",
                "staging",
                "production"
            ]
        );
        assert_eq!(
            config.branches.exclude_patterns,
            vec!["wip/*", "draft/*", "*/wip", "*/draft"]
        );
        assert_eq!(config.branches.default_branch, None);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("default_days = 30"));
        assert!(toml_str.contains("[branches]"));
        assert!(toml_str.contains("protected"));
        assert!(toml_str.contains("exclude_patterns"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [general]
            default_days = 45

            [branches]
            default_branch = "master"
            protected = ["main", "develop"]
            exclude_patterns = ["temp/*"]
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.default_days, 45);
        assert_eq!(config.branches.protected, vec!["main", "develop"]);
        assert_eq!(config.branches.exclude_patterns, vec!["temp/*"]);
        assert_eq!(config.branches.default_branch, Some("master".to_string()));
    }

    #[test]
    fn test_config_set_default_days() {
        let mut config = Config::default();
        config.set("default-days", &["45".to_string()]).unwrap();
        assert_eq!(config.general.default_days, 45);

        // Alternative key name
        config.set("days", &["60".to_string()]).unwrap();
        assert_eq!(config.general.default_days, 60);

        // Dotted key name
        config
            .set("general.default-days", &["75".to_string()])
            .unwrap();
        assert_eq!(config.general.default_days, 75);
    }

    #[test]
    fn test_config_set_default_days_invalid() {
        let mut config = Config::default();
        let result = config.set("default-days", &["not_a_number".to_string()]);
        assert!(result.is_err());

        // Multiple values should fail
        let result = config.set("default-days", &["30".to_string(), "45".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_set_protected_branches() {
        let mut config = Config::default();
        config
            .set(
                "protected-branches",
                &["main".to_string(), "develop".to_string()],
            )
            .unwrap();
        assert_eq!(config.branches.protected, vec!["main", "develop"]);

        // Can set single value
        config
            .set("protected-branches", &["main".to_string()])
            .unwrap();
        assert_eq!(config.branches.protected, vec!["main"]);

        // Dotted key
        config
            .set("branches.protected", &["staging".to_string()])
            .unwrap();
        assert_eq!(config.branches.protected, vec!["staging"]);

        // Can clear with empty string
        config.set("protected-branches", &["".to_string()]).unwrap();
        assert!(config.branches.protected.is_empty());
    }

    #[test]
    fn test_config_set_default_branch() {
        let mut config = Config::default();
        config
            .set("default-branch", &["master".to_string()])
            .unwrap();
        assert_eq!(config.branches.default_branch, Some("master".to_string()));

        // Dotted key
        config
            .set("branches.default-branch", &["main".to_string()])
            .unwrap();
        assert_eq!(config.branches.default_branch, Some("main".to_string()));

        // Can clear with empty string
        config.set("default-branch", &["".to_string()]).unwrap();
        assert_eq!(config.branches.default_branch, None);
    }

    #[test]
    fn test_config_set_default_branch_invalid() {
        let mut config = Config::default();
        // Multiple values should fail
        let result = config.set(
            "default-branch",
            &["main".to_string(), "master".to_string()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_config_set_exclude_patterns() {
        let mut config = Config::default();
        config
            .set(
                "exclude-patterns",
                &["temp/*".to_string(), "*/old".to_string()],
            )
            .unwrap();
        assert_eq!(config.branches.exclude_patterns, vec!["temp/*", "*/old"]);

        // Dotted key
        config
            .set("branches.exclude-patterns", &["test/*".to_string()])
            .unwrap();
        assert_eq!(config.branches.exclude_patterns, vec!["test/*"]);

        // Can clear with empty string
        config.set("exclude-patterns", &["".to_string()]).unwrap();
        assert!(config.branches.exclude_patterns.is_empty());
    }

    #[test]
    fn test_config_set_unknown_key() {
        let mut config = Config::default();
        let result = config.set("unknown-key", &["value".to_string()]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown config key"));
    }

    #[test]
    fn test_config_save_and_load() {
        with_temp_config(|config_path| {
            let temp_dir = config_path.parent().unwrap();

            // Override the config path for this test
            let mut config = Config::default();
            config.general.default_days = 45;
            config.branches.protected = vec!["main".to_string()];

            // Save manually to temp path
            fs::create_dir_all(temp_dir).unwrap();
            let content = toml::to_string_pretty(&config).unwrap();
            fs::write(&config_path, content).unwrap();

            // Load and verify
            let loaded_content = fs::read_to_string(&config_path).unwrap();
            let loaded_config: Config = toml::from_str(&loaded_content).unwrap();
            assert_eq!(loaded_config.general.default_days, 45);
            assert_eq!(loaded_config.branches.protected, vec!["main"]);
        });
    }

    #[test]
    fn test_get_repo_name() {
        let repo_name = Config::get_repo_name();
        // Should be "deadbranch" when running in the deadbranch directory
        assert!(!repo_name.is_empty());
        assert_ne!(repo_name, "unknown-repo");
    }

    #[test]
    fn test_config_paths() {
        // These should not panic and should return valid paths
        let deadbranch_dir = Config::deadbranch_dir();
        assert!(deadbranch_dir.is_ok());

        let config_path = Config::config_path();
        assert!(config_path.is_ok());

        let backups_dir = Config::backups_dir();
        assert!(backups_dir.is_ok());

        let repo_backup = Config::repo_backup_dir("test-repo");
        assert!(repo_backup.is_ok());
        assert!(repo_backup.unwrap().to_string_lossy().contains("test-repo"));
    }
}
