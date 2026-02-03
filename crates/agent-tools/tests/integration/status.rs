//! Status command tests

use super::common::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_status() {
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

    // Ensure directories exist
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Run sync first
    env.cmd().args(["sync"]).assert().success();

    // Status should show linked skills
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample-skill-a"))
        .stdout(predicate::str::contains("symlink"));
}

#[test]
fn test_status_no_config() {
    let env = TestEnv::new();

    // Status without config should work
    env.cmd().args(["status"]).assert().success();
}
