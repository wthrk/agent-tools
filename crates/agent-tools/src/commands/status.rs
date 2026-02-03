use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::config::Config;
use crate::paths;

pub fn run() -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;
    let skills_dir = paths::skills_dir()?;
    let config_path = paths::config_path()?;
    let claude_home = paths::claude_home()?;
    let claude_skills = paths::claude_skills_dir()?;
    let config = Config::load(&config_path)?;

    println!("{}", "agent-tools status".green().bold());
    println!();

    // Agent-tools home
    println!("{}", "Installation:".bold());
    println!(
        "  Home:   {}",
        if agent_tools_home.exists() {
            agent_tools_home.display().to_string().green()
        } else {
            format!("{} (not found)", agent_tools_home.display()).red()
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

    // CLAUDE.md status
    println!();
    println!("{}", "CLAUDE.md:".bold());
    let agent_tools_claude_md = agent_tools_home.join("global/CLAUDE.md");
    let claude_claude_md = claude_home.join("CLAUDE.md");
    println!(
        "  Source:  {}",
        if agent_tools_claude_md.exists() {
            agent_tools_claude_md.display().to_string().green()
        } else {
            format!("{} (not found)", agent_tools_claude_md.display()).dimmed()
        }
    );
    print!("  Target:  ");
    if claude_claude_md.is_symlink() {
        if claude_claude_md.exists() {
            if let Ok(target) = fs::read_link(&claude_claude_md) {
                if target == agent_tools_claude_md {
                    println!("{}", "linked".green());
                } else {
                    println!("{} → {}", "symlink".yellow(), target.display());
                }
            }
        } else {
            println!("{}", "(broken symlink)".red());
        }
    } else if claude_claude_md.exists() {
        println!("{}", "(file exists, not managed)".yellow());
    } else {
        println!("{}", "(not found)".dimmed());
    }
    println!(
        "  Managed: {}",
        if config.manage_claude_md {
            "yes".green()
        } else {
            "no".dimmed()
        }
    );

    // Hooks status
    println!();
    println!("{}", "Hooks:".bold());
    let agent_tools_hooks = agent_tools_home.join("global/hooks");
    let claude_hooks = claude_home.join("hooks");
    println!(
        "  Source:  {}",
        if agent_tools_hooks.exists() {
            agent_tools_hooks.display().to_string().green()
        } else {
            format!("{} (not found)", agent_tools_hooks.display()).dimmed()
        }
    );
    print!("  Target:  ");
    if claude_hooks.is_symlink() {
        if claude_hooks.exists() {
            if let Ok(target) = fs::read_link(&claude_hooks) {
                if target == agent_tools_hooks {
                    println!("{}", "linked".green());
                } else {
                    println!("{} → {}", "symlink".yellow(), target.display());
                }
            }
        } else {
            println!("{}", "(broken symlink)".red());
        }
    } else if claude_hooks.exists() && claude_hooks.is_dir() {
        println!("{}", "(directory exists, not managed)".yellow());
    } else {
        println!("{}", "(not found)".dimmed());
    }
    println!(
        "  Managed: {}",
        if config.manage_hooks {
            "yes".green()
        } else {
            "no".dimmed()
        }
    );

    // List hooks if source exists
    if agent_tools_hooks.exists() {
        if let Ok(entries) = fs::read_dir(&agent_tools_hooks) {
            let hooks: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect();
            if !hooks.is_empty() {
                println!("  Files:");
                for hook in &hooks {
                    println!("    - {}", hook.file_name().to_string_lossy().cyan());
                }
            }
        }
    }

    Ok(())
}
