//! Skill new command tests

use super::common::TestEnv;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
#[allow(deprecated)]
fn test_skill_new_help() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "new", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create a new skill"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--no-auto-deploy"));
}

#[test]
fn test_skill_new_creates_skill() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill without adding to config
    env.cmd()
        .args(["skill", "new", "test-skill", "--no-auto-deploy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created skill"));

    // Verify skill directory and SKILL.md were created
    let skill_dir = env.agent_tools_home.join("skills/test-skill");
    assert!(skill_dir.exists(), "Skill directory should exist");
    assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md should exist");

    // Verify SKILL.md contains correct content
    let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(content.contains("name: test-skill"));
}

#[test]
fn test_skill_new_adds_to_config_and_links() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill with auto-deploy (using --yes to skip prompt)
    env.cmd()
        .args(["skill", "new", "my-skill", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto_deploy_skills"));

    // Verify skill directory was created
    let skill_dir = env.agent_tools_home.join("skills/my-skill");
    assert!(skill_dir.exists());

    // Verify config was updated
    let config_path = env.agent_tools_home.join("config.yaml");
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("my-skill"),
        "Config should contain the skill name"
    );

    // Verify symlink was created
    let link_path = env.claude_home.join("skills/my-skill");
    assert!(link_path.is_symlink(), "Symlink should be created");
}

#[test]
fn test_skill_new_already_exists() {
    let env = TestEnv::new();
    env.create_skill("existing-skill");

    // Try to create skill that already exists
    env.cmd()
        .args(["skill", "new", "existing-skill", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_skill_new_invalid_name_path_traversal() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "../escape", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_slash() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "foo/bar", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_dot() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "skill.name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_special_chars() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "skill@name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_starts_with_hyphen() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "-invalid", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)cannot start|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_with_underscore() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Underscores are not allowed in skill names (only lowercase, numbers, hyphens)
    env.cmd()
        .args(["skill", "new", "my_skill_name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("lowercase"));
}

#[test]
fn test_skill_new_link_already_exists() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create a conflicting file at the link target
    fs::write(env.claude_home.join("skills/conflict-skill"), "existing").unwrap();

    // Try to create skill with auto-deploy (should fail on link creation)
    env.cmd()
        .args(["skill", "new", "conflict-skill", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)exists|error").unwrap());
}

#[test]
fn test_skill_new_creates_config_if_missing() {
    let env = TestEnv::new();

    // Remove config if it exists
    let config_path = env.agent_tools_home.join("config.yaml");
    if config_path.exists() {
        fs::remove_file(&config_path).unwrap();
    }

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill with auto-deploy
    env.cmd()
        .args(["skill", "new", "auto-config-skill", "--yes"])
        .assert()
        .success();

    // Verify config was created
    assert!(config_path.exists(), "Config should be created");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("auto-config-skill"));
    assert!(content.contains("config_version: 1"));
}

#[test]
fn test_skill_new_invalid_name_too_long() {
    let env = TestEnv::new();

    // Create a name that's 65 characters (exceeds 64 limit)
    let long_name = "a".repeat(65);

    env.cmd()
        .args(["skill", "new", &long_name, "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)cannot exceed|too long|error").unwrap());
}
