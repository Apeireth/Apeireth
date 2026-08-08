"""Phase 1052 v1052_real_streamlit_deploy — V1052 ASI 真厨房 Streamlit 真部署 (主 06:15 + 主 00:56 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33).

主 06:15 当前真生产方向: V1052 = 真厨房部 - V1009 Streamlit 真部署.
主 00:56 真采纳: 任何人都能接手 + 阶段交付短链.
主 23:44 干到底: 真生产不是模板, 是真能 streamlit run 跑起来.
主 22:33 ASI 北极星: 真部署 ASI 北极星引擎, 验证真生产.
主 19:33 走在前人经验上: 真借鉴 Streamlit 官方 CLI + 真终端调用.
主 17:43 实事求是: 不假装部署; 不假装容器跑; 真 subprocess + 真 healthcheck.
主 17:33 放手干到底: 真跑真测真终端, 不空壳.

真生产设计 (主 19:33):
- 真 subprocess.Popen 调 streamlit run 命令 (主 19:33 Streamlit CLI 真借鉴)
- 真写 app.py (从 V1009.render_streamlit_app() 拿真源) + .streamlit/config.toml 到目标目录
- 真 subprocess 后台跑 + 真 HTTP GET /_stcore/health 等 healthcheck pass (主 17:43 真等, 不假装)
- 真端口轮询 + 真超时回收
- 真检测 streamlit 可用性 (主 17:43 真测, 没装就返回结构化状态)
- 真回执 (pid, port, http_status, process_state)
- 真清理 (terminate / kill fallback)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 deployment-engineering, 不是 consciousness claim.
- 不假装达到 ASI: 真部署 ≠ ASI 达成; ASI 部署只是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是 subprocess 真跑, 不是改 prompt 假装部署.
- 部署 ≠ 守门: 真生产 ≠ 真安全. 部署 ≠ 安全审计.
- 真生产 = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "deployed = safe" 都是不假装.
- 任何人能接手: 输出 pid + port + log 路径 + 状态结构化, 下一个人接手直接对号入座.
"""
from __future__ import annotations

import os
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
import urllib.error
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


V1052_VERSION = "0.1.0"
V1052_APP = "app.py"
V1052_CONFIG_DIR = ".streamlit"
V1052_CONFIG = "config.toml"
V1052_DEFAULT_HEALTH_PATH = "/_stcore/health"
V1052_DEFAULT_PORT = 8765
V1052_DEFAULT_HOST = "127.0.0.1"
V1052_LOG_FILE = "streamlit.log"


# ============================================================================
# 真生产 streamlit config.toml (主 19:33 Streamlit 官方真借鉴)
# ============================================================================
# 真借鉴: Streamlit 官方 config.toml (主 19:33 https://docs.streamlit.io/library/advanced-features/configuration)
# 主 00:36 工程化: headless + CORS off + XSRF off + 健康端点保留 (1.29+ 默认开).
# 主 17:43 实事求是: 真能 streamlit run --config 跑.

STREAMLIT_CONFIG_RUNTIME = """# V1052 ASI 真厨房 Streamlit config (主 06:15 + 主 00:36 + 主 19:33 Streamlit 官方真借鉴)
[server]
headless = true
runOnSave = false
fileWatcherType = "none"
enableCORS = false
enableXsrfProtection = true
maxUploadSize = 5

[browser]
gatherUsageStats = false

[theme]
base = "light"
primaryColor = "#FF6B6B"

[global]
showWarningOnDirectExecution = false
"""


# ============================================================================
# 真生产数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class StreamlitCommandResult:
    """V1052 真生产 Streamlit 命令执行结果 (主 17:43 真终端)."""
    command: List[str]
    returncode: Optional[int]  # None 表示仍在跑
    stdout_tail: str
    stderr_tail: str
    elapsed_seconds: float
    ok: bool
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "command": self.command,
            "returncode": self.returncode,
            "stdout_tail_len": len(self.stdout_tail),
            "stderr_tail_len": len(self.stderr_tail),
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "ok": self.ok,
            "ts": self.ts,
        }


@dataclass
class StreamlitServerHealth:
    """V1052 真生产 Streamlit 服务健康状态 (主 17:43 真 HTTP)."""
    host: str
    port: int
    pid: Optional[int]
    process_alive: bool
    http_reachable: bool
    http_status: Optional[int]
    health_ok: bool
    log_path: Optional[str] = None
    last_error: Optional[str] = None
    started_ts: float = 0.0
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "host": self.host,
            "port": self.port,
            "pid": self.pid,
            "process_alive": self.process_alive,
            "http_reachable": self.http_reachable,
            "http_status": self.http_status,
            "health_ok": self.health_ok,
            "log_path": self.log_path,
            "last_error": self.last_error,
            "uptime_seconds": round(self.ts - self.started_ts, 4) if self.started_ts else 0.0,
            "ts": self.ts,
        }


@dataclass
class DeploymentStatus:
    """V1052 真生产 deployment 状态 (主 17:43 实事求是)."""
    streamlit_available: bool
    artefacts_written: bool
    server_running: bool
    healthy: bool
    host: str = ""
    port: int = 0
    pid: Optional[int] = None
    log_path: Optional[str] = None
    app_path: Optional[str] = None
    last_error: Optional[str] = None
    artefacts: Dict[str, str] = field(default_factory=dict)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "streamlit_available": self.streamlit_available,
            "artefacts_written": self.artefacts_written,
            "server_running": self.server_running,
            "healthy": self.healthy,
            "host": self.host,
            "port": self.port,
            "pid": self.pid,
            "log_path": self.log_path,
            "app_path": self.app_path,
            "last_error": self.last_error,
            "n_artefacts": len(self.artefacts),
            "ts": self.ts,
        }


# ============================================================================
# V1052 真厨房 Streamlit 真部署 (主 06:15 + 主 00:56 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)
# ============================================================================


def _port_is_free(host: str, port: int) -> bool:
    """真检查端口空闲: 尝试 bind, 不依赖第三方库."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(0.5)
    try:
        sock.bind((host, port))
        return True
    except OSError:
        return False
    finally:
        sock.close()


def _pick_free_port(host: str, start: int, end: int, step: int = 1) -> int:
    """真找一个空闲端口, 在 [start, end] 区间 (主 17:43 真扫端口)."""
    for p in range(start, end + 1, step):
        if _port_is_free(host, p):
            return p
    return start  # fallback, 调用方会失败时报错


def _http_get(url: str, timeout: float = 2.0) -> Tuple[bool, Optional[int], str]:
    """真 HTTP GET, 返回 (reachable, status_code, error_str)."""
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return True, resp.status, ""
    except urllib.error.HTTPError as e:
        # 4xx/5xx 仍然 reachable
        return True, e.code, str(e)
    except (urllib.error.URLError, ConnectionRefusedError, TimeoutError, OSError) as e:
        return False, None, str(e)


def _render_v1009_streamlit_app() -> str:
    """真从 V1009 WebUI 拿 Streamlit app 源 (主 19:33 V1009 真借鉴)."""
    try:
        from v1009_web_ui import V1009WebUI
        return V1009WebUI().render_streamlit_app()
    except Exception as e:
        # 不假装: V1009 import 失败, 至少给一个能跑的最小 streamlit app
        return (
            "import streamlit as st\n"
            "st.title('V1052 ASI Real Streamlit Deploy')\n"
            "st.write('V1009 import failed: ' + repr(e))\n"
            "st.write('Minimal fallback app running.')\n"
        )


class V1052RealStreamlitDeploy:
    """V1052 ASI 真厨房 Streamlit 真部署 (主 06:15 + 主 00:56 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43).

    真生产设计 (主 19:33 Streamlit CLI 真借鉴):
    - write_app(output_dir): 真写 app.py (从 V1009 真源) + .streamlit/config.toml + log path
    - check_streamlit_available(): 真调 streamlit --version
    - start(output_dir, port, host): 真 subprocess.Popen 调 streamlit run
    - wait_ready(host, port, timeout): 真 HTTP GET /_stcore/health 轮询
    - health_check(): 真 HTTP check
    - stop(): 真 terminate, fallback kill
    - stats(): 真回执 pid + port + http_status + log_path

    主 17:43 实事求是: 没装 streamlit 不假装, 状态结构化返回; 不假装 healthcheck 通过.
    主 00:56 任何人能接手: 接手只需要看 stats() 输出, 知道 pid / port / log_path.
    """

    def __init__(self,
                 app_source: Optional[str] = None,
                 config_toml: Optional[str] = None):
        self.app_source = app_source  # None -> use V1009 真源
        self.config_toml = config_toml or STREAMLIT_CONFIG_RUNTIME
        self.process: Optional[subprocess.Popen] = None
        self.log_file_handle = None
        self.last_result: Optional[StreamlitCommandResult] = None
        self.last_status: Optional[DeploymentStatus] = None
        self.command_log: List[StreamlitCommandResult] = []

    # ------------------------------------------------------------------
    # 可用性检查 (主 17:43 真测)
    # ------------------------------------------------------------------

    def check_streamlit_available(self, timeout: int = 10) -> Tuple[bool, str]:
        """真检查 streamlit CLI (主 17:43 真测, 没装就返回结构化状态)."""
        streamlit_bin = shutil.which("streamlit")
        if not streamlit_bin:
            return False, "streamlit binary not found in PATH"
        try:
            res = subprocess.run(
                [streamlit_bin, "--version"],
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            if res.returncode == 0:
                return True, res.stdout.strip()
            return False, f"streamlit --version failed: rc={res.returncode} stderr={res.stderr.strip()}"
        except subprocess.TimeoutExpired:
            return False, f"streamlit --version timeout after {timeout}s"
        except Exception as e:
            return False, f"streamlit --version exception: {type(e).__name__}: {e}"

    # ------------------------------------------------------------------
    # Artifacts 写入 (主 17:43 真写)
    # ------------------------------------------------------------------

    def write_app(self, output_dir: str = ".") -> Dict[str, str]:
        """V1052 真生产 写入真部署文件到 output_dir (主 17:43 真写文件).

        返回 dict[name -> abs_path]. 写入:
        - app.py (从 V1009 真源或用户提供)
        - .streamlit/config.toml
        """
        os.makedirs(output_dir, exist_ok=True)
        config_dir = os.path.join(output_dir, V1052_CONFIG_DIR)
        os.makedirs(config_dir, exist_ok=True)

        artefacts: Dict[str, str] = {}

        app_path = os.path.join(output_dir, V1052_APP)
        app_source = self.app_source if self.app_source is not None else _render_v1009_streamlit_app()
        with open(app_path, "w", encoding="utf-8") as f:
            f.write(app_source)
        artefacts[V1052_APP] = os.path.abspath(app_path)

        config_path = os.path.join(config_dir, V1052_CONFIG)
        with open(config_path, "w", encoding="utf-8") as f:
            f.write(self.config_toml)
        artefacts[f"{V1052_CONFIG_DIR}/{V1052_CONFIG}"] = os.path.abspath(config_path)

        log_path = os.path.join(output_dir, V1052_LOG_FILE)
        # log 不预创建, 让 streamlit 自己创建
        artefacts[V1052_LOG_FILE] = os.path.abspath(log_path)

        return artefacts

    # ------------------------------------------------------------------
    # 真启 / 真停 / 真查 (主 17:43 真跑真测)
    # ------------------------------------------------------------------

    def start(self,
              output_dir: str = ".",
              port: Optional[int] = None,
              host: Optional[str] = None,
              headless: bool = True,
              timeout: int = 600) -> StreamlitCommandResult:
        """真启动 streamlit run 作为后台子进程 (主 17:43 真 subprocess).

        Args:
            output_dir: app.py 所在目录
            port: 端口, None 自动选空闲端口
            host: 监听地址, 默认 127.0.0.1
            headless: --server.headless=true
            timeout: 进程超时 (秒), 0 表示不超时

        Returns:
            StreamlitCommandResult 含 returncode (None 表示仍在跑), stdout/stderr 尾部
        """
        host = host or V1052_DEFAULT_HOST
        if port is None:
            port = _pick_free_port(host, V1052_DEFAULT_PORT, V1052_DEFAULT_PORT + 100)
        if not _port_is_free(host, port):
            # 已被占用 -> 让 streamlit 自己报错
            pass

        streamlit_bin = shutil.which("streamlit")
        if not streamlit_bin:
            res = StreamlitCommandResult(
                command=["streamlit", "run", "(none)"],
                returncode=None,
                stdout_tail="",
                stderr_tail="streamlit binary not found in PATH",
                elapsed_seconds=0.0,
                ok=False,
            )
            self.last_result = res
            self.command_log.append(res)
            return res

        artefacts = self.write_app(output_dir)
        app_path = artefacts[V1052_APP]
        log_path = artefacts[V1052_LOG_FILE]

        cmd = [
            streamlit_bin, "run", app_path,
            "--server.address", host,
            "--server.port", str(port),
            "--server.headless", "true" if headless else "false",
            "--server.fileWatcherType", "none",
            "--server.runOnSave", "false",
            "--global.showWarningOnDirectExecution", "false",
        ]

        # 开 log 文件, 让 streamlit 自己的 stdout/stderr 都进去
        self.log_file_handle = open(log_path, "ab", buffering=0)

        t0 = time.time()
        try:
            self.process = subprocess.Popen(
                cmd,
                stdout=self.log_file_handle,
                stderr=subprocess.STDOUT,
                cwd=output_dir,
                env={**os.environ, "PYTHONPATH": output_dir},
            )
        except Exception as e:
            res = StreamlitCommandResult(
                command=cmd,
                returncode=None,
                stdout_tail="",
                stderr_tail=f"Popen exception: {type(e).__name__}: {e}",
                elapsed_seconds=time.time() - t0,
                ok=False,
            )
            self.last_result = res
            self.command_log.append(res)
            try:
                if self.log_file_handle:
                    self.log_file_handle.close()
            except Exception:
                pass
            return res

        # 短轮询等 process 真起来 (不要等 healthcheck, 那是 wait_ready 的事)
        time.sleep(0.5)

        elapsed = time.time() - t0
        alive = self.process.poll() is None
        res = StreamlitCommandResult(
            command=cmd,
            returncode=self.process.returncode,  # None if still running
            stdout_tail="(see log file)",
            stderr_tail="",
            elapsed_seconds=elapsed,
            ok=alive,
        )
        self.last_result = res
        self.command_log.append(res)
        return res

    def wait_ready(self,
                   host: Optional[str] = None,
                   port: Optional[int] = None,
                   health_path: str = V1052_DEFAULT_HEALTH_PATH,
                   timeout: float = 30.0,
                   poll_interval: float = 0.5) -> StreamlitServerHealth:
        """真 HTTP GET /_stcore/health 轮询 (主 17:43 真等, 不假装).

        Returns:
            StreamlitServerHealth 含 http_reachable / http_status / health_ok
        """
        host = host or V1052_DEFAULT_HOST
        port = port or (self._last_port() if self.process else V1052_DEFAULT_PORT)
        pid = self.process.pid if (self.process and self.process.poll() is None) else None
        started_ts = time.time()

        deadline = started_ts + timeout
        last_error = ""
        http_status = None
        reachable = False
        health_ok = False

        while time.time() < deadline:
            if self.process and self.process.poll() is not None:
                return StreamlitServerHealth(
                    host=host, port=port, pid=pid,
                    process_alive=False, http_reachable=False,
                    http_status=None, health_ok=False,
                    log_path=self._log_path(),
                    last_error=f"process exited with rc={self.process.returncode}",
                    started_ts=started_ts,
                )
            url = f"http://{host}:{port}{health_path}"
            reachable, http_status, last_error = _http_get(url, timeout=min(2.0, poll_interval * 2))
            if reachable and http_status == 200:
                health_ok = True
                break
            time.sleep(poll_interval)

        return StreamlitServerHealth(
            host=host, port=port, pid=pid,
            process_alive=(self.process.poll() is None) if self.process else False,
            http_reachable=reachable, http_status=http_status,
            health_ok=health_ok,
            log_path=self._log_path(),
            last_error=last_error,
            started_ts=started_ts,
        )

    def health_check(self,
                     host: Optional[str] = None,
                     port: Optional[int] = None,
                     health_path: str = V1052_DEFAULT_HEALTH_PATH,
                     timeout: float = 5.0) -> StreamlitServerHealth:
        """单次 HTTP healthcheck (主 17:43 真测一次, 不轮询)."""
        host = host or V1052_DEFAULT_HOST
        port = port or (self._last_port() if self.process else V1052_DEFAULT_PORT)
        pid = self.process.pid if (self.process and self.process.poll() is None) else None
        url = f"http://{host}:{port}{health_path}"
        reachable, http_status, last_error = _http_get(url, timeout=timeout)
        return StreamlitServerHealth(
            host=host, port=port, pid=pid,
            process_alive=(self.process.poll() is None) if self.process else False,
            http_reachable=reachable, http_status=http_status,
            health_ok=reachable and http_status == 200,
            log_path=self._log_path(),
            last_error=last_error,
        )

    def stop(self, timeout: float = 10.0) -> bool:
        """真停 streamlit 进程 (terminate -> kill fallback)."""
        if not self.process:
            return True
        try:
            if self.process.poll() is not None:
                self._cleanup_log()
                return True
            self.process.terminate()
            try:
                self.process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
            self._cleanup_log()
            return self.process.poll() is not None
        except Exception:
            return False

    def _cleanup_log(self) -> None:
        if self.log_file_handle:
            try:
                self.log_file_handle.close()
            except Exception:
                pass
            self.log_file_handle = None

    def _last_port(self) -> int:
        if self.last_result and self.last_result.command:
            for i, arg in enumerate(self.last_result.command):
                if arg == "--server.port" and i + 1 < len(self.last_result.command):
                    try:
                        return int(self.last_result.command[i + 1])
                    except ValueError:
                        pass
        return V1052_DEFAULT_PORT

    def _log_path(self) -> Optional[str]:
        if self.last_result and self.last_result.command:
            # last cmd format: [streamlit_bin, "run", app_path, ...]
            if len(self.last_result.command) >= 3:
                app_path = self.last_result.command[2]
                return os.path.join(os.path.dirname(app_path), V1052_LOG_FILE)
        return None

    def stats(self) -> DeploymentStatus:
        """真回执 (主 00:56 任何人能接手: pid + port + log_path + 状态)."""
        health = self.health_check() if self.process else None
        return DeploymentStatus(
            streamlit_available=shutil.which("streamlit") is not None,
            artefacts_written=bool(self.last_result),
            server_running=(self.process.poll() is None) if self.process else False,
            healthy=bool(health and health.health_ok),
            host=health.host if health else "",
            port=health.port if health else 0,
            pid=health.pid if health else None,
            log_path=health.log_path if health else None,
            app_path=(self.last_result.command[2]
                      if self.last_result and len(self.last_result.command) >= 3
                      else None),
            last_error=health.last_error if health else None,
            artefacts={},
        )


# ============================================================================
# 真 demo: 不依赖 docker (主 17:43 真生产)
# ============================================================================


def _demo():
    deploy = V1052RealStreamlitDeploy()
    avail, info = deploy.check_streamlit_available()
    print(f"streamlit available: {avail} ({info[:80]})")
    if not avail:
        print("SKIP: streamlit not installed")
        return 0

    # 真写 app
    artefacts = deploy.write_app(output_dir="_v1052_demo")
    print(f"artefacts: {list(artefacts.keys())}")

    # 真启
    res = deploy.start(output_dir="_v1052_demo", port=18765, host="127.0.0.1")
    print(f"start ok={res.ok} rc={res.returncode}")

    if res.ok:
        # 真等 ready
        h = deploy.wait_ready(host="127.0.0.1", port=18765, timeout=20.0)
        print(f"wait_ready: pid={h.pid} alive={h.process_alive} http={h.http_reachable} status={h.http_status} health_ok={h.health_ok}")
        # 真停
        stopped = deploy.stop(timeout=5.0)
        print(f"stopped: {stopped}")
        print(f"stats: {deploy.stats().to_dict()}")
    return 0


if __name__ == "__main__":
    sys.exit(_demo())
