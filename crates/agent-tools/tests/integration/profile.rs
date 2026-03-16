//! Profile command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn test_profiles_lists_union_of_targets() {
    let env = TestEnv::new();
    env.create_claude_profile(
        "default",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: false\nmanage_plugins: false\n",
        "{}",
    );
    env.create_codex_profile("default", "model = \"gpt-5.3-codex\"\n");
    env.create_codex_profile("local", "model = \"gpt-5.3-codex\"\n");

    env.cmd()
        .args(["profiles"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("local"));
}

#[test]
fn test_use_profile_applies_active_links_and_state() {
    let env = TestEnv::new();
    fs::write(
        env.claude_home.join("settings.json"),
        "{\"default\":true}\n",
    )
    .unwrap();
    fs::write(
        env.codex_home.join("config.toml"),
        "model = \"gpt-5.3-codex\"\n",
    )
    .unwrap();

    env.create_claude_profile(
        "default",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: true\nmanage_plugins: false\nmanage_claude_md: false\nmanage_hooks: false\nmanage_codex_config: false\n",
        "{\"test\": true}\n",
    );
    env.create_codex_profile("default", "model = \"gpt-5.3-codex\"\n");

    env.cmd().args(["use", "default"]).assert().success();

    assert!(env.claude_home.is_symlink());
    assert!(env.codex_home.is_symlink());
    let claude_link = fs::read_link(&env.claude_home).unwrap();
    let codex_link = fs::read_link(&env.codex_home).unwrap();
    assert_eq!(
        claude_link,
        env.agent_tools_home.join(".local/profiles/claude/default")
    );
    assert_eq!(
        codex_link,
        env.agent_tools_home.join(".local/profiles/codex/default")
    );
    assert!(
        Path::new(&env.agent_tools_home.join("backups")).exists(),
        "backups dir should exist"
    );

    let current =
        fs::read_to_string(env.agent_tools_home.join(".local/state/current.json")).unwrap();
    assert!(current.contains("\"claude\": \"default\""));
    assert!(current.contains("\"codex\": \"default\""));
}

#[test]
fn test_current_shows_state() {
    let env = TestEnv::new();
    env.create_claude_profile(
        "default",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: false\nmanage_plugins: false\n",
        "{}",
    );
    env.cmd().args(["use", "default"]).assert().success();

    env.cmd()
        .args(["current"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current:"))
        .stdout(predicate::str::contains("claude=default"));
}

#[test]
fn test_switch_captures_existing_homes_and_uses_side_namespaces() {
    let env = TestEnv::new();
    fs::write(
        env.claude_home.join("settings.json"),
        "{\"theme\":\"orig\"}\n",
    )
    .unwrap();
    fs::write(env.codex_home.join("config.toml"), "model = \"orig\"\n").unwrap();

    env.create_claude_profile(
        "work",
        "config_version: 1\nauto_deploy_skills: []\nmanage_settings: true\nmanage_plugins: false\nmanage_claude_md: false\nmanage_hooks: false\nmanage_codex_config: false\n",
        "{\"theme\":\"work\"}\n",
    );
    env.create_codex_profile("work", "model = \"gpt-5.3-codex\"\n");

    env.cmd().args(["use", "work"]).assert().success();

    assert_eq!(
        fs::read_to_string(
            env.agent_tools_home
                .join(".local/profiles/claude/default/settings.json")
        )
        .unwrap(),
        "{\"theme\":\"orig\"}\n"
    );
    assert_eq!(
        fs::read_to_string(
            env.agent_tools_home
                .join(".local/profiles/codex/default/config.toml")
        )
        .unwrap(),
        "model = \"orig\"\n"
    );
    assert_eq!(
        fs::read_link(&env.claude_home).unwrap(),
        env.agent_tools_home.join(".local/profiles/claude/work")
    );
    assert_eq!(
        fs::read_link(&env.codex_home).unwrap(),
        env.agent_tools_home.join(".local/profiles/codex/work")
    );

    env.cmd().args(["use", "default"]).assert().success();

    assert_eq!(
        fs::read_link(&env.claude_home).unwrap(),
        env.agent_tools_home.join(".local/profiles/claude/default")
    );
    assert_eq!(
        fs::read_link(&env.codex_home).unwrap(),
        env.agent_tools_home.join(".local/profiles/codex/default")
    );
}
