# Claude RunPod Profile Template

This directory is the `runpod` profile template root for RunPod Serverless.

`runpod.yaml` is profile-bound endpoint config used by:

```bash
agent-tools runpod up runpod
agent-tools runpod status runpod
```

The `docker/` directory contains an image template that runs:
- vLLM (OpenAI-compatible API) on an internal port
- Anthropic-compatible proxy (`/v1/messages`) on the public port
- model warmup at startup (downloads once into `/workspace/model-cache`)

Create a RunPod template from that image, then set `template_id` in `runpod.yaml`.

## Build and push image

```bash
cd templates/claude/runpod/docker
docker build -t ghcr.io/your-org/agent-tools-runpod-vllm:latest .
docker push ghcr.io/your-org/agent-tools-runpod-vllm:latest
```

Update your RunPod template to use the published image, then set `runpod.yaml`
`template_id` to that template id.

## Launch (endpoint create + codex endpoint auto-update)

```bash
agent-tools runpod up runpod
```

Flow:
1. `agent-tools use runpod` (profile switch)
2. `runpodctl serverless create ...` from `runpod.yaml`
3. update `~/.codex/config.local.toml` and `~/.codex/config.toml` `base_url`
4. write `~/.claude/runpod.env` and `~/.claude/runpod_expected_anthropic_base_url`
5. verify Claude endpoint by calling `POST /v1/messages`

### Model download behavior

- First run: model is downloaded into `/workspace/model-cache`
- Next runs: cached model is reused
- `workers_min: 1` in `runpod.yaml` keeps one warm worker to avoid repeated cold downloads

After `runpod up`, load Claude env in the shell before starting Claude:

```bash
source ~/.claude/runpod.env
```

`SessionStart` hook warns if `ANTHROPIC_BASE_URL` is not synced.

Do not store secrets in this repository. Set `RUNPOD_API_KEY` and model auth
tokens via environment variables.
