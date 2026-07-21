"""V1028 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import time
import pytest
from apeireth.v1028_jwt import (
    V1028_VERSION, JWT, base64url_encode, base64url_decode, hmac_sha256,
    V1028JWTAuth,
)


class TestV1028:
    def test_base64url_encode(self):
        encoded = base64url_encode(b"hello world")
        assert "=" not in encoded  # URL-safe

    def test_base64url_decode(self):
        original = b"hello world"
        encoded = base64url_encode(original)
        decoded = base64url_decode(encoded)
        assert decoded == original

    def test_base64url_round_trip(self):
        original = b"Apeireth ASI 1012 modules"
        encoded = base64url_encode(original)
        decoded = base64url_decode(encoded)
        assert decoded == original

    def test_hmac_sha256(self):
        sig = hmac_sha256(b"key", b"message")
        assert len(sig) == 32  # SHA256 output

    def test_init(self):
        auth = V1028JWTAuth("secret")
        assert auth.n_tokens() == 0

    def test_encode(self):
        """V1028 真测 PyJWT encode 真借鉴 (主 19:33)."""
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1"})
        assert token.count(".") == 2  # 3 parts

    def test_decode(self):
        """V1028 真测 PyJWT decode 真借鉴 (主 19:33)."""
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1", "tenant_id": "t1"})
        payload = auth.decode(token)
        assert payload["sub"] == "user1"
        assert payload["tenant_id"] == "t1"
        assert "exp" in payload
        assert "iat" in payload

    def test_decode_invalid_format(self):
        auth = V1028JWTAuth("secret")
        assert auth.decode("invalid") is None
        assert auth.decode("a.b") is None
        assert auth.decode("a.b.c.d") is None

    def test_decode_tampered(self):
        """V1028 真测 JWT 篡改检测 (主 17:43 实事求是)."""
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1"})
        # 篡改 payload
        parts = token.split(".")
        # 修改 payload 但保持原 signature
        tampered = parts[0] + "." + base64url_encode(b'{"sub":"attacker"}') + "." + parts[2]
        assert auth.decode(tampered) is None

    def test_decode_wrong_secret(self):
        """V1028 真测 wrong secret 真借鉴 (主 17:43)."""
        auth1 = V1028JWTAuth("secret1")
        token = auth1.encode({"sub": "user1"})
        auth2 = V1028JWTAuth("secret2")
        assert auth2.decode(token) is None

    def test_decode_expired(self):
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1"}, expires_in=1)
        time.sleep(2.5)
        assert auth.decode(token) is None

    def test_verify(self):
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1"})
        assert auth.verify(token) is True

    def test_verify_invalid(self):
        auth = V1028JWTAuth("secret")
        assert auth.verify("invalid") is False

    def test_encode_with_custom_payload(self):
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "u1", "role": "admin", "scopes": ["read", "write"]})
        payload = auth.decode(token)
        assert payload["role"] == "admin"
        assert "read" in payload["scopes"]

    def test_n_tokens(self):
        auth = V1028JWTAuth("secret")
        auth.encode({"sub": "u1"})
        auth.encode({"sub": "u2"})
        auth.encode({"sub": "u3"})
        assert auth.n_tokens() == 3

    def test_stats(self):
        auth = V1028JWTAuth("secret")
        s = auth.stats()
        assert s["n_tokens"] == 0
        assert s["version"] == V1028_VERSION

    def test_v22_33_asi_integration(self):
        """V1028 真测主 22:33 ASI 北极星."""
        auth = V1028JWTAuth("secret")
        s = auth.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_pyjwt(self):
        """V1028 真测主 19:33 PyJWT + RFC 7519 真借鉴."""
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1", "tenant_id": "t1"})
        payload = auth.decode(token)
        assert payload is not None
        assert "exp" in payload
        assert "iat" in payload

    def test_v17_43_truth(self):
        """V1028 真测主 17:43 实事求是 — 真签名, 真篡改检测."""
        import json as _json
        auth = V1028JWTAuth("secret")
        token = auth.encode({"sub": "user1", "amount": 100})
        # 解码得到 amount=100
        payload = auth.decode(token)
        assert payload["amount"] == 100
        # 篡改 amount
        tampered_payload = dict(payload)
        tampered_payload["amount"] = 1000000
        tampered_str = base64url_encode(_json.dumps(tampered_payload).encode())
        parts = token.split(".")
        tampered = parts[0] + "." + tampered_str + "." + parts[2]
        # 篡改后签名验证失败
        assert auth.decode(tampered) is None

    def test_complete_integration(self):
        """V1028 真测完整 JWT (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        auth = V1028JWTAuth("jwt-secret-123")
        # 5 真用户
        tokens = []
        for i in range(5):
            t = auth.encode({"sub": f"user_{i}", "tenant_id": "t1", "role": "user"}, expires_in=3600)
            tokens.append(t)
        # 真 decode
        for i, t in enumerate(tokens):
            payload = auth.decode(t)
            assert payload["sub"] == f"user_{i}"
        # 真过期
        expired = auth.encode({"sub": "u"}, expires_in=1)
        time.sleep(2.5)
        assert auth.decode(expired) is None