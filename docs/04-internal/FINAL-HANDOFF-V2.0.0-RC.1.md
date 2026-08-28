# Apeireth v2.0.0-rc.1 最终接手报告 (2026-08-28 收盘, 主代理 Mavis 写, Final-2.1 更新: A 块 OrganOrchestrator 完整化 + O-6 三阶审查 amend)

> **本文档定位**: v2.0.0-rc.1 收盘接手报告, 含完整意图 + 进度 + 给新团队的话. **接手人入口文档**.
> **HEAD 状态**: 收盘批 commit (本批交付 commit, 见本文件 §0 下方 git 验证命令; 本批前 = `ccf29c57` 错账修正 + `2550b99d` R12 真实施; **本批新 = A 块 5 stage 真实施 amend 后 commits `c003e078` ~ `0afa733f` + 复盘配对 `bbbfb75b`**). 接手人首件事: 跑 `git log --oneline | head -1` 确认 HEAD 与 §0 一致 (0 装诚实, HEAD 漂移是病).
> **读谁**: 接手 Apeireth v2.0 的新团队 / 未来自我升级 cycle 的实施者.
> **何时写**: 主代理 Mavis 收盘 rc.1 session 写于 2026-08-28, 真 LLM 调通后; Final-2.0 = 2026-08-28 8 spec 收齐 + R12 真实施落地 + 6 处错账修正后更新. **Final-2.1 = 2026-08-28 A 块 OrganOrchestrator 完整化 (5 stage 真实施) + O-6 三阶审查 amend (主代理自检 0 装诚实标修正) 后更新**.
> **关系文档**: 本文 + `HANDOFF-NOTES.md` (子代理 D 接手人手册 11 节) + `v2-architecture-reflection.md` (新架构反思 + 自升级 cycle) + `v2-rc-1-progress-report.md` (本会话进展快照) + `TO-NEW-TEAM.md` (给新团队的话) + `organ-orchestrator-completion-plan.md` (A 块 5 stage 计划 + O-6 复盘 §7) + `A-block-o6-true-account.md` (A 块 O-6 失守 + amend 配对 commit).

```
[Document-Meta]
Document:        docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md
Version:         Final-2.1 (rc.1 收盘 + 8 spec 收齐 + R12 落地 + A 块 完整化 + O-6 amend)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (接手人入口)
Author:          主代理 Mavis (反思 session)
```

---

## 0. TL;DR (1 段总结)

**Apeireth v2.0.0-rc.1 = 新架构完成 (16-crate + 7 capability trait + 认知模块 + 9 哲学锚 + 5 重守门) + 9/10 RC 真实现 + 9 organ 全部真移植 + OrganOrchestrator 串联层真实施落地 (R12) + 8 spec 收齐 (R9-R15 + Z) + 6 处错账修正 + 哲学锚本体 LOCKED 真加 O-6 + 自我升级 cycle 设计完成 + 真 LLM call 1.16s 跑通 (RC-5 MiniMax adapter)**.

**总进度 ≈ 80%** (v2.0.0 release 估 4-6 月, 2027-01-08 至 2027-02 月, 因 A 块 OrganOrchestrator 完整化提前完成). 距离 v1.0 parity = frontend 对接 + 6 DEFERRED slot 激活 + RC-7 真 modality + RC-11 真生产验证 = **3 块真实施** (A 块已 ✅), 估 1-3 月 (2027-Q1 启动).

**本会话累计 85+ commit (从 `ef075420` 基线, 主代理亲算; A 块 5 stage + 复盘配对 amend 后 6 commit)**, **1739 tests passed 0 FAILED** (主代理 2026-08-28 amend 后亲跑 `cargo test --workspace --locked`; 1726 baseline + 13 new A 块), **0 clippy 警告**, **0 触碰 LOCKED 5 项**.

```bash
# 接手人首件事 (0 装诚实核验 HEAD, 不裸信文档):
git log --oneline -5
cargo test --workspace --locked        # 期望: 1739 passed, 0 FAILED
cargo clippy --workspace --all-targets --locked -- -D warnings   # 期望: 0 警告
```

---

## 1. 意图 (Why)

### 1.1 用户的核心意图

**主代理 + 用户原话归纳**:

1. **"新架构 + 1.0 全部功能" = v2.0.0 release 定义**
   - 真完成 = 9 organ 真移植 + 其他 77 crates 功能 + 7 capability trait 全真写
2. **"做完的时候我看结果"** — push commit 不询问, 0 风险立刻做
3. **"干完把架构按最优干完我们就准备补模块了"** — 不假装 "已完成", 真写完
4. **"继续做, 继续推进就行, 每做完一个小阶段, 就让子代理检查你做过的东西"** — 每小段派子代理审查
5. **"派子代理是手段不是为了用而用"** — 派子代理 = 调研/验证/真写 (有目的), 主代理必须拍板
6. **"哲学锚本体加一个就行, 我授权你改locked"** — 9 哲学锚 O-6 加 (LOCKED 0 装诚实授权)
7. **"反思记录下来以后别忘了"** — 反思文档永久保存 (本文 + `v2-architecture-reflection.md`)
8. **"1.0 不止 34 万行吧"** — 触发实测, 真 55 万行 (0 装诚实修正)

### 1.2 主代理 Mavis 哲学锚 9 项 LOCKED (升 8→9, 2026-08-27 O-6 加)

```
S-1 北极星 → S-2 实事求是 → S-3 质量工程化
O-1 安全优先 → O-2 前人肩上 → O-3 干到底
O-4 任何人都能接手 → O-5 不假装 → O-6 永远追求最优 (NEW 2026-08-27, LOCKED 0 装诚实授权)
```

**O-6 三阶审查 (commit message 必含具体回答)**:
- 总体最优: 与 v2 整体语境对齐
- 系统最优: 在子系统依赖图里位置对
- 架构最优: 引入后整个 workspace 边界清晰

**O-6 不做借口清单** (6 条, 0 装诚实标):
- 工作量大 / 等以后做 / alpha 阶段先这样 / v1 时代这样 / 用户没要求 / 派子代理能客观判断 — **全拒绝**

### 1.3 真生产前必做 (主代理判断)

按 O-6 + 13 键降级决策 + 5 重守门自动验证:
- **9 organ 真移植至少 1** (ROADMAP §4 P6 "至少 1 器官移植" 是 v2.0 release 最低门槛)
- **frontend companion-desktop 对接** (ROADMAP §4 P8)
- **RC-11 migration script 真生产前验证** (1-2 天, 有 key 但没 v1 db 验证)

---

## 2. 进度 (Where)

### 2.1 总进度 ≈ 70%

| 维度 | 完成度 | 子项 |
|---|---|---|
| **新架构** | 100% | 16-crate + 7 capability trait + 认知模块 12 slot + 9 哲学锚 + 5 重守门 |
| **RC 真实现** | 90% (9/10) | RC-1/2/3/4/5/6/8/9/10/11 真写, RC-7 真 modality spec 已完 (R14) 待硬件 |
| **9 organ 真移植** | 100% (9/9) | 整合 #2 commit `bbf70293` 一次性拍板, 9 organ trait 抽象 + 1:1 v1 翻译 |
| **OrganOrchestrator 串联层** | ✅ **R12 + A 块完整化已落** | 13 gate + 5 状态机 + 9 organ 顺序 process (R12 commit `2550b99d`) + 5 stage A 块完整化 (amend 后 commits `c003e078` / `087ab2ac` / `50ba2e57` / `29e5ce66` / `0afa733f`, 详 `organ-orchestrator-completion-plan.md`) + O-6 三阶审查 amend (commit `bbbfb75b`, 详 `A-block-o6-true-account.md`); 缺口 D ratify_fresh_policy 5 状态链 / B F1 PAD mood / A check_8_gates + E7 last_hold / C Council decide_with_invoker / E L0-L5 UpgradeCycle driver 全部真实施 |
| **认知模块** | 50% (6/12 slot WIRED) | 6 WIRED + 6 DEFERRED (judge/council 为 WIRED, OFF by default) |
| **8 spec 收齐** | 100% | R9/R10/R11/R13/R14/R15 + Z 审计 + 本报告 |
| **v1.0 parity** | 估 4-6 月 | **剩 3 块真实施** (frontend / 6 DEFERRED / RC-7, A 块已 ✅), 2027-Q1 启动 |

**总进度算法** (估, 主代理主观加权):
- 新架构 100% × 15% = 15%
- RC 90% × 20% = 18%
- 9 organ + 串联层 (R12 + A 块) 100% × 30% = 30%
- 认知 50% × 25% = 12.5%
- 8 spec + 文档 + A 块 plan doc 100% × 10% = 10%
- **小计 ≈ 85.5%**, 扣 3 块真实施未做 ≈ **80%** (0 装诚实标: 这是估, 不是精确测量; A 块提前完成, 总进度从 70% 升至 80%)

### 2.2 RC 真实现进度 (10 RC)

| RC | 状态 | Commit | 测试数 |
|---|---|---|---|
| RC-1 MemoryBackend SqliteBackend 真 SQL | ✅ | `43ec9635` | 7 + 性能基准 |
| RC-2 Experience SQLite (5 张新表) | ✅ | `4e4fba89` | 6 |
| RC-3 PreferenceStore SQLite | ✅ | `61cc0421` | 5 |
| RC-4 SelfAssessmentStore SQLite (场景 D 例 2) | ✅ | `042ad4eb` | 7 |
| RC-5 LlmFactory MiniMax 真实现 + **真 LLM call 1.16s** | ✅ | `02faa6d0` | 7 + 1 ignored |
| RC-6 Council 7 LlmAdvisor + 60s timeout + DeferToHuman | ✅ | `a3768fd6` | 15 + 12 |
| RC-7 Perception 真 modality (Whisper + screen) | ⏳ spec done (R14, 572 行) | 真实施估 2-3 周, 需硬件 |
| RC-8 SubSupervisor std::process 真实现 + 改名 `StdSubSupervisor` | ✅ | `67fc66a0` + `4e4fba89` | 8 |
| RC-9 keyring 真接入 CLI bootstrap (4 backend) | ✅ | `aa661a66` | 4 |
| RC-10 File AES-256-GCM 加密 + line header AAD tamper 保护 | ✅ | `e2a5be08` + `38cc1039` | 9 + 2 |
| RC-11 v1→v2 加密 migration script (Python + Rust test) | ✅ | `926465c8` + `483fb4cd` + `615121bd` | 6 |

### 2.3 哲学锚 9 项 LOCKED 状态

| # | 锚 | 状态 | 备注 |
|---|---|---|---|
| 1-8 | S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 | LOCKED 0 改 | 6 原版锚 + R126 新增 2 锚 (S-3 质量工程化 + O-1 安全优先) |
| 9 | **O-6 永远追求最优** | LOCKED (本批加) | 2026-08-27 LOCKED 0 装诚实授权, 源码 enum 加 `O6AlwaysOptimal` variant + NINE_ANCHORS_HARDCODE 编译期断言 |

### 2.4 5 重守门自动验证 (`.github/workflows/o6-anchor.yml`)

| # | 守门 | 状态 |
|---|---|---|
| 1 | clippy 0 警告 (`cargo clippy --workspace --all-targets --locked -- -D warnings`) | ✅ 0 警告 |
| 2 | workspace tests 0 失败 (`cargo test --workspace --locked`) | ✅ 0 FAILED |
| 3 | legacy compat path < 100 引用 | ✅ |
| 4 | 13 键 LOCKED + 9 哲学锚 + workspace.version 1.2.0 + R11 baseline 3 值 0 触碰 | ✅ 0 触碰 |
| 5 | 哲学锚表头 0 减 | ✅ |

---

## 3. 已完成成分 (What done)

### 3.1 工程形态收敛

- v1 86-crate → v2 16-crate = **70 crates removed (81.4% 收敛)**
- 4 层分组: foundation 6 + engine 6 (含 `apeireth-organ`) + capabilities 1 + adapters 3
- 单向依赖: memory/tools/cli/credentials/orchestration → plugin
- 0 反向, 0 循环, 100+ consumer 0 破

### 3.2 新架构就位

- **16-crate workspace** (Cargo.toml members)
- **7 capability trait** (位置: `apeireth-plugin`)
  - MemoryBackend / Experience / Perception / PreferenceStore / SelfAssessmentStore / LlmFactory / SubSupervisor
- **认知模块 12 slot ledger** (其他 dev 推 5 commit, **6 WIRED + 6 DEFERRED**, judge/council 为 WIRED OFF by default)
- **9 organ** trait 抽象 (`crates/foundation/plugin/src/organ.rs`) + 真实现 (`crates/engine/organ/`)
- **OrganOrchestrator 串联层** (R12 真实施: `crates/engine/runtime/src/canonical/orchestrator.rs`)
- **9 哲学锚** (升 8→9, O-6 加, LOCKED)
- **13 键** (降级为哲学标准, `RUNTIME_ENFORCED = false`)
- **3 项不可变脊柱** (Self-Disable / L0 HA / 13 键 verdict cache 语义)
- **5 重守门** 自动验证 (clippy / tests / legacy path / LOCKED / 哲学锚表头)
- **Triple Onion** (L0 人类审批 + L1-L5 权限层 + DSL 洋葱 Colang)

### 3.3 真写代码 (本会话累计 85 commit, 主代理亲算 `git log ef075420..HEAD`)

| 类别 | commit 数 | LOC |
|---|---|---|
| 真 RC 实现 | 11 commit (rc.1 批) | ~2300 行 (engine/memory + provider + orchestration) |
| 哲学锚 9 项 | 9 commit (rc.1 批, 5 Refactor + #2 + #18+#19+#23 + O-6 加) | ~1500 行 |
| 9 organ 真移植 | 9 sub-agent (Q1 + R1-R8) | 整合 #2 commit `bbf70293` 一次性拍板 |
| **R12 OrganOrchestrator 真实施** | `2550b99d` | **1933 行** (orchestrator.rs + 3 integration tests) |
| 文档交付 + 8 spec + 错账修正 | rc.1 批 + 收盘批 | 8 spec ~4000 行 + R13 接力审 497 行 |
| RC-11 migration | 1 commit | 700 行 (Python + Rust) |

> **0 装诚实**: "85 commit" = `git log ef075420..HEAD --oneline | measure` 实测; 早期文档写 "19 commit" 是少算 (只数了 rc.1 收盘前部分), Final-2.0 已修.

### 3.4 真 LLM call 跑通 (RC-5 子代理 M)

- `real_llm_call_smoke -- --ignored` → **1/1 ok, 1.16s 真 LLM**
- key 走 `APEIRETH_API_KEY` env → `EnvCredentialResolver` → per-turn resolve
- 0 hardcode, 0 commit, 0 print
- 真调 MiniMax-M3-thinking model (per scene-d §5 决策 1)

### 3.5 子代理 20 项报告全部采纳 (独立视角)

**第一批 (rc.1, 14 项)**:

| 子代理 | 任务 | 关键产出 |
|---|---|---|
| A | Send+Sync 注释 | `67fc66a0` |
| B | v1 vs v2 41 项差异 + 5 风险 | HANDOFF-NOTES + ROADMAP §3 |
| C | P0 build break + RC-8 改名 + line header 建议 5 | `4e4fba89` + `38cc1039` |
| D | 接手人手册 11 节 | `0ec9ccae` |
| E | line header 3 建议 | `0e9adb52` |
| F | ledger 数字 + 2 P1 | `0e9adb52` + `ae182c8c` |
| G | ID_LEN_MAX 边界 | `c481b123` |
| H | HEAD 漂移 + 0 装诱导 | `a2f45bea` + `f65bd89c` |
| I | RC-11 migration 真写 | `926465c8` + `483fb4cd` + `615121bd` |
| J | cognitive 5 commit LOCKED 复核 | (无 commit, 验证 0 触碰) |
| K | 哲学锚本体 LOCKED 真改 (O-6 加) | `926465c8` |
| L | v2.0 → 1.0 parity 距离 (5-7 月) | `d5a079ba` + `a2f45bea` + `f65bd89c` + `b7aec182` |
| M | RC-5 LlmFactory MiniMax 真实现 | `02faa6d0` |
| N | RC-6 Council 7 LlmAdvisor 真实现 | `a3768fd6` |

**第二批 (8 spec + 真实施, 2026-08-28 收盘)**:

| 子代理 | 任务 | 关键产出 |
|---|---|---|
| Q1 + R1-R8 | 9 organ 真移植 (E4/F1/F4/F6/W1/W2/W3/E7/Memory) | 整合 #2 commit `bbf70293` |
| R9 | frontend 对接 spec + quickstart | 565 + 224 行 (12 slot 数字错, R13 纠) |
| R10 | cognitive 9 organ 集成 spec | 1001 行 (ledger 数字错, R13 纠) |
| R11 | OrganOrchestrator spec | 500 行 15 节 |
| R12 | **OrganOrchestrator 真实施** | `2550b99d`, 1933 行 (13 gate + 5 状态机 + 9 organ 串联) |
| R13 | frontend 接力审 + 6 处错账 | 497 行, 真账 6 WIRED + 6 DEFERRED |
| R14 | RC-7 真 modality spec | 572 行 (硬件依赖如实标) |
| R15 | preference_learning 激活 spec | 617 行 |
| Z | 0 装诚实独立审计 | 60% 真兑现 + 5 假装标 (4 已修 + 1 commit message 记真账) |

### 3.6 文档完整

- `CHANGELOG.md` [Unreleased] 段 (12/12 ledger + 8/10 RC + 子代理反馈)
- `ROADMAP.md` §3 当前状态 + §3.5 阶段表 + §4 P1-P8
- `docs/04-internal/HANDOFF-NOTES.md` (子代理 D, 11 节, 1508 字)
- `docs/04-internal/v2-rc-1-progress-report.md` (本会话, 11 节 + §12 子代理 I)
- `docs/04-internal/v2.0.0-rc-roadmap.md` (10 RC + 验收 + 接手人清单, 子代理 D 修)
- `docs/04-internal/v2-unabsorbed-features.md` (41 项差异 + P-arch 状态)
- `docs/04-internal/scene-d-v2-plan.md` (场景 D 3 例)
- `docs/04-internal/migration-v1-to-v2.md` (v1→v2 迁移指南)
- `docs/04-internal/o6-session-log-2026-08-27.md` (本会话反思 + 教训)
- `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` (本文)
- `docs/04-internal/TO-NEW-TEAM.md` (给新团队的话)
- `docs/04-internal/v2.0.0-release-path-integration.md` (3 spec 协作 + 4 块真实施依赖链)
- `docs/04-internal/v2.0.0-release-path.md` (8 阶段 release 路径)
- `docs/04-internal/9-organ-progress-2026-08-28.md` (9 organ 实时进度)
- `docs/01-architecture/organ-orchestrator-spec.md` (R11, 500 行)
- `docs/01-architecture/cognitive-9-organ-integration-spec.md` (R10, 1001 行)
- `docs/01-architecture/rc-7-perception-true-modality-spec.md` (R14, 572 行)
- `docs/01-architecture/deferred-slot-activation-preference_learning-spec.md` (R15, 617 行)
- `docs/02-guides/v2-gateway-frontend-integration-spec.md` (R9, 565 行) + `v2-frontend-quickstart.md` (224 行) + `v2-gateway-frontend-integration-spec-r13-review.md` (R13, 497 行)
- `docs/01-architecture/v2-architecture-reflection.md` (新架构反思 + 自升级 cycle, §11 第二批反思)
- `docs/01-architecture/v2-arch-refactor-batch.md` (5 Refactor + 守门)
- `docs/01-architecture/philosophy.md` (9 哲学锚 + O-6 不做借口清单)
- `docs/01-architecture/engineering-report.md` (v1.0 行数 0 装诚实修正: 34 万 → 55 万)
- `docs/archive/cognitive-module-wiring.md` (12 slot ledger)
- `scripts/migrate_v1_to_v2_encrypted.py` (330 行, RC-11 真生产前用)

---

## 4. 完整上下文 (Context for new team)

### 4.1 仓库结构

```
Apeireth-rust/
├── Cargo.toml                    # workspace members (16 crates)
├── ROADMAP.md                    # 顶层路线
├── CHANGELOG.md                  # 12/12 O-6 ledger + 9/10 RC + R12 + 8 spec
├── RELEASE_NOTES.md              # v1.0 0 装诚实行数修正
├── docs/
│   ├── 01-architecture/
│   │   ├── philosophy.md         # 9 哲学锚 + O-6 不做借口
│   │   ├── engineering-report.md # v1.0 数字实测
│   │   ├── v2-arch-refactor-batch.md  # 5 Refactor + 守门
│   │   ├── v2-architecture-reflection.md  # 新架构反思 + 自升级 cycle
│   │   ├── organ-orchestrator-spec.md     # R11 (500 行)
│   │   ├── cognitive-9-organ-integration-spec.md  # R10 (1001 行)
│   │   ├── rc-7-perception-true-modality-spec.md  # R14 (572 行)
│   │   └── deferred-slot-activation-preference_learning-spec.md  # R15 (617 行)
│   ├── 02-guides/
│   │   ├── v2-gateway-frontend-integration-spec.md  # R9 (565 行)
│   │   ├── v2-frontend-quickstart.md                # R9 (224 行)
│   │   └── v2-gateway-frontend-integration-spec-r13-review.md  # R13 (497 行)
│   ├── 04-internal/
│   │   ├── HANDOFF-NOTES.md              # 11 节 接手人手册
│   │   ├── v2.0.0-rc-roadmap.md          # 10 RC + 验收
│   │   ├── v2-unabsorbed-features.md     # 41 项差异
│   │   ├── v2-rc-1-progress-report.md    # 11 节 进展快照
│   │   ├── scene-d-v2-plan.md            # 场景 D 3 例
│   │   ├── migration-v1-to-v2.md         # v1→v2 迁移指南
│   │   ├── o6-session-log-2026-08-27.md  # 反思 + 教训
│   │   ├── v2.0.0-release-path.md        # 8 阶段 release 路径
│   │   ├── v2.0.0-release-path-integration.md  # 3 spec 协作 + 4 块真实施
│   │   ├── 9-organ-progress-2026-08-28.md       # 9 organ 实时进度
│   │   ├── TO-NEW-TEAM.md               # 给新团队的话
│   │   └── FINAL-HANDOFF-V2.0.0-RC.1.md   # 本文
│   └── archive/cognitive-module-wiring.md # 12 slot ledger
├── crates/                       # 16 crates workspace
│   ├── foundation/               # 6 (core/protocol/plugin/governance/credentials/orchestration)
│   ├── engine/                   # 6 (runtime/provider/storage/memory/perception/organ)
│   ├── capabilities/             # 1 (tools)
│   └── adapters/                 # 3 (cli/gateway/sdk)
├── scripts/
│   └── migrate_v1_to_v2_encrypted.py  # RC-11 migration
└── .github/workflows/
    └── o6-anchor.yml             # 5 重守门
```

### 4.2 必跑命令 (接手人验证 baseline)

```bash
# 1. 测试
cargo test --workspace --locked
# 预期: 0 FAILED (含子代理 M 1 ignored + 子代理 N 真 LLM call 1.16s)

# 2. clippy
cargo clippy --workspace --all-targets --locked -- -D warnings
# 预期: 0 warnings

# 3. doc tests
cargo test --workspace --doc --locked
# 预期: 0 FAILED

# 4. git status
git status
# 预期: nothing to commit, working tree clean

# 5. push status
git -c http.proxy=http://127.0.0.1:7897 -c http.sslVerify=false \
    -c http.extraHeader="Host: github.com" -c credential.helper= \
    fetch origin
git status
# 预期: Your branch is up to date with 'origin/main'
```

### 4.3 真 LLM call 验证 (手动)

```bash
# 1. set env (NOT commit key, 0 装诚实)
export APEIRETH_API_KEY=$(cat C:\Users\31683\apikey-ultra.txt)
# 注: 实际是 sk-cp-ku...RsUg (125 chars, MiniMax Coding Plan)

# 2. 真 LLM call test
cargo test -p apeireth-provider --test minimax_llm_factory \
    real_llm_call_smoke -- --ignored --nocapture
# 预期: 1/1 ok, 1.16s 真 LLM, 输出 MiniMax-M3 响应
```

### 4.4 关键 file:line 引用 (接手人必读)

- **哲学锚 9 项** enum: `crates/foundation/core/src/eight_anchors.rs:58-79` (LOCKED 0 装诚实授权 O-6 加)
- **9 锚编译期断言**: `crates/foundation/core/src/eight_anchors.rs:218-326` (NINE_ANCHORS_HARDCODE)
- **13 键降级**: `crates/foundation/core/src/philosophy.rs:142` (`RUNTIME_ENFORCED = false`)
- **7 capability trait**:
  - MemoryBackend: `crates/foundation/plugin/src/memory_backend.rs:75`
  - Experience: `crates/foundation/plugin/src/experience.rs:69`
  - Perception: `crates/foundation/plugin/src/perception.rs:86`
  - PreferenceStore: `crates/foundation/plugin/src/preference.rs:56`
  - SelfAssessmentStore: `crates/foundation/plugin/src/self_assessment.rs:66`
  - LlmFactory: `crates/foundation/plugin/src/llm_factory.rs:151`
  - SubSupervisor: `crates/capabilities/tools/src/std_sub_supervisor.rs:46` (改名)
- **5 重守门**: `.github/workflows/o6-anchor.yml`
- **认知模块 12 slot**: `docs/04-internal/cognitive-module-wiring.md:22-36`
- **真 LLM impl**: `crates/engine/provider/src/minimax_llm_factory.rs` (RC-5 子代理 M)
- **真 Council impl**: `crates/foundation/orchestration/src/council/{mod.rs,advisors_llm.rs}` (RC-6 子代理 N)
- **migration script**: `scripts/migrate_v1_to_v2_encrypted.py` + `crates/engine/memory/tests/migration_v1_to_v2.rs`

### 4.5 真实数字账

| 项 | 真数字 | 来源 |
|---|---|---|
| **v1.0.0 Rust 代码** | **551,208 行** (.rs) | 实测 `git ls-tree -r v1.0.0` + `git show` (2026-08-28) |
| **v1.0.0 总 tracked LOC** | **1,154,516 行** | 实测同上 |
| **v1.0.0 active crates** | 85 (三层生态) | 实测 `git ls-tree -r v1.0.0 crates` |
| **v2 (收盘批) workspace crates** | **16** | Cargo.toml members |
| **v2 测试** | **1726 passed / 0 FAILED** | 主代理 2026-08-28 亲跑 `cargo test --workspace --locked` |
| **v2 真 LLM call 延迟** | 1.16s | 子代理 M `real_llm_call_smoke` 实测 |
| **本会话累计 commit** | **85** (从 `ef075420` 基线) | `git log ef075420..HEAD --oneline | measure` 主代理亲算 |
| **子代理派** | **31** (第一批 14 A-N + 第二批 17 Q1/R1-R15/Z) | (见 §3.5) |
| **0 触碰 LOCKED** | 5 项 LOCKED 数据 0 改 | 子代理 A/B/E/F/G/H/J/K + 主代理收盘亲验 |

---

## 5. 给新团队的话 (主代理 Mavis 写)

### 5.1 0 装诚实原则 (优先)

1. **不假装"已写未写"** — TODO 承诺 ≠ 实现 (子代理 F 0 装诱导修教训)
2. **不假装"v1 兼容"** — 实测 v1.0 55 万行, 文档说 34 万是 0 装低估 (修 3 主文档)
3. **不假装"全部 LOCKED"** — 子代理 B 报"23 项" 实际 12 项 (子代理 F 核验, 主动标解释)
4. **不假装"0 工作量"** — 子代理 D handoff #3 "12 consumer 弃用清理" 实测 0 个 `#[allow(deprecated)]` 在 src, 0 装诚实移出阻塞列表 (子代理 H 独立判断)

### 5.2 派子代理原则 (用户原话 "派是手段不是目的")

1. **派 = 调研 / 验证 / 真写 (有明确目的)** — 主代理拍板
2. **不派 = 等依赖 / 等硬件 / 0 工作量** (主代理亲做)
3. **每小段做 + 派子代理审查** (用户原话 "做完的时候我看结果")
4. **派 ≤ 14 子代理 = 0 装诱导 / 工具不是目的**

### 5.3 真生产前必做 (按优先级, 2026-08-28 收盘 + A 块完成更新)

1. **🟡 frontend companion-desktop 对接** (4-6 周, R9 spec + R13 接力审 done, 估 2027-Q1 启动)
2. **🟡 6 DEFERRED slot 激活** (6-10 周, R10 + R15 spec done, preference_learning 先)
3. **✅ OrganOrchestrator 完整化** (5 stage 真实施 done, 详 `organ-orchestrator-completion-plan.md` §5; amend 后 commits `c003e078` ~ `0afa733f`; O-6 复盘配对 `bbbfb75b`)
4. **🟢 RC-7 Perception 真 modality** (2-3 周, R14 spec done, 需硬件)
5. **🟢 RC-11 migration script 真生产前验证** (1-2 天, 有 key 但没 v1 db)

### 5.4 接手 10 步 (per `v2-architecture-reflection.md` §10 + `TO-NEW-TEAM.md` §3.3)

```
1. 读 ROADMAP.md §3 当前状态 + §3.5 阶段表 + §3.6 A 块完成真账
2. 读 CHANGELOG.md [Unreleased] 段 (12/12 ledger + 9/10 RC + R12 + 8 spec + A 块 5 stage + O-6 复盘 amend)
3. 读 docs/01-architecture/philosophy.md (9 哲学锚 + O-6 不做借口)
4. 读 docs/01-architecture/v2-arch-refactor-batch.md (5 Refactor + 守门)
5. 读 docs/01-architecture/organ-orchestrator-completion-plan.md (A 块 5 stage 计划 + O-6 复盘 §7)
6. 读 docs/04-internal/A-block-o6-true-account.md (A 块 O-6 0 装诚实复盘 + 修订版 + 后续 commit 标准)
7. 读 docs/04-internal/v2.0.0-rc-roadmap.md (10 RC + 验收)
8. 读 docs/04-internal/HANDOFF-NOTES.md (子代理 D 接手人手册)
9. 读 docs/04-internal/TO-NEW-TEAM.md (给新团队的话 + 3 块真实施清单)
10. 读 docs/01-architecture/v2-architecture-reflection.md (新架构反思 + 自升级 cycle)
11. 跑 cargo test --workspace --locked (验证 1739 passed 0 FAILED)
12. 跑 cargo clippy --workspace --all-targets --locked -- -D warnings (验证 0 警告)
```

### 5.5 给新团队的话 (主代理 Mavis 致, 完整版见 `TO-NEW-TEAM.md`)

> Apeireth v2.0.0-rc.1 = **新架构 + 工程形态收敛 + 9 organ 真移植 + OrganOrchestrator 串联层落地** 的真实完成. v2.0.0 release 的最后门槛 = 4 块真实施: **frontend 对接 (4-6 周, R9+R13 spec done) + 6 DEFERRED slot 激活 (6-10 周, R10+R15 spec done) + OrganOrchestrator 完整化 (1-3 周, R12 已落) + RC-7 真 modality (2-3 周, R14 spec done, 需硬件)** — 估 2027-Q1 启动, v2.0.0 release 估 2027-01-08 至 2027-03 月.
>
> 别忘了三件旧账: **RC-11 migration script 真生产验证** (加密文件格式 v1 → v2 不可读, 真生产前必跑); **整合 #2 commit `bbf70293` message 标 "无新外部 dep" 是错的** (真 = 5 新 dep, AES-256-GCM 系, 无法改 commit, 真账在各文档); **12 slot 真账 = 6 WIRED + 6 DEFERRED** (judge/council 是 WIRED OFF by default, 不是 "SLOT READY", R13 接力审纠).
>
> 工作方式: **派子代理 = 手段不为目的, 主代理拍板**; 子代理报告主代理必须亲验 (0 装诱导 prevention 本身可能是 0 装诱导, 子代理 Z 教训). 9 哲学锚本体 LOCKED 0 改 (升 8→9 加 O-6 是 2026-08-27 主人授权的). v2.0 release 后 Apeireth **自我升级** (L0 人类审批 → L1 自我诊断 → L2 提案 → L3 验证 → L4 主人审 → L5 runtime patch), 主代理**只拍板**, Apeireth 跑升级. **0 装诚实原则 = 真兑现, 不假装, 不漂移**.

---

## 6. rc.1 收盘 commit 列表 (历史, `ef075420..395fe0f0` 19 commit)

> **0 装诚实标**: 下面是 rc.1 首次收盘 (Final-1.0) 时的 19 commit 历史列表, 保留原样供追溯. **本会话 (含 9 organ + 8 spec + R12 + 收盘批) 累计 85 commit**, 完整列表跑 `git log ef075420..HEAD --oneline` 实测.

```
395fe0f0 docs: 反思文档 (新架构 + 自升级 cycle) + v1.0 行数实测修正 (34万→55万, 0装诚实)
a3768fd6 RC-6 Council multi-LLM + 60s timeout 真实现 (子代理 N 真兑现, 7 LlmAdvisor 替换 7 NoopAdvisor, 并行 + DeferToHuman)
02faa6d0 RC-5 LlmFactory 真实现 (MiniMax adapter, 子代理 M 真兑现, 复用 MinimaxProviderCapability HTTP client)
483fb4cd docs: 子代理 I RC-11 真兑现 + §11.3 §12 同步 (migration script 已写)
615121bd fix: 加 scripts/migrate_v1_to_v2_encrypted.py (subagent I 真写, force add)
926465c8 哲学锚本体加 O-6 永远追求最优 (LOCKED 0 装诚实授权) + 子代理 I 真兑现 RC-11 migration script
c481b123 docs: 补子代理 G 独立判断 (migration script 必校验 v1 id ≤ 65535)
ae182c8c docs+fix: 补子代理 F 2 P1 (record_id 明文 + migration script ROADMAP P1)
413fe12b chore: gitignore .apeireth/ (cognitive module runtime 产物)
0e9adb52 docs+fix: 哲学锚 ledger 真实数字 + 子代理 E 3 建议落地 (子代理 D actionable #2)
38cc1039 refactor: v2.0.0-rc.1 RC-10 line header AAD tamper 保护 (子代理 C 建议 #5 兑现)
71aaa919 docs: record remote cognitive validation
e5dbca06 fix(runtime): keep judge feedback out of persistence
acd8c5e7 feat(runtime): wire cognitive modules through canonical root
0ec9ccae docs: 接手人交付 (CHANGELOG/ROADMAP/3 internal docs + HANDOFF-NOTES)
67c06d95 fix(runtime): close expired approval transcripts
4e4fba89 refactor: v2.0.0-rc.1 RC-2 Experience trait 真 SQLite + RC-8 改名 (子代理 C 反馈修正)
64e64f46 fix(runtime): preserve cognitive hook lifecycle invariants
1d227d6a feat(runtime): integrate cognitive module hooks and overlays
a699c5f5 feat(runtime): add cognitive module hook ABI
e2a5be08 refactor: v2.0.0-rc.1 RC-10 File AES-256-GCM 加密 (EncryptedFileBackend)
aa661a66 refactor: v2.0.0-rc.1 RC-9 keyring 真接入 CLI bootstrap
67fc66a0 refactor: RC-8 SubSupervisor tokio::process 写真 + 子代理 A 错误类型注释
042ad4eb refactor: v2.0.0-rc.1 RC-4 SelfAssessmentStore SQLite impl (场景 D 例 2)
61cc0421 fix: 子代理审查 3 项修正 (RC-1 + RC-3 真 SQL impl 兑现 + SelfAssessment 单 source of truth)
43ec9635 refactor: v2.0.0-rc.1 RC-1 真实 SQL 重写 (MemoryBackend trait SqliteBackend)
ca0f48e9 refactor: O-6 锚 #18 + #19 + #23 - HistoryEntry typed + Council DeferToHuman + NoopLlmFactory
78ee5d51 feat(plugin): LlmFactory trait 接口定义 (RC-5 前置, 0 装 alpha)
b558c201 refactor: O-6 锚 #2 兑现 - StreamKind 6 流 typed enum + MemoryBackend trait 撤占位
03f5ed71 refactor: RC-3 NoopPreferenceStore + RC-4 SelfAssessmentStore trait 提前
ed0a0913 refactor: O-6 锚 #11 收回 + #5 decision + PreferenceStore trait + 真 core drain 完成
a98a636d docs: ROADMAP §3 + CHANGELOG + philosophy.md O-6 教训整合
240f3277 ci: O-6 锚兑现 #8 #9 (doc test workflow + 5 重守门 workflow)
c55e3911 refactor(plugin): O-6 锚兑现 #10 #11 #12 (文档位置 + kernel re-export + 统一 error)
d42d7c1e refactor(core): O-6 Refactor-5 - core drain 真正重定义 (5/5 完成)
7d48c76e refactor(credentials): O-6 Refactor-4 - plugin_bridge.rs → keyring_resolver.rs 重命名
f2cfaa76 refactor(plugin): O-6 Refactor-2+3 - Experience + Perception traits 搬到 plugin
30d342fa refactor(plugin): O-6 Refactor-1 - MemoryBackend trait 搬到 plugin
```

---

## 7. 给新团队最后一段话 (v2.0 release 后 6 个月内)

```
Apeireth v2.0 = 9 organ 真移植 ✅ + OrganOrchestrator 串联层 ✅ + OrganOrchestrator 完整化 (A 块) ✅ + frontend 对接 + 自我升级 cycle.

新团队:
1. 接手 **3 个**真生产前阻塞 (per §5.3, A 块已 ✅): frontend + 6 DEFERRED + RC-7
2. 派子代理 = 调研/验证/真写 (有目的), 主代理拍板 + 亲验报告
3. 0 装诚实原则 = 真兑现, 不假装, 不漂移 (HEAD 漂移是病, 数字必实测)
4. 9 哲学锚 + 13 键 + 5 重守门 = 信任地基, LOCKED 0 改
5. O-6 三阶审查 = 每 commit 必真答案 + 拒 alternatives + 拒理由 (per `organ-orchestrator-completion-plan.md` §7, 不找借口)
6. v2.0 release 估 2027-01-08 至 2027-02 月, 4-6 月 (因 A 块提前完成)
7. 完整版给新团队的话: docs/04-internal/TO-NEW-TEAM.md
8. A 块 O-6 复盘: docs/04-internal/A-block-o6-true-account.md

主代理 Mavis 收盘 rc.1 + 8 spec + A 块完整化 + O-6 复盘 阶段, 你来接.
有疑问看 docs/04-internal/ + docs/01-architecture/ + docs/02-guides/ + 跑 5 重守门 baseline (期望 1739 tests / 0 clippy 警告).
```

---

_本文档 Final-1.0 首发 (2026-08-28, 主代理 Mavis 写于 rc.1 收盘 session, HEAD = `395fe0f0`, 19 commit). Final-2.0 更新 (2026-08-28): 9 organ 真移植 100% + R12 OrganOrchestrator 真实施落地 (`2550b99d`) + 8 spec 收齐 (R9-R15 + Z) + 6 处错账修正 (`ccf29c57`) + 1726 tests 0 FAILED + 85 commit 实测 + 16 crates. 0 触碰 LOCKED, 真 LLM call 1.16s 跑通. 接手人按 §5.4 10 步读 + §5.3 4 阻塞真做, v2.0.0 release 估 2027-01-08 至 2027-03 月. **Final-2.1 更新 (2026-08-28)**: A 块 OrganOrchestrator 完整化 5 stage 真实施 (amend 后 commits `c003e078` ~ `0afa733f`) + O-6 三阶审查 amend 复盘 (commit `bbbfb75b`); 1739 tests 0 FAILED (1726 baseline + 13 new A 块); 剩 **3 块**真实施 (frontend / 6 DEFERRED / RC-7); v2.0.0 release 估 **2027-01-08 至 2027-02 月, 4-6 月** (因 A 块提前完成, 从 5-7 月缩短为 4-6 月)._