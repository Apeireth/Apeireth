"""V1030 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import json
import pytest
from apeireth.v1030_webhook import (
    V1030_VERSION, WebhookEndpoint, WebhookDelivery,
    sign_payload, verify_signature, V1030Webhook,
)


class TestV1030:
    def test_sign_payload(self):
        """V1030 真测 Stripe HMAC-SHA256 真借鉴 (主 19:33)."""
        sig = sign_payload("secret", b"hello world")
        assert sig.startswith("sha256=")
        assert len(sig) > 10

    def test_sign_deterministic(self):
        s1 = sign_payload("secret", b"x")
        s2 = sign_payload("secret", b"x")
        assert s1 == s2

    def test_sign_different_payload(self):
        s1 = sign_payload("secret", b"x")
        s2 = sign_payload("secret", b"y")
        assert s1 != s2

    def test_sign_different_secret(self):
        s1 = sign_payload("secret1", b"x")
        s2 = sign_payload("secret2", b"x")
        assert s1 != s2

    def test_verify_signature_success(self):
        sig = sign_payload("secret", b"hello")
        assert verify_signature("secret", b"hello", sig) is True

    def test_verify_signature_fail(self):
        sig = sign_payload("secret", b"hello")
        assert verify_signature("secret", b"world", sig) is False

    def test_verify_signature_wrong_secret(self):
        sig = sign_payload("secret1", b"hello")
        assert verify_signature("secret2", b"hello", sig) is False

    def test_init(self):
        wh = V1030Webhook()
        assert wh.n_endpoints() == 0
        assert wh.n_deliveries() == 0

    def test_register_endpoint(self):
        """V1030 真测 Stripe webhook 真借鉴 (主 19:33)."""
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://api.app.com/hooks", ["memory.created"])
        assert wh.n_endpoints() == 1
        assert ep.endpoint_id.startswith("ep_")

    def test_register_endpoint_with_custom_secret(self):
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["*"], secret="my-secret")
        assert ep.secret == "my-secret"

    def test_publish_no_subscribers(self):
        wh = V1030Webhook()
        result = wh.publish("memory.created", {"id": "m1"})
        assert result == []

    def test_publish_to_endpoint(self):
        """V1030 真测 Stripe publish 真借鉴 (主 19:33)."""
        wh = V1030Webhook()
        wh.register_endpoint("https://api.app.com/hooks", ["memory.created"])
        deliveries = wh.publish("memory.created", {"id": "m1"})
        assert len(deliveries) == 1
        assert wh.n_deliveries() == 1

    def test_publish_event_filter(self):
        wh = V1030Webhook()
        wh.register_endpoint("https://x", ["memory.created"])
        wh.register_endpoint("https://y", ["memory.deleted"])
        # memory.created 只发给 endpoint 1
        result = wh.publish("memory.created", {"id": "m1"})
        assert len(result) == 1

    def test_publish_wildcard(self):
        wh = V1030Webhook()
        wh.register_endpoint("https://x", ["*"])
        wh.register_endpoint("https://y", ["memory.created"])
        result = wh.publish("memory.created", {})
        assert len(result) == 2
        result2 = wh.publish("other_event", {})
        assert len(result2) == 1  # 只 wildcard endpoint

    def test_publish_disabled_endpoint(self):
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["memory.created"])
        ep.enabled = False
        result = wh.publish("memory.created", {})
        assert len(result) == 0

    def test_delivery_has_signature(self):
        """V1030 真测 signature 真生产 (主 19:33 HMAC)."""
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["e"], secret="my-secret")
        deliveries = wh.publish("e", {"data": "test"})
        assert deliveries[0].signature.startswith("sha256=")
        # 验证 signature
        body = json.dumps({"data": "test"}).encode()
        assert verify_signature("my-secret", body, deliveries[0].signature)

    def test_attempt_delivery_success(self):
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["e"])
        deliveries = wh.publish("e", {})
        result = wh.attempt_delivery(deliveries[0])
        assert result is True
        assert deliveries[0].success is True
        assert wh.n_successful() == 1

    def test_attempt_delivery_failure_retry(self):
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["e"])
        deliveries = wh.publish("e", {})
        # 第一次失败
        result = wh.attempt_delivery(deliveries[0], success=False, status_code=500)
        assert result is False
        assert deliveries[0].attempts == 1
        # 第二次失败
        result = wh.attempt_delivery(deliveries[0], success=False, status_code=500)
        assert result is False
        assert deliveries[0].attempts == 2
        # 第三次失败 → max retries
        result = wh.attempt_delivery(deliveries[0], success=False, status_code=500)
        assert result is False
        assert deliveries[0].success is False
        assert deliveries[0].attempts == 3

    def test_attempt_delivery_retry_succeeds(self):
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["e"])
        deliveries = wh.publish("e", {})
        # 第一次失败
        wh.attempt_delivery(deliveries[0], success=False, status_code=500)
        # 第二次成功
        result = wh.attempt_delivery(deliveries[0], success=True)
        assert result is True
        assert deliveries[0].success is True

    def test_stats(self):
        wh = V1030Webhook()
        wh.register_endpoint("https://x", ["*"])
        wh.publish("e", {})
        s = wh.stats()
        assert s["n_endpoints"] == 1
        assert s["n_deliveries"] == 1

    def test_v22_33_asi_integration(self):
        """V1030 真测主 22:33 ASI 北极星."""
        wh = V1030Webhook()
        s = wh.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_stripe_slack(self):
        """V1030 真测主 19:33 Stripe + Slack webhook 真借鉴."""
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://hooks.slack.com/services/xxx", ["*"])
        deliveries = wh.publish("memory.created", {"data": "test"})
        assert len(deliveries) == 1
        # 真 signature
        body = json.dumps({"data": "test"}).encode()
        assert verify_signature(ep.secret, body, deliveries[0].signature)

    def test_v17_43_truth(self):
        """V1030 真测主 17:43 实事求是 — 真签名, 真 retry."""
        wh = V1030Webhook()
        ep = wh.register_endpoint("https://x", ["e"], secret="secret123")
        deliveries = wh.publish("e", {"foo": "bar"})
        # 真 signature 验证
        body = json.dumps({"foo": "bar"}).encode()
        assert verify_signature("secret123", body, deliveries[0].signature)
        # 篡改 payload 后 signature 不对
        tampered_body = json.dumps({"foo": "baz"}).encode()
        assert not verify_signature("secret123", tampered_body, deliveries[0].signature)

    def test_complete_integration(self):
        """V1030 真测完整 webhook (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        wh = V1030Webhook(max_retries=3)
        # 2 真 endpoints
        wh.register_endpoint("https://api1.com/hooks", ["memory.created", "memory.deleted"])
        wh.register_endpoint("https://api2.com/hooks", ["*"])
        # 真发布
        deliveries = wh.publish("memory.created", {"id": "m1", "content": "ASI 真生产"})
        assert len(deliveries) == 2
        # 真 delivery
        wh.attempt_delivery(deliveries[0])
        assert wh.n_successful() == 1