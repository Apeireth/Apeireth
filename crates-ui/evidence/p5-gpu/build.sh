#!/usr/bin/env bash
# ============================================================================
# P5 GPU 对照 harness 的完整构建（**一键复跑**）。
# ----------------------------------------------------------------------------
# 步骤：
#   1. 构建 wasm 宿主（release，wasm32）
#   2. wasm-bindgen ⇒ JS 胶水 + wasm
#   3. 跑 CDP 对照（两个后端各出画布 → 逐像素差分）
#
# ⚠️ **产物必须落在 `evidence/p5-gpu/` 本目录**：`index.html` 按**同目录相对路径**
#    引用 `./p5_gpu_harness.js`，而 `drive.py` 服务的正是本目录 ⇒ 放到别处
#    （如 `pkg/`）会得到 404，且**画面判据会静默变 null**（看起来像「渲染失败」，
#    实际是「文件没被找到」）。这个坑本脚本固化了。
#
# 依赖：cargo（wasm32 target）、wasm-bindgen-cli 0.2.129、python3
# （websocket-client + Pillow）、系统 Chromium。
# ============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "${HERE}/../../.." && pwd)"

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

echo "── 1/3 构建 wasm 宿主 ──"
(cd "${HERE}/harness" && cargo build --release --target wasm32-unknown-unknown --locked)

echo "── 2/3 wasm-bindgen（产物落在本目录） ──"
rm -f "${HERE}/p5_gpu_harness.js" "${HERE}/p5_gpu_harness_bg.wasm"
rm -rf "${HERE}/pkg"
"${WASM_BINDGEN}" --target web --no-typescript \
  --out-dir "${HERE}" \
  "${HERE}/harness/target/wasm32-unknown-unknown/release/p5_gpu_harness.wasm"

echo "── 3/3 GPU 对照（两个后端 → 逐像素差分） ──"
# ⚠️ `--out` **必须显式指到本目录**：`drive.py` 的默认是 `${HERE}/evidence`（子目录），
# 而本批证据的**跟踪布局**在**本目录根部**（`report.json` / `*.canvas.png` …）——
# 两者不一致会让重跑产物落到**另一个目录** ⇒ 出现「跟踪的旧证据 + 未跟踪的新证据」两份。
# （P5-b 用的是 `evidence/` 子目录布局；两批布局不同，此处以**已跟踪的布局**为准。）
python3 "${HERE}/drive.py" --out "${HERE}"
