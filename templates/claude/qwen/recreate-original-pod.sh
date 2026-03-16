#!/usr/bin/env bash
set -euo pipefail

TEMPLATE_NAME="${TEMPLATE_NAME:-qwen3-coder-next-fp8-262144-b200-seg}"
POD_NAME="${POD_NAME:-qwen-one}"
GPU_ID="${GPU_ID:-NVIDIA B200}"
GPU_COUNT="${GPU_COUNT:-1}"
DATA_CENTER_IDS="${DATA_CENTER_IDS:-EU-RO-1}"
CONTAINER_DISK_GB="${CONTAINER_DISK_GB:-150}"
VOLUME_GB="${VOLUME_GB:-150}"
VOLUME_MOUNT_PATH="${VOLUME_MOUNT_PATH:-/workspace}"
MODEL_NAME="${MODEL_NAME:-Qwen/Qwen3-Coder-Next-FP8}"
MAX_MODEL_LEN="${MAX_MODEL_LEN:-262144}"
GPU_MEMORY_UTILIZATION="${GPU_MEMORY_UTILIZATION:-0.85}"
API_KEY="${API_KEY:-sk-qwen-runpod}"

echo "Creating template: ${TEMPLATE_NAME}"
template_json="$(runpodctl template create \
  --name "${TEMPLATE_NAME}" \
  --image vllm/vllm-openai:latest \
  --container-disk-in-gb "${CONTAINER_DISK_GB}" \
  --volume-in-gb "${VOLUME_GB}" \
  --volume-mount-path "${VOLUME_MOUNT_PATH}" \
  --ports 8000/http \
  --docker-entrypoint 'python3,-m,vllm.entrypoints.openai.api_server' \
  --docker-start-cmd "--model,${MODEL_NAME},--host,0.0.0.0,--port,8000,--max-model-len,${MAX_MODEL_LEN},--gpu-memory-utilization,${GPU_MEMORY_UTILIZATION},--api-key,${API_KEY},--enable-auto-tool-choice,--tool-call-parser,qwen3_coder" \
  --env "{\"HF_HOME\":\"/workspace/.huggingface\",\"PYTORCH_ALLOC_CONF\":\"expandable_segments:True\",\"VLLM_API_KEY\":\"${API_KEY}\"}")"

template_id="$(printf '%s\n' "${template_json}" | jq -r '.id')"
if [[ -z "${template_id}" || "${template_id}" == "null" ]]; then
  echo "Failed to extract template id" >&2
  exit 1
fi

echo "Creating pod: ${POD_NAME}"
runpodctl pod create \
  --template-id "${template_id}" \
  --gpu-id "${GPU_ID}" \
  --gpu-count "${GPU_COUNT}" \
  --data-center-ids "${DATA_CENTER_IDS}" \
  --name "${POD_NAME}" \
  --container-disk-in-gb "${CONTAINER_DISK_GB}" \
  --volume-in-gb "${VOLUME_GB}"
