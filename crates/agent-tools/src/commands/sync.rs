use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::fs_utils;
use crate::paths;

pub fn run(dry_run: bool, prune: bool) -> Result<()> {
    let agent_tools_home = paths::agent_tools_home()?;
    let claude_source_home = resolve_claude_source_home(&agent_tools_home);
    let codex_source_root = resolve_codex_source_root(&agent_tools_home);
    let config_path = resolve_claude_config_path(&agent_tools_home, &claude_source_home);
    let config = Config::load(&config_path)?;

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
        &claude_source_home,
        &claude_home,
        config.manage_settings,
        dry_run,
    )?;

    // Manage plugins/
    println!();
    println!("{}", "Plugins:".bold());
    sync_plugins(
        &claude_source_home,
        &claude_home,
        config.manage_plugins,
        dry_run,
    )?;

    // Manage CLAUDE.md
    println!();
    println!("{}", "CLAUDE.md:".bold());
    sync_claude_md(
        &claude_source_home,
        &claude_home,
        config.manage_claude_md,
        dry_run,
    )?;

    // Manage hooks/
    println!();
    println!("{}", "Hooks:".bold());
    sync_hooks(
        &claude_source_home,
        &claude_home,
        config.manage_hooks,
        dry_run,
    )?;

    // Manage codex config
    let codex_home = paths::codex_home()?;
    println!();
    println!("{}", "Codex config:".bold());
    sync_codex_config(
        &codex_source_root,
        &codex_home,
        config.manage_codex_config,
        dry_run,
    )?;
    sync_codex_agents(
        &codex_source_root,
        &codex_home,
        config.manage_codex_config,
        dry_run,
    )?;

    // Manage Claude MCP servers
    println!();
    println!("{}", "Claude MCP servers:".bold());
    sync_claude_mcp_servers(&config, &agent_tools_home, dry_run)?;

    // Warn about settings/hooks dependency
    if config.manage_settings && !config.manage_hooks {
        println!();
        println!(
            "{} settings.json references ~/.claude/hooks/ but manage_hooks is false",
            "Warning:".yellow().bold()
        );
        println!("  Consider setting manage_hooks: true or hooks may not be found");
    }

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
    claude_source_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = claude_source_home.join("settings.json");
    let target = claude_home.join("settings.json");

    if !manage {
        println!("  {} Not managed (manage_settings: false)", "·".dimmed());
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    if target.is_symlink() {
        if let Ok(link_target) = fs::read_link(&target) {
            if link_target == source {
                println!("  {} Already linked", "✓".green());
                return Ok(());
            }
            // Broken symlink - repair it
            if !target.exists() {
                if dry_run {
                    println!(
                        "  {} Would repair broken symlink → {}",
                        "→".blue(),
                        source.display()
                    );
                } else {
                    fs::remove_file(&target)?;
                    symlink(&source, &target)?;
                    println!(
                        "  {} Repaired broken symlink → {}",
                        "✓".green(),
                        source.display()
                    );
                }
                return Ok(());
            }
            // Different link target - warn but don't change
            println!(
                "  {} Exists but points to different target: {}",
                "!".yellow(),
                link_target.display()
            );
            return Ok(());
        }
    } else if target.exists() {
        // File exists but is not a symlink - warn but don't change
        println!(
            "  {} Exists but is not a symlink (not managed)",
            "!".yellow()
        );
        return Ok(());
    }

    // Target doesn't exist - create link
    if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}

fn sync_plugins(
    claude_source_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = claude_source_home.join("plugins");
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
                // Check if the symlink is broken (target doesn't exist)
                if !target.exists() {
                    // Broken symlink - update it
                    if dry_run {
                        println!(
                            "  {} Would update broken link to {}",
                            "→".blue(),
                            source.display()
                        );
                    } else {
                        fs::remove_file(&target)?;
                        symlink(&source, &target)?;
                        println!(
                            "  {} Updated broken link to {}",
                            "✓".green(),
                            source.display()
                        );
                    }
                    return Ok(());
                }
                // Different link target but target exists - warn but don't change
                println!(
                    "  {} Exists but points to different target: {}",
                    "!".yellow(),
                    link_target.display()
                );
                return Ok(());
            }
        }
        // Directory exists but is not a symlink - warn but don't change
        println!(
            "  {} Exists but is not a symlink (not managed)",
            "!".yellow()
        );
        return Ok(());
    }

    // Target doesn't exist - create link
    if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}

fn sync_claude_md(
    claude_source_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = claude_source_home.join("global/CLAUDE.md");
    let target = claude_home.join("CLAUDE.md");

    if !manage {
        println!("  {} Not managed (manage_claude_md: false)", "·".dimmed());
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    if target.is_symlink() {
        if let Ok(link_target) = fs::read_link(&target) {
            if link_target == source {
                println!("  {} Already linked", "✓".green());
                return Ok(());
            }
            // Broken symlink - repair it
            if !target.exists() {
                if dry_run {
                    println!(
                        "  {} Would repair broken symlink → {}",
                        "→".blue(),
                        source.display()
                    );
                } else {
                    fs::remove_file(&target)?;
                    symlink(&source, &target)?;
                    println!(
                        "  {} Repaired broken symlink → {}",
                        "✓".green(),
                        source.display()
                    );
                }
                return Ok(());
            }
            // Different link target - warn but don't change
            println!(
                "  {} Exists but points to different target: {}",
                "!".yellow(),
                link_target.display()
            );
            return Ok(());
        }
    } else if target.exists() {
        // File exists but is not a symlink - warn but don't change
        println!(
            "  {} Exists but is not a symlink (not managed)",
            "!".yellow()
        );
        return Ok(());
    }

    // Target doesn't exist - create link
    if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}

fn sync_hooks(
    claude_source_home: &Path,
    claude_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = claude_source_home.join("global/hooks");
    let target = claude_home.join("hooks");

    if !manage {
        println!("  {} Not managed (manage_hooks: false)", "·".dimmed());
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    if target.is_symlink() {
        if let Ok(link_target) = fs::read_link(&target) {
            if link_target == source {
                println!("  {} Already linked", "✓".green());
                return Ok(());
            }
            // Broken symlink - repair it
            if !target.exists() {
                if dry_run {
                    println!(
                        "  {} Would repair broken symlink → {}",
                        "→".blue(),
                        source.display()
                    );
                } else {
                    fs::remove_file(&target)?;
                    symlink(&source, &target)?;
                    println!(
                        "  {} Repaired broken symlink → {}",
                        "✓".green(),
                        source.display()
                    );
                }
                return Ok(());
            }
            // Different link target - warn but don't change
            println!(
                "  {} Exists but points to different target: {}",
                "!".yellow(),
                link_target.display()
            );
            return Ok(());
        }
    } else if target.exists() {
        // Directory exists but is not a symlink - warn but don't change
        println!(
            "  {} Exists but is not a symlink (not managed)",
            "!".yellow()
        );
        return Ok(());
    }

    // Target doesn't exist - create link
    if dry_run {
        println!("  {} Would link to {}", "→".blue(), source.display());
    } else {
        symlink(&source, &target)?;
        println!("  {} Linked to {}", "✓".green(), source.display());
    }

    Ok(())
}

fn sync_codex_config(
    codex_source_root: &Path,
    codex_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = codex_source_root.join("config.toml");
    let local_override = codex_home.join("config.local.toml");
    let target = codex_home.join("config.toml");

    if !manage {
        println!(
            "  {} Not managed (manage_codex_config: false)",
            "·".dimmed()
        );
        return Ok(());
    }

    if !source.exists() {
        println!("  {} Source not found: {}", "!".yellow(), source.display());
        return Ok(());
    }

    let mut merged = load_toml_value(&source)?;
    if local_override.exists() {
        let local = load_toml_value(&local_override)?;
        merge_toml_values(&mut merged, local);
    }
    let rendered = toml::to_string_pretty(&merged)
        .context("Failed to serialize merged codex config as TOML")?;

    if dry_run {
        if local_override.exists() {
            println!(
                "  {} Would render {} + {}",
                "→".blue(),
                source.display(),
                local_override.display()
            );
        } else {
            println!("  {} Would render {}", "→".blue(), source.display());
        }
        if target.is_symlink() {
            println!(
                "  {} Would replace symlink target {}",
                "→".blue(),
                target.display()
            );
        } else if target.exists() {
            println!("  {} Would update {}", "→".blue(), target.display());
        } else {
            println!("  {} Would create {}", "→".blue(), target.display());
        }
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    if target.exists() && target.is_file() && !target.is_symlink() {
        let existing = fs::read_to_string(&target)
            .with_context(|| format!("Failed to read existing {}", target.display()))?;
        if existing == rendered {
            println!("  {} Already up to date", "✓".green());
            return Ok(());
        }
    }

    if target.is_symlink() {
        fs::remove_file(&target)
            .with_context(|| format!("Failed to remove legacy symlink {}", target.display()))?;
        println!(
            "  {} Removed legacy symlink {}",
            "!".yellow(),
            target.display()
        );
    } else if target.exists() {
        let backup_dir = paths::backups_dir()?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("codex_config_{timestamp}.toml"));
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create {}", backup_dir.display()))?;
        fs::rename(&target, &backup_path).with_context(|| {
            format!(
                "Failed to backup existing codex config from {} to {}",
                target.display(),
                backup_path.display()
            )
        })?;
        println!(
            "  {} Backed up existing file to {}",
            "!".yellow(),
            backup_path.display()
        );
    }

    fs::write(&target, rendered)
        .with_context(|| format!("Failed to write {}", target.display()))?;
    println!("  {} Rendered {}", "✓".green(), target.display());

    Ok(())
}

fn sync_codex_agents(
    codex_source_root: &Path,
    codex_home: &Path,
    manage: bool,
    dry_run: bool,
) -> Result<()> {
    let source = codex_source_root.join("agents");
    let target = codex_home.join("agents");

    if !manage {
        return Ok(());
    }
    if !source.exists() {
        return Ok(());
    }

    if dry_run {
        println!(
            "  {} Would sync agents {} -> {}",
            "→".blue(),
            source.display(),
            target.display()
        );
        return Ok(());
    }

    if target.is_symlink() {
        fs::remove_file(&target)
            .with_context(|| format!("Failed to remove legacy symlink {}", target.display()))?;
    } else if target.exists() {
        let backup_dir = paths::backups_dir()?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("codex_agents_{timestamp}"));
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create {}", backup_dir.display()))?;
        fs::rename(&target, &backup_path).with_context(|| {
            format!(
                "Failed to backup existing codex agents from {} to {}",
                target.display(),
                backup_path.display()
            )
        })?;
        println!(
            "  {} Backed up existing agents to {}",
            "!".yellow(),
            backup_path.display()
        );
    }

    fs_utils::copy_dir_recursive(&source, &target).with_context(|| {
        format!(
            "Failed to copy codex agents from {} to {}",
            source.display(),
            target.display()
        )
    })?;
    println!("  {} Synced agents to {}", "✓".green(), target.display());
    Ok(())
}

fn sync_claude_mcp_servers(config: &Config, agent_tools_home: &Path, dry_run: bool) -> Result<()> {
    let state_path = agent_tools_home.join("state/claude_mcp_managed.json");
    let previous_names = load_managed_mcp_names(&state_path)?;
    let current_names: HashSet<String> = config.claude_mcp_servers.keys().cloned().collect();
    let mut stale_names: Vec<String> = previous_names.difference(&current_names).cloned().collect();
    stale_names.sort();
    let stdin_is_terminal = io::stdin().is_terminal();
    let mut not_removed = Vec::new();

    for name in &stale_names {
        if dry_run {
            println!("  {} Would remove stale '{}'", "→".blue(), name.cyan());
            continue;
        }

        if !stdin_is_terminal {
            println!(
                "  {} Skipped stale '{}' (non-interactive session)",
                "!".yellow(),
                name.cyan()
            );
            not_removed.push(name.clone());
            continue;
        }

        if !confirm_mcp_removal(name)? {
            println!(
                "  {} Kept stale '{}' by user choice",
                "·".dimmed(),
                name.cyan()
            );
            not_removed.push(name.clone());
            continue;
        }

        let output = Command::new("claude")
            .args(["mcp", "remove", "-s", "user", name])
            .output()
            .with_context(|| format!("Failed to run 'claude mcp remove' for '{name}'"))?;
        if output.status.success() {
            println!("  {} Removed stale '{}'", "✓".green(), name.cyan());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "  {} Failed to remove stale '{}': {}",
                "!".yellow(),
                name.cyan(),
                stderr.trim()
            );
            // Keep stale entries that failed to remove so they are retried.
            not_removed.push(name.clone());
        }
    }

    let mut names: Vec<String> = config.claude_mcp_servers.keys().cloned().collect();
    names.sort();
    for name in &names {
        let Some(server) = config.claude_mcp_servers.get(name) else {
            continue;
        };
        let json = serde_json::json!({
            "type": server.transport_type,
            "command": server.command,
            "args": server.args,
            "env": server.env,
        });
        let json_str = serde_json::to_string(&json)
            .with_context(|| format!("Failed to serialize MCP server config for '{name}'"))?;

        if dry_run {
            println!(
                "  {} Would register '{}': {} {}",
                "→".blue(),
                name.cyan(),
                server.command,
                server.args.join(" ")
            );
            continue;
        }

        // Remove existing server first (ignore errors if not found)
        let _ = Command::new("claude")
            .args(["mcp", "remove", "-s", "user", name])
            .output();

        let output = Command::new("claude")
            .args(["mcp", "add-json", "-s", "user", name, &json_str])
            .output()
            .with_context(|| format!("Failed to run 'claude mcp add-json' for '{name}'"))?;

        if output.status.success() {
            println!(
                "  {} Registered '{}': {} {}",
                "✓".green(),
                name.cyan(),
                server.command,
                server.args.join(" ")
            );
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "  {} Failed to register '{}': {}",
                "!".yellow(),
                name.cyan(),
                stderr.trim()
            );
        }
    }

    if !dry_run {
        let mut next_names: HashSet<String> = names.into_iter().collect();
        for name in not_removed {
            next_names.insert(name);
        }
        let mut next_names_vec: Vec<String> = next_names.into_iter().collect();
        next_names_vec.sort();
        save_managed_mcp_names(&state_path, &next_names_vec)?;
    }

    Ok(())
}

fn confirm_mcp_removal(name: &str) -> Result<bool> {
    print!("  ? Remove stale MCP '{}'? [y/N]: ", name.cyan());
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read user input")?;

    let normalized = input.trim().to_ascii_lowercase();
    Ok(normalized == "y" || normalized == "yes")
}

fn load_toml_value(path: &Path) -> Result<toml::Value> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read TOML file: {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("Failed to parse TOML file: {}", path.display()))
}

fn merge_toml_values(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                if let Some(base_value) = base_table.get_mut(&key) {
                    merge_toml_values(base_value, overlay_value);
                } else {
                    base_table.insert(key, overlay_value);
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn load_managed_mcp_names(path: &Path) -> Result<HashSet<String>> {
    if !path.exists() {
        return Ok(HashSet::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let names: Vec<String> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(names.into_iter().collect())
}

fn save_managed_mcp_names(path: &Path, names: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(names)
        .with_context(|| format!("Failed to serialize {}", path.display()))?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn resolve_claude_config_path(agent_tools_home: &Path, claude_source_home: &Path) -> PathBuf {
    let active_config = claude_source_home.join("config.yaml");
    if active_config.exists() {
        active_config
    } else {
        agent_tools_home.join("config.yaml")
    }
}

fn resolve_claude_source_home(agent_tools_home: &Path) -> PathBuf {
    let active = agent_tools_home.join(".local/active/claude");
    resolve_link_or_directory(&active).unwrap_or_else(|| agent_tools_home.to_path_buf())
}

fn resolve_codex_source_root(agent_tools_home: &Path) -> PathBuf {
    let active = agent_tools_home.join(".local/active/codex");
    resolve_link_or_directory(&active).unwrap_or_else(|| agent_tools_home.join("codex"))
}

fn resolve_link_or_directory(path: &Path) -> Option<PathBuf> {
    if path.is_symlink() {
        let target = fs::read_link(path).ok()?;
        let absolute = if target.is_absolute() {
            target
        } else {
            path.parent()?.join(target)
        };
        if absolute.is_dir() {
            return Some(absolute);
        }
        return None;
    }
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::merge_toml_values;
    use anyhow::Result;

    #[test]
    fn merge_toml_values_recursively_merges_tables() -> Result<()> {
        let mut base: toml::Value = toml::from_str(
            r#"
model = "gpt-5"

[providers.main]
timeout = 30
retries = 2
"#,
        )?;
        let local: toml::Value = toml::from_str(
            r#"
model = "gpt-5.3-codex"

[providers.main]
timeout = 10
"#,
        )?;

        merge_toml_values(&mut base, local);

        let expected: toml::Value = toml::from_str(
            r#"
model = "gpt-5.3-codex"

[providers.main]
timeout = 10
retries = 2
"#,
        )?;
        assert_eq!(base, expected);
        Ok(())
    }

    #[test]
    fn merge_toml_values_replaces_arrays() -> Result<()> {
        let mut base: toml::Value = toml::from_str(
            r#"
tools = ["a", "b", "c"]
"#,
        )?;
        let local: toml::Value = toml::from_str(
            r#"
tools = ["x"]
"#,
        )?;

        merge_toml_values(&mut base, local);

        let expected: toml::Value = toml::from_str(
            r#"
tools = ["x"]
"#,
        )?;
        assert_eq!(base, expected);
        Ok(())
    }
}
