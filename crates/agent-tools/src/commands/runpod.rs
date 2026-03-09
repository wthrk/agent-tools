use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value as TomlValue;

use crate::commands::profile;
use crate::paths;

#[derive(Debug, Deserialize)]
struct RunpodConfig {
    deployment: Option<String>,
    name: String,
    endpoint_id: Option<String>,
    claude_base_url_template: Option<String>,
    claude_auth_token_env: Option<String>,
    template_id: Option<String>,
    image: Option<String>,
    gpu_id: Option<String>,
    cloud_type: Option<String>,
    compute_type: Option<String>,
    gpu_count: Option<u32>,
    container_disk_in_gb: Option<u32>,
    volume_in_gb: Option<u32>,
    volume_mount_path: Option<String>,
    ports: Option<Vec<String>>,
    data_center_ids: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    workers_min: Option<u32>,
    workers_max: Option<u32>,
    public_ip: Option<bool>,
    global_networking: Option<bool>,
    ssh: Option<bool>,
    start_after_create: Option<bool>,
}

pub fn up(profile_name: &str) -> Result<()> {
    let config = load_config(profile_name)?;
    validate_config(&config)?;
    ensure_runpodctl_exists()?;

    println!("{}", format!("Applying profile '{}'", profile_name).green());
    profile::use_profile(profile_name)?;

    if is_serverless(&config) {
        let endpoint_id = resolve_serverless_endpoint_id(&config)?;
        println!("{} {}", "✓ Endpoint:".green(), endpoint_id.cyan());
        write_codex_endpoint(&endpoint_id)?;
        let claude_base_url = write_claude_endpoint(&endpoint_id, &config)?;
        verify_claude_endpoint(&claude_base_url, &config)?;
        println!(
            "{} {}",
            "✓ Updated Codex endpoint to".green(),
            endpoint_id.cyan()
        );
        println!(
            "{} {}",
            "✓ Wrote Claude env file:".green(),
            paths::claude_home()?.join("runpod.env").display()
        );
        println!("{}", "✓ Claude endpoint verification passed".green());
        return Ok(());
    }

    let create_args = build_create_args(&config)?;
    println!("{}", "Creating RunPod pod...".green().bold());
    let create_output = run_runpodctl(&create_args)?;

    let pod_id = extract_pod_id(&create_output)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract pod id from runpodctl output"))?;
    println!("{} {}", "✓ Created pod:".green(), pod_id.cyan());

    if config.start_after_create.unwrap_or(true) {
        println!("{}", "Starting pod...".green().bold());
        start_pod(&pod_id)?;
        println!("{} {}", "✓ Started pod:".green(), pod_id.cyan());
    } else {
        println!("{}", "Skipping start (start_after_create=false)".yellow());
    }

    Ok(())
}

pub fn status(profile_name: &str) -> Result<()> {
    let config = load_config(profile_name)?;
    validate_config(&config)?;
    ensure_runpodctl_exists()?;

    if is_serverless(&config) {
        let endpoints = run_runpodctl_json(&["-o", "json", "serverless", "list"])?;
        let item = find_item_by_name_or_id(
            &endpoints,
            &config.name,
            config.endpoint_id.as_deref(),
            "endpointId",
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No matching serverless endpoint found for '{}'. Run `agent-tools runpod up {}` first.",
                config.name,
                profile_name
            )
        })?;
        let endpoint_id = extract_identifier(item, "endpointId")
            .ok_or_else(|| anyhow::anyhow!("Failed to read endpoint id from runpodctl output"))?;

        println!("{} {}", "Deployment:".green(), "serverless".cyan());
        println!("{} {}", "Name:".green(), config.name.cyan());
        println!("{} {}", "Endpoint:".green(), endpoint_id.cyan());

        if let Some(state) = extract_state(item) {
            println!("{} {}", "State:".green(), state.cyan());
        }
        if let Some(workers) = extract_workers(item) {
            println!("{} {}", "Workers:".green(), workers.cyan());
        }

        let base_url = render_claude_base_url(&endpoint_id, &config)?;
        println!("{} {}", "Claude base URL:".green(), base_url.cyan());
        verify_claude_endpoint(&base_url, &config)?;
        println!("{}", "✓ Claude endpoint verification passed".green());
        return Ok(());
    }

    let pods = run_runpodctl_json(&["-o", "json", "pod", "list"])?;
    let item = find_item_by_name_or_id(&pods, &config.name, None, "podId").ok_or_else(|| {
        anyhow::anyhow!(
            "No matching pod found for '{}'. Run `agent-tools runpod up {}` first.",
            config.name,
            profile_name
        )
    })?;
    let pod_id = extract_identifier(item, "podId")
        .ok_or_else(|| anyhow::anyhow!("Failed to read pod id from runpodctl output"))?;

    println!("{} {}", "Deployment:".green(), "pod".cyan());
    println!("{} {}", "Name:".green(), config.name.cyan());
    println!("{} {}", "Pod:".green(), pod_id.cyan());
    if let Some(state) = extract_state(item) {
        println!("{} {}", "State:".green(), state.cyan());
    }
    Ok(())
}

fn load_config(profile_name: &str) -> Result<RunpodConfig> {
    let path = paths::claude_templates_dir()?
        .join(profile_name)
        .join("runpod.yaml");
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: RunpodConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}

fn validate_config(config: &RunpodConfig) -> Result<()> {
    if config.name.trim().is_empty() {
        bail!("runpod.yaml: 'name' cannot be empty");
    }

    if is_serverless(config) {
        if config.endpoint_id.is_none() && config.template_id.is_none() {
            bail!("runpod.yaml: serverless requires either 'endpoint_id' or 'template_id'");
        }
        return Ok(());
    }

    if config.template_id.is_none() && config.image.is_none() {
        bail!("runpod.yaml: either 'template_id' or 'image' is required");
    }

    if config.template_id.is_some() && config.image.is_some() {
        bail!("runpod.yaml: specify only one of 'template_id' or 'image'");
    }

    let compute_type = config.compute_type.as_deref().unwrap_or("GPU");
    if compute_type.eq_ignore_ascii_case("GPU") && config.gpu_id.is_none() {
        bail!("runpod.yaml: 'gpu_id' is required when compute_type is GPU");
    }

    Ok(())
}

fn ensure_runpodctl_exists() -> Result<()> {
    let output = Command::new("runpodctl")
        .arg("--version")
        .output()
        .context("Failed to execute runpodctl --version")?;

    if !output.status.success() {
        bail!("runpodctl is not available. Install runpodctl and configure RUNPOD_API_KEY first.");
    }
    Ok(())
}

fn build_create_args(config: &RunpodConfig) -> Result<Vec<String>> {
    let mut args = vec![
        "-o".to_string(),
        "json".to_string(),
        "pod".to_string(),
        "create".to_string(),
        "--name".to_string(),
        config.name.clone(),
    ];

    if let Some(v) = &config.template_id {
        args.push("--template-id".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.image {
        args.push("--image".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.gpu_id {
        args.push("--gpu-id".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.cloud_type {
        args.push("--cloud-type".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.compute_type {
        args.push("--compute-type".to_string());
        args.push(v.clone());
    }
    if let Some(v) = config.gpu_count {
        args.push("--gpu-count".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.container_disk_in_gb {
        args.push("--container-disk-in-gb".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.volume_in_gb {
        args.push("--volume-in-gb".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = &config.volume_mount_path {
        args.push("--volume-mount-path".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.ports {
        if !v.is_empty() {
            args.push("--ports".to_string());
            args.push(v.join(","));
        }
    }
    if let Some(v) = &config.data_center_ids {
        if !v.is_empty() {
            args.push("--data-center-ids".to_string());
            args.push(v.join(","));
        }
    }
    if let Some(v) = &config.env {
        if !v.is_empty() {
            args.push("--env".to_string());
            args.push(env_to_json(v)?);
        }
    }
    if config.public_ip.unwrap_or(false) {
        args.push("--public-ip".to_string());
    }
    if config.global_networking.unwrap_or(false) {
        args.push("--global-networking".to_string());
    }
    if let Some(v) = config.ssh {
        if !v {
            args.push("--ssh=false".to_string());
        }
    }

    Ok(args)
}

fn build_serverless_create_args(config: &RunpodConfig) -> Result<Vec<String>> {
    let template_id = config
        .template_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runpod.yaml: serverless create requires template_id"))?;
    let mut args = vec![
        "-o".to_string(),
        "json".to_string(),
        "serverless".to_string(),
        "create".to_string(),
        "--name".to_string(),
        config.name.clone(),
        "--template-id".to_string(),
        template_id.clone(),
    ];

    if let Some(v) = &config.gpu_id {
        args.push("--gpu-id".to_string());
        args.push(v.clone());
    }
    if let Some(v) = &config.compute_type {
        args.push("--compute-type".to_string());
        args.push(v.clone());
    }
    if let Some(v) = config.gpu_count {
        args.push("--gpu-count".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.workers_min {
        args.push("--workers-min".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = config.workers_max {
        args.push("--workers-max".to_string());
        args.push(v.to_string());
    }
    if let Some(v) = &config.data_center_ids {
        if !v.is_empty() {
            args.push("--data-center-ids".to_string());
            args.push(v.join(","));
        }
    }
    Ok(args)
}

fn resolve_serverless_endpoint_id(config: &RunpodConfig) -> Result<String> {
    if let Some(endpoint_id) = &config.endpoint_id {
        return Ok(endpoint_id.clone());
    }

    if let Some(existing) = find_serverless_endpoint_id_by_name(&config.name)? {
        println!(
            "{} {}",
            "Reusing existing serverless endpoint:".green(),
            existing.cyan()
        );
        return Ok(existing);
    }

    println!(
        "{}",
        "Creating RunPod serverless endpoint...".green().bold()
    );
    ensure_serverless_template_exists(config)?;
    let create_output = run_runpodctl(&build_serverless_create_args(config)?)?;
    extract_endpoint_id(&create_output)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract endpoint id from runpodctl output"))
}

fn ensure_serverless_template_exists(config: &RunpodConfig) -> Result<()> {
    let template_id = config
        .template_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("runpod.yaml: serverless create requires template_id"))?;
    let output = Command::new("runpodctl")
        .args(["-o", "json", "template", "get", template_id])
        .output()
        .context("Failed to run runpodctl template get")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    bail!(
        "runpod.yaml template_id '{}' was not found. Update template_id to an existing RunPod template id.\n{}{}",
        template_id,
        stdout,
        stderr
    );
}

fn find_serverless_endpoint_id_by_name(name: &str) -> Result<Option<String>> {
    let value = run_runpodctl_json(&["-o", "json", "serverless", "list"])?;
    Ok(extract_endpoint_id_from_list_by_name(&value, name))
}

fn env_to_json(env: &BTreeMap<String, String>) -> Result<String> {
    let mut map = Map::new();
    for (k, v) in env {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    serde_json::to_string(&Value::Object(map)).context("Failed to serialize env map")
}

fn run_runpodctl(args: &[String]) -> Result<Value> {
    run_runpodctl_json_owned(args, "runpodctl command")
}

fn run_runpodctl_json(args: &[&str]) -> Result<Value> {
    run_runpodctl_json_owned(
        &args.iter().map(|v| (*v).to_string()).collect::<Vec<_>>(),
        "runpodctl command",
    )
}

fn run_runpodctl_json_owned(args: &[String], context_name: &str) -> Result<Value> {
    let output = Command::new("runpodctl")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run {context_name}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{context_name} failed:\n{}{}", stdout, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).context("Failed to parse runpodctl output as JSON")
}

fn extract_pod_id(value: &Value) -> Option<String> {
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        return Some(id.to_string());
    }
    if let Some(id) = value.get("podId").and_then(Value::as_str) {
        return Some(id.to_string());
    }
    if let Some(id) = value.get("pod_id").and_then(Value::as_str) {
        return Some(id.to_string());
    }
    if let Some(id) = value
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(Value::as_str)
    {
        return Some(id.to_string());
    }
    None
}

fn extract_endpoint_id(value: &Value) -> Option<String> {
    extract_pod_id(value)
}

fn extract_endpoint_id_from_list_by_name(value: &Value, name: &str) -> Option<String> {
    if let Some(arr) = value.as_array() {
        return extract_endpoint_id_from_items(arr, name);
    }
    if let Some(arr) = value.get("data").and_then(Value::as_array) {
        return extract_endpoint_id_from_items(arr, name);
    }
    if let Some(arr) = value.get("items").and_then(Value::as_array) {
        return extract_endpoint_id_from_items(arr, name);
    }
    None
}

fn extract_endpoint_id_from_items(items: &[Value], name: &str) -> Option<String> {
    for item in items {
        let item_name = item
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| item.get("endpointName").and_then(Value::as_str));
        if !name_matches(item_name, name) {
            continue;
        }
        if let Some(id) = item
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| item.get("endpointId").and_then(Value::as_str))
            .or_else(|| item.get("endpoint_id").and_then(Value::as_str))
        {
            return Some(id.to_string());
        }
    }
    None
}

fn find_item_by_name_or_id<'a>(
    value: &'a Value,
    name: &str,
    expected_id: Option<&str>,
    id_fallback_key: &str,
) -> Option<&'a Value> {
    for item in value_items(value) {
        let item_name = item
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| item.get("endpointName").and_then(Value::as_str));
        let item_id = extract_identifier(item, id_fallback_key);

        if expected_id.is_some_and(|id| item_id.as_deref() == Some(id)) {
            return Some(item);
        }
        if name_matches(item_name, name) {
            return Some(item);
        }
    }
    None
}

fn name_matches(item_name: Option<&str>, expected: &str) -> bool {
    item_name.is_some_and(|value| value == expected || value.starts_with(expected))
}

fn value_items(value: &Value) -> &[Value] {
    if let Some(arr) = value.as_array() {
        return arr;
    }
    if let Some(arr) = value.get("data").and_then(Value::as_array) {
        return arr;
    }
    if let Some(arr) = value.get("items").and_then(Value::as_array) {
        return arr;
    }
    &[]
}

fn extract_identifier(item: &Value, fallback_key: &str) -> Option<String> {
    item.get("id")
        .and_then(Value::as_str)
        .or_else(|| item.get("endpointId").and_then(Value::as_str))
        .or_else(|| item.get("endpoint_id").and_then(Value::as_str))
        .or_else(|| item.get("podId").and_then(Value::as_str))
        .or_else(|| item.get("pod_id").and_then(Value::as_str))
        .or_else(|| item.get(fallback_key).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn extract_state(item: &Value) -> Option<String> {
    item.get("status")
        .and_then(Value::as_str)
        .or_else(|| item.get("desiredStatus").and_then(Value::as_str))
        .or_else(|| item.get("workerStatus").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn extract_workers(item: &Value) -> Option<String> {
    let current = item
        .get("workersCurrent")
        .or_else(|| item.get("workerCount"))
        .and_then(Value::as_u64);
    let min = item.get("workersMin").and_then(Value::as_u64);
    let max = item.get("workersMax").and_then(Value::as_u64);

    match (current, min, max) {
        (Some(c), Some(lo), Some(hi)) => Some(format!("{c} (min={lo}, max={hi})")),
        (Some(c), _, _) => Some(c.to_string()),
        _ => None,
    }
}

fn start_pod(pod_id: &str) -> Result<()> {
    let output = Command::new("runpodctl")
        .args(["pod", "start", pod_id])
        .output()
        .context("Failed to run runpodctl pod start")?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if is_already_running_message(&stderr) || is_already_running_message(&stdout) {
        println!("{}", "Pod is already running".yellow());
        return Ok(());
    }

    bail!("runpodctl pod start failed:\n{}{}", stdout, stderr);
}

fn is_already_running_message(text: &str) -> bool {
    text.to_ascii_lowercase().contains("already running")
}

fn is_serverless(config: &RunpodConfig) -> bool {
    config
        .deployment
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("serverless"))
}

fn write_codex_endpoint(endpoint_id: &str) -> Result<()> {
    let codex_home = paths::codex_home()?;
    fs::create_dir_all(&codex_home)
        .with_context(|| format!("Failed to create {}", codex_home.display()))?;
    let base_url = format!("https://api.runpod.ai/v2/{endpoint_id}/openai/v1");

    let local_path = codex_home.join("config.local.toml");
    upsert_base_url(&local_path, &base_url)?;

    let config_path = codex_home.join("config.toml");
    if config_path.exists() {
        upsert_base_url(&config_path, &base_url)?;
    }
    Ok(())
}

fn write_claude_endpoint(endpoint_id: &str, config: &RunpodConfig) -> Result<String> {
    let claude_home = paths::claude_home()?;
    fs::create_dir_all(&claude_home)
        .with_context(|| format!("Failed to create {}", claude_home.display()))?;

    let base_url = render_claude_base_url(endpoint_id, config)?;
    let auth_env = config
        .claude_auth_token_env
        .as_deref()
        .unwrap_or("RUNPOD_API_KEY");
    let token = resolve_runpod_api_key(auth_env)?;
    let env_path = claude_home.join("runpod.env");
    let expected_path = claude_home.join("runpod_expected_anthropic_base_url");
    let env_content = format!(
        "# Generated by agent-tools runpod up\nexport ANTHROPIC_BASE_URL=\"{base_url}\"\nexport ANTHROPIC_AUTH_TOKEN=\"{token}\"\n"
    );
    fs::write(&env_path, env_content)
        .with_context(|| format!("Failed to write {}", env_path.display()))?;
    fs::write(&expected_path, format!("{base_url}\n"))
        .with_context(|| format!("Failed to write {}", expected_path.display()))?;
    Ok(base_url)
}

fn render_claude_base_url(endpoint_id: &str, config: &RunpodConfig) -> Result<String> {
    let template = config
        .claude_base_url_template
        .as_deref()
        .unwrap_or("https://api.runpod.ai/v2/{endpoint_id}");
    if !template.contains("{endpoint_id}") {
        bail!("runpod.yaml: claude_base_url_template must include '{{endpoint_id}}'");
    }
    Ok(template.replace("{endpoint_id}", endpoint_id))
}

fn verify_claude_endpoint(base_url: &str, config: &RunpodConfig) -> Result<()> {
    let auth_env = config
        .claude_auth_token_env
        .as_deref()
        .unwrap_or("RUNPOD_API_KEY");
    let token = resolve_runpod_api_key(auth_env)?;

    let trimmed = base_url.trim_end_matches('/');
    let url = format!("{trimmed}/v1/messages");
    let output = Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-H",
            "content-type: application/json",
            "-H",
            &format!("x-api-key: {token}"),
            "-H",
            "anthropic-version: 2023-06-01",
            "-X",
            "POST",
            "--data",
            r#"{"model":"healthcheck","max_tokens":1,"messages":[{"role":"user","content":"ping"}]}"#,
            &url,
        ])
        .output()
        .context("Failed to execute curl for Claude endpoint verification")?;

    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if is_reachable_http_status(&code) {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!(
        "Claude endpoint verification failed (status={}). url={}\n{}",
        code,
        url,
        stderr
    );
}

fn resolve_runpod_api_key(auth_env: &str) -> Result<String> {
    if let Ok(value) = std::env::var(auth_env) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| anyhow::anyhow!("Failed to resolve home directory from HOME"))?;
    let config_path = home.join(".runpod/config.toml");
    if !config_path.exists() {
        bail!(
            "RunPod API key not found. Set '{}' or configure {}",
            auth_env,
            config_path.display()
        );
    }
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let value = toml::from_str::<TomlValue>(&content)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;
    let key = value
        .get("apikey")
        .and_then(TomlValue::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "RunPod API key not found. Set '{}' or update {}",
                auth_env,
                config_path.display()
            )
        })?;
    Ok(key.to_string())
}

fn is_reachable_http_status(code: &str) -> bool {
    matches!(code, "200" | "400" | "401" | "403" | "404" | "405")
}

fn upsert_base_url(path: &Path, base_url: &str) -> Result<()> {
    let mut root = if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str::<TomlValue>(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?
    } else {
        TomlValue::Table(toml::map::Map::new())
    };

    ensure_table(&mut root, "model_providers");
    if let Some(model_providers) = root
        .get_mut("model_providers")
        .and_then(TomlValue::as_table_mut)
    {
        let provider = model_providers
            .entry("runpod_serverless".to_string())
            .or_insert_with(|| TomlValue::Table(toml::map::Map::new()));
        if let Some(provider_table) = provider.as_table_mut() {
            provider_table.insert(
                "base_url".to_string(),
                TomlValue::String(base_url.to_string()),
            );
        }
    }

    let output = toml::to_string_pretty(&root)
        .with_context(|| format!("Failed to serialize {}", path.display()))?;
    fs::write(path, output).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn ensure_table(root: &mut TomlValue, key: &str) {
    if !root.is_table() {
        *root = TomlValue::Table(toml::map::Map::new());
    }
    if let Some(table) = root.as_table_mut() {
        table
            .entry(key.to_string())
            .or_insert_with(|| TomlValue::Table(toml::map::Map::new()));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RunpodConfig, build_create_args, build_serverless_create_args, extract_endpoint_id,
        extract_endpoint_id_from_list_by_name, extract_pod_id, extract_workers,
        find_item_by_name_or_id, is_already_running_message, is_reachable_http_status,
        render_claude_base_url, upsert_base_url,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn build_create_args_includes_required_and_optional() {
        let mut env = BTreeMap::new();
        env.insert("A".to_string(), "1".to_string());
        let cfg = RunpodConfig {
            deployment: None,
            name: "runpod-llm".to_string(),
            endpoint_id: None,
            claude_base_url_template: None,
            claude_auth_token_env: None,
            template_id: Some("tpl-1".to_string()),
            image: None,
            gpu_id: Some("NVIDIA RTX 4090".to_string()),
            cloud_type: Some("SECURE".to_string()),
            compute_type: Some("GPU".to_string()),
            gpu_count: Some(1),
            container_disk_in_gb: Some(20),
            volume_in_gb: None,
            volume_mount_path: None,
            ports: Some(vec!["8080/http".to_string()]),
            data_center_ids: None,
            env: Some(env),
            workers_min: None,
            workers_max: None,
            public_ip: Some(false),
            global_networking: Some(false),
            ssh: Some(true),
            start_after_create: Some(true),
        };

        let args = build_create_args(&cfg).expect("args");
        assert!(args.iter().any(|a| a == "--template-id"));
        assert!(args.iter().any(|a| a == "tpl-1"));
        assert!(args.iter().any(|a| a == "--gpu-id"));
    }

    #[test]
    fn extract_pod_id_supports_multiple_shapes() {
        assert_eq!(
            extract_pod_id(&json!({"id": "abc"})),
            Some("abc".to_string())
        );
        assert_eq!(
            extract_pod_id(&json!({"podId": "def"})),
            Some("def".to_string())
        );
        assert_eq!(
            extract_pod_id(&json!({"data": {"id": "ghi"}})),
            Some("ghi".to_string())
        );
    }

    #[test]
    fn extract_endpoint_id_supports_multiple_shapes() {
        assert_eq!(
            extract_endpoint_id(&json!({"id": "ep-1"})),
            Some("ep-1".to_string())
        );
        assert_eq!(
            extract_endpoint_id(&json!({"data": {"id": "ep-2"}})),
            Some("ep-2".to_string())
        );
    }

    #[test]
    fn already_running_message_detected_case_insensitive() {
        assert!(is_already_running_message("Pod is already running"));
        assert!(is_already_running_message("ALREADY RUNNING"));
        assert!(!is_already_running_message("unknown error"));
    }

    #[test]
    fn build_serverless_args_includes_template_and_workers() {
        let cfg = RunpodConfig {
            deployment: Some("serverless".to_string()),
            name: "runpod-sls".to_string(),
            endpoint_id: None,
            claude_base_url_template: None,
            claude_auth_token_env: None,
            template_id: Some("tpl-sls".to_string()),
            image: None,
            gpu_id: Some("NVIDIA L40S".to_string()),
            cloud_type: None,
            compute_type: Some("GPU".to_string()),
            gpu_count: Some(1),
            container_disk_in_gb: None,
            volume_in_gb: None,
            volume_mount_path: None,
            ports: None,
            data_center_ids: None,
            env: None,
            workers_min: Some(0),
            workers_max: Some(2),
            public_ip: None,
            global_networking: None,
            ssh: None,
            start_after_create: None,
        };

        let args = build_serverless_create_args(&cfg).expect("args");
        assert!(args.iter().any(|v| v == "serverless"));
        assert!(args.iter().any(|v| v == "--template-id"));
        assert!(args.iter().any(|v| v == "tpl-sls"));
    }

    #[test]
    fn upsert_base_url_updates_nested_table() {
        let dir = TempDir::new().expect("tmp");
        let path = dir.path().join("config.local.toml");
        fs::write(&path, "model = \"x\"\n").expect("write");
        upsert_base_url(&path, "https://api.runpod.ai/v2/ep/openai/v1").expect("upsert");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("[model_providers.runpod_serverless]"));
        assert!(content.contains("base_url = \"https://api.runpod.ai/v2/ep/openai/v1\""));
    }

    #[test]
    fn render_claude_base_url_from_template() {
        let cfg = RunpodConfig {
            deployment: Some("serverless".to_string()),
            name: "runpod-sls".to_string(),
            endpoint_id: None,
            claude_base_url_template: Some("https://api.runpod.ai/v2/{endpoint_id}".to_string()),
            claude_auth_token_env: None,
            template_id: Some("tpl".to_string()),
            image: None,
            gpu_id: Some("NVIDIA L40S".to_string()),
            cloud_type: None,
            compute_type: Some("GPU".to_string()),
            gpu_count: Some(1),
            container_disk_in_gb: None,
            volume_in_gb: None,
            volume_mount_path: None,
            ports: None,
            data_center_ids: None,
            env: None,
            workers_min: Some(0),
            workers_max: Some(2),
            public_ip: None,
            global_networking: None,
            ssh: None,
            start_after_create: None,
        };
        let url = render_claude_base_url("ep123", &cfg).expect("url");
        assert_eq!(url, "https://api.runpod.ai/v2/ep123");
    }

    #[test]
    fn reachable_http_status_accepts_non_5xx() {
        assert!(is_reachable_http_status("200"));
        assert!(is_reachable_http_status("401"));
        assert!(!is_reachable_http_status("500"));
        assert!(!is_reachable_http_status("000"));
    }

    #[test]
    fn extract_endpoint_id_from_list_by_name_works() {
        let value = json!([
            {"id": "ep-1", "name": "alpha"},
            {"id": "ep-2", "name": "runpod-llm"}
        ]);
        let id = extract_endpoint_id_from_list_by_name(&value, "runpod-llm");
        assert_eq!(id, Some("ep-2".to_string()));
    }

    #[test]
    fn extract_endpoint_id_from_list_by_name_supports_name_suffix() {
        let value = json!([
            {"id": "ep-1", "name": "runpod-llm -fb"},
            {"id": "ep-2", "name": "other"}
        ]);
        let id = extract_endpoint_id_from_list_by_name(&value, "runpod-llm");
        assert_eq!(id, Some("ep-1".to_string()));
    }

    #[test]
    fn find_item_by_name_or_id_prefers_id_match() {
        let value = json!([
            {"id": "ep-1", "name": "alpha"},
            {"id": "ep-2", "name": "runpod-llm"}
        ]);
        let item = find_item_by_name_or_id(&value, "runpod-llm", Some("ep-1"), "endpointId")
            .expect("item");
        assert_eq!(item.get("id").and_then(|v| v.as_str()), Some("ep-1"));
    }

    #[test]
    fn extract_workers_formats_current_min_max() {
        let text = extract_workers(&json!({"workersCurrent": 1, "workersMin": 0, "workersMax": 2}))
            .expect("workers");
        assert_eq!(text, "1 (min=0, max=2)");
    }
}
