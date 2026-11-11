#!/usr/bin/env bash
# ==============================================================================
# publish-engine-crates.sh — 引擎 crate 逐个发布到 crates.io（ADR-0017 S2b）
# ------------------------------------------------------------------------------
# 作用:
#   按**依赖拓扑序**把引擎仓的 crate 逐个发布到 crates.io。三个环节:
#     G2  cargo package -p <c> --no-verify   拦「path 依赖缺 version / 依赖未上架 /
#                                             dev-dep 成环」三类发布阻塞
#     G3  cargo publish -p <c> --dry-run      联网, 含 build verify
#     发布 cargo publish -p <c>               真发
#   顺序**必须**逐个推进 —— G2/G3 会把 path 依赖改写成 registry 依赖并去**索引解析**,
#   先发的 crate 必须已在 crates.io 上, 后面的才解析得到。
#
# 关联:
#   决策真源 = docs/adr/0017-engine-product-repo-split.md（Accepted 2026-10-31）
#     §2.5 消费机制 / 附录 C.1 候选与占名 / C.3 B2 裁决 / C.4 拓扑序 + 三段门禁 / C.6 认证
#   手册 = docs/04-internal/engine-publish-runbook.md（owner 侧账号与 token 步骤）
#
# 用法:
#   scripts/publish-engine-crates.sh --print-order          # 只打印拓扑序（离线）
#   scripts/publish-engine-crates.sh --check                 # 离线彩排自检（不联网, 不发）
#   scripts/publish-engine-crates.sh --dry-run-only          # G2+G3 dry-run（不真发）
#   scripts/publish-engine-crates.sh --from apeireth-core    # 从某个 crate 继续（续跑）
#   scripts/publish-engine-crates.sh --allow-dirty            # 允许工作树不干净（慎用）
#   scripts/publish-engine-crates.sh --root-only              # 只发根 workspace（跳过 crates-ui）
#   scripts/publish-engine-crates.sh --crates-ui-only         # 只发 crates-ui 独立 workspace
#
# 环境:
#   CARGO_REGISTRY_TOKEN   crates.io API token（**必填**, 缺失即中止）
#   其它可选: CARGO_REGISTRY=crates-io（默认）, PUBLISH_LOG_DIR=logs,
#             INDEX_RETRY=5（索引传播重试次数）, INDEX_RETRY_WAIT=30（秒）
#
# 0 装诚实边界:
#   - 本脚本**不创建** crates.io 账号、不生成 token: 那是 owner 侧动作（见 runbook）。
#   - crates.io 上已发布的版本**不可删除**, 只能 yank ⇒ 对每个 crate 只发一次,
#     失败即停, 不重试覆盖。
#   - 工作树不干净时**默认拒绝**: 发布出去的源码会与 git 提交状态不一致。
#   - 本机 ~/.cargo/config.toml 用镜像替换了 crates-io ⇒ 脚本强制带
#     `--registry crates-io`（否则 cargo 报"像网络错"的错, 实测报
#     "location searched: rsproxy-sparse index (which is replacing registry crates-io)"）。
#   - **彩排不等价于真实发布**: --check 用**命令行临时 patch**把尚未发布的内部 crate
#     指回本仓源码, 以便在发布前跑通整条链; 真实发布时依赖必须已在注册表。
#     该 patch **不落盘**（不进任何配置文件, 不进版本库）。
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

REGISTRY="${CARGO_REGISTRY:-crates-io}"
PUBLISH_LOG_DIR="${PUBLISH_LOG_DIR:-logs}"
CRATES_UI_DIR="crates-ui"
# 版本轴**唯一真源** = 根 Cargo.toml [workspace.package].version（LOCKED，release authority 改）
# ⇒ 本脚本不硬编码版本号, 随轴自动跟随（避免"bump 后忘了改脚本"的坑）。
WS_VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml | grep -m1 '^version' | sed 's/.*= *"\([^"]*\)".*/\1/')"
if [[ -z "$WS_VERSION" ]]; then
  echo "✗ 读不到根 Cargo.toml [workspace.package].version" >&2; exit 9
fi
INDEX_RETRY="${INDEX_RETRY:-5}"
INDEX_RETRY_WAIT="${INDEX_RETRY_WAIT:-30}"

MODE="publish"           # publish | check | dry-run-only | print-order
FROM_CRATE=""
ALLOW_DIRTY=0
ROOT_ONLY=0              # --root-only: 只发根 workspace（跳过 crates-ui）
UI_ONLY=0                # --crates-ui-only: 只发 crates-ui 独立 workspace（跳过根 workspace）

usage() { sed -n '2,45p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --print-order)   MODE="print-order" ;;
    --check)         MODE="check" ;;
    --dry-run-only)  MODE="dry-run-only" ;;
    --from)          FROM_CRATE="$2"; shift ;;
    --allow-dirty)   ALLOW_DIRTY=1 ;;
    --root-only)     ROOT_ONLY=1 ;;
    --crates-ui-only) UI_ONLY=1 ;;
    -h|--help)       usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

# --- 前置检查 ---------------------------------------------------------------
if [[ "$MODE" != "print-order" && "$MODE" != "check" ]]; then
  # 两种凭据环境变量都接受（Cargo Book: cargo:token provider 读 CARGO_REGISTRIES_<NAME>_TOKEN）
  if [[ -z "${CARGO_REGISTRY_TOKEN:-}" && -z "${CARGO_REGISTRIES_CRATES_IO_TOKEN:-}" ]]; then
    echo "✗ 缺 crates.io token。设下面任一即可：" >&2
    echo "    export CARGO_REGISTRY_TOKEN='<token>'          # 默认 registry" >&2
    echo "    export CARGO_REGISTRIES_CRATES_IO_TOKEN='<token>'  # 具名 registry crates-io" >&2
    echo "  或先跑 cargo login --registry crates-io（写入 ~/.cargo/credentials.toml）" >&2
    echo "  获取步骤见 docs/04-internal/engine-publish-runbook.md §2。" >&2
    exit 3
  fi
fi

if [[ "$MODE" == "publish" && $ALLOW_DIRTY -eq 0 ]]; then
  if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    echo "✗ 工作树不干净, 已中止（发布内容会与 git 状态不一致）。" >&2
    echo "  处理: 提交/暂存后重跑, 或加 --allow-dirty 并知情。" >&2
    exit 4
  fi
fi

# --- 拓扑序 + 绝对路径（由 cargo metadata 现算，含 dev/build 依赖约束）--------
# 输出格式: "<crate>\t<绝对路径>"
compute_order() {
  local manifest="$1" filter="$2"
  python3 - "$manifest" "$filter" <<'PY'
import json, subprocess, sys, collections, os
manifest, filt = sys.argv[1], sys.argv[2]
out = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline",
                      "--manifest-path", manifest], capture_output=True, text=True)
if out.returncode != 0:
    sys.stderr.write(out.stderr[-2000:]); sys.exit(1)
m = json.loads(out.stdout)
pkgs = {p["name"]: p for p in m["packages"]}
if filt == "root":
    # 取**交集**：清单里的 crate 必须真实存在于本 workspace。
    # （发布用 worktree 可能基于较早的 HEAD, 不含后加的 crate（如门面）⇒ 不能硬取全集）
    want = {"apeireth-core","apeireth-protocol","apeireth-plugin","apeireth-governance",
            "apeireth-credentials","apeireth-orchestration","apeireth-runtime",
            "apeireth-runtime-assembly","apeireth-provider","apeireth-storage",
            "apeireth-memory","apeireth-organ","apeireth-guard","apeireth-tools-canonical",
            "apeireth"} \
           | {n for n in pkgs if n.startswith("apeireth-image-")}
    pub = {c for c in want if c in pkgs}
else:
    pub = {n for n in pkgs if n.startswith("apeireth-ui")} - {"apeireth-ui-demo"}
def deps(n):
    r = set()
    for d in pkgs[n]["dependencies"]:
        if d["name"] in pkgs and d["name"] != n and d["name"] in pub:
            r.add(d["name"])           # 含 dev/build: 实测 dev-dep 也是发布顺序约束
    return r
indeg = {n: len(deps(n)) for n in pub}
rev = collections.defaultdict(set)
for n in pub:
    for d in deps(n): rev[d].add(n)
ready = sorted(n for n in pub if indeg[n] == 0); order = []
while ready:
    n = ready.pop(0); order.append(n)
    for x in sorted(rev[n]):
        indeg[x] -= 1
        if indeg[x] == 0: ready.append(x); ready.sort()
cyc = [n for n in pub if indeg[n] > 0]
if cyc:
    sys.stderr.write("✗ 依赖成环, 无可发布顺序: " + ", ".join(sorted(cyc)) + "\n")
    sys.stderr.write("  成因: dev-dependency 也是发布顺序约束（ADR-0017 附录 C.2 B3）\n")
    sys.exit(5)
for n in order:
    d = os.path.dirname(pkgs[n]["manifest_path"])
    print(f"{n}\t{d}")
PY
}

# root 顺序**按需计算**: --crates-ui-only 时不需要（且发布用 worktree 可能基于较早 HEAD,
# 那时 root 侧仍有 B3 dev-dep 环 ⇒ 算 root 会直接失败, 与本次发布无关）。
if [[ $UI_ONLY -eq 1 ]]; then
  ORDER_ROOT=""
else
  ORDER_ROOT="$(compute_order Cargo.toml root)"
fi
ORDER_UI="$(compute_order "$CRATES_UI_DIR/Cargo.toml" ui)"

# --- 彩排用临时 patch（不落盘）---------------------------------------------
patch_flags() {
  # 把整份内部 crate 列表指回本仓源码（模拟"依赖已在注册表"）。
  # 只在 --check（离线彩排）模式使用；真实发布**不用** patch。
  local order="$1"
  while IFS=$'\t' read -r crate dir; do
    [[ -z "$crate" ]] && continue
    printf '%s ' "--config" "patch.crates-io.${crate}.path=\"${dir}\""
    printf '%s ' "--config" "patch.crates-io.${crate}.version=\"${WS_VERSION}\""
  done <<< "$order"
}

if [[ "$MODE" == "print-order" ]]; then
  echo "=== 根 workspace 发布顺序（crate + 本地路径）==="
  i=0; while IFS=$'\t' read -r c d; do echo "  $i $c  ($d)"; i=$((i+1)); done <<< "$ORDER_ROOT"
  echo
  echo "=== crates-ui 独立 workspace 发布顺序 ==="
  i=0; while IFS=$'\t' read -r c d; do echo "  $i $c  ($d)"; i=$((i+1)); done <<< "$ORDER_UI"
  exit 0
fi

mkdir -p "$PUBLISH_LOG_DIR"
# ⚠️ LOG 必须**绝对路径**: publish_one 会 cd 进各 workspace 目录, 相对路径会写错地方
LOG="$REPO_ROOT/$PUBLISH_LOG_DIR/publish-engine-crates-$(date +%Y%m%d-%H%M%S).log"
echo "日志: $LOG"
echo "版本轴: $WS_VERSION（取自根 Cargo.toml [workspace.package].version）"

# G2: cargo package --no-verify
#   - 真实发布模式：**联网**（要能从索引解析到刚发布的依赖）, 失败按"索引未传播"重试
#   - --check 模式：**离线** + 临时 patch（模拟依赖已发布）
run_g2() {
  local wsdir="$1" crate="$2"
  local attempt=1
  while true; do
    if ( cd "$wsdir" && cargo package -p "$crate" --no-verify --allow-dirty 2>&1 | tee -a "$LOG" ); then
      return 0
    fi
    if [[ "$attempt" -ge $INDEX_RETRY ]]; then
      return 1
    fi
    echo "… G2 失败, 可能是 crates.io 索引尚未传播（刚发布的依赖还查不到）; " \
         "$((INDEX_RETRY_WAIT))s 后重试（$attempt/$INDEX_RETRY）" | tee -a "$LOG"
    sleep "$INDEX_RETRY_WAIT"
    attempt=$((attempt+1))
  done
}

publish_one() {
  local wsdir="$1" crate="$2"
  local rehearsal=0
  [[ "$MODE" == "check" ]] && rehearsal=1
  # 彩排用的 patch **只限本 workspace 内部 crate** —— 跨 workspace 的 path patch
  # 会让被 patch 包的 `workspace = true` 继承解析失败（实测: "error inheriting
  # `apeireth-ui-core` … not found in `workspace.dependencies`"）⇒ 必须分 workspace 给。
  local wsorder="$ORDER_ROOT"
  [[ "$wsdir" == "$CRATES_UI_DIR" ]] && wsorder="$ORDER_UI"

  echo "──────── $crate ────────" | tee -a "$LOG"

  if [[ $rehearsal -eq 1 ]]; then
    echo "[G2*] cargo package --no-verify --offline（离线彩排: 临时 patch 模拟依赖已发布）" | tee -a "$LOG"
    # shellcheck disable=SC2046
    if ! ( cd "$wsdir" && cargo package -p "$crate" --no-verify --allow-dirty --offline \
          $(patch_flags "$wsorder") 2>&1 | tee -a "$LOG" ); then
      echo "✗ G2 彩排失败 ⇒ 中止" | tee -a "$LOG"; exit 6
    fi
    echo "✓ G2 彩排通过（不发布）" | tee -a "$LOG"
    return 0
  fi

  echo "[G2] cargo package --no-verify（联网: 依赖须已在注册表）" | tee -a "$LOG"
  if ! run_g2 "$wsdir" "$crate"; then
    echo "✗ G2 失败 ⇒ 中止。" | tee -a "$LOG"
    echo "  常见成因: ① 跳过了一步（依赖尚未发布）② 该 crate 的依赖解析不到。" | tee -a "$LOG"
    exit 6
  fi

  if [[ "$MODE" == "dry-run-only" ]]; then
    echo "[G3] cargo publish --dry-run --registry $REGISTRY" | tee -a "$LOG"
    if ! ( cd "$wsdir" && cargo publish -p "$crate" --dry-run --registry "$REGISTRY" --allow-dirty \
          2>&1 | tee -a "$LOG" ); then
      echo "✗ G3 失败 ⇒ 中止（未发布任何内容）" | tee -a "$LOG"; exit 7
    fi
    echo "✓ G3 通过（dry-run-only 模式, 不发布）" | tee -a "$LOG"
    return 0
  fi

  echo "[发布] cargo publish --registry $REGISTRY" | tee -a "$LOG"
  local attempt=1 rc out
  while true; do
    # ⚠️ set -e 下不能写 `out="$(cmd)"; rc=$?`（失败会先退出、错误被吞）
    rc=0
    out="$( cd "$wsdir" && cargo publish -p "$crate" --registry "$REGISTRY" --allow-dirty 2>&1 )" || rc=$?
    printf '%s\n' "$out" | tee -a "$LOG"
    if [[ $rc -eq 0 ]]; then break; fi
    # 幂等续跑：该版本已上传 ⇒ 视为成功跳过（crates.io 不可覆盖，重跑必须安全）
    if printf '%s' "$out" | grep -qiE "already (exists|uploaded|been published)|crate version .* is already"; then
      echo "… 该版本已存在 ⇒ 跳过（幂等续跑）" | tee -a "$LOG"; break
    fi
    # 传输层瞬时错误（上传端点 408 超时 / 5xx / 连接重置）⇒ 退避重试
    if printf '%s' "$out" | grep -qiE "408|failed to get a 200 OK|timed? out|connection reset|connection closed|502|503|504|stream error|HTTP/2 framing|STREAM_CLOSED|broken pipe|early eof" && [[ $attempt -lt ${NET_RETRY:-6} ]]; then
      echo "… 传输层瞬时错误（非限速）, ${NET_WAIT:-25}s 后重试（$attempt/${NET_RETRY:-6}）" | tee -a "$LOG"
      sleep "${NET_WAIT:-25}"; attempt=$((attempt+1)); continue
    fi
    # crates.io 新 crate 限速 ⇒ 429 / too many requests 时退避重试
    if printf '%s' "$out" | grep -qiE "429|too many requests|rate limit" && [[ $attempt -lt ${RATE_RETRY:-10} ]]; then
      echo "… 触发限速, ${RATE_WAIT:-70}s 后重试（$attempt/${RATE_RETRY:-10}）" | tee -a "$LOG"
      sleep "${RATE_WAIT:-70}"; attempt=$((attempt+1)); continue
    fi
    echo "✗ 发布失败 ⇒ 中止。crates.io 已发布版本不可删除, 只能 yank。" | tee -a "$LOG"
    exit 8
  done
  # crates.io 对新 crate 有速率限制 ⇒ 相邻两个之间留间隔（可用 PUBLISH_INTERVAL 调整）
  sleep "${PUBLISH_INTERVAL:-20}"
  echo "✓ 已发布 $crate" | tee -a "$LOG"
}

started=0
echo "=== 开始（mode=$MODE, registry=$REGISTRY）===" | tee -a "$LOG"
if [[ $UI_ONLY -eq 1 ]]; then
  echo "=== --crates-ui-only: 跳过根 workspace ===" | tee -a "$LOG"
else
while IFS=$'\t' read -r crate dir; do
  [[ -z "$crate" ]] && continue
  if [[ -n "$FROM_CRATE" && $started -eq 0 ]]; then
    if [[ "$crate" == "$FROM_CRATE" ]]; then started=1; else continue; fi
  fi
  publish_one "$REPO_ROOT" "$crate"
done <<< "$ORDER_ROOT"
fi

if [[ $ROOT_ONLY -eq 1 ]]; then
  echo "=== --root-only: 跳过 crates-ui 6 个库 crate ===" | tee -a "$LOG"
else
  while IFS=$'\t' read -r crate dir; do
    [[ -z "$crate" ]] && continue
    publish_one "$CRATES_UI_DIR" "$crate"
  done <<< "$ORDER_UI"
fi
echo "=== 全部完成（mode=$MODE）===" | tee -a "$LOG"
