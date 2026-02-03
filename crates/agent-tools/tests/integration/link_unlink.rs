//! Link/unlink command tests

use super::common::TestEnv;
use std::fs;

#[test]
fn test_link_unlink() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Test link command
    env.cmd()
        .args(["link", "sample-skill-a"])
        .assert()
        .success();

    let skill_path = env.claude_home.join("skills/sample-skill-a");
    assert!(skill_path.is_symlink(), "sample-skill-a should be linked");

    // Test unlink command
    env.cmd()
        .args(["unlink", "sample-skill-a"])
        .assert()
        .success();

    assert!(!skill_path.exists(), "sample-skill-a should be unlinked");
}

#[test]
fn test_link_skill_not_found() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["link", "nonexistent-skill"])
        .assert()
        .failure();
}

#[test]
fn test_unlink_not_linked() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["unlink", "nonexistent-skill"])
        .assert()
        .failure();
}
