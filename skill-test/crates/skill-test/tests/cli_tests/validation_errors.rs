//! Validation error tests for `TestFile` format.
//! These tests verify that proper errors are returned for invalid test files.
//! No Claude CLI required.

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

// =============================================================================
// Validation Errors: Empty Prompt
// =============================================================================

#[test]
fn test_error_empty_prompt() {
    let skill_dir = fixtures_dir().join("error-empty-prompt-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("empty prompt").or(predicate::str::contains("EmptyPrompt")),
        );
}

// =============================================================================
// Validation Errors: Undefined Assertion Reference
// =============================================================================

#[test]
fn test_error_undefined_assertion_ref() {
    let skill_dir = fixtures_dir().join("error-undefined-ref-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("undefined")
                .or(predicate::str::contains("UndefinedAssertionRef")),
        );
}
