#!/usr/bin/env bash
# ==============================================================================
# R20 阶段 6 — cosign 8 包签名 (1.0 release #3 signature, 内部规范)
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
# COSIGN_KEY_B64 is the CI secret material. COSIGN_KEY_PATH is the decoded
# file consumed by cosign; COSIGN_KEY remains a backwards-compatible local
# path alias only. Never print either secret material or a caller-provided path.
COSIGN_KEY_B64="${COSIGN_KEY_B64:-}"
COSIGN_KEY_PATH="${COSIGN_KEY_PATH:-${COSIGN_KEY:-./cosign.key}}"
COSIGN_PUB="${COSIGN_PUB:-./docs/security/cosign.pub}"
DIST_DIR="${DIST_DIR:-./dist}"
SIG_DIR="${DIST_DIR}/signatures"
VERSION="${APEIRETH_VERSION:-1.0.0}"
DRY_RUN=false
KEY_WORK_DIR=""

cleanup_key_material() {
    if [[ -n "${KEY_WORK_DIR}" ]] && [[ -d "${KEY_WORK_DIR}" ]]; then
        rm -f -- "${KEY_WORK_DIR}/cosign.key" "${KEY_WORK_DIR}/derived-cosign.pub"
        rmdir -- "${KEY_WORK_DIR}" 2>/dev/null || true
    fi
}
trap cleanup_key_material EXIT HUP INT TERM

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
echo "  私钥源: $(if [[ -n "${COSIGN_KEY_B64}" ]]; then echo "encoded CI secret"; else echo "local key file"; fi)"
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

# 3. dist 目录始终必须存在. Dry-run validates the same release manifest;
# it only skips cryptographic operations.
if [[ ! -d "${DIST_DIR}" ]]; then
    echo "❌ dist 目录不存在: ${DIST_DIR}"
    exit 1
fi

# 4. Prepare and validate private key (DRY-RUN skips cryptographic material).
if [[ "${DRY_RUN}" == "false" ]]; then
    umask 077
    KEY_WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/apeireth-cosign-key.XXXXXX")

    if [[ -n "${COSIGN_KEY_B64}" ]]; then
        if [[ -z "${COSIGN_PASSWORD:-}" ]]; then
            echo "❌ COSIGN_PASSWORD 未配置，拒绝交互式或空口令签名"
            exit 1
        fi
        COSIGN_KEY_PATH="${KEY_WORK_DIR}/cosign.key"
        if printf '%s' "${COSIGN_KEY_B64}" | base64 --decode > "${COSIGN_KEY_PATH}" 2>/dev/null; then
            :
        elif printf '%s' "${COSIGN_KEY_B64}" | base64 -D > "${COSIGN_KEY_PATH}" 2>/dev/null; then
            :
        else
            echo "❌ COSIGN_KEY_B64 不是有效的 base64 私钥材料"
            exit 1
        fi
    fi

    if [[ ! -s "${COSIGN_KEY_PATH}" ]]; then
        echo "❌ 私钥文件缺失或为空"
        echo "   CI: 配置 COSIGN_KEY(base64) + COSIGN_PASSWORD；本地: 设置 COSIGN_KEY_PATH"
        exit 1
    fi
    # 5. Public key must exist and match the injected private key.
    if [[ ! -s "${COSIGN_PUB}" ]]; then
        echo "❌ 公钥文件不存在: ${COSIGN_PUB}"
        echo "   生成: cosign generate-key-pair 后 commit cosign.pub 到 docs/security/"
        exit 1
    fi
    if ! "${COSIGN_BIN}" public-key --key "${COSIGN_KEY_PATH}" \
        > "${KEY_WORK_DIR}/derived-cosign.pub" 2>/dev/null; then
        echo "❌ 无法从私钥导出公钥（检查私钥格式和 COSIGN_PASSWORD）"
        exit 1
    fi
    if ! cmp -s "${KEY_WORK_DIR}/derived-cosign.pub" "${COSIGN_PUB}"; then
        echo "❌ 私钥与仓库中已审核的公钥不匹配"
        exit 1
    fi
    echo "✓ 私钥材料已解码并与已审核公钥匹配"
fi

mkdir -p "${SIG_DIR}"

# === 通用签名函数 ===
# 用法: sign_blob <pkg_path> <desc>
sign_blob() {
    local pkg="$1"
    local desc="$2"
    local bundle="${SIG_DIR}/$(basename "${pkg}").sigstore.json"

    if [[ ! -f "${pkg}" ]]; then
        echo "  ❌ 缺少预期发布产物 ${desc}: ${pkg}"
        return 1
    fi

    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "  [DRY-RUN] cosign sign-blob --key <key> --bundle ${bundle} ${pkg}"
        return 0
    fi

    # The standardized Sigstore bundle carries the signature and the Rekor
    # inclusion material. Self-managed-key signing does not create a Fulcio
    # certificate, so an independent .cert file would be a false requirement.
    "${COSIGN_BIN}" sign-blob \
        --yes \
        --key "${COSIGN_KEY_PATH}" \
        --bundle "${bundle}" \
        "${pkg}" >/dev/null 2>&1
    if [[ ! -s "${bundle}" ]]; then
        echo "  ❌ cosign 未生成非空 Sigstore bundle: ${bundle}"
        return 1
    fi

    # SHA256 fallback (apt/rpm 自带校验)
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${pkg}" | awk '{print $1}' > "${bundle%.sigstore.json}.sha256"
    fi

    local size
    size=$(stat -c '%s' "${pkg}" 2>/dev/null || stat -f '%z' "${pkg}" 2>/dev/null || echo "?")
    echo "  ✓ ${desc} (${size} bytes) → ${bundle}"
}

# === 8 包签名 ===

echo "[1/8] deb 签名 (Ubuntu/Debian)"
sign_blob \
    "${DIST_DIR}/apeireth_${VERSION}_amd64.deb" \
    "deb (linux/amd64)"

echo "[2/8] rpm 签名 (RHEL/Fedora)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-1.x86_64.rpm" \
    "rpm (linux/amd64)"

echo "[3/8] brew 签名 (macOS formula)"
sign_blob \
    "${DIST_DIR}/apeireth.rb" \
    "brew (macOS)"

echo "[4/8] scoop 签名 (Windows manifest)"
sign_blob \
    "${DIST_DIR}/apeireth.json" \
    "scoop (Windows)"

echo "[5/8] tarball 签名 (Linux 离线包)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-linux-x86_64.tar.gz" \
    "tarball (linux/amd64)"

echo "[6/8] zip 签名 (Windows 通用)"
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-windows-x86_64.zip" \
    "zip (windows/amd64)"

echo "[7/8] MSI 签名 (Windows Installer, signtool + cosign)"
# Authenticode 走 signtool (单独 CI job, per docs/security/cosign-keys.md §1)
# cosign 走供应链签名 (本脚本)
sign_blob \
    "${DIST_DIR}/apeireth-${VERSION}-windows-x86_64.msi" \
    "MSI (windows/amd64)"

echo "[8/8] Docker 签名 (cosign sign OCI, GitHub OIDC Fulcio)"
# Docker 走 cosign sign (不是 sign-blob), 用 OIDC token 而非本地私钥
DOCKER_IMAGE="${DOCKER_IMAGE:-ghcr.io/apeireth/apeireth:${VERSION}}"

if command -v docker >/dev/null 2>&1 && docker image inspect "${DOCKER_IMAGE}" >/dev/null 2>&1; then
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "  [DRY-RUN] cosign sign ${DOCKER_IMAGE} (OIDC)"
    else
        # OIDC 签名: 走 GitHub Actions token (Fulcio CA 短期证书)
        "${COSIGN_BIN}" sign --yes "${DOCKER_IMAGE}" >/dev/null 2>&1
        echo "  ✓ Docker signature stored with OCI image (${DOCKER_IMAGE})"
    fi
else
    echo "  ⚠️  跳过 Docker: ${DOCKER_IMAGE} 不在本地 docker (CI 阶段由 release-1.0.0.yml docker-multi-arch job 推 GHCR 后签)"
fi

# === 验证 (per O-5 不假装 + §6 撤销流程) ===
echo ""
echo "=== 签名验证 (cosign verify-blob) ==="
VERIFY_FAIL=0
VERIFY_TOTAL=0
if [[ "${DRY_RUN}" == "true" ]]; then
    echo "  [DRY-RUN] expected artifact manifest complete; cryptographic verification skipped"
    exit 0
fi
for bundle in "${SIG_DIR}"/*.sigstore.json; do
    [[ -e "${bundle}" ]] || continue
    pkg="${DIST_DIR}/$(basename "${bundle%.sigstore.json}")"
    if [[ ! -f "${pkg}" ]]; then
        echo "  ❌ 签名没有对应发布产物: ${pkg}"
        VERIFY_FAIL=$((VERIFY_FAIL+1))
        continue
    fi
    if [[ ! -s "${bundle}" ]]; then
        echo "  ❌ ${pkg##*/} 缺少非空 Sigstore bundle"
        VERIFY_FAIL=$((VERIFY_FAIL+1))
        continue
    fi
    VERIFY_TOTAL=$((VERIFY_TOTAL+1))
    if "${COSIGN_BIN}" verify-blob \
        --bundle "${bundle}" \
        --key "${COSIGN_PUB}" \
        "${pkg}" >/dev/null 2>&1; then
        echo "  ✓ ${pkg##*/}"
    else
        echo "  ❌ ${pkg##*/} verify 失败"
        VERIFY_FAIL=$((VERIFY_FAIL+1))
    fi
done

if [[ ${VERIFY_TOTAL} -eq 0 ]]; then
    echo "❌ 没有任何可验证签名，拒绝 0/0 通过"
    exit 1
fi

# === 报告 ===
echo ""
echo "=================================================="
echo "  cosign 8 包签名完成 (per R20 阶段 6)"
echo "  签名目录: ${SIG_DIR}/"
echo "  公钥:     ${COSIGN_PUB}"
echo "  验证失败: ${VERIFY_FAIL}/${VERIFY_TOTAL}"
echo "=================================================="
echo ""
echo "用户验证命令 (per docs/security/cosign-keys.md §6.2):"
echo "  cosign verify-blob --bundle <pkg>.sigstore.json --key ${COSIGN_PUB} <pkg>"
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
