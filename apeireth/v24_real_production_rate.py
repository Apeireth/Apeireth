"""Phase 81 v24_real_production_rate — V24 ASI 真生产率真测量 (主 17:33 主人真采纳 + 主 13:31).

主 17:33 "放手干到底" + 主 17:43 "实事求是"

借鉴 (主 13:08):
- V21 北极星 V0.1 透明公式真借鉴
- V19 集成测试真借鉴
- 真生产率 (主 17:43 实事求是)
"""
from __future__ import annotations

import subprocess
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List


V24_VERSION = "0.1.0"


@dataclass
class RealProductionMetric:
    """V24 真生产率真测量 (主 17:33 主人真采纳 + 主 17:43 实事求是)."""
    metric_id: str
    name: str
    value: float = 0.0
    unit: str = ""
    evidence: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "value": round(self.value, 4),
            "unit": self.unit,
            "evidence": self.evidence,
        }


def measure_n_commits(repo_dir: str = ".") -> RealProductionMetric:
    """V24 真生产 git commit 真测量 (主 17:33 + 主 17:43)."""
    try:
        result = subprocess.run(
            ["git", "log", "--oneline"],
            cwd=repo_dir,
            capture_output=True,
            timeout=10,
        )
        try:
            text = result.stdout.decode("utf-8", errors="ignore")
        except Exception:
            text = ""
        n = len(text.strip().splitlines()) if text else 0
    except Exception:
        n = 0
    return RealProductionMetric(
        metric_id=f"m_{uuid.uuid4().hex[:12]}",
        name="n_commits",
        value=float(n),
        unit="commits",
        evidence=f"git log --oneline 真实测量 = {n}",
    )


def measure_n_tests(tests_dir: str = "tests") -> RealProductionMetric:
    """V24 真生产 tests 真测量 (主 17:33 + 主 17:43)."""
    try:
        result = subprocess.run(
            ["python", "-m", "pytest", tests_dir, "--collect-only", "-q"],
            cwd=".",
            capture_output=True,
            timeout=60,
        )
        try:
            text = result.stdout.decode("utf-8", errors="ignore")
        except Exception:
            text = ""
        n = 0
        for line in text.splitlines():
            line = line.strip()
            if "::" in line and line.startswith("tests/"):
                n += 1
        if n == 0:
            for line in text.splitlines():
                if "tests collected" in line:
                    try:
                        n = int(line.split()[0])
                    except Exception:
                        pass
    except Exception:
        n = 0
    return RealProductionMetric(
        metric_id=f"m_{uuid.uuid4().hex[:12]}",
        name="n_tests",
        value=float(n),
        unit="tests",
        evidence=f"pytest --collect-only 真实测量 ≈ {n}",
    )


def measure_n_modules(apeireth_dir: str = "apeireth") -> RealProductionMetric:
    """V24 真生产模块真测量 (主 17:33 + 主 17:43)."""
    path = Path(apeireth_dir)
    n = len(list(path.glob("v*.py")))
    return RealProductionMetric(
        metric_id=f"m_{uuid.uuid4().hex[:12]}",
        name="n_v_modules",
        value=float(n),
        unit="modules",
        evidence=f"v*.py 真生产模块 glob = {n}",
    )


class V24RealProductionRate:
    """V24 ASI 真生产率真测量 (主 17:33 主人真采纳 + 主 17:43 实事求是).

    不刷 KPI, 真生产真测量. V21 V0.1 透明公式 real_production 项真来源.
    """

    def __init__(self):
        self.metrics: List[RealProductionMetric] = []

    def measure_all(self, repo_dir: str = ".", tests_dir: str = "tests",
                   apeireth_dir: str = "apeireth") -> List[RealProductionMetric]:
        """真生产全部真测量 (主 17:33 + 主 17:43 实事求是)."""
        self.metrics = [
            measure_n_commits(repo_dir),
            measure_n_tests(tests_dir),
            measure_n_modules(apeireth_dir),
        ]
        return self.metrics

    def render(self) -> str:
        """V24 真生产渲染 (主 17:33 + 主 17:43)."""
        lines = [
            "# ASI 真生产率真测量 (主 17:43 实事求是)",
            "",
            f"**真测量时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime())}",
            "",
            "| 指标 | 数值 | 单位 | 证据 |",
            "|------|------|------|------|",
        ]
        for m in self.metrics:
            d = m.to_dict()
            lines.append(f"| {d['name']} | {d['value']:.0f} | {d['unit']} | {d['evidence']} |")
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append("**主 17:43 实事求是**: 这些数字来自真实测量 (git log + pytest + glob).")
        lines.append("**主 13:31 大胆激进**: ASI 真生产率 = 真测量, 不刷 KPI.")
        return "\n".join(lines)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_metrics": len(self.metrics),
            "metrics": {m.name: m.value for m in self.metrics},
            "version": V24_VERSION,
            "philosophy": (
                "V24 ASI 真生产率真测量借鉴 (主 13:08 + 主 17:33 主人真采纳 + 主 17:43 实事求是): "
                "git log + pytest --collect-only + glob 真测量. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "主 17:33 放手干到底."
            ),
        }


__all__ = [
    "V24_VERSION",
    "RealProductionMetric",
    "measure_n_commits",
    "measure_n_tests",
    "measure_n_modules",
    "V24RealProductionRate",
]


def _demo():
    print("=" * 60)
    print("=== Phase 81 V24 真生产率真测量 (主 17:33 + 主 17:43) ===")
    print("=" * 60)

    m = V24RealProductionRate()
    m.measure_all()
    print(m.render())
    print("=" * 60)


if __name__ == "__main__":
    _demo()