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

/// Check for uncommitted changes using jj diff (empty output means clean)
pub fn check_jj_clean(path: &Path) -> Result<()> {
    let diff = Command::new("jj")
        .args(["diff"])
        .current_dir(path)
        .output()
        .context("Failed to run jj diff")?;

    if !diff.status.success() {
        let stderr = String::from_utf8_lossy(&diff.stderr);
        bail!("jj diff failed:\n{}", stderr);
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

/// Fetch from remote without modifying working tree
pub fn fetch_remote(path: &Path, vcs: Vcs) -> Result<()> {
    match vcs {
        Vcs::Jj => {
            let output = Command::new("jj")
                .args(["git", "fetch"])
                .current_dir(path)
                .output()
                .context("Failed to run jj git fetch")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("jj git fetch failed:\n{}", stderr);
            }
        }
        Vcs::Git => {
            let output = Command::new("git")
                .args(["fetch", "origin"])
                .current_dir(path)
                .output()
                .context("Failed to run git fetch origin")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("git fetch origin failed:\n{}", stderr);
            }
        }
    }
    Ok(())
}

/// Check if remote has new commits (call after fetch_remote)
pub fn has_remote_updates(path: &Path, vcs: Vcs) -> Result<bool> {
    match vcs {
        Vcs::Jj => {
            let local = Command::new("jj")
                .args(["log", "-r", "main", "--no-graph", "-T", "commit_id"])
                .current_dir(path)
                .output()
                .context("Failed to get local main commit")?;
            let remote = Command::new("jj")
                .args([
                    "log",
                    "-r",
                    "main@origin",
                    "--no-graph",
                    "-T",
                    "commit_id",
                ])
                .current_dir(path)
                .output()
                .context("Failed to get remote main commit")?;

            let local_id = String::from_utf8_lossy(&local.stdout).trim().to_string();
            let remote_id = String::from_utf8_lossy(&remote.stdout).trim().to_string();

            Ok(!local_id.is_empty() && !remote_id.is_empty() && local_id != remote_id)
        }
        Vcs::Git => {
            let local = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(path)
                .output()
                .context("Failed to get local HEAD")?;
            let remote = Command::new("git")
                .args(["rev-parse", "origin/main"])
                .current_dir(path)
                .output()
                .context("Failed to get origin/main")?;

            let local_id = String::from_utf8_lossy(&local.stdout).trim().to_string();
            let remote_id = String::from_utf8_lossy(&remote.stdout).trim().to_string();

            Ok(!local_id.is_empty() && !remote_id.is_empty() && local_id != remote_id)
        }
    }
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
