use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::process::Command;

use crate::paths;

/// Build and install agent-tools binary to bin/
///
/// This is the shared implementation for building and installing.
pub fn build_and_install() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;

    // Cargo build
    println!("{} Building agent-tools...", "→".blue());
    let build = Command::new("cargo")
        .args(["build", "--release", "-p", "agent-tools"])
        .current_dir(&agent_tools_home)
        .output()
        .context("Failed to run cargo build")?;

    if !build.status.success() {
        let stderr = String::from_utf8_lossy(&build.stderr);
        let stdout = String::from_utf8_lossy(&build.stdout);
        println!("{}", "Build failed!".red());
        println!("{}{}", stdout, stderr);
        bail!("Build failed. Please fix the errors and try again.");
    }
    println!("  {} Build successful", "✓".green());

    // Copy new binary to bin/
    let bin_dir = agent_tools_home.join("bin");
    let target_bin = agent_tools_home.join("target/release/agent-tools");
    let current_bin = bin_dir.join("agent-tools");

    if !target_bin.exists() {
        bail!(
            "Built binary not found at {}\n\
             This may happen if CARGO_TARGET_DIR is set to a custom location.",
            target_bin.display()
        );
    }

    fs::create_dir_all(&bin_dir)?;
    fs::copy(&target_bin, &current_bin).context("Failed to copy new binary to bin/")?;
    println!(
        "  {} Installed new binary to {}",
        "✓".green(),
        current_bin.display()
    );

    Ok(())
}

pub fn run() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;

    if !agent_tools_home.exists() {
        bail!(
            "agent-tools home not found: {}\nRun 'agent-tools init' first.",
            agent_tools_home.display()
        );
    }

    println!("{}", "Building agent-tools...".green().bold());
    println!();

    build_and_install()?;

    println!();
    println!("{}", "Build complete!".green().bold());

    Ok(())
}
