#!/usr/bin/env bash
# ============================================================================
# P5-b 生产对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 构建 wasm 宿主（release，wasm32）
#   2. wasm-bindgen ⇒ JS 胶水 + wasm（放进 web/public/engine/pkg/）
#   3. vite build ⇒ 参考页（生产场景）+ 引擎页
#   4. 跑 CDP 对照（生产侧抓取值 → 引擎侧渲染 → 逐像素差分）
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、node + vite、python3
# （websocket-client + Pillow）、系统 Chromium。
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "${HERE}/../../.." && pwd)"

# wasm-bindgen CLI 的版本必须与 crates-ui 的 wasm-bindgen 依赖**精确一致**（0.2.129）。
WASM_BINDGEN="${WASM_BINDGEN:-}"
# ⚠️ **产物路径必须跟随 `CARGO_TARGET_DIR`**（仓库脚本纪律）：
# 否则外部设了 `CARGO_TARGET_DIR` 时 cargo 把产物写到别处，
# 而本脚本仍去读硬编码的 `…/target/…` ⇒ ⇒ **拿到旧 wasm**
# ⇒ ⇒ 实测踩过：注入的修复**没进浏览器** ⇒ ⇒ **假绿/假红**。
TARGET_DIR="${CARGO_TARGET_DIR:-${HERE}/harness/target}"
WASM_ARTIFACT="${TARGET_DIR}/wasm32-unknown-unknown/release/p5b_harness.wasm"
if [ ! -f "${WASM_ARTIFACT}" ]; then
  echo "找不到 wasm 产物：${WASM_ARTIFACT}" >&2
  exit 1
fi
if [ -z "${WASM_BINDGEN}" ]; then
  for candidate in \
    "$(command -v wasm-bindgen || true)" \
    "${REPO}/target/ui-probe-scratch/bin/bin/wasm-bindgen"; do
    if [ -n "${candidate}" ] && [ -x "${candidate}" ]; then
      WASM_BINDGEN="${candidate}"
      break
    fi
  done
fi
if [ -z "${WASM_BINDGEN}" ]; then
  echo "找不到 wasm-bindgen（可用 WASM_BINDGEN=<路径> 指定）" >&2
  exit 1
fi
echo "   wasm-bindgen: ${WASM_BINDGEN} ($("${WASM_BINDGEN}" --version))"

echo "── 1/4 构建 wasm 宿主 ──"
(cd "${HERE}/harness" && cargo build --release --target wasm32-unknown-unknown --locked)

echo "── 2/4 wasm-bindgen ──"
rm -rf "${HERE}/web/public/engine/pkg"
mkdir -p "${HERE}/web/public/engine/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}/web/public/engine/pkg" \
  "${WASM_ARTIFACT}"

echo "── 3/4 vite build ──"
# ⚠️ 用 `-e`（存在性）**加上** `-d`（是目录）两个条件 ⇒ **坏链接**也能被识别并重建。
# ⇒ 只判 `-d` 时：坏链接 ⇒ `-d` 为假但 `-e` 为真 ⇒ 链接**永远不会被重建**
# ⇒ ⇒ `npx vite` 找不到本地 vite ⇒ 想联网装 ⇒ 离线/受限环境里失败
# ⇒ （设置页 harness 的 build.sh 已按此口径修过；此处是同一处坑的另一份拷贝。）
if [ ! -e "${HERE}/web/node_modules" ] || [ ! -d "${HERE}/web/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的只读依赖目录）"
  rm -f "${HERE}/web/node_modules"
  ln -s "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/web/node_modules"
fi
(cd "${HERE}/web" && npx --no-install vite build)

echo "── 4/4 生产对照（逐像素） ──"
python3 "${HERE}/drive.py" --out "${HERE}/evidence" --web "${HERE}/web/dist"
