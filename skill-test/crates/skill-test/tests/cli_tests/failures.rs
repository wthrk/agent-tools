//! Failure and error handling tests (require Claude CLI).
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;
use std::fs;

// =============================================================================
// Failure cases - verify proper error output
// =============================================================================

#[test]
fn test_failure_contract_assertion_shows_id() {
    // When a contract assertion fails, the assertion ID should be in failures array
    let skill_dir = fixtures_dir().join("failure-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "json",
            "--filter", "failure-contract-001",
            "--threshold", "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success() // threshold=0 so test passes even with failures
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // Should show the failed assertion ID in failures array
    assert!(
        output_str.contains("should-fail-no-error"),
        "Failed assertion ID should be in output. Got: {output_str}"
    );
    assert!(
        output_str.contains("\"failures\""),
        "failures array should be present. Got: {output_str}"
    );
}

#[test]
fn test_failure_threshold_not_met_exit_code() {
    // When pass rate doesn't meet threshold, exit code should be 1
    let skill_dir = fixtures_dir().join("failure-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "failure-contract-001",
            "--threshold",
            "100", // Require 100% pass rate
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .code(1); // Should fail with exit code 1
}

#[test]
fn test_failure_threshold_message_in_stderr() {
    // When threshold not met, error message should be in stderr (table format)
    // Note: In JSON format, stderr is intentionally empty; all output goes to stdout as JSON.
    let skill_dir = fixtures_dir().join("failure-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "table",
            "--filter", "failure-contract-001",
            "--threshold", "100",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        // Table format outputs "FAILED" and also "[ERRLOG]" in stderr
        .stderr(predicate::str::contains("[ERRLOG]"));
}

#[test]
fn test_failure_verdict_is_fail() {
    // When contract fails with threshold=100, verdict should be "Fail"
    let skill_dir = fixtures_dir().join("failure-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "failure-contract-001",
            "--threshold",
            "100",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // Verdict should be Fail because pass_rate (0%) < threshold (100%)
    assert!(
        output_str.contains("\"verdict\": \"Fail\"") || output_str.contains("\"verdict\":\"Fail\""),
        "verdict should be Fail. Got: {output_str}"
    );
    // Failures should contain the assertion ID
    assert!(
        output_str.contains("\"failures\":") && !output_str.contains("\"failures\": []"),
        "failures should not be empty. Got: {output_str}"
    );
}

// =============================================================================
// Error handling tests
// =============================================================================

#[test]
fn test_error_duplicate_assertion_id() {
    // When a contract has duplicate assertion IDs, the CLI should fail with an error
    let skill_dir = fixtures_dir().join("error-duplicate-id-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        // Error should mention duplicate assertion ID
        .stderr(predicate::str::contains("duplicate").or(predicate::str::contains("Duplicate")));
}

// =============================================================================
// Error log output tests
// =============================================================================

#[test]
fn test_error_log_created_on_failure() {
    // When a test fails, an error log should be created in .skill-test-logs/
    let skill_dir = fixtures_dir().join("failure-skill");
    let log_dir = skill_dir.join(".skill-test-logs");

    // Clean up any existing logs from previous test runs
    if log_dir.exists() {
        let _ = fs::remove_dir_all(&log_dir);
    }

    // Run a test that will fail (threshold=100 with a failing assertion)
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--filter",
            "failure-contract-001",
            "--threshold",
            "100",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    // Verify log directory was created
    assert!(
        log_dir.exists(),
        ".skill-test-logs directory should be created. Path: {}",
        log_dir.display()
    );

    // Verify at least one JSON log file was created
    let entries: Vec<_> = fs::read_dir(&log_dir)
        .expect("should be able to read log directory")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    assert!(
        !entries.is_empty(),
        "At least one .json log file should be created in .skill-test-logs/"
    );

    // Verify the log file contains valid JSON with expected structure
    let log_path = entries[0].path();
    let content = fs::read_to_string(&log_path).expect("should be able to read log file");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("log file should contain valid JSON");

    // Check expected fields exist (ExecutionReport schema)
    assert!(
        json.get("timestamp").is_some(),
        "Log should have timestamp. Got: {content}"
    );
    assert!(
        json.get("skills").is_some(),
        "Log should have skills array. Got: {content}"
    );
    assert!(
        json.get("summary").is_some(),
        "Log should have summary. Got: {content}"
    );
    // Check skill structure
    let skills = json["skills"]
        .as_array()
        .expect("skills should be an array");
    assert!(!skills.is_empty(), "skills array should not be empty");
    assert!(
        skills[0].get("skill_name").is_some(),
        "Skill should have skill_name. Got: {content}"
    );
    assert!(
        skills[0].get("tests").is_some(),
        "Skill should have tests array. Got: {content}"
    );

    // Clean up
    let _ = fs::remove_dir_all(&log_dir);
}

#[test]
fn test_error_log_message_in_stderr() {
    // When a test fails, [ERRLOG] message should appear in stderr
    let skill_dir = fixtures_dir().join("failure-skill");
    let log_dir = skill_dir.join(".skill-test-logs");

    // Clean up any existing logs
    if log_dir.exists() {
        let _ = fs::remove_dir_all(&log_dir);
    }

    // Run a failing test
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--filter",
            "failure-contract-001",
            "--threshold",
            "100",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        // [ERRLOG] message should appear in stderr (with or without ANSI codes)
        .stderr(predicate::str::contains("[ERRLOG]"));

    // Clean up
    let _ = fs::remove_dir_all(&log_dir);
}

#[test]
fn test_error_log_contains_detailed_results() {
    // The error log should contain detailed test results including assertion info
    let skill_dir = fixtures_dir().join("failure-skill");
    let log_dir = skill_dir.join(".skill-test-logs");

    // Clean up
    if log_dir.exists() {
        let _ = fs::remove_dir_all(&log_dir);
    }

    // Run a failing test
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--filter",
            "failure-contract-001",
            "--threshold",
            "100",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    // Read the log file
    let entries: Vec<_> = fs::read_dir(&log_dir)
        .expect("log directory should exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    let log_path = entries[0].path();
    let content = fs::read_to_string(&log_path).expect("should read log file");
    let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");

    // Verify skills[0].tests array contains detailed results (ExecutionReport schema)
    let skills = json["skills"]
        .as_array()
        .expect("skills should be an array");
    assert!(!skills.is_empty(), "skills array should not be empty");
    let tests = skills[0]["tests"]
        .as_array()
        .expect("tests should be an array");
    assert!(!tests.is_empty(), "tests array should not be empty");

    let test = &tests[0];
    // Check test has detailed fields
    assert!(test.get("name").is_some(), "test should have name");
    assert!(test.get("prompt").is_some(), "test should have prompt");
    assert!(
        test.get("iterations").is_some(),
        "test should have iterations array"
    );

    // Check iteration details
    let iterations = test["iterations"]
        .as_array()
        .expect("iterations should be array");
    assert!(!iterations.is_empty(), "iterations should not be empty");

    let iteration = &iterations[0];
    assert!(
        iteration.get("output").is_some(),
        "iteration should have output"
    );
    assert!(
        iteration.get("output_hash").is_some(),
        "iteration should have output_hash"
    );
    assert!(
        iteration.get("assertions").is_some(),
        "iteration should have assertions"
    );

    // Clean up
    let _ = fs::remove_dir_all(&log_dir);
}
