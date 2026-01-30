//! Claude CLI execution and skill extraction.

use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::Duration;

/// Errors that can occur during Claude execution.
#[derive(Error, Debug)]
pub enum ClaudeError {
    #[error("execution failed: {message}\nstderr: {stderr}\nstdout: {stdout}")]
    ExecutionFailed {
        message: String,
        stderr: String,
        stdout: String,
    },
    #[error("timeout after {timeout_ms}ms\nprompt: {prompt}\npartial_output: {partial_output}")]
    Timeout {
        timeout_ms: u64,
        prompt: String,
        partial_output: String,
    },
    #[error("JSON parse error: {source}\nraw_output: {raw_output}")]
    JsonParse {
        #[source]
        source: serde_json::Error,
        raw_output: String,
    },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Tool use entry from Claude response.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolUse {
    pub name: String,
    pub input: serde_json::Value,
}

/// Content block in assistant message.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
}

/// Assistant message from Claude CLI.
#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// Event from Claude CLI JSON output.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    #[serde(rename = "system")]
    System {
        #[serde(default)]
        tools: Vec<String>,
    },
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        result: String,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(other)]
    Unknown,
}

/// Claude CLI response (parsed from events).
#[derive(Debug, Clone, Default)]
pub struct ClaudeResponse {
    /// All tool uses from assistant messages.
    pub tool_uses: Vec<ToolUse>,
    /// Final result text from the result event.
    pub result: String,
}

impl ClaudeResponse {
    /// Get names of all tools that were called.
    #[must_use]
    pub fn called_tools(&self) -> Vec<&str> {
        self.tool_uses.iter().map(|t| t.name.as_str()).collect()
    }

    /// Parse Claude CLI JSON output (array of events) into `ClaudeResponse`.
    #[must_use]
    pub fn from_events(events: &[ClaudeEvent]) -> Self {
        let mut response = Self::default();

        for event in events {
            match event {
                ClaudeEvent::Assistant { message } => {
                    for block in &message.content {
                        if let ContentBlock::ToolUse { name, input } = block {
                            response.tool_uses.push(ToolUse {
                                name: name.clone(),
                                input: input.clone(),
                            });
                        }
                    }
                }
                ClaudeEvent::Result { result, .. } => {
                    response.result.clone_from(result);
                }
                _ => {}
            }
        }

        response
    }
}

/// Result of executing Claude.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub response: ClaudeResponse,
    pub raw_output: String,
    pub latency_ms: u64,
}

/// Execute Claude CLI with a prompt in a sandboxed environment.
///
/// Creates a temporary directory with the skill linked in `.claude/skills/`,
/// then executes Claude CLI with `--dangerously-skip-permissions`.
///
/// # Errors
/// Returns an error if:
/// - Sandbox setup fails
/// - Command execution fails
/// - Timeout occurs
/// - JSON parsing fails
#[allow(clippy::too_many_lines)]
pub async fn execute_claude(
    prompt: &str,
    model: &str,
    timeout_ms: u64,
    hook_path: Option<&str>,
    skill_dir: &str,
) -> Result<ExecutionResult, ClaudeError> {
    let start = std::time::Instant::now();

    // Create sandbox directory
    let sandbox_dir = tempfile::tempdir().map_err(|e| ClaudeError::ExecutionFailed {
        message: format!("failed to create sandbox directory: {e}"),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    // Setup .claude/skills/ in sandbox
    let sandbox_skills_dir = sandbox_dir.path().join(".claude/skills");
    std::fs::create_dir_all(&sandbox_skills_dir).map_err(|e| ClaudeError::ExecutionFailed {
        message: format!("failed to create skills directory: {e}"),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    // Get skill name from SKILL.md
    let skill_path = Path::new(skill_dir);
    let skill_name = extract_skill_name(skill_path)?;

    // Create symlink to skill directory
    let link_path = sandbox_skills_dir.join(&skill_name);
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        skill_path
            .canonicalize()
            .map_err(|e| ClaudeError::ExecutionFailed {
                message: format!("failed to resolve skill path: {e}"),
                stderr: String::new(),
                stdout: String::new(),
            })?,
        &link_path,
    )
    .map_err(|e| ClaudeError::ExecutionFailed {
        message: format!("failed to create skill symlink: {e}"),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(
        skill_path
            .canonicalize()
            .map_err(|e| ClaudeError::ExecutionFailed {
                message: format!("failed to resolve skill path: {e}"),
                stderr: String::new(),
                stdout: String::new(),
            })?,
        &link_path,
    )
    .map_err(|e| ClaudeError::ExecutionFailed {
        message: format!("failed to create skill symlink: {e}"),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg(prompt)
        .arg("--model")
        .arg(model)
        .arg("--output-format")
        .arg("json")
        .arg("--max-turns")
        .arg("5")
        .arg("--dangerously-skip-permissions")
        .current_dir(sandbox_dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set hook environment variable if provided
    if let Some(hook) = hook_path {
        cmd.env("CLAUDE_USER_PROMPT_SUBMIT_HOOK", hook);
    }

    // kill_on_drop ensures the child process is killed on timeout/drop
    cmd.kill_on_drop(true);

    let child = cmd.spawn().map_err(|e| ClaudeError::ExecutionFailed {
        message: e.to_string(),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    let result =
        tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await;

    let output = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(ClaudeError::ExecutionFailed {
                message: e.to_string(),
                stderr: String::new(),
                stdout: String::new(),
            });
        }
        Err(_) => {
            // Timeout occurred - process should be killed when dropped
            return Err(ClaudeError::Timeout {
                timeout_ms,
                prompt: truncate_string(prompt, 200),
                partial_output: String::from("[process killed due to timeout]"),
            });
        }
    };

    let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(ClaudeError::ExecutionFailed {
            message: format!("exit code: {:?}", output.status.code()),
            stderr,
            stdout: truncate_string(&stdout, 500),
        });
    }

    let events: Vec<ClaudeEvent> =
        serde_json::from_str(&stdout).map_err(|e| ClaudeError::JsonParse {
            source: e,
            raw_output: truncate_string(&stdout, 1000),
        })?;
    let response = ClaudeResponse::from_events(&events);

    Ok(ExecutionResult {
        response,
        raw_output: stdout,
        latency_ms,
    })
}

/// Truncate string to max length, adding "..." if truncated.
/// Uses char boundaries to avoid panic on multi-byte UTF-8.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find the last char boundary at or before max_len
        let truncated = match s.char_indices().take_while(|(i, _)| *i < max_len).last() {
            Some((i, c)) => &s[..i + c.len_utf8()],
            None => "",
        };
        format!("{truncated}...[truncated]")
    }
}

/// Extract skill names from a Claude response.
#[must_use]
pub fn extract_skills(response: &ClaudeResponse) -> Vec<String> {
    response
        .tool_uses
        .iter()
        .filter(|t| t.name == "Skill")
        .filter_map(|t| t.input.get("skill").and_then(|v| v.as_str()))
        .map(ToString::to_string)
        .collect()
}

/// Extract skill name from SKILL.md frontmatter.
fn extract_skill_name(skill_path: &Path) -> Result<String, ClaudeError> {
    let skill_md = skill_path.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).map_err(|e| ClaudeError::ExecutionFailed {
        message: format!("failed to read SKILL.md at {}: {e}", skill_md.display()),
        stderr: String::new(),
        stdout: String::new(),
    })?;

    // Parse YAML frontmatter
    if !content.starts_with("---") {
        return Err(ClaudeError::ExecutionFailed {
            message: "SKILL.md missing YAML frontmatter".to_string(),
            stderr: String::new(),
            stdout: String::new(),
        });
    }

    let end = content[3..]
        .find("---")
        .ok_or_else(|| ClaudeError::ExecutionFailed {
            message: "SKILL.md frontmatter not closed".to_string(),
            stderr: String::new(),
            stdout: String::new(),
        })?;

    let frontmatter = &content[3..3 + end];

    // Extract name field
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            let name = value.trim().trim_matches('"').trim_matches('\'');
            return Ok(name.to_string());
        }
    }

    Err(ClaudeError::ExecutionFailed {
        message: "SKILL.md frontmatter missing 'name' field".to_string(),
        stderr: String::new(),
        stdout: String::new(),
    })
}

/// Compute SHA256 hash of output for deduplication.
#[must_use]
pub fn compute_output_hash(output: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(output.as_bytes());
    let result = hasher.finalize();

    // Convert to hex string
    result.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_skills() {
        let response = ClaudeResponse {
            tool_uses: vec![
                ToolUse {
                    name: "Skill".to_string(),
                    input: serde_json::json!({"skill": "svelte5-runes"}),
                },
                ToolUse {
                    name: "Read".to_string(),
                    input: serde_json::json!({"path": "/tmp/file"}),
                },
                ToolUse {
                    name: "Skill".to_string(),
                    input: serde_json::json!({"skill": "other-skill"}),
                },
            ],
            result: String::new(),
        };

        let skills = extract_skills(&response);
        assert_eq!(skills, vec!["svelte5-runes", "other-skill"]);
    }

    #[test]
    fn test_extract_skills_empty() {
        let response = ClaudeResponse {
            tool_uses: vec![],
            result: String::new(),
        };

        let skills = extract_skills(&response);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_extract_skills_no_skill_tools() {
        let response = ClaudeResponse {
            tool_uses: vec![ToolUse {
                name: "Read".to_string(),
                input: serde_json::json!({"path": "/tmp/file"}),
            }],
            result: String::new(),
        };

        let skills = extract_skills(&response);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_parse_claude_events() -> Result<(), serde_json::Error> {
        let json = r#"[
            {"type": "system", "tools": ["Read", "Write"]},
            {"type": "assistant", "message": {"content": [
                {"type": "text", "text": "Hello"},
                {"type": "tool_use", "name": "Skill", "input": {"skill": "test-skill"}}
            ]}},
            {"type": "result", "result": "Some result", "is_error": false}
        ]"#;

        let events: Vec<ClaudeEvent> = serde_json::from_str(json)?;
        assert_eq!(events.len(), 3);

        let response = ClaudeResponse::from_events(&events);
        assert_eq!(response.tool_uses.len(), 1);
        assert_eq!(response.tool_uses[0].name, "Skill");
        assert_eq!(response.result, "Some result");
        Ok(())
    }

    #[test]
    fn test_parse_multiple_tool_uses() -> Result<(), serde_json::Error> {
        let json = r#"[
            {"type": "assistant", "message": {"content": [
                {"type": "tool_use", "name": "Skill", "input": {"skill": "skill-a"}},
                {"type": "tool_use", "name": "Read", "input": {"path": "/tmp"}},
                {"type": "tool_use", "name": "Skill", "input": {"skill": "skill-b"}}
            ]}},
            {"type": "result", "result": "Done", "is_error": false}
        ]"#;

        let events: Vec<ClaudeEvent> = serde_json::from_str(json)?;
        let response = ClaudeResponse::from_events(&events);

        assert_eq!(response.tool_uses.len(), 3);
        let skills = extract_skills(&response);
        assert_eq!(skills, vec!["skill-a", "skill-b"]);
        Ok(())
    }

    #[test]
    fn test_compute_output_hash() {
        let hash1 = compute_output_hash("hello world");
        let hash2 = compute_output_hash("hello world");
        let hash3 = compute_output_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 hex chars
    }

    #[test]
    fn test_called_tools() {
        let response = ClaudeResponse {
            tool_uses: vec![
                ToolUse {
                    name: "WebSearch".to_string(),
                    input: serde_json::json!({"query": "test"}),
                },
                ToolUse {
                    name: "Read".to_string(),
                    input: serde_json::json!({"path": "/tmp"}),
                },
                ToolUse {
                    name: "WebSearch".to_string(),
                    input: serde_json::json!({"query": "another"}),
                },
            ],
            result: String::new(),
        };

        let tools = response.called_tools();
        assert_eq!(tools, vec!["WebSearch", "Read", "WebSearch"]);
    }

    #[test]
    fn test_called_tools_empty() {
        let response = ClaudeResponse::default();
        let tools = response.called_tools();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_truncate_string_short() {
        let s = "hello";
        let result = truncate_string(s, 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_exact_length() {
        let s = "hello";
        let result = truncate_string(s, 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_truncated() {
        let s = "hello world";
        let result = truncate_string(s, 5);
        assert_eq!(result, "hello...[truncated]");
    }

    #[test]
    fn test_truncate_string_utf8_multibyte() {
        // Japanese characters: each is 3 bytes in UTF-8
        let s = "日本語テスト"; // 6 chars, 18 bytes

        // Truncate at byte 5 - should not panic and should respect char boundary
        let result = truncate_string(s, 5);
        // "日" is 3 bytes, so only first char fits under 5 bytes
        assert!(result.starts_with("日"));
        assert!(result.ends_with("...[truncated]"));
    }

    #[test]
    fn test_truncate_string_utf8_emoji() {
        // Emoji: 4 bytes in UTF-8
        let s = "🎉🎊🎁";

        // Truncate at byte 5 - should not panic
        let result = truncate_string(s, 5);
        // Only first emoji (4 bytes) fits under 5 bytes
        assert!(result.starts_with("🎉"));
        assert!(result.ends_with("...[truncated]"));
    }

    #[test]
    fn test_truncate_string_empty() {
        let s = "";
        let result = truncate_string(s, 10);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_string_zero_max() {
        let s = "hello";
        let result = truncate_string(s, 0);
        assert_eq!(result, "...[truncated]");
    }
}
