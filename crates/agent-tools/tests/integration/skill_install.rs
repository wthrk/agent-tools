//! Skill install command tests

use super::common::TestEnv;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_skill_install() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Verify skill was installed
    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists(), "Skill directory not created");
    assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md not copied");
    assert!(
        skill_dir.join(".skill-meta.yaml").exists(),
        ".skill-meta.yaml not created"
    );
}

#[test]
#[allow(deprecated)]
fn test_skill_install_from_subdir() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Create subdirectory
    let subdir = env.project.join("src/components");
    fs::create_dir_all(&subdir).unwrap();

    // Run from subdirectory
    let mut cmd = Command::cargo_bin("agent-tools").unwrap();
    cmd.env("AGENT_TOOLS_HOME", &env.agent_tools_home);
    cmd.env("CLAUDE_HOME", &env.claude_home);
    cmd.current_dir(&subdir);
    cmd.args(["skill", "install", "test-skill"]);
    cmd.assert().success();

    // Verify skill was installed to project root
    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists(), "Skill not installed to project root");
}

#[test]
fn test_skill_install_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "install", "nonexistent-skill"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)not found|does not exist|error").unwrap());
}
