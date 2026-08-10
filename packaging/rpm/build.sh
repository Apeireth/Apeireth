#!/usr/bin/env bash
# Apeireth rpm 包构建 (8 包之 1)
# 平台: RHEL / Fedora / CentOS
# 工具: cargo-rpm
# 体积: ~50MB

set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION="${APEIRETH_VERSION:-1.0.0}"

echo "=== apeireth rpm build v${VERSION} ==="

# 1. 检查 cargo-rpm 工具链
if ! command -v cargo-rpm >/dev/null 2>&1; then
    echo "[1/4] installing cargo-rpm..."
    cargo install cargo-rpm --locked
fi

# 2. build stage
echo "[2/4] cargo build --release --bin apeireth..."
cargo build --release --bin apeireth --locked
strip target/release/apeireth

# 3. rpm build
echo "[3/4] cargo rpm build..."
cargo rpm build

# 4. 验证产物
RPM_PATH="target/rpm/apeireth-${VERSION}-1.$(uname -m).rpm"
if [[ -f "${RPM_PATH}" ]]; then
    SIZE=$(du -sh "${RPM_PATH}" | cut -f1)
    echo "[4/4] rpm 产物: ${RPM_PATH} (${SIZE})"
    echo "    安装: sudo dnf install ./${RPM_PATH}"
else
    # 尝试在 target/rpm/RPMS/ 下找
    RPM_PATH=$(find target/rpm -name "apeireth-*.rpm" -type f 2>/dev/null | head -1)
    if [[ -n "${RPM_PATH}" && -f "${RPM_PATH}" ]]; then
        SIZE=$(du -sh "${RPM_PATH}" | cut -f1)
        echo "[4/4] rpm 产物: ${RPM_PATH} (${SIZE})"
        echo "    安装: sudo dnf install ./${RPM_PATH}"
    else
        echo "[4/4] WARN: rpm 产物未找到"
        exit 1
    fi
fi
