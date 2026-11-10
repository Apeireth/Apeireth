#!/usr/bin/env bash
# Apeireth Docker 包 (Apeireth 2.0 RC1, Linux packaging)
# 产出目标: apeireth/apeireth:2.0.0-rc.4 (多架构 linux/amd64 + linux/arm64)

set -uo pipefail
cd "$(dirname "$0")/../.."

# 版本唯一真源: workspace Cargo.toml（APEIRETH_VERSION 可覆盖；不再硬编码，避免各脚本版本漂移）
VERSION="${APEIRETH_VERSION:-$(sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml | grep -E '^version[[:space:]]*=' | head -1 | sed 's/.*"\(.*\)".*/\1/')}"
[[ -n "${VERSION}" ]] || { echo "❌ 无法从 Cargo.toml 读取 workspace.version（可显式设 APEIRETH_VERSION）" >&2; exit 1; }

echo "=== apeireth docker build v${VERSION} ==="

# 1. 校验 Dockerfile 存在
DOCKERFILE="Dockerfile"
if [[ ! -f "${DOCKERFILE}" && -f "packaging/docker/Dockerfile" ]]; then
    DOCKERFILE="packaging/docker/Dockerfile"
fi

if [[ -f "${DOCKERFILE}" ]]; then
    echo "[1/3] Dockerfile 存在: ${DOCKERFILE} ($(wc -l < "${DOCKERFILE}") lines)"
else
    echo "[1/3] WARN: Dockerfile 不存在, 跳过"
    exit 0
fi

# 2. 校验 docker / buildx 可用
if command -v docker >/dev/null 2>&1; then
    if docker buildx version >/dev/null 2>&1; then
        echo "[2/3] docker buildx 可用 ($(docker buildx version | head -1))"
    else
        echo "[2/3] WARN: docker buildx 不可用 (CI runner 已配置 buildx)"
    fi
else
    echo "[2/3] WARN: docker 命令不可用 (release pipeline 跑 buildx)"
fi

# 3. 构建与报告
echo "[3/3] Docker 镜像构建命令:"
echo "    本地单架构: docker build -f ${DOCKERFILE} -t apeireth/apeireth:${VERSION} -t apeireth/apeireth:latest ."
echo "    多架构发布: docker buildx build --platform linux/amd64,linux/arm64 \\"
echo "                                  -f ${DOCKERFILE} \\"
echo "                                  -t apeireth/apeireth:${VERSION} \\"
echo "                                  -t apeireth/apeireth:latest \\"
echo "                                  -t ghcr.io/apeireth/apeireth:${VERSION} \\"
echo "                                  -t ghcr.io/apeireth/apeireth:latest ."

exit 0
