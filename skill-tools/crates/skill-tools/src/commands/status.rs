use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::paths;

pub fn run() -> Result<()> {
    let skill_tools_home = paths::skill_tools_home()?;
    let skills_dir = paths::skills_dir()?;
    let config_path = paths::config_path()?;
    let claude_home = paths::claude_home()?;
    let claude_skills = paths::claude_skills_dir()?;

    println!("{}", "skill-tools status".green().bold());
    println!();

    // Skill-tools home
    println!("{}", "Installation:".bold());
    println!(
        "  Home:   {}",
        if skill_tools_home.exists() {
            skill_tools_home.display().to_string().green()
        } else {
            format!("{} (not found)", skill_tools_home.display()).red()
        }
    );
    println!(
        "  Config: {}",
        if config_path.exists() {
            config_path.display().to_string().green()
        } else {
            format!("{} (not found)", config_path.display()).yellow()
        }
    );
    println!();

    // Available skills
    println!("{}", "Available skills:".bold());
    if skills_dir.exists() {
        match fs::read_dir(&skills_dir) {
            Ok(entries) => {
                let skills: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir() && e.path().join("SKILL.md").exists())
                    .collect();

                if skills.is_empty() {
                    println!("  {}", "None".dimmed());
                } else {
                    for skill in &skills {
                        println!("  - {}", skill.file_name().to_string_lossy().cyan());
                    }
                }
            }
            Err(e) => {
                println!("  {} Failed to read skills directory: {}", "!".yellow(), e);
            }
        }
    } else {
        println!("  {}", "(skills directory not found)".yellow());
    }
    println!();

    // Claude home status
    println!("{}", "~/.claude status:".bold());
    println!(
        "  Path:   {}",
        if claude_home.exists() {
            claude_home.display().to_string().green()
        } else {
            format!("{} (not found)", claude_home.display()).yellow()
        }
    );

    if claude_skills.exists() {
        match fs::read_dir(&claude_skills) {
            Ok(entries) => {
                let links: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();

                if links.is_empty() {
                    println!("  Skills: {}", "None".dimmed());
                } else {
                    println!("  Skills:");
                    for link in &links {
                        let path = link.path();
                        let name = link.file_name().to_string_lossy().to_string();

                        if path.is_symlink() {
                            if let Ok(target) = fs::read_link(&path) {
                                println!(
                                    "    {} → {} {}",
                                    name.cyan(),
                                    target.display(),
                                    "(symlink)".dimmed()
                                );
                            } else {
                                println!("    {} {}", name.cyan(), "(broken symlink)".red());
                            }
                        } else {
                            println!("    {} {}", name.cyan(), "(directory)".dimmed());
                        }
                    }
                }
            }
            Err(e) => {
                println!(
                    "  {} Failed to read claude skills directory: {}",
                    "!".yellow(),
                    e
                );
            }
        }
    } else {
        println!("  Skills: {}", "(not found)".dimmed());
    }

    Ok(())
}
