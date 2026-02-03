use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::process::Command;

use crate::commands::vcs::{Vcs, check_git_clean, check_jj_clean, detect_vcs};
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

    println!("{}", "Rebasing agent-tools...".green().bold());
    println!();

    match vcs {
        Vcs::Jj => run_jj_rebase(&agent_tools_home)?,
        Vcs::Git => run_git_rebase(&agent_tools_home)?,
    }

    println!();
    println!("{}", "Rebase complete!".green().bold());

    Ok(())
}

fn run_jj_rebase(agent_tools_home: &std::path::Path) -> Result<()> {
    // Check for uncommitted changes first
    println!("{} Checking for uncommitted changes...", "→".blue());
    check_jj_clean(agent_tools_home)?;
    println!("  {} No uncommitted changes", "✓".green());

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

    // jj rebase -s @ -d main@origin (use remote ref directly to avoid stale local bookmark)
    println!("{} Running jj rebase -s @ -d main@origin...", "→".blue());
    let rebase = Command::new("jj")
        .args(["rebase", "-s", "@", "-d", "main@origin"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run jj rebase")?;

    if !rebase.status.success() {
        let stderr = String::from_utf8_lossy(&rebase.stderr);
        bail!(
            "jj rebase failed:\n{}\nPlease resolve manually in {}",
            stderr,
            agent_tools_home.display()
        );
    }
    let stdout = String::from_utf8_lossy(&rebase.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout);
    }
    println!("  {} jj rebase successful", "✓".green());

    Ok(())
}

fn run_git_rebase(agent_tools_home: &std::path::Path) -> Result<()> {
    // Check for uncommitted changes
    println!("{} Checking for uncommitted changes...", "→".blue());
    check_git_clean(agent_tools_home)?;
    println!("  {} No uncommitted changes", "✓".green());

    // git fetch origin
    println!("{} Running git fetch origin...", "→".blue());
    let fetch = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run git fetch")?;

    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        bail!("git fetch failed:\n{}", stderr);
    }
    println!("  {} git fetch successful", "✓".green());

    // git rebase origin/main
    println!("{} Running git rebase origin/main...", "→".blue());
    let rebase = Command::new("git")
        .args(["rebase", "origin/main"])
        .current_dir(agent_tools_home)
        .output()
        .context("Failed to run git rebase")?;

    if !rebase.status.success() {
        let stderr = String::from_utf8_lossy(&rebase.stderr);
        let stdout = String::from_utf8_lossy(&rebase.stdout);
        bail!(
            "git rebase failed:\n{}{}\nPlease resolve manually in {}",
            stdout,
            stderr,
            agent_tools_home.display()
        );
    }
    println!("  {} git rebase successful", "✓".green());

    Ok(())
}
