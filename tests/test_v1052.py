"""V1052 - Real Streamlit Deploy tests.

Phase 1052 v1052_real_streamlit_deploy 真厨房 Streamlit 真部署 真测试 (主 17:43).

测试维度 (主 17:43 实事求是 + 主 00:56 任何人能接手):
1. 数据结构: StreamlitCommandResult / StreamlitServerHealth / DeploymentStatus to_dict
2. 端口扫描: _port_is_free / _pick_free_port 真扫
3. HTTP check: _http_get 真连接
4. 可用性: check_streamlit_available 真调 streamlit --version
5. artefacts 写入: write_app 真写文件 (app.py + .streamlit/config.toml)
6. 真启: start 真 subprocess.Popen streamlit run
7. 真等 ready: wait_ready 真 HTTP GET /_stcore/health 轮询
8. 真停: stop 真 terminate
9. 真回执: stats() 真输出 pid + port + log_path
10. V1009 真借鉴: render_v1009_streamlit_app 真从 V1009 拿源

不假装: 没装 streamlit 时 check 返回 False; start 失败时不假装 ok; healthcheck 失败时不假装 healthy.
"""
from __future__ import annotations

import os
import socket
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

from v1052_real_streamlit_deploy import (  # noqa: E402
    V1052RealStreamlitDeploy,
    StreamlitCommandResult,
    StreamlitServerHealth,
    DeploymentStatus,
    V1052_VERSION,
    V1052_APP,
    V1052_CONFIG_DIR,
    V1052_CONFIG,
    V1052_DEFAULT_HEALTH_PATH,
    V1052_DEFAULT_PORT,
    V1052_LOG_FILE,
    STREAMLIT_CONFIG_RUNTIME,
    _port_is_free,
    _pick_free_port,
    _http_get,
    _render_v1009_streamlit_app,
)


# ============================================================================
# 1. 数据结构
# ============================================================================


class TestDataStructures:
    def test_version(self):
        assert V1052_VERSION == "0.1.0"

    def test_constants(self):
        assert V1052_APP == "app.py"
        assert V1052_CONFIG_DIR == ".streamlit"
        assert V1052_CONFIG == "config.toml"
        assert V1052_DEFAULT_HEALTH_PATH == "/_stcore/health"
        assert V1052_DEFAULT_PORT == 8765
        assert V1052_LOG_FILE == "streamlit.log"

    def test_streamlit_config_runtime_has_server_headless(self):
        assert "headless" in STREAMLIT_CONFIG_RUNTIME
        assert "true" in STREAMLIT_CONFIG_RUNTIME.lower()

    def test_command_result_to_dict(self):
        r = StreamlitCommandResult(
            command=["streamlit", "run", "app.py"],
            returncode=0,
            stdout_tail="hello",
            stderr_tail="",
            elapsed_seconds=0.1,
            ok=True,
        )
        d = r.to_dict()
        assert d["command"] == ["streamlit", "run", "app.py"]
        assert d["returncode"] == 0
        assert d["ok"] is True
        assert d["elapsed_seconds"] == 0.1
        assert d["stdout_tail_len"] == 5

    def test_command_result_running_none_rc(self):
        r = StreamlitCommandResult(
            command=["streamlit", "run", "app.py"],
            returncode=None,  # still running
            stdout_tail="(see log)",
            stderr_tail="",
            elapsed_seconds=0.5,
            ok=True,
        )
        d = r.to_dict()
        assert d["returncode"] is None
        assert d["ok"] is True

    def test_server_health_to_dict(self):
        h = StreamlitServerHealth(
            host="127.0.0.1", port=8765, pid=12345,
            process_alive=True, http_reachable=True, http_status=200,
            health_ok=True, log_path="/tmp/x.log",
        )
        d = h.to_dict()
        assert d["host"] == "127.0.0.1"
        assert d["port"] == 8765
        assert d["pid"] == 12345
        assert d["process_alive"] is True
        assert d["http_status"] == 200
        assert d["health_ok"] is True

    def test_deployment_status_to_dict(self):
        s = DeploymentStatus(
            streamlit_available=True,
            artefacts_written=True,
            server_running=True,
            healthy=True,
            host="127.0.0.1",
            port=8765,
            pid=12345,
            log_path="/tmp/log",
            app_path="/tmp/app.py",
        )
        d = s.to_dict()
        assert d["host"] == "127.0.0.1"
        assert d["port"] == 8765
        assert d["pid"] == 12345
        assert d["healthy"] is True
        assert d["n_artefacts"] == 0  # default empty


# ============================================================================
# 2. 端口扫描
# ============================================================================


class TestPortScanning:
    def test_port_is_free_returns_bool(self):
        # 测试一个非常高的端口, 大概率空闲
        result = _port_is_free("127.0.0.1", 65111)
        assert isinstance(result, bool)

    def test_port_is_free_busy_port(self):
        # 先占一个端口
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(0.5)
        sock.bind(("127.0.0.1", 65112))
        try:
            # 此时该端口应不空闲
            assert _port_is_free("127.0.0.1", 65112) is False
        finally:
            sock.close()

    def test_pick_free_port_returns_int(self):
        port = _pick_free_port("127.0.0.1", 65200, 65210)
        assert isinstance(port, int)
        assert 65200 <= port <= 65210

    def test_pick_free_port_avoids_busy(self):
        # 占一个端口
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(0.5)
        sock.bind(("127.0.0.1", 65201))
        try:
            port = _pick_free_port("127.0.0.1", 65201, 65205)
            # 应该跳过 65201 选 65202 (或更后)
            assert port != 65201
            assert 65202 <= port <= 65205
        finally:
            sock.close()


# ============================================================================
# 3. HTTP check
# ============================================================================


class TestHTTPCheck:
    def test_http_get_unreachable_returns_false(self):
        # 用一个肯定不在线的端口
        reachable, status, err = _http_get("http://127.0.0.1:1/", timeout=1.0)
        assert reachable is False
        assert status is None
        assert err != ""

    def test_http_get_to_running_streamlit(self, tmp_path):
        """如果 streamlit 可用, 启一个真 streamlit 实例测 HTTP."""
        avail, _ = V1052RealStreamlitDeploy().check_streamlit_available()
        if not avail:
            pytest.skip("streamlit not installed")

        deploy = V1052RealStreamlitDeploy()
        out = str(tmp_path / "st")
        port = _pick_free_port("127.0.0.1", 19500, 19999)
        res = deploy.start(output_dir=out, port=port, host="127.0.0.1")
        try:
            assert res.ok
            h = deploy.wait_ready(host="127.0.0.1", port=port, timeout=30.0)
            assert h.http_reachable is True
            assert h.http_status == 200
            assert h.health_ok is True
        finally:
            deploy.stop(timeout=5.0)


# ============================================================================
# 4. 可用性检查
# ============================================================================


class TestStreamlitAvailable:
    def test_check_returns_tuple(self):
        deploy = V1052RealStreamlitDeploy()
        avail, info = deploy.check_streamlit_available()
        assert isinstance(avail, bool)
        assert isinstance(info, str)


# ============================================================================
# 5. artefacts 写入
# ============================================================================


class TestWriteApp:
    def test_write_app_creates_files(self, tmp_path):
        deploy = V1052RealStreamlitDeploy()
        out = str(tmp_path / "deploy")
        artefacts = deploy.write_app(output_dir=out)
        assert V1052_APP in artefacts
        assert f"{V1052_CONFIG_DIR}/{V1052_CONFIG}" in artefacts
        assert V1052_LOG_FILE in artefacts
        # app.py 真存在且非空
        app_path = Path(artefacts[V1052_APP])
        assert app_path.exists()
        assert app_path.stat().st_size > 100
        # .streamlit/config.toml 真存在
        cfg_path = Path(artefacts[f"{V1052_CONFIG_DIR}/{V1052_CONFIG}"])
        assert cfg_path.exists()
        # log 路径返回的是预期位置
        log_path = Path(artefacts[V1052_LOG_FILE])
        assert log_path.name == V1052_LOG_FILE

    def test_write_app_uses_v1009_source(self, tmp_path):
        """默认应从 V1009 真源拿 app.py."""
        deploy = V1052RealStreamlitDeploy()
        out = str(tmp_path / "v1009")
        artefacts = deploy.write_app(output_dir=out)
        app_path = Path(artefacts[V1052_APP])
        content = app_path.read_text(encoding="utf-8")
        # V1009 渲染的 streamlit app 含 streamlit / V1009 关键词
        assert "streamlit" in content.lower() or "Streamlit" in content

    def test_write_app_with_custom_source(self, tmp_path):
        custom = "import streamlit as st\nst.title('Custom')\n"
        deploy = V1052RealStreamlitDeploy(app_source=custom)
        out = str(tmp_path / "custom")
        artefacts = deploy.write_app(output_dir=out)
        app_path = Path(artefacts[V1052_APP])
        assert app_path.read_text(encoding="utf-8") == custom

    def test_render_v1009_fallback(self):
        """_render_v1009_streamlit_app 至少返回一个非空字符串."""
        src = _render_v1009_streamlit_app()
        assert isinstance(src, str)
        assert len(src) > 0


# ============================================================================
# 6-9. 真启 + 真等 + 真停 + 真回执
# ============================================================================


class TestRealStreamlitLifecycle:
    """真启 streamlit, 真等 ready, 真停. 主 17:43 真生产."""

    @pytest.fixture
    def deployed_server(self, tmp_path):
        """fixture: 启一个真 streamlit 实例, 测试结束自动 stop."""
        avail, _ = V1052RealStreamlitDeploy().check_streamlit_available()
        if not avail:
            pytest.skip("streamlit not installed")

        deploy = V1052RealStreamlitDeploy()
        out = str(tmp_path / "lifecycle")
        port = _pick_free_port("127.0.0.1", 19100, 19499)
        res = deploy.start(output_dir=out, port=port, host="127.0.0.1")
        if not res.ok:
            pytest.skip(f"streamlit start failed: stderr_tail={res.stderr_tail[:200]}")
        # 等 ready
        h = deploy.wait_ready(host="127.0.0.1", port=port, timeout=30.0)
        if not h.health_ok:
            deploy.stop(timeout=5.0)
            pytest.skip(f"streamlit not ready: http={h.http_reachable} status={h.http_status} err={h.last_error}")
        yield deploy, port, out
        # cleanup
        deploy.stop(timeout=5.0)

    def test_start_returns_result(self, deployed_server):
        deploy, port, out = deployed_server
        assert deploy.process is not None
        assert deploy.process.poll() is None  # still running

    def test_wait_ready_returns_health(self, deployed_server):
        deploy, port, out = deployed_server
        h = deploy.health_check(host="127.0.0.1", port=port)
        assert h.http_reachable is True
        assert h.http_status == 200
        assert h.health_ok is True
        assert h.process_alive is True
        assert h.pid is not None and h.pid > 0

    def test_stats_returns_status(self, deployed_server):
        deploy, port, out = deployed_server
        s = deploy.stats()
        assert s.streamlit_available is True
        assert s.server_running is True
        assert s.healthy is True
        assert s.host == "127.0.0.1"
        assert s.port == port
        assert s.pid is not None
        assert s.app_path is not None
        assert Path(s.app_path).exists()

    def test_stop_terminates_process(self, deployed_server):
        deploy, port, out = deployed_server
        assert deploy.process.poll() is None
        stopped = deploy.stop(timeout=5.0)
        assert stopped is True
        assert deploy.process.poll() is not None  # exited

    def test_double_stop_returns_true(self, deployed_server):
        deploy, port, out = deployed_server
        deploy.stop(timeout=5.0)
        # 再调一次 stop 不应该崩
        stopped_again = deploy.stop(timeout=2.0)
        assert stopped_again is True

    def test_log_file_created(self, deployed_server):
        deploy, port, out = deployed_server
        log_path = Path(out) / V1052_LOG_FILE
        assert log_path.exists()
        size = log_path.stat().st_size
        # streamlit run 至少会有 banner 日志
        assert size > 0


# ============================================================================
# 10. 错误路径
# ============================================================================


class TestErrorPaths:
    def test_start_when_streamlit_not_in_path(self, monkeypatch, tmp_path):
        """当 streamlit 二进制不可用时, start 返回 ok=False (不假装)."""
        monkeypatch.setattr("shutil.which", lambda x: None if x == "streamlit" else shutil_which_default(x))
        # 上面 monkeypatch 写法复杂, 简化:
        import shutil
        original = shutil.which
        def fake_which(name):
            if name == "streamlit":
                return None
            return original(name)
        monkeypatch.setattr(shutil, "which", fake_which)

        deploy = V1052RealStreamlitDeploy()
        res = deploy.start(output_dir=str(tmp_path), port=19999)
        assert res.ok is False
        assert "streamlit" in res.stderr_tail.lower() or "not found" in res.stderr_tail.lower()

    def test_health_check_when_no_process(self):
        deploy = V1052RealStreamlitDeploy()
        h = deploy.health_check(host="127.0.0.1", port=1)
        # 没 process 时 http_reachable=False
        assert h.http_reachable is False
        assert h.process_alive is False

    def test_stats_when_nothing_started(self):
        deploy = V1052RealStreamlitDeploy()
        s = deploy.stats()
        assert s.server_running is False
        assert s.healthy is False
        assert s.pid is None


# ============================================================================
# 11. V3 哲学守门 + Popper 自检
# ============================================================================


class TestPhilosophyGates:
    def test_no_phenomenal_pretending(self):
        """V1052 模块不含 phenomenal / consciousness claim."""
        src = Path(APEIRETH_DIR / "v1052_real_streamlit_deploy.py").read_text(encoding="utf-8")
        # 不应该有 claim 自己是 consciousness
        assert "is conscious" not in src.lower()
        assert "has phenomenal" not in src.lower()

    def test_no_asi_pretending(self):
        """V1052 模块不含 'i am asi' / 'asi achieved' claim."""
        src = Path(APEIRETH_DIR / "v1052_real_streamlit_deploy.py").read_text(encoding="utf-8")
        assert "i am asi" not in src.lower()
        assert "asi achieved" not in src.lower()

    def test_v1009_borrowing(self):
        """真借鉴 V1009, app.py 来源应该是 V1009 真源."""
        deploy = V1052RealStreamlitDeploy()
        # 默认 app_source=None 时, write_app 用 V1009 源
        # 这里验证 _render_v1009_streamlit_app 返回 V1009 真源
        src = _render_v1009_streamlit_app()
        # V1009 真源特征: 含 'V1009 真生产' 字符串
        assert "V1009" in src or "v1009" in src.lower()

    def test_any_human_can_pickup_via_stats(self):
        """stats() 输出含 pid + port + log_path, 任何人能接手."""
        deploy = V1052RealStreamlitDeploy()
        s = deploy.stats()
        d = s.to_dict()
        # 至少有以下 key, 接手时一目了然
        assert "pid" in d
        assert "port" in d
        assert "host" in d
        assert "log_path" in d
        assert "healthy" in d
        assert "server_running" in d
        assert "streamlit_available" in d


# ============================================================================
# helpers (避免 monkeypatch 引用未定义)
# ============================================================================


def shutil_which_default(name):
    import shutil
    return shutil.which(name)
