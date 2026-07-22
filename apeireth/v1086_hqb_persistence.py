"""Phase 220 v1086_hqb_persistence — V1086 HQB persistence: guard log + ASI delta (主 21:15 + R2-REQ-01 A).

V1086 = HQB 守门记录 + ASI delta 持久化. 接收 V1085 HonestDecision, 写到独立 artifacts
目录 (artifacts/v1086/), 不写 V1074 artifacts 控制台.

真借鉴 (主 13:08 + 主 18:52 + 主 19:33):
- V1074 asi_snapshot.json (只读 baseline, 不写)
- HARNESS.md §3 Change Manifest (本模块是 HQB gate log, 不是 change manifest)
- HARNESS.md §2.3 HQB 4 维 (持久化层不重建测量, 只持久化决策)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- ASI delta ≠ ASI score (主 17:43 实事求是: delta 是 inventory, 不是 ASI 本身)
- 不假装 guard_log = 真生产 (log 是真生产的一部分, 不是全部)

边界 (主 07-19 4 层安全门):
- 只读 artifacts/asi_snapshot.json (V1074 真生产 artifacts, 不动)
- 只写 artifacts/v1086/ (新建独立目录, 不污染 V1074 控制台)
- 不动 V1074 / V1081 / philosophy_guard
"""
from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

from apeireth.v1085_hqb_core import HonestDecision, HonestDecisionModule, V1085_VERSION


V1086_VERSION = "0.1.0"

DEFAULT_ARTIFACT_DIR = Path("artifacts") / "v1086"
DEFAULT_GUARD_LOG = "guard_log.jsonl"
DEFAULT_BASELINE_KEY = "asi_v03_score"


@dataclass
class GuardLogEntry:
    """V1086 单条守门记录 (主 17:43 实事求是: 每条必带 score_used + verdict)."""
    decision: HonestDecision
    asi_v03_at_record: float
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        d = self.decision.to_dict()
        d["asi_v03_at_record"] = round(self.asi_v03_at_record, 4)
        d["log_ts"] = self.ts
        return d


class HQBPersistence:
    """V1086 HQB 守门持久化 (主 21:15 + HARNESS.md §2.3 真借鉴).

    设计点 (主 17:43 + 主 07-19):
    - 独立 artifacts/v1086/ 目录, 不污染 V1074 控制台
    - 只读 V1074 asi_snapshot.json 拿 baseline (主 07-19: 信任边界 = 文件级 read)
    - JSONL append 模式 (主 17:43 实事求是: 真记录, 不覆盖)
    - asi_delta 是相对值, 不是 ASI 本身 (主 17:43: delta ≠ ASI)
    """

    def __init__(
        self,
        artifact_dir: Path = DEFAULT_ARTIFACT_DIR,
        guard_log_name: str = DEFAULT_GUARD_LOG,
        snapshot_path: Optional[Path] = None,
    ):
        self.artifact_dir = Path(artifact_dir)
        self.guard_log_path = self.artifact_dir / guard_log_name
        # V1074 asi_snapshot.json 默认位置 (主 07-19 文件级 read)
        self.snapshot_path = (
            Path(snapshot_path) if snapshot_path
            else Path("artifacts") / "asi_snapshot.json"
        )
        self.entries: List[GuardLogEntry] = []
        self._baseline_asi_v03: Optional[float] = None

    def _ensure_dir(self) -> None:
        self.artifact_dir.mkdir(parents=True, exist_ok=True)

    def read_baseline_asi_v03(self) -> float:
        """V1086 读 V1074 baseline (只读, 不写). 主 07-19 文件级 read."""
        if not self.snapshot_path.exists():
            self._baseline_asi_v03 = 0.0
            return 0.0
        try:
            with open(self.snapshot_path, encoding="utf-8") as f:
                snap = json.load(f)
            score = float(snap.get(DEFAULT_BASELINE_KEY, 0.0))
        except (json.JSONDecodeError, OSError, ValueError):
            score = 0.0
        self._baseline_asi_v03 = score
        return score

    def record(self, decision: HonestDecision, asi_v03: Optional[float] = None) -> GuardLogEntry:
        """V1086 记录一条守门决策 (主 17:43 真记录, 不假装)."""
        if asi_v03 is None:
            asi_v03 = self.read_baseline_asi_v03()
        entry = GuardLogEntry(decision=decision, asi_v03_at_record=asi_v03)
        self.entries.append(entry)
        self._append_to_log(entry)
        return entry

    def _append_to_log(self, entry: GuardLogEntry) -> None:
        """V1086 JSONL append (主 17:43 实事求是: 真追加, 不覆盖)."""
        self._ensure_dir()
        with open(self.guard_log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry.to_dict(), ensure_ascii=False) + "\n")

    def asi_delta(self, current_hqb_total: float) -> float:
        """V1086 计算 ASI delta (主 17:43: delta 是 inventory, 不是 ASI 本身).

        delta = current_hqb_total - baseline_asi_v03
        """
        baseline = self.read_baseline_asi_v03()
        return float(current_hqb_total - baseline)

    def latest(self, n: int = 5) -> List[Dict[str, Any]]:
        if n <= 0 or not self.entries:
            return []
        return [e.to_dict() for e in self.entries[-n:]]

    def stats(self) -> Dict[str, Any]:
        by_verdict: Dict[str, int] = {}
        for e in self.entries:
            v = e.decision.verdict.value
            by_verdict[v] = by_verdict.get(v, 0) + 1
        return {
            "n_entries": len(self.entries),
            "by_verdict": by_verdict,
            "artifact_dir": str(self.artifact_dir),
            "guard_log": str(self.guard_log_path),
            "baseline_asi_v03": self.read_baseline_asi_v03(),
            "version": V1086_VERSION,
            "philosophy": (
                "V1086 HQB 守门持久化 (主 21:15). ASI delta 是 inventory, 不是 ASI 本身 "
                "(主 17:43). 只读 V1074 artifacts, 写独立目录 (主 07-19 4 层安全门)."
            ),
        }


__all__ = [
    "V1086_VERSION",
    "GuardLogEntry",
    "HQBPersistence",
    "DEFAULT_ARTIFACT_DIR",
    "DEFAULT_GUARD_LOG",
]