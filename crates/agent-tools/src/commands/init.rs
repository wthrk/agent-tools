use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::paths;

pub fn run() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;
    let skills_dir = paths::skills_dir()?;
    let bin_dir = agent_tools_home.join("bin");
    let backups_dir = paths::backups_dir()?;

    println!("{}", "Initializing agent-tools...".green().bold());
    println!();

    // Create directories
    let dirs = [
        (&agent_tools_home, "agent-tools home"),
        (&skills_dir, "skills directory"),
        (&bin_dir, "bin directory"),
        (&backups_dir, "backups directory"),
    ];

    for (dir, name) in &dirs {
        if dir.exists() {
            println!("  {} {} (already exists)", "✓".green(), name);
        } else {
            fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", name))?;
            println!("  {} {} created", "✓".green(), name);
        }
    }

    // Create default config if not exists
    let config_path = paths::config_path()?;
    if !config_path.exists() {
        let default_config = r#"# agent-tools configuration
config_version: 1

# Skills to auto-deploy to ~/.claude/skills/
auto_deploy_skills: []

# Manage ~/.claude/settings.json (link to ~/.agent-tools/settings.json)
manage_settings: false

# Manage ~/.claude/plugins/ (link to ~/.agent-tools/plugins/)
manage_plugins: false
"#;
        fs::write(&config_path, default_config).context("Failed to create config.yaml")?;
        println!("  {} config.yaml created", "✓".green());
    } else {
        println!("  {} config.yaml (already exists)", "✓".green());
    }

    println!();
    println!("{}", "Setup complete!".green().bold());
    println!();

    // PATH instructions
    let bin_path = bin_dir.display().to_string();
    println!(
        "{}",
        "To complete setup, add the following to your shell profile:".yellow()
    );
    println!();
    println!("  # For bash (~/.bashrc or ~/.bash_profile):");
    println!("  export PATH=\"{}:$PATH\"", bin_path);
    println!();
    println!("  # For zsh (~/.zshrc):");
    println!("  export PATH=\"{}:$PATH\"", bin_path);
    println!();
    println!("  # For fish (~/.config/fish/config.fish):");
    println!("  set -gx PATH {} $PATH", bin_path);
    println!();

    Ok(())
}
