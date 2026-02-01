use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::process::Command;

use crate::commands::build;
use crate::paths;

pub fn run() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;

    if !agent_tools_home.exists() {
        bail!(
            "agent-tools home not found: {}\nRun 'agent-tools init' first.",
            agent_tools_home.display()
        );
    }

    println!("{}", "Updating agent-tools...".green().bold());
    println!();

    // Check for dirty state
    println!("{} Checking for uncommitted changes...", "→".blue());
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&agent_tools_home)
        .output()
        .context("Failed to run git status")?;

    if !status.status.success() {
        let stdout = String::from_utf8_lossy(&status.stdout);
        let stderr = String::from_utf8_lossy(&status.stderr);
        bail!(
            "Failed to check git status in {}:\n{}{}\nIs this a git repository?",
            agent_tools_home.display(),
            stdout,
            stderr
        );
    }

    let status_output = String::from_utf8_lossy(&status.stdout);
    if !status_output.trim().is_empty() {
        bail!(
            "Uncommitted changes detected in {}\n\
             Please commit or stash changes before updating.",
            agent_tools_home.display()
        );
    }
    println!("  {} No uncommitted changes", "✓".green());

    // Backup current binary
    let bin_dir = agent_tools_home.join("bin");
    let current_bin = bin_dir.join("agent-tools");

    if current_bin.exists() {
        let backups_dir = paths::backups_dir()?;
        fs::create_dir_all(&backups_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backups_dir.join(format!("agent-tools_{}", timestamp));
        fs::copy(&current_bin, &backup_path).context("Failed to backup current binary")?;
        println!(
            "  {} Backed up current binary to {}",
            "✓".green(),
            backup_path.display()
        );
    }

    // Git pull
    println!("{} Running git pull...", "→".blue());
    let pull = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(&agent_tools_home)
        .output()
        .context("Failed to run git pull")?;

    if !pull.status.success() {
        let stderr = String::from_utf8_lossy(&pull.stderr);
        let stdout = String::from_utf8_lossy(&pull.stdout);
        bail!(
            "Git pull failed:\n{}{}\nPlease resolve manually in {}",
            stdout,
            stderr,
            agent_tools_home.display()
        );
    }
    println!("  {} Git pull successful", "✓".green());

    // Build and install using shared function
    if let Err(e) = build::build_and_install() {
        // Try to restore backup on build failure
        let backups_dir = paths::backups_dir()?;
        if let Ok(entries) = fs::read_dir(&backups_dir) {
            if let Some(latest) = entries.filter_map(|e| e.ok()).max_by_key(|e| e.path()) {
                let backup_path = latest.path();
                if let Err(restore_err) = fs::copy(&backup_path, &current_bin) {
                    bail!(
                        "Build failed: {}\nRestore also failed: {}\nBackup is at: {}",
                        e,
                        restore_err,
                        backup_path.display()
                    );
                }
                println!("{}", "Restored previous binary from backup.".yellow());
            }
        }
        return Err(e);
    }

    println!();
    println!("{}", "Update complete!".green().bold());

    Ok(())
}
