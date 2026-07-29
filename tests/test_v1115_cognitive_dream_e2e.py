"""Tests for V1115 Cognitive-Dream Orchestrator E2E (R9-FE-002)

覆盖:
  - V1115IdentityLockChain 锁链贯通 (5 tests)
  - V1072ManifestBridge 5 Module 桥接 (5 tests)
  - DreamInferenceDualSign 双签 (5 tests)
  - V1115E2EOrchestrator.run_e2e 端到端 (6 tests)
  - 守门 / 集成证据 (4 tests)
  共 ≥ 25 真测试
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, List

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth import v1107_cognitive_core_lift as v1107  # noqa: E402
from apeireth import v1108_dream_v2 as v1108  # noqa: E402
from apeireth import v1060_asi_orchestrator as v1060  # noqa: E402
from apeireth import v1072_asi_central_ai_eternal_identity as v1072  # noqa: E402
from apeireth import v1084_asi_real_llm_inference as v1084  # noqa: E402
from apeireth import v1115_cognitive_dream_orchestrator_e2e as v1115  # noqa: E402


# ============================================================================
# Fixtures
# ============================================================================


@pytest.fixture
def tmp_audit_path(tmp_path) -> Path:
    """提供临时 audit log 路径."""
    return tmp_path / "test_inference_audit.jsonl"


@pytest.fixture
def v1107_lift():
    """一个干净的 V1107 lift 实例 (不 seed, 让测试自己决定)."""
    return v1107.V1107CognitiveLift()


@pytest.fixture
def v1107_lift_seeded():
    """一个 V1107 lift + 已 seed_5_module_framework 实例."""
    lift = v1107.V1107CognitiveLift()
    lift.seed_5_module_framework()
    return lift


@pytest.fixture
def v1072_orchestrator():
    """V1072 orchestrator 实例."""
    return v1072.V1072Orchestrator()


@pytest.fixture
def v1108_dream_obj():
    """V1108 MemoryDreamV2 实例."""
    return v1108.MemoryDreamV2(seed=42)


def _make_dream_candidates(dream_obj, notes):
    """跑 V1108 dream 拿 candidates."""
    result = dream_obj.dream(notes, context={"topic": "test"})
    return list(result.candidates)


def _make_test_notes(n: int = 5) -> List[Any]:
    """造 V1092-like 笔记."""
    notes: List[Any] = []
    for i in range(n):
        notes.append(type("_N", (), {
            "nid": f"n_{i}",
            "topic": f"topic_{i % 3}",
            "claim": f"claim_{i}_about_cognitive_lift",
            "confidence": 0.6 + 0.05 * i,
            "salience": 0.5 + 0.05 * i,
        })())
    return notes


# ============================================================================
# Section 1: V1115IdentityLockChain — IDENTITY-V1 5 Module ↔ V1072 锁链
# ============================================================================


class TestV1115IdentityLockChain:
    """锁链贯通真生产 (主 12:14 中央 AI 是永恒身份)."""

    def test_01_hash_identity_id_deterministic(self):
        """id hash 确定性 (主 17:43 实事求是)."""
        a = v1115.V1115IdentityLockChain._hash_identity_id("ident_test_001")
        b = v1115.V1115IdentityLockChain._hash_identity_id("ident_test_001")
        c = v1115.V1115IdentityLockChain._hash_identity_id("ident_test_002")
        assert a == b
        assert a != c
        assert a.startswith("idlock-")
        assert len(a) == len("idlock-") + 16  # sha256[:16]

    def test_02_lock_default_id_success(self, v1107_lift, v1072_orchestrator):
        """默认 lock 用 V1107.identity.identity_id."""
        lc = v1115.V1115IdentityLockChain(
            v1107_identity_core=v1107_lift.identity,
            v1072_core=v1072_orchestrator.core,
        )
        report = lc.lock()
        assert "identity_id" in report
        assert "identity_id_hash" in report
        assert report["identity_id"] == v1107_lift.identity.identity_id
        assert v1072_orchestrator.core.identity_id == v1107_lift.identity.identity_id
        assert len(lc.records) >= 1
        assert lc.status == v1115.LockChainStatus.LOCKED

    def test_03_lock_explicit_id_overrides(self, v1107_lift, v1072_orchestrator):
        """显式 lock_id override."""
        lc = v1115.V1115IdentityLockChain(
            v1107_identity_core=v1107_lift.identity,
            v1072_core=v1072_orchestrator.core,
        )
        custom_id = "ident_explicit_override"
        report = lc.lock(identity_id=custom_id)
        assert report["identity_id"] == custom_id
        assert v1107_lift.identity.identity_id == custom_id
        assert v1072_orchestrator.core.identity_id == custom_id

    def test_04_lock_philosophy_anchors_synced(self, v1107_lift, v1072_orchestrator):
        """philosophy_keys ↔ philosophy_anchors 同步."""
        lc = v1115.V1115IdentityLockChain(
            v1107_identity_core=v1107_lift.identity,
            v1072_core=v1072_orchestrator.core,
        )
        lc.lock()
        # 至少包含 V1107 全部 philosophy_keys
        v1107_keys = set(v1107_lift.identity.philosophy_keys)
        v1072_anchors = set(v1072_orchestrator.core.philosophy_anchors)
        assert v1107_keys.issubset(v1072_anchors), \
            f"philosophy_anchors not synced: {v1107_keys - v1072_anchors}"

    def test_05_validate_chain_all_ok(self, v1107_lift, v1072_orchestrator):
        """锁链全链路验证 (主 17:43 实事求是)."""
        lc = v1115.V1115IdentityLockChain(
            v1107_identity_core=v1107_lift.identity,
            v1072_core=v1072_orchestrator.core,
        )
        lc.lock()
        v = lc.validate_chain()
        assert v["identity_ids_match"] is True
        assert v["records_match"] is True
        assert v["philosophy_synced"] is True
        assert v["all_ok"] is True
        assert v["status"] == v1115.LockChainStatus.VERIFIED.value


# ============================================================================
# Section 2: V1072ManifestBridge — V1107 5 Module → V1072 IdentityManifest
# ============================================================================


class TestV1072ManifestBridge:
    """5 Module 桥接真生产 (主 19:33 Tulving 1985 + Damasio 1999)."""

    def test_06_bridge_episodes_to_stm(self, v1107_lift, v1072_orchestrator):
        """M2 EpisodeBuffer → STM/event."""
        # seed 一些 episodes
        for i in range(3):
            v1107_lift.episode_buffer.push(v1107.Episode(
                episode_id=f"ep_{i}",
                content={"claim": f"test_episode_{i}"},
                salience=0.5, confidence=0.6, source="real",
            ))
        bridge = v1115.V1072ManifestBridge()
        out = bridge.bridge_episodes(
            v1107_lift.episode_buffer, v1072_orchestrator.manifest,
        )
        assert len(out) == 3
        # 全是 STM/event
        for eid in out:
            e = next(e for e in v1072_orchestrator.manifest.entries if e.entry_id == eid)
            assert e.source == "STM"
            assert e.kind == "event"
        assert bridge.bridged_count == 3

    def test_07_bridge_dream_episode_marked(self, v1107_lift, v1072_orchestrator):
        """dream episode 必须打 _dream 标记 (主 17:58+20:46 不假装)."""
        v1107_lift.episode_buffer.push(v1107.Episode(
            episode_id="dream_ep_1",
            content={"claim": "dream_test", "_dream": True, "source_dream_cid": "dream-abc"},
            salience=0.4, confidence=0.5, source="dream",
        ))
        bridge = v1115.V1072ManifestBridge()
        bridge.bridge_episodes(
            v1107_lift.episode_buffer, v1072_orchestrator.manifest,
        )
        assert bridge.dream_episodes_count == 1
        # 验证 V1072 manifest 里这条 entry 也有 _dream tag
        stm_entries = v1072_orchestrator.manifest.get_by_source("STM")
        assert any("_dream" in e.tags for e in stm_entries)

    def test_08_bridge_notes_to_mtm(self, v1107_lift, v1072_orchestrator):
        """M3 NoteConsolidator → MTM/topic."""
        v1107_lift.note_consolidator.upsert_note(
            topic="lift", claim="add_chunks",
            confidence=0.7, salience=0.6,
        )
        v1107_lift.note_consolidator.upsert_note(
            topic="lift", claim="fix_seeder",
            confidence=0.8, salience=0.7,
        )
        bridge = v1115.V1072ManifestBridge()
        out = bridge.bridge_notes(
            v1107_lift.note_consolidator, v1072_orchestrator.manifest,
        )
        assert len(out) == 2
        # V1072.run() 已经加了 4 个 MTM, bridge 又加 2 个 = 6 个
        mtm_entries = v1072_orchestrator.manifest.get_by_source("MTM")
        v1107_m3_entries = [e for e in mtm_entries if "v1107_M3" in e.tags]
        assert len(v1107_m3_entries) == 2
        for e in v1107_m3_entries:
            assert e.kind == "topic"
            assert "v1107_M3" in e.tags

    def test_09_bridge_relations_to_ltm(self, v1107_lift, v1072_orchestrator):
        """M4 RelationGraph → LTM/relation."""
        v1107_lift.relation_graph.add_node("a", kind="entity")
        v1107_lift.relation_graph.add_node("b", kind="entity")
        v1107_lift.relation_graph.add_edge("a", "b", weight=0.8, relation="causal")
        bridge = v1115.V1072ManifestBridge()
        out = bridge.bridge_relations(
            v1107_lift.relation_graph, v1072_orchestrator.manifest,
        )
        assert len(out) == 1
        ltm_entries = v1072_orchestrator.manifest.get_by_source("LTM")
        rel_entries = [e for e in ltm_entries if e.kind == "relation"]
        # V1072.run() 加 5 LTM fact, bridge 加 1 LTM relation = 6
        v1107_m4 = [e for e in rel_entries if "v1107_M4" in e.tags]
        assert len(v1107_m4) == 1
        assert "causal" in v1107_m4[0].tags

    def test_10_bridge_all_5_modules(self, v1107_lift, v1072_orchestrator):
        """5 Module 一次性全桥接."""
        # seed 5 Module (已有 fixture 提供 episode_buffer/notes/graph 状态)
        # 加一些 content
        for i in range(2):
            v1107_lift.episode_buffer.push(v1107.Episode(
                episode_id=f"ep_{i}", content={"claim": f"c_{i}"},
                salience=0.5, confidence=0.6, source="real",
            ))
        v1107_lift.note_consolidator.upsert_note(
            topic="t", claim="c", confidence=0.6, salience=0.5,
        )
        v1107_lift.relation_graph.add_node("x", kind="entity")
        v1107_lift.relation_graph.add_node("y", kind="entity")
        v1107_lift.relation_graph.add_edge("x", "y", weight=0.7, relation="part_of")
        # 触发 reconsolidation
        v1107_lift.reconsolidation.conflicts_detected = 2
        v1107_lift.reconsolidation.abstractions_formed = 1
        v1107_lift.reconsolidation.forgetting_events = 1

        bridge = v1115.V1072ManifestBridge()
        result = bridge.bridge_all_5_modules(
            v1107_lift, v1072_orchestrator.manifest,
        )
        # M2 episodes from buffer, M3 notes from notes, M4 relations, M5 insights
        assert result["M2_episodes"] == 2
        assert result["M3_notes"] == 1
        assert result["M4_relations"] == 1
        assert result["M5_insights"] == 1
        assert result["total"] == 5


# ============================================================================
# Section 3: DreamInferenceDualSign — V1108 dream audit + V1084 双签
# ============================================================================


class TestDreamInferenceDualSign:
    """V1108 dream audit_trail + V1084 InferenceAuditLog 双签 (主 19:33 W3C PROV-DM)."""

    def test_11_co_sign_dream_candidate_writes_jsonl(self, v1107_lift,
                                                      v1108_dream_obj,
                                                      tmp_audit_path):
        """双签 V1108 dream candidate → V1084 JSONL 真签."""
        notes = _make_test_notes(4)
        candidates = _make_dream_candidates(v1108_dream_obj, notes)
        assert len(candidates) > 0
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        dual = v1115.DreamInferenceDualSign(audit_log=audit)
        # 双签第一个 candidate
        rid = dual.co_sign_dream_candidate(candidates[0])
        assert rid.startswith("req_")
        # JSONL 真有 1 行
        assert tmp_audit_path.exists()
        lines = tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["model_id"] == "v1108_dream_v2"
        assert rec["status"] == "dream"
        assert "request_hash" in rec and "response_hash" in rec
        assert rec["prompt_preview"].startswith("V1108 dream candidate dual-sign")

    def test_12_co_sign_idempotent(self, v1107_lift, v1108_dream_obj,
                                    tmp_audit_path):
        """双签 idempotent (主 17:43 实事求是: 重复 cid 不重复签)."""
        notes = _make_test_notes(4)
        candidates = _make_dream_candidates(v1108_dream_obj, notes)
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        dual = v1115.DreamInferenceDualSign(audit_log=audit)
        rid1 = dual.co_sign_dream_candidate(candidates[0])
        rid2 = dual.co_sign_dream_candidate(candidates[0])
        assert rid1 != "" and rid2 == ""  # 第二次 idempotent, 返回 ""
        # 只有 1 条 JSONL
        lines = tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 1

    def test_13_co_sign_all_candidates(self, v1107_lift, v1108_dream_obj,
                                        tmp_audit_path):
        """双签所有 candidates."""
        notes = _make_test_notes(4)
        candidates = _make_dream_candidates(v1108_dream_obj, notes)
        n_cands = len(candidates)
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        dual = v1115.DreamInferenceDualSign(audit_log=audit)
        for c in candidates:
            dual.co_sign_dream_candidate(c)
        assert dual.n_dream_events_signed == n_cands
        # JSONL 行数 = n_cands
        lines = tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == n_cands

    def test_14_co_sign_cognitive_event(self, tmp_audit_path):
        """V1107 cognitive event 也走 V1084 双签."""
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        dual = v1115.DreamInferenceDualSign(audit_log=audit)
        rid = dual.co_sign_cognitive_event({"event": "test_event", "x": 1})
        assert rid.startswith("req_")
        # JSONL 真有 1 行
        rec = json.loads(tmp_audit_path.read_text(encoding="utf-8").strip())
        assert rec["model_id"] == "v1107_cognitive_lift"
        assert rec["status"] == "cognitive_event"

    def test_15_dual_sign_dream_audit_trail_in_jsonl(self, v1108_dream_obj,
                                                     tmp_audit_path):
        """dream audit_trail 元数据在 V1084 JSONL 留痕 (主 19:33 W3C PROV-DM).

        V1084 record 只存 prompt[:80] 作为 prompt_preview, 所以只验证前 80 字符.
        """
        notes = _make_test_notes(3)
        candidates = _make_dream_candidates(v1108_dream_obj, notes)
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        dual = v1115.DreamInferenceDualSign(audit_log=audit)
        for c in candidates:
            dual.co_sign_dream_candidate(c)
        # 每条 JSONL prompt_preview 必须包含 cid (在 prompt 前 80 字符内)
        lines = tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()
        for line, c in zip(lines, candidates):
            rec = json.loads(line)
            # prompt_preview 是 V1084 截断的 80 字符, 验证 cid 在前部
            assert c.cid in rec["prompt_preview"], \
                f"cid {c.cid} not in prompt_preview"
            # request_hash 是 sha256(prompt), 所以 V1084 真有完整 prompt 的 hash
            assert len(rec["request_hash"]) == 64  # sha256 hex
            # model_id 是 v1108_dream_v2
            assert rec["model_id"] == "v1108_dream_v2"


# ============================================================================
# Section 4: V1115E2EOrchestrator.run_e2e — 端到端真集成
# ============================================================================


class TestV1115E2EOrchestrator:
    """V1115 端到端真集成 (主 23:44 干到底)."""

    def test_16_run_e2e_full_pipeline(self, tmp_audit_path):
        """完整端到端 pipeline (skip V1074/V1077 加速)."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_test_chu_ling",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        # 7 个核心字段都存在
        assert report.identity_lock_chain
        assert report.cognitive_lift
        assert report.dream_cycle
        assert report.v1072_bridge
        assert report.dual_sign
        assert report.v1060_health
        # lock chain all_ok
        assert report.identity_lock_chain.get("validation", {}).get("all_ok") is True
        # 5 Module 桥接 > 0
        assert report.v1072_bridge.get("bridge", {}).get("total", 0) > 0
        # 双签 audit records > 0
        assert report.dual_sign.get("n_audit_records", 0) > 0

    def test_17_run_e2e_dream_candidates_audit(self, tmp_audit_path):
        """end-to-end: V1108 dream candidates 双签到 V1084 JSONL."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_dream_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        n_cands = report.dream_cycle.get("n_candidates", 0)
        n_signed = report.dual_sign.get("n_dream_events_signed", 0)
        assert n_cands == n_signed, \
            f"dream candidates ({n_cands}) != co-signed ({n_signed})"
        # V1108 所有 candidate 必须 _dream=True
        assert report.dream_cycle.get("all_dream", False) is True

    def test_18_run_e2e_lock_chain_audit(self, tmp_audit_path):
        """end-to-end: lock chain co-signed + audit 留痕."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_lockchain_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        lock = report.identity_lock_chain.get("lock", {})
        assert lock.get("co_signed") is True
        assert lock.get("audit_log_id", "").startswith("req_")
        # 锁链 records 中至少 1 条 co_signed
        n_co = report.identity_lock_chain.get("validation", {}).get("n_co_signed", 0)
        assert n_co >= 1

    def test_19_run_e2e_5_module_bridge_manifest(self, tmp_audit_path):
        """end-to-end: 5 Module 桥接 → V1072 manifest LTM/MTM/STM 都加."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_5module_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        bridge = report.v1072_bridge.get("bridge", {})
        # 5 Module 都有 (>= 1)
        assert bridge.get("M2_episodes", 0) >= 1, "M2 episode 桥接空"
        assert bridge.get("M3_notes", 0) >= 1, "M3 note 桥接空"
        assert bridge.get("M4_relations", 0) >= 1, "M4 relation 桥接空"
        # M5 取决于 reconsolidation 是否触发
        # V1072 core n_ltm > 0
        core = report.v1072_bridge.get("core", {})
        assert core.get("n_ltm", 0) > 0

    def test_20_run_e2e_v1060_health_check(self, tmp_audit_path):
        """end-to-end: V1060 orchestrator 健康检查 V1107/V1108."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_v1060_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        vh = report.v1060_health
        assert vh.get("v1107") is not None
        assert vh.get("v1108") is not None
        # V1107/V1108 都有测试文件 (R9-FE-001 留下的)
        assert vh["v1107"].get("has_test_file") is True
        assert vh["v1108"].get("has_test_file") is True

    def test_21_run_e2e_trace_recorded(self, tmp_audit_path):
        """end-to-end: trace 全部事件都被记录 (主 00:56 任何人能接手)."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_e2e_trace_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        trace = report.e2e_trace
        events = [t.get("event") for t in trace]
        # 7 个核心事件都必须有
        for ev in [
            "e2e_start", "v1107_cognitive_lift", "v1108_dream_cycle",
            "v1107_integrate_dream", "v1072_bridge", "v1084_dual_sign",
            "v1060_health_check", "e2e_end",
        ]:
            assert ev in events, f"trace event missing: {ev}"
        assert trace[0]["event"] == "e2e_start"
        assert trace[-1]["event"] == "e2e_end"


# ============================================================================
# Section 5: 守门 / 集成证据
# ============================================================================


class TestV1115Guards:
    """V3 守门 + 集成证据真生产 (主 17:43+17:58+20:46)."""

    def test_22_v3_guards_present(self):
        """V3 哲学守门必须 ≥ 5 条 (主 17:58 不假装)."""
        assert len(v1115.V1115_V3_GUARDS) >= 5
        for k in [
            "integration_asi", "audit_truth", "lock_chain_identity",
            "orchestrator_production", "score_asi",
        ]:
            assert k in v1115.V1115_V3_GUARDS, f"missing guard: {k}"

    def test_23_orchestrator_report_has_all_sections(self, tmp_audit_path):
        """报告有 8 章节 (主 00:56 任何人能接手)."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_report_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        md = v1115.to_markdown(report)
        for sec in [
            "# V1115 E2E Report",
            "## 1. IDENTITY-V1 ↔ V1072 锁链",
            "## 2. V1107 Cognitive Lift",
            "## 3. V1108 Dream V2",
            "## 4. V1107 5 Module → V1072 IdentityManifest",
            "## 5. V1084 InferenceAuditLog 双签",
            "## 6. V1060 Orchestrator Health",
            "## 7. V3 哲学守门",
            "## 8. E2E Trace",
        ]:
            assert sec in md, f"missing section: {sec}"

    def test_24_run_e2e_produces_well_typed_report(self, tmp_audit_path):
        """report dataclass 字段类型正确."""
        report = v1115.run_v1115_e2e(
            identity_id="ident_types_test",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        assert isinstance(report.identity_lock_chain, dict)
        assert isinstance(report.cognitive_lift, dict)
        assert isinstance(report.dream_cycle, dict)
        assert isinstance(report.v1072_bridge, dict)
        assert isinstance(report.dual_sign, dict)
        assert isinstance(report.v1060_health, dict)
        assert isinstance(report.e2e_trace, list)
        assert isinstance(report.philosophy_guard, dict)
        assert report.timestamp  # 非空
        assert report.duration_ms > 0
        assert report.version == v1115.V1115_VERSION

    def test_25_dual_sign_audit_chain_loadable(self, tmp_audit_path):
        """V1084 audit chain JSONL 真能 load_all (主 17:43 实事求是)."""
        v1115.run_v1115_e2e(
            identity_id="ident_audit_loadable",
            audit_log_path=str(tmp_audit_path),
            run_v1074=False,
            run_v1077=False,
        )
        # 真 load 一下
        audit = v1084.InferenceAuditLog(log_path=tmp_audit_path)
        records = audit.load_all()
        assert len(records) > 0
        # 每条都是 dict
        for r in records:
            assert isinstance(r, dict)
            assert "request_id" in r
            assert "request_hash" in r
            assert "response_hash" in r
            assert "model_id" in r
        # 模型分布: V1107 cognitive + V1108 dream + V1115 lock
        models = {r["model_id"] for r in records}
        assert any("v1107" in m for m in models), \
            f"no v1107 records: {models}"
        assert any("v1108" in m for m in models), \
            f"no v1108 records: {models}"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))
