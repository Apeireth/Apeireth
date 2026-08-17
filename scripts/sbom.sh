#!/usr/bin/env bash
# ============================================================================
# scripts/sbom.sh — TP20-S5 塞缝批
# ----------------------------------------------------------------------------
# 用 cargo-cyclonedx 生成 CycloneDX 1.5 SBOM, 落地 cyclonedx-sbom.json。
#
# 触发: release tag push / `make sbom` / 主人手动 `bash scripts/sbom.sh`
#
# 为什么 CycloneDX 1.5:
#   - SPDX 主给 license 归档, CycloneDX 主给 component/supplier/hashing,
#     对供应链审计更友好
#   - cargo-cyclonedx 默认输出 CycloneDX 1.3 / 1.4 / 1.5, 这里钉死 1.5
#   - 安全审计下游工具 (Grype / Dependency-Track / Snyk) 都吃 1.5
#
# 工具链要求 (0 装 PASS 边界):
#   - cargo (Rust stable)
#   - cargo-cyclonedx : 若缺 → SKIP + 文档标 fallback
#   - jq (验证 JSON 合法性): 若缺 → SKIP 验证, exit 0 但打 warning
#
# 安装 fallback:
#   cargo install cargo-cyclonedx --locked
#
# 已知限制 (TP20-S5 主人拍板边界):
#   - cargo-cyclonedx 0.5.9 不支持单文件 workspace SBOM (--output-file 不存在),
#     只生成 per-crate SBOMs 到各 crate 目录
#   - 本脚本: 跑全 workspace → 收集所有 *.cdx.json 到 sbom/ → 选 apeireth-cli
#     的 (覆盖最全传递依赖) 复制为根 cyclonedx-sbom.json 作为「主 SBOM」
#   - 不做 signed SBOM (无 cosign attach), 留给 R20 阶段 6 续
#   - 不接 online CVE DB (走 offline advisory-db 在 vet.sh 里)
#   - 不生成 SPDX 格式 (cyclonedx + spdx 二选一, 本批走 cyclonedx)
#
# 与 release-tools crate 的关系:
#   - release_tools::CYCLONEDX_SPEC_VERSION = "1.5" 是本脚本的对账常量
#   - release_tools::SBOM_FILENAME = "cyclonedx-sbom.json" 是输出文件名约定
#   - 不在脚本里硬编码这两个字符串, 而是从 release_tools crate 读 (编译期对账)
#     简化: 现阶段脚本里直接写 "1.5" / "cyclonedx-sbom.json", 与 crate 同步
#     ponytail: 后续若 spec/filename 频繁变, 再用 `cargo run -p release-tools
#     --bin print-sbom-constants` 替硬编码。
#
# 验收:
#   bash scripts/sbom.sh
#   → cyclonedx-sbom.json 存在 + jq . valid + specVersion = 1.5
#   → sbom/ 目录含全部 per-crate *.cdx.json
#
# 用法:
#   bash scripts/sbom.sh                            # 默认输出 cyclonedx-sbom.json
#   APEIRETH_SBOM_OUT=release/v1.0.0-sbom.json bash scripts/sbom.sh  # 自定义输出
# ============================================================================

set -u
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOLS_DIR="${REPO_ROOT}/tools"
SBOM_OUT="${APEIRETH_SBOM_OUT:-${REPO_ROOT}/cyclonedx-sbom.json}"
SBOM_DIR="${REPO_ROOT}/sbom"
SBOM_SPEC="1.5"  # 与 release_tools::CYCLONEDX_SPEC_VERSION 对账

cd "${REPO_ROOT}"

# 把 tools/bin 加入 PATH (cargo install --root tools/ 时路径约定)
if [[ -d "${TOOLS_DIR}/bin" ]]; then
    export PATH="${TOOLS_DIR}/bin:${PATH}"
fi

echo "============================================================"
echo "  TP20-S5 — CycloneDX ${SBOM_SPEC} SBOM 生成"
echo "  Repo:   ${REPO_ROOT}"
echo "  Output: ${SBOM_OUT}"
echo "  Dir:    ${SBOM_DIR}"
echo "============================================================"
echo ""

# ----------------------------------------------------------------------------
# 0. toolchain 探测
# ----------------------------------------------------------------------------
if ! command -v cargo-cyclonedx >/dev/null 2>&1; then
    echo "❌ cargo-cyclonedx 未装"
    echo "  fallback install: cargo install cargo-cyclonedx --locked"
    echo "  本地 SKIP (CI release.yml 的 security-and-sbom job 必须装)"
    exit 0
fi

# ----------------------------------------------------------------------------
# 1. 生成 SBOM (cargo cyclonedx 全 workspace, 默认 .cdx.json per crate)
# ----------------------------------------------------------------------------
echo ">>> cargo cyclonedx (CycloneDX JSON, spec ${SBOM_SPEC}, 全 workspace)"

# 准备 sbom 目录
mkdir -p "${SBOM_DIR}"
# 清理旧产物 (避免遗留文件混入本次 release)
rm -f "${SBOM_DIR}"/*.cdx.json
find "${REPO_ROOT}/crates" -name "*.cdx.json" -type f -delete 2>/dev/null || true

# cargo-cyclonedx 0.5.9 不支持 --output-file (写死 per-crate to crate dir),
# 我们接受这一行为, 跑完后统一收口到 ${SBOM_DIR}/
cargo cyclonedx \
    --format json \
    --spec-version "${SBOM_SPEC}" \
    2>&1 | tee "${REPO_ROOT}/sbom-cyclonedx.stderr.txt" | tail -20

CYCLONEDX_EXIT=${PIPESTATUS[0]}
echo "  exit=${CYCLONEDX_EXIT}"
echo ""

if [[ "${CYCLONEDX_EXIT}" -ne 0 ]]; then
    echo "❌ cargo cyclonedx 失败, SBOM 未生成"
    exit "${CYCLONEDX_EXIT}"
fi

# ----------------------------------------------------------------------------
# 2. 收口 per-crate SBOM 到 ${SBOM_DIR}/ (honest, 不假装"单 workspace SBOM")
# ----------------------------------------------------------------------------
echo ">>> 收集 per-crate SBOMs 到 ${SBOM_DIR}/"
COLLECTED=0
while IFS= read -r -d '' f; do
    BASENAME="$(basename "${f}")"
    CRATE_DIR="$(dirname "${f}")"
    CRATE_NAME="$(basename "${CRATE_DIR}")"
    # 改名: <crate>__<basename>.cdx.json 避免重名
    mv "${f}" "${SBOM_DIR}/${CRATE_NAME}__${BASENAME}" 2>/dev/null || true
    COLLECTED=$((COLLECTED + 1))
done < <(find "${REPO_ROOT}/crates" -name "*.cdx.json" -type f -print0 2>/dev/null)
echo "  收集了 ${COLLECTED} 个 per-crate SBOM"
echo ""

# ----------------------------------------------------------------------------
# 3. 选「主 SBOM」写到 ${SBOM_OUT}
# ----------------------------------------------------------------------------
# 选 apeireth-cli 的 (它依赖最广, 覆盖最完整的传递依赖图, 作为 release 主 SBOM)
# cargo-cyclonedx 默认 --describe crate 命名 <crate>.cdx.json, 收口时改成
# <crate>__<crate>.cdx.json
PRIMARY="${SBOM_DIR}/apeireth-cli__apeireth-cli.cdx.json"
if [[ ! -f "${PRIMARY}" ]]; then
    # fallback: 找 components 数最大的那个 (传递依赖最完整)
    PRIMARY=""
    MAX_COMPONENTS=0
    while IFS= read -r f; do
        if command -v jq >/dev/null 2>&1; then
            N="$(jq '.components // [] | length' "${f}" 2>/dev/null || echo 0)"
        else
            # 没 jq 时只看文件大小 (大 ≈ components 多)
            N="$(wc -c < "${f}")"
        fi
        if [[ "${N}" -gt "${MAX_COMPONENTS}" ]]; then
            MAX_COMPONENTS="${N}"
            PRIMARY="${f}"
        fi
    done < <(find "${SBOM_DIR}" -name "*.cdx.json" -type f 2>/dev/null)
fi

if [[ -z "${PRIMARY}" ]] || [[ ! -f "${PRIMARY}" ]]; then
    echo "❌ 没找到任何 per-crate SBOM, 主 SBOM 无法产出"
    exit 1
fi

cp "${PRIMARY}" "${SBOM_OUT}"
echo ">>> 主 SBOM: ${SBOM_OUT}"
echo "    (来源: ${PRIMARY##*/})"
echo ""

# ----------------------------------------------------------------------------
# 4. JSON 合法性 + specVersion 对账
# ----------------------------------------------------------------------------
echo ">>> JSON 合法性 + specVersion 对账"
if command -v jq >/dev/null 2>&1; then
    if ! jq empty "${SBOM_OUT}" 2>/dev/null; then
        echo "❌ ${SBOM_OUT} JSON 不合法"
        exit 1
    fi
    GENERATED_SPEC="$(jq -r '.specVersion // empty' "${SBOM_OUT}" 2>/dev/null)"
    if [[ "${GENERATED_SPEC}" != "${SBOM_SPEC}" ]]; then
        echo "❌ specVersion 不对账: 生成=${GENERATED_SPEC:-?} 期望=${SBOM_SPEC}"
        exit 1
    fi
    COMPONENT_COUNT="$(jq '.components // [] | length' "${SBOM_OUT}" 2>/dev/null)"
    echo "  ✅ JSON 合法, specVersion=${GENERATED_SPEC}, components=${COMPONENT_COUNT}"
else
    echo "  ⚠️  jq 未装, 跳过 JSON 验证 (CI release.yml 必须装)"
    echo "  fallback install: choco install jq / apt install jq / brew install jq"
fi

# ----------------------------------------------------------------------------
# 5. 报告
# ----------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "  TP20-S5 SBOM 生成完成"
echo "============================================================"
echo "  主 SBOM: ${SBOM_OUT}"
echo "  Spec:    CycloneDX ${SBOM_SPEC}"
if command -v jq >/dev/null 2>&1; then
    echo "  Comp:    ${COMPONENT_COUNT} components (主 SBOM)"
fi
echo "  Per-crate: ${SBOM_DIR}/ (${COLLECTED} 文件)"
echo "  Stderr:  sbom-cyclonedx.stderr.txt"
echo "============================================================"

exit 0