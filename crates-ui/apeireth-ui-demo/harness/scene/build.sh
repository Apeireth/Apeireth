#!/usr/bin/env bash
# ============================================================================
# 场景层语义对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 构建 wasm 宿主（release，wasm32）
#   2. wasm-bindgen ⇒ JS 胶水 + wasm（放进 web/public/engine/pkg/）
#   3. vite build ⇒ 参考页（生产场景）+ 引擎页
#   4. 跑 CDP 对照（生产侧抓取值 → 引擎侧自算取值 → 逐值 + 逐像素）
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、node + vite、python3
# （websocket-client + Pillow）、系统 Chromium。
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# ⚠️ 仓库根 = scene → harness → apeireth-ui-demo → crates-ui → 仓库根（4 级）。
REPO="$(cd "${HERE}/../../../.." && pwd)"

# ⚠️ 前端工具链必须用 Node 22（本机固定在 /tmp/n22）。
if [ -d /tmp/n22/node_modules/node-linux-x64/bin ]; then
  export PATH="/tmp/n22/node_modules/node-linux-x64/bin:${PATH}"
fi

# wasm-bindgen CLI 的版本必须与 crates-ui 的 wasm-bindgen 依赖**精确一致**（0.2.129）。
WASM_BINDGEN="${WASM_BINDGEN:-}"
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

# ⚠️ 产物目录跟随 CARGO_TARGET_DIR（并行线各自用独立 target 目录）。
TARGET_DIR="${CARGO_TARGET_DIR:-${HERE}/harness/target}"
WASM_FILE="${TARGET_DIR}/wasm32-unknown-unknown/release/scene_harness.wasm"
if [ ! -f "${WASM_FILE}" ]; then
  echo "找不到 wasm 产物：${WASM_FILE}" >&2
  exit 1
fi

echo "── 2/4 wasm-bindgen ──"
rm -rf "${HERE}/web/public/engine/pkg"
mkdir -p "${HERE}/web/public/engine/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}/web/public/engine/pkg" \
  "${WASM_FILE}"

echo "── 3/4 vite build ──"
if [ ! -d "${HERE}/web/node_modules" ]; then
  echo "   （无 node_modules ⇒ 复用生产前端的**只读**依赖目录）"
  ln -s "${REPO}/frontend/companion-desktop/node_modules" "${HERE}/web/node_modules"
fi
(cd "${HERE}/web" && npx --no-install vite build)

echo "── 4/4 生产对照（逐值 + 逐像素） ──"
python3 "${HERE}/drive.py" --out "${HERE}/../evidence/scene" --web "${HERE}/web/dist"
