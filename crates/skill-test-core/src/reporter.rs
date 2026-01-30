//! Test output reporter with cargo test-like formatting.

use crate::types::{SimplifiedTestResult, SkillTestResult, SkillTestSummary, Verdict};
use std::io::{self, Write};
use std::time::Duration;

/// Reporter configuration.
#[derive(Debug, Clone)]
pub struct ReporterConfig {
    /// Show verbose output (detailed assertion info).
    pub verbose: bool,
    /// Use colors in output.
    pub color: bool,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            color: true,
        }
    }
}

/// Test reporter with cargo test-like output.
#[derive(Clone)]
pub struct Reporter {
    config: ReporterConfig,
}

impl Reporter {
    /// Create a new reporter with the given configuration.
    #[must_use]
    pub const fn new(config: ReporterConfig) -> Self {
        Self { config }
    }

    /// Print the start of a skill test run.
    pub fn skill_start(&self, skill_name: &str, test_count: usize) {
        println!();
        println!("running {test_count} tests for {skill_name}");
    }

    /// Print a test result line.
    pub fn test_result(&self, test: &SimplifiedTestResult, desc: Option<&str>) {
        let display_name = desc.unwrap_or(&test.id);
        let status = match test.verdict {
            Verdict::Pass => {
                if self.config.color {
                    "\x1b[32mok\x1b[0m"
                } else {
                    "ok"
                }
            }
            Verdict::Fail => {
                if self.config.color {
                    "\x1b[31mFAILED\x1b[0m"
                } else {
                    "FAILED"
                }
            }
            Verdict::Warn => {
                if self.config.color {
                    "\x1b[33mwarn\x1b[0m"
                } else {
                    "warn"
                }
            }
        };

        println!("test {display_name} ... {status}");

        // In verbose mode, show pass rate
        if self.config.verbose {
            println!(
                "     ({}/{} iterations passed, {:.1}%)",
                test.passed, test.iterations, test.pass_rate
            );
        }
    }

    /// Print failures section.
    pub fn failures(&self, results: &[SkillTestResult]) {
        let has_failures = results
            .iter()
            .any(|r| r.tests.iter().any(|t| t.verdict == Verdict::Fail));

        if !has_failures {
            return;
        }

        println!();
        println!("failures:");
        println!();

        for skill_result in results {
            for test in &skill_result.tests {
                if test.verdict != Verdict::Fail {
                    continue;
                }
                println!("---- {}::{} ----", skill_result.name, test.id);
                for failure in &test.failures {
                    println!("    {failure}");
                }
                if !test.golden_failures.is_empty() {
                    println!("  golden (non-blocking):");
                    for failure in &test.golden_failures {
                        println!("    {failure}");
                    }
                }
                println!();
            }
        }
    }

    /// Print the final summary.
    pub fn summary(&self, summary: &SkillTestSummary, duration: Duration) {
        let status = if summary.failed_tests == 0 {
            if self.config.color {
                "\x1b[32mok\x1b[0m"
            } else {
                "ok"
            }
        } else if self.config.color {
            "\x1b[31mFAILED\x1b[0m"
        } else {
            "FAILED"
        };

        println!();
        println!(
            "test result: {}. {} passed; {} failed; finished in {:.1}s",
            status,
            summary.passed_tests,
            summary.failed_tests,
            duration.as_secs_f64()
        );
    }

    /// Print verbose output for a test iteration (only in verbose mode).
    pub fn verbose_iteration(&self, test_id: &str, iteration: u32, message: &str) {
        if self.config.verbose {
            println!("  [{test_id}:{iteration}] {message}");
        }
    }

    /// Print verbose assertion result (only in verbose mode).
    pub fn verbose_assertion(&self, assertion_id: &str, passed: bool) {
        if self.config.verbose {
            let status = if passed {
                if self.config.color {
                    "\x1b[32m✓\x1b[0m"
                } else {
                    "ok"
                }
            } else if self.config.color {
                "\x1b[31m✗\x1b[0m"
            } else {
                "FAILED"
            };
            println!("    {status} {assertion_id}");
        }
    }

    /// Print a warning message.
    pub fn warn(&self, message: &str) {
        if self.config.color {
            eprintln!("\x1b[33mwarning\x1b[0m: {message}");
        } else {
            eprintln!("warning: {message}");
        }
    }

    /// Print an error message.
    pub fn error(&self, message: &str) {
        if self.config.color {
            eprintln!("\x1b[31merror\x1b[0m: {message}");
        } else {
            eprintln!("error: {message}");
        }
    }

    /// Flush stdout.
    pub fn flush(&self) {
        let _ = io::stdout().flush();
    }
}
