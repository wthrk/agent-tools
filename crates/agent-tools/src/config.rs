use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_config_version")]
    pub config_version: u32,

    /// Skills to auto-deploy to ~/.claude/skills/
    #[serde(default)]
    pub auto_deploy_skills: Vec<String>,

    /// Manage ~/.claude/settings.json (link to ~/.agent-tools/settings.json)
    #[serde(default)]
    pub manage_settings: bool,

    /// Manage ~/.claude/plugins/ (link to ~/.agent-tools/plugins/)
    #[serde(default)]
    pub manage_plugins: bool,

    /// Manage ~/.claude/CLAUDE.md (link to ~/.agent-tools/global/CLAUDE.md)
    #[serde(default)]
    pub manage_claude_md: bool,

    /// Manage ~/.claude/hooks/ (link to ~/.agent-tools/global/hooks/)
    #[serde(default)]
    pub manage_hooks: bool,
}

fn default_config_version() -> u32 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: default_config_version(),
            auto_deploy_skills: Vec::new(),
            manage_settings: false,
            manage_plugins: false,
            manage_claude_md: false,
            manage_hooks: false,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content =
            serde_yaml::to_string(self).with_context(|| "Failed to serialize config to YAML")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }
}

/// Maximum length for skill names to prevent filesystem issues.
const MAX_SKILL_NAME_LENGTH: usize = 64;

/// Validate a skill name.
///
/// Valid names match pattern `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`:
/// - Only lowercase letters, numbers, and hyphens
/// - Must start and end with alphanumeric
/// - No consecutive hyphens
/// - At most 64 characters long
pub fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Skill name cannot be empty");
    }

    if name.len() > MAX_SKILL_NAME_LENGTH {
        bail!(
            "Skill name cannot exceed {} characters: {}",
            MAX_SKILL_NAME_LENGTH,
            name
        );
    }

    // Must start with lowercase alphanumeric
    if !name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        bail!(
            "Invalid skill name '{}': must start with lowercase letter or number",
            name
        );
    }

    // Must end with lowercase alphanumeric (if more than one char)
    if name.len() > 1 && !name.ends_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        bail!(
            "Skill name must end with lowercase letter or number: {}",
            name
        );
    }

    // Only lowercase letters, numbers, and hyphens allowed
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!(
            "Skill name can only contain lowercase letters, numbers, and hyphens: {}",
            name
        );
    }

    // No consecutive hyphens
    if name.contains("--") {
        bail!("Skill name cannot contain consecutive hyphens: {}", name);
    }

    Ok(())
}

/// Add a skill to the auto_deploy_skills list in config.
///
/// If the config file does not exist, creates it with default values.
/// If the skill is already in the list, does nothing.
pub fn add_auto_deploy_skill(config_path: &Path, skill_name: &str) -> Result<()> {
    let mut config = Config::load(config_path)?;

    if !config.auto_deploy_skills.iter().any(|s| s == skill_name) {
        config.auto_deploy_skills.push(skill_name.to_string());
        config.save(config_path)?;
    }

    Ok(())
}
