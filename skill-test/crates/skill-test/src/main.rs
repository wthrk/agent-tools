//! skill-test CLI - Test runner for Claude Code skills.

use clap::Parser;
use comfy_table::{Cell, Color, Table};
use skill_test_core::{
    ConfigOverrides, DetailedTestResult, ExecutionReport, HookType, ProgressEvent, ReportFormat,
    Reporter, ReporterConfig, SimplifiedTestResult, SkillResult, SkillTestResult, SkillTestSummary,
    Verdict, apply_overrides, default_parallelism, detect_skill_dirs, resolve_skill_paths,
    run_all_skill_tests_with_progress,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use time::OffsetDateTime;
use time::macros::format_description;
use tokio::sync::mpsc;

// =============================================================================
// Progress Handling Components
// =============================================================================

/// Handles console output for table format.
/// Encapsulates `enabled` and `verbose` conditionals.
struct TablePrinter {
    reporter: Reporter,
    verbose: bool,
    enabled: bool,
}

impl TablePrinter {
    const fn new(reporter: Reporter, verbose: bool, enabled: bool) -> Self {
        Self {
            reporter,
            verbose,
            enabled,
        }
    }

    fn on_tests_started(&self, total: usize) {
        if !self.enabled {
            return;
        }
        println!("running {total} tests\n");
        self.reporter.flush();
    }

    fn on_test_started(&self, test_id: &str, desc: Option<&str>) {
        if !self.enabled || !self.verbose {
            return;
        }
        let display = desc.unwrap_or(test_id);
        println!("  starting: {display}");
        self.reporter.flush();
    }

    fn on_iteration_started(&self, test_id: &str, iteration: u32, total: u32) {
        if !self.enabled || !self.verbose {
            return;
        }
        self.reporter.verbose_iteration(
            test_id,
            iteration,
            &format!("running ({iteration}/{total})"),
        );
    }

    fn on_iteration_completed(
        &self,
        test_id: &str,
        iteration: u32,
        passed: bool,
        latency_ms: u64,
        output_preview: &str,
    ) {
        if !self.enabled || !self.verbose {
            return;
        }
        let status = if passed { "passed" } else { "failed" };
        self.reporter.verbose_iteration(
            test_id,
            iteration,
            &format!("{status} ({latency_ms}ms): {output_preview}"),
        );
    }

    fn on_assertion_result(
        &self,
        assertion_id: &str,
        assertion_desc: Option<&str>,
        passed: bool,
        is_golden: bool,
    ) {
        if !self.enabled || !self.verbose {
            return;
        }
        let display = assertion_desc.unwrap_or(assertion_id);
        let golden_marker = if is_golden { " (golden)" } else { "" };
        self.reporter
            .verbose_assertion(&format!("{display}{golden_marker}"), passed);
    }

    fn on_test_completed(&self, test_id: &str, desc: Option<&str>, result: &SimplifiedTestResult) {
        if !self.enabled {
            return;
        }
        let display = desc.unwrap_or(test_id);
        let status = if result.verdict == Verdict::Pass {
            "\x1b[32mok\x1b[0m"
        } else {
            "\x1b[31mFAILED\x1b[0m"
        };
        println!("test {display} ... {status}");

        // Print failures if any
        for failure in &result.failures {
            println!("  \x1b[31m✗\x1b[0m {failure}");
        }
        for failure in &result.golden_failures {
            println!("  \x1b[33m⚠\x1b[0m {failure} (golden)");
        }
        self.reporter.flush();
    }

    fn on_error(&self, skill_name: &str, error: &str) {
        if !self.enabled {
            return;
        }
        eprintln!("\x1b[31m[ERROR]\x1b[0m {skill_name}: {error}");
        self.reporter.flush();
    }

    fn on_finished(&self) {
        if !self.enabled {
            return;
        }
        println!();
    }
}

/// Handles error log file writing.
/// Encapsulates `enabled` and `silent` conditionals.
struct ErrorLogWriter {
    enabled: bool,
    silent: bool,
    skill_path_map: HashMap<String, PathBuf>,
}

impl ErrorLogWriter {
    #[allow(clippy::missing_const_for_fn)]
    fn new(enabled: bool, silent: bool, skill_path_map: HashMap<String, PathBuf>) -> Self {
        Self {
            enabled,
            silent,
            skill_path_map,
        }
    }

    fn write_log(
        &self,
        skill_name: &str,
        completed: &[DetailedTestResult],
        partial: &[DetailedTestResult],
        verdict: Verdict,
        error: Option<&str>,
    ) {
        if !self.enabled {
            return;
        }
        let Some(skill_path) = self.skill_path_map.get(skill_name) else {
            return;
        };
        // Skip if no test results
        if completed.is_empty() && partial.is_empty() {
            return;
        }
        if let Ok(path) =
            write_skill_error_log(skill_name, skill_path, completed, partial, verdict, error)
        {
            if !self.silent {
                eprintln!("\x1b[33m[ERRLOG]\x1b[0m {}", path.display());
            }
        }
    }
}

/// Collects detailed test results with order preservation.
struct DetailedCollector {
    results: HashMap<String, Vec<DetailedTestResult>>,
    /// Preserves insertion order of test IDs per skill
    test_order: HashMap<String, Vec<String>>,
}

impl DetailedCollector {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
            test_order: HashMap::new(),
        }
    }

    fn add(&mut self, skill_name: &str, result: DetailedTestResult) {
        let test_id = result.name.clone();
        self.test_order
            .entry(skill_name.to_string())
            .or_default()
            .push(test_id);
        self.results
            .entry(skill_name.to_string())
            .or_default()
            .push(result);
    }

    fn get(&self, skill_name: &str) -> Vec<DetailedTestResult> {
        self.results.get(skill_name).cloned().unwrap_or_default()
    }

    /// Returns results sorted by test ID for deterministic output
    fn into_sorted_results(mut self) -> HashMap<String, Vec<DetailedTestResult>> {
        for tests in self.results.values_mut() {
            tests.sort_by(|a, b| a.name.cmp(&b.name));
        }
        self.results
    }
}

/// Exit codes for the CLI.
mod exit_code {
    pub const SUCCESS: u8 = 0;
    pub const THRESHOLD_NOT_MET: u8 = 1;
    pub const CONFIG_ERROR: u8 = 2;
    pub const EXECUTION_ERROR: u8 = 3;
}

/// Process-global sequence number for log filename collision avoidance.
static LOG_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// Timestamp for both filename and JSON, computed once.
struct LogTimestamp {
    filename: String, // "YYYYMMDD-HHMMSS-mmm"
    iso8601: String,  // "YYYY-MM-DDTHH:MM:SS.mmmZ"
}

impl LogTimestamp {
    fn now() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            filename: format!(
                "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
                now.year(),
                now.month() as u8,
                now.day(),
                now.hour(),
                now.minute(),
                now.second(),
                now.millisecond()
            ),
            iso8601: now
                .format(&format_description!(
                    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
                ))
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

/// Write skill execution log to `.skill-test-logs/` directory.
///
/// The output uses `ExecutionReport` format (same schema as JSON output).
/// Returns the path to the written file on success.
fn write_skill_error_log(
    skill_name: &str,
    skill_path: &Path,
    completed_tests: &[DetailedTestResult],
    partial_tests: &[DetailedTestResult],
    verdict: Verdict,
    error: Option<&str>,
) -> Result<PathBuf, io::Error> {
    let log_dir = skill_path.join(".skill-test-logs");
    fs::create_dir_all(&log_dir)?;

    // Timestamp computed once for both filename and JSON
    let ts = LogTimestamp::now();
    let seq = LOG_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let filename = format!("{}-{seq:04}.json", ts.filename);
    let path = log_dir.join(&filename);

    // Union of completed + partial (no duplicates expected)
    let mut all_tests = completed_tests.to_vec();
    all_tests.extend(partial_tests.iter().cloned());

    // Build SkillResult
    let skill_result = SkillResult {
        skill_name: skill_name.to_string(),
        skill_path: skill_path.display().to_string(),
        tests: all_tests,
        verdict,
        error: error.map(ToString::to_string),
    };

    // Wrap in ExecutionReport (same schema as JSON output)
    let report = ExecutionReport {
        timestamp: ts.iso8601,
        skills: vec![skill_result.clone()],
        summary: SkillTestSummary::from_single(&skill_result),
    };

    let json = serde_json::to_string_pretty(&report).map_err(io::Error::other)?;
    fs::write(&path, json)?;

    Ok(path)
}

#[derive(Parser)]
#[command(name = "skill-test")]
#[command(about = "Test runner for Claude Code skills")]
#[command(version)]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    /// Skill directories to test (default: current directory)
    /// Each directory must contain a SKILL.md file.
    #[arg(value_name = "SKILL_DIR")]
    skill_dirs: Vec<PathBuf>,

    /// Number of iterations per test (overrides config)
    #[arg(long)]
    iterations: Option<u32>,

    /// Hook strategy: none, simple, forced, custom
    #[arg(long)]
    hook: Option<String>,

    /// Path to custom hook script (required when hook=custom)
    #[arg(long)]
    hook_path: Option<PathBuf>,

    /// Model to use (overrides config)
    #[arg(long)]
    model: Option<String>,

    /// Timeout in milliseconds (overrides config)
    #[arg(long)]
    timeout: Option<u64>,

    /// Pass rate threshold percentage (overrides config)
    #[arg(long)]
    threshold: Option<u32>,

    /// Strict mode: error on missing files (overrides config)
    #[arg(long)]
    strict: bool,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Output format: table, json
    #[arg(long, default_value = "table")]
    format: String,

    /// Filter test cases by ID (substring match)
    #[arg(long)]
    filter: Option<String>,

    /// Number of parallel test executions (default: CPU count, 0 = sequential)
    #[arg(short, long)]
    parallel: Option<usize>,

    /// Disable error log file output
    #[arg(long)]
    no_error_log: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    run_command(&cli).await
}

fn parse_hook_type(hook: &str) -> Result<HookType, String> {
    match hook {
        "none" => Ok(HookType::None),
        "simple" => Ok(HookType::Simple),
        "forced" => Ok(HookType::Forced),
        "custom" => Ok(HookType::Custom),
        other => Err(format!(
            "Invalid hook type: {other}. Valid values: none, simple, forced, custom"
        )),
    }
}

fn print_skill_results(results: &[SkillTestResult], verbose: bool) {
    for skill_result in results {
        println!("\n=== Skill: {} ===", skill_result.name);
        println!("Path: {}", skill_result.path.display());

        for test_result in &skill_result.tests {
            let status = match test_result.verdict {
                Verdict::Pass => "✓",
                Verdict::Fail => "✗",
                Verdict::Warn => "⚠",
            };
            println!(
                "  {} {}: {}/{} ({:.1}%)",
                status,
                test_result.id,
                test_result.passed,
                test_result.iterations,
                test_result.pass_rate
            );

            if verbose && !test_result.failures.is_empty() {
                for failure in &test_result.failures {
                    println!("      Failure: {failure}");
                }
            }

            if verbose && !test_result.golden_failures.is_empty() {
                for failure in &test_result.golden_failures {
                    println!("      Golden: {failure}");
                }
            }
        }

        let verdict_str = match skill_result.verdict {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Warn => "WARN",
        };
        println!("  Verdict: {verdict_str}");
    }
}

fn print_results_table(results: &[SkillTestResult]) {
    let mut table = Table::new();
    table.set_header(vec![
        "Skill",
        "Test ID",
        "Iterations",
        "Passed",
        "Pass Rate",
        "Verdict",
    ]);

    for skill_result in results {
        for test_result in &skill_result.tests {
            let verdict_cell = match test_result.verdict {
                Verdict::Pass => Cell::new("Pass").fg(Color::Green),
                Verdict::Fail => Cell::new("Fail").fg(Color::Red),
                Verdict::Warn => Cell::new("Warn").fg(Color::Yellow),
            };

            table.add_row(vec![
                Cell::new(&skill_result.name),
                Cell::new(&test_result.id),
                Cell::new(test_result.iterations),
                Cell::new(test_result.passed),
                Cell::new(format!("{:.1}%", test_result.pass_rate)),
                verdict_cell,
            ]);
        }
    }

    println!("{table}");
}

fn print_summary(summary: &SkillTestSummary) {
    println!("\n=== Summary ===");
    println!(
        "Skills: {}/{} passed",
        summary.passed_skills, summary.total_skills
    );
    println!(
        "Tests:  {}/{} passed",
        summary.passed_tests, summary.total_tests
    );
}

/// Build `ExecutionReport` from test results.
/// This is the unified format for both JSON output and error logs.
fn build_execution_report(
    timestamp: &str,
    results: &[SkillTestResult],
    summary: &SkillTestSummary,
    detailed_results: &HashMap<String, Vec<DetailedTestResult>>,
) -> ExecutionReport {
    let skills: Vec<SkillResult> = results
        .iter()
        .map(|r| {
            let tests = detailed_results.get(&r.name).cloned().unwrap_or_default();
            SkillResult {
                skill_name: r.name.clone(),
                skill_path: r.path.display().to_string(),
                tests,
                verdict: r.verdict,
                error: None, // No execution error for completed skills
            }
        })
        .collect();

    ExecutionReport {
        timestamp: timestamp.to_string(),
        skills,
        summary: summary.clone(),
    }
}

/// Format results as JSON using `ExecutionReport` schema.
/// This is the same schema used for error logs.
fn format_results_json(report: &ExecutionReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
async fn run_command(cli: &Cli) -> ExitCode {
    let reporter = Reporter::new(ReporterConfig {
        verbose: cli.verbose,
        color: !cli.no_color,
    });

    // Parse format
    let report_format: ReportFormat = match cli.format.parse() {
        Ok(f) => f,
        Err(e) => {
            reporter.error(&format!("Invalid format: {e}"));
            return ExitCode::from(exit_code::CONFIG_ERROR);
        }
    };

    // Resolve skill paths (default to current directory if none specified)
    let skill_paths = match resolve_skill_paths(&cli.skill_dirs) {
        Ok(paths) => paths,
        Err(e) => {
            reporter.error(&format!("Failed to resolve skill paths: {e}"));
            return ExitCode::from(exit_code::CONFIG_ERROR);
        }
    };

    // Detect and validate skill directories
    let mut skill_dirs = match detect_skill_dirs(&skill_paths) {
        Ok(dirs) => dirs,
        Err(e) => {
            reporter.error(&format!("Failed to detect skill directories: {e}"));
            return ExitCode::from(exit_code::CONFIG_ERROR);
        }
    };

    if skill_dirs.is_empty() {
        reporter.error("No skill directories found");
        return ExitCode::from(exit_code::CONFIG_ERROR);
    }

    // Parse hook type if specified
    let hook_type = if let Some(ref hook) = cli.hook {
        match parse_hook_type(hook) {
            Ok(h) => Some(h),
            Err(e) => {
                reporter.error(&e);
                return ExitCode::from(exit_code::CONFIG_ERROR);
            }
        }
    } else {
        None
    };

    // Validate custom hook requirements
    if hook_type == Some(HookType::Custom) && cli.hook_path.is_none() {
        reporter.error("--hook-path is required when --hook=custom");
        return ExitCode::from(exit_code::CONFIG_ERROR);
    }

    if cli.hook_path.is_some() && hook_type != Some(HookType::Custom) {
        reporter.error("--hook-path should only be used with --hook=custom");
        return ExitCode::from(exit_code::CONFIG_ERROR);
    }

    // Apply CLI overrides to each skill's config
    let overrides = ConfigOverrides {
        model: cli.model.clone(),
        timeout: cli.timeout,
        iterations: cli.iterations,
        threshold: cli.threshold,
        hook: hook_type,
        hook_path: cli.hook_path.clone(),
        strict: if cli.strict { Some(true) } else { None },
    };

    for skill_dir in &mut skill_dirs {
        skill_dir.config = apply_overrides(skill_dir.config.clone(), &overrides);
    }

    let show_progress = report_format == ReportFormat::Table;
    let start_time = Instant::now();

    // Determine parallelism: default = CPU count, 0 = sequential
    // Note: parallel controls concurrent test case executions within each skill,
    // not skill-level parallelism (skills are processed sequentially).
    let parallel = match cli.parallel {
        Some(0) => None, // Explicit sequential
        Some(n) => Some(n),
        None => Some(default_parallelism()), // Default: CPU count
    };

    // Create skill_path_map for error logging
    let skill_path_map: HashMap<String, PathBuf> = skill_dirs
        .iter()
        .map(|s| (s.name.clone(), s.path.clone()))
        .collect();

    if show_progress {
        let parallel_str = parallel.map_or_else(String::new, |n| format!(" (parallel: {n})"));
        println!(
            "Running tests for {} skill(s){}...\n",
            skill_dirs.len(),
            parallel_str
        );
        for skill_dir in &skill_dirs {
            println!("  - {} ({})", skill_dir.name, skill_dir.path.display());
        }
        println!();
    }

    // Set up progress channel for real-time output
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressEvent>();

    // Create progress handling components
    let table_printer = TablePrinter::new(reporter.clone(), cli.verbose, show_progress);
    let error_logger =
        ErrorLogWriter::new(!cli.no_error_log, !show_progress, skill_path_map.clone());

    // Spawn progress handler task
    let progress_handle = tokio::spawn(async move {
        let mut collector = DetailedCollector::new();

        while let Some(event) = progress_rx.recv().await {
            match event {
                ProgressEvent::AllTestsStarted { total_tests, .. } => {
                    table_printer.on_tests_started(total_tests);
                }
                ProgressEvent::SkillStarted { .. } => {}
                ProgressEvent::SkillCompleted {
                    skill_name,
                    verdict,
                } => {
                    if verdict == Verdict::Fail {
                        let completed = collector.get(&skill_name);
                        error_logger.write_log(&skill_name, &completed, &[], verdict, None);
                    }
                }
                ProgressEvent::TestStarted { test_id, desc, .. } => {
                    table_printer.on_test_started(&test_id, desc.as_deref());
                }
                ProgressEvent::IterationStarted {
                    test_id,
                    iteration,
                    total,
                    ..
                } => {
                    table_printer.on_iteration_started(&test_id, iteration, total);
                }
                ProgressEvent::IterationCompleted {
                    test_id,
                    iteration,
                    passed,
                    output_preview,
                    latency_ms,
                    ..
                } => {
                    table_printer.on_iteration_completed(
                        &test_id,
                        iteration,
                        passed,
                        latency_ms,
                        &output_preview,
                    );
                }
                ProgressEvent::AssertionResult {
                    assertion_id,
                    assertion_desc,
                    passed,
                    is_golden,
                    ..
                } => {
                    table_printer.on_assertion_result(
                        &assertion_id,
                        assertion_desc.as_deref(),
                        passed,
                        is_golden,
                    );
                }
                ProgressEvent::TestCompleted {
                    skill_name,
                    test_id,
                    desc,
                    result,
                    detailed,
                } => {
                    collector.add(&skill_name, detailed);
                    table_printer.on_test_completed(&test_id, desc.as_deref(), &result);
                }
                ProgressEvent::SkillError {
                    skill_name,
                    error,
                    partial_results,
                } => {
                    let completed = collector.get(&skill_name);
                    error_logger.write_log(
                        &skill_name,
                        &completed,
                        &partial_results,
                        Verdict::Fail,
                        Some(&error),
                    );
                    table_printer.on_error(&skill_name, &error);
                }
            }
        }

        table_printer.on_finished();
        collector.into_sorted_results()
    });

    // Run all tests - always send progress for detailed result collection
    let progress_sender = Some(progress_tx);

    let (results, summary) = match run_all_skill_tests_with_progress(
        &skill_dirs,
        cli.filter.as_deref(),
        parallel,
        progress_sender,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            reporter.error(&format!("Test execution failed: {e}"));
            return ExitCode::from(exit_code::EXECUTION_ERROR);
        }
    };

    // Wait for progress handler to finish and get detailed results
    let detailed_results = progress_handle.await.unwrap_or_default();

    if results.is_empty() {
        if cli.filter.is_some() {
            reporter.warn(&format!(
                "No tests match filter '{}'",
                cli.filter.as_deref().unwrap_or("")
            ));
        } else {
            reporter.warn("No tests found in any skill directory");
        }
        return ExitCode::from(exit_code::SUCCESS);
    }

    let duration = start_time.elapsed();

    // Output based on format
    match report_format {
        ReportFormat::Json => {
            let ts = LogTimestamp::now();
            let report = build_execution_report(&ts.iso8601, &results, &summary, &detailed_results);
            println!("{}", format_results_json(&report));
        }
        ReportFormat::Table => {
            print_skill_results(&results, cli.verbose);
            print_summary(&summary);
            reporter.summary(&summary, duration);

            if cli.verbose {
                println!("\n=== Detailed Results ===");
                print_results_table(&results);
            }
        }
    }

    // Determine exit code based on overall pass/fail
    if summary.failed_skills == 0 {
        ExitCode::from(exit_code::SUCCESS)
    } else {
        ExitCode::from(exit_code::THRESHOLD_NOT_MET)
    }
}
