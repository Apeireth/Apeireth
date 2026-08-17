# Apeireth R14 施工团队开工手册（leader 亲自产出，全局观视角）

> **状态**: 主人 2026-07-31 "写一份开工手册给施工团队"
> **生成时间**: 2026-07-31
> **依据**: v6 完整版 + 阶段 5 施工文档 LOCKED + 4 件套 + GLOSSARY v6 + ROADMAP + examples/hello_world.rs + 集成测试
> **路径**: `Apeireth-rust/docs/stage5/construction-kickoff-manual.md`
> **施工团队范围**: **仅后端 Rust 实现**（17 crate + 集成测试 + CI）。**不承担前端**（阶段 7 延后，由另一个团队负责）。

---

## 📜 给施工团队的总览信

**欢迎来到 Apeireth R14 施工期。**

你们是**施工团队**——负责把 R14 的设计图（54 份 LOCKED 设计文档 + v6 完整版）变成**真正可运行的 Rust 代码**。

你们**不承担**前端（阶段 7）。前端是**另一个团队**的责任——他们在阶段 7 实施时根据 `architecture-frontend-design-proposal.md` 工作。

你们的工作**目标**：**1.5 个月内交付最小可行 demo**（Week 1-6），让主人能看到 Apeireth 真的跑起来。

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
| **阶段 5 施工文档 LOCKED**（刚拍板）| 不修改 |
| **v6 修正（独立命名空间）**| 修正链保留 v1-v6，**不删任何历史版本** |
| **R11 baseline 三值**（V1141=0.8682 / V1131=0.8532 / V1136=0.9063）| 不修改 |

### ✅ 可修改（v6 明确）

- V0.5/V1136/9 键 v2（v4.1 §13/§14/§15 提议落地）
- R11 1100 重写（保留 ~30% + 合并 ~60% + 砍 ~10%）
- crates/ 占位（按阶段 4 §2 + v6 完整版重写）
- Cargo.toml metadata
- Self-Disable 防护代码

---

## 🚀 成就驱动最小可行 demo（A1-A8 + A9-A20）

> **主人 2026-07-31 关键洞察**：AI 团队干起来非常快，工期不可能 1-2 week。所以改成**成就驱动**（不是 Week 1-6 时间驱动）。
> **成就单元 = 完成定义（DoD）** ——每个成就有明确的 DoD 和验证命令。

### A1-A8（最小可行 demo = 8 个成就）

| 成就 | 描述 | DoD | 验证命令 | 防跑偏 |
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
| 6 | **ROADMAP.md** | `ROADMAP.md` | Week 1-6 最小 demo + Week 7-12 后端完整 |

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
| 17 | **apeireth-legacy 归档索引** | `apeireth-legacy/README.md` |

---

## 🛠️ 当前已就绪资源（**施工团队立即可用**）

### ✅ 真实代码（已落地）

| 资源 | 路径 | 状态 |
|---|---|---|
| **apeireth-core** | `crates/apeireth-core/src/lib.rs` | **17516 bytes 真正落地**（v6 完整版）|
| **集成测试** | `crates/apeireth-core/tests/integration_session_lifecycle.rs` | **2 真测试通过** |
| **hello_world 示例** | `crates/apeireth-core/examples/hello_world.rs` | **8913 bytes 真的能跑** |
| **Cargo workspace** | `Cargo.toml` | name "apeireth-rust" + 17 crate 推演 |

### ✅ CI / 工程配置

| 资源 | 路径 | 状态 |
|---|---|---|
| **rust-ci.yml** | `.github/workflows/rust-ci.yml` | 898 bytes（ubuntu + stable + cache + build/test/clippy/fmt）|
| **rust-toolchain.toml** | `rust-toolchain.toml` | Rust 1.80 stable 锁定 |
| **CI badges** | README 顶部 | 4 个 badge（CI/License/Rust/Version）|
| **部署配置** | `deploy/18-crates/` | 18 个 Dockerfile + docker-compose + k8s |

### ✅ 设计文档（v6 完整）

| 资源 | 路径 | 状态 |
|---|---|---|
| **主手册** | `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` | 6546 行 LOCKED + 附录 M + 附录 N |
| **R14 启动文档** | `Apeireth-rust/docs/00-R14-START-HERE.md` | v3 修订（阶段 6 不讨论 + 阶段 7 前端延后）|

---

## 🛡️ v6 设计核心（必须理解）

### 6 重防御（v6 完整版）

```
v6 = 5 重治理 + 4 重守门 + 权限发放

5 重治理（修改 E 层时）:
  1. MEWG（最高优先级解释权）
  2. 多人（每个修改需多人参与）
  3. 多 AI（不同 LLM 独立验证）
  4. 物理多签（AI×3 + 人×2 + 密钥×3）
  5. 反思期（修改后 72h 持续审计）

4 重守门（守住原则洋葱 + 权限洋葱）:
  守门 1: 编译时 hardcode（内层，**原则洋葱整体**编译时拒绝）
  守门 2: 运行时拦截（中间层，所有决策前 async check）
  守门 3: 物理隔离（外层，重大修改需物理访问 + 物理多签）
  守门 4: 反思期审计（外层，**守护越权检查**，不与生命力反思混淆）

权限发放（独立机制，不是守门）:
  - 多 AI 一致 + V0.5 v2 24 维
  - 人类决策 + L0 HA
  - 风险分级（critical 7 / high 5 / medium 3 / low 1 / info 0）
```

### E 层修改路径（**关键洞察**）

```
第 1 步: 守门 1-4 默认拒绝（4 重守门嵌套）
第 2 步: 权限发放（5 重治理 + V0.5 v2 24 维 + L0 HA）= 例外允许
第 3 步: 物理多签 + 重新编译 apeireth-core 二进制
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

## 🎯 接手者明确信息

### Apeireth 是什么？

- **高自主性长程 agent** 平台（有生命的智能体，不是软件系统）
- **不是软件**：活系统施工有 3 大特殊性（活系统 / 哲学守门 / 跨载体）
- **北极星**：逼近通用人工智能（ASI）——但**不假装已达**（主 17:58 不假装）

### 施工团队的目标

1. **A8 完成** = 最小可行 demo = 主人可看到 Apeireth 真的跑起来（**按成就节奏，不固定时间**）
2. **A20 完成** = 后端完整 = 17 crate 全部落地 + 集成测试 + CI（**按成就节奏，不固定时间**）
3. **主 23:44 干到底**：不要再多写设计文档，**真正写 Rust 代码**
4. **成就驱动报告**（主人 2026-07-31 新姿态）：AI 干起来非常快，不能用 1-2 week 当工期

---

## 🚨 主人 2026-07-31 关键洞察完整版

> **施工团队 = AI 团队，不是人类团队。AI 干起活儿来会非常快，不是人类的速度，工期也不可能是 1-2 week 之类的。**
> **AI 也没时间观念，所以日报/周报意义不大。**
> **报告最好以"什么成就达成后记录 + 检查 + 回顾文档确认没有跑偏"这样来实现效果。**

### 报告机制（v2 开工手册修订）

| ❌ 之前（人类团队）| ✅ 现在（AI 团队）|
|---|---|---|
| 每日 9:00 + 18:00 同步 | **成就达成 → 立即记录** |
| 每周五周报 | **每 5 个成就 → 1 次回顾**（防跑偏）|
| 时间节点 = 检查点 | **完成定义（DoD）达成 = 检查点** |
| Week 1-6 = 工期 | **成就 A1-A20 = 工期**（不固定时间）|

### 报告路径

```
reports/
├── achievement-A1-<角色>-apeireth-cli-session.md    # 单个成就
├── achievement-A2-<角色>-integration-tests.md
├── ...
├── retrospective-A5-<角色>.md                       # 5 个成就回顾
├── retrospective-A10-<角色>.md
├── ...
└── final-A20-<角色>-all-crate-complete.md           # 总结
```

**鼓励**：
- ✅ 每个成就达成 → 立即记录（不要等"明天同步"）
- ✅ 每 5 个成就 → 强制回顾（防跑偏）
- ✅ 出现跑偏 → 立即纠正 + 记录

### 施工团队的边界

- ✅ 后端 Rust 代码
- ✅ 集成测试 + CI/CD
- ✅ OTA / Self-Disable / Cognitive-Dream 真正实现
- ❌ **不负责前端**（阶段 7 另一个团队）

---

## 📞 联系方式 + 报告机制

### 🚨 主人 2026-07-31 关键洞察：AI 团队 ≠ 人类团队

> **AI 团队干起活来非常快，不是人类的速度，工期不可能是 1-2 week 之类的，AI 也没时间观念。**
> **报告最好以"什么成就达成后记录 + 检查 + 回顾文档确认没有跑偏"这样来实现效果。**

### ✅ 成就驱动报告（不是日报/周报）

**AI 团队报告节奏 = 成就达成后**（**不是**时间固定间隔）：

| ❌ 时间驱动（人类团队）| ✅ 成就驱动（AI 团队，主人 2026-07-31 新姿态）|
|---|---|
| 每日 9:00 + 18:00 同步 | **成就达成 → 立即记录** |
| 每周五周报 | **每 N 个成就 → 1 次回顾**（防跑偏）|
| 时间节点 = 检查点 | **完成定义（DoD）达成 = 检查点** |
| 工期 = 周/月 | **工期 = 成就单元数** |

### 📋 成就单元清单（替代 Week 1-6 时间节奏）

| 成就 # | 描述 | DoD（完成定义）| 验证命令 | 防跑偏检查 |
|---|---|---|---|---|
| **A1** | apeireth-cli session 启动 | `cargo run --bin apeireth-cli session` 输出欢迎信息 | `cargo run` | 对照 v6 §6 设计核心 |
| **A2** | 集成测试通过 | `cargo test --test integration_*` 全绿 | `cargo test --test integration_session_lifecycle -p apeireth-core` | 对照 §1 施工团队责任 |
| **A3** | 12 键编译时 hardcode | `cargo check` 0 error + 5+ 违反测试失败 | `cargo check --workspace` | 对照 §6 v6 守门 1 |
| **A4** | apeireth-memory SQLite 存储 | 10+ tests pass + Episode 写入查询 | `cargo test -p apeireth-memory` | 对照 §6 不修改承诺 |
| **A5** | V1+V2+V3 AND 门完整 impl | 10+ tests pass + 正常+危险决策 | `cargo test --test integration_*` | 对照 §6 v6 §1.3 |
| **A6** | CI coverage/nightly/benchmark | GitHub Actions 3 个新工作流 | README badge 更新 | 对照 §1 CI/CD 责任 |
| **A7** | Self-Disable 5 大机制最小 | 5+ 单元测试 + 集成 | `cargo test --test self_disable` | 对照 §7 Self-Disable 必读 |
| **A8** | R-Measure V0.5/V1136/V3 翻译 | 跑 R11 baseline 三值（0.8682/0.8532/0.9063）| `cargo test --test r_measure` | 对照 R11 baseline LOCKED |
| **A9-A17** | 后续 9 个 crate 真正落地 | 每个 crate DoD：编译通过 + 5+ tests + examples | 各自 `cargo test -p <name>` | 对照 §12 项目结构 |
| **A18** | OTA 7 阶段工程化 | OTA 状态机完整 + 5+ tests | `cargo test -p apeireth-upgrade` | 对照 §1 OTA 责任 |
| **A19** | 反思期审计 6 状态机 | Cognitive-Dream trait + 6 状态转换 | `cargo test -p apeireth-consciousness` | 对照 §1 Cognitive-Dream |
| **A20** | 17 crate 全部集成 | `cargo build --workspace` 0 error | `cargo build --workspace` | 对照 §12 完整项目结构 |

### 🔄 防跑偏机制（每 5 个成就 = 1 次回顾）

**回顾节奏**：每完成 5 个成就（A5/A10/A15/A20） → **强制回顾**

**回顾内容**：
1. **对照 v6 设计核心**（§6）：4 重守门 + 权限发放 + E 层修改路径 是否仍贯彻？
2. **对照不修改承诺**（§14）：是否触动 7 项 LOCKED？
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
├── achievement-A1-<角色>-apeireth-cli-session.md    # 单个成就交付报告
├── achievement-A2-<角色>-integration-tests.md
├── ...
├── retrospective-A5-<角色>.md                       # 5 个成就回顾（防跑偏）
├── retrospective-A10-<角色>.md
├── ...
└── final-A20-<角色>-week-N-complete.md              # 全部 17 crate 完成总结
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

---

## 🚀 立即开干（Week 1 第 1 天）

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

### Day 1 任务

让 `cargo run --bin apeireth-cli session` **真的能跑**：
- 启动 session
- 打印欢迎信息
- 主交互 = 对话流（输入文本 → 走 V1+V2+V3 → 返回）

---

## 📂 项目结构（开干参考）

```
Apeireth-rust/
├── README.md                    ← 顶层入口
├── CONTRIBUTING.md              ← 你正在读的（PR 流程）
├── LICENSE                      ← Apache-2.0
├── CHANGELOG.md                 ← 变更日志
├── INSTALL.md                   ← 安装步骤
├── ROADMAP.md                   ← 路线图
├── GLOSSARY.md                  ← 术语表
├── Cargo.toml                   ← workspace
├── crates/                      ← 17 器官 crate
│   ├── apeireth-core/           ← ✅ 已实装 17516 bytes
│   │   ├── src/lib.rs
│   │   ├── tests/integration_session_lifecycle.rs  ← ✅ 2 真测试
│   │   └── examples/hello_world.rs  ← ✅ 能跑
│   ├── apeireth-cli/            ← ⏳ Week 1 第 1 天开工
│   ├── apeireth-memory/         ← ⏳ Week 2
│   ├── apeireth-perception/     ← ⏳ Week 7
│   ├── apeireth-cognition/      ← ⏳ Week 7
│   ├── apeireth-action/         ← ⏳ Week 8
│   ├── apeireth-evolution/      ← ⏳ Week 11
│   ├── apeireth-motivation/     ← ⏳ Week 8
│   ├── apeireth-value/          ← ⏳ Week 8
│   ├── apeireth-consciousness/  ← ⏳ Week 9
│   ├── apeireth-relation/       ← ⏳ Week 9
│   ├── apeireth-life-force/     ← ⏳ Week 10
│   ├── apeireth-council/        ← ⏳ Week 11
│   ├── apeireth-upgrade/        ← ⏳ Week 11
│   ├── apeireth-bus/            ← ⏳ Week 12
│   ├── apeireth-extension/      ← ⏳ Week 12
│   ├── apeireth-pybridge/       ← ⏳ Week 12
│   ├── apeireth-bench/          ← ⏳ 保留
│   ├── apeireth-test/           ← ⏳ 保留
│   └── apeireth-philosophy/     ← ❌ DEPRECATED（标记删除，阶段 7+ 物理删除）
├── docs/                        ← 56 份设计文档
│   ├── 00-R14-START-HERE.md
│   ├── stage1/  (2201 行 LOCKED)
│   ├── stage2/  (19 文件 LOCKED)
│   ├── stage3-blueprints/  (14 文件 LOCKED)
│   ├── stage4/  (8 子文档 + v3/v4/v5/v6 修正链 LOCKED)
│   ├── stage5/
│   │   ├── stage5-construction-document.md  ← 🔒 LOCKED
│   │   └── construction-kickoff-manual.md  ← 你正在读的（开工手册）
│   ├── adr/                     ← ADR 框架
│   └── architecture-v4-living-intelligence.md  (LOCKED)
├── examples/                    ← 保留（hello_world 备选位置）
├── tests/                       ← 保留（R11 旧测试）
├── deploy/                      ← 18-crates Dockerfile + compose + k8s
├── .github/workflows/rust-ci.yml
└── apeireth-legacy/             ← R11 1305 文件归档索引
```

---

## 🎯 成功标准（成就驱动，不是时间驱动）

> **主人 2026-07-31 关键洞察**：AI 团队干起来非常快，不能用 1-2 week 当工期。**成功标准 = 成就达成**，不是时间节点。

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

**施工团队 = AI 团队（不是人类团队），按成就驱动（A1-A20），不固定时间；前端是另一个团队。设计层 v6 LOCKED 0 触动；真正写代码，不是更多文档；每 5 个成就强制回顾（防跑偏）。**

---

_本开工手册由 leader 亲自产出（按主人 2026-07-31 "全局观视角给施工团队写一份开工手册"）._
_主人拍板让另一个团队准备动工，本手册地址给主人亲自检查._
_主哲学 6 锚穿透. 任何接手者能查._
_施工团队按本手册开干 Week 1._