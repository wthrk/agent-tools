use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Find the project root directory
/// Priority:
/// 1. --project option (if provided)
/// 2. Current directory if it has .claude/
/// 3. Walk up to find .claude/ (preferred over .git/)
/// 4. Error if not found
pub fn find_project_root(explicit_project: Option<&str>) -> Result<PathBuf> {
    // 1. Explicit project path
    if let Some(project) = explicit_project {
        let path = PathBuf::from(project);
        if !path.exists() {
            bail!("Specified project path does not exist: {}", project);
        }
        return Ok(path);
    }

    // 2 & 3. Walk up from current directory
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    find_project_root_from(&current_dir)
}

/// Find project root starting from a given path
pub fn find_project_root_from(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        // Check for .claude/ directory (preferred)
        if current.join(".claude").is_dir() {
            return Ok(current);
        }

        // Move to parent
        if let Some(parent) = current.parent() {
            // Also check if we hit a .git boundary (but continue looking for .claude)
            if current.join(".git").exists() && !current.join(".claude").is_dir() {
                // Found .git but no .claude - create .claude here
                return Ok(current);
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    bail!(
        "Could not find project root (no .claude/ or .git/ found)\n\
         Started from: {}",
        start.display()
    )
}

/// Get the project's skills directory
pub fn project_skills_dir(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_project_root_with_claude_dir() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("my-project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();

        let result = find_project_root_from(&project);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), project);
    }

    #[test]
    fn test_find_project_root_from_subdir() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("my-project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::fs::create_dir_all(project.join("src/components")).unwrap();

        let subdir = project.join("src/components");
        let result = find_project_root_from(&subdir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), project);
    }

    #[test]
    fn test_find_project_root_not_found() {
        let temp = TempDir::new().unwrap();
        let result = find_project_root_from(temp.path());
        assert!(result.is_err());
    }
}
