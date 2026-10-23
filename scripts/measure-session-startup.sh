#!/usr/bin/env bash
# ============================================================================
# measure-session-startup.sh — `apeireth session` 启动到 ready 全进程实测（跨平台）
# ----------------------------------------------------------------------------
# 与 measure-session-startup.ps1 同口径、同输出形态；本脚本面向 macOS / Linux，
# 让"唯一会话启动测量脚本是 Windows-only"这件事不再成立。
#
# 口径: 全进程墙钟 = 进程创建 → 组装到 ready（打印 `canonical runtime ready`）→
#       退出。含 tokio 运行时创建与进程退出（比 cold-start 基准口径宽，如实标注）。
# 形态: 真冷（每轮全新数据目录）与热开（同数据目录重复启动）两组各 N 轮。
# 前置: cargo build --release --locked -p apeireth-cli
# 用法: scripts/measure-session-startup.sh [-n RUNS] [-e EXE]
# 输出: 每组 min/mean/P50/max + 逐轮原始毫秒值；退出码 0 = 全部启动成功。
# 纪律: 数字可复核（本脚本 + 日期 + 工具链，见输出抬头）。
# ============================================================================
set -euo pipefail

RUNS=12
EXE=""
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

while getopts "n:e:" opt; do
  case "$opt" in
    n) RUNS="$OPTARG" ;;
    e) EXE="$OPTARG" ;;
    *) echo "usage: $0 [-n RUNS] [-e EXE]" >&2; exit 2 ;;
  esac
done

if [[ -z "$EXE" ]]; then
  for candidate in "$REPO_ROOT/target/release/apeireth" "$REPO_ROOT/target/release/apeireth.exe"; do
    if [[ -x "$candidate" ]]; then EXE="$candidate"; break; fi
  done
fi
if [[ -z "$EXE" || ! -x "$EXE" ]]; then
  echo "FAIL  缺 target/release/apeireth —— 先跑 cargo build --release --locked -p apeireth-cli" >&2
  exit 2
fi

# 毫秒级墙钟：优先 GNU date（%N），macOS 的 BSD date 无 %N → 退到 python3（系统自带）。
now_ms() {
  if date +%s%3N >/dev/null 2>&1 && [[ "$(date +%s%3N)" != *N* ]]; then
    date +%s%3N
  else
    python3 -c 'import time; print(int(time.time()*1000))'
  fi
}

BASE="$(mktemp -d "${TMPDIR:-/tmp}/apeireth-session-startup-XXXXXX")"
cleanup() { rm -rf "$BASE"; }
trap cleanup EXIT

echo "# apeireth session 启动实测（scripts/measure-session-startup.sh）"
echo "- 时间: $(date '+%Y-%m-%d %H:%M:%S %Z') / 工具链: $(rustc --version 2>/dev/null || echo unknown) / 仓库版本: $(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown) / 轮数: $RUNS"
echo "- 口径: 全进程墙钟（进程创建 → ready → 退出，含 tokio 运行时创建）"
echo "- 平台: $(uname -sm)"
echo ""

measure_session() {
  local dir="$1"
  mkdir -p "$dir"
  local start end out
  start="$(now_ms)"
  out="$(APEIRETH_DATA_DIR="$dir" \
         APEIRETH_SESSION_DB="$dir/sessions.sqlite3" \
         APEIRETH_COGNITIVE_DB="$dir/cognitive.sqlite3" \
         "$EXE" session 2>&1)" || {
    echo "FAIL  apeireth session 启动失败: $out" >&2
    return 1
  }
  end="$(now_ms)"
  if [[ "$out" != *"canonical runtime ready"* ]]; then
    echo "FAIL  启动输出未见 ready 行: $out" >&2
    return 1
  fi
  echo $(( end - start ))
}

stats() {
  local label="$1" note="$2"
  shift 2
  local values=("$@")
  printf '%s' "$label: " | cat
  python3 - "$label" "$note" "${values[@]}" <<'PY'
import sys
label, note, *raw = sys.argv[1:]
vals = sorted(float(v) for v in raw)
n = len(vals)
p50 = vals[int((n - 1) * 0.5)]
print(f"n={n} min={vals[0]:.2f} mean={sum(vals)/n:.2f} P50={p50:.2f} max={vals[-1]:.2f} ms ({note})")
print("  raw: " + " ".join(f"{v:.2f}" for v in vals))
PY
}

cold=()
for ((i = 0; i < RUNS; i++)); do
  cold+=("$(measure_session "$BASE/cold-$i")")
done
stats "真冷（每轮全新数据目录）" "A组" "${cold[@]}"

warm_dir="$BASE/warm"
mkdir -p "$warm_dir"
warm=()
for ((i = 0; i < RUNS; i++)); do
  warm+=("$(measure_session "$warm_dir")")
done
stats "热开（同库重复启动）" "B组" "${warm[@]}"

exit 0
