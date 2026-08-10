#!/usr/bin/env bash
# ==============================================================================
# R20 阶段 6 — cosign 1.0 release 用户侧验证 (per 蓝图 §3.5 #12 signature)
# 主人 2026-08-05 21:14 拍板"ABCD 都派, 内存大放心派"
#
# 用户安装 Apeireth 1.0 release 之前, 跑本脚本验包来源:
#   1. 解压下载的包到本地
#   2. 跑: bash scripts/release/cosign-verify.sh apeireth_1.0.0_amd64.deb
#   3. 看输出: ✓ verified = 真包 / ❌ FAILED = 0 信任 (per O-5 不假装)
#
# 用法:
#   bash scripts/release/cosign-verify.sh <pkg_path>
#   bash scripts/release/cosign-verify.sh /path/to/apeireth-1.0.0.msi
#   COSIGN_PUB=/custom/path/to/cosign.pub bash scripts/release/cosign-verify.sh <pkg>
#
# 必读输入:
#   - docs/security/cosign-keys.md (公钥位置 + 撤销流程)
#   - 8 项不修改承诺: docs/stage4/8-locked-unified-2026-08-05.md §2
# ==============================================================================

set -e
set -u
set -o pipefail

# === 参数 ===
PKG="${1:-}"
COSIGN_BIN="${COSIGN_BIN:-cosign}"
COSIGN_PUB="${COSIGN_PUB:-./docs/security/cosign.pub}"

# === 帮助 ===
if [[ -z "${PKG}" ]] || [[ "${PKG}" == "--help" ]] || [[ "${PKG}" == "-h" ]]; then
    cat <<EOF
用法: $0 <pkg_path>

1.0 release 用户侧 cosign 验证.

参数:
  <pkg_path>   要验证的包路径 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker OCI)
环境变量:
  COSIGN_BIN   cosign 二进制路径 (默认: PATH 里的 cosign)
  COSIGN_PUB   cosign 公钥路径 (默认: ./docs/security/cosign.pub)

示例:
  $0 apeireth_1.0.0_amd64.deb
  $0 /tmp/apeireth-1.0.0-1.x86_64.rpm
  COSIGN_PUB=~/apeireth.pub $0 apeireth.rb
  $0 ghcr.io/apeireth/apeireth:1.0.0   # Docker 走 cosign verify (OCI)

退出码:
  0  验签通过 (✓ 真包)
  1  验签失败 (❌ 0 信任, per O-5 不假装)
  2  参数 / 工具错误

详细见: docs/security/cosign-keys.md §6 撤销流程
EOF
    exit 0
fi

# === 前置检查 ===
if ! command -v "${COSIGN_BIN}" >/dev/null 2>&1; then
    echo "❌ cosign 不在 PATH: ${COSIGN_BIN}"
    echo "   安装: https://docs.sigstore.dev/cosign/installation/"
    exit 2
fi

if [[ ! -f "${COSIGN_PUB}" ]]; then
    echo "❌ 公钥文件不存在: ${COSIGN_PUB}"
    echo "   下载: https://github.com/apeireth/apeireth/blob/main/docs/security/cosign.pub"
    echo "   文档: docs/security/cosign-keys.md §2"
    exit 2
fi

# === Docker OCI 走 cosign verify (无 .sig 文件, 走 Fulcio+Rekor) ===
if [[ "${PKG}" == *"/"* ]] && [[ "${PKG}" != *".deb" ]] && [[ "${PKG}" != *".rpm" ]] \
    && [[ "${PKG}" != *".tar.gz" ]] && [[ "${PKG}" != *".zip" ]] && [[ "${PKG}" != *".msi" ]] \
    && [[ "${PKG}" != *".rb" ]] && [[ "${PKG}" != *".json" ]]; then
    echo ">>> Docker OCI 验签: ${PKG}"
    echo "    (走 Fulcio + Rekor 透明日志, 1.0 release 1-of-1 阈值, per docs/security/cosign-keys.md §4)"
    "${COSIGN_BIN}" verify "${PKG}" 2>&1
    exit $?
fi

# === 文件包验签 (有 .sig 文件) ===
SIG="${PKG}.sig"
if [[ ! -f "${SIG}" ]]; then
    echo "❌ 签名文件不存在: ${SIG}"
    echo "   期望: ${PKG} 旁边有同名 .sig 文件 (per scripts/release/cosign-sign-all.sh 产物)"
    echo "   下载: https://github.com/apeireth/apeireth/releases/tag/v1.0.0 (签名在 assets)"
    exit 2
fi

if [[ ! -f "${PKG}" ]]; then
    echo "❌ 包文件不存在: ${PKG}"
    exit 2
fi

# === 验签 ===
echo ">>> 验签: ${PKG}"
echo "    公钥: ${COSIGN_PUB}"
echo "    签名: ${SIG}"
echo ""

if "${COSIGN_BIN}" verify-blob \
    --key "${COSIGN_PUB}" \
    --signature "${SIG}" \
    "${PKG}"; then
    echo ""
    echo "✅ 验签通过 — 包来源可信, 可安全安装"
    echo "   详情: docs/security/cosign-keys.md §2.1 公钥 fingerprint"
    exit 0
else
    echo ""
    echo "❌ 验签失败 — 0 信任 (per O-5 不假装)"
    echo ""
    echo "下一步 (per docs/security/cosign-keys.md §6 撤销流程):"
    echo "  1. 不安装该包"
    echo "  2. 检查 rekor.sigstore.dev 透明日志, 确认是否签了"
    echo "  3. 报告 issue: https://github.com/apeireth/apeireth/issues (含 verify 输出 + 包 sha256)"
    exit 1
fi
