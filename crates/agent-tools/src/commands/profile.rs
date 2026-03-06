use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use crate::fs_utils;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileState {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub switched_at: Option<String>,
}

pub fn use_profile(name: &str) -> Result<()> {
    let claude_templates = paths::claude_templates_dir()?;
    let codex_templates = paths::codex_templates_dir()?;
    let claude_source = claude_templates.join(name);
    let codex_source = codex_templates.join(name);
    let has_claude = claude_source.is_dir();
    let has_codex = codex_source.is_dir();
    if !has_claude && !has_codex {
        bail!("Profile '{}' not found", name);
    }

    let local_root = paths::local_state_root()?;
    let state_dir = paths::profile_state_dir()?;
    let active_dir = paths::active_templates_dir()?;
    let snapshots_dir = paths::profile_snapshots_dir()?;
    let runtime_profiles_dir = paths::local_state_root()?.join("profiles");
    fs::create_dir_all(&local_root).with_context(|| {
        format!(
            "Failed to create local state root: {}",
            local_root.display()
        )
    })?;
    fs::create_dir_all(&state_dir)
        .with_context(|| format!("Failed to create state dir: {}", state_dir.display()))?;
    fs::create_dir_all(&active_dir)
        .with_context(|| format!("Failed to create active dir: {}", active_dir.display()))?;
    fs::create_dir_all(&snapshots_dir).with_context(|| {
        format!(
            "Failed to create snapshots dir: {}",
            snapshots_dir.display()
        )
    })?;
    fs::create_dir_all(&runtime_profiles_dir).with_context(|| {
        format!(
            "Failed to create runtime profiles dir: {}",
            runtime_profiles_dir.display()
        )
    })?;

    let current_path = state_dir.join("current.json");
    let previous_path = state_dir.join("previous.json");
    let mut current = load_state(&current_path)?;
    save_state(&previous_path, &current)?;
    snapshot_home_dirs(&snapshots_dir)?;

    let runtime_profile_root = runtime_profiles_dir.join(name);
    fs::create_dir_all(&runtime_profile_root).with_context(|| {
        format!(
            "Failed to create runtime profile dir: {}",
            runtime_profile_root.display()
        )
    })?;

    if has_claude {
        let runtime_claude = runtime_profile_root.join("claude");
        ensure_runtime_profile(&claude_source, &runtime_claude)?;
        switch_home_to_runtime("claude", &paths::claude_home()?, &runtime_claude)?;
        set_active_link(&active_dir.join("claude"), &claude_source)?;
        current.claude = Some(name.to_string());
    }
    if has_codex {
        let runtime_codex = runtime_profile_root.join("codex");
        ensure_runtime_profile(&codex_source, &runtime_codex)?;
        switch_home_to_runtime("codex", &paths::codex_home()?, &runtime_codex)?;
        set_active_link(&active_dir.join("codex"), &codex_source)?;
        current.codex = Some(name.to_string());
    }
    current.switched_at = Some(chrono::Utc::now().to_rfc3339());
    save_state(&current_path, &current)?;

    println!(
        "{}",
        format!(
            "Applied profile '{}' ({})",
            name.cyan(),
            match (has_claude, has_codex) {
                (true, true) => "claude + codex",
                (true, false) => "claude only",
                (false, true) => "codex only",
                (false, false) => "none",
            }
        )
        .green()
    );
    Ok(())
}

pub fn list_profiles() -> Result<()> {
    let profiles = collect_profiles()?;
    if profiles.is_empty() {
        println!("{}", "No profiles found".dimmed());
        return Ok(());
    }

    let current = load_state(&paths::profile_state_dir()?.join("current.json"))?;
    for (name, sides) in profiles {
        let is_current =
            current.claude.as_deref() == Some(&name) || current.codex.as_deref() == Some(&name);
        let marker = if is_current { "*" } else { " " };
        println!("{marker} {name} ({})", sides.join(", "));
    }
    Ok(())
}

pub fn load_state(path: &Path) -> Result<ProfileState> {
    if !path.exists() {
        return Ok(ProfileState::default());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let state = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(state)
}

pub fn save_state(path: &Path, state: &ProfileState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(state)
        .with_context(|| format!("Failed to serialize {}", path.display()))?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn collect_profiles() -> Result<Vec<(String, Vec<String>)>> {
    let mut map: HashMap<String, BTreeSet<String>> = HashMap::new();
    let claude_templates = paths::claude_templates_dir()?;
    let codex_templates = paths::codex_templates_dir()?;

    collect_target_profiles(&claude_templates, "claude", &mut map)?;
    collect_target_profiles(&codex_templates, "codex", &mut map)?;

    let mut items: Vec<(String, Vec<String>)> = map
        .into_iter()
        .map(|(name, sides)| (name, sides.into_iter().collect()))
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(items)
}

fn collect_target_profiles(
    dir: &Path,
    side: &str,
    out: &mut HashMap<String, BTreeSet<String>>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        out.entry(name).or_default().insert(side.to_string());
    }
    Ok(())
}

fn snapshot_home_dirs(snapshots_dir: &Path) -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let snapshot_root = snapshots_dir.join(timestamp);
    let mut copied = false;

    let homes = [
        ("claude", paths::claude_home()?),
        ("codex", paths::codex_home()?),
    ];
    for (side, home) in homes {
        let Some(source) = resolve_active_source(&home) else {
            continue;
        };
        if !source.exists() {
            continue;
        }
        let target = snapshot_root.join(side);
        fs::create_dir_all(&snapshot_root)
            .with_context(|| format!("Failed to create {}", snapshot_root.display()))?;
        fs_utils::copy_dir_recursive(&source, &target).with_context(|| {
            format!(
                "Failed to snapshot active profile {} from {} to {}",
                side,
                source.display(),
                target.display()
            )
        })?;
        copied = true;
    }

    if copied {
        println!("{} {}", "✓ Snapshot:".green(), snapshot_root.display());
    }
    Ok(())
}

fn resolve_active_source(path: &Path) -> Option<PathBuf> {
    if path.is_symlink() {
        let target = fs::read_link(path).ok()?;
        let absolute = if target.is_absolute() {
            target
        } else {
            path.parent()?.join(target)
        };
        return Some(absolute);
    }
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    None
}

fn ensure_runtime_profile(template_dir: &Path, runtime_dir: &Path) -> Result<()> {
    if runtime_dir.exists() {
        return Ok(());
    }
    fs_utils::copy_dir_recursive(template_dir, runtime_dir).with_context(|| {
        format!(
            "Failed to initialize runtime profile from {} to {}",
            template_dir.display(),
            runtime_dir.display()
        )
    })
}

fn switch_home_to_runtime(side: &str, home_path: &Path, runtime_dir: &Path) -> Result<()> {
    if let Some(parent) = home_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    if home_path.is_symlink() {
        if let Ok(link_target) = fs::read_link(home_path) {
            let resolved = if link_target.is_absolute() {
                link_target
            } else {
                home_path
                    .parent()
                    .map(|p| p.join(link_target))
                    .unwrap_or_default()
            };
            if resolved == runtime_dir {
                return Ok(());
            }
        }
        fs::remove_file(home_path)
            .with_context(|| format!("Failed to remove {}", home_path.display()))?;
    } else if home_path.exists() {
        let backup_dir = paths::backups_dir()?;
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create {}", backup_dir.display()))?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("profile_switch_{side}_{timestamp}"));
        fs::rename(home_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup existing {} home from {} to {}",
                side,
                home_path.display(),
                backup_path.display()
            )
        })?;
        println!(
            "{} Backed up existing {} home to {}",
            "!".yellow(),
            side,
            backup_path.display()
        );
    }

    symlink(runtime_dir, home_path).with_context(|| {
        format!(
            "Failed to create symlink {} -> {}",
            home_path.display(),
            runtime_dir.display()
        )
    })?;
    Ok(())
}

fn set_active_link(link_path: &Path, source_dir: &Path) -> Result<()> {
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    if link_path.is_symlink() {
        fs::remove_file(link_path)
            .with_context(|| format!("Failed to remove {}", link_path.display()))?;
    } else if link_path.exists() {
        if link_path.is_dir() {
            fs::remove_dir_all(link_path)
                .with_context(|| format!("Failed to remove {}", link_path.display()))?;
        } else {
            fs::remove_file(link_path)
                .with_context(|| format!("Failed to remove {}", link_path.display()))?;
        }
    }
    symlink(source_dir, link_path).with_context(|| {
        format!(
            "Failed to create symlink {} -> {}",
            link_path.display(),
            source_dir.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_active_source;
    use anyhow::Result;
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn resolve_active_source_for_symlink() -> Result<()> {
        let dir = TempDir::new()?;
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::create_dir_all(&target)?;
        symlink(&target, &link)?;
        let resolved = resolve_active_source(&link).expect("resolved");
        assert_eq!(resolved, target);
        Ok(())
    }

    #[test]
    fn resolve_active_source_for_directory() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolve_active_source(dir.path()).expect("resolved");
        assert_eq!(resolved, dir.path());
        Ok(())
    }
}
