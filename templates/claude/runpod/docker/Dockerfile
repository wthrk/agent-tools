FROM vllm/vllm-openai:latest

RUN pip install --no-cache-dir fastapi uvicorn httpx

COPY entrypoint.sh /entrypoint.sh
COPY proxy_server.py /proxy_server.py
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
