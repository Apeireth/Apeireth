# ASI Dockerfile 多阶段构建 (主 00:36 效果 + 工程化 + 主 19:33 Docker 官方真借鉴)
# Stage 1: builder
FROM python:3.13-slim AS builder

WORKDIR /build

# System deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc g++ && rm -rf /var/lib/apt/lists/*

# Python deps
COPY requirements.txt .
RUN pip install --no-cache-dir --user -r requirements.txt

# App source
COPY apeireth/ ./apeireth/
COPY tests/ ./tests/

# Stage 2: runtime
FROM python:3.13-slim AS runtime

WORKDIR /app

# Non-root user
RUN useradd --create-home --shell /bin/bash asi

# Copy Python deps from builder
COPY --from=builder --chown=asi:asi /root/.local /home/asi/.local
COPY --from=builder --chown=asi:asi /build/apeireth /app/apeireth
COPY --from=builder --chown=asi:asi /build/tests /app/tests

# Make scripts executable
ENV PATH=/home/asi/.local/bin:$PATH
ENV PYTHONPATH=/app
ENV ASI_NORTH_STAR=0.7905
ENV ASI_LEVEL=production

USER asi

# Health check (V41 真借鉴)
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD python -c "from apeireth.v1031_integration import V1031Integration; r = V1031Integration().run(); exit(0 if r['pass_rate'] == 1.0 else 1)" \
    || exit 1

# Default command: run integration tests
CMD ["python", "-c", "from apeireth.v1031_integration import V1031Integration; V1031Integration().run()"]
