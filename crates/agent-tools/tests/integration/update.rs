//! Update command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_update_no_git_repo() {
    let env = TestEnv::new();

    // Without git repo, update should fail
    env.cmd()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)git|not found|error").unwrap());
}

#[test]
fn test_update_with_uncommitted_changes() {
    let env = TestEnv::new();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();

    // Create uncommitted file
    fs::write(env.agent_tools_home.join("uncommitted.txt"), "test").unwrap();

    // With uncommitted changes, update should fail
    env.cmd()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)uncommitted|changes").unwrap());
}
