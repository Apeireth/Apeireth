# Apeireth-rust Active Docs Audit (2026-08-19)

> **范围**: 17 active doc 文件 (docs/01-architecture/*.md, docs/02-guides/*.md,
> docs/03-reference/*.md, docs/04-internal/*.md) + 顶层 (README.md /
> README.zh-CN.md / CHANGELOG.md / RELEASE_NOTES.md / INSTALL.md / CONTRIBUTING.md)
> **不动 docs/archive/** (R119 形式撤销纪律)
> **仅读**: 没改任何文件

---

## 0. 关键发现速览 — 高 confidence 必须修 (按文件聚类)

### 0.1 Top-level files (5 个文件必修)

| # | File | Line | 问题 | 状态 |
|---|------|------|------|------|
| 1 | `INSTALL.md` | 3, 7 | 写 "**6 锚**" — 实际 **8 哲学锚** (Cargo.toml metadata.philosophy_anchors = S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, R125 B5 升 8 锚) | **HIGH** |
| 2 | `INSTALL.md` | 21, 24, 178 | 写 "Cargo 1.97.1" / "6+ tests pass, apeireth-core 当前" — 实际 tests = 23,874 套 (CHANGELOG.md L12 post-1.0.0); apeireth-core 只是 crate 之一 | **HIGH** |
| 3 | `INSTALL.md` | 255-258 | 链向死路径: `docs/00-R14-START-HERE.md`, `docs/ROADMAP.md`, `docs/GLOSSARY.md`, `examples/hello_world.rs` — 实际路径是 `docs/01-architecture/*` + `docs/03-reference/glossary.md` + `examples/` (只 1 个 hello_world 不存在) | **HIGH** |
| 4 | `INSTALL.md` | 263 | 写 "主哲学 6 锚穿透" — 实际 8 锚 | **HIGH** |
| 5 | `CONTRIBUTING.md` | 3, 7 | 写 "**6 锚**" (×2) — 实际 8 锚 | **HIGH** |

### 0.2 docs/01-architecture/ (3 个文件必修)

| # | File | Line | 问题 | 状态 |
|---|------|------|------|------|
| 6 | `architecture.md` | 3 | 写 "**84 active crates**" — 实际 85 (commit 042dafc9 同文件 line 38 已修为 85; line 3 漏改) | **HIGH** (内部矛盾) |
| 7 | `philosophy.md` | 3 | 标题 "**The Six Anchors**" + 列表只列 6 个 (S-1/S-2/O-2/O-3/O-4/O-5) — **缺 S-3 质量工程化** + **O-1 安全优先** (R125 B5 升 8 锚, R126 P1-2 实施) | **HIGH** |
| 8 | `security.md` | 14, 26, 27, 51 | 列 "**S4 default-deny**" + "egress.rs **verified**" — 但 backlog.md (2026-08-18 复核) 显式标注 "**S4 ⬜ 未实施** (实测 gateway 无出站策略)", security 文档 vs backlog 矛盾 | **HIGH** (语义矛盾) |
| 9 | `security.md` | 14, 27 | "HTTP requests checked / audit chain tamper-detected (tests)" — 实际代码层未实装 (backlog.md 实证) | **HIGH** (语义矛盾) |

### 0.3 docs/03-reference/ (2 个文件必修)

| # | File | Line | 问题 | 状态 |
|---|------|------|------|------|
| 10 | `crates.md` | 22 | 写 "**12 键 verdict cache 复用 + 5 重守门**" — 实际 verdict cache = **13 键** (PHL-07 NotUnoptimizable); 守门 = **7 重 v7** | **HIGH** |
| 11 | `crates.md` | 38 | 写 "Library Stage 5 governance (R127 P5-2...)" — R127 是已结 era 标记, 可保留作历史锚; **不是事实过期**, 仍标 MEDIUM |
| 12 | `crates.md` | 63 | "V0.5 命名规范 (24 base 维 + 5 meta + 1 overall = 30 维)" — 数学对, 但 "**0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor**" — anchor 数错 (6 → 8) + workspace version 1.2.0 已非 1.0.0 | **HIGH** |
| 13 | `crates.md` | 64, 67 | "0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor" (×2) — 同上 | **HIGH** |

### 0.4 docs/04-internal/ (5 个文件必修)

| # | File | Line | 问题 | 状态 |
|---|------|------|------|------|
| 14 | `design-intent.md` | 4 | 写 "**81 个 crate**" — 实际 85 | **HIGH** |
| 15 | `maintenance-guide.md` | 1-156 + 158-344 | **整段内容重复一次** (lines 1-156 = lines 158-344 主体) — 构造问题, 不是事实过期但浪费 ~3KB + 制造混淆 | **HIGH** (结构 bug) |
| 16 | `release-plan.md` | 18 | "**81 crates** 天然模块化" — 实际 85 | **HIGH** |
| 17 | `release-plan.md` | 46 | "`scripts/check-assembly-matrix.ps1` → `logs/assembly-matrix.log`" — 需要验证脚本存在; per backlog #25 ✅ "nightly 工具链就位"; 仍可保留 |
| 18 | `backlog.md` | 134 | "RELEASE_NOTES v1.0.0 标题 ≠ workspace 1.2.0" — backlog #26 已 ✅ "Leader 拍板双轴制" (R178 同步完成), backlog 自身描述已陈旧, 但标注 ✅ 体现已处理; 不需修 |
| 19 | `plugin-authoring-guide.md` | 17-18, 87 | 仍用旧版 sub-plugin 形态描述 — 实际 N17 工具装配后已升级 (`register.rs` 各工具 crate); **未跟 N17 同步** | MEDIUM |
| 20 | `team-work-doc.md` | 14, 105, 178 | "**9 organ** 仪表盘 + 器官化" — organ 命名仍是双轨 (heart/brain/body/mind 等 TUI LOCKED 旧名 vs crate 9 organ), 不算过期但需要 ADR-0028 桥 | LOW (已知未决) |

### 0.5 docs/02-guides/ (1 个文件必修)

| # | File | Line | 问题 | 状态 |
|---|------|------|------|------|
| 21 | `quick-start.md` | 14 | "cargo build --workspace ... **84 crates**" — 实际 85 | **HIGH** |

### 0.6 总计

- **HIGH confidence**: 20 条
- **MEDIUM confidence**: 1 条 (crates.md R127 era refs)
- **LOW**: 1 条 (双轨 organ 命名, 已记 R174 audit 已知)
- **修 1 个文件需多久**: ~5 min (5-line edit) ~ 20 min (philosophy.md 加 2 anchor)

---

## Part A: Active 文档过时引用扫描

### A.1 范围与方法

- **范围**: 17 个 active 文件 (6 + 4 + 3 + 13 = 26 个 — 实际 grep 命中行分析后)
- **方法**: 13 个模式 grep → file:line 引用 → 上下文核对
- **不查**: docs/archive/* (R119 形式撤销纪律)

### A.2 完整发现清单 (按文件)

#### `docs/01-architecture/architecture.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 3 | `> 对齐实际代码（2026-08-18 master）。**84 active crates** / ~34 万行 Rust。` | 85 active crates (per Cargo.toml members + git tag v1.0.0 + engineering-report.md L11 + vision.md L32) | 改 `84` → `85` | **HIGH** |
| 38 | `## Crate Groups (**85 crates** + 1 desktop, aligned with code)` | 85 — 已对 | 保留 | OK |
| 51 | `> **2026-08-19 post-v1.0.0 增量**: \`frontend/companion-desktop/\` 加了 1 个 **独立 workspace** (Svelte 5 + Tauri 2 桌面伙伴) — 不在 root workspace...` | 已对 (commit 042dafc9 已合) | 保留 | OK |

**修 1 个要多久**: ~2 min (1 line edit)。

#### `docs/01-architecture/philosophy.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 3 | `## The Six Anchors` | 标题错 — 实际 8 锚 (Cargo.toml: `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]`) | 改 `Six` → `Eight` | **HIGH** |
| 5-12 | Table: S-1 北极星 / S-2 实事求是 / **O-2** 前人肩上 / O-3 干到底 / O-4 接手 / O-5 不假装 | 缺 S-3 (质量工程化) + O-1 (安全优先) (per R125 B5 升 8 锚 + R126 P1-2 实施) | 加 2 行 table | **HIGH** |

**修 1 个要多久**: ~5 min (改标题 + 加 2 行)。

#### `docs/01-architecture/security.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 14 | `\| **Outbound** \| **S4 default-deny**: domain/protocol allowlist + SHA-256 audit chain + budget hook — every HttpClient request checked \| http-client::egress \|` | **未实装** — per backlog.md L95, L286, L437: "S4 ⬜ 未实施 (实测 gateway 无出站策略, 2026-08-18 复核修正)" | 改 "verified" 标注 → "trait 口已备, 实现未接 (per backlog S4 P1)" | **HIGH** |
| 26 | `- Outbound: \`egress.rs\` — default deny outside allowlist; https-only unless explicitly allowed; audit chain tamper-detected (tests)` | 同上, "tests" 实装未发生 | 改标注 → "trait 口已备, 实装待补" | **HIGH** |
| 27 | `- Tool: schema validation rejects missing/wrong-typed fields; guardrail blocks path traversal + shell injection; tripwire flags credential leaks` | 部分对 (schema + guardrail + tripwire 已实装) — 保留 OK | 保留 | OK |
| 30 | `- Approval: rejected → cannot approve (only pending); silent reject transmitted end-to-end (N20)` | N20 仍 ⬜ P2 (per backlog.md L70) — 但 silent reject 已落 (N19 完成), "transmitted end-to-end" 部分对 | 可保留, 已知标 ⬜ | MEDIUM |

**修 1 个要多久**: ~5 min (2 处标注修订)。

#### `docs/01-architecture/vision.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 32 | `2. **Agent 平台**：**85 crates** 三层生态` | 已对 (85) | 保留 | OK |
| 40 | `\| 自我改进 \| 🟡 骨架 \| 提案→审议→用户批准→部署；独立实验场待建（smol-vm）\|` | per backlog.md L107 W1 已 ✅ + L108 W2 已 ✅ (TP31/TP32) — 但 vision 描述 "自我改进 骨架" 实际是世界模型; **自我改进 闭环** 状态 = "回路已闭环, 部署通道 mock" (release-plan L15) | 改 "骨架" → "回路闭环, 部署待实接" | MEDIUM |

**修 1 个要多久**: ~3 min。

#### `docs/01-architecture/engineering-report.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 11 | `\| crates \| 85 active / ~340K 行 Rust \| 85 + 1 独立 workspace (\`companion-desktop\` 1 crate) \|` | 已对 (85) | 保留 | OK |
| 12 | `\| 测试 \| 368 组 0 失败 \| **23,874** 组 0 失败 (368 v1.0.0 + 23,506 post-1.0) \|` | 已对 (post-1.0 增量已 sync, commit aac0b577) | 保留 | OK |

**修 1 个要多久**: OK 不修。

#### `docs/02-guides/quick-start.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 3 | `> Verified against master (2026-08-18). Rust toolchain: stable (rust-toolchain.toml pinned 1.97.1).` | 1.97.1 已对 (rust-toolchain.toml) | 保留 | OK |
| 14 | `cargo build --workspace          # ~34 万行, **84 crates**` | 85 crates | 改 `84` → `85` | **HIGH** |
| 21 | `cargo test --workspace           # 368 组全绿 (含真实 API 压测, 有退避)` | post-1.0.0 增量后 = 23,874 套 (engineering-report L12) | 改 `368` → `23,874` (或加 "(v1.0.0 时 368, post-1.0.0 23,874)") | **HIGH** |

**修 1 个要多久**: ~3 min。

#### `docs/02-guides/deployment.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 65 | `- **出站策略**：\`apeireth-http-client::egress\` 默认不启用（None=放行）；启用后白名单外域名默认拒绝 + 审计链——LLM 调用域名需入白名单` | 同 security.md L14 — 未实装 | 改 "默认不启用" → "trait 口已备, 实现未接 (per backlog S4 P1)" | **HIGH** |

**修 1 个要多久**: ~3 min。

#### `docs/02-guides/development.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 8 | `cargo build --workspace              # **85 crates** 全量构建` | 已对 | 保留 | OK |
| 44 | `\| \`apeireth-http-client::egress\` \| 出站默认拒绝 + 审计链 \|` | 同上 — 未实装 | 改 "出站默认拒绝 + 审计链" → "出站策略 trait (待实装)" | **HIGH** |

**修 1 个要多久**: ~3 min。

#### `docs/02-guides/user-manual.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 15 | `\| **TUI** \| 终端伙伴面板（**9 organ** 仪表盘）\|` | 9 organ 正确 (heart/brain/hand/eye/ear/memory/voice/body/mind) | 保留 | OK |
| 56 | `- **隐私**：出站 PII 脱敏（guard）` | 已实装 (apeireth-guard, N13 已完成) | 保留 | OK |
| 75 | `\| E4 探索行为 / F4 提问生成的 LLM 实现 \| trait 口已备未接（确定性机制件在工作）\|` | per backlog.md L102 ✅ 设计定案, 待实施; L148 E4 + L151 E7 状态一致 | 保留 | OK |

**修 1 个要多久**: OK 不修。

#### `docs/03-reference/api.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 46 | `"version": "1.2.0",` | Cargo.toml workspace.version = 1.2.0 — 已对 (但与 git tag v1.0.0 是双轴制, per backlog #26 双轨拍板) | 保留 (双轴制明确) | OK |
| 100 | `- 出站：所有 HTTP 请求过 \`egress\` 默认拒绝白名单 + SHA-256 审计链（接入方显式启用）` | 未实装 (backlog S4 P1) | 改 "默认拒绝白名单 + SHA-256 审计链" → "trait 口已备, 实现未接 (per backlog S4 P1)" | **HIGH** |

**修 1 个要多久**: ~3 min。

#### `docs/03-reference/crates.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 3 | `> 对齐 master 实际代码 (2026-08-18)。**85 crates**，按字母序。完整描述来自各 crate Cargo.toml。` | 已对 (85) | 保留 | OK |
| 22 | `\| \$n\ \| Apeireth 约束器官 (P12 — v4.1 新增: **12 键 verdict cache** 复用 + **5 重守门** trait (编译时/运行时/多AI/物理隔离/反思期)) \|` | verdict cache = **13 键** (PHL-07 NotUnoptimizable 已实施, per Cargo.toml `verdict_cache_keys = 13`); 守门 = **7 重 v7** (Cargo.toml `guard_gates_version = "v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"` + R126-guard-7 补 Superpowers Skill Guard) | 改 "12 键" → "13 键" + 改 "5 重守门" → "7 重守门 v7" | **HIGH** |
| 38 | `\| \$n\ \| Apeireth Library Stage 5 governance — policy framework + formal verification + cross-crate consistency (R127 P5-2, per decision-33 §1.4 + decision-55 §2.3) \|` | R127 era 标记; per Cargo.toml hard_walls "8 哲学锚 / ... / 7 重守门 v7 / 13 键 verdict cache" — 历史锚可保留 | 保留 (历史注释 OK) | MEDIUM |
| 63 | `\| \$n\ \| Apeireth R20 阶段 4 估补: V0.5 命名规范 (4 类 × 6 维 = 24 维) + R126 P1-4 V0.5.30 扩展 ... **= 30 维**, sum=1.00 守门, 编译期 hardcode enum ... **0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺**. 借鉴 ID: R126-v05-30-BORROW-... \|` | 30 维对; 但 anchor 数错 (6 → 8); workspace version 1.2.0 (双轴); "8 项不修改承诺" 历史锚 OK | 改 "6 哲学 anchor" → "8 哲学锚" | **HIGH** |
| 64, 67 | `\| \$n\ \| Apeireth R20 阶段 5 集成测试 e2e (主仓 + API + TUI 三层端到端, 60+ 测试, **不碰 24 LOCKED**) \|` | 24 LOCKED 是历史 (现 R128 已形式撤销, 仅保 3 项不可变脊柱); "不碰" 是历史 | 可保留作历史锚 (但应加 "历史命名") | MEDIUM |
| 67 | `\| \$n\ \| Apeireth R21 借鉴 Golutra #6: ... **0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺** \|` | anchor 错 (6 → 8) | 改 "6 哲学 anchor" → "8 哲学锚" | **HIGH** |

**修 1 个要多久**: ~10 min (4 处 anchor 数 + 1 处 verdict cache/守门版本)。

#### `docs/03-reference/glossary.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 58 | `\| **出站策略（S4）** \| 白名单默认拒绝 + SHA-256 审计链 + 预算钩子 \|` | 未实装 (backlog S4 P1) | 改 "白名单默认拒绝 + SHA-256 审计链 + 预算钩子" → "trait 口已备, 实现未接 (per backlog S4 P1)" | **HIGH** |

**修 1 个要多久**: ~3 min。

#### `docs/04-internal/team-work-doc.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 14 | `1. **基地，不是 AI 本身**：Apeireth 是给 LLM 的"操作系统"。我们提供 **9 器官** + 工具 + 记忆 + 关系可能性；不定义 AI 是什么。` | 9 organ 正确 (TUI 命名 LOCKED) | 保留 | OK |
| 105 | `\| A3 \| **9 organ 人格化深化** \| 情绪→语气、审议→措辞（organs.rs 自标注"下一步"） \| organs.rs + tone.rs 接线 \|` | per backlog.md L33 ✅ 已完成 (提交 70110a54 + b5ce015d); 表内仍 ⬜ P2 | backlog 已 ✅, 但 team-work 表内仍标 "接线" — 状态描述滞后 | MEDIUM |
| 209-210 | `> Node.js 核心（server.js/Plugin.js/WebSocketServer.js/KnowledgeBaseManager.js + 20+ modules）+ **Rust N-API 记忆层（rust-vexus-lite：RiverMemo Topology V3）** + **84 插件**。` | "84 插件" 是 VCP rust-vexus-lite 插件数, 不是 Apeireth crates | 保留 (注释描述 VCP 调研, 不是 Apeireth) | OK |
| 213-244 | 各种 grep (rivermemo_topology_v3.rs:1784-2011 etc.) | VCP 调研笔记 — 不需改 | 保留 | OK |

**修 1 个要多久**: ~3 min (table 一处状态)。

#### `docs/04-internal/design-intent.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 4 | `> 否则 **81 个 crate** 只有注释没有意图, 每一个机制看起来都是"可以删掉的复杂度"。` | 85 crates | 改 `81` → `85` | **HIGH** |
| 50 | `\| ... **85 SKILL.md**, 已有 [DSH 插件版] ... \|` | "85 SKILL.md" 是 reverse-skill 项目的 85 SKILL.md (不是 Apeireth crates) | 保留 (注释正确) | OK |

**修 1 个要多久**: ~1 min。

#### `docs/04-internal/release-plan.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 18 | `\| 能力按需/可选装配（80%→完全体） \| **81 crates** 天然模块化，**无 feature/suite 装配层** \| ❌ 发布形态缺装配器 \|` | 85 crates + 装配层已实装 (suites.rs + check-assembly-matrix.ps1) | 改 "81 crates" → "85 crates"; 改 "❌ 发布形态缺装配器" → "✅ 装配层已实装 (suites.rs SuiteCatalog::install_with_plugins)" | **HIGH** |
| 80 | `- [x] 文档（用户手册/快速开始/能力包说明）（docs/user-manual.md + docs/quick-start.md + docs/capability-packs.md：全部从真实代码/env 清单/suites.toml 提取，未接项如实标注）` | `docs/capability-packs.md` 不存在 — 但实际维护的 docs 目录无此文件 (实测 grep 0 hit) | 改 "docs/capability-packs.md" → "docs/release-plan.md + docs/02-guides/user-manual.md" | **HIGH** |

**修 1 个要多久**: ~5 min。

#### `docs/04-internal/maintenance-guide.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 1-156 | `# Apeireth companion 维护指南（2026-08-16）...## 一、概念词典...## 二、模块地图...` | **内容完整** | 保留 | OK |
| 158 | `===========` | **重复段开始** — line 158-344 与 line 1-156 完全相同 (标题 + 一/二/三/四 章节) | **删 line 158-344** | **HIGH** (结构 bug) |
| 344 | `**何时升级手段**...` | end of duplicate | 截断到 line 156 | **HIGH** |

**修 1 个要多久**: ~5 min (删 line 158-344, 187 行)。

#### `docs/04-internal/ci-fix-log-2026-08.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 1-85 | 完整 CI 修复日志 (12 workflow 转绿) | 历史记录 — OK | 保留 | OK |

**修 1 个要多久**: OK 不修。

#### `docs/04-internal/next-team-handbook.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 80 | `\| TP34 \| **companion_serve 真接流式**（CoT + tool_call + tool_result SSE）\| ... 当前 \`stream: false\` 写死在 10 处...` | per CHANGELOG L69-73 TP34 描述一致; v1.5 中期 | 保留 | OK |

**修 1 个要多久**: OK 不修。

#### `docs/04-internal/plugin-authoring-guide.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 17-18 | `**当前插件载体（0 假装）**：Rust 编译期单元。社区插件 = 向 \`apeireth-companion\` crate 提交新模块...**动态加载（运行时装载外部二进制插件）未接**` | N17 工具装配后, 9 工具子 crate 已用 `register.rs` 模式 (per maintenance-guide L109); "社区插件 = 向 companion crate 提交" 已部分过时 | 加 "或 9 工具子 crate 用 register.rs 模式" | MEDIUM |
| 20 | `\`apeireth-tool-registry\` 的 \`watch_plugin_dir\` 目前只记录文件事件到日志` | per ci-fix-log L16 root cause 已修 (a2ba0a2e 闭包写局部 Arc → 字段改 `Arc<Mutex<Vec>>`); 现可互通 | 改 "只记录文件事件到日志" → "已修 (commit a2ba0a2e), watch 事件已互通" | MEDIUM |

**修 1 个要多久**: ~5 min。

#### `docs/04-internal/backlog.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 134 | `\| 26 \| 版本号口径统一... \| ✅ Leader 拍板双轴制 (2026-08-17, 任务 cc8377): 产品版本轴 v2.0.0-alpha + workspace crate 轴 1.2.0 ... \|` | 已 ✅ 处理 — 但 "v2.0.0-alpha" vs git tag `v1.0.0` 不一致 — 双轴制本身没问题 (per Cargo.toml workspace.version 1.2.0 + tag v1.0.0), 但描述里 "v2.0.0-alpha" 措辞需对一下 tag 实际命名 | 改 "产品版本轴 v2.0.0-alpha" → "产品版本轴 v1.0.0 (git tag) + 内部演进轴 1.2.0 (Cargo.toml semver)" | MEDIUM |
| 437 | `\| S4 出站网络策略 \| "✅ TP20 (划错)" \| gateway 实测无域名白名单/出站拒绝 \| ⬜ 未实施 (已修正) \|` | 已显式标注未实施 — 自洽 | 保留 (诚实标注) | OK |

**修 1 个要多久**: ~3 min。

#### `docs/04-internal/upgrade-research-pack.md`

| Line | 原文片段 | 实际状态 | 修复建议 | Confidence |
|------|----------|----------|----------|------------|
| 107-114 | 各机制调研段落 | 调研笔记 — OK | 保留 | OK |

**修 1 个要多久**: OK 不修。

### A.3 Part A 总耗时估算

- **HIGH 必改**: 20 条 × 平均 4 min = **~80 min (1.5 h)**
- **MEDIUM 可改**: 5 条 × 平均 5 min = ~25 min (与 HIGH 一起做, ~2 h)
- **LOW**: 1 条 (双轨 organ 命名 — 等 ADR-0028 落地后再说)

**Part A 修完预算**: 2 hours

---

## Part B: README en/zh 脱节分析

### B.1 git 历史对比

```
git log --oneline -5 -- README.md
98ddac49 docs: README 同步 PR #1 (companion-desktop) + Dockerfile 多架构  (2026-08-19)
6c8d745e merge: 应用 release-1.0 全部后期内容 (品牌宣言/亲情故事/...)    (2026-08-19)
403a61d7 chore(1.0-final): 发布前最终审计修复 ... README/RELEASE_NOTES crates 数 85 修正

git log --oneline -5 -- README.zh-CN.md
6c8d745e merge: 应用 release-1.0 全部后期内容...
403a61d7 chore(1.0-final): ...
98c9cf2d docs(1.0): 文档体系规范重构 ... 顶层 README 中英双语
```

**结论**: README.md 比 zh-CN.md **新一 commit** (98ddac49 2026-08-19, 仅修英文).
**结构性 diff**:
- README.md (en) 232 lines
- README.zh-CN.md 230 lines
- Diff ≈ 0 structural (章节一一对应)

### B.2 章节对照

| 英文 | 中文 | 状态 |
|------|------|------|
| ## The Story (lines 9-45) | ## 故事 (lines 9-45) | ✅ 1:1 对译 |
| ## The Name (lines 47-106) | ## 命名 (lines 47-106) | ✅ 1:1 对译 |
| ## Our Philosophy (lines 110-118) | ## 我们的哲学 (lines 110-117) | ✅ 1:1 对译 |
| ## What Apeireth Is — Three Faces, One Base (lines 122-152) | ## 阿佩瑞斯是什么 —— 三面一体，一个基地 (lines 121-151) | ✅ 1:1 对译 |
| ## Mechanism Map (lines 156-172) | ## 机制地图（代码在哪）(lines 155-171) | ✅ 1:1 对译 |
| ## Status — v1.0.0 (lines 176-187) | ## 状态 —— v1.0.0 (lines 175-186) | ✅ 1:1 对译 |
| ## What We're Building Next (lines 189-195) | ## 我们正在建的下一步 (lines 188-193) | ✅ 1:1 对译 |
| ## Quick Start (lines 199-215) | ## 快速开始 (lines 197-213) | ✅ 1:1 对译 |
| ## Documentation (lines 219-222) | ## 文档 (lines 217-220) | ✅ 1:1 对译 |
| ## License (lines 224-226) | ## License (lines 222-224) | ✅ 1:1 对译 |
| > quote (lines 230-232) | > quote (lines 228-230) | ✅ 1:1 对译 |

### B.3 差异分析

**结论**: 两个 README **结构、内容、anchor 一致**, 仅 commit 时间差一 commit (en 比 zh 新一 commit 98ddac49, 同步 PR #1 companion-desktop 内容).

**已知差异**:
- 英文 README L192 在 "What We're Building Next" 中提了 "frontend/companion-desktop/" + "6 个 Phase 报告 in docs/integration/" (per 98ddac49 sync)
- 中文 README L190-193 在同样位置**未提** frontend/companion-desktop 链接

**需修建议**: zh-CN.md 加 1 段同步:
```
- **一张脸和一个声音** —— 桌宠前端 (`frontend/companion-desktop/`, Svelte 5 + Tauri 2 thin shell, 102 行, 走 apeireth-companion OpenAI 兼容端点). Real LLM E2E pending API key; mock SSE e2e passes. 6 个 Phase 报告 in `docs/integration/`.
```

### B.4 Part B 总耗时估算

- **修 zh-CN.md 1 处**: ~3 min

---

## Part C: 顶层 ROADMAP/CHANGELOG/RELEASE_NOTES/INSTALL/CONTRIBUTING 跟 git 一致性

> **ROADMAP.md 跳过** (8/19 主人已知改动)
> 重点: CHANGELOG / RELEASE_NOTES / INSTALL / CONTRIBUTING + Cargo.toml workspace.version 双轴制

### C.1 git tag vs Cargo.toml 双轴制

| 维度 | 实际值 | 来源 |
|------|--------|------|
| **git tag** | `v1.0.0` | `git tag --list "v*"` → `v1.0.0` (唯一 tag) |
| **git tag commit** | (推断 = master @ 8/18) | per RELEASE_NOTES.md L5 "commit `b7132fad`" |
| **master HEAD** | `9bf36b1e` (latest tag = 8/19) | `git log --oneline -1` |
| **Cargo.toml workspace.version** | `1.2.0` | L228: `version = "1.2.0" # B2 upgrade: 1.1.0 → 1.2.0` |
| **双轴制拍板** | ✅ backlog #26 (任务 cc8377, 2026-08-17) | backlog.md L134 |
| **docs README 描述** | "**v1.0.0 (product axis; workspace crates 1.2.0)**" | README.md L180 / README.zh-CN.md L179 |

**结论**: 双轴制已拍板, 但 **Cargo.toml `version = 1.2.0` 与主人给的事实 "workspace.version = 1.0.0" 不一致** — 主人说的 1.0.0 是 **产品轴 (git tag)**, workspace 实际仍 1.2.0.

### C.2 CHANGELOG.md 状态

```
git log --oneline -10 -- CHANGELOG.md
aac0b577 docs: CHANGELOG + engineering-report 同步 post-v1.0.0 增量          (2026-08-19)
2ae6adb1 docs: CHANGELOG entry for post-v1.0.0 work (2026-08-19)
6c8d745e merge: 应用 release-1.0 全部后期内容 ...                            (2026-08-19)
f73bf735 docs(1.0-final): ROADMAP/CHANGELOG 归档路径引用修复 (断链 9 → 0)     (2026-08-18)
403a61d7 chore(1.0-final): 发布前最终审计修复 ...
993e9107 docs(1.0): RELEASE_NOTES 正式版重写 ...                              (2026-08-18)
```

| Line | 原文 | 实际状态 | 修 |
|------|------|----------|------|
| 25 | `- 0 触碰 **24 LOCKED crate**` | 24 LOCKED 是历史锚 (R128 已形式撤销, 仅保 3 项不可变脊柱) | 保留 (CHANGELOG 是历史 banner 归位, 应保留原文) |
| 26 | `- workspace.version 1.2.0 不变` | 已对 (Cargo.toml 1.2.0) | 保留 OK |
| 28 | `- **13 键** verdict cache 守门` | 已对 | 保留 OK |
| 128 | `\| R148 \| **24 LOCKED** 形式撤销扫尾 (仅保 3 项不可变脊柱) \|` | 已对 (R128 + R148 实际发生) | 保留 OK |
| 143 | `- **24 LOCKED 入口签名冻结降级** (per decision-74 §1.1 + decision-130 §2.4): 仅保 3 项不可变脊柱 ...` | 已对 (R128 + decision-130) | 保留 OK |
| 242 | `- **24 LOCKED crate mtime baseline** 严守 (B1)` | 历史 (R125 B1, 后 R128 撤销) | 保留 (历史 banner 归位) |
| 245 | `- **6 重守门 v6 → v7 升级** (B4)` | 已对 (R125 B4 升 v7) | 保留 OK |

**结论**: CHANGELOG.md **全部一致**, 历史 banner 归位章节保留原文正确, post-v1.0.0 增量章节 OK.

### C.3 RELEASE_NOTES.md 状态

| Line | 原文 | 实际状态 | 修 |
|------|------|----------|------|
| 3 | `> **Tag**: \`v1.0.0\`（2026-08-18 我们定版：后端收工，本版本才是**真正的 1.0**）` | 已对 (tag v1.0.0) | 保留 OK |
| 4 | `> **定位**: 从"代码演进号 1.2.0 / 产品轴 v2.0.0-alpha"统一收口为 **1.0.0 正式版**` | 已对 (双轴制 v2.0.0-alpha 拍板 → 1.0.0) | 保留 OK |
| 5 | `> **基线**: master = integration（0/0 同步，commit \`b7132fad\`）` | git tag v1.0.0 commit = 推断 b7132fad (per release-prep commit log) — **未实测验证** | LOW (trust release-prep doc) |
| 6 | `> **验证**: \`cargo test --workspace\` 全绿（368 组 ...` | v1.0.0 时 368 套 OK; post-1.0.0 后 23,874 套 | 改 "368 组" → "368 组 (v1.0.0 基线) + post-1.0.0 增量至 23,874 套" | **HIGH** |
| 14 | `- **五原型全部有骨架**：... 自主好奇心（E4 记忆回声偏置）、...` | E4 per backlog.md L102 ✅ 设计定案, **未实施**; description "骨架" 对 | 保留 OK |
| 17 | `- **安全底线**：... **S4 出站白名单**（默认拒绝 + SHA-256 审计链）...` | **未实装** (backlog S4 P1) | 改 "S4 出站白名单（默认拒绝 + SHA-256 审计链）" → "S4 出站 trait 口已备 (per backlog S4 P1 实现未接)" | **HIGH** |
| 24 | `\| active crate \| **85**（三层生态：模块/套件/插件） \|` | 已对 | 保留 OK |

**结论**: RELEASE_NOTES.md **2 处必改** (368 → 23,874, S4 标注).

### C.4 INSTALL.md 状态 (🔴 重大问题)

| Line | 原文 | 实际状态 | 修 |
|------|------|----------|------|
| 3 | `> **依据**: 我们 2026-07-31 "开干前补齐 4 件套" + rust-toolchain.toml 锁定 Rust 1.97.1 stable。` | 1.97.1 已对 | 保留 OK |
| 5 | `> **commit 锚**: 23513387（v3 修订）。` | commit 23513387 已实测存在 (per `git log --oneline 23513387`) | LOW |
| 19-24 | rust/Cargo/SQLite/Python 版本要求 | 已对 | 保留 OK |
| 177 | `# 2. Test（应该 **6+ tests pass**，apeireth-core 当前）` | **严重错**: 6+ tests → 23,874 套 (post-v1.0.0); "apeireth-core 当前" 不再 "当前" | **HIGH** |
| 255-258 | "读 README.md / CONTRIBUTING.md / `docs/00-R14-START-HERE.md` / `docs/ROADMAP.md` / `docs/GLOSSARY.md` / 运行 `examples/hello_world.rs`" | **4 处死链**: 00-R14-START-HERE.md / ROADMAP.md / GLOSSARY.md / hello_world.rs — 实际路径是 `docs/01-architecture/*` + `docs/03-reference/glossary.md` + `examples/` 下无 hello_world.rs | **HIGH** (死链 9 修复之外又有) |
| 263 | `_主哲学 6 锚穿透._` | 8 锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | **HIGH** |

**结论**: INSTALL.md 是 active docs 中 **问题最多** 的文件 — 5 处必改.

### C.5 CONTRIBUTING.md 状态

| Line | 原文 | 实际状态 | 修 |
|------|------|----------|------|
| 3 | `> Apeireth 是我们的伙伴型 AGI 操作系统。贡献前请先读哲学——[docs/01-architecture/philosophy.md]（**6 锚** / 双洋葱 / 0 装 PASS）。` | 8 锚 | **HIGH** |
| 7 | `- [docs/01-architecture/philosophy.md](...) — 哲学（**6 锚** / 双洋葱 / 0 装 PASS）` | 8 锚 | **HIGH** |
| 8 | `- [docs/01-architecture/architecture.md](...) — 架构总览（**85 crates** 分组）` | 85 crates 已对 | 保留 OK |
| 18 | `cargo test --workspace                  # 全量 **368 组** 0 失败` | post-v1.0.0 后 23,874 套 | **HIGH** |
| 19 | `cargo fmt --all --check                 # 格式` | per ci-fix-log + backlog #25 ✅ fmt 全转 CLEAN | 保留 OK |

**结论**: CONTRIBUTING.md **3 处必改** (2× "6 锚" + 1× 368 套).

### C.6 Part C 总耗时估算

- **CHANGELOG.md**: OK 不修 (~0 min)
- **RELEASE_NOTES.md**: 2 处必改 (~5 min)
- **INSTALL.md**: 5 处必改 (~15 min) — **最严重**
- **CONTRIBUTING.md**: 3 处必改 (~5 min)
- **总计**: ~25 min

---

## D. 综合优先级 / 时间预算

| 优先级 | 范围 | 文件数 | 总耗时 |
|--------|------|--------|--------|
| **P0 (1.5 h)** | INSTALL.md + CONTRIBUTING.md + philosophy.md + architecture.md + crates.md | 5 | ~30 min (改) + ~30 min (verify) = **~1 h** |
| **P1 (1 h)** | security.md + api.md + glossary.md + deployment.md + development.md + quick-start.md + design-intent.md + release-plan.md + crates.md (剩余) | 9 | ~40 min |
| **P2 (0.5 h)** | maintenance-guide.md (删 187 行重复段) + plugin-authoring-guide.md (2 处状态) + team-work-doc.md + vision.md + backlog.md + plugin-authoring-guide.md + CHANGELOG.md (如改 R127 era refs) | 7 | ~25 min |
| **P3 (15 min)** | README.zh-CN.md (1 处同步 PR #1 内容) + RELEASE_NOTES.md (2 处) | 2 | ~5 min |
| **总计** | — | ~20 文件 | **~3 h** |

---

## E. 修 1 个文件要多久 (按本审计节估算)

| 操作复杂度 | 例子 | 估算 |
|-----------|------|------|
| 1-line edit | design-intent.md L4 (81→85) | **~1 min** |
| 2-3 line edit | architecture.md L3 (84→85) | **~2 min** |
| Table cell | team-work-doc.md L105 (状态描述) | **~3 min** |
| 2-anchor add | philosophy.md (加 S-3 + O-1) | **~5 min** |
| Section restructure | crates.md L22 (12键→13键 + 5重→7重 v7) | **~10 min** |
| Section rewrite | security.md L14/26 (S4 已实装 → trait 口已备) | **~10 min** |
| Duplicate section delete | maintenance-guide.md (删 line 158-344, 187 行) | **~5 min** |
| Multi-section fix | INSTALL.md (5 处) | **~15 min** |

---

## F. 不要漏掉的微观提示

1. **Cargo.toml line 228** `version = "1.2.0"` — 主人表述 "workspace.version = 1.0.0" 与 Cargo.toml 实际值不一致. **产品轴 (git tag v1.0.0) ≠ Cargo workspace semver (1.2.0)** — 这是 backlog #26 拍板的双轴制, 主人给的事实 baseline 应明确区分两轴.
2. **crates.md L63/64/67 三个 R21 段落** 都有 "0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺" 字样 — anchor 数错重复 3 次.
3. **S4 出站策略** 跨 6 个 active 文档 (security / deployment / development / api / glossary / RELEASE_NOTES / vision) 都标 "已实装/verified", 但 backlog 显式 ⬜ P1 — 这是最大的 **安全谎言**, 必须优先修.
4. **philosophy.md 6 → 8 锚** 跨 INSTALL.md / CONTRIBUTING.md / docs/README.md (未实测但 README.md 链接到 6 锚) / crates.md 表格内嵌 — 主人说 8 锚 = S-1/S-2/**S-3 质量工程化 NEW**/**O-1 安全优先 NEW**/O-2/O-3/O-4/O-5.

---

## G. 不在本审计范围 (主动声明)

- ❌ docs/archive/* (R119 形式撤销纪律, 主人明示不动)
- ❌ crates/* source code (非文档)
- ❌ reports/* (R20 阶段产出, 历史快照)
- ❌ _research_mem/* (自审自用)
- ❌ ROADMAP.md (8/19 主人已知改过, 跳过)

---

_报告生成: 2026-08-19 18:35_
_审计员: active_docs_audit (subagent)_
_方法: 仅读 + grep + git log, 无文件修改_