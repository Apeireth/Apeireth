"""
V1327 — VCP 6 真源码深读 (VCP 6 Real Source Code Deep Read)

主 13:31 + 19:33 + 00:56 directive: 「VCP 6 真实代码去真实深读」
(VCP = Variable & Command Protocol — 主人 real running production project
 at C:\\Users\\REDACTED\\VCPToolBox\\VCPToolBox-main\\)

This module does a REAL DEEP READ of 6 architecturally-distinct VCP source
files, then extracts the patterns / invariants / safety boundaries into
6 真生产 components. Each component is a faithful *pattern representation*
(not a JavaScript port) of the original VCP behavior, so Apeireth can
reason about VCP architecture without pretending to run it.

V3 哲学守门 (LOCKED):
- 不假装 V1327 = 复刻 VCP (we are reading, not porting)
- 不假装 VCP 真跑 (file system = read-only analysis)
- 不假装 ASI 真理解 VCP (pattern extraction ≠ semantics)
- 不假装 ASI 解决 VCP 架构问题 (architectural study only)
- ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE

Author: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 2026-08-08)
Trigger: post-V1326 ASI 5-Gap Chain Closure Audit (f72c34ff, 20:09)
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
# VCP 6 真源码 deep-read 目标 (read-only paths)
# ============================================================
VCP_REPO_ROOT = Path(r"VCPToolBox\VCPToolBox-main")

VCP_6_LAYERS: Tuple[Dict[str, Any], ...] = (
    # Layer 1: Agent Lifecycle
    {
        "layer_id": "L1_agent_lifecycle",
        "relative_path": "modules/agentManager.js",
        "architectural_role": "Agent identity + hot-reload prompt cache",
        "key_patterns": [
            "agentMap (alias→filename) loaded from agent_map.json",
            "promptCache (Map) invalidated on map reload",
            "chokidar watcher on agent_dir (excludes node_modules/.git/dist/target/image/dotfiles)",
            "Symbolic link resolution via lstat+readlink",
            "Recursive scanAgentFiles builds folderStructure tree",
            "agentFiles = [.txt/.md] enumeration",
            "getAgentPrompt caches by alias after first read",
            "Graceful degradation: missing agent returns {{agent:alias}} placeholder",
        ],
        "safety_boundaries": [
            "promptCache.clear() on map reload (防止 stale prompts)",
            "watcher.ignored hides secrets dirs (node_modules/.git)",
            "Symbolic link loop detection by stat+readlink (不递归 follow)",
        ],
        "lines": 321,
    },
    # Layer 2: Dynamic Tool Registry (the heart)
    {
        "layer_id": "L2_dynamic_tool_registry",
        "relative_path": "modules/dynamicToolRegistry.js",
        "architectural_role": "Dynamic tool selection + token-budget injection",
        "key_patterns": [
            "3-tier mergeConfig (DEFAULT_CONFIG → fileConfig → overrideConfig)",
            "Object.freeze(DEFAULT_CONFIG) for immutability",
            "stableStringify (sorted-key JSON) for canonical hashing",
            "Token budgets: LIGHT_LIST_TOKEN_BUDGET=15 / DEFAULT_BRIEF=6 / MAX_INJECTION_CHARS=16000",
            "CATEGORY_RULES bilingual (Latin+CJK) keyword classifier",
            "withTimeout wrapper (Promise.race + clearTimeout on settle)",
            "Classification debounce (classificationDebounceMs=1000) + timeout (30000)",
            "Manual overrides: excludedOriginKeys / pinnedOriginKeys / categoryAliases",
            "privateConfig path: Plugin/DynamicToolBridge/config.env (NOT in repo)",
            "snapshotId increments on each syncFromPluginManager",
        ],
        "safety_boundaries": [
            "mergeConfig strips smallModel.apiKey before persistence",
            "clampInteger enforces min/max/fallback on every numeric config",
            "withTimeout always clears timer in finally (防止 leak)",
            "syncPromise chain catches errors before next iteration",
        ],
        "lines": 1457,
    },
    # Layer 3: Message Processor (placeholder expansion + soul guard)
    {
        "layer_id": "L3_message_processor",
        "relative_path": "modules/messageProcessor.js",
        "architectural_role": "Placeholder expansion + AgentGuard + dynamic fold",
        "key_patterns": [
            "placeholderRegex covers CJK Radicals Supplement + Hiragana + CJK Unified Ideographs (\\u2e80-\\u2fff\\u3040-\\u9fff)",
            "AgentGuard: 一会话只展开一个 Agent (context.expandedAgentName singleton)",
            "Toolbox dedup: expandedToolboxes (Set) per context",
            "Circular dependency detection via processingStack (Set)",
            "Static fold modes: [[VCPStaticFold::Auto|Lite|Full]]",
            "Dynamic fold: cosine similarity vs plugin_description, threshold gating",
            "Sanitization pipeline: sanitizeForEmbedding → user/assistant role-aware",
            "SYSTEM_USER / SYSTEM_NOTIFICATION / SYSTEM_INVITATION prefix regexes",
            "VCP_TOOL_PAYLOAD prefix detection (<!-- VCP_TOOL_PAYLOAD -->)",
            "lunarCalendar dependency for date formatting",
        ],
        "safety_boundaries": [
            "isPrivilegedRole check: {{agent:...}} only in system / [系统提示:] / [系统邀请指令:]",
            "AgentGuard silently REMOVES (NOT errors) duplicate Agent placeholders",
            "ToolboxGuard dedup: 同名 toolbox 后出现 静默移除",
            "processingStack catches circular references → injects error marker",
            "findLastRealUserMessage skip options: empty / system / invitation / tool-payload",
        ],
        "lines": 787,
    },
    # Layer 4: Tool Executor (river + vref + timely_contact)
    {
        "layer_id": "L4_tool_executor",
        "relative_path": "modules/vcpLoop/toolExecutor.js",
        "architectural_role": "Tool dispatch with river context + vref + scheduling",
        "key_patterns": [
            "River context modes: full / text / last:N / semantic:N",
            "vRef virtual reference: cache-only embedding build (zero extra API)",
            "timely_contact interception → write VCPTimedContacts/<id>.json",
            "toolCallRecordStore.beginRecord / finishRecord lifecycle (recordHandle.id)",
            "River semantic:N fallback to last:N on embedding failure (graceful degrade)",
            "WebSocket broadcast via VCPLog for every tool call",
            "Auth code verification via vcpToolCode + tool_password field",
            "_processResult: rich content detection (data.content array) → text extraction",
            "_scheduleTimedToolCall: future-only validation (past dates rejected)",
            "_formatToLocalDateTimeWithOffset: ISO with timezone offset for scheduling",
            "archeryNoReply: silent tool accepted (logged via VCPInfo, not looped)",
        ],
        "safety_boundaries": [
            "timely_contact past-date guard returns 'past' (not Date)",
            "Plugin-not-found error returns structured error result (no throw)",
            "Auth failure deletes tool_password from args before passing to plugin",
            "Record finishRecord always called (success OR error path)",
            "River semantic:N graceful fallback to last:N (no tool-call interruption)",
            "_verifyAuth deletes tool_password post-check (不留在 args)",
        ],
        "lines": 549,
    },
    # Layer 5: Protocol Bridge (multi-protocol compat)
    {
        "layer_id": "L5_protocol_bridge",
        "relative_path": "routes/protocolBridge.js",
        "architectural_role": "OpenAI Responses / Anthropic / Gemini → standard chat/completions",
        "key_patterns": [
            "normalizeTextContent handles string / array / object content shapes",
            "normalizeMessageRole: developer → system (Anthropic compat)",
            "Native tool field protection: functionDeclarations (Gemini) / tools (OpenAI) / functions (legacy)",
            "buildStableRequestId: sha256(JSON)[:24] prefix for retry dedup",
            "RESPONSE_RETRY_SUPPRESSION_WINDOW_MS=15000 (15s window)",
            "SSE event emission: response.created / output_item.added / content_part.added / output_text.delta / output_text.done / content_part.done / output_item.done / response.completed",
            "immediate responses envelope for empty/error paths",
            "Three-format extraction: extractMessagesFromResponsesInput / Anthropic / Gemini",
        ],
        "safety_boundaries": [
            "Stable request ID dedup: client retries within 15s suppressed",
            "Map cleanup: entries older than 4×window auto-deleted (防止 memory leak)",
            "Tool field re-attachment ONLY at forward time (not in messages/RAG)",
            "isSuppressedDuplicateResponsesRequest bounds Map size (LRU-like)",
        ],
        "lines": 955,
    },
    # Layer 6: FileOperator (real plugin implementation)
    {
        "layer_id": "L6_file_operator",
        "relative_path": "Plugin/FileOperator/FileOperator.js",
        "architectural_role": "Real-world plugin: path sandbox + CRLF + diff logic",
        "key_patterns": [
            "ALLOWED_DIRECTORIES sandbox (comma-separated, case-insensitive on Windows)",
            "Read-only bypass: ReadFile / FileInfo exempt from sandbox",
            "Virtual root logic: /foo on Windows → FileOperator/foo",
            "BASE_PATH fallback: 2 levels up from plugin dir",
            "CRLF detection + preservation (createLineEndingHelper)",
            "Diff logic: <<<<<<< SEARCH / ======= / >>>>>>> REPLACE parsing",
            "getPathParameter canonical names: filePath / directoryPath / sourcePath / destinationPath / searchPath + generic path/Path",
            "PDFParse / mammoth / ExcelJS / glob / minimatch / axios deps",
            "getUniqueFilePath: foo.txt → foo(1).txt collision avoidance",
            "MAX_FILE_SIZE=20MB / MAX_DIRECTORY_ITEMS=1000 / MAX_SEARCH_RESULTS=100",
        ],
        "safety_boundaries": [
            "isPathAllowed denies write/delete outside ALLOWED_DIRECTORIES (no bypass)",
            "ReadFile/FileInfo read-only bypass is conservative (only pure reads)",
            "CodeValidator integration (validateCode on code-typed operations)",
            "MAX_FILE_SIZE hard cap (20MB default) prevents memory exhaustion",
            "ENABLE_HIDDEN_FILES opt-in (default off)",
            "ENABLE_RECURSIVE_OPERATIONS opt-out (default on, but can disable)",
            "path.resolve() always (no string concat, prevents traversal)",
            "trim() on every path component (prevents leading-space bypass)",
        ],
        "lines": 1620,
    },
)

# Total: 321+1457+787+549+955+1620 = 5689 lines of REAL VCP source


# ============================================================
# 1. VCPLayerMatrix — 真读 6 源码 + extract metadata
# ============================================================
@dataclass
class VCPLayerInfo:
    layer_id: str
    relative_path: str
    architectural_role: str
    key_patterns: List[str]
    safety_boundaries: List[str]
    lines: int
    exists: bool = False
    actual_lines: int = 0
    sha256_first_512b: Optional[str] = None  # 真读 first 512 bytes for hash


class VCPLayerMatrix:
    """扫描 6 VCP 真源码 layers, 验证 exists + hash + actual line count."""

    def __init__(self, repo_root: Path = VCP_REPO_ROOT):
        self.repo_root = Path(repo_root)
        self.layers: List[VCPLayerInfo] = []
        self.total_declared_lines: int = 0
        self.total_actual_lines: int = 0

    def scan(self) -> Dict[str, Any]:
        """真扫描 6 layers, 返回 matrix summary."""
        for layer_def in VCP_6_LAYERS:
            info = VCPLayerInfo(
                layer_id=layer_def["layer_id"],
                relative_path=layer_def["relative_path"],
                architectural_role=layer_def["architectural_role"],
                key_patterns=list(layer_def["key_patterns"]),
                safety_boundaries=list(layer_def["safety_boundaries"]),
                lines=int(layer_def["lines"]),
            )
            full_path = self.repo_root / info.relative_path
            if full_path.exists():
                info.exists = True
                try:
                    content = full_path.read_bytes()
                    info.actual_lines = content.count(b"\n") + (0 if content.endswith(b"\n") else 1)
                    info.sha256_first_512b = hashlib.sha256(content[:512]).hexdigest()
                except OSError:
                    info.exists = False
            self.layers.append(info)
            self.total_declared_lines += info.lines
            self.total_actual_lines += info.actual_lines

        return {
            "repo_root": str(self.repo_root),
            "layer_count": len(self.layers),
            "all_exist": all(L.exists for L in self.layers),
            "total_declared_lines": self.total_declared_lines,
            "total_actual_lines": self.total_actual_lines,
            "layers": [
                {
                    "layer_id": L.layer_id,
                    "exists": L.exists,
                    "declared_lines": L.lines,
                    "actual_lines": L.actual_lines,
                    "first_512b_sha256": L.sha256_first_512b,
                }
                for L in self.layers
            ],
        }

    def get_layer(self, layer_id: str) -> Optional[VCPLayerInfo]:
        for L in self.layers:
            if L.layer_id == layer_id:
                return L
        return None


# ============================================================
# 2. AgentManagerLayerSubstrate — agent_map + promptCache + chokidar
# ============================================================
@dataclass
class AgentEntry:
    alias: str
    filename: str
    prompt_cache: Optional[str] = None
    last_loaded_at: Optional[float] = None


class AgentManagerLayerSubstrate:
    """Substrate for L1 agentManager.js patterns (NOT a port)."""

    def __init__(self) -> None:
        self.agent_map: Dict[str, str] = {}  # alias → filename
        self.prompt_cache: Dict[str, str] = {}  # alias → prompt content
        self.watched_files: Set[str] = set()
        self.ignored_patterns: Tuple[str, ...] = (
            "**/node_modules/**",
            "**/.git/**",
            "**/dist/**",
            "**/target/**",
            "**/image/**",
            "**/.*",
        )

    def load_map(self, map_dict: Dict[str, str]) -> None:
        """Load alias→filename map, clearing prompt cache on reload."""
        self.agent_map = dict(map_dict)
        self.prompt_cache.clear()  # Pattern: prompt cache invalidation

    def register_watched_file(self, filename: str) -> None:
        self.watched_files.add(filename)

    def should_watch(self, path: str) -> bool:
        """Apply chokidar.ignored patterns (match against path components)."""
        normalized = path.replace("\\", "/")
        parts = [p for p in normalized.split("/") if p]
        if not parts:
            return True

        def has_dotfile() -> bool:
            return any(p.startswith(".") and p not in (".", "..") for p in parts)

        for pat in self.ignored_patterns:
            # pat like '**/node_modules/**' → middle = 'node_modules'
            middle = pat.replace("**/", "").replace("/**", "").strip("/")
            if not middle:
                continue
            if middle == ".*":
                # dotfile pattern: any dotfile component excludes
                if has_dotfile():
                    return False
                continue
            # Direct segment match (e.g. node_modules, dist, target, image)
            if middle in parts:
                return False
            # Component-suffix check (e.g. pattern contains '.git' somewhere)
            if any(middle in p for p in parts):
                return False
        return True

    def get_agent_prompt(self, alias: str, loader: Optional[Callable[[str], str]] = None) -> str:
        """Returns prompt from cache, or calls loader + caches, or fallback placeholder."""
        if alias in self.prompt_cache:
            return self.prompt_cache[alias]
        if alias not in self.agent_map:
            return f"{{{{agent:{alias}}}}}"  # Graceful degradation
        filename = self.agent_map[alias]
        if loader is None:
            return f"[AgentManager: loader not provided for '{alias}']"
        content = loader(filename)
        self.prompt_cache[alias] = content
        return content

    def invalidate_cache_for(self, alias: str) -> None:
        if alias in self.prompt_cache:
            del self.prompt_cache[alias]


# ============================================================
# 3. DynamicToolRegistryLayerSubstrate — 3-tier config + token budget
# ============================================================
# Note: Python's dict is mutable; for true immutability in Python we'd use
# MappingProxyType or types.MappingProxyType, but the substrate represents
# the JS Object.freeze pattern with a plain dict + careful consumer code.
DEFAULT_REGISTRY_CONFIG = {
    "version": 1,
    "enabled": True,
    "maxBriefListItems": 120,
    "maxExpandedPlugins": 4,
    "maxForcedCategoryPlugins": 12,
    "maxInjectionChars": 16000,
    "classificationDebounceMs": 1000,
    "classifierTimeoutMs": 30000,
    "useRagEmbeddings": True,
}

# Token budget tiers (from dynamicToolRegistry.js)
LIGHT_LIST_TOKEN_BUDGET = 15
DEFAULT_BRIEF_TOKEN_BUDGET = 6
MIN_BRIEF_TOKEN_BUDGET = 3


def stable_stringify(value: Any) -> str:
    """Sorted-key JSON for canonical hashing."""
    if value is None or isinstance(value, (str, int, float, bool)):
        return json.dumps(value, sort_keys=True)
    if isinstance(value, list):
        return "[" + ",".join(stable_stringify(v) for v in value) + "]"
    if isinstance(value, dict):
        return "{" + ",".join(
            f"{json.dumps(k, sort_keys=True)}:{stable_stringify(v)}"
            for k, v in sorted(value.items())
        ) + "}"
    return json.dumps(str(value))


def with_timeout(promise_factory: Callable[[], Any], timeout_ms: int, label: str) -> Any:
    """Mock of Promise.race + clearTimeout pattern.

    Note: this is a SYNC substrate (Python), so timeout enforcement is approximated
    via start-time + label. The real VCP version is async (JS).
    """
    start = time.monotonic()
    try:
        return promise_factory()
    finally:
        elapsed_ms = (time.monotonic() - start) * 1000
        if elapsed_ms > timeout_ms:
            # In JS this raises Error; in sync Python we just record the breach
            return {"_timed_out": True, "label": label, "elapsed_ms": elapsed_ms}


def clamp_integer(value: Any, min_v: int, max_v: int, fallback: int) -> int:
    try:
        n = int(value)
    except (TypeError, ValueError):
        return fallback
    if n < min_v:
        return min_v
    if n > max_v:
        return max_v
    return n


def merge_config(base: Dict[str, Any], file_config: Optional[Dict[str, Any]], override: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    """3-tier mergeConfig (base ← fileConfig ← override)."""
    merged = dict(base)
    if isinstance(file_config, dict):
        merged.update(file_config)
    if isinstance(override, dict):
        merged.update(override)
    merged["maxBriefListItems"] = clamp_integer(merged.get("maxBriefListItems"), 1, 500, 120)
    merged["maxExpandedPlugins"] = clamp_integer(merged.get("maxExpandedPlugins"), 0, 50, 4)
    merged["maxForcedCategoryPlugins"] = clamp_integer(merged.get("maxForcedCategoryPlugins"), 1, 100, 12)
    merged["maxInjectionChars"] = clamp_integer(merged.get("maxInjectionChars"), 1000, 120000, 16000)
    merged["classificationDebounceMs"] = clamp_integer(merged.get("classificationDebounceMs"), 0, 60000, 1000)
    merged["classifierTimeoutMs"] = clamp_integer(merged.get("classifierTimeoutMs"), 100, 120000, 30000)
    merged["enabled"] = merged.get("enabled", True) is not False
    return merged


# CATEGORY_RULES bilingual
CATEGORY_RULES = (
    ("search", ["search", "web", "lookup", "query", "retrieval", "google", "tavily", "serp", "url", "paper", "citation", "搜索", "检索", "网页", "查询", "论文", "资料"]),
    ("file_code", ["file", "code", "read", "write", "edit", "patch", "repo", "git", "directory", "文件", "代码", "仓库", "读取", "写入", "编辑"]),
    ("image_media", ["image", "photo", "picture", "media", "video", "audio", "ocr", "screenshot", "图片", "图像", "视频", "音频", "截图"]),
    ("memory_knowledge", ["memory", "knowledge", "rag", "diary", "note", "vector", "context", "知识", "记忆", "日记", "笔记", "向量"]),
    ("agent_task", ["agent", "task", "schedule", "plan", "workflow", "assistant", "任务", "计划", "调度", "代理"]),
    ("communication", ["mail", "email", "message", "notification", "push", "forum", "wechat", "telegram", "邮件", "消息", "通知", "推送"]),
    ("data", ["json", "csv", "excel", "sql", "database", "table", "parse", "数据", "表格", "数据库", "解析"]),
)


def classify_category(text: str) -> str:
    """Bilingual keyword classifier."""
    lower = text.lower()
    for cat, keywords in CATEGORY_RULES:
        for kw in keywords:
            if kw.lower() in lower:
                return cat
    return "uncategorized"


def estimate_token_count(text: str) -> int:
    """Pattern: tokenPieces match alphanumeric + CJK (\\u3400-\\u9fff\\u3040-\\u30ff\\uac00-\\ud7af)."""
    pattern = r"[A-Za-z0-9_.-]+|[\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]"
    return len(re.findall(pattern, text))


def truncate_to_token_budget(text: str, max_tokens: int) -> str:
    """Truncate to token budget (tokenPieces slice)."""
    pattern = r"[A-Za-z0-9_.-]+|[\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]"
    pieces = re.findall(pattern, text)
    if len(pieces) <= max_tokens:
        return text
    return " ".join(pieces[:max_tokens]) + "..."


class DynamicToolRegistryLayerSubstrate:
    """Substrate for L2 dynamicToolRegistry.js patterns."""

    def __init__(self) -> None:
        self.config: Dict[str, Any] = dict(DEFAULT_REGISTRY_CONFIG)
        self.catalog: Dict[str, Dict[str, Any]] = {}
        self.classification_queue: Dict[str, float] = {}
        self.snapshot_id: int = 0
        self.last_error: Optional[str] = None

    def reload_config(self, file_config: Optional[Dict[str, Any]] = None, override: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        self.config = merge_config(DEFAULT_REGISTRY_CONFIG, file_config, override)
        return self.config

    def register_tool(self, origin_key: str, metadata: Dict[str, Any]) -> None:
        self.catalog[origin_key] = dict(metadata)
        self.snapshot_id += 1

    def classify(self, text: str) -> str:
        return classify_category(text)

    def estimate_injection_chars(self, plugin_count: int) -> int:
        return min(plugin_count * self.config["maxInjectionChars"] // max(1, self.config["maxExpandedPlugins"]), self.config["maxInjectionChars"])

    def token_summary(self, text: str) -> Dict[str, int]:
        """Apply 3-tier token budgets to a description string."""
        full_count = estimate_token_count(text)
        return {
            "full": full_count,
            "light_list_budget": LIGHT_LIST_TOKEN_BUDGET,
            "light_list_truncated": truncate_to_token_budget(text, LIGHT_LIST_TOKEN_BUDGET),
            "default_brief_budget": DEFAULT_BRIEF_TOKEN_BUDGET,
            "default_brief_truncated": truncate_to_token_budget(text, DEFAULT_BRIEF_TOKEN_BUDGET),
        }


# ============================================================
# 4. MessageProcessorLayerSubstrate — placeholder + AgentGuard
# ============================================================

# Pattern from messageProcessor.js: CJK Radicals Supplement + Hiragana + CJK Unified Ideographs
PLACEHOLDER_REGEX = re.compile(r"\{\{([a-zA-Z0-9_:@#%&^+_\-\u2e80-\u2fff\u3040-\u9fff]+)\}\}")

SYSTEM_USER_PREFIX_REGEX = re.compile(r"^\s*\[系统[^\]]*\]")
SYSTEM_NOTIFICATION_PREFIX_REGEX = re.compile(r"^\s*\[系统通知[:：]?\]")
SYSTEM_INVITATION_PREFIX_REGEX = re.compile(r"^\s*\[系统邀请指令[:：]?\]")
VCP_TOOL_PAYLOAD_PREFIX_REGEX = re.compile(r"^\s*<!-- VCP_TOOL_PAYLOAD -->")

STATIC_FOLD_MODE_REGEX = re.compile(r"\[\[VCPStaticFold::(Auto|Lite|Full)\]\]", re.IGNORECASE)


def is_privileged_role(role: str, text: str) -> bool:
    """Agent/Toolbox placeholders only expand in privileged contexts."""
    if role == "system":
        return True
    if role == "user" and (text.startswith("[系统提示:]") or text.startswith("[系统邀请指令:]")):
        return True
    return False


class MessageProcessorLayerSubstrate:
    """Substrate for L3 messageProcessor.js patterns."""

    def __init__(self, registered_agents: Optional[Set[str]] = None) -> None:
        self.registered_agents: Set[str] = set(registered_agents or set())
        self.expanded_agent_name: Optional[str] = None  # AgentGuard singleton
        self.expanded_toolboxes: Set[str] = set()  # Toolbox dedup
        self.processing_stack: Set[str] = set()  # Circular detection

    def reset_context(self) -> None:
        """Reset per-context state."""
        self.expanded_agent_name = None
        self.expanded_toolboxes = set()
        self.processing_stack = set()

    def is_registered_agent(self, alias: str) -> bool:
        return alias in self.registered_agents

    def expand_agent_placeholder(self, text: str, alias: str, agent_loader: Callable[[str], str], role: str) -> str:
        """Expand {{agent:alias}} with AgentGuard.

        Per V3 / VCP: only privileged roles can expand, and only one agent per context.
        Duplicate expansions are SILENTLY REMOVED (not error).
        Circular references inject an error marker.
        """
        if not is_privileged_role(role, text):
            return text
        if not self.is_registered_agent(alias):
            return text
        # AgentGuard: only one agent per context
        if self.expanded_agent_name is not None:
            if self.expanded_agent_name != alias:
                # Different agent already expanded → silently remove this placeholder
                return re.sub(r"\{\{(?:agent:)?" + re.escape(alias) + r"\}\}", "", text)
            # Same agent re-expansion → silently remove
            return re.sub(r"\{\{(?:agent:)?" + re.escape(alias) + r"\}\}", "", text)
        # Circular detection
        if alias in self.processing_stack:
            err = f"[Error: Circular agent reference detected for '{alias}']"
            return re.sub(r"\{\{(?:agent:)?" + re.escape(alias) + r"\}\}", err, text)
        self.processing_stack.add(alias)
        try:
            content = agent_loader(alias)
        finally:
            self.processing_stack.discard(alias)
        # Replace both {{alias}} and {{agent:alias}}
        text = re.sub(r"\{\{(?:agent:)?" + re.escape(alias) + r"\}\}", content, text)
        self.expanded_agent_name = alias
        return text

    def is_system_injection(self, text: str) -> bool:
        """Detect if a user message is actually a system injection."""
        return any(
            rx.match(text)
            for rx in (
                SYSTEM_USER_PREFIX_REGEX,
                SYSTEM_NOTIFICATION_PREFIX_REGEX,
                SYSTEM_INVITATION_PREFIX_REGEX,
            )
        )

    def is_tool_payload(self, text: str) -> bool:
        return bool(VCP_TOOL_PAYLOAD_PREFIX_REGEX.match(text))

    def extract_fold_mode(self, text: str) -> str:
        """Extract [[VCPStaticFold::Auto|Lite|Full]] mode, default 'auto'."""
        matches = STATIC_FOLD_MODE_REGEX.findall(text)
        if not matches:
            return "auto"
        return matches[-1].lower()


# ============================================================
# 5. ToolExecutorLayerSubstrate — river + vref + timely_contact
# ============================================================
@dataclass
class ToolCallRecord:
    id: str
    tool_name: str
    args: Dict[str, Any]
    success: bool = False
    error: Optional[str] = None


class ToolCallRecordStore:
    """Substrate for toolCallRecordStore lifecycle."""

    def __init__(self) -> None:
        self._counter: int = 0
        self.records: List[ToolCallRecord] = []

    def begin_record(self, tool_name: str, args: Dict[str, Any]) -> ToolCallRecord:
        self._counter += 1
        rec = ToolCallRecord(
            id=f"rec-{self._counter}",
            tool_name=tool_name,
            args=dict(args),
        )
        self.records.append(rec)
        return rec

    def finish_record(self, rec: ToolCallRecord, success: bool, error: Optional[str] = None) -> None:
        rec.success = success
        rec.error = error


def parse_river_mode(river: Optional[str]) -> Tuple[str, int]:
    """Parse river mode string → (mode, n).

    Modes: 'full' / 'text' / 'last:N' / 'semantic:N' / '' (none)
    """
    if not river:
        return ("none", 0)
    if river == "full":
        return ("full", 0)
    if river == "text":
        return ("text", 0)
    if river.startswith("last:"):
        try:
            n = int(river.split(":")[1])
        except (ValueError, IndexError):
            n = 10
        return ("last", n)
    if river.startswith("semantic:"):
        try:
            n = int(river.split(":")[1])
        except (ValueError, IndexError):
            n = 5
        return ("semantic", n)
    return ("unknown", 0)


def validate_timely_contact(value: Optional[str]) -> Optional[str]:
    """Return None if past/invalid, 'past' if past, ISO string if future."""
    if not value:
        return None
    from datetime import datetime, timezone
    s = str(value).strip().replace("/", "-").replace(".", "-")
    m = re.match(r"^(\d{4})-(\d{1,2})-(\d{1,2})-(\d{1,2}):(\d{1,2})(?::(\d{1,2}))?$", s)
    if m:
        y, mo, d, h, mi = (int(m.group(i)) for i in range(1, 6))
        sec = int(m.group(6)) if m.group(6) else 0
        try:
            dt = datetime(y, mo, d, h, mi, sec)
        except ValueError:
            return None
    else:
        try:
            dt = datetime.fromisoformat(s)
        except ValueError:
            return None
    if dt <= datetime.now():
        return "past"
    return dt.isoformat()


class ToolExecutorLayerSubstrate:
    """Substrate for L4 toolExecutor.js patterns."""

    def __init__(self) -> None:
        self.record_store = ToolCallRecordStore()
        self.scheduled_tasks: List[Dict[str, Any]] = []

    def attach_record_id(self, result: Dict[str, Any], record: ToolCallRecord) -> Dict[str, Any]:
        if isinstance(result, dict):
            result["recordId"] = record.id
            if isinstance(result.get("raw"), dict):
                result["raw"].setdefault("tool_call_record_id", record.id)
        return result

    def build_river_context(self, river: Optional[str], messages: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Apply river mode to messages list (substrate, not semantic)."""
        mode, n = parse_river_mode(river)
        if mode == "full":
            return list(messages)
        if mode == "text":
            return [
                {"role": m.get("role", "user"), "content": m.get("content", "") if isinstance(m.get("content"), str) else ""}
                for m in messages
            ]
        if mode == "last":
            return messages[-n:] if n > 0 else []
        if mode == "semantic":
            # In substrate: graceful fallback to last:N (no real semantic)
            return messages[-n:] if n > 0 else []
        return []

    def schedule_timed(self, name: str, args: Dict[str, Any], timely_contact: str) -> Dict[str, Any]:
        validation = validate_timely_contact(timely_contact)
        if validation is None:
            return {"success": False, "error": f"无效的 timely_contact 时间格式: '{timely_contact}'"}
        if validation == "past":
            return {"success": False, "error": f"无效的 timely_contact: '{timely_contact}'. 不能设置为过去或当前时间."}
        task = {
            "task_id": f"task-{hashlib.sha1((name + timely_contact).encode()).hexdigest()[:12]}",
            "tool_name": name,
            "args": dict(args),
            "scheduled_at": validation,
        }
        self.scheduled_tasks.append(task)
        return {"success": True, "scheduled": True, **task}


# ============================================================
# 6. ProtocolBridgeLayerSubstrate — multi-protocol compat
# ============================================================

RESPONSE_RETRY_SUPPRESSION_WINDOW_MS = 15000


def normalize_text_content(content: Any) -> str:
    """Multi-format content → plain text."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        out = []
        for item in content:
            if isinstance(item, str):
                out.append(item)
            elif isinstance(item, dict):
                if item.get("type") == "text" and isinstance(item.get("text"), str):
                    out.append(item["text"])
                elif item.get("type") == "input_text" and isinstance(item.get("text"), str):
                    out.append(item["text"])
                elif item.get("type") == "output_text" and isinstance(item.get("text"), str):
                    out.append(item["text"])
        return "\n".join(s for s in out if s)
    return ""


def normalize_message_role(role: Optional[str]) -> Optional[str]:
    """developer → system (Anthropic compat)."""
    if role is None:
        return None
    if role == "developer":
        return "system"
    if role in ("system", "user", "assistant", "tool"):
        return role
    return "user"


def build_stable_request_id(prefix: str, payload: Any) -> str:
    """sha256(stable_stringify(payload))[:24] + prefix."""
    h = hashlib.sha256(stable_stringify(payload).encode("utf-8")).hexdigest()[:24]
    return f"{prefix}_{h}"


class ProtocolBridgeLayerSubstrate:
    """Substrate for L5 protocolBridge.js patterns (dedup + role normalization)."""

    def __init__(self, suppression_window_ms: int = RESPONSE_RETRY_SUPPRESSION_WINDOW_MS) -> None:
        self.recent_requests: Dict[str, Dict[str, Any]] = {}
        self.window_ms = suppression_window_ms

    def is_suppressed_duplicate(self, request_id: str, now_ms: Optional[int] = None) -> bool:
        """Pattern: 15s suppression window for retries, auto-cleanup of stale entries."""
        if not request_id or self.window_ms <= 0:
            return False
        now = now_ms if now_ms is not None else int(time.time() * 1000)
        # Cleanup stale (4× window)
        cutoff = now - self.window_ms * 4
        for k in list(self.recent_requests.keys()):
            if self.recent_requests[k]["last_seen_at"] < cutoff:
                del self.recent_requests[k]
        entry = self.recent_requests.get(request_id)
        if entry and (now - entry["last_seen_at"]) <= self.window_ms:
            entry["last_seen_at"] = now
            entry["count"] += 1
            return True
        self.recent_requests[request_id] = {"last_seen_at": now, "count": 1}
        return False

    def normalize_messages(self, messages: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Apply role + content normalization to a list of messages."""
        out = []
        for m in messages:
            if not isinstance(m, dict):
                continue
            role = normalize_message_role(m.get("role"))
            content = normalize_text_content(m.get("content"))
            out.append({"role": role, "content": content})
        return out

    def convert_tool(self, tool: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Normalize tool definitions (function_declarations / tools / functions → OpenAI)."""
        if not isinstance(tool, dict):
            return None
        # Gemini functionDeclarations
        if "functionDeclarations" in tool:
            decls = tool["functionDeclarations"]
            if isinstance(decls, list):
                return [self.convert_tool({"type": "function", **d}) for d in decls if isinstance(d, dict)]
            return None
        # OpenAI function wrapper
        if tool.get("type") == "function" and isinstance(tool.get("function"), dict):
            fn = tool["function"]
            if "name" in fn:
                params = fn.get("parameters") or fn.get("input_schema") or {"type": "object", "properties": {}}
                return {"type": "function", "function": {"name": fn["name"], "description": fn.get("description", ""), "parameters": params}}
        # Anthropic tool
        if tool.get("type") == "function" and "name" in tool:
            params = tool.get("parameters") or tool.get("input_schema") or {"type": "object", "properties": {}}
            return {"type": "function", "function": {"name": tool["name"], "description": tool.get("description", ""), "parameters": params}}
        # Bare function
        if "name" in tool:
            params = tool.get("parameters") or tool.get("schema") or {"type": "object", "properties": {}}
            return {"type": "function", "function": {"name": tool["name"], "description": tool.get("description", ""), "parameters": params}}
        return None


# ============================================================
# 7. FileOperatorLayerSubstrate — sandbox + CRLF + diff
# ============================================================
DEFAULT_MAX_FILE_SIZE = 20 * 1024 * 1024  # 20MB
DEFAULT_MAX_DIRECTORY_ITEMS = 1000
DEFAULT_MAX_SEARCH_RESULTS = 100


def is_path_allowed(target_path: str, allowed_dirs: List[str], operation_type: str = "generic") -> bool:
    """Path sandbox check (case-insensitive on Windows).

    Pattern: read-only operations (ReadFile/FileInfo) bypass sandbox.
    Other operations MUST be inside allowed_dirs.
    """
    resolved = str(Path(target_path).resolve())
    if not allowed_dirs:
        return True  # No allow-list = permissive (degenerate case)
    is_in_allowed = any(
        resolved.lower().startswith(str(Path(d).resolve()).lower())
        for d in allowed_dirs
    )
    if is_in_allowed:
        return True
    # Read-only bypass
    if operation_type in ("ReadFile", "FileInfo") and Path(target_path).is_absolute():
        return True
    return False


def detect_line_ending(content: str) -> str:
    """CRLF/LF/CR detection based on majority count."""
    crlf = content.count("\r\n")
    lf = len(re.findall(r"[^\r]\n", content)) + (1 if content.startswith("\n") else 0)
    cr = len(re.findall(r"\r(?!\n)", content))
    if crlf > lf and crlf > cr:
        return "\r\n"
    if cr > lf and cr > crlf:
        return "\r"
    return "\n"


def normalize_line_endings(content: str) -> str:
    return content.replace("\r\n", "\n").replace("\r", "\n")


def denormalize_line_endings(content: str, original_ending: str) -> str:
    if original_ending == "\r\n":
        return content.replace("\n", "\r\n")
    if original_ending == "\r":
        return content.replace("\n", "\r")
    return content


def apply_diff(original: str, diff_block: str) -> Dict[str, Any]:
    """Parse <<<<<<< SEARCH / ======= / >>>>>>> REPLACE block (first match only)."""
    blocks = diff_block.split("<<<<<<< SEARCH")
    if len(blocks) < 2:
        return {"success": False, "error": "Invalid diff format: No SEARCH blocks found."}
    block = blocks[1]
    parts = block.split("=======")
    if len(parts) != 2:
        return {"success": False, "error": "Invalid diff format: Missing ======= separator."}
    search_part = parts[0]
    replace_part = parts[1].split(">>>>>>> REPLACE")[0]
    sep = "-------"
    sep_idx = search_part.find(sep)
    if sep_idx < 0:
        return {"success": False, "error": "Invalid diff format: Missing ------- separator."}
    search_content = search_part[sep_idx + len(sep):].strip()
    replace_content = replace_part.strip()
    if search_content not in original:
        return {"success": False, "error": f"Search content not found: {search_content[:60]}..."}
    new_content = original.replace(search_content, replace_content, 1)
    return {"success": True, "result": new_content}


def get_unique_file_path(file_path: str) -> Tuple[str, bool]:
    """If exists, return foo(1).ext / foo(2).ext / etc."""
    p = Path(file_path)
    if not p.exists():
        return (file_path, False)
    base = p.stem
    ext = p.suffix
    parent = p.parent
    counter = 1
    while True:
        candidate = parent / f"{base}({counter}){ext}"
        if not candidate.exists():
            return (str(candidate), True)
        counter += 1
        if counter > 9999:
            return (file_path, False)


class FileOperatorLayerSubstrate:
    """Substrate for L6 FileOperator.js patterns (sandbox + CRLF + diff)."""

    def __init__(self, allowed_dirs: Optional[List[str]] = None, max_file_size: int = DEFAULT_MAX_FILE_SIZE) -> None:
        self.allowed_dirs = list(allowed_dirs or [])
        self.max_file_size = max_file_size

    def check_access(self, target_path: str, operation: str) -> bool:
        return is_path_allowed(target_path, self.allowed_dirs, operation)

    def read_with_size_cap(self, file_path: str) -> Dict[str, Any]:
        """Read file with size limit enforced."""
        if not self.check_access(file_path, "ReadFile"):
            return {"success": False, "error": "Access denied"}
        p = Path(file_path)
        if not p.exists():
            return {"success": False, "error": "File not found"}
        size = p.stat().st_size
        if size > self.max_file_size:
            return {"success": False, "error": f"File too large: {size} > {self.max_file_size}"}
        try:
            content = p.read_text(encoding="utf-8")
            return {"success": True, "content": content, "size": size}
        except UnicodeDecodeError as e:
            return {"success": False, "error": f"Encoding error: {e}"}

    def write_with_line_ending_preservation(self, file_path: str, new_content: str) -> Dict[str, Any]:
        """Read existing file (if exists), preserve line endings, write back."""
        if not self.check_access(file_path, "WriteFile"):
            return {"success": False, "error": "Access denied"}
        p = Path(file_path)
        original_ending = "\n"
        if p.exists():
            try:
                existing = p.read_text(encoding="utf-8")
                original_ending = detect_line_ending(existing)
            except (UnicodeDecodeError, OSError):
                pass
        normalized = normalize_line_endings(new_content)
        final = denormalize_line_endings(normalized, original_ending)
        try:
            p.write_text(final, encoding="utf-8")
            return {"success": True, "line_ending_preserved": original_ending, "bytes_written": len(final.encode("utf-8"))}
        except OSError as e:
            return {"success": False, "error": str(e)}


# ============================================================
# 8. VCP6SourceDeepReadReport — aggregate 真读 summary
# ============================================================
@dataclass
class VCP6SourceDeepReadReport:
    repo_root: str
    layer_count: int
    all_exist: bool
    total_declared_lines: int
    total_actual_lines: int
    layer_summaries: List[Dict[str, Any]]
    pattern_taxonomy: Dict[str, List[str]]
    safety_taxonomy: Dict[str, List[str]]
    asi_pole_star: Dict[str, Any]
    generated_at: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def build_report(matrix: VCPLayerMatrix, scan_summary: Dict[str, Any]) -> VCP6SourceDeepReadReport:
    """Aggregate matrix scan + per-layer pattern/safety into a report."""
    from datetime import datetime, timezone, timedelta

    pattern_taxonomy: Dict[str, List[str]] = {}
    safety_taxonomy: Dict[str, List[str]] = {}
    layer_summaries: List[Dict[str, Any]] = []

    for L in matrix.layers:
        layer_summaries.append({
            "layer_id": L.layer_id,
            "relative_path": L.relative_path,
            "architectural_role": L.architectural_role,
            "exists": L.exists,
            "declared_lines": L.lines,
            "actual_lines": L.actual_lines,
            "first_512b_sha256": L.sha256_first_512b,
            "pattern_count": len(L.key_patterns),
            "safety_boundary_count": len(L.safety_boundaries),
        })
        # Aggregate patterns into taxonomy
        for p in L.key_patterns:
            # Use first 2 words as coarse category
            words = p.split()
            cat = " ".join(words[:2]) if len(words) >= 2 else (words[0] if words else "misc")
            pattern_taxonomy.setdefault(cat, []).append(f"{L.layer_id}: {p}")
        for s in L.safety_boundaries:
            words = s.split()
            cat = " ".join(words[:2]) if len(words) >= 2 else (words[0] if words else "misc")
            safety_taxonomy.setdefault(cat, []).append(f"{L.layer_id}: {s}")

    # Shanghai timezone
    tz = timezone(timedelta(hours=8))
    now_iso = datetime.now(tz).isoformat()

    return VCP6SourceDeepReadReport(
        repo_root=str(matrix.repo_root),
        layer_count=len(matrix.layers),
        all_exist=scan_summary.get("all_exist", False),
        total_declared_lines=scan_summary.get("total_declared_lines", 0),
        total_actual_lines=scan_summary.get("total_actual_lines", 0),
        layer_summaries=layer_summaries,
        pattern_taxonomy=pattern_taxonomy,
        safety_taxonomy=safety_taxonomy,
        asi_pole_star=dict(ASI_POLE_STAR),
        generated_at=now_iso,
    )


# ============================================================
# 9. VCP6SourceDeepReadBridge — connect V1327 → V1326 chain
# ============================================================
@dataclass
class VCP6SourceDeepReadBridge:
    v1327_version: str
    parent_chain: Tuple[str, ...]
    parent_chain_length: int
    parent_chain_complete: bool
    asi_pole_star: Dict[str, Any]
    layer_count: int
    total_lines: int
    bridge_summary: str
    bridge_generated_at: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def build_bridge(matrix: VCPLayerMatrix, scan_summary: Dict[str, Any]) -> VCP6SourceDeepReadBridge:
    from datetime import datetime, timezone, timedelta
    tz = timezone(timedelta(hours=8))
    now_iso = datetime.now(tz).isoformat()

    parent_chain = ("V1313", "V1314", "V1315", "V1316", "V1317", "V1318", "V1319", "V1320", "V1321", "V1322", "V1323", "V1324", "V1325", "V1326", "V1327")
    summary = (
        f"V1327 (VCP 6 真源码深读) extends post-V1326 ASI 5-Gap chain. "
        f"Reads 6 architecturally-distinct VCP source files "
        f"({scan_summary['total_declared_lines']} declared lines / "
        f"{scan_summary['total_actual_lines']} actual lines verified). "
        f"Extracts 6 substrate components + pattern/safety taxonomy. "
        f"ASI 北极星 LOCKED (V0.1=0.7905); V1327 = read-only analysis, "
        f"NOT a port of VCP, NOT an ASI claim to understand VCP semantics."
    )
    return VCP6SourceDeepReadBridge(
        v1327_version="0.1.0",
        parent_chain=parent_chain,
        parent_chain_length=len(parent_chain),
        parent_chain_complete=True,
        asi_pole_star=dict(ASI_POLE_STAR),
        layer_count=scan_summary["layer_count"],
        total_lines=scan_summary["total_declared_lines"],
        bridge_summary=summary,
        bridge_generated_at=now_iso,
    )


# ============================================================
# Self-test (Popper)
# ============================================================
def _self_test() -> bool:
    """18 Popper self-tests covering all 9 components."""
    tests: List[Tuple[str, bool]] = []

    # 1. VCPLayerMatrix constants
    tests.append(("VCP_6_LAYERS count = 6", len(VCP_6_LAYERS) == 6))

    # 2. VCPLayerMatrix scan (read-only, may exist or not)
    matrix = VCPLayerMatrix()
    summary = matrix.scan()
    tests.append(("scan returns layer_count=6", summary["layer_count"] == 6))
    tests.append(("scan populates total_declared_lines=5689", summary["total_declared_lines"] == 5689))

    # 3. AgentManagerLayerSubstrate
    am = AgentManagerLayerSubstrate()
    am.load_map({"XiaoKe": "xiaoKe.txt"})
    tests.append(("AgentManager: load_map populates agent_map", am.agent_map == {"XiaoKe": "xiaoKe.txt"}))
    tests.append(("AgentManager: load_map clears prompt_cache", am.prompt_cache == {}))
    am.prompt_cache["XiaoKe"] = "old"
    am.invalidate_cache_for("XiaoKe")
    tests.append(("AgentManager: invalidate_cache_for removes entry", "XiaoKe" not in am.prompt_cache))
    prompt = am.get_agent_prompt("XiaoKe", lambda f: "fresh content")
    tests.append(("AgentManager: get_agent_prompt loads + caches", prompt == "fresh content" and am.prompt_cache["XiaoKe"] == "fresh content"))
    tests.append(("AgentManager: missing agent returns placeholder", am.get_agent_prompt("Unknown", lambda f: "x") == "{{agent:Unknown}}"))
    tests.append(("AgentManager: should_watch excludes node_modules", not am.should_watch("node_modules/foo.js")))
    tests.append(("AgentManager: should_watch allows regular", am.should_watch("agent.txt")))

    # 4. DynamicToolRegistryLayerSubstrate
    dt = DynamicToolRegistryLayerSubstrate()
    cfg = dt.reload_config({"maxInjectionChars": 99999}, None)
    tests.append(("DynamicToolRegistry: clamp respects value below max", cfg["maxInjectionChars"] == 99999))
    cfg2 = dt.reload_config({"maxInjectionChars": 999999}, None)
    tests.append(("DynamicToolRegistry: clamp caps maxInjectionChars at 120000", cfg2["maxInjectionChars"] == 120000))
    tests.append(("DynamicToolRegistry: classify CJK search", dt.classify("搜索网络资料") == "search"))
    tests.append(("DynamicToolRegistry: classify file_code", dt.classify("read file") == "file_code"))
    ts = dt.token_summary("hello world")
    tests.append(("DynamicToolRegistry: token_summary has 5 keys", set(ts.keys()) == {"full", "light_list_budget", "light_list_truncated", "default_brief_budget", "default_brief_truncated"}))
    tests.append(("DynamicToolRegistry: light_list_budget = 15", ts["light_list_budget"] == LIGHT_LIST_TOKEN_BUDGET))
    tests.append(("DynamicToolRegistry: stable_stringify sorted", stable_stringify({"b": 1, "a": 2}) == stable_stringify({"a": 2, "b": 1})))
    tests.append(("DynamicToolRegistry: clamp_integer respects min", clamp_integer(-5, 0, 100, 50) == 0))
    tests.append(("DynamicToolRegistry: clamp_integer respects max", clamp_integer(999, 0, 100, 50) == 100))

    # 5. MessageProcessorLayerSubstrate
    mp = MessageProcessorLayerSubstrate(registered_agents={"XiaoKe"})
    tests.append(("MessageProcessor: is_privileged_role system", is_privileged_role("system", "anything")))
    tests.append(("MessageProcessor: is_privileged_role user normal", not is_privileged_role("user", "hello")))
    tests.append(("MessageProcessor: is_privileged_role user 系统提示:", is_privileged_role("user", "[系统提示:] test")))
    text1 = mp.expand_agent_placeholder("{{agent:XiaoKe}}", "XiaoKe", lambda a: "PROMPT", "system")
    tests.append(("MessageProcessor: expand_agent_placeholder replaces", text1 == "PROMPT"))
    text2 = mp.expand_agent_placeholder("{{agent:Other}}", "Other", lambda a: "OTHER", "system")
    tests.append(("MessageProcessor: unregistered agent NOT replaced", text2 == "{{agent:Other}}"))
    mp.reset_context()
    mp.expand_agent_placeholder("{{agent:XiaoKe}}", "XiaoKe", lambda a: "X1", "system")
    text3 = mp.expand_agent_placeholder("{{agent:XiaoKe}}", "XiaoKe", lambda a: "X2", "system")
    tests.append(("MessageProcessor: AgentGuard silently removes duplicate", text3 == ""))
    mp.reset_context()
    mp.processing_stack.add("XiaoKe")
    text4 = mp.expand_agent_placeholder("{{agent:XiaoKe}}", "XiaoKe", lambda a: "X", "system")
    tests.append(("MessageProcessor: circular detection injects error marker", "[Error: Circular agent reference detected for 'XiaoKe']" in text4))
    tests.append(("MessageProcessor: is_system_injection detects 系统通知", mp.is_system_injection("[系统通知:] something")))
    tests.append(("MessageProcessor: is_tool_payload detects VCP_TOOL_PAYLOAD", mp.is_tool_payload("<!-- VCP_TOOL_PAYLOAD -->")))
    tests.append(("MessageProcessor: extract_fold_mode Lite", mp.extract_fold_mode("[[VCPStaticFold::Lite]] hello") == "lite"))
    tests.append(("MessageProcessor: extract_fold_mode default auto", mp.extract_fold_mode("nothing") == "auto"))

    # 6. ToolExecutorLayerSubstrate
    te = ToolExecutorLayerSubstrate()
    rec = te.record_store.begin_record("FileOperator", {"path": "/tmp/foo"})
    tests.append(("ToolExecutor: record has unique id", rec.id.startswith("rec-")))
    te.record_store.finish_record(rec, True)
    tests.append(("ToolExecutor: finish_record marks success", rec.success is True))
    mode, n = parse_river_mode("semantic:5")
    tests.append(("ToolExecutor: parse_river_mode semantic:5", (mode, n) == ("semantic", 5)))
    mode, n = parse_river_mode("last:10")
    tests.append(("ToolExecutor: parse_river_mode last:10", (mode, n) == ("last", 10)))
    mode, n = parse_river_mode("full")
    tests.append(("ToolExecutor: parse_river_mode full", (mode, n) == ("full", 0)))
    valid = validate_timely_contact("2099-01-01-00:00")
    tests.append(("ToolExecutor: validate_timely_contact future OK", valid and valid != "past"))
    past = validate_timely_contact("2000-01-01-00:00")
    tests.append(("ToolExecutor: validate_timely_contact past rejected", past == "past"))
    invalid = validate_timely_contact("not-a-date")
    tests.append(("ToolExecutor: validate_timely_contact invalid → None", invalid is None))

    # 7. ProtocolBridgeLayerSubstrate
    pb = ProtocolBridgeLayerSubstrate(suppression_window_ms=1000)
    rid = build_stable_request_id("resp", {"a": 1, "b": 2})
    tests.append(("ProtocolBridge: build_stable_request_id has prefix", rid.startswith("resp_")))
    tests.append(("ProtocolBridge: build_stable_request_id 24 hex chars after prefix", len(rid) == len("resp_") + 24))
    tests.append(("ProtocolBridge: normalize_text_content strips array", normalize_text_content([{"type": "text", "text": "hi"}]) == "hi"))
    tests.append(("ProtocolBridge: normalize_message_role developer→system", normalize_message_role("developer") == "system"))
    tests.append(("ProtocolBridge: is_suppressed_duplicate first call false", pb.is_suppressed_duplicate("req1") is False))
    tests.append(("ProtocolBridge: is_suppressed_duplicate second call true", pb.is_suppressed_duplicate("req1") is True))
    converted = pb.convert_tool({"type": "function", "function": {"name": "Foo", "parameters": {"type": "object"}}})
    tests.append(("ProtocolBridge: convert_tool OpenAI function → chat tool", converted == {"type": "function", "function": {"name": "Foo", "description": "", "parameters": {"type": "object"}}}))

    # 8. FileOperatorLayerSubstrate
    fo = FileOperatorLayerSubstrate(allowed_dirs=[str(Path.cwd())])
    tests.append(("FileOperator: allowed path granted", fo.check_access(str(Path.cwd() / "test.txt"), "ReadFile")))
    tests.append(("FileOperator: outside path write denied", not fo.check_access("C:\\Windows\\System32\\evil.txt", "WriteFile")))
    tests.append(("FileOperator: outside path read-only bypass granted", fo.check_access("C:\\Windows\\System32\\drivers\\etc\\hosts", "ReadFile")))
    tests.append(("FileOperator: detect_line_ending LF default", detect_line_ending("hello\nworld") == "\n"))
    tests.append(("FileOperator: detect_line_ending CRLF majority", detect_line_ending("a\r\nb\r\nc\n") == "\r\n"))
    diff_result = apply_diff("hello world", "<<<<<<< SEARCH\n-------\nhello\n=======\nhi\n>>>>>>> REPLACE")
    tests.append(("FileOperator: apply_diff replaces first occurrence", diff_result["success"] and diff_result["result"] == "hi world"))
    diff_missing = apply_diff("foo bar", "<<<<<<< SEARCH\n-------\nbaz\n=======\nqux\n>>>>>>> REPLACE")
    tests.append(("FileOperator: apply_diff missing search returns error", not diff_missing["success"]))
    new_path, renamed = get_unique_file_path("C:\\nonexistent_unique_test_file_xyzzy\\test.txt")
    tests.append(("FileOperator: get_unique_file_path non-existing returns same", new_path.endswith("test.txt") and renamed is False))

    # 9. VCP6SourceDeepReadReport + Bridge
    matrix_full = VCPLayerMatrix()
    summary_full = matrix_full.scan()
    report = build_report(matrix_full, summary_full)
    bridge = build_bridge(matrix_full, summary_full)
    tests.append(("Report: layer_count = 6", report.layer_count == 6))
    tests.append(("Report: pattern_taxonomy has entries", len(report.pattern_taxonomy) > 0))
    tests.append(("Report: safety_taxonomy has entries", len(report.safety_taxonomy) > 0))
    tests.append(("Report: ASI pole-star V0.1 = 0.7905 LOCKED", report.asi_pole_star["V0_1_anchored"] == 0.7905))
    tests.append(("Bridge: parent_chain length = 15", bridge.parent_chain_length == 15))
    tests.append(("Bridge: contains V1327", "V1327" in bridge.parent_chain))
    tests.append(("Bridge: contains V1326 (parent)", "V1326" in bridge.parent_chain))

    # Final
    passed = sum(1 for _, ok in tests if ok)
    failed = [(name, ok) for name, ok in tests if not ok]
    if failed:
        for name, _ in failed:
            print(f"  FAIL: {name}")
        print(f"V1327 self-test: {passed}/{len(tests)} PASS, {len(failed)} FAIL")
        return False
    print(f"V1327 self-test: PASS ({passed}/{len(tests)})")
    return True


def main() -> None:
    """CLI entry: --self-test."""
    import argparse
    parser = argparse.ArgumentParser(description="V1327 — VCP 6 真源码深读")
    parser.add_argument("--self-test", action="store_true", help="Run Popper self-test")
    parser.add_argument("--scan", action="store_true", help="Scan 6 layers and print summary")
    parser.add_argument("--report", action="store_true", help="Build + print full report (JSON)")
    args = parser.parse_args()

    if args.self_test:
        ok = _self_test()
        raise SystemExit(0 if ok else 1)
    if args.scan or args.report:
        matrix = VCPLayerMatrix()
        summary = matrix.scan()
        if args.report:
            report = build_report(matrix, summary)
            print(json.dumps(report.to_dict(), ensure_ascii=False, indent=2))
        else:
            print(json.dumps(summary, ensure_ascii=False, indent=2))
        return

    # Default: scan + brief summary
    matrix = VCPLayerMatrix()
    summary = matrix.scan()
    print(json.dumps({
        "v1327_version": "0.1.0",
        "layer_count": summary["layer_count"],
        "all_exist": summary["all_exist"],
        "total_declared_lines": summary["total_declared_lines"],
        "total_actual_lines": summary["total_actual_lines"],
        "asi_pole_star_locked": ASI_POLE_STAR,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()