#!/usr/bin/env bash
set -euo pipefail

MODEL_ID="${MODEL_ID:-Qwen/Qwen2.5-14B-Instruct}"
MODEL_CACHE_DIR="${MODEL_CACHE_DIR:-/workspace/model-cache}"
SKIP_MODEL_WARMUP="${SKIP_MODEL_WARMUP:-0}"

VLLM_INTERNAL_PORT="${VLLM_INTERNAL_PORT:-8001}"
PROXY_PORT="${PROXY_PORT:-8000}"
TENSOR_PARALLEL_SIZE="${TENSOR_PARALLEL_SIZE:-1}"
MAX_MODEL_LEN="${MAX_MODEL_LEN:-8192}"
GPU_MEMORY_UTILIZATION="${GPU_MEMORY_UTILIZATION:-0.9}"

# Persist Hugging Face cache under workspace so cold starts can reuse it
mkdir -p "${MODEL_CACHE_DIR}"
export HF_HOME="${MODEL_CACHE_DIR}"
export HUGGINGFACE_HUB_CACHE="${MODEL_CACHE_DIR}"
export TRANSFORMERS_CACHE="${MODEL_CACHE_DIR}"

# First-run warmup: download model into cache before serving
if [[ "${SKIP_MODEL_WARMUP}" != "1" ]]; then
  python - <<'PY'
import os
from huggingface_hub import snapshot_download

model_id = os.environ["MODEL_ID"]
cache_dir = os.environ["MODEL_CACHE_DIR"]
marker = os.path.join(cache_dir, ".model_ready_" + model_id.replace("/", "__"))
if not os.path.exists(marker):
    snapshot_download(repo_id=model_id, cache_dir=cache_dir)
    with open(marker, "w", encoding="utf-8") as f:
        f.write("ok\n")
PY
fi

python -m vllm.entrypoints.openai.api_server \
  --host 0.0.0.0 \
  --port "${VLLM_INTERNAL_PORT}" \
  --model "${MODEL_ID}" \
  --download-dir "${MODEL_CACHE_DIR}" \
  --tensor-parallel-size "${TENSOR_PARALLEL_SIZE}" \
  --max-model-len "${MAX_MODEL_LEN}" \
  --gpu-memory-utilization "${GPU_MEMORY_UTILIZATION}" &

for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:${VLLM_INTERNAL_PORT}/v1/models" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

exec uvicorn proxy_server:app --host 0.0.0.0 --port "${PROXY_PORT}"
