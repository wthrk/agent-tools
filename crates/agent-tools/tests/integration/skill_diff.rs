//! Skill diff command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_skill_diff_no_changes() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Diff should show no changes
    env.cmd()
        .args(["skill", "diff", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences"));
}

#[test]
fn test_skill_diff_with_changes() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify installed skill
    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Modified locally\n").unwrap();

    // Diff should show changes
    env.cmd()
        .args(["skill", "diff", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SKILL.md"));
}
