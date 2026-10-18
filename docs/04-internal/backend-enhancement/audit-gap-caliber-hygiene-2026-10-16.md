# 落差清账·口径尾账批（B-04 / B-05 / D13 / F-02）交付报告

> `Status: ✅ B-04 / B-05 / D13 已清；🟡 F-02 只出方案表不动文件（待 owner 拍板后另批执行）`
> 性质：外部审计「宣称与实现落差清单」尾账批（守门级数口径 / 键值口径 / 字母双占 / 大体量资产盘点），
> 文档 + 轻文件操作批。**不 commit**（提交由 owner/整合线拍板）。
> 纪律：中性表述 / 扫描零命中（现值面）/ 校验带 `-j 2` / 历史不追溯改写（带日期档案只加标注不动原值）/
> 测试口径单一真源 [test-baseline.md](../../03-reference/test-baseline.md) 本批 0 触碰。
> 每个数字可复核：命令 + 日期 + 工具链见 §5；本批实测值如实自报，不代改真源。

---

## 0. 四件对照表

| # | 审计项（清单出处） | 审计原状 | 本批动作 | 落点 | 状态 |
|---|---|---|---|---|---|
| 1 | **B-04 守门级数口径**（清单 B-04） | 5 重 / 6 重 v6 / 7 重 v7 三口径并存：`commit-msg-v2.0.0-rc.1-tag.md:35,90,98`、`CHANGELOG.md:557,563,865`、`docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md:292`、`OSS_NOTICE.md:308`、`library/v1.0/_BORROW_IDS.md:114`、`ROADMAP.md:85,191` | 现值统一 **7 重守门 v7**（含 Colang DSL + Skill Guard），**ROADMAP §13「守门级数」行 = 唯一现值出处**；历史文档逐处加「历史口径（日期）」标注、原值不动；v1.0 时代 9 重 v9 留痕加历史口径标注（Cargo.toml metadata）或登记 0 触碰（LOCKED 锚本体） | `ROADMAP.md` §3/§13、`CHANGELOG.md`、`docs/archive/research/commit-msg-v2.0.0-rc.1-tag.md`、`docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`、`OSS_NOTICE.md`、`library/v1.0/_BORROW_IDS.md`、`Cargo.toml` metadata、`docs/02-guides/` 4 处口径注 | ✅ 已清（LOCKED 异口径登记见 §1.3，待 owner 拍板项见 §7） |
| 2 | **B-05 键值口径**（清单 B-05） | `ROADMAP.md:190`（现）写「12 键 → 13 键」而多处历史文档写「13 键 LOCKED」；R11 baseline 3 值只有标签、无可复现入口 | §13 键行现值统一「**13 键**（LOCKED）」（12 键降为行内历史口径）；R11 baseline 3 值加「**无公开复现入口，仅供内部对照**」如实标注（数值 LOCKED 0 触碰） | `ROADMAP.md` §13 两行 | ✅ 已清 |
| 3 | **D13 字母双占**（00-backlog 债栏 D13） | 00-backlog 想法清单「吸收自动化（I 轴）」与星港线「I」同占 I 字母；I1 施工线注释/分期文档同用「I 轴」口径 | 吸收自动化统一改名 **P 轴（Practice 沉淀）**，分期 **I1-I4 → P1-P4**，全库引用同步（16 文件：文档 9 + src 注释 7 文件 8 处）；`I-absorption-automation.md` / `I1-skill-distill-delivery.md` **文件名保留 + 内容别名注记**防断链；星港线保留 **I**（含「I 批」） | `00-backlog.md`（想法清单 + 命名注记 + 债栏）、`I-absorption-automation.md`、`I1-skill-distill-delivery.md`、`absorption-candidates-2026-10.md`、`absorption-ledger.md`、`quick-capture-delivery.md`、`capabilities-matrix.md`、`user-manual.md`、`frontend-data-contract.md`、`skill/{mod,card,distill}.rs`、`tools/lib.rs`、`cli/lib.rs`、`production.rs`、`tool_modules.rs` | ✅ 已清 |
| 4 | **F-02 大体量资产盘点**（清单 F-02） | `reports/`（审计时 1069 件 38MB）、`previews/`（39MB）、`scripts/guard_ml/*.jsonl`（21MB）随库提交 | **只出方案表不动文件**：构成清单 + 四选项（迁 release 资产 / 迁 docs/archive/ / 外置 / git-lfs）逐项建议（§4）；引用面与执行批前置一并查明（`include_str!` 编译期依赖、CI 重生成语义等） | 本文件 §4 + `00-backlog.md` 债栏 D17 | 🟡 方案已出，待 owner 拍板后另批执行 |
| 附 | **00-backlog 债栏状态注记**（任务 #5） | 债栏仅登记 D13 | 四件状态注记齐落：D13 ✅ 已清 / D15（B-04）✅ / D16（B-05）✅ / D17（F-02）🟡 留账 | `00-backlog.md` 债栏 | ✅ |

---

## 1. B-04 守门级数口径

### 1.1 现值（唯一出处 = ROADMAP §13「守门级数」行）

**守门级数现值 = 7 重守门 v7**（1-5 重嵌套 + 6 Colang DSL + 7 Skill Guard），与 §13 LOCKED 行
「5 重守门 → 6 重 v6 → 7 重 v7 (+ Colang DSL + Superpowers Skill Guard)」的历史沿革末值一致。
本批动作 = 把这一个现值写死在 ROADMAP（§3 注记 + §3 表「CI / 守门」行 + §13 行 + 尾注四处同声），
其余文档不再各自宣称级数。

### 1.2 历史文档标注清单（加「历史口径（日期）」前缀/行内标注，原值不动）

| 文件 | 原值（保留） | 标注 |
|---|---|---|
| `CHANGELOG.md:557` | O-6 5 重守门 workflow | 前缀「历史口径（2026-08-27）」 |
| `CHANGELOG.md:563` | 5 重守门（o6-anchor.yml 五项） | 前缀「历史口径（2026-08-27）」+ 现值指针 |
| `CHANGELOG.md:865` | 6 重守门 v6 → v7 升级 (B4) | 前缀「历史口径（2026-08-10）」+ 现值指针 |
| `docs/archive/research/commit-msg-v2.0.0-rc.1-tag.md:35,90,98` | 5 重守门 ×3 | 行内「历史口径（2026-08-28）」+ 现值指针 |
| `docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md:292` | 5 重守门 baseline | 行内「历史口径（2026-08-28）」+ 现值指针 |
| `OSS_NOTICE.md:155` | B4 6 重 v6 | 行内「历史口径（2026-08）」+ 现值指针 |
| `OSS_NOTICE.md:308` | 6 重守门 v6 | 行内「历史口径（2026-08）」+ 现值指针 |
| `library/v1.0/_BORROW_IDS.md:114` | B4 6 重守门 v6 | 行尾「历史口径（v1.0 时代，2026-08）」+ 现值指针 |
| `Cargo.toml:61`（description 注释）、`:116-117`（hard_walls 注释 + 值行尾注）、`:134`（guard_gates_version 注释） | 9 重守门 v9 / 9 重 v9（v1.0 release 留痕） | 同行「历史口径（v1.0 时代，2026-08）」；**值本体 0 改**（hard_walls 字符串原样，仅加 TOML 行尾注释） |
| `docs/02-guides/v2-frontend-quickstart.md:220`、`v2-gateway-frontend-integration-spec.md:422,450`、`v2-gateway-frontend-integration-spec-r13-review.md:229` | 5 重守门（= CI 五门） | 行内口径注：「5 重 = CI 五门（clippy/tests/legacy/LOCKED/哲学锚表头），非守门级数；现值 7 重守门 v7」 |

### 1.3 异口径如实登记（0 触碰 / 不本批代改）

「X 重」在库内有**三个语义**，本批只统一「守门级数」这一义（另两义保留，MANIFESTO §14 已有辨析注）：

| 语义 | 例 | 处置 |
|---|---|---|
| 守门级数（layer lineage） | 5 重 → 6 重 v6 → 7 重 v7（v1.0 时代续到 8 Action Rail / 9 Evidence Guard） | 现值 = 7 重守门 v7（本批统一） |
| CI 五门 | `.github/workflows/o6-anchor.yml` 的 clippy / tests / legacy / LOCKED / 哲学锚表头 5 检查 | 保留；docs/02-guides 4 处加口径注区分 |
| `Gate` 枚举五层 | `crates/foundation/core/src/gate.rs`（编译时 hardcode / 运行时拦截 / 多 AI 一致 / 物理隔离 HA / 反思期审计） | 代码语义本体，不动 |

以下**带 v1.0 时代 9 重 v9 / 6 重 v6 等异口径的留痕点，本批 0 触碰、如实登记**（改它们 = 动 LOCKED 或追溯改写历史，需 owner 另行授权）：

- `docs/01-architecture/philosophy.md:10`（O-1 锚描述「9 重 v9 + 13 键 + 3 项不可变脊柱」）——**哲学锚本体（LOCKED 5 项之一）**，0 触碰；
- `crates/foundation/core/src/eight_anchors.rs`（:14/:47/:77/:78/:111，「6 重守门 v6」「5 重守门 v5」锚描述字符串）——LOCKED 本体，0 触碰；
- `crates/foundation/core/src/gate.rs:1,13,16`、`lib.rs:1,112`、`examples/hello_world.rs`、`tests/*` 的「5 重守门」——core 核心注释/测试（`Gate` 五层语义 + 测试口径行），0 触碰；
- `scripts/release/*`（tag/verify/git-push/deploy/CHECKLIST/README 10 文件，「6 重守门 v7」「6 重 v6 → v7」）——v1.0 发布脚本族留痕，登记待发布脚本归档批；
- `library/v1.0/README.md:191`、`library/_meta/0-装-PASS-严守-声明.md:141`、`library/v1.0/books/30-books-by-9-organ.md` 等（「6 重守门 v6」）——v1.0 资料库档案，带时代属性；
- `legacy/`（源码注释）、`reports/`（450+ 件 v1.0 时代子代理报告，单文件命中可达 188 行）、`docs/archive/**`、带日期的 `docs/04-internal/*-2026-08-*.md` 等——历史卷宗，按纪律只登记不改；
- `Cargo.toml` metadata `hard_walls` 字符串值 / `guard_gates_version = "v9 (...)"` 值——1.0 release 时代自描述值，**值不改**（行尾/段首已加历史口径标注）。

## 2. B-05 键值口径

| 项 | 审计原状 | 本批动作（ROADMAP §13 现值行修） | 0 触碰边界 |
|---|---|---|---|
| 12→13 键 | 行首写「12 键 → 13 键编译期 hardcode」，与历史文档「13 键 LOCKED」两种表述并存 | 现值行首统一「**13 键**（🔒 LOCKED 现值；表述与历史文档「13 键 LOCKED」统一）」；「12 键 → 13 键」降为行内「历史口径（v4.1 时代）」 | 13 键为 LOCKED 5 项之一：**键集与 `verdict_cache_keys = 13` 值 0 触碰**，只统一表述 |
| R11 baseline 3 值 | `0.8682/0.8532/0.9063` 只有标签（R11 ASI R-Measure），无可复现入口 | 行内如实标注「**无公开复现入口，仅供内部对照**」：const 源仅存 `legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44`，active workspace 无 const source，复现入口需 R12 spec 重审后另立 | 数值 LOCKED 0 触碰（`0.8682/0.8532/0.9063` 原样保留）；`ENGINEER-MANIFESTO` §10 / `TO-NEW-TEAM.md` / `maintenance-guide.md` 已有同义诚实注，本批不重复改动 |

## 3. D13 字母双占（吸收自动化 I 轴 → P 轴）

### 3.1 改名映射（统一口径）

| 旧 | 新 | 说明 |
|---|---|---|
| I 轴（吸收自动化） | **P 轴（Practice 沉淀）** | 单字母 P；语义 = 「吸收优秀实践 → 产品本能」的沉淀轴 |
| I1 书→技能沉淀 | **P1 书→技能沉淀** | 已交付件（2026-10-13），行为 0 变 |
| I2 调用面深化 / I3 历史即记忆 / I4 技能生态 | **P2 / P3 / P4** | 待排期件，仅改名 |
| （星港）I / I 批 1-5 | **保留 I** | 星港内置应用面不动，双占解除后 I 单义 |

**文件名保留清单（防断链，内容带别名注记）**：`I-absorption-automation.md`、`I1-skill-distill-delivery.md`
——文内链接路径不改；旧名只在带日期历史记录里保留（历史不追溯改写）。

### 3.2 全库引用同步清单（16 文件：文档 9 + src 注释 7）

| 文件 | 处理 |
|---|---|
| `docs/04-internal/backend-enhancement/00-backlog.md` | 想法清单 I 行 → **P 行**（含「P 轴 · Practice 沉淀」副名）+ 表下命名注记 + 债栏 D13 状态注记 |
| `docs/04-internal/backend-enhancement/I-absorption-automation.md` | 标题改「P 轴——吸收自动化（Practice 沉淀，分期）」+ 文首别名注记 + 分期表 I1-I4 → P1-P4 |
| `docs/04-internal/backend-enhancement/I1-skill-distill-delivery.md` | 标题/§6 改 P1 / P 轴分期 + 文首别名注记（文件名保留）；§5/§7 测试数字行 0 触碰 |
| `docs/04-internal/absorption-candidates-2026-10.md` | 带日期分诊记录：**原值保留**，3 处逐行「历史口径（2026-10）」标注 + 文末命名注记 |
| `docs/04-internal/absorption-ledger.md` | 台账 B 行「I 轴 I1」原值保留 + 行内历史口径标注 |
| `docs/04-internal/backend-enhancement/quick-capture-delivery.md` | 带日期交付记录：「J1/I1 线」原值保留 + 行内历史口径标注（2 处） |
| `docs/03-reference/capabilities-matrix.md` | 「I1 线」→「P1 线（原 I1）」，链接文本改「P 轴分期」（路径不变） |
| `docs/02-guides/user-manual.md` | 「I 轴吸收自动化 I1」→「P 轴吸收自动化 P1（原 I 轴 / I1）」 |
| `docs/02-guides/frontend-data-contract.md` | §10.4 标题/状态改 P1（原 I1），链接文本改「P 轴分期」 |
| `crates/capabilities/tools/src/skill/mod.rs` | 模块注释改 P 轴 P1（首现带「原 I 轴/I1 + D13 指针」别名） |
| `crates/capabilities/tools/src/skill/card.rs`、`skill/distill.rs`、`tools/src/lib.rs` | 注释改「P 轴吸收自动化 P1」（纯注释，0 语义） |
| `crates/adapters/cli/src/lib.rs` | 旋钮注释改「P 轴吸收自动化 P1」 |
| `crates/engine/runtime-assembly/src/canonical/production.rs`、`tool_modules.rs` | 2 处注释/文档注释「I1 调用面」→「P1 调用面（原 I1）」+ 1 处装配注释改 P1 |

星港语境保留（不改）：`I-starport-builtins.md`、`J-life-integration.md`（「星港（I 轴）」「I 轴星港」= 星港线 I）、
`make-portable.ps1`（「I 想法 K / 星港批次推进」= 星港批语境）、frontend `starport/*` 的「I 批 1-5」。

## 4. F-02 大体量资产盘点 + 去留方案表（只出方案不动文件）

### 4.1 构成清单（2026-10-16 实测：PowerShell `Get-ChildItem` + `git ls-files`）

| 资产组 | 文件数（tracked） | 体积 | 构成 |
|---|---|---|---|
| `reports/` | **1071 tracked**（盘上 1079） | **39,894,060 B（38.0 MiB）** | 按类：`agent-*` 子代理报告 450 件 / 27.6MB；UUID 命名工作日志 ~150 件（含 `*-<role>-report.md`）；`decision-*` 136 件 / 1.85MB；`rNN-*` 轮次报告 124 件 / 1.2MB；`1.0-release-*` 21 件 / 0.59MB；`round*` 30 件 / 0.34MB；`achievement-*` 18 件；`dispatch-*` 4 件；非 md 资产 37 件（PNG 截图如 `00-full-canvas.png` 1.53MB、`.log/.html/.yml/.txt`）；子目录 7 个（`audit-2026-08-19` 11、`diagnostics-review` 11 件 2.99MB、`pet-real-smoke-2026-10-11` 3、`r18-snapshots` 7、`r20-stage-6-th` 3、`tp30` 10、`evidence-V10-qa2-A11-A20` 1）。按时代：**v1.0 时代（2026-07/08）797 件 / 32.5MB**、UUID 工作日志 83 件 / 0.62MB、无日期/其他 189 件 / 4.7MB、2026-09 4 件、2026-10 6 件 |
| `previews/` | **34 tracked**（全 PNG） | **40,937,344 B（39.0 MiB）** | `intro-*` 21 件 / ~23.3MB（intro-beat-01..09 + intro-live-* 9 张）；`mode-*` 7 件 / 5.7MB；`tauri-*` 3 件 / 5.8MB（`tauri-tray-menu.png` 3.91MB 最大）；`backend-*` 3 件 / 1.5MB；`day-bridge`/`night-bridge` 2 件 / 2.4MB；`rotation-proof.png` 0.36MB |
| `scripts/guard_ml/*.jsonl` | 4 大件（组内共 13 文件 tracked） | **21,997,011 B（21.0 MiB）** | `action_samples.jsonl` 9.64MB、`generated_features.jsonl` 7.95MB、`trace_samples.jsonl` 3.56MB、`pair_samples.jsonl` 0.85MB；另有 4 个 `.py` 脚本（65KB）、2 个 pair `.json` + `shadow_fixture.jsonl`（19KB）、`scenarios.jsonl`（4.9KB）+ `fixture_artifact.json`（1.2KB） |
| （旁注）`artifacts/` | 361 件 | **157.2MB** | 审计转储（`audit-before/after/final` 文本、`guard-*` 指标、batch 日志）——体量超上三组总和；审计 F-02 原文亦提及，**建议纳入同一处置批**（本批只登记） |

### 4.2 引用面与硬依赖（执行批前置，已查明）

- **编译期硬依赖（必留库）**：`crates/engine/guard/tests/security_scenarios.rs:3-4` 用
  `include_str!` 引用 `scripts/guard_ml/scenarios.jsonl` + `fixture_artifact.json`——移走即**测试编译失败**。
- **CI 重生成语义**：`.github/workflows/guard-v3.3.yml:30-95` 每次先跑
  `cargo run -p apeireth-guard --example export_guard_scenarios` **重生成** 4 大 jsonl → 训练/自检 →
  校验存在性与隐私不变量 → `upload-artifact` 上传 4 件。即库内 4 大 jsonl 是**可再生证据副本**，
  出库后 CI 不缺件（校验步在导出步之后）；`export_guard_scenarios.rs` / `export_v3.py` 是再生入口。
- **文档引用面**：`docs/` 内指向 `reports/` 的引用行 **711**（多数在 `docs/archive/1.0-release-prep` 等档案）；
  活跃文档面引用约 30 件具体报告（`docs/design/*` 的 avatar/benchmark 报告、`docs/04-internal/*` 的
  baseline/验收报告、README 草稿 2 处、`00-backlog.md:68`）。`previews/` 被 `docs/design/frontend-handoff.md:178`
  与 `intro-animation.md:25` 引用（实拍验收图）。
- **工作流引用**：`release-1.0.0.yml:382` 上传 `reports/r20-v1.0.0-release-checklist-*.md`；`release.yml:410`
  输出 `reports/tp20-s5-*.txt`；`pii-leak-detection.yml` / `release-prep.yml` / `doc-caliber.yml` 把
  `reports/**` 列为排除面（目录口径依赖）。
- **既有立场**：`docs/04-internal/gitignore-hygiene-audit-2026-08-28.md:139`「previews/ tracked 34 files，应保留（UI previews）」；
  `docs/04-internal/pm-review-productization-2026-09-25.md:50` 已点名「reports/ 里 1058 个 AI 工作日志被 git 跟踪」。

### 4.3 去留方案表（四选项逐项评估；**本批只出方案，0 文件移动**）

| # | 资产 | 迁 release 资产 | 迁 docs/archive/ | 外置 | git-lfs | **建议** |
|---|---|---|---|---|---|---|
| 1 | `reports/` · v1.0 时代批量工作日志（`agent-*` 450 + `decision-*` 136 + `rNN-*`/`round*`/UUID 日志等，约 950 件 / ~35MB） | ✅ 打包 tar.gz 挂 v1.0 归档 tag 的 Release 资产，git 历史本就可找回 | ❌ 只挪目录不减仓，且 `docs/archive/**` 等 711 处引用面改写成本高 | ✅（备选）独立归档库/对象存储，托管成本略高 | ⚠️（备选）保路径零断链、克隆体立减，但历史 blob 不减、贡献者需 LFS | **迁 release 资产**（首选，外置为备选）；执行批先跑引用审计、对被活跃文档引用件留原位或留 stub 索引 |
| 2 | `reports/` · 近期证据/验收报告（2026-09~10 + 被活跃文档引用件：avatar-*、`benchmark-reproduction.md`、`code-implementation-audit-2026-09-26.md`、`builtin-feature-synthesis-2026-10-11.md`、`pet-real-smoke-2026-10-11/`、`diagnostics-review/`、`disk-build-cache-advisory-acceptance-2026-10-16.md` 等 ~50 件 / ~5MB） | 不必 | 不必 | 不必 | 可选（若后续膨胀） | **留库**（去留 = 留）：体积可控、README 草稿/design/04-internal 活跃引用，出库即断链 |
| 3 | `reports/` · 非 md 资产（PNG 截图 + `.log/.html/.yml/.txt` 37 件 / ~2MB） | ✅ 证据附件挂 Release | ⚠️ 可（`docs/archive/evidence/`），但继续占仓 | ✅ 并入外置归档包 | ⚠️ 可 | **迁 release 资产**（与 #1 同批同包） |
| 4 | `previews/` · 34 PNG / 39MB（实拍验收图） | ⚠️（备选）挂 desktop Release，库内留 3-5 张代表图 | ❌ 不减仓 | ❌ `docs/design` 引用断链、且验收图需随库可见 | ✅ 路径不变零断链，fresh clone 立减 39MB | **git-lfs**（首选；与「应保留」既有立场兼容——仍 tracked 只是走 LFS）；次选 = 迁 release 资产 + 留代表图 |
| 5 | `scripts/guard_ml/` · 4 大 jsonl / 21MB（可再生导出件） | ✅（已随 CI artifact 每跑一版，可再挂 Release） | ❌ 位置错（训练数据不是文档档案） | ✅ 出库，按需由 `export_guard_scenarios` 再生或从 artifact 取回 | ✅（备选）保路径，但「可再生件」占 LFS 配额不划算 | **外置**（首选：移出 git 跟踪，CI 每次重生成 + artifact 兜底；执行批同步微调 `guard-v3.3.yml` 校验步的「存在性断言」时序说明与文档口径）；**必留库**：`scenarios.jsonl` + `fixture_artifact.json`（`include_str!` 编译期依赖）+ `counterfactual_pairs.json`/`negation_pairs.json`/`shadow_fixture.jsonl` + 4 个 `.py` 脚本 |
| 6 | （旁注）`artifacts/` · 361 件 / 157MB 审计转储 | ✅ | ❌ | ✅（首选同口径） | ⚠️ | **外置**，随同一处置批（本批仅登记，超任务范围） |

**执行批（owner 拍板后）最小动作清单**：① 引用审计（`git grep` 出 711 引用面中需改写/留 stub 的清单）；
② #1/#3 打包挂 Release、`docs/development/repository-layout.md:34-35` 与 `CONTRIBUTING` 目录口径同步；
③ #4 `git lfs migrate` 或逐件 `git lfs track 'previews/*.png'`（保路径）；④ #5 出库 4 大 jsonl +
`guard-v3.3.yml` 校验步措辞同步；⑤ 逐批跑 `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` 收口。

## 5. 校验与扫描记录

### 5.1 扫描口径与结果（现值面零命中）

> 口径定义：**现值/活面** = 根文件（ROADMAP / OSS_NOTICE / Cargo.toml）+ `00-backlog` +
> `docs/03-reference/` + `docs/02-guides/`；**历史卷宗** = CHANGELOG 日期条目 / commit-msg 档案 /
> `docs/archive/` / `library/` / `legacy/` / `reports/` / 带日期的 `docs/04-internal|01-architecture` 报告。
> 历史卷宗命中按纪律「如实登记」（§1.2/§1.3 清单），不计入零命中口径。

| 扫描 | 命令（可复核） | 结果 |
|---|---|---|
| ① 现值面守门级数旧口径未标注命中 | `git grep -nE '5 重守门\|6 重守门\|6 重 v[0-9]\|7 重 v[0-9]\|8 重 v[0-9]\|9 重守门\|9 重 v[0-9]\|重守门 v[0-9]' -- ROADMAP.md OSS_NOTICE.md Cargo.toml docs/04-internal/backend-enhancement/00-backlog.md docs/03-reference/ docs/02-guides/ \| grep -v '历史口径\|历史沿革\|现值\|口径注'` | **0 命中** ✅ |
| ② 吸收自动化旧名（I 轴 / I1-I4）未标注命中 | `git grep -nE 'I 轴\|I[1-4] 调用面\|I[1-4] 书\|I[1-4] 线\|I 轴吸收\|I 轴分期' -- . \| grep -v '原 I\|历史口径\|星港\|I 批\|I-starport\|命名注记'` | **0 命中** ✅（git 跟踪面口径；本报告自身是改名映射登记面，旧名以对照表形式存在，不计违规） |
| ③ 中性表述 | `pwsh -File scripts/check-neutral-terms.ps1`（新增行零第三方产品名/来源措辞） | ✅ 零命中（见 §5.3 复核） |
| ④ 测试口径行不被触碰 | `git diff --name-only HEAD`（test-baseline.md 不在改动面）+ 行级核对：改动行中仅 `absorption-ledger.md` 1 行含测试数字文本（「六套件 47 例」验收仪格**原样保留**，改的是同行机制名注记）；`I1-skill-distill-delivery.md` §5/§7 门禁数字、`quick-capture-delivery.md` 门禁表、ROADMAP §3 测试行全部原样 | ✅ 0 触碰 |

### 5.2 门禁（校验 `-j 2`，本批实测 2026-10-16，工具链 rustc/cargo 1.97.1 + MSVC 14.44.35207）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ **exit 0（0 failed）** |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ **exit 0（0 警告）** |
| 格式 | `cargo fmt --check` | ✅ **exit 0（0 diff）** |

> 数字口径：本批为文档 + 纯注释批，**0 新增测试**；按 test-baseline.md「只引用不抄写」纪律，
> 套件/例数不在本文件抄写——全量数字单一真源 = [test-baseline.md](../../03-reference/test-baseline.md)
> （本批 0 触碰）。测试运行含共树在途面（`L-experience-distillation.md` 他线插入，见 §6.4）。

### 5.3 纪律自检

- **0 装 PASS**：B-04/B-05/D13 报「已清」均有扫描 0 命中证据；F-02 报「方案已出、未执行」不假称已瘦身；
- **历史不追溯改写**：CHANGELOG / commit-msg 档案 / 审计卷宗 / 候选池 / 台账 / quick-capture 交付记录 /
  `Cargo.toml` metadata 值 / `library/v1.0/*` 原值全部保留，只加「历史口径（日期）」标注；
- **LOCKED 5 项 0 触碰**：9 哲学锚本体（含 `philosophy.md`、`eight_anchors.rs`）/ 13 键 / 3 不可变脊柱 /
  `workspace.version`（`Cargo.toml:51` 未动）/ R11 baseline 3 值（数值未动，仅加诚实标注）；
- **不 commit**：改动全部留在工作树，`git diff` 供 owner/整合线复核；
- **中性表述**：新增行零第三方产品名与「移植/借鉴」措辞（`check-neutral-terms.ps1` 只查新增行）。

## 6. 顺带发现（登记不代改，超出本批范围）

1. `Cargo.toml:62` `description` 仍写「18-crate 单一工作区」——与现行「workspace = 20 members（19 crates + bench harness）」口径不一（B-02 类残留；`description` 是对外元数据，建议随 B-02 收尾批修正）。
2. LOCKED 内部两口径：`philosophy.md` O-1 锚描述「9 重 v9」与 ROADMAP §13「7 重守门 v7」并存（§1.3 已登记）——两处都属思想层 LOCKED 面，**升/不升需 owner 拍板**，本批按任务口径统一现值到 7 重 v7 并如实登记差异。
3. `artifacts/` 157MB（§4.1 旁注）超 F-02 三组之和，建议并入同一处置批。
4. 共树在途改动：`L-experience-distillation.md` 有 +5 行「执行进度注记（L1/L2 批）」插入——**非本批改动**（他线施工），本批校验数字含该在途面，特此报备。

## 7. 后续（owner 拍板项）

| # | 项 | 建议 |
|---|---|---|
| 1 | F-02 执行批（§4.3 表 1/3/4/5/6 行） | 按「迁 release 资产 → git-lfs → 外置」顺序，先引用审计后动文件；D17 留账 |
| 2 | LOCKED 异口径拍板（§1.3 / §6.2） | 若认「守门级数以 v1.0 lineage 9 重 v9 为准」则反向修 ROADMAP §13；若认 7 重 v7（本批口径）则 philosophy.md O-1 描述的「9 重 v9」待哲学锚修订单处理 |
| 3 | `Cargo.toml` description「18-crate」残留（§6.1） | 随 B-02 收尾批一行修正 |
| 4 | `scripts/release/*` v1.0 发布脚本族「6 重守门 v7」留痕 | 随发布脚本归档批迁 `docs/archive/` 或加时代横幅 |

_本报告由「落差清账·口径尾账批」产出（审计修 B-04 / B-05 / D13 / F-02，2026-10-16）。四件对照表 §0；F-02 去留方案表 §4.3；扫描与门禁记录 §5。_

## 8. 附录：守门口径代码实证（2026-10-18 补测，供 §7 第 2 项拍板用）

§7 第 2 项（LOCKED 异口径：`philosophy.md` 「9 重 v9」vs `ROADMAP` §13「7 重 v7」）此前只有文档侧对照，缺代码侧实证。2026-10-18 补测如下，**结论：v2 实装既不是 7 也不是 9，而是「门值集合」——整数层数是 v1.0 时代 lineage 标签，不是 v2 的工程计数**。

### 8.1 复现命令与结果

| # | 命令（仓库根） | 结果 |
|---|---|---|
| A | `git grep -c -i "seven_fold\|evidence_guard\|action_rail" -- crates` | **0 命中**：v1.0 时代的 `seven_fold_guard` / `evidence_guard` / `action_rail` 三个守门承载 crate **均未迁入 v2** |
| B | `(Get-ChildItem crates/foundation/governance/src -Filter *.rs).Count` | **22** 个治理模块（含 `colang.rs`、`tool_guard.rs`、`evidence.rs`、`action_binding.rs`、`approval_closure.rs`、`rate_limit.rs`、`sandbox_ladder.rs`、`untrusted_mark.rs` 等） |
| C | `git grep -n "five gates in sequence" -- crates/foundation/governance/src/lib.rs` | `lib.rs:29`：「…This is the "five gates in sequence" arrangement from the nested prototype, **generalized so gates are values rather than…**」——v2 明确把门做成**门值**，不再做固定层数装配 |
| D | `Get-Content crates/foundation/governance/src/evidence.rs -TotalCount 5` | 文件头自述：「…**This is a library checker, not a ninth "fold" of the canonical pipeline.**」——v2 代码**显式否认**「第 9 重」这种层数叙事 |
| E | `Get-Content crates/foundation/governance/src/tool_guard.rs -TotalCount 5` | 「monotonic tool guard interface: a guard may refuse, never allow」——守门是**单调哨兵**（只拒不放），可装任意多个，层数无工程意义 |

### 8.2 三口径对照（拍板素材）

| 口径 | 出处 | 性质 | 与代码的关系 |
|---|---|---|---|
| **9 重 v9** | `docs/01-architecture/philosophy.md:10`（O-1 锚描述）、`Cargo.toml:148` `guard_gates_version` 值、`hard_walls` 字符串值 | v1.0 发布时代自描述 | v1.0 代码 lineage 的末值（v6→v7→v8→v9，含 action_rail / flow_executor / evidence_guard）；**v2 代码无这三个载体** |
| **7 重 v7** | `ROADMAP.md` §13「守门级数」行（声明为唯一现值出处） | 2026-10-16 口径尾账批统一后的**现值标签** | v2 代码有 `colang`（第 6 重对应物）与 `tool_guard`/`evidence`（第 7/9 重对应物），但**没有按 7 层装配**，也无编译期断言守 7 |
| **门值集合（实装）** | `crates/foundation/governance/src/*`（22 模块）、`ToolGuard` 单调接口 | **v2 真实工程事实** | 装配多少门由运行时决定（默认装配见 `crates/engine/runtime-assembly/src/canonical/`），可增可减，`-D warnings` 与测试守的是门的行为契约，不是层数 |

### 8.3 建议（给 owner 三选一）

- **建议 A（推荐，最小改动）**：现值口径改为「**守门 = `governance` 门值集合 + `ToolGuard` 单调哨兵；层数（7/9）仅为 v1.0 lineage 标签，v2 不作工程断言**」。落地物 = `ROADMAP.md` §13 该行加一句限定语（**LOCKED 面，需 owner 明示解锁**）+ `philosophy.md` O-1 保持 0 触碰（锚本体不写死数字的做法可留到哲学锚修订单）。
- **建议 B**：反向认 9 重 v9 为准 → 需在 v2 补 `evidence_guard` 等载体或改口径，**成本高且与 v2「门值」架构相悖**，不建议。
- **建议 C**：维持现状两口径并存（本报告 §1.3 已登记）→ 代价是每轮对外文案都要回避层数，**长期债**，不建议。

> 对外文案（README 门面 / 发布说明）本轮已按建议 A 的**工程侧**执行：不把「7 重 / 9 重」写成现行能力事实；层数只在历史沿革与 lineage 卷宗里出现。LOCKED 文本本批 0 触碰，等 owner 一字解锁。

_本节由 2026-10-18 口径补测批补入（只读代码取证 + 文档登记，0 触碰 LOCKED / 0 改 src）。_

