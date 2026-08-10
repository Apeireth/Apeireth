#!/usr/bin/env bash
# Apeireth Homebrew formula 构建 (8 包之 1)
# 平台: macOS
# 体积: ~40MB

set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION="${APEIRETH_VERSION:-1.0.0}"
TAP_REPO="${APEIRETH_TAP_REPO:-apeireth/homebrew-tap}"

echo "=== apeireth brew formula build v${VERSION} ==="

# 1. 计算 source tarball sha256
TARBALL_URL="https://github.com/apeireth/apeireth-rust/archive/refs/tags/v${VERSION}.tar.gz"
echo "[1/4] downloading source tarball for sha256..."
TARBALL_SHA256=$(curl -fsSL "${TARBALL_URL}" | shasum -a 256 | cut -d' ' -f1)
echo "    sha256: ${TARBALL_SHA256}"

# 2. 注入 sha256 到 formula
FORMULA_FILE="packaging/brew/apeireth.rb"
sed -i.bak "s|sha256 \"REPLACE_WITH_RELEASE_SHA256_AT_TAG_TIME\"|sha256 \"${TARBALL_SHA256}\"|" "${FORMULA_FILE}"
rm -f "${FORMULA_FILE}.bak"

# 3. tap 仓库 (per blueprint §3.8 GHCR 推送流程)
echo "[2/4] preparing tap repo: ${TAP_REPO}..."
TAP_DIR="homebrew-tap"
if [[ ! -d "${TAP_DIR}" ]]; then
    git clone "https://github.com/${TAP_REPO}.git" "${TAP_DIR}"
fi
mkdir -p "${TAP_DIR}/Formula"
cp "${FORMULA_FILE}" "${TAP_DIR}/Formula/apeireth.rb"

# 4. 提交 + 推送
echo "[3/4] commit + push..."
cd "${TAP_DIR}"
git add Formula/apeireth.rb
git commit -m "apeireth ${VERSION}" || echo "    (no changes)"
git push origin main || echo "    (push skipped — 期待 release engineer 推)"
cd -

echo "[4/4] brew 产物: ${TAP_DIR}/Formula/apeireth.rb"
echo "    验证: brew tap apeireth/tap && brew install apeireth/tap/apeireth"
