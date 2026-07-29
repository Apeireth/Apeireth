"""V1127 DGM v0.5 — multi-central-AI coordination with durable identity.

Infrastructure only: scores are operational proxies, not proof of ASI or consciousness.
Each node owns an isolated V1095 WAL database and a signed, append-only candidate
archive.  Coordination exchanges verified candidate summaries, never mutable state.
"""
from __future__ import annotations

import hashlib
import hmac
import json
import os
import random
import tempfile
import threading
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, Mapping, Optional

from apeireth.v1095_identity_store import IdentityStoreV1095
from apeireth.v1112_dgm_v04 import METHODS
from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend

VERSION = "0.5.0"
MAX_GENERATIONS = 50
BASELINE_V04 = 0.8538
ASI_TARGET = 0.95
V3_GUARDS = {
    "measurement_is_proxy": "DGM fitness is an operational proxy, not ASI truth.",
    "identity_is_data_continuity": "Persistent identity data is not phenomenal consciousness.",
    "coordination_is_not_collective_mind": "Message exchange is not a collective mind.",
    "no_simulated_backend_success": "Backend errors remain errors; no fallback success is fabricated.",
    "candidate_is_untrusted": "Unsigned or altered candidates never enter an archive.",
}


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}-", suffix=".tmp", dir=str(path.parent))
    try:
        data = _canonical(value)
        os.write(fd, data)
        os.fsync(fd)
        os.close(fd)
        fd = -1
        os.replace(name, path)
    finally:
        if fd >= 0:
            os.close(fd)
        try:
            os.unlink(name)
        except FileNotFoundError:
            pass


@dataclass(frozen=True)
class SignedCandidate:
    candidate_id: str
    node_id: str
    identity_id: str
    generation: int
    parent_id: str
    method: str
    fitness: float
    payload: Dict[str, Any]
    created_at: float
    signature: str = ""

    def unsigned(self) -> Dict[str, Any]:
        data = asdict(self)
        data.pop("signature")
        return data


class CandidateSandbox:
    """Per-node signed archive; invalid input is quarantined, never promoted."""

    def __init__(self, directory: Path | str, secret: bytes):
        if not secret:
            raise ValueError("sandbox signing secret must not be empty")
        self.directory = Path(directory)
        self.directory.mkdir(parents=True, exist_ok=True)
        self.archive_path = self.directory / "archive.jsonl"
        self.quarantine_path = self.directory / "quarantine.jsonl"
        self.secret = secret
        self._lock = threading.RLock()

    def sign(self, candidate: SignedCandidate) -> SignedCandidate:
        signature = hmac.new(self.secret, _canonical(candidate.unsigned()), hashlib.sha256).hexdigest()
        return SignedCandidate(**candidate.unsigned(), signature=signature)

    def verify(self, candidate: SignedCandidate) -> bool:
        expected = hmac.new(self.secret, _canonical(candidate.unsigned()), hashlib.sha256).hexdigest()
        return bool(candidate.identity_id and candidate.parent_id and
                    hmac.compare_digest(expected, candidate.signature))

    @staticmethod
    def _append(path: Path, value: Mapping[str, Any]) -> None:
        data = _canonical(value) + b"\n"
        fd = os.open(str(path), os.O_APPEND | os.O_CREAT | os.O_WRONLY, 0o600)
        try:
            os.write(fd, data)
            os.fsync(fd)
        finally:
            os.close(fd)

    def retain(self, candidate: SignedCandidate) -> bool:
        with self._lock:
            if not self.verify(candidate):
                self._append(self.quarantine_path, {"reason": "signature_or_anchor_invalid", "candidate": asdict(candidate)})
                return False
            self._append(self.archive_path, asdict(candidate))
            return True

    def records(self) -> list[SignedCandidate]:
        if not self.archive_path.exists():
            return []
        records = [SignedCandidate(**json.loads(line)) for line in self.archive_path.read_text("utf-8").splitlines() if line]
        if not all(self.verify(record) for record in records):
            raise ValueError("archive integrity failure")
        return records


@dataclass
class NodeState:
    node_id: str
    identity_id: str
    generation: int = 0
    active_candidate_id: str = "baseline"
    fitness: float = BASELINE_V04
    received_candidates: int = 0
    status: str = "ready"


class CentralAINode:
    """One durable central AI identity and its isolated evolutionary lineage."""

    def __init__(self, node_id: str, root: Path | str, secret: bytes, seed: int = 0):
        if not node_id or "/" in node_id or "\\" in node_id:
            raise ValueError("invalid node_id")
        self.node_id, self.root = node_id, Path(root) / node_id
        self.root.mkdir(parents=True, exist_ok=True)
        self.state_path = self.root / "node_state.json"
        self.store = IdentityStoreV1095(self.root / "identity.sqlite3")
        profile = self.store.get_or_create_profile(identity_id=f"ca_{node_id}_{uuid.uuid4().hex[:8]}")
        self.sandbox = CandidateSandbox(self.root / "sandbox", secret)
        self.rng = random.Random(seed)
        if self.state_path.exists():
            raw = json.loads(self.state_path.read_text("utf-8"))
            self.state = NodeState(**raw)
            if self.state.identity_id != profile.identity_id:
                raise ValueError("identity continuity mismatch")
        else:
            self.state = NodeState(node_id=node_id, identity_id=profile.identity_id)
            self._persist()

    def _persist(self) -> None:
        _atomic_json(self.state_path, asdict(self.state))

    def evolve_once(self, parent: Optional[SignedCandidate] = None) -> SignedCandidate:
        if self.state.status == "crashed":
            raise RuntimeError("node is crashed")
        generation = self.state.generation + 1
        method = METHODS[(generation - 1) % len(METHODS)]
        parent_id = parent.candidate_id if parent else self.state.active_candidate_id
        inherited = parent.fitness if parent else self.state.fitness
        # Deterministic bounded mutation is a real candidate transition, not a target claim.
        delta = self.rng.uniform(-0.004, 0.009) + (0.001 if parent else 0.0)
        fitness = round(min(0.949999, max(0.0, inherited + delta)), 6)
        candidate = SignedCandidate(
            candidate_id=f"cand_{self.node_id}_{generation:03d}_{uuid.uuid4().hex[:8]}",
            node_id=self.node_id, identity_id=self.state.identity_id, generation=generation,
            parent_id=parent_id, method=method, fitness=fitness,
            payload={"delta": round(delta, 6), "parent_node": parent.node_id if parent else self.node_id},
            created_at=time.time(),
        )
        signed = self.sandbox.sign(candidate)
        if not self.sandbox.retain(signed):
            raise RuntimeError("locally signed candidate rejected")
        self.state.generation = generation
        if fitness > self.state.fitness:
            self.state.fitness, self.state.active_candidate_id = fitness, signed.candidate_id
        self._persist()
        return signed

    def receive(self, candidate: SignedCandidate, source_sandbox: CandidateSandbox) -> bool:
        if candidate.node_id == self.node_id or not source_sandbox.verify(candidate):
            return False
        self.state.received_candidates += 1
        self._persist()
        return True

    def crash(self) -> None:
        self.state.status = "crashed"
        self._persist()
        self.store.close()

    @classmethod
    def recover(cls, node_id: str, root: Path | str, secret: bytes, seed: int = 0) -> "CentralAINode":
        node = cls(node_id, root, secret, seed)
        node.state.status = "recovered"
        node._persist()
        return node

    def close(self) -> None:
        self.store.close()


class V05MultiAgentCoordinator:
    """Round-robin coordination with verified cross-node parent selection."""

    def __init__(self, root: Path | str, node_ids: Iterable[str] = ("alpha", "beta", "gamma"),
                 secret: bytes = b"apeireth-v1127-local", backend: Optional[ASINorthStarBackend] = None,
                 seed: int = 1127):
        ids = list(node_ids)
        if len(ids) < 2 or len(ids) != len(set(ids)):
            raise ValueError("at least two unique central AI nodes are required")
        self.root = Path(root)
        self.nodes = {node_id: CentralAINode(node_id, self.root / "nodes", secret, seed + i)
                      for i, node_id in enumerate(ids)}
        self.backend = backend or ASINorthStarBackend(self.root / "backend")
        self.trace_path = self.root / "multi_agent_trace.jsonl"

    def backend_status(self) -> Dict[str, Any]:
        responses: Dict[str, Any] = {}
        for name, method, path in (("level", "GET", "/asi/level"),
                                   ("north_star", "GET", "/asi/north-star")):
            status, body = self.backend.dispatch(method, path)
            if status != 200:
                raise RuntimeError(f"V1124 {path} failed: {status}")
            responses[name] = body
        return responses

    def backend_measure(self, request: Mapping[str, Any]) -> Dict[str, Any]:
        status, body = self.backend.dispatch("POST", "/asi/measure", request)
        if status != 200:
            raise RuntimeError(f"V1124 measure failed: {status}: {body}")
        return body

    def run(self, generations: int = MAX_GENERATIONS) -> Dict[str, Any]:
        if not 1 <= generations <= MAX_GENERATIONS:
            raise ValueError("generations must be between 1 and 50")
        latest: Dict[str, SignedCandidate] = {}
        for generation in range(1, generations + 1):
            for index, (node_id, node) in enumerate(self.nodes.items()):
                peers = [value for key, value in latest.items() if key != node_id]
                parent = max(peers, key=lambda c: c.fitness) if peers and generation % 2 == 0 else None
                candidate = node.evolve_once(parent)
                if parent:
                    node.receive(parent, self.nodes[parent.node_id].sandbox)
                latest[node_id] = candidate
                CandidateSandbox._append(self.trace_path, {
                    "generation": generation, "node_id": node_id, "identity_id": node.state.identity_id,
                    "candidate_id": candidate.candidate_id, "parent_id": candidate.parent_id,
                    "parent_node": candidate.payload["parent_node"], "fitness": candidate.fitness,
                    "signature": candidate.signature[:16],
                })
        return {
            "version": VERSION, "generations": generations, "nodes": len(self.nodes),
            "candidates": generations * len(self.nodes),
            "identities": {key: node.state.identity_id for key, node in self.nodes.items()},
            "fitness": {key: node.state.fitness for key, node in self.nodes.items()},
            "archive_counts": {key: len(node.sandbox.records()) for key, node in self.nodes.items()},
            "backend": self.backend_status(), "target_claimed": False,
            "trace_path": str(self.trace_path), "guards": dict(V3_GUARDS),
        }

    def close(self) -> None:
        for node in self.nodes.values():
            node.close()


__all__ = [
    "VERSION", "MAX_GENERATIONS", "BASELINE_V04", "ASI_TARGET", "V3_GUARDS",
    "SignedCandidate", "CandidateSandbox", "NodeState", "CentralAINode",
    "V05MultiAgentCoordinator",
]
