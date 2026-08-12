# R149 P0 完成 — 终极补弱 5/5 (2026-08-13)

> **定位**: R149 = R148 LOCKED 撤销后的第一个 R 周期, 专攻"全后端终极补弱"方向的最后一里路.
> **本次覆盖**: tool-fetch / skills / runtime / graph / formal 5 个升级点.

---

## 0. 总览

| 子模块 | 目标 | 状态 | 测试增量 |
|---|---|---|---|
| `#1` `apeireth-tool-fetch` | 统一 fetch 引擎, 吸收 VCP 7 插件 | ✅ | +44 |
| `#2` `apeireth-skills::anthropic_skills` | Anthropic Skills 模式 3 层 lazy load | ✅ | +12 |
| `#3` `apeireth-runtime::LlmWorker` | 真 MiniMax API worker (替 SimulatedWorker) | ✅ | +4 |
| `#4` `apeireth-graph::ThreadCheckpointStore` | LangGraph-style MemorySaver + 重入检查点 | ✅ | +8 |
| `#5` `apeireth-formal::l0_ha_physical_multisig` | 补 R131.6 audit 缺的 L0 HA M-of-N Kani proof | ✅ | +10 |

**总计**: 5/5, +78 tests, +1 新 crate (`apeireth-tool-fetch`), +4 新模块.

---

## #1 — `apeireth-tool-fetch` 统一 fetch 引擎

**新建 crate** (替代 R141 临时 inline + 吸收 VCP 7 插件):

| 模块 | 行数 | 职责 |
|---|---|---|
| `lib.rs` | 89 | 公开 API + 9 模块 re-export |
| `engine.rs` | 149 | FetchEngine 统一入口, dispatch URL 解析 |
| `http_fetch.rs` | 66 | HTTP GET (含 redirect / chunked) |
| `html_extract.rs` | 159 | HTML → 文本 (基于 regex + readability 简化版) |
| `cache.rs` | 100 | LRU TTL cache (parking_lot Mutex 保护) |
| `config.rs` | 31 | FetchConfig + 6 字段 (timeout / user-agent / max_bytes / cache_size / cache_ttl / follow_redirects) |
| `search_aggregator.rs` | 144 | 多源搜索聚合 (TavilySearch + AnySearch + VSearch) |
| `deep.rs` | 135 | FlashDeepSearch 深度页面解析 |
| `bilibili.rs` | 157 | Bilibili 视频元数据 fetch |
| `anime.rs` | 108 | AnimeFinder 番剧检索 |
| **合计** | **1138** | |

**吸收 VCP 7 插件** (1:1 复刻但 Rust 重写 + 升级):
1. `UrlFetch` → `engine.rs` + `http_fetch.rs`
2. `TavilySearch` → `search_aggregator.rs` (Tavily 部分)
3. `AnySearch` → `search_aggregator.rs` (any source 部分)
4. `VSearch` → `search_aggregator.rs` (VSearch 部分)
5. `FlashDeepSearch` → `deep.rs`
6. `BilibiliFetch` → `bilibili.rs`
7. `AnimeFinder` → `anime.rs`

**对比 VCP 优势**:
- 7 插件 → 1 crate: 一体化
- 共享 cache + config: 0 重复 IO
- 共享 error type: `FetchError` (thiserror)
- 内置 tokio async: 0 回调地狱

**借鉴来源**:
- `serde_json` (for structured search results)
- `parking_lot` (faster than std Mutex)
- `regex` (HTML extraction)
- `url` (URL parsing)
- `tokio` (async runtime)

---

## #2 — `apeireth-skills::anthropic_skills` Anthropic Skills 模式

**升级来源**: Anthropic 2025 发布的 Skills 协议 (3 层 lazy load pattern).

**新增文件**: `crates/apeireth-skills/src/anthropic_skills.rs` (355 lines).

**3 层 lazy load 架构** (per Anthropic Skills 规范):
1. **Manifest 层**: `SkillManifest { name, description, version, dependencies }` — 元信息
2. **Document 层**: `SkillDocument { manifest, instructions: String }` — 完整文档, 首次需要时 lazy load
3. **Entry 层**: `SkillEntry { manifest, document_hash, lazy_loader }` — 按需加载

**核心组件**:
- `SkillManifest` (POD struct, Kani 友好)
- `SkillDocument` (owned String, 真文档)
- `SkillEntry` (lazy loader 抽象)
- `AnthropicSkillLoader` (3 层缓存: LRU + file system + network)
- mini YAML parser (0 引 `serde_yaml` 350KB, 自写 80 行覆盖 frontmatter 子集)

**12 个新 unit test** 全过.

**借鉴 ID**: `R149-SKILLS-BORROW-anthropic-skills-spec-2025-10`

---

## #3 — `apeireth-runtime::LlmWorker` 真 MiniMax worker

**新增结构**: `LlmWorker { api_base, api_key, model, http_client }`.

**实现**: OpenAI Chat Completions 协议 (MiniMax 端点 `https://api.minimaxi.com/v1/chat/completions`).

**API 调用**: `Bearer <KEY>` 鉴权 + `POST /v1/chat/completions` body `{model, messages, ...}`.

**保留** `SimulatedWorker`: 给测试用 (0 网络, 0 鉴权).

**4 个新 unit test**:
1. `llm_worker_new_with_minimax_default` — 默认 MiniMax-M3 + base URL
2. `llm_worker_chat_request_structure` — 请求 body 序列化正确
3. `llm_worker_auth_bearer_header` — Authorization header 正确
4. `simulated_worker_preserved_for_tests` — SimulatedWorker 仍可用

**API key 位置**: `.openclaw` (per 主人指示, minimax preset 已配).

**Cargo.toml 新增 dep**: `apeireth-http-client = { path = "../apeireth-http-client" }`.

---

## #4 — `apeireth-graph::ThreadCheckpointStore` LangGraph-style

**新增文件**: `crates/apeireth-graph/src/thread_history.rs` (244 lines).

**API** (1:1 对应 LangGraph `MemorySaver`):
- `put(thread_id, checkpoint)` — 存检查点
- `get(thread_id)` — 取最新检查点
- `get_by_checkpoint_id(thread_id, checkpoint_id)` — 按 ID 取
- `history(thread_id)` — 全历史
- `list_threads()` — 列所有 thread
- `total_checkpoints()` — 全统计
- `rewind(thread_id, checkpoint_id)` — 回滚
- `delete_thread(thread_id)` — 删 thread

**存储**: 内存 HashMap (thread_id → Vec\<Checkpoint\>) + 可选 file persistence.

**借鉴来源**: LangGraph `MemorySaver` (Python) → Rust 重写 + parking_lot Mutex 优化.

**借鉴 ID**: `R149-GRAPH-BORROW-langgraph-memory-saver-2025-09`

**8 个新 unit test**:
1. `put_and_get_basic` — 存 + 取最新
2. `get_by_checkpoint_id` — 按 ID 取
3. `history_returns_full_chain` — 历史有序
4. `list_threads_returns_unique_ids` — 去重
5. `total_checkpoints_correct` — 统计
6. `rewind_restores_state` — 回滚
7. `delete_thread_removes_all` — 删清
8. `concurrent_writes_safe` — parking_lot 保护

---

## #5 — `apeireth-formal::l0_ha_physical_multisig` Kani proof

**R131.6 audit 暴露的 missing critical proof** (per `decision-131`).

**新增文件**: `crates/apeireth-formal/src/l0_ha_physical_multisig.rs` (310 lines).

**形式属性** (per `apeireth-sovereignty::physical_multisig` 真实定义):
- L0 HA 物理多签要求 ≥2 签名 + ≥2 distinct kinds + ≥1 witness
- 任意 1 项不满足即拒绝, 3 项全满足才批准
- 0 panic (任意符号化输入)

**6 个 Kani harness**:
1. `kani_verify_l0_ha_physical_multisig_all_conditions_met_approved` — 全满足 → Approved
2. `kani_verify_l0_ha_physical_multisig_insufficient_signatures_pending` — 签名数 < required → PendingSignatures
3. `kani_verify_l0_ha_physical_multisig_single_kind_rejected` — kind 数 < required → Rejected (reason=2)
4. `kani_verify_l0_ha_physical_multisig_no_witness_rejected` — witness=0 → Rejected (reason=3)
5. `kani_verify_l0_ha_physical_multisig_minimum_constants` — 编译期常量 (2/2/1)
6. `kani_verify_l0_ha_physical_multisig_never_panics` — 任意符号化输入 0 panic

**10 个 unit test** (cargo test 跑, 不需 kani 工具链):
1. `minimum_constants_correct`
2. `approved_when_all_conditions_met`
3. `approved_at_exact_minimum` (边界 = 2/2/1)
4. `pending_when_signatures_insufficient`
5. `rejected_when_single_kind`
6. `rejected_when_no_witness`
7. `priority_signatures_over_kinds` (签名优先于 kinds 判定)
8. `priority_kinds_over_witness` (kinds 优先于 witness 判定)
9. `all_5_harness_functions_visible` (函数指针可见性)
10. `r149_l0_ha_physical_multisig_deliverables` (R149 P0 #5 完成定义)

**3 不可变脊柱之一** (per R148 撤销 LOCKED 后保留): Self-Disable / L0 HA / 13-key verdict cache. 本模块只验证 L0 HA 物理多签的**形式属性**, 0 触碰 production code.

---

## 1. 跑法验证

```bash
cd Apeireth-rust

# 单 crate 验证
cargo test -p apeireth-tool-fetch --lib     # 44/44
cargo test -p apeireth-skills --lib         # 188/188 (12 new)
cargo test -p apeireth-runtime --lib        # 14/14 (4 new)
cargo test -p apeireth-graph --lib          # 88/88 (8 new)
cargo test -p apeireth-formal --lib         # 253/253 (10 new)

# 整体编译
cargo check --workspace                     # 0 errors (R148 fix 后 + 2 apeireth-memory pre-existing bug)
```

**pre-existing test bugs 同时修复**:
- `apeireth-memory/src/dailynote/enhanced.rs:81` — 测试块 `use super::*;` 已导入, 不需 `super::mcp::` 前缀
- `apeireth-memory/src/lightmemo/adapter.rs:130` — `tempfile` crate 缺 dev-dep → 加 `tempfile = "3"`

---

## 2. 借鉴 ID 完整列表 (R149)

| ID | 来源 | 用处 |
|---|---|---|
| `R149-FETCH-BORROW-vcp-7-plugins-merge` | VCP 7 fetch/search plugin | `apeireth-tool-fetch` |
| `R149-SKILLS-BORROW-anthropic-skills-spec-2025-10` | Anthropic Skills 协议 | `apeireth-skills::anthropic_skills` |
| `R149-RUNTIME-BORROW-minimax-api-2026-08` | MiniMax API OpenAI 协议 | `apeireth-runtime::LlmWorker` |
| `R149-GRAPH-BORROW-langgraph-memory-saver-2025-09` | LangGraph MemorySaver | `apeireth-graph::ThreadCheckpointStore` |
| `R149-KANI-BORROW-physical-multisig-formal-2026-08` | sovereignty::physical_multisig | `apeireth-formal::l0_ha_physical_multisig` |

---

## 3. 文档交叉引用

- README 顶部 banner: `> **R149 (2026-08-13)**: ...`
- `docs/r149/r149-p0-five-modules.md` (本文件)
- `docs/research/r149-github-survey.md` (P1/P2 调研报告)
- `crates/apeireth-tool-fetch/README.md` (新)
- `crates/apeireth-skills/README.md` (更新 anthropic_skills 模块)
- `crates/apeireth-runtime/README.md` (更新 LlmWorker)
- `crates/apeireth-graph/README.md` (更新 ThreadCheckpointStore)
- `crates/apeireth-formal/README.md` (更新 L0 HA 物理多签)
- `crates/apeireth-formal/KANI.md` (更新 harness 列表)

---

## 4. 下一周期候选 (R150+)

按优先级 (per `docs/research/r149-github-survey.md`):

1. `apeireth-vector` Qdrant protocol compat layer (战区 4 长期记忆升级)
2. `apeireth-pipeline` Temporal-style Activity (战区 2 pipeline 鲁棒性)
3. `apeireth-state` XState-style statechart (Multi-agent 状态机)
4. `apeireth-cron` migrate to `tokio-cron-scheduler` (取代当前 cron 实现)
5. `apeireth-council` session auto-capture (claude-mem pattern, council 记忆)
6. `apeireth-eval` SWE-bench style tasks (战区 1 eval 升级)
7. `apeireth-test` add `proptest` (property-based testing)

**TUI 接入**: 待主人拍板 (后续 R 周期, 不在本 R149 范围).
