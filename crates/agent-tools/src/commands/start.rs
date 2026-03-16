use anyhow::{Context, Result, bail};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::paths;

struct BridgeProcess {
    child: Child,
    state_path: std::path::PathBuf,
}

pub fn run_claude(extra_args: &[String]) -> Result<()> {
    let mut cmd = Command::new("claude");
    cmd.args(extra_args);

    let mut bridge: Option<BridgeProcess> = None;
    let runpod_env = paths::claude_home()?.join("runpod.env");
    if runpod_env.exists() {
        let mut envs = parse_runpod_env_file(&runpod_env)?;
        bridge = maybe_start_runpod_bridge(&mut envs)?;
        for (key, value) in envs {
            cmd.env(key, value);
        }
    }

    let status = cmd.status().context("Failed to start claude command")?;
    if let Some(mut bridge_proc) = bridge {
        let _ = bridge_proc.child.kill();
        let _ = bridge_proc.child.wait();
        let _ = fs::remove_file(&bridge_proc.state_path);
    }
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

fn parse_runpod_env_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut out = BTreeMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        out.insert(key.to_string(), expand_shell_like_value(value.trim()));
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
        if let Some((var_name, default_value)) = inner.split_once(":-") {
            match std::env::var(var_name) {
                Ok(value) if !value.is_empty() => return value,
                _ => return default_value.to_string(),
            }
        }
        return std::env::var(inner).unwrap_or_default();
    }
    trimmed
}

fn maybe_start_runpod_bridge(envs: &mut BTreeMap<String, String>) -> Result<Option<BridgeProcess>> {
    let Some(base_url) = envs.get("ANTHROPIC_BASE_URL").cloned() else {
        return Ok(None);
    };
    let Some(token) = envs.get("ANTHROPIC_AUTH_TOKEN").cloned() else {
        return Ok(None);
    };
    if !base_url.contains("api.runpod.ai/v2/") || !base_url.contains("/openai") {
        return Ok(None);
    }

    let bridge_script = paths::agent_tools_home()?.join("global/claude/runpod_bridge.py");
    if !bridge_script.exists() {
        bail!(
            "RunPod bridge script not found: {}",
            bridge_script.display()
        );
    }

    let listener =
        TcpListener::bind("127.0.0.1:0").context("Failed to reserve local bridge port")?;
    let port = listener
        .local_addr()
        .context("Failed to inspect local bridge port")?
        .port();
    drop(listener);

    let log_dir = paths::logs_dir()?;
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create {}", log_dir.display()))?;
    let claude_home = paths::claude_home()?;
    let log_path = log_dir.join("runpod_bridge.log");
    let state_path = claude_home.join("runpod_bridge_state.json");

    let child = Command::new("python3")
        .arg(&bridge_script)
        .arg("--port")
        .arg(port.to_string())
        .arg("--upstream-base")
        .arg(base_url)
        .arg("--upstream-token")
        .arg(token)
        .env("RUNPOD_BRIDGE_LOG_FILE", &log_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to start runpod bridge: {}", bridge_script.display()))?;

    wait_for_bridge(port).with_context(|| "RunPod bridge failed to become ready")?;
    let state = json!({
        "pid": child.id(),
        "port": port,
        "log_file": log_path,
        "base_url": format!("http://127.0.0.1:{port}")
    });
    fs::write(
        &state_path,
        serde_json::to_string_pretty(&state).context("Failed to serialize bridge state")?,
    )
    .with_context(|| format!("Failed to write {}", state_path.display()))?;

    envs.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        format!("http://127.0.0.1:{port}"),
    );
    envs.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        "local-bridge".to_string(),
    );
    Ok(Some(BridgeProcess { child, state_path }))
}

fn wait_for_bridge(port: u16) -> Result<()> {
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.write_all(b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("Timed out waiting for local bridge on 127.0.0.1:{port}");
}

#[cfg(test)]
mod tests {
    use super::{expand_shell_like_value, parse_runpod_env_file};
    use anyhow::Result;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn expand_shell_like_value_supports_quotes() {
        assert_eq!(expand_shell_like_value("\"abc\""), "abc");
        assert_eq!(expand_shell_like_value("'xyz'"), "xyz");
    }

    #[test]
    fn expand_shell_like_value_supports_default_expression() {
        let key = "AGENT_TOOLS_TEST_RUNPOD_DEFAULT";
        unsafe {
            std::env::remove_var(key);
        }
        assert_eq!(
            expand_shell_like_value(&format!("\"${{{key}:-fallback}}\"")),
            "fallback"
        );

        unsafe {
            std::env::set_var(key, "value");
        }
        assert_eq!(
            expand_shell_like_value(&format!("\"${{{key}:-fallback}}\"")),
            "value"
        );

        unsafe {
            std::env::set_var(key, "");
        }
        assert_eq!(
            expand_shell_like_value(&format!("\"${{{key}:-fallback}}\"")),
            "fallback"
        );

        unsafe {
            std::env::remove_var(key);
        }
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
