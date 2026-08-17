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
#    台账 #46 修复: glob `COPY crates/apeireth-*/Cargo.toml ./crates/` 会把 82 个成员
#    的 Cargo.toml 全部平铺进 ./crates/ 同名互覆盖且不建 member 子目录 → dummy build 必失败.
#    改为逐成员显式 COPY (建 member 子目录; 清单与根 Cargo.toml [workspace] members 逐条对齐).
COPY crates/apeireth-acp/Cargo.toml ./crates/apeireth-acp/Cargo.toml
COPY crates/apeireth-action/Cargo.toml ./crates/apeireth-action/Cargo.toml
COPY crates/apeireth-agent/Cargo.toml ./crates/apeireth-agent/Cargo.toml
COPY crates/apeireth-api/Cargo.toml ./crates/apeireth-api/Cargo.toml
COPY crates/apeireth-arbitration/Cargo.toml ./crates/apeireth-arbitration/Cargo.toml
COPY crates/apeireth-asi/Cargo.toml ./crates/apeireth-asi/Cargo.toml
COPY crates/apeireth-bench/Cargo.toml ./crates/apeireth-bench/Cargo.toml
COPY crates/apeireth-blueprint-impl/Cargo.toml ./crates/apeireth-blueprint-impl/Cargo.toml
COPY crates/apeireth-bus/Cargo.toml ./crates/apeireth-bus/Cargo.toml
COPY crates/apeireth-central/Cargo.toml ./crates/apeireth-central/Cargo.toml
COPY crates/apeireth-cli/Cargo.toml ./crates/apeireth-cli/Cargo.toml
COPY crates/apeireth-cognition/Cargo.toml ./crates/apeireth-cognition/Cargo.toml
COPY crates/apeireth-companion/Cargo.toml ./crates/apeireth-companion/Cargo.toml
COPY crates/apeireth-config/Cargo.toml ./crates/apeireth-config/Cargo.toml
COPY crates/apeireth-consciousness/Cargo.toml ./crates/apeireth-consciousness/Cargo.toml
COPY crates/apeireth-constraint/Cargo.toml ./crates/apeireth-constraint/Cargo.toml
COPY crates/apeireth-context-fold/Cargo.toml ./crates/apeireth-context-fold/Cargo.toml
COPY crates/apeireth-core/Cargo.toml ./crates/apeireth-core/Cargo.toml
COPY crates/apeireth-council/Cargo.toml ./crates/apeireth-council/Cargo.toml
COPY crates/apeireth-credentials/Cargo.toml ./crates/apeireth-credentials/Cargo.toml  # TP33+#46: 补 manifest COPY (0bc9a8c5 后新增成员, 缺则 cargo --workspace 解析失败)
COPY crates/apeireth-cron/Cargo.toml ./crates/apeireth-cron/Cargo.toml
COPY crates/apeireth-environment/Cargo.toml ./crates/apeireth-environment/Cargo.toml
COPY crates/apeireth-eval/Cargo.toml ./crates/apeireth-eval/Cargo.toml
COPY crates/apeireth-evolution/Cargo.toml ./crates/apeireth-evolution/Cargo.toml
COPY crates/apeireth-experience/Cargo.toml ./crates/apeireth-experience/Cargo.toml
COPY crates/apeireth-extension/Cargo.toml ./crates/apeireth-extension/Cargo.toml
COPY crates/apeireth-gateway/Cargo.toml ./crates/apeireth-gateway/Cargo.toml
COPY crates/apeireth-graph/Cargo.toml ./crates/apeireth-graph/Cargo.toml
COPY crates/apeireth-graph-primitive/Cargo.toml ./crates/apeireth-graph-primitive/Cargo.toml
COPY crates/apeireth-guard/Cargo.toml ./crates/apeireth-guard/Cargo.toml
COPY crates/apeireth-host/Cargo.toml ./crates/apeireth-host/Cargo.toml
COPY crates/apeireth-http-client/Cargo.toml ./crates/apeireth-http-client/Cargo.toml
COPY crates/apeireth-i18n/Cargo.toml ./crates/apeireth-i18n/Cargo.toml
COPY crates/apeireth-integration-e2e/Cargo.toml ./crates/apeireth-integration-e2e/Cargo.toml
COPY crates/apeireth-lark/Cargo.toml ./crates/apeireth-lark/Cargo.toml
COPY crates/apeireth-library-governance/Cargo.toml ./crates/apeireth-library-governance/Cargo.toml
COPY crates/apeireth-life-force/Cargo.toml ./crates/apeireth-life-force/Cargo.toml
COPY crates/apeireth-livekit/Cargo.toml ./crates/apeireth-livekit/Cargo.toml
COPY crates/apeireth-llm-iface/Cargo.toml ./crates/apeireth-llm-iface/Cargo.toml
COPY crates/apeireth-mcp/Cargo.toml ./crates/apeireth-mcp/Cargo.toml
COPY crates/apeireth-memory/Cargo.toml ./crates/apeireth-memory/Cargo.toml
COPY crates/apeireth-memory/extensions/Cargo.toml ./crates/apeireth-memory/extensions/Cargo.toml
COPY crates/apeireth-motivation/Cargo.toml ./crates/apeireth-motivation/Cargo.toml
COPY crates/apeireth-naming-v05/Cargo.toml ./crates/apeireth-naming-v05/Cargo.toml
COPY crates/apeireth-onion/Cargo.toml ./crates/apeireth-onion/Cargo.toml
COPY crates/apeireth-perception/Cargo.toml ./crates/apeireth-perception/Cargo.toml
COPY crates/apeireth-pipeline/Cargo.toml ./crates/apeireth-pipeline/Cargo.toml
COPY crates/apeireth-pipeline-g5/Cargo.toml ./crates/apeireth-pipeline-g5/Cargo.toml
COPY crates/apeireth-protocol/Cargo.toml ./crates/apeireth-protocol/Cargo.toml
COPY crates/apeireth-provider/Cargo.toml ./crates/apeireth-provider/Cargo.toml
COPY crates/apeireth-pybridge/Cargo.toml ./crates/apeireth-pybridge/Cargo.toml
COPY crates/apeireth-rate-limiter/Cargo.toml ./crates/apeireth-rate-limiter/Cargo.toml
COPY crates/apeireth-repo-tools/Cargo.toml ./crates/apeireth-repo-tools/Cargo.toml
COPY crates/apeireth-runtime/Cargo.toml ./crates/apeireth-runtime/Cargo.toml
COPY crates/apeireth-sdk/Cargo.toml ./crates/apeireth-sdk/Cargo.toml
COPY crates/apeireth-skills/Cargo.toml ./crates/apeireth-skills/Cargo.toml
COPY crates/apeireth-sovereignty/Cargo.toml ./crates/apeireth-sovereignty/Cargo.toml
COPY crates/apeireth-state/Cargo.toml ./crates/apeireth-state/Cargo.toml
COPY crates/apeireth-stock/Cargo.toml ./crates/apeireth-stock/Cargo.toml  # TP27: 补 manifest COPY (新增 N3 金融源套件)
COPY crates/apeireth-supervisor/Cargo.toml ./crates/apeireth-supervisor/Cargo.toml
COPY crates/apeireth-team-lead/Cargo.toml ./crates/apeireth-team-lead/Cargo.toml
COPY crates/apeireth-telemetry/Cargo.toml ./crates/apeireth-telemetry/Cargo.toml
COPY crates/apeireth-test/Cargo.toml ./crates/apeireth-test/Cargo.toml
COPY crates/apeireth-tool-approval/Cargo.toml ./crates/apeireth-tool-approval/Cargo.toml
COPY crates/apeireth-tool-browser/Cargo.toml ./crates/apeireth-tool-browser/Cargo.toml
COPY crates/apeireth-tool-codesearch/Cargo.toml ./crates/apeireth-tool-codesearch/Cargo.toml
COPY crates/apeireth-tool-fetch/Cargo.toml ./crates/apeireth-tool-fetch/Cargo.toml
COPY crates/apeireth-tool-filesystem/Cargo.toml ./crates/apeireth-tool-filesystem/Cargo.toml
COPY crates/apeireth-tool-image-gen/Cargo.toml ./crates/apeireth-tool-image-gen/Cargo.toml
COPY crates/apeireth-tool-image-process/Cargo.toml ./crates/apeireth-tool-image-process/Cargo.toml
COPY crates/apeireth-tool-registry/Cargo.toml ./crates/apeireth-tool-registry/Cargo.toml
COPY crates/apeireth-tool-runtime/Cargo.toml ./crates/apeireth-tool-runtime/Cargo.toml
COPY crates/apeireth-tool-search/Cargo.toml ./crates/apeireth-tool-search/Cargo.toml
COPY crates/apeireth-tool-shell/Cargo.toml ./crates/apeireth-tool-shell/Cargo.toml
COPY crates/apeireth-tools/Cargo.toml ./crates/apeireth-tools/Cargo.toml
COPY crates/apeireth-tui/Cargo.toml ./crates/apeireth-tui/Cargo.toml
COPY crates/apeireth-tui-e2e/Cargo.toml ./crates/apeireth-tui-e2e/Cargo.toml
COPY crates/apeireth-upgrade/Cargo.toml ./crates/apeireth-upgrade/Cargo.toml
COPY crates/apeireth-value/Cargo.toml ./crates/apeireth-value/Cargo.toml
COPY crates/apeireth-vector/Cargo.toml ./crates/apeireth-vector/Cargo.toml
COPY crates/apeireth-verify/Cargo.toml ./crates/apeireth-verify/Cargo.toml
COPY crates/apeireth-voice/Cargo.toml ./crates/apeireth-voice/Cargo.toml
COPY crates/apeireth-web/Cargo.toml ./crates/apeireth-web/Cargo.toml
COPY crates/apeireth-wiki/Cargo.toml ./crates/apeireth-wiki/Cargo.toml  # TP33+#46: 补 manifest COPY (TP28 后新增成员, 缺则 cargo --workspace 解析失败)
COPY crates/apeireth-workflow/Cargo.toml ./crates/apeireth-workflow/Cargo.toml

# 2. dummy build 触发依赖编译 (占位 src, 编译只为了锁住依赖层)
#    注: `apeireth` binary 来源于 apeireth-cli crate (per crates/apeireth-cli/Cargo.toml [[bin]] name="apeireth")
#    台账 #46 修复: workspace build 要求全部 member 目标可解析 → 逐 member 补占位
#    src (lib.rs + main.rs); 真正源码第 3 步 COPY 覆盖同名占位, 结尾 rm -rf 清残.
RUN set -eu; \
    for d in crates/*/; do \
        mkdir -p "$d/src"; \
        if [ ! -e "$d/src/lib.rs" ]; then echo '' > "$d/src/lib.rs"; fi; \
        if [ ! -e "$d/src/main.rs" ]; then echo 'fn main(){}' > "$d/src/main.rs"; fi; \
    done; \
    cargo build --release --workspace --bin apeireth; \
    rm -rf crates

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

# R128+ LLM env: minimax (MiniMax) endpoint
#   APEIRETH_API_KEY    = minimax coding plan key
#   APEIRETH_BASE_URL   = https://api.minimaxi.com/v1 (default)
#   APEIRETH_MODEL      = MiniMax-M3 (default)
ENV APEIRETH_API_KEY=""
ENV APEIRETH_BASE_URL="https://api.minimaxi.com/v1"
ENV APEIRETH_MODEL="MiniMax-M3"


ENTRYPOINT ["/usr/local/bin/apeireth"]

# multi-arch build (linux/amd64 + linux/arm64):
#   docker buildx build --platform linux/amd64,linux/arm64 \
#     --tag apeireth/apeireth:1.0.0 --tag apeireth/apeireth:latest --push .
