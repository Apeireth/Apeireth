#!/usr/bin/env python3
"""文档里的**源文件路径悬空**检查（只减不增的棘轮）。

**为什么要有这个门禁**：2026-10-25 一次实测发现，**活文档**里有 26 处
「文档说这个源文件在这里，但它已经不在了」—— 绝大多数是 crate 重组后
文档没跟上（`engine/runtime/src/canonical/*` → `engine/runtime-assembly/...`）。
这类悬空**不会让任何测试变红**，但会让「照文档找代码」的人扑空 ⇒
是典型的**叙事滞后（narration-lag）**缺陷 ⇒ 用机检守住。

**判据（刻意收窄，避免误判）**：只判「**crate 目录存在、但文件没了**」。
不判：
  ① 整 crate 都不存在 ⇒ 那是**规划中/待建**的 crate（蓝图里的「目标路径」）；
  ② 历史卷宗类文档（按 `check_doc_caliber.py` 的历史文档判据剔除）。

**用法**：
```
python3 scripts/check_doc_paths.py          # 报告（列出新增悬空）
python3 scripts/check_doc_paths.py --check   # 闸：新增悬空即 exit 1
```
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "doc-path-baseline.txt"

# `crates/<group>/<crate>/<...>.rs`
CRATE_PATH = re.compile(r"crates/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/([A-Za-z0-9_./-]+\.rs)")


def _load_caliber_module():
    """复用 `check_doc_caliber.py` 的**历史文档判据**（避免两处各写一遍必然漂移）。"""
    path = ROOT / "scripts" / "check_doc_caliber.py"
    spec = importlib.util.spec_from_file_location("doc_caliber", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def is_historical_doc(doc: Path, caliber) -> bool:
    """该文档是否属于**历史卷宗**（不判它的悬空）。"""
    # ⚠️ 必须用**相对仓库根**的路径：`SKIP_DIR_PREFIXES` 形如 `docs/archive/`，
    # 拿绝对路径去 startswith 永远不成立 ⇒ 归档卷宗会被当成活文档判悬空。
    rel = doc.relative_to(ROOT).as_posix()
    if any(rel.startswith(prefix) for prefix in caliber.SKIP_DIR_PREFIXES):
        return True
    if any(pattern.search(doc.name) for pattern in caliber.SKIP_NAME_RES):
        return True
    try:
        head = "\n".join(doc.read_text(encoding="utf-8").splitlines()[:20])
    except (OSError, UnicodeDecodeError):
        return True
    return bool(caliber.HISTORY_BANNER_RES.search(head))


def scan_dangling() -> list[tuple[str, str]]:
    """返回 [(文档相对路径, 悬空的源文件引用), ...]（已剔除历史文档与「待建 crate」）。"""
    caliber = _load_caliber_module()
    found: list[tuple[str, str]] = []
    for doc in sorted((ROOT / "docs").glob("**/*.md")):
        if is_historical_doc(doc, caliber):
            continue
        try:
            text = doc.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for match in CRATE_PATH.finditer(text):
            group, crate, relative = match.group(1), match.group(2), match.group(3)
            crate_dir = ROOT / "crates" / group / crate
            if not crate_dir.is_dir():
                continue  # 整 crate 都没有 ⇒ 规划中/待建，不判
            if (crate_dir / relative).exists():
                continue
            found.append((doc.relative_to(ROOT).as_posix(), f"crates/{group}/{crate}/{relative}"))
    return found


def load_baseline() -> dict[str, str]:
    """基线: {文档相对路径: 理由}（同一文档可能有多条例外 ⇒ 理由拼起来）。"""
    entries: dict[str, str] = {}
    if not BASELINE.exists():
        return entries
    for raw in BASELINE.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("#", 1)
        path = parts[0].strip()
        reason = parts[1].strip() if len(parts) > 1 else ""
        entries[path] = (entries.get(path, "") + " " + reason).strip()
    return entries


def main() -> int:
    parser = argparse.ArgumentParser(description="文档源文件路径悬空检查（只减不增）")
    parser.add_argument("--check", action="store_true", help="闸：新增悬空即失败")
    args = parser.parse_args()

    dangling = scan_dangling()
    baseline = load_baseline()

    # 基线按「文档」计（一个文档里可能有多条悬空引用）⇒ 以文档为粒度判定。
    by_doc: dict[str, list[str]] = {}
    for doc_path, reference in dangling:
        by_doc.setdefault(doc_path, []).append(reference)

    new_docs = {doc: refs for doc, refs in by_doc.items() if doc not in baseline}
    resolved = [doc for doc in baseline if doc not in by_doc]

    if args.check:
        if new_docs:
            print(f"文档里的源文件路径悬空：{len(new_docs)} 个文档（新增，必须修或写进基线并给理由）:")
            for doc in sorted(new_docs):
                for reference in sorted(new_docs[doc]):
                    print(f"  {doc} → {reference}")
            print(
                "修法：把文档里的路径改成**现址**（crate 重组后最常见）；"
                "若引用的是**规划中/待建**的文件，则把该文档登记进 "
                "scripts/doc-path-baseline.txt 并写理由。"
            )
            return 1
        if resolved:
            print(f"基线里有 {len(resolved)} 条**已不再悬空**（应删；棘轮只减不增）:")
            for doc in resolved:
                print(f"  {doc}")
            print("修法：从 scripts/doc-path-baseline.txt 删掉这些行。")
            return 1
        print(f"文档源文件路径悬空基线一致（{len(by_doc)} 个文档有已知例外，无新增）")
        return 0

    print(f"文档里的源文件路径悬空：{len(dangling)} 处，涉 {len(by_doc)} 个文档")
    for doc in sorted(by_doc):
        mark = "基线例外" if doc in baseline else "★ 新增"
        print(f"  [{mark}] {doc}")
        for reference in sorted(by_doc[doc]):
            print(f"      → {reference}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
