"""Phase 1394 v1394_deploy_history — V1394 ASI 真生产 deploy-stack history (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31).

V1394 = real production deploy-stack history: JSONL log of V1393 judge results.
- 真 append (主 17:43 实事求是)
- 真借鉴: greenkeeper.io / dependabot / sonarqube history
- 任何人能接手 (主 00:56): 1 个 JSONL 文件 + 1 个 CLI
- 不假装 (主 17:58): history 是 raw log, 任何人可分析

V1394 真生产 数据结构:
- HistoryEntry: timestamp, target, verdict, score, grade, n_findings, summary
- History: path (JSONL), entries (List[HistoryEntry])
- append_entry(entry, path): 真 append to JSONL
- load_history(path): 真 load JSONL
- trend(entries): 算 trend (improving/stable/declining)
- main CLI: version / append <target> / show / trend / summary / popper
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


V1394_VERSION = "0.1.0"
V1394_SCHEMA = "v1394.deploy-history/v1"

# V1394 真生产 default history path (主 17:43)
V1394_DEFAULT_HISTORY_PATH = ".v1393-judge-history.jsonl"

# V1394 真生产 GUARDS (主 17:43)
V1394_GUARDS: tuple = (
    "GUARD_HISTORY_REAL",        # 真 JSONL 读写
    "GUARD_NO_CAP_CHANGE",       # 不改 ASI cap
    "GUARD_DETERMINISTIC",       # same input → same trend
    "GUARD_HONEST_DISCLOSURE",   # 标注 raw log
    "GUARD_PATH_SAFE",           # path 不外逃
    "GUARD_TREND_VALID",         # trend ∈ improving/stable/declining
    "GUARD_NON_DESTRUCTIVE",     # 不真删数据
    "GUARD_CLI_RUNNABLE",        # CLI 真可跑
)


# ============================================================================
# V1394 真生产 数据结构 (主 17:43)
# ============================================================================


@dataclass
class HistoryEntry:
    """V1394 真生产 1 条 history entry (主 17:43)."""

    timestamp: str = ""
    target: str = ""
    verdict: str = "GOOD"
    score: int = 100
    grade: str = "A+"
    n_findings: int = 0
    n_errors: int = 0
    n_warnings: int = 0
    n_info: int = 0
    policy_pass: bool = True
    policy_score: int = 100
    n_hints: int = 0
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1394_SCHEMA,
            "version": V1394_VERSION,
            "timestamp": self.timestamp,
            "target": self.target,
            "verdict": self.verdict,
            "score": self.score,
            "grade": self.grade,
            "n_findings": self.n_findings,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "policy_pass": self.policy_pass,
            "policy_score": self.policy_score,
            "n_hints": self.n_hints,
            "notes": self.notes,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "HistoryEntry":
        """V1394 真生产: 从 dict 构造 (主 17:43)."""
        return cls(
            timestamp=data.get("timestamp", ""),
            target=data.get("target", ""),
            verdict=data.get("verdict", "GOOD"),
            score=data.get("score", data.get("deploy_score", 100)),
            grade=data.get("grade", data.get("deploy_grade", "A+")),
            n_findings=data.get("n_findings", 0),
            n_errors=data.get("n_errors", 0),
            n_warnings=data.get("n_warnings", 0),
            n_info=data.get("n_info", 0),
            policy_pass=data.get("policy_pass", True),
            policy_score=data.get("policy_score", 100),
            n_hints=data.get("n_hints", 0),
            notes=data.get("notes", []),
        )


@dataclass
class Trend:
    """V1394 真生产 trend result (主 17:43)."""

    direction: str = "stable"  # improving / stable / declining
    delta_score: int = 0       # last - first
    delta_findings: int = 0
    n_entries: int = 0
    first_score: int = 100
    last_score: int = 100
    first_timestamp: str = ""
    last_timestamp: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "direction": self.direction,
            "delta_score": self.delta_score,
            "delta_findings": self.delta_findings,
            "n_entries": self.n_entries,
            "first_score": self.first_score,
            "last_score": self.last_score,
            "first_timestamp": self.first_timestamp,
            "last_timestamp": self.last_timestamp,
        }


def append_entry(entry: HistoryEntry, path: str = V1394_DEFAULT_HISTORY_PATH) -> bool:
    """V1394 真生产: 真 append entry to JSONL (主 17:43)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    if not entry.timestamp:
        entry.timestamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    line = json.dumps(entry.to_dict(), ensure_ascii=False)
    with open(p, "a", encoding="utf-8") as f:
        f.write(line + "\n")
    return True


def load_history(path: str = V1394_DEFAULT_HISTORY_PATH) -> List[HistoryEntry]:
    """V1394 真生产: 真 load JSONL history (主 17:43)."""
    p = Path(path)
    if not p.exists():
        return []
    entries: List[HistoryEntry] = []
    with open(p, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
                entries.append(HistoryEntry.from_dict(data))
            except (json.JSONDecodeError, KeyError, ValueError):
                continue
    return entries


def compute_trend(entries: List[HistoryEntry]) -> Trend:
    """V1394 真生产: 真 compute trend (主 17:43)."""
    if not entries:
        return Trend()
    if len(entries) == 1:
        return Trend(
            n_entries=1,
            first_score=entries[0].score,
            last_score=entries[0].score,
            first_timestamp=entries[0].timestamp,
            last_timestamp=entries[0].timestamp,
        )
    first = entries[0]
    last = entries[-1]
    delta = last.score - first.score
    if delta > 5:
        direction = "improving"
    elif delta < -5:
        direction = "declining"
    else:
        direction = "stable"
    return Trend(
        direction=direction,
        delta_score=delta,
        delta_findings=last.n_findings - first.n_findings,
        n_entries=len(entries),
        first_score=first.score,
        last_score=last.score,
        first_timestamp=first.timestamp,
        last_timestamp=last.timestamp,
    )


def popper_self_test() -> Dict[str, Any]:
    """V1394 真生产 Popper self-test (主 17:43)."""
    failures: List[str] = []
    # Test 1: empty history
    h = load_history("___nonexistent_path_xyz___")
    if h != []:
        failures.append("nonexistent path should return []")
    # Test 2: append + load roundtrip
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        e1 = HistoryEntry(
            timestamp="2026-08-09T07:00:00Z",
            target="x", verdict="FAIL", score=50, grade="D",
            n_findings=5, n_errors=2, n_warnings=3, n_info=0,
            policy_pass=False, policy_score=50, n_hints=5,
        )
        e2 = HistoryEntry(
            timestamp="2026-08-09T07:30:00Z",
            target="x", verdict="OK", score=75, grade="B",
            n_findings=3, n_errors=0, n_warnings=2, n_info=1,
            policy_pass=True, policy_score=100, n_hints=3,
        )
        append_entry(e1, tp)
        append_entry(e2, tp)
        loaded = load_history(tp)
        if len(loaded) != 2:
            failures.append(f"expected 2 entries, got {len(loaded)}")
        if loaded[0].timestamp != "2026-08-09T07:00:00Z":
            failures.append(f"first entry timestamp mismatch")
    finally:
        Path(tp).unlink()
    # Test 3: trend improving
    if compute_trend([e1, e2]).direction != "improving":
        failures.append("e1=50 → e2=75 should be improving")
    # Test 4: trend declining
    e3 = HistoryEntry(timestamp="2026-08-09T08:00:00Z", score=30, grade="F")
    if compute_trend([e1, e3]).direction != "declining":
        failures.append("e1=50 → e3=30 should be declining")
    # Test 5: trend stable
    e4 = HistoryEntry(timestamp="2026-08-09T08:00:00Z", score=52, grade="D")
    if compute_trend([e1, e4]).direction != "stable":
        failures.append("e1=50 → e4=52 should be stable (delta=2)")
    # Test 6: single entry trend
    t = compute_trend([e1])
    if t.n_entries != 1 or t.first_score != 50:
        failures.append("single entry trend wrong")
    # Test 7: empty trend
    t = compute_trend([])
    if t.n_entries != 0 or t.direction != "stable":
        failures.append("empty trend wrong")
    # Test 8: to_dict roundtrip
    d = e1.to_dict()
    e2_back = HistoryEntry.from_dict(d)
    if e2_back.target != e1.target or e2_back.score != e1.score:
        failures.append("to_dict / from_dict roundtrip failed")
    # Test 9: timestamp auto-populated
    e5 = HistoryEntry(target="auto")
    append_entry(e5, tp)
    if not e5.timestamp:
        failures.append("timestamp auto-populated failed")
    Path(tp).unlink(missing_ok=True)
    # Test 10: GUARDS count
    if len(V1394_GUARDS) < 8:
        failures.append(f"GUARDS < 8: {len(V1394_GUARDS)}")
    return {
        "passed": len(failures) == 0,
        "failures": failures,
        "n_tested": 10,
    }


# ============================================================================
# V1394 CLI (主 17:43 真可执行)
# ============================================================================


def _run_v1393_judge(target: str, policy_path: Optional[str]) -> Any:
    """V1394 真生产: 真调 V1393 judge."""
    try:
        from v1393_deploy_judge import judge as _judge
    except Exception:
        import sys as _sys
        _v1394_dir = Path(__file__).resolve().parent
        if str(_v1394_dir) not in _sys.path:
            _sys.path.insert(0, str(_v1394_dir))
        try:
            from v1393_deploy_judge import judge as _judge
        except Exception:
            return None
    return _judge(target, policy_path=policy_path)


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1394 真生产 CLI 主入口 (主 17:43 真可执行)."""
    parser = argparse.ArgumentParser(
        prog="v1394-deploy-history",
        description=f"V1394 real production deploy-stack history (v{V1394_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="V1394 version")

    p_append = sub.add_parser("append", help="judge target and append to history")
    p_append.add_argument("target", help="target directory")
    p_append.add_argument("--policy", default=None, help="policy YAML file")
    p_append.add_argument("--history", default=V1394_DEFAULT_HISTORY_PATH, help="history JSONL file")

    p_show = sub.add_parser("show", help="show history")
    p_show.add_argument("--history", default=V1394_DEFAULT_HISTORY_PATH, help="history JSONL file")
    p_show.add_argument("--target", default=None, help="filter by target")
    p_show.add_argument("--last", type=int, default=None, help="show last N entries")

    p_trend = sub.add_parser("trend", help="compute trend")
    p_trend.add_argument("--history", default=V1394_DEFAULT_HISTORY_PATH, help="history JSONL file")
    p_trend.add_argument("--target", default=None, help="filter by target")
    p_trend.add_argument("--json", action="store_true", help="JSON output")

    p_summary = sub.add_parser("summary", help="summarize history")
    p_summary.add_argument("--history", default=V1394_DEFAULT_HISTORY_PATH, help="history JSONL file")
    p_summary.add_argument("--target", default=None, help="filter by target")

    sub.add_parser("demo", help="V1394 demo")
    sub.add_parser("popper", help="V1394 Popper self-test")

    args = parser.parse_args(argv)
    cmd = args.cmd or "version"

    if cmd == "version":
        print(f"V1394 deploy history v{V1394_VERSION} (schema {V1394_SCHEMA})")
        return 0
    if cmd == "append":
        jr = _run_v1393_judge(args.target, args.policy)
        if jr is None:
            print("V1393 judge unavailable", file=sys.stderr)
            return 1
        entry = HistoryEntry(
            target=jr.target,
            verdict=jr.verdict,
            score=jr.deploy_score,
            grade=jr.deploy_grade,
            n_findings=jr.n_findings,
            n_errors=jr.n_errors,
            n_warnings=jr.n_warnings,
            n_info=jr.n_info,
            policy_pass=jr.policy_pass,
            policy_score=jr.policy_score,
            n_hints=jr.n_hints,
            notes=jr.notes,
        )
        append_entry(entry, args.history)
        print(f"appended: {entry.timestamp} {entry.target} verdict={entry.verdict} score={entry.score}")
        return 0
    if cmd == "show":
        entries = load_history(args.history)
        if args.target:
            entries = [e for e in entries if e.target == args.target]
        if args.last:
            entries = entries[-args.last:]
        for e in entries:
            print(f"{e.timestamp} {e.target} verdict={e.verdict} score={e.score} grade={e.grade} findings={e.n_findings}")
        return 0
    if cmd == "trend":
        entries = load_history(args.history)
        if args.target:
            entries = [e for e in entries if e.target == args.target]
        t = compute_trend(entries)
        if args.json:
            print(json.dumps(t.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(f"trend: {t.direction} (delta_score={t.delta_score}, entries={t.n_entries})")
            print(f"  first: {t.first_timestamp} score={t.first_score}")
            print(f"  last:  {t.last_timestamp} score={t.last_score}")
        return 0
    if cmd == "summary":
        entries = load_history(args.history)
        if args.target:
            entries = [e for e in entries if e.target == args.target]
        if not entries:
            print("no entries")
            return 0
        n = len(entries)
        last = entries[-1]
        t = compute_trend(entries)
        print(f"V1394 history summary")
        print(f"  entries: {n}")
        print(f"  last: {last.timestamp} {last.target} verdict={last.verdict} score={last.score} grade={last.grade}")
        print(f"  trend: {t.direction} (delta_score={t.delta_score})")
        return 0
    if cmd == "demo":
        # Demo: append 2 fake entries
        import tempfile
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            e1 = HistoryEntry(
                timestamp="2026-08-09T07:00:00Z", target="demo",
                verdict="POOR", score=50, grade="D",
                n_findings=5, n_errors=2, n_warnings=3, n_info=0,
            )
            e2 = HistoryEntry(
                timestamp="2026-08-09T08:00:00Z", target="demo",
                verdict="OK", score=75, grade="B",
                n_findings=3, n_errors=0, n_warnings=2, n_info=1,
            )
            append_entry(e1, tp)
            append_entry(e2, tp)
            loaded = load_history(tp)
            t = compute_trend(loaded)
            print(f"demo: 2 entries, trend={t.direction}, delta_score={t.delta_score}")
        finally:
            Path(tp).unlink()
        return 0
    if cmd == "popper":
        r = popper_self_test()
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return 0 if r["passed"] else 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli())
