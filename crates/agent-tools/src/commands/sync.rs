use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use crate::config::Config;
use crate::paths;

pub fn run(dry_run: bool, prune: bool) -> Result<()> {
    let config_path = paths::config_path()?;
    let config = Config::load(&config_path)?;

    let agent_tools_home = paths::agent_tools_home()?;
    let skills_source = paths::skills_dir()?;
    let claude_home = paths::claude_home()?;
    let claude_skills = paths::claude_skills_dir()?;

    if dry_run {
        println!(
            "{}",
            "Dry run mode - no changes will be made".yellow().bold()
        );
        println!();
    }

    println!("{}", "Syncing ~/.claude with config.yaml...".green().bold());
    println!();

    // Ensure claude home exists
    if !claude_home.exists() {
        if dry_run {
            println!("{} Would create {}", "→".blue(), claude_home.display());
        } else {
            fs::create_dir_all(&claude_home).context("Failed to create ~/.claude")?;
            println!("{} Created {}", "✓".green(), claude_home.display());
        }
    }

    // Ensure claude skills directory exists
    if !claude_skills.exists() {
        if dry_run {
            println!("{} Would create {}", "→".blue(), claude_skills.display());
        } else {
            fs::create_dir_all(&claude_skills).context("Failed to create ~/.claude/skills")?;
            println!("{} Created {}", "✓".green(), claude_skills.display());
        }
    }

    // Sync skills
    println!("{}", "Skills:".bold());
    let mut linked = 0;
    let mut already_linked = 0;
    let mut orphaned = Vec::new();

    // Process auto_deploy_skills
    for skill_name in &config.auto_deploy_skills {
        let source = skills_source.join(skill_name);
        let target = claude_skills.join(skill_name);

        if !source.exists() {
            println!(
                "  {} '{}': source not found at {}",
                "!".yellow(),
                skill_name.cyan(),
                source.display()
            );
            continue;
        }

        if target.exists() || target.is_symlink() {
            if target.is_symlink() {
                if let Ok(link_target) = fs::read_link(&target) {
                    if link_target == source {
                        println!("  {} '{}' already linked", "✓".green(), skill_name.cyan());
                        already_linked += 1;
                        continue;
                    }
                }
            }
            // Different link or not a link - need to handle
            if dry_run {
                println!(
                    "  {} Would remove existing '{}' and create link",
                    "→".blue(),
                    skill_name.cyan()
                );
            } else {
                // Backup if it's a directory (not a symlink)
                if !target.is_symlink() && target.is_dir() {
                    let backup_dir = paths::backups_dir()?;
                    fs::create_dir_all(&backup_dir)?;
                    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                    let backup_path = backup_dir.join(format!("{skill_name}_{timestamp}"));
                    fs::rename(&target, &backup_path)
                        .context("Failed to backup existing directory")?;
                    println!(
                        "  {} Backed up '{}' to {}",
                        "!".yellow(),
                        skill_name,
                        backup_path.display()
                    );
                } else {
                    fs::remove_file(&target).or_else(|_| fs::remove_dir_all(&target))?;
                }
            }
        }

        if dry_run {
            println!(
                "  {} Would link '{}' → {}",
                "→".blue(),
                skill_name.cyan(),
                source.display()
            );
        } else {
            symlink(&source, &target)
                .with_context(|| format!("Failed to create symlink for '{skill_name}'"))?;
            println!(
                "  {} Linked '{}' → {}",
                "✓".green(),
                skill_name.cyan(),
                source.display()
            );
        }
        linked += 1;
    }

    // Check for orphaned links (symlinks pointing to skills_source but not in config)
    if claude_skills.exists() {
        for entry in fs::read_dir(&claude_skills)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_symlink() {
                if let Ok(link_target) = fs::read_link(&path) {
                    // Check if this symlink points to our skills source
                    if link_target.starts_with(&skills_source)
                        && !config.auto_deploy_skills.contains(&name)
                    {
                        orphaned.push(name);
                    }
                }
            }
        }
    }

    if !orphaned.is_empty() {
        println!();
        if prune {
            println!("{}", "Removing orphaned links:".bold());
            for name in &orphaned {
                let target = claude_skills.join(name);
                if dry_run {
                    println!("  {} Would remove '{}'", "→".blue(), name.cyan());
                } else {
                    fs::remove_file(&target)?;
                    println!("  {} Removed '{}'", "✓".green(), name.cyan());
                }
            }
        } else {
            println!("{}", "Orphaned links (not in config):".yellow().bold());
            for name in &orphaned {
                println!("  - {}", name.cyan());
            }
            println!("{}", "Use --prune to remove these links.".dimmed());
        }
    }

    // Manage settings.json
    println!();
    println!("{}", "Settings:".bold());
    sync_settings(
        &agent_tools_home,
        &claude_home,
        config.manage_settings,
        dry_run,
    )?;

    // Manage plugins/
    println!();
    println!("{}", "Plugins:".bold());
    sync_plugins(
        &agent_tools_home,
        &claude_home,
        config.manage_plugins,
        dry_run,
    )?;

    // Summary
    println!();
    if dry_run {
        println!(
            "{}",
            format!(
                "Would link {linked} skills ({already_linked} already linked, {} orphaned)",
                orphaned.len()
            )
            .dimmed()
        );
    } else {
        println!(
            "{}",
            format!(
                "Synced: {linked} linked, {already_linked} already linked, {} orphaned",
                orphaned.len()
            )
            .dimmed()
        );
    }

    Ok(())
}

fn sync_settings(
    agent_tools_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = agent_tools_home.join("settings.json");
    let target = claude_home.join("settings.json");

    if !manage {
        println!("  {} Not managed (manage_settings: false)", "·".dimmed());
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    if target.exists() || target.is_symlink() {
        if target.is_symlink() {
            if let Ok(link_target) = fs::read_link(&target) {
                if link_target == source {
                    println!("  {} Already linked", "✓".green());
                    return Ok(());
                }
            }
        }

        // Different link or not a link
        if dry_run {
            println!(
                "  {} Would replace with link to {}",
                "→".blue(),
                source.display()
            );
        } else {
            // Backup existing settings
            if !target.is_symlink() {
                let backup_dir = paths::backups_dir()?;
                fs::create_dir_all(&backup_dir)?;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let backup_path = backup_dir.join(format!("settings.json_{timestamp}"));
                fs::copy(&target, &backup_path)?;
                println!(
                    "  {} Backed up existing settings to {}",
                    "!".yellow(),
                    backup_path.display()
                );
            }
            fs::remove_file(&target)?;
            symlink(&source, &target)?;
            println!("  {} Linked to {}", "✓".green(), source.display());
        }
    } else if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}

fn sync_plugins(
    agent_tools_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = agent_tools_home.join("plugins");
    let target = claude_home.join("plugins");

    if !manage {
        println!("  {} Not managed (manage_plugins: false)", "·".dimmed());
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    if target.exists() || target.is_symlink() {
        if target.is_symlink() {
            if let Ok(link_target) = fs::read_link(&target) {
                if link_target == source {
                    println!("  {} Already linked", "✓".green());
                    return Ok(());
                }
            }
        }

        // Different link or not a link
        if dry_run {
            println!(
                "  {} Would replace with link to {}",
                "→".blue(),
                source.display()
            );
        } else {
            // Backup existing plugins
            if !target.is_symlink() && target.is_dir() {
                let backup_dir = paths::backups_dir()?;
                fs::create_dir_all(&backup_dir)?;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let backup_path = backup_dir.join(format!("plugins_{timestamp}"));
                fs::rename(&target, &backup_path)?;
                println!(
                    "  {} Backed up existing plugins to {}",
                    "!".yellow(),
                    backup_path.display()
                );
            } else {
                fs::remove_file(&target).or_else(|_| fs::remove_dir_all(&target))?;
            }
            symlink(&source, &target)?;
            println!("  {} Linked to {}", "✓".green(), source.display());
        }
    } else if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}
