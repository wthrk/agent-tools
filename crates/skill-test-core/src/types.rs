//! Core data types for skill-test.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during validation.
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("line_count assertion requires at least min or max")]
    LineCountMissingBounds,
    #[error("line_count assertion has min ({min}) > max ({max})")]
    LineCountMinGreaterThanMax { min: usize, max: usize },
    #[error("duplicate assertion id: {0}")]
    DuplicateAssertionId(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(String),
}

/// Match policy for expected skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchPolicy {
    #[default]
    All,
    Any,
}

/// Inline validation for test cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validation {
    #[serde(default)]
    pub assertions: Vec<Assertion>,
}

/// A test case definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    pub id: String,
    pub prompt: String,
    pub expected_skills: Vec<String>,
    #[serde(default)]
    pub match_policy: MatchPolicy,
    #[serde(default)]
    pub forbid_skills: Vec<String>,
    pub iterations: Option<u32>,
    /// Inline validation assertions (evaluated after contract assertions).
    pub validation: Option<Validation>,
}

/// Expectation for pattern assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternExpect {
    Present,
    Absent,
}

/// A regex assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegexAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: PatternExpect,
}

/// A contains assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainsAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: PatternExpect,
}

/// A line count assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineCountAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl LineCountAssertion {
    /// Create a new line count assertion with validation.
    ///
    /// # Errors
    /// Returns `ValidationError::LineCountMissingBounds` if both min and max are None.
    /// Returns `ValidationError::LineCountMinGreaterThanMax` if min > max.
    pub fn new(id: &str, min: Option<usize>, max: Option<usize>) -> Result<Self, ValidationError> {
        if min.is_none() && max.is_none() {
            return Err(ValidationError::LineCountMissingBounds);
        }
        if let (Some(min_val), Some(max_val)) = (min, max) {
            if min_val > max_val {
                return Err(ValidationError::LineCountMinGreaterThanMax {
                    min: min_val,
                    max: max_val,
                });
            }
        }
        Ok(Self {
            id: id.to_string(),
            desc: None,
            min,
            max,
        })
    }

    /// Check if the line count is in range.
    #[must_use]
    pub fn check(&self, line_count: usize) -> bool {
        let min_ok = self.min.is_none_or(|m| line_count >= m);
        let max_ok = self.max.is_none_or(|m| line_count <= m);
        min_ok && max_ok
    }
}

/// Expectation for exec assertions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecExpect {
    #[serde(rename = "exit_code:0")]
    ExitCodeZero,
    #[serde(untagged)]
    OutputContains { output_contains: String },
}

/// An exec assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    pub command: String,
    pub language: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    pub expect: ExecExpect,
}

const fn default_timeout_ms() -> u64 {
    10000
}

/// Expectation for LLM evaluation assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmEvalExpect {
    Pass,
    Fail,
}

/// An LLM evaluation assertion.
/// Uses an LLM to semantically evaluate the output against a prompt.
/// Always uses JSON format with default schema: `{"result": boolean, "reason": string}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmEvalAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    /// The prompt/question to evaluate. Use `{{output}}` as placeholder for the output text.
    pub pattern: String,
    pub expect: LlmEvalExpect,
    /// Timeout in milliseconds (default: 60000 = 60s).
    #[serde(default = "default_llm_eval_timeout_ms")]
    pub timeout_ms: u64,
    /// JSON schema to validate the response against.
    /// Default schema: `{"result": boolean, "reason": string}`.
    #[serde(default)]
    pub json_schema: Option<serde_json::Value>,
}

const fn default_llm_eval_timeout_ms() -> u64 {
    60_000
}

/// A `tool_called` assertion.
/// Checks if specific tools were called during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCalledAssertion {
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub desc: Option<String>,
    /// Regex pattern to match tool names.
    pub pattern: String,
    pub expect: PatternExpect,
}

/// Union type for all assertion types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assertion {
    Regex(RegexAssertion),
    Contains(ContainsAssertion),
    LineCount(LineCountAssertion),
    Exec(ExecAssertion),
    LlmEval(LlmEvalAssertion),
    ToolCalled(ToolCalledAssertion),
}

impl Assertion {
    /// Get the assertion ID.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Regex(a) => &a.id,
            Self::Contains(a) => &a.id,
            Self::LineCount(a) => &a.id,
            Self::Exec(a) => &a.id,
            Self::LlmEval(a) => &a.id,
            Self::ToolCalled(a) => &a.id,
        }
    }

    /// Get the assertion description if available.
    #[must_use]
    pub fn desc(&self) -> Option<&str> {
        match self {
            Self::Regex(a) => a.desc.as_deref(),
            Self::Contains(a) => a.desc.as_deref(),
            Self::LineCount(a) => a.desc.as_deref(),
            Self::Exec(a) => a.desc.as_deref(),
            Self::LlmEval(a) => a.desc.as_deref(),
            Self::ToolCalled(a) => a.desc.as_deref(),
        }
    }

    /// Get the display name (desc if available, otherwise id).
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.desc().unwrap_or_else(|| self.id())
    }

    /// Get the assertion type name as a string.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Regex(_) => "regex",
            Self::Contains(_) => "contains",
            Self::LineCount(_) => "line_count",
            Self::Exec(_) => "exec",
            Self::LlmEval(_) => "llm_eval",
            Self::ToolCalled(_) => "tool_called",
        }
    }

    /// Get the pattern or text being tested (if applicable).
    #[must_use]
    pub fn pattern(&self) -> Option<&str> {
        match self {
            Self::Regex(a) => Some(&a.pattern),
            Self::Contains(a) => Some(&a.pattern),
            Self::LineCount(_) | Self::Exec(_) => None,
            Self::LlmEval(a) => Some(&a.pattern),
            Self::ToolCalled(a) => Some(&a.pattern),
        }
    }
}

/// A contract definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub skill: Option<String>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub golden_assertions: Vec<Assertion>,
}

/// Merged contract from common + skill-specific contracts.
#[derive(Debug, Clone, Default)]
pub struct MergedContract {
    pub assertions: Vec<Assertion>,
    pub golden_assertions: Vec<Assertion>,
    pub warnings: Vec<String>,
}

/// Test verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Pass,
    Fail,
    Warn,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "Pass"),
            Self::Fail => write!(f, "Fail"),
            Self::Warn => write!(f, "Warn"),
        }
    }
}

/// Result of a single test iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub iteration: u32,
    pub prompt: String,
    pub expected_skills: Vec<String>,
    pub called_skills: Vec<String>,
    /// All tools that were called during execution.
    pub called_tools: Vec<String>,
    pub output_text: String,
    pub output_hash: String,
    pub skill_passed: bool,
    pub contract_passed: Option<bool>,
    pub golden_passed: Option<bool>,
    pub failures: Vec<String>,
    pub golden_failures: Vec<String>,
    pub verdict: Verdict,
    pub latency_ms: u64,
}

/// Result of contract evaluation.
#[derive(Debug, Clone, Default)]
pub struct ContractResult {
    pub contract_passed: bool,
    pub golden_passed: Option<bool>,
    pub details: Vec<AssertionResult>,
    pub failures: Vec<String>,
    pub golden_failures: Vec<String>,
}

/// Result of a single assertion evaluation.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub id: String,
    pub passed: bool,
    pub is_golden: bool,
}

/// Judgment result with verdict and reason.
#[derive(Debug, Clone)]
pub struct JudgmentResult {
    pub verdict: Verdict,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_count_validation_missing_bounds() {
        let result = LineCountAssertion::new("test", None, None);
        assert!(matches!(
            result,
            Err(ValidationError::LineCountMissingBounds)
        ));
    }

    #[test]
    fn test_line_count_validation_min_greater_than_max() {
        let result = LineCountAssertion::new("test", Some(10), Some(5));
        assert!(matches!(
            result,
            Err(ValidationError::LineCountMinGreaterThanMax { min: 10, max: 5 })
        ));
    }

    #[test]
    fn test_line_count_validation_success() {
        let result = LineCountAssertion::new("test", Some(5), Some(10));
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_count_check() -> Result<(), ValidationError> {
        let assertion = LineCountAssertion::new("test", Some(5), Some(10))?;
        assert!(!assertion.check(4));
        assert!(assertion.check(5));
        assert!(assertion.check(7));
        assert!(assertion.check(10));
        assert!(!assertion.check(11));
        Ok(())
    }

    #[test]
    fn test_line_count_check_min_only() -> Result<(), ValidationError> {
        let assertion = LineCountAssertion::new("test", Some(5), None)?;
        assert!(!assertion.check(4));
        assert!(assertion.check(5));
        assert!(assertion.check(100));
        Ok(())
    }

    #[test]
    fn test_line_count_check_max_only() -> Result<(), ValidationError> {
        let assertion = LineCountAssertion::new("test", None, Some(10))?;
        assert!(assertion.check(0));
        assert!(assertion.check(10));
        assert!(!assertion.check(11));
        Ok(())
    }

    #[test]
    fn test_match_policy_default() {
        assert_eq!(MatchPolicy::default(), MatchPolicy::All);
    }

    #[test]
    fn test_verdict_display() {
        assert_eq!(format!("{}", Verdict::Pass), "Pass");
        assert_eq!(format!("{}", Verdict::Fail), "Fail");
        assert_eq!(format!("{}", Verdict::Warn), "Warn");
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_llm_eval_expect_deserialize() -> TestResult {
        let pass: LlmEvalExpect = serde_yml::from_str("pass")?;
        assert_eq!(pass, LlmEvalExpect::Pass);

        let fail: LlmEvalExpect = serde_yml::from_str("fail")?;
        assert_eq!(fail, LlmEvalExpect::Fail);
        Ok(())
    }

    #[test]
    fn test_llm_eval_assertion_deserialize() -> TestResult {
        let yaml = r#"
id: test-eval
pattern: "Is the output correct?"
expect: pass
"#;
        let assertion: LlmEvalAssertion = serde_yml::from_str(yaml)?;
        assert_eq!(assertion.id, "test-eval");
        assert_eq!(assertion.pattern, "Is the output correct?");
        assert_eq!(assertion.expect, LlmEvalExpect::Pass);
        // Default timeout should be 60 seconds
        assert_eq!(assertion.timeout_ms, 60_000);
        Ok(())
    }

    #[test]
    fn test_llm_eval_assertion_deserialize_with_timeout() -> TestResult {
        let yaml = r#"
id: test-eval
pattern: "Is the output correct?"
expect: pass
timeout_ms: 30000
"#;
        let assertion: LlmEvalAssertion = serde_yml::from_str(yaml)?;
        assert_eq!(assertion.id, "test-eval");
        assert_eq!(assertion.timeout_ms, 30_000);
        Ok(())
    }

    #[test]
    fn test_validation_deserialize() -> TestResult {
        let yaml = r#"
assertions:
  - id: check1
    type: regex
    pattern: "hello"
    expect: present
"#;
        let validation: Validation = serde_yml::from_str(yaml)?;
        assert_eq!(validation.assertions.len(), 1);
        Ok(())
    }

    #[test]
    fn test_test_case_unknown_field_rejected() {
        let yaml = r#"
id: test-001
prompt: "Test prompt"
expected_skills: ["skill-a"]
unknown_field: "should fail"
"#;
        let result: Result<TestCase, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
        let err = result.err();
        assert!(err.is_some_and(|e| e.to_string().contains("unknown field")));
    }

    #[test]
    fn test_contract_unknown_field_rejected() {
        let yaml = r#"
skill: test-skill
assertions: []
unknown_field: "should fail"
"#;
        let result: Result<Contract, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
        let err = result.err();
        assert!(err.is_some_and(|e| e.to_string().contains("unknown field")));
    }

    #[test]
    fn test_assertion_with_llm_eval() -> TestResult {
        let yaml = r#"
type: llm_eval
id: semantic-check
pattern: "Does the output contain {{output}}?"
expect: pass
"#;
        let assertion: Assertion = serde_yml::from_str(yaml)?;
        let Assertion::LlmEval(a) = assertion else {
            return Err("expected LlmEval".into());
        };
        assert_eq!(a.id, "semantic-check");
        assert!(a.pattern.contains("{{output}}"));
        Ok(())
    }

    #[test]
    fn test_assertion_id_for_llm_eval() {
        let assertion = Assertion::LlmEval(LlmEvalAssertion {
            id: "test-id".to_string(),
            desc: None,
            pattern: "test".to_string(),
            expect: LlmEvalExpect::Pass,
            timeout_ms: 60_000,
            json_schema: None,
        });
        assert_eq!(assertion.id(), "test-id");
    }

    #[test]
    fn test_assertion_type_name() {
        // Test all assertion variants return correct type name
        let regex = Assertion::Regex(RegexAssertion {
            id: "r".to_string(),
            desc: None,
            pattern: ".*".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(regex.type_name(), "regex");

        let contains = Assertion::Contains(ContainsAssertion {
            id: "c".to_string(),
            desc: None,
            pattern: "text".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(contains.type_name(), "contains");

        let line_count = Assertion::LineCount(LineCountAssertion {
            id: "lc".to_string(),
            desc: None,
            min: Some(1),
            max: Some(10),
        });
        assert_eq!(line_count.type_name(), "line_count");

        let exec = Assertion::Exec(ExecAssertion {
            id: "e".to_string(),
            desc: None,
            command: "echo test".to_string(),
            language: Some("bash".to_string()),
            timeout_ms: 5000,
            expect: ExecExpect::ExitCodeZero,
        });
        assert_eq!(exec.type_name(), "exec");

        let llm_eval = Assertion::LlmEval(LlmEvalAssertion {
            id: "l".to_string(),
            desc: None,
            pattern: "check".to_string(),
            expect: LlmEvalExpect::Pass,
            timeout_ms: 60_000,
            json_schema: None,
        });
        assert_eq!(llm_eval.type_name(), "llm_eval");

        let tool_called = Assertion::ToolCalled(ToolCalledAssertion {
            id: "t".to_string(),
            desc: None,
            pattern: "Read".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(tool_called.type_name(), "tool_called");
    }

    #[test]
    fn test_assertion_pattern() {
        // Test pattern() returns Some for assertions with patterns
        let regex = Assertion::Regex(RegexAssertion {
            id: "r".to_string(),
            desc: None,
            pattern: "\\d+".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(regex.pattern(), Some("\\d+"));

        let contains = Assertion::Contains(ContainsAssertion {
            id: "c".to_string(),
            desc: None,
            pattern: "hello world".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(contains.pattern(), Some("hello world"));

        let llm_eval = Assertion::LlmEval(LlmEvalAssertion {
            id: "l".to_string(),
            desc: None,
            pattern: "Is {{output}} valid?".to_string(),
            expect: LlmEvalExpect::Pass,
            timeout_ms: 60_000,
            json_schema: None,
        });
        assert_eq!(llm_eval.pattern(), Some("Is {{output}} valid?"));

        let tool_called = Assertion::ToolCalled(ToolCalledAssertion {
            id: "t".to_string(),
            desc: None,
            pattern: "Bash|Read".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(tool_called.pattern(), Some("Bash|Read"));

        // Test pattern() returns None for assertions without patterns
        let line_count = Assertion::LineCount(LineCountAssertion {
            id: "lc".to_string(),
            desc: None,
            min: Some(1),
            max: Some(10),
        });
        assert_eq!(line_count.pattern(), None);

        let exec = Assertion::Exec(ExecAssertion {
            id: "e".to_string(),
            desc: None,
            command: "echo test".to_string(),
            language: Some("bash".to_string()),
            timeout_ms: 5000,
            expect: ExecExpect::ExitCodeZero,
        });
        assert_eq!(exec.pattern(), None);
    }

    #[test]
    fn test_execution_report_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let report = ExecutionReport {
            timestamp: "2026-01-30T06:00:00.000Z".to_string(),
            skills: vec![],
            summary: SkillTestSummary::default(),
        };
        let json = serde_json::to_string(&report)?;
        assert!(json.contains("timestamp"));
        assert!(json.contains("skills"));
        assert!(json.contains("summary"));
        Ok(())
    }

    #[test]
    fn test_skill_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = SkillResult {
            skill_name: "test-skill".to_string(),
            skill_path: "/path/to/skill".to_string(),
            tests: vec![],
            verdict: Verdict::Pass,
            error: None,
        };
        let json = serde_json::to_string(&result)?;
        assert!(json.contains("skill_name"));
        assert!(json.contains("verdict"));
        Ok(())
    }

    #[test]
    fn test_skill_test_summary_from_single_pass() {
        let skill = SkillResult {
            skill_name: "test-skill".to_string(),
            skill_path: "/path/to/skill".to_string(),
            tests: vec![DetailedTestResult {
                name: "test-1".to_string(),
                desc: None,
                prompt: "test prompt".to_string(),
                iterations: vec![],
                summary: SimplifiedTestResult {
                    id: "test-1".to_string(),
                    iterations: 1,
                    passed: 1,
                    failed: 0,
                    pass_rate: 100.0,
                    verdict: Verdict::Pass,
                    failures: vec![],
                    golden_failures: vec![],
                    called_tools: vec![],
                },
            }],
            verdict: Verdict::Pass,
            error: None,
        };
        let summary = SkillTestSummary::from_single(&skill);
        assert_eq!(summary.total_skills, 1);
        assert_eq!(summary.passed_skills, 1);
        assert_eq!(summary.failed_skills, 0);
        assert_eq!(summary.total_tests, 1);
        assert_eq!(summary.passed_tests, 1);
        assert_eq!(summary.failed_tests, 0);
    }

    #[test]
    fn test_skill_test_summary_from_single_fail() {
        let skill = SkillResult {
            skill_name: "test-skill".to_string(),
            skill_path: "/path/to/skill".to_string(),
            tests: vec![DetailedTestResult {
                name: "test-1".to_string(),
                desc: None,
                prompt: "test prompt".to_string(),
                iterations: vec![],
                summary: SimplifiedTestResult {
                    id: "test-1".to_string(),
                    iterations: 1,
                    passed: 0,
                    failed: 1,
                    pass_rate: 0.0,
                    verdict: Verdict::Fail,
                    failures: vec!["assertion failed".to_string()],
                    golden_failures: vec![],
                    called_tools: vec![],
                },
            }],
            verdict: Verdict::Fail,
            error: None,
        };
        let summary = SkillTestSummary::from_single(&skill);
        assert_eq!(summary.total_skills, 1);
        assert_eq!(summary.passed_skills, 0);
        assert_eq!(summary.failed_skills, 1);
        assert_eq!(summary.total_tests, 1);
        assert_eq!(summary.passed_tests, 0);
        assert_eq!(summary.failed_tests, 1);
    }
}

// =============================================================================
// New simplified types for skill-test simplification
// =============================================================================

use std::path::PathBuf;

/// Hook type for skill tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HookType {
    /// No hook.
    None,
    /// Simple reminder hook.
    #[default]
    Simple,
    /// Forced evaluation hook.
    Forced,
    /// Custom hook script.
    Custom,
}

/// Skill test configuration from `skill-test.config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillTestConfig {
    /// Model to use (default: claude-sonnet-4-20250514).
    #[serde(default = "default_model")]
    pub model: String,

    /// Timeout per iteration in milliseconds (default: 60000).
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Number of iterations per test (default: 10).
    #[serde(default = "default_iterations")]
    pub iterations: u32,

    /// Pass threshold percentage (default: 80).
    #[serde(default = "default_threshold")]
    pub threshold: u32,

    /// Hook type (default: simple).
    #[serde(default)]
    pub hook: HookType,

    /// Custom hook path (required when hook is "custom").
    #[serde(rename = "hook-path")]
    pub hook_path: Option<PathBuf>,

    /// Test file patterns (default: ["skill-tests/**/test-*.yaml", ...]).
    #[serde(rename = "test-patterns", default = "default_test_patterns")]
    pub test_patterns: Vec<String>,

    /// Exclude patterns (default: `["node_modules/"]`).
    #[serde(rename = "exclude-patterns", default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// Strict mode for missing files (default: false).
    #[serde(default)]
    pub strict: bool,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

const fn default_timeout() -> u64 {
    60_000
}

const fn default_iterations() -> u32 {
    10
}

const fn default_threshold() -> u32 {
    80
}

fn default_test_patterns() -> Vec<String> {
    vec![
        "skill-tests/**/test-*.yaml".to_string(),
        "skill-tests/**/test-*.yml".to_string(),
        "skill-tests/**/*.spec.yaml".to_string(),
        "skill-tests/**/*.spec.yml".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec!["node_modules/".to_string()]
}

impl Default for SkillTestConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            timeout: default_timeout(),
            iterations: default_iterations(),
            threshold: default_threshold(),
            hook: HookType::default(),
            hook_path: None,
            test_patterns: default_test_patterns(),
            exclude_patterns: default_exclude_patterns(),
            strict: false,
        }
    }
}

/// An assertion or a file reference to load assertions from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssertionOrFile {
    /// Inline assertion.
    Inline(Assertion),
    /// File reference (single file).
    FileRef { file: FileRefValue },
}

/// File reference value - either a single path or multiple paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileRefValue {
    /// Single file path.
    Single(String),
    /// Multiple file paths.
    Multiple(Vec<String>),
}

/// Simplified test case for the new format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimplifiedTestCase {
    /// Test case ID.
    pub id: String,

    /// Test case description (for display).
    #[serde(default)]
    pub desc: Option<String>,

    /// Prompt to send to Claude.
    pub prompt: String,

    /// Number of iterations (overrides config).
    pub iterations: Option<u32>,

    /// Assertions (inline or file references).
    #[serde(default)]
    pub assertions: Vec<AssertionOrFile>,

    /// Golden assertions (do not affect pass/fail).
    #[serde(default)]
    pub golden_assertions: Vec<AssertionOrFile>,
}

/// Skill directory information.
#[derive(Debug, Clone)]
pub struct SkillDir {
    /// Path to the skill directory.
    pub path: PathBuf,

    /// Skill name from SKILL.md frontmatter.
    pub name: String,

    /// Loaded config (from skill-test.config.yaml or defaults).
    pub config: SkillTestConfig,
}

/// Result of running all tests for a skill.
#[derive(Debug, Clone, Serialize)]
pub struct SkillTestResult {
    /// Skill name.
    pub name: String,

    /// Skill directory path.
    pub path: PathBuf,

    /// Individual test results.
    pub tests: Vec<SimplifiedTestResult>,

    /// Overall verdict for the skill.
    pub verdict: Verdict,
}

/// Result of a single simplified test.
#[derive(Debug, Clone, Serialize)]
pub struct SimplifiedTestResult {
    /// Test case ID.
    pub id: String,

    /// Number of iterations.
    pub iterations: u32,

    /// Number of passed iterations.
    pub passed: u32,

    /// Number of failed iterations.
    pub failed: u32,

    /// Pass rate percentage.
    pub pass_rate: f64,

    /// Test verdict.
    pub verdict: Verdict,

    /// Failure messages.
    pub failures: Vec<String>,

    /// Golden assertion failures (informational).
    pub golden_failures: Vec<String>,

    /// Tools called during execution (union of all iterations).
    pub called_tools: Vec<String>,
}

/// Summary of all skill test results.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SkillTestSummary {
    /// Total number of skills.
    pub total_skills: usize,

    /// Number of passed skills.
    pub passed_skills: usize,

    /// Number of failed skills.
    pub failed_skills: usize,

    /// Total number of tests.
    pub total_tests: usize,

    /// Number of passed tests.
    pub passed_tests: usize,

    /// Number of failed tests.
    pub failed_tests: usize,
}

// =============================================================================
// Detailed types for verbose JSON output and error logging
// =============================================================================

/// Detailed assertion result for JSON output.
///
/// This captures detailed information about each assertion evaluation,
/// including the pattern used and any error messages.
#[derive(Debug, Clone, Serialize)]
pub struct DetailedAssertionResult {
    /// Assertion name/ID.
    pub name: String,

    /// Human-readable description.
    pub desc: Option<String>,

    /// Type of assertion ("regex", "contains", "`llm_eval`", etc.).
    pub assertion_type: String,

    /// The pattern or text being tested (if applicable).
    pub pattern: Option<String>,

    /// Whether the assertion passed.
    pub passed: bool,

    /// Error message if evaluation failed.
    pub error: Option<String>,
}

/// Detailed iteration result for JSON output.
///
/// This captures comprehensive information about a single test iteration,
/// including the full output (truncated at `MAX_OUTPUT_CHARS`).
#[derive(Debug, Clone, Serialize)]
pub struct DetailedIterationResult {
    /// Iteration number (1-based).
    pub iteration: u32,

    /// Whether this iteration passed all assertions.
    pub passed: bool,

    /// Execution latency in milliseconds.
    pub latency_ms: u64,

    /// Full output text, truncated at `MAX_OUTPUT_CHARS` (100K characters, not bytes).
    pub output: String,

    /// Hash of the output text.
    pub output_hash: String,

    /// Tools called during this iteration.
    pub called_tools: Vec<String>,

    /// Detailed results for required assertions.
    pub assertions: Vec<DetailedAssertionResult>,

    /// Detailed results for golden (informational) assertions.
    pub golden_assertions: Vec<DetailedAssertionResult>,
}

/// Detailed test result for verbose JSON output.
///
/// This combines the summary with per-iteration details for debugging.
#[derive(Debug, Clone, Serialize)]
pub struct DetailedTestResult {
    /// Test case name/ID.
    pub name: String,

    /// Human-readable description.
    pub desc: Option<String>,

    /// The prompt used for this test.
    pub prompt: String,

    /// Detailed results for each iteration.
    pub iterations: Vec<DetailedIterationResult>,

    /// Summary statistics.
    pub summary: SimplifiedTestResult,
}

/// Result of executing tests for a single skill.
///
/// Used in both JSON output and error logs with identical schema.
#[derive(Debug, Clone, Serialize)]
pub struct SkillResult {
    /// Name of the skill being tested.
    pub skill_name: String,

    /// Path to the skill directory.
    pub skill_path: String,

    /// Detailed results for all tests.
    pub tests: Vec<DetailedTestResult>,

    /// Overall verdict for this skill.
    pub verdict: Verdict,

    /// Error message if the execution failed (not test failures).
    pub error: Option<String>,
}

/// Full execution report (JSON output & error log common format).
///
/// Both JSON output (`--format json`) and error log files use this identical schema.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionReport {
    /// ISO 8601 timestamp of the execution.
    pub timestamp: String,

    /// Results for all skills.
    pub skills: Vec<SkillResult>,

    /// Summary statistics.
    pub summary: SkillTestSummary,
}

impl SkillTestSummary {
    /// Create a summary from a single `SkillResult`.
    #[must_use]
    pub fn from_single(skill: &SkillResult) -> Self {
        let passed_tests = skill
            .tests
            .iter()
            .filter(|t| t.summary.verdict == Verdict::Pass)
            .count();
        let failed_tests = skill.tests.len() - passed_tests;
        let skill_passed = skill.verdict == Verdict::Pass;

        Self {
            total_skills: 1,
            passed_skills: usize::from(skill_passed),
            failed_skills: usize::from(!skill_passed),
            total_tests: skill.tests.len(),
            passed_tests,
            failed_tests,
        }
    }
}

// Keep SkillExecutionLog as type alias for backward compatibility during migration
#[doc(hidden)]
#[deprecated(since = "0.1.0", note = "Use SkillResult instead")]
pub type SkillExecutionLog = SkillResult;

// =============================================================================
// New test file format with scenarios and named assertions
// =============================================================================

use std::collections::HashMap;

/// A test file containing scenarios and optionally named assertions.
///
/// This is the new format that supports:
/// - `desc`: File-level description
/// - `assertions`: Named assertions that can be referenced by scenarios
/// - `scenarios`: Test scenarios (as a `HashMap` with name as key)
///
/// Example:
/// ```yaml
/// desc: "検索機能のテスト"
///
/// assertions:
///   has-numbered-list:
///     desc: "番号付きリストが含まれていること"
///     type: regex
///     pattern: "\\d+\\."
///     expect: present
///
/// scenarios:
///   search-basic:
///     desc: "基本検索テスト"
///     prompt: "Claude Codeのスキルを探して"
///     assertions:
///       - has-numbered-list      # name reference
///       - id: search-result-text  # inline assertion
///         type: contains
///         desc: "検索結果テキスト"
///         pattern: "検索結果"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestFile {
    /// File-level description.
    #[serde(default)]
    pub desc: Option<String>,

    /// Named assertions that can be referenced by scenarios.
    /// Key is the assertion name.
    #[serde(default)]
    pub assertions: HashMap<String, AssertionDef>,

    /// Test scenarios. Key is the scenario name.
    pub scenarios: HashMap<String, Scenario>,
}

/// An assertion definition (without the name, which is the `HashMap` key).
///
/// This is similar to `Assertion` but used for defining named assertions
/// where the name comes from the `HashMap` key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionDef {
    Regex(RegexAssertionDef),
    Contains(ContainsAssertionDef),
    LineCount(LineCountAssertionDef),
    Exec(ExecAssertionDef),
    LlmEval(LlmEvalAssertionDef),
    ToolCalled(ToolCalledAssertionDef),
}

impl AssertionDef {
    /// Convert to Assertion with the given name as ID.
    #[must_use]
    pub fn to_assertion(&self, name: &str) -> Assertion {
        match self {
            Self::Regex(def) => Assertion::Regex(RegexAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                pattern: def.pattern.clone(),
                expect: def.expect,
            }),
            Self::Contains(def) => Assertion::Contains(ContainsAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                pattern: def.pattern.clone(),
                expect: def.expect,
            }),
            Self::LineCount(def) => Assertion::LineCount(LineCountAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                min: def.min,
                max: def.max,
            }),
            Self::Exec(def) => Assertion::Exec(ExecAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                command: def.command.clone(),
                language: def.language.clone(),
                timeout_ms: def.timeout_ms,
                expect: def.expect.clone(),
            }),
            Self::LlmEval(def) => Assertion::LlmEval(LlmEvalAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                pattern: def.pattern.clone(),
                expect: def.expect,
                timeout_ms: def.timeout_ms,
                json_schema: def.json_schema.clone(),
            }),
            Self::ToolCalled(def) => Assertion::ToolCalled(ToolCalledAssertion {
                id: name.to_string(),
                desc: def.desc.clone(),
                pattern: def.pattern.clone(),
                expect: def.expect,
            }),
        }
    }
}

/// Regex assertion definition (name is provided externally).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegexAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: PatternExpect,
}

/// Contains assertion definition (name is provided externally).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainsAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: PatternExpect,
}

/// Line count assertion definition (name is provided externally).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineCountAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// Exec assertion definition (name is provided externally).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub command: String,
    pub language: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    pub expect: ExecExpect,
}

/// LLM eval assertion definition (name is provided externally).
/// Always uses JSON format with default schema: `{"result": boolean, "reason": string}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmEvalAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: LlmEvalExpect,
    #[serde(default = "default_llm_eval_timeout_ms")]
    pub timeout_ms: u64,
    /// JSON schema to validate the response against.
    /// Default schema: `{"result": boolean, "reason": string}`.
    #[serde(default)]
    pub json_schema: Option<serde_json::Value>,
}

/// Tool called assertion definition (name is provided externally).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCalledAssertionDef {
    #[serde(default)]
    pub desc: Option<String>,
    pub pattern: String,
    pub expect: PatternExpect,
}

/// A test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Human-readable description (displayed in test output).
    #[serde(default)]
    pub desc: Option<String>,

    /// Prompt to send to Claude.
    pub prompt: String,

    /// Number of iterations (overrides config).
    #[serde(default)]
    pub iterations: Option<u32>,

    /// Assertions - can be name references or inline definitions.
    #[serde(default)]
    pub assertions: Vec<AssertionRef>,

    /// Golden assertions (do not affect pass/fail).
    #[serde(default)]
    pub golden_assertions: Vec<AssertionRef>,
}

/// Reference to an assertion - either a name reference or inline definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssertionRef {
    /// Reference to a named assertion defined at file level.
    Name(String),
    /// Inline assertion definition with full spec.
    Inline(Assertion),
}

#[cfg(test)]
mod simplified_tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_skill_test_config_defaults() {
        let config = SkillTestConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.timeout, 60_000);
        assert_eq!(config.iterations, 10);
        assert_eq!(config.threshold, 80);
        assert_eq!(config.hook, HookType::Simple);
        assert!(config.hook_path.is_none());
        assert!(!config.strict);
    }

    #[test]
    fn test_skill_test_config_deserialize() -> TestResult {
        let yaml = r"
model: claude-opus-4-20250514
timeout: 120000
iterations: 5
threshold: 90
hook: forced
strict: true
";
        let config: SkillTestConfig = serde_yml::from_str(yaml)?;
        assert_eq!(config.model, "claude-opus-4-20250514");
        assert_eq!(config.timeout, 120_000);
        assert_eq!(config.iterations, 5);
        assert_eq!(config.threshold, 90);
        assert_eq!(config.hook, HookType::Forced);
        assert!(config.strict);
        Ok(())
    }

    #[test]
    fn test_skill_test_config_with_custom_hook() -> TestResult {
        let yaml = r"
hook: custom
hook-path: ./my-hook.sh
";
        let config: SkillTestConfig = serde_yml::from_str(yaml)?;
        assert_eq!(config.hook, HookType::Custom);
        assert_eq!(config.hook_path, Some(PathBuf::from("./my-hook.sh")));
        Ok(())
    }

    #[test]
    fn test_simplified_test_case_deserialize() -> TestResult {
        let yaml = r#"
id: test-001
prompt: "Do something"
iterations: 5
assertions:
  - id: check-output
    type: contains
    pattern: "expected"
    expect: present
golden_assertions:
  - id: best-practice
    type: regex
    pattern: "^//"
    expect: present
"#;
        let test_case: SimplifiedTestCase = serde_yml::from_str(yaml)?;
        assert_eq!(test_case.id, "test-001");
        assert_eq!(test_case.prompt, "Do something");
        assert_eq!(test_case.iterations, Some(5));
        assert_eq!(test_case.assertions.len(), 1);
        assert_eq!(test_case.golden_assertions.len(), 1);
        Ok(())
    }

    #[test]
    fn test_assertion_or_file_inline() -> TestResult {
        let yaml = r#"
id: check
type: contains
pattern: "test"
expect: present
"#;
        let aof: AssertionOrFile = serde_yml::from_str(yaml)?;
        assert!(matches!(aof, AssertionOrFile::Inline(_)));
        Ok(())
    }

    #[test]
    fn test_assertion_or_file_single_file() -> TestResult {
        let yaml = r"
file: ./common.yaml
";
        let aof: AssertionOrFile = serde_yml::from_str(yaml)?;
        let AssertionOrFile::FileRef {
            file: FileRefValue::Single(path),
        } = aof
        else {
            return Err("expected single file ref".into());
        };
        assert_eq!(path, "./common.yaml");
        Ok(())
    }

    #[test]
    fn test_assertion_or_file_multiple_files() -> TestResult {
        let yaml = r"
file:
  - ./base.yaml
  - ./strict.yaml
";
        let aof: AssertionOrFile = serde_yml::from_str(yaml)?;
        let AssertionOrFile::FileRef {
            file: FileRefValue::Multiple(paths),
        } = aof
        else {
            return Err("expected multiple file refs".into());
        };
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "./base.yaml");
        assert_eq!(paths[1], "./strict.yaml");
        Ok(())
    }

    #[test]
    fn test_hook_type_default() {
        assert_eq!(HookType::default(), HookType::Simple);
    }

    #[test]
    fn test_hook_type_deserialize() -> TestResult {
        let none: HookType = serde_yml::from_str("none")?;
        assert_eq!(none, HookType::None);

        let simple: HookType = serde_yml::from_str("simple")?;
        assert_eq!(simple, HookType::Simple);

        let forced: HookType = serde_yml::from_str("forced")?;
        assert_eq!(forced, HookType::Forced);

        let custom: HookType = serde_yml::from_str("custom")?;
        assert_eq!(custom, HookType::Custom);
        Ok(())
    }

    #[test]
    fn test_test_file_deserialize() -> TestResult {
        let yaml = r#"
desc: "検索機能のテスト"

assertions:
  has-numbered-list:
    desc: "番号付きリストが含まれていること"
    type: regex
    pattern: "\\d+\\."
    expect: present
  has-score:
    type: regex
    pattern: "\\d+/35"
    expect: present

scenarios:
  search-basic:
    desc: "基本検索テスト"
    prompt: "Claude Codeのスキルを探して"
    assertions:
      - has-numbered-list
      - has-score
  search-with-inline:
    prompt: "検索テスト"
    assertions:
      - has-numbered-list
      - type: contains
        id: inline-check
        pattern: "検索結果"
        expect: present
"#;
        let test_file: TestFile = serde_yml::from_str(yaml)?;
        assert_eq!(test_file.desc, Some("検索機能のテスト".to_string()));
        assert_eq!(test_file.assertions.len(), 2);
        assert!(test_file.assertions.contains_key("has-numbered-list"));
        assert!(test_file.assertions.contains_key("has-score"));
        assert_eq!(test_file.scenarios.len(), 2);

        let basic = test_file
            .scenarios
            .get("search-basic")
            .ok_or("scenario 'search-basic' not found")?;
        assert_eq!(basic.desc, Some("基本検索テスト".to_string()));
        assert_eq!(basic.prompt, "Claude Codeのスキルを探して");
        assert_eq!(basic.assertions.len(), 2);

        // Check name references
        assert!(matches!(&basic.assertions[0], AssertionRef::Name(n) if n == "has-numbered-list"));
        assert!(matches!(&basic.assertions[1], AssertionRef::Name(n) if n == "has-score"));

        // Check scenario with inline assertion
        let inline = test_file
            .scenarios
            .get("search-with-inline")
            .ok_or("scenario 'search-with-inline' not found")?;
        assert_eq!(inline.assertions.len(), 2);
        assert!(matches!(&inline.assertions[0], AssertionRef::Name(_)));
        assert!(matches!(&inline.assertions[1], AssertionRef::Inline(_)));
        Ok(())
    }

    #[test]
    fn test_assertion_def_to_assertion() {
        let def = AssertionDef::Regex(RegexAssertionDef {
            desc: Some("Check for numbers".to_string()),
            pattern: "\\d+".to_string(),
            expect: PatternExpect::Present,
        });

        let assertion = def.to_assertion("my-check");
        assert_eq!(assertion.id(), "my-check");
        assert_eq!(assertion.desc(), Some("Check for numbers"));
        assert_eq!(assertion.display_name(), "Check for numbers");
    }

    #[test]
    fn test_assertion_display_name_fallback() {
        let assertion = Assertion::Regex(RegexAssertion {
            id: "test-id".to_string(),
            desc: None,
            pattern: "test".to_string(),
            expect: PatternExpect::Present,
        });
        assert_eq!(assertion.display_name(), "test-id");
    }

    #[test]
    fn test_scenario_deserialize() -> TestResult {
        let yaml = r#"
desc: "テストシナリオ"
prompt: "テストを実行"
iterations: 5
assertions:
  - check-a
  - type: contains
    id: inline
    pattern: "test"
    expect: present
golden_assertions:
  - golden-a
"#;
        let scenario: Scenario = serde_yml::from_str(yaml)?;
        assert_eq!(scenario.desc, Some("テストシナリオ".to_string()));
        assert_eq!(scenario.prompt, "テストを実行");
        assert_eq!(scenario.iterations, Some(5));
        assert_eq!(scenario.assertions.len(), 2);
        assert_eq!(scenario.golden_assertions.len(), 1);
        Ok(())
    }
}
