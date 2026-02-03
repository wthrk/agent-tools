use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// Detected version control system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vcs {
    Jj,
    Git,
}

/// Detect the version control system used in the given path.
/// Prefers jj over git (since jj with git backend has both .jj and .git).
pub fn detect_vcs(path: &Path) -> Option<Vcs> {
    // .jj is prioritized (jj+git backend has both)
    if path.join(".jj").is_dir() {
        return Some(Vcs::Jj);
    }
    // .git can be a file (worktree/submodule) or directory, so use exists()
    if path.join(".git").exists() {
        return Some(Vcs::Git);
    }
    None
}

/// Check for uncommitted changes using jj diff --stat (machine-parseable, locale-independent)
pub fn check_jj_clean(path: &Path) -> Result<()> {
    let diff = Command::new("jj")
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .context("Failed to run jj diff --stat")?;

    if !diff.status.success() {
        let stderr = String::from_utf8_lossy(&diff.stderr);
        bail!("jj diff --stat failed:\n{}", stderr);
    }

    let diff_output = String::from_utf8_lossy(&diff.stdout);
    if !diff_output.trim().is_empty() {
        bail!(
            "Uncommitted changes detected in {}\n\
             Please commit or abandon changes before proceeding.",
            path.display()
        );
    }

    Ok(())
}

/// Check for uncommitted changes using git status --porcelain
pub fn check_git_clean(path: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .context("Failed to run git status")?;

    if !status.status.success() {
        let stdout = String::from_utf8_lossy(&status.stdout);
        let stderr = String::from_utf8_lossy(&status.stderr);
        bail!(
            "Failed to check git status in {}:\n{}{}\nIs this a git repository?",
            path.display(),
            stdout,
            stderr
        );
    }

    let status_output = String::from_utf8_lossy(&status.stdout);
    if !status_output.trim().is_empty() {
        bail!(
            "Uncommitted changes detected in {}\n\
             Please commit or stash changes before proceeding.",
            path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_vcs_jj_only() -> Result<()> {
        let temp = TempDir::new()?;
        fs::create_dir(temp.path().join(".jj"))?;

        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Jj));
        Ok(())
    }

    #[test]
    fn test_detect_vcs_git_only() -> Result<()> {
        let temp = TempDir::new()?;
        fs::create_dir(temp.path().join(".git"))?;

        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Git));
        Ok(())
    }

    #[test]
    fn test_detect_vcs_both_prefers_jj() -> Result<()> {
        let temp = TempDir::new()?;
        fs::create_dir(temp.path().join(".jj"))?;
        fs::create_dir(temp.path().join(".git"))?;

        // jj should be preferred when both exist
        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Jj));
        Ok(())
    }

    #[test]
    fn test_detect_vcs_none() -> Result<()> {
        let temp = TempDir::new()?;

        assert_eq!(detect_vcs(temp.path()), None);
        Ok(())
    }
}
