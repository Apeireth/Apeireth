#!/usr/bin/env bash
# Apeireth Docker 包 (8 包之 1, D-06 拍板, Linux 重点优化)
# 主构建在仓库根 Dockerfile + docker-compose.yml, 由 release-1.0.0.yml 的
# `docker buildx build --platform ...` 直接驱动. 本脚本仅做 best-effort
# 本地构建验证, CI 失败不阻塞 (per 1.0 release engineer §D-06).

set -uo pipefail  # 0 -e: best-effort, musl packaging 待 1.0 release engineer 实装
cd "$(dirname "$0")/../.."

VERSION="${APEIRETH_VERSION:-1.0.0}"

echo "=== apeireth docker build v${VERSION} (此脚本为 placeholder) ==="

# 1. 校验 Dockerfile 存在
if [[ -f "Dockerfile" ]]; then
    echo "[1/3] Dockerfile 存在 ($(wc -l < Dockerfile) lines)"
else
    echo "[1/3] WARN: 根目录 Dockerfile 不存在, 跳过"
    exit 0
fi

# 2. 校验 docker / buildx 可用
if command -v docker >/dev/null 2>&1; then
    if docker buildx version >/dev/null 2>&1; then
        echo "[2/3] docker buildx 可用 ($(docker buildx version | head -1))"
    else
        echo "[2/3] WARN: docker buildx 不可用, 跳过 (CI runner 已配置 buildx)"
    fi
else
    echo "[2/3] WARN: docker 命令不可用, 跳过 (release pipeline 跑 buildx)"
fi

# 3. 报告
echo "[3/3] 实际 docker build 由 release-1.0.0.yml 'docker buildx build' step 驱动"
echo "    本地验证: docker buildx build --platform linux/amd64,linux/arm64 \\"
echo "                                -t apeireth/apeireth:${VERSION} --load ."

exit 0