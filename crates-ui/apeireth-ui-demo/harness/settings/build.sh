#!/usr/bin/env bash
# ============================================================================
# 设置页三页对照 harness 的构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 生成两侧入口页（按 `web/shared/panels.js` 的子面注册表）
#   2. 生成数据面替身（按生产的导出面）
#   3. 抽取单一真源资产（按子面分片的 scoped CSS + 共享设计层 CSS）
#   4. 构建 wasm 产物（release）
#   5. wasm-bindgen ⇒ JS 胶水 + wasm（放进本目录的 public/engine/pkg/）
#   6. vite build ⇒ 两侧页面（每面一页）
#   7. 跑对照（像素 + DOM 全等 + 宽度同源）
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、node 22 + vite、
# python3（websocket-client + Pillow）、系统 Chromium。
#
# ⚠️ **node 必须是 22**：本机默认 node 是 v24 ⇒ 前端/vite 相关必须显式用 22。
#    （可用 `NODE_BIN=<路径>` 覆盖。）
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# ⚠️ **层数**（并行面 M4 实测发现，2026-10-24）：本目录是 `harness/settings/`
# ⇒ `..`=harness/ ⇒ `../..`=apeireth-ui-demo/ ⇒ `../../..`=**crates-ui/**
# ⇒ `../../../..`=**仓库根**。
# 原先少一级 ⇒ `REPO` 落到 `crates-ui/` ⇒ `frontend/companion-desktop/node_modules`
# 指向**不存在的路径** ⇒ vite 找不到 ⇒ 构建失败。
CRATES_UI="$(cd "${HERE}/../../.." && pwd)"
REPO="$(cd "${HERE}/../../../.." && pwd)"

#: node（默认取 PATH 上的；⚠️ 必须是 22）。
NODE_BIN="${NODE_BIN:-node}"
if ! command -v "${NODE_BIN}" >/dev/null 2>&1; then
  echo "找不到 node（可用 NODE_BIN=<路径> 指定；本批记录：node 22 在 /tmp/n22/node_modules/node-linux-x64/bin/node）" >&2
  exit 1
fi

WASM_BINDGEN="${WASM_BINDGEN:-${REPO}/target/ui-probe-scratch/bin/bin/wasm-bindgen}"
if ! command -v wasm-bindgen >/dev/null 2>&1 && [ ! -x "${WASM_BINDGEN}" ]; then
  echo "找不到 wasm-bindgen（可用 WASM_BINDGEN=<路径> 指定）" >&2
  exit 1
fi
WASM_BINDGEN="$(command -v wasm-bindgen || echo "${WASM_BINDGEN}")"

echo "── 1/7 生成两侧入口页 ──"
"${NODE_BIN}" "${HERE}/scripts/gen-entry-pages.mjs"

echo "── 2/7 生成数据面替身 ──"
"${NODE_BIN}" "${HERE}/scripts/gen-settings-mock.mjs"

echo "── 3/7 抽取单一真源资产 ──"
"${NODE_BIN}" "${HERE}/scripts/extract-settings-assets.mjs"

echo "── 4/7 构建 wasm 产物 ──"
(cd "${CRATES_UI}" && cargo build -p apeireth-ui-demo --bin apeireth-ui-demo --target wasm32-unknown-unknown --release --locked)

echo "── 5/7 wasm-bindgen ──"
rm -rf "${HERE}/public/engine/pkg"
mkdir -p "${HERE}/public/engine/pkg"
# ⚠️ wasm 产物的位置**取决于 cargo 的 target 目录**：
# 默认 = **工作区** target（`${CRATES_UI}/target`）；若外部设了 `CARGO_TARGET_DIR`
# （并行施工时为了不抢别的线的 target 目录）⇒ 产物在那里 ⇒ 路径必须跟着走。
TARGET_DIR="${CARGO_TARGET_DIR:-${CRATES_UI}/target}"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}/public/engine/pkg" \
  "${TARGET_DIR}/wasm32-unknown-unknown/release/apeireth-ui-demo.wasm"

echo "── 6/7 vite build ──"
# ⚠️ 用 `-e`（存在性）而不是 `-d`（是目录）⇒ **坏链接**也能被识别并重建
# ⇒ 链接目标不存在时 `-d` 为假但 `-e` 为真 ⇒ `-d` 的写法会把坏链接一直留着。
if [ ! -e "${HERE}/node_modules" ] || [ ! -d "${HERE}/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的只读依赖目录）"
  ln -sfn  "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/node_modules"
fi
(cd "${HERE}" && npx --no-install vite build)

echo "── 7/7 对照（像素 + DOM 全等 + 宽度同源） ──"
# ⚠️ 本批**没有迁移** ⇒ 没有引擎侧产物 ⇒ 对照无对象 ⇒ 本步会**显式报未接线**
#    （不是静默通过）。迁移线接线后，这一步产出 `out/` 与 `evidence/`。
python3 "${HERE}/tools/compare_settings.py" --all
