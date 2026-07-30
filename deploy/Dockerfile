# Apeireth canonical runtime image — V1075/R11
# The patch-level base tag and Python dependencies are pinned for repeatable builds.
FROM python:3.13.14-slim-bookworm

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

COPY deploy/requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir -r /tmp/requirements.txt \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin apeireth

COPY --chown=apeireth:apeireth apeireth/ ./apeireth/

USER 10001:10001
EXPOSE 8765

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request,sys; r=urllib.request.urlopen('http://127.0.0.1:8765/health',timeout=3); sys.exit(0 if r.status==200 else 1)"

CMD ["python", "-m", "apeireth.v1075_asi_real_deployment_run", "--serve", "--host", "0.0.0.0", "--port", "8765"]
