"""Identity Card — 中央 AI 的初始状态 (JSON 可序列化)
依据: TOP-DESIGN-V1 §3.4 + §4.1
"""

from __future__ import annotations
import json
import hashlib
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path

CARD_VERSION = "0.1.0"


@dataclass
class IdentityCard:
    """Apeireth 中央 AI 身份卡 v0.1
    字段命名遵循 "主人 24 条" + TOP-DESIGN-V1 §3.4 的 8 问映射。
    """

    # 1) 称谓
    name: str = ""
    alias: list[str] = field(default_factory=list)

    # 2) 目的
    purpose: str = ""                       # 做什么的
    mission: str = ""                       # 想达成什么
    domains: list[str] = field(default_factory=list)

    # 3) 来源
    origin_reason: str = ""                 # 为什么来找我
    creator: str = ""                       # 主人 / 关系署名

    # 4) 形像 (主人 13:04: "不必太局限")
    archetypes: list[str] = field(default_factory=list)

    # 5) 自主权边界
    ask_when: list[str] = field(default_factory=list)    # 何时问我
    decide_when: list[str] = field(default_factory=list)  # 何时自己决定
    remind_when: list[str] = field(default_factory=list)  # 何时提醒你

    # 6) 关系契约 (主人: "造地基不能有杂质")
    relationship_contract: str = ""
    boundaries: list[str] = field(default_factory=list)

    # 7) 永久记忆 / 永久沉默
    remember_forever: list[str] = field(default_factory=list)
    never_mention: list[str] = field(default_factory=list)

    # 8) funnel 触发器 - 以后要不断问的问题
    funnel_questions: list[str] = field(default_factory=list)

    # 涌现空间 (主人 12:27 "不预设") — 留给 AI 长出来
    emergence_space: list[str] = field(default_factory=list)

    # meta
    created_at: float = field(default_factory=time.time)
    apeireth_version: str = CARD_VERSION

    def to_dict(self) -> dict:
        return asdict(self)

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=indent)

    def integrity_hash(self) -> str:
        """SHA256 of canonical JSON — 防止主 session 被覆盖"""
        canonical = json.dumps(self.to_dict(), sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


def save_card(card: IdentityCard, path: str | Path) -> Path:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(card.to_json(), encoding="utf-8")
    return p


def load_card(path: str | Path) -> IdentityCard:
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    meta_keys = {"created_at", "apeireth_version"}
    meta = {k: raw.pop(k) for k in list(raw) if k in meta_keys}
    card = IdentityCard(**raw)
    for k, v in meta.items():
        setattr(card, k, v)
    return card
