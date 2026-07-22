# ASI Dockerfile V1080 (主 17:33 干到底 + 主 19:33 Docker 真借鉴 + 主 17:43 实事求是)
FROM python:3.11-slim

LABEL org.opencontainers.image.title="apeireth-asi" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.source="apeireth" \
      org.opencontainers.image.licenses="Apache-2.0"

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1 \
    ASI_VERSION="0.1.0" \
    OTEL_SERVICE_NAME=apeireth-asi \
    OTEL_SERVICE_VERSION=0.1.0

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY apeireth/ ./apeireth/

EXPOSE 8765

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request,sys;\
r=urllib.request.urlopen('http://127.0.0.1:8765/health',timeout=3);\
sys.exit(0 if r.status==200 else 1)"

CMD ["python", "-c", "import apeireth.v1080_asi_real_subprocess_deploy as v; v.app"]
