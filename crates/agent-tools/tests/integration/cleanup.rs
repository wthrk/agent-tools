//! Cleanup command tests

use super::common::TestEnv;
use std::fs;

#[test]
fn test_cleanup() {
    let env = TestEnv::new();

    // Create old backup directories
    let backups_dir = env.agent_tools_home.join("backups");
    fs::create_dir_all(&backups_dir).unwrap();
    fs::create_dir_all(backups_dir.join("old-backup-dir")).unwrap();
    fs::write(backups_dir.join("old-backup-file.txt"), "backup").unwrap();

    // Cleanup should succeed
    env.cmd().args(["cleanup"]).assert().success();

    // Backups should be cleaned
    let entries: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.is_empty(), "Backups should be cleaned up");
}

#[test]
fn test_cleanup_no_backups() {
    let env = TestEnv::new();

    // Cleanup with no backup directory should succeed
    env.cmd().args(["cleanup"]).assert().success();
}
