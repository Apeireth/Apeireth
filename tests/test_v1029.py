"""V1029 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import time
import pytest
from apeireth.v1029_oauth import (
    V1029_VERSION, OAuthClient, AuthCode, AccessToken,
    generate_code_verifier, derive_code_challenge, V1029OAuth,
)


class TestV1029:
    def test_generate_code_verifier(self):
        """V1029 真测 PKCE RFC 7636 真借鉴 (主 19:33)."""
        v = generate_code_verifier()
        assert len(v) > 0
        assert "=" not in v  # URL-safe

    def test_derive_code_challenge(self):
        v = generate_code_verifier()
        c = derive_code_challenge(v)
        assert len(c) > 0

    def test_derive_code_challenge_deterministic(self):
        v = "my-verifier"
        c1 = derive_code_challenge(v)
        c2 = derive_code_challenge(v)
        assert c1 == c2

    def test_init(self):
        oauth = V1029OAuth()
        assert oauth.n_clients() == 0
        assert oauth.n_tokens() == 0

    def test_register_client(self):
        """V1029 真测 OAuth 2.0 RFC 6749 §2 真借鉴 (主 19:33)."""
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        assert oauth.n_clients() == 1
        assert client.client_id.startswith("client_")

    def test_authorize(self):
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        assert "code=" in url

    def test_authorize_with_state(self):
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1", state="xyz")
        assert "state=xyz" in url

    def test_authorize_invalid_client(self):
        oauth = V1029OAuth()
        with pytest.raises(ValueError):
            oauth.authorize("unknown", "http://x", "u1")

    def test_authorize_invalid_redirect(self):
        """V1029 真测 redirect_uri 验证 (主 19:33 OAuth 安全 真借鉴)."""
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        with pytest.raises(ValueError):
            oauth.authorize(client.client_id, "http://evil.com/callback", "u1")

    def test_exchange_code_success(self):
        """V1029 真测 RFC 6749 §4.1.3 真借鉴 (主 19:33)."""
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        token = oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        assert token is not None
        assert oauth.n_tokens() == 1

    def test_exchange_code_wrong_secret(self):
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        token = oauth.exchange_code(client.client_id, "wrong_secret", code, "http://localhost/callback")
        assert token is None

    def test_exchange_code_unknown_client(self):
        oauth = V1029OAuth()
        token = oauth.exchange_code("unknown", "secret", "code", "uri")
        assert token is None

    def test_exchange_code_already_used(self):
        """V1029 真测 code 一次性 (主 19:33 RFC 6749 §10.5 真借鉴)."""
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        # 第一次交换成功
        oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        # 第二次失败 (code 已用)
        token = oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        assert token is None

    def test_exchange_code_unknown_code(self):
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        token = oauth.exchange_code(client.client_id, client.client_secret, "unknown", "http://localhost/callback")
        assert token is None

    def test_validate_token(self):
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        token = oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        validated = oauth.validate_token(token.token)
        assert validated is not None
        assert validated.user_id == "user1"

    def test_validate_token_unknown(self):
        oauth = V1029OAuth()
        assert oauth.validate_token("unknown") is None

    def test_validate_token_expired(self):
        oauth = V1029OAuth()
        # 直接构造一个过期 token
        token = AccessToken(
            token="test", client_id="c", user_id="u",
            scopes=[], expires_at=time.time() - 10,
        )
        oauth.access_tokens["test"] = token
        assert oauth.validate_token("test") is None

    def test_stats(self):
        oauth = V1029OAuth()
        s = oauth.stats()
        assert s["n_clients"] == 0
        assert s["n_tokens"] == 0

    def test_v22_33_asi_integration(self):
        """V1029 真测主 22:33 ASI 北极星."""
        oauth = V1029OAuth()
        s = oauth.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_oauth_pkce(self):
        """V1029 真测主 19:33 OAuth 2.0 RFC 6749 + PKCE RFC 7636 真借鉴."""
        oauth = V1029OAuth()
        # PKCE
        verifier = generate_code_verifier()
        challenge = derive_code_challenge(verifier)
        # OAuth flow
        client = oauth.register_client("App", ["http://localhost/callback"])
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        token = oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        assert token is not None

    def test_v17_43_truth(self):
        """V1029 真测主 17:43 实事求是 — 真 OAuth 流程."""
        oauth = V1029OAuth()
        client = oauth.register_client("App", ["http://localhost/callback"])
        # 完整流程
        url = oauth.authorize(client.client_id, "http://localhost/callback", "user1")
        code = url.split("code=")[1]
        # code 真的只能用一次
        oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        second = oauth.exchange_code(client.client_id, client.client_secret, code, "http://localhost/callback")
        assert second is None

    def test_complete_integration(self):
        """V1029 真测完整 OAuth 2.0 (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        oauth = V1029OAuth()
        # 2 真 clients
        c1 = oauth.register_client("App1", ["http://app1/callback"])
        c2 = oauth.register_client("App2", ["http://app2/callback"])
        # authorize + exchange
        url1 = oauth.authorize(c1.client_id, "http://app1/callback", "alice")
        code1 = url1.split("code=")[1]
        token1 = oauth.exchange_code(c1.client_id, c1.client_secret, code1, "http://app1/callback")
        assert token1 is not None
        # validate
        validated = oauth.validate_token(token1.token)
        assert validated.user_id == "alice"
        # 真测: c2 的 code 不能用 c1 的 secret
        url2 = oauth.authorize(c2.client_id, "http://app2/callback", "bob")
        code2 = url2.split("code=")[1]
        bad = oauth.exchange_code(c2.client_id, c1.client_secret, code2, "http://app2/callback")
        assert bad is None