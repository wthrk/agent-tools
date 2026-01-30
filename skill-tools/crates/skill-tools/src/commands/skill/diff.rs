use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::paths;
use crate::project::{find_project_root, project_skills_dir};
use crate::skill_meta::SkillMeta;

pub fn run(name: &str, project: Option<&str>) -> Result<()> {
    // Find source skill
    let skills_dir = paths::skills_dir()?;
    let source_skill = skills_dir.join(name);

    if !source_skill.exists() {
        bail!(
            "Source skill '{}' not found\nLooked in: {}",
            name,
            skills_dir.display()
        );
    }

    // Find installed skill
    let project_root = find_project_root(project)?;
    let project_skills = project_skills_dir(&project_root);
    let installed_skill = project_skills.join(name);

    if !installed_skill.exists() {
        bail!(
            "Skill '{}' is not installed in this project\nProject: {}",
            name,
            project_root.display()
        );
    }

    // Load metadata
    let meta_path = installed_skill.join(".skill-meta.yaml");
    let meta = SkillMeta::load(&meta_path).ok();

    println!(
        "{} {} {}",
        "Diff for skill:".bold(),
        name.cyan(),
        format!("({})", installed_skill.display()).dimmed()
    );
    println!();

    if let Some(meta) = &meta {
        println!("  Source:    {}", meta.source.dimmed());
        println!(
            "  Installed: {}",
            meta.installed_at
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .dimmed()
        );
        println!("  Hash:      {}", meta.tree_hash.dimmed());
        println!();
    }

    // Compare files
    let differences = compare_directories(&source_skill, &installed_skill)?;

    if differences.is_empty() {
        println!("{}", "No differences found - skill is up to date.".green());
    } else {
        println!("{}", "Differences:".yellow());
        for diff in &differences {
            match diff {
                FileDiff::Added(path) => {
                    println!("  {} {}", "+".green(), path.display());
                }
                FileDiff::Removed(path) => {
                    println!("  {} {}", "-".red(), path.display());
                }
                FileDiff::Modified(path) => {
                    println!("  {} {}", "~".yellow(), path.display());
                }
            }
        }
        println!();
        println!(
            "{}",
            format!("Total: {} change(s)", differences.len()).dimmed()
        );
    }

    Ok(())
}

#[derive(Debug)]
enum FileDiff {
    Added(std::path::PathBuf),
    Removed(std::path::PathBuf),
    Modified(std::path::PathBuf),
}

fn compare_directories(source: &Path, target: &Path) -> Result<Vec<FileDiff>> {
    let mut diffs = Vec::new();

    // Get all files from source
    let source_files = collect_files(source, source)?;
    let target_files = collect_files(target, target)?;

    // Filter out .skill-meta.yaml from target
    let target_files: std::collections::HashSet<_> = target_files
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n != ".skill-meta.yaml"))
        .collect();

    let source_files: std::collections::HashSet<_> = source_files.into_iter().collect();

    // Files in source but not in target (would be added on update)
    for path in source_files.difference(&target_files) {
        diffs.push(FileDiff::Added(path.clone()));
    }

    // Files in target but not in source (would be removed on update)
    for path in target_files.difference(&source_files) {
        diffs.push(FileDiff::Removed(path.clone()));
    }

    // Files in both - check content
    for path in source_files.intersection(&target_files) {
        let source_content = fs::read(source.join(path))?;
        let target_content = fs::read(target.join(path))?;
        if source_content != target_content {
            diffs.push(FileDiff::Modified(path.clone()));
        }
    }

    diffs.sort_by(|a, b| {
        let path_a = match a {
            FileDiff::Added(p) | FileDiff::Removed(p) | FileDiff::Modified(p) => p,
        };
        let path_b = match b {
            FileDiff::Added(p) | FileDiff::Removed(p) | FileDiff::Modified(p) => p,
        };
        path_a.cmp(path_b)
    });

    Ok(diffs)
}

fn collect_files(dir: &Path, base: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir).context("Failed to read directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(collect_files(&path, base)?);
        } else {
            let relative = path
                .strip_prefix(base)
                .context("Failed to get relative path")?;
            files.push(relative.to_path_buf());
        }
    }

    Ok(files)
}
