use std::fs;
use std::process::Command;

use crate::commands::sync;
use crate::commands::vcs::{self, Vcs};
use crate::config::Config;
use crate::paths;

pub fn run() -> anyhow::Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;
    let config_path = paths::config_path()?;
    let config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("startup: failed to load config: {e}");
            return Ok(());
        }
    };

    // Phase 1: Auto-update check
    if config.auto_update_on_startup {
        if let Err(e) = check_and_update(&agent_tools_home) {
            eprintln!("startup: auto-update check failed: {e}");
        }
    }

    // Phase 2: Sync (always run)
    if let Err(e) = sync::run(false, false) {
        eprintln!("startup: sync failed: {e}");
    }

    Ok(())
}

fn check_and_update(agent_tools_home: &std::path::Path) -> anyhow::Result<()> {
    let vcs = match vcs::detect_vcs(agent_tools_home) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Fetch remote
    vcs::fetch_remote(agent_tools_home, vcs)?;

    // Check for updates
    if !vcs::has_remote_updates(agent_tools_home, vcs)? {
        return Ok(());
    }

    // Check if tree is clean
    let clean = match vcs {
        Vcs::Jj => vcs::check_jj_clean(agent_tools_home).is_ok(),
        Vcs::Git => vcs::check_git_clean(agent_tools_home).is_ok(),
    };

    if !clean {
        eprintln!("startup: updates available but working tree is dirty, skipping auto-update");
        return Ok(());
    }

    // Launch background update
    let logs_dir = agent_tools_home.join("logs");
    fs::create_dir_all(&logs_dir)?;
    let log_path = logs_dir.join("startup-update.log");

    Command::new("nohup")
        .args([
            "agent-tools",
            "update",
        ])
        .stdout(fs::File::create(&log_path)?)
        .stderr(fs::File::create(logs_dir.join("startup-update-err.log"))?)
        .spawn()?;

    Ok(())
}
