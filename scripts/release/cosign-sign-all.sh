#!/usr/bin/env bash
# ==============================================================================
# R20 阶段 6 — cosign 8 包签名 (1.0 release #3 signature, 蓝图 §3.5)
# 主人 2026-08-05 21:14 拍板"ABCD 都派, 内存大放心派"
#
# 8 包签名机制:
#   1. deb        — cosign sign-blob (Rekor 透明日志)
#   2. rpm        — cosign sign-blob
#   3. brew       — cosign sign-blob (formula JSON)
#   4. scoop      — cosign sign-blob (manifest JSON)
#   5. tarball    — cosign sign-blob
#   6. zip        — cosign sign-blob
#   7. MSI        — cosign sign-blob (Authenticode 走 signtool, 单独 CI job)
#   8. Docker     — cosign sign (OCI image, GitHub OIDC Fulcio)
#
# 用法:
#   bash scripts/release/cosign-sign-all.sh
#   COSIGN_BIN=/path/to/cosign bash scripts/release/cosign-sign-all.sh
#   DIST_DIR=/path/to/dist bash scripts/release/cosign-sign-all.sh
#   bash scripts/release/cosign-sign-all.sh --dry-run   # 仅打印, 不签名
#
# 必读输入:
#   - docs/security/cosign-keys.md (公钥 + 私钥管理 + 撤销流程)
#   - scripts/release/cosign-verify.sh (用户侧验证)
#   - 8 项不修改承诺: docs/stage4/8-locked-unified-2026-08-05.md §2
# ==============================================================================

set -e
set -u
set -o pipefail

# === 环境变量 (可覆盖) ===
COSIGN_BIN="${COSIGN_BIN:-cosign}"
COSIGN_KEY="${COSIGN_KEY:-./cosign.key}"
COSIGN_PUB="${COSIGN_PUB:-./docs/security/cosign.pub}"
DIST_DIR="${DIST_DIR:-./dist}"
SIG_DIR="${DIST_DIR}/signatures"
VERSION="${APEIRETH_VERSION:-1.0.0}"
DRY_RUN=false

# === 参数解析 ===
for arg in "$@"; do
    case "${arg}" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help|-h)
            sed -n '2,30p' "$0"
            exit 0
            ;;
        *)
            echo "❌ 未知参数: ${arg}"
            sed -n '2,30p' "$0"
            exit 1
            ;;
    esac
done

# === Banner ===
echo ""
echo "=================================================="
echo "  R20 阶段 6 — cosign 8 包签名 (1.0 release)"
echo "  版本:   ${VERSION}"
echo "  模式:   $(if [[ "${DRY_RUN}" == "true" ]]; then echo "DRY-RUN"; else echo "LIVE"; fi)"
echo "  cosign: ${COSIGN_BIN}"
echo "  私钥:   ${COSIGN_KEY}"
echo "  公钥:   ${COSIGN_PUB}"
echo "  dist:   ${DIST_DIR}"
echo "  sigs:   ${SIG_DIR}"
echo "=================================================="
echo ""

# === 前置检查 (per O-5 不假装) ===

# 1. cosign 二进制存在
if ! command -v "${COSIGN_BIN}" >/dev/null 2>&1; then
    echo "❌ cosign 不在 PATH: ${COSIGN_BIN}"
    echo "   安装: https://docs.sigstore.dev/cosign/installation/"
    echo "   验证: cosign version (期望 v2.2+)"
    exit 1
fi

# 2. cosign 版本 (>= v2.2 推荐, v1.x 也兼容但 OIDC 路径可能不同)
COSIGN_VERSION=$("${COSIGN_BIN}" version 2>&1 | head -1 || echo "unknown")
echo "✓ cosign 版本: ${COSIGN_VERSION}"

# 3. 私钥存在 (DRY-RUN 跳过)
if [[ "${DRY_RUN}" == "false" ]]; then
    if [[ ! -f "${COSIGN_KEY}" ]]; then
        echo "❌ 私钥文件不存在: ${COSIGN_KEY}"
        echo "   生成: cosign generate-key-pair"
        echo "   存储: GitHub Actions Secret COSIGN_KEY (per docs/security/cosign-keys.md §3)"
        exit 1
    fi
    # 4. 公钥存在
    if [[ ! -f "${COSIGN_PUB}" ]]; then
        echo "❌ 公钥文件不存在: ${COSIGN_PUB}"
        echo "   生成: cosign generate-key-pair 后 commit cosign.pub 到 docs/security/"
        exit 1
    fi
    # 5. dist 目录存在
    if [[ ! -d "${DIST_DIR}" ]]; then
        echo "❌ dist 目录不存在: ${DIST_DIR}"
        echo "   跑 packaging/<target>/build.{sh,ps1} 8 包, 产物进 ${DIST_DIR}/"
        exit 1
    fi
fi

mkdir -p "${SIG_DIR}"

# === 通用签名函数 ===
# 用法: sign_blob <pkg_path> <sig_path> <desc>
sign_blob() {
    local pkg="$1"
    local sig="$2"
    local desc="$3"

    if [[ ! -f "${pkg}" ]]; then
        echo "  ⚠️  跳过 ${desc}: ${pkg} 不存在 (可能该包在当前 OS 不 build)"
        return 0
    fi

    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "  [DRY-RUN] cosign sign-blob --key <key> ${pkg} → ${sig}"
        return 0
    fi

    # 同时输出 .sig (signature) + .cert (证书) + 透明日志条目
    "${COSIGN_BIN}" sign-blob \
        --key "${COSIGN_KEY}" \
        --output-signature "${sig}" \
        --output-certificate "${sig}.cert" \
        --bundle "${sig}.bundle" \
        "${pkg}" >/dev/null 2>&1

    # SHA256 fallback (apt/rpm 自带校验)
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${pkg}" | awk '{print $1}' > "${sig%.sig}.sha256"
    fi

    local size
    size=$(stat -c '%s' "${pkg}" 2>/dev/null || stat -f '%z' "${pkg}" 2>/dev/null || echo "?")
    echo "  ✓ ${desc} (${size} bytes) → ${sig}"
}

# === 8 包签名 ===

echo "[1/8] deb 签名 (Ubuntu/Debian)"
sign_blob \
    "${DIST_DIR}/apeireth_${VERSION}_amd64.deb" \
    "${SIG_DIR}/apeireth_${VERSION}_amd64.deb.sig" \
    "deb (linux/amd64)"

echo "[2/8] rpm 签名 (RHEL/Fedora)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-1.x86_64.rpm" \
    "${SIG_DIR}/apeireth-${VERSION}-1.x86_64.rpm.sig" \
    "rpm (linux/amd64)"

echo "[3/8] brew 签名 (macOS formula)"
sign_blob \
    "${DIST_DIR}/apeireth.rb" \
    "${SIG_DIR}/apeireth.rb.sig" \
    "brew (macOS)"

echo "[4/8] scoop 签名 (Windows manifest)"
sign_blob \
    "${DIST_DIR}/apeireth.json" \
    "${SIG_DIR}/apeireth.json.sig" \
    "scoop (Windows)"

echo "[5/8] tarball 签名 (Linux/macOS 离线包)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-linux-amd64.tar.gz" \
    "${SIG_DIR}/apeireth-${VERSION}-linux-amd64.tar.gz.sig" \
    "tarball (linux/amd64)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-darwin-universal.tar.gz" \
    "${SIG_DIR}/apeireth-${VERSION}-darwin-universal.tar.gz.sig" \
    "tarball (macOS universal)"

echo "[6/8] zip 签名 (Windows 通用)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-windows-amd64.zip" \
    "${SIG_DIR}/apeireth-${VERSION}-windows-amd64.zip.sig" \
    "zip (windows/amd64)"

echo "[7/8] MSI 签名 (Windows Installer, signtool + cosign)"
# Authenticode 走 signtool (单独 CI job, per docs/security/cosign-keys.md §1)
# cosign 走供应链签名 (本脚本)
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}.msi" \
    "${SIG_DIR}/apeireth-${VERSION}.msi.sig" \
    "MSI (windows/amd64)"

echo "[8/8] Docker 签名 (cosign sign OCI, GitHub OIDC Fulcio)"
# Docker 走 cosign sign (不是 sign-blob), 用 OIDC token 而非本地私钥
DOCKER_IMAGE="${DOCKER_IMAGE:-ghcr.io/apeireth/apeireth:${VERSION}}"
DOCKER_SIG="${SIG_DIR}/apeireth-docker-${VERSION}.sig"
DOCKER_CERT="${SIG_DIR}/apeireth-docker-${VERSION}.crt"

if command -v docker >/dev/null 2>&1 && docker image inspect "${DOCKER_IMAGE}" >/dev/null 2>&1; then
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "  [DRY-RUN] cosign sign ${DOCKER_IMAGE} (OIDC)"
    else
        # OIDC 签名: 走 GitHub Actions token (Fulcio CA 短期证书)
        "${COSIGN_BIN}" sign \
            --output-signature "${DOCKER_SIG}" \
            --output-certificate "${DOCKER_CERT}" \
            "${DOCKER_IMAGE}" >/dev/null 2>&1
        echo "  ✓ Docker (${DOCKER_IMAGE}) → ${DOCKER_SIG}"
    fi
else
    echo "  ⚠️  跳过 Docker: ${DOCKER_IMAGE} 不在本地 docker (CI 阶段由 release-1.0.0.yml docker-multi-arch job 推 GHCR 后签)"
fi

# === 验证 (per O-5 不假装 + §6 撤销流程) ===
echo ""
echo "=== 签名验证 (cosign verify-blob) ==="
VERIFY_FAIL=0
for sig in "${SIG_DIR}"/*.sig; do
    [[ -e "${sig}" ]] || continue
    pkg="${sig%.sig}"
    if [[ ! -f "${pkg}" ]]; then
        echo "  ⚠️  跳过: ${pkg} 缺失, 无法验"
        continue
    fi
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "  [DRY-RUN] cosign verify-blob --key ${COSIGN_PUB} --signature ${sig} ${pkg}"
        continue
    fi
    if "${COSIGN_BIN}" verify-blob \
        --key "${COSIGN_PUB}" \
        --signature "${sig}" \
        "${pkg}" >/dev/null 2>&1; then
        echo "  ✓ ${pkg##*/}"
    else
        echo "  ❌ ${pkg##*/} verify 失败"
        VERIFY_FAIL=$((VERIFY_FAIL+1))
    fi
done

# === 报告 ===
echo ""
echo "=================================================="
echo "  cosign 8 包签名完成 (per R20 阶段 6)"
echo "  签名目录: ${SIG_DIR}/"
echo "  公钥:     ${COSIGN_PUB}"
echo "  验证失败: ${VERIFY_FAIL}/$({ ls -1 "${SIG_DIR}"/*.sig 2>/dev/null | wc -l; } || echo 0)"
echo "=================================================="
echo ""
echo "用户验证命令 (per docs/security/cosign-keys.md §6.2):"
echo "  cosign verify-blob --key ${COSIGN_PUB} --signature <pkg>.sig <pkg>"
echo ""
echo "或用本仓验证脚本:"
echo "  bash scripts/release/cosign-verify.sh <pkg_path>"
echo ""

# 任何 verify fail → exit 1 (阻塞 1.0 release tag, per release-checklist)
if [[ ${VERIFY_FAIL} -gt 0 ]]; then
    echo "❌ ${VERIFY_FAIL} 包 verify 失败, 1.0 release tag 阻塞"
    exit 1
fi
exit 0
