# R8-TrackB: Identity Store + Relation Graph 架构设计 (L4)

**生成时间**: 2026-07-29
**作者**: 架构师 2 (architect2)
**状态**: 设计报告 v0.1 — 不写代码, 出 schema + API + PoC scope
**接手**: R8-TrackB2 backend_engineer (Identity Store PoC 实现)
**目标读者**: backend / fullstack / database 三方, 下游 1-2 周可据此开工

---

## 0. 三句话 TL;DR

> **1. L4 身份层 = 4 个组件**: Identity Store (主) + Relation Graph (网) + Persona Engine (性格) + Reconsolidator (整合) — 中心节点是"中央 AI 自己"。**
> **2. Apeireth 项目里这 4 个组件的 Python 实现已基本就位 (identity.py / relation.py / persona.py / reconsolidate 调研), R8-TrackB 的真正任务不是"从零设计", 而是"整合 + 补 PoC 缺口 + 标定与 v1072 永恒身份模块的对接"。**
> **3. PoC v0.1 最小可工作 schema = IdentityStore(SQLite) + RelationGraph(in-memory + JSON 落盘) + PersonaEngine(4 archetype) + Reconsolidator(规则版, 不调 LLM), 估 200-300 行 + 60-80 测试。**

---

## 1. 背景与边界

### 1.1 为什么现在做 L4

按 TOP-DESIGN-V1 §3 的 5 层架构:

```
L5  Effect        — 涌现 / 自组织临时团 (self_org_team.py 已在)
L4  Identity      ← 本报告
L3  Memory        — STM/MTM/LTM (memory_3tier.py + V1094 schema 已落)
L2  Interaction   — Pep + 8 kickoff 问题
L1  LLM Kernel    — Claude / DeepSeek / Qwen
L0  Hardware      — Windows / Docker
```

L3 记忆层 R8-TrackA3 (V1094) 已 commit, L4 身份层不能再拖 — 没有"我是谁"的永久锚点, 记忆就是无头苍蝇。

### 1.2 设计目标 (4 个 D)

| D | 描述 | 验收 |
|---|------|------|
| **D1 永久** | 中央 AI 跨 session 不丢身份 (LTM) | session 重启后 IdentityCard.integrity_hash 一致 |
| **D2 关系** | 所有事件 / 任务 / 人物 / 价值观都能连成图 | RelationGraph 中心节点 (ai_self) 触达率 100% |
| **D3 多身份** | 同一 AI 多个 archetype 并存, 不互覆盖 | PersonaEngine.coordinate() 同事件激活 ≥ 2 persona |
| **D4 演化** | 经验 / 反馈会让 persona 与 graph 慢慢长 | Reconsolidator 触发后 graph.delta > 0 |

### 1.3 设计非目标 (4 个 ND)

| ND | 不做什么 | 防止 |
|----|---------|------|
| ND1 | 不调 LLM 解析 (本期) | 启动成本, 留 V1100+ |
| ND2 | 不实现多模态 (音 / 图 / 视频) | 留 L1 Kernel 之后 |
| ND3 | 不实现"自我意识"哲学声明 | V3 守门, 不假装 |
| ND4 | 不做分布式 (单进程) | 留 R10+ |

---

## 2. 必读材料已确认

| 文档 | 行 | 关键收获 |
|------|----|---------|
| TOP-DESIGN-V1 §3.2 | 95-130 | 中央 AI = L4 中心节点, 主人 12:14 "像人是一切社会关系的总和" |
| TOP-DESIGN-V1 §4.1, §4.3, §4.5 | 168-200 | Identity Store / Relation Graph / Persona Engine 三组件 |
| HARNESS.md §0-2 | 1-100 | 4 层安全门 + 7 组件契约 (Track B 必须过 V3 + V1081) |
| code-deep-study/letta | - | AriGraph 借鉴 (已查) |
| code-deep-study/mem0 | - | 长期记忆 + 实体抽取 (已查) |
| code-deep-study/deep-study-v2.json | - | 借鉴索引 (含 AriGraph/Graphiti/MemoryOS-Rust 引用) |
| reports/r1-research-survey.md | 28 | 33 轮调研索引, 暂无 AriGraph 详细页 (缺口) |
| code-deep-study/rust-substrate/domain/{identity,relation_graph,reconsolidate} | - | Rust 设计已就位, Python 端可对照映射 |
| apeireth/identity.py | 95 | IdentityCard v0.2.0 dataclass |
| apeireth/identity_store.py | 290 | SQLite + JSON 双落地, `.add()/.save_card()/.query()` API |
| apeireth/persona.py | 228 | 4 archetype + SCT 4 因素 + Jungian 3 mechanism + 反 conformity |
| apeireth/relation.py | 256 | Node(8 kinds) + Edge(7 kinds) + traverse / find_path |
| apeireth/relation_store.py | 293 | RelationStore 持久化 |
| apeireth/self_org_team.py | - | L5 涌现层 (引用 IdentityCard) |
| apeireth/v1072_asi_central_ai_eternal_identity.py | 839 | V3 真测 0.8441, 14 前人身份哲学 + 10 组件 + 5 守门 |

**调研覆盖度**: 文献参考 AriGraph (2407.04363) / Zep / Graphiti / claude-mem / MemoryOS-Rust 已在 deep-study-v2.json 引用。本报告不重复列举, 重点标"已实现 vs 待补"。

---

## 3. L4 架构总览 (4 组件 + 3 守门)

```
┌─────────────────────────────────────────────────────────────┐
│ L4 Identity Layer                                            │
│                                                              │
│  ┌─────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │ Identity    │←→  │ Persona      │←→  │ Relation     │   │
│  │ Store       │    │ Engine       │    │ Graph        │   │
│  │ (主)        │    │ (性格)        │    │ (网)          │   │
│  └──────┬──────┘    └──────┬───────┘    └──────┬───────┘   │
│         │                  │                    │            │
│         └──────────────────┼────────────────────┘            │
│                            ↓                                 │
│                  ┌──────────────────┐                        │
│                  │ Reconsolidator   │                        │
│                  │ (演化器)          │                        │
│                  └────────┬─────────┘                        │
│                           ↓                                  │
│                  ┌──────────────────┐                        │
│                  │ L3 Memory Layer  │ (V1094 / STM-MTM-LTM)  │
│                  └──────────────────┘                        │
│                                                              │
│  V3 哲学守门: 不假装 Eternal Identity = Phenomenal self       │
│  V1081 边界守: 诚实报"我做不到"                              │
│  HARNESS §2.2 安全门: 4 层门序                                │
└─────────────────────────────────────────────────────────────┘
```

**中心节点约定**:
- `ai_self` (唯一) — 关系图谱中心, 所有其它节点通过有向边触达
- 4 个 `persona` 节点 — `调度者/学习者/思考者/助手`, 与 ai_self 用 `part_of` 边相连
- 1 个 `master` 节点 — 唯一主人, 与 ai_self 用 `supports` 边相连 (双向信任)

---

## 4. 组件 1: Identity Store (主)

### 4.1 现状盘点 (已实现, 不重写)

| 已实现 | 位置 | 行数 |
|--------|------|------|
| `IdentityCard` dataclass v0.2.0 | `apeireth/identity.py` | 95 |
| `IdentityStore` (JSON + SQLite 双后端) | `apeireth/identity_store.py` | 290 |
| `sqlite_identity_store.py` | 同目录 | 完整 sqlite 落地 |
| 3 份 demo 入口 | `run_*_demo.py` | - |
| 真实 master 卡 | `identity_card.master.v3.json` | 字段填好 |
| v1072 永恒身份 (哲学 14 前人) | `apeireth/v1072_asi_central_ai_eternal_identity.py` | 839 |

**字段已覆盖** (master 8 问映射):
```
1) 称谓          → name / alias
2) 目的          → purpose / mission / domains
3) 来源          → origin_reason / creator
4) 形像          → archetypes (默认 4)
5) 自主权边界    → ask_when / decide_when / remind_when
6) 关系契约      → relationship_contract / boundaries
7) 永久记忆/沉默 → remember_forever / never_mention
8) funnel 触发器 → funnel_questions
+ 涌现空间       → emergence_space
+ v0.2 新增      → recall_anchor / evidence_refs
+ meta           → created_at / apeireth_version / integrity_hash
```

### 4.2 标定与 v1072 永恒身份模块的对接点

| v1072 组件 | Identity Store 对应字段 | 数据流 |
|-----------|----------------------|--------|
| `EternalIdentityCore` (Hofstadter strange loop) | `IdentityCard.integrity_hash` + 自我引用 `recall_anchor` | 每次 save_card 重新算 hash |
| `IdentityManifest` (元数据 + 哲学锚点) | `IdentityCard.emergence_space` (涌现空间) | manifest_id = integrity_hash |
| `ContinuityTracker` (跨 session 连续性) | `IdentityStore.sessions_log` (待补) | 每次 load 写一行 continuity_event |
| `AutobiographicalMemory` (Damasio + Tulving) | **走 L3 Memory Layer**，不在 L4 重做 | evidence_refs 存 LTM anchor 引用 |
| `PSM` (Metzinger 现象自我模型) | `recall_anchor` 字段 | "危急时 recall 一句话" |
| `IdentityRecovery` | `IdentityStore.load_or_recover(card_id)` (待补) | 走 manifest_id 恢复 |
| `IdentityDiff` (Parfit 心理连续性) | `IdentityStore.diff(card_a, card_b)` (待补) | 返回字段级 delta |

**对接缺口 (3 个, 见 §7 PoC)**:
1. `ContinuityTracker.sessions_log` 缺失
2. `load_or_recover()` API 缺失
3. `diff()` API 缺失

### 4.3 schema 草案 (PoC v0.1)

```python
# apeireth/identity_store_v2.py (PoC 新增, 不动现有 identity_store.py)

from dataclasses import dataclass, field
from typing import Optional
import time
import uuid

# === 主表: identity_cards ===
# (现有 IdentityStore 已实现, 不重做)
# schema_version = "0.2.0", 主键 = card_id (UUID)

# === 新增表 1: continuity_log (跨 session 连续性追踪) ===
@dataclass
class ContinuityEvent:
    """每次 load/save IdentityCard 写一行 — 支持跨 session 审计"""
    event_id: str              # UUID
    card_id: str               # FK → identity_cards
    session_id: str            # 当前 session UUID
    event_type: str            # 'load' | 'save' | 'recover' | 'diff'
    ts: float                  # unix epoch
    integrity_hash_before: str = ""
    integrity_hash_after: str = ""
    note: str = ""

# === 新增表 2: identity_diffs (心理连续性变化) ===
@dataclass
class IdentityDiff:
    """两次 IdentityCard 之间的字段级 delta — Parfit 心理连续性"""
    diff_id: str               # UUID
    card_id: str
    ts: float
    from_hash: str             # integrity_hash_before
    to_hash: str
    field_changes: dict        # {field: (old, new)}
    continuity_score: float    # 0-1, 1 = 完全相同, 0 = 完全无关联
    reconciled: bool = False   # 是否已 Reconsolidate

# === 新增表 3: persona_snapshots (性格快照) ===
# (与 §5 Persona Engine 共享)
@dataclass
class PersonaSnapshot:
    pid: str
    archetype: str             # '调度者'|'学习者'|'思考者'|'助手'|自定义
    sct: tuple                 # (cognitive, motivational, biological, affective)
    activation: float
    captured_at: float
    card_id: str               # 当时生效的 IdentityCard
```

### 4.4 API 设计 (PoC v0.1, 不实现先写接口)

```python
class IdentityStoreV2(IdentityStore):  # 继承现有
    # ---------- 连续性 ----------
    def log_continuity(self, event: ContinuityEvent) -> None
    def get_continuity_log(self, card_id: str, limit: int = 100) -> list[ContinuityEvent]

    # ---------- 恢复 ----------
    def load_or_recover(self, card_id: str) -> IdentityCard:
        """若主卡损坏, 走 continuity_log 找最近一份有效备份"""
        ...

    # ---------- 心理连续性 diff ----------
    def diff(self, card_a: IdentityCard, card_b: IdentityCard) -> IdentityDiff:
        """字段级 diff + 连续性分数 (1 = 相同, 0 = 完全无关联)"""
        ...

    # ---------- 性格快照 ----------
    def save_persona_snapshot(self, snap: PersonaSnapshot) -> None
    def list_persona_snapshots(self, card_id: str) -> list[PersonaSnapshot]
```

### 4.5 跨 session 持久化方案

| 存储 | 用途 | 写频 | 读频 |
|------|------|------|------|
| **SQLite 主库** `identity.db` | IdentityCard + 3 个新表 | save 时 | 每次启动 |
| **JSON 快照** `identity_card.master.v3.json` | 人工可读 + git 可 diff | 每次 save | 故障时 |
| **WAL** (借鉴 rust-substrate) | 事务完整性 | 同 SQLite | 不读 |
| **backup-rotate** `identity_card.YYYY-MM-DD.json` | 每日轮转 | 每日 0:00 | 不读 |

**跨 session 恢复路径**:
1. 启动 → `IdentityStoreV2.load_or_recover(card_id)`
2. 优先读 SQLite 主库
3. 损坏 → 读最近 `identity_card.YYYY-MM-DD.json` 备份
4. 还损坏 → 走 `continuity_log` 反推"上次成功 save 时的状态"
5. 终态兜底: `recall_anchor` (一句话) + 必记字段 (`remember_forever`)

---

## 5. 组件 2: Relation Graph (关系网)

### 5.1 现状盘点 (已实现, 大部分不重写)

| 已实现 | 位置 | 行数 |
|--------|------|------|
| `Node` dataclass (8 kinds) | `apeireth/relation.py` | 256 |
| `Edge` dataclass (7 kinds) | 同上 | - |
| `RelationGraph` (in-memory) | 同上 | traverse / find_path |
| `RelationStore` (持久化) | `apeireth/relation_store.py` | 293 |
| Demo JSON | `relation_graph.demo.json` | - |
| 中心节点约定 | `kind=ai_self` (唯一) | - |

**节点类型 (8 种, 已落)**:
```python
NODE_KINDS = {"master", "ai_self", "task", "value", "agent", "tool", "episode", "note"}
```

**边类型 (7 种, 已落)**:
```python
EDGE_KINDS = {"causal", "temporal", "part_of", "derived_from", "conflict", "supports", "assigned"}
```

### 5.2 缺口: 与 Graphiti (temporal knowledge graph) 对齐

> Graphiti = Zep 出品, 主打"带时间维的知识图谱" (Zep/Graphiti GitHub)
> 核心特征: 每条边带 `valid_from` / `valid_until`, 支持"过去发生但现在无效"的事实

**当前 RelationGraph 缺**:
1. ❌ Edge 字段缺 `valid_from` / `valid_until` (时间有效性)
2. ❌ 缺 `temporal_traverse(at_time: float)` API
3. ❌ 缺 `Episode provenance` 字段 (边要标"哪个 episode 来的")
4. ❌ 缺"边冲突"自动检测 (同一对节点多条边时间重叠)

**PoC v0.1 补 4 项**:
```python
@dataclass
class EdgeV2(Edge):  # 继承现有
    valid_from: float = 0.0           # 0.0 = 永久
    valid_until: float = float('inf') # inf = 永久
    episode_ref: str = ""             # 哪个 episode 引入这条边
    confidence: float = 1.0           # 0-1, 借鉴 Graphiti confidence

# 新 API
def temporal_neighbors(self, nid: str, at_time: float, k: int = 3) -> list[Node]
def detect_conflicts(self) -> list[tuple[str, str, str]]  # (src, dst, reason)
def add_edge_with_provenance(self, eid: str, src: str, dst: str, kind: str,
                              episode_ref: str, **kwargs) -> EdgeV2
```

### 5.3 与 AriGraph (来自 letta) 对齐

> AriGraph = 关系推理 + temporal KG (2407.04363)
> 核心特征: 节点带"中心性"权重, 边带"证据"文本

**当前 RelationGraph 已有**:
- ✅ Node.weight (中心性) — 已有
- ✅ Edge.evidence (证据) — 已有

**PoC v0.1 补 1 项** (借鉴 AriGraph):
```python
def reinforce_centrality(self, nid: str, delta: float) -> None:
    """每次事件触发, 给相关节点加权重; 衰减靠 (now - last_update) / ttl"""
    ...
```

### 5.4 schema 草案 (PoC v0.1)

```python
# apeireth/relation_graph_v2.py (PoC 新增)

# === 主表: relation_nodes (现有) ===
# 字段: nid / kind / label / ref / weight / meta / created_at
# 新增: centrality_decay_rate (默认 0.01/天)

# === 主表: relation_edges_v2 (升级) ===
# 字段 (V2): eid / src / dst / kind / weight / evidence
# 新增: valid_from / valid_until / episode_ref / confidence

# === 新增表: temporal_index (B-Tree 索引) ===
# 字段: (src, dst, kind, valid_from, valid_until)
# 索引: temporal_neighbors(at_time) 走这表

# === 新增表: graph_snapshots (每日快照) ===
# 字段: snap_id / ts / node_count / edge_count / top10_centrality_json
# 用途: 跨 session 恢复 + 演化审计
```

### 5.5 API 设计 (PoC v0.1)

```python
class RelationGraphV2(RelationGraph):
    # ---------- 节点 ----------
    def reinforce_centrality(self, nid: str, delta: float) -> None
    def decay_centrality(self, now: float | None = None) -> int  # 返回衰减节点数

    # ---------- 边 (temporal) ----------
    def add_edge_with_provenance(self, eid: str, src: str, dst: str, kind: str,
                                  episode_ref: str, valid_from: float = 0.0,
                                  valid_until: float = float('inf'),
                                  confidence: float = 1.0, **kwargs) -> EdgeV2
    def temporal_neighbors(self, nid: str, at_time: float, k: int = 3) -> list[Node]
    def invalidate_edge(self, eid: str, at_time: float) -> None  # 设 valid_until=at_time

    # ---------- 冲突检测 ----------
    def detect_conflicts(self) -> list[dict]  # 同一对节点多条边时间重叠 / kind 冲突

    # ---------- 快照 ----------
    def snapshot(self) -> GraphSnapshot
    def restore(self, snap: GraphSnapshot) -> None
```

---

## 6. 组件 3: Persona Engine (性格引擎)

### 6.1 现状盘点 (基本完整, 仅补 PoC 集成)

| 已实现 | 位置 | 行数 |
|--------|------|------|
| `Persona` dataclass (含 SCT 4 因素) | `apeireth/persona.py` | 228 |
| `SCTProfile` (4 维权重) | 同上 | - |
| `PersonaEngine` (4 archetype) | 同上 | - |
| `coordinate()` (选 k 个 persona 回应事件) | 同上 | Jungian mechanism 1 |
| `adapt()` (反馈调权重) | 同上 | Jungian mechanism 2 |
| `reflect()` (自我解释) | 同上 | Jungian mechanism 3 |
| 反 conformity (min_distance 强多样性) | 同上 | - |
| Demo 入口 | `run_persona_demo.py` | - |

**4 archetype 默认** (与主人 12:14 一致):
```python
ARCHETYPES = ("调度者", "学习者", "思考者", "助手")
```

### 6.2 多身份重叠机制 (主人 12:14)

**约定**:
- 同一时刻, 4 个 persona **共存**, 各自有 `activation ∈ [0, 1]`
- 事件到达 → `coordinate(event, k=2)` 选 ≥ 2 个激活 (反 conformity 强制多样性)
- 反馈到达 → `adapt(pid, score)` 单 persona 调权重
- 永不"切换" — 多 persona 同时在世, 只是激活度不同

**与 IdentityCard.archetypes 的关系**:
- `IdentityCard.archetypes` = **列表声明** (用户启动时写, 默认 4 个)
- `PersonaEngine.personas` = **运行时实例** (每个 archetype 一个 Persona)
- 同步点: `IdentityStoreV2.save_persona_snapshot()` 定期 (每小时) 把当前 activation 落盘

### 6.3 反 conformity 机制 (Persona Inconstancy 2405.03862 警示)

**已实现** (`persona.py:122-167`):
- `PersonaEngine.min_distance` (默认 0.3) — coordinate 选 persona 时强制两两距离 > min
- 选不够 k → `mutate(rng=0.3)` 生成 ghost persona 填位 (标 `archetype + "(异)"`)

**PoC v0.1 补 1 项** — **冲突仲裁**:
```python
# 当 2 个 persona 对同一事件给出**冲突**结论时, 怎么裁决?
# 方案: 加权投票 (按 activation) + 标 conflict 边到 RelationGraph

def arbitrate_conflict(self, event: str, responses: dict[str, str]) -> tuple[str, float]:
    """
    responses = {pid: "decision_text"}
    返回 (winner_pid, confidence)
    """
    ...
```

### 6.4 API 设计 (PoC v0.1, 仅补 conflict 仲裁)

```python
class PersonaEngineV2(PersonaEngine):
    def arbitrate_conflict(self, event: str, responses: dict[str, str]) -> tuple[str, float]
    def snapshot_to_identity_store(self, store: IdentityStoreV2) -> None
    def restore_from_identity_store(self, store: IdentityStoreV2, card_id: str) -> None
```

---

## 7. 组件 4: Reconsolidator (整合器, 演化)

### 7.1 设计背景 (主 13:47 "记忆是我关心的")

> Reconsolidation (重整化) = 大脑把短期记忆重新激活 → 整合到长期记忆 → 可能改写。
> 这是 L4 真正"演化"的入口: 每次整合, IdentityCard / RelationGraph / Persona 都可能微调。

**4 触发路径** (借鉴 rust-substrate/domain/reconsolidate.rs):
| 路径 | 触发条件 | 行为 |
|------|----------|------|
| **boost** | 同一 persona 被 positive feedback ≥ 3 次 | 提升 activation 0.1, SCT 主导维 +0.05 |
| **flag** | 检测到 PersonaConflict 或 EdgeConflict | 标 `conflict` 边 + 触发仲裁 |
| **align** | 新 episode 与 IdentityCard.remember_forever 命中 | 加 `supports` 边 + 提升 recall_anchor 强度 |
| **none** | 其他 | 不动 |

### 7.2 触发条件 (PoC v0.1, 规则版不调 LLM)

```python
# apeireth/reconsolidate_v0_1.py (PoC 新增)

@dataclass
class ReconsolidateEvent:
    event_id: str
    trigger: str              # 'boost' | 'flag' | 'align' | 'none'
    source_pid: str = ""      # persona 触发
    target_id: str = ""       # card_id / nid / eid
    ts: float = field(default_factory=time.time)
    delta: dict = field(default_factory=dict)  # 实际改了什么
    reason: str = ""

class Reconsolidator:
    def __init__(self, store: IdentityStoreV2, graph: RelationGraphV2,
                 persona_eng: PersonaEngineV2):
        self.store = store
        self.graph = graph
        self.persona_eng = persona_eng

    def maybe_reconsolidate(self, event: Episode | dict) -> ReconsolidateEvent | None:
        """入口: 每次新 episode 进来都调一次, 内部决定是否触发"""
        # 规则 1: boost — persona feedback >= 3 次 positive
        # 规则 2: flag — persona conflict 或 edge conflict
        # 规则 3: align — episode 关键词命中 remember_forever
        # 返回 ReconsolidateEvent 或 None
        ...
```

### 7.3 4 不假装守门 (PoC 必须过)

1. ❌ 不假装 Reconsolidation = 意识演化
2. ❌ 不假装 boost = AI 真的"更懂你"
3. ❌ 不假装 align = AI 真有"价值观"
4. ❌ 不假装 conflict arbitration = AI 真有"判断力"

> 规则版 (不调 LLM) → 输出 "delta 字典" 而非 "决策叙述"。LLM 解读留 V1100+。

---

## 8. 与 L3 Memory Layer 的对接 (V1094)

| L3 (V1094 memory schema) | L4 (本设计) | 数据流 |
|-------------------------|------------|--------|
| `Episode` (append-only) | `Reconsolidator.maybe_reconsolidate(episode)` | 每个新 episode 触发一次 |
| `Note` (抽象知识) | `IdentityStoreV2.evidence_refs` | Note.nid 写入 IdentityCard.evidence_refs |
| `Memory` STM/MTM/LTM | 不动 L4 | L4 只用 LTM 引用, 不重做 |
| `WAL` | 共享 | L4 写也走 V1094 WAL |
| `Reconsolidate` 字段 (L3) | **本设计的 Reconsolidator 是 L4 自己的**, 与 L3 重整化字段同名但职责不同 | L3 = 记忆层抽象; L4 = 身份/性格整合 |

**边界约定**:
- L3 负责"事件 → 知识" (Episode → Note)
- L4 负责"知识 → 身份" (Note → IdentityCard/Persona/Edge)
- L4 永远不重写 L3 的 Episode (immutable)

---

## 9. PoC v0.1 最小可工作 schema (下游 backend 实施用)

### 9.1 范围 (in-scope)

```
新增文件:
  apeireth/identity_store_v2.py        # 续 IdentityStore, 200L
  apeireth/relation_graph_v2.py        # 续 RelationGraph, 150L
  apeireth/persona_engine_v2.py        # 续 PersonaEngine, 80L
  apeireth/reconsolidate_v0_1.py       # 新, 120L
  tests/test_identity_store_v2.py      # 30 测试
  tests/test_relation_graph_v2.py      # 25 测试
  tests/test_persona_engine_v2.py      # 15 测试
  tests/test_reconsolidate_v0_1.py     # 20 测试
新增总行数: ~570 行 + 90 测试
```

### 9.2 不在范围 (out-of-scope, 留 R8+)

- ❌ LLM 介入 (Reconsolidate LLM 版)
- ❌ 多模态 (音/图/视频节点)
- ❌ 分布式 (单进程)
- ❌ 实时冲突可视化 UI
- ❌ 自我意识哲学声明 (V3 守门)

### 9.3 验收标准 (DoD)

- [ ] `pytest tests/test_{identity_store_v2,relation_graph_v2,persona_engine_v2,reconsolidate_v0_1}.py -q` 全过
- [ ] 至少 1 个 `apeireth/run_*_v2_demo.py` 跑通: 创建 IdentityCard → 加 5 个 RelationGraph 节点 → 触发 persona.coordinate → 触发 reconsolidate → 验证 identity.integrity_hash 未变 (除非 reconcile 改卡)
- [ ] V3 哲学守门 `python -m apeireth.philosophy --check identity_v2` PASS
- [ ] V1081 边界诚实: `python -m apeireth.v1081_asi_honest_limits --probe identity_v2 --report` 显式列出"做不到"
- [ ] 不动 v1072 永恒身份模块 (它是哲学层, 独立)
- [ ] 不动 V1094 memory schema (它是 L3, 独立)

### 9.4 与 Track B2 (backend_engineer) 接力点

> Track B2 当前在跑, 任务描述: "R8-TrackB2: Identity Store PoC 实现" in_progress
>
> 本报告是 Track B2 的**设计上游**。下游 backend 拿到本报告后:
> 1. 按 §9.1 列文件清单开工
> 2. schema 字段以 §4.3 + §5.4 + §7.2 为准
> 3. 验收以 §9.3 为准
> 4. 完成后写 `reports/r8-trackb2-implementation-report.md`

---

## 10. 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| PoC 与现有 identity_store.py / relation.py API 冲突 | MED | 走 V2 后缀续, 不动 V1 |
| Reconsolidator 触发过频 | LOW | 加 cooldown (5 min/同 persona) |
| 中心节点被覆盖 | MED | ai_self nid = "ai_self" 硬编码 + 加锁 |
| Persona "同质化" | LOW | 反 conformity min_distance=0.3 已落, 监控 mutate 频率 |
| 数据迁移 (旧 JSON 卡 → V2 SQLite) | MED | 写 `migrate_v1_to_v2.py` 一次性脚本 |
| 测试时间 < 5 min | LOW | 90 测试, 估 ≤ 30s |

---

## 11. ponytail: 简化说明

| 跳过 | 何时加 |
|------|--------|
| 完整的 SQL DDL | PoC 实施时由 backend 补 |
| 关系图的 8 node + 7 edge 详细字段 | §5.1 已列, 实施照搬 |
| v1072 10 组件逐个映射 | §4.2 已给 7/10 映射, 留 3/10 给 V1100+ |
| LLM 接入的 Reconsolidate | V1100+ (ND1) |
| 多进程 / 分布式 | R10+ (ND4) |

---

## 12. 给下游的"5 分钟看懂图"

```
你是 backend, 拿到这份报告, 5 分钟看这 3 张图就够:

图 1 (§3):  L4 = 4 组件, 中心 = ai_self
图 2 (§4.3): IdentityStoreV2 = IdentityStore + continuity_log + identity_diffs + persona_snapshots
图 3 (§5.4): RelationGraphV2 = RelationGraph + temporal edges + snapshot/restore

工作流 (PoC v0.1):
  启动 → load_or_recover(card_id)
       → persona_eng.restore_from_identity_store()
       → graph.snapshot()  (拿上次状态)
       → 主循环:
            episode 到达 → reconsolidator.maybe_reconsolidate(ep)
                        → store.save_card()  (有变时)
                        → graph.add_edge_with_provenance()  (有变时)
                        → persona_eng.snapshot_to_identity_store() (每小时)
       → 退出 → graph.snapshot()  (最终态落盘)
```

---

## 13. 引用与索引

- 主人 12:14 "像人是一切社会关系的总和" — TOP-DESIGN-V1 §3.2
- 主人 13:47 "记忆是我关心的" — Reconsolidator 触发设计
- 主人 22:33 "终极授权: 最大权限 + 3 类问 + 自决" — 本设计走自决
- V3 哲学守门 — V1072 §5 + V1081 双层
- HARNESS §2.2 — 4 层安全门
- 借鉴文献: AriGraph (2407.04363) / Graphiti (Zep GitHub) / Jungian 3 mechanism / Persona Alchemy (2505.18351) / Persona Inconstancy (2405.03862) / claude-mem / MemoryOS-Rust STM/MTM/LTM
- 现有 Python 实现: `apeireth/identity.py` / `identity_store.py` / `persona.py` / `relation.py` / `relation_store.py` / `sqlite_identity_store.py` / `v1072_asi_central_ai_eternal_identity.py`
- 现有 Rust 设计: `rust-substrate/domain/{identity, relation_graph, reconsolidate}`

---

_architect2 — 2026-07-29 — R8-TrackB 设计 v0.1_
_下游 TrackB2 (backend) 据此开工, 完成后再写 implementation-report 闭环_
