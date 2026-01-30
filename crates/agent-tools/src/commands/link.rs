use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::os::unix::fs::symlink;

use crate::paths;

pub fn run(name: &str) -> Result<()> {
    let skills_source = paths::skills_dir()?;
    let claude_skills = paths::claude_skills_dir()?;

    let source = skills_source.join(name);
    let target = claude_skills.join(name);

    // Check source exists
    if !source.exists() {
        bail!(
            "Skill '{}' not found in {}\n\
             Run 'skill-tools skill list' to see available skills.",
            name,
            skills_source.display()
        );
    }

    // Ensure claude skills directory exists
    if !claude_skills.exists() {
        fs::create_dir_all(&claude_skills).context("Failed to create ~/.claude/skills")?;
    }

    // Check if already linked
    if target.exists() || target.is_symlink() {
        if target.is_symlink() {
            if let Ok(link_target) = fs::read_link(&target) {
                if link_target == source {
                    println!("{} Skill '{}' is already linked", "✓".green(), name.cyan());
                    return Ok(());
                }
            }
        }

        bail!(
            "A file or directory already exists at {}\n\
             Remove it first or use 'skill-tools sync' to manage all links.",
            target.display()
        );
    }

    // Create symlink
    symlink(&source, &target).with_context(|| format!("Failed to create symlink for '{name}'"))?;

    println!(
        "{} Linked '{}' → {}",
        "✓".green(),
        name.cyan(),
        source.display()
    );

    Ok(())
}
