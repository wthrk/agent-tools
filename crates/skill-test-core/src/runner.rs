//! Test runner for executing skill tests.

use crate::assertion::{AssertionError, evaluate_assertion, evaluate_contract};
use crate::claude::{ClaudeError, compute_output_hash, execute_claude, extract_skills};
use crate::contract::{ContractError, load_contracts_for_called};
use crate::judge::judge;
use crate::loader::{LoaderError, discover_test_files, load_and_resolve_test_case};
use crate::types::{
    Assertion, DetailedAssertionResult, DetailedIterationResult, DetailedTestResult, HookType,
    MatchPolicy, SimplifiedTestCase, SimplifiedTestResult, SkillDir, SkillTestResult,
    SkillTestSummary, TestCase, TestResult, Verdict,
};
use std::path::Path;
use thiserror::Error;
use tokio::sync::mpsc;

/// Progress events emitted during test execution.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// All tests across all skills are starting (flattened execution).
    AllTestsStarted {
        total_tests: usize,
        skill_count: usize,
    },
    /// A skill test run has started.
    SkillStarted {
        skill_name: String,
        test_count: usize,
    },
    /// An individual test has started.
    TestStarted {
        skill_name: String,
        test_id: String,
        desc: Option<String>,
        total_iterations: u32,
    },
    /// An iteration has started (verbose).
    IterationStarted {
        skill_name: String,
        test_id: String,
        iteration: u32,
        total: u32,
    },
    /// An iteration has completed.
    IterationCompleted {
        skill_name: String,
        test_id: String,
        iteration: u32,
        passed: bool,
        output_preview: String,
        latency_ms: u64,
        /// Detailed iteration result (always populated).
        detailed: DetailedIterationResult,
    },
    /// An assertion result (verbose).
    AssertionResult {
        skill_name: String,
        test_id: String,
        iteration: u32,
        assertion_id: String,
        assertion_desc: Option<String>,
        passed: bool,
        is_golden: bool,
    },
    /// An individual test has completed.
    TestCompleted {
        skill_name: String,
        test_id: String,
        desc: Option<String>,
        result: SimplifiedTestResult,
        /// Detailed test result (always populated).
        detailed: DetailedTestResult,
    },
    /// All tests for a skill have completed.
    SkillCompleted {
        skill_name: String,
        verdict: Verdict,
    },
    /// A skill execution failed with an unrecoverable error.
    /// Contains partial results for tests that were in-progress.
    SkillError {
        skill_name: String,
        error: String,
        partial_results: Vec<DetailedTestResult>,
    },
}

/// Sender for progress events.
pub type ProgressSender = mpsc::UnboundedSender<ProgressEvent>;

/// Errors that can occur during test execution.
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Claude execution error: {0}")]
    Claude(#[from] ClaudeError),
    #[error("Contract error: {0}")]
    Contract(#[from] ContractError),
    #[error("Assertion error: {0}")]
    Assertion(#[from] AssertionError),
    #[error("Loader error: {0}")]
    Loader(#[from] LoaderError),
}

/// Configuration for the test runner.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub model: String,
    pub timeout_ms: u64,
    pub hook_path: Option<String>,
    pub contracts_dir: String,
    pub strict_contracts: bool,
    pub iterations: u32,
    pub skill_dir: String,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_ms: 60000,
            hook_path: None,
            contracts_dir: "./contracts".to_string(),
            strict_contracts: false,
            iterations: 10,
            skill_dir: ".".to_string(),
        }
    }
}

/// Run a single test case iteration.
///
/// # Errors
/// Returns an error if:
/// - Claude execution fails
/// - Contract loading fails
/// - Assertion evaluation fails
#[allow(clippy::too_many_lines)]
pub async fn run_single_iteration(
    test: &TestCase,
    iteration: u32,
    config: &RunnerConfig,
) -> Result<TestResult, RunnerError> {
    // Execute Claude in sandbox
    let exec_result = execute_claude(
        &test.prompt,
        &config.model,
        config.timeout_ms,
        config.hook_path.as_deref(),
        &config.skill_dir,
    )
    .await?;

    let called_skills = extract_skills(&exec_result.response);
    let called_tools: Vec<String> = exec_result
        .response
        .called_tools()
        .into_iter()
        .map(ToString::to_string)
        .collect();
    let output_hash = compute_output_hash(&exec_result.response.result);

    // Check skill matching
    let skill_passed = match test.match_policy {
        MatchPolicy::All => test
            .expected_skills
            .iter()
            .all(|s| called_skills.contains(s)),
        MatchPolicy::Any => test
            .expected_skills
            .iter()
            .any(|s| called_skills.contains(s)),
    };

    // Load and evaluate contracts (only if skills matched)
    let (mut contract_passed, golden_passed, mut failures, golden_failures) = if skill_passed {
        // For match_policy: any, only load contracts for skills that were actually called
        let skills_to_load = if test.match_policy == MatchPolicy::Any {
            called_skills
                .iter()
                .filter(|s| test.expected_skills.contains(s))
                .cloned()
                .collect()
        } else {
            test.expected_skills.clone()
        };

        let contract = load_contracts_for_called(
            Path::new(&config.contracts_dir),
            &skills_to_load,
            config.strict_contracts,
        )?;

        // TODO: Contract warnings (e.g., missing skill contract in non-strict mode)
        // are currently not surfaced to the user. Consider returning warnings
        // in TestResult or using a callback mechanism.
        let _ = &contract.warnings;

        let result =
            evaluate_contract(&exec_result.response.result, &contract, &called_tools).await?;

        (
            Some(result.contract_passed),
            result.golden_passed,
            result.failures,
            result.golden_failures,
        )
    } else {
        (None, None, vec![], vec![])
    };

    // Evaluate inline validation assertions (if present and skill matched)
    if skill_passed {
        if let Some(ref validation) = test.validation {
            for assertion in &validation.assertions {
                let assertion_id = assertion.id().to_string();

                match evaluate_assertion(&exec_result.response.result, assertion, &called_tools)
                    .await
                {
                    Ok(passed) => {
                        if !passed {
                            failures.push(assertion_id.clone());
                            if contract_passed == Some(true) {
                                contract_passed = Some(false);
                            }
                        }
                    }
                    Err(e) => {
                        failures.push(format!("{assertion_id}: {e}"));
                        if contract_passed == Some(true) {
                            contract_passed = Some(false);
                        }
                    }
                }
            }
        }
    }

    // Judge verdict
    let contract_result_for_judge = contract_passed.map(|passed| crate::types::ContractResult {
        contract_passed: passed,
        golden_passed,
        details: vec![],
        failures: failures.clone(),
        golden_failures: golden_failures.clone(),
    });

    let judgment = judge(
        &test.expected_skills,
        &test.forbid_skills,
        &called_skills,
        test.match_policy,
        contract_result_for_judge.as_ref(),
    );

    Ok(TestResult {
        test_id: test.id.clone(),
        iteration,
        prompt: test.prompt.clone(),
        expected_skills: test.expected_skills.clone(),
        called_skills,
        called_tools,
        output_text: exec_result.response.result,
        output_hash,
        skill_passed,
        contract_passed,
        golden_passed,
        failures,
        golden_failures,
        verdict: judgment.verdict,
        latency_ms: exec_result.latency_ms,
    })
}

/// Run all iterations for a test case.
///
/// # Errors
/// Returns an error if any iteration fails.
pub async fn run_test_case(
    test: &TestCase,
    config: &RunnerConfig,
) -> Result<Vec<TestResult>, RunnerError> {
    let iterations = test.iterations.unwrap_or(config.iterations);
    let mut results = Vec::with_capacity(usize::try_from(iterations).unwrap_or(usize::MAX));

    for i in 1..=iterations {
        let result = run_single_iteration(test, i, config).await?;
        results.push(result);
    }

    Ok(results)
}

/// Summary of test results.
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub warned: u32,
    pub pass_rate: f64,
}

impl TestSummary {
    /// Create a summary from test results.
    #[must_use]
    pub fn from_results(results: &[TestResult]) -> Self {
        let total = u32::try_from(results.len()).unwrap_or(u32::MAX);
        let passed = u32::try_from(
            results
                .iter()
                .filter(|r| r.verdict == Verdict::Pass)
                .count(),
        )
        .unwrap_or(u32::MAX);
        let failed = u32::try_from(
            results
                .iter()
                .filter(|r| r.verdict == Verdict::Fail)
                .count(),
        )
        .unwrap_or(u32::MAX);
        let warned = u32::try_from(
            results
                .iter()
                .filter(|r| r.verdict == Verdict::Warn)
                .count(),
        )
        .unwrap_or(u32::MAX);

        // Warn counts as pass for pass rate
        let pass_count = passed + warned;
        let pass_rate = if total > 0 {
            (f64::from(pass_count) / f64::from(total)) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            passed,
            failed,
            warned,
            pass_rate,
        }
    }

    /// Check if pass rate meets threshold.
    #[must_use]
    pub fn meets_threshold(&self, threshold: u32) -> bool {
        self.pass_rate >= f64::from(threshold)
    }
}

/// Compute average latency from results.
///
/// Latency values are converted to f64 for averaging. For practical latency
/// values (less than 2^52 milliseconds, approximately 142 years), precision
/// is maintained.
#[must_use]
pub fn compute_avg_latency(results: &[TestResult]) -> Option<f64> {
    if results.is_empty() {
        return None;
    }
    let count = results.len();
    // Saturate at u32::MAX to avoid precision loss lint when converting u64 to f64.
    // For practical latency values this has no precision impact.
    let total: f64 = results
        .iter()
        .map(|r| {
            // Saturate at u32::MAX for conversion
            let ms = u32::try_from(r.latency_ms).unwrap_or(u32::MAX);
            f64::from(ms)
        })
        .sum();
    let count_f64 = f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    Some(total / count_f64)
}

// =============================================================================
// Simplified runner functions for the new skill directory format
// =============================================================================

/// Result of a single simplified test iteration.
#[derive(Debug, Clone)]
pub struct SimplifiedIterationResult {
    pub test_id: String,
    pub iteration: u32,
    pub output_text: String,
    pub output_hash: String,
    pub called_tools: Vec<String>,
    pub assertion_passed: bool,
    pub golden_passed: Option<bool>,
    pub failures: Vec<String>,
    pub golden_failures: Vec<String>,
    pub latency_ms: u64,
}

/// Get hook path string based on hook type and skill directory.
fn resolve_hook_path(
    hook: &HookType,
    hook_path: Option<&Path>,
    skill_dir: &Path,
) -> Option<String> {
    match hook {
        HookType::None => None,
        HookType::Simple => {
            // Use built-in simple hook
            Some("simple".to_string())
        }
        HookType::Forced => {
            // Use built-in forced hook
            Some("forced".to_string())
        }
        HookType::Custom => {
            // Use custom hook path (relative to skill directory)
            hook_path.map(|p| {
                if p.is_absolute() {
                    p.display().to_string()
                } else {
                    skill_dir.join(p).display().to_string()
                }
            })
        }
    }
}

/// Run a single iteration for a simplified test case.
///
/// # Errors
/// Returns an error if Claude execution or assertion evaluation fails.
pub async fn run_simplified_iteration(
    test: &SimplifiedTestCase,
    assertions: &[Assertion],
    golden_assertions: &[Assertion],
    iteration: u32,
    skill_dir: &SkillDir,
) -> Result<SimplifiedIterationResult, RunnerError> {
    let (result, _) = run_simplified_iteration_with_progress(
        test,
        assertions,
        golden_assertions,
        iteration,
        skill_dir,
        None,
    )
    .await?;
    Ok(result)
}

/// Run a single iteration of a simplified test case with progress reporting.
///
/// Returns a tuple of (`SimplifiedIterationResult`, `DetailedIterationResult`).
/// The detailed result is always populated for error logging purposes.
///
/// # Errors
/// Returns an error if Claude execution fails or assertion evaluation fails.
#[allow(clippy::too_many_lines)]
pub async fn run_simplified_iteration_with_progress(
    test: &SimplifiedTestCase,
    assertions: &[Assertion],
    golden_assertions: &[Assertion],
    iteration: u32,
    skill_dir: &SkillDir,
    progress: Option<&ProgressSender>,
) -> Result<(SimplifiedIterationResult, DetailedIterationResult), RunnerError> {
    let hook_path = resolve_hook_path(
        &skill_dir.config.hook,
        skill_dir.config.hook_path.as_deref(),
        &skill_dir.path,
    );

    // Execute Claude in sandbox
    let exec_result = execute_claude(
        &test.prompt,
        &skill_dir.config.model,
        skill_dir.config.timeout,
        hook_path.as_deref(),
        skill_dir.path.to_str().unwrap_or("."),
    )
    .await?;

    let called_tools: Vec<String> = exec_result
        .response
        .called_tools()
        .into_iter()
        .map(ToString::to_string)
        .collect();
    let output_hash = compute_output_hash(&exec_result.response.result);

    // Evaluate assertions (always collect detailed results for error logging)
    let mut assertion_passed = true;
    let mut failures = Vec::new();
    let mut detailed_assertions = Vec::new();

    for assertion in assertions {
        let assertion_id = assertion.id().to_string();
        let assertion_desc = assertion.desc().map(ToString::to_string);

        let (passed, error) = match evaluate_assertion(
            &exec_result.response.result,
            assertion,
            &called_tools,
        )
        .await
        {
            Ok(p) => {
                if !p {
                    assertion_passed = false;
                    failures.push(assertion_id.clone());
                }
                (p, None)
            }
            Err(e) => {
                assertion_passed = false;
                let error_str = e.to_string();
                failures.push(format!("{assertion_id}: {error_str}"));
                (false, Some(error_str))
            }
        };

        // Always collect detailed assertion result for error logging
        detailed_assertions.push(build_detailed_assertion(
            assertion,
            passed,
            error.as_deref(),
        ));

        // Emit assertion result event
        if let Some(tx) = progress {
            let _ = tx.send(ProgressEvent::AssertionResult {
                skill_name: skill_dir.name.clone(),
                test_id: test.id.clone(),
                iteration,
                assertion_id,
                assertion_desc,
                passed,
                is_golden: false,
            });
        }
    }

    // Evaluate golden assertions (do not affect pass/fail)
    let mut golden_passed = if golden_assertions.is_empty() {
        None
    } else {
        Some(true)
    };
    let mut golden_failures = Vec::new();
    let mut detailed_golden_assertions = Vec::new();

    for assertion in golden_assertions {
        let assertion_id = assertion.id().to_string();
        let assertion_desc = assertion.desc().map(ToString::to_string);

        let (passed, error) = match evaluate_assertion(
            &exec_result.response.result,
            assertion,
            &called_tools,
        )
        .await
        {
            Ok(p) => {
                if !p {
                    golden_passed = Some(false);
                    golden_failures.push(assertion_id.clone());
                }
                (p, None)
            }
            Err(e) => {
                golden_passed = Some(false);
                let error_str = e.to_string();
                golden_failures.push(format!("{assertion_id}: {error_str}"));
                (false, Some(error_str))
            }
        };

        // Always collect detailed golden assertion result for error logging
        detailed_golden_assertions.push(build_detailed_assertion(
            assertion,
            passed,
            error.as_deref(),
        ));

        // Emit golden assertion result event
        if let Some(tx) = progress {
            let _ = tx.send(ProgressEvent::AssertionResult {
                skill_name: skill_dir.name.clone(),
                test_id: test.id.clone(),
                iteration,
                assertion_id,
                assertion_desc,
                passed,
                is_golden: true,
            });
        }
    }

    let result = SimplifiedIterationResult {
        test_id: test.id.clone(),
        iteration,
        output_text: exec_result.response.result.clone(),
        output_hash: output_hash.clone(),
        called_tools: called_tools.clone(),
        assertion_passed,
        golden_passed,
        failures,
        golden_failures,
        latency_ms: exec_result.latency_ms,
    };

    // Always build detailed iteration result for error logging
    let detailed = DetailedIterationResult {
        iteration,
        passed: assertion_passed,
        latency_ms: exec_result.latency_ms,
        output: truncate_utf8(&exec_result.response.result, MAX_OUTPUT_CHARS),
        output_hash,
        called_tools,
        assertions: detailed_assertions,
        golden_assertions: detailed_golden_assertions,
    };

    Ok((result, detailed))
}

/// Run all iterations for a simplified test case.
///
/// Execution errors (timeout, etc.) are treated as failed iterations rather than
/// aborting the entire test run.
///
/// # Errors
/// Returns an error only for non-recoverable issues like test discovery failures.
pub async fn run_simplified_test_case(
    test: &SimplifiedTestCase,
    assertions: &[Assertion],
    golden_assertions: &[Assertion],
    skill_dir: &SkillDir,
) -> Result<SimplifiedTestResult, RunnerError> {
    let (result, _) = run_simplified_test_case_with_progress(
        test,
        assertions,
        golden_assertions,
        skill_dir,
        None,
    )
    .await?;
    Ok(result)
}

/// Run all iterations for a simplified test case with progress reporting.
///
/// Returns a tuple of (`SimplifiedTestResult`, `DetailedTestResult`).
/// The detailed result is always populated for error logging purposes.
///
/// Execution errors (timeout, etc.) are treated as failed iterations rather than
/// aborting the entire test run.
///
/// # Errors
/// Returns an error only for non-recoverable issues like test discovery failures.
#[allow(clippy::too_many_lines)]
pub async fn run_simplified_test_case_with_progress(
    test: &SimplifiedTestCase,
    assertions: &[Assertion],
    golden_assertions: &[Assertion],
    skill_dir: &SkillDir,
    progress: Option<&ProgressSender>,
) -> Result<(SimplifiedTestResult, DetailedTestResult), RunnerError> {
    let iterations = test.iterations.unwrap_or(skill_dir.config.iterations);
    let mut results = Vec::with_capacity(usize::try_from(iterations).unwrap_or(usize::MAX));
    let mut all_failures = Vec::new();
    let mut all_golden_failures = Vec::new();
    let mut detailed_iterations = Vec::new();

    for i in 1..=iterations {
        // Emit iteration started event
        if let Some(tx) = progress {
            let _ = tx.send(ProgressEvent::IterationStarted {
                skill_name: skill_dir.name.clone(),
                test_id: test.id.clone(),
                iteration: i,
                total: iterations,
            });
        }

        let (result, detailed_iter, output_preview) = match run_simplified_iteration_with_progress(
            test,
            assertions,
            golden_assertions,
            i,
            skill_dir,
            progress,
        )
        .await
        {
            Ok((r, d)) => {
                let preview = truncate_output(&r.output_text, 100);
                (r, d, preview)
            }
            Err(e) => {
                // Treat execution errors (timeout, etc.) as failed iteration
                let error_msg = e.to_string();
                let preview = truncate_output(&error_msg, 100);
                let result = SimplifiedIterationResult {
                    test_id: test.id.clone(),
                    iteration: i,
                    output_text: String::new(),
                    output_hash: String::new(),
                    called_tools: vec![],
                    assertion_passed: false,
                    golden_passed: None,
                    failures: vec![error_msg.clone()],
                    golden_failures: vec![],
                    latency_ms: 0,
                };
                // Always create a detailed iteration for error case (for error logging)
                let detailed = DetailedIterationResult {
                    iteration: i,
                    passed: false,
                    latency_ms: 0,
                    output: String::new(),
                    output_hash: String::new(),
                    called_tools: vec![],
                    assertions: vec![DetailedAssertionResult {
                        name: "execution".to_string(),
                        desc: Some("Claude execution".to_string()),
                        assertion_type: "execution".to_string(),
                        pattern: None,
                        passed: false,
                        error: Some(error_msg),
                    }],
                    golden_assertions: vec![],
                };
                (result, detailed, preview)
            }
        };

        // Emit iteration completed event
        if let Some(tx) = progress {
            let _ = tx.send(ProgressEvent::IterationCompleted {
                skill_name: skill_dir.name.clone(),
                test_id: test.id.clone(),
                iteration: i,
                passed: result.assertion_passed,
                output_preview,
                latency_ms: result.latency_ms,
                detailed: detailed_iter.clone(),
            });
        }

        // Always collect detailed iteration result for error logging
        detailed_iterations.push(detailed_iter);

        if !result.failures.is_empty() {
            all_failures.push(format!("iteration {i}: {}", result.failures.join(", ")));
        }
        if !result.golden_failures.is_empty() {
            all_golden_failures.push(format!(
                "iteration {i}: {}",
                result.golden_failures.join(", ")
            ));
        }
        results.push(result);
    }

    let passed =
        u32::try_from(results.iter().filter(|r| r.assertion_passed).count()).unwrap_or(u32::MAX);
    let failed = iterations.saturating_sub(passed);
    let pass_rate = if iterations > 0 {
        (f64::from(passed) / f64::from(iterations)) * 100.0
    } else {
        0.0
    };

    // Determine verdict based on threshold
    let verdict = if pass_rate >= f64::from(skill_dir.config.threshold) {
        Verdict::Pass
    } else {
        Verdict::Fail
    };

    // Collect all called tools (union across all iterations)
    let mut all_called_tools: Vec<String> = results
        .iter()
        .flat_map(|r| r.called_tools.iter().cloned())
        .collect();
    all_called_tools.sort();
    all_called_tools.dedup();

    let summary = SimplifiedTestResult {
        id: test.id.clone(),
        iterations,
        passed,
        failed,
        pass_rate,
        verdict,
        failures: all_failures,
        golden_failures: all_golden_failures,
        called_tools: all_called_tools,
    };

    // Always build detailed test result for error logging
    let detailed = DetailedTestResult {
        name: test.id.clone(),
        desc: test.desc.clone(),
        prompt: test.prompt.clone(),
        iterations: detailed_iterations,
        summary: summary.clone(),
    };

    Ok((summary, detailed))
}

/// Truncate output for preview display (UTF-8 safe, first line only).
fn truncate_output(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    // Use char_indices to safely truncate at char boundaries (UTF-8 safe)
    let char_count = first_line.chars().count();
    if char_count <= max_len {
        first_line.to_string()
    } else {
        let end_idx = first_line
            .char_indices()
            .nth(max_len)
            .map_or(first_line.len(), |(i, _)| i);
        format!("{}...", &first_line[..end_idx])
    }
}

/// Maximum output size in characters (not bytes) for detailed logging.
/// With UTF-8, 100K chars could be up to 400KB in memory (4 bytes/char max).
pub const MAX_OUTPUT_CHARS: usize = 100_000;

/// Truncate string at character boundary for full output storage (UTF-8 safe).
/// Unlike `truncate_output`, this preserves all lines up to `max_chars` characters.
#[must_use]
pub fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}... [truncated]")
    } else {
        truncated
    }
}

/// Build a `DetailedAssertionResult` from an assertion and its evaluation result.
fn build_detailed_assertion(
    assertion: &Assertion,
    passed: bool,
    error: Option<&str>,
) -> DetailedAssertionResult {
    DetailedAssertionResult {
        name: assertion.id().to_string(),
        desc: assertion.desc().map(ToString::to_string),
        assertion_type: assertion.type_name().to_string(),
        pattern: assertion.pattern().map(ToString::to_string),
        passed,
        error: error.map(ToString::to_string),
    }
}

/// Run all tests for a skill directory.
///
/// # Errors
/// Returns an error if test discovery or execution fails.
pub async fn run_skill_tests(
    skill_dir: &SkillDir,
    filter: Option<&str>,
) -> Result<SkillTestResult, RunnerError> {
    run_skill_tests_with_progress(skill_dir, filter, None, None).await
}

/// Run all tests for a skill directory with progress reporting.
///
/// # Arguments
/// * `skill_dir` - The skill directory to test
/// * `filter` - Optional filter for test case IDs
/// * `parallel` - Number of parallel test executions within the skill (None = sequential)
/// * `progress` - Optional progress sender for real-time updates
///
/// # Errors
/// Returns an error if test discovery or execution fails.
#[allow(clippy::too_many_lines)]
pub async fn run_skill_tests_with_progress(
    skill_dir: &SkillDir,
    filter: Option<&str>,
    parallel: Option<usize>,
    progress: Option<&ProgressSender>,
) -> Result<SkillTestResult, RunnerError> {
    // Discover test files
    let test_files = discover_test_files(
        &skill_dir.path,
        &skill_dir.config.test_patterns,
        &skill_dir.config.exclude_patterns,
    )?;

    if test_files.is_empty() {
        return Ok(SkillTestResult {
            name: skill_dir.name.clone(),
            path: skill_dir.path.clone(),
            tests: vec![],
            verdict: Verdict::Pass, // No tests = pass
        });
    }

    // Load and run all tests
    let skill_tests_dir = skill_dir.path.join("skill-tests");

    // First pass: count tests for progress reporting
    let mut total_tests = 0;
    let mut all_resolved = Vec::new();
    for test_file in &test_files {
        let resolved_tests = load_and_resolve_test_case(test_file, &skill_tests_dir)?;

        // Filter test cases if filter is specified
        let resolved_tests: Vec<_> = if let Some(filter_str) = filter {
            resolved_tests
                .into_iter()
                .filter(|(test_case, _, _)| test_case.id.contains(filter_str))
                .collect()
        } else {
            resolved_tests
        };

        total_tests += resolved_tests.len();
        all_resolved.push(resolved_tests);
    }

    // Emit skill started event
    if let Some(tx) = progress {
        let _ = tx.send(ProgressEvent::SkillStarted {
            skill_name: skill_dir.name.clone(),
            test_count: total_tests,
        });
    }

    // Flatten all resolved tests into a single list
    let all_test_cases: Vec<_> = all_resolved.into_iter().flatten().collect();

    // Run tests (parallel or sequential)
    let test_results = if let Some(n) = parallel {
        use futures::{StreamExt, stream};

        let n = n.max(1);
        let skill_name = skill_dir.name.clone();

        let futures =
            all_test_cases
                .into_iter()
                .map(|(test_case, assertions, golden_assertions)| {
                    let skill_dir = skill_dir.clone();
                    let progress = progress.cloned();
                    let skill_name = skill_name.clone();

                    async move {
                        let total_iterations =
                            test_case.iterations.unwrap_or(skill_dir.config.iterations);

                        // Emit test started event
                        if let Some(ref tx) = progress {
                            let _ = tx.send(ProgressEvent::TestStarted {
                                skill_name: skill_name.clone(),
                                test_id: test_case.id.clone(),
                                desc: test_case.desc.clone(),
                                total_iterations,
                            });
                        }

                        let (result, detailed) = run_simplified_test_case_with_progress(
                            &test_case,
                            &assertions,
                            &golden_assertions,
                            &skill_dir,
                            progress.as_ref(),
                        )
                        .await?;

                        // Emit test completed event
                        if let Some(ref tx) = progress {
                            let _ = tx.send(ProgressEvent::TestCompleted {
                                skill_name: skill_name.clone(),
                                test_id: result.id.clone(),
                                desc: test_case.desc.clone(),
                                result: result.clone(),
                                detailed,
                            });
                        }

                        Ok::<_, RunnerError>(result)
                    }
                });

        stream::iter(futures)
            .buffer_unordered(n)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
    } else {
        // Sequential execution
        let mut results = Vec::new();
        for (test_case, assertions, golden_assertions) in all_test_cases {
            let total_iterations = test_case.iterations.unwrap_or(skill_dir.config.iterations);

            // Emit test started event
            if let Some(tx) = progress {
                let _ = tx.send(ProgressEvent::TestStarted {
                    skill_name: skill_dir.name.clone(),
                    test_id: test_case.id.clone(),
                    desc: test_case.desc.clone(),
                    total_iterations,
                });
            }

            let (result, detailed) = run_simplified_test_case_with_progress(
                &test_case,
                &assertions,
                &golden_assertions,
                skill_dir,
                progress,
            )
            .await?;

            // Emit test completed event
            if let Some(tx) = progress {
                let _ = tx.send(ProgressEvent::TestCompleted {
                    skill_name: skill_dir.name.clone(),
                    test_id: result.id.clone(),
                    desc: test_case.desc.clone(),
                    result: result.clone(),
                    detailed,
                });
            }

            results.push(result);
        }
        results
    };

    // Determine overall skill verdict (all tests must pass)
    let all_passed = test_results.iter().all(|r| r.verdict == Verdict::Pass);
    let verdict = if all_passed {
        Verdict::Pass
    } else {
        Verdict::Fail
    };

    // Emit skill completed event
    if let Some(tx) = progress {
        let _ = tx.send(ProgressEvent::SkillCompleted {
            skill_name: skill_dir.name.clone(),
            verdict,
        });
    }

    Ok(SkillTestResult {
        name: skill_dir.name.clone(),
        path: skill_dir.path.clone(),
        tests: test_results,
        verdict,
    })
}

/// Get the default parallelism level (number of CPU cores).
#[must_use]
pub fn default_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

/// Run tests for multiple skill directories.
///
/// # Arguments
/// * `skill_dirs` - Skill directories to test
/// * `filter` - Optional filter for test case IDs
/// * `parallel` - Number of parallel test executions (None = sequential)
///
/// # Errors
/// Returns an error if any skill test execution fails.
pub async fn run_all_skill_tests(
    skill_dirs: &[SkillDir],
    filter: Option<&str>,
    parallel: Option<usize>,
) -> Result<(Vec<SkillTestResult>, SkillTestSummary), RunnerError> {
    run_all_skill_tests_with_progress(skill_dirs, filter, parallel, None).await
}

/// Run tests for multiple skill directories with progress reporting.
///
/// # Arguments
/// * `skill_dirs` - Skill directories to test
/// * `filter` - Optional filter for test case IDs
/// * `parallel` - Global limit on concurrent test executions (None = sequential)
/// * `progress` - Optional progress sender for real-time updates
///
/// # Note
/// All tests across all skills are flattened and run with a global concurrency
/// limit via `buffer_unordered(parallel)`. This ensures efficient parallelization
/// even when there are many skills with few tests each.
///
/// # Errors
/// Returns an error if any skill test execution fails.
#[allow(clippy::too_many_lines)]
pub async fn run_all_skill_tests_with_progress(
    skill_dirs: &[SkillDir],
    filter: Option<&str>,
    parallel: Option<usize>,
    progress: Option<ProgressSender>,
) -> Result<(Vec<SkillTestResult>, SkillTestSummary), RunnerError> {
    use futures::{StreamExt, stream};

    // Phase 1: Discover and load all test cases across all skills
    let mut all_test_items: Vec<(SkillDir, SimplifiedTestCase, Vec<Assertion>, Vec<Assertion>)> =
        Vec::new();

    for skill_dir in skill_dirs {
        let test_files = discover_test_files(
            &skill_dir.path,
            &skill_dir.config.test_patterns,
            &skill_dir.config.exclude_patterns,
        )?;

        let skill_tests_dir = skill_dir.path.join("skill-tests");

        for test_file in &test_files {
            let resolved_tests = load_and_resolve_test_case(test_file, &skill_tests_dir)?;

            for (test_case, assertions, golden_assertions) in resolved_tests {
                // Apply filter if specified
                if let Some(filter_str) = filter {
                    if !test_case.id.contains(filter_str) {
                        continue;
                    }
                }
                all_test_items.push((skill_dir.clone(), test_case, assertions, golden_assertions));
            }
        }
    }

    let total_tests = all_test_items.len();

    // Count tests per skill for progress tracking
    let mut tests_per_skill: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (skill_dir, _, _, _) in &all_test_items {
        *tests_per_skill.entry(skill_dir.name.clone()).or_insert(0) += 1;
    }

    // Emit global start event
    if let Some(ref tx) = progress {
        let _ = tx.send(ProgressEvent::AllTestsStarted {
            total_tests,
            skill_count: skill_dirs.len(),
        });

        // Emit SkillStarted for each skill so aggregator knows expected counts
        for (skill_name, test_count) in &tests_per_skill {
            let _ = tx.send(ProgressEvent::SkillStarted {
                skill_name: skill_name.clone(),
                test_count: *test_count,
            });
        }
    }

    // Phase 2: Run all tests with flattened parallelization
    let test_results: Vec<(String, SimplifiedTestResult)> = if let Some(n) = parallel {
        let n = n.max(1);

        let futures = all_test_items.into_iter().map(
            |(skill_dir, test_case, assertions, golden_assertions)| {
                let progress = progress.clone();
                let skill_name = skill_dir.name.clone();
                let test_id = test_case.id.clone();
                let test_desc = test_case.desc.clone();

                async move {
                    let total_iterations =
                        test_case.iterations.unwrap_or(skill_dir.config.iterations);

                    // Emit test started event
                    if let Some(ref tx) = progress {
                        let _ = tx.send(ProgressEvent::TestStarted {
                            skill_name: skill_name.clone(),
                            test_id: test_id.clone(),
                            desc: test_desc.clone(),
                            total_iterations,
                        });
                    }

                    let (result, detailed) = run_simplified_test_case_with_progress(
                        &test_case,
                        &assertions,
                        &golden_assertions,
                        &skill_dir,
                        progress.as_ref(),
                    )
                    .await?;

                    // Emit test completed event
                    if let Some(ref tx) = progress {
                        let _ = tx.send(ProgressEvent::TestCompleted {
                            skill_name: skill_name.clone(),
                            test_id: result.id.clone(),
                            desc: test_case.desc.clone(),
                            result: result.clone(),
                            detailed,
                        });
                    }

                    Ok::<_, RunnerError>((skill_name, result))
                }
            },
        );

        stream::iter(futures)
            .buffer_unordered(n)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
    } else {
        // Sequential execution
        let mut results = Vec::new();
        for (skill_dir, test_case, assertions, golden_assertions) in all_test_items {
            let skill_name = skill_dir.name.clone();
            let total_iterations = test_case.iterations.unwrap_or(skill_dir.config.iterations);

            // Emit test started event
            if let Some(ref tx) = progress {
                let _ = tx.send(ProgressEvent::TestStarted {
                    skill_name: skill_name.clone(),
                    test_id: test_case.id.clone(),
                    desc: test_case.desc.clone(),
                    total_iterations,
                });
            }

            let (result, detailed) = run_simplified_test_case_with_progress(
                &test_case,
                &assertions,
                &golden_assertions,
                &skill_dir,
                progress.as_ref(),
            )
            .await?;

            // Emit test completed event
            if let Some(ref tx) = progress {
                let _ = tx.send(ProgressEvent::TestCompleted {
                    skill_name: skill_name.clone(),
                    test_id: result.id.clone(),
                    desc: test_case.desc.clone(),
                    result: result.clone(),
                    detailed,
                });
            }

            results.push((skill_name, result));
        }
        results
    };

    // Phase 3: Group results by skill
    let mut skill_results_map: std::collections::HashMap<String, Vec<SimplifiedTestResult>> =
        std::collections::HashMap::new();

    for (skill_name, result) in test_results {
        skill_results_map
            .entry(skill_name)
            .or_default()
            .push(result);
    }

    // Build SkillTestResult for each skill
    let mut results: Vec<SkillTestResult> = skill_dirs
        .iter()
        .map(|skill_dir| {
            let tests = skill_results_map
                .remove(&skill_dir.name)
                .unwrap_or_default();
            let all_passed = tests.iter().all(|t| t.verdict == Verdict::Pass);
            let verdict = if all_passed {
                Verdict::Pass
            } else {
                Verdict::Fail
            };

            SkillTestResult {
                name: skill_dir.name.clone(),
                path: skill_dir.path.clone(),
                tests,
                verdict,
            }
        })
        .collect();

    // Emit SkillCompleted events for each skill
    if let Some(ref tx) = progress {
        for result in &results {
            let _ = tx.send(ProgressEvent::SkillCompleted {
                skill_name: result.name.clone(),
                verdict: result.verdict,
            });
        }
    }

    // Sort results by skill name for deterministic output ordering
    results.sort_by(|a, b| a.name.cmp(&b.name));

    let total_skills = results.len();
    let passed_skills = results
        .iter()
        .filter(|r| r.verdict == Verdict::Pass)
        .count();
    let failed_skills = total_skills - passed_skills;

    let total_tests: usize = results.iter().map(|r| r.tests.len()).sum();
    let passed_tests: usize = results
        .iter()
        .flat_map(|r| &r.tests)
        .filter(|t| t.verdict == Verdict::Pass)
        .count();
    let failed_tests = total_tests - passed_tests;

    let summary = SkillTestSummary {
        total_skills,
        passed_skills,
        failed_skills,
        total_tests,
        passed_tests,
        failed_tests,
    };

    Ok((results, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_from_results() {
        let results = vec![
            TestResult {
                test_id: "test-1".into(),
                iteration: 1,
                prompt: String::new(),
                expected_skills: vec![],
                called_skills: vec![],
                called_tools: vec![],
                output_text: String::new(),
                output_hash: String::new(),
                skill_passed: true,
                contract_passed: Some(true),
                golden_passed: None,
                failures: vec![],
                golden_failures: vec![],
                verdict: Verdict::Pass,
                latency_ms: 1000,
            },
            TestResult {
                test_id: "test-1".into(),
                iteration: 2,
                prompt: String::new(),
                expected_skills: vec![],
                called_skills: vec![],
                called_tools: vec![],
                output_text: String::new(),
                output_hash: String::new(),
                skill_passed: false,
                contract_passed: None,
                golden_passed: None,
                failures: vec![],
                golden_failures: vec![],
                verdict: Verdict::Fail,
                latency_ms: 1500,
            },
            TestResult {
                test_id: "test-1".into(),
                iteration: 3,
                prompt: String::new(),
                expected_skills: vec![],
                called_skills: vec![],
                called_tools: vec![],
                output_text: String::new(),
                output_hash: String::new(),
                skill_passed: true,
                contract_passed: Some(true),
                golden_passed: None,
                failures: vec![],
                golden_failures: vec![],
                verdict: Verdict::Warn,
                latency_ms: 1200,
            },
        ];

        let summary = TestSummary::from_results(&results);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warned, 1);
        // Pass rate: (1 pass + 1 warn) / 3 total = 66.67%
        assert!((summary.pass_rate - 66.67).abs() < 0.01);
    }

    #[test]
    fn test_meets_threshold() {
        let summary = TestSummary {
            total: 10,
            passed: 7,
            failed: 1,
            warned: 2,
            pass_rate: 90.0,
        };

        assert!(summary.meets_threshold(80));
        assert!(summary.meets_threshold(90));
        assert!(!summary.meets_threshold(91));
    }

    #[test]
    fn test_compute_avg_latency() {
        let results = vec![
            TestResult {
                test_id: "test".into(),
                iteration: 1,
                prompt: String::new(),
                expected_skills: vec![],
                called_skills: vec![],
                called_tools: vec![],
                output_text: String::new(),
                output_hash: String::new(),
                skill_passed: true,
                contract_passed: None,
                golden_passed: None,
                failures: vec![],
                golden_failures: vec![],
                verdict: Verdict::Pass,
                latency_ms: 1000,
            },
            TestResult {
                test_id: "test".into(),
                iteration: 2,
                prompt: String::new(),
                expected_skills: vec![],
                called_skills: vec![],
                called_tools: vec![],
                output_text: String::new(),
                output_hash: String::new(),
                skill_passed: true,
                contract_passed: None,
                golden_passed: None,
                failures: vec![],
                golden_failures: vec![],
                verdict: Verdict::Pass,
                latency_ms: 2000,
            },
        ];

        let avg = compute_avg_latency(&results);
        assert_eq!(avg, Some(1500.0));
    }

    #[test]
    fn test_compute_avg_latency_empty() {
        let avg = compute_avg_latency(&[]);
        assert_eq!(avg, None);
    }

    #[test]
    fn test_truncate_output_short() {
        let output = "Short output";
        let truncated = truncate_output(output, 100);
        assert_eq!(truncated, "Short output");
    }

    #[test]
    fn test_truncate_output_long() {
        let output = "This is a very long output that should be truncated at the specified length";
        let truncated = truncate_output(output, 20);
        assert_eq!(truncated, "This is a very long ...");
    }

    #[test]
    fn test_truncate_output_multiline() {
        let output = "First line\nSecond line\nThird line";
        let truncated = truncate_output(output, 100);
        // Only first line is returned
        assert_eq!(truncated, "First line");
    }

    #[test]
    fn test_truncate_output_error_message() {
        // Simulate a timeout error message (multiline with long content)
        let error = "timeout after 60000ms\nprompt: Find Claude Code skills\npartial_output: [process killed]";
        let truncated = truncate_output(error, 30);
        // Only first line, truncated
        assert_eq!(truncated, "timeout after 60000ms");
    }

    #[test]
    fn test_truncate_utf8_short() {
        let text = "Short text";
        let truncated = truncate_utf8(text, 100);
        assert_eq!(truncated, "Short text");
    }

    #[test]
    fn test_truncate_utf8_long() {
        let text = "This is a longer text that exceeds the limit";
        let truncated = truncate_utf8(text, 20);
        assert_eq!(truncated, "This is a longer tex... [truncated]");
    }

    #[test]
    fn test_truncate_utf8_multiline_preserved() {
        // Unlike truncate_output, truncate_utf8 preserves all lines
        let text = "Line 1\nLine 2\nLine 3";
        let truncated = truncate_utf8(text, 100);
        assert_eq!(truncated, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_truncate_utf8_japanese() {
        // Test with Japanese characters (multi-byte UTF-8)
        let text = "日本語テキストの例です";
        let truncated = truncate_utf8(text, 5);
        // Should truncate at character boundary, not byte boundary
        assert_eq!(truncated, "日本語テキ... [truncated]");
    }

    #[test]
    fn test_truncate_utf8_exact_boundary() {
        let text = "12345";
        let truncated = truncate_utf8(text, 5);
        // Exactly at limit, no truncation marker
        assert_eq!(truncated, "12345");
    }
}
