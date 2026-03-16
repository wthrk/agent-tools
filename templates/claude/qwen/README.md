# Claude Qwen Profile

This directory contains two separate Qwen paths.

- `runpod.yaml`: serverless experiment for `Qwen/Qwen3-Coder-Next-FP8`
- [original-pod.md](./original-pod.md): last known working pod configuration

The serverless profile is tuned for the "half of 262144" target:

- `MAX_MODEL_LEN=131072`
- `NVIDIA H100 NVL`
- OpenAI-compatible endpoint for Qwen CLI

Set `template_id` in [runpod.yaml](./runpod.yaml) to your RunPod Serverless template id, then run:

```bash
agent-tools runpod up qwen
```

Notes:

- This profile is intended for Qwen CLI, not Claude compatibility.
- If `131072` is unstable on your template/GPU combination, move back to `NVIDIA B200` or reduce `MAX_MODEL_LEN`.
- Point `~/.qwen/settings.json` at the created endpoint's `/v1` URL.
- For the previously working `B200 + 262144` pod flow, use [recreate-original-pod.sh](./recreate-original-pod.sh).
