"""R13 MVP — IdentityCard (主人真实身份背景 + 演化).

Ponytail ceiling: JSON file persistence + fields seeded from公开可查 + 主人已知.
When Phase 1.3 lands, IdentityCard consolidates from Notes.

公开可查 + 主人已知 (主 22:33 / 主 23:09 实事求是):
- 地方出生, 老家养老问题长期关注
- 
- AgentMemory 自研方向
- 
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import List, Optional, Dict, Any

DEFAULT_CARD = Path.home() / ".apeireth_mvp" / "identity_card.json"


@dataclass
class IdentityCard:
    """主人身份 + Agent 角色卡. JSON 持久化, 跨 session."""
    version: str = "0.1.0"
    owner_id: str = field(default_factory=lambda: uuid.uuid4().hex[:12])
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)

    # 主人真实背景 (种子事实, 演化从 Phase 1.3 开始)
    owner_background: List[str] = field(default_factory=lambda: [
        "",  # 公开
        "老家养老问题长期关注",  # 公开 + 已知
        "",  # 公开
        "AgentMemory 自研方向",  # 已知
        "",  # 公开
    ])
    owner_values: List[str] = field(default_factory=lambda: [
        "实事求是 (主 17:43 + 主 17:58)",  # 不刷 KPI
        "干到底 (主 23:44 + 主 23:09)",  # 不半途而废
        "借鉴而非闭门 (主 19:33)",  # 真理 > 闭门造车
        "不刷 KPI",  # 主 17:43
        "不假装达到 ASI",  # 主 17:58 + 主 20:46
    ])
    owner_red_lines: List[str] = field(default_factory=lambda: [
        "不重写 V0.5 公式",
        "不重做 V1136 真测引擎",
        "不重写哲学守门",
        "不写 ASI 北极星公式",
    ])

    # Agent 角色 (MVP 阶段: 仅 echo + retrieve)
    agent_role: str = "R13 MVP CLI agent — 跨 session 记忆 + 检索"
    agent_capabilities: List[str] = field(default_factory=lambda: [
        "Episode append-only 存储",
        "Note 合并 / 遗忘",
        "BM25 + salience 检索",
        "IdentityCard 演化 (Phase 1.3)",
    ])

    # 演化记录 (Phase 1.3 consolidate 时追加)
    evolution_log: List[Dict[str, Any]] = field(default_factory=list)

    # 自由扩展字段 (Phase 1.3 起 Note → Card consolidation 写入)
    custom: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return asdict(self)

    def touch(self) -> None:
        self.updated_at = time.time()

    def evolve(self, key: str, value: Any, source: str = "user") -> None:
        """Ponytail: simple key-based evolution. Phase 1.3 会改用 LLM 提炼."""
        self.touch()
        entry = {"ts": self.updated_at, "key": key, "value": value,
                 "source": source}
        self.evolution_log.append(entry)
        if key.startswith("owner_background."):
            field_name = key.split(".", 1)[1]
            if field_name not in self.owner_background:
                self.owner_background.append(field_name)
        elif key.startswith("owner_values."):
            field_name = key.split(".", 1)[1]
            if field_name not in self.owner_values:
                self.owner_values.append(field_name)
        elif key.startswith("agent_capabilities."):
            field_name = key.split(".", 1)[1]
            if field_name not in self.agent_capabilities:
                self.agent_capabilities.append(field_name)
        else:
            # arbitrary custom key
            self.custom[key] = value


def load(path: Optional[Path] = None) -> IdentityCard:
    # Ponytail: default arg must be evaluated at call time so monkeypatch
    # of DEFAULT_CARD takes effect during tests.
    p = Path(path) if path is not None else DEFAULT_CARD
    if not p.exists():
        card = IdentityCard()
        save(card, p)
        return card
    data = json.loads(p.read_text(encoding="utf-8"))
    return IdentityCard(**data)


def save(card: IdentityCard, path: Optional[Path] = None) -> None:
    p = Path(path) if path is not None else DEFAULT_CARD
    p.parent.mkdir(parents=True, exist_ok=True)
    card.touch()
    p.write_text(
        json.dumps(card.to_dict(), ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def render(card: IdentityCard) -> str:
    """Ponytail: human-readable summary for CLI."""
    lines = [
        f"IdentityCard v{card.version} (owner_id={card.owner_id})",
        f"  created: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(card.created_at))}",
        f"  updated: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(card.updated_at))}",
        "",
        "  owner_background:",
    ]
    for b in card.owner_background:
        lines.append(f"    - {b}")
    lines.append("")
    lines.append("  owner_values:")
    for v in card.owner_values:
        lines.append(f"    - {v}")
    lines.append("")
    lines.append("  owner_red_lines:")
    for r in card.owner_red_lines:
        lines.append(f"    - {r}")
    lines.append("")
    lines.append(f"  agent_role: {card.agent_role}")
    lines.append("  agent_capabilities:")
    for c in card.agent_capabilities:
        lines.append(f"    - {c}")
    if card.evolution_log:
        lines.append("")
        lines.append(f"  evolution_log ({len(card.evolution_log)} entries):")
        for e in card.evolution_log[-3:]:
            lines.append(f"    [{time.strftime('%H:%M:%S', time.localtime(e['ts']))}]"
                         f" {e['key']}={e['value']!r} (src={e['source']})")
    return "\n".join(lines)