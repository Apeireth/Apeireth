"""V1127 DGM v0.5 multi-central-AI integration tests."""
from __future__ import annotations

import json
import sys
from dataclasses import replace
from pathlib import Path

import pytest

from apeireth.v1127_dgm_v05_multi_agent import (
    ASI_TARGET, BASELINE_V04, MAX_GENERATIONS, VERSION, V3_GUARDS,
    CandidateSandbox, CentralAINode, SignedCandidate, V05MultiAgentCoordinator,
)

SECRET = b"test-secret"


def candidate(node="a", identity="ca_a", generation=1):
    return SignedCandidate("cand_1", node, identity, generation, "baseline", "parent_child",
                           0.86, {"delta": 0.006}, 1.0)


def test_01_version(): assert VERSION == "0.5.0"
def test_02_fifty_round_constant(): assert MAX_GENERATIONS == 50
def test_03_baseline(): assert BASELINE_V04 == 0.8538
def test_04_target(): assert ASI_TARGET == 0.95
def test_05_guards(): assert len(V3_GUARDS) >= 5 and "candidate_is_untrusted" in V3_GUARDS


def test_06_sandbox_requires_secret(tmp_path):
    with pytest.raises(ValueError): CandidateSandbox(tmp_path, b"")


def test_07_signature_roundtrip(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); signed = box.sign(candidate())
    assert box.verify(signed)


def test_08_signature_detects_payload_change(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); signed = box.sign(candidate())
    assert not box.verify(replace(signed, fitness=0.99))


def test_09_wrong_secret_rejects(tmp_path):
    signed = CandidateSandbox(tmp_path / "a", SECRET).sign(candidate())
    assert not CandidateSandbox(tmp_path / "b", b"other").verify(signed)


def test_10_unanchored_rejected_and_quarantined(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); signed = box.sign(candidate(identity=""))
    assert not box.retain(signed) and box.quarantine_path.exists()


def test_11_retained_archive_is_jsonl(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); assert box.retain(box.sign(candidate()))
    assert box.archive_path.read_bytes().endswith(b"\n")


def test_12_archive_records_verified(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); signed = box.sign(candidate()); box.retain(signed)
    assert box.records() == [signed]


def test_13_archive_tampering_detected(tmp_path):
    box = CandidateSandbox(tmp_path, SECRET); box.retain(box.sign(candidate()))
    box.archive_path.write_text(box.archive_path.read_text().replace("0.86", "0.99"))
    with pytest.raises(ValueError): box.records()


def test_14_node_rejects_bad_id(tmp_path):
    with pytest.raises(ValueError): CentralAINode("../bad", tmp_path, SECRET)


def test_15_node_creates_durable_identity(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET); identity = node.state.identity_id; node.close()
    recovered = CentralAINode("a", tmp_path, SECRET)
    try: assert recovered.state.identity_id == identity
    finally: recovered.close()


def test_16_nodes_have_distinct_identity_and_db(tmp_path):
    a, b = CentralAINode("a", tmp_path, SECRET), CentralAINode("b", tmp_path, SECRET)
    try:
        assert a.state.identity_id != b.state.identity_id
        assert (tmp_path / "a" / "identity.sqlite3").exists()
        assert (tmp_path / "b" / "identity.sqlite3").exists()
    finally: a.close(); b.close()


def test_17_evolution_is_real_transition(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET, 1)
    try:
        item = node.evolve_once(); assert item.parent_id == "baseline" and node.state.generation == 1
    finally: node.close()


def test_18_methods_rotate(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET, 1)
    try: assert [node.evolve_once().method for _ in range(3)] == ["parent_child", "sexual", "asexual"]
    finally: node.close()


def test_19_fitness_never_claims_target(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET, 1)
    try:
        for _ in range(50): node.evolve_once()
        assert node.state.fitness < ASI_TARGET
    finally: node.close()


def test_20_crashed_node_cannot_evolve(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET); node.crash()
    with pytest.raises(RuntimeError): node.evolve_once()


def test_21_crash_recovery_keeps_identity(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET); identity = node.state.identity_id
    node.evolve_once(); node.crash(); recovered = CentralAINode.recover("a", tmp_path, SECRET)
    try: assert recovered.state.identity_id == identity and recovered.state.status == "recovered"
    finally: recovered.close()


def test_22_crash_recovery_keeps_generation(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET); node.evolve_once(); node.crash()
    recovered = CentralAINode.recover("a", tmp_path, SECRET)
    try: assert recovered.state.generation == 1
    finally: recovered.close()


def test_23_receive_rejects_self(tmp_path):
    node = CentralAINode("a", tmp_path, SECRET)
    try:
        item = node.evolve_once(); assert not node.receive(item, node.sandbox)
    finally: node.close()


def test_24_receive_rejects_bad_signature(tmp_path):
    a, b = CentralAINode("a", tmp_path, SECRET), CentralAINode("b", tmp_path, SECRET)
    try:
        item = replace(a.evolve_once(), fitness=0.99)
        assert not b.receive(item, a.sandbox) and b.state.received_candidates == 0
    finally: a.close(); b.close()


def test_25_receive_valid_peer_without_archive_pollution(tmp_path):
    a, b = CentralAINode("a", tmp_path, SECRET), CentralAINode("b", tmp_path, SECRET)
    try:
        item = a.evolve_once(); assert b.receive(item, a.sandbox)
        assert len(b.sandbox.records()) == 0
    finally: a.close(); b.close()


def test_26_coordinator_requires_two_unique_nodes(tmp_path):
    with pytest.raises(ValueError): V05MultiAgentCoordinator(tmp_path, ["a"])
    with pytest.raises(ValueError): V05MultiAgentCoordinator(tmp_path / "x", ["a", "a"])


def test_27_coordinator_generations_validation(tmp_path):
    coord = V05MultiAgentCoordinator(tmp_path, ["a", "b"])
    try:
        with pytest.raises(ValueError): coord.run(51)
    finally: coord.close()


def test_28_multi_agent_cross_parent_and_trace(tmp_path):
    coord = V05MultiAgentCoordinator(tmp_path, ["a", "b"], SECRET)
    try:
        result = coord.run(2); lines = [json.loads(x) for x in Path(result["trace_path"]).read_text().splitlines()]
        assert result["candidates"] == 4 and any(x["parent_node"] != x["node_id"] for x in lines)
    finally: coord.close()


def test_29_real_fifty_round_evolution_isolated(tmp_path):
    coord = V05MultiAgentCoordinator(tmp_path, ["a", "b", "c"], SECRET)
    try:
        result = coord.run(50)
        assert result["generations"] == 50 and result["candidates"] == 150
        assert set(result["archive_counts"].values()) == {50}
        assert len(set(result["identities"].values())) == 3
        assert result["target_claimed"] is False
    finally: coord.close()


def test_30_asi_level_cross_session_continuity(tmp_path):
    first = V05MultiAgentCoordinator(tmp_path, ["a", "b"], SECRET)
    identities = first.run(2)["identities"]; first.close()
    second = V05MultiAgentCoordinator(tmp_path, ["a", "b"], SECRET)
    try:
        result = second.run(1)
        assert result["identities"] == identities
        assert result["backend"]["level"]["score"] == BASELINE_V04
    finally: second.close()


def test_31_backend_north_star_real_dispatch(tmp_path):
    coord = V05MultiAgentCoordinator(tmp_path, ["a", "b"], SECRET)
    try: assert coord.backend_status()["north_star"]["current"]["target"] == ASI_TARGET
    finally: coord.close()


def test_32_backend_measure_real_process(tmp_path):
    coord = V05MultiAgentCoordinator(tmp_path, ["a", "b"], SECRET)
    try:
        result = coord.backend_measure({"provider": "local", "model": "test-process", "prompt": "hello",
                                        "command": [sys.executable, "-c", "import sys; print(sys.stdin.read())"]})
        assert result["evidence"]["real"] and result["evidence"]["transport"] == "process"
    finally: coord.close()
