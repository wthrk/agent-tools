use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::paths;

pub fn run() -> Result<()> {
    let skills_dir = paths::skills_dir()?;

    if !skills_dir.exists() {
        println!("{}", "No skills directory found.".yellow());
        println!("Expected at: {}", skills_dir.display().to_string().dimmed());
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&skills_dir)
        .context("Failed to read skills directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join("SKILL.md").exists())
        .collect();

    if entries.is_empty() {
        println!("{}", "No skills available.".yellow());
        println!(
            "Skills directory: {}",
            skills_dir.display().to_string().dimmed()
        );
        return Ok(());
    }

    println!("{}", "Available skills:".green().bold());
    println!();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Try to read skill description from SKILL.md
        let skill_md = entry.path().join("SKILL.md");
        let description = if let Ok(content) = fs::read_to_string(&skill_md) {
            // Extract first heading or first non-empty line
            content
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim_start_matches('#').trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        if description.is_empty() {
            println!("  {}", name_str.cyan());
        } else {
            println!("  {} - {}", name_str.cyan(), description.dimmed());
        }
    }

    println!();
    println!("{}", format!("Total: {} skill(s)", entries.len()).dimmed());

    Ok(())
}
