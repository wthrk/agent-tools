use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::paths;

pub fn run() -> Result<()> {
    let backups_dir = paths::backups_dir()?;

    if !backups_dir.exists() {
        println!("{}", "No backups directory found.".yellow());
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&backups_dir)
        .context("Failed to read backups directory")?
        .filter_map(|e| e.ok())
        .collect();

    if entries.is_empty() {
        println!("{}", "No backups to clean up.".green());
        return Ok(());
    }

    println!("{}", "Cleaning up old backups...".green().bold());
    println!();

    let mut removed = 0;
    let mut failed = 0;

    for entry in &entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        match result {
            Ok(()) => {
                println!("  {} Removed {}", "✓".green(), name.dimmed());
                removed += 1;
            }
            Err(e) => {
                println!("  {} Failed to remove {}: {}", "✗".red(), name, e);
                failed += 1;
            }
        }
    }

    println!();
    println!(
        "{}",
        format!("Cleanup complete: {} removed, {} failed", removed, failed).dimmed()
    );

    Ok(())
}
