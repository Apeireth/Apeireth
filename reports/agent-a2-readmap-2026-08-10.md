# Agent A-2 战区 (.github 工程化) — Readmap 报告
**日期**: 2026-08-10
**作者**: Mavis 派 — Agent A-2 (Apeireth-rust 后端升级)
**任务**: 补全 `.github/` 3 类文件 (dependabot.yml / ISSUE_TEMPLATE / PULL_REQUEST_TEMPLATE)
**阶段**: A2-1 读图 (0-1h)
**接 A**: A 已完成 vector + memory (sqlite-vec + semantic_search + user_profile, 30 min 完成)

---

## 1. 任务 vs 现状 (诚实核验)

| # | 任务描述 | 实际现状 (2026-08-10 02:58 mtime) | 决策 |
|---|---|---|---|
| 1 | "D-1 留的 R26+ TODO: 写 `.github/dependabot.yml`" | **已存在** (R18 写, 89 行, 完整 4 group + Cargo + GitHub Actions + 周一 06:00 UTC + Major skip + labels) | ✅ 任务前提过期 — **0 改** (不重复造轮子, 主人偏好 #6) |
| 2 | "D-1 留的 R26+ TODO: 写 `ISSUE_TEMPLATE/bug_report.yml` + `feature_request.yml` + `config.yml`" | **有 3 个 .md** (bug.md / feature.md / 1.0-blocker.md, R20 写, R23 续), 但**没有** .yml 格式 | ✏️ **写 3 个新 .yml** (YAML 是 GitHub 2024 新版 1:1) + 保留 3 个 .md (向后兼容, 1.0-blocker 是 1.0 release 专用, 删了破坏 12 项 checklist) |
| 3 | "D-1 留的 R26+ TODO: 写 `PULL_REQUEST_TEMPLATE.md`" | **已存在** (R20 写, 45 行, 5 段 checklist: R20 阶段 1-6 / 4 决策拍板 / 测试 / 文档 / 1.0 release 12 项) | ✏️ **重写对齐 R26+ 5 项硬约束** (R20 阶段 1-6 已过期, 任务要的 5 项 = 0 触碰 24 LOCKED / 0 改 workspace.version / 0 改 R11 baseline / cargo test pass / 0 假装 跟 R20 模板 70% 重复, 但 R20 模板的"R20 阶段 1-6 必做"已过期, "4 决策拍板"是 D-01/D-02/D-06/D-07 1.0 决策, 不是 R26 决策) |
| 4 | "`dependabot-upgrade.yml` 已有 (R20 #6)" | **存在, 但它是 workflow** (PR auto-merge 流程, 86 行, 4 jobs), **不是** dependabot config | ✅ 不动 — 跟 dependabot.yml 无关 |

**任务前提过期率**: 3/4 = 75% (跟 D-1 一样的处境, R18-R20-R23 已做大量基础工作)

---

## 2. 现有 .github/ 资产全貌

```
.github/
├── ISSUE_TEMPLATE/
│   ├── 1.0-blocker.md       (35 行, R20 写, 1.0 release 专用) — 保留 ✅
│   ├── bug.md               (57 行, R20 写, 1.0 release 影响 + 6 哲学 + 8 项 + 复现) — 保留 ✅
│   └── feature.md           (35 行, R20 写, R20 阶段 + 6 哲学 + 8 项) — 保留 ✅
├── workflows/               (18 yml, D-1 整理 — 0 触碰) ✅
│   ├── bench.yml
│   ├── benchmark-tracking.yml
│   ├── cargo-audit.yml
│   ├── cargo-deny.yml
│   ├── cosign.yml
│   ├── coverage.yml
│   ├── dependabot-upgrade.yml    (PR auto-merge workflow, 86 行) ✅
│   ├── eval-live.yml
│   ├── kani.yml
│   ├── miri.yml
│   ├── protocol-e2e.yml
│   ├── release-1.0.0.yml
│   ├── release.yml
│   ├── rust-ci.yml
│   ├── rust-lint.yml
│   ├── rust.yml                (D-1 新, 3 OS matrix + nextest)
│   ├── rustdoc.yml
│   └── rustfmt.yml             (D-1 新, nightly fmt --check)
├── dependabot.yml           (89 行, R18 写, 完整) ✅ 0 改
├── PULL_REQUEST_TEMPLATE.md (45 行, R20 写) — 重写 ✏️
└── (没有 ISSUE_TEMPLATE/*.yml 也没有 config.yml) — 新建 🆕
```

**根目录 `CONTRIBUTING.md` 已存在** (107 行, R119-4d 后, 含 0 触碰实查 + 6 哲学 anchor + 8 项 + PR 流程 + 1.0 release 12 项). 跟我新建的 PR template 1:1 引用.

---

## 3. dependabot.yml 0 改决策 (不重写, 不重复造轮子)

**现有 dependabot.yml 内容核验** (89 行, R18 写, 跟任务要"补全" 完全 1:1):

| 任务要求 | 现有内容 | 状态 |
|---|---|---|
| 抄 qdrant 模式 (Cargo + GitHub Actions 双周更新) | Cargo (`package-ecosystem: "cargo"`, directory: `/`) + GitHub Actions (`package-ecosystem: "github-actions"`) | ✅ 1:1 |
| 周一 06:00 UTC 跑 | Cargo + GitHub Actions 都是 `interval: weekly` / `day: monday` / `time: 06:00` / `timezone: UTC` | ✅ 1:1 |
| 4 个 group (tokio / http / serde / wasm) + catch-all "dependencies" | tokio (tokio*+mio*+socket2*) / http (reqwest*+hyper*+http*+h2*) / serde (serde*+toml*) / wasm (wasm*+leptos*+js-sys) / dependencies (catch-all `*` + exclude 11 个 pattern) | ✅ 1:1 |
| Major 版本不自动合 | `ignore: [{dependency-name: "*", update-types: ["version-update:semver-major"]}]` | ✅ 1:1 |
| PR labels: dependencies + automated | Cargo: `["dependencies", "automated"]` / GitHub Actions: `["dependencies", "automated", "ci"]` | ✅ 1:1 |

**结论**: dependabot.yml 跟任务要"补全"的内容 **字字 1:1**, 我**0 改**. 写决策日志 (per 主人偏好 #10).

**跟 dependabot-upgrade.yml 区分**:
- `dependabot.yml` = dependabot 自身的 config (周一 06:00 UTC 跑, 哪些 group, 哪些 ignore, 哪些 labels)
- `dependabot-upgrade.yml` = dependabot 开 PR 后的 GitHub Actions workflow (auto-merge patch/minor, skip major)
- 两者职责完全不同, 不冲突.

---

## 4. ISSUE_TEMPLATE 新建 3 个 .yml (YAML 1:1 qdrant 模式)

**qdrant 实际 ISSUE_TEMPLATE 结构** (业界惯例, 2024+ GitHub 推荐 YAML 1:1):
- `bug_report.yml` — 报告 bug, name + about + title + labels + body (assignees / type dropdown / OS / version / repro / expected / actual)
- `feature_request.yml` — 提 feature, name + about + title + labels + body (problem / proposed solution / alternatives / context)
- `config.yml` — issue chooser (blank_issues_enabled + contact_links + 链接到 Discord/讨论区/discussions)

**Apeireth 适配 (加 6 哲学 anchor + 8 项不修改承诺必填 + 默认标签)**:
- bug_report.yml: 加 "0 触碰实查" 段 (24 LOCKED / workspace version / 7 LOCKED 文档) — **不强制填, 但默认勾选 none** (per 现有 bug.md)
- feature_request.yml: 加 "8 项不修改承诺验证" 段
- config.yml: 跟 1.0-blocker.md 兼容 (加 1.0-blocker.md + bug.md + feature.md 3 个 .md 链接, 让用户多选)

**保留 3 个 .md 决策** (重要!):
- `1.0-blocker.md`: 1.0 release 专用, 跟 1.0 release 12 项 checklist 强绑定, 删了破坏 12 项
- `bug.md`: R20 写, 跟 1.0 release 影响段有交叉, 保留向后兼容
- `feature.md`: R20 写, 同上

**但 GitHub ISSUE_TEMPLATE 优先级** (业界惯例):
- 如果同时有 .md 和 .yml, GitHub 优先显示 .yml (2024 新版行为)
- 用户 1 选 issue type 时, .yml 显示在 .md 上面
- .md 仍可被选 (但优先级低)
- 加 config.yml 设 `blank_issues_enabled: false` 强制走模板 (跟 qdrant 1:1)

---

## 5. PULL_REQUEST_TEMPLATE.md 重写 (R20 阶段 1-6 → R26+ 5 项硬约束)

**现有 PULL_REQUEST_TEMPLATE.md (R20 写) 5 段**:
1. R20 阶段 1-6 checklist (5 项) — **已过期** (R20 完成于 2026-08-05)
2. 4 决策拍板 (D-01/D-02/D-06/D-07) — **1.0 release 决策, 不适用 R26+ 日常**
3. 测试 (4 项) — **保留** (cargo check + cargo test + 0 引 NewAPI + 0 重复造轮子)
4. 文档 (3 项) — **保留** (CHANGELOG + ROADMAP + docs/stage4)
5. 1.0 release 12 项 checklist — **保留** (1.0 release 收尾用)

**任务要"5 项 checklist"**:
1. 0 触碰 24 LOCKED
2. 0 改 workspace.version
3. 0 改 R11 baseline (V1141 / V1131 / V1136)
4. cargo test pass
5. 0 假装

**重写决策** (per 主人偏好 #7 诚实 + 主人偏好 #6 不重复造轮子):
- 保留 R20 模板的 **测试段 4 项** (cargo check + cargo test + 0 引 NewAPI + 0 重复造轮子) — 跟任务要的 5 项 70% 重叠
- 保留 R20 模板的 **1.0 release 12 项 checklist** — 给 1.0 release 收尾用
- 保留 R20 模板的 **文档段 3 项** — 严守规范
- 砍掉 **R20 阶段 1-6 段** (已过期) — 改用 R26+ 5 项硬约束替代
- 砍掉 **4 决策拍板段** (1.0 release 决策, 不适用 R26+ 日常) — 决策走 reports/ 流程
- **新增** R26+ 5 项硬约束段 (任务要的 5 项, 1:1 引用 CONTRIBUTING.md §0 触碰实查)

**目标结构** (5 段):
1. **R26+ 5 项硬约束** (任务要的, 0 触碰 24 LOCKED / 0 改 workspace.version / 0 改 R11 baseline / cargo test pass / 0 假装)
2. **测试** (4 项, 沿用 R20 模板)
3. **文档** (3 项, 沿用 R20 模板)
4. **6 哲学 anchor 穿透** (6 项, S-1 / S-2 / O-2 / O-3 / O-4 / O-5)
5. **1.0 release 12 项 checklist** (12 项, 沿用 R20 模板)

---

## 6. 硬约束核验 (本次严守)

| 约束 | 现状 | 本次动作 |
|---|---|---|
| 0 改 workspace.version (1.1.0) | Cargo.toml:246 = `"1.1.0"` | **不读不写这一行** (本来也不动 Cargo.toml) |
| 0 改 R11 baseline 3 值 (V1141 / V1131 / V1136) | `apeireth-asi/src/lib.rs` | **不读不写** |
| 0 改 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 / 9 器官 | apeireth-asi / cognition / core | **不触碰** |
| 0 触碰 24 LOCKED | CONTRIBUTING.md 列出 24 crate 名单 (apeireth-{supervisor, agent, council, bus, protocol, mcp, tool-registry, tool-runtime, graph, pipeline, tool-approval, extension, evolution, api, core, memory, asi, tools, cli, bench, cognition, action, life-force, constraint}) | **不触碰** (我改的 3 个文件都在 .github/) |
| 0 改现有 18 个 workflow yml | D-1 已整理 | **0 触碰** |
| 0 改 src/ | — | **0 触碰** (我改的 3 个文件都在 .github/ + reports/) |
| 0 主动 commit | git status | **0 commit** (写完 untracked, 等主人验收) |
| 不与 A/B/C/D-2/D-3 冲突 | A 改 vector/memory, B 改 api/{cache,retry,routing}, C 改各 product tests, D-2 改 tool-registry, D-3 改 council | ✅ 我只改 .github/ + reports/, **0 触碰** 任何 crate 源码 |

---

## 7. 风险点 + 决策日志

| # | 风险 | 决策 | 备选 |
|---|---|---|---|
| R1 | dependabot.yml 重写覆盖掉 R18 已稳定的 4 group 模式 | **0 改** (R18 写的跟任务 1:1, 不重复造轮子) | 重写更"完美" (破坏现状) |
| R2 | ISSUE_TEMPLATE 3 个 .md 全删, 破坏 1.0 release 12 项 checklist | **保留 .md + 新建 .yml** (向后兼容, GitHub 优先显示 .yml) | 删 .md (破坏 1.0 release) |
| R3 | PULL_REQUEST_TEMPLATE 重写删了 R20 阶段 1-6 / 4 决策拍板段 | **重写对齐 R26+ 5 项** (R20 已过期, 4 决策是 1.0 收尾用) | 保留 R20 模板 (过期) |
| R4 | YAML 解析错 (config.yml 缩进 / yml 缺 name 段) | **PyYAML 严格 parse** (复用 D-1 模式, reports/agent-a2-yaml-verify.py) | yamllint 不可用 (per D-1 经验) |
| R5 | web_fetch 超时查不到 qdrant/tokio 实际模板 | **用 R18 已抄过 qdrant/tokio 的模式** (D-1 final 报告 §3 + R18 dependabot.yml 注释 "业界来源: tokio / wasmtime / qdrant 默认配置" 已实证抄过) | 写 blocked 报告等联网 (浪费 7h 预算) |
| R6 | 主人 10:00 验收时还没全部跑通 | **A2-6 写清楚哪些完成 / 哪些 0 改 / 哪些新建**;不假装"complete" | 自己假设 OK |

---

## 8. 阶段成果

- ✅ 完整摸清 .github/ 现状 (3 类文件: 1 已存在完整, 1 有 .md 缺 .yml, 1 有 R20 模板需重写)
- ✅ 锁定 3 大技术决策: dependabot 0 改 / ISSUE_TEMPLATE 保留 .md 加 .yml / PR template 重写对齐 R26+
- ✅ 严守 8 项硬约束 (0 改 workspace.version / 0 改 R11 baseline / 0 触碰 24 LOCKED / 0 改 18 workflow yml / 0 改 src/ / 0 主动 commit / 0 重复造轮子 / 不与 A/B/C/D-2/D-3 冲突)
- ✅ 风险表 R1-R6
- ✅ 任务前提过期 75% 诚实核验 (per 主人偏好 #7)

**A2-1 阶段完成。开始 A2-2 阶段: 写 ISSUE_TEMPLATE 3 个 .yml (最高价值, 用户接触面) + 重写 PR template.**
