//! Core library for skill-test CLI.
//!
//! This crate provides the core functionality for testing Claude Code skills:
//! - Test case loading from YAML
//! - Contract loading and merging
//! - Assertion evaluation (regex, contains, `line_count`, exec)
//! - Claude CLI execution and skill extraction
//! - Verdict judgment based on truth table
//! - Report generation

pub mod assertion;
pub mod claude;
pub mod codeblock;
pub mod config;
pub mod contract;
pub mod judge;
pub mod loader;
pub mod report;
pub mod reporter;
pub mod runner;
pub mod skill_dir;
pub mod types;

pub use assertion::{evaluate_assertion, evaluate_contract, evaluate_tool_called};
pub use claude::{
    ClaudeEvent, ClaudeResponse, compute_output_hash, execute_claude, extract_skills,
};
pub use codeblock::{extract_code_block, extract_code_blocks};
pub use config::{ConfigError, ConfigOverrides, apply_overrides, load_config};
pub use contract::{load_contracts, load_contracts_for_called};
pub use judge::judge;
pub use loader::{
    LoaderError, ResolvedScenario, discover_test_files, load_and_resolve_test_case,
    load_and_resolve_test_file, load_simplified_test_cases, load_test_cases, load_test_file,
    resolve_assertions, resolve_test_file,
};
pub use report::ReportFormat;
pub use reporter::{Reporter, ReporterConfig};
pub use runner::{
    MAX_OUTPUT_CHARS, ProgressEvent, ProgressSender, RunnerConfig, SimplifiedIterationResult,
    TestSummary, compute_avg_latency, default_parallelism, run_all_skill_tests,
    run_all_skill_tests_with_progress, run_simplified_iteration,
    run_simplified_iteration_with_progress, run_simplified_test_case,
    run_simplified_test_case_with_progress, run_single_iteration, run_skill_tests,
    run_skill_tests_with_progress, run_test_case, truncate_utf8,
};
pub use skill_dir::{
    SkillDirError, detect_skill_dir, detect_skill_dirs, is_skill_dir, resolve_skill_paths,
};
pub use types::*;
