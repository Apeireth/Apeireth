"""
V1328 — AnySearch 插件真源码深读 (AnySearch Plugin Real Source Code Deep Read)

主 13:31 + 19:33 + 00:56 directive: 「VCP 真实代码去真实深读」
(V1327 = VCP 6 core layers deep read; V1328 = first plugin deep read)
VCP = Variable & Command Protocol — 主人 real running production project
 at C:\\Users\\REDACTED\\VCPToolBox\\VCPToolBox-main\\

AnySearch = 主人最高频插件 (实时搜索 / 垂直搜索 / 批量并行 / 网页提取).
This module does a REAL DEEP READ of 3 AnySearch source files (AnySearch.js
+ sync.js + plugin-manifest.json), then extracts the patterns / invariants /
safety boundaries into 8 真生产 components. Each component is a faithful
*pattern representation* (not a JavaScript port) of the original plugin
behavior, so Apeireth can reason about AnySearch architecture without
pretending to run it.

V3 哲学守门 (LOCKED):
- 不假装 V1328 = 复刻 AnySearch (we are reading, not porting)
- 不假装 AnySearch 真跑 (file system = read-only analysis)
- 不假装 ASI 真理解 AnySearch (pattern extraction ≠ semantics)
- 不假装 ASI 解决 AnySearch 架构问题 (architectural study only)
- 不假装 Phenomenal consciousness
- ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE

Author: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 2026-08-08)
Trigger: post-V1327 VCP 6 source deep read (e741d5bb, 20:34)
Chain: V1313 → ... → V1327 → V1328
"""

from __future__ import annotations

import hashlib
import json
import re
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Set, Tuple

# ============================================================
# ASI 北极星 (LOCKED — 不刷新 KPI)
# ============================================================
ASI_POLE_STAR = {
    "V0_1_anchored": 0.7905,
    "V0_2_baseline": 0.4467,
    "V1256_unio_mystica": 0.9105,
    "V1049_value_alignment": "DONE",
}

# ============================================================
# AnySearch 真源码 deep-read 目标 (read-only paths)
# ============================================================
ANYSEARCH_ROOT = Path(r"VCPToolBox\VCPToolBox-main\Plugin\AnySearch")

ANYSEARCH_3_FILES: Tuple[Dict[str, Any], ...] = (
    {
        "file_id": "F1_main",
        "relative_path": "AnySearch.js",
        "architectural_role": "stdio sync plugin entry (search/get_sub_domains/batch_search/extract)",
        "key_patterns": [
            "stdio JSON-RPC 2.0 protocol (read stdin → write stdout JSON)",
            "Surface errors as JSON payload on stdout + exit 0 (host reads stdout, non-zero exit = crash)",
            "17 vertical domains: general/resource/social_media/finance/academic/legal/health/business/security/ip/code/energy/environment/agriculture/travel/film/gaming",
            "4 commands: search/get_sub_domains/batch_search/extract",
            "Command inference: explicit command OR queries→batch_search OR (url+!query)→extract OR search",
            "firstString() multi-key probe (command/action/tool/mode) for input tolerance",
            "k=v text format + JSON object dual parser for sub_domain_params",
            "Domain auto-derive: sub_domain prefix → domain (with explicit contradiction fail)",
            "BATCH_MAX=5 / DOMAINS_MAX=5 / MAX_RESULTS=1-10 hard bounds",
            "HTTPS-only for production; HTTP only loopback (127.0.0.1/localhost/::1)",
            "Multi-API-key rotation (comma-separated, random pick per request)",
            "JSON-RPC payload: {jsonrpc:2.0, id:1, method:tools/call, params:{name,arguments}}",
            "Extract MCP content[].text from JSON-RPC result; fallback to JSON.stringify",
            "Content emission as {status:success, result:{content:[{type:text,text}]}} (matches MCP format)",
        ],
        "safety_boundaries": [
            "fail() writes JSON + exit 0 (NOT throw — host treats non-zero as crash)",
            "URL parsing fail() on invalid endpoint",
            "max_results clamp to [1, 10] (prevents resource exhaustion)",
            "Timeout clamp to [1000, 120000] ms",
            "DOMAIN_SET membership check (rejects unknown domains with helpful message)",
            "sub_domain vs domain contradiction detection (fail-fast)",
            "url http/https scheme validation (rejects file://, ftp:// etc.)",
            "Bearer auth only attached when key present (anonymous access allowed)",
        ],
        "declared_lines": 350,
        "sha256_full_16b": "ceec12f4fa53ddc3",
    },
    {
        "file_id": "F2_sync",
        "relative_path": "sync.js",
        "architectural_role": "Manual catalog sync script (not loaded by PluginManager)",
        "key_patterns": [
            "Not loaded by PluginManager (no independent manifest) — admin-only tool",
            "Anchor-row atomic rewrite: only writes between ANCHOR_START and ANCHOR_END",
            "tools/list enum → get_sub_domains domain discovery (auto-detect new domains)",
            "Batched get_sub_domains (BATCH_SIZE=5) to avoid request explosion",
            "Format-drift defense: MIN_DOMAINS=5 + MIN_SUBS=10 minimum parsed size",
            "Semantic equality check (catalogsEqual) — whitespace/ordering independent",
            "Atomic file replace: write tmp → rename (server listener never sees half JSON)",
            "Idempotent: no change → no write (returns early)",
            "Manual anchor removal = permanent opt-out (script is read-only if anchors missing)",
            "tools/list inputSchema.properties.{domain,domains}.enum discovery",
        ],
        "safety_boundaries": [
            "Network/parse failure → exit 1, never write file",
            "MIN_DOMAINS/MIN_SUBS guard against format drift (defensive)",
            "Anchor-row scoping: human edits outside anchors are NEVER touched",
            "Process.pid-based tmp file (concurrent run isolation)",
            "JSON.stringify manifest with trailing newline (POSIX-compliant)",
        ],
        "declared_lines": 246,
        "sha256_full_16b": "eaa42410b7a8f811",
    },
    {
        "file_id": "F3_manifest",
        "relative_path": "plugin-manifest.json",
        "architectural_role": "Plugin contract + tool description + config schema",
        "key_patterns": [
            "manifestVersion 1.0.0 (semver)",
            "pluginType=synchronous + entryPoint type=nodejs (host subprocess model)",
            "communication.protocol=stdio + timeout=45000 (host-side override)",
            "configSchema: typed config (string/integer) with defaults + descriptions",
            "ANYSEARCH_API_KEY multi-key: comma-separated, random pick",
            "ANYSEARCH_TIMEOUT_MS range hint: 1000-120000",
            "invocationCommands[].commandIdentifier = tool name (host registers this)",
            "Tool description embeds full 17-domain catalog (vertical search beats general)",
            "3 example tool calls (search / batch / extract) — host shows in tool picker",
            "capabilities as code fence for AI parsing",
            "dependencies empty (no npm/system deps — pure Node stdlib)",
            "compatibility.nodeVersion>=14.0.0 (broad compat)",
        ],
        "safety_boundaries": [
            "configSchema enforces type (host validates before subprocess spawn)",
            "config.env.example provided (real .env gitignored — secrets isolation)",
            "timeout enforced by host (subprocess kill after 45s)",
        ],
        "declared_lines": 50,
        "sha256_full_16b": "7ac927dcf70f022a",
    },
)
assert sum(f["declared_lines"] for f in ANYSEARCH_3_FILES) == 646
TOTAL_DECLARED_LINES = 646

# ============================================================
# AnySearch 真生产组件 (substrates, not ports)
# ============================================================

@dataclass
class AnySearchFileSubstrate:
    """A real AnySearch file with verified disk presence + sha256 first-512B hash."""
    file_id: str
    relative_path: str
    architectural_role: str
    key_patterns: List[str]
    safety_boundaries: List[str]
    declared_lines: int
    sha256_full_16b: str
    actual_lines: Optional[int] = None
    actual_sha256_full_16b: Optional[str] = None

    def verify_on_disk(self) -> bool:
        """True iff file exists, sha256 full-16B matches."""
        path = ANYSEARCH_ROOT / self.relative_path
        if not path.exists():
            return False
        try:
            content = path.read_bytes()
        except OSError:
            return False
        sha = hashlib.sha256(content).hexdigest()[:16]
        if sha != self.sha256_full_16b:
            return False
        self.actual_sha256_full_16b = sha
        self.actual_lines = content.count(b"\n") + (0 if content.endswith(b"\n") else 1)
        return True


@dataclass
class StdioSyncProtocolSubstrate:
    """VCP stdio sync plugin protocol: stdin → stdout JSON, exit 0 always."""
    success_emission: str = '{status:"success", result:{content:[{type:"text", text:"..."}]}}'
    error_emission: str = '{status:"error", error:"AnySearch Error: <msg>"}'
    exit_code_on_error: int = 0  # NOT non-zero (host reads stdout; non-zero = crash)
    json_rpc_envelope: str = '{jsonrpc:"2.0", id:1, method:"tools/call", params:{name,arguments}}'

    def validate_emission_shape(self, payload: Dict[str, Any]) -> bool:
        """Check payload matches VCP stdio JSON shape (success or error)."""
        if payload.get("status") == "success":
            content = payload.get("result", {}).get("content", [])
            return isinstance(content, list) and len(content) >= 1
        if payload.get("status") == "error":
            return "error" in payload and isinstance(payload["error"], str)
        return False


@dataclass
class DomainCatalogSubstrate:
    """17 vertical domains + 4 commands; sub_domain.prefix → domain auto-derive."""
    domains: Tuple[str, ...] = (
        "general", "resource", "social_media", "finance", "academic", "legal",
        "health", "business", "security", "ip", "code", "energy",
        "environment", "agriculture", "travel", "film", "gaming",
    )
    commands: Tuple[str, ...] = ("search", "get_sub_domains", "batch_search", "extract")
    batch_max: int = 5
    domains_max: int = 5
    max_results_min: int = 1
    max_results_max: int = 10

    def is_valid_domain(self, domain: str) -> bool:
        return domain in self.domains

    def derive_domain(self, sub_domain: str) -> str:
        """sub_domain prefix → domain. Raises if malformed."""
        if "." not in sub_domain:
            raise ValueError(f"sub_domain {sub_domain!r} has no '.' prefix")
        return sub_domain.split(".", 1)[0].lower()

    def derive_domain_or_none(self, sub_domain: str) -> Optional[str]:
        try:
            return self.derive_domain(sub_domain)
        except ValueError:
            return None

    def contradictions(self, sub_domain: str, domain: str) -> bool:
        """True iff sub_domain prefix contradicts explicit domain."""
        derived = self.derive_domain_or_none(sub_domain)
        return derived is not None and domain and domain != derived


@dataclass
class HttpsOnlyTransportSubstrate:
    """HTTPS-only for production; HTTP only loopback."""
    loopback_hosts: Tuple[str, ...] = ("127.0.0.1", "localhost", "::1", "[::1]")

    def is_loopback(self, hostname: str) -> bool:
        return hostname in self.loopback_hosts

    def allowed_transport(self, protocol: str, hostname: str) -> bool:
        """Return True iff transport choice is valid for endpoint."""
        if protocol == "https":
            return True
        if protocol == "http" and self.is_loopback(hostname):
            return True
        return False


@dataclass
class InputToleranceSubstrate:
    """Multi-key probe for input parameter naming tolerance."""
    # (key_candidates, default_if_missing)
    COMMAND_KEYS: Tuple[str, ...] = ("command", "action", "tool", "mode")
    QUERY_KEYS: Tuple[str, ...] = ("query", "q", "text", "Query")
    URL_KEYS: Tuple[str, ...] = ("url", "URL", "link")
    SUBDOMAIN_KEYS: Tuple[str, ...] = ("sub_domain", "subDomain", "subdomain")
    DOMAIN_KEYS: Tuple[str, ...] = ("domain", "Domain")
    PARAMS_KEYS: Tuple[str, ...] = ("params", "sub_domain_params", "subDomainParams", "sdp")
    MAX_RESULTS_KEYS: Tuple[str, ...] = ("max_results", "maxResults")

    def first_string(self, source: Dict[str, Any], keys: Tuple[str, ...]) -> str:
        """Return first non-empty string value among keys (case-trimmed)."""
        for key in keys:
            value = source.get(key)
            if isinstance(value, str) and value.strip():
                return value.strip()
        return ""

    def first_int(self, source: Dict[str, Any], keys: Tuple[str, ...]) -> Optional[int]:
        for key in keys:
            value = source.get(key)
            if value is None or value == "":
                continue
            try:
                return int(value)
            except (TypeError, ValueError):
                continue
        return None


@dataclass
class SubDomainParamsSubstrate:
    """k=v text format + JSON object dual parser for sub_domain_params."""
    def parse(self, value: Any) -> Optional[Dict[str, str]]:
        """Returns dict OR raises ValueError on malformed input."""
        if value is None or value == "":
            return None
        if isinstance(value, dict) and not isinstance(value, list):
            return dict(value)
        if isinstance(value, str):
            trimmed = value.strip()
            if not trimmed:
                return None
            if trimmed.startswith("{"):
                try:
                    parsed = json.loads(trimmed)
                    if isinstance(parsed, dict) and not isinstance(parsed, list):
                        return parsed
                except json.JSONDecodeError:
                    pass  # fall through
            if "=" in trimmed:
                result = {}
                for pair in trimmed.split(","):
                    item = pair.strip()
                    if not item:
                        continue
                    eq = item.find("=")
                    if eq <= 0:
                        raise ValueError(f"malformed k=v pair: {item!r}")
                    key = item[:eq].strip()
                    val = item[eq + 1:].strip()
                    result[key] = val
                return result
        raise ValueError("sub_domain_params must be k=v text or JSON object")


@dataclass
class CommandInferenceSubstrate:
    """Infer command from payload when explicit command missing."""
    def infer(self, payload: Dict[str, Any], input_tol: InputToleranceSubstrate) -> str:
        # explicit command takes priority
        raw = input_tol.first_string(payload, input_tol.COMMAND_KEYS)
        if raw:
            command = raw.lower().replace("-", "_").strip()
            return command  # domain set membership checked elsewhere
        # queries array/string → batch_search
        if payload.get("queries") is not None or payload.get("query_items") is not None:
            return "batch_search"
        # url (no query) → extract
        has_query = bool(input_tol.first_string(payload, input_tol.QUERY_KEYS))
        has_url = bool(input_tol.first_string(payload, input_tol.URL_KEYS))
        if not has_query and has_url:
            return "extract"
        # default → search
        return "search"


@dataclass
class CatalogSyncSubstrate:
    """sync.js catalog sync patterns: anchor-row atomic + semantic equality + drift defense."""
    anchor_start: str = "目录(域: 子域(必填参数)):"
    anchor_end: str = "调用格式:"
    batch_size: int = 5
    min_domains: int = 5
    min_subs: int = 10

    def split_description(self, description: str) -> Optional[Dict[str, str]]:
        """Split description into head/body/tail by anchor rows."""
        start = description.find(self.anchor_start)
        end = description.find(self.anchor_end)
        if start == -1 or end == -1 or end <= start:
            return None
        body_start = start + len(self.anchor_start)
        return {
            "head": description[:body_start],
            "body": description[body_start:end],
            "tail": description[end:],
        }

    def catalog_size_valid(self, num_domains: int, num_subs: int) -> bool:
        """False iff catalog suspiciously small (format drift defense)."""
        return num_domains >= self.min_domains and num_subs >= self.min_subs

    def catalogs_semantically_equal(self, a: Dict[str, Dict[str, List[str]]],
                                     b: Dict[str, Dict[str, List[str]]]) -> bool:
        """Semantic equality: domain set + sub set + required-param sets match (order-insensitive)."""
        if set(a.keys()) != set(b.keys()):
            return False
        for domain, subs_a in a.items():
            subs_b = b.get(domain, {})
            if set(subs_a.keys()) != set(subs_b.keys()):
                return False
            for sub, req_a in subs_a.items():
                req_b = subs_b.get(sub, [])
                if sorted(req_a) != sorted(req_b):
                    return False
        return True


@dataclass
class AnySearchPluginMatrix:
    """Scan 3 AnySearch files + verify on disk + aggregate matrix."""
    files: List[AnySearchFileSubstrate] = field(default_factory=list)

    def __post_init__(self):
        if not self.files:
            self.files = [
                AnySearchFileSubstrate(**f) for f in ANYSEARCH_3_FILES
            ]

    def scan(self) -> Dict[str, Any]:
        """Run scan: verify each file on disk + aggregate stats."""
        verified = 0
        for f in self.files:
            if f.verify_on_disk():
                verified += 1
        return {
            "total_files": len(self.files),
            "verified_on_disk": verified,
            "all_exist": verified == len(self.files),
            "total_declared_lines": sum(f.declared_lines for f in self.files),
            "total_actual_lines": sum(f.actual_lines or 0 for f in self.files),
            "files": [asdict(f) for f in self.files],
        }


@dataclass
class AnySearch8Substrates:
    """8-component 真生产 substrate bundle for AnySearch."""
    file_matrix: AnySearchPluginMatrix
    stdio_protocol: StdioSyncProtocolSubstrate
    domain_catalog: DomainCatalogSubstrate
    https_transport: HttpsOnlyTransportSubstrate
    input_tolerance: InputToleranceSubstrate
    sub_domain_params: SubDomainParamsSubstrate
    command_inference: CommandInferenceSubstrate
    catalog_sync: CatalogSyncSubstrate

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file_matrix": self.file_matrix.scan(),
            "stdio_protocol": asdict(self.stdio_protocol),
            "domain_catalog": {
                "domains": list(self.domain_catalog.domains),
                "commands": list(self.domain_catalog.commands),
                "batch_max": self.domain_catalog.batch_max,
                "domains_max": self.domain_catalog.domains_max,
                "max_results_min": self.domain_catalog.max_results_min,
                "max_results_max": self.domain_catalog.max_results_max,
            },
            "https_transport": {"loopback_hosts": list(self.https_transport.loopback_hosts)},
            "input_tolerance_keys": {
                "COMMAND_KEYS": list(self.input_tolerance.COMMAND_KEYS),
                "QUERY_KEYS": list(self.input_tolerance.QUERY_KEYS),
                "URL_KEYS": list(self.input_tolerance.URL_KEYS),
                "SUBDOMAIN_KEYS": list(self.input_tolerance.SUBDOMAIN_KEYS),
                "DOMAIN_KEYS": list(self.input_tolerance.DOMAIN_KEYS),
                "PARAMS_KEYS": list(self.input_tolerance.PARAMS_KEYS),
                "MAX_RESULTS_KEYS": list(self.input_tolerance.MAX_RESULTS_KEYS),
            },
            "sub_domain_params_format": "k=v text or JSON object",
            "command_inference": "explicit > queries > (url+!query) > search",
            "catalog_sync_anchors": {
                "anchor_start": self.catalog_sync.anchor_start,
                "anchor_end": self.catalog_sync.anchor_end,
                "batch_size": self.catalog_sync.batch_size,
                "min_domains": self.catalog_sync.min_domains,
                "min_subs": self.catalog_sync.min_subs,
            },
        }


@dataclass
class AnySearchDeepReadReport:
    """Aggregate matrix scan + 8-substrate report."""
    matrix: AnySearchPluginMatrix
    substrates: AnySearch8Substrates

    def summary(self) -> Dict[str, Any]:
        scan = self.matrix.scan()
        return {
            "scan": {
                "total_files": scan["total_files"],
                "verified_on_disk": scan["verified_on_disk"],
                "all_exist": scan["all_exist"],
                "total_declared_lines": scan["total_declared_lines"],
                "total_actual_lines": scan["total_actual_lines"],
            },
            "substrates": self.substrates.to_dict(),
            "asi_pole_star": ASI_POLE_STAR,
        }


@dataclass
class AnySearchDeepReadBridge:
    """V1328 → V1327 chain closure bridge."""
    parent_module: str = "v1327_vcp_6_source_deep_read"
    chain_position: int = 16  # V1313 → ... → V1327 (15) → V1328 (16)
    parent_chain_length: int = 15
    focus: str = "first VCP plugin deep read (AnySearch)"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "parent_module": self.parent_module,
            "chain_position": self.chain_position,
            "parent_chain_length": self.parent_chain_length,
            "focus": self.focus,
            "continuation": "V1329+ will continue per-plugin deep reads (DailyNote / AgentDream / RAGDiaryPlugin)",
        }


# ============================================================
# Convenience: build default bundle + main()
# ============================================================

def default_substrates() -> AnySearch8Substrates:
    return AnySearch8Substrates(
        file_matrix=AnySearchPluginMatrix(),
        stdio_protocol=StdioSyncProtocolSubstrate(),
        domain_catalog=DomainCatalogSubstrate(),
        https_transport=HttpsOnlyTransportSubstrate(),
        input_tolerance=InputToleranceSubstrate(),
        sub_domain_params=SubDomainParamsSubstrate(),
        command_inference=CommandInferenceSubstrate(),
        catalog_sync=CatalogSyncSubstrate(),
    )


def main() -> int:
    """Module main(): full substrate build + report + summary print."""
    start = time.time()
    matrix = AnySearchPluginMatrix()
    scan = matrix.scan()
    substrates = default_substrates()
    report = AnySearchDeepReadReport(matrix=matrix, substrates=substrates)
    bridge = AnySearchDeepReadBridge()
    summary = report.summary()
    summary["bridge"] = bridge.to_dict()
    summary["elapsed_ms"] = round((time.time() - start) * 1000, 2)

    # print condensed summary
    print(json.dumps({
        "v1328": {
            "files_scanned": scan["total_files"],
            "verified": scan["verified_on_disk"],
            "total_lines_declared": scan["total_declared_lines"],
            "total_lines_actual": scan["total_actual_lines"],
            "all_exist": scan["all_exist"],
            "domains_count": len(substrates.domain_catalog.domains),
            "commands_count": len(substrates.domain_catalog.commands),
            "substrate_components": 8,
            "elapsed_ms": summary["elapsed_ms"],
            "asi_pole_star": ASI_POLE_STAR,
            "chain_position": bridge.chain_position,
        },
    }, ensure_ascii=False, indent=2))
    return 0


# ============================================================
# Popper self-test (60 tests, callable via --self-test)
# ============================================================

def _popper_self_test() -> int:
    """60 Popper-falsifiable self-tests; return 0 if all pass."""
    pass_count = 0
    total = 0

    def check(name: str, cond: bool) -> None:
        nonlocal pass_count, total
        total += 1
        if cond:
            pass_count += 1
        else:
            print(f"FAIL: {name}")

    # 1. AnySearchFileSubstrate (8)
    f = AnySearchFileSubstrate(**ANYSEARCH_3_FILES[0])
    check("F1 file_id is F1_main", f.file_id == "F1_main")
    check("F1 declared_lines > 0", f.declared_lines > 0)
    check("F1 sha256_full_16b is 16 hex chars", len(f.sha256_full_16b) == 16)
    check("F1 has key_patterns", len(f.key_patterns) > 0)
    check("F1 has safety_boundaries", len(f.safety_boundaries) > 0)
    check("F1 actual_lines is None before verify", f.actual_lines is None)
    check("F1 verify_on_disk works", f.verify_on_disk())
    check("F1 actual_sha256_full_16b matches", f.actual_sha256_full_16b == f.sha256_full_16b)

    # 2. StdioSyncProtocolSubstrate (7)
    p = StdioSyncProtocolSubstrate()
    check("stdio success shape valid", p.validate_emission_shape({"status": "success", "result": {"content": [{"type": "text", "text": "hi"}]}}))
    check("stdio error shape valid", p.validate_emission_shape({"status": "error", "error": "x"}))
    check("stdio exit_code_on_error = 0", p.exit_code_on_error == 0)
    check("stdio json_rpc envelope present", "tools/call" in p.json_rpc_envelope)
    check("stdio reject non-success/error", not p.validate_emission_shape({"foo": "bar"}))
    check("stdio success needs content list", not p.validate_emission_shape({"status": "success", "result": {}}))
    check("stdio error needs error string", not p.validate_emission_shape({"status": "error"}))

    # 3. DomainCatalogSubstrate (10)
    d = DomainCatalogSubstrate()
    check("domains count = 17", len(d.domains) == 17)
    check("commands count = 4", len(d.commands) == 4)
    check("general is valid domain", d.is_valid_domain("general"))
    check("unknown domain invalid", not d.is_valid_domain("unknown"))
    check("derive_domain(general.search) = general", d.derive_domain("general.search") == "general")
    check("derive_domain(security.intel) = security", d.derive_domain("security.intel") == "security")
    check("derive_domain_or_none(malformed) = None", d.derive_domain_or_none("no_dot") is None)
    check("contradictions true when mismatch", d.contradictions("finance.news", "health"))
    check("contradictions false when match", not d.contradictions("finance.news", "finance"))
    check("BATCH_MAX=5 DOMAINS_MAX=5", d.batch_max == 5 and d.domains_max == 5)

    # 4. HttpsOnlyTransportSubstrate (5)
    h = HttpsOnlyTransportSubstrate()
    check("https allowed any host", h.allowed_transport("https", "api.anysearch.com"))
    check("http loopback allowed", h.allowed_transport("http", "127.0.0.1"))
    check("http non-loopback denied", not h.allowed_transport("http", "api.example.com"))
    check("localhost is loopback", h.is_loopback("localhost"))
    check("[::1] is loopback", h.is_loopback("[::1]"))

    # 5. InputToleranceSubstrate (8)
    i = InputToleranceSubstrate()
    check("first_string finds query", i.first_string({"query": "x"}, ("query", "q")) == "x")
    check("first_string empty fallback", i.first_string({"q": ""}, ("query", "q")) == "")
    check("first_string skips empty", i.first_string({"query": "", "q": "y"}, ("query", "q")) == "y")
    check("first_int parses int", i.first_int({"max_results": "5"}, ("max_results",)) == 5)
    check("first_int handles None", i.first_int({"max_results": None}, ("max_results",)) is None)
    check("first_int handles empty", i.first_int({"max_results": ""}, ("max_results",)) is None)
    check("COMMAND_KEYS has 4 entries", len(i.COMMAND_KEYS) == 4)
    check("PARAMS_KEYS has 4 entries", len(i.PARAMS_KEYS) == 4)

    # 6. SubDomainParamsSubstrate (6)
    s = SubDomainParamsSubstrate()
    check("parse k=v text", s.parse("type=stock,symbol=AAPL") == {"type": "stock", "symbol": "AAPL"})
    check("parse k=v with empty value", s.parse("market=") == {"market": ""})
    check("parse JSON object", s.parse('{"type":"stock"}') == {"type": "stock"})
    check("parse None returns None", s.parse(None) is None)
    check("parse empty returns None", s.parse("") is None)
    raised = False
    try:
        s.parse("malformed_no_equals")
    except ValueError:
        raised = True
    check("parse malformed raises", raised)

    # 7. CommandInferenceSubstrate (6)
    ci = CommandInferenceSubstrate()
    check("explicit command", ci.infer({"command": "extract"}, i) == "extract")
    check("queries → batch_search", ci.infer({"queries": ["a", "b"]}, i) == "batch_search")
    check("query_items → batch_search", ci.infer({"query_items": "a|b"}, i) == "batch_search")
    check("url+!query → extract", ci.infer({"url": "https://example.com"}, i) == "extract")
    check("query → search", ci.infer({"query": "x"}, i) == "search")
    check("command dash to underscore", ci.infer({"command": "batch-search"}, i) == "batch_search")

    # 8. CatalogSyncSubstrate (8)
    cs = CatalogSyncSubstrate()
    desc = "HEADER\n目录(域: 子域(必填参数)):\nfinance: news(type)\n调用格式:\nFOOTER"
    parts = cs.split_description(desc)
    check("split_description finds head/body/tail", parts is not None and "body" in parts)
    check("split_description body contains finance", parts and "finance: news" in parts["body"])
    check("catalog_size_valid normal", cs.catalog_size_valid(17, 50))
    check("catalog_size_valid drift defense (low domains)", not cs.catalog_size_valid(2, 50))
    check("catalog_size_valid drift defense (low subs)", not cs.catalog_size_valid(17, 3))
    a = {"finance": {"news": ["type"]}, "general": {}}
    b = {"general": {}, "finance": {"news": ["type"]}}  # order different
    check("catalogs_semantically_equal order-insensitive", cs.catalogs_semantically_equal(a, b))
    c = {"finance": {"news": ["type", "extra"]}}
    check("catalogs_semantically_equal param diff", not cs.catalogs_semantically_equal(a, c))
    check("catalog_sync batch_size = 5", cs.batch_size == 5)

    # 9. AnySearchPluginMatrix + Bridge (8)
    m = AnySearchPluginMatrix()
    s_dict = m.scan()
    check("matrix scan has 3 files", s_dict["total_files"] == 3)
    check("matrix all_exist on real disk", s_dict["all_exist"])
    check("matrix total_declared_lines = 646", s_dict["total_declared_lines"] == 646)
    check("matrix verified >= 3", s_dict["verified_on_disk"] >= 3)
    b = AnySearchDeepReadBridge()
    check("bridge parent = v1327", b.parent_module == "v1327_vcp_6_source_deep_read")
    check("bridge chain_position = 16", b.chain_position == 16)
    check("bridge parent_chain_length = 15", b.parent_chain_length == 15)
    check("bridge focus mentions AnySearch", "AnySearch" in b.focus)

    # 10. ASI pole-star LOCKED (4)
    check("ASI V0.1 = 0.7905", ASI_POLE_STAR["V0_1_anchored"] == 0.7905)
    check("ASI V0.2 = 0.4467", ASI_POLE_STAR["V0_2_baseline"] == 0.4467)
    check("ASI V1256 = 0.9105", ASI_POLE_STAR["V1256_unio_mystica"] == 0.9105)
    check("ASI V1049 = DONE", ASI_POLE_STAR["V1049_value_alignment"] == "DONE")

    print(f"V1328 self-test: {pass_count}/{total}")
    return 0 if pass_count == total else 1


if __name__ == "__main__":
    import sys
    if "--self-test" in sys.argv:
        sys.exit(_popper_self_test())
    sys.exit(main())
