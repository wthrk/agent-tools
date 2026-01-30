//! Output format tests for JSON and error logs.
#![cfg(feature = "integration-test")]

use super::{fixtures_dir, skill_test};
use predicates::prelude::*;
use std::fs;

/// JSON出力とエラーログが同じスキーマを持つことを確認
#[test]
fn test_json_and_error_log_schema_parity() {
    let skill_dir = fixtures_dir().join("failure-skill");
    let log_dir = skill_dir.join(".skill-test-logs");

    // Clean up
    if log_dir.exists() {
        let _ = fs::remove_dir_all(&log_dir);
    }

    // Get JSON output
    let json_output = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json", "--threshold", "0"])
        .output()
        .expect("failed to execute");

    // Get error log (run again with table format to generate log)
    let _ = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--threshold", "100"])
        .output();

    // Read error log
    let log_files: Vec<_> = fs::read_dir(&log_dir)
        .expect("log dir should exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    assert!(!log_files.is_empty(), "error log should be created");

    let json_str = String::from_utf8_lossy(&json_output.stdout);
    let log_str = fs::read_to_string(log_files[0].path()).unwrap();

    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let log_value: serde_json::Value = serde_json::from_str(&log_str).unwrap();

    // Both should have the same top-level keys
    let json_keys: Vec<_> = json_value.as_object().unwrap().keys().collect();
    let log_keys: Vec<_> = log_value.as_object().unwrap().keys().collect();
    assert_eq!(
        json_keys, log_keys,
        "JSON and error log should have same schema"
    );

    // Clean up
    let _ = fs::remove_dir_all(&log_dir);
}

/// --no-error-log オプションでエラーログが作成されないことを確認
#[test]
fn test_no_error_log_option() {
    let skill_dir = fixtures_dir().join("failure-skill");
    let log_dir = skill_dir.join(".skill-test-logs");

    // Clean up
    if log_dir.exists() {
        let _ = fs::remove_dir_all(&log_dir);
    }

    // Run with --no-error-log
    let _ = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--threshold", "100", "--no-error-log"])
        .output();

    // Log directory should not exist or be empty
    assert!(
        !log_dir.exists() || fs::read_dir(&log_dir).unwrap().count() == 0,
        "error log should not be created with --no-error-log"
    );
}

/// CSV形式が削除されたことを確認
#[test]
fn test_csv_format_removed() {
    let skill_dir = fixtures_dir().join("simple-skill");
    skill_test()
        .arg(&skill_dir)
        .args(["--format", "csv"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("csv").or(predicate::str::contains("invalid")));
}

/// 出力順序が決定的であることを確認（テストIDでソート）
#[test]
fn test_output_order_deterministic() {
    let skill_dir = fixtures_dir().join("multi-test-skill");

    // Run twice and compare
    let output1 = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .output()
        .expect("first run");

    let output2 = skill_test()
        .arg(&skill_dir)
        .args(["--iterations", "1", "--format", "json"])
        .output()
        .expect("second run");

    let json1: serde_json::Value = serde_json::from_slice(&output1.stdout).unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&output2.stdout).unwrap();

    // Test order should be the same
    let tests1: Vec<_> = json1["skills"][0]["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    let tests2: Vec<_> = json2["skills"][0]["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    assert_eq!(tests1, tests2, "test order should be deterministic");
}
