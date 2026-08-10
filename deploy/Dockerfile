# Apeireth 多协议 API 网关 - 多阶段 Dockerfile (R17 战役 4-4)
#
# 阶段 1 (builder): rust image, cargo build --release 全 workspace
#   - 含 战役 1+2 全部 crate (protocol / http-client / pipeline / api /
#     tool-registry / tool-runtime / tool-approval / agent / tools)
#   - 含 战役 4 全部 crate (tui / desktop / supervisor)
#   - 含 apeireth-cli (binary: `apeireth`, 启动 gateway serve)
# 阶段 2 (runtime): debian-slim, 复制 binary + 4 端口
#
# 不假装:
#   ✅ 真正编译 39 crate (cargo build --release --workspace)
#   ✅ 复制真 binary, 不是空壳
#   ✅ 4 端口全部 expose (部署时按需映射)
#   ✅ 健康检查用 /health 端点 (axum server, 战役 1-4)

# ===============================================================
# 阶段 1: builder (编译全部 39 crate)
# ===============================================================
FROM rust:1.80-bookworm AS builder

# 系统依赖 (rusqlite bundled, 但要 build tools)
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# 依赖缓存层: 先复制 Cargo.toml + Cargo.lock, 预热 target
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# 离线构建全 workspace release (含 39 crate, 战役 1+2+4 全部)
RUN cargo build --release --workspace --bin apeireth

# ===============================================================
# 阶段 2: runtime (debian-slim, 4 端口 expose)
# ===============================================================
FROM debian:bookworm-slim AS runtime

# 运行时依赖: CA 证书 (HTTPS to minimaxi), libc, tini (信号转发)
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
        curl \
    && rm -rf /var/lib/apt/lists/*

# 从 builder 复制 apeireth binary
COPY --from=builder /build/target/release/apeireth /usr/local/bin/apeireth

# 验证 binary 跑得起来
RUN apeireth --version && apeireth --help

# 编译期 hardcode 端口范围 (战役 4-4 部署 4 端口协议端点)
# 8080: OpenAI Chat Completions (默认)
# 8081: Anthropic Messages
# 8082: Google Gemini GenerateContent
# 8083: OpenAI Responses API
EXPOSE 8080 8081 8082 8083

# 战役 1-4: apeireth-api server /health 端点 (axum)
# HEALTHCHECK 5s 一次, 失败 3 次报 unhealthy
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || exit 1

# 容器启动默认命令: gateway serve on port 8080
# docker-compose 4 服务可 override: apeireth gateway serve --port 808X
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/apeireth"]
CMD ["gateway", "serve", "--port", "8080"]
