#!/usr/bin/env bash
# ============================================================================
# 治理卷宗面对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 抽取单一真源资产（四个 Tab + 六个共享子件的 scoped CSS / 图标 / 固定数据）
#   2. 生成 runtime 替身（按生产导出面派生）
#   3. 构建 wasm 产物（**自带的独立 workspace** ⇒ 不进 crates-ui 依赖图）
#   4. wasm-bindgen ⇒ JS 胶水 + wasm（放进 harness 的 public/engine/pkg/）
#   5. vite build ⇒ 八个页面（参考侧 / 引擎侧 × 四个卷宗面）
#   6. 跑对照（像素 + DOM 全等 + 逐帧）
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATES_UI="$(cd "${HERE}/../../.." && pwd)"
REPO="$(cd "${CRATES_UI}/.." && pwd)"
HOST="${HERE}/engine-host"

WASM_BINDGEN="${WASM_BINDGEN:-${REPO}/target/ui-probe-scratch/bin/bin/wasm-bindgen}"
if ! command -v wasm-bindgen >/dev/null 2>&1 && [ ! -x "${WASM_BINDGEN}" ]; then
  echo "找不到 wasm-bindgen（可用 WASM_BINDGEN=<路径> 指定）" >&2
  exit 1
fi
WASM_BINDGEN="$(command -v wasm-bindgen || echo "${WASM_BINDGEN}")"

echo "── 1/6 抽取单一真源资产 ──"
node "${HERE}/scripts/extract-governance-assets.mjs"

echo "── 2/6 生成 runtime 替身 ──"
node "${HERE}/scripts/gen-runtime-mock.mjs"

echo "── 3/6 构建 wasm 产物（独立 workspace） ──"
(cd "${HOST}" && cargo build --release --target wasm32-unknown-unknown --locked)

echo "── 4/6 wasm-bindgen ──"
# ⚠️ **产物路径必须跟随 `CARGO_TARGET_DIR`**（仓库脚本纪律）：
# 否则外部设了 `CARGO_TARGET_DIR` 时，cargo 把产物写到别处，
# 而本脚本仍去读 `${HOST}/target/...` ⇒ ⇒ **拿到旧 wasm** ⇒
# ⇒ ⇒ 实测踩过：注入的修复**没进浏览器**（页面仍显 UTC）⇒ **假绿/假红**。
TARGET_DIR="${CARGO_TARGET_DIR:-${HOST}/target}"
WASM_ARTIFACT="${TARGET_DIR}/wasm32-unknown-unknown/release/governance_harness.wasm"
if [ ! -f "${WASM_ARTIFACT}" ]; then
  echo "找不到 wasm 产物：${WASM_ARTIFACT}" >&2
  exit 1
fi
rm -rf "${HERE}/public/engine/pkg"
mkdir -p "${HERE}/public/engine/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-name governance-harness \
  --out-dir "${HERE}/public/engine/pkg" \
  "${WASM_ARTIFACT}"

echo "── 5/6 vite build ──"
if [ ! -d "${HERE}/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的只读依赖目录）"
  ln -s "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/node_modules"
fi
(cd "${HERE}" && npx --no-install vite build)

echo "── 6/6 对照（像素 + DOM + 逐帧） ──"
python3 "${HERE}/tools/compare_governance.py"
