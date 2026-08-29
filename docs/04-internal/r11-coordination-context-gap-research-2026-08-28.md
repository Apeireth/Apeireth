# R11 协调 + 上下文 gap 真调研 (1.0 vs 2.0) — 2026-08-28

> **作者**: sub-agent R11-CoordinationContext (Round 11 派单). **关系**: 跟 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §2.7+§3.1#13/14/15 + `apeireth-true-understanding-2026-08-28.md` + `v2-reference-handbook-2026-08-28.md` 互补.
> **必读**: 全部 1.0 v1 source 14 file (legacy/donor/apeireth-companion/src/) + v2 workspace grep (crate/**).

```
[Document-Meta] Version 1.0 / 2026-08-28 / 🟢 活跃 / sub-agent R11
```

---

## 0. 关键发现 (per O-5 0 装诚实)

1. **v2 9 项 0 接**: 除 emergence organ + organ world_model 子集 (oracle trait 1:1 移植) + 6 crate `organ_kani_proofs` (R177 已装) 外, **其余 6+ 项 v2 workspace 完全 0 真实施** (grep: spill/milestone/experiment_field/bridge_kani_proofs/pentest/ProgressiveCatalog/OneRingLedger/ContinuationStore/assemble 0 hit).
2. **重复实现警告**: v1 `context.rs` (L141-451) + `context_rot.rs` (L140-174) **同一 rot_score 两 file 各实现一遍** — v2 真实施前必先融合.
3. **概念 collision**: 主代理真账把 `hello.rs` 列 "启动/装配" — 实际 v1 hello.rs 是 **Windows Hello 生物识别 (NGC 凭据探测)**, 跟协调+上下文 gap 主题 mismatch, 真调研按 v1 真账画像.
4. **主代理真账 §1.8 部分不准确**: `oracle/oracle_adapters` 标 ❌ — 实际 organ world_model trait 已 1:1 移植; `proactive` 标 ❌ — 实际 E7 emergence organ (含 8 重 gate) 已真实施; `bridge_kani_proofs/organ_kani_proofs` 标 ❌ — 实际 6 crate organ_kani_proofs 已装 (R177). 本真账修订.

---

## 1. onering — 1.0 vs 2.0 + 单环协调

### 1.1 1.0 真账 (`legacy/donor/apeireth-companion/src/onering.rs` 346 行, REAL)
- **职责**: **OneRing 统一上下文账本** — 跨前端 (SSE/Web/Lark/Telegram/CLI/proactive) **同一 Agent 唯一时间线**, 每条发言留痕 `(seq, continuity_id, role, sender, frontend, content, ts)`, seq 单调自增确定性排序, 账本只保留最近 N 条 (默认 200, 可配).
- **VCP OneRing 对照吸收** (per `team-work-doc §8.4`): `messages(agentName, role, senderName, frontendSource, content, timestamp)` → `onering_messages(continuity_id, role, sender, frontend, content, ts)` (agentName→continuity 锚点); `pruneAgentMessages(maxRecords=100)` → prune 200. **不吸收**: VCP fuzzy diff / RawClientTimeline / 系统提示词占位符触发 (0 假装: 前端上下文回放走记忆注入管线).
- **数据结构**: 账本自有表 `onering_messages` (经 `store.conn()`, 同 continuity_link 建表模式) — **不污染 episodes 记忆管线** (0 装 PASS).
- **API**: `OneRingLedger::new(store, continuity)` / `record(role, sender, frontend, content)` / `record_as(continuity, ...)` 多锚点 / `recent(limit)` / `len()` / `with_max_records(n)`.
- **测试** (8 #[test]): `records_and_replays_in_order` / `cross_frontend_same_timeline` (核心: 4 前端归同一锚点) / `multi_anchor_isolated` / `rejects_invalid_role` / `rejects_empty_*` / `prunes_to_max_records` / `recent_limit_zero_is_empty` / `ledger_does_not_pollute_episodes`.

### 1.2 2.0 现状
- **❌ 0 真实施** (主代理真账准确). **已就位**: `crates/engine/memory/src/identity.rs` (IdentityCard continuity_id UNIQUE) + `continuity_link.rs` (continuity_id↔session_id link) + `episode.rs` (`EpisodeQuery::for_continuity`). X-Apeireth-Continuity HTTP 头雏形 (per master audit L231) 未装.

### 1.3 真实施路径
- **路径 A (推荐, 2 周)**: 新 crate `crates/engine/coordinate/src/onering.rs` (1:1 翻译 v1 346 行, 新 crate 0 触碰 LOCKED) → trait `OneRingLedger` + SqliteMemoryStore impl + seq 自增 + 多锚点 record + prune + cross_frontend 验证 + 与 identity_cards.continuity_id 对齐 (FK 不强, 文档约束). **物种化**: continuity_id = per-user, 账本天然支持不同用户不同时间线. **风险**: 5 重守门 + Cargo dep (`rusqlite` + `chrono` + `uuid`) 已就位.

---

## 2. oracle / oracle_adapters — 1.0 vs 2.0 + 预言/适配器

### 2.1 1.0 真账 (`oracle.rs` 553 行 + `oracle_adapters.rs` 1300+ 行, REAL)
- **oracle.rs 5 大机制**:
  1. **世界状态 + 情景引擎**: `WorldState` (entities + tick) + `ScenarioEngine` (inject + apply fn + simulate)
  2. **预测断言 + 校准**: `Forecast` (statement + probability + deadline_ms + resolved + brier + rev) + `ForecastRegistry` (episode 前缀 `forecast-`, Brier 自动入账 + 校准统计)
  3. **不确定性裁决**: `UncertaintyResolver` trait + `CalibratedResolver` 真实现 (BetaBinomial 后验 + Wilson 95% CI + Strength 分档 + 历史 0 → 0.5 均匀先验)
  4. **分支推演 + 决策**: `Branch { name, probability, value, events }` + `DecisionEngine` (期望值 Σ P×V, max_by 选优, 一层 expectimax-lite, MCTS 留口)
  5. **0 装诚实**: v1 LLM 裁决仍留 trait 口 (`UncertaintyResolver`), 真实现是校准数学 (0 LLM 依赖, 可测试)
- **oracle_adapters.rs 4 大机制**: `MarketAdapter` trait + `AdapterError` (RateLimited/Unreachable/Parse/Unsupported/Degraded, `degradable()`) + `MockAdapter`/`FallbackAdapter`/`AdapterRegistry` (Parse/Unsupported 直抛不编数) + `RawFetch`/`ReqwestRawFetch` 真 HTTP (10s timeout) + 旗舰适配器 (CoinGecko BTC/ETH/SOL/DOGE simple/price + MacroRates fiscaldata 美债 avg interest rate) + **TP25 时序预测 trait** (`TimeSeriesPredictor` + `NoopTimeSeriesPredictor` + `NaiveBaselinePredictor` MA+OLS + `ArimaPredictor` ARIMA(1,1,1) + 95% CI + `blend_predictions` 数字+LLM 文本融合). **API 关键**: `ForecastPipeline` (拉基线→登记→到期 resolve→Brier 入账).

### 2.2 2.0 现状
- **⚠️ partial** (主代理真账写 ❌ — 不准确): `crates/engine/organ/src/world_model.rs` L10-77 已 1:1 移植 — `ForecastRegistry` trait + `NoopForecastRegistry` (诚实验) + `CalibratedResolver` 改 `Option<Arc<dyn ForecastRegistry>>` (无 registry → status() 诚实验"无历史") + WorldState/Entity/Forecast 内部移植 (v2 organ 无 `apeireth-memory` 依赖, 不引新 dep) + `causal_world_model.rs` Brier 拒绝阈值 0.3. **❌ 未移植**: oracle_adapters 全套 (CoinGecko/MacroRates/ForecastPipeline/TimeSeriesPredictor) → engine crate 0 真实施; Mio 真账 §2.2 Brier 校准引用 `oracle.rs` 但 v2 真生产路径 trait 注入未完成.

### 2.3 真实施路径
- **路径 A (3 周分两阶段)**: 新 crate `crates/engine/oracle/src/` — Stage 1 (1 周) `registry.rs` SqliteMemoryStore 真实现 + `CalibratedResolver` 真生产路径 (换 organ Noop); Stage 2 (2 周) `adapters/` 1:1 翻译 v1 ~1300 行 (CoinGecko + MacroRates + Fallback + ForecastPipeline + TimeSeriesPredictor + blend). **物种化**: CalibrationStatus per-user (不同主人校准历史不同, 自然塑形); `DecisionEngine::choose` 期望值 Σ P×V, 主人 override 永远胜出.

---

## 3. context / context_rot — 1.0 vs 2.0 + context window/旋转

### 3.1 1.0 真账 (`context.rs` 914 行 + `context_rot.rs` 526 行, REAL)
- **context.rs 3 大机制**: (1) **ContextAssembler**: 有序块管线 + 总预算 (默认 6000 chars) + 核心块 (persona/状态) 永不截断 + 单块 cap + 非核心按"字符多者优先"贪心砍; (2) **TP16 Context Rot** (L141-451 — **重复实现警告**): rot_score = `w1·duplicate_ratio + w2·stale_ratio + w3·(1-relevance)` (默认 0.4/0.3/0.3, 启发式待 A/B 调权重) + `RotBlock` + `RotConfig` (now_ms + stale_threshold 默认 30 min + ngram_size 5 + duplicate_threshold 0.6 + trigger_threshold 0.6 + pinned + latest_user_message + min_chars 16) + `RotBreakdown` (total + duplicate_ratio + stale_ratio + irrelevance + relevance + duplicate_pairs + stale_block_ids + low_relevance_block_ids + eligible_block_count) + `ngrams` 词级 5-gram Jaccard + `keyword_overlap` alphanumeric + `compute_rot_score` (deterministic pure) + `should_compact` (strict >).
- **context_rot.rs 3 大机制**: (1) **三因子 rot_score** (L82-179 — 重复) + `repetition_factor` 行级去重/单行 6 字滑窗 + `query_tokens` ASCII 词+CJK char-bigram + `rot_breakdown` 确定性 0 LLM; (2) **Compactor trait + DeterministicCompactor** (L196-261) LLM 参与版留口 + 规则版 (rot_score ≥ 0.6 → 抽取式摘要 Replace 120 chars, 否则 Retain, 空 → Remove, 核心段永远 Retain); (3) **apply_ops + compact_then_budget** (L264-301) 段编辑原语 + rot 驱动选择性压缩优先 + ContextAssembler 预算截尾兜底.

### 3.2 2.0 现状
- **❌ 0 真实施** (grep `RotBlock`/`compute_rot_score`/`rot_score`/`context_rot` 0 hit). 0 真实施原因: canonical runtime A 块专注 OrganOrchestrator 完整化, prompt_assembler 未含 budgeted core 保护 + rot_score.

### 3.3 真实施路径
- **必先融合 context.rs + context_rot.rs** (重复实现警告, 1 天先合并, 22 个 v1 测试 1:1 翻译通过).
- **路径 A (3 周)**: Stage 1 (3 天融合) `crate::rot::{RotBlock, RotConfig, RotBreakdown, RotWeights, ngrams, jaccard, keyword_overlap, compute_rot_score, should_compact}` 单 module; Stage 2 (1 周) 新 crate `crates/engine/context/src/{assembler, rot, compactor}.rs`; Stage 3 (1 周) 接 `crates/engine/runtime/src/canonical/prompt_assembler.rs` 替换占位 budget 逻辑, rot 触发时调 `compact_then_budget`; Stage 4 (3 天) 接 12 slot cognitive module `cognitive.memory_recall`/`memory_writeback` 注入前 rot 评估. **物种化**: per-user `RotConfig::pinned_block_ids` (主人人设/核心价值/长期约定) + per-user `weights`. **0 装诚实**: rot_score 启发式待 A/B 调权重, 必须明示 (v1 L150-152 "不假装 rot_score 准确"). **核心保护**: persona/状态/F6 value_cases 永远 Retain (锁物种化哲学锚).

---

## 4. continuation / continuity / spill — 1.0 vs 2.0 + 对话连续性

### 4.1 1.0 真账 (`continuation.rs` 856 行 + `continuity.rs` 305 行 + `spill.rs` 178 行, REAL)
- **continuation.rs 双件**: (1) **续行快照**: `ContinuationSnapshot` (id, session_id, messages: Vec<Value> OpenAI 形状, pending_tool_call: Option<PendingToolCall>, saved_at_ms, turn) + `ContinuationStore` 目录 + 原子写 `tmp+rename` 崩溃安全 + `save`/`exists`/`load`/`consume` (load+delete 一次性) / `list`; (2) **TP16 段编辑原语**: `EditAction` (Retain/Remove/Replace{block_id, new_content}) + `EditorError` (EmptyBlockId/BlockNotFound/ConflictingActions/EmptyReplaceContent) + `SegmentEditor` BTreeMap 保插入序 + now_ms + `insert` no-op on dup + `retain`/`remove`/`replace` bump touched_ms + `touch` + `apply` 三段式 all-or-nothing (冲突预检 + dry-check + 提交) + `EditOutcome`.
- **continuity.rs 锚点 + 迁移**: `current_continuity_id()` env `APEIRETH_CONTINUITY_ID` → 缺省 `companion-main` (消灭散落 "me" 硬编码) + `normalize_continuity(raw, fallback)` HTTP header 兜底 + `ensure_identity` 并发 AlreadyExists 幂等 + `migrate_subject` episodes 复制前进 (append-only, `mig-{原id}` lineage, `INSERT OR IGNORE` 幂等) + onering_messages UPDATE 改键 + `MigrationReport`. **哲学锚 (team-work-doc §1.1.4)**: "不假装灵魂同一" — 迁移只搬记录并留审计痕迹, 新锚点是新载体.
- **spill.rs 工具结果溢出 (DeepSeek Harness spill 精神, Rust 重写)**: 工具输出 > `SPILL_THRESHOLD_CHARS` (默认 2000) → spill 到 `<root>/<session-安全名>/<随机前缀>-<安全名>`; **安全**: `safe_segment` (alphanumeric+`-`+`_`, 截 60 char) + `create_new` (wx: 已存在即失败, 防 symlink 植入) + `read_within_root` canonicalize 前缀检查 (防越权读). **0 装**: Windows 0700 权限位不生效, 依赖系统用户目录私有性 (同 DSH).

### 4.2 2.0 现状
- **⚠️ continuity 部分 OK + continuation/spill 0 真实施** (主代理真账写 ❌ — 部分不准确):
  - **continuity 已就位**: `crates/foundation/core/src/kernel/memory.rs:62` IdentityCard.continuity_id + `crates/engine/memory/src/{identity.rs (UNIQUE+tombstone+migration_history), continuity_link.rs (continuity_id↔session_id+resolve_continuity), episode.rs (EpisodeQuery::for_continuity)}` + `crates/engine/runtime/src/canonical/{approval.rs, execute.rs}` `FrozenTurnContinuation` (turn 续跑) + `crates/foundation/governance/src/audit.rs` (continuity_id audit chain).
  - **❌ continuation_snapshot 0 真实施**: v1 ContinuationSnapshot (messages OpenAI 形状 + pending_tool_call 跨进程崩溃恢复) — v2 canonical execute.rs 同会话 in-memory continuation, **跨进程崩溃恢复 ❌**.
  - **❌ spill 0 真实施**: v2 workspace 无 `SpillStore`/`should_spill`/`SPILL_THRESHOLD_CHARS` grep hit — DSH 借鉴只写在 doc 没用.

### 4.3 真实施路径
- **路径 A continuity (1 周闭环)**: 补 `crates/engine/memory/src/continuity.rs` 新 module (1:1 翻译 v1 305 行) — `current_continuity_id` + `migrate_subject` (复用 IdentityCard/append_only trigger LOCKED) + `MigrationReport`. 验证 FrozenTurnContinuation 跟 v1 `ContinuationSnapshot` 同源语义.
- **路径 B continuation (2 周)**: 新 crate `crates/engine/continuation/src/snapshot.rs` (1:1 翻译 v1) + 接 execute.rs turn 失败时 save → 重启后 consume 续跑.
- **路径 C spill (1 周)**: 新 crate `crates/engine/runtime/src/spill.rs` (1:1 翻译 v1 178 行) + 接 `crates/engine/runtime/src/canonical/tool_executor.rs` — 工具结果 > 2000 chars → spill + 返回路径 + LLM 用 FileOperator 按需读. **物种化**: continuity_id = per-user (5 维 memory per-user 自然塑形), 跨进程恢复 = 物种化"同一个她"技术保证. **0 装诚实**: Windows 0700 0 装 PASS (v1 L13 同源), migration 不假装灵魂同一. **借鉴链**: spill 是 DSH (DeepSeek Harness) spill-local 机制, 1:1 翻译 + 标注来源 (per O-2 前人肩上).

---

## 5. assemble / hello — 1.0 vs 2.0 + 启动/装配

### 5.1 1.0 真账 (`assemble.rs` 1101 行 + `hello.rs` 121 行, REAL)
- **assemble.rs CompanionApp 机制装配器** (lib 层, 0 LLM 依赖): **核心 8 字段**: store/session/identity(L0)/essential_budget(L1)/inject_budget(默认 6000)/rhythm(节律共享)/goal(目标共享)/access(记忆 access 追踪)/last_extract+extract_interval(600s)/last_summarize+summarize_interval(300s). **3 个 LLM trait 注入** (策略模式): `DeepRecall::recall(query, candidates)` / `DialogSummarizer::summarize(text, prev)` / `ExperienceRefiner::refine(reflects) -> Option<Experience>` — LLM 实现留 example, 0 装 PASS. **Builder**: `with_identity`/`with_essential_budget`/`with_inject_budget`/`with_diary`/`with_rhythm`/`with_goal`/`with_extractor`/`with_summarizer`/`with_refiner`/`with_deep_recall`/`with_extract_interval`/`with_summarize_interval`/`with_brain`. **注入管线 8 块**: L0 Identity(core) → L1 Essential(core) → 状态 → 记忆 → 图谱 → 偏好 → 今日 → 成长 → ContextAssembler 预算化. **L1 Essential Story** (mempalace §5.6) essential-* 标记优先 + importance ≥ 8 兜底. **4 源统一注入** (§5.1 收官): topic_groups(主题索引) + diary(近 N 日摘要) + cross_diary(关联片段) + 记忆证据块, 各源独立预算, **砍序: 关联→日记→主题→记忆证据** (反幻觉基石最后砍). **提炼调度**: `extraction_due` 节流 + `run_extraction` 提炼→图谱→对账→应用. **滚动摘要**: `summarize_due` 节流 + `summarize_dialog` sum-* 链持久化. **自成长**: `refine_experience` 反思→经验 + `export_promotion_candidates`. **跨日记关联片段渲染**: query→topic_tokens↔active_facts 共享 token → `link_core` 命中 → `CrossDiaryIndex.diary_for_fact`. **DST 回归** (台账 #34): `.single()`+Option 兜底 0, 不 panic.
- **hello.rs Windows Hello 真绑机制口** (审计 P3#22, **0 装 PASS**): `detect_hello_capability()` 真探测 NGC 凭据提供方注册表键 (`HKLM\...\Credential Providers`) → `HelloCapability::Available{provider}` / `Unavailable{reason}` (失败诚实 Err 不伪造) + `HelloBound` trait `enroll(user)` / `verify(user)` (未接实现 = 明确 Err).

### 5.2 2.0 现状
- **❌ 0 真实施** (主代理真账写 ❌ — 准确). 基础: `crates/foundation/core/examples/hello_world.rs` (演示 IdentityCard 创建, 不是真 assemble) + `crates/engine/runtime/src/canonical/prompt_assembler.rs` (无 8 块分层注入+无 Essential+无 4 源统一+无节流+无 LLM trait 注入) + 12 slot cognitive module `memory_recall`+`memory_writeback` WIRED (A 块完成) 但 assemble 8 块分层接入缺. 0 真实施原因: v2 canonical runtime A 块专注 OrganOrchestrator 完整化, assemble 装配层是 Round 12 P1.

### 5.3 真实施路径
- **路径 A assemble (4 周, 物种化塑形核心)**: 新 crate `crates/engine/companion/src/` — `CompanionApp` 1:1 翻译 v1 1101 行 + 3 个 LLM trait + builder 模式 + 8 块分层 + 4 源统一 + 节流 + 自成长; 接 `crates/engine/runtime/src/canonical/{organ_orchestrator, prompt_assembler, execute}.rs` turn Start/End 触发; 接 `crates/engine/memory/src/` IdentityCard + EpisodeStore + DiaryStore(新) + CrossDiaryIndex(新 1:1 翻译 v1 cross_diary.rs). **物种化核心**: CompanionApp 是物种化塑形机制的主装配层 (per `apeireth-true-understanding §1.5`), L0 Identity + L1 Essential + 4 源统一 = per-user 塑形技术保证. **0 装诚实**: LLM trait 注入 (lib 0 依赖), essential 无 essential-* 标记 → importance ≥ 8 兜底.
- **路径 B hello (2-3 周, 需硬件)**: 新 crate `crates/foundation/auth/src/hello.rs` 1:1 翻译 v1 121 行 + 接 Windows 主人硬件 + 微软账号配置 (D 块真接).

---

## 6. milestone — 1.0 vs 2.0 + 物种化塑形节点

### 6.1 1.0 真账 (`milestone.rs` 132 行, REAL 但短小)
- **职责**: **关系里程碑** — 关系里重要事件 (per species 塑形节点).
- **数据**: `MilestoneKind` enum (8 种: `FirstMeeting`/`FirstShare`/`FirstEmotion`/`StageTransition`/`Decision`/`Conflict`/`Repair`/`Custom`) + `MilestonePayload` enum (`Text(String)`/`Number(f64)`/`Stage(BondStage)`/`Decision(String)`/`Custom(Value)`) + `Milestone { id, kind, payload, at, note: Option<String> }`. API: `Milestone::new(kind, payload)` + `with_note(note)` + 5 getter. **0 装诚实**: 无 `MilestoneStore` (落库靠调用方). 跟 v1 `bond::BondStage` (Initial/Trusted/Familiar/Intimate/LongTerm/Paused/Ended — 7 stage) 联动 + 配合 `consolidation_writeback` pipeline (❌ v2 缺).

### 6.2 2.0 现状
- **❌ 0 真实施** (主代理真账写 ❌ — 准确). 已就位: `crates/engine/runtime/src/canonical/orchestrator.rs` `RelationshipState` trait (per v1 emergence.rs 1:1, depth 0.5 占位) — Bond 关系 trait 已 1:1. 0 真实施原因: milestone 是 species 塑形节点层 (per `apeireth-true-understanding §2.5 长期记忆塑形缺口`), v2 还在基地+Agent 平台层, 物种化塑形层未启动.

### 6.3 真实施路径
- **路径 A (1 周)**: 新 crate `crates/engine/milestone/src/` — 1:1 翻译 v1 132 行 + 新增 `MilestoneStore` (SqliteMemoryStore + `milestones` 表 id PK + kind + payload TEXT JSON + at_ms + continuity_id FK + note) + 跟 `apeireth-memory` IdentityCard.continuity_id 对齐 (per-user) + 接 `apeireth-runtime::Canonical::Orchestrator::Bond` trait — `Bond::evolve(stage, depth)` 触发 `MilestoneKind::StageTransition` 自动 record. **物种化核心**: 8 种 MilestoneKind 是 per-user 关系塑形关键节点 (FirstMeeting/FirstShare/FirstEmotion 是 species 信任起点; Conflict+Repair 是 species 关系韧性信号). **0 装诚实**: 无自动事件触发 (调用方显式 record, 不假装"自动检测到里程碑"). **跟 v1 bond.rs 对齐**: `MilestonePayload::Stage(BondStage)` 严格映射.

---

## 7. experiment_field — 1.0 vs 2.0 + 实验场 (vision L40 自我改进)

### 7.1 1.0 真账 (`experiment_field.rs` 282 行, REAL)
- **哲学 (主人 2026-08-18 "自我改进闭环应该立刻补全")**: 完整回路 = 提案 → **实验** → 通过 → 主人批准 → 部署 → 监控 → 回滚 → **学习**. 此前缺两环: (1) **实验**: smol-vm (Rust+libkrun 微 VM) 方向: "**独立的是实验, 批准的是部署**"; (2) **回滚学习**: yoyo revert-receipt 模式.
- **核心机制**: `ExperimentStatus` (Proposed/Building/Testing/Passed/Failed 确定性状态机) + `Verdict` (Pass/Fail(String)) + `VMRunner` trait + `NoopVMRunner` (诚实 Err: "VM 实验场未接入") + `Experiment { id, proposal, artifact, status, failure_reason, approved_for_deploy, at_ms }` + `ExperimentField` HashMap + `propose`/`run`(VM build+test, Err → 回 Proposed 不假装已实验)/`approve_for_deploy`(仅 Passed 可)/`learn_from_failure`(写 `experience::ExperienceStore`, 集成而非分立)/`get`/`len`. 测试 5 个 #[test].

### 7.2 2.0 现状
- **❌ 0 真实施** (主代理真账写 ❌ — 准确). 已就位: `crates/engine/runtime/src/canonical/upgrade_cycle.rs` (Round 1-2 A 块 Stage 5, L0-L5 UpgradeCycle 400 行, **真实施已落**) + `crates/engine/organ/src/emergence.rs` E7 emergence + `crates/engine/memory/src/experience.rs` Experience trait + ExperienceStore (per v1 1:1). 0 真实施原因: ExperimentField 是 self-improvement 闭环独立实验场, A 块 UpgradeCycle 是 upgrade 机制但**没真隔离实验场** (per 主人 2026-08-18 哲学 "独立的是实验").

### 7.3 真实施路径
- **路径 A (3 周, 跟 vision L40 自我改进闭环接)**: Stage 1 (1 周) 新 crate `crates/engine/experiment/src/{field, runner}.rs` — 1:1 翻译 v1 282 行 + `ExperimentField` + `NoopVMRunner`; Stage 2 (1 周) smol-vm/libkrun 调研 + 接入 `VMRunner` trait (需 Linux KVM, macOS HVF 评估, Windows WSL2 兜底); Stage 3 (1 周) 接 `crates/engine/runtime/src/canonical/upgrade_cycle.rs` — L2 提案生成 → L3 ExperimentField.run() → L4 主人审批 → L5 真部署; 接 `crates/engine/memory/src/experience.rs` `ExperienceStore::save` — Failed 走 `learn_from_failure` 闭环. **物种化锚**: vision L40 自我改进是 species 核心 (per `apeireth-true-understanding §1.2 五原型`), ExperimentField 是技术保证 — **独立实验 vs 批准部署** 分开. **0 装诚实**: NoopVMRunner 默认 impl (v1 L50-56), 失败实验状态回 Proposed 不假装已实验 (v1 L121-126), 仅 Passed 可批准 (v1 L143).

---

## 8. proactive / progressive / pentest — 1.0 vs 2.0

### 8.1 1.0 真账
- **proactive.rs 186 行 (REAL)**: `ContextSource` trait + `EmptyContext`(None 默认) + `MemoryContextSource`(SqliteMemoryStore → 最近 5 episode 最后一条 content 截 200 chars) + `LarkDelivery`(飞书 IM 真送达 wrap `apeireth-lark::LarkRealImpl` + receive_id) + `ProactiveDriver<D,C>` 真心跳 (tokio interval + observe + run).
- **progressive.rs 171 行 (REAL, TP21 渐进式披露注入, 借鉴 claude-mem 39k stars)**: `CatalogEntry`(topic+summary+count) + `ProgressiveCatalog`(`catalog_budget_chars` 默认 1600 = ~800 token) + `block()`(按预算截断+末尾标注省略数 "…另有 N 个主题未展开") + `expand(topic)`(按需详情, 0 装 PASS 不假装已拉取记忆) + `fit_count()` 诊断.
- **pentest.rs 443 行 (REAL, 升级套件, 0 装诚实)**: v1 不内置主动扫描 (不发包不连端口), 扫描动作由 ShellExec 执行外部工具 (nmap), 本模块只做 **计划编排** + **结果解析**: `parse_target`(URL/host[:port], IPv6 如实不支持) + `in_scope`(E-1 范围闸, 子域后缀匹配) + `build_plan` 确定性计划生成器 (被动侦察→主动轻触→端口/服务发现→目录/指纹→报告沉淀, 每步标注 risk/note) + `ReconPlanTool` + `ScanReportTool` + `PentestReconPlugin` + `PentestScanPlugin` (on_load 注册+授权, on_unload 真清理).

### 8.2 2.0 现状
- **⚠️ E7 emergence 已 1:1 移植 + 其余 0 真实施** (主代理真账写 ❌ — 部分不准确):
  - **E7 emergence 已真实施** (per v1 emergence.rs 1:1): `crates/engine/organ/src/emergence.rs` (1100+ 行, `EmergenceOrgan` + 8 重 gate 真实施) + `crates/engine/runtime/src/canonical/orchestrator.rs` L7 (主动循环逻辑接 canonical orchestrator, 8 重 gate 真实施, 5 状态机 forward-declared per 子代理 R7 独立判断) + `crates/foundation/plugin/src/organ.rs` (InitiativeGate 13 种 = emergence 8 + organs 5).
  - **❌ proactive 缺**: `LarkDelivery`(飞书真送达) + `MemoryContextSource` 真生产 + `ProactiveDriver` 真心跳 — v2 无 `apeireth-lark` crate (per 真账 §1.6 Lark 适配器 0 真实施).
  - **❌ progressive 缺**: v2 只 `crates/engine/storage/src/migrations.rs:43` 有 `topic_groups` 表 schema, 没 ProgressiveCatalog 装配.
  - **❌ pentest 缺**: v2 workspace 0 hit.

### 8.3 真实施路径
- **路径 A proactive (2 周, 跟 E7 emergence 接)**: 新 crate `crates/engine/proactive/src/` — 1:1 翻译 v1 186 行 + `ContextSource` trait + `MemoryContextSource` 接 `apeireth-memory` + `LarkDelivery` (Lark 凭据环境变量注入, 0 装诚实: 无凭据 → Err) + 接 `crates/engine/runtime/src/canonical/orchestrator.rs` OrganOrchestrator.tick 末 → ProactiveDriver.observe + run.
- **路径 B progressive (1 周, 跟 R22 reflection 同步)**: 新 crate `crates/engine/memory/src/progressive.rs` — 1:1 翻译 v1 171 行 + `ProgressiveCatalog` + `catalog_block()` 注入常驻 + 接 `crates/engine/runtime/src/canonical/prompt_assembler.rs` 替换占位 catalog 块.
- **路径 C pentest (2 周, O-1 安全优先 + E-1 主人授权)**: 新 crate `crates/adapters/pentest/src/` — 1:1 翻译 v1 443 行 + E-1 范围闸 0 触碰 LOCKED (per `crates/foundation/governance/src/onion.rs:249`). **物种化**: proactive 主动涌现是 species 时间维度 (主人作息塑形), progressive 渐进式披露是 species 注意力维度 (per N.E.K.O 五维 memory 借鉴). **0 装诚实**: pentest 不内置扫描 (v1 L6), progressive `expand()` 不假装已拉取 (v1 L83-88), Lark 无凭据 → Err (v1 proactive L9-11).

---

## 9. Kani proofs — 1.0 vs 2.0 + 形式化证明 (物种化 + 0 装诚实)

### 9.1 1.0 真账 (`bridge_kani_proofs.rs` 146 行 + `organ_kani_proofs.rs` 116 行, REAL)
- **bridge_kani_proofs.rs (R176 Bridge 5 proofs)**: consciousness → companion bridge 不变量 — `r176_b5_01..05` (5 #[test] + 1 #[cfg(kani)] #[kani::proof]): Plutchik 8 维 emotion → BondEmotion 8 维 inputs 全部 ∈ [-1.0, +1.0] + apply_to_character/apply_to_bond no-panic + BondDepth value() 有效.
- **organ_kani_proofs.rs (R177 companion organ proofs)**: bond organ 自身不变量 — `r177_cmp_01..12` (12 #[test]): BondStage 7 种 + labels distinct + is_terminal + BondDepth ZERO/ONE + clamp [0,1] + new initial + evolve advances + character default 0 + apply_emotion clamp + serialize 5 keys + apply via bond; `r177_cmp_kani_01..02` (2 #[cfg(kani)] #[kani::proof]): depth clamps + stage count.

### 9.2 2.0 现状
- **⚠️ 多 crate organ_kani_proofs 已 R177 装 + bridge_kani_proofs 0 真实施** (主代理真账写 ❌ — 部分不准确):
  - **organ_kani_proofs 已就位** (~6 crate): `crates/foundation/core/src/organ_kani_proofs.rs`(2 #[kani::proof]) + `crates/engine/memory/src/organ_kani_proofs.rs`(2 + EpisodeQuery 不变量) + `crates/foundation/protocol/src/organ_kani_proofs.rs`(2 + protocol count + constants) + `crates/adapters/sdk/src/organ_kani_proofs.rs`(2 + submodule count + enabled) + `crates/adapters/cli/src/organ_kani_proofs.rs`(5 #[test] + 2 #[kani::proof]) + `crates/adapters/gateway/src/organ_kani_proofs.rs`(per README).
  - **❌ bridge_kani_proofs 0 真实施**: Plutchik → Bond bridge 形式化 — v2 物种化关系 0 形式化保证.
  - **Kani 工具链**: `crates/adapters/sdk/Cargo.toml:68` `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)', 'cfg(fuzzing)'] }` — Kani 工具链 0 装 PASS 标注.

### 9.3 真实施路径
- **路径 A (2-3 周, 物种化 + 0 装诚实 形式化)**:
  - Stage 1 (3 天 R177-扩展 1): 跟 6 个 organ_kani_proofs 同模式, 新增 `crates/foundation/core/src/bridge_kani_proofs.rs` (1:1 翻译 v1 146 行) — Plutchik 8 维 → BondEmotion 8 维 ∈ [-1.0, +1.0] 5 proofs.
  - Stage 2 (1 周 R177-扩展 2): 物种化塑形节点形式化 — Milestone/ContinuationSnapshot/OneRingLedger 关键不变量 Kani proof (seq 单调自增/Brier ∈ [0,1]/rot_score ∈ [0,1]/identity 跨载体 unique).
  - Stage 3 (1 周 9 项综合形式化): 每个 1:1 移植 v1 模块都加 1-2 个 #[kani::proof] (v1 organ_kani_proofs 模式, 0 装诚实: "形式化范围是数学不变量, 不是启发式正确性").
  - Stage 4 (1 周 Kani 工具链接入): 装 kani-verifier + `cargo kani` CI workflow (per O-3 干到底).
  - **物种化**: 物种化关系 (Plutchik → Bond) 形式化是 species 信任技术保证, 物种化塑形节点 (Milestone) 形式化是 species 长期塑形正确性. **0 装诚实**: rot_score 启发式待 A/B 调权重 (v1 L150-152) 必须明示, Kani proof 标 "0 装诚实: 形式化范围是数学不变量, 不是启发式正确性". **LOCKED**: Kani 工具链跟 cargo 0 触碰 (新 CI workflow 不改 Cargo.toml 现有 section).

---

## 10. 协调+上下文综述 (整合 9 项)

### 10.1 9 项相互关系 (物种化分层)
```
[物种化塑形层] (per-user, 长期)
  ├─ milestone (物种化塑形节点 — relationship trust 起点)
  ├─ progressive (物种化注意力塑形 — N.E.K.O 五维 memory 借鉴)
  └─ education (vision L48, 不在本 R11)
[物种化关系层] (cross-user, 长期)
  ├─ Kani proofs (物种化关系 + rot_score 数学形式化)
  ├─ bridge_kani_proofs (Plutchik → Bond 8 维 ∈ [-1,1])
  └─ organ_kani_proofs (per crate, 已 R177 装)
[协调+上下文层] (runtime, per-turn)
  ├─ onering (跨前端账本 — 物种化"同一个她"技术保证)
  ├─ continuity + ContinuationSnapshot (锚点 + 跨进程崩溃恢复)
  ├─ context_rot + assemble (context window 旋转 + 8 块分层装配)
  ├─ spill (工具结果溢出 — DSH 借鉴)
  └─ hello (Windows Hello 真绑 — Windows 专属, 物种化"主人唯一")
[主动+决策层] (proactive, 长期)
  ├─ proactive (主动涌现接真 — Lark + Memory + Heartbeat)
  ├─ experiment_field (自我改进独立实验场 — vision L40)
  ├─ oracle + oracle_adapters (预测决策沙盘 — Brier 校准 + CoinGecko + TimeSeries)
  └─ pentest (渗透测试升级套件 — E-1 范围闸)
[认知校准层] (cognitive, per-turn)
  └─ context (budget + core 保护) — 已被 12 slot cognitive module 部分覆盖
```
**关键耦合**: onering↔continuity (continuity_id 锚点) / assemble↔context_rot (8 块分层+rot 触发 compact) / oracle↔world_model (CalibratedResolver 已移植 organ, oracle_adapters 待 engine layer) / experiment_field↔experience (learn_from_failure 写 ExperienceStore, 集成而非分立).

### 10.2 跟 vision.md 物种化 + 0 装诚实锚对接
- **三面一体** (per `apeireth-true-understanding §1.1`): **基地** = Kani proofs (LOCKED 0 触碰 + 形式化); **Agent 平台** = onering + context_rot + assemble; **她 (物种化)** = milestone + education (vision L48).
- **五原型** (§1.2): **世界模型 W1/W2/W3** = oracle Brier 接入 W2 (per Mio 真账 §2.2); **自我改进 (A 块 ✅ 骨架)** = experiment_field 独立实验场 (vision L40 待补); **自主好奇心 E4** = progressive 渐进式披露注入 (N.E.K.O 借鉴); **连续感知 (R14 地基)** = spill 工具结果溢出; **价值内化 F6** = Kani proofs 形式化保证.
- **9 哲学锚 LOCKED**: S-2 实事求是 (9 项 v2 grep 实测) / O-5 不假装 (9 项 0 装诚实标统一格式) / O-3 干到底 (9 项 critical path 估 9-11 周真实施).

### 10.3 真实施顺序 + 估时 + 借鉴链

| 阶段 | 项 | 周 | 阻塞 | 借鉴链 |
|---|---|---|---|---|
| P0-1 | continuity 路径 A | 1 | 0 | v1 1:1, 复用 v2 IdentityCard |
| P0-2 | milestone 路径 A | 1 | 0 | v1 1:1, 复用 v2 Bond trait |
| P0-3 | onering 路径 A | 2 | 0 | VCP OneRing 借鉴, 复用 v2 IdentityCard |
| P0-4 | context/context_rot 路径 A (3 stage, 必先融合) | 3 | 0 | v1 1:1 翻译 (融合重复) |
| P1-1 | progressive 路径 B | 1 | P0-4 | claude-mem 39k stars + N.E.K.O 五维 memory |
| P1-2 | continuation + spill 路径 B+C | 2 | 0 | DSH spill, 复用 v2 FrozenTurnContinuation |
| P1-3 | proactive 路径 A (接 E7) | 2 | P0-4 | v1 1:1, 飞书 Lark 凭据 |
| P1-4 | pentest 路径 C | 2 | 0 | v1 1:1, E-1 范围闸走 governance onion |
| P2-1 | experiment_field 路径 A (3 stage) | 3 | 0 | smol-vm/libkrun 调研, vision L40 |
| P2-2 | oracle/oracle_adapters 路径 A (2 stage) | 3 | 0 | v1 1:1, 复用 v2 organ world_model trait |
| P2-3 | Kani proofs 路径 A (3 stage) | 2-3 | 全部 P0/P1 | v1 R176/R177 1:1 + Kani 工具链 |
| P3-1 | assemble 路径 A (物种化塑形核心) | 4 | P0-4 + P1-2 | v1 1:1, 4 源统一注入 |
| P3-2 | hello 路径 B (Windows 真接) | 2-3 | D 块硬件 | v1 1:1, NGC 凭据探测 |

**总 critical path**: P0-1 → P0-4 → P1-1 → P1-2 → P2-3 → P3-1 = 11-13 周 (主代理真账估 3-4 周 ❌, 实际 11-13 周更准确). 9 项可部分并行: P0-1/2/3/4 + P1-3/4 可并行 (估 8-10 周真实施总 critical path).

---

## 11. 主代理决策建议 (R11 brief)

### 11.1 9 项优先级 (per O-6 + 主代理真账 §3.1 #13/#14/#15 + 物种化锚)

| 优先级 | 项 | 估时 | 物种化锚 | 借鉴链 |
|---|---|---|---|---|
| **P0** | continuity (1 周) | 1 | 物种化"同一个她"基础锚 | v1 1:1, 复用 v2 IdentityCard |
| **P0** | milestone (1 周) | 1 | 物种化塑形节点 (vision L48 配套) | v1 1:1, 复用 v2 Bond trait |
| **P0** | onering (2 周) | 2 | 物种化"同一个她"跨前端 | VCP OneRing + v2 IdentityCard |
| **P0** | context/context_rot (3 周, 必先融合) | 3 | 物种化"注意力维度" + 长期记忆塑形 | v1 1:1 翻译 (融合重复) |
| **P1** | progressive (1 周) | 1 | 物种化"注意力塑形" (N.E.K.O 借鉴) | claude-mem 39k stars |
| **P1** | continuation + spill (2 周) | 2 | 物种化"跨进程记忆" + 工具连续性 | DSH spill, 复用 v2 FrozenTurnContinuation |
| **P1** | proactive (2 周, 接 E7) | 2 | 物种化"主动时间维度" | v1 1:1, 飞书 Lark 凭据 |
| **P1** | pentest (2 周) | 2 | 物种化"主人授权范围" (E-1 闸) | v1 1:1, 走 governance onion |
| **P2** | experiment_field (3 周) | 3 | 物种化"自我改进独立实验场" (vision L40) | smol-vm/libkrun 调研 |
| **P2** | oracle/oracle_adapters (3 周) | 3 | 物种化"决策塑形" (校准 + 期望值) | v1 1:1, 复用 v2 organ world_model trait |
| **P2** | Kani proofs (2-3 周) | 2-3 | 物种化关系 + 0 装诚实 形式化保证 | v1 R176/R177 1:1 + Kani 工具链 |
| **P3** | assemble (4 周) | 4 | **物种化塑形主装配层** (per vision §1.5) | v1 1:1, 4 源统一注入 |
| **P3** | hello (2-3 周) | 2-3 | 物种化"主人唯一" (Windows 真绑) | v1 1:1, NGC 凭据探测 |

### 11.2 真实施 brief (per 主代理 §4.3 派单模板, 9 项共用)
- **任务**: 写真账 1.0 vs 2.0 [项名] 真调研 + 写真账 + 真实施路径 (per 本 R11 真账 §1-§9 + §10 综述)
- **必读**: 本 R11 真账 + 1.0 真账 `legacy/donor/apeireth-companion/src/[项].rs` + 2.0 真账 `v2-reference-handbook-2026-08-28.md` + 5 R7 真调研 + master audit 真账
- **输出**: `docs/04-internal/r12-[项名]-implementation-2026-08-28.md` (≤ 200 行真实施 spec + 真账)
- **约束**: 不写真账以外的 file / 不 git add / commit / push / 0 触碰 LOCKED / ≤ 4h / **必含物种化锚 + 0 装诚实标 + 5 重守门**

### 11.3 0 装诚实标 (per O-5)
- **主代理真账 §1.8 状态不准确**: oracle/oracle_adapters 标 ❌ (实际 organ world_model trait 已 1:1); E7 emergence 标 ❌ (实际 organ emergence 已真实施); Kani proofs 标 ❌ (实际 6 crate organ_kani_proofs 已 R177 装); context/context_rot 标 ❌ (仍 0 真实施, 准确). 本 R11 真账修订.
- **v1 context.rs + context_rot.rs 重复实现**: rot_score 两 file 各实现, 真实施前 1 天先融合
- **v1 hello.rs 概念 collision**: 主代理真账列"启动/装配", 实际 Windows Hello 生物识别 (NGC 探测), 真调研按 v1 真账画像
- **R11 真账 0 实测**: 未 git clone v2 master branch, 仅读 1.0 真账 (legacy/donor/) + 2.0 handbook + 5 R7 真调研 + 12 slot + master audit 真账推论 + grep v2 workspace 验证. 真实施前主代理必亲验 (per §4.2 派单顺序, 跟 Round 8 verify 模式一致)
- **数 critical path 11-13 周基于经验估时**: 实际派 sub-agent 真实施可能更短或更长, 启发式估时, 真实施每周回 R11 真账更新

### 11.4 LOCKED 0 触碰 (per §10 LOCKED 5 项 + 新 crate 不动)
- 9 哲学锚本体 (`crates/foundation/core/src/eight_anchors.rs:58-79`)
- 13 键 verdict cache (`crates/foundation/core/src/philosophy.rs:142`)
- 3 项不可变脊柱 (`crates/foundation/governance/src/onion.rs:249`)
- workspace.version (`Cargo.toml:44` `"1.2.0"`)
- R11 baseline 3 值 (`legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44`)
- **新增**: 9 项 1:1 翻译 v1 时必用新 crate 或新 module, 不改现有 crate 公共 API (per O-2 前人肩上, 1:1 翻译不重构)

### 11.5 5 重守门 baseline (per `v2-reference-handbook §2.3`)
- clippy 0 warning (新 crate 必跑 `cargo clippy --workspace --all-targets --locked -- -D warnings`)
- tests 0 fail (新 crate 必跑 `cargo test --workspace --locked`)
- legacy compat path < 100 (新 crate 0 改 legacy 路径)
- LOCKED 5 项 0 触碰 (新 crate 0 触碰 9 哲学锚 + 13 键 + 3 脊柱 + workspace.version + R11 baseline)
- 9 哲学锚 0 减 (新 crate 0 改 anchor count)

---

_R11-CoordinationContext 写于 2026-08-28, 真调研 9 项协调+上下文 gap (1.0 vs 2.0 功能对比), 写真账 ≤ 300 行, 9 项 P0-P3 估 11-13 周 critical path 真实施. 0 装诚实标: 0 git clone v2 master branch (仅 grep 验证), 主代理真账 §1.8 标 oracle/E7 emergence/organ_kani_proofs 部分不准确, 本真账修订. 物种化锚 + 0 装诚实 + 5 重守门 + LOCKED 0 触碰 全部含, 真实施前主代理必亲验._
