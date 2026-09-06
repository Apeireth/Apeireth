# Apeireth Cognitive Infrastructure vNext — Production Closure Report

日期：2026-09-06
工作分支：`feature/cognitive-infrastructure-vnext`
本轮基准：`origin/main` / `upstream/main` = `7647d2c91d55901aeae7202f3842b65233bd053c`。分支已 rebase 到该提交；当前已推送提交：`eee68ad005444102acdfb807a2388a2bcb63812e`。

## 1. 状态定义

| 状态 | 含义 |
|---|---|
| **IMPLEMENTED** | 有真实 canonical Rust 类型、算法、数据契约和测试。 |
| **PRODUCTION_WIRED** | Composition Root / CLI / RuntimeAssembly 可真实路由到该能力。 |
| **VERTICAL_TESTED** | 有跨边界或端到端调用链证据，不以内部 fake 状态冒充生产闭环。 |
| **CI_VERIFIED** | 远端 CI 已针对最终推送 SHA 运行并通过；本地通过不等于此状态。 |
| **DEFERRED** | 契约保留，但依赖未批准的模型权重、远程能力或执行器，明确延后。 |
| **NOT_IMPLEMENTED** | 当前路线未提供该能力。 |

## 2. 最终状态矩阵

`CI_VERIFIED` 在最终 SHA 有 commit-specific GitHub Actions run 前保持 `NO`；不会把本地绿灯写成远端绿灯。

| 能力 | IMPLEMENTED | PRODUCTION_WIRED | VERTICAL_TESTED | CI_VERIFIED | 结论 / 证据 |
|---|---:|---:|---:|---:|---|
| CLI runtime event sink additive fan-out | YES | YES | YES | NO | CLI 真实 bootstrap 同时安装 Guard dataset observer 与 trace/audit observer；`canonical_cli_bootstrap.rs`。 |
| Guard dataset privacy + safe lifecycle taxonomy | YES | YES | YES | NO | `guard-dataset-v2`；失败只落盘 `success`、`provider_failure`、`runtime_failure` 等受控标签，不落 raw error。 |
| Dataset exporter / validation gate | YES | NO（script gate） | NO | NO | `scripts/guard-dataset-export.py` 执行 forbidden-key、长度、credential、URL/path、关联性检查。 |
| MemoryScope contract | YES | YES | YES | NO | Global/User/Project/Persona/Session；`MemoryCandidateQuery` + `ScopedMemoryBackend`。 |
| Scope persistence + migration | YES | YES | YES | NO | `episode_memory_metadata` sidecar；scope/layer expression indexes；memory migration V9/V10。 |
| Cross-session / project / persona storage query | YES | YES | YES | NO | `MemoryCoordinator` 直接调用存储级 scope query；T3–T5 + production `SqliteBackend` T12。 |
| Legacy no-metadata fail-narrow | YES | YES | YES | NO | 仅 `e.session_id` 匹配的 legacy row 可见；不会因请求 Global 而外泄；T6 + T12。 |
| Canonical hybrid recall pipeline | YES | YES | YES | NO | `MemoryCoordinator` → candidate collection → `HybridRetrievalPipeline`。 |
| Real BM25 lexical retrieval | YES | YES | YES | NO | `Bm25LexicalCandidateSource` 使用 canonical `Bm25Index` / Okapi 计算；T7/T8。 |
| Embedding fallback truthfulness | YES | YES | YES | NO | 未配置 provider 时 `used_lexical_fallback=true` 且 `semantic=0.0`；T8。 |
| Semantic + lexical fusion | YES | YES | YES | NO | 测试注入 deterministic fake embedding，仅证明融合路径，不宣称远程 embedding 质量；T9。 |
| Forget / vector invalidation | YES | YES | YES | NO | forget 过滤并清空 working ring；内容变更重算 hash、删除旧 vector；T10/T11。 |
| Production memory module assembly | YES | YES | YES | NO | CLI 注入 canonical `SqliteBackend` + `MemoryCoordinator`；写入 → provider prompt → 重启 → forget；`cognitive_vnext_production.rs`。 |
| ContextWindowManager | YES | NO（仅基础设施） | NO | NO | 可生成 bounded projection，但生产 context compaction 明确 DEFERRED；已有 unit coverage。 |
| Durable persona profile store | NO | NO | NO | NO | 当前只有 revision-checked `InMemoryPersonaProfileStore`；SQLite durable persona 表延后。 |
| Memory extraction lifecycle | YES | NO | NO | NO | `MemoryExtractor` / `RuleMemoryExtractor` 可用作确定性离线构件；模型抽取未接入生产写回，DEFERRED。 |
| Guard classifier / ML status | YES（NoClassifier + interface） | YES（确定性默认） | NO | NO | 默认 `NoClassifier`，availability/model 状态诚实；远程/训练 ONNX classifier DEFERRED。 |
| Enforcement directive | YES | NO | NO | NO | `EnforcementDirective` 可由治理决策派生；host-side containment policy 不是本轮完成项。 |
| Compensation dataset lifecycle | YES | NO（API only） | NO | NO | approval/execution/compensation record v2 与 safe normalization 已实现。 |
| Production compensation execution | NO | NO | NO | NO | 没有针对各 capability 的幂等逆操作执行器；DEFERRED / NOT_IMPLEMENTED。 |

## 3. 四个 Blocker 的收口说明

### Blocker A — CLI sink overwrite

CLI 现在通过 `Runtime::add_event_sink` 增量安装 observer，不覆盖既有 sink。真实 CLI 测试使用 mock vendor、临时 SQLite、Guard dataset 文件和 CLI data directory，执行 canonical turn 后同时验证：

- `guard-dataset.jsonl` 有 `outcome` / `success`；
- `traces.jsonl` 有 provider trace；
- `daemon-audit.jsonl` 有 `chat.turn.completed`。

运行时通用 fan-out 也由 `test_t1_cli_event_sink_fanout` 覆盖；真实 CLI 组合证据为 `the_cli_bootstrap_fans_out_guard_dataset_and_trace_audit_observation`。

### Blocker B — raw error 不得进入 dataset

`GuardExecutionOutcome` 的允许值为：`success`、`capability_failure`、`provider_failure`、`timeout`、`cancelled`、`approval_denied`、`governance_denied`、`runtime_failure`、`internal_failure`。approval 仅允许 `approved`、`rejected`、`cancelled`、`expired`、`unknown`。

`GuardDatasetObserver` 对 `TurnFailed` 只调用 `from_failure_hint`，不序列化 error 原文；approval、execution、compensation 也都经过 normalization。exporter 额外拒绝 prompt/secret/token/password/credential/reasoning/COT/arguments 字段以及凭证、URL、路径和超长字符串。

### Blocker C — scope query 与 legacy 边界

`MemoryScope` 包含 Global、User、Project、Persona（精确 `(user_id, persona_id)`）和 Session。生产 CLI 使用 `SqliteBackend`，其 SQL 在存储层依据 `MemoryCandidateQuery.visible_scopes` 过滤，并通过 `episode_memory_metadata` 的 scope/layer 索引支持持久化查询。

无 metadata 的旧 episode 只允许 `m.metadata_json IS NULL AND e.session_id = ?` 的 source-session fallback。它不会被解释成 Global、Project 或其它 session。T12 直接使用生产 `SqliteBackend`、真实迁移和真实 coordinator，补证 Global/User/Project/Persona/Session 与 legacy boundary。

### Blocker D — canonical hybrid recall

生产链路是：

`MemoryCoordinator::recall` → `collect_candidates` → `Bm25LexicalCandidateSource`（canonical `Bm25Index`）→ 可选 `StaticVectorCandidateSource` → `HybridRetrievalPipeline::retrieve_with_status` → centralized ranking / budget。

无 embedding provider 时不伪造 semantic score；有 provider 时才加入 vector candidate。T7 证明 production coordinator 使用 BM25，T8 证明诚实 lexical fallback，T9 用 deterministic fake embedding 证明 semantic + lexical 融合，T11 证明内容变更会删除陈旧 vector metadata。

## 4. 测试证据矩阵

### RuntimeAssembly vertical suite

命令：`cargo test -p apeireth-runtime-assembly --test cognitive_convergence_vertical --locked`

| 编号 | 测试 | 结果 |
|---|---|---|
| T1 | `test_t1_cli_event_sink_fanout` | PASSED |
| T2 | `test_t2_dataset_privacy_safe_taxonomy`（含 direct TurnFailed privacy assertion） | PASSED |
| T3 | `test_t3_cross_session_global_scope` | PASSED |
| T4 | `test_t4_project_scope_isolation` | PASSED |
| T5 | `test_t5_persona_scope_isolation` | PASSED |
| T6 | `test_t6_legacy_episode_fail_narrow` | PASSED |
| T7 | `test_t7_production_coordinator_hybrid_recall` | PASSED |
| T8 | `test_t8_missing_embedding_truthful_lexical_fallback` | PASSED |
| T9 | `test_t9_semantic_retrieval_with_fake_embedding_provider` | PASSED |
| T10 | `test_t10_forget_and_hybrid_index_dynamic_invalidation` | PASSED |
| T11 | `test_t11_content_update_invalidates_vector` | PASSED |
| T12 | `test_t12_production_sqlite_backend_scope_and_legacy_boundary` | PASSED |

生产组装：`cognitive_vnext_production.rs` 2/2 PASSED。CLI 真实观察组合：`canonical_cli_bootstrap.rs` 新增测试 1/1 PASSED。

### Context / extraction / persona trust boundary

- `ContextWindowManager` 是 IMPLEMENTED 基础设施，但当前没有进入 provider 生产路径；其摘要 projection 不能被描述为已完成的长会话压缩，也不作为新的 trust-boundary 证明。
- `MemoryExtractor` trait 和 `RuleMemoryExtractor` 是 deterministic contract/离线 fallback；没有未经批准的 remote LLM call、credential 读取或模型抽取生产写回。
- persona profile/delta/revision 语义与 in-memory store 存在；durable production persona store 未完成。

## 5. 本地验证记录

本轮实际执行结果如下；任何本地通过都不替代最终 SHA 的远端验证：

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python scripts/check_no_legacy_deps.py
cargo deny check
cargo audit
```

- `cargo fmt --all -- --check`：PASSED；
- `cargo check --workspace --all-targets --locked`：PASSED；
- `cargo test --workspace --all-targets --locked`：PASSED，退出码 0；
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：PASSED，退出码 0；
- `cargo test -p apeireth-memory --lib --locked`：663/663 PASSED；
- `cargo test -p apeireth-guard --test guard_tests --locked`：7/7 PASSED；
- runtime-assembly vertical：12/12 PASSED；production assembly：2/2 PASSED；CLI 真实观察组合：1/1 PASSED；
- `python scripts/check_no_legacy_deps.py`：PASSED，path dependency violations 0，transitive legacy packages 0；
- `cargo deny check`：PASSED（advisories / bans / licenses / sources）；
- `cargo audit` 0.22.2：PASSED，0 个阻断漏洞；1 个 allowed warning（`chacha20 0.10.1` yanked）；
- `git diff --check` 与 exporter Python AST parse：PASSED；
- 前端 `pnpm test`：7/7 PASSED；`pnpm check`：0 errors / 5 个既有 warning；`pnpm build`：PASSED。

补充验证：

- `git diff --check`；
- exporter Python AST parse；
- `cargo tree -p apeireth-runtime --edges normal --depth 2`，确认 runtime 不依赖 memory、guard、storage、runtime-assembly、rusqlite、onnxruntime 或 lightgbm；
- `frontend/companion-desktop`：`pnpm test`、`pnpm check`、`pnpm build`。

## 6. 远端 CI 与 PR readiness

远端 CI 必须按最终推送 SHA 查询，而不是只看分支最新状态。至少记录 Ubuntu / Windows / macOS、fmt、clippy、nextest / workspace tests、docs、audit、deny、secret scan、desktop checks 和 coverage workflow 的实际状态；仓库未配置的 workflow 标记为 `NOT_PRESENT`，不能虚报为 passed。

实际结果：`git ls-remote origin refs/heads/feature/cognitive-infrastructure-vnext` 已指向 `eee68ad005444102acdfb807a2388a2bcb63812e`；GitHub Actions 按 `feature/cognitive-infrastructure-vnext` 过滤为 0 个 run，因此 commit-specific CI = **NOT_VERIFIED / NO RUN**。本次未创建 PR（用户只要求推送分支），故 PR 触发的门禁不会自动出现。只要仍有 `PENDING`、`FAILURE`、`NOT_VERIFIED`，PR readiness 就是 **NO**；全部要求项针对同一最终 SHA 通过后才是 **YES**。

## 7. Deferred / explicitly not implemented

1. remote/trained Guard classifier 与模型权重加载；
2. durable SQLite persona profile store；
3. remote LLM memory extraction 与自动生产写回；
4. production context compaction / summarization，以及任何新的 trust-boundary 结论；
5. capability-specific idempotent compensation execution；
6. 未批准的外部 memory provider bridge 和平台绑定 UI 实现。

## 8. DoD

| DoD 项 | 状态 |
|---|---|
| Rebase 到精确最新 main | YES — `7647d2c91d55901aeae7202f3842b65233bd053c` |
| CLI fan-out / dataset privacy / scope / hybrid recall 真实代码与垂直证据 | YES |
| Persona / extraction / context / ML / compensation 状态诚实拆分 | YES |
| Rust + dependency + frontend 本地门禁 | YES — all required local commands passed |
| Final branch force-with-lease push | YES — remote branch = `eee68ad005444102acdfb807a2388a2bcb63812e` |
| Final-SHA remote CI | NO — no commit-specific run exists for this feature branch push |
| PR ready | NO — until final push and commit-specific CI verification |

证据文件：

- `crates/adapters/cli/tests/canonical_cli_bootstrap.rs`
- `crates/engine/runtime-assembly/tests/cognitive_convergence_vertical.rs`
- `crates/engine/runtime-assembly/tests/cognitive_vnext_production.rs`
- `scripts/guard-dataset-export.py`
- `reports/yanshu-memory-donor-mapping.md`
