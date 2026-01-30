use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the skill-tools home directory (~/.skill-tools)
/// Can be overridden with SKILL_TOOLS_HOME environment variable
pub fn skill_tools_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("SKILL_TOOLS_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = directories::BaseDirs::new()
        .context("Failed to get home directory")?
        .home_dir()
        .to_path_buf();

    Ok(home.join(".skill-tools"))
}

/// Get the skills directory (~/.skill-tools/skills)
pub fn skills_dir() -> Result<PathBuf> {
    Ok(skill_tools_home()?.join("skills"))
}

/// Get the config file path (~/.skill-tools/config.yaml)
pub fn config_path() -> Result<PathBuf> {
    Ok(skill_tools_home()?.join("config.yaml"))
}

/// Get the backups directory (~/.skill-tools/backups)
pub fn backups_dir() -> Result<PathBuf> {
    Ok(skill_tools_home()?.join("backups"))
}

/// Get the Claude directory (~/.claude)
/// Can be overridden with CLAUDE_HOME environment variable
pub fn claude_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CLAUDE_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = directories::BaseDirs::new()
        .context("Failed to get home directory")?
        .home_dir()
        .to_path_buf();

    Ok(home.join(".claude"))
}

/// Get the Claude skills directory (~/.claude/skills)
pub fn claude_skills_dir() -> Result<PathBuf> {
    Ok(claude_home()?.join("skills"))
}
