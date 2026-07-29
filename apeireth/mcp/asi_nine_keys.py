"""apeireth.mcp.asi_nine_keys — ASI 9 键 LOCKED 真测注入 (主 22:33 + V3 守门).

继承 V1114 主哲学 9 键 + 加 MCP-specific 1 键 (production_is_not_autonomy) = 9 键 LOCKED.

V1123 把这 9 键作为强制校验:
  - 每个 tool 返回 content 之前, 注入 philosophy_guard 块
  - 每个 client 调用之前, 验证 9 键全部 True (主 17:43 实事求是)
  - 任何 1 键 False → 整个 dispatcher 拒服 (V3 守门)

借鉴 V1072 philosophy_guard 5 键 + V1097 actor_whitelist + V1114 主哲学 9 键.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


ASI_NINE_KEYS = (
    "not_undo",
    "not_proof",
    "not_safe",
    "not_clone",
    "not_perfect",
    "not_uuid",
    "spec_is_not_proof",
    "counterexample_is_not_bug",
    "production_is_not_autonomy",
)


# 每键的实义解释 (主 17:58 不假装: 公开语义, 便于审计)
ASI_NINE_KEYS_DOCS: Dict[str, str] = {
    "not_undo": "PHL-02b: 真演化不可撤销 (history 永久 + fork 不归并)",
    "not_proof": "PHL-02b: V1074 真测不是真生产的形式证明",
    "not_safe": "PHL-02b: 真生产 ≠ 真安全 (部署 ≠ 守门)",
    "not_clone": "PHL-01: 自我复制 ≠ 真自我 (Ricoeur narrative)",
    "not_perfect": "PHL-01: 不假装 V1123 完美 (缺陷必记录)",
    "not_uuid": "PHL-01: UUID ≠ 身份 (V1072 永恒身份核心更复杂)",
    "spec_is_not_proof": "PHL-03: 规范不是证明 (Anthropic 协议不是 ASI)",
    "counterexample_is_not_bug": "PHL-03: 反例不是缺陷 (Toulmin 反驳是知识)",
    "production_is_not_autonomy": "MCP-specific: 真生产 ≠ 自主 (V1114 自动评估 ≠ 自主意识)",
}


@dataclass
class AsiNineKeyLock:
    """ASI 9 键 LOCKED 真测注入 (主 22:33).

    默认全部 True (V1123 baseline). 调用方可以覆写, 但缺一不可.
    """

    values: Dict[str, bool] = field(default_factory=lambda: {k: True for k in ASI_NINE_KEYS})

    def __post_init__(self) -> None:
        # 守门: keys 必须是 ASI_NINE_KEYS 全集 (主 23:44 干到底)
        missing = [k for k in ASI_NINE_KEYS if k not in self.values]
        if missing:
            raise ValueError(f"AsiNineKeyLock missing keys: {missing}")

    def all_locked(self) -> bool:
        return all(self.values.get(k, False) for k in ASI_NINE_KEYS)

    def failed_keys(self) -> List[str]:
        return [k for k in ASI_NINE_KEYS if not self.values.get(k, False)]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "asi_nine_keys_locked": self.all_locked(),
            "values": dict(self.values),
            "failed_keys": self.failed_keys(),
            "n_locked": sum(1 for v in self.values.values() if v),
            "n_total": len(ASI_NINE_KEYS),
        }

    def to_guard_block(self) -> Dict[str, Any]:
        """注入 tool result 的 philosophy_guard 块 (MCP 规范允许 content[].data 自由结构)."""
        return {
            "asi_nine_keys_locked": self.all_locked(),
            "n_locked": sum(1 for v in self.values.values() if v),
            "n_total": len(ASI_NINE_KEYS),
            "keys": dict(self.values),
        }


def inject_guard_block(result: Dict[str, Any], lock: AsiNineKeyLock) -> Dict[str, Any]:
    """把 philosophy_guard 注入到 tool result 的 content[0].data (主 23:44 干到底).

    借鉴 V1097 写工具 fsync 后 success 的强制注入模式.
    """
    out = dict(result)
    content = list(out.get("content", []))
    if not content:
        content = [{"type": "json", "data": {}}]
    if content[0].get("type") == "json":
        data = dict(content[0].get("data", {}) or {})
        data["philosophy_guard"] = lock.to_guard_block()
        content[0] = {"type": "json", "data": data}
    elif content[0].get("type") == "text":
        text = content[0].get("text", "")
        content[0] = {
            "type": "json",
            "data": {"text": text, "philosophy_guard": lock.to_guard_block()},
        }
    else:
        # resource / unknown → 追加 json content
        content.append({"type": "json", "data": {"philosophy_guard": lock.to_guard_block()}})
    out["content"] = content
    return out


def verify_or_raise(lock: AsiNineKeyLock) -> None:
    """任一键失败 → 抛异常, 防止 dispatcher 进入 isError=False 路径 (主 23:44)."""
    if not lock.all_locked():
        failed = lock.failed_keys()
        raise RuntimeError(
            f"ASI 9 键 LOCKED 失败: {failed} (主 17:43 实事求是: dispatcher 拒服)"
        )


__all__ = [
    "ASI_NINE_KEYS", "ASI_NINE_KEYS_DOCS",
    "AsiNineKeyLock",
    "inject_guard_block", "verify_or_raise",
]
