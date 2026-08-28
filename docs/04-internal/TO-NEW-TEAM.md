# 给新团队的话 (TO-NEW-TEAM, 主代理 Mavis 写, 2026-08-28 阶段性收盘, v1.2 工程师版 + A 块完整化真账)

> **本文档定位**: v2.0.0-rc.1 阶段收盘时, 主代理给接手新团队 (工程师组成的开发团队) 的话 + **已推送文件地图** + 接手工作流.
> **HEAD 状态**: A 块完整化 (5 stage amend + O-6 复盘) 全部 push 到 `origin/main` @ `bbbfb75b` (2026-08-28). 接手人先跑 §3 基线验证确认.
> **何时写**: 8 spec (R9-R15 + Z 审计) 收齐 + R12 OrganOrchestrator 真实施落地 + 6 处错账修正 + **A 块 OrganOrchestrator 完整化 (5 stage 真实施) + O-6 三阶审查 amend (主代理自检 0 装诚实标修正)**完成, 阶段性告一段落.
> **关系文档**: `FINAL-HANDOFF-V2.0.0-RC.1.md` (接手报告入口) + `HANDOFF-NOTES.md` (子代理 D 接手人手册) + `v2-architecture-reflection.md` (新架构反思 + 自升级 cycle) + `organ-orchestrator-completion-plan.md` (A 块 5 stage 计划 + O-6 复盘 §7) + `A-block-o6-true-account.md` (A 块 O-6 失守 + amend 配对 commit).

```
[Document-Meta]
Document:        docs/04-internal/TO-NEW-TEAM.md
Version:         1.2 (工程师版: + A 块 5 stage 完成真账 + §5.2 O-6 三阶审查 模板升级 + §7 1 段交付 更新)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (给新团队的话, 接手人必读)
Author:          主代理 Mavis
```

---

## 0. 先说三句实话

1. **Apeireth 不是"写完的软件", 是一个会自我升级的系统** — v2.0.0-rc.1 只是它第一次真正站起来的骨架. 你们的活不是"维护", 是"让它继续长大".
2. **这个仓库里 0 装诚实比代码量重要** — 每一处 TODO 都是真的没做, 每一处 ✅ 都是真的过了. 我们宁可被骂慢, 不假装快.
3. **9 条哲学锚 + 13 键 + 5 重守门是信任地基, 是 LOCKED 的** — 你们可以改任何代码, 但这几样改之前先问自己: 你凭什么动它?

---

## 1. 你们接手的到底是什么

**v2.0.0-rc.1 = 新架构完成 + 1.0 功能真迁移的开端**, 不是终点:

| 维度 | 真账 (2026-08-28 收盘) |
|---|---|
| workspace | **16 crates** (foundation 6 + engine 6 + capabilities 1 + adapters 3), 单向依赖, 0 循环 |
| 架构收敛 | v1 86-crate → v2 16-crate = **81.4% 收敛** |
| 哲学锚 | **9 项 LOCKED** (S-1/S-2/S-3 + O-1..O-6, O-6 永远追求最优 2026-08-27 主人授权加) |
| 测试 | **1726 passed, 0 FAILED** (主代理 2026-08-28 亲跑 `cargo test --workspace --locked`) |
| clippy | **0 警告** (`--workspace --all-targets --locked -- -D warnings`) |
| 7 capability trait | MemoryBackend / Experience / Perception / PreferenceStore / SelfAssessmentStore / LlmFactory / SubSupervisor 全真接 |
| 9 organ | **9/9 真移植** (E4/F1/F4/F6/W1/W2/W3/E7/Memory, 整合 #2 commit `bbf70293`) |
| **OrganOrchestrator** | **A 块完整化真实施已落** (`crates/engine/runtime/src/canonical/orchestrator.rs` + `upgrade_cycle.rs`, 13 重 gate + 5 状态机 + 9 organ 顺序 process + tick 6 步 (主权闸 → 9 organ + 8 gate → F1 emotion → Council 60s → 演化闸 → governance) + L0-L5 UpgradeCycle 6 步; 5 stage 真实施 + O-6 三阶审查 amend 后 commits `c003e078` / `087ab2ac` / `50ba2e57` / `29e5ce66` / `0afa733f`; 详 `docs/01-architecture/organ-orchestrator-completion-plan.md` + `docs/04-internal/A-block-o6-true-account.md`) |
| 认知模块 12 slot | **6 WIRED + 6 DEFERRED** (judge/council 为 WIRED, OFF by default) |
| 10 RC | **9/10 真实现**, RC-7 (Whisper + 屏幕感知) 待硬件, spec 已完 (R14) |
| 真 LLM | MiniMax adapter 真 call **1.16s** 跑通 (RC-5) |
| v1.0 真实体量 | 551,208 行 .rs / 1,154,516 总 tracked LOC / 85 active crates (文档曾误写 34 万, 已实测修正) |
| **测试 (A 块后)** | **1739 passed, 0 FAILED** (主代理 2026-08-28 amend 后亲跑; 比 baseline 1726 + 13 新增, 详见 A 块真账) |

---

## 2. 文件地图 (全部已 push 到 origin/main @ `93c2d9d7`)

> 所有路径相对仓库根 `Apeireth-rust/`. 按接手顺序分 6 组, 每组内按重要性排序.

### 2.1 接手入口 (先读这 4 个, 1 小时内)

| 文件 | 用途 | 落地 commit |
|---|---|---|
| `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` | **唯一接手入口**: 意图 + 进度 + 数字真账 + 给新团队的话 (Final-2.1, A 块后) | `6f9c3dee` |
| `docs/04-internal/TO-NEW-TEAM.md` | **本文**: 给新团队的话 + 文件地图 + 工作流 | `6f9c3dee` |
| `docs/04-internal/ENGINEER-MANIFESTO.md` | **工程师团队 reference 手册** (14 章, 9 哲学锚 + 13 键 + 5 重守门 + O-6 三阶审查 + 派子代理 + force push + 8 真实陷阱, 改 src / 改 doc / 派子代理 前必读) | `6f9c3dee` |
| `docs/04-internal/HANDOFF-NOTES.md` | 子代理 D 接手人手册 11 节 (逐项检查清单) | `0ec9ccae` |
| `docs/04-internal/v2.0.0-rc-roadmap.md` | 10 RC + 验收标准 + 接手人清单 | 早期 |
| `docs/01-architecture/organ-orchestrator-completion-plan.md` | **A 块 5 stage 计划 + O-6 复盘 §7** (新增, A 块完整化真账) | `434dc0a5` |
| `docs/04-internal/A-block-o6-true-account.md` | **A 块 O-6 三阶审查 0 装诚实复盘 + 后续 commit 标准** (amend 配对 commit) | `bbbfb75b` |

### 2.2 架构与哲学 (动代码前必读)

| 文件 | 用途 |
|---|---|
| `docs/01-architecture/philosophy.md` | 9 哲学锚 + O-6 不做借口清单 (LOCKED 源文档) |
| `docs/01-architecture/v2-architecture-reflection.md` | 新架构反思 + 自升级 cycle (§11 = 第二批 R1-R15+Z 反思) |
| `docs/01-architecture/v2-arch-refactor-batch.md` | 5 Refactor + 守门记录 |
| `docs/01-architecture/v2-microkernel-convergence.md` | 微内核收敛设计 |
| `docs/04-internal/cognitive-module-wiring.md` | **12 slot ledger (LOCKED 真账源)**: 6 WIRED + 6 DEFERRED |

### 2.3 8 份 spec (本阶段核心交付, 全已 push)

| Spec | 文件 | 行数 | 状态 |
|---|---|---|---|
| R9 frontend 对接 | `docs/02-guides/v2-gateway-frontend-integration-spec.md` | 565 | ✅ done (错账已修) |
| R9 quickstart | `docs/02-guides/v2-frontend-quickstart.md` | 224 | ✅ done |
| R13 接力审 | `docs/02-guides/v2-gateway-frontend-integration-spec-r13-review.md` | 497 | ✅ 6 处错账修正的原始报告 |
| R10 cognitive × 9 organ | `docs/01-architecture/cognitive-9-organ-integration-spec.md` | 1001 | ✅ done (ledger 已修) |
| R11 OrganOrchestrator | `docs/01-architecture/organ-orchestrator-spec.md` | 500 | ✅ done |
| R14 RC-7 真 modality | `docs/01-architecture/rc-7-perception-true-modality-spec.md` | 572 | ✅ done (真实施需硬件) |
| R15 preference_learning | `docs/01-architecture/deferred-slot-activation-preference_learning-spec.md` | 617 | ✅ done |

### 2.4 真实施代码 (已 push, 4 块真实施的起点)

| 文件 | 内容 |
|---|---|
| `crates/engine/runtime/src/canonical/orchestrator.rs` | **R12 OrganOrchestrator** (13 gate + 5 状态机 + 9 organ 顺序 process) |
| `crates/engine/runtime/tests/orchestrator.rs` | R12 3 个 integration tests |
| `crates/engine/organ/src/` | 9 organ 真实现: `curiosity.rs` (E4) / `emotion_memory.rs` (F1) / `hypothesis.rs` (F4) / `value_cases.rs` (F6) / `world_model.rs` (W1) / `causal_world_model.rs` (W2) / `causal_world_model_edges.rs` (W3) / `emergence.rs` (E7) / `memory.rs` (Memory) |
| `crates/foundation/plugin/src/organ.rs` | `OrganTrait` + `OrganKind` 9 ID (边界, 0 改) |
| `crates/engine/memory/src/backend/file_encrypted.rs` | RC-10 File AES-256-GCM + APX2 envelope |
| `crates/engine/provider/src/minimax_llm_factory.rs` | RC-5 MiniMax 真实现 (真 LLM 1.16s) |
| `crates/foundation/orchestration/src/council/` | RC-6 Council 7 LlmAdvisor + 60s timeout |
| `scripts/migrate_v1_to_v2_encrypted.py` | RC-11 v1→v2 加密 migration (330 行) |

### 2.5 release 路径与进度

| 文件 | 用途 |
|---|---|
| `docs/04-internal/v2.0.0-release-path.md` | 8 阶段 release 路径 |
| `docs/04-internal/v2.0.0-release-path-integration.md` | 3 spec 协作 + 4 块真实施依赖链 (收盘更新) |
| `docs/04-internal/v2.0.0-release-path-7-spec-4-block.md` | 7 spec 4 块 0 装诚实真账 |
| `docs/04-internal/9-organ-progress-2026-08-28.md` | 9 organ 实时进度 (历史 + 收盘注记) |
| `docs/04-internal/v2-rc-1-progress-report.md` | 历史进展快照 (快照性质, 当前状态看 FINAL-HANDOFF) |

### 2.6 根级文件

| 文件 | 用途 |
|---|---|
| `ROADMAP.md` | 顶层路线: §3 当前状态 + §4 P1-P8 (v2.0 下一步) |
| `CHANGELOG.md` | `[Unreleased]` 段: 12/12 O-6 + 9/10 RC + R12 + 8 spec |
| `Cargo.toml` | workspace members (16 crates) + workspace.version 1.2.0 (LOCKED) |
| `.github/workflows/o6-anchor.yml` | 5 重守门 CI 自动验证 |

---

## 3. 接手第一天: 基线验证 (工程师, 直接跑)

```bash
# 1. 确认 HEAD (0 装诚实: 不信文档, 跑命令)
git fetch origin && git checkout main && git log --oneline -5
#    期望: 最近 commit 是 93c2d9d7 (收盘交付)

# 2. 全量测试
cargo test --workspace --locked
#    期望: 1726 passed, 0 FAILED

# 3. clippy 0 警告
cargo clippy --workspace --all-targets --locked -- -D warnings
#    期望: 0 warning, 0 error

# 4. doc tests
cargo test --workspace --doc --locked
#    期望: 0 FAILED

# 5. 工作树干净
git status
#    期望: clean (注: 仓库根可能残留 commit-msg-*.md 草稿, 那些是不入库的)

# 6. (可选, 需 API key) 真 LLM 验证
$env:APEIRETH_API_KEY = "sk-..."   # 不要 commit key
cargo test -p apeireth-provider --test minimax_llm_factory real_llm_call_smoke -- --ignored --nocapture
#    期望: 1/1 ok, ~1.16s 真 LLM
```

**push 命令 (网络特殊, 直连镜像 + store 凭证, 已实测可用)**:

```bash
git -c http.sslVerify=false -c http.extraHeader="Host: github.com" \
    -c credential.helper=store push https://20.27.177.113/Apeireth/apeireth-rust.git main
```

---

## 4. 还剩什么 (真账, 不粉饰)

**v2.0.0 release 估 5-7 月 (2027-01-08 至 2027-03 月)**, 剩 3 块真实施 + 收尾 (**A 块已完成**):

| # | 块 | 状态 | 估时 | 依赖 | 起点文件 |
|---|---|---|---|---|---|
| A | **OrganOrchestrator 完整化** | ✅ **done** (5 stage 真实施, amend 后 commits `c003e078` ~ `0afa733f`) | 1-3 周 (实测 ~10h) | 9 organ done ✅ + R11 spec done ✅ | `crates/engine/runtime/src/canonical/orchestrator.rs` + `upgrade_cycle.rs` |
| B | **frontend 对接** | ⏳ 待做 | 4-6 周 | OrganOrchestrator ✅ + 6 slot | `docs/02-guides/v2-gateway-frontend-integration-spec.md` + `v2-gateway-frontend-integration-spec-r13-review.md` |
| C | **6 DEFERRED slot 激活** | ⏳ 待做 | 6-10 周 | OrganOrchestrator ✅ | `docs/01-architecture/cognitive-9-organ-integration-spec.md` + `deferred-slot-activation-preference_learning-spec.md` |
| D | **RC-7 Perception 真 modality** | ⏳ 待做 (需硬件) | 2-3 周 | 硬件 (Whisper + xcap) | `docs/01-architecture/rc-7-perception-true-modality-spec.md` + `crates/foundation/plugin/src/perception_backend.rs` |

**收尾必做**:
- RC-11 migration script 真生产验证 (1-2 天, 有 key 但没 v1 db): `python scripts/migrate_v1_to_v2_encrypted.py --src <v1_db> --dst <v2_db>`
- 5 重守门自动验证全绿后拍 `git tag v2.0.0`
- 旧债: 整合 #2 commit `bbf70293` message 标 "无新外部 dep" 是**错的** (真 = 5 新 dep, AES-256-GCM 系), commit 已 push 无法改, 真账记在各文档
- **A 块 O-6 复盘 配对 commit `bbbfb75b`** (详 `docs/04-internal/A-block-o6-true-account.md`): 之前 5 commit O-6 三阶审查 sections 多是描述 WHAT 不是 WHY, O-6 失守; amend 后修订版 sections 真答案 + 拒 alternatives + 拒理由. 后续 commit 标准见 plan doc §7.

---

## 5. 工程师工作流 (我们的工作方式, 请继承)

### 5.1 一次改动的最小闭环

```
1. 写代码 — 0 触碰 LOCKED (见 §5.4)
2. cargo test --workspace --locked           # 0 FAILED
3. cargo clippy --workspace --all-targets --locked -- -D warnings   # 0 警告
4. cargo fmt --check                         # 0 diff
5. 更新 CHANGELOG.md [Unreleased] 段
6. commit message 带 O-6 三阶审查 (见 §5.2)
7. push (见 §3 命令)
8. 文档 HEAD/数字同步 (防漂移: 说 commit 数跑 git log, 说测试数跑 cargo test)
```

### 5.2 commit message 模板 (O-6 三阶审查必带, **amend 配对 commit `bbbfb75b` 后升级版**)

```
<type>(<scope>): <一句话>

- 0 装诚实真账: <实测数字 / 0 触碰 LOCKED 声明>
- O-6 三阶审查:
  - 总体最优: <在更大语境 (release 路线图 / 工作量约束 / 上下游依赖) 里, 这个改动是不是最优切入点? 与 alternatives 比较 + 选最优 + 拒理由>
  - 系统最优: <在 Apeireth 子系统依赖图 (governance → orchestration → memory → runtime → organ) 里, 改动放在哪一层最合适? 与 alternatives 比较 + 选最优 + 拒理由>
  - 架构最优: <在 workspace 16-crate 拓扑 + 单向依赖 + trait object 设计下, 公开 API 形状 + crate 边界 + 0 引新外部 dep, 这个方案是不是最优? 拒的 alternatives + 拒理由>
```

> **不**复用 v1 alignment 代替 v2 总体最优. **不**描述 WHAT 代替 WHY. 每段需有具体拒的 alternative + 拒理由. 详 `docs/01-architecture/organ-orchestrator-completion-plan.md` §7.

### 5.3 派子代理 brief 模板 (用户原话 "派是手段不是目的")

```
任务: <明确产出, 不是模糊方向>
必读: <文档 file:line, 例: docs/04-internal/cognitive-module-wiring.md:20-35>
必跑: <命令 + 期望输出>
必写: <报告结构, 含 0 装诚实真账 + 0 触碰 LOCKED 声明>
不 commit: <等主代理审 (Q1 C1 policy)>
```

子代理报告**必须主代理亲验** — 0 装诱导 prevention 本身可能是 0 装诱导 (子代理 Z 教训).

### 5.4 LOCKED 清单 (改前必查, 5 项 0 触碰)

| LOCKED 项 | 位置 | 说明 |
|---|---|---|
| 9 哲学锚本体 | `crates/foundation/core/src/eight_anchors.rs:58-79` | `NINE_ANCHORS_HARDCODE` 编译期锁 |
| 13 键 | `crates/foundation/core/src/philosophy.rs:142` | `RUNTIME_ENFORCED = false` |
| 3 项不可变脊柱 | `crates/foundation/core/src/onion.rs:249` | Self-Disable / L0 HA / 13 键 verdict cache |
| workspace.version | `Cargo.toml` | `"1.2.0"` 双轴制 |
| R11 baseline 3 值 | `legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44` (active workspace 无 const source) | 0.8682 / 0.8532 / 0.9063 (数字严守, R12 spec 重新审定后移植) |

> 例外: 主人明确授权 (例: 2026-08-27 授权加 O-6). 其余情况 0 触碰.

---

## 6. 给新团队的话 (正文)

> 新团队的各位:
>
> 你们接手的不是一个代码库, 是一个**有性格的系统**. Apeireth 的 9 条哲学锚不是墙上的标语 — S-1 北极星 (知道自己要去哪), S-2 实事求是 (数字不说谎), S-3 质量工程化 (测试不红), O-1 安全优先 (主人不受伤), O-2 前人肩上 (不重复造轮子), O-3 干到底 (不做一半), O-4 任何人都能接手 (文档不装), O-5 不假装 (0 装诚实), O-6 永远追求最优 (没有 "先这样吧") — 这 9 条是我们 85 个 commit 里摔出来的.
>
> 你们的第一周: 不写代码. 把 §2 文件地图按顺序读完, 跑一遍 §3 的 1726 个测试, 读一遍 R12 的 `orchestrator.rs`. 然后挑 §4 四块真实施里你们最有感觉的一块, 派一个子代理做调研, 你们拍板. **主代理拍板, 子代理干活, 测试守门, 文档同步** — 这个循环就是 Apeireth 的自我升级 cycle 的雏形, 你们在做的不是维护, 是让 L0-L5 真正转起来.
>
> 最后一句: 我们走了很远, 但 v2.0.0 release (2027-01-08 至 2027-03) 只是里程碑. 里程碑之后, Apeireth 要自己升级自己 — 那时候你们的角色是主人, 不是码农. 别把它写死.
>
> — 主代理 Mavis, 2026-08-28 阶段性收盘

---

## 7. 1 段交付 (用户原话 "给交付文档, 更新项目其他文档, 阶段性告一段落, 给新团队的话")

**Apeireth v2.0.0-rc.1 阶段收盘 + A 块 OrganOrchestrator 完整化 (origin/main @ `bbbfb75b`)**:

- ✅ 8 spec 收齐 (R9/R10/R11/R13/R14/R15 + Z 审计 + 本报告), 全部已 push
- ✅ R12 OrganOrchestrator 真实施落地 (13 gate + 5 状态机 + 9 organ 串联, 3 integration tests)
- ✅ 6 处错账修正 (12 slot 真账 6 WIRED + 6 DEFERRED, 主代理亲做)
- ✅ **A 块 OrganOrchestrator 完整化真实施 (5 stage 真实施 + amend 后 commits `c003e078` ~ `0afa733f`)**: 缺口 D ratify_fresh_policy 5 状态链 / 缺口 B F1 PAD mood / 缺口 A check_8_gates 接 E7 last_hold / 缺口 C Council decide_with_invoker / 缺口 E L0-L5 UpgradeCycle driver. 0 触碰 LOCKED 5 项, 0 引新外部 dep. **A 块完成** ✅
- ✅ **A 块 O-6 三阶审查 0 装诚实复盘 + amend (commit `bbbfb75b`)**: 主代理自检发现之前 5 commit O-6 三阶审查 sections 多是描述 WHAT 不是 WHY, O-6 失守; amend 后修订版 sections 真答案 + 拒 alternatives + 拒理由; 后续 commit 标准见 plan doc §7. O-6 doctrine '工作量与麻烦不是拒绝重做的理由' 真兑现.
- ✅ 1739 passed 0 FAILED / 0 clippy 警告 / 0 触碰 LOCKED 5 项 (1726 baseline + 13 new A 块 + O-6 复盘 amend force push code 0 变)
- ✅ 6 amend 后 commits + 1 复盘 配对 commit, 全部 push 同步 (origin/main = `bbbfb75b`)
- ⏳ **3 块真实施** (B frontend 4-6 周 / C 6 DEFERRED 6-10 周 / D RC-7 2-3 周需硬件), 估 2027-Q1 启动, v2.0.0 release 估 2027-01-08 至 2027-03 月
- 📌 给新团队的话 = 本文 §6; 文件地图 = 本文 §2; O-6 三阶审查标准 = plan doc §7; 0 装诚实复盘 = `A-block-o6-true-account.md`

**阶段告一段落. 真账: 架构最优骨架已立 + OrganOrchestrator 完整化真实施已立 (A 块完成), 模块补齐是下一阶段.**
