#!/usr/bin/env python3
"""K3.1 交付验收器 —— 把交接包的"验收门"从人读改成机读。

用法:
    python3 scripts/k31_delivery_check.py --dir <交付目录> [--json] [--strict]

判据来源（交接包 v1.1 / 创作简报 v3.5 / 施工规格 v1.5）:
    - SOURCE-OF-TRUTH.md 是开工前置；无它不开始 T0
    - 每个检查点更新 PROGRESS / MANIFEST / EVIDENCE / DECISIONS + checkpoints/vN（只增不改）
    - D2 = 冻结屏幕矩阵 4 类 × 明暗 = 8 张（workspace / application / control-tower /
      holographic-base），每张声明 screen_id / state_id / theme / viewport / source / 证据路径
    - 数据来源标签只允许 LIVE / MOCK / STALE / UNAVAILABLE / NOT_WIRED
    - EVIDENCE 每项至少写 artifact_id / viewport / DPI / theme / state_id / source /
      复现步骤 / 截图或录屏路径 / 实际结果 / 未验证项
    - 未测写"未验证"，不写"通过"

诚实边界:
    本脚本只做**结构与口径**的机械检查，不判断美术好坏、不判断交互是否可用；
    布局不同也能跑：优先读机器可读清单（*.manifest.json / manifest.json），
    其次从 MANIFEST.md 的表格/条目里尽力解析；解析不出的项一律报"未核验"，
    **不猜、不当作已通过**。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

SCREEN_IDS = {"workspace", "application", "control-tower", "holographic-base"}
THEMES = {"light", "dark"}
STATE_IDS = {
    "empty",
    "active",
    "selected",
    "loading",
    "awaiting-approval",
    "error",
    "unavailable",
}
SOURCES = {"LIVE", "MOCK", "STALE", "UNAVAILABLE", "NOT_WIRED"}

REQUIRED_FILES = [
    "SOURCE-OF-TRUTH.md",
    "PROGRESS.md",
    "MANIFEST.md",
    "EVIDENCE.md",
    "DECISIONS.md",
]

EVIDENCE_FIELDS = [
    ("artifact_id", ["artifact_id", "artifact id", "产物 id", "产物id"]),
    ("viewport", ["viewport", "视口", "窗口尺寸"]),
    ("dpi", ["dpi"]),
    ("theme", ["theme", "主题", "明暗"]),
    ("state_id", ["state_id", "state id", "状态 id", "状态id"]),
    ("source", ["source", "来源"]),
    ("复现步骤", ["复现步骤", "repro", "复现"]),
    ("证据路径", ["截图", "录屏", "路径", "evidence path", "artifact path", ".png", ".mp4"]),
    ("实际结果", ["实际结果", "结果", "actual"]),
    ("未验证项", ["未验证", "unverified", "未实测"]),
]

# 生产代码痕迹：交付物里出现这些，说明超出了 DESIGN_ONLY 的范围
PRODUCTION_PATTERNS = [
    ("crates-ui/", "引擎仓库路径"),
    ("src-tauri/", "宿主代码路径"),
    ("frontend/companion-desktop/", "旧生产前端路径"),
]

PATHY = re.compile(r"[\w./\\-]+\.(png|jpe?g|webp|gif|mp4|webm|mov|svg|pdf|json|md)", re.I)
CLAIM_WORDS = ("已通过", "已验证", "全部通过", "验证通过", "PASSED")
SCREEN_KEYS = {
    "screen_id": ["screen_id", "screen id", "屏幕 id", "屏幕id"],
    "state_id": ["state_id", "state id", "状态 id", "状态id"],
    "theme": ["theme", "主题"],
    "viewport": ["viewport", "视口", "窗口尺寸"],
    "source": ["source", "来源"],
    "evidence": ["evidence", "证据", "路径", "screenshot", "recording"],
}



def parse_tables(text: str) -> list[tuple[list[str], list[list[str]]]]:
    """解析 markdown 表格：返回 [(表头, 数据行)]。"""
    tables: list[tuple[list[str], list[list[str]]]] = []
    headers: list[str] | None = None
    rows: list[list[str]] = []

    def flush() -> None:
        nonlocal headers, rows
        if headers and rows:
            tables.append((headers, rows))
        headers, rows = None, []

    for line in text.splitlines():
        s = line.strip()
        if not s.startswith("|"):
            flush()
            continue
        cells = [c.strip() for c in s.strip("|").split("|")]
        if all(re.fullmatch(r"[-: ]+", c or "-") for c in cells):
            continue
        if headers is None:
            headers = [c.lower() for c in cells]
        else:
            rows.append(cells)
    flush()
    return tables


def col_index(headers: list[str], aliases: list[str]) -> int | None:
    for i, h in enumerate(headers):
        for a in aliases:
            if a.lower() in h:
                return i
    return None


@dataclass
class Report:
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    checked: dict = field(default_factory=dict)

    def err(self, msg: str) -> None:
        self.errors.append(msg)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)

    def note(self, msg: str) -> None:
        self.notes.append(msg)


def read_text(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def find_file(root: Path, name: str) -> Path | None:
    for p in sorted(root.rglob(name)):
        if p.is_file():
            return p
    return None


def parse_records_machine(root: Path) -> list[dict]:
    """优先读机器可读清单。"""
    out: list[dict] = []
    for cand in sorted(root.rglob("*.manifest.json")) + sorted(root.rglob("manifest.json")):
        try:
            data = json.loads(read_text(cand))
        except (json.JSONDecodeError, OSError):
            continue
        items = data.get("screens") if isinstance(data, dict) else data
        if isinstance(items, list):
            for it in items:
                if isinstance(it, dict):
                    it = dict(it)
                    it["_source_file"] = str(cand)
                    out.append(it)
    return out


def parse_records_text(root: Path) -> list[dict]:
    """从 MANIFEST.md / EVIDENCE.md / INDEX.md 里解析屏幕记录（表格优先，键值兜底）。"""
    out: list[dict] = []
    for fname in ("MANIFEST.md", "EVIDENCE.md", "INDEX.md"):
        p = root / fname
        if not p.exists():
            continue
        text = read_text(p)
        for headers, rows in parse_tables(text):
            idx = {k: col_index(headers, aliases) for k, aliases in SCREEN_KEYS.items()}
            if idx.get("screen_id") is None:
                continue
            for cells in rows:
                rec: dict = {"_raw": " | ".join(cells)}
                for k, i in idx.items():
                    if i is not None and i < len(cells):
                        rec[k] = cells[i]
                out.append(rec)
        if not out:
            for line in text.splitlines():
                low = line.lower()
                if not any(sid in low for sid in SCREEN_IDS):
                    continue
                rec = {"_raw": line.strip()}
                for key, aliases in SCREEN_KEYS.items():
                    for alias in aliases:
                        m = re.search(
                            re.escape(alias) + r"\s*[:=|：]\s*`?([A-Za-z0-9_\-./]+)", line, re.I
                        )
                        if m:
                            rec[key] = m.group(1)
                            break
                ev = PATHY.search(line)
                if ev:
                    rec.setdefault("evidence", ev.group(0))
                out.append(rec)
    return out


def check_required_files(root: Path, rep: Report) -> None:
    missing = []
    for name in REQUIRED_FILES:
        if find_file(root, name) is None:
            missing.append(name)
    if missing:
        rep.err(f"缺必备文件：{', '.join(missing)}")
    else:
        rep.note(f"必备文件齐备（{len(REQUIRED_FILES)} 项）")
    rep.checked["required_files"] = len(REQUIRED_FILES) - len(missing)


def check_checkpoints(root: Path, rep: Report) -> None:
    cks = sorted(
        {p.name for p in root.rglob("checkpoints/v*") if p.is_dir() and p.name.startswith("v")},
        key=lambda s: int(re.sub(r"\D", "", s) or 0),
    )
    if not cks:
        rep.err("没有 checkpoints/vN 快照目录（检查点未落盘）")
        return
    rep.note(f"检查点：{', '.join(cks)}")
    prev: set[str] | None = None
    for c in cks:
        d = root / "checkpoints" / c
        files = {p.relative_to(d).as_posix() for p in d.rglob("*") if p.is_file()}
        if prev is not None:
            lost = prev - files
            if lost:
                rep.err(f"检查点 {c} 相对上一版少了文件（应只增不改）：{', '.join(sorted(lost)[:5])}")
        for req in ("PROGRESS.md", "MANIFEST.md", "EVIDENCE.md", "DECISIONS.md"):
            if req not in files:
                rep.warn(f"检查点 {c} 内缺 {req}")
        prev = files
    rep.checked["checkpoints"] = len(cks)


def check_screens(root: Path, rep: Report) -> None:
    records = parse_records_machine(root) or parse_records_text(root)
    if not records:
        rep.warn("未解析到任何屏幕记录（缺屏幕矩阵清单）⇒ D2 无法核验")
        rep.checked["screens"] = 0
        return
    pairs: set[tuple[str, str]] = set()
    states: set[str] = set()
    bad_screen = bad_theme = bad_state = bad_source = 0
    missing_fields = 0
    for rec in records:
        sid = str(rec.get("screen_id", "")).strip().strip("`")
        theme = str(rec.get("theme", "")).strip().strip("`").lower()
        state = str(rec.get("state_id", "")).strip().strip("`").lower()
        src = str(rec.get("source", "")).strip().strip("`").upper()
        if sid:
            if sid.lower() not in SCREEN_IDS:
                bad_screen += 1
                rep.err(f"screen_id 非法：{sid}")
            else:
                pairs.add((sid.lower(), theme or "?"))
        if theme and theme not in THEMES:
            bad_theme += 1
            rep.err(f"theme 非法：{theme}")
        if state:
            if state not in STATE_IDS:
                bad_state += 1
                rep.err(f"state_id 非法：{state}")
            else:
                states.add(state)
        if src and src not in SOURCES:
            bad_source += 1
            rep.err(f"source 标签非法：{src}")
        if sid and not (rec.get("viewport") and rec.get("source") and rec.get("evidence")):
            missing_fields += 1
    if bad_theme == 0 and bad_state == 0 and bad_source == 0:
        rep.note(f"枚举校验通过（screen/state/theme/source）")
    missing_pairs = {
        (s, t) for s in SCREEN_IDS for t in THEMES
    } - pairs
    if missing_pairs:
        rep.warn(
            "D2 矩阵不完整，缺："
            + ", ".join(f"{s}/{t}" for s, t in sorted(missing_pairs))
        )
    else:
        rep.note("D2 矩阵完整（4 类 × 明暗 = 8）")
    missing_states = STATE_IDS - states
    if missing_states:
        rep.warn(f"state_id 未覆盖：{', '.join(sorted(missing_states))}")
    else:
        rep.note("state_id 七态全覆盖")
    if missing_fields:
        rep.warn(f"{missing_fields} 条屏幕记录缺 viewport/source/证据路径")
    missing_ev = 0
    for rec in records:
        ev = str(rec.get("evidence", "")).strip().strip("`")
        if not ev:
            continue
        if not (root / ev).exists():
            missing_ev += 1
            if missing_ev <= 5:
                rep.warn(f"证据路径在交付目录内找不到：{ev}")
    if missing_ev:
        rep.warn(f"共 {missing_ev} 条记录的证据路径未在交付目录内找到")
    else:
        rep.note("证据路径均在交付目录内存在")
    rep.checked["screens"] = len(records)
    rep.checked["screen_pairs"] = len(pairs)
    rep.checked["states_covered"] = len(states)


def check_evidence_fields(root: Path, rep: Report) -> None:
    p = find_file(root, "EVIDENCE.md")
    if p is None:
        return
    text = read_text(p)
    total = 0
    bad = 0
    seen_table = False
    for headers, rows in parse_tables(text):
        idx = {name: col_index(headers, aliases) for name, aliases in EVIDENCE_FIELDS}
        if idx.get("artifact_id") is None:
            continue
        seen_table = True
        for cells in rows:
            total += 1
            missing = []
            for name, i in idx.items():
                if i is None:
                    missing.append(name)
                elif i >= len(cells) or not cells[i].strip():
                    missing.append(name)
            if missing:
                bad += 1
                first = cells[0].strip()[:24] if cells else ""
                rep.warn(f"EVIDENCE 行缺字段：{', '.join(missing)}（行首：{first}）")
    if not seen_table:
        rep.warn("EVIDENCE.md 里没读到含 artifact_id 的表格（证据项可能未成表）")
    elif bad == 0:
        rep.note(f"EVIDENCE 字段齐备（{total} 行）")
    rep.checked["evidence_rows"] = total
    rep.checked["evidence_rows_missing_fields"] = bad


def check_source_labels(root: Path, rep: Report) -> None:
    """所有出现的数据来源词，都应带五类标签之一。"""
    suspicious = re.compile(
        r"(遥测|工具结果|Agent\s*状态|AI\s*建议|审批状态|文件版本)", re.I
    )
    hits = 0
    for p in sorted(root.rglob("*.md")):
        for i, line in enumerate(read_text(p).splitlines(), 1):
            if not suspicious.search(line):
                continue
            if not any(s in line.upper() for s in SOURCES):
                hits += 1
                if hits <= 5:
                    rep.warn(f"疑似缺来源标签：{p.name}:{i} —— {line.strip()[:60]}")
    if hits == 0:
        rep.note("来源标签抽查通过")
    rep.checked["unlabeled_source_lines"] = hits


def check_honesty(root: Path, rep: Report) -> None:
    hits = 0
    for p in sorted(root.rglob("*.md")):
        for i, line in enumerate(read_text(p).splitlines(), 1):
            if any(w in line for w in CLAIM_WORDS) and not PATHY.search(line):
                hits += 1
                if hits <= 5:
                    rep.warn(f"声称通过但未见证据路径：{p.name}:{i} —— {line.strip()[:60]}")
    rep.checked["claims_without_evidence_path"] = hits


def check_scope(root: Path, rep: Report) -> None:
    for pat, label in PRODUCTION_PATTERNS:
        for p in root.rglob("*"):
            if pat in p.as_posix():
                rep.err(f"交付目录内出现生产代码痕迹（{label}）：{p}")
                break
    for p in root.rglob("*.rs"):
        rep.err(f"交付目录内出现 Rust 源码（超出 DESIGN_ONLY）：{p}")
        break


def main() -> int:
    ap = argparse.ArgumentParser(description="K3.1 交付验收器（结构与口径，不判美术）")
    ap.add_argument("--dir", required=True, help="交付目录")
    ap.add_argument("--json", action="store_true", help="输出 JSON")
    ap.add_argument("--strict", action="store_true", help="有 ERROR 时退出码非 0")
    args = ap.parse_args()

    root = Path(args.dir).expanduser().resolve()
    rep = Report()
    if not root.is_dir():
        print(f"[k31-check] 目录不存在：{root}", file=sys.stderr)
        return 2

    check_required_files(root, rep)
    check_checkpoints(root, rep)
    check_screens(root, rep)
    check_evidence_fields(root, rep)
    check_source_labels(root, rep)
    check_honesty(root, rep)
    check_scope(root, rep)

    if args.json:
        print(
            json.dumps(
                {
                    "dir": str(root),
                    "errors": rep.errors,
                    "warnings": rep.warnings,
                    "notes": rep.notes,
                    "checked": rep.checked,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        print(f"[k31-check] 目录：{root}")
        for n in rep.notes:
            print(f"  · {n}")
        for w in rep.warnings:
            print(f"  ! 警告 {w}")
        for e in rep.errors:
            print(f"  ✗ 错误 {e}")
        print(
            f"[k31-check] 错误 {len(rep.errors)} / 警告 {len(rep.warnings)} / "
            f"已核验项 {len(rep.checked)}"
        )
        print("[k31-check] 说明：本器只核结构与口径，不判美术与交互手感；未核验项不等于通过。")

    if rep.errors and args.strict:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
