#!/usr/bin/env bash
# 下载内部卷用的**公有领域小说**语料（Project Gutenberg）。
#
# # 为什么需要它
#
# 内部卷（设计稿 §7.1）需要**真实语料**。实测：
# * `<数据根>/diary/*.json` ⇒ 本机只有 1 条（dream 写的认知梦境日志，非用户内容）⇒ 不可用；
# * git 提交历史 ⇒ 可用（已是主语料源之一），但它是「工程决策记忆」，**不等于叙事记忆**；
# * ⇒ 补一个**叙事**语料源：**小说**（owner 2026-10-26 授权：「下两本小说当语料算了」）。
#
# # 为什么是这两本
#
# * 《紅樓夢》—— **123 回**、人物关系复杂、前后**同一件事有不同说法**（⇒ conflict 题有真素材）；
# * 《水滸傳》—— **71 回**、事件按「路遇 → 收服 → 排座次」推进（⇒ temporal 题有真素材）。
# 两本都**公有领域**（明清白话小说），Project Gutenberg 收录（US-only copyright 已过期）。
#
# # 纪律
#
# * ⚠️ **数据不入库**：文件落在 **仓外**（`$APEIRETH_DATA_DIR/novels/`，默认 `~/.apeireth/novels/`）
#   ⇒ 仓里只有这个脚本 + 适配器，**不提交小说正文**（正文入库会让仓体积失控、
#   且让「数据可复现」变成「数据被冻结」）。
# * **校验和钉住**：每次下载都核对 SHA-256 ⇒ 内容漂移（Gutenberg 修订）会**当场报错**，
#   而不是悄悄换掉语料（悄悄换语料 ⇒ 卷子分数不可比）。
# * **不删已有文件**：已存在且校验通过 ⇒ 跳过（幂等）。
#
# # 用法
#
# ```bash
# bash scripts/download_novels.sh              # 下载到默认目录
# bash scripts/download_novels.sh --dir /tmp/x # 指定目录
# ```

set -euo pipefail

DATA_DIR="${APEIRETH_DATA_DIR:-$HOME/.apeireth}"
OUT_DIR="$DATA_DIR/novels"

# id | 文件名 | 标题 | SHA-256 | 回数（用于自检）
#
# ⚠️ 回数是**解析出来的真实回数**（2026-10-26 实测）：
#   《紅樓夢》120 回 / 《水滸傳》70 回。
#   ⚠️ **不是** `grep -c "^第.*回"` 的行数（那是 123 / 71）——
#   差值来自：① 正文里有 3~4 行**恰好以「第X回」开头**（回述上一回）；
#   ② 《紅樓夢》第四十五回**标题行重复**一次。
#   ⇒ 脚本下面那行 grep 只是**提示**（不做断言），别当回数用。
NOVELS=(
  "24264|redchamber.txt|紅樓夢|ff1526996bf4b81807651921a85e5c1c0f1d1d123c9fa4553057ba6a3ec72011|120"
  "23863|watershed.txt|水滸傳|9d5b723b57f462545d412593870e7f48157829777b9329350a29bc44fd654789|70"
)

while [ $# -gt 0 ]; do
  case "$1" in
    --dir) OUT_DIR="$2"; shift 2 ;;
    -h|--help) sed -n '2,40p' "$0"; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done

mkdir -p "$OUT_DIR"

sha_of() { sha256sum "$1" | cut -d' ' -f1; }

for entry in "${NOVELS[@]}"; do
  IFS='|' read -r id filename title expected chapters <<<"$entry"
  target="$OUT_DIR/$filename"

  if [ -f "$target" ]; then
    actual="$(sha_of "$target")"
    if [ "$actual" = "$expected" ]; then
      echo "跳过（已存在且校验通过）：$title → $target"
      continue
    fi
    echo "⚠️ 已存在但校验和不同 ⇒ 重下：$title" >&2
    mv "$target" "$target.bak.$(date +%s)"
  fi

  url="https://www.gutenberg.org/cache/epub/$id/pg$id.txt"
  echo "下载 $title（PG #$id）→ $target"
  # Gutenberg 的 eBook 在部分镜像上会走 http ⇒ 只允许 https（明文 http 会被中间人改内容）。
  # ⚠️ **断点续传 + 强重试**：本机到 gutenberg.org 的连接会**中途断流**
  # （`SSL_read: unexpected eof`）⇒ 单次下载会半途而废。
  # ⇒ `--continue-at -`（续传）+ `--retry 8` + `--retry-all-errors`（含 SSL 断流）
  #   + 失败后**删掉半截文件**（半截文件会被下一次当成"已存在但校验不符"⇒ 噪声）。
  for attempt in 1 2 3 4 5; do
    if curl --fail --silent --show-error --location --http1.1 \
      --retry 8 --retry-delay 3 --retry-all-errors --retry-max-time 180 \
      --continue-at - --output "$target" "$url"; then
      break
    fi
    echo "  第 $attempt 次下载中断 ⇒ 重试（续传）" >&2
    sleep $((attempt * 3))
    if [ "$attempt" -eq 5 ]; then
      echo "✗ 连试 5 次仍失败：$title" >&2
      rm -f "$target"
      exit 1
    fi
  done

  actual="$(sha_of "$target")"
  if [ "$actual" != "$expected" ]; then
    echo "✗ 校验和不符：$title" >&2
    echo "  期望 $expected" >&2
    echo "  实际 $actual" >&2
    echo "  ⇒ Gutenberg 内容可能已修订 ⇒ **不要**把新内容当同一份语料用" >&2
    echo "  （要么把新校验和写进本脚本并记档，要么换一本）" >&2
    exit 1
  fi

  found="$(grep -cE '^\s*第.{1,8}回' "$target" || true)"
  echo "  ✓ 校验通过（$chapters 回，检出 $found 个章节标题）"
done

echo
echo "语料目录：$OUT_DIR"
ls -la "$OUT_DIR"
echo
echo "⚠️ 提醒：这些文件**在仓外** ⇒ 不会被提交 ⇒ 换机器要重跑本脚本。"
