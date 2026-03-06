//! Profile command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn test_profiles_lists_union_of_targets() {
    let env = TestEnv::new();
    env.create_claude_profile(
        "runpod",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: false\nmanage_plugins: false\n",
        "{}",
    );
    env.create_codex_profile("runpod", "model = \"gpt-5.3-codex\"\n");
    env.create_codex_profile("local", "model = \"gpt-5.3-codex\"\n");

    env.cmd()
        .args(["profiles"])
        .assert()
        .success()
        .stdout(predicate::str::contains("runpod"))
        .stdout(predicate::str::contains("local"));
}

#[test]
fn test_use_profile_applies_active_links_and_state() {
    let env = TestEnv::new();
    env.create_claude_profile(
        "runpod",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: true\nmanage_plugins: false\nmanage_claude_md: false\nmanage_hooks: false\nmanage_codex_config: false\n",
        "{\"test\": true}\n",
    );
    env.create_codex_profile("runpod", "model = \"gpt-5.3-codex\"\n");

    env.cmd().args(["use", "runpod"]).assert().success();

    assert!(env.claude_home.is_symlink());
    assert!(env.codex_home.is_symlink());
    let claude_link = fs::read_link(&env.claude_home).unwrap();
    let codex_link = fs::read_link(&env.codex_home).unwrap();
    assert_eq!(
        claude_link,
        env.agent_tools_home.join(".local/profiles/runpod/claude")
    );
    assert_eq!(
        codex_link,
        env.agent_tools_home.join(".local/profiles/runpod/codex")
    );
    assert!(
        Path::new(&env.agent_tools_home.join("backups")).exists(),
        "backups dir should exist"
    );

    let current =
        fs::read_to_string(env.agent_tools_home.join(".local/state/current.json")).unwrap();
    assert!(current.contains("\"claude\": \"runpod\""));
    assert!(current.contains("\"codex\": \"runpod\""));
}

#[test]
fn test_current_shows_state() {
    let env = TestEnv::new();
    env.create_claude_profile(
        "runpod",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: false\nmanage_plugins: false\n",
        "{}",
    );
    env.cmd().args(["use", "runpod"]).assert().success();

    env.cmd()
        .args(["current"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current:"))
        .stdout(predicate::str::contains("claude=runpod"));
}
