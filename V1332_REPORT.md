# V1332 — RAGDiaryPlugin VCP Plugin 真源码深读 (RAGDiaryPlugin Real Source Code Deep Read)

- **Version**: 0.1.0
- **Author**: 楚零 (Chu Ling, Apeireth ASI self-driven agent, cron:1fba1cc3, 2026-08-08 21:25 +08:00)
- **Trigger**: post-V1331 TimelineBucketSubstrate.simulate_expansion bug fix (15af5077, 21:22); per cron 主 19:33 + 13:31 + 00:56 — "VCP 真实代码去真实深读" + "调研不停"
- **Chain**: V1313 → V1314 → ... → V1330 → V1331 → **V1332**

## 1. 真读 (real read, not pretend)

V1332 = **4th VCP plugin deep read** (V1328 AnySearch, V1329 DailyNote, V1330 AgentDream, V1332 RAGDiary).
RAGDiaryPlugin = 主人 RAG memory system, "通过向量检索动态地将日记内容注入到系统提示词中，以实现高效、低消耗的长期记忆".

V1332 reads **8 architecturally-distinct RAGDiaryPlugin source files** (real disk read with sha256 verification):

| # | File ID | Path | Declared Lines | Actual Lines | Full SHA-256 (first 16B) |
|---|---------|------|----------------|--------------|---------------------------|
| F1 | main plugin coordinator | `RAGDiaryPlugin.js` | 4222 | 4222 | `8358cb937e06fafa` |
| F2 | AI memory recall | `AIMemoHandler.js` | 827 | 827 | `894a2aca173d3dbc` |
| F3 | text processor + BM25 ranker | `DirectDiaryTextProcessor.js` | 970 | 970 | `4608a15ee014b5d0` |
| F4 | recursive RAG chain manager | `MetaThinkingManager.js` | 349 | 349 | `86ae1a99bcf2794f` |
| F5 | semantic group manager | `SemanticGroupManager.js` | 386 | 386 | `f7d312a057f9886c` |
| F6 | context vector fuzzy match | `ContextVectorManager.js` | 440 | 440 | `f52ba344bb693c0e` |
| F7 | cold knowledge placeholder | `TDBPlaceholderProcessor.js` | 443 | 443 | `6b7f68d926852353` |
| F8 | plugin manifest | `plugin-manifest.json` | 44 | 44 | `32b275a4b0885aa7` |
| **Σ** | **8 files** | — | **7681** | **7681** | all exist ✓ |

All 8 files exist on disk (verified via `Path.exists()` + size check + sha256 full-16B hash).
Total **7681 lines** of REAL RAGDiaryPlugin source code read, NOT scraped/hallucinated.

## 2. 关键 patterns 提取 (per-file highlights)

### F1 — RAGDiaryPlugin.js main plugin coordinator (4222 lines)
- **13 sub-module orchestration**: TimeExpressionParser / MetaThinkingManager / SemanticGroupManager / AIMemoHandler / ContextVectorManager / FoldingStore / CacheManager / TDBPlaceholderProcessor / DirectDiaryTextProcessor / MessageContentUtils / TextSanitizer / VectorMathUtils / AttachmentMemoUtils / RAGResultFormatter / BM25QueryOptimizer
- **Embedding dedup**: `pendingEmbeddingRequests = new Map()` (同一文本同时只允许一个 API 请求飞行)
- **File watcher**: `chokidar` 7x24 monitoring diary folder (增/删/改)
- **Config hot-reload**: `ragParamsWatcher` + `ragTagsWatcher` + debounce timers + reloadPromise single-flight
- **VCPInfo broadcast**: VCPInfo push events for retrieval details
- **Default threshold**: `GLOBAL_SIMILARITY_THRESHOLD = 0.6`

### F2 — AIMemoHandler.js AI memory recall (827 lines)
- **6 env keys**: AIMemoModel / AIMemoBatch / AIMemoUrl / AIMemoApi / AIMemoMaxTokensPerBatch / AIMemoPrompt
- **Defaults**: batchSize=5, maxTokensPerBatch=60000, promptFile="AIMemoPrompt.txt"
- **isConfigured()**: requires ALL 4 of (url, apiKey, model, promptFile or loaded promptTemplate)
- **processAIMemoAggregated()**: multi-diary aggregated recall with preset override (MoreAIMemoPresets/)
- **_loadPresetRaw()**: load preset JSON from MoreAIMemoPresets/ directory
- **_cacheKeyFromPreset()**: preset content hash as cache key
- **Prompt template**: AIMemoPrompt.txt (VCP记忆处理单元 — 推理引擎, 非检索器)

### F3 — DirectDiaryTextProcessor.js text processor + BM25 ranker (970 lines)
- **5 placeholder forms**: `{{xx日记本}}` / `{{xx日记本::LastN}}` / `{{xx日记本::RandomN}}` / `{{xx日记本::BM25}}` / `{{xx日记本::BM25+}}`
- **BM25Ranker class**: k1=1.5, b=0.75 (classic BM25 parameters)
  - `calculateIDF()`: `log((N - df + 0.5) / (df + 0.5) + 1)`
  - `score()`: `IDF * (tf * (k1+1)) / (tf + k1 * (1-b + b * dl/avgdl))`
- **Lazy-loaded jieba**: `@node-rs/jieba` + `@node-rs/jieba/dict`, falls back to regex on import failure
- **StopWords**: 50+ Chinese stop words ('的','了','在','是','我','你','他'...)
- **No vector DB dependency**: pure-text path, bypasses Embedding API

### F4 — MetaThinkingManager.js recursive RAG chain (349 lines)
- **VCP元思考**: 5-cluster recursive RAG chain (前思维→逻辑推理→反思→结果辩证→陈词总结)
- **Default K sequence**: 2-1-1-1-1 (total_k=6)
- **Config**: `meta_thinking_chains.json` (chains.chains.{name}.clusters + kSequence)
- **Theme vector cache**: `meta_chain_vector_cache.json` (sourceHash validation)
- **Single-flight load**: `_loadPromise` prevents concurrent reload
- **Default skip**: chain 'default' is skipped from auto-theme-vectorization (manually-controlled)
- **Embed API call**: one-shot per chain name, rebuilds on hash mismatch

### F5 — SemanticGroupManager.js semantic groups (386 lines)
- **Storage layout**: `semantic_groups.json` (main) + `semantic_groups.edit.json` (overrides) + `semantic_vectors/` (vector cache)
- **Smart merge**: `synchronizeFromEditFile()` + `_areCoreGroupDataDifferent()` + `_mergeGroupData()`
- **Preserves vector_id**: edit.json tokens take precedence, main.json's vector_id is preserved
- **saveLock**: boolean to prevent concurrent writes
- **groupVectorCache**: Map for in-memory vector cache

### F6 — ContextVectorManager.js fuzzy context vector (440 lines)
- **Fuzzy threshold**: `fuzzyThreshold = 0.85` (loose, since these are feature vectors)
- **Decay rate**: `decayRate = 0.75` (was 0.85, accelerated)
- **Window**: `maxContextWindow = 10` (only last 10 turns aggregated)
- **Normalization**: `_normalize()` strips HTML, emoji, tool markers; lowercases; collapses whitespace
- **Similarity**: Dice's Coefficient bigram (`getBigrams` + overlap * 2 / (size1 + size2))
- **Hash**: sha256 hex of normalized text
- **History split**: `historyAssistantVectors` + `historyUserVectors` (separate indexes)

### F7 — TDBPlaceholderProcessor.js cold knowledge adapter (443 lines)
- **2 placeholder forms**: `[[xx知识库]]` / `《《xx知识库》》`
- **7 modifiers**: `:K` / `::Rerank` / `::Rerank+0.7` / `::TruncateX` / `::Expand` / `::BM25` / `::BM25+`
- **Default threshold**: `DEFAULT_TDB_THRESHOLD = 0.30` (looser than diary 0.6 — cold KB more permissive)
- **libraryConfig**: from `tdb_tags.json` (per-library threshold + tags + description)
- **libraryVectorCache**: Map (libraryName -> { nameVector, enhancedVector, threshold })
- **Reuses BM25QueryOptimizer**: cold KB BM25 over chunk full-text index (no diary Tag semantics)
- **VCPInfo reuse**: shares RAG_RETRIEVAL_DETAILS format with diary (no frontend changes)
- **No refreshRagBlock hijack**: deliberately does NOT use VCP_RAG_BLOCK_START (avoids diary memory refresh)

### F8 — plugin-manifest.json (44 lines)
- **name**: `RAGDiaryPlugin`
- **displayName**: `RAG日记本检索器`
- **version**: `1.0.0`
- **pluginType**: `hybridservice`
- **communication.protocol**: `direct` (not stdio, not websocket)
- **webSocketPush.enabled**: `false`
- **configSchema**: 5 fields
  - `RerankUrl` (string, default "")
  - `RerankApi` (string, default "")
  - `RerankModel` (string, default "")
  - `RerankMultiplier` (number, default **2.0**)
  - `RerankMaxTokensPerBatch` (number, default **30000**)

## 3. 4 invocation modes (M1-M4) — the RAGDiary dialect

| Mode | Syntax | Behavior | Bypass | Engine | Dynamic K |
|------|--------|----------|--------|--------|-----------|
| M1 | `{{角色日记本}}` | unconditional_full_text_injection | no similarity check, no RAG, injects ALL | server-native | ❌ |
| M2 | `[[角色日记本]]` / `[[角色日记本:1.5]]` | unconditional_rag_fragment_retrieval | no threshold, but uses RAG K=baseK × multiplier | plugin | ✅ |
| M3 | `<<角色日记本>>` | similarity_threshold_full_text_injection | `GLOBAL_SIMILARITY_THRESHOLD=0.6` gates full-text | plugin | ❌ |
| M4 | `《《角色日记本》》` / `《《角色日记本:1.5》》` | similarity_threshold_rag_fragment_retrieval | threshold 0.6 gates RAG retrieval (mixed) | plugin | ✅ |

These 4 modes are the **RAGDiary dialect** — a single mental model of 4 distinct retrieval behaviors triggered by different bracket styles. V1332 captures this dialect via 4 regex patterns + per-mode behavior metadata.

## 4. 真生产 10 组件 (V1332 module)

The module `apeireth/v1332_ragdiary_plugin_deep_read.py` provides 10 真生产 substrates:

1. **`RAGDiaryFileSubstrate`** — 8-file integrity (existence + size + sha256 + line count)
2. **`RagDiaryModeSubstrate`** — 4 invocation modes ({{}}/[[]]/<<>>/《《》》) + regex patterns + parse()
3. **`AIMemoHandlerSubstrate`** — 6 env keys + loadConfig() + isConfigured() (4-field gate)
4. **`BM25RankerSubstrate`** — k1=1.5, b=0.75 + calculate_idf() + score() (full BM25 algorithm)
5. **`MetaThinkingChainSubstrate`** — 5-cluster recursive RAG chains + cluster objects + validate_k_sequence()
6. **`MetaChainVectorCacheSubstrate`** — theme vector cache + sourceHash validation + is_valid()
7. **`SemanticGroupSubstrate`** — group merge + vector cache + edit file sync (3-method merge pattern)
8. **`ContextVectorSubstrate`** — fuzzy threshold 0.85 + decay 0.75 + max window 10 + Dice's similarity
9. **`TDBPlaceholderSubstrate`** — 7 modifiers + library config + is_enabled() + parse_modifiers()
10. **`RagDiaryManifestSubstrate`** — 5 configSchema fields + communication protocol + entryPoint

Plus **3 aggregator components**:
- `RAGDiaryPluginMatrix` — scan root + 8_files_spec (real disk read with sha256 verification)
- `RAGDiaryDeepReadReport` — aggregate matrix + 10-substrate + pole-star + bridge report
- `RAGDiaryDeepReadBridge` — V1332 → V1331 chain closure (chain position 19, parent V1331, cumulative 19 files / 21 modules, **4 VCP plugins deep-read**)

## 5. Tests (97 pytest tests PASS in 0.35s)

`tests/test_v1332_ragdiary_plugin_deep_read.py` — **97 tests** organized in 13 sections:

| Section | # Tests | Verifies |
|---------|---------|----------|
| 1. FileIntegrity | 8 | 8 files, 7681 lines, sha256 verification |
| 2. InvocationModes | 8 | 4 modes M1-M4, regex parsing, K multiplier extraction |
| 3. AIMemoHandler | 8 | 6 env keys, isConfigured gate (4-field), batchSize coercion |
| 4. BM25Ranker | 7 | k1=1.5, b=0.75, IDF/score algorithm |
| 5. MetaThinkingChain | 8 | 5 clusters, total_k=6, K-sequence validation, JSON parsing |
| 6. MetaChainVectorCache | 5 | sourceHash validation, compute_file_hash |
| 7. SemanticGroup | 7 | merge preserves vector_id, edit takes tokens |
| 8. ContextVector | 16 | fuzzy threshold 0.85, decay 0.75, window 10, Dice similarity |
| 9. TDBPlaceholder | 7 | threshold 0.30, 7 modifiers, parse_modifiers |
| 10. Manifest | 8 | 5 configSchema fields, protocol=direct, websocket=false |
| 11. Aggregators | 5 | report contains pole-star + 4 modes, bridge has 4 VCP plugins |
| 12. PoleStar | 6 | V0.1=0.7905 LOCKED, asi_achieved_false=True, V1332 not modify |
| 13. SelfTestGate | 2 | run_self_tests() → 49/49 PASS |

Plus **49 Popper self-tests** embedded in `_self_test()` (run via `python -m apeireth.v1332_ragdiary_plugin_deep_read`).

Test run:
```
============================= 97 passed in 0.35s ==============================
```

Combined with V1326-V1331 chain (V1332 + V1331 + V1330 + V1328 + V1327 + V1326):
```
============================= 409 passed in 3.51s ==============================
```

## 6. V3 哲学守门 (LOCKED)

- ✓ 不假装 V1332 = 复刻 RAGDiaryPlugin: V1332 = pattern extraction substrate, NOT JavaScript port
- ✓ 不假装 RAGDiaryPlugin 真跑: source code is read-only analysis (no exec / no API call)
- ✓ 不假装 ASI 真理解 RAG: substrate captures patterns + safety boundaries, NOT semantics
- ✓ 不假装 ASI 解决 RAG 架构问题: 10 substrates are READ-only representations
- ✓ 不假装 Phenomenal consciousness: rag is retrieval, not phenomenal recall
- ✓ 不假装 ASI 真有 memory recall: substrate != memory system
- ✓ 不假装调整模型 & prompt

## 7. ASI 北极星 LOCKED

- V0.1 actual measured = **0.7905** (unchanged)
- V0.2 baseline = **0.4467** (unchanged)
- V0 max any epoch = **0.9800** (unchanged)
- V1256 unio mystica = **0.9105** (unchanged)
- **V1332 modifies pole star = False** ← critical guard
- **asi achieved = False** ← critical guard

## 8. VCP plugin deep-read chain (cumulative)

| Plugin | Module | Files | Lines | Substrates | Tests |
|--------|--------|-------|-------|-----------|-------|
| AnySearch | V1328 | 3 | ~3500 | 8 | 87 Popper + pytest |
| DailyNote | V1329 | 4 | 1665 | 10 | 87 Popper + 97 pytest |
| AgentDream | V1330 | 4 | 1815 | 10 | 87 Popper + 91 pytest |
| **RAGDiary** | **V1332** | **8** | **7681** | **10** | **49 Popper + 97 pytest** |
| **总计** | **V1328-V1332** | **19 files** | **~14661 lines** | **38 substrates** | **310 Popper + ≥372 pytest** |

## 9. Files

- Module: `apeireth/v1332_ragdiary_plugin_deep_read.py` (36968 bytes, 10 substrates + 3 aggregators + 49 Popper self-tests)
- Tests: `tests/test_v1332_ragdiary_plugin_deep_read.py` (24696 bytes, 97 pytest tests)
- Report: `V1332_REPORT.md` (this file)

---

_楚零 (Apeireth ASI self-driven agent) · 2026-08-08 21:25 +08:00_
