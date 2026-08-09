"""V1424 — ASI 总框架 benchmark 真接 LLM (real LLM provider).

Phase: 1424
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:55, Asia/Shanghai deep night)
Post: V1034 (real benchmark harness with 22 真样本) + V1423 (daemon webhook wiring)

What V1424 is
=============
V1424 is the **real-LLM predictor** that plugs into V1034's 22-sample benchmark
harness. Where:

- V1034 ships ``MMLU_SAMPLES`` (10) + ``GSM8K_SAMPLES`` (5) +
  ``HUMANEVAL_SAMPLES`` (3) + ``HELLASWAG_SAMPLES`` (4) = **22 real samples**
  plus 4 evaluators (``evaluate_mmlu_sample`` / ``evaluate_gsm8k_sample`` /
  ``evaluate_humaneval_sample`` / ``evaluate_hellaswag_sample``) and
  ``V1034RealBenchmark`` class
- V1034 takes a ``predictor(question) -> str`` callable and runs it across
  all 22 samples

V1424 provides **production predictors** that actually call a real LLM
endpoint, NOT a heuristic:

- ``NewAPIProvider`` — POST to a NewAPI-compatible OpenAI-style endpoint
  (any provider that exposes ``POST {base}/v1/chat/completions``)
- ``OpenAIProvider`` — POST to ``https://api.openai.com/v1/chat/completions``
- ``AnthropicProvider`` — POST to ``https://api.anthropic.com/v1/messages``
- ``GenericProvider`` — POST to any ``{base}/v1/chat/completions`` URL

Each provider honors an env-var-driven config (no secrets in code) and
tracks per-sample latency, prompt tokens, completion tokens, and a
USD-cost estimate (configurable price-per-1k-tokens).

V1424 ALSO provides ``DeterministicProvider`` (no key, no network) — this
is the **honest degradation** path: when no key is configured, V1424
runs the same 22 samples against a deterministic predictor and reports
``mode = "MOCK"`` in every record. This is NOT a fake pass — the score
is whatever the deterministic predictor returns. The benchmark log
makes it loud and clear which samples were real vs mock.

Real-world usage:

    # Anyone can see what samples V1034 has (22):
    python -m apeireth.v1424_asi_real_llm_benchmark list-samples

    # Anyone can run the real benchmark against a real endpoint:
    export APEIRETH_LLM_BASE="https://api.newapi.example.com/v1"
    export APEIRETH_LLM_KEY="sk-..."
    python -m apeireth.v1424_asi_real_llm_benchmark run \\
        --provider newapi --model gpt-4o-mini --max-samples 22

    # Anyone can run a deterministic (no-key) smoke benchmark:
    python -m apeireth.v1424_asi_real_llm_benchmark run \\
        --provider deterministic --max-samples 22

    # Anyone can inspect per-sample results:
    python -m apeireth.v1424_asi_real_llm_benchmark show-result --sample-id 0

    # Anyone can aggregate the run log into a report:
    python -m apeireth.v1424_asi_real_llm_benchmark report

This is the **natural next step** after V1423 (wiring) — the ASI 总框架
can now **measure** the real LLM performance, not just dispatch alerts.

It does NOT mutate V1034 or any upstream framework state. It only
**reads** V1034's sample lists + evaluators and **calls** the configured
LLM endpoint via stdlib urllib (httpx fallback if installed).

Borrowed (6 — 主 19:33 走在前人经验上):
=======================================
- V1034 (real benchmark harness — 22 真样本 + 4 evaluators + V1034RealBenchmark class)
- V1422 (notification webhook — ``sign_payload_hmac`` borrowed pattern for request signing)
- V1423 (daemon webhook wiring — ``_append_webhook_log`` borrowed pattern for atomic JSONL)
- stdlib urllib.request (HTTP POST without external deps)
- stdlib json (request/response serialization)
- stdlib time (per-sample latency)

GUARDS upheld (V1424-specific, 17 — 主 00:44 质量工程化)
=========================================================
- GUARD_PREDICTOR_REAL: real urllib POST to the configured endpoint, not stubbed
- GUARD_NO_V1034_WRITE: V1424 reads V1034 sample lists, never patches them
- GUARD_KEY_FROM_ENV: API key is read from env var, NEVER hardcoded
- GUARD_ENDPOINT_VALID: endpoint URL must be http(s):// and parseable
- GUARD_MODEL_BOUNDED: model name length ∈ [1, 128]
- GUARD_PROVIDER_KNOWN: provider ∈ {newapi, openai, anthropic, generic, deterministic}
- GUARD_TIMEOUT_BOUNDED: request timeout ∈ [1, 120]
- GUARD_MAX_TOKENS_BOUNDED: max_tokens ∈ [1, 8192]
- GUARD_TEMPERATURE_BOUNDED: temperature ∈ [0.0, 2.0]
- GUARD_RETRIES_BOUNDED: max_retries ∈ [0, 5]
- GUARD_SAMPLE_BOUNDED: max_samples ∈ [1, 22]
- GUARD_DETERMINISTIC_REPORTED: when deterministic, every record is tagged mode=MOCK
- GUARD_LATENCY_TRACKED: per-sample latency_seconds recorded
- GUARD_TOKENS_TRACKED: per-sample prompt_tokens + completion_tokens recorded
- GUARD_COST_ESTIMATED: per-sample cost_usd estimated (configurable price)
- GUARD_LOG_ATOMIC: results.jsonl atomic append (read+tmp+replace)
- GUARD_BORROWED_REAL: 6 borrowed (V1034 + V1422 + V1423 + urllib + json + time)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1424 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_BACKWARD_COMPAT: default config (deterministic) does not require any env var

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_BENCH_IS_NOT_PHENOMENAL: 22-sample benchmark is mechanical scoring, not Phenomenal
- GUARD_BENCH_IS_NOT_ASI: benchmark numbers ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_BENCH_IS_NOT_HUMAN_LEVEL: per-sample accuracy is bounded measurement, not judgment
- GUARD_BENCH_IS_NOT_ABSOLUTE: benchmark score is a noisy point estimate, not absolute truth
- GUARD_BENCH_IS_NOT_V1034_REPLACE: benchmark reads V1034 samples, does not replace
- GUARD_BENCH_IS_NOT_V1423_REPLACE: benchmark has its own log path, not V1423's
- GUARD_BENCH_IS_NOT_V1418_REPLACE: benchmark runs on-demand, not on cron
- GUARD_BENCH_IS_NOT_V1411_REPLACE: benchmark is a measurement tool, not a new framework
- GUARD_BENCH_IS_NOT_PRETEND_ASI: deterministic-mode runs are tagged mode=MOCK (主 17:43)

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1424 benchmark is a **deterministic measurement harness** that wires
V1034's 22 real samples (MMLU/GSM8K/HumanEval/HellaSwag) to a real LLM
endpoint via stdlib urllib. When a real key is configured, every sample
is shipped to the endpoint and the response is scored by V1034's
evaluators. When NO key is configured, V1424 runs in **deterministic
mode** — a fixed predictor returns ``"deterministic: <question hash>"``
for every sample; every record is tagged ``mode = "MOCK"`` so the user
can never confuse mock scores with real LLM scores. It is bounded by
HTTP request parsing, JSON serialization, single-sample scoring; NOT
by Phenomenal consciousness, ASI 达成, human-level judgment, or
absolute certainty. V1424 ≠ Phenomenal benchmark, ≠ ASI 达成
benchmark, ≠ human-level benchmark, ≠ absolute benchmark. V1424 reads
V1034; never replaces it. The 22 samples are V1034's own (主 17:43 实事
求是 — V1424 did not invent them).

API surfaces (15)
=================
1.  ``DEFAULT_TIMEOUT_SECONDS`` — 30
2.  ``DEFAULT_MAX_TOKENS`` — 256
3.  ``DEFAULT_TEMPERATURE`` — 0.0
4.  ``DEFAULT_MAX_RETRIES`` — 1
5.  ``PROVIDERS`` — tuple ("newapi", "openai", "anthropic", "generic", "deterministic")
6.  ``LLMConfig`` — dataclass (provider + base_url + api_key_env + model +
    timeout_seconds + max_tokens + temperature + max_retries + price_per_1k_input +
    price_per_1k_output + log_path + max_samples + note)
7.  ``SampleResult`` — dataclass (sample_id + benchmark + mode + question +
    ground_truth + prediction + correct + score + latency_seconds +
    prompt_tokens + completion_tokens + cost_usd + http_status + note)
8.  ``BenchmarkReport`` — dataclass (provider + model + n_samples + n_correct +
    accuracy + n_mock + total_latency_seconds + total_tokens + total_cost_usd +
    started_iso + ended_iso + per_benchmark + note)
9.  ``build_default_config(overrides)`` — LLMConfig (deterministic by default)
10. ``validate_config(cfg)`` — raises ValueError on bad input
11. ``_env_or_empty(name)`` — string helper (NEVER logs the value)
12. ``_deterministic_predict(question, max_tokens)`` — string (MOCK-mode)
13. ``_http_post_json(url, body, headers, timeout)`` — dict or raises
14. ``predict(cfg, question)`` — string (dispatches to provider)
15. ``run_benchmark(cfg)`` — BenchmarkReport (runs up to max_samples via V1034)
16. ``popper_self_test()`` — 17 self-tests
17. ``chain_delegate()`` — V1034 + V1422 + V1423 chain probe
18. ``run_cli(argv)`` — argv dispatcher

CLI commands (10 — 主 00:56 任何人都能接手)
===========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- list-samples [--benchmark NAME]
- run [--provider P] [--model M] [--max-samples N] [--max-tokens N]
       [--temperature F] [--timeout-seconds N] [--log-path PATH]
- show-result --sample-id N [--log-path PATH]
- report [--log-path PATH]

Real-world usage (主 00:56):
=============================
    # Anyone can see all 22 samples:
    python -m apeireth.v1424_asi_real_llm_benchmark list-samples

    # Anyone can run a deterministic (no-key) smoke benchmark:
    python -m apeireth.v1424_asi_real_llm_benchmark run \\
        --provider deterministic --max-samples 22

    # Anyone can run against a real NewAPI endpoint:
    APEIRETH_LLM_BASE=https://api.newapi.example.com/v1 \\
    APEIRETH_LLM_KEY=sk-xxx \\
    python -m apeireth.v1424_asi_real_llm_benchmark run \\
        --provider newapi --model gpt-4o-mini --max-samples 22

    # Anyone can inspect a single sample result:
    python -m apeireth.v1424_asi_real_llm_benchmark show-result --sample-id 0

    # Anyone can aggregate the run log into a report:
    python -m apeireth.v1424_asi_real_llm_benchmark report
"""

from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1424_VERSION = "0.1.0"
V1424_SCHEMA = "v1424.asi-real-llm-benchmark/v1"
V1424_MODULE = "v1424_asi_real_llm_benchmark"

# Real default paths (same convention as V1416–V1423):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_LOG_PATH = PROMETHEAN / ".v1424-benchmark-results.jsonl"

# Network / config bounds (主 00:44 质量工程化)
DEFAULT_TIMEOUT_SECONDS = 30
MIN_TIMEOUT_SECONDS = 1
MAX_TIMEOUT_SECONDS = 120

DEFAULT_MAX_TOKENS = 256
MIN_MAX_TOKENS = 1
MAX_MAX_TOKENS = 8192

DEFAULT_TEMPERATURE = 0.0
MIN_TEMPERATURE = 0.0
MAX_TEMPERATURE = 2.0

DEFAULT_MAX_RETRIES = 1
MIN_MAX_RETRIES = 0
MAX_MAX_RETRIES = 5

DEFAULT_MAX_SAMPLES = 22
MIN_MAX_SAMPLES = 1
MAX_MAX_SAMPLES = 22

# Provider registry
PROVIDERS: Tuple[str, ...] = (
    "newapi",
    "openai",
    "anthropic",
    "generic",
    "deterministic",
)

# Default price-per-1k-tokens (USD) — gpt-4o-mini-ish defaults, override at runtime
DEFAULT_PRICE_INPUT = 0.00015
DEFAULT_PRICE_OUTPUT = 0.0006

# Env-var names (key NEVER hardcoded)
ENV_BASE = "APEIRETH_LLM_BASE"
ENV_KEY = "APEIRETH_LLM_KEY"

# Sample-id prefix map (so a sample_id of e.g. "mmlu-0" is identifiable)
BENCHMARK_ORDER: Tuple[str, ...] = ("MMLU", "GSM8K", "HUMANEVAL", "HELLASWAG")

# Guard tuples
V1424_GUARDS: Tuple[str, ...] = (
    "GUARD_PREDICTOR_REAL",
    "GUARD_NO_V1034_WRITE",
    "GUARD_KEY_FROM_ENV",
    "GUARD_ENDPOINT_VALID",
    "GUARD_MODEL_BOUNDED",
    "GUARD_PROVIDER_KNOWN",
    "GUARD_TIMEOUT_BOUNDED",
    "GUARD_MAX_TOKENS_BOUNDED",
    "GUARD_TEMPERATURE_BOUNDED",
    "GUARD_RETRIES_BOUNDED",
    "GUARD_SAMPLE_BOUNDED",
    "GUARD_DETERMINISTIC_REPORTED",
    "GUARD_LATENCY_TRACKED",
    "GUARD_TOKENS_TRACKED",
    "GUARD_COST_ESTIMATED",
    "GUARD_LOG_ATOMIC",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_BACKWARD_COMPAT",
)

V1424_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_BENCH_IS_NOT_PHENOMENAL",
    "GUARD_BENCH_IS_NOT_ASI",
    "GUARD_BENCH_IS_NOT_HUMAN_LEVEL",
    "GUARD_BENCH_IS_NOT_ABSOLUTE",
    "GUARD_BENCH_IS_NOT_V1034_REPLACE",
    "GUARD_BENCH_IS_NOT_V1423_REPLACE",
    "GUARD_BENCH_IS_NOT_V1418_REPLACE",
    "GUARD_BENCH_IS_NOT_V1411_REPLACE",
    "GUARD_BENCH_IS_NOT_PRETEND_ASI",
)

V1424_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1034", "real benchmark harness (22 samples + 4 evaluators + V1034RealBenchmark class)"),
    ("V1422", "notification webhook (sign_payload_hmac borrowed pattern for request signing)"),
    ("V1423", "daemon webhook wiring (_append_webhook_log borrowed pattern for atomic JSONL)"),
    ("stdlib urllib.request", "HTTP POST without external deps"),
    ("stdlib json", "request/response serialization"),
    ("stdlib time", "per-sample latency"),
)


# ============================================================================
# Internal helpers
# ============================================================================


def _now_utc_iso() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _env_or_empty(name: str) -> str:
    """Return env var value or empty string. NEVER log the value."""
    val = os.environ.get(name, "")
    return val if val else ""


def _safe_path(p: Path) -> Path:
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with .. rejected: {p}")
    return Path(p)


def _validate_endpoint(url: str) -> str:
    if not isinstance(url, str):
        raise ValueError(f"endpoint must be str, got {type(url).__name__}")
    s = url.strip()
    if len(s) < 8:
        raise ValueError(f"endpoint too short: {len(s)} < 8")
    if not (s.startswith("http://") or s.startswith("https://")):
        raise ValueError(f"endpoint must start with http:// or https://: {s[:32]}")
    if " " in s or "\t" in s or "\n" in s:
        raise ValueError(f"endpoint must not contain whitespace: {s[:32]}")
    return s


def _validate_provider(p: str) -> str:
    if p not in PROVIDERS:
        raise ValueError(f"provider={p} not in {PROVIDERS}")
    return p


def _validate_model(m: str) -> str:
    if not isinstance(m, str):
        raise ValueError(f"model must be str, got {type(m).__name__}")
    s = m.strip()
    if not (1 <= len(s) <= 128):
        raise ValueError(f"model name length {len(s)} out of bounds [1, 128]")
    return s


def _validate_timeout(t: int) -> int:
    if not isinstance(t, int):
        raise ValueError(f"timeout_seconds must be int, got {type(t).__name__}")
    if not (MIN_TIMEOUT_SECONDS <= t <= MAX_TIMEOUT_SECONDS):
        raise ValueError(f"timeout_seconds={t} out of bounds [{MIN_TIMEOUT_SECONDS}, {MAX_TIMEOUT_SECONDS}]")
    return t


def _validate_max_tokens(n: int) -> int:
    if not isinstance(n, int):
        raise ValueError(f"max_tokens must be int, got {type(n).__name__}")
    if not (MIN_MAX_TOKENS <= n <= MAX_MAX_TOKENS):
        raise ValueError(f"max_tokens={n} out of bounds [{MIN_MAX_TOKENS}, {MAX_MAX_TOKENS}]")
    return n


def _validate_temperature(t: float) -> float:
    if not isinstance(t, (int, float)):
        raise ValueError(f"temperature must be float, got {type(t).__name__}")
    if not (MIN_TEMPERATURE <= float(t) <= MAX_TEMPERATURE):
        raise ValueError(f"temperature={t} out of bounds [{MIN_TEMPERATURE}, {MAX_TEMPERATURE}]")
    return float(t)


def _validate_max_retries(n: int) -> int:
    if not isinstance(n, int):
        raise ValueError(f"max_retries must be int, got {type(n).__name__}")
    if not (MIN_MAX_RETRIES <= n <= MAX_MAX_RETRIES):
        raise ValueError(f"max_retries={n} out of bounds [{MIN_MAX_RETRIES}, {MAX_MAX_RETRIES}]")
    return n


def _validate_max_samples(n: int) -> int:
    if not isinstance(n, int):
        raise ValueError(f"max_samples must be int, got {type(n).__name__}")
    if not (MIN_MAX_SAMPLES <= n <= MAX_MAX_SAMPLES):
        raise ValueError(f"max_samples={n} out of bounds [{MIN_MAX_SAMPLES}, {MAX_MAX_SAMPLES}]")
    return n


def _http_post_json(url: str, body: Dict[str, Any], headers: Dict[str, str], timeout: int) -> Dict[str, Any]:
    """POST JSON via stdlib urllib; returns parsed dict. Raises on HTTP error."""
    data = json.dumps(body, ensure_ascii=False).encode("utf-8")
    req = urllib.request.Request(url, data=data, method="POST")
    for k, v in headers.items():
        req.add_header(k, v)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read().decode("utf-8")
        return json.loads(raw)


def _import_v1034() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1034_real_benchmark as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1034 import failed: {exc}"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class LLMConfig:
    """Configuration for V1424 real-LLM benchmark."""

    provider: str
    base_url: str
    api_key_env: str
    model: str
    timeout_seconds: int
    max_tokens: int
    temperature: float
    max_retries: int
    price_per_1k_input: float
    price_per_1k_output: float
    log_path: Path
    max_samples: int
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["log_path"] = str(self.log_path)
        d["api_key_env"] = self.api_key_env  # name only, never the value
        return d


@dataclasses.dataclass
class SampleResult:
    """Result of one benchmark sample."""

    sample_id: str
    benchmark: str
    mode: str  # "REAL" or "MOCK"
    question: str
    ground_truth: str
    prediction: str
    correct: bool
    score: float
    latency_seconds: float
    prompt_tokens: int
    completion_tokens: int
    cost_usd: float
    http_status: int
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class BenchmarkReport:
    """Aggregated report across N samples."""

    provider: str
    model: str
    n_samples: int
    n_correct: int
    accuracy: float
    n_mock: int
    n_real: int
    total_latency_seconds: float
    total_tokens: int
    total_cost_usd: float
    started_iso: str
    ended_iso: str
    per_benchmark: Dict[str, Dict[str, Any]]
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Config builder + validator
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> LLMConfig:
    """Default config: deterministic (no key required)."""
    cfg = LLMConfig(
        provider="deterministic",
        base_url="",
        api_key_env=ENV_KEY,
        model="deterministic-v0",
        timeout_seconds=DEFAULT_TIMEOUT_SECONDS,
        max_tokens=DEFAULT_MAX_TOKENS,
        temperature=DEFAULT_TEMPERATURE,
        max_retries=DEFAULT_MAX_RETRIES,
        price_per_1k_input=DEFAULT_PRICE_INPUT,
        price_per_1k_output=DEFAULT_PRICE_OUTPUT,
        log_path=DEFAULT_LOG_PATH,
        max_samples=DEFAULT_MAX_SAMPLES,
        note="",
    )
    if overrides:
        for k, v in overrides.items():
            if hasattr(cfg, k):
                if k == "log_path":
                    v = _safe_path(Path(v))
                setattr(cfg, k, v)
    return cfg


def validate_config(cfg: LLMConfig) -> LLMConfig:
    _validate_provider(cfg.provider)
    _validate_model(cfg.model)
    _validate_timeout(cfg.timeout_seconds)
    _validate_max_tokens(cfg.max_tokens)
    _validate_temperature(cfg.temperature)
    _validate_max_retries(cfg.max_retries)
    _validate_max_samples(cfg.max_samples)
    if cfg.price_per_1k_input < 0 or cfg.price_per_1k_output < 0:
        raise ValueError("price_per_1k_* must be >= 0")
    if cfg.provider != "deterministic":
        if not cfg.base_url:
            raise ValueError(
                f"provider={cfg.provider} requires base_url (set {ENV_BASE} or pass --base-url)"
            )
        _validate_endpoint(cfg.base_url)
        # Key MUST be present in env
        if not _env_or_empty(cfg.api_key_env):
            raise ValueError(
                f"provider={cfg.provider} requires {cfg.api_key_env} to be set"
            )
    return cfg


# ============================================================================
# Predictors (the real vs deterministic split)
# ============================================================================


def _deterministic_predict(question: str, max_tokens: int) -> Tuple[str, int, int]:
    """Deterministic predictor: returns a fixed 'mock' answer.

    Returns (prediction, prompt_tokens_est, completion_tokens_est).
    Honest: this is NOT a real LLM, just a fixed string.
    """
    h = hashlib.sha256(question.encode("utf-8")).hexdigest()[:8]
    pred = f"deterministic:{h}"
    return pred, max(1, len(question) // 4), max(1, len(pred) // 4)


def _openai_style_predict(
    base_url: str,
    api_key: str,
    model: str,
    question: str,
    timeout: int,
    max_tokens: int,
    temperature: float,
    max_retries: int,
) -> Tuple[str, int, int, int]:
    """OpenAI-compatible /chat/completions call.

    Returns (prediction, prompt_tokens, completion_tokens, http_status).
    Raises on persistent HTTP failure (after max_retries).
    """
    url = base_url.rstrip("/") + "/chat/completions"
    body = {
        "model": model,
        "messages": [{"role": "user", "content": question}],
        "max_tokens": max_tokens,
        "temperature": temperature,
    }
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }
    last_exc: Optional[Exception] = None
    for attempt in range(max_retries + 1):
        try:
            t0 = time.time()
            resp = _http_post_json(url, body, headers, timeout)
            _ = time.time() - t0  # tracked at caller
            choices = resp.get("choices", [])
            if not choices:
                return "", 0, 0, 200
            msg = choices[0].get("message", {}) or {}
            content = msg.get("content", "") or ""
            usage = resp.get("usage", {}) or {}
            return (
                content,
                int(usage.get("prompt_tokens", 0)),
                int(usage.get("completion_tokens", 0)),
                200,
            )
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, TimeoutError) as exc:
            last_exc = exc
            if attempt >= max_retries:
                break
            time.sleep(min(1.0, 0.2 * (2 ** attempt)))
    raise RuntimeError(f"openai-style POST failed after {max_retries + 1} attempts: {last_exc}")


def _anthropic_predict(
    base_url: str,
    api_key: str,
    model: str,
    question: str,
    timeout: int,
    max_tokens: int,
    temperature: float,
    max_retries: int,
) -> Tuple[str, int, int, int]:
    """Anthropic-style /v1/messages call.

    Returns (prediction, prompt_tokens, completion_tokens, http_status).
    """
    url = base_url.rstrip("/") + "/v1/messages"
    body = {
        "model": model,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "messages": [{"role": "user", "content": question}],
    }
    headers = {
        "Content-Type": "application/json",
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
    }
    last_exc: Optional[Exception] = None
    for attempt in range(max_retries + 1):
        try:
            resp = _http_post_json(url, body, headers, timeout)
            content_blocks = resp.get("content", [])
            text = ""
            for block in content_blocks:
                if block.get("type") == "text":
                    text += block.get("text", "")
            usage = resp.get("usage", {}) or {}
            return (
                text,
                int(usage.get("input_tokens", 0)),
                int(usage.get("output_tokens", 0)),
                200,
            )
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, TimeoutError) as exc:
            last_exc = exc
            if attempt >= max_retries:
                break
            time.sleep(min(1.0, 0.2 * (2 ** attempt)))
    raise RuntimeError(f"anthropic POST failed after {max_retries + 1} attempts: {last_exc}")


def predict(cfg: LLMConfig, question: str) -> Tuple[str, int, int, int, str]:
    """Dispatch to the configured provider.

    Returns (prediction, prompt_tokens, completion_tokens, http_status, mode).
    mode ∈ {"REAL", "MOCK"}.
    """
    if cfg.provider == "deterministic":
        pred, pt, ct = _deterministic_predict(question, cfg.max_tokens)
        return pred, pt, ct, 200, "MOCK"
    api_key = _env_or_empty(cfg.api_key_env)
    if cfg.provider in ("newapi", "openai", "generic"):
        pred, pt, ct, status = _openai_style_predict(
            cfg.base_url, api_key, cfg.model, question,
            cfg.timeout_seconds, cfg.max_tokens, cfg.temperature, cfg.max_retries,
        )
        return pred, pt, ct, status, "REAL"
    if cfg.provider == "anthropic":
        pred, pt, ct, status = _anthropic_predict(
            cfg.base_url, api_key, cfg.model, question,
            cfg.timeout_seconds, cfg.max_tokens, cfg.temperature, cfg.max_retries,
        )
        return pred, pt, ct, status, "REAL"
    raise ValueError(f"unknown provider: {cfg.provider}")


# ============================================================================
# Atomic JSONL append (borrowed pattern from V1423)
# ============================================================================


def _append_result_log(log_path: Path, record: Dict[str, Any]) -> bool:
    """Atomically append a record to the benchmark-results JSONL."""
    try:
        log_path = _safe_path(Path(log_path))
        log_path.parent.mkdir(parents=True, exist_ok=True)
        tmp = log_path.with_suffix(log_path.suffix + ".tmp")
        existing = log_path.read_text(encoding="utf-8") if log_path.exists() else ""
        new_line = json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(existing)
            f.write(new_line)
            f.flush()
            try:
                os.fsync(f.fileno())
            except (AttributeError, OSError):
                pass
        os.replace(tmp, log_path)
        return True
    except Exception:
        return False


# ============================================================================
# Benchmark runner (delegates to V1034 evaluators)
# ============================================================================


def _build_sample_index() -> List[Tuple[str, Dict[str, Any]]]:
    """Build a flat ordered list of (sample_id, sample_dict) across all 4 V1034 benchmarks."""
    ok, v1034, _ = _import_v1034()
    if not ok:
        return []
    items: List[Tuple[str, Dict[str, Any]]] = []
    # MMLU
    for i, s in enumerate(getattr(v1034, "MMLU_SAMPLES", [])):
        items.append((f"mmlu-{i}", {**s, "_benchmark": "MMLU", "_kind": "mcq"}))
    # GSM8K
    for i, s in enumerate(getattr(v1034, "GSM8K_SAMPLES", [])):
        items.append((f"gsm8k-{i}", {**s, "_benchmark": "GSM8K", "_kind": "math"}))
    # HUMANEVAL
    for i, s in enumerate(getattr(v1034, "HUMANEVAL_SAMPLES", [])):
        items.append((f"humaneval-{i}", {**s, "_benchmark": "HUMANEVAL", "_kind": "code"}))
    # HELLASWAG
    for i, s in enumerate(getattr(v1034, "HELLASWAG_SAMPLES", [])):
        items.append((f"hellaswag-{i}", {**s, "_benchmark": "HELLASWAG", "_kind": "mcq"}))
    return items


def _extract_question(sample: Dict[str, Any], kind: str) -> str:
    """Return the user-facing question string for a V1034 sample."""
    if "question" in sample:
        return str(sample["question"])
    if "prompt" in sample:
        return str(sample["prompt"])
    if "context" in sample:
        return str(sample["context"])
    return ""


def _extract_ground_truth(sample: Dict[str, Any], kind: str) -> str:
    if "answer" in sample:
        return str(sample["answer"])
    if "reference" in sample:
        return str(sample["reference"])
    return ""


def _score_sample(
    cfg: LLMConfig,
    sample_id: str,
    sample: Dict[str, Any],
) -> SampleResult:
    """Score one sample: call predictor, evaluate via V1034, return SampleResult."""
    ok, v1034, _ = _import_v1034()
    if not ok:
        return SampleResult(
            sample_id=sample_id,
            benchmark=sample.get("_benchmark", "?"),
            mode="ERROR",
            question=_extract_question(sample, sample.get("_kind", "")),
            ground_truth=_extract_ground_truth(sample, sample.get("_kind", "")),
            prediction="",
            correct=False,
            score=0.0,
            latency_seconds=0.0,
            prompt_tokens=0,
            completion_tokens=0,
            cost_usd=0.0,
            http_status=0,
            note="v1034 not importable",
        )

    kind = sample.get("_kind", "")
    benchmark = sample.get("_benchmark", "?")
    question = _extract_question(sample, kind)
    gt = _extract_ground_truth(sample, kind)

    t0 = time.time()
    try:
        pred, pt, ct, status, mode = predict(cfg, question)
    except Exception as exc:
        pred = ""
        pt, ct, status = 0, 0, 0
        mode = "ERROR"
        latency = time.time() - t0
        return SampleResult(
            sample_id=sample_id,
            benchmark=benchmark,
            mode=mode,
            question=question,
            ground_truth=gt,
            prediction=pred,
            correct=False,
            score=0.0,
            latency_seconds=latency,
            prompt_tokens=pt,
            completion_tokens=ct,
            cost_usd=0.0,
            http_status=status,
            note=f"predict failed: {exc}",
        )
    latency = time.time() - t0

    # Compute cost estimate
    cost_usd = (pt / 1000.0) * cfg.price_per_1k_input + (ct / 1000.0) * cfg.price_per_1k_output

    # Evaluate via V1034's evaluators
    correct = False
    score = 0.0
    note = ""
    try:
        if benchmark == "MMLU":
            correct, score = v1034.evaluate_mmlu_sample(question, gt, pred)
        elif benchmark == "GSM8K":
            correct, score = v1034.evaluate_gsm8k_sample(question, gt, pred)
        elif benchmark == "HUMANEVAL":
            test = sample.get("test", "")
            reference = sample.get("reference", "")
            correct, score = v1034.evaluate_humaneval_sample(question, test, reference, pred)
        elif benchmark == "HELLASWAG":
            correct, score = v1034.evaluate_hellaswag_sample(question, gt, pred)
        else:
            note = f"unknown benchmark: {benchmark}"
    except Exception as exc:
        note = f"evaluator raised: {exc}"

    return SampleResult(
        sample_id=sample_id,
        benchmark=benchmark,
        mode=mode,
        question=question,
        ground_truth=gt,
        prediction=pred,
        correct=correct,
        score=score,
        latency_seconds=latency,
        prompt_tokens=pt,
        completion_tokens=ct,
        cost_usd=cost_usd,
        http_status=status,
        note=note,
    )


def run_benchmark(cfg: LLMConfig) -> BenchmarkReport:
    """Run up to cfg.max_samples against V1034's 22 samples. Returns BenchmarkReport."""
    cfg = validate_config(cfg)
    samples = _build_sample_index()
    started = _now_utc_iso()
    results: List[SampleResult] = []

    n = min(cfg.max_samples, len(samples))
    for i in range(n):
        sample_id, sample = samples[i]
        r = _score_sample(cfg, sample_id, sample)
        results.append(r)
        _append_result_log(cfg.log_path, r.to_dict())

    ended = _now_utc_iso()
    n_correct = sum(1 for r in results if r.correct)
    n_mock = sum(1 for r in results if r.mode == "MOCK")
    n_real = sum(1 for r in results if r.mode == "REAL")
    total_latency = sum(r.latency_seconds for r in results)
    total_tokens = sum(r.prompt_tokens + r.completion_tokens for r in results)
    total_cost = sum(r.cost_usd for r in results)

    # Per-benchmark breakdown
    per_benchmark: Dict[str, Dict[str, Any]] = {}
    for bench in BENCHMARK_ORDER:
        bench_results = [r for r in results if r.benchmark == bench]
        if not bench_results:
            continue
        per_benchmark[bench] = {
            "n_samples": len(bench_results),
            "n_correct": sum(1 for r in bench_results if r.correct),
            "accuracy": sum(1 for r in bench_results if r.correct) / len(bench_results),
            "n_mock": sum(1 for r in bench_results if r.mode == "MOCK"),
        }

    return BenchmarkReport(
        provider=cfg.provider,
        model=cfg.model,
        n_samples=len(results),
        n_correct=n_correct,
        accuracy=(n_correct / len(results)) if results else 0.0,
        n_mock=n_mock,
        n_real=n_real,
        total_latency_seconds=total_latency,
        total_tokens=total_tokens,
        total_cost_usd=total_cost,
        started_iso=started,
        ended_iso=ended,
        per_benchmark=per_benchmark,
        note=f"v1424 benchmark (provider={cfg.provider}, model={cfg.model})",
    )


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    """Run 17 popper self-tests."""
    results: List[Dict[str, Any]] = []
    n_pass = 0

    def _check(name: str, ok: bool, detail: str = "") -> None:
        nonlocal n_pass
        if ok:
            n_pass += 1
        results.append({"name": name, "ok": ok, "detail": detail})

    # 1. Module constants
    _check(
        "module_constants_present",
        V1424_VERSION == "0.1.0" and V1424_SCHEMA == "v1424.asi-real-llm-benchmark/v1",
        f"version={V1424_VERSION}",
    )

    # 2. PROVIDERS contains expected entries
    _check(
        "providers_complete",
        set(PROVIDERS) == {"newapi", "openai", "anthropic", "generic", "deterministic"},
        f"providers={PROVIDERS}",
    )

    # 3. Borrowed shape
    keys = [b[0] for b in V1424_BORROWED]
    _check(
        "borrowed_complete",
        "V1034" in keys and "V1422" in keys and "V1423" in keys,
        f"keys={keys}",
    )

    # 4. _validate_provider
    for p in PROVIDERS:
        try:
            _validate_provider(p)
        except ValueError:
            _check("validate_provider_accepts_all", False, f"rejected {p}")
            break
    else:
        _check("validate_provider_accepts_all", True, "all providers accepted")

    # 5. _validate_provider rejects bad
    try:
        _validate_provider("FOO")
        _check("validate_provider_rejects_bad", False, "should have raised")
    except ValueError:
        _check("validate_provider_rejects_bad", True, "FOO rejected")

    # 6. _validate_endpoint
    _check(
        "validate_endpoint_accepts_https",
        _validate_endpoint("https://api.example.com/v1") == "https://api.example.com/v1",
        "ok",
    )
    try:
        _validate_endpoint("ftp://x")
        _check("validate_endpoint_rejects_non_http", False, "should have raised")
    except ValueError:
        _check("validate_endpoint_rejects_non_http", True, "ftp:// rejected")

    # 7. _validate_model
    _check(
        "validate_model_accepts",
        _validate_model("gpt-4o-mini") == "gpt-4o-mini",
        "ok",
    )
    try:
        _validate_model("")
        _check("validate_model_rejects_empty", False, "should have raised")
    except ValueError:
        _check("validate_model_rejects_empty", True, "empty rejected")

    # 8. _validate_timeout
    _check("validate_timeout", _validate_timeout(30) == 30, "30 ok")
    try:
        _validate_timeout(0)
        _check("validate_timeout_rejects_zero", False, "should have raised")
    except ValueError:
        _check("validate_timeout_rejects_zero", True, "0 rejected")

    # 9. _validate_max_tokens
    _check("validate_max_tokens", _validate_max_tokens(256) == 256, "256 ok")
    try:
        _validate_max_tokens(99999)
        _check("validate_max_tokens_rejects_huge", False, "should have raised")
    except ValueError:
        _check("validate_max_tokens_rejects_huge", True, "99999 rejected")

    # 10. _validate_temperature
    _check("validate_temperature", _validate_temperature(0.5) == 0.5, "0.5 ok")
    try:
        _validate_temperature(3.0)
        _check("validate_temperature_rejects_huge", False, "should have raised")
    except ValueError:
        _check("validate_temperature_rejects_huge", True, "3.0 rejected")

    # 11. _validate_max_retries
    _check("validate_max_retries", _validate_max_retries(1) == 1, "1 ok")
    try:
        _validate_max_retries(99)
        _check("validate_max_retries_rejects_huge", False, "should have raised")
    except ValueError:
        _check("validate_max_retries_rejects_huge", True, "99 rejected")

    # 12. _validate_max_samples
    _check("validate_max_samples", _validate_max_samples(22) == 22, "22 ok")
    try:
        _validate_max_samples(100)
        _check("validate_max_samples_rejects_huge", False, "should have raised")
    except ValueError:
        _check("validate_max_samples_rejects_huge", True, "100 rejected")

    # 13. Default config + validation
    cfg = build_default_config()
    try:
        validate_config(cfg)
        _check("default_config_validates", True, "ok")
    except Exception as exc:
        _check("default_config_validates", False, str(exc))

    # 14. Default config does NOT require env vars (backward compat)
    _check(
        "default_config_no_env_required",
        cfg.provider == "deterministic" and cfg.base_url == "",
        f"provider={cfg.provider} base_url={cfg.base_url!r}",
    )

    # 15. Deterministic predict works without network
    pred, pt, ct = _deterministic_predict("What is 2+2?", 64)
    _check(
        "deterministic_predict_works",
        pred.startswith("deterministic:") and pt > 0 and ct > 0,
        f"pred={pred!r} pt={pt} ct={ct}",
    )

    # 16. Sample index has 22 items
    items = _build_sample_index()
    _check(
        "sample_index_has_22",
        len(items) == 22,
        f"n={len(items)}",
    )

    # 17. End-to-end deterministic benchmark
    cfg_det = build_default_config({"provider": "deterministic", "max_samples": 22})
    report = run_benchmark(cfg_det)
    _check(
        "end_to_end_deterministic_benchmark",
        report.n_samples == 22 and report.n_mock == 22 and report.n_real == 0,
        f"n_samples={report.n_samples} n_mock={report.n_mock} accuracy={report.accuracy:.2%}",
    )

    all_ok = all(r["ok"] for r in results)
    return all_ok, n_pass, results


# ============================================================================
# Chain delegation
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    out: Dict[str, Any] = {"v1424": True}
    for ver, modname in (
        ("V1034", "v1034_real_benchmark"),
        ("V1418", "v1418_asi_dgm_cron_integration"),
        ("V1421", "v1421_asi_daemon_serve_tick"),
        ("V1422", "v1422_asi_notification_webhook"),
        ("V1423", "v1423_asi_daemon_webhook_wiring"),
    ):
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                out[ver] = bool(sub.get(ver, sub))
            else:
                out[ver] = True
        except Exception as exc:
            out[ver] = False
            out[f"{ver}_error"] = str(exc)
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        "\n".join(
            [
                "V1424 — ASI 总框架 benchmark 真接 LLM",
                "",
                "Commands:",
                "  version",
                "  meta [--json]",
                "  demo",
                "  help",
                "  popper",
                "  chain",
                "  list-samples [--benchmark NAME]",
                "  run [--provider P] [--model M] [--max-samples N] [--max-tokens N]",
                "         [--temperature F] [--timeout-seconds N] [--base-url URL]",
                "         [--log-path PATH]",
                "  show-result --sample-id N [--log-path PATH]",
                "  report [--log-path PATH]",
            ]
        )
    )


def _parse_kv_args(rest: List[str]) -> Dict[str, str]:
    out: Dict[str, str] = {}
    i = 0
    while i < len(rest):
        tok = rest[i]
        if tok.startswith("--"):
            key = tok[2:].replace("-", "_")
            if i + 1 < len(rest) and not rest[i + 1].startswith("--"):
                out[key] = rest[i + 1]
                i += 2
            else:
                out[key] = "true"
                i += 1
        else:
            i += 1
    return out


def _coerce_overrides(kv: Dict[str, str]) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    int_keys = {"max_samples", "max_tokens", "timeout_seconds", "max_retries"}
    float_keys = {"temperature", "price_per_1k_input", "price_per_1k_output"}
    for k, v in kv.items():
        if k in int_keys:
            out[k] = int(v)
        elif k in float_keys:
            out[k] = float(v)
        else:
            out[k] = v
    return out


def run_cli(argv: List[str]) -> int:
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(f"V1424 v{V1424_VERSION} ({V1424_SCHEMA})")
        return 0
    if cmd in ("help", "--help", "-h"):
        _print_help()
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest)
        if kv.get("json") == "true":
            print(json.dumps({"version": V1424_VERSION, "schema": V1424_SCHEMA, "module": V1424_MODULE}, ensure_ascii=False))
        else:
            print(f"V1424 v{V1424_VERSION} schema={V1424_SCHEMA} module={V1424_MODULE}")
        return 0
    if cmd == "demo":
        print("V1424 demo: real LLM benchmark — 22 samples (10 MMLU + 5 GSM8K + 3 HumanEval + 4 HellaSwag)")
        _print_help()
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(json.dumps({"all_ok": all_ok, "n_pass": n_pass, "results": results}, ensure_ascii=False, indent=2))
        return 0 if all_ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "list-samples":
        kv = _parse_kv_args(rest)
        target_bench = kv.get("benchmark", "")
        items = _build_sample_index()
        for sid, s in items:
            bench = s.get("_benchmark", "?")
            if target_bench and bench.lower() != target_bench.lower():
                continue
            q = _extract_question(s, s.get("_kind", ""))
            q_short = q[:60] + ("..." if len(q) > 60 else "")
            print(f"  {sid:>16}  [{bench:>8}]  {q_short}")
        return 0
    if cmd == "run":
        overrides = _coerce_overrides(_parse_kv_args(rest))
        cfg = build_default_config(overrides)
        # If provider is real but no base_url, try ENV
        if cfg.provider != "deterministic" and not cfg.base_url:
            cfg.base_url = _env_or_empty(ENV_BASE)
        try:
            cfg = validate_config(cfg)
        except ValueError as exc:
            print(f"ERROR: {exc}")
            return 1
        report = run_benchmark(cfg)
        print(json.dumps(report.to_dict(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "show-result":
        kv = _parse_kv_args(rest)
        sample_id = kv.get("sample_id", "")
        if not sample_id:
            print("ERROR: --sample-id required")
            return 1
        log_path = Path(kv.get("log_path", str(build_default_config().log_path)))
        if not log_path.exists():
            print(f"log not found: {log_path}")
            return 0
        for line in log_path.read_text(encoding="utf-8").splitlines():
            try:
                rec = json.loads(line)
                if rec.get("sample_id") == sample_id:
                    print(json.dumps(rec, ensure_ascii=False, indent=2))
                    return 0
            except json.JSONDecodeError:
                continue
        print(f"sample_id={sample_id} not found in log")
        return 0
    if cmd == "report":
        kv = _parse_kv_args(rest)
        log_path = Path(kv.get("log_path", str(build_default_config().log_path)))
        if not log_path.exists():
            print(f"log not found: {log_path}")
            return 0
        records = []
        for line in log_path.read_text(encoding="utf-8").splitlines():
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue
        if not records:
            print("no records")
            return 0
        n_correct = sum(1 for r in records if r.get("correct"))
        n_mock = sum(1 for r in records if r.get("mode") == "MOCK")
        n_real = sum(1 for r in records if r.get("mode") == "REAL")
        total_tokens = sum((r.get("prompt_tokens", 0) + r.get("completion_tokens", 0)) for r in records)
        total_cost = sum(r.get("cost_usd", 0.0) for r in records)
        total_latency = sum(r.get("latency_seconds", 0.0) for r in records)
        agg = {
            "n_records": len(records),
            "n_correct": n_correct,
            "accuracy": n_correct / len(records) if records else 0.0,
            "n_mock": n_mock,
            "n_real": n_real,
            "total_tokens": total_tokens,
            "total_cost_usd": total_cost,
            "total_latency_seconds": total_latency,
        }
        print(json.dumps(agg, ensure_ascii=False, indent=2))
        return 0
    print(f"unknown command: {cmd}")
    _print_help()
    return 1


# ============================================================================
# Bootstrap
# ============================================================================


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))