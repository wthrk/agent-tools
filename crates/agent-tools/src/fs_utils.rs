//! File system utilities for skill-tools

use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Recursively copy a directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Copy directory contents (files only, not the directory itself)
pub fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Calculate a hash for a directory's contents
pub fn calculate_tree_hash(path: &Path) -> Result<String> {
    let mut hasher = DefaultHasher::new();
    hash_dir(path, &mut hasher, None)?;
    Ok(format!("{:016x}", hasher.finish()))
}

/// Calculate a hash for a directory's contents, excluding specific files
pub fn calculate_tree_hash_excluding(path: &Path, exclude: &[&str]) -> Result<String> {
    let mut hasher = DefaultHasher::new();
    hash_dir(path, &mut hasher, Some(exclude))?;
    Ok(format!("{:016x}", hasher.finish()))
}

fn hash_dir(dir: &Path, hasher: &mut DefaultHasher, exclude: Option<&[&str]>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .context("Failed to read directory")?
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(exclude_list) = exclude {
                let name = e.file_name();
                !exclude_list.iter().any(|ex| name == *ex)
            } else {
                true
            }
        })
        .collect();

    // Sort for consistent ordering
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        name.hash(hasher);

        if path.is_dir() {
            hash_dir(&path, hasher, exclude)?;
        } else if path.is_file() {
            let content = fs::read(&path).context("Failed to read file")?;
            content.hash(hasher);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_copy_dir_recursive() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        // Create source structure
        fs::write(src.path().join("file.txt"), "content").unwrap();
        fs::create_dir(src.path().join("subdir")).unwrap();
        fs::write(src.path().join("subdir/nested.txt"), "nested").unwrap();

        let dst_path = dst.path().join("copied");
        copy_dir_recursive(src.path(), &dst_path).unwrap();

        assert!(dst_path.join("file.txt").exists());
        assert!(dst_path.join("subdir/nested.txt").exists());
    }

    #[test]
    fn test_calculate_tree_hash_consistent() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let hash1 = calculate_tree_hash(dir.path()).unwrap();
        let hash2 = calculate_tree_hash(dir.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_calculate_tree_hash_changes_on_content() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "content1").unwrap();
        let hash1 = calculate_tree_hash(dir.path()).unwrap();

        fs::write(dir.path().join("file.txt"), "content2").unwrap();
        let hash2 = calculate_tree_hash(dir.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_tree_hash_excluding() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();
        fs::write(dir.path().join(".skill-meta.yaml"), "meta").unwrap();

        let hash_with = calculate_tree_hash(dir.path()).unwrap();
        let hash_without =
            calculate_tree_hash_excluding(dir.path(), &[".skill-meta.yaml"]).unwrap();

        assert_ne!(hash_with, hash_without);

        // Hash excluding meta should be same as hash of just file.txt
        let dir2 = TempDir::new().unwrap();
        fs::write(dir2.path().join("file.txt"), "content").unwrap();
        let hash_just_file = calculate_tree_hash(dir2.path()).unwrap();

        assert_eq!(hash_without, hash_just_file);
    }
}
