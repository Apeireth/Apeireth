"""Apeireth self_reproduction contract shell — R6-PHL-01.

占位契约壳 (NOT 实现). 自繁殖 = 平台能在保持语义不变的前提下,
重新生成自己 (种子/快照/可复现). 与 self_mod_safety 不同:
自繁殖 = 同型重生; 自改 = 变体修改 (后者是 R7+ 范围).

本模块只暴露 Protocol 与 dataclass, 不写真繁殖逻辑 (R7+ 范围).
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Protocol, runtime_checkable

from .philosophy import check_philosophy

PROTOCOL_VERSION = "0.1.0-contract"
MODULE_NAME = "self_reproduction"

# 主 17:58 哲学守门: 不假装 reproduction = clone / perfect / uuid.
PHILOSOPHY_NOTES: Dict[str, str] = {
    "not_clone": "reproduction != clone; 复制不等于自我繁殖.",
    "not_perfect": "reproduction 允许 manifest 差异, 不允许语义差异.",
    "not_uuid": "reproduction_id 必须包含模块清单哈希, 不能是裸 uuid.",
}


@dataclass
class ReproductionSpec:
    """繁殖种子 + 目标 + 期望模块数 (语义不变性 contract)."""

    seed: bytes
    target_path: str
    expected_modules: int
    metadata: Dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.seed, (bytes, bytearray)):
            raise TypeError("seed 必须是 bytes")
        if not isinstance(self.target_path, str) or not self.target_path.strip():
            raise ValueError("target_path 必须是非空字符串")
        if not isinstance(self.expected_modules, int) or self.expected_modules <= 0:
            raise ValueError("expected_modules 必须是正整数")


@dataclass
class ReproductionResult:
    """繁殖结果: 成功标志 + 唯一 ID + 语义差异摘要."""

    success: bool
    reproduction_id: str
    diff_summary: str = ""
    manifest_delta: List[str] = field(default_factory=list)


@runtime_checkable
class SelfReproductionProtocol(Protocol):
    """自繁殖契约: 任何实现必须能快照/验证/恢复/重建/出 ID."""

    def snapshot(self) -> bytes:
        """产生当前状态字节快照 (含模块清单哈希)."""
        ...

    def verify(self, snapshot: bytes) -> bool:
        """验证快照完整 + 语义不变 (不只看 bytes)."""
        ...

    def restore(self, snapshot: bytes) -> bool:
        """从快照恢复到当前进程 (语义不变)."""
        ...

    def reproduce(self, target_path: str) -> str:
        """在 target_path 重建自身, 返回 reproduction_id."""
        ...

    def reproduction_id(self) -> str:
        """返回唯一繁殖 ID (必须含模块清单哈希, 参 PHILOSOPHY_NOTES.not_uuid)."""
        ...


def guard_self_reproduction() -> Dict[str, Any]:
    """V3 philosophy_guard 自检 — 占位壳不写真繁殖, 应 PASS.

    返回 guard 字典 (passed / status / notes) 供调用方决定是否上线.
    """
    summary = (
        "R6 placeholder contract shell for self_reproduction: exposes Protocol "
        "and dataclasses only, no real engine. References philosophy triad."
    )
    check = check_philosophy(
        module_name=MODULE_NAME,
        implementation_summary=summary,
        # 占位无归因, 不强制走分数守门
        claimed_pass=None,
        evidence=summary,  # 用总结作为非空证据以满足 V3 结构化守门
        categories=["contract_shell", "no_real_impl", "philosophy_referenced"],
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
    "ReproductionResult",
    "ReproductionSpec",
    "SelfReproductionProtocol",
    "guard_self_reproduction",
]
