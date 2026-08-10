# apeireth-sdk — 多阶段 Dockerfile (V2 跨战区 P1)
#
# 跨战区工具: 多语言 SDK 统一 (Python / Node.js / Go / Rust binding)
# 提供 apeireth_sdk::Client (Rust) + 各语言 ffi binding 模板
#
# 参照: deploy/Dockerfile (多阶段 builder + runtime)
# 镜像层加固:
#   - 多阶段 builder + runtime
#   - runtime 加 ca/tini/curl + python3 (Python binding smoke test)
#   - 非 root USER
#   - HEALTHCHECK 探活 (axum /health)
#   - 端口 8093 (SDK registry / version service)
#
# 不假装:
#   ✅ 真编译 apeireth-sdk (cargo build --release -p apeireth-sdk)
#   ✅ 复制真 binary

# ===============================================================
# 阶段 1: builder
# ===============================================================
FROM rust:1.80-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p apeireth-sdk

# ===============================================================
# 阶段 2: runtime
# ===============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
        curl \
        python3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 apeireth \
    && useradd  --system --uid 1001 --gid apeireth \
                --home-dir /app --shell /usr/sbin/nologin \
                apeireth \
    && mkdir -p /app/data \
    && chown -R apeireth:apeireth /app
WORKDIR /app

COPY --from=builder --chown=apeireth:apeireth \
    /build/target/release/apeireth-sdk /usr/local/bin/apeireth-sdk

RUN apeireth-sdk --version

USER apeireth:apeireth

# SDK registry / version 端口 (默认 8093)
EXPOSE 8093

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD curl -f http://localhost:${APEIRETH_SDK_PORT:-8093}/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/apeireth-sdk"]
CMD ["serve", "--port", "8093"]