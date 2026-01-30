//! Configuration loader for skill-test.

use crate::types::{HookType, SkillTestConfig};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during config loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("hook-path is required when hook is 'custom'")]
    CustomHookWithoutPath,
    #[error("hook-path should not be set when hook is not 'custom'")]
    HookPathWithoutCustom,
}

/// Load skill test configuration from a YAML file.
///
/// If the file doesn't exist, returns default configuration.
///
/// # Errors
/// Returns an error if:
/// - The file exists but cannot be read
/// - The YAML is invalid
/// - hook is 'custom' but hook-path is not set
pub fn load_config(skill_dir: &Path) -> Result<SkillTestConfig, ConfigError> {
    let config_path = skill_dir.join("skill-test.config.yaml");

    let config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_yml::from_str(&content)?
    } else {
        SkillTestConfig::default()
    };

    validate_config(&config)?;
    Ok(config)
}

/// Validate the configuration.
const fn validate_config(config: &SkillTestConfig) -> Result<(), ConfigError> {
    match config.hook {
        HookType::Custom => {
            if config.hook_path.is_none() {
                return Err(ConfigError::CustomHookWithoutPath);
            }
        }
        _ => {
            if config.hook_path.is_some() {
                return Err(ConfigError::HookPathWithoutCustom);
            }
        }
    }
    Ok(())
}

/// CLI override options for configuration.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub model: Option<String>,
    pub timeout: Option<u64>,
    pub iterations: Option<u32>,
    pub threshold: Option<u32>,
    pub hook: Option<HookType>,
    pub hook_path: Option<std::path::PathBuf>,
    pub strict: Option<bool>,
}

/// Apply CLI overrides to a configuration.
#[must_use]
pub fn apply_overrides(
    mut config: SkillTestConfig,
    overrides: &ConfigOverrides,
) -> SkillTestConfig {
    if let Some(ref model) = overrides.model {
        config.model.clone_from(model);
    }
    if let Some(timeout) = overrides.timeout {
        config.timeout = timeout;
    }
    if let Some(iterations) = overrides.iterations {
        config.iterations = iterations;
    }
    if let Some(threshold) = overrides.threshold {
        config.threshold = threshold;
    }
    if let Some(ref hook) = overrides.hook {
        config.hook = hook.clone();
    }
    if let Some(ref hook_path) = overrides.hook_path {
        config.hook_path = Some(hook_path.clone());
    }
    if let Some(strict) = overrides.strict {
        config.strict = strict;
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_load_config_default_when_missing() -> TestResult {
        let dir = TempDir::new()?;
        let config = load_config(dir.path())?;
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.iterations, 10);
        Ok(())
    }

    #[test]
    fn test_load_config_from_file() -> TestResult {
        let dir = TempDir::new()?;
        let config_content = r"
model: claude-opus-4-20250514
iterations: 5
threshold: 90
";
        std::fs::write(dir.path().join("skill-test.config.yaml"), config_content)?;

        let config = load_config(dir.path())?;
        assert_eq!(config.model, "claude-opus-4-20250514");
        assert_eq!(config.iterations, 5);
        assert_eq!(config.threshold, 90);
        Ok(())
    }

    #[test]
    fn test_load_config_custom_hook_without_path() -> TestResult {
        let dir = TempDir::new()?;
        let config_content = r"
hook: custom
";
        std::fs::write(dir.path().join("skill-test.config.yaml"), config_content)?;

        let result = load_config(dir.path());
        assert!(matches!(result, Err(ConfigError::CustomHookWithoutPath)));
        Ok(())
    }

    #[test]
    fn test_load_config_hook_path_without_custom() -> TestResult {
        let dir = TempDir::new()?;
        let config_content = r"
hook: simple
hook-path: ./hook.sh
";
        std::fs::write(dir.path().join("skill-test.config.yaml"), config_content)?;

        let result = load_config(dir.path());
        assert!(matches!(result, Err(ConfigError::HookPathWithoutCustom)));
        Ok(())
    }

    #[test]
    fn test_load_config_custom_hook_with_path() -> TestResult {
        let dir = TempDir::new()?;
        let config_content = r"
hook: custom
hook-path: ./my-hook.sh
";
        std::fs::write(dir.path().join("skill-test.config.yaml"), config_content)?;

        let config = load_config(dir.path())?;
        assert_eq!(config.hook, HookType::Custom);
        assert_eq!(
            config.hook_path,
            Some(std::path::PathBuf::from("./my-hook.sh"))
        );
        Ok(())
    }

    #[test]
    fn test_apply_overrides() {
        let config = SkillTestConfig::default();
        let overrides = ConfigOverrides {
            model: Some("claude-opus-4-20250514".to_string()),
            iterations: Some(3),
            threshold: Some(95),
            ..Default::default()
        };

        let result = apply_overrides(config, &overrides);
        assert_eq!(result.model, "claude-opus-4-20250514");
        assert_eq!(result.iterations, 3);
        assert_eq!(result.threshold, 95);
        // Non-overridden values should remain default
        assert_eq!(result.timeout, 60_000);
    }

    #[test]
    fn test_apply_overrides_empty() {
        let config = SkillTestConfig::default();
        let overrides = ConfigOverrides::default();

        let result = apply_overrides(config.clone(), &overrides);
        assert_eq!(result.model, config.model);
        assert_eq!(result.iterations, config.iterations);
    }
}
