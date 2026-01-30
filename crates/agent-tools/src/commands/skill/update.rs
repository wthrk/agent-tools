use anyhow::{Context, Result, bail};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::fs_utils::{calculate_tree_hash_excluding, copy_dir_contents};
use crate::paths;
use crate::project::{find_project_root, project_skills_dir};
use crate::skill_meta::{SkillMeta, calculate_tree_hash};

pub fn run(name: Option<&str>, all: bool, force: bool, project: Option<&str>) -> Result<()> {
    if name.is_none() && !all {
        bail!("Please specify a skill name or use --all to update all skills");
    }

    let project_root = find_project_root(project)?;
    let project_skills = project_skills_dir(&project_root);
    let skills_source = paths::skills_dir()?;

    if !project_skills.exists() {
        bail!(
            "No skills installed in this project\nProject: {}",
            project_root.display()
        );
    }

    let skills_to_update: Vec<String> = if all {
        // Get all installed skills
        fs::read_dir(&project_skills)
            .context("Failed to read project skills directory")?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| e.path().join("SKILL.md").exists())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect()
    } else {
        vec![name.expect("name should be set").to_string()]
    };

    if skills_to_update.is_empty() {
        println!("{}", "No skills to update.".yellow());
        return Ok(());
    }

    let mut updated = 0;
    let mut up_to_date = 0;
    let mut conflicts = 0;

    for skill_name in &skills_to_update {
        let source_skill = skills_source.join(skill_name);
        let installed_skill = project_skills.join(skill_name);

        if !source_skill.exists() {
            println!(
                "{} Skill '{}': source not found, skipping",
                "!".yellow(),
                skill_name.cyan()
            );
            continue;
        }

        if !installed_skill.exists() {
            println!(
                "{} Skill '{}': not installed, skipping",
                "!".yellow(),
                skill_name.cyan()
            );
            continue;
        }

        match update_single_skill(&source_skill, &installed_skill, skill_name, force)? {
            UpdateResult::Updated => updated += 1,
            UpdateResult::UpToDate => up_to_date += 1,
            UpdateResult::Conflict => conflicts += 1,
        }
    }

    println!();
    println!(
        "{}",
        format!(
            "Summary: {} updated, {} up to date, {} conflicts",
            updated, up_to_date, conflicts
        )
        .dimmed()
    );

    if conflicts > 0 && !force {
        println!("{}", "Use --force to overwrite local changes.".yellow());
    }

    Ok(())
}

enum UpdateResult {
    Updated,
    UpToDate,
    Conflict,
}

fn update_single_skill(
    source: &Path,
    target: &Path,
    name: &str,
    force: bool,
) -> Result<UpdateResult> {
    // Calculate current source hash
    let source_hash = calculate_tree_hash(source)?;

    // Load installed metadata
    let meta_path = target.join(".skill-meta.yaml");
    let meta = SkillMeta::load(&meta_path).ok();

    // Calculate installed hash (excluding .skill-meta.yaml)
    let installed_hash = calculate_tree_hash_excluding(target, &[".skill-meta.yaml"])?;

    // Case 1: Installed files already match source - up to date
    if installed_hash == source_hash {
        // Update meta if it's stale or missing
        if meta.as_ref().is_none_or(|m| m.tree_hash != source_hash) {
            let new_meta = SkillMeta {
                source: source.display().to_string(),
                tree_hash: source_hash,
                installed_at: meta
                    .as_ref()
                    .map(|m| m.installed_at)
                    .unwrap_or_else(Utc::now),
                updated_at: Utc::now(),
            };
            new_meta.save(&meta_path)?;
        }
        println!("{} '{}' is up to date", "✓".green(), name.cyan());
        return Ok(UpdateResult::UpToDate);
    }

    // Case 2: Source unchanged from last install - check for local changes
    let source_unchanged = meta.as_ref().is_some_and(|m| m.tree_hash == source_hash);
    if source_unchanged {
        // Local changes exist but source hasn't changed - nothing to update
        println!(
            "{} '{}' has local changes (source unchanged)",
            "!".yellow(),
            name.cyan()
        );
        return Ok(UpdateResult::UpToDate);
    }

    // Case 3: Source changed - check for local changes (conflict detection)
    let local_changed = meta.as_ref().is_some_and(|m| m.tree_hash != installed_hash);

    if local_changed && !force {
        println!(
            "{} '{}' has local changes. Use --force to overwrite or 'skill diff {}' to see changes.",
            "!".yellow(),
            name.cyan(),
            name
        );
        return Ok(UpdateResult::Conflict);
    }

    // Perform update
    println!("{} Updating '{}'...", "→".blue(), name.cyan());

    // Remove old files (except .skill-meta.yaml)
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|n| n != ".skill-meta.yaml") {
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    }

    // Copy new files
    copy_dir_contents(source, target)?;

    // Update metadata
    let new_meta = SkillMeta {
        source: source.display().to_string(),
        tree_hash: source_hash,
        installed_at: meta.map(|m| m.installed_at).unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
    };
    new_meta.save(&meta_path)?;

    println!("{} Updated '{}'", "✓".green(), name.cyan());

    Ok(UpdateResult::Updated)
}
