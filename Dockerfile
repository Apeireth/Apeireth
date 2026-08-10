# Apeireth R20 阶段 3 — Multi-stage Dockerfile (1.0 release)
# Per blueprint §3.2 (r20-stage-2-3-prep-2026-08-05.md):
# - 3 阶段: builder (~2GB) + runtime-deps (~200MB) + final (~150MB distroless)
# - 依赖层缓存 (Cargo.toml 先 copy → dummy build → 真正 build)
# - multi-arch: docker buildx --platform linux/amd64,linux/arm64
# - 8 包之 1 (D-06 拍板)
#
# 注意 (per 主人 user memory: TUI 是"集成测试床", 瘦客户端, 不进生产镜像):
# - 本镜像只含 API server binary `apeireth` (apeireth-api crate)
# - TUI (`apeireth-tui`) 由 dev 在本机跑, 不进 Docker image

# === Stage 1: builder ===
FROM rust:1.80-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libsqlite3-dev libgit2-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build

# 1. 依赖层缓存 (per O-5 编译期守门)
COPY Cargo.toml Cargo.lock ./
COPY crates/apeireth-*/Cargo.toml ./crates/

# 2. dummy build 触发依赖编译 (写空 main.rs, 编译只为了锁住依赖层)
#    注: `apeireth` binary 来源于 apeireth-cli crate (per crates/apeireth-cli/Cargo.toml [[bin]] name="apeireth")
RUN mkdir -p crates/apeireth-cli/src \
    && echo "fn main(){}" > crates/apeireth-cli/src/main.rs \
    && cargo build --release --workspace --bin apeireth \
    && rm -rf crates/apeireth-cli/src

# 3. 真正 copy 源码
COPY crates/ ./crates/

# 4. 真正 build (workspace release, 编译期 hardcode 守 V1141/V1131/V1136 baseline)
RUN cargo build --release --workspace --bin apeireth \
    && strip target/release/apeireth

# === Stage 2: runtime-deps (运行时动态库) ===
FROM debian:bookworm-slim AS runtime-deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libsqlite3-0 libgit2-1.7 \
    && rm -rf /var/lib/apt/lists/*

# === Stage 3: final (distroless, ~150MB, 非 root 用户) ===
FROM gcr.io/distroless/cc-debian12:nonroot AS final
COPY --from=runtime-deps /usr/lib/x86_64-linux-gnu/ /usr/lib/x86_64-linux-gnu/
COPY --from=runtime-deps /lib/x86_64-linux-gnu/ /lib/x86_64-linux-gnu/
COPY --from=builder /build/target/release/apeireth /usr/local/bin/apeireth

# 默认数据/配置/日志目录 (volumes 挂载)
ENV APEIRETH_HOME=/var/lib/apeireth \
    APEIRETH_CONFIG=/etc/apeireth/config.toml \
    APEIRETH_LOG_DIR=/var/log/apeireth \
    APEIRETH_AUDIT_LOG=/var/log/apeireth/audit.log \
    RUST_LOG=info,apeireth_api=debug \
    APEIRETH_METRICS_PORT=9090

USER nonroot:nonroot
EXPOSE 8080 9090

# 健康检查 (调 --health-check 标志, 由 apeireth binary 自身处理)
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/apeireth", "--health-check"]

ENTRYPOINT ["/usr/local/bin/apeireth"]

# multi-arch build (linux/amd64 + linux/arm64):
#   docker buildx build --platform linux/amd64,linux/arm64 \
#     --tag apeireth/apeireth:1.0.0 --tag apeireth/apeireth:latest --push .
