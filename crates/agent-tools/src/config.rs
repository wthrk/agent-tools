use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_config_version")]
    pub config_version: u32,

    /// Skills to auto-deploy to ~/.claude/skills/
    #[serde(default)]
    pub auto_deploy_skills: Vec<String>,

    /// Manage ~/.claude/settings.json (link to ~/.skill-tools/settings.json)
    #[serde(default)]
    pub manage_settings: bool,

    /// Manage ~/.claude/plugins/ (link to ~/.skill-tools/plugins/)
    #[serde(default)]
    pub manage_plugins: bool,
}

fn default_config_version() -> u32 {
    1
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }
}
