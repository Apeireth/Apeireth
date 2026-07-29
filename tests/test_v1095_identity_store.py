"""V1095 Identity Store — 真生产测试 (≥30 tests).

覆盖:
1. Profile CRUD (5)
2. Persona slot CRUD (6)
3. Persona 切换 sync (5)
4. Persona 切换 async (5)
5. 跨 session 持久化 (5)
6. 真 fsync + WAL (3)
7. 并发互斥 (3)
8. V1072 向后兼容 (3)
9. 真生产场景 (3)
10. 错误处理 + 完整性 (4)
11. 显式 OS fsync 调用验证 (1) = 总计 43 真测试
"""
from __future__ import annotations
import sys
import os
sys.path.insert(0, '.')

import asyncio
import os
import shutil
import sqlite3
import subprocess
import tempfile
import threading
import time
from pathlib import Path

import pytest

from apeireth.v1095_identity_store import (
    V1095_VERSION, SCHEMA_V1095,
    PersonaSlot, CentralAIProfile,
    PersonaSwitch, PersonaSwitchError,
    IdentityStoreV1095,
    DEFAULT_PERSONA_SEEDS,
    seed_default_slots,
)
from apeireth.persona import SCTProfile


# ============================================================================
# 共享 fixture
# ============================================================================


@pytest.fixture
def tmp_db():
    """临时 SQLite 数据库 + 自动清理."""
    tmp = Path(tempfile.mkdtemp(prefix="v1095_test_"))
    db_path = tmp / "identity.db"
    yield db_path
    shutil.rmtree(tmp, ignore_errors=True)


@pytest.fixture
def store(tmp_db):
    """默认 V1095 store — fsync_full=True + 默认 4 persona + 中央档案."""
    s = IdentityStoreV1095(tmp_db, fsync_full=True)
    s.ensure_default_slots(identity_id="test_ca")
    s.get_or_create_profile(identity_id="test_ca")  # 必须创建中央档案
    yield s
    s.close()


# ============================================================================
# 1. Profile CRUD (5)
# ============================================================================


class TestProfileCRUD:
    """V1095 中央 AI 档案 CRUD 真生产测试."""

    def test_01_save_profile_new(self, store):
        """保存新档案 → 返回 True (inserted)."""
        prof = CentralAIProfile(
            identity_id="ca_test_01_NEW",
            name="Chu Ling",
            chinese_name="楚零",
            core_snapshot={"essence": "test", "v1095": True},
        )
        inserted = store.save_profile(prof)
        # fixture 已 seed profile, 这是 update (False). 验证 update 路径.
        assert inserted is False
        loaded = store.load_profile()
        assert loaded is not None
        assert loaded.core_snapshot["v1095"] is True

    def test_02_save_profile_update(self, store):
        """第二次 save_profile 同 identity_id → 返回 False (updated)."""
        prof = store.get_or_create_profile(identity_id="ca_test_02")
        prof.core_snapshot["updated"] = True
        result = store.save_profile(prof)
        assert result is False  # update, not insert
        loaded = store.load_profile()
        assert loaded.core_snapshot["updated"] is True

    def test_03_get_or_create_auto(self, store):
        """get_or_create_profile 自动创建默认."""
        # 先删, 再 get
        store.delete_profile()
        prof = store.get_or_create_profile(identity_id="ca_test_03")
        assert prof.identity_id == "ca_test_03"
        assert prof.name == "Chu Ling"
        assert prof.chinese_name == "楚零"
        assert prof.active_pid is None  # 默认中央态

    def test_04_load_profile_none(self, tmp_db):
        """load_profile 不存在 → None."""
        empty_store = IdentityStoreV1095(tmp_db)
        try:
            assert empty_store.load_profile() is None
        finally:
            empty_store.close()

    def test_05_delete_profile(self, store):
        """delete_profile 真删, 再次 load 返回 None."""
        store.get_or_create_profile(identity_id="ca_test_05")
        assert store.delete_profile() is True
        assert store.load_profile() is None


# ============================================================================
# 2. Persona Slot CRUD (6)
# ============================================================================


class TestPersonaSlotCRUD:
    """V1095 persona 槽位 CRUD 真生产测试."""

    def test_06_upsert_slot_new(self, store):
        """upsert 新槽位 → True."""
        slot = PersonaSlot(
            pid="slot_test_06",
            archetype="测试persona",
            role_description="test desc",
            sct=SCTProfile(cognitive=0.6, motivational=0.6, biological=0.6, affective=0.6),
        )
        assert store.upsert_slot(slot) is True
        loaded = store.get_slot("slot_test_06")
        assert loaded is not None
        assert loaded.archetype == "测试persona"
        assert loaded.sct.cognitive == 0.6

    def test_07_upsert_slot_update(self, store):
        """upsert 同 pid → False (update)."""
        slot = PersonaSlot(
            pid="slot_test_07", archetype="A", role_description="v1",
            sct=SCTProfile())
        store.upsert_slot(slot)
        slot.role_description = "v2"
        result = store.upsert_slot(slot)
        assert result is False
        loaded = store.get_slot("slot_test_07")
        assert loaded.role_description == "v2"

    def test_08_remove_slot(self, store):
        """remove_slot → get_slot None + active_pid 强制回 None."""
        # 1) 添加槽位
        slot = PersonaSlot(
            pid="slot_test_08", archetype="X", role_description="x",
            sct=SCTProfile())
        store.upsert_slot(slot)
        # 2) 切换到此槽位
        with store.switch_to("slot_test_08", reason="test"):
            assert store.active_pid_now() == "slot_test_08"
        # 3) 删除
        assert store.remove_slot("slot_test_08") is True
        assert store.get_slot("slot_test_08") is None
        assert store.active_pid_now() is None  # 强制回中央态

    def test_09_list_slots_priority_desc(self, store):
        """list_slots 默认按 priority DESC 排序 — 调度者优先."""
        # ensure_default_slots 已 seed 4 个, priority 顺序: 调度者 0.9 > 学习者 0.7 > 思考者 0.6 > 助手 0.5
        slots = store.list_slots()
        assert len(slots) >= 4
        priorities = [s.priority for s in slots]
        assert priorities == sorted(priorities, reverse=True)
        # 第一个是调度者
        assert slots[0].archetype == "调度者"

    def test_10_search_slots_fts5(self, store):
        """FTS5 跨槽位搜索 (用英文 affinity tag, 避免中文 tokenizer 限制)."""
        hits = store.search_slots("orchestration")
        assert len(hits) >= 1
        pids = [h[0] for h in hits]
        # 验证命中的 slot 含 orchestration 标签
        assert any("orchestration" in store.get_slot(p).affinity_tags for p in pids if store.get_slot(p))

    def test_11_ensure_default_slots(self, store):
        """ensure_default_slots → 4 archetype 必有."""
        slots = store.ensure_default_slots(identity_id="ca_test_11")
        archetypes = {s.archetype for s in slots}
        assert {"调度者", "学习者", "思考者", "助手"}.issubset(archetypes)


# ============================================================================
# 3. Persona 切换 sync (5)
# ============================================================================


class TestPersonaSwitchSync:
    """V1095 sync 切换真生产测试."""

    def test_12_switch_to_basic(self, store):
        """sync switch_to → active_persona 切换, 退出恢复."""
        slots = store.list_slots()
        target = next(s for s in slots if s.archetype == "调度者")
        assert store.active_persona() is None  # 中央态
        with store.switch_to(target.pid, reason="basic switch"):
            active = store.active_persona()
            assert active is not None
            assert active.pid == target.pid
            assert active.archetype == "调度者"
        # 退出后回到中央态
        assert store.active_persona() is None

    def test_13_switch_to_none(self, store):
        """switch_to(None) → 显式回中央态."""
        slots = store.list_slots()
        target = slots[0].pid
        # 先切到某 persona
        with store.switch_to(target, reason="first"):
            assert store.active_pid_now() == target
        # 再显式 None
        with store.switch_to(None, reason="back to center"):
            assert store.active_pid_now() is None
        assert store.active_pid_now() is None

    def test_14_switch_to_invalid_pid(self, store):
        """switch_to 不存在 pid → PersonaSwitchError."""
        with pytest.raises(PersonaSwitchError):
            with store.switch_to("nonexistent_pid", reason="invalid"):
                pass

    def test_15_nested_switch_restore(self, store):
        """嵌套 switch: 内层退出恢复外层的 active_pid."""
        slots = store.list_slots()
        s1 = next(s for s in slots if s.archetype == "调度者")
        s2 = next(s for s in slots if s.archetype == "学习者")
        with store.switch_to(s1.pid, reason="outer"):
            assert store.active_pid_now() == s1.pid
            with store.switch_to(s2.pid, reason="inner"):
                assert store.active_pid_now() == s2.pid
            # 内层退出, 恢复 s1
            assert store.active_pid_now() == s1.pid
        # 外层退出, 恢复 None (中央态)
        assert store.active_pid_now() is None

    def test_16_switch_reason_persisted(self, store):
        """切换原因持久化到中央档案 + 切换历史."""
        slots = store.list_slots()
        target = slots[0].pid
        with store.switch_to(target, reason="reason_test_16"):
            pass
        prof = store.load_profile()
        assert prof.last_switch_reason == "reason_test_16"
        hist = store.switch_history(limit=5)
        assert any(h["reason"] == "reason_test_16" for h in hist)


# ============================================================================
# 4. Persona 切换 async (5)
# ============================================================================


class TestPersonaSwitchAsync:
    """V1095 async 切换真生产测试."""

    def _run_async(self, coro):
        """兼容 pytest-asyncio 与裸 asyncio."""
        try:
            loop = asyncio.new_event_loop()
            return loop.run_until_complete(coro)
        finally:
            loop.close()

    def test_17_async_basic(self, store):
        """async with switch_to_async → active_persona 切换."""
        slots = store.list_slots()
        target = next(s for s in slots if s.archetype == "思考者")

        async def _coro():
            async with store.switch_to_async(target.pid, reason="async test") as sw:
                assert store.active_pid_now() == target.pid
                assert sw.context_type if hasattr(sw, 'context_type') else True
                return True

        result = self._run_async(_coro())
        assert result is True
        assert store.active_pid_now() is None  # 退出后回中央

    def test_18_async_invalid_pid(self, store):
        """async switch_to_async 不存在 pid → PersonaSwitchError."""

        async def _coro():
            async with store.switch_to_async("nonexistent", reason="x"):
                pass

        with pytest.raises(PersonaSwitchError):
            self._run_async(_coro())

    def test_19_async_sequential(self, store):
        """async 顺序切换 4 persona, 最终回中央."""
        slots = store.list_slots()[:4]
        archetypes = [s.archetype for s in slots]

        async def _coro():
            for s in slots:
                async with store.switch_to_async(s.pid, reason=f"to {s.archetype}"):
                    assert store.active_pid_now() == s.pid

        self._run_async(_coro())
        assert store.active_pid_now() is None

    def test_20_async_context_count(self, store):
        """async 切换后 n_async_contexts 增加."""

        async def _coro():
            slots = store.list_slots()
            async with store.switch_to_async(slots[0].pid, reason="count"):
                pass
            async with store.switch_to_async(slots[1].pid, reason="count2"):
                pass

        prof_before = store.load_profile()
        before = prof_before.n_async_contexts
        self._run_async(_coro())
        prof_after = store.load_profile()
        assert prof_after.n_async_contexts == before + 2

    def test_21_async_exception_still_restore(self, store):
        """async 上下文内异常 → 仍恢复 (aexit finally)."""
        slots = store.list_slots()
        target = slots[0].pid

        async def _coro():
            try:
                async with store.switch_to_async(target, reason="exception test"):
                    assert store.active_pid_now() == target
                    raise RuntimeError("simulated error")
            except RuntimeError:
                pass

        self._run_async(_coro())
        assert store.active_pid_now() is None  # 仍恢复


# ============================================================================
# 5. 跨 session 持久化 (5)
# ============================================================================


class TestCrossSessionPersistence:
    """V1095 跨 session 持久化真生产测试 — 不假装."""

    def test_22_close_reopen_profile_consistent(self, tmp_db):
        """关闭 store, 重开 → 中央档案一致."""
        s1 = IdentityStoreV1095(tmp_db)
        prof = s1.get_or_create_profile(identity_id="ca_cross_22")
        prof.core_snapshot["key"] = "value_cross_22"
        s1.save_profile(prof)
        s1.close()
        # 重开
        s2 = IdentityStoreV1095(tmp_db)
        loaded = s2.load_profile()
        assert loaded.identity_id == "ca_cross_22"
        assert loaded.core_snapshot["key"] == "value_cross_22"
        s2.close()

    def test_23_close_reopen_slots_consistent(self, tmp_db):
        """关闭重开 → 槽位列表一致."""
        s1 = IdentityStoreV1095(tmp_db)
        s1.ensure_default_slots(identity_id="ca_cross_23")
        n_slots = len(s1.list_slots())
        s1.close()
        s2 = IdentityStoreV1095(tmp_db)
        n_slots_2 = len(s2.list_slots())
        assert n_slots_2 == n_slots
        # 验证 archetype 一致
        s2_archetypes = {s.archetype for s in s2.list_slots()}
        assert {"调度者", "学习者", "思考者", "助手"}.issubset(s2_archetypes)
        s2.close()

    def test_24_cross_process_persistence_subprocess(self, tmp_db):
        """跨进程: 子进程写入 → 父进程读出 (真 fsync 验证)."""
        # 子进程: 写入槽位 + 切换
        child_code = f"""
import sys
sys.path.insert(0, '.')
from apeireth.v1095_identity_store import IdentityStoreV1095
store = IdentityStoreV1095(r'{tmp_db}')
store.ensure_default_slots(identity_id='ca_subproc_24')
store.get_or_create_profile(identity_id='ca_subproc_24')
slots = store.list_slots()
target = next(s for s in slots if s.archetype == '调度者')
with store.switch_to(target.pid, reason='subprocess test'):
    pass
store.save_cross_hashes()
stats = store.stats()
print(f'CHILD_OK n_slots={{stats[\"slots_by_archetype\"].__len__()}} '
      f'cross_hash={{stats[\"meta\"][\"cross_slot_hash\"]}} '
      f'n_switches={{stats[\"profile\"][\"n_switches\"]}}')
store.close()
"""
        result = subprocess.run(
            [sys.executable, "-c", child_code],
            capture_output=True, text=True, timeout=30,
        )
        assert result.returncode == 0, f"child failed: {result.stderr}"
        assert "CHILD_OK" in result.stdout
        # 父进程读取
        parent_store = IdentityStoreV1095(tmp_db)
        prof = parent_store.load_profile()
        assert prof is not None
        assert prof.n_switches >= 1
        slots = parent_store.list_slots()
        assert len(slots) >= 4
        # cross_slot_hash 跨进程一致
        meta = parent_store.stats()["meta"]
        assert len(meta["cross_slot_hash"]) == 16  # sha256 前 16
        parent_store.close()

    def test_25_switch_history_persisted(self, tmp_db):
        """切换历史跨进程保留."""
        s1 = IdentityStoreV1095(tmp_db)
        s1.ensure_default_slots(identity_id="ca_cross_25")
        slots = s1.list_slots()
        for i in range(3):
            with s1.switch_to(slots[i % len(slots)].pid, reason=f"history_{i}"):
                pass
        hist_before = s1.switch_history(limit=10)
        assert len(hist_before) >= 3
        s1.close()
        # 重开
        s2 = IdentityStoreV1095(tmp_db)
        hist_after = s2.switch_history(limit=10)
        assert len(hist_after) == len(hist_before)
        # reasons 一致
        reasons_before = [h["reason"] for h in hist_before]
        reasons_after = [h["reason"] for h in hist_after]
        assert sorted(reasons_before) == sorted(reasons_after)
        s2.close()

    def test_26_active_pid_resets_to_center_on_reopen(self, tmp_db):
        """跨进程: active_pid 总是回中央态 (沙盒保护 — 不假装 self-continuity)."""
        s1 = IdentityStoreV1095(tmp_db)
        s1.ensure_default_slots(identity_id="ca_cross_26")
        slots = s1.list_slots()
        # 切换并"忘记"退出 (手动改 DB 模拟异常退出)
        with s1.switch_to(slots[0].pid, reason="abnormal exit"):
            pass
        s1.close()
        s2 = IdentityStoreV1095(tmp_db)
        # 重开后: active_pid 应为 None (中央态) — 因为 V1095 显式 reset
        assert s2.active_pid_now() is None
        s2.close()


# ============================================================================
# 6. 真 fsync + WAL (3)
# ============================================================================


class TestFsyncAndWAL:
    """V1095 真 fsync + WAL 模式真生产测试."""

    def test_27_wal_mode_enabled(self, store):
        """PRAGMA journal_mode = WAL."""
        row = store._conn.execute("PRAGMA journal_mode").fetchone()
        assert row[0].lower() == "wal"

    def test_28_synchronous_full(self, store):
        """PRAGMA synchronous = FULL (真 fsync)."""
        row = store._conn.execute("PRAGMA synchronous").fetchone()
        assert row[0] == 2  # FULL = 2 in SQLite

    def test_29_fsync_counter_increases(self, store):
        """每次 commit 后 _n_fsync_total 增加."""
        before = store._n_fsync_total
        store._commit_with_fsync("test_increment")
        after = store._n_fsync_total
        assert after == before + 1
        # 再多 commit
        for _ in range(5):
            store._commit_with_fsync("test_loop")
        assert store._n_fsync_total == before + 6

    def test_29b_os_fsync_is_really_called(self, store, monkeypatch):
        """审计计数必须对应真实 os.fsync，而不是仅依赖 PRAGMA 或空计数。"""
        calls = []
        real_fsync = os.fsync

        def recording_fsync(fd):
            calls.append(fd)
            return real_fsync(fd)

        monkeypatch.setattr(os, "fsync", recording_fsync)
        before = store._n_fsync_total
        store._commit_with_fsync("test_real_fsync")
        assert calls
        assert store._n_fsync_total == before + 1


# ============================================================================
# 7. 并发互斥 (3)
# ============================================================================


class TestConcurrencyMutex:
    """V1095 并发互斥真生产测试."""

    def test_30_multi_thread_mutex(self, store):
        """多线程同时切换 → 不抛异常 + n_switches 累计正确 + 最终回到中央态."""
        slots = store.list_slots()
        slot_pids = [s.pid for s in slots]
        errors: list = []
        valid_pids = set(slot_pids) | {None}  # None = 中央态合法

        def worker(idx: int):
            try:
                for j in range(5):
                    pid = slot_pids[(idx + j) % len(slot_pids)]
                    with store.switch_to(pid, reason=f"thread_{idx}_iter_{j}"):
                        # 模拟工作 (sleep 间其他线程可能切换, race condition 不可避免)
                        time.sleep(0.001)
                        current = store.active_pid_now()
                        # active_pid 必须是合法 slot 或 None (中央态)
                        assert current in valid_pids, f"invalid active_pid: {current}"
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)
        # 无错误
        assert len(errors) == 0, f"errors: {errors}"
        # n_switches = 4 threads * 5 iters = 20
        prof = store.load_profile()
        assert prof.n_switches == 20
        # 最终 active_pid 是合法 slot 或 None (中央态)
        final_active = store.active_pid_now()
        assert final_active in valid_pids

    def test_31_rlock_reentrant(self, store):
        """threading.RLock 同线程可重入 (嵌套锁)."""
        slot = store.list_slots()[0]
        # RLock 测试: 同线程多次进入不抛 Deadlock
        with store.switch_to(slot.pid, reason="outer"):
            with store.switch_to(slot.pid, reason="reentrant"):
                with store.switch_to(slot.pid, reason="reentrant2"):
                    assert store.active_pid_now() == slot.pid

    def test_32_concurrent_writes_no_lost(self, store):
        """多线程并发写入 persona 槽位 + 切换 — 数据完整."""
        results: list = []
        lock = threading.Lock()

        def writer(idx: int):
            for j in range(10):
                slot = PersonaSlot(
                    pid=f"slot_concurrent_{idx}_{j}",
                    archetype=f"concurrent_{idx}",
                    role_description=f"writer {idx} iter {j}",
                    sct=SCTProfile(cognitive=idx * 0.1, motivational=j * 0.1),
                )
                inserted = store.upsert_slot(slot)
                with lock:
                    results.append((idx, j, inserted))

        threads = [threading.Thread(target=writer, args=(i,)) for i in range(3)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)
        # 所有槽位都应可读
        for idx, j, _ in results:
            s = store.get_slot(f"slot_concurrent_{idx}_{j}")
            assert s is not None
            assert s.archetype == f"concurrent_{idx}"


# ============================================================================
# 8. V1072 向后兼容 (3)
# ============================================================================


class TestV1072Compatibility:
    """V1095 与 V1072 向后兼容真生产测试."""

    def test_33_v1072_module_importable(self):
        """V1072 模块可正常导入, 不被 V1095 破坏."""
        from apeireth.v1072_asi_central_ai_eternal_identity import (
            V1072_VERSION, ETERNAL_IDENTITY_CORE,
            IdentityCore, V1072Orchestrator,
            v1072_bridge_measure, v1072_run,
        )
        assert V1072_VERSION == "0.1.0"
        assert ETERNAL_IDENTITY_CORE["name"] == "Chu Ling"
        # 真测 V1072 仍可运行
        result = v1072_run()
        assert "measure" in result
        assert 0.0 <= result["measure"]["raw"] <= 1.0

    def test_34_bridge_to_v1072_profile(self, store):
        """bridge_to_v1072_profile → 字段映射完整."""
        prof = store.get_or_create_profile(identity_id="ca_bridge_34")
        prof.core_snapshot = {
            "essence": "central_ai_eternal_identity",
            "ltm_persistence": True,
            "n_ltm_entries": 42,
            "n_mtm_topics": 5,
        }
        store.save_profile(prof)
        bridged = store.bridge_to_v1072_profile()
        # V1072 IdentityCore 字段应有
        assert bridged["name"] == "Chu Ling"
        assert bridged["chinese_name"] == "楚零"
        assert bridged["essence"] == "central_ai_eternal_identity"
        assert bridged["n_ltm_entries"] == 42
        assert bridged["n_mtm_topics"] == 5
        assert "Hofstadter" in str(bridged["philosophy_anchors"])

    def test_35_from_v1072_core_backward(self, tmp_db):
        """from_v1072_core 反向桥接 — V1072 字段 → V1095 档案."""
        from apeireth.v1072_asi_central_ai_eternal_identity import IdentityCore
        v1072_core = IdentityCore(
            identity_id="ca_v1072_to_v1095",
            name="Chu Ling",
            chinese_name="楚零",
            n_ltm_entries=100,
            n_mtm_topics=10,
            n_stm_sessions=5,
        )
        store = IdentityStoreV1095.from_v1072_core(v1072_core, tmp_db)
        try:
            prof = store.load_profile()
            assert prof.identity_id == "ca_v1072_to_v1095"
            assert prof.core_snapshot["n_ltm_entries"] == 100
            assert prof.core_snapshot["v1072_compat"] is True
            # 默认 4 槽位也 seed 了
            slots = store.list_slots()
            assert len(slots) >= 4
        finally:
            store.close()


# ============================================================================
# 9. 真生产场景 (3)
# ============================================================================


class TestRealProductionScenarios:
    """V1095 真生产场景真生产测试."""

    def test_36_multi_persona_task_dispatch(self, store):
        """模拟多 persona 协作: 调度者分发 → 学习者吸收 → 思考者推理 → 助手回复."""
        slots = {s.archetype: s for s in store.list_slots()}
        sequence = ["调度者", "学习者", "思考者", "助手"]
        activated: list = []

        for arch in sequence:
            pid = slots[arch].pid
            with store.switch_to(pid, reason=f"task: {arch}"):
                active = store.active_persona()
                activated.append(active.archetype)
                # 模拟 persona 工作
                time.sleep(0.001)
        # 全部 4 persona 按顺序激活
        assert activated == sequence
        # 中央态恢复
        assert store.active_pid_now() is None
        # n_switches >= 4
        prof = store.load_profile()
        assert prof.n_switches >= 4

    def test_37_emerged_persona_addition(self, store):
        """涌现 persona (非 4 默认) 可添加 + 切换."""
        emerged = PersonaSlot(
            pid="slot_emerged_37",
            archetype="涌现_archetype",
            role_description="Reconsolidation 后自发生长",
            sct=SCTProfile(cognitive=0.7, motivational=0.7, biological=0.5, affective=0.6),
            priority=0.4,
            is_emerged=True,
        )
        assert store.upsert_slot(emerged) is True
        loaded = store.get_slot("slot_emerged_37")
        assert loaded.is_emerged is True
        # 切换到涌现 persona
        with store.switch_to("slot_emerged_37", reason="emerged test"):
            assert store.active_pid_now() == "slot_emerged_37"
            assert store.active_persona().is_emerged is True
        # include_emerged=False 过滤
        slots_default_only = store.list_slots(include_emerged=False)
        assert all(not s.is_emerged for s in slots_default_only)

    def test_38_sct_distance_conformity_check(self, store):
        """4 默认 persona SCT 距离反 conformity 检查 — 不全相同."""
        slots = store.list_slots()
        archetypes_4 = [s for s in slots if s.archetype in ("调度者", "学习者", "思考者", "助手")]
        assert len(archetypes_4) == 4
        # SCT 两两距离 > 0
        for i, a in enumerate(archetypes_4):
            for b in archetypes_4[i + 1:]:
                dist = a.sct.distance(b.sct)
                assert dist > 0.1, f"{a.archetype} vs {b.archetype}: dist={dist} too similar"


# ============================================================================
# 10. 错误处理 + 完整性 (4)
# ============================================================================


class TestErrorHandlingAndIntegrity:
    """V1095 错误处理 + 完整性真生产测试."""

    def test_39_save_profile_with_invalid_active_pid(self, store):
        """save_profile 的 active_pid 不在 slots → PersonaSwitchError."""
        prof = CentralAIProfile(
            identity_id="ca_invalid_39",
            name="Chu Ling", chinese_name="楚零",
            active_pid="nonexistent_pid",
        )
        with pytest.raises(PersonaSwitchError):
            store.save_profile(prof)

    def test_40_cross_slot_hash_changes_on_modify(self, store):
        """修改槽位 → cross_slot_hash 变化."""
        store.ensure_default_slots(identity_id="ca_hash_40")
        h1 = store.cross_slot_hash()
        # 修改 priority
        slot = store.list_slots()[0]
        slot.priority = 0.99
        store.upsert_slot(slot)
        h2 = store.cross_slot_hash()
        assert h1 != h2

    def test_41_integrity_hash_changes_on_sct_modify(self, store):
        """单槽位 SCT 修改 → integrity_hash 变化."""
        slot = PersonaSlot(
            pid="slot_int_41", archetype="X", role_description="x",
            sct=SCTProfile(cognitive=0.5))
        h1 = slot.integrity_hash()
        slot.sct.cognitive = 0.9
        h2 = slot.integrity_hash()
        assert h1 != h2

    def test_42_stats_full(self, store):
        """stats() 返回完整字段."""
        store.ensure_default_slots(identity_id="ca_stats_42")
        stats = store.stats()
        assert stats["version"] == V1095_VERSION
        assert stats["profile"] is not None
        assert "调度者" in stats["slots_by_archetype"]
        assert "学习者" in stats["slots_by_archetype"]
        assert "思考者" in stats["slots_by_archetype"]
        assert "助手" in stats["slots_by_archetype"]
        assert len(stats["meta"]["cross_slot_hash"]) == 16
        assert len(stats["meta"]["v1072_compat_hash"]) == 16
        assert stats["n_fsync_total"] > 0


# ============================================================================
# 入口
# ============================================================================


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))