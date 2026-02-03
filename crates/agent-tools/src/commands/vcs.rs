use std::path::Path;

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
    if path.join(".git").exists() {
        return Some(Vcs::Git);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_vcs_jj_only() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".jj")).unwrap();

        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Jj));
    }

    #[test]
    fn test_detect_vcs_git_only() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Git));
    }

    #[test]
    fn test_detect_vcs_both_prefers_jj() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".jj")).unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        // jj should be preferred when both exist
        assert_eq!(detect_vcs(temp.path()), Some(Vcs::Jj));
    }

    #[test]
    fn test_detect_vcs_none() {
        let temp = TempDir::new().unwrap();

        assert_eq!(detect_vcs(temp.path()), None);
    }
}
