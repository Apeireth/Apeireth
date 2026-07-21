"""V1024 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1024_config import (
    V1024_VERSION, parse_dotenv, parse_yaml_simple, V1024Config,
)


class TestV1024:
    def test_parse_dotenv(self):
        """V1024 真测 python-dotenv 真借鉴 (主 19:33)."""
        text = """
# comment
KEY1=value1
KEY2="value2"
KEY3='value3'
"""
        parsed = parse_dotenv(text)
        assert parsed["KEY1"] == "value1"
        assert parsed["KEY2"] == "value2"
        assert parsed["KEY3"] == "value3"

    def test_parse_dotenv_empty(self):
        assert parse_dotenv("") == {}

    def test_parse_yaml(self):
        """V1024 真测 OmegaConf 真借鉴 (主 19:33)."""
        text = """
key1: value1
key2: 42
key3: 3.14
key4: true
key5: false
key6: null
"""
        parsed = parse_yaml_simple(text)
        assert parsed["key1"] == "value1"
        assert parsed["key2"] == 42
        assert parsed["key3"] == 3.14
        assert parsed["key4"] is True
        assert parsed["key5"] is False
        assert parsed["key6"] is None

    def test_parse_yaml_comments(self):
        parsed = parse_yaml_simple("# comment\nkey: value\n")
        assert parsed["key"] == "value"

    def test_init(self):
        c = V1024Config()
        assert c.n_keys() == 0

    def test_init_defaults(self):
        c = V1024Config(defaults={"a": 1, "b": 2})
        assert c.get("a") == 1
        assert c.get("b") == 2

    def test_load_dotenv(self):
        c = V1024Config()
        c.load_dotenv("KEY=value")
        assert c.get("KEY") == "value"

    def test_load_yaml(self):
        c = V1024Config()
        c.load_yaml("key: value\nnum: 42")
        assert c.get("key") == "value"
        assert c.get("num") == 42

    def test_load_json(self):
        c = V1024Config()
        c.load_json('{"a": 1, "b": 2}')
        assert c.get("a") == 1
        assert c.get("b") == 2

    def test_get_default(self):
        c = V1024Config()
        assert c.get("missing", default="x") == "x"

    def test_get_dot_path(self):
        """V1024 真测 OmegaConf.dot path 真借鉴 (主 19:33)."""
        c = V1024Config()
        c.set("a.b.c", "deep")
        assert c.get("a.b.c") == "deep"

    def test_set(self):
        c = V1024Config()
        c.set("key", "value")
        assert c.get("key") == "value"

    def test_set_nested(self):
        c = V1024Config()
        c.set("a.b.c", 1)
        assert c.get("a.b.c") == 1

    def test_load_env_vars(self):
        """V1024 真测 env vars 真借鉴 (主 19:33)."""
        import os
        os.environ["TEST_VAR_1024"] = "test_value"
        c = V1024Config()
        c.load_env_vars(prefix="TEST_VAR_1024")
        assert c.get("TEST_VAR_1024") == "test_value"
        del os.environ["TEST_VAR_1024"]

    def test_merge(self):
        """V1024 真测 Hydra 真借鉴 (主 19:33)."""
        c1 = V1024Config({"a": 1, "b": {"c": 2}})
        c2 = V1024Config({"b": {"d": 3}})
        c1.merge(c2)
        assert c1.get("a") == 1
        assert c1.get("b.c") == 2
        assert c1.get("b.d") == 3

    def test_keys(self):
        c = V1024Config({"a": 1, "b": 2})
        assert "a" in c.keys()
        assert "b" in c.keys()

    def test_stats(self):
        c = V1024Config({"a": 1})
        s = c.stats()
        assert s["n_keys"] == 1
        assert s["version"] == V1024_VERSION

    def test_v22_33_asi_integration(self):
        """V1024 真测主 22:33 ASI 北极星."""
        c = V1024Config()
        s = c.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_dotenv_omegaconf_hydra(self):
        """V1024 真测主 19:33 dotenv + OmegaConf + Hydra 真借鉴."""
        c = V1024Config()
        # dotenv
        c.load_dotenv("X=1")
        # OmegaConf dot path
        c.set("a.b.c", 2)
        # Hydra merge
        c.merge(V1024Config({"d": 3}))
        assert c.get("X") == "1"
        assert c.get("a.b.c") == 2
        assert c.get("d") == 3

    def test_v17_43_truth(self):
        """V1024 真测主 17:43 实事求是 — 真 parse."""
        parsed = parse_dotenv("K=v")
        assert parsed["K"] == "v"

    def test_complete_integration(self):
        """V1024 真测完整 config (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        c = V1024Config()
        c.load_dotenv("ASI_LEVEL=0.7905")
        c.load_yaml("workers: 4")
        c.load_json('{"version": "1.0.0"}')
        assert c.get("ASI_LEVEL") == "0.7905"
        assert c.get("workers") == 4
        assert c.get("version") == "1.0.0"