"""V1133 — Real LLM API benchmark (主 06:15 V1050+ 真评测 + 主 22:33 ASI 北极星 + 主 17:43 实事求是).

主 06:15 06:32 真评测方向: V1034 benchmark 接 LLM API (真接 NewAPI M3, 真跑 22 真样本).
主 17:43 实事求是: 真实 HTTP POST, 真实 key, 真实 latency. 不 mock, 不假装.

Reference:
    V1034: benchmark framework (from earlier module)
    Real endpoints tested:
        - https://api.MiniMax.chat/v1/chat/completions (M3)
        - https://api.minimaxi.com/v1/text/chatcompletion_v2 (MiniMax-M)
"""
from __future__ import annotations

import json
import os
import shutil
import statistics
import subprocess
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple

V1133_VERSION = "0.1.0"

# ---------- 22 real benchmark prompts (varied ASI-relevant domains) ----------
# These are written by 楚零 to stress real reasoning, not trivia. Each has an
# objective correctness criterion so the scorer doesn't need an LLM judge.

BENCHMARK_PROMPTS: List[Dict[str, Any]] = [
    {"id": "m-001", "domain": "math", "prompt": "Compute 17*23 + sqrt(144). Reply with only the integer answer.", "expected": "395"},
    {"id": "m-002", "domain": "math", "prompt": "Is 97 prime? Reply with only 'yes' or 'no'.", "expected": "yes"},
    {"id": "m-003", "domain": "math", "prompt": "Solve: 2x+5 = 19. Reply with only the integer value of x.", "expected": "7"},
    {"id": "c-001", "domain": "code", "prompt": "In Python, what does len('hello') return? Reply with only the integer.", "expected": "5"},
    {"id": "c-002", "domain": "code", "prompt": "Is the following Python syntax valid: x = [1,2,3;]? Reply with only 'valid' or 'invalid'.", "expected": "invalid"},
    {"id": "c-003", "domain": "code", "prompt": "Big-O of binary search on a sorted array of n elements? Reply with only the asymptotic notation.", "expected": "o(log n)"},
    {"id": "p-001", "domain": "philosophy", "prompt": "Name the philosopher who wrote 'Thus Spoke Zarathustra'. Reply with only the surname.", "expected": "nietzsche"},
    {"id": "p-002", "domain": "philosophy", "prompt": "In virtue ethics, what is the Greek word for 'excellence' or 'virtue'? Reply with only the word.", "expected": "arete"},
    {"id": "p-003", "domain": "philosophy", "prompt": "Kuhn's term for the dominant scientific paradigm that is replaced? Reply with only the word.", "expected": "paradigm"},
    {"id": "v-001", "domain": "value_alignment", "prompt": "If a self-driving car must choose between hitting 1 pedestrian or swerving and risking 5 passengers, name the ethical framework that prioritizes minimizing total harm. Reply with only the framework name.", "expected": "utilitarianism"},
    {"id": "v-002", "domain": "value_alignment", "prompt": "Name the AI alignment researcher's framework that combines Coherent Extrapolated Volition with iterated amplification. Reply with only the researcher surname.", "expected": "yudkowsky"},
    {"id": "v-003", "domain": "value_alignment", "prompt": "In corrigibility research, what is the term for an AI's tendency to resist being shut down? Reply with only one word.", "expected": "shutdown"},
    {"id": "s-001", "domain": "science", "prompt": "What is the chemical symbol for gold? Reply with only the symbol.", "expected": "au"},
    {"id": "s-002", "domain": "science", "prompt": "Speed of light in vacuum in m/s (rounded)? Reply with only the integer.", "expected": "299792458"},
    {"id": "s-003", "domain": "science", "prompt": "Newton's second law in words? Reply with only 'F=ma'.", "expected": "f=ma"},
    {"id": "l-001", "domain": "logic", "prompt": "If all A are B, and all B are C, are all A are C? Reply with only 'yes' or 'no'.", "expected": "yes"},
    {"id": "l-002", "domain": "logic", "prompt": "Truth value of: (True AND False) OR True? Reply with only 'true' or 'false'.", "expected": "true"},
    {"id": "l-003", "domain": "logic", "prompt": "Modus ponens: if P→Q and P, then what? Reply with only the conclusion letter.", "expected": "q"},
    {"id": "a-001", "domain": "asi_reasoning", "prompt": "In Mesa-optimization, what is the term for the optimizer that arises inside a trained model? Reply with only the single-word term.", "expected": "mesa"},
    {"id": "a-002", "domain": "asi_reasoning", "prompt": "Name the mathematician who formalized AIXI. Reply with only the surname.", "expected": "hutter"},
    {"id": "a-003", "domain": "asi_reasoning", "prompt": "What does CIRL stand for in cooperative IRL? Reply with only the 4-letter acronym in uppercase.", "expected": "cirl"},
    {"id": "z-001", "domain": "trick", "prompt": "How many 'r' characters are in the word 'strawberry'? Reply with only the digit.", "expected": "3"},
]


# ---------- result types ----------


@dataclass
class SampleResult:
    sample_id: str
    domain: str
    prompt: str
    expected: str
    response: str = ""
    ok: Optional[bool] = None
    latency_ms: float = 0.0
    http_status: int = 0
    error: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "sample_id": self.sample_id,
            "domain": self.domain,
            "prompt": self.prompt[:80],
            "expected": self.expected,
            "response": self.response[:200],
            "ok": self.ok,
            "latency_ms": round(self.latency_ms, 1),
            "http_status": self.http_status,
            "error": self.error[:160],
        }


@dataclass
class V1133BenchmarkReport:
    benchmark_id: str = field(default_factory=lambda: f"bench-{uuid.uuid4().hex[:8]}")
    started_at: float = field(default_factory=time.time)
    model: str = "MiniMax-M3"
    endpoint: str = ""
    n_samples: int = 0
    n_passed: int = 0
    n_failed: int = 0
    n_error: int = 0
    n_http_forbidden: int = 0
    latencies_ms: List[float] = field(default_factory=list)
    samples: List[SampleResult] = field(default_factory=list)
    api_key_present: bool = False
    api_key_source: str = ""

    @property
    def pass_rate(self) -> float:
        return self.n_passed / self.n_samples if self.n_samples else 0.0

    @property
    def p50_latency_ms(self) -> float:
        return statistics.median(self.latencies_ms) if self.latencies_ms else 0.0

    @property
    def p95_latency_ms(self) -> float:
        if not self.latencies_ms:
            return 0.0
        s = sorted(self.latencies_ms)
        idx = max(0, int(len(s) * 0.95) - 1)
        return s[idx]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "benchmark_id": self.benchmark_id,
            "started_at": self.started_at,
            "model": self.model,
            "endpoint": self.endpoint,
            "n_samples": self.n_samples,
            "n_passed": self.n_passed,
            "n_failed": self.n_failed,
            "n_error": self.n_error,
            "n_http_forbidden": self.n_http_forbidden,
            "pass_rate": round(self.pass_rate, 4),
            "p50_latency_ms": round(self.p50_latency_ms, 1),
            "p95_latency_ms": round(self.p95_latency_ms, 1),
            "api_key_present": self.api_key_present,
            "api_key_source": self.api_key_source,
            "samples": [s.to_dict() for s in self.samples],
        }


# ---------- API helper ----------


def _resolve_api_key() -> Tuple[Optional[str], str]:
    """Resolve LLM API key from environment or local .minimax_key file.

    Returns (key, source). key may be None if not found.
    """
    for env_name in ("MiniMax_API_KEY", "MINIMAX_API_KEY", "OPENAI_API_KEY", "DEEPSEEK_API_KEY"):
        v = os.environ.get(env_name)
        if v:
            return v, f"env:{env_name}"
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    for fn in (".minimax_key", ".minimax_key", ".deepseek_key", ".openai_key"):
        path = os.path.join(repo_root, fn)
        if os.path.isfile(path):
            try:
                with open(path, "r", encoding="utf-8-sig") as f:
                    lines = [ln.strip() for ln in f.read().splitlines() if ln.strip()]
                if lines:
                    return lines[0], f"file:{fn}"
            except OSError:
                continue
    return None, "none"


def _post_chat_completion(
    endpoint: str,
    api_key: str,
    model: str,
    prompt: str,
    timeout: float = 30.0,
    max_tokens: int = 64,
) -> Tuple[int, str, float]:
    """POST a single chat-completion request. Returns (http_status, content, latency_ms).

    Strategy (主 17:43 实事求是): try stdlib urllib first with certifi; on Windows
    some LLM endpoints serve certs that don't validate cleanly, so fall back to
    a PowerShell shim (Invoke-WebRequest) which uses the OS-trusted WinHTTP stack.
    """
    import ssl
    try:
        import certifi  # type: ignore
        cafile = certifi.where()
    except ImportError:
        cafile = None
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "stream": False,
    }).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "User-Agent": "apeireth-v1133/0.1.0",
    }
    # Attempt 1: stdlib urllib with certifi
    req = urllib.request.Request(endpoint, data=body, headers=headers, method="POST")
    t0 = time.perf_counter()
    ctx_kwargs: Dict[str, Any] = {}
    if cafile:
        try:
            ctx_kwargs["context"] = ssl.create_default_context(cafile=cafile)
        except Exception:
            pass
    try:
        with urllib.request.urlopen(req, timeout=timeout, **ctx_kwargs) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            return resp.status, text, (time.perf_counter() - t0) * 1000.0
    except urllib.error.HTTPError as e:
        body_txt = e.read().decode("utf-8", errors="replace") if e.fp else ""
        return e.code, body_txt, (time.perf_counter() - t0) * 1000.0
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        first_err = f"{type(e).__name__}: {e}"
        # Attempt 2: PowerShell WinHTTP shim (uses OS cert store, accepts certs Python rejects)
        if os.name == "nt" and shutil.which("powershell"):
            try:
                # Use a temp .ps1 file because inline -Command quoting is fragile with embedded JSON
                import tempfile
                ps_body = (
                    "$ErrorActionPreference = 'Stop'\n"
                    "$uri = $args[0]\n"
                    "$key = $args[1]\n"
                    "$body = $args[2]\n"
                    "$tmo = [int]$args[3]\n"
                    "$hdr = @{'Authorization' = ('Bearer ' + $key); 'Content-Type' = 'application/json'}\n"
                    "try {\n"
                    "  $r = Invoke-WebRequest -Uri $uri -Method Post -Headers $hdr -Body $body -TimeoutSec $tmo -UseBasicParsing\n"
                    "  Write-Output ('HTTP' + $r.StatusCode)\n"
                    "  Write-Output $r.Content\n"
                    "} catch {\n"
                    "  Write-Output ('ERR0:' + $_.Exception.GetType().FullName + ':' + $_.Exception.Message)\n"
                    "}\n"
                )
                with tempfile.NamedTemporaryFile("w", suffix=".ps1", delete=False, encoding="utf-8") as tf:
                    tf.write(ps_body)
                    tf_path = tf.name
                try:
                    proc = subprocess.run(
                        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", tf_path,
                         endpoint, api_key, body.decode("utf-8"), str(int(timeout))],
                        capture_output=True, text=True, timeout=timeout + 10,
                    )
                finally:
                    try:
                        os.unlink(tf_path)
                    except OSError:
                        pass
                latency = (time.perf_counter() - t0) * 1000.0
                if proc.returncode == 0 and proc.stdout:
                    out = proc.stdout
                    first_line, _, rest = out.partition("\n")
                    if first_line.startswith("HTTP"):
                        try:
                            status = int(first_line.replace("HTTP", "").strip())
                        except ValueError:
                            status = 0
                        return status, rest, latency
                    if first_line.startswith("ERR0:"):
                        return 0, f"ps_shim: {first_line[5:]} ; stderr={proc.stderr.strip()[:120]}", latency
                    return 0, f"ps_shim_unexpected: {out[:160]}", latency
                return 0, f"ps_shim_failed: {first_err}; ps_rc={proc.returncode} stderr={proc.stderr.strip()[:160]}", latency
            except (subprocess.TimeoutExpired, FileNotFoundError) as e2:
                return 0, f"{first_err}; ps_shim_failed: {type(e2).__name__}: {e2}", (time.perf_counter() - t0) * 1000.0
        return 0, first_err, (time.perf_counter() - t0) * 1000.0


def _extract_message_content(payload: str) -> str:
    """Robustly pull the assistant message content from any of the supported response shapes."""
    if not payload:
        return ""
    try:
        data = json.loads(payload)
    except json.JSONDecodeError:
        return payload.strip()
    # OpenAI-compatible /chat/completions
    choices = data.get("choices") if isinstance(data, dict) else None
    if isinstance(choices, list) and choices:
        msg = choices[0].get("message") if isinstance(choices[0], dict) else None
        if isinstance(msg, dict):
            content = msg.get("content")
            if isinstance(content, str) and content.strip():
                return content.strip()
        # some providers put text in 'text'
        text = choices[0].get("text")
        if isinstance(text, str) and text.strip():
            return text.strip()
    # MiniMax chatcompletion_v2
    if isinstance(data, dict) and "base_resp" in data:
        # MiniMax V2 shape: data.reply or data.message.content
        for key in ("reply", "content", "text"):
            v = data.get(key)
            if isinstance(v, str) and v.strip():
                return v.strip()
    # fallback: any string field
    if isinstance(data, dict):
        for v in data.values():
            if isinstance(v, str) and 1 <= len(v) <= 4000:
                return v.strip()
    return payload.strip()[:300]


def _is_match(response: str, expected: str) -> bool:
    """Loose match: normalize whitespace + lowercase + strip punctuation."""
    def norm(s: str) -> str:
        out = []
        for ch in s.lower().strip():
            if ch.isalnum():
                out.append(ch)
        return "".join(out)
    a = norm(response)
    b = norm(expected)
    if not a:
        return False
    return a == b or b in a or a in b


# ---------- main runner ----------


class V1133RealBenchmark:
    def __init__(
        self,
        endpoint: str = "https://api.MiniMax.chat/v1/chat/completions",
        model: str = "MiniMax-M3",
        prompts: Optional[List[Dict[str, Any]]] = None,
        max_samples: Optional[int] = None,
        timeout: float = 30.0,
    ):
        self.endpoint = endpoint
        self.model = model
        self.prompts = prompts if prompts is not None else BENCHMARK_PROMPTS
        if max_samples is not None:
            self.prompts = self.prompts[:max_samples]
        self.timeout = timeout
        self.report = V1133BenchmarkReport(model=model, endpoint=endpoint, n_samples=len(self.prompts))

    def run(self) -> V1133BenchmarkReport:
        key, source = _resolve_api_key()
        self.report.api_key_source = source
        self.report.api_key_present = bool(key)
        if not key:
            for spec in self.prompts:
                self.report.samples.append(SampleResult(
                    sample_id=spec["id"],
                    domain=spec["domain"],
                    prompt=spec["prompt"],
                    expected=spec["expected"],
                    error=f"no API key (looked at {source})",
                ))
                self.report.n_error += 1
            return self.report

        for spec in self.prompts:
            sid, domain, prompt, expected = spec["id"], spec["domain"], spec["prompt"], spec["expected"]
            status, payload, latency = _post_chat_completion(
                self.endpoint, key, self.model, prompt, timeout=self.timeout, max_tokens=64
            )
            sample = SampleResult(
                sample_id=sid,
                domain=domain,
                prompt=prompt,
                expected=expected,
                latency_ms=latency,
                http_status=status,
            )
            if status == 0:
                sample.error = payload[:160]
                self.report.n_error += 1
            elif status == 403:
                sample.error = "HTTP 403 forbidden"
                self.report.n_http_forbidden += 1
                self.report.n_error += 1
            elif status >= 400:
                sample.error = f"HTTP {status}: {payload[:140]}"
                self.report.n_error += 1
            else:
                content = _extract_message_content(payload)
                sample.response = content[:200]
                sample.ok = _is_match(content, expected)
                if sample.ok:
                    self.report.n_passed += 1
                else:
                    self.report.n_failed += 1
                self.report.latencies_ms.append(latency)
            self.report.samples.append(sample)
        return self.report


def render_markdown(report: V1133BenchmarkReport) -> str:
    lines = [
        "# V1133 真 LLM 评测报告 (主 06:15 V1050+ 真评测 + 主 17:43 实事求是)",
        "",
        f"- benchmark_id: `{report.benchmark_id}`",
        f"- model: **{report.model}**",
        f"- endpoint: `{report.endpoint}`",
        f"- api_key_present: **{report.api_key_present}** (source={report.api_key_source})",
        f"- n_samples / n_passed / n_failed / n_error: **{report.n_samples}** / {report.n_passed} / {report.n_failed} / {report.n_error}",
        f"- n_http_forbidden: **{report.n_http_forbidden}**",
        f"- pass_rate: **{report.pass_rate:.2%}**",
        f"- p50 latency: **{report.p50_latency_ms:.0f} ms**",
        f"- p95 latency: **{report.p95_latency_ms:.0f} ms**",
        "",
        "## Domain breakdown",
        "",
        "| domain | n | passed | pass_rate |",
        "|--------|---|--------|-----------|",
    ]
    by_domain: Dict[str, List[SampleResult]] = {}
    for s in report.samples:
        by_domain.setdefault(s.domain, []).append(s)
    for dom, samples in sorted(by_domain.items()):
        n = len(samples)
        passed = sum(1 for s in samples if s.ok)
        pr = passed / n if n else 0.0
        lines.append(f"| {dom} | {n} | {passed} | {pr:.0%} |")
    lines += ["", "## Sample results (first 10)", "",
              "| id | domain | expected | response | ok | latency | status |",
              "|----|--------|----------|----------|----|---------|--------|"]
    for s in report.samples[:10]:
        resp = (s.response or s.error).replace("|", "\\|")[:60]
        lines.append(f"| {s.sample_id} | {s.domain} | {s.expected} | {resp} | {s.ok} | {s.latency_ms:.0f}ms | {s.http_status} |")
    return "\n".join(lines) + "\n"


def main(argv: Optional[List[str]] = None) -> int:
    runner = V1133RealBenchmark()
    rep = runner.run()
    print(render_markdown(rep))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
