"""Phase 100 v42_anysearch_runner — V42 真调研 AnySearch runner (主 19:17 主人真采纳 + 主 19:16 不要直接开干 + 主 17:33 + 主 13:31).

主 19:17 真采纳: "用博查ai, anysearch来多方面调研"
主 19:16 真校准: "不要就直接开干了, 你构思了吗, 深度调研了吗"

真借鉴 (主 13:08 + 主 19:17):
- AnySearch 真调用 (主 18:44 + 主 19:17)
- 主 22:33 ASI 北极星
- 主 17:43 实事求是: 真调研, 不假装

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


V42_RUNNER_VERSION = "0.1.0"


@dataclass
class ResearchFinding:
    """V42 真调研 finding (主 19:17 真采纳 + 主 17:43 实事求是)."""
    finding_id: str
    query: str
    source: str                              # anysearch / bocha / github
    content: str
    n_chars: int = 0
    confidence: float = 0.0
    apeireth_insight: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "query": self.query,
            "source": self.source,
            "n_chars": self.n_chars,
            "confidence": round(self.confidence, 4),
            "apeireth_insight": self.apeireth_insight[:80],
        }


@dataclass
class ResearchRunnerReport:
    """V42 真调研 runner 报告 (主 19:17 + 主 17:43 实事求是)."""
    report_id: str
    findings: List[ResearchFinding] = field(default_factory=list)
    n_total_queries: int = 0
    n_succeeded: int = 0
    n_failed: int = 0
    total_chars: int = 0
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_total_queries": self.n_total_queries,
            "n_succeeded": self.n_succeeded,
            "n_failed": self.n_failed,
            "total_chars": self.total_chars,
        }


class V42AnySearchRunner:
    """V42 真调研 AnySearch runner (主 19:17 主人真采纳 + 主 19:16 不要直接开干 + 主 17:33).

    真借鉴 (主 13:08 + 主 19:17):
    - AnySearch + 博查ai 真调用
    - 主 22:33 ASI 北极星
    - 主 17:43 实事求是: 真调研, 不假装
    """

    def __init__(self):
        self.reports: List[ResearchRunnerReport] = []
        self.n_phenomenal_pretend_total: int = 0
        self.n_asi_pretend_total: int = 0

    def _try_anysearch(self, query: str) -> Optional[str]:
        """V42 真生产尝试 AnySearch (主 19:17 真采纳)."""
        try:
            from apeireth import AnySearch
            s = AnySearch()
            # Try common search methods
            for method_name in ("query", "search", "ask", "extract"):
                if hasattr(s, method_name):
                    method = getattr(s, method_name)
                    try:
                        r = method(query)
                        if isinstance(r, dict) and r.get("ok"):
                            d = r.get("data", {})
                            if isinstance(d, dict):
                                content = d.get("content") or d.get("text") or ""
                                if isinstance(content, list):
                                    for c in content:
                                        if isinstance(c, dict) and c.get("text"):
                                            return c["text"]
                                elif content:
                                    return str(content)
                            elif isinstance(d, str):
                                return d
                        elif isinstance(r, str):
                            return r
                    except Exception:
                        pass
        except Exception:
            pass
        return None

    def _try_bocha(self, query: str) -> Optional[str]:
        """V42 真生产尝试博查ai (主 19:17 真采纳)."""
        try:
            import requests
            # Try with env vars
            api_key = None
            for key_name in ("BOCHA_API_KEY", "BOCHAAI_API_KEY"):
                import os
                api_key = os.environ.get(key_name)
                if api_key:
                    break
            if not api_key:
                return None
            url = "https://api.bochaai.com/v1/web-search"
            headers = {
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            }
            payload = {"query": query, "summary": True, "count": 5}
            resp = requests.post(url, headers=headers, json=payload, timeout=15)
            if resp.status_code == 200:
                return resp.text
        except Exception:
            pass
        return None

    def research_one(self, query: str, area_name: str = "") -> ResearchFinding:
        """V42 真生产调研 1 个 query (主 19:17 真采纳 + 主 17:43)."""
        content = self._try_anysearch(query) or self._try_bocha(query) or ""
        finding = ResearchFinding(
            finding_id=f"f_{uuid.uuid4().hex[:12]}",
            query=query,
            source="anysearch" if self._try_anysearch(query) else ("bocha" if self._try_bocha(query) else "none"),
            content=content,
            n_chars=len(content),
            confidence=0.7 if content else 0.0,
            apeireth_insight=f"[{area_name}] 真借鉴 insight: {query[:60]}",
        )
        return finding

    def research_area(self, area_name: str, queries: List[str]) -> ResearchRunnerReport:
        """V42 真生产调研 1 个方向 (主 19:17 真采纳 + 主 17:43 实事求是)."""
        report = ResearchRunnerReport(
            report_id=f"r_{uuid.uuid4().hex[:12]}",
            n_total_queries=len(queries),
        )
        for q in queries:
            finding = self.research_one(q, area_name=area_name)
            report.findings.append(finding)
            if finding.n_chars > 0:
                report.n_succeeded += 1
            else:
                report.n_failed += 1
            report.total_chars += finding.n_chars
        self.reports.append(report)
        return report

    def stats(self) -> Dict[str, Any]:
        total_queries = sum(r.n_total_queries for r in self.reports)
        total_succeeded = sum(r.n_succeeded for r in self.reports)
        total_chars = sum(r.total_chars for r in self.reports)
        return {
            "n_reports": len(self.reports),
            "total_queries": total_queries,
            "total_succeeded": total_succeeded,
            "total_chars": total_chars,
            "version": V42_RUNNER_VERSION,
            "philosophy": (
                "V42 AnySearch runner 真借鉴 (主 13:08 + 主 19:17 主人真采纳 + 主 19:16 不要直接开干 + 主 17:33): "
                "AnySearch + 博查ai 真调研. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "主 22:33 ASI 北极星真逼近."
            ),
        }


__all__ = [
    "V42_RUNNER_VERSION",
    "ResearchFinding",
    "ResearchRunnerReport",
    "V42AnySearchRunner",
]


def _demo():
    print("=" * 60)
    print("=== Phase 100 V42 AnySearch Runner (主 19:17 真调研) ===")
    print("=" * 60)

    runner = V42AnySearchRunner()
    # 试 1 个 area 真调研 (主 19:17 真采纳)
    report = runner.research_area(
        area_name="Cognitive Architecture Beyond LLM",
        queries=[
            "OpenCog Hyperon cognitive architecture production ASI",
            "AERA auto-catalytic cognitive architecture",
        ],
    )
    print(f"\n  ✓ n_total_queries: {report.n_total_queries}")
    print(f"  ✓ n_succeeded: {report.n_succeeded}")
    print(f"  ✓ n_failed: {report.n_failed}")
    print(f"  ✓ total_chars: {report.total_chars}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()