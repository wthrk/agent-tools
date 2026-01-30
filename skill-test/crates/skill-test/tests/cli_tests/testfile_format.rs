//! TestFile format tests (scenarios + named assertions).
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

// =============================================================================
// TestFile Format: Basic
// =============================================================================

#[test]
fn test_testfile_format_named_assertions() {
    // Tests that named assertion references work correctly
    // threshold=0 because LLM response content is non-deterministic
    let skill_dir = fixtures_dir().join("testfile-format-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "greeting-test",
            "--threshold",
            "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify assertion structure is correct (names appear in output)
        .stdout(predicate::str::contains("has-hello"))
        .stdout(predicate::str::contains("has-world"));
}

#[test]
fn test_testfile_format_inline_assertion() {
    let skill_dir = fixtures_dir().join("testfile-format-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "inline-assertion-test",
            "--threshold",
            "0", // May fail due to exclamation mark
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

#[test]
fn test_testfile_format_shared_assertion() {
    // Test that same assertion can be used in both assertions and golden_assertions
    // threshold=0 because LLM response content is non-deterministic
    let skill_dir = fixtures_dir().join("testfile-format-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "shared-assertion-test",
            "--threshold",
            "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify same assertion appears in both assertions and golden_assertions
        .stdout(predicate::str::contains("\"assertions\""))
        .stdout(predicate::str::contains("\"golden_assertions\""))
        .stdout(predicate::str::contains("has-hello"));
}

#[test]
fn test_testfile_format_desc_in_output() {
    // Verify that test description appears in output
    // threshold=0 because LLM response content is non-deterministic
    let skill_dir = fixtures_dir().join("testfile-format-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--filter",
            "greeting-test",
            "--threshold",
            "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Test description from scenarios should appear in output
        .stdout(predicate::str::contains("Test greeting response"));
}
