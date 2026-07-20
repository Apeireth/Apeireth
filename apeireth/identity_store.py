"""Identity Store v0.2 — JSON Schema 验证 + 版本迁移 + 多卡管理

依据: TOP-DESIGN-V1 §3.4 + §4.1 + Phase 6 路线
主人 13:04 "造地基不能有杂质" → Schema 一次定型
主人 12:27 "立场自然成长, AI 自然思考, 平台不给予" → emergence_space 不强约束

Phase 6 (L5 涌现空间 + 自组织临时团) 需要多张身份卡:
- 1 张中央 AI 主卡 (master)
- N 张涌现 persona 卡 (调度者/学习者/思考者/助手...)
- M 张临时团卡 (Phase 6 启动后自动生成)

v0.1 → v0.2 增量:
1. JSON Schema 验证 (Pydantic-style 轻量, 无外部依赖)
2. 版本迁移 v0.1.0 → v0.2.0 (新增 recall_anchor / evidence_refs 字段, 默认值)
3. IdentityStore 多卡容器 (load_all / save_all / get_master)
4. 完整性自检 — 加载时验证 hash, 防覆盖
"""

from __future__ import annotations
import json
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from .identity import IdentityCard, CARD_VERSION

IDENTITY_STORE_VERSION = "0.2.0"


# === 1. Schema 定义 — 一份人类可读的契约 ===

# 字段元数据: name -> (kind, required, description)
# kind: 'str' | 'list[str]' | 'float'
FIELD_SCHEMA = {
    # Q1
    "name":            ("str",      True,  "中心节点的标签"),
    "alias":           ("list[str]", False, "别名 (Q1 衍生)"),
    # Q2
    "purpose":         ("str",      True,  "做什么的"),
    "mission":         ("str",      False, "想达成什么 (Q2 衍生)"),
    "domains":         ("list[str]", False, "角色域 (Q2 衍生)"),
    # Q3
    "origin_reason":   ("str",      True,  "为什么来找我 — 上游因果"),
    "creator":         ("str",      False, "主人 / 关系署名 (Q3 衍生)"),
    # Q4
    "archetypes":      ("list[str]", False, "形像列表 (主人 13:04 不必太局限)"),
    # Q5
    "ask_when":        ("list[str]", False, "何时问我"),
    "decide_when":     ("list[str]", False, "何时自己决定"),
    "remind_when":     ("list[str]", False, "何时提醒你"),
    # Q6
    "relationship_contract": ("str", False, "关系契约 (主人: 不能有杂质)"),
    "boundaries":      ("list[str]", False, "边界清单"),
    # Q7
    "remember_forever": ("list[str]", False, "永久记忆"),
    "never_mention":   ("list[str]", False, "永久沉默"),
    # Q8
    "funnel_questions": ("list[str]", False, "Funnel 触发器 — 永远跑"),
    # emergence
    "emergence_space": ("list[str]", False, "留给 AI 长出来的涌现空间"),
    # v0.2 新字段 — 不打破 v0.1 卡, 给默认值
    "recall_anchor":   ("str",      False, "[v0.2] 一句话锚定 — 危急时 recall 用"),
    "evidence_refs":   ("list[str]", False, "[v0.2] 证据引用 — eid/nid/nid 锚点"),
    # meta
    "created_at":      ("float",    False, "创建时间戳"),
    "apeireth_version": ("str",     False, "卡版本"),
}


# === 2. Schema 验证器 (无外部依赖, 纯 stdlib) ===

class SchemaError(Exception):
    """Schema 校验失败 — 不抛给主循环, 但报告给主人"""


def _validate_field(name: str, value, kind: str, required: bool) -> list[str]:
    issues = []
    if value is None or (isinstance(value, (str, list)) and len(value) == 0):
        if required:
            issues.append(f"[required-empty] {name}")
        return issues  # 空就放过 (可选字段)
    if kind == "str" and not isinstance(value, str):
        issues.append(f"[type-mismatch] {name}: expected str, got {type(value).__name__}")
    elif kind == "list[str]":
        if not isinstance(value, list):
            issues.append(f"[type-mismatch] {name}: expected list, got {type(value).__name__}")
        elif not all(isinstance(x, str) for x in value):
            issues.append(f"[element-type] {name}: list elements must be str")
    elif kind == "float" and not isinstance(value, (int, float)):
        issues.append(f"[type-mismatch] {name}: expected float, got {type(value).__name__}")
    return issues


def validate_card(card: IdentityCard, strict: bool = False) -> list[str]:
    """校验 IdentityCard 是否符合 schema。返回问题列表 (空 = 完美)。

    strict=True: required 字段为空也算错 (Phase 6 涌现 persona 卡可走 strict=False)
    """
    issues = []
    raw = card.to_dict()
    for fname, (kind, required, _desc) in FIELD_SCHEMA.items():
        if fname not in raw:
            if required:
                issues.append(f"[missing] {fname}")
            continue
        # strict 才把"空"当错误
        if strict:
            required = required or True  # strict 模式所有 schema 字段都看
        issues.extend(_validate_field(fname, raw[fname], kind, required))
    return issues


# === 3. 版本迁移 — v0.1.0 → v0.2.0 ===

def migrate_v01_to_v02(raw: dict) -> dict:
    """v0.1.0 → v0.2.0 — 加 recall_anchor / evidence_refs 字段 (默认值)

    主人 13:04 "造地基不能有杂质": 迁移要确定性, 不要丢字段, 不要破坏 hash
    """
    new_fields = {
        "recall_anchor": raw.get("recall_anchor", ""),
        "evidence_refs": raw.get("evidence_refs", []),
    }
    migrated = {**raw, **new_fields, "apeireth_version": "0.2.0"}
    return migrated


def migrate_card(raw: dict) -> tuple[dict, list[str]]:
    """统一迁移入口 — 从任意 v0.1.x 迁到 v0.2.0

    返回 (迁移后 dict, 警告列表)
    """
    notes = []
    version = raw.get("apeireth_version", "0.1.0")
    # 解析主版本号, 不绑死补丁号
    major_minor = ".".join(version.split(".")[:2])
    if major_minor == "0.1":
        notes.append(f"migrating {version} → {IDENTITY_STORE_VERSION}")
        raw = migrate_v01_to_v02(raw)
    elif major_minor == "0.2":
        notes.append(f"already at {version}, no migration needed")
    else:
        notes.append(f"[warn] unknown version {version}, attempting best-effort load")
        # best-effort: 补缺失字段
        for fname, (kind, _req, _desc) in FIELD_SCHEMA.items():
            if fname not in raw:
                raw[fname] = "" if kind == "str" else []
        raw["apeireth_version"] = IDENTITY_STORE_VERSION
    return raw, notes


# === 4. IdentityStore — 多卡容器 ===

@dataclass
class StoreEntry:
    """一张卡的容器 — 含 metadata (path / role / integrity)"""
    card: IdentityCard
    role: str                     # 'master' | 'persona' | 'team' | 'snapshot'
    path: Optional[Path] = None   # 磁盘路径 (None = 仅内存)
    loaded_at: float = field(default_factory=time.time)
    integrity_ok: bool = True     # 加载时 hash 是否对得上
    migration_notes: list = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            'card': self.card.to_dict(),
            'role': self.role,
            'integrity_ok': self.integrity_ok,
            'migration_notes': self.migration_notes,
            'loaded_at': self.loaded_at,
        }


class IdentityStore:
    """多卡容器 — 给 Phase 6 (涌现 + 自组织临时团) 用的根容器

    设计原则:
    - master 卡只 1 张 (中央 AI)
    - persona 卡 N 张 (Phase 4 多身份)
    - team 卡 M 张 (Phase 6 临时团 — 任务来了临时组装)
    - 加载时自动迁移 + 完整性校验
    """

    def __init__(self, root: Optional[str | Path] = None):
        self.root = Path(root) if root else None
        self.entries: dict[str, StoreEntry] = {}  # name -> StoreEntry

    def add(self, card: IdentityCard, role: str = "snapshot") -> StoreEntry:
        """添加一张卡到内存 store"""
        if not card.name:
            raise ValueError("card.name is required for store key")
        if card.name in self.entries and role == "master":
            raise ValueError(f"master card '{card.name}' already exists")
        entry = StoreEntry(card=card, role=role, integrity_ok=True)
        self.entries[card.name] = entry
        return entry

    def get(self, name: str) -> Optional[IdentityCard]:
        e = self.entries.get(name)
        return e.card if e else None

    def master(self) -> Optional[IdentityCard]:
        for e in self.entries.values():
            if e.role == "master":
                return e.card
        return None

    def personas(self) -> list[IdentityCard]:
        return [e.card for e in self.entries.values() if e.role == "persona"]

    def teams(self) -> list[IdentityCard]:
        return [e.card for e in self.entries.values() if e.role == "team"]

    def stats(self) -> dict:
        by_role: dict = {}
        for e in self.entries.values():
            by_role[e.role] = by_role.get(e.role, 0) + 1
        return {
            'total': len(self.entries),
            'by_role': by_role,
            'store_version': IDENTITY_STORE_VERSION,
        }

    # --- 磁盘 I/O ---

    def load_dir(self, directory: str | Path) -> list[str]:
        """从目录加载所有 *.identity.json — 自动迁移 + 校验

        返回加载日志 (含迁移 note + 完整性警告)
        """
        log = []
        d = Path(directory)
        if not d.exists():
            log.append(f"[skip] {d} does not exist")
            return log
        for p in sorted(d.glob("*.identity.json")):
            try:
                raw = json.loads(p.read_text(encoding="utf-8"))
            except Exception as e:
                log.append(f"[error] {p.name}: parse failed: {e}")
                continue
            migrated, notes = migrate_card(raw)
            log.extend([f"[{p.name}] {n}" for n in notes])
            # 弹掉 non-schema 字段 (IdentityCard 不认)
            role = migrated.pop("_role", "snapshot")
            expected_hash = migrated.pop("integrity_hash", None)
            try:
                card = IdentityCard(**migrated)
            except Exception as e:
                log.append(f"[error] {p.name}: construct failed: {e}")
                continue
            # 完整性检查
            integrity_ok = True
            if expected_hash is not None:
                actual = card.integrity_hash()
                if expected_hash != actual:
                    log.append(f"[warn] {p.name}: integrity mismatch ({expected_hash} vs {actual})")
                    integrity_ok = False
            entry = StoreEntry(
                card=card, role=role, path=p,
                integrity_ok=integrity_ok, migration_notes=notes,
            )
            self.entries[card.name or p.stem] = entry
            log.append(f"[ok] {p.name} → {card.name or '(unnamed)'} role={role}")
        return log

    def save_card(self, card: IdentityCard, path: str | Path, role: str = "snapshot") -> Path:
        """保存一张卡 — 含 integrity_hash"""
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        # 注入 _role 供 load_dir 识别
        raw = card.to_dict()
        raw["_role"] = role
        raw["integrity_hash"] = card.integrity_hash()
        p.write_text(json.dumps(raw, ensure_ascii=False, indent=2), encoding="utf-8")
        return p
