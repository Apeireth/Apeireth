"""Tests for V1434 — ASI VCP consistency HTTP adapter (主 00:56 任何人都能接手)."""

from __future__ import annotations

import ast
import json
import sys
import tempfile
from pathlib import Path

import pytest


def test_v1434_importable():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert m.V1434_VERSION == "0.1.0"


def test_v1434_guards_count():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert len(m.V1434_GUARDS) == 14
    assert len(m.V1434_V3_GUARDS) == 5


def test_v1434_borrowed_count():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert len(m.V1434_BORROWED) == 5


def test_v1434_endpoints_count():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert len(m.ENDPOINTS) == 2
    assert "/api/asi/vcp-consistency" in m.ENDPOINTS
    assert "/api/asi/version" in m.ENDPOINTS


def test_v1434_artifact_dataclass():
    import apeireth.v1434_asi_vcp_consistency_http as m
    a = m.Artifact(kind="test", filename="x.txt", content="hello\n")
    assert a.kind == "test"
    assert a.lines >= 1
    assert a.bytes >= 5
    assert len(a.sha256) == 16
    d = a.to_dict()
    assert d["kind"] == "test"


def test_v1434_generate_python_server_script_is_valid_python():
    """Generated python script parses with ast.parse."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    py = m.generate_python_server_script()
    assert m.validate_python_script(py)
    # Actually parse to confirm
    ast.parse(py)


def test_v1434_generate_python_server_script_imports_v1433():
    """Generated script imports V1433 build_report."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    py = m.generate_python_server_script()
    assert "from apeireth.v1433_asi_vcp_structural_consistency import build_report" in py
    assert "VcpConsistencyHandler" in py
    assert "/api/asi/vcp-consistency" in py


def test_v1434_generate_nginx_snippet_has_required_directives():
    """nginx snippet has upstream + location + proxy_pass."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    nginx = m.generate_nginx_snippet()
    assert m.validate_nginx_snippet(nginx)
    assert "upstream" in nginx
    assert "location" in nginx
    assert "proxy_pass" in nginx


def test_v1434_generate_docker_compose_has_two_services():
    """docker-compose has 2 services + ports + healthcheck."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    compose = m.generate_docker_compose_snippet()
    assert m.validate_compose_snippet(compose)
    assert "v1433-consistency" in compose
    assert "v1434-nginx" in compose
    assert "ports:" in compose
    assert "healthcheck:" in compose


def test_v1434_generate_consistency_json_returns_parseable():
    """consistency_json is parseable JSON dict."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    j = m.generate_consistency_json()
    parsed = json.loads(j)
    assert isinstance(parsed, dict)
    # Should have V1433 keys
    assert "rows" in parsed or "error" in parsed


def test_v1434_generate_markdown_summary_has_table():
    """markdown summary has Coverage table."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    md = m.generate_markdown_summary()
    assert "Coverage" in md
    assert "forward_coverage" in md
    assert "parity" in md
    assert "End-to-end" in md or "Endpoints" in md
    assert "Honest disclosure" in md


def test_v1434_build_all_artifacts_returns_five():
    """build_all_artifacts returns 5 artifacts."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    arts = m.build_all_artifacts()
    assert len(arts) == 5
    kinds = {a.kind for a in arts}
    assert "python_server_script" in kinds
    assert "nginx_snippet" in kinds
    assert "docker_compose_snippet" in kinds
    assert "consistency_json" in kinds
    assert "markdown_summary" in kinds


def test_v1434_render_artifacts_index_md_has_table():
    import apeireth.v1434_asi_vcp_consistency_http as m
    arts = m.build_all_artifacts()
    md = m.render_artifacts_index_md(arts)
    assert "V1434" in md
    assert "kind" in md
    assert "filename" in md
    assert "sha256" in md
    for a in arts:
        assert a.filename in md


def test_v1434_validate_python_rejects_invalid():
    """validate_python_script rejects broken python."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert m.validate_python_script("def x(:\n  pass") is False
    assert m.validate_python_script("not python at all") is False


def test_v1434_validate_nginx_rejects_bare_text():
    """validate_nginx_snippet rejects bare text."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert m.validate_nginx_snippet("just a paragraph") is False


def test_v1434_validate_compose_rejects_bare_text():
    """validate_compose_snippet rejects bare text."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert m.validate_compose_snippet("just a paragraph") is False


def test_v1434_chain_delegate_returns_three_modules():
    """chain_delegate probes 3 upstream modules."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    chain = m.chain_delegate()
    assert chain["n_modules"] == 3
    assert "V1433" in chain["chain"]
    assert "V1420" in chain["chain"]
    assert "V1431" in chain["chain"]


def test_v1434_chain_delegate_all_ok():
    """All 3 modules importable (V1433 + V1420 + V1431)."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    chain = m.chain_delegate()
    assert chain["all_ok"] is True


def test_v1434_module_meta_keys():
    import apeireth.v1434_asi_vcp_consistency_http as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["module"] == "v1434_asi_vcp_consistency_http"
    assert meta["schema"] == "v1434.asi-vcp-consistency-http/v1"
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5
    assert meta["n_borrowed"] == 5
    assert meta["n_endpoints"] == 2
    assert meta["n_artifacts"] == 5


def test_v1434_popper_self_test_all_pass():
    """Popper self-test: 14/14 must pass."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    result = m.popper_self_test()
    assert result["n_total"] == 14
    assert result["n_pass"] == 14, f"failed: {[k for k, v in result['results'].items() if not v[0]]}"
    assert result["ok"] is True


def test_v1434_now_iso_is_string():
    import apeireth.v1434_asi_vcp_consistency_http as m
    iso = m._now_iso()
    assert isinstance(iso, str)
    assert "T" in iso


def test_v1434_no_socket_bind_guard():
    """GUARD_NO_SOCKET_BIND is present (V1434 doesn't bind sockets)."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_NO_SOCKET_BIND" in m.V1434_GUARDS


def test_v1434_artifact_generated_guard():
    """GUARD_ARTIFACT_GENERATED is present."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_ARTIFACT_GENERATED" in m.V1434_GUARDS


def test_v1434_python_valid_guard():
    """GUARD_PYTHON_VALID is present."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_PYTHON_VALID" in m.V1434_GUARDS


def test_v1434_nginx_valid_guard():
    """GUARD_NGINX_VALID is present."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_NGINX_VALID" in m.V1434_GUARDS


def test_v1434_compose_valid_guard():
    """GUARD_COMPOSE_VALID is present."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_COMPOSE_VALID" in m.V1434_GUARDS


def test_v1434_v3_guards_no_phenomenal():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_NO_PHENOMENAL_ARTIFACT" in m.V1434_V3_GUARDS


def test_v1434_v3_guards_no_asi():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_NO_ASI_ARTIFACT" in m.V1434_V3_GUARDS


def test_v1434_v3_guards_no_v1433_replace():
    """GUARD_NO_V1433_REPLACE is present."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "GUARD_NO_V1433_REPLACE" in m.V1434_V3_GUARDS


def test_v1434_honest_disclosure_in_docstring():
    import apeireth.v1434_asi_vcp_consistency_http as m
    assert "Honest disclosure" in (m.__doc__ or "")


def test_v1434_cli_help_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["help"])
    assert rc == 0


def test_v1434_cli_version_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1434_cli_meta_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1434_cli_meta_json_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1434_cli_popper_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1434_cli_chain_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["chain"])
    assert rc == 0


def test_v1434_cli_python_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["python"])
    assert rc == 0


def test_v1434_cli_nginx_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["nginx"])
    assert rc == 0


def test_v1434_cli_compose_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["compose"])
    assert rc == 0


def test_v1434_cli_json_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["json"])
    assert rc == 0


def test_v1434_cli_summary_runs():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["summary"])
    assert rc == 0


def test_v1434_cli_all_writes_artifacts():
    """`all` command writes all 5 artifacts to a tempdir."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    with tempfile.TemporaryDirectory() as tmp:
        rc = m.main(["all", tmp])
        assert rc == 0
        # Files should exist
        assert (Path(tmp) / "serve_vcp_consistency.py").exists()
        assert (Path(tmp) / "v1434-nginx.conf").exists()
        assert (Path(tmp) / "docker-compose.v1434.yml").exists()
        assert (Path(tmp) / "v1433-consistency.json").exists()
        assert (Path(tmp) / "v1434-summary.md").exists()
        assert (Path(tmp) / "INDEX.md").exists()


def test_v1434_cli_unknown_command_returns_1():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main(["bogus_command"])
    assert rc == 1


def test_v1434_cli_no_args_returns_help():
    import apeireth.v1434_asi_vcp_consistency_http as m
    rc = m.main([])
    assert rc == 0


def test_v1434_no_external_deps_in_imports():
    """V1434 must use only stdlib."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    import inspect
    source = inspect.getsource(m)
    forbidden = ["import requests", "import httpx", "import fastapi", "import aiohttp", "import flask"]
    for f in forbidden:
        assert f not in source, f"forbidden import found: {f}"


def test_v1434_artifacts_have_unique_shas():
    """Each artifact has a unique sha256."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    arts = m.build_all_artifacts()
    shas = [a.sha256 for a in arts]
    assert len(set(shas)) == len(shas), "duplicate shas found"


def test_v1434_artifact_lines_count_correct():
    """Artifact lines count is correct."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    a = m.Artifact(kind="t", filename="t.txt", content="a\nb\nc\n")
    # 3 newlines → 3 lines
    assert a.lines == 3


def test_v1434_artifact_bytes_count_correct():
    """Artifact bytes count is correct."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    a = m.Artifact(kind="t", filename="t.txt", content="hello")
    assert a.bytes == 5


def test_v1434_artifact_sha256_deterministic():
    """Same content → same sha256."""
    import apeireth.v1434_asi_vcp_consistency_http as m
    a1 = m.Artifact(kind="t", filename="x", content="hello")
    a2 = m.Artifact(kind="t", filename="x", content="hello")
    assert a1.sha256 == a2.sha256


if __name__ == "__main__":
    import apeireth.v1434_asi_vcp_consistency_http as m
    sys.exit(m.main(["popper"]))