"""Apeireth self_mod_safety contract shell — R6-PHL-02.

占位契约壳 (NOT 实现). 自改安全 = 变体修改的安全边界.
self_reproduction (R6-PHL-01 同型重生, replica safety) ≠ self_mod_safety (variant safety — 本模块).
本模块只暴露 Protocol + dataclass + guard, 不写真改逻辑 (R7+ 范围).
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Protocol, runtime_checkable

from .philosophy import check_philosophy

PROTOCOL_VERSION = "0.1.0-contract"
MODULE_NAME = "self_mod_safety"

# 主 17:58 哲学守门: 不假装 rollback / verify / dry_run.
PHILOSOPHY_NOTES: Dict[str, str] = {
    "not_undo": "rollback != undo; rollback 是状态恢复, 不是撤销历史.",
    "not_proof": "verify 是 heuristic 校验, 不是形式化证明 (那是 formal_verify).",
    "not_safe": "dry_run 可能与真跑有差异; dry_run 不等于 safe.",
}


@dataclass
class Checkpoint:
    """检查点引用 — 落盘 ID + 时间戳 + 范围 (R6-PHL-02 契约)."""
    label: str
    checkpoint_id: str
    ts: float
    scope: str = "module"

    def __post_init__(self) -> None:
        if not isinstance(self.label, str) or not self.label.strip():
            raise ValueError("label 必须是非空字符串")
        if not isinstance(self.checkpoint_id, str) or not self.checkpoint_id.strip():
            raise ValueError("checkpoint_id 必须是非空字符串")
        if not isinstance(self.scope, str) or not self.scope.strip():
            raise ValueError("scope 必须是非空字符串")


@dataclass
class SafetyVerification:
    """自改安全性验证结果 — 含 risk_score (连续 0-1) 与 rationale."""
    mutation_id: str
    verified: bool
    risk_score: float
    rationale: str = ""

    def __post_init__(self) -> None:
        if not isinstance(self.mutation_id, str) or not self.mutation_id.strip():
            raise ValueError("mutation_id 必须是非空字符串")
        if not isinstance(self.verified, bool):
            raise TypeError("verified 必须是 bool")
        if not isinstance(self.risk_score, (int, float)) or not 0.0 <= self.risk_score <= 1.0:
            raise ValueError("risk_score 必须是 [0.0, 1.0] 的数")


@dataclass
class DryRunResult:
    """干跑结果 — 预期影响 + 副作用列表 (干跑 ≠ 真跑, 参 PHILOSOPHY_NOTES.not_safe)."""
    mutation_id: str
    expected_impact: Dict[str, Any] = field(default_factory=dict)
    side_effects: List[str] = field(default_factory=list)


@runtime_checkable
class SelfModSafetyProtocol(Protocol):
    """自改安全契约: 5 方法 (snapshot/checkpoint/rollback/verify/dry_run)."""
    def snapshot(self) -> bytes:
        """拍当前状态快照 (bytes, 含必要元数据)."""
        ...

    def checkpoint(self, label: str) -> str:
        """打检查点, 返回 checkpoint_id (后续 rollback 用)."""
        ...

    def rollback(self, checkpoint_id: str) -> bool:
        """回滚到指定检查点 (状态恢复, 不是撤销历史)."""
        ...

    def verify(self, code: bytes) -> bool:
        """验证修改是否安全 (heuristic, 非形式化 — 参 PHILOSOPHY_NOTES.not_proof)."""
        ...

    def dry_run(self, mutation: Dict) -> Dict:
        """干跑修改, 返回预期影响 (可能与真跑有差异 — 参 PHILOSOPHY_NOTES.not_safe)."""
        ...


def guard_self_mod_safety() -> Dict[str, Any]:
    """V3 philosophy_guard 自检 — 占位壳不写真改, 应 PASS."""
    summary = (
        "R6 placeholder contract shell for self_mod_safety: 5-method Protocol "
        "(snapshot/checkpoint/rollback/verify/dry_run) + 3 dataclass + guard. "
        "Distinct from self_reproduction (variant vs replica). No real engine (R7+)."
    )
    check = check_philosophy(
        module_name=MODULE_NAME,
        implementation_summary=summary,
        claimed_pass=None,
        evidence=summary,
        categories=["contract_shell", "no_real_impl", "philosophy_referenced", "distinct_from_reproduction"],
        required_categories=["contract_shell", "no_real_impl", "philosophy_referenced"],
    )
    return {
        "module": MODULE_NAME,
        "protocol_version": PROTOCOL_VERSION,
        "guard_passed": check.passed,
        "guard_status": check.status,
        "guard_notes": dict(PHILOSOPHY_NOTES),
        "deviation_count": len(check.deviations),
    }


__all__ = [
    "MODULE_NAME",
    "PHILOSOPHY_NOTES",
    "PROTOCOL_VERSION",
    "Checkpoint",
    "SafetyVerification",
    "DryRunResult",
    "SelfModSafetyProtocol",
    "guard_self_mod_safety",
]
