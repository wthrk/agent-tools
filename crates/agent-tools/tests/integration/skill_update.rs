//! Skill update command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_skill_update_up_to_date() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Update should report up to date
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date"));
}

#[test]
fn test_skill_update_source_changed() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify source skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Updated content\n").unwrap();

    // Update should detect and apply changes
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify update was applied
    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    let content = fs::read_to_string(installed_skill).unwrap();
    assert!(content.contains("Updated content"));
}

#[test]
fn test_skill_update_local_changes_conflict() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify both source and installed skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Source updated\n").unwrap();

    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Local changes\n").unwrap();

    // Update should report conflict
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local changes"));
}

#[test]
fn test_skill_update_force() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify both source and installed skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Source updated\n").unwrap();

    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Local changes\n").unwrap();

    // Force update should overwrite local changes
    env.cmd()
        .args(["skill", "update", "test-skill", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify source content was applied
    let content = fs::read_to_string(installed_skill).unwrap();
    assert!(content.contains("Source updated"));
}

#[test]
fn test_skill_update_all() {
    let env = TestEnv::new();
    env.create_skill("skill-a");
    env.create_skill("skill-b");

    // Install skills
    env.cmd()
        .args(["skill", "install", "skill-a"])
        .assert()
        .success();
    env.cmd()
        .args(["skill", "install", "skill-b"])
        .assert()
        .success();

    // Update all should process both
    env.cmd()
        .args(["skill", "update", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-a"))
        .stdout(predicate::str::contains("skill-b"));
}

#[test]
fn test_skill_update_not_installed() {
    let env = TestEnv::new();
    env.create_skill("test-skill");
    env.create_skill("other-skill");

    // Install a different skill so .claude/skills exists
    env.cmd()
        .args(["skill", "install", "other-skill"])
        .assert()
        .success();

    // Update uninstalled skill should report not installed
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not installed"));
}
