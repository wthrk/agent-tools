//! Skill directory detection and validation.

use crate::config::{ConfigError, load_config};
use crate::types::SkillDir;
use glob::glob;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during skill directory operations.
#[derive(Error, Debug)]
pub enum SkillDirError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SKILL.md not found in {0}")]
    SkillMdNotFound(PathBuf),
    #[error("SKILL.md missing YAML frontmatter in {0}")]
    MissingFrontmatter(PathBuf),
    #[error("SKILL.md frontmatter not closed in {0}")]
    UnclosedFrontmatter(PathBuf),
    #[error("SKILL.md missing 'name' field in {0}")]
    MissingName(PathBuf),
    #[error("duplicate skill name '{name}': first in {first}, also in {second}")]
    DuplicateName {
        name: String,
        first: PathBuf,
        second: PathBuf,
    },
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
    #[error("no skill directories found matching pattern: {0}")]
    NoSkillDirsFound(String),
}

/// Detect a single skill directory.
///
/// # Errors
/// Returns an error if:
/// - The path doesn't exist or isn't a directory
/// - SKILL.md doesn't exist
/// - SKILL.md is malformed
/// - name field is missing
pub fn detect_skill_dir(path: &Path) -> Result<SkillDir, SkillDirError> {
    // Verify path exists and is a directory
    if !path.exists() {
        return Err(SkillDirError::DirectoryNotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(SkillDirError::DirectoryNotFound(path.to_path_buf()));
    }

    // Extract skill name from SKILL.md
    let name = extract_skill_name(path)?;

    // Load config
    let config = load_config(path)?;

    Ok(SkillDir {
        path: path.to_path_buf(),
        name,
        config,
    })
}

/// Detect multiple skill directories and validate no duplicate names.
///
/// # Errors
/// Returns an error if:
/// - Any individual skill directory detection fails
/// - There are duplicate skill names
pub fn detect_skill_dirs(paths: &[PathBuf]) -> Result<Vec<SkillDir>, SkillDirError> {
    let mut skill_dirs = Vec::with_capacity(paths.len());
    let mut name_to_path: HashMap<String, PathBuf> = HashMap::new();

    for path in paths {
        let skill_dir = detect_skill_dir(path)?;

        // Check for duplicate names
        if let Some(first_path) = name_to_path.get(&skill_dir.name) {
            return Err(SkillDirError::DuplicateName {
                name: skill_dir.name,
                first: first_path.clone(),
                second: path.clone(),
            });
        }

        name_to_path.insert(skill_dir.name.clone(), path.clone());
        skill_dirs.push(skill_dir);
    }

    Ok(skill_dirs)
}

/// Check if a path is a skill directory (has SKILL.md).
#[must_use]
pub fn is_skill_dir(path: &Path) -> bool {
    path.is_dir() && path.join("SKILL.md").exists()
}

/// Extract skill name from SKILL.md frontmatter.
fn extract_skill_name(skill_path: &Path) -> Result<String, SkillDirError> {
    let skill_md = skill_path.join("SKILL.md");

    if !skill_md.exists() {
        return Err(SkillDirError::SkillMdNotFound(skill_path.to_path_buf()));
    }

    let content = std::fs::read_to_string(&skill_md)?;

    // Parse YAML frontmatter
    if !content.starts_with("---") {
        return Err(SkillDirError::MissingFrontmatter(skill_path.to_path_buf()));
    }

    let end = content[3..]
        .find("---")
        .ok_or_else(|| SkillDirError::UnclosedFrontmatter(skill_path.to_path_buf()))?;

    let frontmatter = &content[3..3 + end];

    // Extract name field
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            let name = value.trim().trim_matches('"').trim_matches('\'');
            if name.is_empty() {
                return Err(SkillDirError::MissingName(skill_path.to_path_buf()));
            }
            return Ok(name.to_string());
        }
    }

    Err(SkillDirError::MissingName(skill_path.to_path_buf()))
}

/// Check if a path string contains glob wildcards.
fn contains_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Resolve skill directory paths from arguments.
///
/// Supports glob patterns (e.g., `*`, `./skills/*`).
/// If no paths are given, uses the current directory.
/// Validates that all resolved paths are skill directories.
///
/// # Errors
/// Returns an error if:
/// - Glob pattern is invalid
/// - No skill directories found
/// - Any resolved path is not a valid skill directory
pub fn resolve_skill_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, SkillDirError> {
    if paths.is_empty() {
        let cwd = std::env::current_dir()?;
        if !is_skill_dir(&cwd) {
            return Err(SkillDirError::SkillMdNotFound(cwd));
        }
        return Ok(vec![cwd]);
    }

    let mut resolved = Vec::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        if contains_glob_chars(&path_str) {
            // Expand glob pattern
            let mut found_any = false;
            for p in glob(&path_str)?.flatten() {
                if is_skill_dir(&p) {
                    resolved.push(p);
                    found_any = true;
                }
            }
            if !found_any {
                return Err(SkillDirError::NoSkillDirsFound(path_str.into_owned()));
            }
        } else {
            // Direct path - validate it exists and is a skill directory
            if !path.exists() {
                return Err(SkillDirError::DirectoryNotFound(path.clone()));
            }
            if !is_skill_dir(path) {
                return Err(SkillDirError::SkillMdNotFound(path.clone()));
            }
            resolved.push(path.clone());
        }
    }

    // Sort for deterministic order
    resolved.sort();
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn create_skill_dir(dir: &Path, name: &str) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(dir)?;
        let skill_md = format!(
            r"---
name: {name}
---
# {name}
A test skill.
"
        );
        std::fs::write(dir.join("SKILL.md"), skill_md)?;
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_success() -> TestResult {
        let temp = TempDir::new()?;
        create_skill_dir(temp.path(), "test-skill")?;

        let result = detect_skill_dir(temp.path())?;
        assert_eq!(result.name, "test-skill");
        assert_eq!(result.path, temp.path());
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_not_found() {
        let result = detect_skill_dir(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(SkillDirError::DirectoryNotFound(_))));
    }

    #[test]
    fn test_detect_skill_dir_no_skill_md() -> TestResult {
        let temp = TempDir::new()?;
        let result = detect_skill_dir(temp.path());
        assert!(matches!(result, Err(SkillDirError::SkillMdNotFound(_))));
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_missing_frontmatter() -> TestResult {
        let temp = TempDir::new()?;
        std::fs::write(temp.path().join("SKILL.md"), "# No frontmatter")?;
        let result = detect_skill_dir(temp.path());
        assert!(matches!(result, Err(SkillDirError::MissingFrontmatter(_))));
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_unclosed_frontmatter() -> TestResult {
        let temp = TempDir::new()?;
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\nname: test\n# No closing",
        )?;
        let result = detect_skill_dir(temp.path());
        assert!(matches!(result, Err(SkillDirError::UnclosedFrontmatter(_))));
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_missing_name() -> TestResult {
        let temp = TempDir::new()?;
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\ndescription: test\n---\n# Test",
        )?;
        let result = detect_skill_dir(temp.path());
        assert!(matches!(result, Err(SkillDirError::MissingName(_))));
        Ok(())
    }

    #[test]
    fn test_detect_skill_dir_empty_name() -> TestResult {
        let temp = TempDir::new()?;
        std::fs::write(temp.path().join("SKILL.md"), "---\nname: \n---\n# Test")?;
        let result = detect_skill_dir(temp.path());
        assert!(matches!(result, Err(SkillDirError::MissingName(_))));
        Ok(())
    }

    #[test]
    fn test_detect_skill_dirs_success() -> TestResult {
        let temp = TempDir::new()?;
        let skill_a = temp.path().join("skill-a");
        let skill_b = temp.path().join("skill-b");

        create_skill_dir(&skill_a, "skill-a")?;
        create_skill_dir(&skill_b, "skill-b")?;

        let result = detect_skill_dirs(&[skill_a, skill_b])?;
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "skill-a");
        assert_eq!(result[1].name, "skill-b");
        Ok(())
    }

    #[test]
    fn test_detect_skill_dirs_duplicate_name() -> TestResult {
        let temp = TempDir::new()?;
        let skill_a = temp.path().join("skill-a");
        let skill_b = temp.path().join("skill-b");

        create_skill_dir(&skill_a, "same-name")?;
        create_skill_dir(&skill_b, "same-name")?;

        let result = detect_skill_dirs(&[skill_a, skill_b]);
        assert!(matches!(result, Err(SkillDirError::DuplicateName { .. })));
        Ok(())
    }

    #[test]
    fn test_is_skill_dir() -> TestResult {
        let temp = TempDir::new()?;
        assert!(!is_skill_dir(temp.path()));

        create_skill_dir(temp.path(), "test")?;
        assert!(is_skill_dir(temp.path()));
        Ok(())
    }

    #[test]
    fn test_resolve_skill_paths_empty_uses_cwd() -> TestResult {
        let temp = TempDir::new()?;
        create_skill_dir(temp.path(), "test")?;

        // Change to temp dir
        let original = std::env::current_dir()?;
        std::env::set_current_dir(temp.path())?;

        let result = resolve_skill_paths(&[]);
        std::env::set_current_dir(original)?;

        assert!(result.is_ok());
        let paths = result?;
        assert_eq!(paths.len(), 1);
        Ok(())
    }

    #[test]
    fn test_resolve_skill_paths_validates() -> TestResult {
        let temp = TempDir::new()?;
        // Don't create SKILL.md

        let result = resolve_skill_paths(&[temp.path().to_path_buf()]);
        assert!(matches!(result, Err(SkillDirError::SkillMdNotFound(_))));
        Ok(())
    }
}
