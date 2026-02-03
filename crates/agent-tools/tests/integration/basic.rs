//! Basic CLI tests

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
#[allow(deprecated)]
fn test_version() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"agent-tools \d+\.\d+\.\d+").unwrap());
}

#[test]
#[allow(deprecated)]
fn test_help_shows_subcommands() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("build"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("link"))
        .stdout(predicate::str::contains("unlink"))
        .stdout(predicate::str::contains("skill"))
        .stdout(predicate::str::contains("cleanup"));
}
