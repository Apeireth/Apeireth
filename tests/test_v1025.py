"""V1025 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1025_secrets import (
    V1025_VERSION, Secret, xor_encrypt, xor_decrypt, derive_key,
    V1025SecretsManager,
)


class TestV1025:
    def test_xor_encrypt_decrypt(self):
        plaintext = b"hello world"
        key = b"mykey"
        ct = xor_encrypt(plaintext, key)
        pt = xor_decrypt(ct, key)
        assert pt == plaintext

    def test_xor_different_key(self):
        plaintext = b"hello world"
        ct = xor_encrypt(plaintext, b"key1")
        pt = xor_decrypt(ct, b"key2")
        assert pt != plaintext

    def test_xor_empty_key(self):
        with pytest.raises(ValueError):
            xor_encrypt(b"hello", b"")

    def test_derive_key_deterministic(self):
        k1 = derive_key("password")
        k2 = derive_key("password")
        assert k1 == k2

    def test_derive_key_different_password(self):
        k1 = derive_key("password1")
        k2 = derive_key("password2")
        assert k1 != k2

    def test_init(self):
        sm = V1025SecretsManager()
        assert sm.n_secrets() == 0

    def test_init_with_key(self):
        sm = V1025SecretsManager(master_key="my-key")
        assert sm.n_secrets() == 0

    def test_put(self):
        """V1025 真测 Vault KV put 真借鉴 (主 19:33)."""
        sm = V1025SecretsManager(master_key="key1")
        sid = sm.put("api/key", "secret_value")
        assert sm.n_secrets() == 1

    def test_get(self):
        """V1025 真测 Vault KV get 真借鉴 (主 19:33)."""
        sm = V1025SecretsManager(master_key="key1")
        sid = sm.put("api/key", "secret_value_xyz")
        value = sm.get(sid)
        assert value == "secret_value_xyz"

    def test_get_wrong_key(self):
        """V1025 真测 用错 master key 真借鉴 (主 17:43 实事求是)."""
        sm1 = V1025SecretsManager(master_key="key1")
        sid = sm1.put("api/key", "secret")
        sm2 = V1025SecretsManager(master_key="key2")
        value = sm2.get(sid)
        # XOR 加密错 key 解密结果不对
        assert value != "secret"

    def test_get_unknown(self):
        sm = V1025SecretsManager(master_key="key1")
        assert sm.get("unknown") is None

    def test_get_by_path(self):
        sm = V1025SecretsManager(master_key="key1")
        sm.put("api/openai", "value_v1")
        sm.put("api/openai", "value_v2")
        latest = sm.get_by_path("api/openai")
        assert latest == "value_v2"

    def test_get_by_path_specific_version(self):
        sm = V1025SecretsManager(master_key="key1")
        sm.put("api/openai", "value_v1")
        sm.put("api/openai", "value_v2")
        v1 = sm.get_by_path("api/openai", version=1)
        v2 = sm.get_by_path("api/openai", version=2)
        assert v1 == "value_v1"
        assert v2 == "value_v2"

    def test_get_by_path_missing(self):
        sm = V1025SecretsManager(master_key="key1")
        assert sm.get_by_path("missing") is None

    def test_delete(self):
        sm = V1025SecretsManager(master_key="key1")
        sid = sm.put("api/key", "value")
        assert sm.delete(sid) is True
        assert sm.get(sid) is None

    def test_delete_unknown(self):
        sm = V1025SecretsManager(master_key="key1")
        assert sm.delete("unknown") is False

    def test_list_paths(self):
        sm = V1025SecretsManager(master_key="key1")
        sm.put("api/openai", "v1")
        sm.put("api/anthropic", "v2")
        sm.put("db/postgres", "v3")
        paths = sm.list_paths()
        assert "api/openai" in paths
        assert "api/anthropic" in paths
        assert "db/postgres" in paths

    def test_audit_log(self):
        sm = V1025SecretsManager(master_key="key1")
        sid = sm.put("api/key", "value")
        sm.get(sid)
        sm.delete(sid)
        assert len(sm.audit_log) == 3
        actions = [a["action"] for a in sm.audit_log]
        assert "put" in actions
        assert "get" in actions
        assert "delete" in actions

    def test_stats(self):
        sm = V1025SecretsManager(master_key="key1")
        sm.put("a", "1")
        sm.put("b", "2")
        s = sm.stats()
        assert s["n_secrets"] == 2
        assert s["n_paths"] == 2

    def test_v22_33_asi_integration(self):
        """V1025 真测主 22:33 ASI 北极星."""
        sm = V1025SecretsManager(master_key="key1")
        s = sm.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_vault_aws(self):
        """V1025 真测主 19:33 HashiCorp Vault + AWS Secrets Manager 真借鉴."""
        sm = V1025SecretsManager(master_key="key1")
        # Vault KV 风格
        sid = sm.put("secret/data/openai", "sk-abc")
        assert sm.get(sid) == "sk-abc"
        # AWS 风格 by path
        sm.put("prod/db", "postgres://...")
        assert sm.get_by_path("prod/db") == "postgres://..."

    def test_v17_43_truth(self):
        """V1025 真测主 17:43 实事求是 — 真加密, 真解密."""
        sm = V1025SecretsManager(master_key="key1")
        sid = sm.put("test", "secret_value_123")
        # 内部 value 不是明文
        assert sm.secrets[sid].value != b"secret_value_123"
        # 用对的 key 解密能拿到明文
        assert sm.get(sid) == "secret_value_123"

    def test_complete_integration(self):
        """V1025 真测完整 secrets manager (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        sm = V1025SecretsManager(master_key="my-master-key")
        # 5 真 secrets
        sm.put("api/openai", "sk-1234")
        sm.put("api/anthropic", "sk-ant-5678")
        sm.put("db/postgres", "postgres://user:pass@host/db")
        sm.put("redis/url", "redis://host:6379")
        sm.put("jwt/secret", "jwt-secret-key")
        # 真 get
        assert sm.get_by_path("api/openai") == "sk-1234"
        assert sm.get_by_path("db/postgres").startswith("postgres://")
        # audit log (5 puts + 2 gets = 7)
        assert len(sm.audit_log) == 7
        # 真路径列表
        paths = sm.list_paths()
        assert len(paths) == 5