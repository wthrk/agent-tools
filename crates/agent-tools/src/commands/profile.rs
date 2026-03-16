use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use crate::config::validate_skill_name;
use crate::fs_utils;
use crate::paths;

const DEFAULT_PROFILE_NAME: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileState {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub switched_at: Option<String>,
}

pub fn use_profile(name: &str) -> Result<()> {
    validate_profile_name(name)?;

    let claude_templates = paths::claude_templates_dir()?;
    let codex_templates = paths::codex_templates_dir()?;
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
    initialize_default_profiles(&runtime_profiles_dir, &mut current)?;

    let claude_source =
        resolve_profile_source("claude", name, &claude_templates, &runtime_profiles_dir);
    let codex_source =
        resolve_profile_source("codex", name, &codex_templates, &runtime_profiles_dir);
    let has_claude = claude_source.is_some();
    let has_codex = codex_source.is_some();
    if !has_claude && !has_codex {
        bail!("Profile '{}' not found", name);
    }

    save_state(&previous_path, &current)?;
    snapshot_home_dirs(&snapshots_dir)?;

    if let Some(claude_source) = claude_source {
        let runtime_claude = runtime_profiles_dir.join("claude").join(name);
        ensure_runtime_profile(&claude_source, &runtime_claude)?;
        switch_home_to_runtime("claude", &paths::claude_home()?, &runtime_claude)?;
        set_active_link(&active_dir.join("claude"), &claude_source)?;
        current.claude = Some(name.to_string());
    }
    if let Some(codex_source) = codex_source {
        let runtime_codex = runtime_profiles_dir.join("codex").join(name);
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
    let runtime_profiles_dir = paths::local_state_root()?.join("profiles");

    collect_target_profiles(&claude_templates, "claude", &mut map)?;
    collect_target_profiles(&codex_templates, "codex", &mut map)?;
    collect_target_profiles(&runtime_profiles_dir.join("claude"), "claude", &mut map)?;
    collect_target_profiles(&runtime_profiles_dir.join("codex"), "codex", &mut map)?;

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
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%9f").to_string();
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

fn validate_profile_name(name: &str) -> Result<()> {
    validate_skill_name(name).map_err(|e| anyhow::anyhow!("Invalid profile name: {}", e))
}

fn initialize_default_profiles(
    runtime_profiles_dir: &Path,
    current: &mut ProfileState,
) -> Result<()> {
    initialize_default_profile_for_side(
        "claude",
        &paths::claude_home()?,
        runtime_profiles_dir,
        &mut current.claude,
    )?;
    initialize_default_profile_for_side(
        "codex",
        &paths::codex_home()?,
        runtime_profiles_dir,
        &mut current.codex,
    )?;
    Ok(())
}

fn initialize_default_profile_for_side(
    side: &str,
    home_path: &Path,
    runtime_profiles_dir: &Path,
    current_slot: &mut Option<String>,
) -> Result<()> {
    let Some(source) = resolve_active_source(home_path) else {
        return Ok(());
    };

    let side_profiles_dir = runtime_profiles_dir.join(side);
    fs::create_dir_all(&side_profiles_dir)
        .with_context(|| format!("Failed to create {}", side_profiles_dir.display()))?;
    let default_dir = side_profiles_dir.join(DEFAULT_PROFILE_NAME);

    if source.starts_with(&side_profiles_dir) {
        if current_slot.is_none() {
            current_slot.clone_from(&extract_profile_name(&source, &side_profiles_dir));
        }
        return Ok(());
    }

    if !default_dir.exists() {
        fs_utils::copy_dir_recursive(&source, &default_dir).with_context(|| {
            format!(
                "Failed to initialize default {} profile from {} to {}",
                side,
                source.display(),
                default_dir.display()
            )
        })?;
    }

    if current_slot.is_none() {
        *current_slot = Some(DEFAULT_PROFILE_NAME.to_string());
    }
    Ok(())
}

fn extract_profile_name(source: &Path, side_profiles_dir: &Path) -> Option<String> {
    let relative = source.strip_prefix(side_profiles_dir).ok()?;
    let name = relative.components().next()?;
    Some(name.as_os_str().to_string_lossy().to_string())
}

fn resolve_profile_source(
    side: &str,
    name: &str,
    templates_dir: &Path,
    runtime_profiles_dir: &Path,
) -> Option<PathBuf> {
    let template_source = templates_dir.join(name);
    if template_source.is_dir() {
        return Some(template_source);
    }

    let runtime_source = runtime_profiles_dir.join(side).join(name);
    if runtime_source.is_dir() {
        return Some(runtime_source);
    }

    None
}

fn ensure_runtime_profile(template_dir: &Path, runtime_dir: &Path) -> Result<()> {
    if template_dir == runtime_dir {
        return Ok(());
    }
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
    use super::{resolve_active_source, validate_profile_name};
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

    #[test]
    fn resolve_active_source_for_symlink_to_file_returns_none() -> Result<()> {
        let dir = TempDir::new()?;
        let file = dir.path().join("file.txt");
        let link = dir.path().join("link");
        fs::write(&file, "x")?;
        symlink(&file, &link)?;
        let resolved = resolve_active_source(&link);
        assert!(resolved.is_none());
        Ok(())
    }

    #[test]
    fn validate_profile_name_rejects_path_traversal() {
        assert!(validate_profile_name("../bad").is_err());
        assert!(validate_profile_name("bad/name").is_err());
    }
}
