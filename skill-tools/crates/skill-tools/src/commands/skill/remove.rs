use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;

use crate::project::{find_project_root, project_skills_dir};

pub fn run(name: &str, project: Option<&str>) -> Result<()> {
    let project_root = find_project_root(project)?;
    let project_skills = project_skills_dir(&project_root);
    let skill_path = project_skills.join(name);

    if !skill_path.exists() {
        bail!(
            "Skill '{}' is not installed in this project\nProject: {}",
            name,
            project_root.display()
        );
    }

    println!("{} Removing skill '{}'...", "→".blue(), name.cyan());

    fs::remove_dir_all(&skill_path).context("Failed to remove skill directory")?;

    println!(
        "{} Removed '{}' from {}",
        "✓".green(),
        name.cyan(),
        project_root.display().to_string().dimmed()
    );

    Ok(())
}
