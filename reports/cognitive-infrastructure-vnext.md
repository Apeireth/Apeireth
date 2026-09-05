# Apeireth Cognitive Infrastructure vNext Production Closure Report

日期：2026-09-05  
工作分支：`feature/cognitive-infrastructure-vnext`  
基准合并点：基于 `upstream/main`（PR #7 微内核微内核解耦合并后）最新提交重构闭环。

---

## 1. 交付物与状态全景分类法 (6-State Taxonomy)

本报告严格区分工程落地六层状态，绝不伪造未经验证的闭环：

- **IMPLEMENTED**：Canonical crate 具备真实 Rust 类型、算法逻辑、数据契约与单元测试。
- **PRODUCTION_WIRED**：从系统的组装入口（Composition Root / CLI / RuntimeAssembly）能真实路由到该模块，无 fake stub 或死代码断层。
- **VERTICAL_TESTED**：具备真实端到端或垂直调用链验证，不通过 mock 内部状态伪造成功。
- **CI_VERIFIED**：远端 CI 已对目标 commit 运行并通过完整构建与测试流水线（注：本地通过但尚未 push 至远端时，真实标记为“本地验证通过，远端 commit 待验证”）。
- **DEFERRED**：经过系统架构评审，由于依赖外部未定资产（模型权重、跨进程执行器）而诚实延后，契约与接口已封存，绝不伪装为已就绪。
- **NOT_IMPLEMENTED**：当前架构路线明确不做或未启动。

---

## 2. 核心基础设施落地状态矩阵

| 核心组件 / 功能点 | 状态 | 核心代码路径 | 真实生产调用链 | 垂直测试用例 / 文件 | CI / 本地验证状态 | 延后 / 未实现理由 |
|---|---|---|---|---|---|---|
| **CLI Event Sink 扇出 (Blocker A)** | VERTICAL_TESTED | `crates/adapters/cli/src/lib.rs`<br>`crates/engine/runtime/src/canonical/runtime.rs` | `cli::main` → `Runtime::builder()` → `runtime.add_event_sink(terminal)` + `runtime.add_event_sink(guard_observer)` | `test_t1_cli_event_sink_fanout`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **Guard 数据集脱敏与受控分类法 (Blocker B)** | VERTICAL_TESTED | `crates/engine/guard/src/dataset.rs`<br>`crates/engine/runtime-assembly/src/canonical/guard_observer.rs`<br>`scripts/guard-dataset-export.py` | `RuntimeEvent::TurnFailed` → `GuardExecutionOutcome::from_failure_hint` → `recorder.record_outcome` (格式 `guard-dataset-v2`) | `test_t2_dataset_privacy_safe_taxonomy`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **存储级 Scoped Memory 查询 (Blocker C)** | VERTICAL_TESTED | `crates/engine/memory/src/scope.rs`<br>`crates/engine/memory/src/backend/sqlite.rs`<br>`crates/engine/memory/src/migrations.rs` (V10) | `MemoryCoordinator::recall` → `ScopedMemoryBackend::query_candidates` → SQL `WHERE ({scope_sql})` (带索引) | `test_t3_cross_session_global_scope`<br>`test_t4_project_scope_isolation`<br>`test_t5_persona_scope_isolation`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **Legacy Episode 窄化保护 (Blocker C)** | VERTICAL_TESTED | `crates/engine/memory/src/backend/sqlite.rs`<br>`crates/engine/memory/src/lib.rs` | `ScopedMemoryBackend` SQL: `(m.metadata_json IS NULL AND e.session_id = ?)`，绝不泄露给 Global/Project | `test_t6_legacy_episode_fail_narrow`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **真实生产 Hybrid Recall (Blocker D)** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs`<br>`crates/engine/memory/src/retrieval_pipeline.rs`<br>`crates/engine/runtime-assembly/src/canonical/production.rs` | `MemoryCoordinator::recall` → `collect_candidates` (4 层) → `Bm25LexicalCandidateSource` → `HybridRetrievalPipeline::retrieve_with_status` | `test_t7_production_coordinator_hybrid_recall`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **Embedding 缺失时真实降级 (Blocker D)** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs`<br>`crates/engine/memory/src/retrieval_pipeline.rs` | `MemoryCoordinator::recall` (无 `embedding_provider`) → pure BM25 lexical fallback，`used_lexical_fallback: true`，`semantic: 0.0` | `test_t8_missing_embedding_truthful_lexical_fallback`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **语义与词法融合检索 (Blocker D)** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs`<br>`crates/engine/memory/src/retrieval_pipeline.rs` | `MemoryCoordinator::with_embedding_provider` → `StaticVectorCandidateSource` + `Bm25LexicalCandidateSource` → `retrieve_with_status` | `test_t9_semantic_retrieval_with_fake_embedding_provider`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **遗忘与混合索引联动** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs`<br>`crates/engine/memory/src/memory_governance.rs` | `coordinator.forget_episode` → Ring buffer 清除 + `episode_governance.status = 'forgotten'` → Recall 过滤 | `test_t10_forget_and_hybrid_index_dynamic_invalidation`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **内容变更更新 Hash 并失效向量** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs`<br>`crates/engine/memory/src/backend/sqlite.rs` | `coordinator.update_episode_content` → 重算 sha256 `content_hash` → 清除 `"vector"` 缓存字段 | `test_t11_content_update_invalidates_vector`<br>(`cognitive_convergence_vertical.rs`) | 本地通过；远端待验证 | - |
| **关闭世界上下文组装** | VERTICAL_TESTED | `crates/engine/memory/src/context_compiler.rs` | `ClosedWorldContextCompiler::compile` → 生成 `<governed_memory>` 块，移除一切 legacy 前缀 | `test_closed_world_memory_contract`<br>(`convergence_production_integration.rs`) | 本地通过；远端待验证 | - |
| **Guard Action ID 关联** | VERTICAL_TESTED | `crates/engine/guard/src/dataset.rs`<br>`crates/engine/runtime-assembly/src/canonical/guard_observer.rs` | `(trace_id, action_id)` 精准关联 tool_call 与 capability completion，写入 `guard.jsonl` | `test_guard_action_id_and_trace_boundary`<br>(`convergence_production_integration.rs`) | 本地通过；远端待验证 | - |
| **时间戳毫秒/秒级契约** | VERTICAL_TESTED | `crates/engine/memory/src/coordinator.rs` | writeback `timestamp_ms / 1000` 持久化，recall 还原为 `timestamp_ms`，半衰期指数衰减 | `test_timestamp_ms_to_s_contract`<br>(`convergence_production_integration.rs`) | 本地通过；远端待验证 | - |
| **Context Compaction** | **DEFERRED** | `crates/engine/memory/src/context_window.rs` | `ContextWindowManager` 仅支持 provider-facing 投影生成 | - | - | 真实长会话 transcript 压缩算法依赖外部语义 LLM 总结服务，不应在离线无模型下伪造；保证原始 transcript 不被破坏 |
| **Durable Persona SQLite Store** | **DEFERRED** | `crates/engine/memory/src/scope.rs` | 目前提供 `InMemoryPersonaProfileStore` 具备 revision-checked delta 合并语义 | - | - | 独立 persona 表的持久化迁移评估中；目前优先保证 6 历史流和 episode 统一存储稳定性 |
| **Automated Model Extraction** | **DEFERRED** | `crates/engine/memory/src/extraction.rs` | 提供 `RuleMemoryExtractor` 作为离线基于规则的兜底提取器 | - | - | 真正的模型语义记忆提取依赖外部大语言模型能力，未装配外部推理提供商前不伪称为生产已就绪 |
| **Trained ONNX Classifier** | **DEFERRED** | `crates/engine/guard/src/classifier.rs` | `ChainRiskClassifier` 接口就绪，默认 `NoClassifier`，支持安全启发式评估 | - | - | 真实 ONNX 权重文件与 Runtime 运行时绑定未进入本次微内核收口范畴，确定性 Guard 已足以提供 fail-closed 安全 |
| **Compensating Capability Execution** | **DEFERRED** | `crates/engine/guard/src/lib.rs` | `EnforcementDirective::Compensate` 枚举与数据契约已定义 | - | - | 自动回滚事务补偿器需要针对每种 capability（如文件修改、网络调用）编写幂等逆操作，本轮不盲目扩大范围 |

---

## 3. 四大核心收口点技术细节说明

### 3.1 Blocker A: CLI Event Sink 扇出
- **问题根因**：原 CLI 组装流程中直接使用覆盖式 `set_event_sink`，导致终端交互 sink 与 Guard 数据集记录 sink 相互覆盖。
- **解决实现**：
  - 在 `crates/adapters/cli/src/lib.rs` 中使用 `runtime.add_event_sink(...)` 进行增量注册。
  - 核心微内核 `Runtime` 提供加法式注册与内部 `CompositeRuntimeEventSink` 广播。
  - 垂直测试 `test_t1_cli_event_sink_fanout` 证明终端 sink 与 observer sink 接收到完全一致的事件流，事件类型与顺序严格相同。

### 3.2 Blocker B: Guard Dataset 隐私防护与受控分类法
- **问题根因**：原系统在 turn 失败或 capability 异常时，直接将未经脱敏的原始 error 字符串写入 dataset 作为 execution outcome，存在凭证、本地敏感路径泄漏风险。
- **解决实现**：
  - 在 `crates/engine/guard/src/dataset.rs` 中定义 `GuardExecutionOutcome` 受控枚举（`success`, `capability_failure`, `runtime_failure`, `provider_failure`, `governance_blocked`, `timeout`, `cancelled`）。
  - 数据集版本提升至 `guard-dataset-v2`。
  - `GuardDatasetObserver` 强制经由 `from_failure_hint` 映射为枚举安全字符串，绝不记录未经清洗的原始报错。
  - `scripts/guard-dataset-export.py` 增加前缀扫描与凭证清洗逻辑。
  - 垂直测试 `test_t2_dataset_privacy_safe_taxonomy` 验证包含 Bearer token 与 admin 私钥路径的失败 turn，在落盘的 JSONL 中 100% 不存在敏感字符串，只记录 `provider_failure`。

### 3.3 Blocker C: 存储级 Scoped Memory 查询与 Legacy 窄化保护
- **问题根因**：跨 session 查询（如 Global 范围）在底层存储层缺乏过滤，且未对历史无 metadata 的 episode 设防，存在会话间记忆穿透隐患。
- **解决实现**：
  - 移除 `impl Default for MemoryScope`，强制所有写入与查询必须具备显式 scope。
  - 引入 `ScopedMemoryBackend` trait 与 `MemoryCandidateQuery`，定义 storage-level 过滤标准。
  - 在 `SqliteBackend` 与 `SqliteMemoryStore` 中实现带有索引的 SQL 过滤，新增 V10 迁移为 `metadata_json` 创建 scope 与 layer 检索索引。
  - 历史无 metadata 记录严格匹配：`((json_extract(...) = 'session' AND ...) OR (m.metadata_json IS NULL AND e.session_id = ?))`，严格窄化（fail-narrow）至源 session，绝不泄露至 Global 或其它 session。
  - 垂直测试 `test_t3_cross_session_global_scope`、`test_t4_project_scope_isolation`、`test_t5_persona_scope_isolation`、`test_t6_legacy_episode_fail_narrow` 验证隔离与全局可见性的真实性。

### 3.4 Blocker D: 生产 MemoryCoordinator Hybrid Recall 真实性
- **问题根因**：原协调器直接返回硬编码 1.0 得分，未真实接入 `HybridRetrievalPipeline` 与 BM25 词法引擎。
- **解决实现**：
  - `MemoryCoordinator` 在 `collect_candidates` 中聚合 Working Memory（环形缓冲区）、Episodic Memory（存储层）、Semantic Memory 与 Relational Memory。
  - 接入 `Bm25LexicalCandidateSource`，基于真实分词与 `Bm25Index` 计算 Okapi BM25 权重。
  - 未配置 embedding provider 时：
    - 纯同步计算，零 tokio/async 运行时阻塞开销；
    - `retrieval_status.used_lexical_fallback == true`；
    - `score_components.semantic == 0.0`，绝不凭空捏造 1.0 或随机分；
  - 配置 embedding provider 时：
    - 向量候选与 BM25 词法候选联合归一化加权融合。
  - 任何调用 `update_episode_content` 会自动使旧向量缓存失效（从 JSON 中剔除），并重新计算内容 sha256 hash。
  - 垂直测试 `test_t7` 至 `test_t11` 全面覆盖。

---

## 4. 垂直测试验证证据矩阵

测试文件：`crates/engine/runtime-assembly/tests/cognitive_convergence_vertical.rs`  
运行命令：`cargo test -p apeireth-runtime-assembly --test cognitive_convergence_vertical --locked`

| 测试用例编号 | 测试名称 | 验证场景与断言标准 | 结果 |
|---|---|---|---|
| **T1** | `test_t1_cli_event_sink_fanout` | 验证多 sink 注册不相互覆盖，CLI terminal sink 与 recorder sink 均收到事件且一致 | **PASSED** (0.01s) |
| **T2** | `test_t2_dataset_privacy_safe_taxonomy` | 敏感凭证 (Bearer token) 与路径报错被捕获，落盘 JSONL 无敏感字串，outcome 为安全受控枚举 | **PASSED** (0.01s) |
| **T3** | `test_t3_cross_session_global_scope` | Session A 写入 Global 记忆，Session B 可召回；未指定 Global 时不可见 | **PASSED** (0.01s) |
| **T4** | `test_t4_project_scope_isolation` | Project Alpha 写入记忆，Project Beta 检索为空，Project Alpha 检索命中 | **PASSED** (0.01s) |
| **T5** | `test_t5_persona_scope_isolation` | User 1 + Persona X 写入，不同 Persona 或不同 User 检索为空，匹配时命中 | **PASSED** (0.01s) |
| **T6** | `test_t6_legacy_episode_fail_narrow` | 无 metadata 的历史 episode 仅源 session 可见，跨 session 或全局查询严格不可见 | **PASSED** (0.01s) |
| **T7** | `test_t7_production_coordinator_hybrid_recall` | 验证生产 `MemoryCoordinator` 执行 BM25 算分，返回合法的 `score_components` 与 `retrieval_status` | **PASSED** (0.01s) |
| **T8** | `test_t8_missing_embedding_truthful_lexical_fallback` | 无 embedding provider 时真实报告 `used_lexical_fallback: true`，semantic 得分为 0.0 | **PASSED** (0.01s) |
| **T9** | `test_t9_semantic_retrieval_with_fake_embedding_provider` | 配置确定性向量提供者时，向量与 BM25 加权融合，`used_lexical_fallback: false` | **PASSED** (0.01s) |
| **T10** | `test_t10_forget_and_hybrid_index_dynamic_invalidation` | 写入后 forget，后续混合检索不再召回，工作记忆环形缓冲区同步清除 | **PASSED** (0.01s) |
| **T11** | `test_t11_content_update_invalidates_vector` | 内容更新后重新计算 content_hash，旧缓存向量被剔除，防止陈旧向量误匹配 | **PASSED** (0.01s) |

---

## 5. 本地全门禁校验汇总 (Local Gates Verification)

- **Rust 编译与代码格式**：
  - `cargo fmt --all -- --check`：通过（0 格式违规）。
  - `cargo check --workspace --all-targets --locked`：通过（0 错误）。
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`：通过（0 warning）。
- **自动化测试**：
  - `cargo test -p apeireth-memory --lib --locked`：663/663 passed (0 failed).
  - `cargo test -p apeireth-guard --test guard_tests --locked`：7/7 passed (0 failed).
  - `cargo test -p apeireth-runtime-assembly --locked`：全套 12 个集成测试用例 + 34 个单元测试全绿 (100% passed).
  - `cargo test -p apeireth-runtime-assembly --test cognitive_convergence_vertical --locked`：11/11 passed (0 failed).
- **架构依赖检查与治理**：
  - `python scripts/check_no_legacy_deps.py`：通过（没有引入任何禁用或 legacy 模块）。
  - `cargo deny check`：通过（Advisories / Bans / Licenses / Sources 全部合规）。
  - `cargo audit --no-fetch`：通过（无已确认漏洞）。
  - `cargo tree -p apeireth-runtime --edges normal --depth 2`：`apeireth-runtime` 保持纯粹微内核，零 `apeireth-memory`、零 `apeireth-guard`、零 `rusqlite`、零 `onnxruntime` 依赖。
- **前端与界面层 (`apeireth-ui`)**：
  - `pnpm test`：7/7 passed.
  - `pnpm check`：0 errors.
  - `pnpm build`：构建成功（0 errors）。
