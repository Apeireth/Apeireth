# apeireth-graph — 多阶段 Dockerfile (V2 战役 3 P0)
#
# 战区 3: Multi-Agent — 图编排 (LangGraph 风格) + checkpoint 持久化
# 提供 Graph / Node trait / executor / checkpoint 持久化 (SQLite)
#
# 参照: deploy/Dockerfile (多阶段 builder + runtime)
# 镜像层加固:
#   - 多阶段 (builder + runtime)
#   - runtime 加 ca/tini/curl + sqlite3 CLI (checkpoint 调试用)
#   - 非 root USER + 持久化卷 /app/data
#   - HEALTHCHECK 探活 (axum /health, 30s)
#   - 端口 8091 (graph serve)
#
# 不假装:
#   ✅ 真编译 apeireth-graph (cargo build --release -p apeireth-graph)
#   ✅ 复制真 binary (target/release/apeireth-graph)
#   ✅ 显式卷声明 /app/data (checkpoint SQLite 持久化)

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

RUN cargo build --release -p apeireth-graph

# ===============================================================
# 阶段 2: runtime
# ===============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
        curl \
        sqlite3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 apeireth \
    && useradd  --system --uid 1001 --gid apeireth \
                --home-dir /app --shell /usr/sbin/nologin \
                apeireth \
    && mkdir -p /app/data \
    && chown -R apeireth:apeireth /app
WORKDIR /app

COPY --from=builder --chown=apeireth:apeireth \
    /build/target/release/apeireth-graph /usr/local/bin/apeireth-graph

RUN apeireth-graph --version

USER apeireth:apeireth

# graph serve HTTP 端口 (默认 8091)
EXPOSE 8091

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=15s \
    CMD curl -f http://localhost:${APEIRETH_GRAPH_PORT:-8091}/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/apeireth-graph"]
CMD ["serve", "--port", "8091", "--data-dir", "/app/data"]

# 持久化卷: checkpoint SQLite 数据库 + 图执行历史
VOLUME ["/app/data"]