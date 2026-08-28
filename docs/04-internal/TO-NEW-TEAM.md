# 给新团队的话 (TO-NEW-TEAM, 主代理 Mavis 写, 2026-08-28 阶段性收盘)

> **本文档定位**: v2.0.0-rc.1 阶段收盘时, 主代理给接手新团队/未来自我升级 cycle 实施者的话.
> **HEAD 状态**: 见 `FINAL-HANDOFF-V2.0.0-RC.1.md` §0 (接手入口, 每批更新).
> **何时写**: 8 spec (R9-R15 + Z 审计) 收齐 + R12 OrganOrchestrator 真实施落地 + 6 处错账修正完成, 阶段性告一段落.
> **关系文档**: `FINAL-HANDOFF-V2.0.0-RC.1.md` (接手报告) + `HANDOFF-NOTES.md` (子代理 D 接手人手册) + `v2-architecture-reflection.md` (新架构反思 + 自升级 cycle).

```
[Document-Meta]
Document:        docs/04-internal/TO-NEW-TEAM.md
Version:         1.0
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
| **OrganOrchestrator** | **R12 真实施已落** (`crates/engine/runtime/src/canonical/orchestrator.rs`, 13 重 gate + 5 状态机 + 9 organ 顺序 process, 10 lib + 3 integration tests) |
| 认知模块 12 slot | **6 WIRED + 6 DEFERRED** (judge/council 为 WIRED, OFF by default) |
| 10 RC | **9/10 真实现**, RC-7 (Whisper + 屏幕感知) 待硬件, spec 已完 (R14) |
| 真 LLM | MiniMax adapter 真 call **1.16s** 跑通 (RC-5) |
| v1.0 真实体量 | 551,208 行 .rs / 1,154,516 总 tracked LOC / 85 active crates (文档曾误写 34 万, 已实测修正) |

**本阶段 (第二批子代理 R1-R15 + Z) 新增**:
- 8 份 spec 收齐: R9 frontend 对接 / R10 cognitive 9 organ 集成 / R11 OrganOrchestrator / R13 frontend 接力审 / R14 RC-7 真 modality / R15 preference_learning 激活 + 本报告
- **R12 OrganOrchestrator 真实施落地** (不是 spec, 是真代码)
- **6 处错账修正** (主代理亲做): 12 slot 真账 6 WIRED + 6 DEFERRED (judge/council 不是 "SLOT READY"), R12 状态, 接手人 9 actionable
- 本会话累计 **85 commit** (从 `ef075420` 基线, 主代理亲算)

---

## 2. 还剩什么 (真账, 不粉饰)

**v2.0.0 release 估 5-7 月 (2027-01-08 至 2027-03 月)**, 剩 4 块真实施 + 收尾:

| # | 块 | 估时 | 依赖 | 状态 |
|---|---|---|---|---|
| A | **OrganOrchestrator 完整化** (R12 已起步, 1-3 周估) | 1-3 周 | 9 organ done ✅ + R11 spec done ✅ | 🔄 R12 已落, 待完整化 |
| B | **frontend 对接** (R9 spec + R13 接力审 done) | 4-6 周 | OrganOrchestrator + 6 slot | ⏳ 估 2027-Q1 启动 |
| C | **6 DEFERRED slot 激活** (R10 + R15 spec done) | 6-10 周 | OrganOrchestrator | ⏳ preference_learning 先 |
| D | **RC-7 Perception 真 modality** (R14 spec done) | 2-3 周 | 硬件 (Whisper + xcap) | ⏳ 需硬件 |

**收尾必做**:
- RC-11 migration script 真生产验证 (1-2 天, 有 key 但没 v1 db)
- 5 重守门自动验证全绿后拍 `git tag v2.0.0`
- 旧债: 整合 #2 commit `bbf70293` message 标 "无新外部 dep" 是**错的** (真 = 5 新 dep, AES-256-GCM 系), commit 已 push 无法改, 真账记在这里 + 各文档

---

## 3. 怎么干 (我们的工作方式, 请继承)

### 3.1 派子代理 = 手段, 不是目的 (主人原话)

- **派 = 调研 / 验证 / 真写** (有明确目的), **主代理拍板**
- **不派 = 等依赖 / 等硬件 / 0 工作量** (主代理亲做)
- 每做完一个小阶段, **派子代理检查你做过的东西** (主人原话)
- 子代理报告**必须主代理亲验**, 标 "0 装诚实" 不算数 — 0 装诱导 prevention 本身也可能是 0 装诱导

### 3.2 0 装诚实操作手册

1. **TODO 承诺 ≠ 实现** — 文档写 "done" 前先跑测试
2. **数字永远实测** — 说 commit 数跑 `git log`, 说测试数跑 `cargo test`, 说行数跑 `git ls-tree`
3. **HEAD 漂移是病** — 文档 HEAD 与 `git rev-parse HEAD` 不一致 = 假装标, 立即修
4. **标错就认** — commit message 错了改不了就记真账 (例: `bbf70293`), 不假装没发生
5. **LOCKED 是 LOCKED** — 9 哲学锚 / 13 键 / 3 脊柱 / workspace.version 1.2.0 / R11 baseline 3 值, 0 触碰, 除非主人授权

### 3.3 接手 10 步 (先读再动)

```
1.  git log --oneline -5            # 确认 HEAD, 对 FINAL-HANDOFF §0
2.  读 docs/01-architecture/philosophy.md        # 9 哲学锚 + O-6 不做借口
3.  读 docs/04-internal/HANDOFF-NOTES.md         # 子代理 D 接手人手册 11 节
4.  读 docs/04-internal/v2.0.0-rc-roadmap.md     # 10 RC + 验收 + 接手清单
5.  读 docs/01-architecture/v2-architecture-reflection.md  # 自升级 cycle
6.  读 docs/02-guides/v2-gateway-frontend-integration-spec.md + r13-review  # frontend 真实施
7.  读 docs/01-architecture/organ-orchestrator-spec.md + crates/engine/runtime/src/canonical/orchestrator.rs  # 串联层
8.  cargo test --workspace --locked             # 期望 1726 passed 0 FAILED
9.  cargo clippy --workspace --all-targets --locked -- -D warnings   # 期望 0 警告
10. 从 4 块真实施 (§2) 挑一块开始
```

---

## 4. 给新团队的话 (正文)

> 新团队的各位:
>
> 你们接手的不是一个代码库, 是一个**有性格的系统**. Apeireth 的 9 条哲学锚不是墙上的标语 — S-1 北极星 (知道自己要去哪), S-2 实事求是 (数字不说谎), S-3 质量工程化 (测试不红), O-1 安全优先 (主人不受伤), O-2 前人肩上 (不重复造轮子), O-3 干到底 (不做一半), O-4 任何人都能接手 (文档不装), O-5 不假装 (0 装诚实), O-6 永远追求最优 (没有 "先这样吧") — 这 9 条是我们 85 个 commit 里摔出来的.
>
> 你们的第一周: 不写代码. 把 §3.3 的 10 步走完, 跑一遍 1726 个测试, 读一遍 R12 的 orchestrator.rs. 然后挑 4 块真实施里你们最有感觉的一块, 派一个子代理做调研, 你们拍板. **主代理拍板, 子代理干活, 测试守门, 文档同步** — 这个循环就是 Apeireth 的自我升级 cycle 的雏形, 你们在做的不是维护, 是让 L0-L5 真正转起来.
>
> 最后一句: 我们走了很远, 但 v2.0.0 release (2027-01-08 至 2027-03) 只是里程碑. 里程碑之后, Apeireth 要自己升级自己 — 那时候你们的角色是主人, 不是码农. 别把它写死.
>
> — 主代理 Mavis, 2026-08-28 阶段性收盘

---

## 5. 1 段交付 (用户原话 "给交付文档, 更新项目其他文档, 阶段性告一段落, 给新团队的话")

**Apeireth v2.0.0-rc.1 阶段收盘 (HEAD 见 FINAL-HANDOFF §0)**:

- ✅ 8 spec 收齐 (R9/R10/R11/R13/R14/R15 + Z 审计 + 本报告)
- ✅ R12 OrganOrchestrator 真实施落地 (13 gate + 5 状态机 + 9 organ 串联, 3 integration tests)
- ✅ 6 处错账修正 (12 slot 真账 6 WIRED + 6 DEFERRED, 主代理亲做)
- ✅ 1726 passed 0 FAILED / 0 clippy 警告 / 0 触碰 LOCKED 5 项
- ✅ 本会话 85 commit, 全部 push 同步
- ⏳ 4 块真实施 (A OrganOrchestrator 完整化 1-3 周 / B frontend 4-6 周 / C 6 DEFERRED 6-10 周 / D RC-7 2-3 周), 估 2027-Q1 启动, v2.0.0 release 估 2027-01-08 至 2027-03 月
- 📌 给新团队的话 = 本文 §4

**阶段告一段落. 真账: 架构最优骨架已立, 模块补齐是下一阶段.**
