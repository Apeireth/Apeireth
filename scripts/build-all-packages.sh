#!/usr/bin/env bash
# Apeireth 8 包齐发总入口 (R20 阶段 3, D-06 拍板)
# 8 包: deb / rpm / brew / scoop / tarball / zip / Windows MSI / Docker
# Linux 4 包重点优化 (deb / rpm / tarball / Docker)
#
# 用法:
#   ./scripts/build-all-packages.sh           # 实装全部 8 包
#   ./scripts/build-all-packages.sh --dry-run # 仅打印命令, 不真跑
#   APEIRETH_VERSION=1.0.0 ./scripts/build-all-packages.sh
#
# 前置:
#   rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
#   cargo install cargo-deb cargo-rpm cargo-wix
#   docker buildx create --use

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${APEIRETH_VERSION:-1.0.0}"
DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

echo "=================================================="
echo "  Apeireth OS v${VERSION} — 8 包齐发"
echo "  $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
echo "  DRY_RUN=${DRY_RUN}"
echo "=================================================="
echo ""

run_or_print() {
    local desc="$1"; shift
    echo ""
    echo ">>> ${desc}"
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "    \$ $*"
    else
        "$@"
    fi
}

# === 1. Docker (multi-arch) — Linux 重点 1/4 ===
run_or_print "[1/8] Docker (multi-arch linux/amd64 + linux/arm64)" \
    docker buildx build \
        --platform linux/amd64,linux/arm64 \
        --tag apeireth/apeireth:${VERSION} \
        --tag apeireth/apeireth:latest \
        --push \
        .

# === 2. deb (apt repo 配) — Linux 重点 2/4 ===
run_or_print "[2/8] deb (Debian/Ubuntu apt install apeireth)" \
    ./packaging/deb/build.sh

# === 3. rpm (dnf repo 配) — Linux 重点 3/4 ===
run_or_print "[3/8] rpm (RHEL/Fedora/CentOS dnf install apeireth)" \
    ./packaging/rpm/build.sh

# === 4. tarball (musl 静态链接, 任何 Linux/Unix 通用) — Linux 重点 4/4 ===
run_or_print "[4/8] tarball (musl static, 任何 Linux/Unix, AUR 基础)" \
    ./packaging/tarball/build.sh

# === 5. brew formula (macOS) ===
run_or_print "[5/8] brew formula (macOS brew install apeireth/tap/apeireth)" \
    ./packaging/brew/build.sh

# === 6. zip (Windows 通用) ===
run_or_print "[6/8] zip (Windows Expand-Archive 解压即用)" \
    powershell -ExecutionPolicy Bypass -File packaging/zip/build.ps1

# === 7. Scoop manifest (Windows 包管理器) ===
run_or_print "[7/8] Scoop manifest (scoop install apeireth)" \
    powershell -ExecutionPolicy Bypass -File packaging/scoop/build.ps1

# === 8. Windows MSI (WiX) ===
run_or_print "[8/8] Windows MSI (msiexec /i apeireth.msi)" \
    powershell -ExecutionPolicy Bypass -File packaging/msi/build.ps1

echo ""
echo "=================================================="
echo "  8 包齐发完成 (v${VERSION})"
echo "  产物路径见 packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/"
echo "=================================================="
