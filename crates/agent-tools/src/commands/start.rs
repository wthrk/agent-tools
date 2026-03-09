use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use crate::paths;

pub fn run_claude(extra_args: &[String]) -> Result<()> {
    let mut cmd = Command::new("claude");
    cmd.args(extra_args);

    let runpod_env = paths::claude_home()?.join("runpod.env");
    if runpod_env.exists() {
        let envs = parse_runpod_env_file(&runpod_env)?;
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    let status = cmd.status().context("Failed to start claude command")?;
    if status.success() {
        return Ok(());
    }
    if let Some(code) = status.code() {
        bail!("claude exited with status code {}", code);
    }
    bail!("claude terminated by signal");
}

pub fn run_codex(extra_args: &[String]) -> Result<()> {
    let status = Command::new("codex")
        .args(extra_args)
        .status()
        .context("Failed to start codex command")?;
    if status.success() {
        return Ok(());
    }
    if let Some(code) = status.code() {
        bail!("codex exited with status code {}", code);
    }
    bail!("codex terminated by signal");
}

fn parse_runpod_env_file(path: &std::path::Path) -> Result<BTreeMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut out = BTreeMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        let value = expand_shell_like_value(v.trim());
        out.insert(key.to_string(), value);
    }
    Ok(out)
}

fn expand_shell_like_value(value: &str) -> String {
    let mut trimmed = value.trim().to_string();
    if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        && trimmed.len() >= 2
    {
        trimmed = trimmed[1..trimmed.len() - 1].to_string();
    }

    if let Some(inner) = trimmed.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
        if let Some(var_name) = inner.strip_suffix(":-") {
            return std::env::var(var_name).unwrap_or_default();
        }
        return std::env::var(inner).unwrap_or_default();
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::{expand_shell_like_value, parse_runpod_env_file};
    use anyhow::Result;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn expand_shell_like_value_supports_default_syntax() {
        assert_eq!(expand_shell_like_value("\"abc\""), "abc");
        assert_eq!(expand_shell_like_value("'xyz'"), "xyz");
    }

    #[test]
    fn parse_runpod_env_file_reads_exports() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("runpod.env");
        fs::write(
            &path,
            "export ANTHROPIC_BASE_URL=\"https://x\"\nexport ANTHROPIC_AUTH_TOKEN=\"${RUNPOD_API_KEY:-}\"\n",
        )?;
        let parsed = parse_runpod_env_file(&path)?;
        assert_eq!(
            parsed.get("ANTHROPIC_BASE_URL"),
            Some(&"https://x".to_string())
        );
        assert!(parsed.contains_key("ANTHROPIC_AUTH_TOKEN"));
        Ok(())
    }
}
