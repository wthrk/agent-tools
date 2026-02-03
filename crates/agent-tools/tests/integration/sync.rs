//! Sync command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_sync_settings() {
    let env = TestEnv::new();
    env.create_settings(r#"{"test": true}"#);
    env.create_config(
        r#"config_version: 1
auto_deploy_skills: []
manage_settings: true
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check settings.json is linked
    let settings_path = env.claude_home.join("settings.json");
    assert!(
        settings_path.is_symlink(),
        "settings.json should be a symlink"
    );

    let link_target = fs::read_link(&settings_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("settings.json"),
        "settings.json should link to agent-tools home"
    );
}

#[test]
fn test_sync_skills() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_skill("sample-skill-b");
    env.create_config(
        r#"config_version: 1
auto_deploy_skills:
  - sample-skill-a
manage_settings: false
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check sample-skill-a is linked
    let skill_a_path = env.claude_home.join("skills/sample-skill-a");
    assert!(
        skill_a_path.is_symlink(),
        "sample-skill-a should be a symlink"
    );

    let link_target = fs::read_link(&skill_a_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("skills/sample-skill-a"),
        "sample-skill-a should link to agent-tools home"
    );

    // Check sample-skill-b is NOT linked (not in config)
    let skill_b_path = env.claude_home.join("skills/sample-skill-b");
    assert!(
        !skill_b_path.exists(),
        "sample-skill-b should not be linked"
    );
}

#[test]
fn test_sync_dry_run() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_settings(r#"{"test": true}"#);
    env.create_config(
        r#"config_version: 1
auto_deploy_skills:
  - sample-skill-a
manage_settings: true
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["sync", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    // Check that no symlinks were created
    let settings_path = env.claude_home.join("settings.json");
    assert!(
        !settings_path.is_symlink(),
        "settings.json should not be linked in dry-run mode"
    );

    let skill_a_path = env.claude_home.join("skills/sample-skill-a");
    assert!(
        !skill_a_path.is_symlink(),
        "sample-skill-a should not be linked in dry-run mode"
    );
}

#[test]
fn test_sync_claude_md() {
    let env = TestEnv::new();

    // Create global/CLAUDE.md
    let global_dir = env.agent_tools_home.join("global");
    fs::create_dir_all(&global_dir).unwrap();
    fs::write(global_dir.join("CLAUDE.md"), "# Global CLAUDE.md\n").unwrap();

    env.create_config(
        r#"config_version: 1
auto_deploy_skills: []
manage_settings: false
manage_plugins: false
manage_claude_md: true
manage_hooks: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check CLAUDE.md is linked
    let claude_md_path = env.claude_home.join("CLAUDE.md");
    assert!(claude_md_path.is_symlink(), "CLAUDE.md should be a symlink");

    let link_target = fs::read_link(&claude_md_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("global/CLAUDE.md"),
        "CLAUDE.md should link to agent-tools global/CLAUDE.md"
    );
}

#[test]
fn test_sync_hooks() {
    let env = TestEnv::new();

    // Create global/hooks directory with files
    let hooks_dir = env.agent_tools_home.join("global/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(hooks_dir.join("test-hook.sh"), "#!/bin/bash\nexit 0\n").unwrap();

    env.create_config(
        r#"config_version: 1
auto_deploy_skills: []
manage_settings: false
manage_plugins: false
manage_claude_md: false
manage_hooks: true
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check hooks is linked
    let hooks_path = env.claude_home.join("hooks");
    assert!(hooks_path.is_symlink(), "hooks should be a symlink");

    let link_target = fs::read_link(&hooks_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("global/hooks"),
        "hooks should link to agent-tools global/hooks"
    );
}

#[test]
fn test_settings_json_hook_paths_match_sync_target() {
    // This test verifies that settings.json hook paths ($HOME/.claude/hooks/)
    // are consistent with sync.rs which creates ~/.claude/hooks/ symlink

    let settings_content = include_str!("../../../../settings.json");

    // All hook commands should reference $HOME/.claude/hooks/
    // NOT $HOME/.agent-tools/hooks/ or $HOME/.agent-tools/global/hooks/
    assert!(
        !settings_content.contains("$HOME/.agent-tools/hooks/"),
        "settings.json should not reference $HOME/.agent-tools/hooks/ - use $HOME/.claude/hooks/ instead"
    );
    assert!(
        !settings_content.contains("$HOME/.agent-tools/global/hooks/"),
        "settings.json should not reference $HOME/.agent-tools/global/hooks/ - use $HOME/.claude/hooks/ instead"
    );

    // Verify it uses the correct path
    assert!(
        settings_content.contains("$HOME/.claude/hooks/"),
        "settings.json should reference $HOME/.claude/hooks/"
    );
}
