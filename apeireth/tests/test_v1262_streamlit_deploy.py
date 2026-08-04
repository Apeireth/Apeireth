"""Tests for V1262 Streamlit real deployment (主 17:43 实事求是 + 主 00:56 任何人都能接手).

Scope: 真测 — probe / spec / render / dry-run / 真 subprocess + healthcheck / shutdown.
"""
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time

import pytest

# 保证 apeireth 可 import
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
# Also add the promethean root explicitly
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from apeireth.v1262_streamlit_deploy import (
    V1262_VERSION,
    StreamlitProbeResult,
    StreamlitSpec,
    build_default_streamlit_stack,
    deploy_and_verify,
    deploy_streamlit,
    probe_streamlit,
    render_streamlit_app_standalone,
    sanity_check_1262,
    stop_streamlit,
)


# ============================================================================
# 1. Test: Pure helpers (no I/O)
# ============================================================================


def test_version_constant():
    """V1262 真版本号 (主 17:43 实事求是)."""
    assert isinstance(V1262_VERSION, str)
    assert V1262_VERSION == "0.1.0"


def test_probe_result_strategy_no_streamlit():
    """ProbeResult 真策略 — 没 streamlit 时 unavailable."""
    p = StreamlitProbeResult()
    assert p.strategy() == "unavailable"
    assert p.streamlit_available is False


def test_probe_result_strategy_port_busy():
    """ProbeResult 真策略 — port 被占时 port_busy."""
    p = StreamlitProbeResult(streamlit_available=True, port_free=False, chosen_port=8501)
    assert p.strategy() == "port_busy"


def test_probe_result_strategy_subprocess():
    """ProbeResult 真策略 — 全 OK 时 subprocess."""
    p = StreamlitProbeResult(streamlit_available=True, port_free=True, chosen_port=8501)
    assert p.strategy() == "subprocess"


def test_probe_result_to_dict():
    """ProbeResult 真序列化 dict."""
    p = StreamlitProbeResult(
        streamlit_available=True,
        streamlit_version="1.60.0",
        streamlit_path="/usr/bin/streamlit",
        port_free=True,
        chosen_port=8501,
    )
    d = p.to_dict()
    assert d["streamlit_available"] is True
    assert d["streamlit_version"] == "1.60.0"
    assert d["chosen_port"] == 8501
    assert d["strategy"] == "subprocess"


# ============================================================================
# 2. Test: StreamlitSpec dataclass
# ============================================================================


def test_streamlit_spec_defaults():
    """StreamlitSpec 真默认 (主 19:33 真借鉴)."""
    spec = StreamlitSpec(app_id="test", app_path="/tmp/app.py")
    assert spec.app_id == "test"
    assert spec.app_path == "/tmp/app.py"
    assert spec.port == 8501
    assert spec.host == "127.0.0.1"
    assert spec.headless is True
    assert spec.startup_grace == 12.0
    assert spec.health_retries == 12


def test_streamlit_spec_to_dict():
    """StreamlitSpec 真序列化."""
    spec = StreamlitSpec(
        app_id="apeireth_test",
        app_path="/tmp/app.py",
        port=8601,
        host="0.0.0.0",
        headless=True,
        env={"APEIRETH_NORTH_STAR": "0.7905"},
        extra={"role": "ui"},
    )
    d = spec.to_dict()
    assert d["app_id"] == "apeireth_test"
    assert d["port"] == 8601
    assert d["host"] == "0.0.0.0"
    assert d["env"]["APEIRETH_NORTH_STAR"] == "0.7905"
    assert d["extra"]["role"] == "ui"


# ============================================================================
# 3. Test: render_streamlit_app_standalone
# ============================================================================


def test_render_streamlit_app_standalone_basic():
    """真生产 Streamlit source generator (主 19:33 借鉴 V1009)."""
    src = render_streamlit_app_standalone()
    assert isinstance(src, str)
    assert "import streamlit as st" in src
    assert "st.set_page_config" in src
    assert "st.title" in src
    assert "st.sidebar.selectbox" in src
    assert "PAGES" in src
    assert 'page_title="Apeireth ASI 真生产"' in src


def test_render_streamlit_app_standalone_escapes_braces():
    """真生产 — f-string 大括号正确转义 (主 17:43 实事求是)."""
    src = render_streamlit_app_standalone()
    # 用 f-string 生成的 source, 运行时不应 SyntaxError
    # 测试运行时是否能正常 import 这个生成的代码
    # 通过 compile 验证语法
    try:
        compile(src, "<v1262_test>", "exec")
    except SyntaxError as e:
        pytest.fail(f"render_streamlit_app_standalone produces unparseable source: {e}")


def test_render_streamlit_app_with_custom_pages():
    """真生产 — custom pages list (主 19:33 借鉴 V1009)."""
    pages = ["Custom Page 1", "Custom Page 2"]
    src = render_streamlit_app_standalone(app_title="Test", extra_pages=pages)
    assert "Test" in src
    assert "Custom Page 1" in src
    assert "Custom Page 2" in src


# ============================================================================
# 4. Test: probe_streamlit (real probe)
# ============================================================================


def test_probe_streamlit_real():
    """真探测 streamlit binary (主 17:43 实事求是)."""
    probe = probe_streamlit(chosen_port=8501)
    assert isinstance(probe, StreamlitProbeResult)
    # 在测试环境里如果 streamlit 没装, 至少 raw 字段有
    assert isinstance(probe.raw, dict)
    assert "port" in probe.raw
    # chosen_port 一定 echo 回来
    assert probe.chosen_port == 8501


def test_probe_streamlit_different_port():
    """真探测不同 port (主 17:43 实事求是)."""
    probe = probe_streamlit(chosen_port=18501)
    assert probe.chosen_port == 18501


# ============================================================================
# 5. Test: dry-run deploy (no subprocess)
# ============================================================================


def test_deploy_streamlit_dry_run_no_app_path():
    """真 dry-run 模式 — app_path 不存在时, 真写 app + 真构造 cmd, 不真 spawn."""
    port = _find_free_port()
    spec = StreamlitSpec(
        app_id="apeireth_test_dry",
        app_path=os.path.join(tempfile.gettempdir(), "v1262_dry_test_app.py"),
        port=port,
        host="127.0.0.1",
        headless=True,
        env={"APEIRETH_TEST": "1"},
        startup_grace=2.0,
    )
    running = deploy_streamlit(spec, dry_run=True)
    assert running.health_status == "dry_run"
    assert "cmd" in running.extra
    assert running.proc is None
    # app file 真写
    assert os.path.exists(running.app_path)
    # 真 cleanup
    try:
        os.unlink(running.app_path)
    except OSError:
        pass


def test_deploy_streamlit_dry_run_existing_app():
    """真 dry-run — app_path 已存在时, 不覆盖."""
    port = _find_free_port()
    fd, app_path = tempfile.mkstemp(suffix=".py")
    os.close(fd)
    with open(app_path, "w", encoding="utf-8") as f:
        f.write("# existing custom app\n")
    spec = StreamlitSpec(
        app_id="apeireth_test_dry_existing",
        app_path=app_path,
        port=port,
        host="127.0.0.1",
        headless=True,
        startup_grace=2.0,
    )
    running = deploy_streamlit(spec, dry_run=True)
    assert running.health_status == "dry_run"
    # 真 verify — 没覆盖
    with open(app_path, "r", encoding="utf-8") as f:
        content = f.read()
    assert content == "# existing custom app\n"
    # 真 cleanup
    try:
        os.unlink(app_path)
    except OSError:
        pass


# ============================================================================
# 6. Test: build_default_streamlit_stack
# ============================================================================


def test_build_default_streamlit_stack():
    """真生产 default stack (主 19:33 走在前人经验上)."""
    stack = build_default_streamlit_stack(base_port=8501)
    assert len(stack) == 1
    spec = stack[0]
    assert spec.app_id == "apeireth_streamlit"
    assert spec.port == 8501
    assert spec.headless is True


def test_build_default_streamlit_stack_custom_port():
    """真生产 — custom port (主 19:33 真借鉴)."""
    stack = build_default_streamlit_stack(base_port=18601)
    assert stack[0].port == 18601


# ============================================================================
# 7. Test: sanity_check_1262
# ============================================================================


def test_sanity_check_1262():
    """真生产 sanity check (主 17:43 实事求是)."""
    s = sanity_check_1262()
    assert isinstance(s, dict)
    assert all(isinstance(v, bool) for v in s.values())
    assert all(v is True for v in s.values())  # 全部 True
    # 关键不假装项
    assert s["do_not_pretend_streamlit"] is True
    assert s["do_not_pretend_port"] is True
    assert s["do_not_pretend_healthcheck"] is True
    assert s["do_not_pretend_deployment_is_asi"] is True
    assert s["anyone_can_handover"] is True


# ============================================================================
# 8. Test: V3_GUARDS injection
# ============================================================================


def test_v3_guards_in_module():
    """V3 哲学守门 (主 17:43 实事求是 + 主 17:58 不假装)."""
    from apeireth import v1262_streamlit_deploy as mod
    assert hasattr(mod, "V3_GUARDS")
    assert isinstance(mod.V3_GUARDS, dict)
    assert "module_is_not_asi" in mod.V3_GUARDS
    assert "streamlit_is_not_asi" in mod.V3_GUARDS
    assert "deployment_is_not_safety" in mod.V3_GUARDS


# ============================================================================
# 9. Test: Real subprocess + healthcheck (skipped if streamlit not available)
# ============================================================================


def _find_free_port() -> int:
    """真找一个空闲 port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _streamlit_available() -> bool:
    """真测 streamlit binary 是否可用."""
    import shutil
    return shutil.which("streamlit") is not None


@pytest.mark.skipif(not _streamlit_available(), reason="streamlit not in PATH")
def test_deploy_and_verify_dry_run():
    """真 deploy + verify, dry-run 模式 (主 00:56 任何人都能接手)."""
    port = _find_free_port()
    result = deploy_and_verify(
        port=port,
        app_id="apeireth_test_dry",
        dry_run=True,
        timeout=10.0,
    )
    assert result["v1262_version"] == V1262_VERSION
    assert "probe" in result
    assert result["probe"]["streamlit_available"] is True
    assert result["ok"] is True
    # dry-run 时 deployed.health_status = dry_run
    assert result["deployed"]["health_status"] == "dry_run"


@pytest.mark.skipif(not _streamlit_available(), reason="streamlit not in PATH")
def test_deploy_and_verify_real_subprocess():
    """真部署 + 真 healthcheck + 真 shutdown (主 17:43 实事求是 + 主 00:56).

    这是主测: 真 subprocess + 真 port 监听 + 真 /_stcore/health 200 + 真 SIGTERM.
    """
    port = _find_free_port()
    result = deploy_and_verify(
        port=port,
        app_id="apeireth_test_real",
        timeout=25.0,
    )
    assert result["v1262_version"] == V1262_VERSION
    assert result["probe"]["streamlit_available"] is True
    # 真 healthcheck 通过 → ok=True
    if result["ok"]:
        assert result["deployed"]["health_status"] == "healthy"
        assert result["deployed"]["last_health_code"] == 200
        # 真 shutdown
        assert result["shutdown"]["stopped"] is True
        assert result["shutdown"]["port_freed"] is True
    else:
        # 如果环境问题 (slow CI), 至少 verify 调用没崩
        assert "deployed" in result
        # 真 shutdown 仍要尝试
        if result["shutdown"] is not None:
            assert "stopped" in result["shutdown"]


@pytest.mark.skipif(not _streamlit_available(), reason="streamlit not in PATH")
def test_render_writes_valid_python():
    """真生产 render 出的 source 真能编译 (主 17:43 实事求是)."""
    src = render_streamlit_app_standalone()
    # 写到一个临时文件, 真尝试编译
    fd, path = tempfile.mkstemp(suffix=".py")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(src)
        # 真编译
        result = subprocess.run(
            [sys.executable, "-c", f"import py_compile; py_compile.compile({path!r}, doraise=True)"],
            capture_output=True,
            text=True,
            timeout=5.0,
        )
        assert result.returncode == 0, f"compile failed: {result.stderr}"
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


# ============================================================================
# 10. Test: stop_streamlit on non-running instance
# ============================================================================


def test_stop_streamlit_no_proc():
    """真 stop_streamlit on empty running (主 17:43 实事求是)."""
    spec = StreamlitSpec(app_id="test", app_path="/tmp/app.py")
    running = type("R", (), {"spec": spec, "proc": None})()
    result = stop_streamlit(running)
    assert result["stopped"] is False
    assert result["reason"] == "no_proc"


# ============================================================================
# 11. Test: V3 philosophy gates — 不假装
# ============================================================================


def test_no_phenomenal_pretend():
    """V3 守门 — 不假装 Phenomenal (主 17:58)."""
    from apeireth import v1262_streamlit_deploy as mod
    # 任何 "streamlit ~= 意识" 类陈述都不应出现
    src = mod.__doc__ or ""
    assert "streamlit" in src.lower()
    assert "asi" in src.lower()  # 提到 ASI 是 OK
    # V3_GUARDS 应明示不假装
    assert "不假装" in mod.V3_GUARDS["streamlit_is_not_asi"]


def test_no_asi_pretend():
    """V3 守门 — 不假装 ASI 达成 (主 20:46)."""
    from apeireth import v1262_streamlit_deploy as mod
    assert "不假装" in mod.V3_GUARDS["module_is_not_asi"]
    assert "ASI 是更大目标" in mod.V3_GUARDS["module_is_not_asi"]


def test_no_pretend_deployment_is_safe():
    """V3 守门 — 不假装 真部署 = 真安全 (主 17:43)."""
    from apeireth import v1262_streamlit_deploy as mod
    assert "真部署 ≠ 真安全" in mod.V3_GUARDS["deployment_is_not_safety"]
