# Codex RunPod Profile Template

This directory is the `runpod` Codex profile template root.

`config.toml` configures Codex to use a RunPod Serverless OpenAI-compatible
endpoint.

## Activate profile

```bash
agent-tools use runpod
```

## Endpoint base_url update

`agent-tools runpod up runpod` updates:
- `~/.codex/config.local.toml`
- `~/.codex/config.toml`

with the created endpoint id.

Export API key:

```bash
export RUNPOD_API_KEY=...
```

When `manage_codex_config: true`, `agent-tools sync` merges:
- template base: `templates/codex/runpod/config.toml`
- local overlay: `~/.codex/config.local.toml`
