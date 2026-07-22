# ASI Real Deployment Dockerfile — V1075 (主 23:44 干到底)
# 借鉴 Docker 2013 HEALTHCHECK + 12-Factor 2011 config + OTel 2019 resource
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
COPY deploy/ ./deploy/

EXPOSE 18800

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request,sys;\
r=urllib.request.urlopen('http://127.0.0.1:18800/health',timeout=3);\
sys.exit(0 if r.status==200 else 1)"

CMD ["python", "-m", "apeireth.v1075_asi_real_deployment_run", "--serve", "--host", "0.0.0.0", "--port", "18800"]
