"""Phase 21 LLM Kernel — Plug any LLM into Apeireth (MiniMax default).

主人 20:39 LLM 政策:
  - 默认: MiniMax (minimax/MiniMax-M3)
  - 避免: DeepSeek (主人额度低)
  - 架构: LLM-agnostic (任何 LLM 都能 plug in)

主人 20:29 真哲学:
  "ASI就是我们的目标, 让任何大模型接入我们的平台后成为ASI"

真生产 LLM 接入 — 主人 14:27 "聚集全人类智慧来打造他":
  - MiniMax-M3 (主人默认)
  - Claude / GPT / Qwen (备选)
  - 本地 Ollama (可选)

LLM Kernel API:
  - call_llm(prompt: str) -> str
  - call_llm_chat(messages: list) -> str
  - embed(text: str) -> list[float]
"""
from __future__ import annotations

import os
import time
from dataclasses import dataclass, field
from typing import Optional


LLM_KERNEL_VERSION = "0.1.0"


@dataclass
class LLMConfig:
    """LLM Kernel configuration."""
    provider: str = "minimax"           # minimax | openai | anthropic | ollama | custom
    model: str = "MiniMax-M3"           # default MiniMax (主人 20:39)
    base_url: Optional[str] = None      # e.g. "http://localhost:3000/v1" for NewAPI
    api_key: Optional[str] = None       # 默认 None, 走环境变量
    timeout: float = 30.0               # seconds
    max_retries: int = 3
    temperature: float = 0.7

    @classmethod
    def minimax_default(cls) -> "LLMConfig":
        """主人 20:39 默认 — MiniMax-M3 via NewAPI local."""
        return cls(
            provider="minimax",
            model="MiniMax-M3",
            base_url=os.environ.get("NEWAPI_BASE_URL", "http://localhost:3000/v1"),
            api_key=os.environ.get("NEWAPI_API_KEY", os.environ.get("MINIMAX_API_KEY")),
        )


@dataclass
class LLMResponse:
    """LLM call response — uniform across providers."""
    content: str
    model: str
    tokens_in: int = 0
    tokens_out: int = 0
    latency_ms: float = 0.0
    provider: str = ""
    # 借鉴 anthropic-sdk (主 11:10 round-20 P0): prompt caching 独立计费
    # anthropic SDK 返回 cache_creation_input_tokens + cache_read_input_tokens
    cache_hit: bool = False               # 任何 cache 命中 (creation or read)
    cache_creation_tokens: int = 0       # 本次创建 cache 用的 input tokens
    cache_read_tokens: int = 0           # 本次读 cache 用的 input tokens (节省的真实成本)

    @property
    def cache_savings_ratio(self) -> float:
        """Cache 节省比率 = cache_read / (cache_read + tokens_in), [0, 1]. 用于监控."""
        denom = self.cache_read_tokens + self.tokens_in
        if denom == 0:
            return 0.0
        return self.cache_read_tokens / denom

    def to_dict(self) -> dict:
        return {
            "content": self.content,
            "model": self.model,
            "tokens_in": self.tokens_in,
            "tokens_out": self.tokens_out,
            "latency_ms": self.latency_ms,
            "provider": self.provider,
            "cache_hit": self.cache_hit,
            "cache_creation_tokens": self.cache_creation_tokens,
            "cache_read_tokens": self.cache_read_tokens,
            "cache_savings_ratio": round(self.cache_savings_ratio, 4),
        }


def call_llm_minimax(prompt: str, config: LLMConfig = None) -> LLMResponse:
    """Call MiniMax-M3 via NewAPI (主人 20:39 默认).

    Requires `requests` library + valid api_key.
    """
    config = config or LLMConfig.minimax_default()
    try:
        import requests
    except ImportError:
        # Fallback: return template response (don't consume quota)
        return LLMResponse(
            content=f"[minimax-template] Would call {config.model} with: {prompt[:100]}...",
            model=config.model,
            provider="minimax-template",
        )

    if not config.api_key:
        return LLMResponse(
            content=f"[minimax-no-key] Need NEWAPI_API_KEY env. Would call: {prompt[:100]}...",
            model=config.model,
            provider="minimax-no-key",
        )

    url = f"{config.base_url}/chat/completions"
    headers = {"Authorization": f"Bearer {config.api_key}", "Content-Type": "application/json"}
    payload = {
        "model": config.model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": config.temperature,
    }
    t0 = time.time()
    try:
        r = requests.post(url, headers=headers, json=payload, timeout=config.timeout)
        latency = (time.time() - t0) * 1000
        if r.status_code == 200:
            data = r.json()
            choice = data.get("choices", [{}])[0]
            msg = choice.get("message", {})
            usage = data.get("usage", {})
            # 借鉴 anthropic-sdk (主 11:10 round-20 P0): cache_creation/cache_read 独立计费
            cache_creation = usage.get("cache_creation_input_tokens", 0)
            cache_read = usage.get("cache_read_input_tokens", 0)
            return LLMResponse(
                content=msg.get("content", ""),
                model=data.get("model", config.model),
                tokens_in=usage.get("prompt_tokens", 0),
                tokens_out=usage.get("completion_tokens", 0),
                latency_ms=latency,
                provider=config.provider,
                cache_hit=(cache_creation + cache_read) > 0,
                cache_creation_tokens=cache_creation,
                cache_read_tokens=cache_read,
            )
        else:
            return LLMResponse(
                content=f"[minimax-err-{r.status_code}] {r.text[:200]}",
                model=config.model,
                latency_ms=latency,
                provider=f"{config.provider}-err",
            )
    except Exception as e:
        return LLMResponse(
            content=f"[minimax-conn-err] {str(e)[:200]}",
            model=config.model,
            provider=f"{config.provider}-err",
        )


def call_llm_template(prompt: str) -> str:
    """Template LLM — no API call, returns structured response for testing."""
    return f"[template-LLM]\nQuery: {prompt[:80]}\nResponse: [simulated-thinking-pattern]"


def make_call_llm(provider: str = "minimax") -> callable:
    """Factory — returns appropriate call_llm function."""
    if provider == "minimax":
        def call_llm(prompt: str) -> str:
            return call_llm_minimax(prompt).content
        return call_llm
    elif provider == "template":
        return call_llm_template
    else:
        # custom — try OpenAI-compat
        def call_llm(prompt: str) -> str:
            try:
                import requests
                cfg = LLMConfig(provider=provider)
                url = f"{cfg.base_url or 'http://localhost:3000/v1'}/chat/completions"
                headers = {"Authorization": f"Bearer {cfg.api_key or 'none'}"}
                r = requests.post(url, headers=headers,
                                  json={"model": cfg.model, "messages": [{"role": "user", "content": prompt}]},
                                  timeout=cfg.timeout)
                if r.status_code == 200:
                    return r.json()["choices"][0]["message"]["content"]
            except Exception:
                pass
            return f"[{provider}-err] " + prompt[:100]
        return call_llm


__all__ = [
    "LLM_KERNEL_VERSION",
    "LLMConfig",
    "LLMResponse",
    "call_llm_minimax",
    "call_llm_template",
    "make_call_llm",
]
