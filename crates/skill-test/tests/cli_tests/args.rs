//! CLI argument tests (no Claude CLI required).

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

#[test]
fn test_arg_help() {
    skill_test()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Test runner for Claude Code skills",
        ));
}

#[test]
fn test_arg_version() {
    skill_test()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-test"));
}

#[test]
fn test_arg_invalid_format() {
    skill_test()
        .args([".", "--format", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("Invalid")));
}

#[test]
fn test_arg_valid_format_json() {
    // Should fail due to missing skill-tests dir, but format parsing should succeed
    skill_test()
        .args(["/nonexistent", "--format", "json"])
        .assert()
        .failure();
}

#[test]
fn test_arg_valid_format_table() {
    skill_test()
        .args(["/nonexistent", "--format", "table"])
        .assert()
        .failure();
}

#[test]
fn test_arg_hook_none() {
    // --hook none should be valid (default)
    skill_test()
        .args(["/nonexistent", "--hook", "none"])
        .assert()
        .failure(); // Fails due to missing skill-tests dir, but hook parsing succeeds
}

#[test]
fn test_arg_hook_simple() {
    // --hook simple should be valid
    skill_test()
        .args(["/nonexistent", "--hook", "simple"])
        .assert()
        .failure(); // Fails due to missing skill-tests dir, but hook parsing succeeds
}

#[test]
fn test_arg_hook_forced() {
    // --hook forced should be valid
    skill_test()
        .args(["/nonexistent", "--hook", "forced"])
        .assert()
        .failure(); // Fails due to missing skill-tests dir, but hook parsing succeeds
}

#[test]
fn test_arg_hook_custom_path() {
    // --hook with custom path should be valid
    skill_test()
        .args(["/nonexistent", "--hook", "/path/to/custom/hook.sh"])
        .assert()
        .failure(); // Fails due to missing skill-tests dir, but hook parsing succeeds
}

#[test]
fn test_error_yaml_parse_unknown_field() {
    // When YAML has unknown fields, the CLI should fail with a parse error
    let fixture_dir = fixtures_dir().join("error-yaml-parse");
    skill_test()
        .arg(&fixture_dir)
        .assert()
        .failure()
        // Error should mention unknown field
        .stderr(
            predicate::str::contains("unknown field").or(predicate::str::contains("unknown_field")),
        );
}
