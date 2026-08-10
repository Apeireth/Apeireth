# apeireth-formal — 多阶段 Dockerfile (V2 战役 5 P2)
#
# 战区 5: Tool Protocol — 形式化验证 (Kani + 模型检查)
# 提供 invariant_check() + reachability analysis + 配合 Sovereignty 验证
#
# 参照: deploy/Dockerfile (多阶段 builder + runtime)
# 镜像层加固:
#   - 多阶段 builder + runtime
#   - runtime 加 ca/tini/curl + 验证用 python3 (解析 CBMC 输出)
#   - 非 root USER + 验证产物 /app/artifacts
#   - HEALTHCHECK 探活 (axum /health)
#   - 端口 8094 (formal verify daemon)
#
# 不假装:
#   ✅ 真编译 apeireth-formal (cargo build --release -p apeireth-formal)
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

RUN cargo build --release -p apeireth-formal

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
    && mkdir -p /app/artifacts \
    && chown -R apeireth:apeireth /app
WORKDIR /app

COPY --from=builder --chown=apeireth:apeireth \
    /build/target/release/apeireth-formal /usr/local/bin/apeireth-formal

RUN apeireth-formal --version

USER apeireth:apeireth

# formal verify daemon 端口 (默认 8094)
EXPOSE 8094

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=15s \
    CMD curl -f http://localhost:${APEIRETH_FORMAL_PORT:-8094}/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/apeireth-formal"]
CMD ["serve", "--port", "8094", "--artifacts-dir", "/app/artifacts"]

# 持久化卷: 验证产物 (counter-example traces + proof certificates)
VOLUME ["/app/artifacts"]