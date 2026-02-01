//! Integration tests for skill-test CLI.
//!
//! Each test targets a specific feature to ensure failures clearly identify what broke.
//!
//! ## Test Categories
//!
//! ### CLI Arguments (no Claude CLI required)
//! - help, version, invalid args
//!
//! ### Assertion Types (require Claude CLI)
//! - regex (present, absent, complex pattern)
//! - contains (present, absent)
//! - `line_count` (min, max, min+max)
//! - exec (`exit_code`, `output_contains`)
//! - `tool_called` (present, absent, regex pattern)
//! - `llm_eval` (pass, fail, `json_schema` - always uses JSON format)
//!
//! ### Features (require Claude CLI)
//! - `golden_assertions`
//! - validation (inline assertions)
//! - iterations
//! - threshold
//! - strict
//! - filter
//! - format (json, table)
//! - `TestFile` format (scenarios + named assertions)
//!
//! ### Validation Errors (no Claude CLI required)
//! - Empty prompt
//! - Undefined assertion reference
//!
//! Run with: `cargo test --features integration-test`

#[path = "cli_tests/args.rs"]
mod args;
#[path = "cli_tests/assertions.rs"]
mod assertions;
#[path = "cli_tests/failures.rs"]
mod failures;
#[path = "cli_tests/features.rs"]
mod features;
#[path = "cli_tests/llm_eval_json.rs"]
mod llm_eval_json;
#[path = "cli_tests/output_format.rs"]
mod output_format;
#[path = "cli_tests/testfile_format.rs"]
mod testfile_format;
#[path = "cli_tests/validation_errors.rs"]
mod validation_errors;

use std::path::PathBuf;

#[must_use]
pub fn binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/skill-test -> crates
    path.pop(); // crates -> workspace root
    path.push("target");
    path.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    path.push("skill-test");
    path
}

/// Create a skill-test command for integration testing.
///
/// Always uses `--parallel 1` to reduce CPU load during integration tests,
/// since cargo test already runs tests in parallel.
#[must_use]
pub fn skill_test() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::new(binary_path());
    cmd.arg("--parallel").arg("1");
    cmd
}

#[must_use]
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
