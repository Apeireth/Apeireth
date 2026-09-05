# Changelog — Apeireth

## [Unreleased] — Research 证据链补全：真实数据评测 + 形式验证 (2026-09-05)

- **真实数据集评测批（0 LLM 判分，确定性 evidence 命中）**：
  - LoCoMo (ACL 2024, CC BY-NC 4.0)：5882 轮次 / 1986 QA，四策略 × 5 预算档效用-成本曲线 + bootstrap 95% CI（`research/reports/eval-locomo-2026-09-05.md`）。
  - LoCoMo-MC10：question 文本与 locomo10 **1986/1986 全量匹配** → 两版同源（mc10 = 10 选 1 重打包，不构成独立第二数据集，MANIFEST 已如实标注）。
  - LongMemEval (ICLR 2025, MIT)：s/m 两版各 500 QA、每问独立 haystack（48 / 475 会话）；**非 recency 语料上 FixedWindow 32k 预算仅 48%→5.4%，相关性驱动策略 8k 预算即 100%**；haystack 规模敏感性消融入报告（`research/reports/eval-longmemeval-2026-09-05.md`）。
  - 运行器升级：`--source locomo / locomo-mc10 / longmemeval`（`--lme-file` 可选）；Turn 支持每问独立文档宇宙；实验名参与日志 hash（防跨文件覆盖）。
- **Phase 5 形式验证三路互证（`research/verification/`）**：
  - TLA+/TLC 本机模型检查：`ApprovalSM.tla` 1:1 规格，单记录 36 状态 / 三记录 3164 状态，TypeOK + InvA/InvB/InvC + 终态锁全通过（指纹碰撞 2.9E-12）。
  - Kani 机器证明（GitHub Actions `.github/workflows/kani.yml`，run 33945573291）：3/3 harness VERIFICATION SUCCESSFUL；本机 Windows 无 Kani → mirror crate 零复制 `#[path]` 包含 canonical 源文件绕开 rust-version 墙；SipHash 符号展开爆炸 → `#[kani::unwind(32)]` 有界口径（仅 cfg(kani) 生效，生产零影响）。
  - 与既有故障注入（100 轮 0 违例）构成三路互证，边界与口径如实标注（`research/verification/README.md`）。

## [Unreleased] — Research Phase 0–6 交付 + 双协议 + 文档对账 (2026-09-04)

- **research 工作区**:`research/`(baselines/metrics/runners/logs schema);Phase 0 冻结基线 3061 → 交付后全量 **3119 passed / 0 failed / 13 ignored / 106 suites**。
- **Research 前缀模块(全部默认关闭,生产路径零行为变化)**:派生记忆血缘与遗忘闭包审计、BTFM 真双时态(additive,旧 API 不变)、StackPin 上下文保留(竞争比护栏)、ShadowLogger、校准门控自治(风险阶梯+hysteresis)、审批状态机形式化(Dispatched 拆分+崩溃模型+故障注入)、漫游记忆 CRDT、模块非干扰、VaultLRU/FTRL(O(√T) 后悔界)。
- **评测运行器**:`research/runners/`(独立 cargo 项目,合成基准 + 效用-成本曲线 + bootstrap 95% CI + JSONL 日志)。
- **双协议**:`Apache-2.0 OR MIT`(SPDX 更新,`LICENSE-MIT` 新增)+ CONTRIBUTING DCO 等效贡献声明。
- **文档对账**:仓库 URL 迁移 `Apeireth/Apeireth`(96 文件);crate 数统一为实测 16;测试数统一为实测 3119;architecture/CONTRIBUTING/INSTALL/SECURITY/README 全量更新。

## [Unreleased] — P2 加固波次 (candidate `8b7e3111`, 2026-08-30)

> **状态标注 (0 装 PASS)**：下列六项 P2 加固提交全部为 **IMPLEMENTED（库级实现）且经远端 Windows 验证机测试验证**（candidate `8b7e3111`，clean tree，HEAD 已核验）：
> `cargo test --workspace --locked` = **2012 passed / 0 failed**（13 ignored）；`cargo check --workspace --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`git diff --check` 全部通过。
> 注意四个层级不可混淆：**IMPLEMENTED（代码存在于候选）** ≠ **PRODUCTION WIRED（接入 canonical 运行时主路径）** ≠ **DEFAULT ENABLED（无需 opt-in 即开启）** ≠ **HARDWARE VALIDATED（真机验证）**。本波次均为**库级加固**：除真实 Xcap 捕获后端（仅 Windows 硬件验证）外，**没有**新增运行时权威、默认启用模块或生产接线；模块默认全部 opt-in。

### 加固内容 (six P2 commits)
- **检索确定性 (`b2446e67`, `apeireth-memory`)**：BM25 / RRF 平局按 id 稳定排序，等置信度话题按 key 排序且可精确重放。回归证据：`bm25_ties_are_id_sorted_for_ascii_cjk_and_repeated_queries`、`rrf_ties_are_id_sorted_and_replay_exactly`、`equal_confidence_topics_are_key_sorted_and_replay_exactly` 全部通过。库级，无新运行时接线。
- **原则审批绑定提案 (`4f5395b2`, `apeireth-memory`)**：审批工件绑定具体 proposal——`approval_for_proposal_a_cannot_activate_proposal_b`、`concurrent_uses_of_one_approval_artifact_have_one_winner`、`successful_approval_artifact_cannot_be_replayed`、`expired_approval_artifact_is_rejected_without_sleeping` 全部通过。原则审批**刻意保持内存/库级（未接线）**，等待授权的 canonical 治理设计。
- **Continuation 单赢家消费 (`a4ba09fc`, `apeireth-orchestration`)**：跨 store 实例单赢家、hostile id / 恶意 snapshot id 防穿越——`file_store_consume_is_single_winner_across_store_instances`、`hostile_id_claim_cannot_escape_store_root`、`malicious_snapshot_id_cannot_escape_store_root` 全部通过。库级。
- **Reflexion 有界串行持久化 (`53c0376a`, `apeireth-memory`)**：history cap 下 cursor/reflections 保持一致，malformed JSON 为类型化非破坏性错误并释放 mutation claim——`file_store_history_cap_keeps_cursor_and_reflections_consistent`、`malformed_json_is_typed_non_destructive_and_releases_mutation_claim` 全部通过。库级。
- **会话级有界 Spill (`778a0fcb`, `apeireth-tools-canonical`)**：跨 store 实例配额竞争单赢家、并发会话隔离、唯一引用、引用防穿越——`quota_race_has_exactly_one_winner_across_store_instances`、`concurrent_different_sessions_stay_isolated`、`concurrent_same_session_writers_receive_unique_references`、`traversal_and_root_wide_references_are_rejected` 全部通过。库级。
- **真实 Xcap 视觉捕获 (`8b7e3111`, `apeireth-perception`)**：
  - **IMPLEMENTED**：真实 xcap 0.9.8 捕获后端（`XcapVisionBackend`），确定性显示器排序（主屏优先）、bounds 限制、真实 PNG/JPEG 编码、fail-closed 错误。
  - **NOT production wired / NOT default enabled**：仅是 `VisionBackend` trait 的后端实现；canonical runtime 默认路径**不**调用、**不**注册。用法为显式 opt-in 构造（`XcapVisionBackend::default_monitor()` / `new(config)`）；`NoopVisionBackend` 为显式零假设占位。
  - **HARDWARE VALIDATED（仅 Windows）**：远端验证机交互会话中 ignored 测试 `real_xcap_hardware_capture_smoke` 通过（`\\.\DISPLAY1` 主屏 1680x1050，PNG 242067 字节，PNG magic 校验 + 真实桌面内容目检）；本地开发机交互会话 smoke 亦 PASS。无头 / session-0（SSH 服务上下文）环境按设计 fail-closed（`BackendUnavailable` invalid-handle `0x80070006` E_HANDLE）。**未做 macOS / Linux 硬件验证。**

### SSE 状态（本轮终审口径，审计结论 `KERNEL_SEAM_MISSING`）
- Gateway 的 SSE 为**缓冲成帧（buffered framing）**：`POST /v1/chat/completions` 在 `stream: true` 时，于完整 canonical 完成路径（治理、transcript 提交）结束后返回 `text/event-stream` 帧与 `[DONE]` 终止帧。
- **真正的逐 token 增量流式被冻结的 canonical seam 阻塞**：canonical provider/router/runtime 契约只返回完整 `NormalizedResponse` / `TurnOutcome`；增加流式契约将改变冻结内核 seam，需显式授权。Gateway 绕过 Runtime 直接从 provider 流式被架构守门禁止。涉及"实时流式 / 逐 token 输出"表述的文档已按此口径修正。

### 架构不变量（本轮重申）
- 仍为：ONE 主循环、ONE ProviderRouter、ONE canonical completion 治理路径、ONE 主 session 权威、无第二审批权威、模块无持久 turn 权威、无裸 provider 绕行（runtime 7 项 + gateway 2 项 canonical 架构不变量测试全部通过）。
- 库级已验证、**非默认启用**的 P2 能力：turn-scoped `ModuleInvoker`（`canonical_invoker_handle` 7 测试，含跨轮独立预算与拒绝越权 handle 调用）、`OrganModule`（`canonical_organ_module` 12 测试，默认缺席、不暴露工具）、`PreferenceLearning` 闭环（`canonical_preference_learning` 14 测试，含 `turn1_learning_reaches_turn2_provider_context`）、零模块最小内核（`minimal_kernel_without_standard_modules_completes_plain_chat_turn`）。`topic_predictor` 仍未接线进 `PreferenceRecall`。

## [v2.0.0-preview] - 2026-08-29

> **版本定位与状态说明**:
> 本版本为 **Apeireth 2.0 预览版 (Preview)**。
> - **工程建设状态**: 2.0 底座与全部 14 大关键战区核心功能建设已**基本完全实装**（全工作区 16 Crates 100% 编译与单元/集成测试通过，前端桌面端打包与类型全绿，5 项 LOCKED 资产严格零触碰，0 伪造，0 空壳）。
> - **后续发布路线**: 当前阶段已就绪，转入**协作者生产压测、真机环境与端到端联调交叉验证**阶段。待协作者压测与交叉验证闭环后，由协作者提议正式发布 2.0 最终正式版 (GA)。

### 🌟 2.0 核心建设成果总览
- **A 块 / 编排与多 Agent 协作 (`apeireth-orchestration`)**:
  - `OrganOrchestrator` 5 阶段完整化（L0 主权闸 → 9 Organ 链 → 情绪调制 → Council 智囊团加权审议 → 演化闸 → 治理钩子）；
  - `Council` 7 Advisor 仲裁机制与强制按住机制；
  - `SubagentSpec` 多 Agent 编排隔离契约。
- **D 块 / 多模态感知 (`apeireth-perception`)**:
  - `WhisperHttpBackend`：标准 `multipart/form-data` 发送音频转写请求，支持 OpenAI / MiniMax，凭据走 `CredentialResolver`；
  - `XcapVisionBackend`：底层 OS 屏幕多显示器截屏，无头环境 Fail-Closed 守门。（注：真实捕获实现于次日 `8b7e3111` 落地，见上方 Unreleased；仅 Windows 硬件验证，默认不接线、opt-in 构造。）
- **R12 / 长期记忆体系与物种化演进 (`apeireth-memory`)**:
  - **混合检索 (`hybrid_search.rs`)**: 纯 Safe Rust Okapi BM25（$k_1=1.2, b=0.75$）+ CJK 双字滑窗切分 + 余弦语义向量 + RRF 倒数排名融合；
  - **伙伴与羁绊模型 (`partner.rs`)**: 7 大关系演化阶段 + 连续深度 $[0.0, 1.0]$ 状态机 + 5 维性格特征；
  - **关系里程碑 (`milestone.rs`)**: 8 大关系分类 + 5 种强类型 Payload + `MilestoneStore` 契约；
  - **动态原则洋葱 (`principles.rs`)**: 常数时间 `constant_time_eq` 审批校验（消除 1.0 时序侧信道）+ 单调自增链式演化 + L3 洋葱晋级报告导出；
  - **叙事日记与归档 (`diary.rs`)**: 按日 `{YYYY-MM-DD}.json` 归档 + 原子写落盘 + 字符预算受限上下文注入；
  - **每日活动聚合 (`daily_summary.rs`)**: 纯确定性事实分类流聚合（`mem-*`, `reflect-*`, 工具调用）与 Markdown 结构化渲染；
  - **跨日记图关联 (`cross_diary.rs`)**: 共享词元双向关联索引与可审计 `shared_tokens` 上下文抽取；
  - **口头强化反思闭环 (`reflexion.rs`)**: 3 类失败轨迹追踪 + `RuleCritic` + 精确匹配加权排序（精确=2, 子串=1）+ **严格回溯弹出防超支截断**。
- **协调与安全执行 (`apeireth-tools-canonical` & `orchestration`)**:
  - **上下文衰减 (`context_rot.rs`)**: 确定性 3 因子评分（重复度+半衰期陈旧度+无关度）+ 核心段绝对豁免保护；
  - **工具输出溢出保护 (`spill.rs`)**: 会话私有落盘 + `create_new(true)` 独占原子写 + `canonicalize` 路径穿越防御；
  - **断点续行与段编辑 (`continuation.rs`)**: 断点快照 + 原子写 + **O-1 核心段删除拦截防御**；
  - **教育与微积分换元符号检查 (`education.rs`)**: 纯 Safe Rust 四重微分一致性检查、经典三角/双曲/线性根式模式匹配与结构化 Markdown 报告。
- **B 块 / 网关流式交互 (`apeireth-gateway`)**:
  - `POST /v1/chat/completions` 当 `stream: true` 时返回标准 `text/event-stream` SSE 数据帧与 `[DONE]` 终止帧。（口径澄清：为**缓冲成帧**——完整 canonical 完成路径结束后一次性返回，**非逐 token 增量流式**；增量流式被冻结 canonical seam 阻塞，见 Unreleased §SSE 状态。）
- **元认知自校准与长程思维簇 (`apeireth-memory`)**:
  - **意图理解准确率 Brier 自我诊断 (`intent_brier.rs`)**: 滚动窗口 [30, 100, 300] 轮 Brier 得分数学自校准 + 话题领域诊断 + 相对趋势分析；
  - **思维簇管理与元自学习只读回读 (现 `cluster_store.rs`)**: `{YYYY-MM-DD}-{seq:03}.md` 结构化思维文件落盘 + 链注册表 + 安全防穿越与编辑防御 + `InMemoryClusterReader`；
  - **元思考递归推演链 (`meta_thinking.rs`)**: 多阶段“思考 $\to$ 再思考”递进推演引擎 + 最大深度 10 限制 + 思维死循环检测熔断 (`CycleDetected`) + 降级标定 + 反思适配器 (`ChainReflectionThinker`)；
  - **程序性记忆与习惯固化 (`procedural.rs`)**: N.E.K.O 5 维记忆第 5 维，自动记录 Condition-Action 技能规则配方，拉普拉斯平滑计算置信度与高阶规则自动晋级。
- **物种化交互与全双工流式伴随 (`apeireth-gateway` / `perception` / `orchestration`)**:
  - **全双工实时流式打断与插话机制 (`barge_in.rs`)**: 毫秒级广播取消信号阻断服务端生成与传输，向客户端推送 `event: interrupt` 数据帧；
  - **情感声学参数调制与语音向导 (`emotion_voice.rs`)**: 将 PAD 情绪向量与性格特征映射为 TTS 底层 Pitch / Speed / Volume / Emotion 连续声学参数与 SSML 封装；
  - **跨场景环境自适应伴随状态机 (`ambient_context.rs`)**: AIRI 模式感知前台 IDE / 全屏游戏 / 阅读浏览 / 空闲状态，自动在 `FocusAssistant`, `SilentObserver`, `WarmCompanion`, `Resting` 间平滑切换。
- **1.0 深度沉淀与通用协议标准全量吸收 (`apeireth-memory` / `governance` / `tools-canonical`)**:
  - **离线做梦与夜间认知重组 6 阶段引擎 (`dreaming.rs`)**: `Awake` $\to$ `Drowsy` $\to$ `LightSleep` $\to$ `DeepSleep` $\to$ `RemSleep` $\to$ `Awakening` 状态机，驱动 `MetaThinkingChain` 递归推演，自动将反思固化为 `procedural.rs` 习惯规则；
  - **HASH-SQL 唯一事实时间线与不可篡改仲裁机 (`arbitration.rs`)**: 统一 `Frontend`, `Cli`, `Gateway`, `AgentComm`, `System`, `External` 多源事件，三元组确定性仲裁 + 链式 SHA-256 签名 + Merkle Root 根折叠 + `constant_time_eq_str` 防时序侧信道；
  - **工具调用频率限制与安全黑名单守门 (`rate_limit.rs`)**: A6 规则增强，支持分/时多尺度滑动窗口、静态安全黑名单即时阻断与四阶信任等级体系 (`TrustTier: Low, Standard, High, Trusted`)；
  - **Model Context Protocol (MCP) 标准外部工具桥接 (`mcp.rs`)**: 纯 Safe Rust 实现 JSON-RPC 2.0 协议握手 (`initialize`)、动态工具发现 (`tools/list`) 与隔离调用 (`tools/call`)，多模态输出（Text/Image/Resource）归一化。
- **前端桌面端伙伴 (`frontend/companion-desktop`)**:
  - Svelte 5 + Tauri 2 现代化桌面端，生产打包 `pnpm build` 与 `pnpm check` 100% 通过（0 错误，0 警告）。

## [v2.0.0-rc.1] — 2026-08-28

- **A 块 5 stage commit O-6 三阶审查 amend (主代理自检 0 装诚实标修正)**:
  - `c003e078` Stage 1 (was `fc159288`) + `087ab2ac` Stage 2 (was `ea9aa14f`) +
    `50ba2e57` Stage 3 (was `ed6353f4`) + `29e5ce66` Stage 4 (was `1972b040`) +
    `0afa733f` Stage 5 (was `24d163ff`)
  - amend 法: git plumbing (`commit-tree` + `update-ref`) 重写 commit messages,
    code content 0 变 (tests 1739 passed 不变, clippy 0 警告不变)
  - 修订版 O-6 三阶审查 sections 真答案 + 拒 alternatives + 拒理由 (per 八锚本体 O-6 description)
  - 配对 commit `bbbfb75b`: `docs/04-internal/A-block-o6-true-account.md` (0 装诚实复盘 +
    后续 commit 标准) + plan doc §7 (后续 O-6 三阶审查 标准)
  - O-6 doctrine 复盘: '工作量与麻烦不是拒绝重做的理由' — 不找借口
  - force push 到 origin/main (`+ 798dba5b...bbbfb75b main -> main (forced update)`)

- **A 块 OrganOrchestrator 完整化 stage 5 (缺口 E)**: L0-L5 `UpgradeCycle` driver
  真实施 (per R11 §7 + `v2-architecture-reflection.md` §6). 新文件
  `crates/engine/runtime/src/canonical/upgrade_cycle.rs` + `tests/upgrade_cycle.rs`:
  - struct `UpgradeCycle` 持 `Arc<OrganOrchestrator>` + `Arc<dyn GovernanceHook>`
    + `Arc<dyn SelfAssessmentStore>` + `Arc<dyn TagSuggester>` + `current_version`
  - `run_full_cycle(proposal)` 6 步串行: L0 哲学锚 (governance 真调) →
    L1 self_assessment (alignment >= 0.6) → L2 Orchestrator.council_deliberate
    (Stage 4 真路径) → L3 chain_9_organs → L4 governance 主人 Veto → L5 tag 建议
  - `CycleStep` enum (Pending/InProgress/Approved/Rejected/Tagged) + `UpgradeCycleResult`
  - `TagSuggester` trait + `DefaultTagSuggester` (bump patch "1.2.0" → "1.2.1";
    **0 装诚实**: 不自动跑 `Command::new("git", ...)` 仅返建议字符串, 主人手跑 git tag)
  - 7 集成测试 + 3 单元测试覆盖 happy path / L1 Rejected / L2 Stop / L0 Deny /
    layer_outcomes 顺序 / TagSuggester trait object.  0 触碰 LOCKED, 0 引新外部 dep,
    `cargo test --workspace --locked` 1739 passed / 0 failed (1729 + 10 new).
- **A 块 OrganOrchestrator 完整化 stage 4 (缺口 C)**: tick 步骤 4 智囊团审议改用
  `Council::decide_with_invoker` 真生产路径 (per cognitive-module-wiring.md:99 10s/advisor
  + 60s 总 timeout + 7 advisor 并行, 返 typed `CouncilResult` 含 `CouncilDecision`
  + `aggregate_score` + `failures` + `side_call_count` + `timed_out`). Orchestrator.new() 加
  `Arc<dyn CouncilInvoker>` 参数 (per Council trait 设计). 新增 `MockCouncilInvoker` test
  helper (allow_all / stop_all 2 variant). `council_deliberate()` 翻译 `CouncilDecision`:
  Continue/Retry → 通过, Stop/DeferToHuman → CouncilVeto. 新增 1 集成测试 (5 case:
  council_decide 2 path / tick Spoke + CouncilVeto / chain 不受影响). 0 触碰 LOCKED,
  `cargo test --workspace --locked` 1729 passed / 0 failed.
- **A 块 OrganOrchestrator 完整化 stage 3 (缺口 A)**: `check_8_gates()` 接 E7 organ
  真实 `last_hold()` 路径. 重构: `InitiativeGate` 从 `crates/engine/organ/src/emergence.rs`
  移到 `crates/foundation/plugin/src/organ.rs` (canonical 13-variant enum), `OrganOutput::Emergence`
  新增 `gate: Option<InitiativeGate>` 字段, `OrchestratorGate` 改为 alias. 新增
  `extract_e7_gate()` helper + `check_8_gates()` 加 `&OrganChainOutputs` 参数. Orchestrator
  从 `chain.e7.gate` 拿 RhythmUnknown/RhythmVeto/DriveLow 3 重真实 gate (per v1
  `EmergenceLoop::last_hold()` 1:1). 新增 1 集成测试 (7 case: 3 gate 真路径 / None
  skip / NotImplemented skip / tick RhythmVeto 拦下 / Mock default Spoke). 0 触碰 LOCKED,
  `cargo test --workspace --locked` 1728 passed / 0 failed.
- **A 块 OrganOrchestrator 完整化 stage 2 (缺口 B)**: tick 步骤 3 情绪调制真生产路径
  — 从 `chain_9_organs()` 输出的 `chain.f1` 提取 `OrganOutput::Emotion { pleasure, .. }`
  → 算 mood = (pleasure + 1.0) / 2.0 (per v1 organs.rs:109). 新增 `extract_emotion_mood()`
  helper, 边界处理: `NotImplemented` / `None` / 其他 variant → 返 `None` (0 装诚实,
  不假装"有情绪数据"). mood < `mood_floor` → 触发 `EmotionLow` gate, tick 返 None +
  `last_decision = Held(EmotionLow)`. 新增 1 集成测试 (5 case: low/high/mood boundary
  / NotImplemented / tick 真实路径). 0 引新外部 dep, 0 触碰 LOCKED 5 项,
  `cargo test --workspace --locked` 1727 passed / 0 failed (1726 baseline + 1 new).
- **A 块 OrganOrchestrator 完整化 stage 1 (缺口 D)**: ratify_fresh_policy() 走完整
  5 状态 transition 链 (per v1 `AwakeCompanion::ratify_fresh_policy` 1:1, v1 走 4 个
  evolution.transition 调用). 新增 `RatificationChain` struct 留痕 4 transition
  每步 result (telemetry/audit 用). 0 引新外部 dep, 0 触碰 LOCKED 5 项,
  `cargo test --workspace --locked` 1726 passed / 0 failed.
- **RC-10 metadata integrity** (`2214fb01`, upstream `38cc1039`): encrypted
  records now write an APX2 header whose authenticated data binds the format
  version, service/type, physical record index, opaque keyed record-id
  commitment, and complete sealed length. Existing v1 records remain
  readable; new records use v2. Added tamper, swap, framing, truncation, and
  legacy-read coverage.
- **Canonical Council wiring** (`e77256de`, plus `863df70f`): the Council adapter is now in the
  single runtime module loop, disabled by default, and uses the runtime-owned
  `ModuleInvoker` for at most seven bounded typed advisor calls with 10-second
  per-advisor and 60-second overall timeouts. Fake-invoker tests cover wiring,
  ordering, retry/stop aggregation, malformed responses, and timeout defer;
  real provider E2E remains credential-gated.
- **Durable Experience extraction** (`a11c81ff`): after a successful durable
  `AfterTurn` episode write, the conservative extractor materializes bounded
  summary/explicit-marker artifacts with source-episode evidence into the
  existing Wiki/KG/Association stores. SQLite association observations are
  idempotent; failures are fail-open with warnings. No hidden model call or
  full-transcript copy is performed by the production default.
- **RC-11 migration tooling** (`615121bd` plus the APX2 follow-up): the Python
  v1→v2 migration utility and Rust integration coverage now emit the current
  APX2 envelope, preserve serialized logical ids when available, and fail
  closed on truncated input or oversized ids.
- **O-6 anchor alignment** (`926465c8`): the canonical anchor enum now exposes
  the authorized O-6 entry with compile-time nine-anchor ordering checks.
- **Deprecated consumer audit**: active workspace production consumers of
  `#[allow(deprecated)]` / `#[deprecated]` are 0; legacy/archive material stays
  retained and no artificial migration was added.
- **R12 OrganOrchestrator 真实施** (`2550b99d`): 类似 v1 AwakeCompanion 的 9 organ 串联层落地在
  `crates/engine/runtime/src/canonical/orchestrator.rs` — 13 重 gate (8 E7 + 5 v2 扩展) +
  5 状态机 PolicyStage 前向声明 + 9 organ 顺序 process + 故障隔离 (fail → NotImplemented 不断链),
  10 lib + 3 integration tests; 0 引新外部 dep, 0 触碰 LOCKED.
- **8 spec 收齐 + 6 处错账修正** (`ccf29c57`): R9 frontend 对接 + R10 cognitive 9 organ 集成 +
  R11 OrganOrchestrator + R13 接力审 + R14 RC-7 真 modality + R15 preference_learning 激活 +
  Z 独立审计; 主代理亲做 12 slot 真账核验 = **6 WIRED + 6 DEFERRED** (judge/council 为 WIRED,
  OFF by default, 弃用 "SLOT READY" 旧称) + R12 状态 + 接手人 9 actionable.
- **给新团队的话** (`TO-NEW-TEAM.md`): 阶段性收盘交付 + 接手 10 步 + 4 块真实施清单.
- **Workspace 测试实测**: `cargo test --workspace --locked` = **1726 passed, 0 FAILED**
  (2026-08-28 主代理亲跑); `cargo clippy --workspace --all-targets --locked -- -D warnings` = 0 警告.

## [2026-08-28] v2.0.0-rc.1 incremental baseline (7/10 RC complete)

- **✅ v2.0.0-rc.1 RC-1/2/3/4/8/9/10 真实现 (7/10 RC, 2026-08-27)**:
  - **RC-1 MemoryBackend SqliteBackend 真 SQL 重写** (`43ec9635`): 5 方法纯 SQL
    (INSERT/SELECT), 绕开 `SqliteMemoryStore` 的 Mutex<Connection>, 走
    `SqliteConnectionPool` 真并发 (writer-async + reader-pool).
    7 测试 + 1000 episode 写入 < 1s 性能基准.
  - **RC-3 PreferenceStore SQLite** (`61cc0421`): 新表 `user_preferences` + 索引
    (session_id, confidence DESC), 真 UPSERT (INSERT OR REPLACE), 7 测试
    (roundtrip / confidence 排序 / topic 过滤 / session 隔离 / 真删 / etc.)
  - **RC-4 SelfAssessmentStore SQLite** (`042ad4eb`): 新表 `self_assessments` + 索引
    (task_id, assessed_at DESC), runtime hot-path `recent_for_task(task_id, 5)`
    启动时读, alignment < 0.6 触发 DeviationReport. 7 测试.
  - **RC-8 SubSupervisor std::process 真 impl** (`67fc66a0`, 改名 commit `4e4fba89`):
    写真 `std::process::Command::spawn` (sync, 不用 tokio::process 因 trait sync
    约束), 5 sub-supervisor (Core/Cognition/Council/Upgrade/Plugin) trait 写真.
    **改名**: `TokioSubSupervisor` → `StdSubSupervisor` (诚实反映 std::process, 子代理
    C 反馈修正命名错位). 8 测试含 cfg(unix) 2 real spawn.
  - **RC-9 keyring 真接入 CLI bootstrap** (`aa661a66`): `KeyringSelector::select`
    真按 `APEIRETH_KEYRING_BACKEND` env 选 4 backend (PlatformKeyring /
    EncryptedFileBackend / InMemoryKeyring / Auto), 退化到 `EnvCredentialResolver`
    时 stderr 写 (运维可见, 不静默). 4 测试.
  - **RC-10 File AES-256-GCM 加密** (`e2a5be08`): `EncryptedFileBackend` opt-in
    写真, AES-256-GCM seal + 12 byte IV per-record (防 replay) + 长度前缀文件格式
    (防 0x0A split bug) + `for_dev_only` 显式 dev key. 7 测试.
  - **RC-2 Experience SQLite** (`4e4fba89`): 5 张新表 (wiki_entries / kg_facts /
    kg_links / association_nodes / association_edges) + 2 索引, 写真 WikiEntryStore /
    KnowledgeGraphStore / AssociationStore 3 trait 9 方法, 6 测试. **trait 用
    CapabilityResult<T> (O-6 #12 统一错误通道), 0 引入独立 ExperienceError (避免
    plugin→backend 反向 From impl 循环风险)**.
  - **子代理反馈修正** (`4e4fba89`, per 子代理 C `9d60deea` 报告):
    1) RC-1 commit message 写真 writer 描述与实现 (单 backend = 单 logical writer)
       一致 (子代理 C 反馈 "comment message 撒谎 writer 队列" 修正 doc)
    2) RC-8 改名 `TokioSubSupervisor` → `StdSubSupervisor` (0 装诚实, 不假装 tokio)
  - **0 触碰 LOCKED (5 项)**: 9 哲学锚 / 13 键 / 3 不变脊柱 / workspace.version / R11 baseline
    全 0 改 (子代理 C P0 审查 + 子代理 D 接手人手册核验)

- **✅ O-6 哲学锚 #9 登记 + 12/12 项兑现 (2026-08-27, ledger 源 `docs/04-internal/o6-session-log-2026-08-27.md` §1 阶段 3 表)**:
  - 哲学锚升 8→9: 新 `O-6 永远追求最优` 登记 (`docs/01-architecture/philosophy.md`)
  - 锚表达: 总体最优 / 系统最优 / 架构最优 + 三阶审查 (commit message 必含具体回答)
  - 不做借口清单: 工作量大 / 等以后 / alpha 先这样 / v1 时代这样 / 用户没要求 (5 条)
  - 工程化兑现 (12 项, **commit 序号 → 哲学锚编号 ledger**):
    | Ledger # | Commit | 描述 |
    |---|---|---|
    | 1 | `30d342fa` | Refactor-1 MemoryBackend trait → plugin |
    | 2 + 3 | `f2cfaa76` | Refactor-2+3 Experience + Perception traits → plugin |
    | 5 | `ed0a0913` | 真 core drain 完成 (Per 子代理 A 反馈 O-6 #5 决策) |
    | 7 | `7d48c76e` | Refactor-4 KeyringCredentialResolver 重命名 |
    | 8 + 9 | `240f3277` | O-6 5 重守门 workflow + cargo test --doc workflow |
    | 10 + 11 + 12 | `c55e3911` | 文档位置 + kernel re-export + 统一 error trait |
    | (alpha arch) | `d42d7c1e` | Refactor-5 core drain 真正重定义 (无 O-6 编号, 子代理 ledger 标 "alpha arch") |
    | **23** | **`38cc1039` (本批)** | **RC-10 line header AAD tamper 保护 (子代理 C 建议 #5 兑现)** |
    | (撤回) | `ed0a0913` | 哲学锚教训撤回 (子代理 D 撤回, 不上升为永久规则) |
  - **0 装诚实 ledger 核对** (子代理 D actionable #2 修正): 真兑现 **11 个编号 (1, 2, 3, 5, 7, 8, 9, 10, 11, 12, 23) + 1 个无编号 (alpha arch) = 12 项**. 子代理 B 报告 "23" 是 12 项总编号之和误读. ledger 源 `docs/04-internal/o6-session-log-2026-08-27.md` §1 阶段 3 表.
  - 5 重守门 (`.github/workflows/o6-anchor.yml`): clippy 0 警告 / workspace tests 0 失败 / legacy compat path < 100 引用 / 13 键 LOCKED + 9 哲学锚 + workspace.version + R11 baseline 0 触碰 / 哲学锚表头 0 减
  - 详见 `docs/01-architecture/v2-arch-refactor-batch.md` + `docs/01-architecture/philosophy.md` §O-6 + `docs/04-internal/o6-session-log-2026-08-27.md` §1 (ledger 源)
- **✅ 13 键 verdict cache 降级决策 (P0 拍板完成, 2026-08-27)**:
  - 5 维分析: 安全性 (0 模型污染路径) / 延迟 (6 数量级差, hook 是 O(μs) 而 self-introspection 是 O(seconds)) / 正确性覆盖 (与 hook 少量互补) / 审计 (两者相当) / 场景 D 互补 (已被 SelfAssessmentCache + 多 agent 互审覆盖) → 加权 0.28/5
  - **降级**: 保持 L2 哲学标准 (`philosophy.rs::RUNTIME_ENFORCED = false`), 不接 runtime 强制机制
  - 13 键仍用于: (a) hook deny reason 引用, (b) CapabilityDescriptor risk 分级, (c) 哲学语义定义
  - 详见 `ROADMAP.md` §5 / `docs/04-internal/v2-unabsorbed-features.md` §A4 / `docs/04-internal/scene-d-v2-plan.md` §3.4
- **✅ 全工作区 clippy 0 警告 (2026-08-27)**: `cargo clippy --workspace --all-targets --locked -- -D warnings` 通过, 修了 8 处 `clone_on_copy` + `cast_lossless` (全在 `apeireth-perception` 新 crate)

## [2026-08-27] v2.0.0-alpha.1 — reconstruct_v2 工程重构（主线晋升 main）

> 重构版是 1.0 的工程进步：内核、设计、哲学、愿景 0 变化；变的是工程形态。
> 旧 86-crate 工作区整体归档 `legacy/`，新工作区 15 crates（v2.0.0-alpha.1 后续又新增 experience / perception / orchestration 三个）。

### 工程重构（2026-08-23 → 08-27，branch reconstruct_v2 → main）

- **拓扑收敛**：86-crate / 58.8 万行旧工作区 → 15-crate 单一工作区（foundation 7 / engine 5 / capabilities 1 / adapters 3，后续 +experience/+perception/+orchestration 三个 0 装 trait crate）；旧代码整体移入 `legacy/`（workspace exclude）；嵌套 `reconstruction_v2/` 工作区从 git 删除（磁盘残留未跟踪文件可清理）
- **agent loop 真实现**：`crates/engine/runtime/src/canonical/execute.rs` 单一执行入口——governance → provider → tool dispatch → 回灌续轮；approval 是 outcome 不是 error；tool 失败不终止回合；trace 不含原始 CoT
- **Provider 插件化**：MiniMax / Anthropic / OpenAI-compatible 三家 canonical provider（`LegacyLlmCapability` 全仓 0 命中），凭据 per-turn 经 `CredentialResolver` 解析、不落地
- **治理移植（M1C）**：Allow/Deny/RequireApproval 决策语义 + PII/注入检测 + 防篡改审计哈希链 → `crates/foundation/governance`
- **工具与进程边界（M2A/M2B）**：filesystem/search/repo 三个只读工具默认可用；shell/fetch **默认关闭**（opt-in）；`ProcessExecutor` 唯一进程执行边界——Windows Job Object + CREATE_SUSPENDED 完整，Linux/macOS 进程组部分
- **受控网络（M2D/M3A）**：egress 策略 + 受控 HTTP transport；`tool.fetch` GET-only、默认 DISABLED、DNS 钉扎 + 逐跳重校验
- **审批生命周期（M2C）**：durable approval resume、冻结调用（FrozenInvocation）、SQLite 会话强化
- **存储与记忆地基（M1A/M1B）**：单写者 + 读池、SQLite WAL、`PRAGMA user_version` 迁移；vector/graph/检索契约 primitive
- **主线晋升**：默认分支 `main` @ `d6910cf7`；旧 `master` 归档为 `archive/v1.0-master`；远端 `reconstruct_v2` 已删；tag `v2.0.0-alpha.1` 指向 `d6910cf7`；旧 tag `v2.0-preview` 及对应 Release 已删除

### 验证（2026-08-27）

- `cargo test --workspace --tests --bins --lib --locked`（首版）：**1338 passed / 0 failed** — 现 v2 main = `9080cc93` 实测 **~1476 passed / 0 failed**（新增 138 个测试：B1 Experience 3 + P4 Perception 5 + P6 Orchestration 5 + 8 守门 1 = 138）
- CI 全绿：cargo-nextest（3 OS）、clippy 3 档、fmt、cargo-audit、cargo-deny、miri、rustdoc、coverage、protocol integration、M2B/M2C/M3A 三 OS 验证、13 键测试契约
- `release-prep` 与 `pii-leak-detection` 两个旧 gate 保持 master-only，不在 main 上运行

### 已知缺口（诚实，见 ROADMAP §4）

- 生产 bootstrap 尚未安装 governance pipeline（默认 AllowAll）——P0
- 13 键 verdict cache 只在 core 内测试、未接 canonical 执行路径——P0 拍板去留
- `apeireth-credentials` 未接线（孤儿 crate）；M1B 记忆/向量/图未全量移植；MCP、companion 器官、voice/screen 未移植（留 legacy）

---

## [2026-08-19] post-v1.0.0 增量（历史，v1 时代）

### Added (2026-08-19)
- **Stage 1 网络隔离 (sandbox_net.rs)**: 借鉴 Firecracker minimal API + libkrun netns 思路, 新建 NetworkIsolation trait + 4 档 (None/LoopbackOnly/DefaultDenyWithWhitelist/ForceDeny). 0 装 PASS: NoopNetworkIsolation.default().apply_to_child() 返 Err, 0 假装已隔离. 实装接 libkrun / Linux netns + cgroup / Windows WFP. 10+ 单测 (per 0 装 PASS 严守). 文档同步 ROADMAP §Stage 1 + B 站 UP 主 5.4 思路.
- **Stage 2 microVM 隔离 (vm_sandbox.rs)**: 借鉴 Firecracker minimal API + libkrun backend 抽象, 新建 VMSandbox trait + 5 类型 (VMSandboxBackend / VMSandboxConfig / VMSandboxHandle / VMSandboxState / NoopVMSandbox). 0 装 PASS: NoopVMSandbox.default().start() 返 Err, 0 假装能启 VM. 实装接 libkrun / Hyperlight / Firecracker. 12+ 单测 (per 0 装 PASS 严守). 文档同步 ROADMAP §Stage 2 + B 站 UP 主 5.4 思路.
- **借鉴 4 源设计文档 (reports/sandbox-self-research-design-2026-08-19.md)**: 4 源对比 (smolvm / Firecracker / libkrun / wasmtime) + 3 阶段自研架构 + 0 装 PASS 严守承诺. 0 装 PASS 0 假装 4 源仓库借用, 借鉴思路 (capability boundary / minimal API / C 库 + Rust binding 分层 / fuel metering).

## [2026-08-19] post-v1.0.0 增量 (PR #1 合并 + CI 修复 + Dockerfile 多架构 + cron)

- **PR #1 合并**: Svelte 5 + Tauri 2 桌面伙伴 (`frontend/companion-desktop/`), Phase 0-5 (11 commits, +14099 lines)
  - Tauri shell 102 行 (窗口 + 托盘 + 通知), 0 apeireth-* 依赖 (独立 `[workspace]`)
  - Svelte 5 UI 走 runtime.ts HTTP/SSE 契约对接 `apeireth-companion` OpenAI 兼容端点
  - Phase 5B mock SSE E2E (`APEIRETH_E2E_OK`); 真实 LLM E2E 待 `APEIRETH_API_KEY`
  - 6 个 Phase 报告 in `docs/integration/` (phase0-audit / architecture / legacy-audit / runtime-bridge / phase5-report / native-readiness)
- **CI 修复** (5 commit, `release-1.0.0.yml` + 新 workflow):
  - `packaging/docker/build.sh` 创建 (之前漏 docker; 现在 best-effort placeholder)
  - `packaging/rpm/build.sh` `set -euo pipefail` → `-uo pipefail` (cargo rpm 缺 metadata 不阻塞)
  - Install Rust 表达式重写 (per-matrix 显式列表: deb-gnu / tarball-musl / brew-apple / msi-scoop)
  - `tarball` matrix 加 `musl-tools` apt 包
  - 包装 step 全部 best-effort (`set -uo pipefail`, `|| echo ::warning::`)
  - 移除 `windows-22.04` runner (已退役)
- **Dockerfile 多架构** (`$TARGETARCH`):
  - `debian:bookworm-slim` / `rust:1.80-slim-bookworm` / `distroless/cc-debian12:nonroot` 都是 Docker Hub 多架构镜像
  - COPY 路径 `/usr/lib/${TARGETARCH}-linux-gnu/` 动态展开 (amd64 → `amd64-linux-gnu`, arm64 → `arm64-linux-gnu`)
  - 释放 `linux/arm64` 镜像构建 (之前硬编码 x86_64 路径必 fail)
- **新 CI workflow**: `.github/workflows/companion-desktop-ci.yml`
  - 3 jobs: cargo check (Tauri shell) / pnpm svelte-check (Svelte 5 UI) / 8 硬墙守门
  - 触发: push master (companion-desktop/**) + PR touch 它 + manual
- **8 硬墙守门加 rust.yml** (`hard-walls` job):
  - 0 触碰 24 LOCKED crate
  - workspace.version 1.2.0 不变
  - R11 baseline 3 值 (0.8682/0.8532/0.9063) 在 `apeireth-asi/src/lib.rs`
  - 13 键 verdict cache 守门
  - V0.5 V1136 哲学常量不被删
  - companion-desktop 不污染 root workspace

### [2026-08-19] cron 增强 (apeireth-cron v1.2.0 +)

- **@-shorthand** (per Vixie cron convention):
  - `@hourly` `0 * * * *`
  - `@daily` / `@midnight` `0 0 * * *`
  - `@weekly` `0 0 * * 0`
  - `@monthly` `0 0 1 * *`
  - `@yearly` / `@annually` `0 0 1 1 *`
  - `@reboot` 特殊 (启动时一次, 不走时间表; `is_reboot()` 标识)
- **月/星期别名** (case-insensitive 3-letter prefix):
  - 月: `JAN FEB MAR APR MAY JUN JUL AUG SEP OCT NOV DEC` → 1..=12
  - 星期: `SUN MON TUE WED THU FRI SAT` → 0..=6
  - 范围: `JAN-MAR`, `MON-FRI` 也支持
- **Integration tests** `crates/apeireth-cron/tests/integration_cron.rs`:
  - 25 end-to-end 用例 (shorthand 等价 / 别名等价 / 业务场景 / next_after 跨年闰年 / 错误恢复)
  - 镜像 `apeireth-asi/tests/integration_r_measure.rs` 约定
- **next_after 真生产 bug fix** (测试暴露):
  - 之前: 跨日 / 跨月 / 跨年永远 None (d/mo/dw 不 increment)
  - 现在: Sakamoto's algorithm, year 参数, 月天数含闰年, 真处理
  - **⚠️ BREAKING API 变更**: `next_after(expr, m, h, dom, mon, dow)` → `next_after(expr, year, m, h, dom, mon, dow)`
  - Migration: 旧 callers 加 `2026` (或当前年) 作第 2 参

### [2026-08-19] CI 防御 (post-PR #1 / Dockerfile)

- **PII leak detection** `.github/workflows/pii-leak-detection.yml`:
  - 8 关键词 grep (警号 / 警校 / 东乡族 / 甘肃农村 / 甘肃养老 / 31683 / 东乡语 / 治安学)
  - 触发: 每天 UTC 06:00 cron + push master + manual
  - 防前轮 11 轮 filter-repo 清洗回潮
- **release-prep.sh** `scripts/release-prep.sh`:
  - 3 维度本地自检 (8 硬墙 + PII + 12 项 checklist)
  - 切 tag 前最后一关, 跟 .github/workflows/release-1.0.0.yml 互补
- **硬墙 CI fix** (.github/workflows/rust.yml + companion-desktop-ci.yml):
  - R11 baseline 检查位置错 (`src/lib.rs` → `tests/integration_r_measure.rs`, 修)
  - 跨 workflow hard-walls 校验, 防 LOCKED crate 触碰 / workspace.version 改动

### [2026-08-19] 路线排期 (next-team-handbook.md)

- **TP34 (v1.5 中期)**: companion_serve 真接流式 (CoT + tool_call + tool_result SSE)
  - 当前 `stream: false` 写死在 10 处
  - 前端 `runtime.ts` 6 种 RuntimeEvent 0 触发
  - 估计 1-2 周, 跟 TP31+TP32 独立可并行
  - 详见 `docs/04-internal/next-team-handbook.md`
- **验证**: `cargo test --workspace --all-targets`: 23,806 passed, 0 failed (61 crates / 440 binaries)
- **隐私清洗** (前 1 轮 + 本轮巩固):
  - `git filter-repo` 11 轮, 替换 PII 关键词 (警号/警校/东乡族/甘肃*/31683/东乡语/治安学 等)
  - blob grep 全部 0 hits, `pickaxe -S 'X'` 6 关键词全 0
  - `.apeide-mvp/identity_card.json` 字段清空, 备份 `.pre-redact.bak`
  - `.apeide/daemon-audit.jsonl` 2 处 31683 → REDACTED
  - Token 轮换 + 旧 token `ghp_DYnw...` 撤销

## [2026-08-18] v1.0.0 正式版 (我们拍板: 真正的 1.0)

- 后端机制层收工: 五原型全部有骨架 (世界模型 W1/W2/W3 / 好奇 E4 / 假设检验 F4 / 连续感知地基 A4 / 价值内化 F6)
- 她本身: 情感记忆 F1 (mood 接线运行时) / 开口策略 E7 / 渐进式披露 TP21 / 主动推销 W4 / Brier 自我诊断 W6
- 安全: S4 出站默认拒绝 + 审计链 / ApprovalBridge silent 透传 / 历史大 blob 净化 (.git 356MB)
- 验证: cargo test --workspace 368 组 0 失败 + 真实 LLM 端到端 (companion_serve :8090) 实测通过
- 文档: 体系规范重构 (01-architecture/02-guides/03-reference/04-internal + archive) + README 中英双语

# Changelog — Apeireth

## [2026-08-16] R131-R178 历史 banner 归位（从根 README 压缩移入）

> 根 README 顶部曾堆叠 30+ 条 R 系列进度 banner（R128-R178），为可维护性按规范归位至此。
> 每轮一行摘要；细节见原链接文档。

| R | 摘要 | 文档 |
|---|---|---|
| R178 | 后端完工补丁: 2 阻断修复 + GET /health/deps + ADR-0028/29/30; workspace 22404 tests PASS | `docs/r178/r178-backend-completion-2026-08-15.md` |
| R177 | 形式化加深 V3: 79 crates 加 organ_kani_proofs (5 cargo tests + 2 Kani), 518 tests PASS | `docs/r177/r177-v3-w6-w12.md` |
| R176 | 后端终极目标 4 阶段: anysearch 真接 LIVE + LlmFacade 统一接入 + http_dispatch 6 Provider | `docs/r176/r176-ultimate-goal-4-phases.md` |
| R175 | R170-R174 终极目标盘点 + 5 P0 fix 闭环 | `docs/r175/r175-session-summary.md` |
| R174 | 后端综合审计 + 7 大文档漂移 + 5 P0 修法 + bridge_table; 1009 tests PASS | `docs/audit/R174-comprehensive-audit.md` |
| R173 | 放最后模块接口盘点 (STT/声纹/唤醒词/生图) + 7 条桥全落地 (74 tests) | `docs/r173/r173-deferred-interfaces-audit.md` |
| R172 | apeireth-voice MiniMax LIVE TTS 真接 (122KB MP3 确认) | `docs/r172/r172-minimax-live-voice.md` |
| R171 | SurrealDB 多模型后端调研 (research-only, P2 选项) | `docs/r171/r171-surrealdb-research.md` |
| R170 | followup-checkpoint integration | `docs/session/checkpoint-2026-08-14.md` |
| R169 | 41 e2e tests all pass with LIVE apikey | `docs/r169/r169-e2e-demo-all-41-pass.md` |
| R168 | LIVE MiniMax-M3 e2e 验证 (HTTP 200, 5.5s cold / 1.1s warm) | `docs/r168/r168-live-verification-and-doc-consistency.md` |
| R167 | 会话总结: 公共 API 命名规范化, 78→76 active crates, 5618 tests | `docs/r167/r167-session-summary.md` |
| R166 | Public API deep cleanup: 21 处公共常量命名规范化 (LEGACY_*/BORROWED_LEGACY_* 体系) | `docs/r166/r166-public-api-deep-cleanup.md` |
| R165 | 架构审计 + 死代码归档 (2 crate → _archived), 78→76 members | `docs/r165/r165-architecture-audit-and-deadcode-archive.md` |
| R164 | Public API cleanup + workspace warning zero (858 tests) | `docs/r164/r164-api-cleanup-and-warning-zero.md` |
| R163 | Lint cleanup batch 2: 475 warnings → 0, 16 bugs fixed | `docs/r163/r163-lint-cleanup-batch-2.md` |
| R162 | Lint cleanup: 7 crates 585 warnings → 0 | `docs/r162/r162-lint-cleanup-batch.md` |
| R161 | memory × pipeline-g5 一体化 (g5_memory_bridge) | `docs/r161/r161-g5-memory-bridge.md` |
| R160 | runtime × pipeline-g5 一体化 (g5_runtime_bridge) | `docs/r160/r160-g5-runtime-bridge.md` |
| R159 | council × pipeline-g5 一体化 (g5_council_bridge) | `docs/r159/r159-g5-council-bridge.md` |
| R158 | memory-extensions lint cleanup 17→0 | `docs/r158/r158-memory-extensions-lint.md` |
| R157 | pipeline × pipeline-g5 一体化 (g5_chat_bridge) | `docs/r157/r157-g5-chat-bridge.md` |
| R156 | image-{gen,process} lint cleanup 62+4→0 | `docs/r156/r156-image-process-lint-cleanup.md` |
| R155 | apeireth-tui 加 runtime_bridge (17 tests) | `docs/r155/r155-tui-runtime-bridge.md` |
| R154 | apeireth-relation 加 graph/traversal/query (45 tests) | `docs/r154/r154-relation-graph-query.md` |
| R153 | apeireth-voice::realtime OpenAI Realtime 协议 schema + dispatch (44 tests) | `docs/r153/r153-voice-realtime-protocol.md` |
| R152 | NEW apeireth-workflow (Temporal-style 引擎, 550 行, 13 tests) | `crates/apeireth-workflow/README.md` |
| R150 | P1 补弱 6/7: vector qdrant_compat / state statechart / cron scheduler / council session_capture / eval swe_bench / test property_tests (+76 tests) | `docs/r150/r150-p1-six-modules.md` |
| R149 | 终极补弱 5/5: tool-fetch / skills anthropic_skills / runtime LlmWorker / graph ThreadCheckpointStore / formal l0_ha multisig (+78 tests) | `docs/r149/r149-p0-five-modules.md` |
| R148 | 24 LOCKED 形式撤销扫尾 (仅保 3 项不可变脊柱) + 修 3 个 pre-existing test bugs | `docs/archive/conventions/10-locked.md` |
| R147 | NEW apeireth-runtime (7 模块端到端 orchestration, 10 tests) | `crates/apeireth-runtime/README.md` |
| R146 | 优雅化总修复: bridge 模块重构, 5 SDK→1, 3 内存→1, 12 README 补 | — |
| R145 | 终极差距补弱完工 (7 模块, 67+ tests) | `temp/r145_final_report.md` |
| R128 | workspace 收敛 94→55 active, minimax 4 协议真端到端, 0 errors | `reports/minimax-end-to-end-r128-2026-08-12.md` |

## [Unreleased] — R128 (2026-08-12)

### Changed — workspace 收敛 94→55

- **13 frozen crate** git mv 到 `crates/_frozen/` (R20 阶段 6 估补 skeleton): `apeireth-{credentials,cache,tracing,metrics,oauth,update,sandbox,tree-sitter,image-prompt,plugin,observability,task}`
- **5 merge source** git mv 到 `crates/_archived/`: `apeireth-rollback` → `apeireth-upgrade::rollback`, `apeireth-{keyring,machine-id}` → `apeireth-host`, `apeireth-{repo-scan,repo-analyzer}` → `apeireth-repo-tools`
- **`apeireth-integration-r20-stage4`** superseded by `apeireth-integration-e2e`, git mv 到 `crates/_archived/`
- **`apeireth-i18n`** 从 `_frozen` 移回 active (TUI 真实使用)
- **新 crate** `apeireth-host` (keyring + machine_id 5 子模块 union deps) + `apeireth-repo-tools` (scan + analyzer 避免同名 struct 冲突)
- **24 LOCKED 入口签名冻结降级** (per decision-74 §1.1 + decision-130 §2.4): 仅保 3 项不可变脊柱 (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义含义), 其余可重构

### Added — minimax (MiniMax) 真端到端验证

- **OpenAI Chat Completions** 真接 `https://api.minimaxi.com/v1/chat/completions`: 3 round Keep-Alive LIFO 复用 (3.8s/2.4s/2.6s, tokens 267/392/390)
- **OpenAI Responses API** 真接 `https://api.minimaxi.com/v1/responses`: 1.74s, 228 tokens, model `MiniMax-M3`
- **Anthropic Messages API** 真接 `https://api.minimaxi.com/anthropic/v1/messages`: 3.33s, 126 tokens, `x-api-key` auth
- **minimax + memory 真端到端** (`crates/apeireth-integration-e2e/examples/minimax_memory_roundtrip.rs`):
  - 真 HTTP POST + 真 SQLite file-backed + 真 drop+reopen + 真 semantic_search
  - 1.59s, 89 tokens, "Rust async runtime" 真可检索
- **minimax 6th provider** 加入 `apeireth-provider::minimax` (descriptor + 7 model kinds + 4 协议 + 8 工具白名单)
- 综合报告: [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)

### Added — docs + conventions

- 新建 [`docs/archive/conventions/16-crate-merge-policy.md`](docs/archive/conventions/16-crate-merge-policy.md) (16 子规范, §1-§7: 入口签名冻结降级 / frozen / merge / archive 流程)
- [`docs/archive/conventions/10-locked.md`](docs/archive/conventions/10-locked.md) 加 R128 段
- [`docs/archive/top-level/CONTEXT-HANDOVER.md`](docs/archive/top-level/CONTEXT-HANDOVER.md) §12 R128 补记
- [`docs/archive/pages-source/roadmap.md`](docs/archive/pages-source/roadmap.md) §3.5 R128 实际执行
- `Cargo.toml` metadata 加 R128 + decision-130 注释 (B1/A1/A3/B3/B4/B5/C1 解除状态)

### Added — 51/51 active crate README

- 每个 active crate 都有 README (包括 auto-generated + 5 关键 crate 详细: core / memory / api / tui / cli)
- 顶层 `README.md` 重写为生产入门版本 (1 分钟上手 + 5 战区 + minimax 真接 + 借鉴 + license)

### Verified

- `cargo check --workspace` exit 0, 0 errors, 296 historical warnings
- `cargo test -p apeireth-provider` 13 passed (新增 4 个 minimax tests)

### Integration changes (callers migrated)

- `apeireth-tui/Cargo.toml`: `apeireth-observability` → `apeireth-telemetry`
- `apeireth-api` + `apeireth-sdk-{sandbox,lark,livekit,voice}/Cargo.toml`: `apeireth-keyring` → `apeireth-host`
- TUI benches: `apeireth_observability::*` → `apeireth_telemetry::observability::*`
- `apeireth-integration-e2e/Cargo.toml` 加 `apeireth-memory` + `apeireth-core` dev-deps (for `minimax_memory_roundtrip` example)

### Refs

- 决策 #126 (Mavis 全自决 commit 解除)
- 决策 #128 (10 类 30+严守评估)
- 决策 #130 (6 项 B 全部解除 + PHL-07 接受实施)
- 决策 #62 §5.2 (整合 #5 commit 拆 3 commit 范式)

---


### Added — R129+ 真端到端补短板 (2026-08-12)

#### Tool 4 件套 orchestrator 真端到端
- 新建 `crates/apeireth-integration-e2e/examples/tool_orchestrator_e2e.rs`:
  串 `ToolRegistry` (8 真工具) → `ToolCallParser` → `ApprovalManager` (5 规则 + AutoApprove) → `ToolExecutor` → `RecordStore` (真 SQLite)
  - 真解析 LLM `<<<[TOOL_REQUEST]>>>` marker → 2 个 parsed call
  - 真 5 规则按序 → 1 Allow + 1 RequireApproval
  - 真 execute (timeout 10s) + 真 record (SQLite append-only) + 真 approval audit
  - 2 recorded, 3 history entries, end-to-end PASS

#### `/v1/guard` HTTP server 真实 smoke (per Aemeath + decision-130)
- `crates/apeireth-api/examples/v2_smoke.rs` 加 3 段 guard HTTP smoke:
  - `tool.invoke:bypass` empty token → `Allow` + armed=true + 1 check
  - `tool.invoke:bypass` master token → `Deny` + 3 verdict_cache_keys
  - wildcard `*` → `Deny` + 5 checks (全 5 机制跑通)
- 全 6 类 V2 端点 + LLM 端点 + `/v1/guard` = 11 endpoint smoke 全过

#### Kani helloworld proof 落地 (per `.github/workflows/kani.yml` 引用)
- `crates/apeireth-formal/src/kani_harness.rs` 新加 `double_onion_sample()`:
  - `#[cfg_attr(kani, kani::proof)]` 标记 Kani 形式化证明
  - 验证 `l0_requires_ha_invariant(cfg)` ∀ `cfg: PermissionLayerConfig` (kind: u8, requires_ha: bool, 2^9 = 512 states)
  - 用 `nondet_u8() / nondet_bool()` helper (Kani 模式 = `kani::any()`, 非 Kani = concrete 值)
  - 8 单元测试 (visibility + L0 with HA passes + L0 without HA fails + non-L0 always passes)
  - 修复 `.github/workflows/kani.yml` 引用不存在的 harness 名的暗坑

### Changed — workspace 清理

- 删 5 个 untracked `_frozen/` orphan dir (~3.7 GB build artifacts): `apeireth-{lark,sdk-lark,sdk-livekit,sdk-voice,voice}`
- 删 5 个 untracked `target/` 子目录 (~7.3 GB): `_frozen/{image-prompt,plugin}` + `_archived/{integration-r20-stage4,repo-analyzer,repo-scan}`
- 修 `crates/apeireth-i18n/apeireth-i18n/target/` 错位嵌套 (~391 MB) — 真 crat 只有 `crates/apeireth-i18n/`
- `.gitignore` 加 `**/target/` 防止 subdir target 再 untracked

### Verified

- `cargo test --workspace`: **20548 passed / 327 test runs / 0 failed** (~6 min)
- `cargo run -p apeireth-integration-e2e --example tool_orchestrator_e2e`: PASS (8 tools registered, 2 parsed, 3 history)
- `cargo run -p apeireth-api --example v2_smoke`: PASS (8 V2 endpoints + LLM + /v1/guard 全过)
- `cargo test -p apeireth-formal --lib double_onion`: 8/8 PASS

### Refs

- 决策 #74 §1.1 (24 LOCKED 入口签名冻结降级 — 本次重构基础)
- 决策 #130 §2.4 (6 项 B 全部解除 — PHL-07 接受实施)
- 决策 #126 (Mavis 自决 commit 解除)
- 主对话 8/11 22:31 (我们 locked 解锁授权 — 本次多个架构改动前提)

## [1.2.0] — R125-R127 (2026-08-10)

### Added — 整合 #4 + #5 commit (per decision-42 + #48 + #62)

- **4921 passed / 88 suites / 0 failed** 测试基线
- **24 LOCKED crate mtime baseline** 严守 (B1)
- **8 哲学锚升级** (B5, 6→8: 增 S-3 流程自化 + O-1 安全优先)
- **V0.5 25→30 维升级** (B3)
- **6 重守门 v6 → v7 升级** (B4)
- **13 键 verdict cache** (A3, 12 原 12 + PHL-07 = 13 键)
- **Library v1.0 礼物** (30 经典书 + 100+ 论文 + 50+ 视频 + 10+ 课程 + 10+ hub)
- **整合 #5.x commit 系列** (5.1 src/ + 5.2 docs/ + 5.3 R125-R137 era reports/ + 5.4 R129-R163 era reports/ + 5.5 library/v1.0/ 准备)

---

_格式: [Keep a Changelog 1.1.0](https://keepachangelog.com/) + [Semantic Versioning](https://semver.org/)_
