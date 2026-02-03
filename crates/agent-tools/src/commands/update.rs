use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::process::Command;

use crate::commands::build;
use crate::commands::vcs::{Vcs, detect_vcs};
use crate::paths;

pub fn run() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;

    if !agent_tools_home.exists() {
        bail!(
            "agent-tools home not found: {}\nRun 'agent-tools init' first.",
            agent_tools_home.display()
        );
    }

    let vcs = detect_vcs(&agent_tools_home).ok_or_else(|| {
        anyhow::anyhow!(
            "No VCS detected in {}\nExpected .jj or .git directory.",
            agent_tools_home.display()
        )
    })?;

    println!("{}", "Updating agent-tools...".green().bold());
    println!();

    match vcs {
        Vcs::Jj => run_jj_update(&agent_tools_home)?,
        Vcs::Git => run_git_update(&agent_tools_home)?,
    }

    // Build and install using shared function
    let bin_dir = agent_tools_home.join("bin");
    let current_bin = bin_dir.join("agent-tools");

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

fn run_jj_update(agent_tools_home: &std::path::Path) -> Result<()> {
    // Check for uncommitted changes
    println!("{} Checking for uncommitted changes...", "→".blue());
    let status = Command::new("jj")
        .args(["status"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run jj status")?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        bail!(
            "Failed to check jj status in {}:\n{}\nIs this a jj repository?",
            agent_tools_home.display(),
            stderr
        );
    }

    // jj status shows "Working copy changes:" if there are changes
    let status_output = String::from_utf8_lossy(&status.stdout);
    if status_output.contains("Working copy changes:") {
        bail!(
            "Uncommitted changes detected in {}\n\
             Please commit or abandon changes before updating.",
            agent_tools_home.display()
        );
    }
    println!("  {} No uncommitted changes", "✓".green());

    // Backup current binary
    backup_binary(agent_tools_home)?;

    // jj git fetch
    println!("{} Running jj git fetch...", "→".blue());
    let fetch = Command::new("jj")
        .args(["git", "fetch"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run jj git fetch")?;

    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        bail!("jj git fetch failed:\n{}", stderr);
    }
    println!("  {} jj git fetch successful", "✓".green());

    // Update main bookmark to point to origin/main
    println!(
        "{} Running jj bookmark set main -r main@origin...",
        "→".blue()
    );
    let bookmark_set = Command::new("jj")
        .args(["bookmark", "set", "main", "-r", "main@origin"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run jj bookmark set")?;

    if !bookmark_set.status.success() {
        let stderr = String::from_utf8_lossy(&bookmark_set.stderr);
        bail!("jj bookmark set main -r main@origin failed:\n{}", stderr);
    }
    println!("  {} jj bookmark set main successful", "✓".green());

    // jj new main (create new change from main, move working copy to latest main)
    println!("{} Running jj new main...", "→".blue());
    let new_main = Command::new("jj")
        .args(["new", "main"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run jj new main")?;

    if !new_main.status.success() {
        let stderr = String::from_utf8_lossy(&new_main.stderr);
        bail!(
            "jj new main failed:\n{}\nPlease resolve manually in {}",
            stderr,
            agent_tools_home.display()
        );
    }
    println!("  {} jj new main successful", "✓".green());

    Ok(())
}

fn run_git_update(agent_tools_home: &std::path::Path) -> Result<()> {
    // Check for uncommitted changes
    println!("{} Checking for uncommitted changes...", "→".blue());
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(agent_tools_home)
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
    backup_binary(agent_tools_home)?;

    // Git pull
    println!("{} Running git pull...", "→".blue());
    let pull = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(agent_tools_home)
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

    Ok(())
}

fn backup_binary(agent_tools_home: &std::path::Path) -> Result<()> {
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

    Ok(())
}
