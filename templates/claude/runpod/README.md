# Claude RunPod Profile Template

This directory is the `runpod` profile template root.

`runpod.yaml` is profile-bound pod config used by:

```bash
agent-tools runpod up runpod
```

The `docker/` directory contains an image template that downloads/serves the
model at container startup via vLLM.

## Build and push image

```bash
cd templates/claude/runpod/docker
docker build -t ghcr.io/your-org/agent-tools-runpod-vllm:latest .
docker push ghcr.io/your-org/agent-tools-runpod-vllm:latest
```

Update `runpod.yaml` `image` to the published image.

## Launch

```bash
agent-tools runpod up runpod
```

Flow:
1. `agent-tools use runpod` (profile switch)
2. `runpodctl pod create ...` from `runpod.yaml`
3. `runpodctl pod start <pod-id>`

Do not store secrets in this repository. Set `RUNPOD_API_KEY` and model auth
tokens via environment variables.
