//! Skill remove command tests

use super::common::TestEnv;
use std::fs;

#[test]
fn test_skill_remove() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists());

    // Remove skill
    env.cmd()
        .args(["skill", "remove", "test-skill"])
        .assert()
        .success();

    assert!(!skill_dir.exists(), "Skill should be removed");
}

#[test]
fn test_skill_remove_not_installed() {
    let env = TestEnv::new();

    // Ensure project skills dir exists
    fs::create_dir_all(env.project.join(".claude/skills")).unwrap();

    env.cmd()
        .args(["skill", "remove", "nonexistent-skill"])
        .assert()
        .failure();
}
