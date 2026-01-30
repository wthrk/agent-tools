use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::project::{find_project_root, project_skills_dir};
use crate::skill_meta::SkillMeta;

pub fn run(project: Option<&str>) -> Result<()> {
    let project_root = find_project_root(project)?;
    let skills_dir = project_skills_dir(&project_root);

    if !skills_dir.exists() {
        println!("{}", "No skills installed in this project.".yellow());
        println!("Project: {}", project_root.display().to_string().dimmed());
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&skills_dir)
        .context("Failed to read project skills directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join("SKILL.md").exists())
        .collect();

    if entries.is_empty() {
        println!("{}", "No skills installed in this project.".yellow());
        println!("Project: {}", project_root.display().to_string().dimmed());
        return Ok(());
    }

    println!(
        "{} ({})",
        "Installed skills:".green().bold(),
        project_root.display().to_string().dimmed()
    );
    println!();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let skill_path = entry.path();

        // Try to read metadata
        let meta_path = skill_path.join(".skill-meta.yaml");
        let meta_info = if let Ok(meta) = SkillMeta::load(&meta_path) {
            format!(
                "installed: {}, hash: {}",
                meta.installed_at.format("%Y-%m-%d"),
                &meta.tree_hash[..8.min(meta.tree_hash.len())]
            )
        } else {
            "no metadata".to_string()
        };

        println!("  {} ({})", name_str.cyan(), meta_info.dimmed());
    }

    println!();
    println!("{}", format!("Total: {} skill(s)", entries.len()).dimmed());

    Ok(())
}
