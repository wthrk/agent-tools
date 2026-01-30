use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the agent-tools home directory (~/.agent-tools)
/// Can be overridden with AGENT_TOOLS_HOME environment variable
pub fn agent_tools_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("AGENT_TOOLS_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = directories::BaseDirs::new()
        .context("Failed to get home directory")?
        .home_dir()
        .to_path_buf();

    Ok(home.join(".agent-tools"))
}

/// Get the skills directory (~/.agent-tools/skills)
pub fn skills_dir() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join("skills"))
}

/// Get the config file path (~/.agent-tools/config.yaml)
pub fn config_path() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join("config.yaml"))
}

/// Get the backups directory (~/.agent-tools/backups)
pub fn backups_dir() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join("backups"))
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
