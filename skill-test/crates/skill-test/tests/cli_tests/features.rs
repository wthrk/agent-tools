//! Feature tests (require Claude CLI).
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// =============================================================================
// Feature: golden_assertions
// =============================================================================

#[test]
fn test_feature_golden_assertions_tracked() {
    let skill_dir = fixtures_dir().join("golden-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("golden_passed") || output_str.contains("golden_failures"),
        "JSON output must track golden assertions"
    );
}

#[test]
fn test_feature_golden_assertions_dont_fail() {
    // Golden assertions should not cause test failure even if they don't pass
    let skill_dir = fixtures_dir().join("golden-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--threshold", "100"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// =============================================================================
// Feature: validation (inline assertions)
// =============================================================================

#[test]
fn test_feature_validation_llm_eval() {
    let skill_dir = fixtures_dir().join("validation-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--filter", "validation-llm-eval-001"])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();
}

#[test]
fn test_feature_validation_regex() {
    let skill_dir = fixtures_dir().join("validation-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--filter", "validation-regex-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

#[test]
fn test_feature_validation_multiple() {
    let skill_dir = fixtures_dir().join("validation-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--filter", "validation-multiple-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

#[test]
fn test_feature_validation_llm_eval_expect_fail() {
    // Tests llm_eval with expect: fail - expects the LLM to answer NO
    let skill_dir = fixtures_dir().join("validation-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--filter",
            "validation-llm-eval-fail-001",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();
}

#[test]
fn test_feature_validation_assertion_failure() {
    // Tests that validation assertion failure is properly reported
    let skill_dir = fixtures_dir().join("validation-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "json",
            "--filter", "validation-failure-001",
            "--threshold", "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success() // threshold=0
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // failures array should contain the validation assertion ID
    assert!(
        output_str.contains("validation-should-fail-contains"),
        "failures should contain validation assertion ID. Got: {output_str}"
    );
}

// =============================================================================
// Feature: iterations
// =============================================================================

#[test]
fn test_feature_iterations_single() -> TestResult {
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // JSON output is {skills: [...], summary: {...}}
    let result: serde_json::Value = serde_json::from_str(&output_str)?;
    // iterations is an array of DetailedIterationResult objects
    let iterations_count = result["skills"][0]["tests"][0]["iterations"]
        .as_array()
        .ok_or("iterations should be an array")?
        .len();
    assert_eq!(iterations_count, 1, "should have 1 iteration");
    Ok(())
}

#[test]
fn test_feature_iterations_multiple() -> TestResult {
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "3",
            "--format",
            "json",
            "--filter",
            "regex-present-001",
            "--threshold",
            "50",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // JSON output is {skills: [...], summary: {...}}
    let result: serde_json::Value = serde_json::from_str(&output_str)?;
    // iterations is an array of DetailedIterationResult objects
    let iterations_count = result["skills"][0]["tests"][0]["iterations"]
        .as_array()
        .ok_or("iterations should be an array")?
        .len();
    assert_eq!(iterations_count, 3, "should have 3 iterations");
    Ok(())
}

// =============================================================================
// Feature: threshold
// =============================================================================

#[test]
fn test_feature_threshold_pass() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--threshold",
            "100",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

#[test]
fn test_feature_threshold_in_output() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--threshold", "80", "--filter", "regex-present-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Output shows pass rate as percentage and verdict
        .stdout(predicate::str::contains("%)")
            .and(predicate::str::contains("PASS").or(predicate::str::contains("FAIL"))));
}

// =============================================================================
// Feature: strict
// =============================================================================

#[test]
fn test_feature_strict_pass() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--strict"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// =============================================================================
// Feature: filter
// =============================================================================

#[test]
fn test_feature_filter_single() {
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("regex-present-001"));
    assert!(!output_str.contains("regex-absent-001"));
}

#[test]
fn test_feature_filter_multiple() {
    // Tests that --filter matches multiple test cases when the pattern is a common substring
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "regex", // Should match all regex-* test cases
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // Should have results for all regex test cases
    assert!(
        output_str.contains("regex-present-001"),
        "Should contain regex-present-001. Got: {output_str}"
    );
    assert!(
        output_str.contains("regex-absent-001"),
        "Should contain regex-absent-001. Got: {output_str}"
    );
    assert!(
        output_str.contains("regex-complex-001"),
        "Should contain regex-complex-001. Got: {output_str}"
    );
}

// =============================================================================
// Feature: format
// =============================================================================

#[test]
fn test_feature_format_json() {
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // JSON output should be parseable
    assert!(output_str.starts_with("{") || output_str.starts_with("["));
}

#[test]
fn test_feature_format_json_includes_detailed_results() -> TestResult {
    // JSON output should always include detailed results (same as error log)
    let skill_dir = fixtures_dir().join("regex-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&output_str)?;

    // Check that detailed results are included
    let skills = json["skills"].as_array().ok_or("skills should be array")?;
    assert!(!skills.is_empty(), "skills array should not be empty");

    let skill = &skills[0];
    let tests = skill["tests"].as_array().ok_or("tests should be array")?;
    assert!(!tests.is_empty(), "tests array should not be empty");

    // Verify test result contains expected fields (ExecutionReport schema)
    let test = &tests[0];
    assert!(test.get("name").is_some(), "test should have name");
    assert!(test.get("prompt").is_some(), "test should have prompt");
    assert!(
        test.get("iterations").is_some(),
        "test should have iterations"
    );

    // Check iteration details
    let iterations = test["iterations"]
        .as_array()
        .ok_or("iterations should be array")?;
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
        iteration.get("called_tools").is_some(),
        "iteration should have called_tools"
    );
    assert!(
        iteration.get("assertions").is_some(),
        "iteration should have assertions"
    );

    Ok(())
}

#[test]
fn test_feature_format_table() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "table", "--filter", "regex-present-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Table format shows verdict as "PASS"/"FAIL" (uppercase)
        .stdout(predicate::str::contains("PASS").or(predicate::str::contains("FAIL")));
}

// Note: CSV format has been removed. See output_format::test_csv_format_removed for the test
// that verifies CSV format is no longer supported.

// =============================================================================
// Feature: verbose
// =============================================================================

#[test]
fn test_feature_verbose() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "table",
            "--filter", "regex-present-001",
            "--verbose",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verbose mode should show "Detailed Results" section
        .stdout(predicate::str::contains("Detailed Results"));
}

// =============================================================================
// Comprehensive test
// =============================================================================

#[test]
fn test_comprehensive_all_features() {
    let skill_dir = fixtures_dir().join("comprehensive-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "comprehensive-001",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();
}

// =============================================================================
// Feature Tests: New simplified format features
// =============================================================================

#[test]
fn test_feature_multiple_skill_dirs() -> TestResult {
    // Test that multiple skill directories can be specified and processed
    let skill_a = fixtures_dir().join("multi-skill-a");
    let skill_b = fixtures_dir().join("multi-skill-b");

    let output = skill_test()
        .arg(&skill_a)
        .arg(&skill_b)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(120))
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify both skills were processed (regardless of pass/fail)
    assert!(
        stdout.contains("multi-skill-a"),
        "Output should contain multi-skill-a"
    );
    assert!(
        stdout.contains("multi-skill-b"),
        "Output should contain multi-skill-b"
    );
    assert!(
        stdout.contains("\"total_skills\": 2"),
        "Should have 2 total skills"
    );
    Ok(())
}

#[test]
fn test_feature_spec_yaml_pattern() {
    // Test that *.spec.yaml files are discovered
    let skill_dir = fixtures_dir().join("spec-yaml-skill");

    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Test ID from basic.spec.yaml should be found
        .stdout(predicate::str::contains("spec-001"));
}

#[test]
fn test_feature_yml_extension() {
    // Test that .yml extension files are discovered
    let skill_dir = fixtures_dir().join("yml-extension-skill");

    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Test ID from test-yml.yml should be found
        .stdout(predicate::str::contains("yml-001"));
}

#[test]
fn test_feature_node_modules_excluded() {
    // Test that node_modules/ directory is excluded from test discovery
    let skill_dir = fixtures_dir().join("node-modules-skill");

    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Main test should be found
        .stdout(predicate::str::contains("main-001"))
        // Excluded test should NOT be found
        .stdout(predicate::str::contains("excluded-001").not());
}

#[test]
fn test_feature_file_reference() -> TestResult {
    // Test that file: directive loads assertions from external files
    let skill_dir = fixtures_dir().join("file-ref-skill");

    let output = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify test was discovered and run
    assert!(
        stdout.contains("file-ref-001"),
        "Test file-ref-001 should be discovered"
    );
    // Verify file reference was resolved - shared-check assertion ID should appear in failures
    // (since the skill output won't match, but the assertion ID being present proves it was loaded)
    assert!(
        stdout.contains("shared-check") || stdout.contains("inline-check"),
        "Assertions from file reference should be loaded and evaluated"
    );
    Ok(())
}

#[test]
fn test_feature_config_yaml() -> TestResult {
    // Test that skill-test.config.yaml is loaded and applied
    let skill_dir = fixtures_dir().join("config-skill");

    let output = skill_test()
        .arg(&skill_dir)
        .args(["--format", "json"])
        .timeout(std::time::Duration::from_secs(120))
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Config sets iterations: 2, so output should show 2 iterations
    let result: serde_json::Value = serde_json::from_str(&output_str)?;
    // iterations is an array of DetailedIterationResult objects
    let iterations_count = result["skills"][0]["tests"][0]["iterations"]
        .as_array()
        .ok_or("iterations should be an array")?
        .len();

    assert_eq!(
        iterations_count, 2,
        "config.yaml should set iterations to 2"
    );
    Ok(())
}
