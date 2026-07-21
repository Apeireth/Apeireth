"""Phase 82 v25_production_history — V25 ASI 真生产率持续测量 + 增长曲线 (主 17:33 主人真采纳 + 主 17:43 实事求是).

主 17:33 "放手干到底" + 主 17:43 "实事求是"

借鉴 (主 13:08):
- V24 真生产率真测量真借鉴
- 主 22:33 ASI 北极星真借鉴
- 真生产率 (主 17:43 实事求是)
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List


V25_VERSION = "0.1.0"


@dataclass
class ProductionSnapshot:
    """V25 真生产率快照 (主 17:33 + 主 17:43)."""
    snapshot_id: str
    label: str                          # 真生产 checkpoint 标签
    n_commits: int = 0
    n_tests: int = 0
    n_modules: int = 0
    n_doc_md: int = 0
    n_total_lines: int = 0
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "label": self.label,
            "n_commits": self.n_commits,
            "n_tests": self.n_tests,
            "n_modules": self.n_modules,
            "n_doc_md": self.n_doc_md,
            "n_total_lines": self.n_total_lines,
            "ts": round(self.ts, 2),
        }


def count_lines_python(dir_path: str = "apeireth") -> int:
    """V25 真生产 Python 行数真测量 (主 17:43 实事求是)."""
    total = 0
    for path in Path(dir_path).glob("v*.py"):
        try:
            total += sum(1 for _ in path.open("r", encoding="utf-8", errors="ignore"))
        except Exception:
            pass
    return total


def count_doc_md(dir_path: str = ".") -> int:
    """V25 真生产 markdown 文档真测量 (主 17:43 实事求是)."""
    n = 0
    for path in Path(dir_path).glob("*.md"):
        name = path.name.upper()
        if name.startswith(("ASI-", "APEIRETH-", "V3-", "PHASE-", "README")) or "ASI" in name or "APEIRETH" in name:
            n += 1
    return n


def take_snapshot(label: str = "checkpoint",
                  repo_dir: str = ".",
                  tests_dir: str = "tests",
                  apeireth_dir: str = "apeireth") -> ProductionSnapshot:
    """V25 真生产快照 (主 17:33 + 主 17:43)."""
    import subprocess
    try:
        result = subprocess.run(
            ["git", "log", "--oneline"],
            cwd=repo_dir,
            capture_output=True,
            timeout=10,
        )
        text = (result.stdout or b"").decode("utf-8", errors="ignore")
        n_commits = len([l for l in text.splitlines() if l.strip()])
    except Exception:
        n_commits = 0
    try:
        result = subprocess.run(
            ["python", "-m", "pytest", tests_dir, "--collect-only", "-q"],
            cwd=".",
            capture_output=True,
            timeout=60,
        )
        text = (result.stdout or b"").decode("utf-8", errors="ignore")
        n_tests = 0
        for line in text.splitlines():
            if "tests collected" in line:
                for tok in line.split():
                    try:
                        n_tests = int(tok)
                        break
                    except Exception:
                        pass
                break
    except Exception:
        n_tests = 0
    n_modules = len(list(Path(apeireth_dir).glob("v*.py")))
    n_doc_md = count_doc_md()
    n_total_lines = count_lines_python(apeireth_dir)
    return ProductionSnapshot(
        snapshot_id=f"s_{uuid.uuid4().hex[:12]}",
        label=label,
        n_commits=n_commits,
        n_tests=n_tests,
        n_modules=n_modules,
        n_doc_md=n_doc_md,
        n_total_lines=n_total_lines,
    )


class V25ProductionHistory:
    """V25 ASI 真生产率持续测量 (主 17:33 + 主 17:43 实事求是)."""

    def __init__(self, history_file: str = ".apeireth_production_history.json"):
        self.history_file = Path(history_file)
        self.snapshots: List[ProductionSnapshot] = []
        self._load()

    def _load(self) -> None:
        """真生产加载历史 (主 17:43 实事求是)."""
        if self.history_file.exists():
            try:
                data = json.loads(self.history_file.read_text(encoding="utf-8"))
                for snap in data.get("snapshots", []):
                    self.snapshots.append(ProductionSnapshot(
                        snapshot_id=snap.get("snapshot_id", f"s_{uuid.uuid4().hex[:12]}"),
                        label=snap.get("label", "checkpoint"),
                        n_commits=snap.get("n_commits", 0),
                        n_tests=snap.get("n_tests", 0),
                        n_modules=snap.get("n_modules", 0),
                        n_doc_md=snap.get("n_doc_md", 0),
                        n_total_lines=snap.get("n_total_lines", 0),
                        ts=snap.get("ts", time.time()),
                    ))
            except Exception:
                pass

    def _save(self) -> None:
        """真生产持久化历史 (主 17:43 实事求是)."""
        try:
            data = {"snapshots": [s.to_dict() for s in self.snapshots]}
            self.history_file.write_text(
                json.dumps(data, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
        except Exception:
            pass

    def add_snapshot(self, snapshot: ProductionSnapshot) -> None:
        """真生产加快照 (主 17:33)."""
        self.snapshots.append(snapshot)
        self._save()

    def take_and_store(self, label: str = "checkpoint") -> ProductionSnapshot:
        """真生产快照并存储 (主 17:33)."""
        snapshot = take_snapshot(label=label)
        self.add_snapshot(snapshot)
        return snapshot

    def growth(self) -> Dict[str, float]:
        """真生产增长曲线 (主 17:43 实事求是)."""
        if len(self.snapshots) < 2:
            return {"commits_growth": 0.0, "tests_growth": 0.0, "modules_growth": 0.0}
        first, last = self.snapshots[0], self.snapshots[-1]
        return {
            "commits_growth": float(last.n_commits - first.n_commits),
            "tests_growth": float(last.n_tests - first.n_tests),
            "modules_growth": float(last.n_modules - first.n_modules),
            "duration_seconds": round(last.ts - first.ts, 2),
        }

    def render(self) -> str:
        """V25 真生产渲染 (主 17:33)."""
        lines = [
            "# ASI 真生产率持续测量 + 增长曲线 (主 17:43 实事求是)",
            "",
            f"**真测量时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime())}",
            f"**总快照数**: {len(self.snapshots)}",
            "",
            "| label | commits | tests | modules | doc_md | lines | ts |",
            "|-------|---------|-------|---------|--------|-------|----|",
        ]
        for s in self.snapshots:
            d = s.to_dict()
            lines.append(
                f"| {d['label']} | {d['n_commits']} | {d['n_tests']} | "
                f"{d['n_modules']} | {d['n_doc_md']} | {d['n_total_lines']} | "
                f"{time.strftime('%H:%M:%S', time.localtime(d['ts']))} |"
            )
        growth = self.growth()
        lines.append("")
        lines.append(f"**增长**: commits +{growth['commits_growth']}, "
                     f"tests +{growth['tests_growth']}, modules +{growth['modules_growth']}, "
                     f"duration {growth['duration_seconds']}s")
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append("**主 17:43 实事求是**: 增长曲线来自真实快照.")
        lines.append("**主 13:31 大胆激进**: ASI 真生产率 = 真测量 + 真增长.")
        return "\n".join(lines)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_snapshots": len(self.snapshots),
            "latest": self.snapshots[-1].to_dict() if self.snapshots else None,
            "growth": self.growth(),
            "version": V25_VERSION,
            "philosophy": (
                "V25 ASI 真生产率持续测量 + 增长曲线借鉴 (主 13:08 + 主 17:33 主人真采纳 + 主 17:43 实事求是): "
                "git log + pytest + glob + 行数真测量, 增长曲线真计算. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "主 17:33 放手干到底."
            ),
        }


__all__ = [
    "V25_VERSION",
    "ProductionSnapshot",
    "take_snapshot",
    "count_lines_python",
    "count_doc_md",
    "V25ProductionHistory",
]


def _demo():
    print("=" * 60)
    print("=== Phase 82 V25 真生产率持续测量 (主 17:33 + 主 17:43) ===")
    print("=" * 60)

    h = V25ProductionHistory()
    h.take_and_store(label="v25_initial")
    h.take_and_store(label="v25_after")
    print(h.render())
    print(f"\n  ✓ stats: {h.stats()}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()