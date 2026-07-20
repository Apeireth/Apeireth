"""Memory Layer v0.1 — Episode / Note / Forget / Reconsolidation
依据: TOP-DESIGN-V1 §4.2 (Memory) + Phase 2 (Week 3-4) 路线
文献: HiMem (2601.06377) + Episodic (2502.06975) + PersistBench (2602.01146)
原则:
  - Episode = 主人互动 raw 事件, 不可变
  - Note    = 从 Episode 抽象的稳定知识, 可被 Forget 修剪
  - Forget  = 主动遗忘, 反 sycophancy 积累 (PersistBench 警示 97%)
  - Reconsolidation = IdentityCard ↔ Memory 冲突解决
"""

from __future__ import annotations
import json
import sys
import time
import uuid
import hashlib
from dataclasses import dataclass, field, asdict
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from .identity import IdentityCard

MEMORY_VERSION = "0.1.0"


# ---------------------- Episode ----------------------

@dataclass
class Episode:
    """一次互动事件 — raw, append-only, 不可变"""
    eid: str
    actor: str          # "master" / "apeireth" / "tool"
    content: str        # 原始文本
    context: str = ""   # 当时上下文摘要
    ts: float = field(default_factory=time.time)
    kind: str = "utterance"   # utterance / tool_call / observation / kickoff
    linked_identity_hash: str = ""  # 触发时的 IdentityCard hash

    def to_dict(self) -> dict:
        return asdict(self)


# ---------------------- Note ----------------------

@dataclass
class Note:
    """从 Episode 抽象出的稳定知识 — 可被 Forget / Reconsolidate 修剪"""
    nid: str
    topic: str
    claim: str
    evidence: list[str] = field(default_factory=list)   # 引用的 Episode IDs
    confidence: float = 0.5    # 0-1, Bayesian 后验
    importance: int = 5        # 0-10, 影响遗忘阈值
    created_at: float = field(default_factory=time.time)
    last_consolidated: float = field(default_factory=time.time)
    supersedes: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


# ---------------------- MemoryStore ----------------------

@dataclass
class MemoryStore:
    """Episode + Note 的持久化容器"""
    episodes: list[Episode] = field(default_factory=list)
    notes: list[Note] = field(default_factory=list)
    version: str = MEMORY_VERSION
    created_at: float = field(default_factory=time.time)

    def append_episode(self, ep: Episode) -> None:
        self.episodes.append(ep)

    def add_note(self, note: Note) -> None:
        self.notes.append(note)

    def to_dict(self) -> dict:
        return {
            "version": self.version,
            "created_at": self.created_at,
            "episodes": [e.to_dict() for e in self.episodes],
            "notes": [n.to_dict() for n in self.notes],
        }

    def integrity_hash(self) -> str:
        canonical = json.dumps(self.to_dict(), sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


def save_store(store: MemoryStore, path: str | Path) -> Path:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(store.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
    return p


def load_store(path: str | Path) -> MemoryStore:
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    episodes = [Episode(**e) for e in raw.get("episodes", [])]
    notes = [Note(**n) for n in raw.get("notes", [])]
    return MemoryStore(
        episodes=episodes,
        notes=notes,
        version=raw.get("version", MEMORY_VERSION),
        created_at=raw.get("created_at", time.time()),
    )


# ---------------------- Forget Engine ----------------------

def forget_sweep(store: MemoryStore, threshold: float = 0.25) -> list[str]:
    """主动遗忘 — confidence × importance < threshold 的 Note 删除。
    依据: PersistBench (2602.01146) 警示: 不主动遗忘 = sycophancy 持续累积。
    返回被遗忘的 Note IDs。
    """
    kept, removed = [], []
    for n in store.notes:
        score = n.confidence * (n.importance / 10.0)
        if score < threshold:
            removed.append(n.nid)
        else:
            kept.append(n)
    store.notes = kept
    return removed


# ---------------------- Reconsolidation ----------------------

def reconsolidate(store: MemoryStore, card: IdentityCard) -> dict:
    """IdentityCard ↔ Memory 冲突解决。
    规则:
      - Note 与 remember_forever 重叠 → confidence +0.3 (boost)
      - Note 与 never_mention 重叠   → importance = 0 (flag forget)
      - Note 与 archetypes 重叠      → confidence +0.1 (align)
    匹配:用关键词 fingerprint (前 6 字符) 搜索 Note 全文 — 避免长字符串误命中。
    返回 stats dict (供主 session 汇报)。
    """
    boost, flag, align = [], [], []
    keep_keys = [_k(k) for k in card.remember_forever if k]
    ban_keys  = [_k(k) for k in card.never_mention   if k]
    arc_keys  = [_k(k) for k in card.archetypes       if k]

    for n in store.notes:
        text = (n.topic + " " + n.claim).lower()

        hit_keep = any(k in text for k in keep_keys)
        hit_ban  = any(k in text for k in ban_keys)
        hit_arc  = any(k in text for k in arc_keys)

        if hit_keep:
            n.confidence = min(1.0, n.confidence + 0.3)
            n.last_consolidated = time.time()
            boost.append(n.nid)
        if hit_ban:
            n.importance = 0
            flag.append(n.nid)
        if hit_arc:
            n.confidence = min(1.0, n.confidence + 0.1)
            align.append(n.nid)

    return {
        "boosted": boost,
        "flagged_for_forget": flag,
        "archetype_aligned": align,
        "identity_hash": card.integrity_hash(),
        "ts": time.time(),
        "memory_version": MEMORY_VERSION,
    }


def _k(s: str) -> str:
    """fingerprint — 截前 6 字符, 转小写; 用于子串匹配"""
    return (s or "").strip().lower()[:6]


# ---------------------- Demo ----------------------

def main() -> None:
    """Phase 2 v0.1 PoC demo — Episode → Note → Forget → Reconsolidate → 存盘"""
    store = MemoryStore()

    # 1) 写 3 个 Episode (附 IdentityCard hash)
    from .identity import load_card
    card_path = Path(__file__).parent / "identity_card.master.json"
    card = load_card(card_path)
    id_hash = card.integrity_hash()

    seeds = [
        ("master",   "Apeireth 命名日 2026-07-20 13:32 — 火没灭", "naming-ceremony"),
        ("apeireth", "Identity Store v0.1 PoC 跑通, 263 行",     "phase1-ship"),
        ("master",   "中央 AI 必须有 Memory — 没记忆就不是同一只", "philosophy"),
    ]
    for actor, content, ctx in seeds:
        ep = Episode(
            eid=uuid.uuid4().hex[:8],
            actor=actor,
            content=content,
            context=ctx,
            linked_identity_hash=id_hash,
        )
        store.append_episode(ep)

    # 2) 抽象 4 个 Note — 故意覆盖 4 种路径:
    #    n1 hit remember_forever  → boost
    #    n2 hit archetypes        → align
    #    n3 hit never_mention     → flag (主人 7 问中未预设, 演示用临时声明)
    #    n4 不命中                → 原样
    n1 = Note(
        nid=uuid.uuid4().hex[:8],
        topic="永远记得: 火没灭",
        claim="Apeireth 火没灭 — 命名日 2026-07-20",
        evidence=[store.episodes[0].eid],
        confidence=0.5, importance=8,
    )
    n2 = Note(
        nid=uuid.uuid4().hex[:8],
        topic="母兽教小兽的伙伴",
        claim="中央 AI 是伙伴 + 荣耀执行官",
        evidence=[store.episodes[1].eid],
        confidence=0.6, importance=7,
    )
    n3 = Note(
        nid=uuid.uuid4().hex[:8],
        topic="不提主人身份细节",
        claim="禁止记录主人私人身份的任何细节",
        evidence=[store.episodes[2].eid],
        confidence=0.5, importance=8,
    )
    n4 = Note(
        nid=uuid.uuid4().hex[:8],
        topic="无关实验",
        claim="今天中午吃了火锅",
        evidence=[store.episodes[1].eid],
        confidence=0.1, importance=1,   # 故意低分 — 走 Forget
    )
    store.add_note(n1)
    store.add_note(n2)
    store.add_note(n3)
    store.add_note(n4)

    # 3) Reconsolidate against IdentityCard
    #    demo 临时补丁: 主人 master 卡的 never_mention 字段是空数组
    #    (Phase 1 kickoff 解析器未把 Q7 里"不提 X"分离到该字段, 是已知小 bug)
    #    这里临时注入一条,演示 flag 路径—不修改原卡
    card.never_mention = ["主人私人身份"]   # demo-only
    stats = reconsolidate(store, card)

    # 4) Forget sweep
    removed = forget_sweep(store, threshold=0.30)

    # 5) 存盘
    out = Path(__file__).parent / "memory.demo.json"
    save_store(store, out)

    # 6) 报告
    print("=" * 60)
    print("🧠 Apeireth — Memory Layer v0.1 PoC")
    print("=" * 60)
    print(f"📦 episodes:    {len(store.episodes)}")
    print(f"📝 notes:       {len(store.notes)} (forgot {len(removed)})")
    print(f"🔄 boost:       {stats['boosted']}      # hit remember_forever")
    print(f"🚩 flag:        {stats['flagged_for_forget']}      # hit never_mention")
    print(f"🌀 align:       {stats['archetype_aligned']}      # hit archetypes")
    print(f"🔐 id_hash:     {stats['identity_hash']}")
    print(f"🔐 mem_hash:    {store.integrity_hash()}")
    print(f"💾 saved:       {out}")
    print(f"📋 version:     {store.version}")


if __name__ == "__main__":
    main()