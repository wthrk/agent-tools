//! LLM eval json_schema tests.
//!
//! llm_eval always uses JSON format with default schema: `{"result": boolean, "reason": string}`.
//! Custom schemas can be specified via `json_schema`.
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;

// =============================================================================
// LLM Eval: JSON format (always enabled)
// =============================================================================

#[test]
fn test_llm_eval_json_format() {
    // Tests that llm_eval uses JSON format by default
    let skill_dir = fixtures_dir().join("llm-eval-json-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "llm-eval-json-format-001",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"verdict\": \"Pass\"")
                .or(predicate::str::contains("\"verdict\":\"Pass\"")),
        );
}

#[test]
fn test_llm_eval_json_schema() {
    let skill_dir = fixtures_dir().join("llm-eval-json-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "llm-eval-json-schema-001",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"verdict\": \"Pass\"")
                .or(predicate::str::contains("\"verdict\":\"Pass\"")),
        );
}

#[test]
fn test_llm_eval_json_expect_fail() {
    let skill_dir = fixtures_dir().join("llm-eval-json-skill");
    skill_test()
        .arg(&skill_dir)
        .args([
            "--iterations",
            "1",
            "--format",
            "json",
            "--filter",
            "llm-eval-json-fail-001",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"verdict\": \"Pass\"")
                .or(predicate::str::contains("\"verdict\":\"Pass\"")),
        );
}
