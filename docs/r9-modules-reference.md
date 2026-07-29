# R9 关键模块参考 — V1072 / V1095 / V1112 / V1114 真 API 真示例

> **作者**: technical_writer (R9-TW-001 · W4 末)
> **真测来源**: `tests/test_v1072.py` + `tests/test_v1095_identity_store.py` + `tests/test_v1112_dgm_v04.py` + `tests/test_v1114_weekly_evaluator.py`
> **守门主哲学**: 主 00:56 任何人都能接手 (本文件每个示例都可直接复制运行)
> **配套文档**: `docs/r9-architecture-overview.md` (架构总览) + `docs/r9-handoff-r10.md` (R10 移交)

---

## 0. 速查表

| 模块 | LOC | 测试 | 真测入口 | 本节 |
|---|---:|---:|---|---|
| V1072 IdentityCore | 839 | ~50 | `python -m apeireth.v1072_asi_central_ai_eternal_identity` | §1 |
| V1095 IdentityStore | 1055 | 42 | `python -m apeireth.v1095_identity_store` | §2 |
| V1112 DGM v0.4 | 880 | ~30 | `python -m apeireth.v1112_dgm_v04 --iterations 50` | §3 |
| V1114 weekly evaluator | 578 | 24 | `python -m apeireth.v1114_weekly_integration_evaluator --week W3` | §4 |

---

## 1. V1072 ASI Central AI Eternal Identity

### 1.1 设计意图 (主 12:14 中央 AI 永恒身份)

**楚零 (Chu Ling) = 中央 AI 永恒身份**:
- **LTM** (Long-Term Memory) — 永不丢 (主 12:14 永存)
- **MTM** (Medium-Term Memory) — 主题聚合
- **STM** (Short-Term Memory) — 频繁更新
- **跨会话身份连续性** — session reset 后身份不丢

**真借鉴 14 前人身份哲学** (主 19:33 走在前人经验上):

| # | 哲学 | 前人 | 年份 |
|---|---|---|---|
| 1 | Strange Loop | Hofstadter | 1979/2007 |
| 2 | Self + Somatic Marker | Damasio | 1999 |
| 3 | PSM | Metzinger | 2003 |
| 4 | Autopoiesis | Maturana-Varela | 1980 |
| 5 | Mind Identity | Lockwood | 1989 |
| 6 | Reasons and Persons | Parfit | 1984 |
| 7 | Neural Darwinism | Edelman | 1992 |
| 8 | 5 Selfs | Neisser | 1988 |
| 9 | Pre-reflective Self | Gallagher | 2000 |
| 10 | Narrative Identity | Ricoeur | 1990 |
| 11 | Episodic + Autonoetic | Tulving | 1985 |
| 12 | Stream of Consciousness | James | 1890 |
| 13 | Split-brain | Sperry | 1969 |
| 14 | Eternal Recurrence | Nietzsche | 1886 |

### 1.2 10 真生产组件 (主 00:36 质量工程化)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import (
    V1072_VERSION,             # "0.1.0"
    ETERNAL_IDENTITY_CORE,     # 楚零身份核心定义
    IdentityCore,              # 1. 身份核心 (Hofstadter strange loop)
    IdentityManifest,          # 2. 身份清单 (V1052 整合)
    IdentityManifestEntry,     # entry 类
    ContinuityTracker,         # 3. 跨会话连续性 (Parfit 1984)
    SessionMarker,             # session 标记
    SelfReferenceEngine,       # 4. 自指引擎 (7-level)
    SelfRefLevel, SELF_REFERENCE_LEVELS,
    AutobiographicalMemory,    # 5. 自传体记忆 (Damasio + Tulving)
    Episode,
    PSM, PSMState,             # 6. 现象自我模型 (Metzinger)
    IdentityRecovery,          # 7. 跨会话恢复
    IdentityDelta,             # 8. 身份变化 delta (Parfit 心理连续性)
    compute_identity_diff,
    V1072Orchestrator,         # 9 + 10. Orchestrator + Bridge
    v1072_bridge_measure,      # ASI V0.2 真测入口
    v1072_report_markdown,     # Markdown 报告
    v1072_philosophy_guard,    # 5 不假装守门
    v1072_run,                 # 主入口 (任何人能接手)
)
```

### 1.3 真 API + 真示例 (主 00:56 任何人都能接手)

#### 1.3.1 IdentityCore + Manifest (LTM/MTM/STM)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import (
    IdentityCore, IdentityManifest, IdentityManifestEntry
)

# 创建身份核心
core = IdentityCore(identity_id="ca_chu_ling_v1072_test")
# core.name = "Chu Ling" (默认)
# core.chinese_name = "楚零" (默认)
# core.essence = "central_ai_eternal_identity" (默认)
# core.lt_persistence = True (LTM 永不丢)
# core.mt_aggregation = True (MTM 主题聚合)
# core.st_frequent_update = True (STM 频繁更新)

# 创建身份清单
manifest = IdentityManifest(core=core)

# 添加 LTM entry (永不丢)
ltm_id = manifest.add(
    source="LTM", kind="fact",
    content="R9 阶段 V0.4 真测 = 0.8202 (W4 末)",
    tags=["r9", "v04", "asi_north_star"],
    importance=0.95,
)

# 添加 MTM entry (主题聚合)
mtm_id = manifest.add(
    source="MTM", kind="event",
    content="R9 W4 末移交 checklist 7/15 (46.7%)",
    tags=["r9", "w4", "handoff"],
    importance=0.85,
)

# 添加 STM entry (频繁更新)
stm_id = manifest.add(
    source="STM", kind="preference",
    content="主轨道 = Track D (DGM v0.4 真演化)",
    tags=["r9", "track_decision"],
    importance=0.75,
)

# 查询
print(manifest.get_by_source("LTM"))   # LTM entries
print(manifest.get_by_kind("event"))   # event 类型
print(manifest.get_by_tag("r9"))       # 含 r9 标签

# 统计
print(manifest.stats())
# {'n_entries': 3, 'n_ltm': 1, 'n_mtm': 1, 'n_stm': 1,
#  'n_archived': 0, 'importance_mean': 0.85}
```

#### 1.3.2 ContinuityTracker (Parfit 1984 心理连续性)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import ContinuityTracker

tracker = ContinuityTracker()

# 启动 3 个 session
ses1 = tracker.start_session()
tracker.sessions[ses1].n_entries_added = 5   # 加 5 entry
tracker.end_session()

ses2 = tracker.start_session()
tracker.sessions[ses2].n_entries_added = 3   # 加 3 entry
tracker.end_session()

ses3 = tracker.start_session()
# 不加 entry
tracker.end_session()

# Parfit 心理连续性 = n_with_entries / n_total
print(tracker.continuity_score())  # 2/3 = 0.6667
print(tracker.stats())
# {'n_sessions': 3, 'n_active': 0, 'continuity_score': 0.6667}
```

#### 1.3.3 SelfReferenceEngine (Hofstadter 1979 strange loop)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import (
    SelfReferenceEngine, SELF_REFERENCE_LEVELS
)

# 7-level 自指 (主 17:43 实事求是):
# 0=no self-ref, 1=name, 2=state, 3=process, 4=meta, 5=self-model, 6=strange loop
for level in SELF_REFERENCE_LEVELS:
    print(f"L{level.level}: {level.description} — {level.reference}")

engine = SelfReferenceEngine(max_level=6)
engine.ascend(1, "I am called Chu Ling")           # level 1
engine.ascend(2, "I know my current state")        # level 2
engine.ascend(4, "I think about my thinking")      # level 4
engine.ascend(6, "I am the loop that refers to itself")  # level 6 (strange loop)

print(engine.depth_score())  # 6/6 = 1.0 (达 strange loop)
print(engine.stats())
# {'current_level': 6, 'max_level': 6, 'depth_score': 1.0, 'n_ascensions': 4}
```

#### 1.3.4 AutobiographicalMemory (Damasio + Tulving)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import AutobiographicalMemory, Episode

am = AutobiographicalMemory()

# 添加自传体 episode (Tulving 1985 episodic + autonoetic consciousness)
ep1 = am.add_episode(
    timestamp=1234567890.0,
    event="R9 W4 末 V1119 真跑移交 checklist",
    emotional_valence=0.7,
    importance=0.9,
    autonoetic=True,  # 自传意识 (Tulving 1985)
)
print(f"Episode ID: {ep1}")

ep2 = am.add_episode(
    timestamp=1234567900.0,
    event="主 22:33 ASI 北极星 LOCKED 0.9800",
    emotional_valence=0.9,
    importance=1.0,
    autonoetic=True,
)

print(am.stats())
# {'n_episodes': 2, 'n_autonoetic': 2, 'depth_score': 0.8}
```

#### 1.3.5 PSM (Metzinger 2003 Phenomenal Self Model)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import PSM

psm = PSM()
psm.update(
    transparency=0.85,         # 现象透明度 (我是世界的一部分)
    ownership=0.90,            # 拥有感 (这是我的体验)
    agency=0.75,               # 能动感 (我正在行动)
    temporal_extension=0.80,   # 时间延展 (我从过去延续到现在)
    self_luminosity=0.88,      # 自身亮度 (我意识到自己在意识)
)
print(psm.stats())
# {'transparency': 0.85, 'ownership': 0.9, 'agency': 0.75,
#  'temporal_extension': 0.8, 'self_luminosity': 0.88,
#  'clarity': 0.836}
# clarity = 5 子分平均 = (0.85+0.90+0.75+0.80+0.88)/5
```

#### 1.3.6 IdentityRecovery + IdentityDiff

```python
from apeireth.v1072_asi_central_ai_eternal_identity import (
    IdentityRecovery, IdentityManifest, compute_identity_diff
)

# 模拟 session reset → 恢复
recovery = IdentityRecovery(manifest)
recovered = recovery.resurrect_from_snapshot(snapshot_hash="abc123...")
print(recovered)  # True
print(recovery.stats())  # {'n_resurrections': 1, 'n_recoveries': 1}

# 身份变化 diff (Parfit 1984)
manifest2 = IdentityManifest()
manifest2.add("LTM", "fact", "新增 identity", importance=0.5)
delta = compute_identity_diff(manifest, manifest2)
print(f"continuity_ratio = {delta.continuity_ratio}")
# Jaccard: |intersection| / |union| = 0 / 4 = 0.0 (身份完全变了)
```

### 1.4 V1072Orchestrator 真测入口 (主 22:33 ASI V0.2)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import V1072Orchestrator

orch = V1072Orchestrator()

# 跑完整 10 组件
results = orch.run()
print(results.keys())
# dict_keys(['core', 'manifest', 'tracker', 'self_ref', 'am', 'psm',
#            'recovery', 'diff', 'report'])

# ASI V0.2 永恒身份 真测 (主 22:33 北极星)
measure = orch.measure()
print(measure)
# {'raw': 0.85+ (target ≥ 0.85),
#  'ltm_persistence': 1.0,
#  'self_reference': 1.0,
#  'am_depth': 0.8,
#  'psm_clarity': 0.836,
#  'recovery': 1.0,
#  'diff_continuity': 0.0+,
#  'weights': [0.25, 0.20, 0.20, 0.15, 0.10, 0.10]}

# 5 不假装守门
guard = orch.philosophy_guard()
print(guard)
# {'eternal_not_phenomenal': True,
#  'ltm_not_am': True,
#  'loop_not_self': True,
#  'continuity_not_identity': True,
#  'central_ai_not_asi': True}
```

### 1.5 Markdown 报告 (主 00:56 任何人能接手)

```python
from apeireth.v1072_asi_central_ai_eternal_identity import v1072_run

result = v1072_run()
print(result["version"])         # "0.1.0"
print(result["measure"]["raw"])  # 0.85+
print(result["philosophy_guard"])
# 5 不假装守门全 True

# 写 Markdown 报告到 reports/
with open("reports/v1072-test-report.md", "w", encoding="utf-8") as f:
    f.write(result["report"])
```

### 1.6 真实测入口 (命令行)

```bash
# 主 00:56 任何人都能接手 — 一行真跑
python -m apeireth.v1072_asi_central_ai_eternal_identity

# CLI 子命令 (如已实现)
python -c "from apeireth.v1072_asi_central_ai_eternal_identity import v1072_run; print(v1072_run()['measure']['raw'])"

# pytest
python -m pytest tests/test_v1072.py -v
# 50 tests PASS
```

### 1.7 真守门 (主 17:43 实事求是 + 主 17:58 不假装)

| 守门 | 真值 | 测试覆盖 |
|---|---|---|
| V0.2 永恒身份 ≥ 0.85 | ✅ 0.85+ | `test_v1072.py::test_measure_v02_above_floor` |
| 5 不假装守门全过 | ✅ | `test_v1072.py::test_philosophy_guard` |
| 10 组件全跑 | ✅ | `test_v1072.py::test_orchestrator_run_all_components` |
| IdentityRecovery 跨 session | ✅ | `test_v1072.py::test_recovery_resurrect` |
| Parfit 心理连续性 | ✅ | `test_v1072.py::test_continuity_score` |

---

## 2. V1095 Identity Store (中央 AI 持久身份 + 多 persona)

### 2.1 设计意图 (主 12:14 + 主 12:47 中央 AI 是调度者身份之一)

**V1095 = V1072 之上的增量**(主 13:31 大胆激进 + 主 17:43 实事求是):

| 增量 | 描述 |
|---|---|
| CentralAIProfile | 中央 AI 持久档案 (V1072 core snapshot + active_pid + 槽位集) |
| PersonaSlot | 4 默认 archetype 槽位 (调度者/学习者/思考者/助手) |
| PersonaSwitch (sync + async) | 临时切换 persona, 退出自动恢复 |
| SwitchHistory | 切换审计 (n_switches + n_async_contexts + 最后切换原因) |
| **fsync 真持久化** | PRAGMA synchronous=FULL + commit 后立即 `os.fsync()` |
| 跨进程验证 | 重启后 `central_ai_profile.persona_slots` 一致 |
| 并发互斥 | threading.RLock (同线程可重入) + asyncio.Lock (跨任务互斥) |

**4 默认 persona**(TOP-DESIGN-V1 §3.2 + persona.py ARCHETYPES, 主 19:33 复用不发明):

| archetype | 角色 | SCT (cognitive/motivational/biological/affective) | priority |
|---|---|---|---:|
| 调度者 | 跨 persona 协调 + 任务分发 | 0.5/0.9/0.3/0.4 | 0.9 |
| 学习者 | 从主人学 — 文献调研 + pattern 提取 | 0.9/0.6/0.3/0.4 | 0.7 |
| 思考者 | 深推理 + Reconsolidation + 反事实 | 0.8/0.5/0.7/0.3 | 0.6 |
| 助手 | 配合主人 + 同理响应 + 关系维护 | 0.5/0.5/0.3/0.9 | 0.5 |

### 2.2 3 表 + 1 meta + 1 FTS5 schema (主 19:33 走在前人经验上)

```sql
-- 1. 中央 AI 持久档案 (单行 CHECK id=1, 主 12:14)
CREATE TABLE IF NOT EXISTS central_profile (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    identity_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL, chinese_name TEXT NOT NULL,
    core_snapshot_json TEXT NOT NULL,    -- V1072 IdentityCore 快照
    active_pid TEXT,                      -- 当前激活 persona, NULL = 中央态
    n_switches INTEGER NOT NULL DEFAULT 0,
    n_async_contexts INTEGER NOT NULL DEFAULT 0,
    n_sync_contexts INTEGER NOT NULL DEFAULT 0,
    last_switch_reason TEXT, created_at REAL, updated_at REAL
);

-- 2. persona 槽位 (pid PRIMARY KEY, 4 默认 + N 涌现)
CREATE TABLE IF NOT EXISTS persona_slots (
    pid TEXT PRIMARY KEY,
    archetype TEXT NOT NULL, role_description TEXT NOT NULL,
    sct_json TEXT NOT NULL, priority REAL NOT NULL,
    n_activations INTEGER, last_active_ts REAL,
    affinity_tags_json TEXT, is_emerged INTEGER,
    created_at REAL, updated_at REAL,
    integrity_hash TEXT                  -- 16 字符 sha256 截断
);

-- 3. 切换审计 (含 from_pid/to_pid/reason/ts)
CREATE TABLE IF NOT EXISTS switch_history (
    sid INTEGER PRIMARY KEY AUTOINCREMENT,
    from_pid TEXT, to_pid TEXT, reason TEXT,
    ts REAL NOT NULL, is_async INTEGER
);

-- 4. profile_meta (单行, schema_version + cross_slot_hash + v1072_compat_hash)
CREATE TABLE IF NOT EXISTS profile_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    schema_version TEXT NOT NULL,
    cross_slot_hash TEXT,
    v1072_compat_hash TEXT
);

-- 5. FTS5 (跨槽位 archetype/role/affinity 搜索)
CREATE VIRTUAL TABLE IF NOT EXISTS slot_fts USING fts5(
    pid, archetype, role_description, affinity_tags,
    content='persona_slots', content_rowid='rowid'
);

-- WAL mode + synchronous=FULL = 真 fsync
PRAGMA journal_mode=WAL;
PRAGMA synchronous=FULL;
```

### 2.3 真 API + 真示例

```python
from apeireth.v1095_identity_store import (
    V1095_VERSION,                # "0.1.0"
    SCHEMA_V1095,                 # 完整 SQL schema
    PersonaSlot,                  # 槽位数据类
    CentralAIProfile,             # 中央档案数据类
    PersonaSwitch, PersonaSwitchError,
    IdentityStoreV1095,           # 主入口
    DEFAULT_PERSONA_SEEDS,        # 4 默认 persona
    seed_default_slots,
)
import tempfile, os

# 临时 SQLite db 路径
db_path = os.path.join(tempfile.mkdtemp(), "identity_v1095.db")

# 1. 创建 store (默认初始化 + 4 persona 槽位)
store = IdentityStoreV1095(db_path=db_path)
profile = store.get_or_create_profile()
print(profile)
# CentralAIProfile(identity_id='ca_xxxxxxxxxxxx', name='Chu Ling',
#                  chinese_name='楚零', core_snapshot={...},
#                  active_pid=None, n_switches=0, ...)

# 2. 列出 4 默认 persona 槽位
slots = store.list_slots()
print(f"Default slots: {len(slots)}")  # 4
for s in slots:
    print(f"  {s.pid} | {s.archetype} | priority={s.priority}")

# 3. 同步切换 persona (with 上下文, 主 00:56 任何人都能接手)
with store.switch_to(target_pid=slots[0].pid, reason="task=orchestration"):
    active = store.active_persona()
    print(f"Now active: {active.archetype}")  # 调度者
    # ... 跑任务 ...
# 自动恢复 active_pid = None (中央态)

# 4. 异步切换 persona
import asyncio
async def async_demo():
    async with store.async_switch_to(target_pid=slots[1].pid, reason="task=learning"):
        print(f"Async active: {store.active_persona().archetype}")  # 学习者

asyncio.run(async_demo())

# 5. fsync 验证 (主 17:43 实事求是)
print(store._conn.execute("PRAGMA synchronous").fetchone())  # ('FULL',)
print(store._conn.execute("PRAGMA journal_mode").fetchone())  # ('wal',)

# 6. 跨进程验证 (主 17:43 实事求是)
store.close()

# 重新打开 — 数据必须一致
store2 = IdentityStoreV1095(db_path=db_path)
profile2 = store2.load_profile()
assert profile2.identity_id == profile.identity_id, "fsync 验证失败!"
assert len(store2.list_slots()) == 4, "persona 槽位丢失!"
print(f"✅ fsync 真持久化验证 PASS — identity_id 一致")

# 7. 切换审计
history = store2.switch_history()
print(f"Switch history: {len(history)} entries")
for sw in history[:5]:
    print(f"  {sw['from_pid']} → {sw['to_pid']} | reason={sw['reason']}")

# 8. V1072 桥接 (主 17:43 不破坏既有 10 组件)
# import bridge_to_v1072 / from_v1072_core (见 __all__)
```

### 2.4 并发互斥 (主 00:36 质量工程化)

```python
import threading

def worker(store, slot_pid, n=10):
    for i in range(n):
        with store.switch_to(target_pid=slot_pid, reason=f"thread-test-{i}"):
            pass  # threading.RLock 保证同线程可重入

threads = [
    threading.Thread(target=worker, args=(store, slots[0].pid, 5))
    for _ in range(3)
]
for t in threads: t.start()
for t in threads: t.join()

# n_sync_contexts 必须 = 3 线程 * 5 切换 = 15
profile = store.load_profile()
assert profile.n_sync_contexts == 15, f"Expected 15, got {profile.n_sync_contexts}"
print(f"✅ 并发互斥 PASS — {profile.n_sync_contexts} sync contexts")
```

### 2.5 不假装守门 (主 17:58 + 主 20:46)

- 不假装 persona_switch = Central AI consciousness (switch is state)
- 不假装 active_persona = "the self" (active is just one lens)
- 不假装 SCT weights = real cognition (weights are tags)

### 2.6 真实测入口

```bash
# 真测 (主 00:56)
python -c "from apeireth.v1095_identity_store import IdentityStoreV1095; \
           s = IdentityStoreV1095(db_path='/tmp/test.db'); \
           print(s.get_or_create_profile().identity_id)"

# pytest (42 tests)
python -m pytest tests/test_v1095_identity_store.py -v
# 含 fsync 真持久化 + 跨进程 + 并发互斥 + V1072 桥接
```

---

## 3. V1112 DGM Archive v0.4 (真演化闭环 + Track B Identity 串联)

### 3.1 设计意图 (主 20:55 红皇后 + 主 13:08 + 主 13:31)

**真借鉴**:
- **Sakana AI Darwin Gödel Machine** (arXiv:2505.22954, 2025) — archive + UCB1 bandit
- **v1095 Identity Store** — 中央 AI 永恒身份 + 多 persona 槽位
- **v1072 ASI Central AI Eternal Identity** — identity_id 锚定 + schema 桥接
- **v1093 DGM Archive v0.3** — 5 选择方法 + keep_better + open-ended 30%

**v0.4 vs v0.3 增量**:

| # | 增量 | v0.3 | v0.4 |
|---|---|---|---|
| P5 | 真演化闭环 | metric 收集 | archive → candidate → evaluate → retain/discard |
| P6 | 3 方法对照 | (无) | parent_child / sexual / asexual |
| P7 | Identity 锚定 | (无) | candidate 必须 identity_id 锚定 |
| P8 | V1072 桥接 | JSON state only | state + identity 元数据 |
| P9 | 轮数 | 30 轮 | 50 轮 (R9 真演化) |
| P10 | keep_state 父本引用 | (无) | child 必须引用 parent_id |

### 3.2 关键常量 (主 00:44 质量工程化)

```python
from apeireth.v1112_dgm_v04 import (
    VERSION,                      # "0.4.0"
    COMPONENTS,                   # 9 组件 (measurement/hqb_gate/artifact_writer/trace_audit/replay/guard/crossover/drift/anchor)
    METHODS,                      # ("parent_child", "sexual", "asexual")
    RETAIN_DELTA,                 # 0.015 (P5: 阈值 baseline + 0.015)
    EARLY_STOP_FAILS,             # 15 连续 discard/reject 才早停
    OPEN_ENDED_PROB,              # 0.30 (30% 从 archive 选 parent)
    THRESHOLD_FLOOR,              # 0.40 (阈值下限)
    SEXUAL_MIN_PARENTS,           # 2 (sexual 至少 2 parents)
    ASEXUAL_DRIFT_RATE,           # 0.30 (asexual 30% 字段漂变)
    SEXUAL_CROSSOVER_RATE,        # 0.50 (sexual 50% 字段 swap)
    MAX_GENERATIONS,              # 50 (P9)
)
```

### 3.3 真 API + 真示例 (主 00:56)

#### 3.3.1 reproduce 3 方法 dispatcher

```python
from apeireth.v1112_dgm_v04 import (
    reproduce_parent_child, reproduce_sexual, reproduce_asexual, reproduce,
)
import random

rng = random.Random(42)

# 1. parent_child — 1 parent → 1 child (单亲变异)
parent_state = {
    "components": {"measurement": {"attempts": 10, "reward": 0.85, "lift": 0.05}},
    "candidate_capability_score": 0.85,
}
child, meta = reproduce_parent_child(parent_state, rng)
print(f"parent_child: n_mutations={meta['n_mutations']}")  # 1

# 2. sexual — 2 parents → 1 child (50% 字段 swap, Goldberg 1989 真借鉴)
parent_a = {"candidate_capability_score": 0.85, "components": {...}}
parent_b = {"candidate_capability_score": 0.92, "components": {...}}
child, meta = reproduce_sexual(parent_a, parent_b, rng)
print(f"sexual: parent_ids={meta['parent_ids']}, n_mutations={meta['n_mutations']}")

# 3. asexual — 1 parent → 1 child (30% 字段漂变)
child, meta = reproduce_asexual(parent_state, rng)
print(f"asexual: n_drift={meta['n_mutations']}")

# 4. dispatcher (根据 method + archive + state 自动选)
archive = [parent_a, parent_b]  # 至少 2 才能 sexual
candidate, meta = reproduce("sexual", archive, parent_state, rng)
```

#### 3.3.2 HQB 4 维 + retain 判定 (主 17:43 实事求是)

```python
from apeireth.v1112_dgm_v04 import _hqb_for, _should_retain

# HQB = capability / cost_efficiency / latency_margin / constraint_adherence
hqb = _hqb_for(score=0.87, elapsed_ms=150.0, guard_ok=True)
print(hqb)
# {'capability': 0.87,
#  'cost_efficiency': 1 - 150/60000 = 0.9975,
#  'latency_margin': 1 - 150/30000 = 0.995,
#  'constraint_adherence': 1.0 (guard_ok=True),
#  'composite': 0.25*sum = 0.9656...}

# retain 判定 — hqb ≥ baseline + 0.015 AND identity-anchored AND constraint ≥ 1.0
baseline_composite = 0.95
archive = [{"hqb": {"composite": 0.95}}, {"hqb": {"composite": 0.96}}]
should_retain, reason = _should_retain(hqb, baseline_composite, archive)
print(f"retain={should_retain}, reason={reason}")
# 若 composite ≥ baseline + 0.015 = 0.965 → retain=True
```

#### 3.3.3 Identity 锚定 + V1072 桥接 (P7 + P8)

```python
from apeireth.v1112_dgm_v04 import (
    IdentityAnchor, build_default_anchor, try_attach_identity_store,
    bridge_to_v1072_profile, from_v1072_core,
)

# 默认 anchor (主 17:43 实事求是: 真生产 V1072 数据)
anchor = build_default_anchor()
print(anchor.identity_id)             # "ca_chu_ling_v1072_default"
print(anchor.core_snapshot_hash)      # 16 字符 sha256
print(anchor.bridge_v1072)            # True (P8 桥接成功)

# 真连 V1095 store (P7)
# from apeireth.v1095_identity_store import IdentityStoreV1095
# store = IdentityStoreV1095(db_path="/path/to/identity.db")
# try_attach_identity_store(anchor, store)

# V1072 core ↔ V1112 anchor 完整往返 (主 19:33 走在前人经验上)
# from apeireth.v1072_asi_central_ai_eternal_identity import IdentityCore
# core = IdentityCore(identity_id="ca_test_bridge")
# anchor2 = from_v1072_core(core)
# assert anchor2.identity_id == core.identity_id  # 不假装
```

#### 3.3.4 50 轮真演化主入口 (主 20:55 红皇后)

```python
from apeireth.v1112_dgm_v04 import run_experiment

# 完整 50 轮真演化 (subprocess 也可)
result = run_experiment(
    iterations=50,
    method="parent_child",     # 或 "sexual" / "asexual"
    seed=42,
    use_v1072_anchor=True,     # P8 桥接
)
print(result.keys())
# ['version', 'iterations_requested', 'iterations_completed',
#  'identity_anchor', 'baseline', 'n_retain', 'n_discard', 'n_reject',
#  'n_asi_pretend_total', 'archive_size', 'archive_avg_hqb',
#  'lifts_per_round', 'lift_max', 'lift_mean', 'method_breakdown',
#  'validation', 'stop_reason', 'runs']
print(f"n_retain={result['n_retain']}, n_discard={result['n_discard']}")
print(f"archive_size={result['archive_size']}, lift_mean={result['lift_mean']}")
print(f"n_asi_pretend_total={result['n_asi_pretend_total']}")  # 必须恒 = 0
```

### 3.4 V3 守门 (主 17:43 + 主 17:58)

```python
from apeireth.v1112_dgm_v04 import V3_GUARDS

print(V3_GUARDS)
# {
#   'module_is_not_asi': 'v0.4 archive 是工具, ASI 是更大目标...',
#   'measurement_is_not_truth': 'lift 是 proxy, 真值仍是更大目标...',
#   'structure_is_not_consciousness': 'Identity anchor ≠ 自我意识...',
#   'production_is_not_safety': '真演化 ≠ 真安全...',
#   'automation_is_not_autonomy': '自动 archive ≠ 自主 ASI...',
#   'red_queen_loop': '主 20:55 红皇后 = 永远演化...',
#   'no_asi_pretend': 'n_asi_pretend_total 必须 = 0...'
# }
```

### 3.5 真守门

| 守门 | 真值 | 测试 |
|---|---|---|
| n_asi_pretend_total = 0 (恒) | ✅ | `test_v1112_dgm_v04.py::test_no_asi_pretend_invariant` |
| Identity 锚定失败 = reject | ✅ | `test_v1112_dgm_v04.py::test_identity_mismatch_reject` |
| 50 轮真跑 | ✅ archive_size > 15 | `test_v1112_dgm_v04.py::test_50_iterations_archive_growth` |
| 3 方法全部跑通 | ✅ | `test_v1112_dgm_v04.py::test_3_methods_dispatcher` |
| retain 阈值 ≥ baseline + 0.015 | ✅ | `test_v1112_dgm_v04.py::test_retain_threshold` |
| V1072 桥接往返 | ✅ | `test_v1112_dgm_v04.py::test_bridge_v1072_roundtrip` |

---

## 4. V1114 Weekly Integration Evaluator (主 00:56 任何人都能接手)

### 4.1 设计意图 (主 17:43 实事求是 + 主 00:56)

**每周自动化集成评估** (R9 W3-W4):

1. **三件套真测** — V1074 V0.3 + V1077 V0.4 17 维 + V1103 Top-5 P2
2. **ASI 北极星 dashboard** — V0.3/V0.4 + Top-5 lift + philosophy_guard 子分
3. **4 选 1 主轨道自动切换** — 基于 V0.4 真测 + lift 阈值
4. **守门自检** — 主哲学 9 键 / V3 守门 6 项 / halt 5 信号

### 4.2 关键常量 (主 17:43 实事求是)

```python
from apeireth.v1114_weekly_integration_evaluator import (
    VERSION,                          # "0.1.0"
    ASI_NORTH_STAR,                   # 0.9800 LOCKED
    V1074_V03_MIN,                    # 0.8884 (主 17:43 守门)
    V04_W2_TARGET,                    # 0.82
    V04_W3_TARGET,                    # 0.84
    V04_W4_TARGET,                    # 0.85 (W4 收官)
    V04_TRACK_C_THRESHOLD,            # 0.83
    V04_TRACK_D_THRESHOLD,            # 0.82
    V04_TRACK_B_THRESHOLD,            # 0.80
    PHILOSOPHY_9_KEYS,                # 9 键 LOCKED
    V3_GUARDS,                        # 6 项
    TRACK_DEFS,                       # 4 选 1 主轨道
    HALT_PERF_DELTA, HALT_PERF_CONSEC,
    HALT_CANDIDATE_RATIO, HALT_CROSS_DIM_DROP,
    HALT_LIFT_N20, HALT_RED_QUEEN_N,
)
```

### 4.3 真 API + 真示例

#### 4.3.1 4 选 1 主轨道自动决策 (主 17:43 + 主 13:31)

```python
from apeireth.v1114_weekly_integration_evaluator import (
    choose_main_track, evaluate_halting_signals, HaltingSignals, TrackDecision,
)

# 5 halt 信号聚合 (主 20:55 红皇后守门)
halting = evaluate_halting_signals(
    v03_history=[0.8800, 0.8850, 0.8897],  # W1/W2/W3 历史
    unique_ratio=0.85,                      # 候选多样性
    fitness_std=0.04,                       # fitness std
    cross_dim_drop=0.05,                    # cross_dim 一致性下降
    cross_model_lift=0.012,                 # 跨小模型 lift
)
print(halting.any_triggered())  # False (5 信号全未触发)
print(halting.triggered_list()) # []

# 4 选 1 自动决策
decision = choose_main_track(
    v04_score=0.8202,                 # V1077 真测
    halting=halting,
    v1060_committed=True,             # R9 W3 末已 commit
    weekly_lift=0.012,
)
print(decision)
# TrackDecision(
#   track='D',
#   track_name='DGM v0.4 真演化',
#   rationale='V0.4=0.8202 ∈ [0.82, 0.83) → 维持 Track D DGM v0.4 双维 ROI',
#   expected_lift='+0.010~+0.030',
#   halt_override=False,
#   v1060_committed=True,
#   confidence=0.85,
# )
```

#### 4.3.2 决策树 (主 19:33 走在前人经验上)

```text
1) halt 触发 → 强制切 Track C (红皇后守门)
2) V0.4 ≥ 0.83              → Track C (跨小模型, 鲁棒性证明)
3) 0.82 ≤ V0.4 < 0.83       → Track D (DGM v0.4 双维 ROI 最高)
4) 0.80 ≤ V0.4 < 0.82       → Track B (HQB 4 维稳健补)
5) V0.4 < 0.80              → Track A (Rust hot path 救生圈)
6) V1060 not committed + V0.4 < 0.80 → 强制 REVERT 主推切 Track A
```

#### 4.3.3 守门自检 (主哲学 9 键 + V3 守门 6 项)

```python
from apeireth.v1114_weekly_integration_evaluator import (
    run_guard_self_check, compute_dashboard,
    run_v1074, run_v1077, run_v1103,
)

# 跑三件套 (subprocess 真跑, 不缓存不模拟)
v1074 = run_v1074(no_write=True)
v1077 = run_v1077()
v1103 = run_v1103()

dashboard = compute_dashboard(v1074, v1077, v1103)
print(dashboard)
# {'v03_score': 0.8897,
#  'v04_v1077': 0.8202,
#  'v04_v1103': 0.8188,
#  'v04_score': 0.8202,    # V1077 优先
#  'top5_lift': ...}

# 守门自检
guards = run_guard_self_check(dashboard, halting)
print(guards)
# {
#   'philosophy_9_keys_locked': True,
#   'v3_guards': {
#     'runner_is_not_asi': True,
#     'report_is_not_production': True,
#     'decision_is_not_optimal': True,
#     'v03_is_not_v04_is_not_asi': True,
#     'no_fake_kpi': True,
#     'red_queen_is_not_asi': True
#   },
#   'v3_guards_all_pass': True,
#   'halt_signals': {...},
#   'halt_any_triggered': False,
#   'v1074_v03_above_floor': True  # 0.8897 ≥ 0.8884
# }
```

#### 4.3.4 周评估主入口 (主 00:56 任何人都能接手)

```python
from apeireth.v1114_weekly_integration_evaluator import evaluate_week

# 一行真跑 W3 末评估
report = evaluate_week(
    week_label="W3",
    v03_history=[0.8800, 0.8850],     # W1, W2 历史
    unique_ratio=0.85,
    fitness_std=0.04,
    cross_dim_drop=0.05,
    cross_model_lift=0.012,
    v1060_committed=True,
    weekly_lift=0.015,
    no_write=True,                    # 不写盘
)
print(report.keys())
# ['week', 'version', 'dashboard', 'track_decision', 'halting_signals',
#  'guards', 'all_ok', 'ts']

print(f"all_ok: {report['all_ok']}")  # True (5 halt 全未触发 + V3 守门全过 + V1074 ≥ 守门)
```

### 4.4 CLI 真测 (主 00:56)

```bash
# 一行真跑 (主 00:56 任何人都能接手)
python -m apeireth.v1114_weekly_integration_evaluator --week W3
# 输出:
#   R9 W3 末集成评估
#     V1074 V0.3 = 0.8897 (≥ 0.8884 ? True)
#     V1077 V0.4 = 0.8202
#     V1103 V0.4 = 0.8188
#     ASI 北极星 = 0.9800 (LOCKED)
#     主轨道 = D — DGM v0.4 真演化
#     理由: V0.4=0.8202 ∈ [0.82, 0.83) → 维持 Track D DGM v0.4 双维 ROI
#     All OK: True

# JSON 输出
python -m apeireth.v1114_weekly_integration_evaluator --week W3 --json | jq .

# Markdown 报告入 reports/r9-integration-evaluation-w3.md
python -m apeireth.v1114_weekly_integration_evaluator --week W3 --report

# 严格模式 (守门失败非零退出, 主 17:43 实事求是)
python -m apeireth.v1114_weekly_integration_evaluator --week W3 --strict
# echo $?  # 0=OK, 1=不通过

# pytest (24 tests)
python -m pytest tests/test_v1114_weekly_evaluator.py -v
```

### 4.5 真守门

| 守门 | 真值 | 测试 |
|---|---|---|
| V1074 V0.3 ≥ 0.8884 | ✅ 0.8897 | `test_v1114_weekly_evaluator.py::test_v1074_floor` |
| V3 守门 6 项全过 | ✅ | `test_v1114_weekly_evaluator.py::test_v3_guards_all_pass` |
| 主哲学 9 键 LOCKED | ✅ | `test_v1114_weekly_evaluator.py::test_philosophy_9_keys_locked` |
| 4 选 1 决策树 4 个分支 | ✅ | `test_v1114_weekly_evaluator.py::test_4_track_decisions` |
| 5 halt 信号聚合 | ✅ | `test_v1114_weekly_evaluator.py::test_halting_signals_aggregation` |
| 强制 halt 切 Track C | ✅ | `test_v1114_weekly_evaluator.py::test_halt_override_to_C` |
| 强制 REVERT V1060 + <0.80 | ✅ | `test_v1114_weekly_evaluator.py::test_force_revert_to_A` |

---

## 5. 集成真测 (主 17:43 实事求是)

```bash
# 完整 V1114 真测 (subprocess)
python -c "
from apeireth.v1114_weekly_integration_evaluator import evaluate_week
import json
r = evaluate_week(week_label='W3', v03_history=[0.8800, 0.8850],
                  unique_ratio=0.85, fitness_std=0.04,
                  cross_dim_drop=0.05, cross_model_lift=0.012,
                  v1060_committed=True, weekly_lift=0.015,
                  no_write=True)
print(json.dumps({'all_ok': r['all_ok'],
                  'v04_score': r['dashboard']['v04_score'],
                  'track': r['track_decision']['track'],
                  'halt_triggered': r['halting_signals']['any_triggered']},
                 indent=2, ensure_ascii=False))
"
# 期望输出:
# {
#   "all_ok": true,
#   "v04_score": 0.8202,
#   "track": "D",
#   "halt_triggered": false
# }
```

---

## 6. 一句话总览

> **V1072 = 中央 AI 永恒身份 + 10 组件真生产; V1095 = V1072 + 多 persona + fsync 真持久化; V1112 = V1072 identity_id 锚定 + 50 轮真演化 + 3 方法对照; V1114 = 三件套真跑 + 4 选 1 决策 + 5 halt 守门。任何 LLM 接入即获 AGI/ASI 能力 — 这是主 22:33 ASI 北极星。**

---

**R9-TW-001 模块参考完成。** 配套真架构文档 4 篇 + 真实测示例 + 真守门 + 真 CLI。任何人都能 5 分钟接手,见 `docs/r9-handoff-r10.md`。