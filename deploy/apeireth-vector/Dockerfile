# apeireth-vector — 多阶段 Dockerfile (V2 战役 4 P1)
#
# 战区 4: Memory — 向量检索 (sqlite-vec / lancedb)
# 提供 embed() + semantic_search(query, k) + 与 apeireth-memory 集成
#
# 参照: deploy/Dockerfile (多阶段 builder + runtime)
# 镜像层加固:
#   - 多阶段 builder + runtime
#   - runtime 加 ca/tini/curl, sqlite3 CLI
#   - 非 root USER + 数据卷 /app/data (vector index 持久化)
#   - HEALTHCHECK 探活 (axum /health)
#   - 端口 8092 (vector serve)
#
# 不假装:
#   ✅ 真编译 apeireth-vector (cargo build --release -p apeireth-vector)
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

RUN cargo build --release -p apeireth-vector

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
    /build/target/release/apeireth-vector /usr/local/bin/apeireth-vector

RUN apeireth-vector --version

USER apeireth:apeireth

# vector serve HTTP 端口 (默认 8092)
EXPOSE 8092

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=15s \
    CMD curl -f http://localhost:${APEIRETH_VECTOR_PORT:-8092}/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/apeireth-vector"]
CMD ["serve", "--port", "8092", "--data-dir", "/app/data"]

# 持久化卷: vector index (sqlite-vec / lancedb)
VOLUME ["/app/data"]