#!/usr/bin/env bash
# ============================================================================
# 状态条面对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 抽取单一真源资产（组件 scoped CSS / 设计令牌 CSS / 图标数据 / 固定数据）
#   2. 构建 wasm 产物（release；**独立 crate**，不进 crates-ui 依赖图）
#   3. wasm-bindgen ⇒ JS 胶水 + wasm（放进 harness 的 public/engine/pkg/）
#   4. vite build ⇒ 两个页面（参考侧 / 引擎侧）+ 静态资产
#   5. 跑对照（像素 + DOM 全等 + 逐帧 + 点击回调证据）
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、node + vite、python3
# （websocket-client + Pillow）、系统 Chromium。
#
# ⚠️ **本 harness 自带 vite 配置与 wasm 宿主 crate** ⇒ 不碰共享的
#    `harness/vite.config.mjs` 与 `harness/scripts/`（共享点）。
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# ⚠️ 相对层数：`harness/statusbar/` → `apeireth-ui-demo/` → `crates-ui/` → **仓库根**
# ⇒ 四层（`../../../..`）。
REPO="$(cd "${HERE}/../../../.." && pwd)"
HOST="${HERE}/engine-host"

WASM_BINDGEN="${WASM_BINDGEN:-${REPO}/target/ui-probe-scratch/bin/bin/wasm-bindgen}"
if ! command -v wasm-bindgen >/dev/null 2>&1 && [ ! -x "${WASM_BINDGEN}" ]; then
  echo "找不到 wasm-bindgen（可用 WASM_BINDGEN=<路径> 指定）" >&2
  exit 1
fi
WASM_BINDGEN="$(command -v wasm-bindgen || echo "${WASM_BINDGEN}")"

# 构建目录（默认独立 ⇒ 不与其它并行线抢同一个 target 锁）。
TARGET_DIR="${STATUSBAR_TARGET_DIR:-${REPO}/target/statusbar-scratch/wasm}"

echo "── 1/5 抽取单一真源资产 ──"
node "${HERE}/scripts/extract-statusbar-assets.mjs"

echo "── 2/5 构建 wasm 产物（独立 crate） ──"
(cd "${HOST}" && CARGO_TARGET_DIR="${TARGET_DIR}" cargo build --release --target wasm32-unknown-unknown)

echo "── 3/5 wasm-bindgen ──"
rm -rf "${HERE}/public/engine/pkg"
mkdir -p "${HERE}/public/engine/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}/public/engine/pkg" \
  "${TARGET_DIR}/wasm32-unknown-unknown/release/statusbar_harness.wasm"

echo "── 4/5 vite build ──"
if [ ! -d "${HERE}/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的只读依赖目录）"
  ln -s "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/node_modules"
fi
(cd "${HERE}" && npx --no-install vite build)

echo "── 5/5 对照（像素 + DOM 全等 + 逐帧 + 点击回调） ──"
python3 "${HERE}/tools/compare_statusbar.py"
