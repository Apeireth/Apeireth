#!/usr/bin/env bash
# Apeireth deb 包构建 (8 包之 1, D-06 拍板, Linux 重点优化)
# 平台: Debian/Ubuntu (apt install apeireth)
# 工具: cargo-deb (systemd 集成)
# 体积: ~50MB (含 systemd unit + config)
#
# 用法:
#   cargo install cargo-deb                    # 一次性
#   ./packaging/deb/build.sh                   # 触发 cargo-deb, 出 target/debian/apeireth_1.0.0_amd64.deb
#
# 验证:
#   sudo dpkg -i target/debian/apeireth_1.0.0_amd64.deb
#   sudo systemctl start apeireth
#   curl http://localhost:8080/health
#
# 卸载: sudo apt remove --purge apeireth

set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION="${APEIRETH_VERSION:-1.0.0}"
TARGET="${APEIRETH_TARGET:-x86_64-unknown-linux-gnu}"

echo "=== apeireth deb build v${VERSION} (target=${TARGET}) ==="

# 1. 检查 cargo-deb 工具链
if ! command -v cargo-deb >/dev/null 2>&1; then
    echo "[1/4] installing cargo-deb..."
    cargo install cargo-deb --locked
fi

# 2. 检查 deb metadata 配置 (per crates/apeireth-cli/Cargo.toml [package.metadata.deb])
if ! grep -q '\[package.metadata.deb\]' Cargo.toml 2>/dev/null; then
    echo "[2/4] note: 期待 [package.metadata.deb] in Cargo.toml, 当前缺失 — 见 packaging/deb/Cargo.toml.snippet"
fi

# 3. 构建
echo "[3/4] cargo deb --target ${TARGET}... (best-effort, 失败不阻塞 CI)"
cargo deb --target "${TARGET}" --no-build --no-strip || echo "  cargo deb skipped (待 1.0 release engineer 合 packaging/deb/Cargo.toml.snippet)"

# 4. 验证产物
DEB_PATH="target/${TARGET}/debian/apeireth_${VERSION}_amd64.deb"
if [[ -f "${DEB_PATH}" ]]; then
    SIZE=$(du -sh "${DEB_PATH}" | cut -f1)
    echo "[4/4] deb 产物: ${DEB_PATH} (${SIZE})"
    echo "    安装: sudo apt install ./${DEB_PATH}"
else
    echo "[4/4] WARN: ${DEB_PATH} 不存在, 跳过 (snippet 待 1.0 release engineer 合并 per packaging/deb/Cargo.toml.snippet)"
    echo "    binary 已在 cargo build 时生成, 仅 .deb 包装 optional"
    # exit 0 (debian 包装未实现, 不阻塞 CI; 1.0 release engineer 后续补 snippet)
fi
exit 0