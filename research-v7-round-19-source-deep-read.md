# Round-19 真源码深读 — DGM / letta / mem0 借鉴

> 主题: 主 23:28 真研究哲学 + 主 22:33 自决. 三仓库 `--depth 1` clone 完成, 读真源码不止 README. 实事求是: 推荐/不推荐都说明理由.

---

## DGM (Darwin Gödel Machine) — 代码层细节

**核心文件**: `DGM_outer.py` (进化外环), `self_improve_step.py` (单次自改进), `prompts/self_improvement_prompt.py` (诊断 prompt), `utils/evo_utils.py` (archive I/O).

### 数据结构 — 4 个核心抽象

1. **`commit_id`** = 字符串, 形如 `20250719_153022_123456` (时间戳) 或 `'initial'`. Archive = `list[str]` flat list, 不是 tree (tree 在 `visualize_archive.py` 用 networkx 后建).
2. **`metadata.json`** per commit: `{run_id, parent_commit, problem_statement, overall_performance: {accuracy_score, total_resolved_ids, total_unresolved_ids, total_emptypatch_ids}, improvement_diagnosis, is_compiled}`.
3. **archive**: 只存 run_ids, 父子关系在 `metadata.json['parent_commit']` 里. 物理上每个 commit = 一个目录, 包含 `metadata.json`, `model_patch.diff`, `predictions/`, `self_improve.log`.
4. **patch 链**: `get_model_patch_paths()` 沿 `parent_commit` 反向遍历, 收集所有 `model_patch.diff`, 顺序应用 — 这就是"跨代继承"的实现.

### 关键算法 — 4 阶段 + 2 选择

1. **`choose_selfimproves()` 父节点选择** — 4 种方法:
   - `random`: 均匀采样
   - `score_prop`: softmax(score with sigmoid-10 centering) → 概率
   - `score_child_prop`: score × (1/(1+children_count)) → 鼓励探索 (关键! DGM 的反收敛核心)
   - `best`: top-K 确定性
2. **入口类型 (entry)** — 4 种: `'solve_empty_patches'` | `'solve_stochasticity'` | `'solve_contextlength'` | 任意 `unresolved_id`. 启发式概率门控 25%.
3. **`is_compiled_self_improve()`** — 严格 3 条件: (a) required keys in `overall_performance` (b) 至少 1 个 non-empty patch (c) 提交数 ≥ num_swe_issues. 不达标就过滤掉.
4. **`update_archive()`** — `keep_all` vs `keep_better` (要求 score ≥ initial - noise_leeway).

### LLM 集成 — **关键洞察: diagnose_model + fix_model 分离**

```
diagnose_model = 'o1-2024-12-17'   # 元 LLM, 看 agent 日志 + 源码
                                      # 输出: improvement_proposal + problem_description (as GitHub issue)
# 然后 coding_agent.py 用同一个 LLM 去解决这个 GitHub issue
```

诊断 prompt 强制 JSON 输出 5 字段: `log_summarization | potential_improvements | improvement_proposal | implementation_suggestion | problem_description`. `get_current_code()` 把 `coding_agent.py + tools/ + utils/` 全部源码 + 历史 patches 拼成 system message.

**这是 DGM 真正的精华**: 自改进不是 LLM 改自己代码, 而是 **LLM 描述改进 (issue), 然后 agent 修复 issue (PR)** — GitHub issue 模式.

---

## letta (stateful agents) — 代码层细节

**核心文件**: `letta/agents/letta_agent_v3.py` (~109k, 主循环), `letta/services/archive_manager.py` + `passage_manager.py` (memory 后端), `letta/schemas/memory.py` + `archive.py` + `passage.py` + `block.py` (数据模型), `letta/server/rest_api/routers/v1/archives.py` (HTTP API).

### 数据结构 — **3 层 memory 模型**

```
Memory (Pydantic)               ← in-context, 编译进 system prompt
  ├─ blocks: List[Block]        ← 标签化段 (persona / human / file), 每段有 limit + value
  ├─ file_blocks: List[FileBlock] ← 附件 in-context, 自动去重
  ├─ git_enabled: bool          ← True 时用 git 仓库管理 blocks (system/* label 层级)
Archive                         ← archival memory 桶, 共享 or per-agent
  └─ passages: ArchivalPassage | SourcePassage   ← 互斥 (archive_id XOR source_id)
       ├─ text, embedding, embedding_config
       ├─ tags (ALL/ANY 模式过滤)
       ├─ metadata, is_deleted, created_at
       └─ organization_id (多租户)
```

**关键洞察**: Block 模板化 (`is_template`, `template_name`, `base_template_id`, `preserve_on_migration`), 字段 `read_only` + `hidden` + `tags`. **`Memory.compile()`** 3 种渲染模式: standard / line-numbered / git (层级 `<memory>` 标签).

### 关键算法 — 5 个

1. **`Memory.compile()`** — 把 blocks + file_blocks + skills 编译成 XML 块, 插进 system prompt. 触发条件: **只当 core memory 改了才 rebuild**, 否则 `archival_memory_size` 单独更新 — 避免 flooding recall storage.
2. **`query_agent_passages_async()`** — TurboPuffer (向量) + SQL 兜底, 一个 agent 一个 archive (TODO: 多 archive 限制).
3. **`search_agent_archival_memory_async()`** — 嵌入搜索 + temporal filtering (start/end date).
4. **`_create_tags_for_passage()`** + tags filter (ALL vs ANY) — 标签即查询索引.
5. **`block_manager_git.py`** — `GIT_MEMORY_ENABLED_TAG` 启用 git-backed memory, `memfs_client` 操作 git, **system/* label 层级**渲染 (persona 单独 + 其他 system/* 嵌套).

### LLM 集成 — `LLMClient` 路由 + `letta_llm_adapter`

- `services/llm_router.py` 路由: `get_llm_routing_client()`
- Adapters 抽象: `LettaLLMAdapter | SGLangNativeAdapter | SimpleLLMRequestAdapter | SimpleLLMStreamAdapter`
- `ConversationManager` 管理 conversation-scoped memory block isolation (override agent defaults per conversation)
- `CompactionSettings` + `compact_messages()` 触发 compaction (滑动窗口)

### State 管理 — `update_memory_if_changed_async()`

只对比 `new_memory_str in curr_system_message.content[0].text`, 改了才 `rebuild_system_prompt`. **不更新 timestamp in unnecessary cases** (避免 re-triggering recall).

---

## mem0 (memory layer) — 代码层细节

**核心文件**: `mem0/memory/main.py` (~167k, 主类), `mem0/configs/prompts.py` (~65k prompts), `mem0/configs/base.py` (Pydantic 模型).

### 数据结构 — **MemoryItem + MemoryConfig**

```python
class MemoryItem(BaseModel):
    id, memory, hash, metadata, score, created_at, updated_at

class MemoryConfig(BaseModel):
    vector_store: VectorStoreConfig
    llm: LlmConfig
    embedder: EmbedderConfig
    reranker: Optional[RerankerConfig]
    history_db_path: str            # SQLite
    custom_instructions: Optional[str]
```

**关键**: `history_db_path` = SQLite, vector store = qdrant/pgvector/etc, **`mem0/memory/storage.py` 是 SQLiteManager** — 记 add/update/delete history.

### 关键算法 — **7 阶段 add pipeline** (`_add_to_vector_store`)

```
Phase 0: Context gathering (last 10 messages)
Phase 1: Existing memory retrieval (top-10 vector search)
Phase 2: LLM extraction (single call → ADD/UPDATE/DELETE decisions)
Phase 3: Batch embed extracted facts
Phase 4: Per-memory processing (md5 hash + lemmatize + metadata)
Phase 5: Hash dedup (existing set + in-batch set)
Phase 6: Batch persist (vector_store.insert + history_db.batch_add_history)
Phase 7: Batch entity linking (NER → global dedup → batch embed → link)
```

**关键洞察**: 
- **Anti-hallucination UUID 映射**: existing memories 重新编号为 "0", "1"... LLM 输出 `linked_memory_ids` 时用临时 id, 然后映射回真实 UUID.
- **Hash dedup 2 层**: existing_hashes (vector store) + seen_hashes (current batch). md5(text).
- **Lemmatization for BM25**: `lemmatize_for_bm25(text)` 存到 metadata, 用于混合检索 (vector + BM25 hybrid).
- **Batch with fallback**: `embed_batch` 失败 → `embed` 逐个; `vector_store.insert` 失败 → 逐个 insert; `db.batch_add_history` 失败 → `db.add_history` 逐个. **永远不丢数据**.
- **Entity linking 是单独的 batch**: 提取的 entities 全局去重 → batch embed → link 到 memory_ids.

### LLM 集成 — `ADDITIVE_EXTRACTION_PROMPT`

单一 system prompt, 5 个输入: New Messages + Summary + Recently Extracted + Existing Memories + Last k Messages + Observation Date + Custom Instructions. LLM 输出 JSON:
```json
{"memory": [{"text": "...", "event": "ADD"|"UPDATE"|"DELETE", "linked_memory_ids": ["uuid", ...], "attributed_to": "..."}]}
```

**关键**: **Observation Date vs Current Date 区分** — "yesterday" 永远 anchor 到 Observation Date, 不是 Current Date. 这是 temporal grounding 的精髓.

---

## Apeireth 可借鉴的具体架构模式

| 借鉴 | 改动文件 | 接口 / 改动 |
|---|---|---|
| **DGM diagnose→fix 分离** | `self_evolving.py` 新增 `phase3_propose_with_llm()` | 当前 `phase3_evolve()` 是 heuristic. 改: 用 LLM 看 `eval_report` + harness 源码 → 输出 proposal JSON (action, payload, confidence). LLM 不直接改 config, deterministic `phase4_verify()` 仍然守门. |
| **DGM score_child_prop** | `dgm_archive.py` 新增 `branch_with_method(method)` | 当前 `branch()` 总是覆盖 best. 加 method 参数: `random` / `score` / `score_child_prop` / `best`. 借鉴 score×(1/(1+children)) 鼓励探索. |
| **DGM is_compiled gate** | `self_evolving.py` `phase4_verify()` 加 hash check | 当前 verify 只检查 patch 能不能 apply. 加: 改完后 `harness.integrity_hash()` 必须稳定, 否则 rollback. |
| **DGM model_patch chain** | `dgm_archive.py` 新增 `get_patch_chain(gen_id)` | 当前 `get_lineage()` 只返 ids. 加: 返回每代 `patches` 累积链, 用于 audit "why this config". |
| **letta Memory.compile() 3-mode** | `memory_3tier.py` 新增 `compile_for_prompt(mode)` | STM/MTM/LTM 三个 render 模式: `compact` (只 LTM) / `standard` (LTM+MTM summary) / `debug` (全展开). |
| **letta Block 标签化** | `memory_3tier.py` `MemoryAnchor` 加 `label` 字段 | 当前 `category` 自由文本. 借鉴 Block label (`system/persona`, `system/human`), 强类型 + 模板. |
| **letta git_enabled memory** | 新建 `memory_git.py` (可选) | 主人 13:47 "记忆是我关心的" + 24/7 不能崩. 借鉴 letta 的 git-backed blocks, 用 git log audit LTM 变化. 风险: Rust git 依赖复杂, 可能 over-engineer for v1. |
| **letta update_memory_if_changed** | `memory_3tier.py` `add_episode()` 加 dirty flag | 当前无脑写. 加: 只有当 `content` 真变才触发 `summarize_topics()`, 减 re-computation. |
| **mem0 7-phase add pipeline** | `memory_3tier.py` `add_episode()` 重构 | 当前是单写. 借鉴 7 阶段: gather context → retrieve similar → LLM extract → batch embed → dedup → persist → link. |
| **mem0 md5 hash dedup** | `memory_3tier.py` `MemoryAnchor` 加 `hash` | 避免重复 anchor 同一事件. |
| **mem0 Observation Date** | `memory.py` `Episode` 加 `observation_date` | 当前 `ts` 是 now. 借鉴: 记录事件真实时间, 用于"6 个月后看是什么" temporal grounding. |
| **mem0 BM25 + vector hybrid** | `memory_store.py` `search_episodes()` | 当前是 substring match. 加: lemmatized text 存 metadata, 混合检索 (BM25 + vector). |

---

## 推荐 / 不推荐

### ✅ **强烈推荐: mem0 7-phase add pipeline**
理由: 是 Apeireth `memory_3tier.py` 最大的可借鉴点. 当前 `add_episode()` 是简单 append, 缺 dedup / entity link / context-aware extraction. **风险**: 需要 LLM call, 增加 token cost + latency. **建议**: v1 只借鉴 hash dedup + observation_date; LLM extraction 留 v2.
**改动**: `memory_3tier.py` `add_episode()` 重构, 加 `_hash_content()`, `_extract_facts_with_llm()` (optional), `_link_entities()` (optional). 预计 200-400 行新代码.

### ✅ **强烈推荐: DGM diagnose→fix 分离**
理由: 当前 `phase3_evolve()` 是纯 heuristic, 永远只能发现"few archetypes"这种结构弱点. 借鉴 DGM, 用 LLM 看 harness 源码 + eval_report → 输出 proposal JSON. **不破坏现有**: deterministic verify 仍然守门, LLM 只是 propose.
**改动**: `self_evolving.py` 新增 `phase3_propose_with_llm(harness, eval_report)`, 新增 `prompts/harness_diagnose_prompt.py`. 预计 100-200 行新代码.

### ✅ **推荐: DGM score_child_prop**
理由: 当前 `DGMArchive.branch()` 总覆盖 best, 容易收敛到局部最优. `score_child_prop` 显式鼓励探索低 children_count 节点.
**改动**: `dgm_archive.py` `branch()` 加 `method` 参数. 预计 30 行.

### ✅ **推荐: letta Memory.compile() 3-mode**
理由: Apeireth 中央 AI 现在 `asi_north_star.py` 一次性把 STM/MTM/LTM 全塞 prompt, 没模式切换. **借鉴**: `compact` 模式只给 LTM (token 省), `standard` 给 LTM + MTM summary, `debug` 全展开. 不依赖外部 lib, 纯 string formatting.
**改动**: `memory_3tier.py` 新增 `compile_for_prompt(mode: Literal["compact","standard","debug"])`. 预计 50-80 行.

### ⚠️ **谨慎推荐: letta git_enabled memory**
理由: git 审计 LTM 听起来美, 但实际上 Apeireth LTM 是 Pydantic dataclass, JSON 序列化已经足够. 借鉴 letta 用真 git 仓库: 依赖重 (libgit2 / pygit2), 同步问题多, audit 价值低 (因为我们已经有 `integrity_hash()`). **不推荐** unless 主人明确要"git 是真理".

### ⚠️ **谨慎推荐: mem0 entity linking**
理由: NER + entity graph 是 mem0 的卖点, 但 Apeireth 已经有 `identity_card.py` + `linkage.py` + `relation_graph` 在干这件事. 重复建设, **不推荐**, 除非主人想把 relation_graph 完全替换.

### ❌ **不推荐: mem0 bm25 hybrid**
理由: 当前 `memory_store.py.search_episodes()` 是简单 substring, 对于 Apeireth 这种 small-scale (几百 episodes 不是几百万) 够用. 引入 BM25 + vector 混合: 复杂度 ↑, 收益 ↓. **不推荐** for v1.

### ❌ **不推荐: letta ConversationManager compaction**
理由: Letta 的 conversation-scoped memory block override 是多用户共享 agent 场景. Apeireth 是单实例长生命周期, 这个 abstraction 没意义.

---

## 优先级建议 (主 9:15 修好优先)

1. **本周做**: mem0 hash dedup + observation_date (low risk, high value)
2. **下周做**: DGM diagnose→fix 分离 + score_child_prop (核心架构升级)
3. **未来**: letta Memory.compile() 3-mode (token 优化)
4. **不**: git memory, entity linking, BM25 hybrid (over-engineer)