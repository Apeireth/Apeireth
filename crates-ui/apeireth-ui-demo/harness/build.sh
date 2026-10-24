#!/usr/bin/env bash
# ============================================================================
# P3 试点对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 抽取单一真源资产（卡片 scoped CSS / 设计令牌 CSS / 图标数据 / 固定数据）
#   2. 构建 wasm 产物（release）
#   3. wasm-bindgen ⇒ JS 胶水 + wasm（放进 harness 的 public/engine/pkg/）
#   4. vite build ⇒ 两个页面（参考侧 / 引擎侧）+ 静态资产
#   5. 跑对照（像素 + 逐帧 + DOM 全等 + JSON 差分）
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、node + vite、python3
# （websocket-client + Pillow）、系统 Chromium。
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATES_UI="$(cd "${HERE}/../.." && pwd)"
REPO="$(cd "${CRATES_UI}/.." && pwd)"

WASM_BINDGEN="${WASM_BINDGEN:-${REPO}/target/ui-probe-scratch/bin/bin/wasm-bindgen}"
if ! command -v wasm-bindgen >/dev/null 2>&1 && [ ! -x "${WASM_BINDGEN}" ]; then
  echo "找不到 wasm-bindgen（可用 WASM_BINDGEN=<路径> 指定）" >&2
  exit 1
fi
WASM_BINDGEN="$(command -v wasm-bindgen || echo "${WASM_BINDGEN}")"

echo "── 1/5 抽取单一真源资产 ──"
node "${HERE}/scripts/extract-assets.mjs"
# 工具卡面批二（工具栏 + 工具网格）：`runtime` 替身 + 本面资产（区块级 CSS + 图标 + 固定数据）。
node "${HERE}/scripts/gen-runtime-mock.mjs"
node "${HERE}/scripts/extract-assets.mjs" --manifest "${HERE}/faces/toolgrid.json"
node "${HERE}/scripts/extract-toolgrid.mjs"

echo "── 2/5 构建 wasm 产物 ──"
(cd "${CRATES_UI}" && cargo build -p apeireth-ui-demo --bin apeireth-ui-demo --target wasm32-unknown-unknown --release --locked)

echo "── 3/5 wasm-bindgen ──"
rm -rf "${HERE}/public/engine/pkg"
mkdir -p "${HERE}/public/engine/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}/public/engine/pkg" \
  "${CRATES_UI}/target/wasm32-unknown-unknown/release/apeireth-ui-demo.wasm"

echo "── 4/5 vite build ──"
if [ ! -d "${HERE}/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的只读依赖目录）"
  ln -s "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/node_modules"
fi
(cd "${HERE}" && npx --no-install vite build)

echo "── 5/5 对照（像素 + 逐帧） ──"
# 试点（审批卡）对照。
python3 "${HERE}/tools/compare.py"
# 工具卡面对照（两张卡 + JSON 差分）。
python3 "${HERE}/tools/compare_toolcard.py"
# 工具卡面批二对照（工具栏 + 工具网格）。
python3 "${HERE}/tools/compare_toolgrid.py"
