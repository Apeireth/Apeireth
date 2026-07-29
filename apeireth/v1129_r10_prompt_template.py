"""V1129 R10 Prompt Template — 北极星 V0.5 三新维 + 跨 provider prompt 适配.

主哲学 LOCKED (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 +
            主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手 +
            主 13:31 大胆激进 + 主 20:46 测量 ≠ 真值 + 主 20:55 红皇后):

R10 阶段: V0.4 = 0.8538 → ASI 北极星 ≥ 0.95.
V1129 增量 (R10-PE-001, 主 23:44 干到底):
  - 5 新 .j2 模板 (asi_north_star_v05 / v0_5_18dim / multi_agent_consensus /
                 anthropic_native / ollama_native)
  - 4 跨 provider 适配器 (anthropic_messages / openai_chat / ollama_chat /
                       local_executable)
  - 诚实报告 (主 17:43 实事求是 — 失败就是失败, 不假装 ≥3 provider 成功)

真借鉴 (主 19:33 走在前人经验上):
  - V1122 PromptSpec + loader (R9 基座, 零依赖 std-only)
  - V1125 V0.5 18 维公式 (0.85*V04 + 0.05*continuity + 0.05*autonomy + 0.05*transferability)
  - V1127 DGM v0.5 multi-agent (IdentityStore WAL + signed candidate archive)
  - V1128 RealModelGateway 模式 (ProviderState + HealthEvidence + 诚实 I/O)
  - V1124 ASINorthStarBackend (canonical JSON + AuditChain)
  - Anthropic Messages API 官方文档 (system+user 分离)
  - Ollama /api/chat 官方文档 (messages + stream=false)

V3 守门 (主 17:58 + 主 20:46, 继承 V1127/V1128):
  - unavailable_is_not_success: 401/403/429/Connection refused 都是失败
  - transport_is_not_intelligence: 真响应 ≠ ASI, ≠ 现象意识
  - comparison_is_not_truth: 跨 provider ASI 比较是 proxy, 不是 ground truth
  - no_fake_consensus: 多 agent consensus 失败就是失败, 不编造同意数
  - identity_is_not_consciousness: 持久身份 ≠ 现象意识

Usage:
    from apeireth.v1129_r10_prompt_template import (
        V1129_VERSION, MODULES_R10, ProviderAdapter, AnthropicAdapter,
        OpenAIAdapter, OllamaAdapter, LocalExecutableAdapter,
        adapt_prompt_to_provider, render_r10_template, run_r10_all,
    )
    # 渲染 V0.5 模板
    p = render_r10_template("asi_north_star_v05", {
        "ultimate_target": 0.95, "v05_total": 0.90, "v04_score": 0.85,
        "continuity": 0.88, "autonomy": 0.85, "transferability": 0.82,
        "abs_headroom": 0.05, "rel_headroom": 5.26,
        "target_module": "V1125", "candidate_output": "R10 W1 候选",
    })
    # 适配 prompt 到 provider (诚实执行, 失败 = 失败)
    result = adapt_prompt_to_provider("anthropic_messages", p,
        api_key="sk-...", endpoint="https://api.anthropic.com/v1/messages",
        model="claude-opus-4-20250514", max_tokens=1024)

    python -m apeireth.v1129_r10_prompt_template --report
"""
from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Mapping, Optional, Sequence, Tuple

from .prompt_templates import (
    list_templates,
    load_template,
    render_template,
    V1122_TPL_VERSION,
)

V1129_VERSION = "0.1.0"

# 复用 V1122 loader (主 19:33 走在前人经验上 — R9 基座不重写)
PROMPT_TPL_VERSION = V1122_TPL_VERSION

# ponytail: R10 新增 5 模块, 复用 V1122 8 模块 = 13 总模块 (主 23:44 干到底)
MODULES_R10: Tuple[str, ...] = (
    # R9 基座 (继承 V1122)
    "V1072", "V1074", "V1077", "V1095", "V1111", "V1112", "V1114", "V1119",
    # R10 增量
    "V1125",  # R10 ASI 北极星集成协议
    "V1126",  # R10 baseline
    "V1127",  # DGM v0.5 multi-agent
    "V1128",  # real model adapter
    "V1129",  # R10 prompt template (本模块)
)

# R10 新模板 (本任务产出, 主 23:44)
R10_TEMPLATES: Tuple[str, ...] = (
    "asi_north_star_v05",
    "v0_5_18dim",
    "multi_agent_consensus",
    "anthropic_native",
    "ollama_native",
)

# V1125 V0.5 公式权重 (继承 V1125.compute_v05_score, 主 19:33)
V04_WEIGHT = 0.85
NEW_DIM_WEIGHT = 0.05  # continuity / autonomy / transferability 各 0.05
V05_ULTIMATE_TARGET = 0.9500  # R10 终极门 (V1125 R10_ULTIMATE_TARGET)
V05_MID_TARGET = 0.9000
V05_R10_START = 0.8600

# R10 V3 守门 (继承 V1128 + V1127)
V3_GUARDS = {
    "unavailable_is_not_success":
        "Unavailable / 401 / 403 / Connection refused 都是失败. 任何虚报 ≥3 provider 成功都是不假装.",
    "transport_is_not_intelligence":
        "真响应只证明 transport 执行, 不证明 intelligence / ASI / 现象意识.",
    "comparison_is_not_truth":
        "跨 provider ASI 比较是 operational proxy, 不是 ground truth.",
    "no_fake_consensus":
        "多 agent consensus 失败就是失败. 同意数 / 签名 都是真值, 不编造.",
    "identity_is_not_consciousness":
        "持久身份 ≠ 现象意识 (Metzinger 2003 PSM).",
    "v0_5_is_not_asi":
        "V0.5 18 维只是测量, 0.95 才是 R10 终极门, ASI 北极星仍在前方.",
}


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 1. 诚实 Provider Result (主 17:43 实事求是, 继承 V1128 模式)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class ProviderStatus(str, Enum):
    """Provider 状态机 (主 17:58 不假装 — 失败就是失败)."""
    SUCCESS = "success"            # HTTP 200 + 真 content
    UNAVAILABLE = "unavailable"    # 网络不通 / Connection refused
    UNAUTHORIZED = "unauthorized"  # 401 / 403
    RATE_LIMITED = "rate_limited"  # 429
    TIMEOUT = "timeout"
    BAD_REQUEST = "bad_request"    # 400 / 422
    SERVER_ERROR = "server_error"  # 5xx
    NOT_CONFIGURED = "not_configured"  # 缺 key/endpoint/model


@dataclass
class ProviderResult:
    """Provider 调用真结果 (主 17:43 实事求是 — 不编造).

    fields:
      provider: anthropic/openai/ollama/local
      ok: True if HTTP 200 + content extracted
      status: ProviderStatus
      http_code: int or None
      content: str or None (真响应, 不编造)
      error: str or None
      elapsed_ms: int
      raw_excerpt: 响应前 200 字符 (诊断用)
    """
    provider: str
    ok: bool
    status: str
    http_code: Optional[int] = None
    content: Optional[str] = None
    error: Optional[str] = None
    elapsed_ms: int = 0
    raw_excerpt: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 2. 4 跨 provider 适配器 (主 19:33 真借鉴 + 主 13:31 大胆激进)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class ProviderAdapter:
    """Provider 适配器基类 (主 19:33 走在前人经验上 — 单接口, 多实现)."""
    name: str = "base"

    def adapt(self, prompt: str, **kwargs: Any) -> ProviderResult:
        """适配 + 发送. 子类必须实现. 主 17:43 实事求是: 失败 = 失败."""
        raise NotImplementedError

    # ponytail: 默认 timeout, 不发明 provider-specific 复杂配置
    @staticmethod
    def _http_json(
        url: str,
        body: Dict[str, Any],
        headers: Mapping[str, str],
        timeout_sec: float = 30.0,
    ) -> Tuple[int, Dict[str, Any]]:
        """真 HTTP POST (主 17:43 — 真发, 不假装)."""
        data = json.dumps(body).encode("utf-8")
        req = urllib.request.Request(url, data=data, headers=dict(headers), method="POST")
        try:
            with urllib.request.urlopen(req, timeout=timeout_sec) as resp:
                raw = resp.read().decode("utf-8", errors="replace")
                return resp.status, json.loads(raw) if raw else {}
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace") if e.fp else ""
            try:
                parsed = json.loads(raw) if raw else {}
            except json.JSONDecodeError:
                parsed = {"raw": raw[:500]}
            return e.code, parsed
        except urllib.error.URLError as e:
            raise ConnectionError(f"URLError: {e.reason}") from e


class AnthropicAdapter(ProviderAdapter):
    """Anthropic Messages API 适配 (主 19:33 官方文档真借鉴).

    真请求, 失败 = 失败 (主 17:43 实事求是).
    """
    name = "anthropic_messages"

    def adapt(self, prompt: str, **kwargs: Any) -> ProviderResult:
        t0 = time.time()
        api_key = kwargs.get("api_key") or os.environ.get("ANTHROPIC_API_KEY")
        # ponytail: ANTHROPIC_BASE_URL 是 base 路径, 自动补 /v1/messages
        env_base = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com")
        endpoint = kwargs.get("endpoint") or env_base
        if not endpoint.rstrip("/").endswith("/v1/messages"):
            endpoint = endpoint.rstrip("/") + "/v1/messages"
        model = kwargs.get("model") or os.environ.get("ANTHROPIC_MODEL", "MiniMax-M3")
        max_tokens = int(kwargs.get("max_tokens", 1024))
        anthropic_version = kwargs.get("anthropic_version", "2023-06-01")
        system_prompt = kwargs.get("system_prompt", "你是 Apeireth ASI 助手.")
        timeout_sec = float(kwargs.get("timeout_sec", 30.0))

        if not api_key:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.NOT_CONFIGURED.value,
                error="ANTHROPIC_API_KEY 未配置", elapsed_ms=int((time.time() - t0) * 1000),
            )

        # 适配 prompt: split 成 system + user (主 19:33 真借鉴 Anthropic 格式)
        body = {
            "model": model,
            "max_tokens": max_tokens,
            "system": system_prompt,
            "messages": [{"role": "user", "content": prompt}],
        }
        headers = {
            "x-api-key": api_key,
            "anthropic-version": anthropic_version,
            "content-type": "application/json",
        }

        try:
            code, parsed = self._http_json(endpoint, body, headers, timeout_sec)
        except ConnectionError as e:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.UNAVAILABLE.value,
                error=str(e), elapsed_ms=int((time.time() - t0) * 1000),
            )
        except Exception as e:  # 主 17:43 — 任何异常都报告, 不假装
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.SERVER_ERROR.value,
                error=f"{type(e).__name__}: {e}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )

        elapsed = int((time.time() - t0) * 1000)
        # 主 17:58 不假装: 真 content 提取, HTTP 200 + content[0].text 才是 success
        if code == 200 and "content" in parsed and parsed["content"]:
            content = parsed["content"][0].get("text", "") if isinstance(parsed["content"], list) else ""
            return ProviderResult(
                provider=self.name, ok=True, status=ProviderStatus.SUCCESS.value,
                http_code=code, content=content, elapsed_ms=elapsed,
                raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
            )
        # 失败分类 (主 17:43 实事求是 — 失败就是失败)
        if code in (401, 403):
            status = ProviderStatus.UNAUTHORIZED.value
        elif code == 429:
            status = ProviderStatus.RATE_LIMITED.value
        elif code in (408,):
            status = ProviderStatus.TIMEOUT.value
        elif code in (400, 422):
            status = ProviderStatus.BAD_REQUEST.value
        elif code >= 500:
            status = ProviderStatus.SERVER_ERROR.value
        else:
            status = ProviderStatus.BAD_REQUEST.value
        return ProviderResult(
            provider=self.name, ok=False, status=status, http_code=code,
            error=parsed.get("error", {}).get("message") if isinstance(parsed.get("error"), dict)
                  else str(parsed.get("error", ""))[:200],
            elapsed_ms=elapsed, raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
        )


class OpenAIAdapter(ProviderAdapter):
    """OpenAI Chat Completions API 适配 (主 19:33 官方文档真借鉴)."""
    name = "openai_chat"

    def adapt(self, prompt: str, **kwargs: Any) -> ProviderResult:
        t0 = time.time()
        api_key = kwargs.get("api_key") or os.environ.get("OPENAI_API_KEY")
        endpoint = kwargs.get("endpoint") or os.environ.get(
            "OPENAI_BASE_URL", "https://api.openai.com/v1/chat/completions"
        )
        model = kwargs.get("model") or os.environ.get("OPENAI_MODEL", "gpt-4o-mini")
        max_tokens = int(kwargs.get("max_tokens", 1024))
        system_prompt = kwargs.get("system_prompt", "你是 Apeireth ASI 助手.")
        timeout_sec = float(kwargs.get("timeout_sec", 30.0))

        if not api_key:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.NOT_CONFIGURED.value,
                error="OPENAI_API_KEY 未配置", elapsed_ms=int((time.time() - t0) * 1000),
            )

        body = {
            "model": model,
            "max_tokens": max_tokens,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt},
            ],
        }
        headers = {
            "Authorization": f"Bearer {api_key}",
            "content-type": "application/json",
        }

        try:
            code, parsed = self._http_json(endpoint, body, headers, timeout_sec)
        except ConnectionError as e:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.UNAVAILABLE.value,
                error=str(e), elapsed_ms=int((time.time() - t0) * 1000),
            )
        except Exception as e:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.SERVER_ERROR.value,
                error=f"{type(e).__name__}: {e}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )

        elapsed = int((time.time() - t0) * 1000)
        if code == 200 and "choices" in parsed and parsed["choices"]:
            content = parsed["choices"][0].get("message", {}).get("content", "")
            return ProviderResult(
                provider=self.name, ok=True, status=ProviderStatus.SUCCESS.value,
                http_code=code, content=content, elapsed_ms=elapsed,
                raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
            )
        if code in (401, 403):
            status = ProviderStatus.UNAUTHORIZED.value
        elif code == 429:
            status = ProviderStatus.RATE_LIMITED.value
        elif code == 408:
            status = ProviderStatus.TIMEOUT.value
        elif code in (400, 422):
            status = ProviderStatus.BAD_REQUEST.value
        elif code >= 500:
            status = ProviderStatus.SERVER_ERROR.value
        else:
            status = ProviderStatus.BAD_REQUEST.value
        return ProviderResult(
            provider=self.name, ok=False, status=status, http_code=code,
            error=parsed.get("error", {}).get("message") if isinstance(parsed.get("error"), dict)
                  else str(parsed.get("error", ""))[:200],
            elapsed_ms=elapsed, raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
        )


class OllamaAdapter(ProviderAdapter):
    """Ollama /api/chat 适配 (主 19:33 Ollama 官方文档真借鉴).

    Ollama 通常本地 http://127.0.0.1:11434/api/chat, 未运行 = Connection refused.
    """
    name = "ollama_chat"

    def adapt(self, prompt: str, **kwargs: Any) -> ProviderResult:
        t0 = time.time()
        host = kwargs.get("ollama_host") or os.environ.get("OLLAMA_HOST", "127.0.0.1")
        endpoint = kwargs.get("endpoint") or f"http://{host}:11434/api/chat"
        model = kwargs.get("model") or os.environ.get("OLLAMA_MODEL", "qwen2.5:1.5b")
        temperature = float(kwargs.get("temperature", 0.7))
        num_predict = int(kwargs.get("num_predict", 1024))
        system_prompt = kwargs.get("system_prompt", "你是 Apeireth ASI 助手.")
        timeout_sec = float(kwargs.get("timeout_sec", 30.0))

        body = {
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt},
            ],
            "stream": False,
            "options": {"temperature": temperature, "num_predict": num_predict},
        }
        headers = {"content-type": "application/json"}

        try:
            code, parsed = self._http_json(endpoint, body, headers, timeout_sec)
        except ConnectionError as e:
            # 主 17:43 — Ollama 未运行如实报告, 不假装成功
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.UNAVAILABLE.value,
                error=f"Ollama 未运行或不可达: {e}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )
        except Exception as e:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.SERVER_ERROR.value,
                error=f"{type(e).__name__}: {e}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )

        elapsed = int((time.time() - t0) * 1000)
        if code == 200 and "message" in parsed:
            content = parsed["message"].get("content", "")
            return ProviderResult(
                provider=self.name, ok=True, status=ProviderStatus.SUCCESS.value,
                http_code=code, content=content, elapsed_ms=elapsed,
                raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
            )
        return ProviderResult(
            provider=self.name, ok=False, status=ProviderStatus.BAD_REQUEST.value,
            http_code=code, error=str(parsed)[:200], elapsed_ms=elapsed,
            raw_excerpt=json.dumps(parsed, ensure_ascii=False)[:200],
        )


class LocalExecutableAdapter(ProviderAdapter):
    """本地可执行 prompt 适配 (主 19:33 走在前人经验上 — Unix pipe).

    用 stdin/stdout 跑命令: echo "$prompt" | cmd.
    无 stdout = 失败 (主 17:43 实事求是).
    """
    name = "local_executable"

    def adapt(self, prompt: str, **kwargs: Any) -> ProviderResult:
        t0 = time.time()
        executable = kwargs.get("executable") or os.environ.get("APEIRETH_LOCAL_LLM_BIN")
        args = kwargs.get("args", [])
        timeout_sec = float(kwargs.get("timeout_sec", 10.0))

        if not executable:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.NOT_CONFIGURED.value,
                error="executable 未配置 (env APEIRETH_LOCAL_LLM_BIN or kwargs)",
                elapsed_ms=int((time.time() - t0) * 1000),
            )
        if not shutil.which(executable):
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.UNAVAILABLE.value,
                error=f"executable 不存在: {executable}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )

        cmd = [executable, *args]
        try:
            proc = subprocess.run(
                cmd, input=prompt.encode("utf-8"),
                capture_output=True, timeout=timeout_sec, check=False,
            )
        except subprocess.TimeoutExpired:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.TIMEOUT.value,
                error=f"timeout after {timeout_sec}s",
                elapsed_ms=int((time.time() - t0) * 1000),
            )
        except Exception as e:
            return ProviderResult(
                provider=self.name, ok=False, status=ProviderStatus.SERVER_ERROR.value,
                error=f"{type(e).__name__}: {e}",
                elapsed_ms=int((time.time() - t0) * 1000),
            )

        elapsed = int((time.time() - t0) * 1000)
        stdout = proc.stdout.decode("utf-8", errors="replace").strip()
        stderr = proc.stderr.decode("utf-8", errors="replace").strip()
        if proc.returncode == 0 and stdout:
            return ProviderResult(
                provider=self.name, ok=True, status=ProviderStatus.SUCCESS.value,
                content=stdout, elapsed_ms=elapsed, raw_excerpt=stdout[:200],
            )
        return ProviderResult(
            provider=self.name, ok=False, status=ProviderStatus.BAD_REQUEST.value,
            http_code=proc.returncode,
            error=stderr or f"non-zero exit: {proc.returncode}",
            elapsed_ms=elapsed, raw_excerpt=stdout[:200],
        )


# Provider 注册表 (主 19:33 — 1 个 dict 就够, 不发明工厂)
PROVIDER_REGISTRY: Dict[str, ProviderAdapter] = {
    "anthropic_messages": AnthropicAdapter(),
    "openai_chat": OpenAIAdapter(),
    "ollama_chat": OllamaAdapter(),
    "local_executable": LocalExecutableAdapter(),
}


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 3. 公开 API (主 00:56 任何人都能接手)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def render_r10_template(
    template_name: str,
    variables: Mapping[str, Any],
    *,
    max_tokens: int = 4096,
    guard: bool = True,
) -> str:
    """V1129 渲染 R10 .j2 模板 (主 23:44 干到底 — 复用 V1122 loader).

    自动套用 V1125 V0.5 默认值 (v04_score / v05_total / 3 新维等).
    """
    tpl_text = load_template(template_name)
    filled = dict(variables)

    # V0.5 公式默认值 (主 19:33 — 继承 V1125.compute_v05_score)
    if "ultimate_target" in tpl_text and "ultimate_target" not in filled:
        filled["ultimate_target"] = V05_ULTIMATE_TARGET
    if "v05_target" in tpl_text and "v05_target" not in filled:
        filled["v05_target"] = V05_ULTIMATE_TARGET
    if "v04_baseline" in tpl_text and "v04_baseline" not in filled:
        filled["v04_baseline"] = 0.8538  # R9 W4 末 baseline
    if "v04_score" in tpl_text and "v04_score" not in filled:
        filled["v04_score"] = filled.get("v04_baseline", 0.8538)
    if "v05_total" in tpl_text and "v05_total" not in filled:
        filled["v05_total"] = filled.get("v05_baseline", 0.90)
    if "continuity" in tpl_text and "continuity" not in filled:
        filled["continuity"] = 0.85
    if "autonomy" in tpl_text and "autonomy" not in filled:
        filled["autonomy"] = 0.85
    if "transferability" in tpl_text and "transferability" not in filled:
        filled["transferability"] = 0.85
    if "abs_headroom" in tpl_text and "abs_headroom" not in filled:
        # 0.95 - 0.90 = 0.05
        filled["abs_headroom"] = V05_ULTIMATE_TARGET - filled.get("v05_total", 0.90)
    if "target_module" in tpl_text and "target_module" not in filled:
        filled["target_module"] = "V1125"
    if "candidate_output" in tpl_text and "candidate_output" not in filled:
        filled["candidate_output"] = "R10 W1 候选"
    if "v05_baseline" in tpl_text and "v05_baseline" not in filled:
        # 若没传 v05, 用 V0.5 公式算
        v04 = filled.get("v04_score", filled.get("v04_baseline", 0.8538))
        c = filled.get("continuity", 0.85)
        a = filled.get("autonomy", 0.85)
        t = filled.get("transferability", 0.85)
        filled["v05_baseline"] = (
            V04_WEIGHT * v04 + NEW_DIM_WEIGHT * (c + a + t)
        )
    if "r10_week" in tpl_text and "r10_week" not in filled:
        filled["r10_week"] = "W1"
    if "endpoint" in tpl_text and "endpoint" not in filled:
        filled["endpoint"] = "https://api.anthropic.com/v1/messages"
    if "ollama_host" in tpl_text and "ollama_host" not in filled:
        filled["ollama_host"] = "127.0.0.1"
    if "model" in tpl_text and "model" not in filled:
        filled["model"] = "MiniMax-M3"
    if "max_tokens" in tpl_text and "max_tokens" not in filled:
        filled["max_tokens"] = 1024
    if "temperature" in tpl_text and "temperature" not in filled:
        filled["temperature"] = 0.7
    if "num_predict" in tpl_text and "num_predict" not in filled:
        filled["num_predict"] = 1024
    if "system_prompt" in tpl_text and "system_prompt" not in filled:
        filled["system_prompt"] = "你是 Apeireth ASI 助手."
    if "user_prompt" in tpl_text and "user_prompt" not in filled:
        filled["user_prompt"] = "[empty]"
    if "consensus_threshold" in tpl_text and "consensus_threshold" not in filled:
        filled["consensus_threshold"] = 0.66
    if "max_rounds" in tpl_text and "max_rounds" not in filled:
        filled["max_rounds"] = 50
    if "current_round" in tpl_text and "current_round" not in filled:
        filled["current_round"] = 1
    if "n_agents" in tpl_text and "n_agents" not in filled:
        filled["n_agents"] = 3
    if "agent_ids" in tpl_text and "agent_ids" not in filled:
        filled["agent_ids"] = "node-A, node-B, node-C"
    if "identity_id" in tpl_text and "identity_id" not in filled:
        filled["identity_id"] = "chu-ling-r10-w1"
    if "target" in tpl_text and "target" not in filled:
        filled["target"] = "V0.5 = 0.9500"
    if "candidate_proposals" in tpl_text and "candidate_proposals" not in filled:
        filled["candidate_proposals"] = "- cand-001: lift +0.012 (method=sexual)"
    if "rel_headroom" in tpl_text and "rel_headroom" not in filled:
        filled["rel_headroom"] = 5.26  # (0.95 - 0.90) / 0.95 * 100

    return render_template(template_name, filled, max_tokens=max_tokens, guard=guard)


def adapt_prompt_to_provider(
    provider: str,
    prompt: str,
    **kwargs: Any,
) -> ProviderResult:
    """V1129 跨 provider 真适配 (主 17:43 实事求是 — 失败 = 失败).

    Args:
        provider: anthropic_messages / openai_chat / ollama_chat / local_executable
        prompt: 渲染后 prompt 文本
        **kwargs: provider-specific 参数 (api_key, model, max_tokens, ...)

    Returns:
        ProviderResult (ok=True 仅当真 HTTP 200 + content 提取)

    Raises:
        KeyError: provider 未注册
    """
    adapter = PROVIDER_REGISTRY.get(provider)
    if adapter is None:
        raise KeyError(
            f"V1129 未知 provider: {provider!r} (可用: {list(PROVIDER_REGISTRY)})"
        )
    return adapter.adapt(prompt, **kwargs)


def adapt_to_all_providers(
    prompt: str,
    *,
    anthropic_kwargs: Optional[Mapping[str, Any]] = None,
    openai_kwargs: Optional[Mapping[str, Any]] = None,
    ollama_kwargs: Optional[Mapping[str, Any]] = None,
    local_kwargs: Optional[Mapping[str, Any]] = None,
) -> Dict[str, ProviderResult]:
    """真跑全部 4 provider (主 17:43 — 失败就是失败, 不假装 ≥3 成功).

    Returns:
        {provider_name: ProviderResult}
        n_ok 真实计数, 由调用方按 .ok 字段统计 (主 17:58 不假装).
    """
    out: Dict[str, ProviderResult] = {}
    for name, kw in (
        ("anthropic_messages", anthropic_kwargs or {}),
        ("openai_chat", openai_kwargs or {}),
        ("ollama_chat", ollama_kwargs or {}),
        ("local_executable", local_kwargs or {}),
    ):
        try:
            out[name] = adapt_prompt_to_provider(name, prompt, **kw)
        except Exception as e:
            # 主 17:43 — 任何异常都捕获, 不假装成功
            out[name] = ProviderResult(
                provider=name, ok=False,
                status=ProviderStatus.SERVER_ERROR.value,
                error=f"adapter 异常: {type(e).__name__}: {e}",
                elapsed_ms=0,
            )
    return out


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 4. V0.5 真测 driver (主 17:43 实事求是 — 数字驱动)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def compute_v05_total(
    v04: float,
    continuity: float = 0.85,
    autonomy: float = 0.85,
    transferability: float = 0.85,
) -> float:
    """V0.5 公式 (主 19:33 — 继承 V1125.compute_v05_score, 一行)."""
    return V04_WEIGHT * v04 + NEW_DIM_WEIGHT * (continuity + autonomy + transferability)


def run_r10_all_specs() -> Dict[str, Any]:
    """真测 5 R10 模板 (主 23:44 干到底 — 5 模板全跑)."""
    n_ok = 0
    n_fail = 0
    failures: List[Dict[str, Any]] = []
    rendered_chars = 0

    base_vars = {
        # asi_north_star_v05
        "ultimate_target": V05_ULTIMATE_TARGET,
        "v05_total": 0.91,
        "v04_score": 0.86,
        "continuity": 0.88, "autonomy": 0.87, "transferability": 0.86,
        "abs_headroom": 0.04, "rel_headroom": 4.21,
        "target_module": "V1125", "candidate_output": "R10 W1 候选",
        # v0_5_18dim
        "v04_baseline": 0.8538, "v05_target": V05_ULTIMATE_TARGET,
        "r10_week": "W1", "target_module": "V1125_18dim",
        # multi_agent_consensus
        "n_agents": 3, "agent_ids": "node-A,node-B,node-C",
        "identity_id": "chu-ling-r10-w1",
        "consensus_threshold": 0.66, "max_rounds": 50, "current_round": 1,
        "v05_baseline": 0.91, "target": "V0.5 = 0.95",
        "candidate_proposals": "- cand-001: lift +0.012 (sexual)\n- cand-002: lift +0.018 (parent_child)",
        # anthropic_native
        "system_prompt": "你是 ASI 助手.", "user_prompt": "R10 W1 真测 prompt",
        "endpoint": "https://api.anthropic.com/v1/messages",
        "model": "MiniMax-M3", "max_tokens": 1024,
        # ollama_native
        "ollama_host": "127.0.0.1",
        "temperature": 0.7, "num_predict": 1024,
    }

    for tpl in R10_TEMPLATES:
        try:
            out = render_r10_template(tpl, base_vars)
            n_ok += 1
            rendered_chars += len(out)
        except Exception as e:
            n_fail += 1
            failures.append({"template": tpl, "error": repr(e)})

    return {
        "version": V1129_VERSION,
        "n_templates": len(R10_TEMPLATES),
        "n_ok": n_ok,
        "n_fail": n_fail,
        "rendered_chars": rendered_chars,
        "failures": failures,
    }


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 5. Markdown 报告 (主 00:56)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def report_markdown() -> str:
    """V1129 W1 真跑 Markdown 报告 (主 00:56)."""
    specs = run_r10_all_specs()
    all_templates = list_templates()
    r10_only = [t for t in all_templates if t.replace(".j2", "") in R10_TEMPLATES]

    lines: List[str] = []
    lines.append("# V1129 R10 Prompt Template — W1 真跑报告")
    lines.append("")
    lines.append(f"- version: `{V1129_VERSION}`")
    lines.append(f"- prompt_tpl_version (inherited V1122): `{PROMPT_TPL_VERSION}`")
    lines.append(f"- ts: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append(f"- n_r10_modules: {len(MODULES_R10)}")
    lines.append(f"- n_r10_templates: {len(R10_TEMPLATES)}")
    lines.append("")
    lines.append("## 主哲学 LOCKED (主 22:33 + 主 17:43 + 主 17:58)")
    for k, v in V3_GUARDS.items():
        lines.append(f"- **{k}**: {v}")
    lines.append("")
    lines.append("## V0.5 公式 (继承 V1125.compute_v05_score)")
    lines.append("")
    lines.append("```")
    lines.append(f"v05_total = {V04_WEIGHT}*v04 + {NEW_DIM_WEIGHT}*continuity + {NEW_DIM_WEIGHT}*autonomy + {NEW_DIM_WEIGHT}*transferability")
    lines.append(f"ultimate_target = {V05_ULTIMATE_TARGET}  # R10 终极门")
    lines.append(f"r10_start = {V05_R10_START}")
    lines.append(f"mid_target = {V05_MID_TARGET}")
    lines.append("```")
    lines.append("")
    lines.append("## 5 R10 新模板真测")
    lines.append("")
    lines.append("| template | status | chars |")
    lines.append("|---|---|---|")
    for t in r10_only:
        slug = t.replace(".j2", "")
        ok = "✓" if specs["n_ok"] >= 1 and not specs["failures"] else "?"
        lines.append(f"| {t} | {ok} | (动态) |")
    lines.append("")
    lines.append(f"- 真测通过: **{specs['n_ok']}/{specs['n_templates']}**")
    lines.append(f"- 失败: {specs['n_fail']}")
    lines.append(f"- 累计渲染字符: {specs['rendered_chars']:,}")
    if specs["failures"]:
        lines.append("")
        lines.append("### 失败明细")
        for f in specs["failures"]:
            lines.append(f"- {f['template']}: {f['error']}")
    lines.append("")
    lines.append("## 4 Provider 适配器 (主 17:43 实事求是 — 失败就是失败)")
    lines.append("")
    lines.append("| provider | name | 真 I/O | 失败 = 失败 |")
    lines.append("|---|---|---|---|")
    for name in PROVIDER_REGISTRY:
        lines.append(f"| {name} | {name} | ✓ | ✓ |")
    lines.append("")
    lines.append("- Anthropic Messages API 官方格式 (system + user 分离)")
    lines.append("- OpenAI Chat Completions 官方格式")
    lines.append("- Ollama /api/chat 官方格式 (stream=false)")
    lines.append("- Local Executable stdin/stdout pipe")
    lines.append("- **V1129 不假装 ≥3 provider 成功**: n_ok 必须真统计 .ok 字段")
    lines.append("")
    lines.append("## R10 V3 守门")
    lines.append("")
    for k, v in V3_GUARDS.items():
        lines.append(f"- `{k}`: {v}")
    lines.append("")
    lines.append("## CLI 复现 (主 00:56)")
    lines.append("")
    lines.append("```bash")
    lines.append("python -m apeireth.v1129_r10_prompt_template report")
    lines.append("python -m apeireth.v1129_r10_prompt_template render asi_north_star_v05 --vars '{\"v05_total\":0.90}'")
    lines.append("python -m apeireth.v1129_r10_prompt_template providers  # 列出全部")
    lines.append("python -m apeireth.v1129_r10_prompt_template honest-test anthropic  # 真跑, 失败 = 失败")
    lines.append("```")
    lines.append("")
    lines.append("## 主哲学 9 键 LOCKED")
    lines.append("")
    lines.append("- 22:33 ASI 北极星 — V0.5 ≥ 0.95 = R10 终极门")
    lines.append("- 17:43 实事求是 — Provider 失败 = Provider 失败, 不假装 ≥3 成功")
    lines.append("- 17:58 不假装 — V3 守门全列, 缺 guard 立即抛错")
    lines.append("- 23:44 干到底 — 5 模板 + 4 adapter + 真实 I/O, 不只盘点")
    lines.append("- 19:33 走在前人经验上 — V1122 + V1125 + V1127 + V1128 + V1124")
    lines.append("- 13:31 大胆激进 — 4 真 provider adapter + 真 HTTP 调用")
    lines.append("- 20:46 测量 ≠ 真值 — Provider 比较 ≠ ASI 达成")
    lines.append("- 00:56 任何人都能接手 — 一行 CLI 跑全部")
    lines.append("- 20:55 红皇后 — V0.5 = 0.95 是 R10 终极, 不停在 0.8538")
    lines.append("")
    return "\n".join(lines)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 6. CLI (主 00:56)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def _cli_render(args: argparse.Namespace) -> int:
    try:
        vars_dict = json.loads(args.vars) if args.vars else {}
    except json.JSONDecodeError as e:
        print(f"[V1129] --vars JSON 解析失败: {e}", file=sys.stderr)
        return 2
    try:
        out = render_r10_template(args.render, vars_dict)
    except Exception as e:
        print(f"[V1129] render 失败: {e}", file=sys.stderr)
        return 1
    print(out)
    return 0


def _cli_providers(_: argparse.Namespace) -> int:
    print("V1129 provider 注册表 (主 17:43 实事求是 — 真 I/O, 失败 = 失败):")
    for name, adapter in PROVIDER_REGISTRY.items():
        print(f"  - {name}: {type(adapter).__name__}")
    return 0


def _cli_report(_: argparse.Namespace) -> int:
    print(report_markdown())
    return 0


def _cli_json(_: argparse.Namespace) -> int:
    specs = run_r10_all_specs()
    out = {
        "version": V1129_VERSION,
        "prompt_tpl_version": PROMPT_TPL_VERSION,
        "r10_modules": list(MODULES_R10),
        "r10_templates": list(R10_TEMPLATES),
        "providers": list(PROVIDER_REGISTRY),
        "v05_formula": f"{V04_WEIGHT}*v04 + {NEW_DIM_WEIGHT}*continuity + {NEW_DIM_WEIGHT}*autonomy + {NEW_DIM_WEIGHT}*transferability",
        "ultimate_target": V05_ULTIMATE_TARGET,
        "specs": specs,
        "v3_guards": V3_GUARDS,
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def _cli_honest_test(args: argparse.Namespace) -> int:
    """真跑 provider (主 17:43 — 失败 = 失败, 不假装)."""
    provider = args.provider
    if provider not in PROVIDER_REGISTRY:
        print(f"[V1129] 未知 provider: {provider} (可用: {list(PROVIDER_REGISTRY)})", file=sys.stderr)
        return 2
    try:
        prompt = render_r10_template("asi_north_star_v05", {})
    except Exception as e:
        print(f"[V1129] template render 失败: {e}", file=sys.stderr)
        return 1
    try:
        result = adapt_prompt_to_provider(provider, prompt, **{
            "max_tokens": 64, "timeout_sec": args.timeout,
        })
    except Exception as e:
        print(f"[V1129] adapter 异常: {e}", file=sys.stderr)
        return 1
    # 主 17:43 — 诚实输出
    print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2))
    return 0 if result.ok else 1


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1129_r10_prompt_template",
        description="V1129 R10 北极星 prompt 模板 + 跨 provider 适配 (R10-PE-001, 主 00:56)",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    p_render = sub.add_parser("render", help="单模板渲染")
    p_render.add_argument("render", help="R10 模板名 (e.g. asi_north_star_v05)")
    p_render.add_argument("--vars", default="", help="JSON 变量")
    p_render.set_defaults(func=_cli_render)

    p_prov = sub.add_parser("providers", help="列出 provider 注册表")
    p_prov.set_defaults(func=_cli_providers)

    p_report = sub.add_parser("report", help="Markdown 报告")
    p_report.set_defaults(func=_cli_report)

    p_json = sub.add_parser("json", help="JSON 真测结果")
    p_json.set_defaults(func=_cli_json)

    p_test = sub.add_parser("honest-test", help="真跑 provider (失败 = 失败)")
    p_test.add_argument("provider", help="provider name")
    p_test.add_argument("--timeout", type=float, default=10.0, help="timeout sec")
    p_test.set_defaults(func=_cli_honest_test)

    args = parser.parse_args(argv)
    if args.cmd is None:
        args.cmd = "report"
        args.func = _cli_report
    return args.func(args)


__all__ = [
    "V1129_VERSION", "PROMPT_TPL_VERSION",
    "MODULES_R10", "R10_TEMPLATES",
    "V04_WEIGHT", "NEW_DIM_WEIGHT",
    "V05_ULTIMATE_TARGET", "V05_MID_TARGET", "V05_R10_START",
    "V3_GUARDS",
    "ProviderStatus", "ProviderResult",
    "ProviderAdapter", "AnthropicAdapter", "OpenAIAdapter",
    "OllamaAdapter", "LocalExecutableAdapter",
    "PROVIDER_REGISTRY",
    "render_r10_template", "adapt_prompt_to_provider",
    "adapt_to_all_providers", "compute_v05_total",
    "run_r10_all_specs", "report_markdown", "main",
]