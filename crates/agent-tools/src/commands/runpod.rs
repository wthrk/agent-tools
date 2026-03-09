use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use crate::commands::profile;
use crate::paths;

#[derive(Debug, Deserialize)]
struct RunpodConfig {
    name: String,
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

fn env_to_json(env: &BTreeMap<String, String>) -> Result<String> {
    let mut map = Map::new();
    for (k, v) in env {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    serde_json::to_string(&Value::Object(map)).context("Failed to serialize env map")
}

fn run_runpodctl(args: &[String]) -> Result<Value> {
    let output = Command::new("runpodctl")
        .args(args)
        .output()
        .context("Failed to run runpodctl pod create")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("runpodctl pod create failed:\n{}", stderr);
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

#[cfg(test)]
mod tests {
    use super::{RunpodConfig, build_create_args, extract_pod_id, is_already_running_message};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn build_create_args_includes_required_and_optional() {
        let mut env = BTreeMap::new();
        env.insert("A".to_string(), "1".to_string());
        let cfg = RunpodConfig {
            name: "runpod-llm".to_string(),
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
    fn already_running_message_detected_case_insensitive() {
        assert!(is_already_running_message("Pod is already running"));
        assert!(is_already_running_message("ALREADY RUNNING"));
        assert!(!is_already_running_message("unknown error"));
    }
}
