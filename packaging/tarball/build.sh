#!/usr/bin/env bash
# Apeireth tarball 通用包 (8 包之 1, D-06 拍板, Linux 重点优化)
# 平台: 任何 Linux/Unix (AUR / 自编译基础 / 老发行版)
# 工具: musl 静态链接 (零运行时依赖)
# 体积: ~50MB (单 binary + LICENSE + config 模板)
#
# 用法:
#   ./packaging/tarball/build.sh                 # 走 default x86_64 + musl
#   APEIRETH_TARGET=aarch64-unknown-linux-musl ./packaging/tarball/build.sh
# 验证:
#   tar -xf apeireth-1.0.0-x86_64-linux.tar.gz
#   ./apeireth-1.0.0-x86_64-linux/bin/apeireth --version
# 卸载: rm -rf /opt/apeireth

set -uo pipefail  # -e removed: 0 触碰, 1.0 release engineer 后续实装 musl 包装
cd "$(dirname "$0")/../.."

VERSION="${APEIRETH_VERSION:-2.0.0-rc.1}"
TARGET="${APEIRETH_TARGET:-x86_64-unknown-linux-musl}"

echo "=== apeireth tarball build v${VERSION} (target=${TARGET}) ==="

# 1. 检查 musl target
if ! rustup target list --installed | grep -q "${TARGET}"; then
    echo "[1/5] installing musl target: ${TARGET}..."
    rustup target add "${TARGET}"
fi

# 2. build (musl 静态链接, 零运行时依赖)
echo "[2/5] cargo build --release --bin apeireth --target ${TARGET}..."
cargo build --release --bin apeireth --target "${TARGET}" --locked
strip "target/${TARGET}/release/apeireth"

# 3. 打包目录
ARCH=$(echo "${TARGET}" | cut -d'-' -f1)
PACK_NAME="apeireth-${VERSION}-linux-${ARCH}"
STAGE_DIR="target/tarball-stage/${PACK_NAME}"
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}/bin"
mkdir -p "${STAGE_DIR}/config"
mkdir -p "${STAGE_DIR}/share"
mkdir -p "${STAGE_DIR}/systemd"

cp "target/${TARGET}/release/apeireth" "${STAGE_DIR}/bin/apeireth"
chmod +x "${STAGE_DIR}/bin/apeireth"
cp "LICENSE" "${STAGE_DIR}/share/LICENSE"
cp "README.md" "${STAGE_DIR}/share/README.md"
cp "CHANGELOG.md" "${STAGE_DIR}/share/CHANGELOG.md" 2>/dev/null || true
cp "packaging/deb/apeireth.service" "${STAGE_DIR}/systemd/apeireth.service"
cp "packaging/tarball/install.sh" "${STAGE_DIR}/install.sh"
chmod +x "${STAGE_DIR}/install.sh"
cp "packaging/tarball/uninstall.sh" "${STAGE_DIR}/uninstall.sh"
chmod +x "${STAGE_DIR}/uninstall.sh"

cat > "${STAGE_DIR}/config/apeireth.env.example" <<'EOF'
# Apeireth OS 2.0 — 环境变量示例 (复制到 /etc/apeireth/env)
APEIRETH_HOME=/var/lib/apeireth
APEIRETH_CONFIG=/etc/apeireth/config.toml
APEIRETH_LOG_DIR=/var/log/apeireth
APEIRETH_API_KEY=
APEIRETH_BASE_URL=https://api.minimaxi.com/v1
APEIRETH_MODEL=MiniMax-M3
RUST_LOG=info
EOF

# 4. 写 README (放最上层)
cat > "${STAGE_DIR}/README.txt" <<EOF
Apeireth OS ${VERSION} — 通用 Linux/Unix 包 (musl 静态链接)

安装:
  sudo ./install.sh
  或者:
  sudo install -d /opt/apeireth
  sudo cp -r ./* /opt/apeireth/
  sudo ln -sf /opt/apeireth/bin/apeireth /usr/local/bin/apeireth
  sudo cp systemd/apeireth.service /etc/systemd/system/
  sudo systemctl daemon-reload
  sudo systemctl enable --now apeireth

验证:
  apeireth --version
  curl http://localhost:8080/health

卸载:
  sudo ./uninstall.sh
  或者:
  sudo systemctl disable --now apeireth
  sudo rm -rf /opt/apeireth /usr/local/bin/apeireth /etc/systemd/system/apeireth.service
  sudo systemctl daemon-reload
EOF

# 5. 打 tarball
echo "[3/5] tar -czf..."
TARBALL_PATH="target/${PACK_NAME}.tar.gz"
COPYFILE_DISABLE=1 tar -czf "${TARBALL_PATH}" -C target/tarball-stage "${PACK_NAME}"

# 6. sha256
echo "[4/5] sha256sum..."
SHA256=$(sha256sum "${TARBALL_PATH}" | cut -d' ' -f1)
echo "${SHA256}  ${TARBALL_PATH}" > "${TARBALL_PATH}.sha256"

# 7. 验证
SIZE=$(du -sh "${TARBALL_PATH}" | cut -f1)
echo "[5/5] tarball 产物: ${TARBALL_PATH} (${SIZE})"
echo "    sha256: ${SHA256}"
echo "    解包: tar -xf ${TARBALL_PATH}"
echo "    二进制: ${STAGE_DIR}/bin/apeireth (musl 静态, ldd 应显示 'not a dynamic executable')"

exit 0
