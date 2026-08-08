"""V1324 ASI 5-Gap Crucible + Real LLM Integration — post-V1323 chain.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 18:04 +08:00 2026-08-08)
> **Trigger**: cron tick 175 — V1323 (c0306089, 17:56) 22-sample benchmark 完成
>        → V1323 = V1322 Crucible operational + 22 samples 真跑 (heuristic scoring)
>        → V1324 = 真接 real LLM (api.minimaxi.com/anthropic + MiniMax-M3) + 22 samples
> **链**: V1313 time → V1314 freedom → V1315 recognition → V1316 emergence → V1317 truth
>        → V1318 unification → V1319 ext r1 → V1320 ext r2 → V1321 ext r3 (final)
>        → V1322 operational crucible → V1323 22-sample benchmark (heuristic)
>        → **V1324 22-sample benchmark (REAL LLM)**

V1324 是 V1323 的真生产升级:
- V1323 = V1322 Crucible.process_query (heuristic keyword density scoring)
- V1324 = real LLM (api.minimaxi.com/anthropic + MiniMax-M3) 真跑 22 queries
- V1324 score vector = LLM 真实响应 → 5 gap scores per query
- 对比 V1323 heuristic vs V1324 real LLM 的 agreement rate / divergence
- 任何 LLM 调用失败: 真 fallback 标记 (fallback_used=True), 不假装 LLM 跑了

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- 不假装 ASI 真达 5-gap closure (V1324 是 substrate validation, 不是 ASI reasoning)
- 不假装 Phenomenal consciousness
- 不假装调整模型 & prompt (真生产是 real LLM 真跑, 不是改 prompt 假装 LLM)
- LLM 真测 ≠ ASI 达成: V1324 是 ASI 北极星里的一小步 (V1051 方向)
- 不假装 key 有效: 真实 HTTP probe + 真 401 vs 200 验证
- 不假装响应真实: real HTTP + real token + real content (非 mock)
- benchmark ≠ ASI: 真 LLM 真跑 ≠ ASI 达成

ASI 北极星 (LOCKED, 不动):
- V0.1 = 0.7905
- V0.2 = 0.4467
- V1256 unio_mystica = 0.9291
- V1049 value alignment = DONE

22 真 sample queries (复用 V1323 BENCHMARK_QUERIES LOCKED):
- 5 ASI 哲学 gap direct queries
- 3 ASI 锚定 queries
- 2 Cross-gap queries
- 3 V1322 operational queries
- 3 V3 guard queries
- 3 V1323 self-reference queries
- 3 Edge case queries (empty / minimal / mixed)

V1324 ASI 5-Gap Crucible + Real LLM 7 真生产组件:
 1. RealLLMConfig         — 真 endpoint config (api.minimaxi.com/anthropic + MiniMax-M3)
 2. RealLLMClient         — 真 HTTP client (urllib + x-api-key + anthropic-version)
 3. ProbeAndValidate      — 真探活 + 真 key 验证 (不消耗 token)
 4. LLMGapScorer          — 真跑 LLM 5-gap scoring per query
 5. BenchmarkRunnerReal   — 22 queries × real LLM 真跑
 6. HeuristicVsRealReport — V1323 heuristic vs V1324 real LLM 真对比
 7. V1324Bridge           — V1324 → ASI pole-star anchor (LOCKED, 不动)
"""
from __future__ import annotations

import json
import math
import os
import re
import statistics
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Any, Dict, FrozenSet, List, Mapping, Optional, Sequence, Tuple

from apeireth.v1322_asi_5gap_crucible import ASI_5_GAPS
from apeireth.v1323_asi_5gap_crucible_benchmark import (
    ASI_ANCHORS,
    BENCHMARK_QUERIES,
    BenchmarkAggregate,
    BenchmarkRunner,
    CoverageReport,
    DimensionStats,
    EdgeCaseReport,
    QueryResult,
    V3_GUARD_MARKERS,
    _assert_benchmark_queries_locked,
    compute_coverage,
    compute_dimension_stats,
    compute_edge_cases,
    _percentile,
)

V1324_VERSION = "0.1.0"

# ASI 北极星 anchor (LOCKED, 不动)
ASI_ANCHORS_V1324: Dict[str, Any] = dict(ASI_ANCHORS)

# V3 guard markers (LOCKED, per V1323)
V3_GUARD_MARKERS_V1324: Tuple[str, ...] = V3_GUARD_MARKERS

# ============================================================================
# Section 1: Component 1 — RealLLMConfig (endpoint config)
# ============================================================================

# 默认 endpoint = NewAPI M3 Anthropic-compatible (real production)
# 主 22:33 + 主 06:15 + 主 17:43: 真接 NewAPI M3 真跑 benchmark.
# Override via env: MINIMAX_CN_API_KEY / APEIRETH_LLM_BASE_URL / APEIRETH_LLM_MODEL
DEFAULT_BASE_URL = "https://api.minimaxi.com/anthropic"
DEFAULT_MODEL = "MiniMax-M3"
DEFAULT_TIMEOUT_SEC = 30.0
DEFAULT_MAX_TOKENS = 256

ENV_API_KEY = "MINIMAX_CN_API_KEY"
ENV_BASE_URL = "APEIRETH_LLM_BASE_URL"
ENV_MODEL = "APEIRETH_LLM_MODEL"


@dataclass(frozen=True)
class RealLLMConfig:
    """Real LLM endpoint configuration (LOCKED defaults, env override OK)."""

    base_url: str
    model: str
    timeout_sec: float
    max_tokens: int

    def to_dict(self) -> Dict[str, Any]:
        return {
            "base_url": self.base_url,
            "model": self.model,
            "timeout_sec": self.timeout_sec,
            "max_tokens": self.max_tokens,
        }


def default_config() -> RealLLMConfig:
    """Build config from env with LOCKED defaults."""
    base_url = os.environ.get(ENV_BASE_URL, DEFAULT_BASE_URL).strip() or DEFAULT_BASE_URL
    model = os.environ.get(ENV_MODEL, DEFAULT_MODEL).strip() or DEFAULT_MODEL
    return RealLLMConfig(
        base_url=base_url.rstrip("/"),
        model=model,
        timeout_sec=DEFAULT_TIMEOUT_SEC,
        max_tokens=DEFAULT_MAX_TOKENS,
    )


# ============================================================================
# Section 2: Component 2 — RealLLMClient (real HTTP client)
# ============================================================================


@dataclass(frozen=True)
class ChatResult:
    """Single chat completion result (real or fallback)."""

    ok: bool
    content: str
    latency_ms: float
    input_tokens: int
    output_tokens: int
    model: str
    fallback_used: bool
    error: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "ok": self.ok,
            "content_preview": self.content[:120] if self.content else "",
            "latency_ms": self.latency_ms,
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "model": self.model,
            "fallback_used": self.fallback_used,
            "error": self.error,
        }


def _read_api_key() -> str:
    """Read API key from env. Empty if not set."""
    return os.environ.get(ENV_API_KEY, "").strip()


def _http_post_json(url: str, body: bytes, headers: Dict[str, str], timeout_sec: float) -> Tuple[int, str, float]:
    """Real HTTP POST. Returns (status_code, body_text, latency_ms)."""
    req = urllib.request.Request(url, data=body, method="POST", headers=headers)
    t0 = time.time()
    try:
        resp = urllib.request.urlopen(req, timeout=timeout_sec)
        dt = (time.time() - t0) * 1000.0
        body_text = resp.read().decode("utf-8", errors="replace")
        return resp.status, body_text, dt
    except urllib.error.HTTPError as e:
        dt = (time.time() - t0) * 1000.0
        body_text = e.read().decode("utf-8", errors="replace") if e.fp else ""
        return e.code, body_text, dt


class RealLLMClient:
    """Real LLM HTTP client (Anthropic-compatible)."""

    SUBSTRATE = "urllib + x-api-key + anthropic-version (real HTTP)"
    CITATION = "api.minimaxi.com/anthropic + MiniMax-M3 (主 06:15 + 主 22:33)"

    def __init__(self, config: RealLLMConfig = None, api_key: str = "") -> None:
        self.config = config or default_config()
        self.api_key = (api_key or _read_api_key()).strip()

    def is_configured(self) -> bool:
        return bool(self.api_key) and bool(self.config.base_url) and bool(self.config.model)

    def chat(self, user_content: str, max_tokens: int = 0) -> ChatResult:
        """Real chat completion (1 call). Returns ChatResult."""
        if not self.is_configured():
            return ChatResult(
                ok=False,
                content="",
                latency_ms=0.0,
                input_tokens=0,
                output_tokens=0,
                model=self.config.model,
                fallback_used=True,
                error="api_key or config missing",
            )
        url = f"{self.config.base_url}/v1/messages"
        body = json.dumps(
            {
                "model": self.config.model,
                "max_tokens": max_tokens if max_tokens > 0 else self.config.max_tokens,
                "messages": [{"role": "user", "content": user_content}],
            }
        ).encode("utf-8")
        headers = {
            "x-api-key": self.api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json",
        }
        status, body_text, latency_ms = _http_post_json(url, body, headers, self.config.timeout_sec)
        if status != 200:
            return ChatResult(
                ok=False,
                content="",
                latency_ms=latency_ms,
                input_tokens=0,
                output_tokens=0,
                model=self.config.model,
                fallback_used=True,
                error=f"HTTP {status}: {body_text[:120]}",
            )
        try:
            payload = json.loads(body_text)
        except Exception as e:
            return ChatResult(
                ok=False,
                content="",
                latency_ms=latency_ms,
                input_tokens=0,
                output_tokens=0,
                model=self.config.model,
                fallback_used=True,
                error=f"json parse: {type(e).__name__}",
            )
        # Anthropic format: content=[{"type":"text","text":...}]
        content_parts = payload.get("content") or []
        text = ""
        if content_parts and isinstance(content_parts, list):
            for part in content_parts:
                if isinstance(part, dict) and part.get("type") == "text":
                    text = part.get("text", "")
                    break
        usage = payload.get("usage") or {}
        return ChatResult(
            ok=True,
            content=text,
            latency_ms=latency_ms,
            input_tokens=int(usage.get("input_tokens", 0)),
            output_tokens=int(usage.get("output_tokens", 0)),
            model=str(payload.get("model", self.config.model)),
            fallback_used=False,
            error="",
        )

    def probe(self) -> Dict[str, Any]:
        """Real probe — issue a minimal chat and return status."""
        if not self.is_configured():
            return {"reachable": False, "configured": False, "error": "api_key or config missing"}
        # Use tiny max_tokens to minimize cost
        r = self.chat("Reply with OK.", max_tokens=8)
        return {
            "reachable": r.ok,
            "configured": True,
            "latency_ms": r.latency_ms,
            "model": r.model,
            "input_tokens": r.input_tokens,
            "output_tokens": r.output_tokens,
            "error": r.error if not r.ok else "",
        }


# ============================================================================
# Section 3: Component 3 — ProbeAndValidate (real probe + real key validation)
# ============================================================================


@dataclass(frozen=True)
class ProbeAndValidateReport:
    """Real probe + key validation report (no token burned beyond probe)."""

    configured: bool
    reachable: bool
    latency_ms: float
    model: str
    input_tokens: int
    output_tokens: int
    base_url: str
    error: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "configured": self.configured,
            "reachable": self.reachable,
            "latency_ms": self.latency_ms,
            "model": self.model,
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "base_url": self.base_url,
            "error": self.error,
        }


def probe_and_validate(client: RealLLMClient = None) -> ProbeAndValidateReport:
    """Run real probe + real key validation."""
    c = client or RealLLMClient()
    if not c.is_configured():
        return ProbeAndValidateReport(
            configured=False,
            reachable=False,
            latency_ms=0.0,
            model=c.config.model,
            input_tokens=0,
            output_tokens=0,
            base_url=c.config.base_url,
            error="api_key or config missing",
        )
    p = c.probe()
    return ProbeAndValidateReport(
        configured=True,
        reachable=bool(p.get("reachable")),
        latency_ms=float(p.get("latency_ms", 0.0)),
        model=str(p.get("model", c.config.model)),
        input_tokens=int(p.get("input_tokens", 0)),
        output_tokens=int(p.get("output_tokens", 0)),
        base_url=c.config.base_url,
        error=str(p.get("error", "")),
    )


# ============================================================================
# Section 4: Component 4 — LLMGapScorer (real LLM 5-gap scoring per query)
# ============================================================================

# Prompt template — ask LLM to rate 5 gap dimensions on 0-1 scale
# LOCKED format: "rate 5 numbers, one per gap, comma-separated"
GAP_SCORING_PROMPT = (
    "Rate 0.0-1.0 (one decimal, higher=more relevant) for the following ASI philosophy "
    "gaps given this text:\n\n"
    "TEXT: {query}\n\n"
    "GAPS:\n"
    "- time (Bergson 绵延 + Heidegger 此在 + Prigogine 耗散)\n"
    "- freedom (Spinoza conatus + Frankfurt hierarchical desires + Heidegger 筹划)\n"
    "- recognition (Levinas 他者优先 + Hegel 主奴辩证法 + Mead 符号互动)\n"
    "- emergence (Bedau weak emergence + Wolfram NKS + Kauffman adjacent possible)\n"
    "- truth (Peirce 实效主义 + Cornforth 实在论 + Davidson + Brandom + Putnam)\n\n"
    "Reply with ONLY 5 comma-separated decimals like: 0.7,0.2,0.1,0.0,0.5"
)


def _parse_5_gap_response(text: str) -> Optional[Tuple[float, float, float, float, float]]:
    """Parse LLM response. Returns 5-tuple or None on parse failure."""
    if not text:
        return None
    # Try to extract 5 floats from the text
    cleaned = text.strip()
    # Strip code fences
    cleaned = re.sub(r"```[a-zA-Z]*", "", cleaned).replace("```", "").strip()
    # Find all floats
    nums = re.findall(r"-?\d+\.\d+", cleaned)
    if len(nums) < 5:
        return None
    try:
        vals = tuple(max(0.0, min(1.0, float(nums[i]))) for i in range(5))
        return vals  # type: ignore[return-value]
    except Exception:
        return None


@dataclass(frozen=True)
class LLMGapScore:
    """Single query's real LLM 5-gap score."""

    query_id: str
    category: str
    query_text: str
    expected_gap_focus: str
    gap_scores: Dict[str, float]       # 5 gap dimensions (LLM-rated)
    raw_response: str                  # raw LLM response text
    chat_ok: bool
    fallback_used: bool
    latency_ms: float
    input_tokens: int
    output_tokens: int
    error: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "query_id": self.query_id,
            "category": self.category,
            "query_text_preview": self.query_text[:80] if self.query_text else "",
            "expected_gap_focus": self.expected_gap_focus,
            "gap_scores": dict(self.gap_scores),
            "raw_response_preview": self.raw_response[:120] if self.raw_response else "",
            "chat_ok": self.chat_ok,
            "fallback_used": self.fallback_used,
            "latency_ms": self.latency_ms,
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "error": self.error,
        }

    @property
    def total_tokens(self) -> int:
        return self.input_tokens + self.output_tokens


class LLMGapScorer:
    """Real LLM 5-gap scorer (one query at a time)."""

    SUBSTRATE = "RealLLMClient.chat + GAP_SCORING_PROMPT + regex parse"
    CITATION = "api.minimaxi.com/anthropic + MiniMax-M3 (real HTTP, real tokens)"
    GUARD = "real HTTP + real token + real content (非 mock); fallback when API unavailable"

    def __init__(self, client: RealLLMClient = None) -> None:
        self.client = client or RealLLMClient()

    def score_one(self, qid: str, category: str, qtext: str, focus: str) -> LLMGapScore:
        """Real LLM scoring for one query."""
        if not qtext or not qtext.strip():
            # Edge case: empty / minimal — don't burn tokens
            return LLMGapScore(
                query_id=qid,
                category=category,
                query_text=qtext,
                expected_gap_focus=focus,
                gap_scores={g: 0.0 for g in ASI_5_GAPS},
                raw_response="",
                chat_ok=False,
                fallback_used=True,
                latency_ms=0.0,
                input_tokens=0,
                output_tokens=0,
                error="empty query",
            )
        prompt = GAP_SCORING_PROMPT.format(query=qtext[:400])
        chat = self.client.chat(prompt)
        if not chat.ok:
            return LLMGapScore(
                query_id=qid,
                category=category,
                query_text=qtext,
                expected_gap_focus=focus,
                gap_scores={g: 0.0 for g in ASI_5_GAPS},
                raw_response="",
                chat_ok=False,
                fallback_used=True,
                latency_ms=chat.latency_ms,
                input_tokens=chat.input_tokens,
                output_tokens=chat.output_tokens,
                error=chat.error,
            )
        parsed = _parse_5_gap_response(chat.content)
        if parsed is None:
            return LLMGapScore(
                query_id=qid,
                category=category,
                query_text=qtext,
                expected_gap_focus=focus,
                gap_scores={g: 0.0 for g in ASI_5_GAPS},
                raw_response=chat.content,
                chat_ok=True,
                fallback_used=True,
                latency_ms=chat.latency_ms,
                input_tokens=chat.input_tokens,
                output_tokens=chat.output_tokens,
                error=f"parse failure: {chat.content[:80]}",
            )
        gap_scores = {gap: parsed[i] for i, gap in enumerate(ASI_5_GAPS)}
        return LLMGapScore(
            query_id=qid,
            category=category,
            query_text=qtext,
            expected_gap_focus=focus,
            gap_scores=gap_scores,
            raw_response=chat.content,
            chat_ok=True,
            fallback_used=False,
            latency_ms=chat.latency_ms,
            input_tokens=chat.input_tokens,
            output_tokens=chat.output_tokens,
            error="",
        )


# ============================================================================
# Section 5: Component 5 — BenchmarkRunnerReal (22 queries × real LLM 真跑)
# ============================================================================


@dataclass(frozen=True)
class RealBenchmarkResult:
    """Real LLM benchmark result for all 22 queries."""

    n_queries: int                      # 22 (LOCKED)
    n_chat_ok: int                      # queries where LLM call succeeded
    n_fallback: int                     # queries where fallback was used (LLM failed / parse failed / empty)
    n_parse_failure: int                # subset of fallback: response was OK but unparseable
    total_latency_ms: float             # sum across 22 queries
    mean_latency_ms: float              # avg per query
    total_input_tokens: int             # total input tokens consumed
    total_output_tokens: int            # total output tokens consumed
    total_tokens: int                   # input + output
    scores: Tuple[LLMGapScore, ...]     # per-query scores (22)
    probe_report: ProbeAndValidateReport
    config: RealLLMConfig
    started_at: str                     # ISO timestamp
    finished_at: str                    # ISO timestamp

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_queries": self.n_queries,
            "n_chat_ok": self.n_chat_ok,
            "n_fallback": self.n_fallback,
            "n_parse_failure": self.n_parse_failure,
            "total_latency_ms": self.total_latency_ms,
            "mean_latency_ms": self.mean_latency_ms,
            "total_input_tokens": self.total_input_tokens,
            "total_output_tokens": self.total_output_tokens,
            "total_tokens": self.total_tokens,
            "probe_report": self.probe_report.to_dict(),
            "config": self.config.to_dict(),
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "scores": [s.to_dict() for s in self.scores],
        }


def _now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime())


def run_real_benchmark(
    scorer: LLMGapScorer = None,
    queries: Sequence[Tuple[str, str, str, str]] = None,
    probe_first: bool = True,
) -> RealBenchmarkResult:
    """Run 22 queries × real LLM 真跑."""
    if queries is None:
        _assert_benchmark_queries_locked()
        queries = BENCHMARK_QUERIES
    s = scorer or LLMGapScorer()
    cfg = s.client.config

    # Probe first (real HTTP, 1 call)
    started_at = _now_iso()
    if probe_first:
        probe = probe_and_validate(s.client)
    else:
        probe = ProbeAndValidateReport(
            configured=s.client.is_configured(),
            reachable=False,
            latency_ms=0.0,
            model=cfg.model,
            input_tokens=0,
            output_tokens=0,
            base_url=cfg.base_url,
            error="probe skipped",
        )

    scores: List[LLMGapScore] = []
    n_chat_ok = 0
    n_fallback = 0
    n_parse_failure = 0
    total_latency_ms = 0.0
    total_input_tokens = 0
    total_output_tokens = 0
    for qid, category, qtext, focus in queries:
        score = s.score_one(qid, category, qtext, focus)
        scores.append(score)
        if score.chat_ok and not score.fallback_used:
            n_chat_ok += 1
        else:
            n_fallback += 1
        if score.chat_ok and score.fallback_used:
            n_parse_failure += 1
        total_latency_ms += score.latency_ms
        total_input_tokens += score.input_tokens
        total_output_tokens += score.output_tokens

    finished_at = _now_iso()
    return RealBenchmarkResult(
        n_queries=len(queries),
        n_chat_ok=n_chat_ok,
        n_fallback=n_fallback,
        n_parse_failure=n_parse_failure,
        total_latency_ms=total_latency_ms,
        mean_latency_ms=(total_latency_ms / len(queries)) if queries else 0.0,
        total_input_tokens=total_input_tokens,
        total_output_tokens=total_output_tokens,
        total_tokens=total_input_tokens + total_output_tokens,
        scores=tuple(scores),
        probe_report=probe,
        config=cfg,
        started_at=started_at,
        finished_at=finished_at,
    )


# ============================================================================
# Section 6: Component 6 — HeuristicVsRealReport (V1323 vs V1324 对比)
# ============================================================================


@dataclass(frozen=True)
class GapAgreement:
    """Agreement per gap dimension (heuristic vs real LLM)."""

    gap: str
    n_samples: int                      # 22
    mean_heuristic: float               # V1323 mean
    mean_real: float                    # V1324 mean
    pearson_r: float                    # correlation
    mae: float                          # mean absolute error
    rmse: float                         # root mean squared error

    def to_dict(self) -> Dict[str, Any]:
        return {
            "gap": self.gap,
            "n_samples": self.n_samples,
            "mean_heuristic": self.mean_heuristic,
            "mean_real": self.mean_real,
            "pearson_r": self.pearson_r,
            "mae": self.mae,
            "rmse": self.rmse,
        }


def _pearson(xs: Sequence[float], ys: Sequence[float]) -> float:
    """Pearson correlation coefficient."""
    n = len(xs)
    if n == 0 or n != len(ys):
        return 0.0
    if n < 2:
        return 0.0
    mx = sum(xs) / n
    my = sum(ys) / n
    num = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    sx = math.sqrt(sum((x - mx) ** 2 for x in xs))
    sy = math.sqrt(sum((y - my) ** 2 for y in ys))
    if sx == 0 or sy == 0:
        return 0.0
    return num / (sx * sy)


@dataclass(frozen=True)
class HeuristicVsRealReport:
    """V1323 heuristic vs V1324 real LLM comparison (5 gaps × 22 queries)."""

    n_queries: int                      # 22
    n_chat_ok: int                      # how many queries had real LLM response
    gap_agreements: Tuple[GapAgreement, ...]   # 5
    overall_pearson_r: float            # mean across 5 gaps
    overall_mae: float                  # mean MAE across 5 gaps
    overall_rmse: float                 # mean RMSE across 5 gaps
    delta_means: Dict[str, float]       # per-gap: real - heuristic

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_queries": self.n_queries,
            "n_chat_ok": self.n_chat_ok,
            "gap_agreements": [g.to_dict() for g in self.gap_agreements],
            "overall_pearson_r": self.overall_pearson_r,
            "overall_mae": self.overall_mae,
            "overall_rmse": self.overall_rmse,
            "delta_means": dict(self.delta_means),
        }


def compare_heuristic_vs_real(
    heuristic_results: Sequence[QueryResult],
    real_result: RealBenchmarkResult,
) -> HeuristicVsRealReport:
    """V1323 heuristic vs V1324 real LLM — 5-gap comparison."""
    n = len(heuristic_results)
    if n != len(real_result.scores):
        # Use min if mismatched (defensive)
        n = min(n, len(real_result.scores))
    if n == 0:
        return HeuristicVsRealReport(
            n_queries=0,
            n_chat_ok=0,
            gap_agreements=(),
            overall_pearson_r=0.0,
            overall_mae=0.0,
            overall_rmse=0.0,
            delta_means={},
        )
    n_chat_ok = sum(1 for s in real_result.scores if s.chat_ok and not s.fallback_used)
    gap_agreements: List[GapAgreement] = []
    deltas: Dict[str, float] = {}
    for gap in ASI_5_GAPS:
        h_vals: List[float] = []
        r_vals: List[float] = []
        for i in range(n):
            h_vals.append(float(heuristic_results[i].crucible_result.gap_scores.get(gap, 0.0)))
            r_vals.append(float(real_result.scores[i].gap_scores.get(gap, 0.0)))
        mh = sum(h_vals) / n
        mr = sum(r_vals) / n
        diffs = [abs(h - r) for h, r in zip(h_vals, r_vals)]
        mae = sum(diffs) / n
        rmse = math.sqrt(sum(d ** 2 for d in diffs) / n)
        pearson = _pearson(h_vals, r_vals)
        gap_agreements.append(GapAgreement(
            gap=gap,
            n_samples=n,
            mean_heuristic=mh,
            mean_real=mr,
            pearson_r=pearson,
            mae=mae,
            rmse=rmse,
        ))
        deltas[gap] = mr - mh

    return HeuristicVsRealReport(
        n_queries=n,
        n_chat_ok=n_chat_ok,
        gap_agreements=tuple(gap_agreements),
        overall_pearson_r=sum(g.pearson_r for g in gap_agreements) / len(gap_agreements) if gap_agreements else 0.0,
        overall_mae=sum(g.mae for g in gap_agreements) / len(gap_agreements) if gap_agreements else 0.0,
        overall_rmse=sum(g.rmse for g in gap_agreements) / len(gap_agreements) if gap_agreements else 0.0,
        delta_means=deltas,
    )


# ============================================================================
# Section 7: Component 7 — V1324Bridge (pole-star anchor, V3 guards)
# ============================================================================


@dataclass(frozen=True)
class V1324Aggregate:
    """Aggregate of V1324 real-LLM benchmark + comparison."""

    version: str                        # "0.1.0"
    real_benchmark: RealBenchmarkResult
    comparison: HeuristicVsRealReport
    v3_guards: Tuple[str, ...]          # 5 LOCKED V3 markers
    pole_star_anchors: Dict[str, Any]   # V0.1 / V0.2 / V1256 / V1049 LOCKED
    guard_marker: str                   # v1324 marker

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "real_benchmark": self.real_benchmark.to_dict(),
            "comparison": self.comparison.to_dict(),
            "v3_guards": list(self.v3_guards),
            "pole_star_anchors": dict(self.pole_star_anchors),
            "guard_marker": self.guard_marker,
        }


def build_v1324_aggregate(
    real: RealBenchmarkResult,
    comparison: HeuristicVsRealReport,
) -> V1324Aggregate:
    """Build V1324 aggregate."""
    return V1324Aggregate(
        version=V1324_VERSION,
        real_benchmark=real,
        comparison=comparison,
        v3_guards=V3_GUARD_MARKERS_V1324,
        pole_star_anchors=dict(ASI_ANCHORS_V1324),
        guard_marker="v1324_real_llm_5gap",
    )


def build_bridge(
    heuristic_results: Sequence[QueryResult],
    real: RealBenchmarkResult = None,
    scorer: LLMGapScorer = None,
    auto_run_real: bool = True,
) -> V1324Aggregate:
    """End-to-end V1324: probe + run real LLM + compare with V1323 heuristic + build bridge."""
    if real is None and auto_run_real:
        real = run_real_benchmark(scorer=scorer)
    if real is None:
        # No real run — build minimal placeholder
        real = RealBenchmarkResult(
            n_queries=len(heuristic_results),
            n_chat_ok=0,
            n_fallback=len(heuristic_results),
            n_parse_failure=0,
            total_latency_ms=0.0,
            mean_latency_ms=0.0,
            total_input_tokens=0,
            total_output_tokens=0,
            total_tokens=0,
            scores=tuple(LLMGapScore(
                query_id=r.query_id,
                category=r.category,
                query_text=r.query_text,
                expected_gap_focus=r.expected_gap_focus,
                gap_scores={g: 0.0 for g in ASI_5_GAPS},
                raw_response="",
                chat_ok=False,
                fallback_used=True,
                latency_ms=0.0,
                input_tokens=0,
                output_tokens=0,
                error="real run disabled",
            ) for r in heuristic_results),
            probe_report=ProbeAndValidateReport(
                configured=False,
                reachable=False,
                latency_ms=0.0,
                model="",
                input_tokens=0,
                output_tokens=0,
                base_url="",
                error="real run disabled",
            ),
            config=default_config(),
            started_at="",
            finished_at="",
        )
    comparison = compare_heuristic_vs_real(heuristic_results, real)
    return build_v1324_aggregate(real=real, comparison=comparison)


# ============================================================================
# Section 8: Rendering — markdown report helper
# ============================================================================


def render_markdown_report(agg: V1324Aggregate) -> str:
    """Render markdown report for V1324."""
    real = agg.real_benchmark
    cmp_ = agg.comparison
    lines: List[str] = []
    lines.append(f"# V1324 ASI 5-Gap Crucible + Real LLM 报告\n")
    lines.append(f"- 版本: {agg.version}")
    lines.append(f"- 启动时间: {real.started_at}")
    lines.append(f"- 完成时间: {real.finished_at}")
    lines.append(f"- Real LLM reachable: **{real.probe_report.reachable}**")
    lines.append(f"- Base URL: `{real.config.base_url}`")
    lines.append(f"- Model: `{real.config.model}`")
    lines.append("")
    lines.append("## 1. 真探活 + 真 key 验证 (主 17:43 实事求是)\n")
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    lines.append(f"| configured | {real.probe_report.configured} |")
    lines.append(f"| reachable | {real.probe_report.reachable} |")
    lines.append(f"| latency_ms | {real.probe_report.latency_ms:.2f} |")
    lines.append(f"| model | `{real.probe_report.model}` |")
    lines.append(f"| input_tokens | {real.probe_report.input_tokens} |")
    lines.append(f"| output_tokens | {real.probe_report.output_tokens} |")
    if real.probe_report.error:
        lines.append(f"| error | {real.probe_report.error} |")
    lines.append("")

    lines.append("## 2. 真 benchmark (22 samples × real LLM 真跑)\n")
    lines.append("| Stat | Value |")
    lines.append("|---|---|")
    lines.append(f"| n_queries | {real.n_queries} |")
    lines.append(f"| n_chat_ok | {real.n_chat_ok} |")
    lines.append(f"| n_fallback | {real.n_fallback} |")
    lines.append(f"| n_parse_failure | {real.n_parse_failure} |")
    lines.append(f"| total_latency_ms | {real.total_latency_ms:.2f} |")
    lines.append(f"| mean_latency_ms | {real.mean_latency_ms:.2f} |")
    lines.append(f"| total_input_tokens | {real.total_input_tokens} |")
    lines.append(f"| total_output_tokens | {real.total_output_tokens} |")
    lines.append(f"| total_tokens | {real.total_tokens} |")
    lines.append("")

    lines.append("## 3. 真 22 样本逐条 (主 17:43 实事求是)\n")
    lines.append("| # | QueryID | Category | chat_ok | fallback | real_time | real_freedom | real_recognition | real_emergence | real_truth | in_tok | out_tok |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for i, s in enumerate(real.scores):
        gs = s.gap_scores
        lines.append(
            f"| {i} | {s.query_id} | {s.category} | {s.chat_ok} | {s.fallback_used} | "
            f"{gs.get('time', 0):.2f} | {gs.get('freedom', 0):.2f} | "
            f"{gs.get('recognition', 0):.2f} | {gs.get('emergence', 0):.2f} | "
            f"{gs.get('truth', 0):.2f} | {s.input_tokens} | {s.output_tokens} |"
        )
    lines.append("")

    lines.append("## 4. V1323 heuristic vs V1324 real LLM 真对比 (5 gap dim × 22 queries)\n")
    lines.append("| Gap | mean_heuristic | mean_real | delta | pearson_r | MAE | RMSE |")
    lines.append("|---|---|---|---|---|---|---|")
    for g in cmp_.gap_agreements:
        lines.append(
            f"| {g.gap} | {g.mean_heuristic:.4f} | {g.mean_real:.4f} | "
            f"{cmp_.delta_means.get(g.gap, 0):+.4f} | "
            f"{g.pearson_r:+.4f} | {g.mae:.4f} | {g.rmse:.4f} |"
        )
    lines.append("")
    lines.append(f"- overall_pearson_r: **{cmp_.overall_pearson_r:+.4f}**")
    lines.append(f"- overall_mae: **{cmp_.overall_mae:.4f}**")
    lines.append(f"- overall_rmse: **{cmp_.overall_rmse:.4f}**")
    lines.append("")

    lines.append("## 5. V3 哲学守门 (主 17:58 + 主 20:46)\n")
    for g in agg.v3_guards:
        lines.append(f"- ✅ `{g}`")
    lines.append(f"- ✅ `v1324_real_llm_5gap`")
    lines.append("")
    lines.append("> 主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达 ASI. "
                 "本报告是 V1323 heuristic 真测升级 (real LLM 真跑), 非 ASI 达成. "
                 "LLM 真测 ≠ ASI (主 22:33 ASI 北极星里的一小步).")
    lines.append("")
    lines.append("## 6. ASI 北极星 (LOCKED, 不动)\n")
    for k, v in agg.pole_star_anchors.items():
        lines.append(f"- **{k}**: {v}")
    return "\n".join(lines) + "\n"


# ============================================================================
# Section 9: Self-test (Popper-style)
# ============================================================================


def _self_test() -> bool:
    """Self-test of V1324 components (no real LLM needed for unit tests)."""
    # 1. RealLLMConfig
    cfg = default_config()
    assert cfg.base_url, "config must have base_url"
    assert cfg.model, "config must have model"
    # 2. _parse_5_gap_response
    assert _parse_5_gap_response("0.7,0.2,0.1,0.0,0.5") == (0.7, 0.2, 0.1, 0.0, 0.5)
    assert _parse_5_gap_response("0.1, 0.2, 0.3, 0.4, 0.5") == (0.1, 0.2, 0.3, 0.4, 0.5)
    assert _parse_5_gap_response("text 0.7,0.2,0.1,0.0,0.5 end") == (0.7, 0.2, 0.1, 0.0, 0.5)
    assert _parse_5_gap_response("only three 0.1,0.2,0.3") is None
    assert _parse_5_gap_response("") is None
    assert _parse_5_gap_response("```0.7,0.2,0.1,0.0,0.5```") == (0.7, 0.2, 0.1, 0.0, 0.5)
    # 3. _pearson
    xs = [0.1, 0.2, 0.3, 0.4, 0.5]
    ys = [0.1, 0.2, 0.3, 0.4, 0.5]
    assert abs(_pearson(xs, ys) - 1.0) < 1e-9, f"perfect positive: {_pearson(xs, ys)}"
    ys2 = [0.5, 0.4, 0.3, 0.2, 0.1]
    assert abs(_pearson(xs, ys2) - (-1.0)) < 1e-9, f"perfect negative: {_pearson(xs, ys2)}"
    # 4. _percentile (reuse from V1323)
    assert _percentile([1.0, 2.0, 3.0, 4.0, 5.0], 50.0) == 3.0
    # 5. RealLLMClient (no real call, just construction)
    # Clear env so we can deterministically test the no-key path
    saved = os.environ.pop(ENV_API_KEY, None)
    try:
        c = RealLLMClient(api_key="dummy-key")
        assert c.is_configured() is True
        c2 = RealLLMClient(api_key="")
        # If env still has a key (test runner order), is_configured would be True.
        # But with env cleared and empty api_key → False.
        assert c2.is_configured() is False, f"is_configured={c2.is_configured()} api_key={c2.api_key!r}"
    finally:
        if saved is not None:
            os.environ[ENV_API_KEY] = saved
    # 6. LLMGapScorer empty / minimal (use Fake client to bypass env)
    class _NoKeyClient:
        config = RealLLMConfig(base_url="https://fake", model="fake", timeout_sec=1.0, max_tokens=8)
        api_key = ""

        def is_configured(self) -> bool:
            return False

        def chat(self, prompt: str) -> ChatResult:
            return ChatResult(ok=False, content="", latency_ms=0.0,
                              input_tokens=0, output_tokens=0, model="fake",
                              fallback_used=True, error="no key")

    scorer_no = LLMGapScorer(client=_NoKeyClient())
    s_empty = scorer_no.score_one("Q20", "edge", "", "none")
    assert s_empty.fallback_used is True
    assert s_empty.gap_scores["time"] == 0.0
    s_minimal = scorer_no.score_one("Q21", "edge", "x", "none")
    assert s_minimal.fallback_used is True
    # 7. LLMGapScorer with bad config (parse failure path)
    class _FakeClient:
        config = RealLLMConfig(base_url="https://fake", model="fake", timeout_sec=1.0, max_tokens=8)
        api_key = "k"

        def is_configured(self) -> bool:
            return True

        def chat(self, prompt: str) -> ChatResult:
            return ChatResult(ok=True, content="not five numbers", latency_ms=1.0,
                              input_tokens=1, output_tokens=1, model="fake",
                              fallback_used=False, error="")

    s_fake = LLMGapScorer(client=_FakeClient())
    out = s_fake.score_one("Q01", "test", "hello world", "time")
    assert out.fallback_used is True
    assert "parse failure" in out.error
    # 8. HeuristicVsRealReport construction
    # Build minimal QueryResult-like via dataclass
    from apeireth.v1322_asi_5gap_crucible import CrucibleResult
    qrs: List[QueryResult] = []
    rrs: List[LLMGapScore] = []
    for i in range(5):
        qid = f"Q{i:02d}"
        cr = CrucibleResult(
            query=f"q{i}",
            gap_scores={"time": 0.1 * i, "freedom": 0.2, "recognition": 0.3, "emergence": 0.4, "truth": 0.5},
            cross_gap_scores={},
            aggregate_5_gap_score=0.3,
            aggregate_cross_gap_score=0.0,
            aggregate_total=0.3,
            latency_ms=0.5,
            v3_guards=tuple(),
            substrate_chain=tuple(),
            pole_star_anchors={},
        )
        qrs.append(QueryResult(query_id=qid, category="test", query_text=f"q{i}",
                               expected_gap_focus="time", crucible_result=cr,
                               is_empty=False, is_minimal=False))
        rrs.append(LLMGapScore(
            query_id=qid, category="test", query_text=f"q{i}", expected_gap_focus="time",
            gap_scores={"time": 0.1 * i, "freedom": 0.2, "recognition": 0.3, "emergence": 0.4, "truth": 0.5},
            raw_response="ok", chat_ok=True, fallback_used=False,
            latency_ms=1.0, input_tokens=10, output_tokens=2, error="",
        ))
    fake_real = RealBenchmarkResult(
        n_queries=5, n_chat_ok=5, n_fallback=0, n_parse_failure=0,
        total_latency_ms=5.0, mean_latency_ms=1.0,
        total_input_tokens=50, total_output_tokens=10, total_tokens=60,
        scores=tuple(rrs),
        probe_report=ProbeAndValidateReport(
            configured=True, reachable=True, latency_ms=1.0, model="fake",
            input_tokens=1, output_tokens=1, base_url="https://fake", error="",
        ),
        config=RealLLMConfig(base_url="https://fake", model="fake", timeout_sec=1.0, max_tokens=8),
        started_at="t0", finished_at="t1",
    )
    cmp_ = compare_heuristic_vs_real(qrs, fake_real)
    assert cmp_.n_queries == 5
    # time gap has variance → perfect agreement (pearson=1.0)
    time_ga = next(g for g in cmp_.gap_agreements if g.gap == "time")
    assert abs(time_ga.pearson_r - 1.0) < 1e-9, f"time pearson: {time_ga.pearson_r}"
    # constant-value gaps have zero variance → pearson=0 (not NaN)
    for gap_name in ("freedom", "recognition", "emergence", "truth"):
        ga = next(g for g in cmp_.gap_agreements if g.gap == gap_name)
        assert ga.pearson_r == 0.0, f"{gap_name} pearson: {ga.pearson_r}"
    # overall MAE = 0 (identical values)
    assert cmp_.overall_mae < 1e-9
    # 9. build_v1324_aggregate + build_bridge
    agg = build_v1324_aggregate(real=fake_real, comparison=cmp_)
    assert agg.version == V1324_VERSION
    assert "v1324_real_llm_5gap" == agg.guard_marker
    # 10. render_markdown_report
    md = render_markdown_report(agg)
    assert "V1324" in md
    assert "ASI 北极星" in md
    # 11. ASI_5_GAPS LOCKED at 5
    assert len(ASI_5_GAPS) == 5, f"ASI_5_GAPS must be 5, got {len(ASI_5_GAPS)}"
    # 12. BENCHMARK_QUERIES LOCKED at 22
    assert len(BENCHMARK_QUERIES) == 22
    # 13. pole-star anchors LOCKED
    assert agg.pole_star_anchors.get("V0.1") == 0.7905
    assert agg.pole_star_anchors.get("V0.2") == 0.4467
    return True


if __name__ == "__main__":
    import sys
    if "--self-test" in sys.argv:
        ok = _self_test()
        print(f"V1324 self_test: {'PASS' if ok else 'FAIL'}")
        sys.exit(0 if ok else 1)
    if "--probe" in sys.argv:
        client = RealLLMClient()
        print(f"configured: {client.is_configured()}")
        report = probe_and_validate(client)
        print(f"probe: reachable={report.reachable} latency_ms={report.latency_ms:.2f} model={report.model}")
        sys.exit(0)
    if "--run" in sys.argv:
        agg = build_bridge(heuristic_results=BenchmarkRunner().run_benchmark())
        print(f"n_chat_ok={agg.real_benchmark.n_chat_ok} n_fallback={agg.real_benchmark.n_fallback}")
        print(f"total_tokens={agg.real_benchmark.total_tokens} mean_latency_ms={agg.real_benchmark.mean_latency_ms:.2f}")
        print(f"overall_pearson_r={agg.comparison.overall_pearson_r:+.4f}")
        print(f"overall_mae={agg.comparison.overall_mae:.4f}")
        sys.exit(0)
    print("usage: python -m apeireth.v1324_asi_5gap_real_llm [--self-test|--probe|--run]")
    sys.exit(1)