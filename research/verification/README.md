# Phase 5 审批状态机 —— 形式验证总览 (RA-5)

三路互证的证据链, 全部指向同一条不变量 (InvA 无双副作用 / InvB 批准意图不丢 /
InvC 效果不确定强制 fail-closed):

| 路 | 方法 | 状态 | 强度 |
|---|---|---|---|
| ① 模型级故障注入 | `research_approval_sm.rs` fault injection (6 持久化点 × 100 轮随机事件/崩溃交错) | ✅ 0 违例 (Phase 5 交付时) | 随机采样, 概率保证 |
| ② TLA+/TLC 穷举 | `tla/ApprovalSM.tla` + TLC 2.16 (JDK 17) | ✅ 2026-09-05: 单记录 36 状态 / 三记录 3164 状态全通过 | **全可达状态** 枚举, 指纹碰撞 2.9E-12 |
| ③ Kani 机器证明 | `kani/` mirror crate (零复制 `#[path]` 包含 canonical) + GitHub Actions | ✅ 2026-09-05: **3/3 harness 全部 VERIFICATION SUCCESSFUL** (run 33945573291) | 符号执行, 有界展开 (unwind 32) |

## 关键工程事实 (2026-09-05)

- **Kani 平台限制**: 本机 Windows 无 Kani (仅 Linux/macOS) → CI ubuntu runner 跑;
  TLC 本机 JDK 17 跑通。
- **rust-version 墙**: workspace 要求 rustc 1.97, Kani 0.67 (crates.io 最新) 自带 nightly 1.93
  → 用 `kani/` mirror crate (`#[path]` 直接包含 canonical 源文件, 零复制漂移) 绕开。
- **SipHash 符号展开爆炸**: Kani 把 String 字节缓冲当符号长度, HashMap 哈希循环无限展开
  (CI 日志实测 1900+ 迭代 × 2s) → 3 个 harness 加 `#[kani::unwind(32)]` (仅 cfg(kani) 生效,
  生产零影响)。真实执行用具体短键, 32 覆盖全部真实执行; 属**有界模型检查**口径, 与②互补。
- **TLC 枚举要求**: primed 变量须可赋值形式 (`v' \in S`), 规格内 Tick 已等价改写并注释。

## 复现

```powershell
# ② 本机 TLC (JDK 17 + tla2tools-1.7.1.jar, MIT, 自取见 tla/README.md)
java -cp tla2tools-1.7.1.jar tlc2.TLC -config ApprovalSM3.cfg ApprovalSM.tla
# ③ Kani CI: .github/workflows/kani.yml (workflow_dispatch 可手动触发)
```

## 诚实边界

- 三条路都建立在**同一份状态机语义**上 (TLA+ 规格与 Rust 实现 1:1 对照见 `tla/README.md`
  映射表); 跨路一致性 = 语义编码没写错 + 不变量确实成立。
- 崩溃模型是 durable 前缀布尔抽象, 未建模真实 fsync/部分写 (见 canonical 文件学术账本)。
- Kani 为有界展开口径 (unwind 32), TLC 为有限模型 (REC ≤ 3, 时钟有界);
  无限模型归纳泛化未做, 如实标注。

---

# 性质族扩展 (2026-09-26): panic-freedom / 记忆守恒 / 治理 / 配额 / 沙箱 / SAGA

三通道口径不变 (Kani mirror 零复制 + TLC 本机 + 本地桩), 验证目标扩展为六条性质族。
Kani harness 全部 `#[cfg(kani)]` 门控, assertion 即命题, 输入由 `kani::any()`
生成并显式有界; 形状前置条件写作早退守卫 (与 `kani::assume` 语义等价, 命题为
cond ⇒ P); 每个 harness 注释一句话写明"证明什么、边界是什么"。

本地新增可跑层: `kani-typecheck/` (kani API 桩 crate) —— `#[path]` 引入 mirror
同一份 harness 源, `--cfg kani` 下做全量类型检查 + 默认值输入冒烟执行;
真实符号证明仍由 CI (ubuntu) 完成。

## 性质族 1 · Panic-freedom (对应"零未捕获异常")

| 命题 | harness | 边界 (有界/unwind) |
|---|---|---|
| 任意 ≤8 字节串经 `has_fold_markers`/`parse_fold_blocks`/`render_fold_blocks` 无 panic, 渲染记账恒等 (expanded+hidden==blocks) | `kani_panic_free_fold_block_string_pipeline` | 8 字节, unwind 64 |
| `OrthogonalResidualPyramid::analyze` 对 ≤3 维有限查询 + 有界召回闭包无 panic, 输出信号均落 [0,1], 层数 ≤ max_levels | `kani_panic_free_residual_pyramid_analyze` | 维 1..=3, 召回 2 向量, unwind 64 |
| `SemanticAxisBridge::{fit, project}` 无 panic, 归一化熵/逻辑深度落 [0,1], 长度不匹配查询走早退 | `kani_panic_free_semantic_axis_fit_project` | 维 1..=3, 样本 ≤3, 形状契约见观察 1, unwind 96 |
| `RiverObservability::measure_omega` 对任意计数与任意 f32 流量 (含 NaN/±inf) 无 panic; 空拓扑恒 (0.0, Collapsed) | `kani_panic_free_river_measure_omega` | 流量 ≤3, unwind 32 |
| `DualScaledFieldSolver::solve` 方阵输入无 panic, 输出两场长度 == 维度 | `kani_panic_free_dual_scaled_field_solve` | 维 1..=3, 迭代 2 步, 形状契约见观察 2, unwind 48 |
| `TransparentFileFetcher::compute_cache_key` 无 panic, 输出 64 位小写十六进制 (SHA-256 格式健全) | `kani_panic_free_compute_cache_key` | URL 2 字节 (单 SHA-256 压缩块), sha2 force-soft 后端, unwind 128 |
| `base64_decode` 对任意 ≤8 字节串无 panic, 唯一失败模式 Decode 错误 | `kani_panic_free_base64_decode` (随 `file_fetcher.rs` 内 `kani_base64_proofs`, 见观察 3) | 8 字节, unwind 48 |
| `AsyncContextPipeline::{push_message, assemble_prompt_context, post_inference_cleanup}` 无 panic, 装配守恒 (组装条数==总条数; 清理后仅剩 durable+summary) | `kani_panic_free_async_context_pipeline` | ≤3 条消息, unwind 48 |

## 性质族 2 · 记忆守恒 (对应"记忆不会丢")

真实 protect/forget/retention 转移规则在 SQLite 路径
(`memory_governance::forget_episode_impl` 的 Protected/AlreadyForgotten 前置检查、
`retention::sweep_session` 的 protected 跳过、`universal_forget` 的事务边界),
Kani 无法驱动 DB。依任务口径降级为同 crate 纯 API
`bitemporal_graph::BitemporalGraph` (撤回 = 墓碑版本, 永不物理删除) 上的纯模型
投影; SQL 转移规则本身由 `tla/ProtectForget.tla` 全状态空间直接建模 (见下)。

| 命题 | harness | 边界 |
|---|---|---|
| 撤回 (forget) 永不物理删除旧版本 —— 每个先前版本仍在自身信念切片可见 (append-only 守恒) | `kani_memory_retract_never_loses_versions` | 具体短键 (符号键会触发 SipHash 展开), 时刻严格递增 (单调时钟), unwind 64 |
| forget 幂等 —— 二次撤回与一次撤回的任意双时态切片 (facts_as_of/retrospective) 完全一致 | `kani_memory_forget_idempotent_observational` | beliefs_as_of 属 append-only 审计维度, 按设计记录每次操作, 不在幂等域 (观察 5) |
| forget 只作用于目标键 —— 无关键 (protect 语义纯模型投影) 在任意切片的可见集不变 | `kani_memory_forget_confined_to_target_key` | 两对具体短键, 每键 ≤2 写 + 1 目标键操作, unwind 64 |

## 性质族 3 · 治理单调性 (对应"默认拒绝")

策略评估入口 (`PermissionPolicy::decision_for_capability` /
`ApprovalPolicyEngine::evaluate`) 返回 canonical `Decision`, 其编译依赖闭包
(apeireth_core 等) 无法零复制装进 mirror crate; 依任务口径降级为
`crates/foundation/governance` 纯判定函数的命题证明 (映射边界: 三态 Decision
到纯核的装配在 guard/semantics, 不在量词域)。

| 命题 | harness | 边界 |
|---|---|---|
| 任一策略轴置 Deny 后, 其余任意参数 (意图类别/显式度/操作集/其余策略轴/时间戳) 变化不得使该轴放行 | `kani_governance_deny_axis_never_allows` | 量词域 = `TaskIntentEnvelopeV1` 全部非策略字段符号组合, unwind 32 |
| 默认拒绝: 信封未声明任何操作时, 任何效果类操作恒不放行 (任意意图类别/策略轴) | `kani_governance_default_deny_unrequested_ops` | Read/Search 只读例外与 `allows_publish` 的 intent_class 设计内旁路不在量词域 (观察 4) |
| 风险标签单调: 任意 (原始,提案) 标签字节串, 严格降级必被 `check_no_degrade` 触发; Pass 蕴含未降级 | `kani_governance_no_degrade_never_weakens` | 标签任意 ≤4 字节串 (对抗拼写变体是量词域核心), unwind 32 |
| fail-closed 短路: verify 失败则 prepare/apply 均不执行, prepare 失败则 apply 不执行 ("RequireApproval 不得跳过审批直接放行"的纯判定投影); 全过 ⇔ Ok | `kani_governance_fail_closed_no_bypass` | 三阶段结局 2³ 全量化, 阶段体为探针 |

## 性质族 4 · 配额与调度安全 (`cognitive_quota_scheduler`)

| 命题 | harness | 边界 |
|---|---|---|
| 配额记账单调不减 (各维度不为负、不回绕), 耗尽状态粘滞, `consume_step` 返回值与 `is_exhausted` 一致 | `kani_quota_ledger_monotone_non_negative` | 记账为无符号 saturating 累加, 非负性由构造保证, 证明其非平凡面; 2 次扣减, 数值任意 |
| 首次出队必为最高紧急度层最早提交者 (紧急度优先 + 平级 FIFO; 高优先级不被低优先级抢先) | `kani_quota_highest_priority_dispatched_first` | 3 任务有界情形; 无限任务流公平性是活性质, 不在安全不变量域, unwind 48 |
| PIP 防反转: `boost_priority_for_lock` 提升后的持锁者先于请求者出队, effective 继承请求者优先级 | `kani_quota_pip_boost_prevents_inversion` | 2 任务, 请求者 P0..P2 (提升条件恒满足), unwind 48 |

## 性质族 5 · 路径沙箱不逃逸 (`sensitive_path` / `file_fetcher`)

| 命题 | harness | 边界 |
|---|---|---|
| `validate_path_safety` 精确刻画: 无白名单时拒绝 iff 含 `..` 或 NUL (穿越必拒, 且不过度拒绝), 错误型恒 PathTraversal | `kani_sandbox_validate_rejects_traversal_and_nul` | 路径 8 字节, unwind 32 |
| 白名单模式下根外路径必拒: 通过 ⇒ 词法位于允许根之下 (且不含穿越/NUL) | `kani_sandbox_whitelist_rejects_outside_root` | 单允许根, `starts_with` 词法口径 (不含符号链接/fs 解析) |
| 敏感目录封闭性: `.ssh/.aws/.gnupg/.secret/.secrets/.kube/.docker` 与 `.config/gcloud` 之下任意子名必拒 | `kani_sandbox_sensitive_dir_blocks_any_child` | 单段文件名分量 ≤4 字节 (Path::join 单段名契约, 含分隔符/盘符时 join 改写根属另一语义) |
| 敏感后缀封闭性: `.key/.pem/.p12/.pfx/.jks/.kdbx` + 任意前缀必拒 | `kani_sandbox_sensitive_suffix_blocks_any_name` | 同上 |
| 凭据模式封闭性: `.env.{任意}` / `id_rsa.{任意}` / `{任意}apikey` 必拒 | `kani_sandbox_credential_pattern_blocks_any_affix` | 同上 |

## 性质族 6 · SAGA/CoW 回滚精确性 (`causal_world_model`)

`fork_branch`/`commit_branch`/`rollback_branch` 的真实形态是分支级 CoW +
SAGA 补偿栈: rollback 仅作用于**未提交**分支 (abort: 世界恢复 fork 基线,
补偿栈 LIFO 逆序交还), commit 后 rollback 显式拒绝。故"rollback(commit(x))
恢复 x"的实现形态命题为 (a)+(b):

| 命题 | harness | 边界 |
|---|---|---|
| (a) rollback(fork(S0) ⊕ writes) ≡ S0: 世界快照逐字段恢复基线; 补偿序列恰为入栈顺序的 LIFO 逆序; 回滚后分支关闭拒绝再写 | `kani_saga_rollback_restores_base_lifo` | ≤2 投机写 + ≤2 补偿动作, 具体短键, unwind 32 |
| (b) commit 持久: commit(x) 的效果 x 落地且未修改基线条目随 CoW 保留; rollback(commit(x)) 返回 Err 且世界不变 (无隐式撤销) | `kani_saga_commit_durable_rollback_rejected` | 两段式场景 (建基线→提交), checksum 值任意 ≤2 字节串 |

## TLA+ 双轨实跑结果 (2026-09-26, JDK 25, tla2tools-1.7.1)

| 模型 | 命题 | 生成状态 | 去重状态 | 深度 | 结果 |
|---|---|---|---|---|---|
| `tla/ProtectForget` (REC={a,b}, MAX_REV=2) | InvProtectedNeverForgotten / InvSweepKeepsProtected / InvForgetIdempotent / InvNoResurrection + TypeOK | 1 408 | 144 | 5 | **全部不变量通过, No error** |
| `tla/ProtectForget3` (REC={a,b,c}) | 同上 | 24 256 | 1 728 | 7 | **全部不变量通过, No error** |
| `tla/QuotaSchedule` (TASKS={a,b}, MAXQ=2, STEPS=2) | InvQuotaNeverNegative / InvQuotaExhaustSticky / InvQueueSorted / InvHeadMostUrgent / InvPipNoInversion / InvBoostNeverUpgrades + TypeOK | 67 480 | 4 995 | 9 | **全部不变量通过, No error** |
| `tla/QuotaSchedule3` (TASKS={a,b,c}) | 同上 | 12 912 065 | 674 460 | 14 | **全部不变量通过, No error** |

指纹碰撞漏检概率 (TLC 报告, 乐观界): 9.9E-15 / 2.1E-12 / 1.7E-11 / 4.5E-7。

复现:

```powershell
cd research/verification/tla
java -XX:+UseParallelGC -cp tla2tools-1.7.1.jar tlc2.TLC -config ProtectForget.cfg ProtectForget.tla
java -XX:+UseParallelGC -cp tla2tools-1.7.1.jar tlc2.TLC -config ProtectForget3.cfg ProtectForget.tla
java -XX:+UseParallelGC -cp tla2tools-1.7.1.jar tlc2.TLC -config QuotaSchedule.cfg QuotaSchedule.tla
java -XX:+UseParallelGC -cp tla2tools-1.7.1.jar tlc2.TLC -config QuotaSchedule3.cfg QuotaSchedule.tla
```

模型口径 (0 装): `ProtectForget` 直接刻画 SQL 转移规则的纯函数形态
(forget_episode_impl 前检查 + 软删; sweep_session 的 protected/已遗忘跳过),
forget 幂等以算子不变量 f∘f = f 检查 (hyperproperty 的标准状态不变量化);
`QuotaSchedule` 的层内 FIFO 以序列表示 (push_back/push_front 语义), quota 到
MAXQ 饱和, PIP 以"最近一次提升"为挂起对 (与实现每次 boost 覆盖记账一致);
两模型以 `Stutter` 避免有界化造成的假死锁报警。

## 实现层观察 (证明过程发现, 停在报告, 未改业务码)

1. **`SemanticAxisBridge::fit` 缺形状校验 (panic 面)**: `weighted_mean` /
   `center_and_scale` 按 `0..dimension` 索引 `vector[i]`, 样本向量短于
   `self.dimension` 时索引越界 panic; 签名/文档未声明该形状契约。
   `kani_panic_free_semantic_axis_fit_project` 在契约内证明 panic 自由,
   契约外失败面如实记录 (建议后续以 `debug_assert!`/`Result` 显式化)。
2. **`DualScaledFieldSolver::solve` 缺方阵校验 (panic 面)**: 仅检查
   `adjacency_matrix.len() == n`, 不检查各行长度; 行短于 n 时 `relax` 内
   `adjacency_matrix[j][i]` 索引越界 panic。同上口径处理。
3. **`base64_decode` 可见性**: 模块私有函数, mirror harness 无法从外部触达;
   其 `#[cfg(kani)]` 证明段 `kani_base64_proofs` 附着于 canonical 文件内
   (与 `research_approval_sm.rs` 既有 `kani_proofs` 段同构)。这是本次唯一
   一处业务文件触碰 (纯验证段, cfg(kani) 门控, 生产零参与)。
4. **`TaskIntentEnvelopeV1::allows_publish` 设计内旁路**:
   `allows_publish = allows_operation(Publish) || intent_class == RepositoryPublish`,
   intent_class 参数可使 publish 判定为真而无需列入 allowed_effects, 且
   `mutation_policy = Deny` 不约束 publish 轴。按设计 (策略轴独立) 而非缺陷,
   记录为跨轴语义观察。
5. **forget 幂等的维度边界**: `bitemporal_graph::beliefs_as_of` (append-only
   信念审计维度) 按设计记录第二次撤回; 幂等命题在 `facts_as_of`/
   `retrospective` (当前态/双时态切片) 维度成立并被机械证明。
6. **sha2 后端选择**: mirror crate 显式启用 `sha2/force-soft` —— x86 SHA-NI
   后端的 SIMD/CPUID 内建在 CBMC 翻译层属高风险路径; 待验证对象是
   `file_fetcher` 自身逻辑而非第三方压缩函数。
7. **环境性 flaky (与本次改动无因果)**: `apeireth-tools-canonical --test
   shell_execution` 偶发 `CreateAppContainerProfile failed: HRESULT=0x8000ffff`
   (进程收容沙箱), 单跑/重跑即绿; 属运行环境限制。

## 本地校验命令与结果 (2026-09-26, Windows, rustc 1.97.1)

| 命令 | 结果 |
|---|---|
| `cargo check --workspace --all-targets --locked` | **通过** (EXIT=0) |
| `cargo test --workspace --locked` | **通过** (EXIT=0; 首跑命中观察 7 环境 flaky, 重跑全绿) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | **通过** (EXIT=0) |
| `cargo fmt --all -- --check` | **通过** (EXIT=0) |
| `cargo fmt --manifest-path research/verification/kani/Cargo.toml -- --check` | 通过 (EXIT=0) |
| `cargo fmt --manifest-path research/verification/kani-typecheck/Cargo.toml -- --check` | 通过 (EXIT=0) |
| `cargo check --manifest-path research/verification/kani/Cargo.toml` | 通过 (canonical 零复制引入编译) |
| `$env:RUSTFLAGS="--cfg kani"; cargo check --manifest-path research/verification/kani-typecheck/Cargo.toml --tests` | **通过** (25 新 harness [24 在 src/harness_* + 1 在 file_fetcher] + 既有 3 harness 全量类型检查) |
| `$env:RUSTFLAGS="--cfg kani"; cargo test --manifest-path research/verification/kani-typecheck/Cargo.toml` | **通过** (81 passed / 0 failed, 含 25 个新 harness 冒烟) |
| `java -XX:+UseParallelGC -cp tla2tools-1.7.1.jar tlc2.TLC ...` ×4 | **全部 No error** (见上表) |
| `cargo kani --manifest-path research/verification/kani/Cargo.toml --harness <name>` | **待 CI** (本机 Windows 无 Kani; `kani.yml` 已按族分步接线, 沿用 continue-on-error 口径) |

## 诚实边界 (扩展部分)

1. Kani 侧 25 个新 harness (24 个 `src/harness_*` + `file_fetcher` 内 1 个) 的
   **符号证明**尚待 ubuntu runner 执行; 本地证据
   是类型检查 + 默认值冒烟, 不等于已证明。
2. 各命题的"任意"均为显式有界量词 (字节/维度/任务数/次数上界见各表),
   属有界模型检查口径; 无限域归纳泛化未做。
3. `compute_cache_key` 的 SHA-256 符号执行成本较高 (unwind 128, 单块输入);
   若 CBMC 超时, 影响仅限该 harness 单步 (kani.yml 各步独立)。
4. 记忆守恒 SQL 侧 (protected 标志) 语义由 TLA+ `ProtectForget` 全状态空间
   覆盖, Kani 侧为纯模型投影, 两者互补而非重复。
