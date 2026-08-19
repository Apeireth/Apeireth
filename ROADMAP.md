# ROADMAP — Apeireth (1.0 已发布 → 后 1.0 阶段)

```
[Document-Meta]
Document:        ROADMAP.md
Version:         2.0-post1.0
R-Cycle:         后-R178 (master 已 8/18 v1.0.0 正式发布 + 后续 CI 收尾 + companion-desktop PR #1 合并)
Last-Modified:   2026-08-19
Status:          🟢 活跃
Source-of-Truth: CHANGELOG.md [2026-08-18] v1.0.0 + RELEASE_NOTES.md + 决策 #62/#74/#126/#128/#130
0 主动 commit:   解除 (per 决策 #126 Mavis 全自决 commit); 本次 ROADMAP 更新按 doc-change commit
0 主动 push:     仍严守 (等 1.0 release 配 GitHub remote 后主人拍板)
master HEAD:     37fa420e (fix(ci): simplify Install Rust targets to flat list)
release-1.0:     e27ac0b2 (feat(companion): 连续感知①②) — release-1.0 分支顶
v1.0.0 tag:      993e9107 (docs(1.0): RELEASE_NOTES 正式版重写 (主人拍板真正的 1.0))
cherry-pick:     进行中 (master HEAD 在 pick d1912c53 — feat(companion): 连续感知①②
                  修改了 2 个 generated schema 文件: desktop-schema.json / windows-schema.json;
                  主人表示"有活在干", 暂不 continue/skip/abort)
```

> **本次重写 (后-R178, 2026-08-19)**：顶层 ROADMAP 从 R127-2 时代 (8/10) 升级到 v1.0 实际已发布 (8/18) 的真实状态，
> 反映 R128-R178 横扫 + 1.0-final 收尾 + companion-desktop PR #1 + 11 项 CI 修复 + 当前 cherry-pick 状态。
> 详单下沉 `docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md`。

---

## 0. TL;DR

**Apeireth v1.0.0 已正式发布 (2026-08-18, 主人拍板"真正的 1.0", tag `v1.0.0` → commit `993e9107`)**。

- **v1.0 实际发布路径**：不是 R127 那个 1.2.0 的"1.0 era"，是 R128 (8/12, 51 active) → R129-R177 (8/12-8/17, tool orchestrator/Kani helloworld/20548 tests/形式化 79 crates) → **12d4323b "Apeireth v1.0.0 正式版"** (8/18 主人拍板) → 993e9107 RELEASE_NOTES 重写 → v1.0.0 tag
- **51 → 85 active crates**（workspace 收敛 + companion_serve + 5 新 crate: runtime/workflow/host/repo-tools/companion_serve）
- **后 v1.0 阶段 (8/18-8/19)**：brand 定稿 (Apeiron 命名 + 完整品牌宣言) + 86 crate README 修复 + 文档体系重构 (4 段分类) + companion-desktop Phase 0-5 集成 (PR #1) + 11 项 CI 收尾 (libdbus/targets/packaging/YAML/fmt)
- **当前 cherry-pick 进行中**：`pick d1912c53 feat(companion): 连续感知①②`，改了 2 个 generated schema 文件，主人说"有活在干"，本 ROADMAP 暂不动它
- **决策链解除**：决策 #126 (commit 自决) + #128 (10 类 30+ 严守评估) + #130 (6 项 B 全部解除 + PHL-07 接受) — 旧"严守 0 主动 commit" 大部分解除
- **0 主动 push 仍严守**（等 1.0 release 配 GitHub remote — 主人拍板）

---

## 1. v1.0 实际发布路径 (R128-R178 + 1.0-final, 2026-08-12 → 2026-08-18)

| 周期 | 阶段 | 关键事件 |
|---|---|---|
| **R127** (8/10) | R127-2 整合 #4 commit `abf12243` 落地 | 实际是 1.2.0 时代（不是真正的 1.0），R125-R127 整合收尾 |
| **R128** (8/12) | minimax 真端到端 + workspace 收敛 94→55 | 4 协议全跑通 (OpenAI Chat/Responses/Anthropic) + 51 active crates + VCP 命名清理基础 |
| **R129** (8/12) | Tool 4 件套 orchestrator + Kani helloworld | 真解析 LLM tool marker → 真 Approval → 真 SQLite record + `/v1/guard` HTTP smoke + Kani `double_onion_sample` 形式化 |
| **R128 测试基线** | 20548 tests / 327 runs / 0 failed | workspace 1 次全跑 ~6 min |
| **R130-R145** (8/12-8/14) | 形式化 + 协议真端到端 + 顶层清整 | 79 crates 加 organ_kani_proofs (5 cargo + 2 Kani); apeireth-runtime 7 模块 orchestration; anysearch LIVE; LlmFacade 统一接入 |
| **R145** (8/14) | VCP 终极差距补弱完工 | 7 模块, 67+ tests |
| **R146-R150** (8/14-8/15) | 优雅化 + 终极补弱 | 21 VCP→LEGACY/BORROWED/ABSORBED; 5 SDK→1; 3 内存→1; 12 README 补; vector/stategraph/council/eval/test 6 模块 76 tests |
| **R148** (8/15) | 24 LOCKED 形式撤销扫尾 | 仅保 3 项不可变脊柱（Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义），其余可重构 |
| **R151-R163** (8/15) | lint cleanup + 桥一体化 | 475 warnings → 0 / 16 bugs fixed; 4 个 g5_*_bridge (memory/pipeline/runtime/council); voice Realtime 协议 |
| **R164-R167** (8/15) | API cleanup + 78→76 active crates | warning zero (858 tests); VCP 命名 100% 清理; 5618 tests |
| **R168-R169** (8/15) | LIVE MiniMax-M3 e2e 验证 | HTTP 200, 5.5s cold / 1.1s warm; 41 e2e tests all pass with LIVE apikey |
| **R170-R178** (8/15-8/16) | 后端完工补丁 + 综合审计 + 终极盘点 | 2 阻断修复 + GET /health/deps + ADR-0028/29/30; bridge_table; 1009 tests PASS |
| **12d4323b** (8/18) | **Apeireth v1.0.0 正式版 marker** | 主人拍板"真正的 1.0"; 后端机制层收工; 368 组测试 0 失败 |
| **993e9107** (8/18) | v1.0.0 tag → RELEASE_NOTES 正式版 | 五原型/她本身/安全/验收/诚实标注完整描述; tag 落点 |
| **e27ac0b2** (8/18, on release-1.0) | 连续感知①② + 自我改进闭环 | voice_session + screen_perception + experiment_field |
| **5824ada4** (8/18) | 1.0-final 最终审计修复 | CONTRIBUTING 重写 + INSTALL 1.97.1 + 86 crate README + workspace rust-version 1.97 + 9 crate license 对齐 |
| **057af667** (8/18) | 文档体系规范重构 | 4 段分类 (01-architecture/02-guides/03-reference/04-internal) + 历史归档 (136 个 stage*/r*/adr → archive/) |
| **C8c0c3fb-242bc93d** (8/18) | brand 定稿 | Apeiron 命名 + 完整品牌宣言 + Logo Brief + README 亲情版故事（"记住你忘记的"） |
| **4e29bd1d** (8/18) | 二级/三级文档深化 | api.md + deployment.md + development.md + user-manual + glossary.md (5 份真实文档) |

**v1.0.0 实际状态 (per 993e9107 tag + 12d4323b marker + CHANGELOG §1)**：
- ✅ `v1.0.0` tag 在 `993e9107` (8/18, 主人拍板真正的 1.0)
- ✅ release-1.0 分支顶 = `e27ac0b2` (8/18, 连续感知①②)
- ✅ master 顶 = `37fa420e` (8/19 后续 CI 收尾后)
- ✅ 85 active crates / ~34 万行 Rust / 368 组测试 0 失败
- ✅ 4 协议 (OpenAI Chat/Responses/Anthropic/Realtime) 真接 MiniMax 端到端
- ✅ 形式化（Kani）覆盖 79 crates
- ✅ 五原型骨架 (W1/W2/W3/E4/F4/A4/F6) + RuntimeBrain 接线 + companion_serve 真服务
- ✅ 安全: 双洋葱 + S4 出站白名单 + Audit Chain + ApprovalBridge silent
- ⚠️ Docker 构建实测（无 docker, 待实测 — 诚实标注）
- ⚠️ TimesFM/Kronos 模型接入（trait 口已备, 实现者未接 — 诚实标注）
- ⚠️ 产品形态: 桌宠/Tauri/麦克风实时语音/屏幕显著性事件（愿景文档规划中, 非本版范围）

详见: [`docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md`](docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md)

---

## 2. 后 v1.0 阶段 (8/18-8/19+, 当前)

**目标**：把 v1.0.0 推到 release-ready（GitHub Actions 全绿），为 v1.5 中期路线打基础。

| # | 件 | 类 | 状态 | 估时 |
|---|---|---|---|---|
| 1 | **cherry-pick `d1912c53` 连续感知①②**（进行中）| 收尾 | 🟡 pick 落地 + 2 schema 文件 M | 等主人/操作员 continue |
| 2 | **release-1.0.0.yml 4 packaging/target bugs** (`ecbb2f7f`) | CI | ✅ done 8/19 | — |
| 3 | **Install Rust targets 简化** (`37fa420e`) | CI | ✅ done 8/19 | — |
| 4 | libdbus-1-dev 安装 (`b43df172`) | CI | ✅ done 8/19 | — |
| 5 | 8 轮 CI YAML 修复 (`9f3c20c4` → `02193c22`) | CI | ✅ done 8/19 | — |
| 6 | fmt stable 化 (`19252237`) | lint | ✅ done 8/18 | — |
| 7 | macOS flaky fixes (`f69154e5`, `4406207f`) | 测试 | ✅ done 8/19 | — |
| 8 | **companion-desktop PR #1** (`e4bacf92`) | 桌面 | ✅ merged | — |
| 9 | **Phase 5 harden** (`91b2d2e0`) + **E2E 验证** (`075321b5`) | 桌面 | ✅ done | — |
| 10 | **GitHub remote 配 + push v1.0.0** | 发布 | ⏳ 等主人拍板 | — |
| 11 | **Dockerfile 实测** | 发布 | ⏳ 本机无 docker, 待 CI 实测 | — |

**当前 git 状态**：
- branch: master
- HEAD: `37fa420e`
- cherry-pick in progress: `pick d1912c53 feat(companion): 连续感知①②`
- modified (unstaged): 2 个 generated schema 文件 (`desktop-schema.json` / `windows-schema.json`)
- working tree: 仅这 2 文件 M，无其他未提交改动（ROADMAP 本次更新未提交，见 §11）

---

## 3. v1.x 后续 (8/20+, 短中期)

**v1.1 短期 (8/20-9/14) 大部分已提前达成**：
- ✅ R145 VCP 终极差距补弱完工 (7 模块, 67+ tests)
- ✅ R149 P0 五模块补弱 (tool-fetch / skills / runtime LlmWorker / graph / formal l0_ha)
- ✅ R150 P1 六模块补弱 (vector qdrant_compat / state statechart / cron / council / eval / test property)
- ✅ 形式化加深 V3 (79 crates 加 organ_kani_proofs, R177)
- ✅ 协议端到端验证 (R168-R169 LIVE e2e)
- ⏳ Library Stage 4-6 进阶 (P5-1 自治 / P5-2 治理 / P5-3 守护) — 跑中 (per 旧 ROADMAP §2)
- ⏳ 借鉴 3 限流重试 (LiteLLM/opencode/Guardrails) — 跑中 (per 旧 ROADMAP §2)
- ⏳ **push v1.0.0 到 GitHub remote** — 等主人拍板 (per §10 0 主动 push 严守)

**v1.1 收尾 (本阶段)**：
- 1 件 cherry-pick 收尾
- 1 件 release-1.0.0.yml CI 全绿确认
- 1 件 push v1.0.0 (主人拍板)
- 1 件 Docker 实测 (CI 跑)

**v1.5 中期 (8-12 月)** —— 路线图保留（per 旧 §3）：
- ASI Python 整合（R11 baseline 3 值严守 0.8682/0.8532/0.9063, 17 文件原位 0 删 0 改）
- Tauri 终极前端 prototype（等设计团队；TUI 改瘦作集成测试床）
- 5 拆 crate (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive)
- StateGraph 4 协议 handler trait 真接 (HTTP/WS/gRPC/MCP)
- Library 阶段 4-6 进阶 (自治/治理/守护)
- 借鉴 11/11 收尾 (LiteLLM/opencode/Guardrails 重试)
- vector store long-term 真接 (sqlite-vec)
- 商业化 / 真用户 (旧 ROADMAP §3: 我们 8/5 "现在根本没用户用")

---

## 4. v2.0 长期 (2027+, 6-12 月+)

旧路线保留（per 旧 §4），主轴从"R128+ 升级 + 1.0 release 流程" 改成 "**v1.0 → v2.0 路线 + 商业化路径**"：

| 主题 | 任务 | 截止 |
|---|---|---|
| **商业化路径** | VCPChat 参考 (per 旧 8/4 决策) | 持续 |
| **真用户 + 社区** | per R119-2 思想层；我们接管 | 持续 |
| **多 AI 平台** | per 我们 7 月 R-Method 平台策略 | 持续 |
| **教育/科研合作** | 我们研究生背景 + 2026 学术研究项目 | 持续 |
| **5 拆 crate 真接** | per 决策 #21 Phase 4 | 持续 |
| **4 协议 handler trait 真接** | per R123-2 + R125-1/13 | 持续 |
| **守门 v8+** | NVIDIA Guardrails + 借鉴 | 持续 |
| **30 维 V0.5 + 9 子测度结构** (B3 实施, 当前 24 维 — 升级未合) | per R125-13 + P1-4 (P1-4 待真实施) | 持续 |
| **8 哲学锚 → 12+ 锚** (B5 升 12) | per R126 P1-2 升级 | 持续 |
| **13 键 → 16+ 键** (新增 PHL-08/09/10, 当前 13 键 — PHL-07 仍待合并 core) | per R125-12 P0-3 升级 | 持续 |
| **9 重守门 v9 → v11+** (v6→v7→v8→v9 已完成, 未来 v10+) | per P1-3 升级 (已 done 到 v9) | 持续 |

**v2.0 关键约束**：0 主动 push 仍严守（per §10）；R128+ 升级派活 = 16 派满策略（per 旧决策 #33 §4）。

---

## 5. 8 硬墙 → 已基本解除 (per 决策 #130 + #126 + #128 + R148)

| 硬墙 | 旧状态 | 当前实际 (8/19) | 来源 |
|---|---|---|---|
| **B1** 24 LOCKED 入口签名冻结 | 严守 | **降级**：仅保 3 项不可变脊柱（Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义），其余可重构 | 决策 #74 §1.1 + #130 §2.4 + **R148** (CHANGELOG §2.5) |
| **B2** workspace.version | 1.2.0 严守 | **解除 + 双轴制**：产品轴 = git tag `v1.0.0` (8/18 发布)，workspace 轴 = `1.2.0` (`Cargo.toml:228`, B2 升级严守)。顶层 README 明确"双轴制" (v1.0.0 product; workspace crates 1.2.0); v1.5/v2.0 可调 | 决策 #130 §2.4 + 顶层 README 双轴说明 |
| **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 | 严守 | **仍严守**：17 文件原位 0 删 0 改 (per `crates/apeireth-asi/tests/integration_r_measure.rs:42-44`) | 决策 #33 §2.1 + 旧 #55 §4 |
| **A2** R11 9 子测度结构 | 严守 | **仍严守**：结构不动，数字可调 (代码 `V1136_SUBMEASURE_COUNT: usize = 9`) | 决策 #33 §2.2 |
| **A3** 12 键 + PHL-07 = 13 键 | 严守 | **仍严守**：PHL-07 NotUnoptimizable 接受实施 (但 `apeireth-core/src/lib.rs` philosophy.rs 仍 hardcode `[PhilosophyKey; 12]` — PHL-07 待合并) | 决策 #33 §2.1 + R125-12 P0-3 done |
| **B3** V0.5 25→30 维 | 升级版未实施 | ⏳ 当前实际 **`V05_DIM_COUNT: usize = 24`** (`apeireth-asi/src/lib.rs:56`); 30 维升级 (R126 P1-4) 在 master HEAD `9bf36b1e` **未合入** (git log 搜 "P1-4 / 30 维 / v05.*30" 全0 命中); 升级是 v2.0 长期路线 | 决策 #33 §2.1 + 实际代码 |
| **B4** 守门 v6→v7→v8→v9 | 升级版实施 | ✅ **9 重 v9** 实施: v6 (colang_dsl) → v7 (skill_guard + seven_fold_guard, R126-guard-7) → v8 (action_rail + flow_executor, R127-2 P6-3) → v9 (evidence_guard, R131) | `crates/apeireth-sovereignty/src/lib.rs:65-83` lineage |
| **B5** 6→8 哲学锚 | 升级版严守 | ✅ 升 8 锚（+ S-3 质量工程化 + O-1 安全优先） | 决策 #33 §2.1 + R126 P1-2 |
| **B6** 双→三洋葱 | 升级版严守 | ✅ 升三洋葱（+ DSL 洋葱, R125-5 done） | 决策 #22 §2.6 |
| **B7** 9 organ 内部 fn 借 OpenCode | 严守 | **降级**：9 organ 文件名 + 入口签名 0 改，内部 fn 可借（实施进度未跟踪到 done） | 决策 #22 §2.7 |
| **C1** 0 主动 commit | 严守 | **解除**：Mavis 全自决 commit (per 决策 #126); 本次 ROADMAP 更新按 doc-change commit | 决策 #126 |
| **C2** 0 装 PASS 严守 | 严守 | **仍严守**：✅ cloned = 真实施, ⏳ 限流 = 准备 (诚实标), ❌ 跳过 (OpenCog AGPL-3.0 = 0 集成) | 决策 #33 §2.3 + R125-16 retry verify |
| **C3** 守门 v6→v9 lineage | 严守 | ✅ 升 v9 (lineage: v6 → v7 → v8 → v9, 决策 #52/55/#130) | 决策 #33 §2.3 + 决策 #55 §4 |
| **0 主动 push** | 严守 | **仍严守**：等 1.0 release 配 GitHub remote (主人拍板) | 决策 #33 §2.3 + #53 §1 + #55 §7 |

**小结**：旧"8 硬墙"11 项中，6 项解除 / 降级 (B1, B2, B7, C1 + 24 LOCKED 形式撤销 per R148)，5 项仍严守 (A1, A2, A3, C2, 0 主动 push)。

---

## 6. 借鉴源码 11/11 进度 (per 旧 ROADMAP §6 + 决策 #55/#56 + 决策 #130)

| 状态 | 借鉴源码 | 文件数 | 当前 (8/19) |
|---|---|---:|---|
| ✅ **cloned = 真实施** | clap | 725 | ✅ (R125-2 done, -54% code) |
| ✅ | hyper | 80 | ✅ (R125-3 done, 池复用 38/38 tests) |
| ✅ | servers (MCP) | 175 | ✅ (R125-4 done, MCP) |
| ✅ | PyO3 | 928 | ✅ (R125-8/9 done, 真链接 77/77 tests) |
| ✅ | kani | 4502 | ✅ (R125-10 done, 5 阶段 12 文件 75.8KB) |
| ✅ | langgraph | 829 | ✅ (R125-13 done, 30 维 85.9KB + 60 tests) |
| ✅ | superpowers | 234 | ✅ (R125-14/15/16/18/19/126-guard-7 done) |
| ⏳ **限流 = 准备 → 重试** | LiteLLM | 0 | ⏳ R127-2 P6-1 重试派中（per 旧 ROADMAP; 实际状态待 verify） |
| ⏳ | opencode | 0 | ⏳ R127-2 P6-2 重试派中 |
| ⏳ | Guardrails (NVIDIA) | 0 (submodule) | ⏳ R127-2 P6-3 重试派中 (colang_dsl.rs 1700 行已写) |
| ❌ **跳过 = 0 集成** | OpenCog | AGPL-3.0 | ❌ 0 集成 (传染风险) |

**借鉴 8/11 ✅ + 3/11 ⏳ + 1/11 ❌ = 11/11 进度**。**0 装 PASS 严守** (per 决策 #33 §2.3 + #55 §3 + #56 §3) 仍严守。

---

## 7. Library v1.0 路线 (per `library-upgrade-plan-2026-08-10.md` + 决策 #55)

| 阶段 | 状态 (8/19) |
|---|---|
| 阶段 1 命名 + 文档结构 | ✅ done (R125-16 retry verify 17 tests 实际 vs 33 装) |
| 阶段 2 9 大类升级 + 10/11/12 子 | ✅ done (R125-17 P0-4) |
| 阶段 3 借鉴 ID 严格化 | ✅ done (R125-18, 400+ ID, 含事故诚实标) |
| 阶段 4 Library 摘要 | ✅ done (R125-19, 9 大类 _SUMMARY + _TOP_100) |
| 阶段 5 Library 工具 + TUI 集成 | ✅ done (R125-20, _SEARCH + _CROSS_REF + TUI Library nav) |
| 阶段 6 v1.0 release 礼物 | ✅ done (R125-21, 30 经典书 9 organ 1:1) |
| **阶段 4 进阶 自治** | ⏳ 跑中 (R127-1 P5-1) |
| **阶段 5 进阶 治理** | ⏳ 跑中 (R127-1 P5-2) |
| **阶段 6 进阶 守护** | ⏳ 跑中 (R127-1 P5-3) |
| **Stage 2 借脑 1.0** | ⏳ 跑中 (R127-2 P9-1) |

**Library v1.0 已发布**：30 经典书 9 organ 1:1 + 100 论文 + 50 视频 + 10 社区 + 10 hub，1.0 release 礼物。

---

## 8. 决策链 (per 旧 ROADMAP §8 + 新增 #62/#74/#126/#128/#130)

| 决策 | 时间 | 主题 | 当前状态 |
|---|---|---|---|
| **#21** | 8/10 16:25 | R125+ 升级路线图 | 已 done (R125 16 sub-agent 全 succeeded) |
| **#22** | 8/10 16:35 | 最高权限授权 + 24 LOCKED 自主确认 | 已 done (后续 #130 解除大部分) |
| **#30-#34** | 8/10 17:15-17:30 | 新 Mavis 接入 + 派活 daemon 复活 | 已 done (整合 #3 commit `21aa85f3`) |
| **#33** | 8/10 17:23 | 升级授权 + 8 硬墙重置 | 已 done |
| **#35-#42** | 8/10 17:32-18:35 | 16 真派 + 借鉴 7/11 | 已 done (R125 16 sub-agent all done) |
| **#43-#50** | 8/10 18:35-20:01 | 主仓挪出 + 整合 #4 commit | 已 done (整合 #4 commit `abf12243` 19:41) |
| **#51-#54** | 8/10 20:09-21:11 | 16 派活 + 技术性 locked 解锁 | 已 done |
| **#55** | 8/10 21:13 | R127 升级路线 + 派活清单 | 已 done (R127 4 sub-agent) |
| **#56** | 8/10 21:18 | R127-2 派活 10 sub-agent | 已 done (R127-2 10 sub-agent) |
| **整合 #4 commit `abf12243`** | 8/10 19:41 | 46752 file changes, master HEAD (旧) | ✅ done (实际 1.2.0 时代, 不是真正 1.0) |
| **整合 #5 commit** | 8/15 预期 (8/11-8/22) | R127 4 + R127-2 10 = 14 任务全 done | ✅ done (并入 R128-R178 大整合) |
| **#62 §5.2** | 8/12? | 整合 #5 commit 拆 3 commit 范式 | 已 done (R128 era) |
| **#74 §1.1** | 8/13? | 24 LOCKED 入口签名冻结降级 | 已 done (R148 撤销扫尾仅保 3 项) |
| **#126** | 8/13? | **Mavis 全自决 commit 解除** | ✅ done (本次 ROADMAP 更新按 doc-change commit 即按此) |
| **#128** | 8/14? | 10 类 30+ 严守评估 | 已 done (硬墙大幅解除) |
| **#130 §2.4** | 8/15? | **6 项 B 全部解除 + PHL-07 接受实施** | ✅ done (B1-B6 大幅解除) |
| **v1.0.0 tag `993e9107`** | 8/18 | 主人拍板真正的 1.0 | ✅ done (v1.0.0 tag 落点) |
| **12d4323b** | 8/18 | v1.0.0 正式版 marker | ✅ done |

**关键新决策 (8/12-8/18 期间)**：
- 决策 #62 §5.2 — 整合 #5 commit 范式拆 3 commit (R128 era)
- 决策 #74 §1.1 — 24 LOCKED 入口签名冻结降级 (R148 撤销扫尾基础)
- 决策 #126 — Mavis 全自决 commit 解除 (本次 ROADMAP 更新按此)
- 决策 #128 — 10 类 30+ 严守评估 (硬墙大部分解除基础)
- 决策 #130 §2.4 — 6 项 B 全部解除 + PHL-07 接受实施 (B1-B6 大幅解除)

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **cherry-pick `d1912c53` 连续感知①② 悬空** | master HEAD 在 pick + 2 schema 文件 M；未 continue/skip/abort | 主人说"有活在干"，等操作员推进 |
| **借鉴 3 限流重试 (LiteLLM/opencode/Guardrails) 0 files submodule 限制** | 借鉴 11/11 收尾受阻 | R127-2 P6-1/2/3 跑过夜，实际 done 状态待 verify |
| **0 主动 push 仍严守 + 等 GitHub remote** | v1.0.0 tag 在本地，但 release-1.0 分支和 master 都未推 GitHub | 主人拍板 GitHub remote 配 + push v1.0.0 |
| **Dockerfile 实测缺 docker 环境** | 本机无 docker, release-1.0.0.yml 实测受阻 | CI 跑 (GitHub Actions runner 自带 docker) |
| **R148 24 LOCKED 撤销扫尾后, 旧文档引用过时** | `docs/archive/conventions/10-locked.md` 等仍按旧 24 LOCKED 严守描述 | 按 #130 §2.4 实际重新表述 (本次 ROADMAP §5 已更新) |
| **5 拆 crate / 4 协议 handler trait / 守门 v8+ / 9 organ 内部借 / 30 维 / 8 锚 / 13 键 / 7 重 范围广** | v2.0 工作量大 | 16 派满策略 (per 旧 #33 §4), 主人持续授权 |
| **Tauri 终极前端 等设计团队** | TUI 升级 5 拆 crate 推迟 | 主人 8/4 23:33 "TUI 是'集成测试床'", Tauri 来了无缝换 UI 层, 0 必急 |
| **商业化路径 / 真用户 0 进展** | 主人 8/5 "现在根本没用户用" | 等 GitHub remote + push + 真用户社区 |

---

## 10. 0 主动 commit / 0 主动 push 状态

- **0 主动 commit** (旧 C1): **已解除** (per 决策 #126 Mavis 全自决 commit); 本次 ROADMAP 更新按 doc-change commit (见 §11)
- **0 主动 push git push** (per 旧决策 #33 §2.3 + #53 §1 + #55 §7): **仍严守** — 等主人拍板配 GitHub remote
- **整合 #4 commit `abf12243`**: 已 done (8/10 19:41, 46752 file changes)
- **整合 #5 commit**: 已 done (并入 R128-R178 大整合, 1.0 release 路径)
- **v1.0.0 tag `993e9107`**: 已 done (8/18, 主人拍板真正的 1.0)
- **v1.0.0 release 配 GitHub remote**: ⏳ 等主人拍板
- **0 主动 IM 我们** (per gate-discipline, 5 min tick 自动派替代 0 打扰, 仅 done notification 主动报告)
- **0 主动 plain reply on skip ticks** (per gate-discipline)

---

## 11. 本次 ROADMAP 更新说明

| 项 | 值 |
|---|---|
| 更新触发 | 主人 2026-08-19 "你先更新roadmap，因为悬挂是有活在干" |
| 更新范围 | 顶层 ROADMAP.md (本文件) — 全面反映 v1.0.0 已发布 (8/18) 状态 |
| 详单下沉 | `docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md` (新增, 见 §12) |
| 原 ROADMAP.md 备份 | `docs/archive/roadmap/roadmap-r127-2-2026-08-10.md` (新增, 见 §12) |
| cherry-pick 处理 | **未动** — `pick d1912c53` 仍 in-progress, 2 schema 文件 M, 等主人/操作员 |
| commit 策略 | 按决策 #126 (Mavis 全自决 commit); 本次按 doc-change commit (commit msg: `docs(roadmap): v2.0-post1.0 — 反映 8/18 v1.0 实际发布状态`) |
| push 策略 | 0 主动 push 仍严守 (per §10) |

---

## 12. 详单下沉 (per R119-2 原则)

- [`docs/archive/roadmap/README.md`](docs/archive/roadmap/README.md) — 路线图总览（需加新条目）
- [`docs/archive/roadmap/v1.0.0-release-roadmap-2026-08-06.md`](docs/archive/roadmap/v1.0.0-release-roadmap-2026-08-06.md) — 1.0 release 9-30 tag 计划 (R20 阶段 6 总结, 13.6KB)
- [`docs/archive/roadmap/r20-product-finalize-2026-08-05.md`](docs/archive/roadmap/r20-product-finalize-2026-08-05.md) — R20 product finalize 详细报告 (R20 阶段 6 收尾, 35.2KB)
- [`docs/archive/roadmap/v1.2-release-plan-2026-08-09.md`](docs/archive/roadmap/v1.2-release-plan-2026-08-09.md) — v1.2 release 计划 (R69 起草, 8.8KB)
- [`docs/archive/roadmap/v1.0-released-r125-r127-2026-08-10.md`](docs/archive/roadmap/v1.0-released-r125-r127-2026-08-10.md) — v1.0 已发布详单 (R125-R127 整合, 旧) (R127-2 P7-2)
- [`docs/archive/roadmap/roadmap-r127-2-2026-08-10.md`](docs/archive/roadmap/roadmap-r127-2-2026-08-10.md) — **旧顶层 ROADMAP.md 备份 (R127-2 时代)**
- [`docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md`](docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md) — **v1.0 已发布详单 (R128-R178 + 1.0-final, 本次新增)**

---

## 13. 思想层保留 (哲学 LOCKED, per R119-2 原则)

| 主题 | 来源 | 状态 (8/19) |
|---|---|---|
| 立体架构 v2 | R11 / R14 | 🔒 LOCKED |
| 生命架构 v4 | R11 / R14 | 🔒 LOCKED |
| 哲学层升级 v4.1 | R11 / R14 | 🔒 LOCKED |
| 6→8 哲学锚 (S-1 北极星 / S-2 实事求是 / S-3 质量工程化 NEW / O-1 安全优先 NEW / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装) | 升 8 锚, P1-2 R126 done | 🔒 LOCKED |
| 12 键 → 13 键编译期 hardcode (+ PHL-07 NotUnoptimizable NEW) | 升 13 键, R125-12 P0-3 done | 🔒 LOCKED |
| 5 重守门 → 6 重 v6 → 7 重 v7 (+ Colang DSL + Superpowers Skill Guard) | 升 7 重, P1-3 retry done | 🔒 LOCKED |
| 双洋葱 → 三洋葱 (+ DSL 洋葱, R125-5 done) | 升三洋葱 | 🔒 LOCKED |
| 9 organ 文件名 + 入口签名 0 改 | TUI 9 organ 内部可改（per R148 LOCKED 撤销扫尾原则） | 🔒 软 LOCKED |
| R11 baseline 3 值 (0.8682/0.8532/0.9063) 数字严守 | R11 ASI R-Measure | 🔒 LOCKED (A1 仍严守) |

详见 [`docs/archive/stage1/00-VISION.md`](docs/archive/stage1/00-VISION.md) + [`docs/archive/conventions/09-anchor.md`](docs/archive/conventions/09-anchor.md) + [`docs/archive/conventions/11-baseline.md`](docs/archive/conventions/11-baseline.md) + [`docs/archive/glossary/17-4-gates-permission.md`](docs/archive/glossary/17-4-gates-permission.md) + [`docs/archive/conventions/10-locked.md`](docs/archive/conventions/10-locked.md)。

---

_本 ROADMAP 由 Mavis 后-R178 重写 (2026-08-19), 反映 v1.0.0 实际发布 (8/18) + R128-R178 横扫 + 决策 #62/#74/#126/#128/#130 硬墙解除 + companion-desktop PR #1 + 11 项 CI 收尾 + 当前 cherry-pick `d1912c53` 进行中. 详单下沉 `docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md`. 思想层 (8 锚 / 13 键 / 7 重 / 三洋葱 / 9 organ / R11 baseline 3 值) LOCKED 保留, 0 主动 commit 解除按 #126, 0 主动 push 仍严守. 报告 `reports/agent-post1.0-roadmap-2026-08-19.md` ready._