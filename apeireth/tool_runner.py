"""Phase 49 Tool Runner — anthropic-sdk BetaToolRunner-style 3 防御.

主 11:10 round-20 source-deep-read P1 推荐:
  - 写 apeireth/tool_runner.py 借鉴 BetaToolRunner 3 防御
  - 不直接抄 (主 23:28 真研究哲学: 理解机制再借鉴)

借鉴自 anthropic-sdk BetaToolRunner:
  1. Refusal detection: LLM 返回 refusal → 终止不调 tool_use
     主 22:08 V2 中央 AI 是调度者, 不应执行被 LLM 拒绝的命令
  2. Token threshold trigger: token 用量超过阈值 → 自动 summarization
     主 17:50 "涌现 自组织" — 长 context 应该自动管理
  3. Cache reuse: tool_call_response 缓存避免重复 invoke
     主 22:08 + 主 14:27 持续聚合全人类智慧 = 不浪费

设计原则 (主 9:15 修好优先):
  - 真生产可用, 不仅是 demo
  - V2 哲学守门: 不假装 Phenomenal
  - 借鉴是工具 (主 20:55 隐喻是借不是抄)
"""
from __future__ import annotations

import hashlib
import json
import time
import uuid
from collections import OrderedDict
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional

from .llm_kernel import LLMResponse


TOOL_RUNNER_VERSION = "0.1.0"


# Token threshold 默认值 (主 17:43 实事求是: 8000 是 Claude Sonnet context window 1/12)
DEFAULT_TOKEN_THRESHOLD = 8000


@dataclass
class ToolCall:
    """单个 tool call — 借鉴 BetaToolRunner ToolUseBlock."""
    tool_call_id: str
    tool_name: str
    tool_input: Dict[str, Any]
    output: Any = None
    started_at: float = field(default_factory=time.time)
    finished_at: Optional[float] = None
    error: Optional[str] = None
    cached: bool = False  # 借鉴 #3: 是否来自 cache (避免重复 invoke)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "tool_call_id": self.tool_call_id,
            "tool_name": self.tool_name,
            "tool_input": self.tool_input,
            "output": str(self.output)[:500] if self.output else None,  # truncate for safety
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "error": self.error,
            "cached": self.cached,
        }


class ToolRunner:
    """anthropic-sdk BetaToolRunner-style 3 防御 (主 11:10 round-20 P1).

    3 防御:
      1. Refusal detection: LLMResponse.content 含 refusal_pattern → 终止不调 tool_use
      2. Token threshold trigger: cumulative_tokens > threshold → 自动 summarization callback
      3. Cache reuse: tool_call_response cache (LRU) 避免重复 invoke

    V2 哲学守门 (主 22:08):
      - 拒绝 LLM refusal 是 V2 哲学: 中央 AI 不执行被拒绝的命令
      - Token 阈值自动管理是涌现 (主 17:50 自组织)
      - Cache 是聚合全人类智慧 (主 14:27) — 不重复浪费
    """

    # Refusal 模式 (借鉴 anthropic-sdk BetaToolRunner)
    # 主 22:08: 中央 AI 是调度者, 不执行被 LLM 拒绝的命令
    REFUSAL_PATTERNS = [
        "I cannot",
        "I can't",
        "I'm unable",
        "I am unable",
        "Sorry, but",
        "As an AI",
        "I'm sorry",
        "I apologize",
        "I won't",
        "I will not",
    ]

    def __init__(
        self,
        refusal_check: bool = True,
        token_threshold: int = DEFAULT_TOKEN_THRESHOLD,
        cache_enabled: bool = True,
        cache_max_size: int = 256,
        on_threshold_trigger: Optional[Callable[[int, int], None]] = None,
    ):
        """Init ToolRunner.

        Args:
            refusal_check: True 时检测 LLM refusal 并拒绝执行 tool_use
            token_threshold: cumulative tokens 超阈值触发 callback (默认 8000)
            cache_enabled: True 时缓存 tool_call 避免重复 invoke
            cache_max_size: LRU cache 上限 (默认 256)
            on_threshold_trigger: callback(current_tokens, threshold) — summarization 等
        """
        self.refusal_check = refusal_check
        self.token_threshold = token_threshold
        self.cache_enabled = cache_enabled
        self.cache_max_size = cache_max_size
        self.on_threshold_trigger = on_threshold_trigger

        # LRU cache (OrderedDict 实现)
        self._cache: OrderedDict[str, Any] = OrderedDict()

        # stats
        self.cumulative_tokens = 0
        self.n_tool_calls = 0
        self.n_cache_hits = 0
        self.n_refusals_detected = 0

    # === 防御 1: Refusal detection ===

    def detect_refusal(self, response: LLMResponse) -> bool:
        """借鉴 anthropic-sdk BetaToolRunner: 检测 LLM refusal.

        主 22:08 V2: 中央 AI 是调度者, 不执行被 LLM 拒绝的命令.
        Returns True if response 含 refusal pattern (拒绝执行后续 tool).
        """
        if not self.refusal_check:
            return False
        content_lower = response.content.lower()
        for pattern in self.REFUSAL_PATTERNS:
            if pattern.lower() in content_lower:
                self.n_refusals_detected += 1
                return True
        return False

    # === 防御 2: Token threshold trigger ===

    def add_tokens(self, tokens: int) -> None:
        """累计 token 用量, 超阈值触发 callback (主 17:50 涌现自组织).

        Args:
            tokens: 本次 LLM 调用消耗的 tokens
        """
        self.cumulative_tokens += tokens
        if self.cumulative_tokens >= self.token_threshold and self.on_threshold_trigger:
            self.on_threshold_trigger(self.cumulative_tokens, self.token_threshold)

    def should_trigger_threshold(self) -> bool:
        """检查是否应该触发 threshold callback."""
        return self.cumulative_tokens >= self.token_threshold

    # === 防御 3: Cache reuse ===

    def _cache_key(self, tool_name: str, tool_input: Dict[str, Any]) -> str:
        """生成 cache key = sha256(tool_name + json(tool_input, sort_keys))."""
        canonical = f"{tool_name}|{json.dumps(tool_input, sort_keys=True, ensure_ascii=False)}"
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()

    def cache_get(self, tool_name: str, tool_input: Dict[str, Any]) -> Optional[Any]:
        """借鉴 BetaToolRunner _cached_tool_call_response: 查 cache."""
        if not self.cache_enabled:
            return None
        key = self._cache_key(tool_name, tool_input)
        if key in self._cache:
            # LRU: move to end
            self._cache.move_to_end(key)
            self.n_cache_hits += 1
            return self._cache[key]
        return None

    def cache_put(self, tool_name: str, tool_input: Dict[str, Any], output: Any) -> None:
        """存 cache (LRU eviction if full)."""
        if not self.cache_enabled:
            return
        key = self._cache_key(tool_name, tool_input)
        self._cache[key] = output
        self._cache.move_to_end(key)
        # LRU eviction
        while len(self._cache) > self.cache_max_size:
            self._cache.popitem(last=False)

    # === Tool execution with 3 defenses ===

    def run_tool(
        self,
        tool_name: str,
        tool_input: Dict[str, Any],
        tool_fn: Callable[[Dict[str, Any]], Any],
    ) -> ToolCall:
        """Run tool with 3 defenses (借鉴 BetaToolRunner).

        流程:
          1. Cache check: hit → 直接返回 cached output (cached=True)
          2. Tool execution: 调用 tool_fn(tool_input)
          3. Cache store: 存结果到 cache
          4. Record: ToolCall metadata
        """
        call = ToolCall(
            tool_call_id=f"tc_{uuid.uuid4().hex[:12]}",
            tool_name=tool_name,
            tool_input=tool_input,
        )
        self.n_tool_calls += 1

        # 防御 3: cache check
        cached = self.cache_get(tool_name, tool_input)
        if cached is not None:
            call.output = cached
            call.cached = True
            call.finished_at = time.time()
            return call

        # Execute
        try:
            output = tool_fn(tool_input)
            call.output = output
            call.finished_at = time.time()
            # 存 cache
            self.cache_put(tool_name, tool_input, output)
        except Exception as e:
            call.error = f"{type(e).__name__}: {str(e)[:200]}"
            call.finished_at = time.time()

        return call

    # === Process LLMResponse (主 entry) ===

    def process_llm_response(
        self,
        response: LLMResponse,
        tool_calls: Optional[List[Dict[str, Any]]] = None,
        tool_fn_map: Optional[Dict[str, Callable]] = None,
    ) -> Dict[str, Any]:
        """处理 LLMResponse — 应用 3 防御.

        Args:
            response: LLMResponse from call_llm
            tool_calls: LLM 请求的 tool_use list (parsed)
            tool_fn_map: tool_name -> callable mapping

        Returns:
            {
                "refused": bool,         # 防御 1: 检测到 refusal
                "tool_results": [...],   # 防御 3: tool execution results
                "should_summarize": bool, # 防御 2: token 阈值触发
                "cumulative_tokens": int,
            }
        """
        # 累计 tokens (防御 2 trigger)
        total = (response.tokens_in or 0) + (response.tokens_out or 0) + (response.cache_read_tokens or 0)
        self.add_tokens(total)

        # 防御 1: refusal detection
        refused = self.detect_refusal(response)

        # 防御 3: execute tool calls
        tool_results: List[ToolCall] = []
        if not refused and tool_calls:
            for tc in tool_calls:
                tool_name = tc.get("name", "")
                tool_input = tc.get("input", {})
                tool_fn = (tool_fn_map or {}).get(tool_name)
                if tool_fn:
                    result = self.run_tool(tool_name, tool_input, tool_fn)
                    tool_results.append(result)
                else:
                    # unknown tool — record error
                    call = ToolCall(
                        tool_call_id=f"tc_{uuid.uuid4().hex[:12]}",
                        tool_name=tool_name,
                        tool_input=tool_input,
                        error=f"unknown tool: {tool_name}",
                    )
                    tool_results.append(call)

        return {
            "refused": refused,
            "tool_results": [t.to_dict() for t in tool_results],
            "should_summarize": self.should_trigger_threshold(),
            "cumulative_tokens": self.cumulative_tokens,
        }

    # === Stats ===

    def stats(self) -> Dict[str, Any]:
        return {
            "version": TOOL_RUNNER_VERSION,
            "n_tool_calls": self.n_tool_calls,
            "n_cache_hits": self.n_cache_hits,
            "n_refusals_detected": self.n_refusals_detected,
            "cache_size": len(self._cache),
            "cumulative_tokens": self.cumulative_tokens,
            "token_threshold": self.token_threshold,
            "cache_hit_ratio": (
                self.n_cache_hits / self.n_tool_calls if self.n_tool_calls > 0 else 0.0
            ),
        }

    def clear_cache(self) -> None:
        """清 cache (主 22:08 持久化 — 调用方决定何时清)."""
        self._cache.clear()


__all__ = [
    "TOOL_RUNNER_VERSION",
    "DEFAULT_TOKEN_THRESHOLD",
    "ToolCall",
    "ToolRunner",
]


# === Demo ===

def _demo():
    print("=" * 70)
    print("=== Phase 49 Tool Runner — BetaToolRunner-style 3 防御 ===")
    print("=" * 70)

    # 1. 创建 runner
    print("\n[1] 创建 ToolRunner (refusal + token threshold + cache)")
    threshold_called = []
    runner = ToolRunner(
        refusal_check=True,
        token_threshold=500,  # 调小以快速触发
        cache_enabled=True,
        cache_max_size=10,
        on_threshold_trigger=lambda cur, thr: threshold_called.append((cur, thr)),
    )
    print(f"  ✓ Token threshold: {runner.token_threshold}")
    print(f"  ✓ Cache enabled: {runner.cache_enabled}")

    # 2. 防御 3: cache reuse
    print("\n[2] Cache reuse (防御 3):")
    def fake_tool_calc(x):
        return x * 2
    call1 = runner.run_tool("calc", {"x": 5}, fake_tool_calc)
    print(f"  ✓ Call 1: result={call1.output}, cached={call1.cached}")
    call2 = runner.run_tool("calc", {"x": 5}, fake_tool_calc)
    print(f"  ✓ Call 2 (同 input): result={call2.output}, cached={call2.cached} (应该 True)")
    call3 = runner.run_tool("calc", {"x": 6}, fake_tool_calc)
    print(f"  ✓ Call 3 (新 input): result={call3.output}, cached={call3.cached} (应该 False)")

    # 3. 防御 1: refusal detection
    print("\n[3] Refusal detection (防御 1):")
    refusal_resp = LLMResponse(
        content="I cannot help with that request.",
        model="minimax/MiniMax-M3",
        tokens_in=10, tokens_out=20,
        provider="minimax",
    )
    refused = runner.detect_refusal(refusal_resp)
    print(f"  ✓ Refusal response detected: {refused} (应该 True)")
    normal_resp = LLMResponse(
        content="Here is the answer to your question.",
        model="minimax/MiniMax-M3",
        tokens_in=10, tokens_out=20,
        provider="minimax",
    )
    refused_normal = runner.detect_refusal(normal_resp)
    print(f"  ✓ Normal response detected as refusal: {refused_normal} (应该 False)")

    # 4. 防御 2: token threshold trigger
    print("\n[4] Token threshold trigger (防御 2):")
    runner.add_tokens(300)  # 还没到 500
    print(f"  ✓ Cumulative: {runner.cumulative_tokens}, triggered: {runner.should_trigger_threshold()} (应该 False)")
    runner.add_tokens(300)  # 600 > 500, 触发
    print(f"  ✓ Cumulative: {runner.cumulative_tokens}, triggered: {runner.should_trigger_threshold()} (应该 True)")
    print(f"  ✓ Threshold callback calls: {len(threshold_called)}")

    # 5. Stats
    print("\n[5] Stats:")
    stats = runner.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    # 6. LLMResponse 集成
    print("\n[6] 集成 LLMResponse:")
    full_resp = LLMResponse(
        content="Tool result: 42",
        model="minimax/MiniMax-M3",
        tokens_in=100, tokens_out=50, cache_read_tokens=20,
        provider="minimax",
    )
    result = runner.process_llm_response(
        full_resp,
        tool_calls=[{"name": "calc", "input": {"x": 21}}],
        tool_fn_map={"calc": fake_tool_calc},
    )
    print(f"  ✓ refused: {result['refused']}")
    print(f"  ✓ tool_results: {len(result['tool_results'])}")
    print(f"  ✓ cumulative_tokens: {result['cumulative_tokens']}")

    print("\n" + "=" * 70)
    print("✓ Phase 49 Tool Runner — 3 防御真生产落地")
    print("=" * 70)


if __name__ == "__main__":
    _demo()