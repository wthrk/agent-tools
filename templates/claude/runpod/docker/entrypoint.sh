#!/usr/bin/env bash
set -euo pipefail

MODEL_ID="${MODEL_ID:-Qwen/Qwen2.5-14B-Instruct}"
MODEL_CACHE_DIR="${MODEL_CACHE_DIR:-/workspace/model-cache}"
SKIP_MODEL_WARMUP="${SKIP_MODEL_WARMUP:-1}"
STATUS_FILE="${STATUS_FILE:-/workspace/runpod-status.json}"
INIT_LOG_FILE="${INIT_LOG_FILE:-/workspace/runpod-init.log}"

VLLM_INTERNAL_PORT="${VLLM_INTERNAL_PORT:-8001}"
PROXY_PORT="${PROXY_PORT:-8000}"
TENSOR_PARALLEL_SIZE="${TENSOR_PARALLEL_SIZE:-1}"
MAX_MODEL_LEN="${MAX_MODEL_LEN:-8192}"
GPU_MEMORY_UTILIZATION="${GPU_MEMORY_UTILIZATION:-0.9}"

mkdir -p "${MODEL_CACHE_DIR}"
mkdir -p "$(dirname "${STATUS_FILE}")"
mkdir -p "$(dirname "${INIT_LOG_FILE}")"

write_status() {
  local phase="$1"
  local ready="$2"
  local message="$3"
  cat >"${STATUS_FILE}" <<EOF
{"phase":"${phase}","ready":${ready},"message":"${message}","model_id":"${MODEL_ID}"}
EOF
}

touch "${INIT_LOG_FILE}"
write_status "booting" "false" "container booting"
export HF_HOME="${MODEL_CACHE_DIR}"
export HUGGINGFACE_HUB_CACHE="${MODEL_CACHE_DIR}"
export TRANSFORMERS_CACHE="${MODEL_CACHE_DIR}"

{
  echo "[init] starting at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "[init] model=${MODEL_ID} cache=${MODEL_CACHE_DIR}"
} >>"${INIT_LOG_FILE}"

start_vllm() {
  if [[ "${SKIP_MODEL_WARMUP}" == "1" ]]; then
    {
      echo "[init] warmup skipped"
    } >>"${INIT_LOG_FILE}"
  else
    write_status "downloading" "false" "warming model cache"
    python - <<'PY' >>"${INIT_LOG_FILE}" 2>&1
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

  write_status "loading" "false" "starting vllm"
  python -m vllm.entrypoints.openai.api_server \
    --host 0.0.0.0 \
    --port "${VLLM_INTERNAL_PORT}" \
    --model "${MODEL_ID}" \
    --download-dir "${MODEL_CACHE_DIR}" \
    --tensor-parallel-size "${TENSOR_PARALLEL_SIZE}" \
    --max-model-len "${MAX_MODEL_LEN}" \
    --gpu-memory-utilization "${GPU_MEMORY_UTILIZATION}" >>"${INIT_LOG_FILE}" 2>&1
}

start_vllm &
VLLM_PID=$!
echo "${VLLM_PID}" > /tmp/vllm.pid

(
  for _ in $(seq 1 600); do
    if curl -fsS "http://127.0.0.1:${VLLM_INTERNAL_PORT}/v1/models" >/dev/null 2>&1; then
      write_status "ready" "true" "vllm is ready"
      echo "[init] ready at $(date -u +%Y-%m-%dT%H:%M:%SZ)" >>"${INIT_LOG_FILE}"
      exit 0
    fi
    if ! kill -0 "${VLLM_PID}" >/dev/null 2>&1; then
      write_status "error" "false" "vllm exited during initialization"
      echo "[init] vllm exited before ready" >>"${INIT_LOG_FILE}"
      exit 1
    fi
    sleep 2
  done
  write_status "error" "false" "vllm readiness timeout"
  echo "[init] readiness timeout" >>"${INIT_LOG_FILE}"
  kill "${VLLM_PID}" >/dev/null 2>&1 || true
  exit 1
) &

wait_for_vllm() {
  wait "${VLLM_PID}" || true
  if [[ -f "${STATUS_FILE}" ]] && grep -q '"ready":true' "${STATUS_FILE}"; then
    write_status "error" "false" "vllm exited after becoming ready"
  elif [[ -f "${STATUS_FILE}" ]] && grep -q '"phase":"error"' "${STATUS_FILE}"; then
    :
  else
    write_status "error" "false" "vllm exited"
  fi
}
wait_for_vllm &

exec python /proxy_server.py
