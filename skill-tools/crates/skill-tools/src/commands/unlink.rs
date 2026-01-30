use anyhow::{Result, bail};
use colored::Colorize;
use std::fs;

use crate::paths;

pub fn run(name: &str) -> Result<()> {
    let skills_source = paths::skills_dir()?;
    let claude_skills = paths::claude_skills_dir()?;

    let target = claude_skills.join(name);

    // Check if target exists
    if !target.exists() && !target.is_symlink() {
        bail!(
            "Skill '{}' is not linked in ~/.claude/skills/\n\
             Run 'skill-tools status' to see linked skills.",
            name
        );
    }

    // Only unlink if it's a symlink pointing to our skills source
    if target.is_symlink() {
        if let Ok(link_target) = fs::read_link(&target) {
            if !link_target.starts_with(&skills_source) {
                bail!(
                    "Skill '{}' is not managed by skill-tools\n\
                     (symlink points to: {})",
                    name,
                    link_target.display()
                );
            }
        }
    } else {
        bail!(
            "'{}' is not a symlink, refusing to remove\n\
             This may be a manually installed skill. Remove it manually if intended.",
            target.display()
        );
    }

    // Remove the symlink
    fs::remove_file(&target)?;

    println!("{} Unlinked '{}'", "✓".green(), name.cyan());

    Ok(())
}
