#!/usr/bin/env python3
"""构建**真实查询考卷**（MIRACL 中文）—— 从原始数据到可跑批的考卷。

# 为什么要有这个脚本
2026-11-07 实测发现：本项目两套考卷都是**模板题**
⇒ ⇒ 「依赖题面形态」的结论（例如重排特征 `first_pos`）**不能外推**。
⇒ ⇒ ⇒ 本脚本从 **MIRACL 中文 dev**（393 条**真实母语查询** + 官方相关性标注）
构建一个**可复现**的考卷 ⇒ ⇒ 用它做「方向闸门」。

# 数据来源与许可
* 查询与标注：`miracl/miracl`（HF）⇒ `miracl-v1.0-zh/{topics,qrels}`
* 语料：`miracl/miracl-corpus`（HF）⇒ `miracl-corpus-v1.0-zh/docs-*.jsonl.gz`
* 语料与标注来自维基百科（**CC BY-SA**）⇒ ⇒ 仅作**研究用途**
  ⇒ ⇒ ⚠️ **不要**把语料打包进本仓
* 镜像（本机实测可用）：`https://hf-mirror.com/datasets/...`

# 用法
```sh
# ① 下载（约 700MB；需要磁盘余量，**别放 /tmp**，那里是 tmpfs）
mkdir -p ~/miracl && cd ~/miracl
for i in 0 1 2 3 4 5 6 7 8 9; do
  curl -sSL -C - -o docs-$i.jsonl.gz \\
    "https://hf-mirror.com/datasets/miracl/miracl-corpus/resolve/main/miracl-corpus-v1.0-zh/docs-$i.jsonl.gz"
done
curl -sSL -o topics.tsv \\
  "https://hf-mirror.com/datasets/miracl/miracl/resolve/main/miracl-v1.0-zh/topics/topics.miracl-v1.0-zh-dev.tsv"
curl -sSL -o qrels.tsv \\
  "https://hf-mirror.com/datasets/miracl/miracl/resolve/main/miracl-v1.0-zh/qrels/qrels.miracl-v1.0-zh-dev.tsv"

# ② 构建（默认输出到 $APEIRETH_DATA_DIR/miracl-exam）
python3 scripts/build_real_query_exam.py --miracl-dir ~/miracl
```

# 口径（数字必须带这段一起引用）
* **采样池**：语料 = 全部**被标注**的段落 + **确定性采样**的未标注段落
  （判据 = `sha1(docid)` 前 4 位 < 阈值 ⇒ **与内容无关、可复现、跨机器一致**）
  ⇒ ⇒ **池子远小于真实全库（5.9M）** ⇒ ⇒ **召回偏乐观**
  ⇒ ⇒ ⇒ **绝对值不可与 MIRACL 官方榜对比**；只看**相对变化**
* **相关性**：`rel == 1` 算相关；多相关文档时**任一命中**即算命中
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path

# 采样阈值：`sha1(docid)` 前 4 位小于该值即被采入（≈ 全库的 0.39%）
SAMPLE_THRESHOLD = int("0100", 16)


def load_qrels(path: Path) -> dict[str, dict[str, int]]:
    qrels: dict[str, dict[str, int]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.rstrip("\n").split("\t")
        if len(parts) < 4:
            continue
        qid, _, docid, rel = parts[0], parts[1], parts[2], int(parts[3])
        qrels.setdefault(qid, {})[docid] = rel
    return qrels


def load_queries(path: Path) -> dict[str, str]:
    queries: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.rstrip("\n").split("\t")
        if len(parts) >= 2:
            queries[parts[0]] = parts[1]
    return queries


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--miracl-dir", required=True, help="原始数据目录（含 docs-*.jsonl.gz / topics / qrels）")
    parser.add_argument("--out", default=None, help="输出目录（默认 $APEIRETH_DATA_DIR/miracl-exam）")
    args = parser.parse_args()

    source = Path(args.miracl_dir)
    if args.out:
        out = Path(args.out)
    elif os.environ.get("APEIRETH_DATA_DIR"):
        out = Path(os.environ["APEIRETH_DATA_DIR"]) / "miracl-exam"
    else:
        out = Path.home() / ".apeireth" / "miracl-exam"
    out.mkdir(parents=True, exist_ok=True)

    topics = source / "topics.miracl-v1.0-zh-dev.tsv"
    qrels_path = source / "qrels.miracl-v1.0-zh-dev.tsv"
    if not topics.exists() or not qrels_path.exists():
        print(f"· 缺 topics/qrels（期望 {topics} 与 {qrels_path}）⇒ **没建成**")
        return 1
    shards = sorted(source.glob("docs-*.jsonl.gz"))
    if not shards:
        print(f"· {source} 里没有 docs-*.jsonl.gz ⇒ **没建成**")
        return 1

    qrels = load_qrels(qrels_path)
    queries = load_queries(topics)
    wanted = set()
    for rels in qrels.values():
        wanted.update(rels.keys())
    print(f"· 标注：{len(qrels)} 条查询 · {sum(len(v) for v in qrels.values())} 对 · 被标注段落 {len(wanted)}")

    docs: dict[str, tuple[str, str]] = {}
    for shard in shards:
        with gzip.open(shard, "rt", encoding="utf-8") as handle:
            for line in handle:
                try:
                    obj = json.loads(line)
                except Exception:
                    continue
                docid = obj.get("docid", "")
                if not docid:
                    continue
                if docid in wanted:
                    docs[docid] = (obj.get("title", ""), obj.get("text", ""))
                elif int(hashlib.sha1(docid.encode()).hexdigest()[:4], 16) < SAMPLE_THRESHOLD:
                    docs[docid] = (obj.get("title", ""), obj.get("text", ""))
        print(f"  {shard.name}: 累计 {len(docs)} 段")

    cases = []
    for qid, rels in qrels.items():
        gold = sorted(d for d, r in rels.items() if r == 1 and d in docs)
        if not gold or qid not in queries:
            continue
        cases.append({"id": qid, "query": queries[qid], "gold": gold})
    print(f"· 可用题：{len(cases)}（至少一条相关文档在池里）")

    with open(out / "corpus.jsonl", "w", encoding="utf-8") as handle:
        for docid, (title, text) in docs.items():
            handle.write(json.dumps({"id": docid, "title": title, "text": text}, ensure_ascii=False) + "\n")
    with open(out / "exam.jsonl", "w", encoding="utf-8") as handle:
        for case in cases:
            handle.write(json.dumps(case, ensure_ascii=False) + "\n")
    print(f"✅ 已写：{out}/corpus.jsonl · {out}/exam.jsonl")
    print("   ⚠️ 口径：采样池 ⇒ **召回偏乐观** ⇒ 绝对值不可与 MIRACL 官方榜对比")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
