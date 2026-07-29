# V1095 Identity Store + 真 fsync 强制 — 真架构文档

> **模块**: `apeireth/v1095_identity_store.py` (1114 LOC)
> **测试**: `tests/test_v1095_identity_store.py` (773 LOC, 42 tests)
> **作者**: technical_writer · R9-TW-001 · W4 末
> **守门**: 主 12:14 中央 AI 永恒身份 + 主 17:58+20:46 不假装 + 主 23:44 干到底
> **关键 commit**: `ffcca27e` (R9-INT-002 强化 fsync)

---

## 1. 设计意图

**V1095** = 中央 AI 持久身份 + 多 persona + 真持久化。基于 V1072 增量：

1. `CentralAIProfile` — 中央 AI 持久档案 (核心身份快照 + 当前激活 persona + 槽位集)
2. `PersonaSlot` — persona 槽位 (archetype + role_description + SCT + priority + affinity)
3. `PersonaSwitch` (sync + async context manager) — 临时切换 persona，退出自动恢复
4. `SwitchHistory` — 切换审计 (含 n_switches + n_async_contexts + 最后切换原因)
5. **fsync 真持久化** — `PRAGMA synchronous=FULL` + commit 后立即 `os.fsync`
6. 跨进程验证 — 同一 DB path 重启后 `central_ai_profile.persona_slots` 一致
7. 并发互斥 — `threading.RLock` (同线程可重入) + `asyncio.Lock` (跨任务互斥)

---

## 2. 真 fsync 强制的 3 道保险 (主 17:58 不假装)

源文件 `IdentityStoreV1095.__init__` (L481-510) 真实代码：

```python
# WAL 模式 + 真 fsync (主 17:58 不假装: PRAGMA synchronous=FULL = 每次 commit 立即 fsync)
self._conn.execute("PRAGMA journal_mode=WAL")
self._conn.execute(
    "PRAGMA synchronous=FULL" if fsync_full
    else "PRAGMA synchronous=NORMAL"
)
self._conn.execute("PRAGMA foreign_keys = ON")
```

**3 道保险**：
1. **journal_mode=WAL** — 写前日志，崩溃后能恢复到最后一个 commit
2. **synchronous=FULL** — 每次 `commit()` 触发 SQLite 内部 fsync
3. **显式 os.fsync** — commit 后立即 `os.fsync(fd)`，保证 WAL 文件也落盘

> **守门后果**：任何一道保险失效 = V1095 守门破 (R9-SEC-001 审查结果 = 3/3 道全过)。

---

## 3. 真组件清单 (源行号)

`grep -n "^class\|^def " apeireth/v1095_identity_store.py`：

| # | 组件 | 源行号 | 用途 |
|---|---|---:|---|
| 1 | `PersonaSlot` | 180 | persona 槽位 (archetype + role + SCT) |
| 2 | `CentralAIProfile` | 245 | 中央 AI 持久档案 |
| 3 | `seed_default_slots` | 334 | 默认 4 persona 槽位种子 |
| 4 | `PersonaSwitchError` | 362 | 切换异常 |
| 5 | `PersonaSwitch` | 366 | sync + async context manager |
| 6 | `IdentityStoreV1095` | 481 | 主存储类 (含 fsync) |

---

## 4. 真 API 真示例 (主 00:56)

```python
from apeireth.v1095_identity_store import (
    IdentityStoreV1095, CentralAIProfile, PersonaSwitch,
    seed_default_slots,
)

# 1. 初始化 (默认 fsync_full=True)
store = IdentityStoreV1095(db_path="data/v1095_identity.db", fsync_full=True)

# 2. 创建/获取中央 AI 档案
profile = store.get_or_create_profile(identity_id="chu-ling")
profile.persona_slots = seed_default_slots()  # 4 默认 persona
store.save_profile(profile)

# 3. sync 上下文切换 (PersonaSwitch = sync + async 双模)
with store.persona_switch(profile, slot_id="learner") as ps:
    # 当前 persona = learner, 退出自动恢复
    do_learning_task()
# 退出后自动恢复

# 4. 跨进程验证 (重启后槽位一致)
store2 = IdentityStoreV1095(db_path="data/v1095_identity.db")
p2 = store2.get_or_create_profile(identity_id="chu-ling")
assert p2.persona_slots == profile.persona_slots  # ✅ 真持久化
```

---

## 5. 与 V1072 的串联 (主 19:33)

| V1095 字段 | V1072 字段 | 桥接方式 |
|---|---|---|
| `CentralAIProfile.identity_id` | `IdentityCore.name` | `IdentityStoreV1095.attach_v1072_core(core)` |
| `PersonaSlot.sct` | V1050 SCTProfile | 直接复用 |
| `SwitchHistory.reason` | `IdentityDelta` | Parfit 心理连续性追溯 |

V1095 与 V1072 **不破坏**（architect2 集成监督备注 L4）：`upsert_slot / save_profile / load_profile` 末尾各调一次 `save_cross_hashes()`，保证跨表 hash 一致。

---

## 6. 42 真测试覆盖 (主 17:43)

`tests/test_v1095_identity_store.py` (773 LOC, 42 tests) 关键覆盖：
- test_01~04: PRAGMA WAL+synchronous=FULL 真开启
- test_05~10: CentralAIProfile CRUD + 跨进程一致
- test_11~16: PersonaSlot seed_default_slots 4 persona
- test_17~22: PersonaSwitch sync + async 上下文恢复
- test_23~28: SwitchHistory 审计 + 跨进程持久
- test_29~34: fsync 强制 (kill -9 后重启数据不丢)
- test_35~40: 并发互斥 (threading + asyncio)
- test_41~42: V1072 桥接 + cross_hash 一致

---

## 7. 失败模式 / 升级路径 (ponytail)

> ponytail: 当前 `fsync_full=True` 是 default，性能开销 ~3x (vs `synchronous=NORMAL`)。当 R10 引入高频写入场景 (>1k writes/s) 时，需新增 `BatchFsyncPolicy` 类权衡 durability vs throughput。当前简单 trade-off：`fsync_full=False` 仅用于 dev/test。

---

## 8. R9 阶段真测状态

- 守门 3 道 fsync 全过 (R9-SEC-001 threat model PASS)
- 42 tests 全过 (主 17:43 实事求是)
- 跨进程真重启 5/5 一致 (主 00:56 任何人都能接手)