use anyhow::Context;
use std::fs;
use std::process::Command;
use std::time::SystemTime;

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
            eprintln!("startup: failed to load config: {e}, using defaults");
            Config::default()
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
    match vcs {
        Vcs::Jj => {
            if let Err(e) = vcs::check_jj_clean(agent_tools_home) {
                eprintln!("startup: {e}, skipping auto-update");
                return Ok(());
            }
        }
        Vcs::Git => {
            if let Err(e) = vcs::check_git_clean(agent_tools_home) {
                eprintln!("startup: {e}, skipping auto-update");
                return Ok(());
            }
        }
    }

    // Acquire lock to prevent concurrent background updates
    let lock_path = agent_tools_home.join("logs").join("startup-update.lock");
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Remove stale lock (older than 10 minutes)
    if lock_path.exists() {
        if let Ok(metadata) = fs::metadata(&lock_path) {
            if let Ok(modified) = metadata.modified() {
                if SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default()
                    .as_secs()
                    > 600
                {
                    let _ = fs::remove_file(&lock_path);
                }
            }
        }
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Ok(());
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    // Launch background update
    let logs_dir = agent_tools_home.join("logs");
    let log_path = logs_dir.join("startup-update.log");

    let exe = std::env::current_exe().context("Failed to get current executable path")?;
    let spawn_result = Command::new(&exe)
        .arg("update")
        .stdout(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?,
        )
        .stderr(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logs_dir.join("startup-update-err.log"))?,
        )
        .spawn();

    if let Err(e) = spawn_result {
        let _ = fs::remove_file(&lock_path);
        return Err(e.into());
    }

    Ok(())
}
