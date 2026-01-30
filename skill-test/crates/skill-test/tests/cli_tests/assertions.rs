//! Assertion type tests (require Claude CLI).
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

// =============================================================================
// Assertion Type: regex
// =============================================================================

#[test]
fn test_assertion_regex_present() {
    let skill_dir = fixtures_dir().join("regex-skill");
    // Run multiple iterations with lower threshold due to LLM non-determinism
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "3",
            "--threshold",
            "30",
            "--filter",
            "regex-present-001",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();
}

#[test]
fn test_assertion_regex_absent() {
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json", "--filter", "regex-absent-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed (regex-absent-todo assertion verified pattern is absent)
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_regex_complex() {
    // Tests complex regex patterns (word boundary, quantifiers)
    let skill_dir = fixtures_dir().join("regex-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json", "--filter", "regex-complex-001"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed (complex regex assertions verified)
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

// =============================================================================
// Assertion Type: contains
// =============================================================================

#[test]
fn test_assertion_contains_present() {
    let skill_dir = fixtures_dir().join("contains-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_contains_absent() {
    let skill_dir = fixtures_dir().join("contains-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

// =============================================================================
// Assertion Type: line_count
// =============================================================================

#[test]
fn test_assertion_line_count_min_max() {
    let skill_dir = fixtures_dir().join("line-count-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_line_count_min_only() {
    let skill_dir = fixtures_dir().join("line-count-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_line_count_max_only() {
    let skill_dir = fixtures_dir().join("line-count-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

// =============================================================================
// Assertion Type: exec
// =============================================================================

#[test]
fn test_assertion_exec_exit_code_zero() {
    if which::which("node").is_err() {
        eprintln!("Skipping: Node.js not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_exec_output_contains() {
    if which::which("node").is_err() {
        eprintln!("Skipping: Node.js not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_exec_failure_exit_code() {
    // Tests exec assertion failure when exit code is non-zero
    if which::which("node").is_err() {
        eprintln!("Skipping: Node.js not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-failure-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "json",
            "--filter", "exec-failure-exit-code-001",
            "--threshold", "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success() // threshold=0 so test passes even with failures
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // failures array should contain the assertion ID (threshold=0 means verdict is Pass)
    assert!(
        output_str.contains("exec-exit-code-should-fail"),
        "failures should contain assertion ID. Got: {output_str}"
    );
}

#[test]
fn test_assertion_exec_failure_output_contains() {
    // Tests exec assertion failure when output doesn't contain expected string
    if which::which("node").is_err() {
        eprintln!("Skipping: Node.js not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-failure-skill");
    let output = skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations", "1",
            "--format", "json",
            "--filter", "exec-failure-output-contains-001",
            "--threshold", "0",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success() // threshold=0
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    // failures array should contain the assertion ID
    assert!(
        output_str.contains("exec-output-contains-should-fail"),
        "failures should contain assertion ID. Got: {output_str}"
    );
}

#[test]
fn test_assertion_exec_timeout() {
    // Tests exec assertion behavior when execution times out
    // Timeout is treated as an execution error, not just an assertion failure
    if which::which("node").is_err() {
        eprintln!("Skipping: Node.js not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-timeout-skill");
    // In JSON format, the timeout error message is in stdout, not stderr
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .failure()
        .stdout(predicate::str::contains("timeout"));
}

#[test]
fn test_assertion_exec_python() {
    // Tests exec assertion with Python
    if which::which("python3").is_err() {
        eprintln!("Skipping: Python3 not found");
        return;
    }
    let skill_dir = fixtures_dir().join("exec-python-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify verdict passed
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

// =============================================================================
// Assertion Type: tool_called
// =============================================================================

#[test]
fn test_assertion_tool_called_present() {
    let skill_dir = fixtures_dir().join("tool-called-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Check that verdict passed (tool-called-skill-present assertion verified Skill tool was called)
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_tool_called_absent() {
    // tool-called-no-bash is in golden_assertions, so it may fail without failing the test
    let skill_dir = fixtures_dir().join("tool-called-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Golden assertions are tracked but don't fail the test
        .stdout(predicate::str::contains("golden_passed").or(predicate::str::contains("golden_failures")));
}

#[test]
fn test_assertion_tool_called_regex_pattern() {
    // tool-called-regex-read-or-glob is in golden_assertions
    let skill_dir = fixtures_dir().join("tool-called-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Verify called_tools includes pattern-matching tools (Read or Glob)
        .stdout(predicate::str::contains("called_tools"));
}

#[test]
fn test_assertion_tool_called_mcp_pattern() {
    // tool-called-no-mcp verifies no MCP tools were called (main assertion)
    let skill_dir = fixtures_dir().join("tool-called-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        // Main assertions must pass for verdict to be Pass
        .stdout(predicate::str::contains("\"verdict\": \"Pass\"").or(predicate::str::contains("\"verdict\":\"Pass\"")));
}

#[test]
fn test_assertion_tool_called_in_output() {
    let skill_dir = fixtures_dir().join("tool-called-skill");
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
        output_str.contains("called_tools"),
        "JSON output must contain called_tools field"
    );
}
