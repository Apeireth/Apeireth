"""V1124 backend tests: real files, sockets, grpc HTTP/2, and subprocesses."""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import grpc
import pytest

from apeireth.v1072_asi_central_ai_eternal_identity import IdentityManifest
from apeireth.v1124_asi_north_star_backend import (
    ASI_NORTH_STAR_TARGET,
    BASELINE_V04,
    V1124_VERSION,
    ASINorthStarBackend,
    AuditChain,
    DurableIdentityStore,
    IntegrityError,
    ModelRequest,
    RealModelGateway,
    V1124Error,
    V3_GUARDS,
    start_grpc_server,
    start_http_server,
)


def local_command() -> list[str]:
    return [sys.executable, "-c", "import sys; print('local-engine:'+sys.stdin.read())"]


@pytest.fixture
def backend(tmp_path: Path) -> ASINorthStarBackend:
    return ASINorthStarBackend(tmp_path / "state")


class ProviderHandler(BaseHTTPRequestHandler):
    response_kind = "openai"

    def do_POST(self):  # noqa: N802
        length = int(self.headers["Content-Length"])
        request = json.loads(self.rfile.read(length))
        if self.path == "/api/chat":
            payload = {"model": request["model"], "message": {"content": "ollama-real-http"}}
        elif self.path == "/messages":
            payload = {"model": request["model"], "content": [{"type": "text", "text": "claude-real-http"}],
                       "usage": {"input_tokens": 3, "output_tokens": 2}}
        else:
            payload = {"model": request["model"], "choices": [{"message": {"content": "gpt-real-http"}}],
                       "usage": {"prompt_tokens": 4, "completion_tokens": 2}}
        raw = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def log_message(self, *args):
        pass


@pytest.fixture
def provider_server():
    server = ThreadingHTTPServer(("127.0.0.1", 0), ProviderHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{server.server_port}"
    server.shutdown()
    server.server_close()
    thread.join(timeout=2)


# Basics and guards (8)
def test_version_and_thresholds():
    assert V1124_VERSION == "0.1.0"
    assert BASELINE_V04 == 0.8538
    assert ASI_NORTH_STAR_TARGET == 0.95


def test_guards_are_explicit():
    assert len(V3_GUARDS) >= 5
    assert "not_phenomenal_consciousness" in V3_GUARDS


def test_error_payload_is_stable():
    assert V1124Error("bad", "detail", 400).payload() == {"error": {"code": "bad", "message": "detail"}}


def test_level_does_not_claim_target(backend):
    result = backend.level()
    assert result["score"] == BASELINE_V04
    assert result["target_reached"] is False


def test_level_marks_proxy(backend):
    assert backend.level()["claim"] == "operational_proxy_not_asi_truth"


def test_north_star_lists_both_protocols(backend):
    assert set(backend.north_star()["protocols"]) == {"http", "grpc"}


def test_north_star_inherits_identity_guards(backend):
    assert backend.north_star()["identity_guards"]["not_eternal_as_phenomenal"]


def test_dispatch_unknown_is_404(backend):
    status, body = backend.dispatch("GET", "/missing")
    assert status == 404 and body["error"]["code"] == "not_found"


# Audit and fsync durability (12)
def test_audit_empty_chain(tmp_path):
    assert AuditChain(tmp_path / "a.jsonl").verify()["records"] == 0


def test_audit_append_is_jsonl(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("one", {"x": 1})
    assert chain.path.read_bytes().endswith(b"\n")


def test_audit_links_records(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    first = chain.append("one", {})
    second = chain.append("two", {})
    assert second["prev_hash"] == first["hash"]


def test_audit_detects_payload_tampering(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("one", {"safe": True})
    chain.path.write_text(chain.path.read_text().replace("true", "false"))
    with pytest.raises(IntegrityError, match="hash mismatch"):
        chain.verify()


def test_audit_detects_hash_tampering(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("one", {})
    raw = chain.path.read_text()
    chain.path.write_text(raw.replace('"hash":"', '"hash":"f', 1))
    with pytest.raises(IntegrityError):
        chain.verify()


def test_chaos_torn_audit_tail_is_rejected(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("one", {})
    with chain.path.open("ab") as stream:
        stream.write(b'{"crash":')
        stream.flush()
        os.fsync(stream.fileno())
    with pytest.raises(IntegrityError, match="torn"):
        chain.verify()


def test_torn_tail_can_be_inspected_for_recovery(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("one", {})
    with chain.path.open("ab") as stream:
        stream.write(b"partial")
    assert len(chain.records(tolerate_torn_tail=True)) == 1


def test_audit_rejects_empty_event(tmp_path):
    with pytest.raises(ValueError):
        AuditChain(tmp_path / "a.jsonl").append("", {})


def test_audit_concurrent_appends_remain_valid(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    threads = [threading.Thread(target=chain.append, args=("parallel", {"n": n})) for n in range(12)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert chain.verify()["records"] == 12


def test_real_fsync_makes_record_visible_from_new_descriptor(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("durable", {"value": 7})
    fd = os.open(chain.path, os.O_RDONLY)
    try:
        assert b'"event":"durable"' in os.read(fd, 4096)
    finally:
        os.close(fd)


def test_audit_head_changes(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    empty = chain.verify()["head"]
    chain.append("x", {})
    assert chain.verify()["head"] != empty


def test_audit_rejects_broken_previous_hash(tmp_path):
    chain = AuditChain(tmp_path / "a.jsonl")
    chain.append("x", {})
    raw = chain.path.read_text().replace("0" * 64, "1" * 64)
    chain.path.write_text(raw)
    with pytest.raises(IntegrityError, match="chain broken"):
        chain.verify()


# Durable V1072 integration and chaos (11)
def test_store_round_trip_identity(tmp_path):
    store = DurableIdentityStore(tmp_path)
    manifest = IdentityManifest()
    manifest.add("LTM", "fact", "never lose")
    store.save(manifest)
    loaded = store.load()
    assert loaded.core.identity_id == manifest.core.identity_id
    assert loaded.entries[0].content == "never lose"


def test_store_preserves_unicode(tmp_path):
    store = DurableIdentityStore(tmp_path)
    manifest = IdentityManifest()
    manifest.add("LTM", "fact", "楚零")
    store.save(manifest)
    assert store.load().entries[0].content == "楚零"


def test_store_new_self_check(tmp_path):
    check = DurableIdentityStore(tmp_path).startup_self_check()
    assert check["ok"] and check["state"] == "new"


def test_store_recovered_self_check(tmp_path):
    store = DurableIdentityStore(tmp_path)
    manifest = IdentityManifest()
    store.save(manifest)
    assert store.startup_self_check(manifest.core.identity_id)["state"] == "recovered"


def test_store_rejects_wrong_expected_identity(tmp_path):
    store = DurableIdentityStore(tmp_path)
    store.save(IdentityManifest())
    with pytest.raises(IntegrityError, match="identity mismatch"):
        store.startup_self_check("wrong")


def test_chaos_corrupt_snapshot_is_rejected(tmp_path):
    store = DurableIdentityStore(tmp_path)
    store.save(IdentityManifest())
    store.snapshot_path.write_bytes(b"{power-loss")
    with pytest.raises(IntegrityError, match="unreadable"):
        store.load()


def test_chaos_lost_snapshot_is_rejected(tmp_path):
    store = DurableIdentityStore(tmp_path)
    store.save(IdentityManifest())
    store.snapshot_path.unlink()
    with pytest.raises(IntegrityError, match="missing"):
        store.load()


def test_chaos_snapshot_rollback_detected(tmp_path):
    store = DurableIdentityStore(tmp_path)
    manifest = IdentityManifest()
    store.save(manifest)
    old = store.snapshot_path.read_bytes()
    manifest.add("LTM", "fact", "new")
    store.save(manifest)
    store.snapshot_path.write_bytes(old)
    with pytest.raises(IntegrityError, match="audit head"):
        store.load()


def test_chaos_audit_loss_detected(tmp_path):
    store = DurableIdentityStore(tmp_path)
    store.save(IdentityManifest())
    store.audit.path.unlink()
    with pytest.raises(IntegrityError, match="no audit commit"):
        store.load()


def test_store_overwrites_atomically_without_temp_leak(tmp_path):
    store = DurableIdentityStore(tmp_path)
    manifest = IdentityManifest()
    store.save(manifest)
    store.save(manifest)
    assert not list(tmp_path.glob(".identity-*.tmp"))


def test_backend_recovers_same_identity_after_restart(tmp_path):
    first = ASINorthStarBackend(tmp_path)
    identity_id = first.identity.core.identity_id
    second = ASINorthStarBackend(tmp_path)
    assert second.identity.core.identity_id == identity_id


# Real provider transports and network partition (9)
def test_local_provider_runs_real_process():
    evidence = RealModelGateway().call(ModelRequest("local", "process-engine", "ping", command=local_command()))
    assert evidence.real and evidence.transport == "process" and "ping" in evidence.content


def test_local_provider_nonzero_is_failure():
    command = [sys.executable, "-c", "import sys; sys.exit(3)"]
    with pytest.raises(V1124Error, match="local model failed"):
        RealModelGateway().call(ModelRequest("local", "bad", "x", command=command))


def test_local_provider_empty_is_failure():
    command = [sys.executable, "-c", "pass"]
    with pytest.raises(V1124Error, match="empty output"):
        RealModelGateway().call(ModelRequest("local", "empty", "x", command=command))


def test_openai_adapter_uses_real_http(provider_server):
    evidence = RealModelGateway().call(ModelRequest("openai", "gpt-test", "x", base_url=provider_server, api_key="test"))
    assert evidence.content == "gpt-real-http" and evidence.tokens_out == 2


def test_ollama_adapter_uses_real_http(provider_server):
    evidence = RealModelGateway().call(ModelRequest("ollama", "qwen-test", "x", base_url=provider_server))
    assert evidence.content == "ollama-real-http"


def test_anthropic_adapter_uses_real_http(provider_server):
    evidence = RealModelGateway().call(ModelRequest("anthropic", "claude-test", "x", base_url=provider_server, api_key="test"))
    assert evidence.content == "claude-real-http" and evidence.tokens_in == 3


def test_network_partition_is_explicit_failure():
    with pytest.raises(V1124Error) as error:
        RealModelGateway().call(ModelRequest("ollama", "none", "x", timeout_seconds=0.05,
                                             base_url="http://127.0.0.1:1"))
    assert error.value.code == "provider_unavailable"


def test_unsupported_provider_rejected():
    with pytest.raises(V1124Error) as error:
        RealModelGateway().call(ModelRequest("imaginary", "x", "x"))
    assert error.value.status == 400


def test_empty_prompt_rejected_before_transport():
    with pytest.raises(V1124Error) as error:
        RealModelGateway().call(ModelRequest("ollama", "x", "   "))
    assert error.value.code == "invalid_prompt"


# Backend model write-through and validation (7)
def test_measure_writes_identity_and_audit(backend):
    result = backend.measure({"provider": "local", "model": "process-engine", "prompt": "measure",
                              "command": local_command()})
    assert result["evidence"]["real"]
    assert backend.store.load().entries[-1].kind == "model_measurement"
    events = [r["event"] for r in backend.store.audit.records()]
    assert events[-2:] == ["identity_snapshot_committed", "asi_measurement"]


def test_measure_does_not_persist_raw_model_content(backend):
    secret = "model-output-secret"
    command = [sys.executable, "-c", f"print('{secret}')"]
    backend.measure({"provider": "local", "model": "x", "prompt": "x", "command": command})
    assert secret not in backend.store.audit.path.read_text()


def test_measure_requires_provider_and_model(backend):
    status, result = backend.dispatch("POST", "/asi/measure", {})
    assert status == 400 and result["error"]["code"] == "invalid_request"


def test_measure_rejects_bad_command_shape(backend):
    status, result = backend.dispatch("POST", "/asi/measure", {"provider": "local", "model": "x",
                                                                "prompt": "x", "command": "bad"})
    assert status == 400


def test_measure_rejects_excessive_timeout(backend):
    status, result = backend.dispatch("POST", "/asi/measure", {"provider": "local", "model": "x",
                                                                "prompt": "x", "timeout_seconds": 121})
    assert status == 400


def test_failed_measurement_is_not_written(backend):
    before = len(backend.store.audit.records())
    status, _ = backend.dispatch("POST", "/asi/measure", {"provider": "local", "model": "x", "prompt": "x"})
    assert status == 503 and len(backend.store.audit.records()) == before


def test_measurement_public_evidence_is_hashed(backend):
    result = backend.measure({"provider": "local", "model": "x", "prompt": "x", "command": local_command()})
    assert len(result["evidence"]["content_sha256"]) == 64
    assert "content" not in result["evidence"]


# HTTP end-to-end (7)
def http_json(url: str, method: str = "GET", body=None):
    raw = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(url, data=raw, method=method, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return response.status, json.loads(response.read()), response.headers
    except urllib.error.HTTPError as exc:
        return exc.code, json.loads(exc.read()), exc.headers


def test_http_level_end_to_end(backend):
    server = start_http_server(backend)
    try:
        status, body, headers = http_json(f"http://127.0.0.1:{server.server_port}/asi/level")
        assert status == 200 and body["score"] == BASELINE_V04
        assert headers["X-Content-Type-Options"] == "nosniff"
    finally:
        server.shutdown(); server.server_close()


def test_http_north_star_end_to_end(backend):
    server = start_http_server(backend)
    try:
        status, body, _ = http_json(f"http://127.0.0.1:{server.server_port}/asi/north-star")
        assert status == 200 and body["north_star"] == "ASI >= 0.95"
    finally:
        server.shutdown(); server.server_close()


def test_http_measure_end_to_end_real_process(backend):
    server = start_http_server(backend)
    try:
        status, body, _ = http_json(f"http://127.0.0.1:{server.server_port}/asi/measure", "POST",
                                    {"provider": "local", "model": "process-engine", "prompt": "http",
                                     "command": local_command()})
        assert status == 200 and body["evidence"]["transport"] == "process"
    finally:
        server.shutdown(); server.server_close()


def test_http_invalid_json_is_400(backend):
    server = start_http_server(backend)
    try:
        req = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/asi/measure", data=b"{",
                                     method="POST")
        with pytest.raises(urllib.error.HTTPError) as error:
            urllib.request.urlopen(req)
        assert error.value.code == 400
    finally:
        server.shutdown(); server.server_close()


def test_http_empty_body_is_400(backend):
    server = start_http_server(backend)
    try:
        status, body, _ = http_json(f"http://127.0.0.1:{server.server_port}/asi/measure", "POST")
        assert status == 400 and body["error"]["code"] == "invalid_body"
    finally:
        server.shutdown(); server.server_close()


def test_http_unknown_is_404(backend):
    server = start_http_server(backend)
    try:
        status, body, _ = http_json(f"http://127.0.0.1:{server.server_port}/unknown")
        assert status == 404 and body["error"]["code"] == "not_found"
    finally:
        server.shutdown(); server.server_close()


def test_http_concurrent_reads(backend):
    server = start_http_server(backend)
    results = []
    try:
        threads = [threading.Thread(target=lambda: results.append(
            http_json(f"http://127.0.0.1:{server.server_port}/asi/level")[0])) for _ in range(8)]
        for thread in threads: thread.start()
        for thread in threads: thread.join()
        assert results == [200] * 8
    finally:
        server.shutdown(); server.server_close()


# gRPC end-to-end (4)
def grpc_call(channel, name, request):
    method = channel.unary_unary(f"/apeireth.v1124.ASINorthStar/{name}",
                                 request_serializer=lambda value: json.dumps(value).encode(),
                                 response_deserializer=json.loads)
    return method(request, timeout=5)


def test_grpc_level_end_to_end(backend):
    server, port = start_grpc_server(backend)
    try:
        with grpc.insecure_channel(f"127.0.0.1:{port}") as channel:
            assert grpc_call(channel, "Level", {})["score"] == BASELINE_V04
    finally:
        server.stop(0).wait()


def test_grpc_north_star_end_to_end(backend):
    server, port = start_grpc_server(backend)
    try:
        with grpc.insecure_channel(f"127.0.0.1:{port}") as channel:
            assert "guards" in grpc_call(channel, "NorthStar", {})
    finally:
        server.stop(0).wait()


def test_grpc_measure_end_to_end_real_process(backend):
    server, port = start_grpc_server(backend)
    try:
        with grpc.insecure_channel(f"127.0.0.1:{port}") as channel:
            result = grpc_call(channel, "Measure", {"provider": "local", "model": "process-engine",
                                                     "prompt": "grpc", "command": local_command()})
            assert result["evidence"]["real"] and result["evidence"]["transport"] == "process"
    finally:
        server.stop(0).wait()


def test_grpc_validation_maps_status(backend):
    server, port = start_grpc_server(backend)
    try:
        with grpc.insecure_channel(f"127.0.0.1:{port}") as channel:
            with pytest.raises(grpc.RpcError) as error:
                grpc_call(channel, "Measure", {})
            assert error.value.code() == grpc.StatusCode.INVALID_ARGUMENT
    finally:
        server.stop(0).wait()
