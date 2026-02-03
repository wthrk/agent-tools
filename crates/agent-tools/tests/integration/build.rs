//! Build command tests

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_build_help() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Build agent-tools"));
}

#[test]
#[allow(deprecated)]
fn test_build_no_agent_tools_home() {
    let home = TempDir::new().unwrap();

    // Point to non-existent agent-tools home
    let mut cmd = Command::cargo_bin("agent-tools").unwrap();
    cmd.env("AGENT_TOOLS_HOME", home.path().join("nonexistent"));
    cmd.args(["build"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)not found|error").unwrap());
}
