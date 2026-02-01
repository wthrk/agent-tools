use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};

use crate::commands::link;
use crate::config::{add_auto_deploy_skill, validate_skill_name};
use crate::paths;

/// SKILL.md template
fn skill_template(name: &str) -> String {
    format!(
        r#"---
name: {name}
description: TODO: Add description
allowed-tools: []
user-invocable: true
argument-hint:
---

# {name}

TODO: Add skill instructions
"#
    )
}

/// Ask user a yes/no question with default.
///
/// Returns true if user answers yes (or presses Enter for default yes).
fn ask_yes_no(prompt: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", prompt, suffix);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Ok(default_yes);
    }

    Ok(input == "y" || input == "yes")
}

pub fn run(name: &str, add_to_config: Option<bool>) -> Result<()> {
    // Validate skill name
    validate_skill_name(name)?;

    let skills_dir = paths::skills_dir()?;
    let skill_dir = skills_dir.join(name);

    // Check if skill already exists
    if skill_dir.exists() {
        bail!("Skill '{}' already exists at {}", name, skill_dir.display());
    }

    // Create skills directory if it doesn't exist
    if !skills_dir.exists() {
        fs::create_dir_all(&skills_dir).with_context(|| {
            format!(
                "Failed to create skills directory: {}",
                skills_dir.display()
            )
        })?;
    }

    // Create skill directory
    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create skill directory: {}", skill_dir.display()))?;

    // Create SKILL.md
    let skill_md_path = skill_dir.join("SKILL.md");
    fs::write(&skill_md_path, skill_template(name))
        .with_context(|| format!("Failed to create SKILL.md: {}", skill_md_path.display()))?;

    println!(
        "{} Created skill '{}' at {}",
        "✓".green(),
        name.cyan(),
        skill_dir.display()
    );

    // Ask about adding to auto_deploy_skills
    let should_add = match add_to_config {
        Some(value) => value,
        None => ask_yes_no("Add to auto_deploy_skills?", true)?,
    };

    if should_add {
        // Create symlink first (so if it fails, config is not modified)
        link::run(name)?;

        // Add to config.yaml
        let config_path = paths::config_path()?;
        add_auto_deploy_skill(&config_path, name)?;
        println!(
            "  {} Added '{}' to auto_deploy_skills in config.yaml",
            "✓".green(),
            name.cyan()
        );
    }

    println!();
    println!(
        "{} Edit {} to define your skill",
        "→".blue(),
        skill_md_path.display()
    );

    Ok(())
}
