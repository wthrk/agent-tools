use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;

use crate::fs_utils::copy_dir_recursive;
use crate::paths;
use crate::project::{find_project_root, project_skills_dir};
use crate::skill_meta::{SkillMeta, calculate_tree_hash};

pub fn run(name: &str, project: Option<&str>) -> Result<()> {
    // Find source skill
    let skills_dir = paths::skills_dir()?;
    let source_skill = skills_dir.join(name);

    if !source_skill.exists() {
        bail!(
            "Skill '{}' not found\nLooked in: {}",
            name,
            skills_dir.display()
        );
    }

    if !source_skill.join("SKILL.md").exists() {
        bail!(
            "Invalid skill '{}': missing SKILL.md\nPath: {}",
            name,
            source_skill.display()
        );
    }

    // Find project root
    let project_root = find_project_root(project)?;
    let project_skills = project_skills_dir(&project_root);

    // Ensure .claude/skills exists
    fs::create_dir_all(&project_skills).context("Failed to create .claude/skills directory")?;

    // Check if already installed
    let target_skill = project_skills.join(name);
    if target_skill.exists() {
        bail!(
            "Skill '{}' is already installed\nPath: {}\n\nUse 'skill-tools skill update {}' to update it",
            name,
            target_skill.display(),
            name
        );
    }

    // Copy skill
    println!("{} Installing skill '{}'...", "→".blue(), name.cyan());

    copy_dir_recursive(&source_skill, &target_skill).context("Failed to copy skill")?;

    // Calculate tree hash and create metadata
    let tree_hash = calculate_tree_hash(&source_skill)?;
    let meta = SkillMeta::new(&source_skill, &tree_hash);
    meta.save(&target_skill.join(".skill-meta.yaml"))?;

    println!(
        "{} Installed '{}' to {}",
        "✓".green(),
        name.cyan(),
        target_skill.display().to_string().dimmed()
    );

    Ok(())
}
