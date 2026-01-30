//! Assertion evaluation engine.

use crate::codeblock::{extract_code_block, language_to_extension};
use crate::types::{
    Assertion, AssertionResult, ContainsAssertion, ContractResult, ExecAssertion, ExecExpect,
    LineCountAssertion, LlmEvalAssertion, MergedContract, PatternExpect, RegexAssertion,
    ToolCalledAssertion,
};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// Errors that can occur during assertion evaluation.
#[derive(Error, Debug)]
pub enum AssertionError {
    #[error("invalid regex pattern '{pattern}': {source}")]
    InvalidRegex {
        pattern: String,
        #[source]
        source: regex::Error,
    },
    #[error("exec assertion failed: {0}")]
    ExecFailed(String),
    #[error("exec timeout after {0}ms")]
    ExecTimeout(u64),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    #[error("JSON schema validation failed: {0}")]
    JsonSchemaValidation(String),
}

/// Evaluate a regex assertion.
///
/// # Errors
/// Returns `AssertionError::InvalidRegex` if the pattern is invalid.
pub fn evaluate_regex(output: &str, assertion: &RegexAssertion) -> Result<bool, AssertionError> {
    let re = Regex::new(&assertion.pattern).map_err(|e| AssertionError::InvalidRegex {
        pattern: assertion.pattern.clone(),
        source: e,
    })?;

    let found = re.is_match(output);

    Ok(match assertion.expect {
        PatternExpect::Present => found,
        PatternExpect::Absent => !found,
    })
}

/// Evaluate a contains assertion.
#[must_use]
pub fn evaluate_contains(output: &str, assertion: &ContainsAssertion) -> bool {
    let found = output.contains(&assertion.pattern);

    match assertion.expect {
        PatternExpect::Present => found,
        PatternExpect::Absent => !found,
    }
}

/// Evaluate a line count assertion.
#[must_use]
pub fn evaluate_line_count(output: &str, assertion: &LineCountAssertion) -> bool {
    let line_count = output.lines().count();
    assertion.check(line_count)
}

/// Evaluate an exec assertion.
///
/// # Errors
/// Returns an error if:
/// - No matching code block is found
/// - Command execution fails
/// - Timeout occurs
/// - IO operations fail
pub async fn evaluate_exec(
    output: &str,
    assertion: &ExecAssertion,
) -> Result<bool, AssertionError> {
    // Extract code block
    let block = extract_code_block(output, assertion.language.as_deref());
    let Some(block) = block else {
        return Err(AssertionError::ExecFailed(
            "no matching code block found".into(),
        ));
    };

    // Create temporary file
    let ext = block
        .language
        .as_ref()
        .map_or("txt", |l| language_to_extension(l));

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(block.content.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8])
    };

    let temp_path = std::env::temp_dir().join(format!("exec-{hash}.{ext}"));

    // Write content to temp file
    {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(block.content.as_bytes())?;
    }

    // Execute command
    let result = timeout(
        Duration::from_millis(assertion.timeout_ms),
        Command::new(&assertion.command)
            .arg(&temp_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    let output_result = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(AssertionError::ExecFailed(e.to_string())),
        Err(_) => return Err(AssertionError::ExecTimeout(assertion.timeout_ms)),
    };

    // Check expectation
    match &assertion.expect {
        ExecExpect::ExitCodeZero => Ok(output_result.status.success()),
        ExecExpect::OutputContains { output_contains } => {
            let stdout = String::from_utf8_lossy(&output_result.stdout);
            Ok(stdout.contains(output_contains))
        }
    }
}

/// Default JSON schema for `llm_eval` assertions.
const DEFAULT_LLM_EVAL_SCHEMA: &str = r#"{"result": boolean, "reason": string}"#;

/// Evaluate an LLM evaluation assertion.
///
/// Uses Claude CLI with haiku model to semantically evaluate the output.
/// Always requests JSON format response with schema: `{"result": boolean, "reason": string}`.
///
/// # Errors
/// Returns an error if LLM evaluation fails or times out.
pub async fn evaluate_llm_eval(
    output: &str,
    assertion: &LlmEvalAssertion,
) -> Result<bool, AssertionError> {
    use crate::types::LlmEvalExpect;

    // Replace {{output}} placeholder with actual output
    let prompt = assertion.pattern.replace("{{output}}", output);

    // Build evaluation prompt - always use JSON format
    let schema_instruction = assertion
        .json_schema
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok())
        .unwrap_or_else(|| DEFAULT_LLM_EVAL_SCHEMA.to_string());

    let eval_prompt = format!(
        "{prompt}\n\nRespond with JSON matching this schema: {schema_instruction}\nSet \"result\" to true if the evaluation passes, false otherwise. Include a brief \"reason\" explaining your judgment."
    );

    // Execute Claude CLI with haiku model
    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg(&eval_prompt)
        .arg("--model")
        .arg("claude-haiku-4-5-20251001")
        .arg("--max-turns")
        .arg("1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = cmd
        .spawn()
        .map_err(|e| AssertionError::ExecFailed(format!("failed to spawn claude: {e}")))?;

    let result = timeout(
        Duration::from_millis(assertion.timeout_ms),
        child.wait_with_output(),
    )
    .await;

    let output_result = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(AssertionError::ExecFailed(format!(
                "failed to execute claude: {e}"
            )));
        }
        Err(_) => return Err(AssertionError::ExecTimeout(assertion.timeout_ms)),
    };

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(AssertionError::ExecFailed(format!(
            "claude exited with error: {stderr}"
        )));
    }

    let response = String::from_utf8_lossy(&output_result.stdout);
    let response_trimmed = response.trim();

    // Parse JSON response
    let llm_result = parse_json_response(response_trimmed, assertion)?;

    // Compare with expectation
    Ok(match assertion.expect {
        LlmEvalExpect::Pass => llm_result,
        LlmEvalExpect::Fail => !llm_result,
    })
}

/// Parse JSON response from LLM and optionally validate against schema.
fn parse_json_response(
    response: &str,
    assertion: &LlmEvalAssertion,
) -> Result<bool, AssertionError> {
    // Try to extract JSON from response (may be wrapped in markdown code block)
    let json_str = extract_json_from_response(response);

    // Parse JSON
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AssertionError::JsonParse(format!("{e}: {json_str}")))?;

    // Validate against schema if provided
    if let Some(schema) = &assertion.json_schema {
        validate_json_schema(&json_value, schema)?;
    }

    // Extract result field
    let result = json_value
        .get("result")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            AssertionError::JsonParse("missing or invalid 'result' boolean field".to_string())
        })?;

    Ok(result)
}

/// Extract JSON content from response (handles markdown code blocks).
fn extract_json_from_response(response: &str) -> &str {
    // Try to extract from ```json ... ``` block
    if let Some(start) = response.find("```json") {
        let content_start = start + 7;
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Try to extract from ``` ... ``` block
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        // Skip to next line if there's a language identifier
        let content_start = response[content_start..]
            .find('\n')
            .map_or(content_start, |n| content_start + n + 1);
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Return as-is (try to parse the whole response)
    response.trim()
}

/// Validate JSON against a JSON schema.
fn validate_json_schema(
    json: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), AssertionError> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| AssertionError::JsonSchemaValidation(format!("invalid schema: {e}")))?;

    if !validator.is_valid(json) {
        let error_messages: Vec<String> =
            validator.iter_errors(json).map(|e| e.to_string()).collect();
        return Err(AssertionError::JsonSchemaValidation(
            error_messages.join("; "),
        ));
    }

    Ok(())
}

/// Evaluate a `tool_called` assertion.
///
/// # Errors
/// Returns `AssertionError::InvalidRegex` if the pattern is invalid.
pub fn evaluate_tool_called(
    called_tools: &[String],
    assertion: &ToolCalledAssertion,
) -> Result<bool, AssertionError> {
    let re = Regex::new(&assertion.pattern).map_err(|e| AssertionError::InvalidRegex {
        pattern: assertion.pattern.clone(),
        source: e,
    })?;

    let found = called_tools.iter().any(|tool| re.is_match(tool));

    Ok(match assertion.expect {
        PatternExpect::Present => found,
        PatternExpect::Absent => !found,
    })
}

/// Evaluate a single assertion.
///
/// # Errors
/// Returns an error if the specific assertion type evaluation fails.
pub async fn evaluate_assertion(
    output: &str,
    assertion: &Assertion,
    called_tools: &[String],
) -> Result<bool, AssertionError> {
    match assertion {
        Assertion::Regex(a) => evaluate_regex(output, a),
        Assertion::Contains(a) => Ok(evaluate_contains(output, a)),
        Assertion::LineCount(a) => Ok(evaluate_line_count(output, a)),
        Assertion::Exec(a) => evaluate_exec(output, a).await,
        Assertion::LlmEval(a) => evaluate_llm_eval(output, a).await,
        Assertion::ToolCalled(a) => evaluate_tool_called(called_tools, a),
    }
}

/// Evaluate all assertions in a contract.
///
/// # Errors
/// Returns an error if any assertion evaluation fails.
pub async fn evaluate_contract(
    output: &str,
    contract: &MergedContract,
    called_tools: &[String],
) -> Result<ContractResult, AssertionError> {
    let mut details = Vec::new();
    let mut failures = Vec::new();

    // Evaluate required assertions
    for assertion in &contract.assertions {
        let passed = evaluate_assertion(output, assertion, called_tools).await?;
        let id = assertion.id().to_string();

        if !passed {
            failures.push(id.clone());
        }

        details.push(AssertionResult {
            id,
            passed,
            is_golden: false,
        });
    }

    let contract_passed = failures.is_empty();

    // Evaluate golden assertions (don't affect contract_passed)
    let mut golden_failures = Vec::new();

    for assertion in &contract.golden_assertions {
        let passed = evaluate_assertion(output, assertion, called_tools).await?;
        let id = assertion.id().to_string();

        if !passed {
            golden_failures.push(id.clone());
        }

        details.push(AssertionResult {
            id,
            passed,
            is_golden: true,
        });
    }

    let golden_passed = if contract.golden_assertions.is_empty() {
        None
    } else {
        Some(golden_failures.is_empty())
    };

    Ok(ContractResult {
        contract_passed,
        golden_passed,
        details,
        failures,
        golden_failures,
    })
}

// hex encoding helper
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            result.push(char::from(HEX_CHARS[usize::from(byte >> 4)]));
            result.push(char::from(HEX_CHARS[usize::from(byte & 0xf)]));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_present() -> Result<(), AssertionError> {
        let assertion = RegexAssertion {
            id: "test".to_string(),
            desc: None,
            pattern: r"\$state\s*\(".to_string(),
            expect: PatternExpect::Present,
        };

        let output = "let count = $state(0);";
        assert!(evaluate_regex(output, &assertion)?);

        let output = "let count = 0;";
        assert!(!evaluate_regex(output, &assertion)?);
        Ok(())
    }

    #[test]
    fn test_regex_absent() -> Result<(), AssertionError> {
        let assertion = RegexAssertion {
            id: "test".to_string(),
            desc: None,
            pattern: r"TODO|FIXME".to_string(),
            expect: PatternExpect::Absent,
        };

        let output = "let count = 0;";
        assert!(evaluate_regex(output, &assertion)?);

        let output = "// TODO: fix this";
        assert!(!evaluate_regex(output, &assertion)?);
        Ok(())
    }

    #[test]
    fn test_regex_invalid() {
        let assertion = RegexAssertion {
            id: "test".to_string(),
            desc: None,
            pattern: r"[invalid".to_string(),
            expect: PatternExpect::Present,
        };

        let result = evaluate_regex("test", &assertion);
        assert!(result.is_err());
    }

    #[test]
    fn test_contains_present() {
        let assertion = ContainsAssertion {
            id: "test".to_string(),
            desc: None,
            pattern: "hello".to_string(),
            expect: PatternExpect::Present,
        };

        assert!(evaluate_contains("hello world", &assertion));
        assert!(!evaluate_contains("goodbye world", &assertion));
    }

    #[test]
    fn test_contains_absent() {
        let assertion = ContainsAssertion {
            id: "test".to_string(),
            desc: None,
            pattern: "error".to_string(),
            expect: PatternExpect::Absent,
        };

        assert!(evaluate_contains("success!", &assertion));
        assert!(!evaluate_contains("error occurred", &assertion));
    }

    #[test]
    fn test_line_count() {
        let assertion = LineCountAssertion {
            id: "test".to_string(),
            desc: None,
            min: Some(5),
            max: Some(10),
        };

        assert!(!evaluate_line_count("1\n2\n3\n4", &assertion)); // 4 lines
        assert!(evaluate_line_count("1\n2\n3\n4\n5", &assertion)); // 5 lines
        assert!(evaluate_line_count(
            "1\n2\n3\n4\n5\n6\n7\n8\n9\n10",
            &assertion
        )); // 10 lines
        assert!(!evaluate_line_count(
            "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11",
            &assertion
        )); // 11 lines
    }

    #[tokio::test]
    async fn test_exec_exit_code() -> Result<(), AssertionError> {
        let assertion = ExecAssertion {
            id: "test".to_string(),
            desc: None,
            command: "node".to_string(),
            language: Some("javascript".to_string()),
            timeout_ms: 5000,
            expect: ExecExpect::ExitCodeZero,
        };

        let output = r#"
```javascript
console.log("hello");
```
"#;

        // This test requires Node.js to be installed
        // Skip if node is not available
        if which::which("node").is_ok() {
            let result = evaluate_exec(output, &assertion).await?;
            assert!(result);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_exec_no_code_block() {
        let assertion = ExecAssertion {
            id: "test".to_string(),
            desc: None,
            command: "node".to_string(),
            language: Some("javascript".to_string()),
            timeout_ms: 5000,
            expect: ExecExpect::ExitCodeZero,
        };

        let output = "No code block here";
        let result = evaluate_exec(output, &assertion).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_evaluate_contract() -> Result<(), AssertionError> {
        let contract = MergedContract {
            assertions: vec![
                Assertion::Contains(ContainsAssertion {
                    id: "has-hello".to_string(),
                    desc: None,
                    pattern: "hello".to_string(),
                    expect: PatternExpect::Present,
                }),
                Assertion::Regex(RegexAssertion {
                    id: "no-error".to_string(),
                    desc: None,
                    pattern: "error".to_string(),
                    expect: PatternExpect::Absent,
                }),
            ],
            golden_assertions: vec![Assertion::Contains(ContainsAssertion {
                id: "has-world".to_string(),
                desc: None,
                pattern: "world".to_string(),
                expect: PatternExpect::Present,
            })],
            warnings: vec![],
        };

        let output = "hello world";
        let result = evaluate_contract(output, &contract, &[]).await?;

        assert!(result.contract_passed);
        assert_eq!(result.golden_passed, Some(true));
        assert!(result.failures.is_empty());
        assert!(result.golden_failures.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_contract_failure() -> Result<(), AssertionError> {
        let contract = MergedContract {
            assertions: vec![Assertion::Contains(ContainsAssertion {
                id: "has-foo".to_string(),
                desc: None,
                pattern: "foo".to_string(),
                expect: PatternExpect::Present,
            })],
            golden_assertions: vec![],
            warnings: vec![],
        };

        let output = "bar baz";
        let result = evaluate_contract(output, &contract, &[]).await?;

        assert!(!result.contract_passed);
        assert_eq!(result.failures, vec!["has-foo"]);
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Claude CLI and API access"]
    async fn test_llm_eval_pass() -> Result<(), AssertionError> {
        use crate::types::{LlmEvalAssertion, LlmEvalExpect};

        let assertion = LlmEvalAssertion {
            id: "test-eval".to_string(),
            desc: None,
            pattern: "Does the following text contain the word 'hello'?\n\n{{output}}".to_string(),
            expect: LlmEvalExpect::Pass,
            timeout_ms: 60_000,
            json_schema: None,
        };

        let output = "hello world";
        let result = evaluate_llm_eval(output, &assertion).await?;
        assert!(
            result,
            "LLM should return result:true for 'hello' in 'hello world'"
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Claude CLI and API access"]
    async fn test_llm_eval_fail() -> Result<(), AssertionError> {
        use crate::types::{LlmEvalAssertion, LlmEvalExpect};

        let assertion = LlmEvalAssertion {
            id: "test-eval".to_string(),
            desc: None,
            pattern: "Does the following text contain the word 'banana'?\n\n{{output}}".to_string(),
            expect: LlmEvalExpect::Pass,
            timeout_ms: 60_000,
            json_schema: None,
        };

        let output = "hello world";
        let result = evaluate_llm_eval(output, &assertion).await?;
        assert!(
            !result,
            "LLM should return result:false for 'banana' in 'hello world'"
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Claude CLI and API access"]
    async fn test_llm_eval_expect_fail() -> Result<(), AssertionError> {
        use crate::types::{LlmEvalAssertion, LlmEvalExpect};

        // Expect fail means we want LLM to return result:false
        let assertion = LlmEvalAssertion {
            id: "test-eval".to_string(),
            desc: None,
            pattern: "Does the following text contain errors?\n\n{{output}}".to_string(),
            expect: LlmEvalExpect::Fail,
            timeout_ms: 60_000,
            json_schema: None,
        };

        let output = "This is correct code without any errors.";
        let result = evaluate_llm_eval(output, &assertion).await?;
        assert!(
            result,
            "expect:fail should pass when LLM returns result:false"
        );
        Ok(())
    }

    #[test]
    fn test_tool_called_present() -> Result<(), AssertionError> {
        let assertion = ToolCalledAssertion {
            id: "uses-websearch".to_string(),
            desc: None,
            pattern: "WebSearch".to_string(),
            expect: PatternExpect::Present,
        };

        let called_tools = vec![
            "Read".to_string(),
            "WebSearch".to_string(),
            "Write".to_string(),
        ];
        assert!(evaluate_tool_called(&called_tools, &assertion)?);

        let no_websearch = vec!["Read".to_string(), "Write".to_string()];
        assert!(!evaluate_tool_called(&no_websearch, &assertion)?);

        Ok(())
    }

    #[test]
    fn test_tool_called_absent() -> Result<(), AssertionError> {
        let assertion = ToolCalledAssertion {
            id: "no-bash".to_string(),
            desc: None,
            pattern: "Bash".to_string(),
            expect: PatternExpect::Absent,
        };

        let no_bash = vec!["Read".to_string(), "Write".to_string()];
        assert!(evaluate_tool_called(&no_bash, &assertion)?);

        let with_bash = vec!["Read".to_string(), "Bash".to_string()];
        assert!(!evaluate_tool_called(&with_bash, &assertion)?);

        Ok(())
    }

    #[test]
    fn test_tool_called_regex() -> Result<(), AssertionError> {
        let assertion = ToolCalledAssertion {
            id: "uses-web-tools".to_string(),
            desc: None,
            pattern: "Web(Search|Fetch)".to_string(),
            expect: PatternExpect::Present,
        };

        let with_websearch = vec!["Read".to_string(), "WebSearch".to_string()];
        assert!(evaluate_tool_called(&with_websearch, &assertion)?);

        let with_webfetch = vec!["Read".to_string(), "WebFetch".to_string()];
        assert!(evaluate_tool_called(&with_webfetch, &assertion)?);

        let no_web = vec!["Read".to_string(), "Write".to_string()];
        assert!(!evaluate_tool_called(&no_web, &assertion)?);

        Ok(())
    }

    #[test]
    fn test_tool_called_mcp_pattern() -> Result<(), AssertionError> {
        let assertion = ToolCalledAssertion {
            id: "uses-mcp".to_string(),
            desc: None,
            pattern: "mcp__.*".to_string(),
            expect: PatternExpect::Present,
        };

        let with_mcp = vec![
            "Read".to_string(),
            "mcp__plugin_serena_serena__find_symbol".to_string(),
        ];
        assert!(evaluate_tool_called(&with_mcp, &assertion)?);

        let no_mcp = vec!["Read".to_string(), "Write".to_string()];
        assert!(!evaluate_tool_called(&no_mcp, &assertion)?);

        Ok(())
    }

    #[test]
    fn test_extract_json_from_response() {
        // Plain JSON
        assert_eq!(
            extract_json_from_response(r#"{"result": true}"#),
            r#"{"result": true}"#
        );

        // JSON in code block
        assert_eq!(
            extract_json_from_response("```json\n{\"result\": true}\n```"),
            r#"{"result": true}"#
        );

        // JSON in plain code block
        assert_eq!(
            extract_json_from_response("```\n{\"result\": false}\n```"),
            r#"{"result": false}"#
        );

        // JSON with surrounding text
        let response =
            "Here is my response:\n```json\n{\"result\": true, \"reason\": \"test\"}\n```";
        assert_eq!(
            extract_json_from_response(response),
            r#"{"result": true, "reason": "test"}"#
        );
    }

    #[test]
    fn test_validate_json_schema_valid() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["result"],
            "properties": {
                "result": { "type": "boolean" },
                "reason": { "type": "string" }
            }
        });
        let valid_json = serde_json::json!({
            "result": true,
            "reason": "test passed"
        });
        assert!(validate_json_schema(&valid_json, &schema).is_ok());
    }

    #[test]
    fn test_validate_json_schema_invalid() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["result"],
            "properties": {
                "result": { "type": "boolean" }
            }
        });
        let invalid_json = serde_json::json!({
            "result": "not a boolean"
        });
        assert!(validate_json_schema(&invalid_json, &schema).is_err());
    }

    #[test]
    fn test_validate_json_schema_missing_required() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["result", "reason"],
            "properties": {
                "result": { "type": "boolean" },
                "reason": { "type": "string" }
            }
        });
        let missing_field = serde_json::json!({
            "result": true
        });
        assert!(validate_json_schema(&missing_field, &schema).is_err());
    }
}
