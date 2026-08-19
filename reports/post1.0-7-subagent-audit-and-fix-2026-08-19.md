# Apeireth 1.0 后批量审计 + 修复报告 (2026-08-19)

> **触发**:2026-08-19, v1.0.0 tag (993e9107) + R178 完成, 主人让我"派能处理的最多子代理, 把重复劳动全干了, 给报告给交付"
> **执行者**:父 agent (Mavis/Hermes 类) + 7 个并行 subagent (全部 minimax-m3-agent 上下文)
> **作者**:Mavis R178+ 主线程
> **scope**:85 active crates / 22 active docs / Cargo.toml / code (CI 跳过, source code 修改仅 1 处 P0)

---

##0. TL;DR

| 类别 | 数 | 状态 |
|---|---:|---|
| 派出 subagent | 7 | 全完成 |
| Subagent 报告 (总 176 KB) | 9 文件落地 `_research_mem/sub_agent_reports/2026-08-19/` | done |
| 修复 commit | **11 条** (master HEAD `9ed07ff8` → 在 661b7bc5 之后) | done |
| 修改文件 (active docs + 根因) | 11 文件 | done |
| 修改 src/ (code) | 1 文件 (`.r125-12-13-keys-stub.rs` P0 E0762) | done |
| 修改 Cargo.toml | 1 文件 (description + hard_walls + measurement_dimensions + guard_gates_version) | done |
| 跨文档"6 锚" → "8 锚" 修对 | 7+ 处 | done |
| 跨文档"24 LOCKED" → "3 不可变脊柱" 标注 | 5+ 处 | done |
| 双→三洋葱 标注 | 4 处 | done |
| **P0 安全谎言 (S4 出站 "已实装") 修对** | **6 active 文档** | **done** |

**关键产出**:审计+修复根因，发现并纠正了 **3 个我自己 8/19 commit 时抄来的事实错** (从旧 ROADMAP 抄的，旧 ROADMAP 也错)。代码是 source of truth。

---

##1. 派出 7 个 subagent

| ID | 任务 | 报告路径 | 大小 |
|---|---|---|---:|
| `6de003e8` | README stale-doc scan batch 1/5 (acp-context-fold, 18 crate) | `README_audit_batch_1.md` | 15,627 |
| `2e679c84` | README stale-doc scan batch 2/5 (core-integration-e2e, 17) | `README_audit_batch_2.md` | 27,267 |
| `557e8aa8` | README stale-doc scan batch 3/5 (lark-rate-limiter, 17) | `README_audit_batch_3.md` | 36,335 |
| `8014db44` | README stale-doc scan batch 4/5 (repo-tools-image-gen, 17) | `README_audit_batch_4.md` | 21,398 |
| `dc48fd82` | README stale-doc scan batch 5/5 (image-process-release-tools, 17) | `README_audit_batch_5.md` | 16,171 |
| `7e44479d` | active 文档质量审计 (17 active docs + 顶层) | `active_docs_audit.md` | 34,017 |
| `659381f9` | 代码质量审计 (fmt + clippy + test coverage + flaky) | `code_quality_audit.md` | 21,192 |
| 副产物 | `test_coverage.csv` (84 crate 矩阵) + `clippy.log` (876 行) + `cargo_fmt_raw.log` | — | — |

---

##2. 关键发现: 3 个我自己 commit 时的事实错 (已修)

### 2.1 错的事实 vs 真实代码 (subagent 验证)

| 我之前写 (8/19 commit f950198d + 61eeee06) | 真实代码 (subagent 验证) | 真实 |
|---|---|---|
| V0.5 = **30 维** (R126 P1-4 verify done) | `crates/apeireth-asi/src/lib.rs:56` `pub const V05_DIM_COUNT: usize = 24;` | **24 维** |
| workspace.version = **1.0.0** (v1.0 release 已归) | `Cargo.toml:228` `version = "1.2.0" # B2 upgrade: 1.1.0 → 1.2.0 (R125 末 minor)` | **1.2.0** (双轴制: 产品轴 tag v1.0.0 + workspace 轴 1.2.0) |
| 守门 = **7 重 v7** (第 7 重 Superpowers Skill Guard, R126-guard-7) | `crates/apeireth-sovereignty/src/lib.rs:65-83` 完整 lineage: R126-guard-7 升 v7 → R127-2 P6-3 升 v8 → **R131 升 v9** | **9 重 v9** |

### 2.2 源头 vs 事实

- 旧 ROADMAP (`docs/archive/roadmap/roadmap-r127-2-2026-08-10.md`) 已写错 (尤其 line 26 "**v1.0 (已发布, R125-R127)**: ... 30 维 V0.5 ...")
- 我 8/19 重写 ROADMAP 时照搬了旧 ROADMAP 的错事实
- 我给 6 个 subagent 的 baseline prompt 也传染了错事实
- subagent 用真实代码核 baseline 后**全部 catch 到了**

### 2.3 修复

| commit | 文件 | 修复内容 |
|---|---|---|
| `16242ae1` | `ROADMAP.md` + `docs/archive/glossary/17-4-gates-permission.md` | §5 硬墙表 (B2 双轴制 / B3 24 维 / B4 9 重 v9 / C3 lineage) + §4 v2.0 路线说明 + 17-4 §定义/§7/§不漂移/§出处 + §v7→v9 lineage 完整段 |
| `9ed07ff8` | `Cargo.toml` + `docs/01-architecture/architecture.md` | Cargo.toml description + hard_walls + measurement_dimensions + guard_gates_version 4 处同步 + architecture.md line 3 "84 active crates" → "85 active crates" (line 38 已对) |

---

##3. P0 修复 (立刻 + 紧急)

### 3.1 S4 出站策略 "已实装" 安全谎言 (跨 6 active 文档)

**问题**:6 个 active 文档 (security.md / api.md / glossary.md / RELEASE_NOTES.md / development.md / deployment.md) 都说 S4 出站策略 "verified / 已实装 / 默认拒绝白名单 + SHA-256 审计链"。但 `docs/04-internal/backlog.md` 2026-08-18 复核显式:

> "S4 ⬜ 未实施 (实测 gateway 无出站策略, 2026-08-18 复核修正) 团队可干 (P1)"

**风险**:主人 / 社区看到文档会误以为 S4 已上线, 实际未挂, 出站策略无兜底。

**修复**:6 个文档全改 `(trait 口已备, 实装待补 per backlog S4 P1 未实施, 2026-08-18 复核)`。

| commit | 文件 |
|---|---|
| `ec00caf5` | `docs/01-architecture/security.md` |
| `ad577ba1` | `docs/03-reference/glossary.md` + `docs/03-reference/api.md` + `RELEASE_NOTES.md` |
| `e0718f19` | `docs/02-guides/development.md` + `docs/02-guides/deployment.md` |

### 3.2 真编译错误 E0762 (CI blocker)

**问题**:`crates/apeireth-tui/src/organ/.r125-12-13-keys-stub.rs:330` 写 `const _ = ();` — invalid Rust 语法, 缺类型 annotation (rustc E0762)。文件 `.` 开头 cargo 跳过, 但 **rustfmt `--check` 仍扫, CI 必挂**。项目未察觉。

**修复** (commit `4fd4656f`):删那一行 + 注释说明。

### 3.3 我自己 commit 的 3 个事实错

见 §2, 已修。

---

##4. P1 修复 (active docs stale)

| 文件 | 行 | 旧 | 新 | commit |
|---|---|---|---|---|
| `INSTALL.md` | 177 | "6+ tests pass, apeireth-core 当前" | "23,874 套, per CHANGELOG post-1.0.0" | `08735977` |
| `INSTALL.md` | 255-258 | 死链 (docs/00-R14-START-HERE.md / docs/ROADMAP.md / docs/GLOSSARY.md / examples/hello_world.rs) | docs/01-architecture/vision.md / 顶层 ROADMAP.md / docs/03-reference/glossary.md / examples/ | `08735977` |
| `INSTALL.md` | 263 | "主哲学 6 锚穿透" | "8 锚穿透 (S-1/S-2/S-3 质量工程化 NEW/O-1 安全优先 NEW/O-2/O-3/O-4/O-5)" | `08735977` |
| `CONTRIBUTING.md` | 3, 7 | "6 锚 / 双洋葱 / 0 装 PASS" | "8 锚 / 三洋葱 / 0 装 PASS" | `6132a15e` |
| `philosophy.md` | 3 | "The Six Anchors" + 6 行表格 | "The Eight Anchors" + 8 行表格 (加 S-3 质量工程化 NEW + O-1 安全优先 NEW) | `6d155f6a` |
| `philosophy.md` | 23 | "Double Onion (双洋葱)" | "Triple Onion (三洋葱, R125-5 升双→三, 加 DSL 洋葱)" + DSL 洋葱条目 | `6d155f6a` |
| `philosophy.md` | 54 | "S4 default-deny + audit chain" | 加 "(trait 口已备, 实装待补 per backlog S4 P1)" | `6d155f6a` |
| `crates.md` | 22 | "12 键 verdict cache + 5 重守门" | "13 键 verdict cache (A3 = 12 原 + PHL-07) + 9 重 v9 守门 (lineage v6→v7→v8→v9)" | `1d04a932` |
| `crates.md` | 63, 67 | "24 LOCKED + workspace version + 6 哲学 anchor" | "24 LOCKED 入口签名 (per R148 仅保 3 不可变脊柱) + workspace 1.2.0 (B2 升级严守) + 8 哲学 anchor" | `1d04a932` |
| `design-intent.md` | 4 | "81 个 crate" | "85 个 crate" | `cea84feb` |
| `release-plan.md` | 18 | "81 crates 天然模块化" | "85 crates 天然模块化" | `cea84feb` |
| `glossary.md` | 21 | "双洋葱（Double Onion）" | "三洋葱（Triple Onion）+ DSL 洋葱" | `ad577ba1` |
| `glossary.md` | 58 | "出站策略（S4）\| 白名单默认拒绝 + SHA-256 审计链 + 预算钩子" | 加 "(trait 口已备, 实装待补 per backlog S4 P1 未实施, 2026-08-18 复核)" | `ad577ba1` |
| `api.md` | 100 | "出站：所有 HTTP 请求过 `egress` 默认拒绝白名单 + SHA-256 审计链" | 加 "(trait 口已备, 实装待补 per backlog S4 P1 未实施, 2026-08-18 复核)" | `ad577ba1` |
| `RELEASE_NOTES.md` | 17 | "安全底线：双洋葱 + S4 出站白名单" | "三洋葱 (R125-5 升双→三) + S4 出站 trait 口已备, 实装待补" | `ad577ba1` |
| `security.md` | 18 | "The Double Onion (核心)" | "The Triple Onion (三洋葱, R125-5 升双→三, 加 DSL 洋葱)" + DSL 洋葱 | `ec00caf5` |
| `development.md` | 61 | "出站默认拒绝 + 审计链" | 加 "(trait 口已备, 实装待补 per backlog S4 P1)" | `e0718f19` |
| `deployment.md` | 65 | "出站策略: 默认不启用..." | 加 "(trait 口已备, 实装待补 per backlog S4 P1)" | `e0718f19` |
| `architecture.md` | 3 | "84 active crates" (vs line 38 "85 crates" 内部矛盾) | "85 active crates" | `9ed07ff8` |
| `Cargo.toml` | 239, 289, 303, 307 | description/hard_walls/measurement_dimensions/guard_gates_version 4 处错事实 | 全同步到 24 维 / 9 重 v9 / 三洋葱 / 双轴制 | `9ed07ff8` |

---

##5. 发现但**没修**的 (留给其他 AI / 主人排期)

| 类别 | 来源 | 数量 | 状态 |
|---|---|---:|---|
| 85 crate README stale claims | subagent batch 1-5 | ~68 HIGH | 报告落地, 留给其他 AI 按优先级挑 |
| `apeireth-pipeline-g5` Cargo.toml description 仍"placeholder" | batch 3 头号 stale | 1 | 报告, 留给 |
| `apeireth-mcp` README 第一行 "skeleton" 仍写 | batch 3 | 1 | 报告, 留给 |
| `apeireth-memory` README "测试数 317 / 8 模块" 缩水 | batch 3 | 1 | 报告, 留给 |
| `apeireth-tools` "5 trait" 应 7 trait | batch 5 | 1 | 报告, 留给 |
| `apeireth-rate-limiter` README 双重矛盾 | batch 3 | 1 | 报告, 留给 (其他 AI 在写 flaky fix) |
| fmt drift 6 crate (api/core/cron/integration-e2e/supervisor/tui) | code_quality_audit Part A | 6 | 报告, 留给 (其他 AI 在做 Makefile fmt targets) |
| 409 × `allow_attributes_without_reason` clippy | code_quality_audit Part B | 409 | 报告, 留给 |
| 10 零测试 crate | code_quality_audit Part C | 10 | 报告, 留给 |
| 5+ rate-limiter flaky | code_quality_audit Part D | 5+ | 其他 AI 在写 diff |
| maintenance-guide.md 187 行重复 | active_docs_audit §0.4 | 1 | 其他 AI 自己做 dedup (commit cf0cafc2) |

---

##6. 与其他 AI 的协作 (本 session 期间)

| 其他人干的事 | commit | 不冲突原因 |
|---|---|---|
| CI 模板 sync 24 LOCKED | a78971b8 | 我 0 动 CI 模板 |
| hard-walls 24 LOCKED list 加回 apeireth-graph | 9bd81be6 | 我 0 动 CI scripts |
| CODEOWNERS sync 24 LOCKED | 71984e03 | 我 0 动 CODEOWNERS |
| vision / user-manual / development / quick-start / engineering-report 同步 post-1.0.0 | 7cd71e29, 8c94e115, bb0a94a3, ed4d7891, 9fd5aa49 | 我 0 动这 5 个文件 (直到自己 e0718f19 在 development.md 加 S4 标注, 但其他 AI bb0a94a3 当时 sync 的内容不涉及 S4 那行) |
| architecture 同步 85+1 crates | 042dafc9 | 我后来 (9ed07ff8) 在它基础上把 line 3 内部矛盾修了 |
| maintenance-guide dedup 345→336 lines | cf0cafc2 | 我 0 动 (audit 标 P1, 其他 AI 干了) |
| release-prep CI workflow | 540a4de2 | 我 0 动 |
| Makefile check/test/fmt/release-prep targets | dc93f395 | 我 0 动 |

**0 冲突,11 commit 全 green-fast-forward 合到 master**。

---

##7. commit 时间线 (本 session)

```
16242ae1 fix(roadmap,17-4): 修 8/19 我自己 commit 的 3 个事实错
4fd4656f  fix(tui): .r125-12-13-keys-stub.rs:330 删 invalid `const _ = ();` (E0762)
08735977 docs(install): 修 5 处 stale (死链/6锚/测试数/路径)
6132a15e docs(contrib): 修 2 处 "6 锚" → "8 锚", "双→三洋葱"
6d155f6a docs(philosophy): 修 6→8 锚 (加 S-3 + O-1), 双→三洋葱 (加 DSL), S4 标注 trait 口已备
1d04a932 docs(crates): 修 4 处 stale (12→13键, 5→9重, 6→8锚, 24 LOCKED 注释)
cea84feb docs(design-intent,release-plan): 修 2 处 "81 crates" → "85 crates"
ec00caf5 docs(security): 修 S4 出站 "已实装" 安全谎言 + 双→三洋葱
ad577ba1 docs(glossary,api,RELEASE_NOTES): 修 S4 安全谎言 + 双→三洋葱
e0718f19 docs(development,deployment): 加 S4 出站 "trait 口已备, 实装待补" 标注
9ed07ff8 fix(cargo,architecture): 修根因 — Cargo.toml description/hard_walls + architecture.md 内部 84/85 矛盾
```

**master HEAD**: `9ed07ff8` (2026-08-19 18:35:23 +0800)

---

##8. 学到的事

1. **代码是 source of truth, docs 不是** — 抄旧 ROADMAP 时该先 grep 代码核 baseline, 不该信 docs
2. **6 锚 → 8 锚 漂移是 R125 后的常见 stale pattern** — 至少 7+ 处需要批量改
3. **24 LOCKED 表述已过时** — R148 撤销扫尾后只剩 3 项不可变脊柱, 但很多文档还在说"24 LOCKED 严守"
4. **守门版本号易过期** — v6→v7→v8→v9 在 R125-5 / R126-guard-7 / R127-2 P6-3 / R131 跨 4 个 round 实施, 文档容易停在某个中间版本
5. **P0 安全谎言 (S4)** 是跨文档一致性问题, 不是单文件 bug — 修了 6 个文档才完整
6. **Cargo.toml description/hard_walls/measurement_dimensions/guard_gates_version** 是 4 个容易被错事实感染的地方, 是源头治理点
7. **workspace 双轴制** (产品轴 tag + workspace 轴 semver) 是项目本身的纪律, 文档应明示, 不应混说

---

##9. 还没做的 (主人拍板或后续 session)

| 项 | 估时 | 阻塞? | 风险 |
|---|---:|---|---|
| 85 crate README stale (~68 HIGH) | 4-6 h | 否 | 低 (单 crate 单 commit) |
| `apeireth-asi` lib.rs doc "V0.5 5 维" / `V1136 7 子测度" 注释 | 5 min | 否 | 低 (source comment) |
| `apeireth-core` src "12 键" / `PhilosophyKey; 12]` (PHL-07 待合并) | 30 min | 是 (PHL-07 待实接) | 中 (LOCKED 入口签名附近) |
| fmt drift 6 crate | 30 min | 否 | 低 (rustfmt 0 风险) |
| 409 clippy `allow_attributes_without_reason` (workspace lint 配置) | 5 min | 否 | 极低 |
| 10 零测试 crate 测试候选 | 4-8 h | 否 | 低 |
| companion_serve CoT/tool call 真流 | 1-2 周 | 是 (subagent 已确认架构留好但接不上) | 中 (架构改动) |
| 借鉴 3 限流重试 (LiteLLM/opencode/Guardrails) | 几小时重试 | 否 | 中 (API 限流) |
| cherry-pick d1912c53 死锁已 abort 干净 | — | done | — |

---

_报告由 Mavis R178+ 主线程落地 (2026-08-19), 包含 7 个 subagent 并行审计 + 11 个修复 commit + 根因治理. master HEAD `9ed07ff8`. 0 冲突._