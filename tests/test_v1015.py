"""V1015 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
import tempfile, os
from apeireth.v1015_audit_log import (
    V1015_VERSION, AuditEvent, V1015AuditLog,
)


class TestV1015:
    def test_init(self):
        al = V1015AuditLog()
        assert al.n_events() == 0

    def test_log_event(self):
        al = V1015AuditLog()
        ev = al.log("user1", "read", "/api/memories")
        assert ev.actor == "user1"
        assert ev.action == "read"
        assert ev.resource == "/api/memories"
        assert ev.result == "success"
        assert al.n_events() == 1

    def test_log_event_failure(self):
        al = V1015AuditLog()
        ev = al.log("user1", "delete", "/api/memories/123", result="failure")
        assert ev.result == "failure"

    def test_log_event_denied(self):
        al = V1015AuditLog()
        ev = al.log("user1", "write", "/api/admin", result="denied")
        assert ev.result == "denied"

    def test_log_invalid_result(self):
        al = V1015AuditLog()
        with pytest.raises(ValueError):
            al.log("u", "a", "r", result="invalid")

    def test_signature_computed(self):
        """V1015 真测 Sigstore 签名真借鉴."""
        al = V1015AuditLog()
        ev = al.log("user1", "read", "/api")
        assert ev.signature != ""
        assert len(ev.signature) == 64  # sha256 hex

    def test_verify_signature(self):
        """V1015 真测 verify 真借鉴 (主 17:43 实事求是)."""
        al = V1015AuditLog()
        ev = al.log("user1", "read", "/api")
        assert al.verify(ev) is True

    def test_verify_tampered(self):
        """V1015 真测篡改检测 (主 17:43 实事求是 — 真检测)."""
        al = V1015AuditLog()
        ev = al.log("user1", "read", "/api")
        ev.action = "delete"  # 篡改
        assert al.verify(ev) is False

    def test_query_by_actor(self):
        """V1015 真测 CloudTrail Insights 真借鉴 (主 19:33)."""
        al = V1015AuditLog()
        al.log("user1", "read", "/api")
        al.log("user2", "read", "/api")
        result = al.query(actor="user1")
        assert len(result) == 1
        assert result[0].actor == "user1"

    def test_query_by_action(self):
        al = V1015AuditLog()
        al.log("user1", "read", "/api")
        al.log("user1", "write", "/api")
        result = al.query(action="write")
        assert len(result) == 1

    def test_query_by_result(self):
        al = V1015AuditLog()
        al.log("u", "a", "r", result="success")
        al.log("u", "a", "r", result="failure")
        al.log("u", "a", "r", result="denied")
        assert len(al.query(result="success")) == 1
        assert len(al.query(result="failure")) == 1
        assert len(al.query(result="denied")) == 1

    def test_query_by_since_ts(self):
        import time as _t
        al = V1015AuditLog()
        al.log("u", "a", "r")
        result = al.query(since_ts=_t.time() + 1)
        assert len(result) == 0

    def test_query_no_filter(self):
        al = V1015AuditLog()
        al.log("u", "a", "r")
        al.log("u", "a", "r")
        result = al.query()
        assert len(result) == 2

    def test_export_jsonl(self):
        """V1015 真测 JSONL 导出 (主 19:33 Sigstore 真借鉴)."""
        al = V1015AuditLog()
        al.log("u1", "read", "/api")
        al.log("u2", "write", "/api")
        with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
            tmp = f.name
        result = al.export_jsonl(tmp)
        assert result is True
        content = open(tmp, encoding='utf-8').read()
        assert "u1" in content
        assert "u2" in content
        assert content.count("\n") == 2
        os.remove(tmp)

    def test_export_invalid_path(self):
        al = V1015AuditLog()
        al.log("u", "a", "r")
        result = al.export_jsonl("/nonexistent/dir/x.jsonl")
        assert result is False

    def test_stats(self):
        al = V1015AuditLog()
        al.log("u", "a", "r", result="success")
        al.log("u", "a", "r", result="failure")
        s = al.stats()
        assert s["n_events"] == 2
        assert s["results"]["success"] == 1
        assert s["results"]["failure"] == 1

    def test_v22_33_asi_integration(self):
        """V1015 真测主 22:33 ASI 北极星."""
        al = V1015AuditLog()
        s = al.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_cloudtrail_sigstore(self):
        """V1015 真测主 19:33 CloudTrail + Sigstore 真借鉴."""
        al = V1015AuditLog()
        ev = al.log("u", "a", "r")
        # Sigstore 签名
        assert ev.signature != ""
        # CloudTrail Insights query
        assert al.query(actor="u")[0].event_id == ev.event_id

    def test_v17_43_tamper_detection(self):
        """V1015 真测主 17:43 实事求是 — 真篡改检测, 不假装."""
        al = V1015AuditLog()
        ev = al.log("u", "read", "/api")
        assert al.verify(ev)
        ev.actor = "attacker"
        assert not al.verify(ev)

    def test_complete_integration(self):
        """V1015 真测完整 audit log (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        al = V1015AuditLog()
        al.log("admin", "create", "/tenants/1")
        al.log("user", "read", "/tenants/1")
        al.log("attacker", "delete", "/tenants/1", result="denied")
        s = al.stats()
        assert s["n_events"] == 3
        assert s["results"]["denied"] == 1
        # 真查 denied
        denied = al.query(result="denied")
        assert denied[0].actor == "attacker"