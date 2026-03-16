# Original Qwen Pod

This file records the last known working RunPod pod configuration for Qwen CLI.

## Working shape

- GPU: `NVIDIA B200 x1`
- Region: `EU-RO-1`
- Image: `vllm/vllm-openai:latest`
- Model: `Qwen/Qwen3-Coder-Next-FP8`
- `--max-model-len 262144`
- `--gpu-memory-utilization 0.85`
- `--enable-auto-tool-choice`
- `--tool-call-parser qwen3_coder`
- `HF_HOME=/workspace/.huggingface`
- `PYTORCH_ALLOC_CONF=expandable_segments:True`
- `VLLM_API_KEY=sk-qwen-runpod`
- Container disk: `150`
- Volume: `150`
- Port: `8000/http`

## Recreate

Use [recreate-original-pod.sh](./recreate-original-pod.sh):

```bash
bash templates/claude/qwen/recreate-original-pod.sh
```

## Notes

- This is the pod-based path that previously reached a successful `qwen` check.
- This is not the serverless setup.
- The current `runpod.yaml` remains a separate serverless experiment and should not be treated as the known-good configuration.
