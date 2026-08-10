# Locked Audit — 9 项实质 + 24 LOCKED 名单 + 大胆更新建议

**Date**: 2026-08-10 16:35
**Author**: Mavis (root session, 主人 16:27 "为了升级或更好, 要改动现有的 locked, 不必犹豫" 明确授权)
**关联决策**: `decision-21-upgrade-roadmap-2026-08-10.md` + `upgrade-roadmap-post-r124-2026-08-10.md`
**状态**: 🟡 Audit 完成, Mavis 大胆提建议, 待主人拍板 (0 主动 commit, 0 越界)

---

## 0. 触发事件

**主人 16:27 拍板**:
> "然后就是有关locked, 你现在遵守的给我发一下我看看有没有需要更新的, 如果为了升级或更好, 要改动现有的locked, 不必犹豫, 完全可以, 因为locked也是过去制定的, 会逐渐过时"

**主人 8/10 01:14 拍板** (per 10-locked.md, R119 重建):
> "locked 全部解锁, 我们只是要求原意不变"
> "原意不变, 但关于不能改变的原意得变一下了, 不再要"
> "按你建议来, 真正理解项目, 核验实际"
> "朝最整齐的方向走"

**两次拍板一致**: 主人 14:42 + 16:27 两次授权 — **locked 形式可重整 + 实质可为了升级更新**.

---

## 1. 当前 9 项实质 Locked (R119 后仍严守, per 10-locked.md)

### 1.1 全清单

| # | 实质 Locked | 当前值 | 何时定 | 来源文件 | 主人 16:27 态度 |
|---|---|---|---|---|---|
| **1** | **24 LOCKED crate mtime baseline** | 16:34 之前 (2026-08-05) | R20 阶段 6 (8/5 22:13 收口) | `8-promise-audit.md` + `1.0.0-release-report §6` | 🟢 可重整名单 |
| **2** | **workspace.version** | **1.1.0** (semver 严守) | R38 (a64fe197, 8/5 1.0→1.1 升级) | `Cargo.toml:1-30` | 🟢 可更新 |
| **3** | **R11 baseline 3 值** | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 | R11 (历史快照) | `integration_r_measure.rs:42-44` | 🔒 数字严守 |
| **4** | **V0.5 24 维公式** | sum=1.00 守门, 编译期 hardcode enum | R11 | `apeireth-asi/src/lib.rs:V05_DIM_COUNT=24` | 🟢 可扩展 (R125-10) |
| **5** | **12 键 verdict cache** | V3 9 键 + v4.1 3 键 (编译期 hardcode) | R11 + v4.1 | `glossary/07-12-keys-verdict-cache.md` | 🟢 可加键 |
| **6** | **5 重守门 (v5 修正)** | 4 重嵌套 + 权限发放独立机制 | v5 修正 (2026-07-31) | `glossary/17-4-gates-permission.md` | 🟢 可加第 5 (R125-5) |
| **7** | **6 哲学锚穿透** | S-1/S-2 + O-2/O-3/O-4/O-5 | R11 | `conventions/09-anchor.md` | 🟢 可加 S-3 / O-1 (8 锚) |
| **8** | **双洋葱架构** | 原则洋葱嵌入权限洋葱 | R11 | `onion-wall-architecture-2026-07-31.md` | 🟢 可加第 3 洋葱 (R125-5) |
| **9** | **9 器官代码** | body/brain/ear/eye/hand/heart/memory/mind/voice | R11 | `apeireth-tui/src/organ/*.rs` (9 + mod.rs) | 🟢 器官内 fn 可借 R125-12 |

### 1.2 8 项不修改承诺 (R119 后状态, per 10-locked.md §24-35)

| # | 原 8 项 (R20 阶段 6) | R119 后 | 原意保留 |
|---|---|---|---|
| 1 | 阶段 1+2+3 LOCKED 文档 | 🟢 形式撤销 | ✅ 内容严守 (`docs/omnibus/stage1-3/`) |
| 2 | v2 / v4 / v4.1 LOCKED | 🟢 形式撤销 | ✅ 内容严守 (`docs/omnibus/design-v*/`) |
| 3 | 阶段 4 核心 LOCKED (`6ca80776`) | 🟢 形式撤销 | ✅ 内容严守 (`docs/omnibus/stage4/`) |
| 4 | 阶段 5 施工 LOCKED (631 行) | 🟢 形式撤销 | ✅ 内容严守 (`docs/omnibus/stage5/`) |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层) | 🟢 形式撤销 | ✅ 内容严守 |
| 6 | R11 baseline 3 值 | 🔒 严守 | 🔒 数字 0 动 |
| 7 | 顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) | 🟢 形式撤销 | ✅ 已下沉 `docs/conventions/` / `versioning/` / `glossary/` |
| 8 | workspace.version 1.1.0 | 🔒 严守 | 🔒 数据不动 |

**关键发现**: R119 后, 1-5 + 7 已"形式撤销" (可改文件名/位置/章节/引用/命名), **仅 6 (R11 baseline 数字) + 8 (workspace.version 1.1.0) 实质严守**. 但 9 项实质 (per §1.1) 仍是 R125 阶段 0 触碰的 baseline — 形式可重整, 实质 0 改数字.

---

## 2. 24 LOCKED crate 名单审计 (Mavis 大胆质疑)

### 2.1 名单来源 (per `8-promise-audit.md` §3.4 + `1.0.0-release-report.md` §6.1)

| # | crate | 路径 | 来源 |
|---:|---|---|---|
| 1 | apeireth-supervisor | `crates/apeireth-supervisor/src/lib.rs` | §3.1 实查 |
| 2 | apeireth-agent | `crates/apeireth-agent/src/lib.rs` | §3.1 实查 |
| 3 | apeireth-bus | `crates/apeireth-bus/src/lib.rs` | §3.1 实查 |
| 4 | apeireth-council | `crates/apeireth-council/src/lib.rs` | §3.1 实查 |
| 5 | apeireth-evolution | `crates/apeireth-evolution/src/lib.rs` | §3.1 实查 |
| 6 | apeireth-extension | `crates/apeireth-extension/src/lib.rs` | §3.1 实查 |
| 7 | apeireth-graph | `crates/apeireth-graph/src/lib.rs` | §3.1 实查 |
| 8 | apeireth-mcp | `crates/apeireth-mcp/src/lib.rs` | §3.1 实查 |
| 9 | apeireth-pipeline | `crates/apeireth-pipeline/src/lib.rs` | §3.1 实查 |
| 10 | apeireth-tool-registry | `crates/apeireth-tool-registry/src/lib.rs` | §3.1 实查 |
| 11 | apeireth-tool-runtime | `crates/apeireth-tool-runtime/src/lib.rs` | §3.1 实查 |
| 12 | apeireth-protocol | `crates/apeireth-protocol/src/lib.rs` (+8 lines 模块导出声明) + `ws_v1.rs` (新文件 513 行, R20 阶段 2 续时授权) | §3.3 例外 |
| 13-24 | 估 13 其他 LOCKED crate | per `1.0.0-release-report-2026-08-05.md` §6.1 完整清单 | **未列** |

### 2.2 ⚠️ 关键问题

**24 LOCKED 完整名单从来没有完整列出来过!** §3.4 只列 12 个具体 + 估 13 其他 (实际引用 `1.0.0-release-report-2026-08-05.md` §6.1), 但 `docs/release/1.0.0/release-report.md` §6.1 也只列 11 个 + "... (24 LOCKED 全部)" + 1 个例外 (apeireth-protocol).

**Mavis 实查 mtime** (16:34 之前):
- 总 84 个 crate mtime < 2026-08-10 02:55 (R19 阶段 4 起的所有 crate)
- 远大于 24 LOCKED 名单

**Mavis 之前"0 触碰 24 LOCKED" 实际只 verify 了 11 个明确 + 1 个例外 (apeireth-protocol) = 12 个**. 13-24 的 13 个 crate 我从未具体 list 过, 只是泛指"24 LOCKED".

**这是 audit 漏洞, 主人 16:27 拍板"为了升级更好"后, 应该重新 audit + 重新列.**

### 2.3 实际可能的 24 LOCKED crate (Mavis 推测, 待主人拍板)

按 R19 阶段 4 起的核心 crate 推测 24 LOCKED 名单:
1. apeireth-supervisor (明确) ✅
2. apeireth-agent (明确) ✅
3. apeireth-bus (明确) ✅
4. apeireth-council (明确) ✅
5. apeireth-evolution (明确) ✅
6. apeireth-extension (明确) ✅
7. apeireth-graph (明确) ✅
8. apeireth-mcp (明确) ✅
9. apeireth-pipeline (明确) ✅
10. apeireth-tool-registry (明确) ✅
11. apeireth-tool-runtime (明确) ✅
12. apeireth-protocol (例外) ✅
13. **apeireth-asi** (LOCKED V0.5/V1136, per 17-APEIRETH-VS-VCP §597)
14. **apeireth-onion** (per architecture-v3-aircraft-carrier 5 重守门来源)
15. **apeireth-sovereignty** (per 274KB LOCKED 安全核心, R124-3 调研 0 触碰)
16. **apeireth-constraint** (5 重守门核心, R124-3 调研 0 触碰)
17. **apeireth-memory** (per R120 A 真接, 9 LOCKED memory 文件 untouched)
18. **apeireth-cognition** (per R124-2 B-028 借鉴目标)
19. **apeireth-perception** (per R120 哲学 crate)
20. **apeireth-consciousness** (per R20 哲学 crate)
21. **apeireth-motivation** (per R20 哲学 crate)
22. **apeireth-life-force** (per R20 哲学 crate)
23. **apeireth-relation** (per R20 哲学 crate)
24. **apeireth-value** (per R20 哲学 crate)

**注**: 这是 Mavis 推测. 实际 R19 阶段 4 起的 LOCKED 哲学 crate 应该都是. 主人 8/10 1.1-release/README.md 提到 "**24 LOCKED + 9 organ + 8 LOCKED**" — 24 是核心, 9 organ 是 9 器官 crate, 8 LOCKED 是另外 8 LOCKED 文档 (per 8-promise-audit §4).

---

## 3. Mavis 提议更新 (按类别 A/B/C)

### 类别 A: 数字类 (主人 16:27 "数字严守" 暗示) — 🔒 0 改

| 项 | 当前值 | 理由 |
|---|---|---|
| **R11 baseline 3 值** | 0.8682 / 0.8532 / 0.9063 | 历史快照, baseline 之上有 current 值 0.92 (per 11-baseline.md) |
| **R11 Python 9 子测度** | 9 子测度 | 跟 V1136 综合 0.9063 强绑定 |
| **12 键 verdict cache** | V3 9 键 + v4.1 3 键 | 哲学核心, 0 改 |

**Mavis 严守**: 这 3 项数字 0 改, 形式可重命名/重排, 但数字不动.

### 类别 B: 结构类 (主人 16:27 暗示"可以重整") — 🟢 大胆建议改

| # | 项 | 当前 | Mavis 提议 | 触发 |
|---|---|---|---|---|
| **B1** | **24 LOCKED crate 名单** | 11 明确 + 1 + 13 估 (实际 84 mtime) | **重 audit 24 名单, 列出全部 24 个** (per §2.3 推测, 待主人拍板) | R125-1 R125-12 等借鉴实施前, 必须明确 24 名单 |
| **B2** | **workspace.version 1.1.0** | 1.1.0 (R38 1.0→1.1 升级) | R125 末 → **1.2.0** (minor, 借鉴实施) + R127 1.0 release → **1.0.0** (历史归 0) | R125-1/2/3 实施 |
| **B3** | **V0.5 24 维公式** | 24 维, 编译期 hardcode enum | R125-10 Kani + R125-13 SWE-bench 借鉴后, **可扩展到 25-30 维** (新增 "Robustness 鲁棒性" 等借鉴维度) | R125-10/13 实施 |
| **B4** | **5 重守门 (v5 修正)** | 4 重 + 权限发放 | R125-5 NVIDIA Guardrails Colang DSL 借鉴后, **v6 修正加"Colang DSL 守门"** 作为第 5 重 | R125-5 实施 |
| **B5** | **6 哲学锚** | S-1/S-2 + O-2/O-3/O-4/O-5 | **加 S-3 (质量工程化) + O-1 (安全优先) = 8 锚** (per R123-1 clippy+doc 清 + 5 重守门) | R123-1 done + R125-5 实施 |
| **B6** | **双洋葱架构** | 原则 + 权限 | R125-5 Colang DSL 后, **升级"三洋葱" (原则 + 权限 + DSL)** | R125-5 实施 |
| **B7** | **9 器官代码** | 9 organ (TUI organ/*.rs) | **保留 9 organ** (per R124-2 §14.4 科学依据), 器官**内部 fn 可借 R125-12 OpenCode 重构** (199KB → 120KB, -40%) | R125-12 实施 |

### 类别 C: 策略类 (主人 16:27 "为了升级更好"暗示) — 🟢 Mavis 0 改

| # | 项 | 当前 | Mavis 提议 | 理由 |
|---|---|---|---|---|
| **C1** | **0 主动 commit** | 主人 14:56 拍板 R125 续自动派, 0 主动 commit | 0 改 (R125 实施仍 0 主动 commit, 17:30 整合 #3 拍板) | 主人拍板策略持续 |
| **C2** | **0 装 (O-5)** | 12 键编译期 hardcode | 0 改 (严守, 0 假装原则不动) | 哲学核心 |
| **C3** | **0 装 5 项** | 编译期 hardcode + 运行时拦截 + 多 AI 一致 + 物理隔离 HA + 反思期审计 | 0 改 (5 项不假装是 5 守门每层都适用) | 跟 B4 升级正交 |

---

## 4. R125-R127 升级路线图 (locked 维度)

### 4.1 R125 末 (8/31, 借鉴实施完)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B1**: 重 audit 24 LOCKED 名单 (完整 24 个) | R125 借鉴实施前必做 | `docs/conventions/10-locked.md` §24-35 + `1.0-release/8-promise-audit.md` §3.4 | R11 baseline 数字 + 12 键 + 9 organ 9 文件名 |
| **B2**: workspace.version 1.1.0 → 1.2.0 (minor) | R125 借鉴实施 14 commit 后 | `Cargo.toml:1-30` workspace.package.version | 1.1.0 → 1.2.0 增量 |
| **B5**: 6 哲学锚 → 8 锚 (加 S-3 + O-1) | R123-1 clippy+doc 清 + R125-5 守门借鉴 | `conventions/09-anchor.md` + 文档 + test | 6 锚原意 |
| **B6**: 双洋葱 → 三洋葱 (原则 + 权限 + DSL) | R125-5 NVIDIA Guardrails Colang 借鉴 | `onion-wall-architecture-*.md` + `apeireth-sovereignty/src/lib.rs` | 双洋葱原则 |

### 4.2 R126 (Q4 2026, 9-10 月, 5 拆 crate)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B3**: V0.5 24 维 → 25 维 (Robustness 鲁棒性) | R125-10 Kani 形式化 + R125-13 SWE-bench 借鉴 | `apeireth-asi/src/lib.rs:V05_DIM_COUNT=24` → 25 + 新维 enum | V1136 9 子测度 |
| **B4**: 5 重守门 (v5) → 6 重守门 (v6, 加 Colang DSL) | R125-5 NVIDIA Guardrails 借鉴 | `apeireth-constraint/src/lib.rs` + 5 守门 → 6 守门 | 4 重 + 权限发放原意 |
| **B7**: 9 器官内部 fn 借 OpenCode 重构 (199KB → 120KB) | R125-12 OpenCode 子代理借鉴 | `apeireth-tui/src/organ/*.rs` (9 文件) | 9 organ 文件名 + 9 organ 入口签名 |

### 4.3 R127 (1.0 release 前, 11-12 月)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B2**: workspace.version 1.2.0 → 1.0.0 (历史归 0, release 时) | 1.0 release 节点 | `Cargo.toml:1-30` workspace.package.version | semver 严守 |
| 5 拆 crate (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive) | R125 末 + R126 续 | workspace members | 92 crate 总数 |
| 4 协议 handler trait 真接 (R123-2 骨架) | R125-1 续 | `apeireth-api/src/protocol_handler_trait.rs` | 11 agent 公共 API |

### 4.4 总 9 项实质 Locked 状态 (R127 1.0 release)

| # | 项 | R119 状态 | R125 末 | R126 | R127 release |
|---|---|---|---|---|---|
| 1 | 24 LOCKED crate mtime | 16:34 baseline | 重 audit 名单 | 沿用 | 沿用 |
| 2 | workspace.version | 1.1.0 | 1.2.0 | 1.2.x | 1.0.0 (release) |
| 3 | R11 baseline 3 值 | 0.8682/0.8532/0.9063 | 0 改 | 0 改 | 0 改 (历史) |
| 4 | V0.5 24 维公式 | 24 维 | 24 维 | 25 维 (Robustness) | 25 维 |
| 5 | 12 键 verdict cache | 12 键 | 12 键 | 12 键 + 1 借鉴键 (R125-12 OpenCode) | 13 键 |
| 6 | 5 重守门 (v5) | 4 重 + 权限 | 5 重 (v6 + Colang DSL) | 6 重 (v6.1) | 6 重 |
| 7 | 6 哲学锚 | 6 锚 | 8 锚 (S-3 + O-1) | 8 锚 | 8 锚 |
| 8 | 双洋葱架构 | 双洋葱 | 三洋葱 (原则 + 权限 + DSL) | 三洋葱 | 三洋葱 |
| 9 | 9 器官代码 | 9 organ | 9 organ (内部 fn 借 OpenCode) | 9 organ | 9 organ |

**净效果**: 9 项实质 locked 中, **2 项不动 (R11 baseline 数字 + 12 键原 12)**, **7 项 R125-R127 期间合理升级**. 1.0 release 时 locked 状态升级, 但形式 (10-locked.md) 跟实质 (9 项 baseline) 仍严守.

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **24 LOCKED 名单重 audit 漏列** | R125 借鉴实施破 LOCKED 风险 | Mavis §2.3 推测名单 + 主人拍板确认, 实施前 verify |
| **workspace.version 1.1.0 → 1.2.0 破坏 semver** | semver 严守是 8 项不修改承诺 #8 | R125 末 1.1 → 1.2 minor (新增借鉴功能) 合 semver, 1.2 → 1.0 release 大版本归 0 |
| **V0.5 24 维 → 25 维破坏 R11 baseline** | baseline 3 值 0 改 (R11 是 17 维综合 0.8682) 跟 24 维 V0.5 是不同概念 | V0.5 公式可扩展, R11 baseline 数字 0 改, 形式可分 |
| **9 器官 OpenCode 重构破坏器官设计** | 9 器官是 R11 哲学核心, R124-2 §14.4 认知科学有依据 | 9 organ 文件名 + 入口签名 0 改, 内部 fn 借 OpenCode 重构 |
| **加 S-3 + O-1 锚穿透破坏 6 锚** | 6 锚是 R11 哲学, 加 2 锚 = 8 锚要主人拍板 | Mavis 提议加, 主人确认 6 锚 → 8 锚后实施 |
| **三洋葱架构破坏双洋葱** | 双洋葱是 R11 核心架构 | R125-5 NVIDIA Guardrails 实施后, DSL 层是新增"嵌入"还是"独立"待定 |
| **主人 16:27 授权 ≠ 主人详细 audit** | Mavis 大胆提议可能跟主人意图偏差 | 本报告出, 主人审阅, 主人拍板, Mavis 实施 |

---

## 6. 拍板执行 (Mavis 自主, 主人 16:27 + 8/10 01:14 双授权)

### 6.1 Mavis 0 主动执行 (待主人拍板)

- [x] 写本 audit 报告 `locked-audit-2026-08-10.md`
- [x] 完整盘点 9 项实质 + 8 项不修改承诺 + 24 LOCKED 名单推测
- [x] 大胆提议 B1-B7 (7 项结构类更新) + 严守 A1-A3 (3 项数字类) + 0 改 C1-C3 (3 项策略类)
- [x] R125-R127 升级路线图 (locked 维度)
- [ ] **主人审阅 + 拍板**: B1-B7 哪些可立即实施, 哪些留 R125 末, 哪些 R126 / R127

### 6.2 主人可立即拍的 5 个关键决定

1. **B1 24 LOCKED 名单**: 主人审阅 Mavis §2.3 推测, 确认 24 个具体 crate, 主人可补 13-24 实际名单
2. **B5 6 锚 → 8 锚**: 主人确认加 S-3 (质量工程化, 跟 R123-1 clippy+doc 清关联) + O-1 (安全优先, 跟 5 守门关联)
3. **B6 双洋葱 → 三洋葱**: 主人确认 R125-5 NVIDIA Guardrails 实施后, DSL 层作为"嵌入"还是"独立"洋葱
4. **B2 workspace.version**: 主人确认 R125 末 1.1 → 1.2 (minor) + R127 release 1.0 (大版本归 0)
5. **9 器官保留 / 9 器官内部重构**: 主人确认 R125-12 OpenCode 借鉴只动器官内部 fn, 9 organ 文件名 + 入口签名 0 改

### 6.3 17:30 final report 节点

主人 16:27 拍板后, 17:30 写 final-17-30 报告时:
- 如果主人 B1-B7 全 OK: R125 派活 14 任务 (含 7 借鉴 + 7 抽象 + 1 拆 crate + 1 dead code) 一次拍板
- 如果主人 B1-B7 部分 OK: R125 派活 14 任务中只实施主人 OK 的, 留 R126 续
- 整合 #3 commit 仍按主人 14:56 + 16:27 拍板, Mavis 自主

---

## 7. 0 LOCKED 严守 vs 大胆更新 (Mavis 立场)

### 7.1 严守 (🔒, Mavis 0 主动改)

- **R11 baseline 3 值数字** (0.8682 / 0.8532 / 0.9063) — 历史快照, 0 改
- **R11 Python 9 子测度** — 0 改
- **12 键原 12** (V3 9 键 + v4.1 3 键) — 0 改原 12 键
- **workspace.version 1.1.0 → 升级前数字** — 0 改 (semver 严守)

### 7.2 大胆提议 (🟢, Mavis 提议, 待主人拍板)

- **24 LOCKED 名单重 audit** (B1) — 主人授权, Mavis 推测 §2.3, 待主人确认 13-24
- **6 哲学锚 → 8 锚** (B5) — 加 S-3 + O-1, 待主人拍板
- **双洋葱 → 三洋葱** (B6) — 加 DSL 层, 待主人拍板
- **5 重守门 → 6 重守门** (B4) — 加 Colang DSL, 待主人拍板 (跟 B6 同步)
- **V0.5 24 维 → 25 维** (B3) — 加 Robustness, R125 末/R126 实施
- **9 organ 内部 fn 借 OpenCode** (B7) — 199KB → 120KB, R125-12 实施

### 7.3 0 改 (🟢, Mavis 0 主动改, 0 假装原则)

- **0 主动 commit** (C1) — 主人 14:56 拍板, 0 改
- **0 装 (O-5)** (C2) — 12 键编译期 hardcode 原则, 0 改
- **0 装 5 项** (C3) — 5 守门每层都适用, 0 改

---

**Mavis 16:35 状态**: 9 项实质 + 24 LOCKED 名单 + 8 项不修改承诺全盘点完. 7 项结构类 (B1-B7) 大胆提议, 3 项数字类严守 (A1-A3), 3 项策略类 0 改 (C1-C3). 主人 16:27 拍板 + 8/10 01:14 拍板双授权下, 0 主动执行, 写 audit 报告待主人审阅 + 拍板 5 关键决定. 17:30 整合 #3 节点准备就绪.
