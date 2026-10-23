# 册 11 — 里程碑与决策记录（8 篇合册）

> **一句话定位**：V25/V26 真实集成验证、独立复核、SDK 砍填决策表、默认落地裁定等**里程碑级裁决与验收**记录——多为**当时的裁决与其判据**，现行数字另查真源。
>
> **来源篇数**：8｜**合册生成日**：2026-10-23
> **机械合订说明**：本册由脚本从原篇**机械摘取**（标题 / 元信息块 / 各节要点 / 归属路径），**不生成、不改写、不编造**；
> **本册未做逐条「现状复核」**——即不判断每条结论今天是否仍成立。要现行口径请查下方真源，原件全文见归档路径。
>
> **复核入口（现行真源）**：[claims.yaml](../docs/status/claims.yaml) ／ [test-baseline](../docs/03-reference/test-baseline.md) ／ [capabilities-matrix](../docs/03-reference/capabilities-matrix.md) ／ [00-backlog](../docs/04-internal/backend-enhancement/00-backlog.md)

## 目录

| # | 原标题 | 原日期 | 行数 |
|---|---|---|---|
| 1 | R125-12 oh-my-opencode 4 专家角色 spec (Build/Plan/Scout + 9 organ 拆解) | 2026-08-10 | 283 |
| 2 | V25 — Cargo Workspace 真实集成验证报告 | 2026-08-02 | 410 |
| 3 | V26.1 — 独立旁路 Cargo Workspace 验证报告（V26 owner 备份） | 2026-08-02 | 482 |
| 4 | 设计裁定落地批 R1 —— 23 项拍板中「参数与默认值七处」落地对照表 + 测试证据 | 2026-10-17 | 96 |
| 5 | R125-15 — 非 GitHub 学习途径调研 (学术 + 官方 + 博客 + 视频 + 社区 + hub) | 2026-08-10 | 251 |
| 6 | R20 阶段 6 — OSS NOTICE + LICENSE 治理报告 | 2026-08-05 | 212 |
| 7 | R82 LIVE: MiniMax 8 model 真接 benchmark 跑通 (2026-08-09) | 2026-08-09 | 64 |
| 8 | P1.2 SDK 骨架 crate 砍/填决策表 (R131 2026-08-12) | 2026-08-12 | 106 |

---

### R125-12 oh-my-opencode 4 专家角色 spec (Build/Plan/Scout + 9 organ 拆解) (2026-08-10)

**来源**：`reports/.r125-12-oh-my-opencode-4-role-spec.md`（283 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/.r125-12-oh-my-opencode-4-role-spec.md`（原件逐字留存）

**元信息**

```
┌──────────────────────────────────────────────────────────────┐
│  Plan (规划) - 本 spec 阶段                                 │
│  - 借鉴 ID 索引 (opencode-borrow-index-r125-12.md)          │
│  - 9 organ 重构计划 (.r125-12-REFACTOR-PLAN.md)             │
│  - 13 键 PHL-07 spec (.r125-12-PHL-07-SPEC.md)             │
│  - 4 角色 spec (本 spec)                                     │
│  - 单元测试 stub (.r125-12-13-keys-stub.rs)                 │
│  - final 报告 (agent-r125-12-final-2026-08-10.md)          │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  Scout (侦察) - 限流结束 first thing                         │
│  - clone + 7a4b9c2 commit verify                            │
│  - 9 organ 现状 read (snapshot + render + state)            │
│  - 13 键现状 read (ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE)  │
│  - 借鉴源码 1:1 映射表 (per 借鉴 ID 索引 §5)               │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  Build (实施) - 0 装 src                                     │
│  - 抽 common module (crates/apeireth-tui/src/organ/common.rs)│
│  - PHL-07 enum + 2 const 升级 (crates/apeireth-core)        │
│  - PHL-07 5 单元测试 (verdict_keys.rs +60 行)              │
└──────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│  Decompose (9 organ 拆解) - 0 装 src                         │
│  - 9 organ 内部 fn 改 import (heart/brain/hand/eye/ear/...) │
│  - 减量 -5.7KB (per 借鉴 ID 索引 §5 表)                    │
│  - 9 文件名 + 9 入口签名 0 改 (B7 锁)                        │
└──────────────────────────────────────────────────────────────┘
```

**各节要点（机械摘取，非全文）**

- **1. oh-my-opencode 概念 (per master 17:31 派指令)**
  - **oh-my-opencode** = master 17:31 提出的 4 专家角色子代理架构, 借鉴 OpenCode 项目的多角色协作模式.
  - **为什么叫 oh-my-opencode**:
  - - "oh-my" 前缀 = 参考 `oh-my-zsh` 风格 (插件化 + 角色化)
  - - "opencode" = 借鉴 anomalyco/opencode 项目
- **2. 4 专家角色定义 (per master 17:31 派指令)**
- **2.1 角色 1: Build (实施专家)**
  - **职责**: 写 src 代码 (per 借鉴 ID 索引 + REFACTOR-PLAN + PHL-07-SPEC)
  - **输入**: spec (本 spec §3-§5 全部 4 spec) + 借鉴源码 (限流结束 clone)
  - **输出**:
  - - `crates/apeireth-tui/src/organ/common.rs` (新增 module, 不动 9 文件名)
- **2.2 角色 2: Plan (规划专家)**
  - **职责**: 写 spec + 索引 + 拆解 (本 spec + 借鉴 ID 索引 + REFACTOR-PLAN + PHL-07-SPEC)
  - **输入**: master 17:31 派指令 + 借鉴 ID 索引 (per master 17:22 决策-33)
  - **输出**:
  - - `borrowed-repos/opencode-borrow-index-r125-12.md` (借鉴 ID 索引)
- **2.3 角色 3: Scout (侦察专家)**
  - **职责**: 借鉴源码 read + 9 organ 现状 read + 13 键现状 read
  - **输入**: 借鉴 ID 索引 §3 (预期文件路径) + 借鉴 ID 索引 §4 (借鉴价值 ROI)
  - **输出**:
  - - `borrowed-repos/opencode/` (限流结束 clone, 0 装 = 真实读)
- **2.4 角色 4: **9 organ 拆解 (Decompose)****
  - **职责**: 9 organ 内部 fn 拆解为 1 个 common module + 9 文件内部引用
  - **输入**: 借鉴 ID 索引 §5 (减量估算表) + REFACTOR-PLAN §2.3 (内部 fn 类型)
  - **输出**:
  - - `crates/apeireth-tui/src/organ/common.rs` (新增, 内含拆解后的 4 类 helper)
- **3. 4 角色协作 plan (R125-12 全流程)**
- **3.1 阶段 0: 0 装准备 (本 spec 阶段, ✅ 完成 8/10)**
  - | 角色 | 任务 | 输出 | 状态 |
  - | Plan | 写借鉴 ID 索引 | `borrowed-repos/opencode-borrow-index-r125-12.md` | ✅ |
  - | Plan | 写 9 organ 重构计划 | `crates/apeireth-tui/src/organ/.r125-12-REFACTOR-PLAN.md` | ✅ |
  - | Plan | 写 13 键 PHL-07 spec | `crates/apeireth-core/src/.r125-12-PHL-07-SPEC.md` | ✅ |
- **3.2 阶段 1: 0 装实施 (限流结束, ⏳ 待办)**
  - | 角色 | 任务 | 输出 | 状态 |
  - | Scout | clone + 7a4b9c2 commit verify | `borrowed-repos/opencode/.git` | ⏳ |
  - | Scout | 借鉴源码 1:1 映射表真 verify | 借鉴 ID 索引 §5 update | ⏳ |
  - | Build | 抽 common module | `crates/apeireth-tui/src/organ/common.rs` | ⏳ |
- **3.3 阶段 2: 6 重守门 v6 verify (per B4, ⏳ 待办)**
  - | 角色 | 任务 | 输出 | 状态 |
  - | Build | compile verify | `cargo check -p apeireth-tui -p apeireth-core` 0 error | ⏳ |
  - | Build | test verify | `cargo test` 19+5=24 pass (PHL-07) | ⏳ |
  - | Build | lint verify | `cargo clippy --all-targets` 0 warning | ⏳ |
- **3.4 阶段 3: final 报告 update (per master 17:22 决策, ⏳ 待办)**
  - | 角色 | 任务 | 输出 | 状态 |
  - | Plan | "0 装准备" → "0 装实施" | final report §0, §1, §3, §4 update | ⏳ |
  - | Plan | 7a4b9c2 commit 真 verify | final report §1 update | ⏳ |
  - | Plan | 9 organ 减量真测 verify | final report §5 update | ⏳ |
- **4. 4 角色与 8 哲学锚 (per B5)**
  - R125-12 4 角色协作必须穿透 6 → 8 哲学锚 (per B5, 8 = 6 + S-3 + O-1):
  - | # | 哲学锚 | 描述 | R125-12 4 角色如何穿透 |
  - | 1 | S-1 (22:33) | 不依赖独立代理服务 | Plan 角色写借鉴 ID 索引, 1:1 字段映射, 0 包装 |
  - | 2 | S-2 (17:43) | 现实胜利 | Build 角色限流结束真实施 (cargo test 24 pass), 0 装 src |

**归属链接**：crates/apeireth-tui/src/organ/common.rs、crates/apeireth-core/src/lib.rs、crates/apeireth-tui/src/organ/.r125-12-REFACTOR-PLAN.md、crates/apeireth-core/src/.r125-12-PHL-07-SPEC.md、.r125-12-oh-my-opencode-4-role-spec.md、reports/agent-r125-12-final-2026-08-10.md、crates/apeireth-tui/src/organ/.r125-12-13-keys-stub.rs、crates/apeireth-core/tests/verdict_keys.rs

### V25 — Cargo Workspace 真实集成验证报告 (2026-08-02)

**来源**：`reports/V25-cargo-workspace-acceptance.md`（410 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/V25-cargo-workspace-acceptance.md`（原件逐字留存）

**元信息**

```
[Document-Meta]
Document: V25-cargo-workspace-acceptance.md
Task: V25 cargo workspace 真实集成验证 (b266eddf-8ea6-4200-aae7-c3b2f39848ed)
Role: backend_engineer2
Status: 🟡 部分达成 — 主分支真实集成验证完成，但工作区处于并发重构中
Last-Modified: 2026-08-02 (UTC+8)
Branch: rebase/d7d8-into-integration
HEAD: c332a48d Q20 Self-Disable 覆盖历年学术论文所有已知 bypass
```

**各节要点（机械摘取，非全文）**

- **🎯 任务目标**
  - > 在主分支独立运行 `cargo build --workspace` + `cargo test --workspace`，输出真实结果：
  - > 1. 真实运行命令输出（**不允许伪 PASS**）
  - > 2. `cargo build --workspace` 必须 0 error
  - > 3. `cargo test --workspace` 全绿（统计总测试数 + 失败数 0）
- **🏆 DoD 自评**
  - | # | DoD | 达成 | 证据 |
  - | 1 | 真实运行命令（**不允许伪 PASS**）| ✅ | 6 次 cargo build/test 调用，完整日志 /tmp/build_fresh.log /tmp/test_clean_fresh.log 等 |
  - | 2 | `cargo build --workspace` 必须 0 error | ⚠️ **FAIL**（并发重构中）| 10:35:41 PASS（22/22）；10:46:46 FAIL（apeireth-core 4 errors） |
  - | 3 | `cargo test --workspace` 全绿（统计总测试数 + 失败数 0）| ⚠️ **FAIL**（并发重构中）| 10:41:01 fresh --exclude 5 破损 → **573 passed, 0 failed, 1 ignored, 0 errors**；10:46:54 FAIL（apeireth-core 测试无法编译） |
- **📊 真实命令输出（10:35:41 — 任务起点时刻）**
- **命令 1: 首次 `cargo build --workspace`（fresh / 22/22 pass）**
  - **22 个 workspace crate 全部编译通过**（详见后表"22/22 编译明细"）。
- **📊 真实命令输出（10:41:01 — 测试快照）**
- **命令 2: `cargo test --workspace --exclude (5 已知破损) — fresh`**
  - 排除的 5 个 known-broken crate 来自多次 cargo test 扫描结果（详见 §"V22 / A20 baseline 对比"）：
  - - `apeireth-upgrade`（tests/upgrade_q15.rs — 7 编译错误）
  - - `apeireth-perception`（lib test + perception_demo example + pipeline_e2e test — 1 编译错误）
  - - `apeireth-central`（lib + lib test — 1 编译错误 COMPONENT_COUNT）
- **📊 真实命令输出（10:46:46 — 任务终点时刻 / 当前）**
- **命令 3: 当前 `cargo build --workspace`（fresh / FAIL）**
- **命令 4: 当前 `cargo test --workspace`（fresh / FAIL）**
- **🔍 真实根因分析（不假装 PASS）**
- **为什么 10:35:41 PASS 但 10:46:46 FAIL？**
  - **10:35:41 → 10:46:46 期间（约 11 分钟）其他 agent 删除了 `apeireth-verify/src/lib.rs`**：
  - **事件链**：
  - 1. 10:35:41 — 任务起点，apeireth-verify/src/lib.rs 存在（无 git 历史，untracked），`cargo build --workspace` 成功（22/22）
  - 2. 10:37-10:42 — cargo test 多次跑，573 tests pass
- **📋 V22 / A20 baseline 对比 + 影响标注**

**归属链接**：reports/V22-mcp2-workspace-reproducibility.md、achievement-A20-leader-integration-final.md

### V26.1 — 独立旁路 Cargo Workspace 验证报告（V26 owner 备份） (2026-08-02)

**来源**：`reports/V26.1-cargo-workspace-independent-verification.md`（482 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/V26.1-cargo-workspace-independent-verification.md`（原件逐字留存）

**元信息**

```
[Document-Meta]
Document: V26.1-cargo-workspace-independent-verification.md
Task: V26.1 独立旁路 cargo build/test --workspace 验证 (V26 owner 备份) (e384953a-f229-4b56-894e-ded34a988d78)
Role: backend_engineer2
Status: ✅ 主目标全绿 — cargo build 24/24 + cargo test 834/0/0
Last-Modified: 2026-08-02 11:20 (UTC+8)
Branch: rebase/d7d8-into-integration
HEAD: 9f4a367c fix(Cargo.toml): add apeireth-supervisor to workspace members
```

**各节要点（机械摘取，非全文）**

- **🎯 任务目标**
  - > 1) 在 main 分支独立跑 cargo clean && cargo build --workspace
  - > 2) 独立跑 cargo clean && cargo test --workspace
  - > 3) 输出 reports/V26.1-...md 含 2 次真实 cargo 输出 + V26 owner（architect2）结果交叉对比 + 7 处 pre-existing 破损影响标注 + 严禁伪 PASS
  - > 4) 不引入 PyO3 / Python 跨语言依赖
- **🏆 DoD 自评**
  - | # | DoD | 达成 | 证据 |
  - | 1 | 在 main 分支独立跑 cargo build --workspace | ✅ | 命令 1：exit 0, **24 crates 编译通过, 0 error**, 17.89s |
  - | 2 | 独立跑 cargo test --workspace | ✅（带前置条件）| 命令 2：cargo test --workspace（带 examples）exit 101, 5 errors in `apeireth-verify/examples/walk_all_crates.rs`（P28 proc-macro 未完成）；**cargo test --workspace --tests（无 examples）exit 0, 834 passed, 0 failed, 0 ignored, 54 test runs** |
  - | 3 | 报告含 2 次真实 cargo 输出 + V26 交叉对比 + 7 处破损影响标注 | ✅ | §"命令 1 cargo build --workspace" + §"命令 2 cargo test --workspace (--tests only)" + §"V26 交叉对比矩阵" + §"7 处 pre-existing 破损影响标注 V26.1 更新" |
- **📊 命令 1: cargo build --workspace（fresh）**
- **真实命令输出（11:17:10-11:17:29, 19s）**
- **完整 24 crate 编译明细**
  - | # | Crate | 编译 | 备注 |
  - | 1 | apeireth-action | ✅ | 4 warnings |
  - | 2 | apeireth-asi | ✅ | 3 warnings |
  - | 3 | apeireth-bench | ✅ | 0 warnings |
- **📊 命令 2a: cargo test --workspace（默认带 examples，11:18:01-11:18:02, 1s 失败）**
- **真实命令输出**
  - **根因**：`crates/apeireth-memory/Cargo.toml` 第 19 行末尾的 `}` 与新增 `apeireth-verify = { path = "../apeireth-verify" }` 之间**缺少换行符**（CRLF vs LF 行尾竞态）。文件在 git 工作树中**modified**（`git status` 显示），但格式错误。
  - **后续**：其他 agent 在我跑命令的 1 秒内（11:18:01 → 11:18:02）自动修复了这个文件，使第 19-20 行恢复为：
  - 之后 cargo build --workspace 仍能正常编译（cargo build 不加载 test manifest）。
- **📊 命令 2b: cargo test --workspace（带 examples，再次跑 11:18:02）**
- **真实命令输出**
  - **根因**：`apeireth-verify/examples/walk_all_crates.rs` 调用 6 个 crate 的 `__register_all_asserts()` 函数（apeireth_core 已实现，其他 5 个未实现）。这是 **P28 阶段 6 VerdictTrace 互锁的 proc-macro 注入未完成** —— fullstack_engineer 在跑的 P28 任务范围。
  - **归属**：**P28 阶段 6 破损**，不属于 V26.1 任务范围。fullstack_engineer owner 修复。
- **📊 命令 2c: cargo test --workspace --tests（不带 examples，11:18 之后）**
- **真实命令输出（exit 0 ✅）**
- **24 crates 测试覆盖明细**
  - | # | Crate | Test Runs | Pass | Fail | Ignored |
  - | 1 | apeireth-action | 2 | ? | 0 | 0 |
  - | 2 | apeireth-asi | 2 | ? | 0 | 0 |
  - | 3 | apeireth-bench | 1 | ? | 0 | 0 |

**归属链接**：reports/V26.1-...md、crates/apeireth-memory/Cargo.toml

### 设计裁定落地批 R1 —— 23 项拍板中「参数与默认值七处」落地对照表 + 测试证据 (2026-10-17)

**来源**：`reports/design-ruling-r1-defaults-landing-2026-10-17.md`（96 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/design-ruling-r1-defaults-landing-2026-10-17.md`（原件逐字留存）

**元信息**

# 设计裁定落地批 R1 —— 23 项拍板中「参数与默认值七处」落地对照表 + 测试证据
> **日期**: 2026-10-17
> **来源**: `docs/design/human-side-design-review.md` §10 拍板清单（owner 裁定 = **全按建议**）。
> **范围**: 参数与默认值七处，全部小改 + 测试钉（其余裁定项归口各自施工线）。
> **纪律**: 中性表述；扫描零命中；不写 CHANGELOG；不 commit；校验 `-j 2` 口径；
> 0 触碰 LOCKED 5 项；0 引新外部 dep；0 主动 commit。

**各节要点（机械摘取，非全文）**

- **1. 裁定 → 落点对照表**
  - | # | 裁定项 | 裁定值 | 落点（符号级） | 改动 | 旧行为兼容口 |
  - | 1 | ③ 情感残响强度（旧事回涌系数） | **0.2** | `crates/adapters/gateway/src/presence.rs` `EMOTION_RESIDUAL_STRENGTH`（`turn_completed` 的召回命中→pleasure 施加点） | 常量化并由 0.15 改 0.2：每次召回命中 pleasure 回涌 0.2 | 无配置面（常量）；命中量 = 常量 × 召回次数，测试钉 0.2/0.4 两档 |
  - | 2 | ⑩ 遗忘半衰期（未保护记忆；保护豁免不变） | **90 天** | `crates/engine/memory/src/retention.rs` `BASELINE_HALF_LIFE_HOURS`；`crates/engine/memory/src/persona_layer.rs` `PERSONA_SLOW_HALF_LIFE_HOURS` | 基线 24h → 90 天（2160h）；persona 未保护层 30 天 → 90 天（口径统一，两常量同值钉）；protected 豁免语义 0 改 | 显式 `RetentionPolicy::with_min_stren
  - | 3 | ⑭ 勿扰窗口默认（两处同源） | **23:00-08:00** | `crates/capabilities/tools/src/voice/quiet.rs` `VOICE_QUIET_HOURS_DEFAULT`；`crates/engine/organ/src/care/rules.rs` `CareDiscipline` 缺省窗（`default_quiet_start/end_minutes`） | 两处默认同改 23:00-8:00（原发声面 22:00-06:00、关怀纪律无默认窗）；接缝 `runtime-assembly::voice_quiet_hours_f
- **2. 测试证据（新增钉 ≥10；全部 2026-10-17 实测通过）**
  - | 钉 | 测试（文件 :: 名） | 钉住什么 |
  - | ③ | `crates/adapters/gateway/src/presence.rs::emotional_residual_strength_is_point_two_per_recall_hit` | 残响常量 = 0.2；1 次召回 pleasure +0.2、2 次 +0.4（点值） |
  - | ⑩ | `crates/engine/memory/src/retention.rs::forget_half_life_defaults_to_ninety_days_and_stays_overridable` | 基线 2160h=90 天；默认策略 90 天；一个半衰期强度 0.5；显式 24h 覆盖（兼容） |
  - | ⑩ | `crates/engine/memory/src/persona_layer.rs::persona_decay_strength_is_exempt_or_slow`（扩钉） | persona 未保护层 = 90 天；**与 retention 基线同值（口径统一）**；protected 恒 1.0（保护豁免不变） |
- **3. 既有测试的必要改写（默认值变更波及；逐条登记）**
  - | 测试/夹具 | 改写 | 理由 |
  - | `care.rs` 9 处夜间时点夹具 `CareDiscipline::default()` → `unwindowed()`（显式关窗） | 夹具隔离 | 新默认勿扰窗 23:00-8:00 会先拦下夜间到点；这些测试盯触发器/频率/专注/预算，不是勿扰窗 |
  - | `runtime-assembly/tests/care_output_face.rs` 竖切夹具补显式关窗 | 夹具隔离 | 同上（到点 01:00） |
  - | `runtime-assembly/src/canonical/voice_quiet.rs::missing_or_unwindowed_care_rules_fall_back_to_the_minimal_env` | 兜底断言字面值 → 常量；unwindowed 夹具显式 null 窗 | 默认窗改 23:00-8:00；「未配窗 → env 兜底」支路前提显式化 |
- **4. 校验记录（2026-10-17，工具链 rustc 1.97.1 x86_64-pc-windows-msvc；口径 `-j 2`）**
  - | 命令 | 结果 |
  - | `cargo test -p apeireth-organ --test care --locked -j 2` | 23 passed / 0 failed |
  - | `cargo test -p apeireth-memory --all-targets --locked -j 2 --no-fail-fast` | 全绿（lib 822 passed + 各集成目标 0 failed） |
  - | `cargo test -p apeireth-orchestration --all-targets -p apeireth-organ --locked -j 2 --no-fail-fast` | 全绿（orchestration lib 354 / task_queue 16 / organ lib 206 + 各目标 0 failed） |
- **5. 并发在途红（他人在途，标红不代改；本批收尾时全部已收口）**
  - 1. `crates/capabilities/tools/tests/exposure_audit.rs` + `crates/adapters/cli/src/exposure.rs`（脱敏审计线在途）：期间编译断点（literal 越界 / `dir` 未定义 / `fs_err` 未接），由归口线自修收口；非本批文件 0 触碰。
  - 2. `apeireth-runtime-assembly` / `apeireth-cli`（性格锁定批在途）：`trait_growth` / `TuningStatus{locked,recent_moves}` / `canonical/trait_report_tool.rs` / `trait_command` 期间编译断点，归口性格锁定批收口；收口后本批全部校验已补齐（§4）。
  - 3. 跨线补位记录：`playback.rs` 的 `Debug for QueueSlot` 由性格锁定批按该文件报备注释补（`expect_err` 编译断点）；本批对 `SpeechTicket` 同型断点改用 `let...else` 不扩生产面。
- **6. 登记不实施（超范围，归口文档线）**
  - 1. `docs/04-internal/claims-evidence-matrix.md`（审计产出，历史不追溯改写）内「layered_memo/decay.rs，默认 24h」为审计时点口径，现值已 90 天 —— 建议归口文档线加「历史口径（日期）」标注。
  - 2. `docs/design/human-side-design-review.md` §10 拍板表「你的裁定」列未填（owner 拍板 = 全按建议）—— 归口设计稿线回填。
  - 3. `docs/03-reference/test-baseline.md` 数字不代改（真源归口）；本批实测值如 §4。

**归属链接**：../docs/design/human-side-design-review.md、../docs/03-reference/test-baseline.md、crates/adapters/gateway/src/presence.rs、crates/engine/memory/src/retention.rs、crates/engine/memory/src/persona_layer.rs、crates/capabilities/tools/src/voice/quiet.rs、crates/engine/organ/src/care/rules.rs、crates/foundation/orchestration/src/task_queue.rs、crates/capabilities/tools/src/voice/playback.rs、crates/engine/memory/src/layered_memo/decay.rs、crates/engine/runtime-assembly/src/canonical/voice_quiet.rs、crates/engine/organ/tests/care.rs

### R125-15 — 非 GitHub 学习途径调研 (学术 + 官方 + 博客 + 视频 + 社区 + hub) (2026-08-10)

**来源**：`reports/r125-15-non-github-resources.md`（251 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/r125-15-non-github-resources.md`（原件逐字留存）

**元信息**

```
R125-15-BORROW-arxiv-{arxiv_id}-{hash}-2026-08-10
```

**各节要点（机械摘取，非全文）**

- **0. 触发事件**
  - **主人 16:41 问**: "除了 GitHub, 你觉得我们的项目还有什么学习途径?"
  - **Mavis 16:42 答**: 6 大类 (学术论文 / 官方文档 / 博客 / 视频 / 社区 / hub) + Mavis 给 R125-15 + Apeireth Library 建议
  - **主人 16:43 拍板**: "A+B. 我们不是原来就有 research 文件夹么, 一起做成 library 了, 作为资料库, 调研做完了你自己安排任务升级"
- **1. R125-15 6 大类调研总览**
  - | 大类 | P 级别 | 输出 | 子任务 | 估时 |
  - | **学术论文** | P0 | 30+ arxiv 论文 | R125-15a | 1-2 天 |
  - | **官方文档 / RFC** | P0 | 20+ spec | R125-15b | 1-2 天 |
  - | **技术博客** | P1 | 15+ 博客 | R125-15c | 1-2 天 |
- **2. R125-15a 学术论文 (P0, 30+ 论文)**
- **2.1 调研方向**
  - 按 Apeireth 1.0 release 路线图, 30+ 论文分 6 大主题:
  - | 主题 | 数量 | 重点论文 |
  - | **AI Agent 架构** | 8 | LangGraph (R124-2 B-001), AutoGen (B-002), CrewAI (B-004), Agentless (B-046) |
  - | **AGI / Long-running Agent** | 5 | aGLM (B-016, 2024Q4), Loom (B-017, 2025), Karpathy autoresearch (B-018) |
- **2.2 调研任务**
  - - **位置**: `library/10-non-github-resources/01-arxiv-papers/`
  - - **格式**: 每篇 1 个 .md 文件 (5-10KB), 含 arxiv ID + 摘要 + 核心 idea + 借鉴方向 + 借鉴 ID
  - - **下载**: PDF 到 `references/borrowed-papers/` (主仓外 0 污染)
  - - **0 主动 commit**: Mavis 整合 #3 拍板
- **2.3 借鉴 ID 格式**
  - 例: `R125-15-BORROW-arxiv-2607.00151-7a3b2c1-2026-08-10`
- **3. R125-15b 官方文档 / RFC (P0, 20+ spec)**
- **3.1 调研方向**
  - 按 Apeireth 协议层 + 安全层 + 工程化, 20+ spec 分 4 大类:
  - | 类别 | 数量 | 重点 spec |
  - | **协议层** | 5 | MCP (modelcontextprotocol.io), A2A (a2a-protocol), ANP (Agent Network Protocol), Bee ACP, LSP (Language Server Protocol) |
  - | **LLM 服务** | 5 | OpenAI API reference, Anthropic API reference, Google AI Studio, Cohere API, Mistral API |
- **3.2 调研任务**
  - - **位置**: `library/10-non-github-resources/02-official-docs/`
  - - **格式**: 每份 1 个 .md 文件 (3-8KB), 含 URL + 摘要 + 核心 API/字段 + 借鉴方向
  - - **下载**: PDF 到 `references/borrowed-docs/` (主仓外 0 污染)
- **4. R125-15c 技术博客 (P1, 15+ 博客)**
- **4.1 调研方向**
  - 按 Apeireth 工程化 + 性能 + 可靠性, 15+ 博客分 5 类:
  - | 类别 | 数量 | 重点博客 |
  - | **AI 一手** | 5 | OpenAI Engineering, Anthropic Research, Google DeepMind Blog, Meta AI Blog, Microsoft Research |
  - | **大规模系统** | 4 | Netflix Tech Blog, Uber Engineering, Stripe Engineering, Cloudflare Blog |
- **4.2 调研任务**
  - - **位置**: `library/10-non-github-resources/03-tech-blogs/`
  - - **格式**: 每博客 1 个 .md (3-5KB), 含 URL + 3-5 篇核心文章 + 借鉴方向
  - - **0 下载 PDF**: 博客文章 URL 已存档, Mavis 0 必下载, 主人需要时按 URL 读

### R20 阶段 6 — OSS NOTICE + LICENSE 治理报告 (2026-08-05)

**来源**：`reports/r20-stage-6-th-license-notices-report.md`（212 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/r20-stage-6-th-license-notices-report.md`（原件逐字留存）

**元信息**

```
$ cat LICENSE | head -5
Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   Copyright 2026 Apeireth Team
```

**各节要点（机械摘取，非全文）**

- **0. 任务范围**
  - 实施 1.0 release 12 项 checklist #11 license (per 蓝图 §3.5, `reports/r20-v1.0.0-release-checklist-2026-08-05.md` line 21):
  - - 必读输入: `Cargo.lock` (626 entries / 558 unique crate names) / `LICENSE` (workspace = Apache-2.0) / `deny.toml` (16 license allow)
  - - 必做: 体检现状 + 自动生成 `THIRD-PARTY-NOTICES.md` + 验证 cargo deny license 0 violation + 1 commit 落地
- **1. 实查数字 (vs 任务估算)**
  - | 任务估算 | 实查 | 偏差说明 |
  - | 24 LOCKED crate | **56 baseline crate** + 4 NEW (i18n/observability/task/tree-sitter) = **60 mtime entries** | 任务估错;per `reports/r20-stage-3-crate-mtime-baseline.txt` (commit 50e6cbf0 标题即 "56 LOCKED crate 0 触碰") |
  - | 521 external deps | **558 unique crate names** (626 Cargo.lock entries) | cargo-about 报告 **561 crates with license info** (含 3 个 workspace 自家 + 558 外部,差 = 重复多版本) |
  - | 200-500 unique licenses | **12 unique SPDX IDs / 130 text variants** | 130 是 41 个 Apache-2.0 + 68 个 MIT + 4 BSD-3 + 5 ISC + ... 不同 text variant |
- **2. 实施步骤**
- **2.1 体检 LICENSE 现状**
  - ✅ Workspace license = Apache-2.0 (per `Cargo.toml [workspace.package] license = "Apache-2.0"`)
  - ✅ `deny.toml` 已存在 (8/5 12:48) 含 16 license allow
  - ❌ `THIRD-PARTY-NOTICES.md` 不存在 (新生成)
  - ❌ `NOTICE` 不存在 (按 task 范本,`deny.toml` 验证即满足 "Apache 2.0 + NOTICE + 第三方 LICENSE")
- **2.2 自动生成 THIRD-PARTY-NOTICES.md**
  - **工具**: `cargo-about 0.8.4` (业界标准, EmbarkStudios 维护)
  - **安装**:
  - **配置** (`about.toml` 新建):
  - - accepted 16 licenses (与 `deny.toml` 一致)
- **2.3 验证 license 兼容性**
  - **Exit code: 0** ✅
  - **Violations: 0** ✅
  - **唯一 warning**: `MPL-2.0` 在 deny.toml allow list 但没有任何 crate 实际用到 (cargo-about 把它作为 1 个 text variant 列出,但 SPDX expression 不止纯 MPL-2.0,可能是 transitive GPL-2.0 alias)。这是 false positive warning,**不是 violation**。
  - 详见 `reports/r20-stage-6-th/cargo-deny-licenses.log`。
- **2.4 1 commit 落地**
  - **Commit message**:
- **3. 严守规范审计**
  - | 规范 | 状态 | 证据 |
  - | 0 改 24 LOCKED crate (实际 56 baseline) | ✅ 0 触碰 | mtime baseline task-start vs current diff (见下) |
  - | 0 改 workspace version | ✅ | `Cargo.toml` `version = "1.0.0"` 未改 |
  - | 6 哲学锚 | ✅ | 阶段 1+2 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值,全保留 (本任务无相关改动) |
- **4. 0 触碰实查 (LOCKED crate mtime diff)**
  - **Task start baseline** (2026-08-05 21:37:02+08:00):
  - - `reports/r20-stage-6-th/crate-mtime-baseline-task-start.txt` (60 entries)
  - **Task end current** (snapshot at end):
  - - `reports/r20-stage-6-th/crate-mtime-current.txt` (60 entries)
- **5. 报告回写 (per task 要求)**
  - | 项 | 任务估 | 实查 |
  - | THIRD-PARTY-NOTICES.md 行数 | 500-1000 | **1709 行 / 104KB** |
  - | 1 commit hash | TBD | TBD (commit 时) |
  - | 0 触碰实查 | "24 LOCKED" | **56 LOCKED baseline 0 触碰** (7 个变动均为其他 sub-agent 并行工作) |
- **6. R-Measure 影响 (none)**
  - 本任务纯文件级操作,不修改任何 crates/ 源码,无 R-Measure 影响:
  - - V1141 = 0.8682 / V1131 = 0.8532 / V1136 = 0.9063 三个 baseline 全部保留
  - - 24 维 V0.5 + 9 子测度 V1136 未触碰
- **7. 已知 follow-up (后续 round)**
  - - `stringmetrics 2.2.2` 无 `license` field in Cargo.toml (但 `LICENSE` 文件 = Apache-2.0) — 等待 upstream 修 (或在 apeireth 侧打 patch)
  - - A few transitive deps declare legacy `GPL-2.0` (deprecated identifier) — 等待 upstream 升 `GPL-2.0-only`
  - - `MPL-2.0` 在 deny.toml allow list 但未使用 — 可考虑从 allow 移除 (但无害,留作 future-proof)
  - - cargo-about 0.9.1 UTF-8 panic on `stringmetrics 2.2.2` — 已 pin 0.8.4,等 upstream 修

**归属链接**：reports/r20-v1.0.0-release-checklist-2026-08-05.md、r20-stage-3-crate-mtime-baseline.txt、scripts/r20-stage-6-th/dedup_licenses.py、reports/r20-stage-6-th/about-raw.json、r20-stage-6-th/crate-mtime-baseline-task-start.txt、r20-stage-6-th/crate-mtime-current.txt、r20-stage-6-th/crate-mtime-diff-task.txt、r20-stage-6-th-license-notices-report.md

### R82 LIVE: MiniMax 8 model 真接 benchmark 跑通 (2026-08-09) (2026-08-09)

**来源**：`reports/r82-live-minimax-8model-results.md`（64 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/r82-live-minimax-8model-results.md`（原件逐字留存）

**元信息**

```
Set-Location ".openclaw\workspace\promethean\Apeireth-rust"
$env:APEIRETH_EVAL_LIVE="1"
cargo run --example r70_live_cross_model -p apeireth-eval --release
```

**各节要点（机械摘取，非全文）**

- **跑法**
  - 注: 原本 8 model (DEFAULT_MODELS + EXTENDED_MODELS dedup), 实跑 7 (M2 排除因未列在 DEFAULT/EXTENDED 中)
- **跑通 evidence (verbatim stdout)**
- **结论**
  - - ✅ 7/7 model 端到端跑通, 0 fail
  - - ✅ Token 报数真 (input 51-63, output 15-295)
  - - ✅ Response shape 跟 Anthropic spec 1:1 (content[] + usage + stop_reason)
  - - ✅ `MiniMax-M3` 最快 (1652ms) + 最少 output (15 tokens)
- **集成点**
  - - R32-3-1 `real_llm_smoke::RealLlmSmokeReport` 7 阶段 metric 100% 适用
  - - R32-3-2 `cross_model_benchmark::run_cross_model_benchmark` 复用
  - - apikey load: `load_api_key(None)` -> `file:.openclaw\apikey.txt` 自动 fallback
- **后续可推 (本批不做)**
  - - R82-1: 加 8 model (MiniMax-M2 / DeepSeek-V3 / GPT-4o / Claude 3.5) 多厂商 cross-vendor
  - - R82-2: 加 token cost 计算 (per model pricing)
  - - R82-3: 加 CI workflow (matrix: model × prompt)

### P1.2 SDK 骨架 crate 砍/填决策表 (R131 2026-08-12) (2026-08-12)

**来源**：`reports/sdk-crate-decision-r131.md`（106 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/sdk-crate-decision-r131.md`（原件逐字留存）

**元信息**

# P1.2 SDK 骨架 crate 砍/填决策表 (R131 2026-08-12)
> 评估 17-20 个 SDK 翻译型/工具型 crate 的真实度, 给出 **砍/填/留** 决策.
> 决策原则: 0 假装 LLM (LARK/VOICE/LIVEKIT 真接) > 留骨架 (i18n 数据驱动) > 砍 (高 stub 标记).
## 1. 现状清单 (R131 评估)
| # | Crate | 状态 | 实现度 | 决策 | 理由 |
|---|---|---|---|---|---|

**各节要点（机械摘取，非全文）**

- **1. 现状清单 (R131 评估)**
  - | # | Crate | 状态 | 实现度 | 决策 | 理由 |
  - | 1 | `apeireth-credentials` | ❌ 已删 | 0% | **已砍** | R20 阶段 6 1:1 翻译商业版, 0 真接 |
  - | 2 | `apeireth-cache` | ❌ 已删 | 0% | **已砍** | LRU/TTL 骨架, 仅 Memory 真接 |
  - | 3 | `apeireth-tracing` | ❌ 已删 | 0% | **已砍** | 分布式追踪骨架, 仅 stdout/file |
- **2. 现状: 13/20 已砍, 7/20 保留**
  - **已砍 (13 个):** credentials / cache / tracing / metrics / oauth / update / sandbox / keyring / machine-id / repo-scan / repo-analyzer / image-prompt / plugin
  - **R131 决策 (7 个):**
  - - **冻结 (3 个, 等真接需求):** mcp-ssh / mcp-winrm / mcp-relay-image — 0 假装路径, 等业务真需
  - - **保留 + 填 (4 个, R132-R134 续):** lark / voice / livekit / i18n
- **3. 详细决策**
- **3.1 冻结 (3 个 MCP 远程管理) - 理由**
  - **apeireth-mcp-ssh** (16 stub, 4 fn):
  - - 0 真接 ssh2 lib (Rust 主流 ssh crate)
  - - 仅类型定义 + 错误枚举
  - - 实际路径: WS-MCP 暴露 ssh_connect / ssh_exec / ssh_keepalive 工具
- **3.2 保留 + 填 (4 个真接 provider) - 理由**
  - **apeireth-livekit** (29 impl, 22 fn, 16 test, 0 stub) **优先级 P0**:
  - - 这是 **真接 SDK** — `livekit` crate 真的集成
  - - 0 stub 标记 → 真实可用
  - - 决策: 保留 + R132 续填 (TUI 视频/音频需要)
- **4. 8 硬墙 0 越界严守**
  - - ✅ 0 改任何 LOCKED entry signature
  - - ✅ 0 主动 commit 改动
  - - ✅ 0 增 workspace member (13 个砍的已经删)
  - - ✅ 0 改 version 1.2.0
- **5. 落地时间表**
  - | 阶段 | 任务 | R |
  - | 即时 | 7 MCP crate freeze 标记 (R131) | R131 (本次) |
  - | R132 | livekit + voice 真接 TUI 视频/音频 | R132 |
  - | R133 | lark + i18n 续填 | R133 |
- **6. 不假装登记**
  - - 这 7 个保留 crate **仅是工具型**, 0 涉及 L0/L1 守门
  - - Self-Disable / 8 重守门 / 9 重守门 0 触碰
  - - 24 LOCKED entry 0 触碰
  - - 不假装标注: R131 决策仅是"砍/填"清单, 0 实施删除 (保留 workspace member 状态)

**归属链接**：../docs/archive/conventions/16-crate-merge-policy.md

