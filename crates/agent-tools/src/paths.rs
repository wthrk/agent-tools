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

/// Get the logs directory (~/.agent-tools/logs)
pub fn logs_dir() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join("logs"))
}

/// Get the templates root (~/.agent-tools/templates)
pub fn templates_dir() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join("templates"))
}

/// Get the Claude profile templates dir (~/.agent-tools/templates/claude)
pub fn claude_templates_dir() -> Result<PathBuf> {
    Ok(templates_dir()?.join("claude"))
}

/// Get the Codex profile templates dir (~/.agent-tools/templates/codex)
pub fn codex_templates_dir() -> Result<PathBuf> {
    Ok(templates_dir()?.join("codex"))
}

/// Get local state root (~/.agent-tools/.local)
pub fn local_state_root() -> Result<PathBuf> {
    Ok(agent_tools_home()?.join(".local"))
}

/// Get active template links dir (~/.agent-tools/.local/active)
pub fn active_templates_dir() -> Result<PathBuf> {
    Ok(local_state_root()?.join("active"))
}

/// Get profile state dir (~/.agent-tools/.local/state)
pub fn profile_state_dir() -> Result<PathBuf> {
    Ok(local_state_root()?.join("state"))
}

/// Get profile snapshots dir (~/.agent-tools/.local/snapshots)
pub fn profile_snapshots_dir() -> Result<PathBuf> {
    Ok(local_state_root()?.join("snapshots"))
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

/// Get the Codex directory (~/.codex)
/// Can be overridden with CODEX_HOME environment variable
pub fn codex_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = directories::BaseDirs::new()
        .context("Failed to get home directory")?
        .home_dir()
        .to_path_buf();

    Ok(home.join(".codex"))
}
