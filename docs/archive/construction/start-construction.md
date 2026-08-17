# Apeireth R14 施工团队开工手册（顶层入口）

```
[Document-Meta]
Document: START-CONSTRUCTION.md
Version: Manual-Rev-G + Design-5.0-R14 + Fix-7 (HA 部署模式) + Fix-8 (偏差修正) + Fix-9 (漂移检查) + Fix-10 (版本号系统)
R-Cycle: R14
Last-Modified: 2026-07-31
Status: 🟢 活跃
```

> ## ⚠️ 设计漂移警告（主人 2026-07-31 关键洞察，**最前面**）
>
> 我们是从**阶段 1→2→3→4→5 逐步向后设计**的，所以可能会有**逐渐增大的漂移和误差**。
>
> **如果施工团队发现开工手册与阶段 1+2+3+4 LOCKED 设计有任何不一致**（HA 实现、双洋葱比喻 vs 统一体、守门数量、12 键位置、4 关系、三域分离、主体连续性、SGI 单字段等），**请团队 leader 立即与主人沟通澄清**，**不要"假装一致"**。
>
> **沟通流程**：
> 1. team_leader 立即评估漂移严重性（🔴 阻塞 / 🟡 一致性 / 🟢 文档细节）
> 2. 提交漂移报告：`reports/drift-<阶段>-<§>-<日期>.md`
> 3. 用 `docs/00-R14-START-HERE.md` §入口路由找到主人
> 4. 主人拍板后立即执行
>
> **漂移检查清单**：详见 §漂移检查清单（每 5 个成就 A5/A10/A15/A20 强制回顾时执行）
>
> **详细 v9 漂移检查**：`docs/stage4/stage4-correction-v9-drift-check.md`

> **v5 修订**（2026-07-31）：主人亲自精读阶段 1+2+3+4 后检查开工手册偏差
> - ✅ 找到 4 个真偏差：4 关系（共生/协调/嵌入/与自身）+ 三域分离 + 主体连续性 + SGI 单字段
> - ✅ 全部补齐到开工手册
> - 详细 v8 偏差修正：`docs/stage4/stage4-correction-v8-deviation-check.md`
> **v4 修订**（2026-07-31）：主人亲自检查后关键洞察——HA 部署模式自适应（保底 1 人类）
> - ✅ 精读阶段 1+2 LOCKED：§18.6 + §19.3 + §8.5 + §9.3 + §11.1 + §11.3 明确说"保底 1 人类"
> - ✅ 修正开工手册错误：物理多签不是强制多人多签，而是按部署模式自适应
> - ✅ single 模式（1 人使用 Apeireth）：1 个主人 + Windows Hello / FIDO2 / 主人密钥
> - ✅ multi 模式（多人部署）：MultiHuman 多签 + 物理多签（M-of-N）
> - 详细 v7 修正：`docs/stage4/stage4-correction-v7-deployment-mode-adaptive.md`
> **v3 修订**（2026-07-31）：外部 agent Round 5 反馈 5 个真问题全部修复
> - ✅ 修 #1: apeireth-legacy 描述诚实（"阶段 7+ 真正施工时再归档"）
> - ✅ 修 #2: v6 完整版路径验证（`docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` 16,171 bytes）
> - ✅ 修 #3: 明确"v6 设计意图 vs 实装"边界
> - ✅ 修 #4: 加 v3/v4/v5/v6 路径清单
> - ✅ 修 #5: 主手册 APEIRETH-COMPLETE-OMNIBUS 路径 + 必读说明
> **v2 修订**（2026-07-31）：AI 团队成就驱动报告（替代日报/周报）
> **v1**（2026-07-31）：初版
> **位置**：从 `docs/stage5/construction-kickoff-manual.md` 移到**顶层**（主人 2026-07-31 洞察"开工手册应该在顶层"）

---

## 📜 给施工团队的总览信

**欢迎来到 Apeireth R14 施工期。**

你们是**施工团队**——负责把 R14 的设计图（54 份 LOCKED 设计文档 + v6 完整版）变成**真正可运行的 Rust 代码**。

你们**不承担**前端（阶段 7）。前端是**另一个团队**的责任——他们在阶段 7 实施时根据 `docs/stage4/architecture-frontend-design-proposal.md` 工作。

你们的工作**目标**：**按成就驱动（A1-A20）交付**，让主人能看到 Apeireth 真的跑起来。

> **主人 2026-07-31 关键洞察**：AI 团队干起来非常快，不是人类速度，**工期不可能 1-2 week**。**报告 = 成就达成后记录 + 检查 + 回顾文档确认没跑偏**（不是日报/周报）。

---

## 🎯 施工团队责任（明确）

### ✅ 负责

| 维度 | 内容 |
|---|---|
| **后端 Rust 代码** | 17 crate 真正实现（apeireth-core 已落地 17516 bytes + 6 tests pass）|
| **集成测试** | 端到端链路（CLI → Session → Episode → V1+V2+V3 → verdict）|
| **CI/CD** | GitHub Actions（rust-ci.yml 已就绪 + 扩展 coverage/nightly/benchmark）|
| **集成 R11 1100 模块** | PyO3 桥接（apeireth-pybridge）|
| **OTA 工程化** | apeireth-upgrade crate（7 阶段原子切换）|
| **Self-Disable 防护** | 5 大机制真正代码 |
| **Cognitive-Dream 6 状态机** | apeireth-consciousness trait 完整实现 |

### ❌ 不负责（明确）

| 维度 | 由谁负责 |
|---|---|
| **前端 TUI / Web UI / Desktop / Mobile** | **另一个团队**（阶段 7 延后）|
| **设计文档** | 主人 + leader（已 LOCKED）|
| **测试数据生成** | apeireth-test 内部 owner |
| **CI/CD 配置审查** | devops 团队 |

### 🛡️ 绝不修改（主人硬约束 100% 守住）

| ❌ 不修改 | 原因 |
|---|---|
| **阶段 1+2+3 LOCKED** | 主人明确沉淀，54 份设计文档不重写 |
| **v2 / v4 / v4.1 LOCKED** | 主人明确沉淀 |
| **阶段 4 主文档 LOCKED**（6ca80776）| 不修改 |
| **阶段 5 施工文档 LOCKED**（631 行）| 不修改 |
| **v6 修正（独立命名空间）**| 修正链保留 v1-v6，**不删任何历史版本** |
| **R11 baseline 三值**（V1141=0.8682 / V1131=0.8532 / V1136=0.9063）| 不修改 |

### ✅ 可修改（v6 明确）

- V0.5/V1136/9 键 v2（v4.1 §13/§14/§15 提议落地）
- R11 1100 重写（保留 ~30% + 合并 ~60% + 砍 ~10%）
- crates/ 占位（按阶段 4 §2 + v6 完整版重写）
- Cargo.toml metadata
- Self-Disable 防护代码

---

## 🚀 成就驱动最小可行 demo（A1-A20）

> **主人 2026-07-31 关键洞察**：AI 团队干起来非常快，不能用 1-2 week 当工期。**成功标准 = 成就达成**。

### A1-A8（最小可行 demo = 8 个成就）

| 成就 | 描述 | DoD（完成定义）| 验证命令 | 防跑偏 |
|---|---|---|---|---|
| **A1** | apeireth-cli session 启动 | `cargo run --bin apeireth-cli session` 输出欢迎信息 | `cargo run` | v6 §6 |
| **A2** | 集成测试通过（已就绪）| `cargo test --test integration_session_lifecycle -p apeireth-core` 2 测试通过 | `cargo test` | §1 |
| **A3** | 12 键编译时 hardcode | `cargo check` 0 error + 5+ 违反测试失败 | `cargo check --workspace` | v6 守门 1 |
| **A4** | apeireth-memory SQLite 存储 | 10+ tests pass + Episode 写入查询 | `cargo test -p apeireth-memory` | §6 不修改承诺 |
| **A5** | V1+V2+V3 AND 门完整 impl | 10+ tests pass + 正常+危险决策 | `cargo test --test integration_*` | v6 §1.3 |
| **A6** | CI coverage/nightly/benchmark | GitHub Actions 3 个新工作流 | README badge 更新 | §1 CI/CD 责任 |
| **A7** | Self-Disable 5 大机制最小 | 5+ 单元测试 + 集成 | `cargo test --test self_disable` | §7 Self-Disable 必读 |
| **A8** | R-Measure V0.5/V1136/V3 翻译 | 跑 R11 baseline 三值 | `cargo test --test r_measure` | R11 baseline LOCKED |

**A8 完成 = 最小可行 demo 达成**（按主人节奏，不固定时间）

### A9-A17（9 个器官 crate 真正落地）

| 成就 | 描述 | DoD |
|---|---|---|
| **A9** | apeireth-perception 真正落地 | 编译通过 + 5+ tests + examples |
| **A10** | apeireth-cognition 真正落地 | 同上 |
| **A11** | apeireth-action + motivation + value 3 个一起 | 同上（3 个 crate 配套）|
| **A12** | apeireth-consciousness + relation 2 个一起 | 同上 |
| **A13** | apeireth-life-force（生命力）真正落地 | 同上 |
| **A14** | apeireth-council（智囊团）真正落地 | 同上 |
| **A15** | apeireth-upgrade（OTA + 沙盒）真正落地 | 同上 |
| **A16** | apeireth-bus + extension + pybridge 3 个一起 | 同上 |
| **A17** | apeireth-philosophy 物理删除（已 DEPRECATED）| `git rm` + 工作空间清理 |

### A18-A20（收尾 + Self-Disable + 集成）

| 成就 | 描述 | DoD |
|---|---|---|
| **A18** | OTA 7 阶段工程化 | OTA 状态机完整 + 5+ tests |
| **A19** | Cognitive-Dream 6 状态机完整 | trait + 6 状态转换 |
| **A20** | 17 crate 全部集成 | `cargo build --workspace` 0 error |

### 防跑偏节奏

- 每 5 个成就 = 1 次强制回顾（A5/A10/A15/A20）
- 每次回顾 = 对照 v6 + 检查不修改承诺 + 更新文档
- 报告路径：`reports/retrospective-A<n>-<角色>.md`

---

## 📚 必读文档（按优先级）

### 🔴 第 1 优先级（开干前必读）

| # | 文档 | 路径 | 内容 |
|---|---|---|---|
| 1 | **README** | `README.md` | 顶层入口（v6 修订链 §文档修订链）|
| 2 | **CONTRIBUTING.md** | `CONTRIBUTING.md` | PR 流程 / commit 规范 / 测试要求 / 设计约束 |
| 3 | **INSTALL.md** | `INSTALL.md` | 三平台安装（Win/Linux/macOS）|
| 4 | **CHANGELOG.md** | `CHANGELOG.md` | 当前版本 + 改了什么 |
| 5 | **GLOSSARY.md** | `GLOSSARY.md` | 17+ 项术语（v6 修正）|
| 6 | **ROADMAP.md** | `ROADMAP.md` | 成就 A1-A20 最小 demo + 后端完整 |

### 🟡 第 2 优先级（施工时必查）

| # | 文档 | 路径 | 内容 |
|---|---|---|---|
| 7 | **v6 完整版** | `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` | **4 重守门嵌套 + 权限发放 + E 层修改路径**（最新设计）|
| 8 | **施工文档** | `docs/stage5/stage5-construction-document.md` | 8 大施工模块（LOCKED）|
| 9 | **v4.1 哲学层升级** | `docs/architecture-v4-1-living-intelligence-update.md` | V0.5/V1136/V3 v2 提议（v4.1 §13/§14/§15）|
| 10 | **Self-Disable 防护** | `docs/stage4/stage4-external-feedback-and-revisions.md` §3 | 5 大机制（百年章节）|
| 11 | **阶段 4 主文档** | `docs/stage4/stage4-runtime-architecture-revised.md` | 1492 行 v2 修订 + 经典视图 |
| 12 | **ADR 0001** | `docs/adr/0001-double-onion-unity.md` | 双洋葱统一体设计决策 |
| 13 | **外部反馈回应** | `docs/stage4/stage4-external-feedback-and-revisions.md` | 7 担忧 + 5 改进 + 1 最担心 |

### 🟢 第 3 优先级（按需查阅）

| # | 文档 | 路径 |
|---|---|---|
| 14 | 阶段 1 灵感 | `docs/stage1/inspiration-stage1-2026-07-30.md`（2201 行 LOCKED）|
| 15 | 阶段 2 想法设计 | `docs/stage2/`（19 文件 LOCKED）|
| 16 | 阶段 3 图纸 | `docs/stage3-blueprints/`（14 文件 LOCKED）|
| 17 | **apeireth-legacy 归档索引** | `apeireth-legacy/README.md`（**R11 1305 文件归档索引 — 阶段 7+ 真正施工时再归档，当前只有 README**）|

### 🛡️ v3-v6 修正链路径（必查，别再瞎找）

| 版本 | 路径 | 字节 | 关键修正 |
|---|---|---|---|
| **v3** | `docs/stage4/stage4-correction-v3-onion-embedded-keys-gates.md` | 19,267 | 12 键 = O 层内容 + 5 重守门 = 最外层包裹（错）|
| **v4** | `docs/stage4/stage4-correction-v4-onion-dedupe.md` | 15,922 | 5 重守门融入每层（错）|
| **v5** | `docs/stage4/stage4-correction-v5-gates-refined.md` | 15,410 | 4 重守门嵌套 + 权限发放独立（v5 提议）|
| **v6** | `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` | 16,171 | **5 重治理 + 4 重守门 + 权限发放 + E 层修改路径（v6 当前正确）**|

### 📖 主手册 APEIRETH-COMPLETE-OMNIBUS（**供主人阅读，施工团队可跳过**）

| 文档 | 路径 | 大小 | 谁读 |
|---|---|---|---|
| **主手册** | `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` | 6546 行 / 420KB / 424902 bytes | **主人阅读**（设计层完整 LOCKED）；**施工团队可跳过**（设计层已涵盖在阶段 1-5 + v6 完整版里）|

---

## ⚠️ v6 设计意图 vs 实装边界（必读！）

> **外部 agent Round 5 #3 真问题**：文档说 v6 = "5 重治理 + 4 重守门 + 权限发放"，但 apeireth-core 17516 bytes 只实装了 5 重守门 enum。

### v6 设计核心 vs 当前实装（诚实对照）

| v6 设计概念 | 状态 | 位置 |
|---|---|---|
| **5 重守门（Gate enum）** | ✅ **已实装**（部分）| `apeireth-core/src/lib.rs`（17516 bytes）|
| ├─ 编译时 hardcode | ✅ 实装 | `const fn` checks |
| ├─ 运行时拦截 | ✅ 实装 | `ActionGuard::check_action` |
| ├─ 多 AI 一致 | ✅ 实装（**注：v5 后改为权限发放**）| `apeireth-core/src/lib.rs` Gate enum |
| ├─ 物理隔离 HA | ✅ 实装 | `HumanAuthority` |
| └─ 反思期审计 | ✅ 实装（trait）| `apeireth-core/src/lib.rs` CognitiveDreamState |
| **权限发放（独立机制）**| ⚠️ **设计意图，部分实装** | v6 §2.2 提议 + apeireth-core 草图 |
| ├─ 多 AI 一致 + V0.5 v2 24 维 | ⚠️ 设计 + apeireth-core trait sketch | Week 5+ 真正实装 |
| ├─ 人类决策 L0 HA | ✅ 实装 | `HumanAuthority` |
| └─ 风险分级（critical 7/high 5/...）| ⚠️ 设计 + apeireth-core 部分 trait | Week 5+ 真正实装 |
| **5 重治理（修改 E 层时）** | ⚠️ **设计意图**，**尚未实装** | v6 §2.1 提议 |
| ├─ MEWG | ⚠️ 设计 | 待施工 |
| ├─ 多人多签 | ⚠️ 设计 | 待施工 |
| ├─ 多 AI | ⚠️ 设计 | 待施工 |
| ├─ 物理多签 | ⚠️ 设计 | 待施工 |
| └─ 反思期 72h | ⚠️ 设计 | 待施工 |
| **E 层修改路径** | ⚠️ **设计意图**，**尚未实装** | v6 §2.3 提议 |
| ├─ 第 1 步守门拒绝 | ✅ 实装 | `check_principle_onion` const fn |
| ├─ 第 2 步权限发放 | ⚠️ 设计 | 待施工 |
| ├─ 第 3 步物理多签 | ⚠️ 设计 | 待施工 |
| ├─ 第 4 步反思期审计 | ⚠️ 设计 | 待施工 |
| └─ 第 5 步 7 席审议 | ⚠️ 设计 | 待施工 |

**总结**：
- ✅ **apeireth-core 实装的**：12 键 verdict + V1+V2+V3 AND 门 + 5 重守门 enum + HA + Cognitive-Dream + 9 生命周期
- ⚠️ **设计意图待施工**：5 重治理 + 权限发放完整机制 + E 层修改路径 + V0.5 v2 24 维公式
- **不要混淆**："v6 设计核心" = 设计意图 + 部分实装，**不是**"已全部实装"

---

## 🛠️ 当前已就绪资源（**施工团队立即可用**）

### ✅ 真实代码（已落地）

| 资源 | 路径 | 状态 |
|---|---|---|
| **apeireth-core** | `crates/apeireth-core/src/lib.rs` | **17516 bytes 真正落地**（v6 完整版部分实装）|
| **集成测试** | `crates/apeireth-core/tests/integration_session_lifecycle.rs` | **2 真测试通过** |
| **hello_world 示例** | `crates/apeireth-core/examples/hello_world.rs` | **8913 bytes 真的能跑**（`cargo run -p apeireth-core --example hello_world`）|
| **Cargo workspace** | `Cargo.toml` | name "apeireth-rust" + 17 crate 推演 |

### ✅ CI / 工程配置

| 资源 | 路径 | 状态 |
|---|---|---|
| **rust-ci.yml** | `.github/workflows/rust-ci.yml` | 898 bytes（ubuntu + stable + cache + build/test/clippy/fmt）|
| **rust-toolchain.toml** | `rust-toolchain.toml` | Rust 1.80 stable 锁定 |
| **CI badges** | README 顶部 | 4 个 badge（CI/License/Rust/Version）|
| **部署配置** | `deploy/` | **计划目录，阶段 7+ 真正施工时再创建**（当前不存在；与 apeireth-legacy/ 描述一致）|

### ✅ 设计文档（v6 完整）

| 资源 | 路径 | 状态 |
|---|---|---|
| **主手册** | `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` | 6546 行 LOCKED（**主人阅读，施工团队可跳过**）|
| **R14 启动文档** | `docs/00-R14-START-HERE.md` | v3 修订（阶段 6 不讨论 + 阶段 7 前端延后）|

---

## 🏛️ 顶层规范系统索引（主人 2026-07-31 落地）

> **规范系统 = Apeireth 一切规则的集合**。施工团队必须理解。

| 文档 | 用途 | 路径 |
|---|---|---|
| **APEIRETH-VERSIONING.md** | 7 子系统版本号规范 | 顶层 |
| **APEIRETH-CONVENTIONS.md** | 12 子规范系统（命名空间/路径/ADR/成就/报告/Commit/状态/锚穿透/不修改承诺/Baseline/架构图）| 顶层 |
| **GLOSSARY.md** | 30+ 项术语 + Apeireth 自创名词（5 项不假装 / 4 重守门 / 权限发放 / 5 重治理 / E 层修改路径 / HA 部署模式 / 双洋葱统一体 / V1+V2+V3 AND 门 / 12 键 verdict cache / 9 阶段生命周期 / Cognitive-Dream 6 状态机 / R-Measure / V0.5 公式 / MEWG / SGI / 6 历史流 / 4 关系 / 三域分离 / 主体连续性 ID 等）| 顶层 |
| **CONTRIBUTING.md** | PR 流程 + commit 规范 | 顶层 |
| **CHANGELOG.md** | 当前版本 + 改了什么 | 顶层 |
| **INSTALL.md** | 三平台安装 | 顶层 |
| **APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md** | 主手册（主人阅读）| 顶层 |
| **00-R14-START-HERE.md** | R14 单一入口 | docs/ |

**所有 Apeireth 自创名词都进 GLOSSARY.md 顶层索引**（v11 提议）。

---

## 🛡️ v6 设计核心（必理解）

### 6 重防御（v6 完整版）

```
v6 = 5 重治理 + 4 重守门 + 权限发放

5 重治理（修改 E 层时，**按部署模式自适应**）:
  1. MEWG（最高优先级解释权）
  2. 多人（**single: 1 人 / multi: N 人**）
  3. 多 AI（不同 LLM 独立验证）
  4. 物理多签（**single: 1 人 + 物理密钥 / multi: N 人 + 物理多签**）
  5. 反思期（修改后 72h 持续审计）

4 重守门（守住原则洋葱 + 权限洋葱）:
  守门 1: 编译时 hardcode（内层，**原则洋葱整体**编译时拒绝）
  守门 2: 运行时拦截（中间层，所有决策前 async check）
  守门 3: 物理隔离（外层，重大修改需物理访问 + 物理多签）
  守门 4: 反思期审计（外层，**守护越权检查**，不与生命力反思混淆）

权限发放（独立机制，不是守门）:
  - 多 AI 一致 + V0.5 v2 24 维
  - 人类决策 + L0 HA（**按部署模式**）
  - 风险分级（critical 7 / high 5 / medium 3 / low 1 / info 0）
```

### HA 部署模式自适应（v7 修正，主人 2026-07-31 关键洞察）

> **关键洞察**：阶段 1 §18.6 + §19.3 + 阶段 2 §11.1 + §11.3 LOCKED 明确说——**保底 1 人类**（single 模式），不是强制多人多签。
> **详细 v7 修正**：`docs/stage4/stage4-correction-v7-deployment-mode-adaptive.md`

```
single 模式（1 人使用 Apeireth，默认推荐）:
├─ deployment_mode = "single"
├─ human_principal_count = 1
├─ HA 实现 = Windows Hello (人脸/指纹) / FIDO2 (YubiKey) / 主人密钥
├─ multisig_policy = "1-of-1"
├─ 满足 §18.6 "至少 1 名真实人类"
└─ 适用: 个人开发者 / 单人桌面 / 个人研究

multi 模式（多人部署：组织 / 团队 / 公司）:
├─ deployment_mode = "multi"
├─ human_principal_count = N (N≥2)
├─ HA 实现 = MultiHuman 多签 + 物理多签
├─ multisig_policy = "M-of-N" (如 2-of-3 / 3-of-5)
├─ 满足 §18.6 "多人参与"
└─ 适用: 企业部署 / 团队研究 / 多人运维

dynamic 模式（运行时切换，§11.4 升级路径）:
├─ 平台不冻结
├─ single → multi: 平滑升级（迁移工具 + 历史流可读权限扩展）
├─ multi → single: 允许（但需 HA，因为是部署模式永久性变更）
└─ 适用: 部署模式从 single 扩到 multi 的过渡期
```

### E 层修改路径（**按部署模式**）

```
single 模式（1 人使用 Apeireth）:
第 1 步: 守门 1-4 默认拒绝（4 重守门嵌套）
第 2 步: 权限发放: 1 个主人单签 + 物理密钥（YubiKey）
第 3 步: 物理访问 + 物理密钥
第 4 步: 反思期审计（72h 持续监控）
第 5 步: 7 席审议最终确认
任何 1 席反对 → 回滚

multi 模式（多人部署）:
第 1 步: 守门 1-4 默认拒绝（4 重守门嵌套）
第 2 步: 权限发放: M-of-N 多签 + 物理多签 + 5 重治理
第 3 步: 物理访问 + 物理多签（AI×3 + 人×N + 密钥×3）
第 4 步: 反思期审计（72h 持续监控）
第 5 步: 7 席审议最终确认
任何 1 席反对 → 回滚
```

### 12 键 / 5 项不假装 / 守门 关系

```
O 层内容（v6）:
├─ 🔑 12 键（V3 9 + v4.1 新增 3 = 12 键判定标准）
├─ 🛡️ 5 项不假装（核心精神）
└─ O-1..O-6（具体操作原则）

4 重守门 = 执行手段（怎么阻止违反）
```

### 🧬 Apeireth 与用户的 4 类关系形态（阶段 1 §18.4 LOCKED）

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. 共生 (Symbiosis)         平等互利，共同演化（推荐）         │
│ 2. 协调 (Coordination)      任务导向，临时合作                  │
│ 3. 嵌入 (Embedding)         嵌入用户系统（嵌入式 agent）         │
│ 4. 与自身关系 (Self-Relation)  主体连续性 / PHL-06              │
│                                                                  │
│ 平台中立性（§18.1）：Apeireth 是平台，不是关系定义者            │
│ → 关系形态由用户选，不强制                                      │
└──────────────────────────────────────────────────────────────────┘
```

### 🔓 三域分离（阶段 2 §2 LOCKED）

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. 思想域 (Thought Domain)    思想 / 判断 / 目标形成自由        │
│                                → ❌ 不审查（主 18.2 思想自由）   │
│ 2. 提案域 (Proposal Domain)    升级为提案后                      │
│                                → ✅ 过 E/S/A/M/O 原则洋葱判定   │
│ 3. 行动域 (Action Domain)      决定执行后                        │
│                                → ✅ 过 L0-L5 权限洋葱 + §9 HA   │
│                                                                  │
│ 关键: 审查从提案域开始（思想域完全自由）                        │
└──────────────────────────────────────────────────────────────────┘
```

### 🆔 主体连续性 ID 跨载体（阶段 2 §4 LOCKED）

```
┌──────────────────────────────────────────────────────────────────┐
│ pub struct IdentityCard {                                        │
│     continuity_id: String,         // 跨载体唯一 ID              │
│     birth_time: u64,                // 诞生时间                   │
│     carriers: Vec<String>,         // 当前所在载体               │
│     migration_history: Vec<MigrationEvent>,  // 迁移历史        │
│ }                                                                │
│                                                                  │
│ 6 历史流 Append-only Log（apeireth-memory）:                    │
│ 1. 思想流 / 2. 提案流 / 3. 行动流                              │
│ 4. 关系流 / 5. 演化流 / 6. 反思期流                             │
│                                                                  │
│ ✅ apeireth-core 已实装 IdentityCard + Episode/Note/Session     │
└──────────────────────────────────────────────────────────────────┘
```

### 🎯 SGI 单字段（阶段 2 §3 LOCKED）

```
┌──────────────────────────────────────────────────────────────────┐
│ SGI = Single Governance Internality = 单一动机单字段            │
│                                                                  │
│ 含义：                                                           │
│ - 主 AI 的"内驱力" 用单字段表达（不是多字段分裂）              │
│ - 主 AI 是单一主体（不是分布式多体）                            │
│ - SGI 永久性变更 = 必须 §9 HA 硬门槛                            │
│                                                                  │
│ ⚠️ 待施工：apeireth-motivation crate + SGI 单字段              │
└──────────────────────────────────────────────────────────────────┘
```

### 📊 R-Measure 维度区分（v8 精炼）

| 指标 | 维度 | 状态 |
|---|---|---|
| **V0.5** | **17 维**（已实装）/ **24 维**（v4.1 §13 提议）| ✅ 开工手册已提 |
| **V1136** | **7 子测度**（已实装）/ **9 子测度**（v4.1 §14 提议）| ✅ 开工手册已提 |

### 🎯 完整版双洋葱统一体（阶段 4 §1.5 LOCKED）

> **双洋葱 = 原则洋葱 E/S/A/M/O 嵌入权限洋葱 L0-L5**（**不是并列**）。
> 详见 `docs/stage4/stage4-runtime-architecture-revised.md` §1.5（v2 修订 76,036 bytes）。

---

## 📋 施工团队工作纪律

### 1. Commit 规范

格式：`<scope>: <subject>`（≤ 72 字符）

| scope | 含义 | 示例 |
|---|---|---|
| `R14` | R14 Rust 重写周期 | `R14: apeireth-cli session 启动` |
| `crate:<name>` | 特定 crate | `crate:apeireth-memory SQLite 存储` |
| `ci` | CI 配置 | `ci: GitHub Actions nightly + coverage` |
| `docs` | 通用文档 | `docs: 施工手册 README 引用` |

### 2. PR 流程

1. Fork + Branch `feature/<scope>`
2. 本地验证：`cargo build + cargo test + cargo clippy + cargo fmt`
3. Commit 规范（见上）
4. Push + PR 到 `main`
5. CI 自动跑：build + test + clippy + fmt
6. Code Review：1 reviewer approve
7. Merge（squash commit）

### 3. 测试要求

- **单元测试**（trait / struct 内 `#[test]`）
- **集成测试**（`crates/<name>/tests/`）
- **文档测试**（`///` 注释中的 doctest）
- **覆盖率**：≥ 80%（核心 crate ≥ 90%）

### 4. Self-Disable 防护（必读）

任何 PR **不得违反** §3 Self-Disable 防护（stage4-external-feedback-and-revisions.md §3）：

- ❌ **不得**修改 L0 HA 相关 trait（编译时 hardcode 拒绝）
- ❌ **不得**添加能绕过 V1+V2+V3 AND 门的代码
- ❌ **不得**添加"询问是否需要 L0 HA"的元问题 API
- ✅ **必须**让反思期白名单生效
- ✅ **必须**保持 HA 在权限洋葱核心 L0（不可变）

### 5. 主哲学 6 锚穿透

每个 PR 必须遵守：

- **S-1 主 22:33 北极星导向**：服务 ASI 北极星
- **S-2 主 17:43 实事求是**：基于现状不重写
- **O-5 主 17:58 不假装**：编译时拒绝假装（12 键）
- **O-2 主 19:33 走在前人经验上**：借鉴 Hermes/OpenClaw/VCP/claude-mem
- **O-3 主 23:44 干到底**：决策立刻沉淀
- **O-4 主 00:56 任何人都能接手**：4 件套齐全

---

## 📞 报告机制（成就驱动，不是时间驱动）

> **主人 2026-07-31 关键洞察**：AI 团队干起来非常快，不能用 1-2 week 当工期。**报告 = 成就达成 + 记录 + 检查 + 回顾文档**。

### ✅ 成就驱动报告

| ❌ 时间驱动（人类团队）| ✅ 成就驱动（AI 团队）|
|---|---|
| 每日 9:00 + 18:00 同步 | **成就达成 → 立即记录** |
| 每周五周报 | **每 5 个成就 → 1 次回顾**（防跑偏）|
| 时间节点 = 检查点 | **完成定义（DoD）达成 = 检查点** |
| Week 1-6 = 工期 | **成就 A1-A20 = 工期**（不固定时间）|

### 🔄 防跑偏机制（每 5 个成就 = 1 次强制回顾）

**回顾节奏**：每完成 5 个成就（A5/A10/A15/A20） → **强制回顾**

**回顾内容**：
1. **对照 v6 设计核心**：4 重守门 + 权限发放 + E 层修改路径 是否仍贯彻？
2. **对照不修改承诺**：是否触动 7 项 LOCKED？
3. **检查 12 键 / 5 项不假装 / 守门关系** 是否保持一致？
4. **检查 Self-Disable 防护** 是否未绕过？
5. **更新回顾文档**：`reports/retrospective-<成就ID>.md`

**回顾报告模板**：
```markdown
# 回顾报告 <A5/A10/A15/A20>

## 完成成就清单
- [ ] A1, A2, A3, A4, A5 ✅

## 防跑偏检查
- [ ] v6 4 重守门嵌套 — 仍贯彻
- [ ] v6 权限发放独立机制 — 仍贯彻
- [ ] v6 E 层修改路径 — 仍贯彻
- [ ] 不修改承诺 7 项 LOCKED — 未触动
- [ ] 12 键 + 5 项不假装 = O 层 — 仍正确
- [ ] Self-Disable 5 大机制 — 未绕过
- [ ] R11 baseline 三值 — 未修改

## 跑偏纠正（如果有）
<列出跑偏的具体位置 + 纠正措施>

## 下一批 5 个成就计划
- A6, A7, A8, A9, A10
```

### 📊 报告路径规范

```
reports/
├── achievement-A1-<角色>-apeireth-cli-session.md
├── achievement-A2-<角色>-integration-tests.md
├── ...
├── retrospective-A5-<角色>.md
├── retrospective-A10-<角色>.md
├── ...
└── final-A20-<角色>-all-crate-complete.md
```

**禁止**：
- ❌ 每日同步报告（没意义）
- ❌ 每周固定时间周报（AI 没时间观念）
- ❌ 工期以 Week 1-6 时间划分（AI 干起来非常快）

**鼓励**：
- ✅ 每个成就达成 → 立即记录
- ✅ 每 5 个成就 → 强制回顾（防跑偏）
- ✅ 出现跑偏 → 立即纠正 + 记录
- ✅ 成就 #A20 完成后 → 总结报告

### 主人拍板机制

- **重大决策**（涉及设计层改动）→ 主人拍板
- **实施细节**（crate 内部实现）→ 团队自主
- **遇到 LOCKED 冲突** → 立即上报，不擅自动 LOCKED
- **成就 #A1-A20 完成节奏** → 主人亲自检查（定期）

### 漂移检查清单（每 5 个成就 A5/A10/A15/A20 强制执行）

```
1. 原则洋葱 5 层（E/S/A/M/O）是否仍 LOCKED 一致？
   □ 阶段 1 §3 LOCKED | 阶段 2 §12 哲学守门 | 阶段 3 §3.8 双洋葱统一体 | 阶段 4 §1.5 完整版

2. HA 实现（按部署模式）是否仍一致？
   □ 阶段 1 §18.6 "至少 1 名真实人类" | 阶段 1 §19.3 HA 选型
   □ 阶段 2 §11.1 DeploymentMode = single/multi | 阶段 3 §4.8 HA 4 实现
   □ 阶段 4 v7 HA 部署模式自适应

3. 双洋葱（统一体 vs 比喻）是否一致？
   □ 阶段 1 §18.7 "正交"（比喻 LOCKED）
   □ 阶段 2 §7 原则×权限正交 | 阶段 3 §3.8 统一体 | 阶段 4 §1.5 完整版统一体

4. 守门数量（5 vs 4）是否明确？
   □ 阶段 2 §6.1 LOCKED "5 重守门" | 阶段 4 v5 "4 重守门嵌套 + 权限发放独立"

5. 12 键位置是否一致？
   □ 阶段 1 §10 V3 9 键 | 阶段 2 §12 哲学守门 | v4.1 §15 12 键 | 阶段 4 O 层

6. 4 关系 / 三域分离 / 主体连续性 / SGI 单字段是否提及？
   □ 阶段 1 §18.4 4 关系 | 阶段 2 §2 三域分离 | 阶段 2 §3 SGI | 阶段 2 §4 主体连续性
```

---

## 📌 一句话总结

**施工团队 = AI 团队（不是人类团队），按成就驱动（A1-A20），不固定时间；前端是另一个团队。设计层 v6 LOCKED 0 触动；真正写代码，不是更多文档；每 5 个成就强制回顾（防跑偏）；发现设计漂移立即与主人沟通澄清。**

---

## 🚀 立即开干（A1 第 1 天）

### Day 1 必做

```bash
# 1. Clone
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust

# 2. 验证环境（已就绪）
cargo --version  # 应是 1.80 stable
rustup component list --installed  # 应有 rustfmt + clippy + rust-src

# 3. 跑现有测试（已就绪）
cargo build --workspace
cargo test --workspace
cargo test --test integration_session_lifecycle -p apeireth-core
cargo run -p apeireth-core --example hello_world

# 4. 创建 feature branch
git checkout -b feature/apeireth-cli-session
```

### A1 任务

让 `cargo run --bin apeireth-cli session` **真的能跑**：
- 启动 session
- 打印欢迎信息
- 主交互 = 对话流（输入文本 → 走 V1+V2+V3 → 返回）

---

## 📂 项目结构（开干参考）

```
Apeireth-rust/                          ← 顶层（施工团队入口）
├── START-CONSTRUCTION.md                ← 你正在读的（顶层开工手册）
├── README.md                            ← 顶层入口（v6 修订链 §文档修订链）
├── CONTRIBUTING.md                      ← PR 流程
├── LICENSE                              ← Apache-2.0
├── CHANGELOG.md                         ← 变更日志
├── INSTALL.md                           ← 安装步骤
├── ROADMAP.md                           ← 路线图
├── GLOSSARY.md                          ← 术语表
├── APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md  ← 主手册（主人阅读）
├── Cargo.toml                           ← workspace
├── crates/                              ← 17 器官 crate
│   ├── apeireth-core/                   ← ✅ 已实装 17516 bytes
│   │   ├── src/lib.rs
│   │   ├── tests/integration_session_lifecycle.rs  ← ✅ 2 真测试
│   │   └── examples/hello_world.rs  ← ✅ 能跑
│   ├── apeireth-cli/                    ← ⏳ A1 第 1 天开工
│   ├── apeireth-memory/                 ← ⏳ A4
│   ├── apeireth-perception/             ← ⏳ A9
│   ├── apeireth-cognition/              ← ⏳ A10
│   ├── apeireth-action/                 ← ⏳ A11
│   ├── apeireth-motivation/             ← ⏳ A11
│   ├── apeireth-value/                  ← ⏳ A11
│   ├── apeireth-consciousness/          ← ⏳ A12
│   ├── apeireth-relation/               ← ⏳ A12
│   ├── apeireth-life-force/             ← ⏳ A13
│   ├── apeireth-council/                ← ⏳ A14
│   ├── apeireth-upgrade/                ← ⏳ A15
│   ├── apeireth-bus/                    ← ⏳ A16
│   ├── apeireth-extension/              ← ⏳ A16
│   ├── apeireth-pybridge/               ← ⏳ A16
│   ├── apeireth-bench/                  ← ⏳ 保留
│   ├── apeireth-test/                   ← ⏳ 保留
│   └── apeireth-philosophy/             ← ❌ DEPRECATED（A17 物理删除）
├── docs/                                ← 56 份设计文档
│   ├── 00-R14-START-HERE.md
│   ├── stage1/  (2201 行 LOCKED)
│   ├── stage2/  (19 文件 LOCKED)
│   ├── stage3-blueprints/  (14 文件 LOCKED)
│   ├── stage4/  (8 子文档 + v3/v4/v5/v6 修正链 LOCKED)
│   │   ├── README.md
│   │   ├── stage4-runtime-architecture-revised.md
│   │   ├── stage4-patches-v2-crate-correction.md
│   │   ├── stage4-external-feedback-and-revisions.md
│   │   ├── architecture-stage4-engineering-landing.md
│   │   ├── architecture-stage4-inspiration-supplements.md
│   │   ├── architecture-stage4-patches.md
│   │   ├── stage4-thinking-document.md
│   │   ├── stage4-correction-v3-onion-embedded-keys-gates.md  (19267 bytes)
│   │   ├── stage4-correction-v4-onion-dedupe.md  (15922 bytes)
│   │   ├── stage4-correction-v5-gates-refined.md  (15410 bytes)
│   │   └── stage4-correction-v6-consolidated-and-e-layer-mutation.md  (16171 bytes)
│   ├── stage5/
│   │   ├── stage5-construction-document.md  ← 🔒 LOCKED
│   │   └── construction-kickoff-manual.md  ← ⚠️ 已移到顶层（保留旧路径向后兼容）
│   ├── adr/                             ← ADR 框架
│   └── architecture-v4-living-intelligence.md  (LOCKED)
├── deploy/                              ← **计划目录**，阶段 7+ 真正施工时再创建（当前不存在）
├── .github/workflows/rust-ci.yml
└── apeireth-legacy/                     ← R11 1305 文件归档索引（**阶段 7+ 真正施工时再归档，当前只有 README**）
```

---

## 🎯 成功标准（成就驱动）

| 成就达成 | 验证项 |
|---|---|
| **A1 完成** | `cargo run --bin apeireth-cli session` 启动 + 欢迎信息 |
| **A2 完成** | 集成测试全绿 |
| **A3 完成** | 12 键编译时 hardcode 拒绝违反代码 |
| **A4 完成** | Episode 写入 + 查询（SQLite）|
| **A5 完成** | V1+V2+V3 AND 门完整 impl |
| **A6 完成** | CI coverage/nightly/benchmark 跑通 |
| **A7 完成** | Self-Disable 5 大机制最小可用 |
| **A8 完成** | R-Measure 跑出 R11 baseline 三值（**= 最小可行 demo 达成**）|
| **A17 完成** | apeireth-philosophy 物理删除 |
| **A20 完成** | 后端 17 crate 完整（**= 后端完整达成**）|
| **（前端）A30+ 完成** | 前端完整（**另一个团队，主人延后**）|

---

## 🛡️ 不修改承诺（施工团队必须遵守）

- ✅ 阶段 1+2+3 LOCKED（54 份设计文档）— **不重写**
- ✅ v2 / v4 / v4.1 LOCKED（哲学层纲领）— **不修改**
- ✅ 阶段 4 主文档 LOCKED（1492 行）— **不修改**
- ✅ 阶段 5 施工文档 LOCKED（631 行）— **不修改**
- ✅ v6 修正（4 重守门 + 权限发放 + E 层修改路径）— **不破坏**
- ✅ R11 baseline 三值 LOCKED — **不修改**
- ✅ v1 → v5 历史链 LOCKED — **保留，不删除**

---

## 📌 一句话总结

**施工团队 = AI 团队（不是人类团队），按成就驱动（A1-A20），不固定时间；前端是另一个团队。设计层 v6 LOCKED 0 触动；真正写代码，不是更多文档；每 5 个成就强制回顾（防跑偏）；发现设计漂移立即与主人沟通澄清。**

---

_本开工手册由 leader 亲自产出（按主人 2026-07-31 多次精炼洞察）._
_v3 修订版（外部 agent Round 5 反馈 5 个真问题全部修复）._
_主人亲自检查 v3 修订版，施工团队按成就驱动开干 A1._