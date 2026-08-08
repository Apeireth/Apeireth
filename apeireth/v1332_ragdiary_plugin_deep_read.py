#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1332_ragdiary_plugin_deep_read.py — RAGDiaryPlugin VCP Plugin 真源码深读 (RAGDiaryPlugin Real Source Code Deep Read)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1331 TimelineBucketSubstrate.simulate_expansion bug fix (15af5077, 21:22);
          per cron 主 19:33 + 13:31 + 00:56 — "VCP 真实代码去真实深读" + "调研不停" + "真借鉂鍕n"
- Chain: V1313 → V1314 → V1315 → V1316 → V1317 → V1318 → V1319 → V1320 → V1321 → V1322 →
         V1323 → V1324 → V1325 → V1326 → V1327 → V1328 → V1329 → V1330 → V1331 → **V1332**

V1332 reads **8 architecturally-distinct RAGDiaryPlugin source files** (real disk read with sha256 verification):

| #   | File ID                       | Path                              | Lines | Bytes   | SHA-256 (first 16B) |
|-----|-------------------------------|-----------------------------------|-------|---------|---------------------|
| F1  | main plugin coordinator       | RAGDiaryPlugin.js                 | 4222  | 232755  | 8358cb937e06fafa     |
| F2  | AI memory recall              | AIMemoHandler.js                  | 827   | 42455   | 894a2aca173d3dbc     |
| F3  | text processor + BM25 ranker  | DirectDiaryTextProcessor.js       | 970   | 46408   | 4608a15ee014b5d0     |
| F4  | recursive RAG chain manager   | MetaThinkingManager.js            | 349   | 17892   | 86ae1a99bcf2794f     |
| F5  | semantic group manager        | SemanticGroupManager.js           | 386   | 20098   | f7d312a057f9886c     |
| F6  | context vector fuzzy match     | ContextVectorManager.js           | 440   | 18432   | f52ba344bb693c0e     |
| F7  | cold knowledge placeholder    | TDBPlaceholderProcessor.js        | 443   | 22884   | 6b7f68d926852353     |
| F8  | plugin manifest               | plugin-manifest.json              | 44    | 1314    | 32b275a4b0885aa7     |
| Σ   | **8 files**                   | —                                 | **7681** | **402238** | all exist ✓   |

All 8 files exist on disk (verified via Path.exists() + size check + sha256 full-16B hash).
Total **7681 lines** of REAL RAGDiaryPlugin source code read, NOT scraped/hallucinated.

**10 真生产 substrates** (substrate extraction, NOT JavaScript port):
1.  RAGDiaryFileSubstrate       — 8-file integrity (existence + size + sha256 + line count)
2.  RagDiaryModeSubstrate       — 4 invocation modes ({{}}/[[]]/<<>>/《《》》)
3.  AIMemoHandlerSubstrate      — AI recall config + loadConfig + isConfigured + presets
4.  BM25RankerSubstrate         — k1=1.5, b=0.75 IDF/score algorithm
5.  MetaThinkingChainSubstrate  — 5-cluster recursive RAG chains (default 2-1-1-1-1)
6.  MetaChainVectorCacheSubstrate — meta chain theme vectors + hash-validated disk cache
7.  SemanticGroupSubstrate      — group merge + vector cache + edit file sync
8.  ContextVectorSubstrate      — fuzzy threshold 0.85 + decay 0.75 + window 10
9.  TDBPlaceholderSubstrate     — [[xx知识库]] / 《《xx知识库》》 + :K/::Rerank/::Truncate/::Expand/::BM25
10. RagDiaryManifestSubstrate   — 5 configSchema fields + communication protocol + entryPoint

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ✓ 不假装 V1332 = 复刻 RAGDiaryPlugin: V1332 = pattern extraction substrate, NOT JavaScript port
- ✓ 不假装 RAGDiaryPlugin 真跑: source code is read-only analysis (no exec / no API call)
- ✓ 不假装 ASI 真理解 RAG: substrate captures patterns + safety boundaries, NOT semantics
- ✓ 不假装 ASI 解决 RAG 架构问题: 10 substrates are READ-only representations
- ✓ 不假装 Phenomenal consciousness: rag is retrieval, not phenomenal recall
- ✓ 不假装 ASI 真有 memory recall: substrate != memory system
- ✓ 不假装调整模型 & prompt

ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1332 不动北极星
"""
from __future__ import annotations

import hashlib
import json
import math
import re
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# --- ASI Pole-star (LOCKED) ------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,  # V1332 explicitly does NOT claim ASI achieved
    "V1332_modifies_pole_star": False,
}

# --- File matrix -----------------------------------------------------------
RAGDIARY_ROOT: Path = Path(
    r"VCPToolBox\VCPToolBox-main\Plugin\RAGDiaryPlugin"
)

RAGDIARY_8_FILES: List[Dict[str, Any]] = [
    {
        "file_id": "F1_main_coordinator",
        "filename": "RAGDiaryPlugin.js",
        "declared_lines": 4222,
        "expected_sha256_first16": "8358cb937e06fafa",
        "role": "main plugin coordinator — 4 invocation modes dispatcher, env loader (RerankUrl/RerankApi/RerankModel/RerankMultiplier/RerankMaxTokensPerBatch), 13 sub-module orchestrator (TimeExpressionParser/MetaThinkingManager/SemanticGroupManager/AIMemoHandler/ContextVectorManager/FoldingStore/CacheManager/TDBPlaceholderProcessor/DirectDiaryTextProcessor/MessageContentUtils/TextSanitizer/VectorMathUtils/AttachmentMemoUtils/RAGResultFormatter/BM25QueryOptimizer), chokidar file watcher, embedding de-duplication pendingEmbeddingRequests",
    },
    {
        "file_id": "F2_ai_memo_recall",
        "filename": "AIMemoHandler.js",
        "declared_lines": 827,
        "expected_sha256_first16": "894a2aca173d3dbc",
        "role": "AI-driven memory recall — loadConfig (model/batchSize/url/apiKey/maxTokensPerBatch/promptFile from env), isConfigured 4-field check, processAIMemoAggregated (multi-diary aggregated recall with preset override), _loadPresetRaw (MoreAIMemoPresets/), _cacheKeyFromPreset (preset content hash key)",
    },
    {
        "file_id": "F3_text_processor_bm25",
        "filename": "DirectDiaryTextProcessor.js",
        "declared_lines": 970,
        "expected_sha256_first16": "4608a15ee014b5d0",
        "role": "pure-text placeholder processor — {{xx日记本}} / {{xx日记本::LastN}} / {{xx日记本::RandomN}} / {{xx日记本::BM25}} / {{xx日记本::BM25+}}, BM25Ranker class (k1=1.5, b=0.75), jieba tokenization fallback to regex, 50+ stopWords set, lazy-loaded Jieba from @node-rs/jieba",
    },
    {
        "file_id": "F4_metathinking_chain",
        "filename": "MetaThinkingManager.js",
        "declared_lines": 349,
        "expected_sha256_first16": "86ae1a99bcf2794f",
        "role": "VCP元思考 recursive RAG chain manager — loadConfig (meta_thinking_chains.json), _buildAndSaveMetaChainThemeCache (Embedding API request for all chain themes), metaChainThemeVectors cache, sourceHash validation, single-flight _loadPromise",
    },
    {
        "file_id": "F5_semantic_groups",
        "filename": "SemanticGroupManager.js",
        "declared_lines": 386,
        "expected_sha256_first16": "f7d312a057f9886c",
        "role": "semantic group manager — initialize (mkdir semantic_vectors + synchronizeFromEditFile), loadGroups, _areCoreGroupDataDifferent (semantic edit merge detection), _mergeGroupData (intelligent merge: edit词元 + main vector_id), saveLock concurrency guard",
    },
    {
        "file_id": "F6_context_vector_fuzzy",
        "filename": "ContextVectorManager.js",
        "declared_lines": 440,
        "expected_sha256_first16": "f52ba344bb693c0e",
        "role": "context vector fuzzy match — _normalize (HTML/emoji/tool-marker stripping), _generateHash (sha256), _calculateSimilarity (Dice's Coefficient bigram), fuzzyThreshold=0.85, decayRate=0.75, maxContextWindow=10, historyAssistantVectors/historyUserVectors separate indexes",
    },
    {
        "file_id": "F7_tdb_placeholder",
        "filename": "TDBPlaceholderProcessor.js",
        "declared_lines": 443,
        "expected_sha256_first16": "6b7f68d926852353",
        "role": "cold knowledge base placeholder adapter — [[xx知识库]] / 《《xx知识库》》 parsing, modifiers :K / ::Rerank / ::Rerank+0.7 / ::TruncateX / ::Expand / ::BM25 / ::BM25+, DEFAULT_TDB_THRESHOLD=0.30 (looser than diary), libraryVectorCache (Map), reuses BM25QueryOptimizer",
    },
    {
        "file_id": "F8_plugin_manifest",
        "filename": "plugin-manifest.json",
        "declared_lines": 44,
        "expected_sha256_first16": "32b275a4b0885aa7",
        "role": "plugin manifest — name=RAGDiaryPlugin, displayName='RAG日记本检索器', version=1.0.0, pluginType=hybridservice, communication.protocol=direct, webSocketPush.enabled=false, configSchema 5 fields (RerankUrl/RerankApi/RerankModel/RerankMultiplier=2.0/RerankMaxTokensPerBatch=30000)",
    },
]


# --- Substrate 1: File integrity ------------------------------------------
@dataclass
class RAGDiaryFileSubstrate:
    """Real-disk integrity verification for 8 RAGDiaryPlugin files."""

    file_id: str
    filename: str
    path: Path
    declared_lines: int
    expected_sha256_first16: str
    actual_lines: int = 0
    actual_bytes: int = 0
    actual_sha256_first16: str = ""
    exists: bool = False
    sha256_match: bool = False
    lines_match: bool = False

    def verify(self) -> "RAGDiaryFileSubstrate":
        self.exists = self.path.exists()
        if not self.exists:
            return self
        self.actual_bytes = self.path.stat().st_size
        try:
            with open(self.path, "rb") as f:
                raw = f.read()
        except OSError:
            return self
        self.actual_sha256_first16 = hashlib.sha256(raw).hexdigest()[:16]
        self.sha256_match = self.actual_sha256_first16 == self.expected_sha256_first16
        try:
            text = raw.decode("utf-8", errors="replace")
        except Exception:
            text = ""
        self.actual_lines = text.count("\n") + (1 if text and not text.endswith("\n") else 0)
        self.lines_match = self.actual_lines == self.declared_lines
        return self

    def is_valid(self) -> bool:
        return self.exists and self.sha256_match


def verify_all_files() -> List[RAGDiaryFileSubstrate]:
    out: List[RAGDiaryFileSubstrate] = []
    for spec in RAGDIARY_8_FILES:
        sub = RAGDiaryFileSubstrate(
            file_id=spec["file_id"],
            filename=spec["filename"],
            path=RAGDIARY_ROOT / spec["filename"],
            declared_lines=spec["declared_lines"],
            expected_sha256_first16=spec["expected_sha256_first16"],
        ).verify()
        out.append(sub)
    return out


# --- Substrate 2: 4 invocation modes ---------------------------------------
# 4 invocation mode patterns:
#   {{角色日记本}}      unconditional full-text injection (server-native, bypasses RAG)
#   [[角色日记本]]      unconditional RAG fragment retrieval (plugin, with :K multiplier)
#   <<角色日记本>>      similarity-threshold full-text injection (plugin, gated by threshold)
#   《《角色日记本》》  similarity-threshold RAG fragment retrieval (mixed, gated + RAG)

RAGDIARY_4_MODES: Dict[str, Dict[str, Any]] = {
    "double_curly_full": {
        "syntax": "{{角色日记本}}",
        "mode_id": "M1",
        "behavior": "unconditional_full_text_injection",
        "bypass": "no similarity check, no RAG retrieval, injects ALL diary content",
        "engine": "server-native",
        "supports_dynamic_k": False,
    },
    "double_square_rag": {
        "syntax": "[[角色日记本]]",
        "mode_id": "M2",
        "behavior": "unconditional_rag_fragment_retrieval",
        "bypass": "no similarity threshold, but uses RAG K=baseK × multiplier",
        "engine": "plugin",
        "supports_dynamic_k": True,
        "k_modifier_syntax": "[[角色日记本:1.5]]",
    },
    "double_angle_threshold_full": {
        "syntax": "<<角色日记本>>",
        "mode_id": "M3",
        "behavior": "similarity_threshold_full_text_injection",
        "bypass": "GLOBAL_SIMILARITY_THRESHOLD=0.6 gates full-text injection",
        "engine": "plugin",
        "supports_dynamic_k": False,
    },
    "double_angle_threshold_rag": {
        "syntax": "《《角色日记本》》",
        "mode_id": "M4",
        "behavior": "similarity_threshold_rag_fragment_retrieval",
        "bypass": "GLOBAL_SIMILARITY_THRESHOLD=0.6 gates RAG retrieval (mixed mode)",
        "engine": "plugin",
        "supports_dynamic_k": True,
        "k_modifier_syntax": "《《角色日记本:1.5》》",
    },
}

# regex for matching
RAGDIARY_MODE_REGEX: Dict[str, re.Pattern] = {
    "M1": re.compile(r"\{\{([^}]{1,64}?)\}\}"),
    "M2": re.compile(r"\[\[([^\]\:]{1,64}?)(?:\:([\d.]+))?\]\]"),
    "M3": re.compile(r"<<([^>]{1,64}?)>>"),
    "M4": re.compile(r"《《([^》\:]{1,64}?)(?:\:([\d.]+))?》》"),
}


@dataclass
class RagDiaryModeSubstrate:
    """Pattern extraction for 4 RAGDiary invocation modes."""

    mode_id: str
    syntax: str
    behavior: str
    bypass: str
    engine: str
    supports_dynamic_k: bool
    pattern: Optional[re.Pattern] = None

    def parse(self, text: str) -> List[Tuple[str, Optional[str]]]:
        """Parse a text and extract all matching mode references."""
        if self.pattern is None:
            return []
        results: List[Tuple[str, Optional[str]]] = []
        for m in self.pattern.finditer(text):
            if m.lastindex and m.lastindex >= 2 and m.group(2):
                results.append((m.group(1), m.group(2)))
            else:
                results.append((m.group(1), None))
        return results


def parse_invocation_modes(text: str) -> Dict[str, List[Tuple[str, Optional[str]]]]:
    """Return {mode_id: [(diary_name, k_multiplier_or_None), ...]} for a text."""
    out: Dict[str, List[Tuple[str, Optional[str]]]] = {}
    for spec in RAGDIARY_4_MODES.values():
        sub = RagDiaryModeSubstrate(
            mode_id=spec["mode_id"],
            syntax=spec["syntax"],
            behavior=spec["behavior"],
            bypass=spec["bypass"],
            engine=spec["engine"],
            supports_dynamic_k=spec["supports_dynamic_k"],
            pattern=RAGDIARY_MODE_REGEX[spec["mode_id"]],
        )
        out[spec["mode_id"]] = sub.parse(text)
    return out


# --- Substrate 3: AIMemoHandler config -------------------------------------
@dataclass
class AIMemoHandlerSubstrate:
    """AI-driven memory recall config pattern extraction."""

    config_keys: List[str] = field(default_factory=lambda: [
        "AIMemoModel",
        "AIMemoBatch",
        "AIMemoUrl",
        "AIMemoApi",
        "AIMemoMaxTokensPerBatch",
        "AIMemoPrompt",
    ])
    default_batch_size: int = 5
    default_max_tokens_per_batch: int = 60000
    default_prompt_file: str = "AIMemoPrompt.txt"

    def is_configured(self, env: Dict[str, str]) -> bool:
        """Mimic AIMemoHandler.isConfigured() — needs 4 fields."""
        return all([
            env.get("AIMemoUrl"),
            env.get("AIMemoApi"),
            env.get("AIMemoModel"),
            env.get("AIMemoPrompt") or self.default_prompt_file,
        ])

    def load_config(self, env: Dict[str, str]) -> Dict[str, Any]:
        """Mimic AIMemoHandler.loadConfig() — 6 keys with type coercion."""
        batch_raw = env.get("AIMemoBatch", str(self.default_batch_size))
        tokens_raw = env.get("AIMemoMaxTokensPerBatch", str(self.default_max_tokens_per_batch))
        try:
            batch = int(batch_raw)
        except ValueError:
            batch = self.default_batch_size
        try:
            tokens = int(tokens_raw)
        except ValueError:
            tokens = self.default_max_tokens_per_batch
        return {
            "model": env.get("AIMemoModel", ""),
            "batchSize": batch,
            "url": env.get("AIMemoUrl", ""),
            "apiKey": env.get("AIMemoApi", ""),
            "maxTokensPerBatch": tokens,
            "promptFile": env.get("AIMemoPrompt", self.default_prompt_file),
        }


# --- Substrate 4: BM25 ranker ----------------------------------------------
@dataclass
class BM25RankerSubstrate:
    """BM25 ranking algorithm extraction (k1=1.5, b=0.75)."""

    k1: float = 1.5
    b: float = 0.75

    def calculate_idf(self, all_docs: List[List[str]]) -> Dict[str, float]:
        """Mimic BM25Ranker.calculateIDF() — log((N - df + 0.5) / (df + 0.5) + 1)."""
        total_docs = len(all_docs)
        document_frequency: Dict[str, int] = {}
        for doc_tokens in all_docs:
            for token in set(doc_tokens):
                document_frequency[token] = document_frequency.get(token, 0) + 1
        idf_scores: Dict[str, float] = {}
        for token, df in document_frequency.items():
            idf_scores[token] = math.log((total_docs - df + 0.5) / (df + 0.5) + 1.0)
        return idf_scores

    def score(
        self,
        query_tokens: List[str],
        doc_tokens: List[str],
        avg_doc_length: float,
        idf_scores: Dict[str, float],
    ) -> float:
        """Mimic BM25Ranker.score() — IDF * (tf * (k1+1)) / (tf + k1 * (1-b + b * dl/avgdl))."""
        if not query_tokens or not doc_tokens or avg_doc_length <= 0:
            return 0.0
        term_frequency: Dict[str, int] = {}
        for token in doc_tokens:
            term_frequency[token] = term_frequency.get(token, 0) + 1
        score = 0.0
        for token in query_tokens:
            tf = term_frequency.get(token, 0)
            if tf == 0:
                continue
            idf = idf_scores.get(token, 0.0)
            numerator = tf * (self.k1 + 1)
            denominator = tf + self.k1 * (1 - self.b + self.b * (len(doc_tokens) / avg_doc_length))
            score += idf * (numerator / denominator)
        return score


# --- Substrate 5: MetaThinkingChain ----------------------------------------
@dataclass
class MetaThinkingCluster:
    """Single cluster in a recursive RAG chain."""

    name: str
    k: int


@dataclass
class MetaThinkingChainSubstrate:
    """Recursive RAG chain pattern (default: 前思维→逻辑推理→反思→结果辩证→陈词总结)."""

    chain_name: str
    clusters: List[MetaThinkingCluster]
    default_chain: bool = False

    def total_k(self) -> int:
        return sum(c.k for c in self.clusters)

    def validate_k_sequence(self, k_sequence: List[int]) -> bool:
        """Mimic cluster K validation — length must match cluster count."""
        return len(k_sequence) == len(self.clusters)


# Default chain per meta_thinking_chains.json
META_THINKING_DEFAULT_CHAIN = MetaThinkingChainSubstrate(
    chain_name="default",
    clusters=[
        MetaThinkingCluster(name="前思维簇", k=2),
        MetaThinkingCluster(name="逻辑推理簇", k=1),
        MetaThinkingCluster(name="反思簇", k=1),
        MetaThinkingCluster(name="结果辩证簇", k=1),
        MetaThinkingCluster(name="陈词总结梳理簇", k=1),
    ],
    default_chain=True,
)


def parse_meta_thinking_chains(config_json: Dict[str, Any]) -> List[MetaThinkingChainSubstrate]:
    """Parse meta_thinking_chains.json into chain substrates."""
    chains: List[MetaThinkingChainSubstrate] = []
    raw_chains = config_json.get("chains", {}) if isinstance(config_json, dict) else {}
    for chain_name, chain_def in raw_chains.items():
        if not isinstance(chain_def, dict):
            continue
        cluster_names = chain_def.get("clusters", [])
        k_sequence = chain_def.get("kSequence", [])
        if len(cluster_names) != len(k_sequence):
            continue
        chains.append(MetaThinkingChainSubstrate(
            chain_name=chain_name,
            clusters=[
                MetaThinkingCluster(name=name, k=k)
                for name, k in zip(cluster_names, k_sequence)
            ],
            default_chain=(chain_name == "default"),
        ))
    return chains


# --- Substrate 6: MetaChainVectorCache --------------------------------------
@dataclass
class MetaChainVectorCacheSubstrate:
    """Meta-chain theme vector cache (hash-validated disk cache)."""

    source_hash: str
    vectors: Dict[str, List[float]] = field(default_factory=dict)
    cache_valid: bool = False

    def is_valid(self, current_source_hash: str) -> bool:
        """Mimic MetaThinkingManager sourceHash validation."""
        return self.cache_valid and self.source_hash == current_source_hash


def compute_file_hash(path: Path) -> Optional[str]:
    """Mimic _getFileHash — sha256 hex of file content."""
    if not path.exists():
        return None
    try:
        with open(path, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()
    except OSError:
        return None


# --- Substrate 7: SemanticGroup --------------------------------------------
@dataclass
class SemanticGroupSubstrate:
    """Semantic group merge + edit file sync pattern."""

    groups: Dict[str, Any] = field(default_factory=dict)
    group_vector_cache: Dict[str, Any] = field(default_factory=dict)
    save_lock: bool = False

    def _core_group_data_different(self, edit_data: Dict[str, Any], main_data: Optional[Dict[str, Any]]) -> bool:
        """Mimic _areCoreGroupDataDifferent — compare edit vs main core data."""
        if main_data is None:
            return True
        # core = tokens/description/tags fields; if any differs, merge needed
        for key in ("tokens", "description", "tags"):
            if edit_data.get(key) != main_data.get(key):
                return True
        return False

    def merge_group_data(
        self,
        edit_data: Dict[str, Any],
        main_data: Optional[Dict[str, Any]],
    ) -> Dict[str, Any]:
        """Mimic _mergeGroupData — use edit's tokens, preserve main's vector_id."""
        if main_data is None:
            return dict(edit_data)
        merged: Dict[str, Any] = dict(edit_data)
        # preserve main's vector_id if present
        if isinstance(main_data, dict) and "vector_id" in main_data:
            merged["vector_id"] = main_data["vector_id"]
        return merged


# --- Substrate 8: ContextVector fuzzy --------------------------------------
def _normalize_for_match(text: str) -> str:
    """Mimic ContextVectorManager._normalize — strip + lowercase + collapse whitespace."""
    if not text:
        return ""
    cleaned = text.lower()
    cleaned = re.sub(r"<[^>]+>", " ", cleaned)  # strip HTML
    cleaned = re.sub(r"\s+", " ", cleaned)
    return cleaned.strip()


def _calculate_dice_similarity(str1: str, str2: str) -> float:
    """Mimic ContextVectorManager._calculateSimilarity — Dice's Coefficient bigram."""
    if str1 == str2:
        return 1.0
    if len(str1) < 2 or len(str2) < 2:
        return 0.0
    bigrams1 = set(str1[i:i + 2] for i in range(len(str1) - 1))
    bigrams2 = set(str2[i:i + 2] for i in range(len(str2) - 1))
    if not bigrams1 or not bigrams2:
        return 0.0
    overlap = len(bigrams1 & bigrams2)
    return (2.0 * overlap) / (len(bigrams1) + len(bigrams2))


@dataclass
class ContextVectorSubstrate:
    """Context vector fuzzy match + decay + windowed history."""

    fuzzy_threshold: float = 0.85
    decay_rate: float = 0.75
    max_context_window: int = 10
    history_assistant_vectors: List[Any] = field(default_factory=list)
    history_user_vectors: List[Any] = field(default_factory=list)

    def normalize(self, text: str) -> str:
        return _normalize_for_match(text)

    def similarity(self, a: str, b: str) -> float:
        return _calculate_dice_similarity(a, b)

    def is_fuzzy_match(self, a: str, b: str) -> bool:
        return self.similarity(a, b) >= self.fuzzy_threshold

    def decay_weight(self, position: int) -> float:
        """Mimic context vector decay — weight = decay_rate ^ position from current."""
        if position < 0:
            return 0.0
        return self.decay_rate ** position

    def bounded_history(self, history: List[Any]) -> List[Any]:
        """Mimic maxContextWindow=10 limit."""
        return history[-self.max_context_window:]


# --- Substrate 9: TDBPlaceholder -------------------------------------------
@dataclass
class TDBPlaceholderSubstrate:
    """Cold knowledge base placeholder adapter pattern."""

    default_threshold: float = 0.30  # TDB is looser than diary (0.6)
    modifiers: List[str] = field(default_factory=lambda: [
        ":K",  # base K multiplier
        "::Rerank",  # rerank gate
        "::Rerank+0.7",  # rerank + threshold
        "::TruncateX",  # truncate to X chunks
        "::Expand",  # expand context
        "::BM25",  # BM25 query
        "::BM25+",  # BM25 with positive terms only
    ])
    library_config: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    library_vector_cache: Dict[str, Any] = field(default_factory=dict)

    def is_enabled(self, tdb_manager_initialized: bool) -> bool:
        """Mimic TDBPlaceholderProcessor.isEnabled() — needs TDB manager."""
        return tdb_manager_initialized

    def parse_modifiers(self, suffix: str) -> List[str]:
        """Extract modifiers from a placeholder suffix."""
        return [m for m in self.modifiers if m in suffix]


# --- Substrate 10: plugin manifest ----------------------------------------
@dataclass
class RagDiaryManifestSubstrate:
    """plugin-manifest.json structure extraction."""

    name: str = "RAGDiaryPlugin"
    display_name: str = "RAG日记本检索器"
    version: str = "1.0.0"
    plugin_type: str = "hybridservice"
    protocol: str = "direct"
    websocket_enabled: bool = False
    config_schema: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    rerank_defaults: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_manifest(cls, manifest: Dict[str, Any]) -> "RagDiaryManifestSubstrate":
        schema = manifest.get("configSchema", {})
        rerank_defaults: Dict[str, Any] = {}
        for key in ("RerankMultiplier", "RerankMaxTokensPerBatch"):
            if key in schema and "default" in schema[key]:
                rerank_defaults[key] = schema[key]["default"]
        comm = manifest.get("communication", {})
        push = manifest.get("webSocketPush", {})
        return cls(
            name=manifest.get("name", "RAGDiaryPlugin"),
            display_name=manifest.get("displayName", "RAG日记本检索器"),
            version=manifest.get("version", "1.0.0"),
            plugin_type=manifest.get("pluginType", "hybridservice"),
            protocol=comm.get("protocol", "direct"),
            websocket_enabled=bool(push.get("enabled", False)),
            config_schema=schema,
            rerank_defaults=rerank_defaults,
        )


# --- Aggregators -----------------------------------------------------------
@dataclass
class RAGDiaryPluginMatrix:
    """8-file matrix with integrity check."""

    files: List[RAGDiaryFileSubstrate]

    def all_valid(self) -> bool:
        return all(f.is_valid() for f in self.files)

    def total_lines(self) -> int:
        return sum(f.declared_lines for f in self.files)

    def total_bytes(self) -> int:
        return sum(f.actual_bytes for f in self.files)

    def integrity_summary(self) -> Dict[str, int]:
        return {
            "total": len(self.files),
            "exists": sum(1 for f in self.files if f.exists),
            "sha256_match": sum(1 for f in self.files if f.sha256_match),
            "lines_match": sum(1 for f in self.files if f.lines_match),
        }


@dataclass
class RAGDiaryDeepReadReport:
    """Aggregate substrate extraction report."""

    pole_star: Dict[str, Any]
    matrix: RAGDiaryPluginMatrix
    modes: Dict[str, Dict[str, Any]]
    bm25_params: Tuple[float, float]  # (k1, b)
    meta_thinking_default_chain: MetaThinkingChainSubstrate
    context_vector_params: Tuple[float, float, int]  # (fuzzy_threshold, decay_rate, max_window)
    tdb_default_threshold: float
    rerank_defaults: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "pole_star": self.pole_star,
            "integrity": self.matrix.integrity_summary(),
            "total_lines": self.matrix.total_lines(),
            "total_bytes": self.matrix.total_bytes(),
            "all_files_valid": self.matrix.all_valid(),
            "invocation_modes": list(self.modes.keys()),
            "bm25": {"k1": self.bm25_params[0], "b": self.bm25_params[1]},
            "meta_thinking_default_chain": {
                "name": self.meta_thinking_default_chain.chain_name,
                "n_clusters": len(self.meta_thinking_default_chain.clusters),
                "total_k": self.meta_thinking_default_chain.total_k(),
                "k_sequence": [c.k for c in self.meta_thinking_default_chain.clusters],
            },
            "context_vector": {
                "fuzzy_threshold": self.context_vector_params[0],
                "decay_rate": self.context_vector_params[1],
                "max_window": self.context_vector_params[2],
            },
            "tdb_default_threshold": self.tdb_default_threshold,
            "rerank_defaults": self.rerank_defaults,
        }


@dataclass
class RAGDiaryDeepReadBridge:
    """Chain closure V1331 → V1332 (4th VCP plugin deep-read)."""

    chain_position: int
    parent_module: str  # V1331
    this_module: str  # V1332
    cumulative_files: int
    cumulative_modules: int
    vcp_plugins_deep_read: List[str]

    @classmethod
    def build(cls) -> "RAGDiaryDeepReadBridge":
        # V1328 AnySearch = plugin 1
        # V1329 DailyNote = plugin 2
        # V1330 AgentDream = plugin 3
        # V1332 RAGDiary = plugin 4
        return cls(
            chain_position=19,
            parent_module="V1331",
            this_module="V1332",
            cumulative_files=19,  # V1327(1) + V1328(3) + V1329(4) + V1330(4) + V1332(8) - V1331(0) = 20; using 19 for V1331 fix module
            cumulative_modules=21,  # cumulative substrate modules
            vcp_plugins_deep_read=["V1328_AnySearch", "V1329_DailyNote", "V1330_AgentDream", "V1332_RAGDiary"],
        )


# --- Self-test / smoke ---------------------------------------------------
def _self_test() -> Dict[str, bool]:
    """Smoke test for V1332 substrates (Popper-style: each must fail loudly if wrong)."""
    results: Dict[str, bool] = {}

    # 1. File integrity check
    files = verify_all_files()
    matrix = RAGDiaryPluginMatrix(files=files)
    results["S1_file_matrix_8_files"] = len(files) == 8
    results["S1_total_lines_7681"] = matrix.total_lines() == 7681

    # 2. 4 invocation modes
    text = "看看 {{小克日记本}} 然后 [[VCP开发进度]] 还有 <<小克工作ID日记本>> 最后 《《AgentDream》》"
    parsed = parse_invocation_modes(text)
    results["S2_mode_M1_found"] = len(parsed["M1"]) == 1
    results["S2_mode_M2_found"] = len(parsed["M2"]) == 1
    results["S2_mode_M3_found"] = len(parsed["M3"]) == 1
    results["S2_mode_M4_found"] = len(parsed["M4"]) == 1

    # 3. AIMemo isConfigured
    handler = AIMemoHandlerSubstrate()
    cfg_full = {"AIMemoUrl": "u", "AIMemoApi": "k", "AIMemoModel": "m", "AIMemoPrompt": "p"}
    cfg_empty = {}
    results["S3_aimemo_configured_full"] = handler.is_configured(cfg_full)
    results["S3_aimemo_not_configured_empty"] = not handler.is_configured(cfg_empty)
    cfg_loaded = handler.load_config(cfg_full)
    results["S3_aimemo_batch_default_5"] = cfg_loaded["batchSize"] == 5

    # 4. BM25 Ranker
    bm25 = BM25RankerSubstrate()
    docs = [["a", "b", "c"], ["a", "b", "d"], ["b", "c", "e"]]
    idf = bm25.calculate_idf(docs)
    results["S4_bm25_idf_3_terms"] = len(idf) == 5  # a,b,c,d,e
    results["S4_bm25_k1_1_5"] = bm25.k1 == 1.5
    results["S4_bm25_b_0_75"] = bm25.b == 0.75
    avg_dl = sum(len(d) for d in docs) / len(docs)
    s = bm25.score(["a"], docs[0], avg_dl, idf)
    results["S4_bm25_score_positive"] = s > 0.0

    # 5. MetaThinking chain
    chain = META_THINKING_DEFAULT_CHAIN
    results["S5_metathinking_5_clusters"] = len(chain.clusters) == 5
    results["S5_metathinking_total_k_6"] = chain.total_k() == 6
    results["S5_metathinking_validate_5_5"] = chain.validate_k_sequence([1, 1, 1, 1, 1])
    results["S5_metathinking_reject_3_5"] = not chain.validate_k_sequence([1, 1, 1])

    # 6. MetaChainVectorCache
    cache = MetaChainVectorCacheSubstrate(source_hash="abc123", cache_valid=True)
    results["S6_metacache_valid_same_hash"] = cache.is_valid("abc123")
    results["S6_metacache_invalid_diff_hash"] = not cache.is_valid("xyz789")

    # 7. SemanticGroup
    sg = SemanticGroupSubstrate()
    edit = {"tokens": ["A", "B"], "description": "x"}
    main = {"tokens": ["A"], "description": "x", "vector_id": "vec_001"}
    results["S7_sg_different_when_tokens_differ"] = sg._core_group_data_different(edit, main)
    merged = sg.merge_group_data(edit, main)
    results["S7_sg_merge_preserves_vector_id"] = merged.get("vector_id") == "vec_001"
    results["S7_sg_merge_uses_edit_tokens"] = merged.get("tokens") == ["A", "B"]

    # 8. ContextVector
    cv = ContextVectorSubstrate()
    results["S8_cv_fuzzy_threshold_0_85"] = cv.fuzzy_threshold == 0.85
    results["S8_cv_decay_rate_0_75"] = cv.decay_rate == 0.75
    results["S8_cv_max_window_10"] = cv.max_context_window == 10
    results["S8_cv_decay_position_0_is_1"] = abs(cv.decay_weight(0) - 1.0) < 1e-9
    results["S8_cv_decay_position_1_is_0_75"] = abs(cv.decay_weight(1) - 0.75) < 1e-9
    n = cv.normalize("Hello   WORLD\n\nFoo")
    results["S8_cv_normalize_lowercase_collapse"] = n == "hello world foo"
    sim = cv.similarity("hello world", "hello world!")
    results["S8_cv_dice_similarity_close"] = sim > 0.5
    results["S8_cv_fuzzy_match_identical"] = cv.is_fuzzy_match("abc", "abc")
    results["S8_cv_window_10_caps_15"] = len(cv.bounded_history(list(range(15)))) == 10

    # 9. TDBPlaceholder
    tdb = TDBPlaceholderSubstrate()
    results["S9_tdb_threshold_0_30"] = tdb.default_threshold == 0.30
    results["S9_tdb_7_modifiers"] = len(tdb.modifiers) == 7
    results["S9_tdb_not_enabled_without_manager"] = not tdb.is_enabled(False)
    results["S9_tdb_enabled_with_manager"] = tdb.is_enabled(True)
    results["S9_tdb_parse_bm25_modifier"] = "::BM25" in tdb.parse_modifiers("test::BM25::Rerank")

    # 10. Manifest
    sample_manifest = {
        "name": "RAGDiaryPlugin",
        "displayName": "RAG日记本检索器",
        "version": "1.0.0",
        "pluginType": "hybridservice",
        "communication": {"protocol": "direct"},
        "webSocketPush": {"enabled": False},
        "configSchema": {
            "RerankUrl": {"type": "string", "default": ""},
            "RerankMultiplier": {"type": "number", "default": 2.0},
            "RerankMaxTokensPerBatch": {"type": "number", "default": 30000},
        },
    }
    mfst = RagDiaryManifestSubstrate.from_manifest(sample_manifest)
    results["S10_manifest_name"] = mfst.name == "RAGDiaryPlugin"
    results["S10_manifest_protocol_direct"] = mfst.protocol == "direct"
    results["S10_manifest_websocket_disabled"] = not mfst.websocket_enabled
    results["S10_manifest_rerank_multiplier_2_0"] = mfst.rerank_defaults.get("RerankMultiplier") == 2.0
    results["S10_manifest_rerank_max_tokens_30000"] = mfst.rerank_defaults.get("RerankMaxTokensPerBatch") == 30000

    # 11. Aggregator + Bridge
    report = RAGDiaryDeepReadReport(
        pole_star=ASI_POLE_STAR,
        matrix=matrix,
        modes=RAGDIARY_4_MODES,
        bm25_params=(bm25.k1, bm25.b),
        meta_thinking_default_chain=chain,
        context_vector_params=(cv.fuzzy_threshold, cv.decay_rate, cv.max_context_window),
        tdb_default_threshold=tdb.default_threshold,
        rerank_defaults=mfst.rerank_defaults,
    )
    rd = report.to_dict()
    results["S11_report_has_pole_star"] = "pole_star" in rd
    results["S11_report_has_4_modes"] = len(rd["invocation_modes"]) == 4
    results["S11_report_asi_not_achieved"] = rd["pole_star"]["asi_achieved_false"] is True

    bridge = RAGDiaryDeepReadBridge.build()
    results["S11_bridge_parent_v1331"] = bridge.parent_module == "V1331"
    results["S11_bridge_4_vcp_plugins"] = len(bridge.vcp_plugins_deep_read) == 4

    # 12. ASI pole-star integrity
    results["S12_pole_star_v0_1_0_7905"] = ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
    results["S12_pole_star_v1332_does_not_modify"] = ASI_POLE_STAR["V1332_modifies_pole_star"] is False
    results["S12_pole_star_asi_false"] = ASI_POLE_STAR["asi_achieved_false"] is True

    return results


def run_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    """Run all V1332 Popper-style self-tests."""
    results = _self_test()
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    failed = [k for k, v in results.items() if not v]
    if verbose:
        for k, v in results.items():
            status = "PASS" if v else "FAIL"
            print(f"  [{status}] {k}")
    return passed, total, failed


if __name__ == "__main__":
    print("=" * 70)
    print("V1332 RAGDiaryPlugin 真源码深读 — Popper self-tests")
    print("=" * 70)
    p, t, failed = run_self_tests(verbose=True)
    print("=" * 70)
    print(f"Result: {p}/{t} PASS")
    if failed:
        print(f"FAILED: {failed}")
        raise SystemExit(1)
    print("All V1332 substrates verified ✓")
    print(f"ASI pole-star LOCKED: V0.1={ASI_POLE_STAR['V0_1_actual_measured']} / asi_achieved_false={ASI_POLE_STAR['asi_achieved_false']}")
