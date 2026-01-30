use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::process::Command;

use crate::paths;

pub fn run() -> Result<()> {
    let skill_tools_home = paths::skill_tools_home()?;

    if !skill_tools_home.exists() {
        bail!(
            "skill-tools home not found: {}\nRun 'skill-tools init' first.",
            skill_tools_home.display()
        );
    }

    println!("{}", "Updating skill-tools...".green().bold());
    println!();

    // Check for dirty state
    println!("{} Checking for uncommitted changes...", "→".blue());
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&skill_tools_home)
        .output()
        .context("Failed to run git status")?;

    if !status.status.success() {
        let stdout = String::from_utf8_lossy(&status.stdout);
        let stderr = String::from_utf8_lossy(&status.stderr);
        bail!(
            "Failed to check git status in {}:\n{}{}\nIs this a git repository?",
            skill_tools_home.display(),
            stdout,
            stderr
        );
    }

    let status_output = String::from_utf8_lossy(&status.stdout);
    if !status_output.trim().is_empty() {
        bail!(
            "Uncommitted changes detected in {}\n\
             Please commit or stash changes before updating.",
            skill_tools_home.display()
        );
    }
    println!("  {} No uncommitted changes", "✓".green());

    // Backup current binary
    let bin_dir = skill_tools_home.join("bin");
    let current_bin = bin_dir.join("skill-tools");

    if current_bin.exists() {
        let backups_dir = paths::backups_dir()?;
        fs::create_dir_all(&backups_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backups_dir.join(format!("skill-tools_{}", timestamp));
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
        .current_dir(&skill_tools_home)
        .output()
        .context("Failed to run git pull")?;

    if !pull.status.success() {
        let stderr = String::from_utf8_lossy(&pull.stderr);
        let stdout = String::from_utf8_lossy(&pull.stdout);
        bail!(
            "Git pull failed:\n{}{}\nPlease resolve manually in {}",
            stdout,
            stderr,
            skill_tools_home.display()
        );
    }
    println!("  {} Git pull successful", "✓".green());

    // Cargo build
    println!("{} Building skill-tools...", "→".blue());
    let build = Command::new("cargo")
        .args(["build", "--release", "-p", "skill-tools"])
        .current_dir(&skill_tools_home)
        .output()
        .context("Failed to run cargo build")?;

    if !build.status.success() {
        let stderr = String::from_utf8_lossy(&build.stderr);
        let stdout = String::from_utf8_lossy(&build.stdout);
        println!("{}", "Build failed!".red());
        println!("{}{}", stdout, stderr);

        // Try to restore backup
        let backups_dir = paths::backups_dir()?;
        if let Ok(entries) = fs::read_dir(&backups_dir) {
            if let Some(latest) = entries.filter_map(|e| e.ok()).max_by_key(|e| e.path()) {
                let backup_path = latest.path();
                if let Err(e) = fs::copy(&backup_path, &current_bin) {
                    bail!(
                        "Restore failed: {}\nBackup is at: {}",
                        e,
                        backup_path.display()
                    );
                }
                println!("{}", "Restored previous binary from backup.".yellow());
            }
        }

        bail!("Build failed. Please fix the errors and try again.");
    }
    println!("  {} Build successful", "✓".green());

    // Copy new binary to bin/
    let target_bin = skill_tools_home.join("target/release/skill-tools");
    if target_bin.exists() {
        fs::create_dir_all(&bin_dir)?;
        fs::copy(&target_bin, &current_bin).context("Failed to copy new binary to bin/")?;
        println!(
            "  {} Installed new binary to {}",
            "✓".green(),
            current_bin.display()
        );
    }

    println!();
    println!("{}", "Update complete!".green().bold());

    Ok(())
}
