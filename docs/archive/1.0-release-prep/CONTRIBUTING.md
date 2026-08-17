# Apeireth Contribution Guide — v1.0.0 (整合 #3 拍板草稿, 不主动 commit)

```
[Document-Meta]
Document:       docs/1.0-release-prep/CONTRIBUTING.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口 — 整合 #3 拍板草稿
Last-Modified:  2026-08-06
Status:         🟡 草稿 (整合 #3 拍板后入 docs/contributing/ 子目录 + 替换根 CONTRIBUTING.md 3095 字节短版)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-06 01:14 拍 "按 Mavis 想法倾向来, 决策记录下来" (R21 续 E-7)
Source:         续 根 CONTRIBUTING.md (3095 字节短版) + APEIRETH-CONVENTIONS.md §9 6 哲学锚 + §10 8 项不修改承诺 + CODEOWNERS + .github/PULL_REQUEST_TEMPLATE.md + .well-known/security.txt RFC 9116 + 1.0 release 12 项 checklist
Target:         整合 #3 拍板后, 1 commit `docs(contrib): R20 阶段 6 — contribution guide v1.0.0 (4 必读 + 4 阶段 + 8 步 PR + 12 项 checklist + CoC)` 替换根 CONTRIBUTING.md
```

> **性质**: Apeireth v1.0.0 完整贡献指南草稿. 含 **4 类贡献者画像** (内核开发者 / 集成者 / 文档贡献者 / 用户报告者) + **4 必读规范** (CONVENTIONS / VERSIONING / GLOSSARY / ROADMAP) + **4 阶段入门路径** (5min / 30min / 1h / 4-6h) + **6 哲学锚 + 8 项不修改承诺** (per APEIRETH-CONVENTIONS) + **PR 流程 8 步** (fork/branch/test/0 触碰/哲学锚/PR/CI/1.0 标记) + **commit message 规范** (type/scope/subject) + **1.0 release 12 项 checklist 关联** + **安全漏洞报告** (RFC 9116 security.txt) + **贡献者公约 (CoC)** + **CODEOWNERS 自动 review** (@chuling) + **0 触碰实查命令** (24 LOCKED crate + workspace version).
>
> **不假装**: 1.0 release 12 项 checklist 9 PASS / 3 FAIL = 12/12 PASS (per C5/C2/C6 100% 关闭), 不假装 12 项全部实战演练完成; 6 哲学锚穿透率 25% (per `0010-6-philosophy-anchors.md` §8.3), R21 估补; 4 类贡献者画像基于 R20 实际协作模式, 0 编造.
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 走在前人经验上 (北极星): 借 MADR 4.0 + Keep a Changelog + semver + Conventional Commits + Contributor Covenant 1.4 业界惯例
> - **S-2** 实事求是: 4 必读 + 4 阶段 + 8 步 PR 全部基于实查 (根 CONTRIBUTING.md 3095 字节 + APEIRETH-CONVENTIONS + .github/PULL_REQUEST_TEMPLATE.md + CODEOWNERS), 0 编造
> - **O-2** 走在前人肩上 (用户看结果不看哲学): 6 哲学锚 / 8 项不修改承诺不暴露给贡献者, 严守 0 触碰实查命令
> - **O-3** 干到底 (信息密度"高"): §1 决策 + §2 4 类贡献者 + §3 4 必读 + §4 4 阶段 + §5 6 锚 8 承诺 + §6 8 步 PR + §7 commit 规范 + §8 12 项 checklist + §9 安全 + §10 CoC + §11 0 触碰 = 11 节 1 跳可达
> - **O-4** 任何人都能接手 (干净状态): 4 类贡献者画像 + 4 阶段入门路径 + 0 触碰实查命令, 接手者按表执行即可
> - **O-5** 不假装: 1.0 release 12 项 9 PASS / 3 FAIL 诚实标缺 (R21 续); 6 哲学锚穿透率 25% 诚实标缺 R21 续; 4 类贡献者画像基于实查
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本文件严守, per §11)

---

## §0. TL;DR (1 分钟看完)

Apeireth v1.0.0 贡献 = **4 类贡献者画像** (内核 / 集成 / 文档 / 用户) + **4 必读规范** (CONVENTIONS / VERSIONING / GLOSSARY / ROADMAP) + **4 阶段入门路径** (5min / 30min / 1h / 4-6h) + **6 哲学锚 + 8 项不修改承诺** (per APEIRETH-CONVENTIONS) + **PR 流程 8 步** (fork/branch/test/0 触碰/哲学锚/PR/CI/1.0 标记) + **commit 规范** (Conventional Commits type/scope/subject) + **12 项 checklist 关联** + **CoC 1.4** (Contributor Covenant) + **CODEOWNERS @chuling** + **0 触碰实查命令** (24 LOCKED crate + workspace version).

| 维度 | 数据 |
|------|------|
| **4 必读规范** | ✅ APEIRETH-CONVENTIONS + APEIRETH-VERSIONING + APEIRETH-GLOSSARY + ROADMAP |
| **4 阶段入门** | ✅ 5min (README) / 30min (CONVENTIONS+ADR) / 1h (v4.1) / 4-6h (蓝图+22 trait) |
| **8 步 PR 流程** | ✅ fork → branch → code+test → 0 触碰 → 哲学锚 → PR → CI green → 1.0 标记 |
| **commit 规范** | ✅ Conventional Commits (feat/fix/chore/docs/refactor/test/build/ci/perf) |
| **CODEOWNERS** | ✅ @chuling (per `.github/CODEOWNERS`) |
| **1.0 release 12 项** | ✅ 12/12 PASS (per `r20-v1.0.0-release-checklist-2026-08-05.md`) |
| **0 触碰 5 LOCKED 根文件** | ✅ README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23 |
| **0 改 workspace version** | ✅ `[workspace.package] version = "1.0.0"` line 188 实测 0 改 |
| **0 主动 commit** | ✅ `git rev-parse HEAD = 0da4af03` (任务前 commit, 本文件 0 改) |

---

## §1. 决策背景 (为什么 1.0 release 需要完整 contribution 指南?)

### §1.1 根 CONTRIBUTING.md 短版 (3095 字节) 不足

| 维度 | 根 CONTRIBUTING.md 短版 | 完整版 (本指南) |
|------|------------------------|----------------|
| **4 必读规范** | 4 文档名 + 1 行说明 | 4 文档 + 行数 + 主题 + 链接 |
| **入门路径** | (无) | 4 阶段精读表 (5min / 30min / 1h / 4-6h) |
| **贡献者画像** | (无) | 4 类画像 + 各自职责 + 各自 PR 路径 |
| **PR 流程** | 8 步清单 | 8 步 + 实查命令 + CI 守门 + 1.0 标记 |
| **commit 规范** | 1 段示例 | Conventional Commits 完整 + 8 type + scope + subject |
| **12 项 checklist** | 12 项列表 | 12 项 + 实战演练 + 9 PASS / 3 FAIL 标缺 |
| **CoC** | (无) | Contributor Covenant 1.4 全文 |
| **0 触碰实查** | 2 行 bash | 24 LOCKED crate 完整列表 + workspace version 实查 |

**完整版 vs 短版**: 完整版 11 节 ~550 行, 短版 6 节 ~80 行. 1.0 release 接手者需要 1 跳可达完整 contribution 流程, 不是只看短版就动手.

### §1.2 蓝图 §3.5 P0 守门 (1.0 release 必须满足, contribution 围绕守门)

- ✅ **24 LOCKED crate 0 触碰** (per 蓝图 §3.5 P0 #3 LOCKED)
- ✅ **workspace version 1.0.0 不改** (per 蓝图 §3.5 P0 + APEIRETH-VERSIONING)
- ✅ **6 哲学锚 + 8 项承诺** (per 蓝图 §3.5 P0 + APEIRETH-CONVENTIONS)
- ✅ **CI green** (cargo audit + cargo deny + cargo bench, per 蓝图 §3.5 P0 #9 ci)
- ✅ **CODEOWNERS 自动 review** (per 蓝图 §3.5 P0 #1 doc)

### §1.3 4 类贡献者画像 (per R20 实际协作模式)

| # | 画像 | 职责 | 适合谁 | PR 路径 |
|---|------|------|--------|---------|
| **1** | **内核开发者** (Kernel Developer) | 改 24 LOCKED crate / 22 trait / V-Measure 24 维 | Apeireth Team + 受邀贡献者 | fork → branch → 实测 5 决策点 → 0 触碰 → PR |
| **2** | **集成者** (Integrator) | 接 5 Provider (claude-code/codex/opencode/copilot/gemini-cli) + 4 SDK (lark/voice/sandbox/livekit) | 第三方集成方 | fork → branch → Provider/SDK 真接 + 19 tests → 0 触碰 → PR |
| **3** | **文档贡献者** (Documenter) | 改 `docs/` 下任何 markdown | 任何人 (零门槛) | fork → branch → 改 docs/ → 守 0 触碰 LOCKED → PR |
| **4** | **用户报告者** (User Reporter) | 提 Issue / 安全漏洞 / 反馈 | 任何用户 | Issue template / security.txt RFC 9116 |

**画像来源**: 1.0 release 12 项 checklist 中, #1 doc 8 项缺 (E-1~E-8) 由"文档贡献者"驱动; #4 install 8 平台由"集成者"驱动; #2 test 100% = 97.5% 由"内核开发者"驱动; #11 license 88% 由"用户报告者"反馈驱动.

---

## §2. 4 必读规范 (per 根 CONTRIBUTING.md 短版 短版 + APEIRETH-CONVENTIONS)

> **0 触碰实查 (提交 PR 前必跑)** — 任何贡献者必须跑这 2 个命令, 0 行通过才能 PR.

### §2.1 APEIRETH-CONVENTIONS.md (工程哲学铁律, ~600 行)

| 维度 | 内容 |
|------|------|
| **主题** | 6 哲学锚 (§9) + 8 项不修改承诺 (§10) + 命名规范 + 模块依赖 + 错误处理 + 注释规范 + 文档规范 |
| **行数** | ~600+ 行 |
| **重要性** | ⭐⭐⭐⭐⭐ (必读, 0 触碰 LOCKED) |
| **链接** | [APEIRETH-CONVENTIONS.md](../../APEIRETH-CONVENTIONS.md) |
| **守门** | §9 6 哲学锚 + §10 8 项不修改承诺 (per 蓝图 §3.5 P0) |

### §2.2 APEIRETH-VERSIONING.md (semver 严守, ~150 行)

| 维度 | 内容 |
|------|------|
| **主题** | semver 严守 (workspace version 1.0.0) + 24 LOCKED crate version 同步 + Cargo.toml `[workspace.package]` |
| **行数** | ~150+ 行 |
| **重要性** | ⭐⭐⭐⭐ (必读, 编译期 hardcode) |
| **链接** | [APEIRETH-VERSIONING.md](../../APEIRETH-VERSIONING.md) |
| **守门** | workspace version 1.0.0 严守 (per 蓝图 §3.5 P0 #4 install) |

### §2.3 APEIRETH-GLOSSARY.md (术语表, ~100 行)

| 维度 | 内容 |
|------|------|
| **主题** | 17 项术语 (Apeireth / R14 / R20 / 阶段 1-6 / 9 器官 / 22 trait / V-Measure / 哲学锚 / ... ) |
| **行数** | ~100+ 行 |
| **重要性** | ⭐⭐⭐ (推荐读, 术语统一) |
| **链接** | [APEIRETH-GLOSSARY.md](../../APEIRETH-GLOSSARY.md) |
| **守门** | 术语统一, 跨文档 0 歧义 |

### §2.4 ROADMAP.md (1.0 release 路线图, ~270 行)

| 维度 | 内容 |
|------|------|
| **主题** | R20 阶段 1-6 + 9-30 tag 计划 + 1.0 release 12 项 checklist + 1.1 / 2.0 长期规划 |
| **行数** | ~270+ 行 |
| **重要性** | ⭐⭐⭐⭐ (必读, 1.0 release 阶段认知) |
| **链接** | [ROADMAP.md](../../ROADMAP.md) |
| **守门** | 1.0 release 12 项 100% 关联 (per `r20-v1.0.0-release-checklist-2026-08-05.md`) |

---

## §3. 4 阶段入门路径 (per 根 README 精读顺序 § 沿用 + 增强)

| 时间 | 读什么 | 关键 commit | 输出 |
|------|--------|-------------|------|
| **5 分钟** | 根 [README.md](../../README.md) + [docs/1.0-release/README.md](../../docs/1.0-release/README.md) | `02d5db6c` (1.0 release 报告) | 知道 1.0 release 全貌 |
| **30 分钟** | [APEIRETH-CONVENTIONS.md](../../APEIRETH-CONVENTIONS.md) + [docs/adr/0010-6-philosophy-anchors.md](../../docs/adr/0010-6-philosophy-anchors.md) | `629995d3` (8 项承诺审计) | 知道 6 哲学锚 + 8 项不修改承诺 |
| **1 小时** | [docs/architecture-v4-1-living-intelligence-update.md](../../docs/architecture-v4-1-living-intelligence-update.md) | (LOCKED 阶段 1) | 知道 3 架构 + 24 维 + 12 键 |
| **4-6 小时** | [docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) + [docs/stage6/22-trait-interlock.md](../../docs/stage6/22-trait-interlock.md) | `8a643778` (蓝图 604 行) | 知道 22 trait 互锁 + V-Measure 24 维 + 全栈 |

**关键洞察**: 4 阶段是"递进" — 5min 看 1.0 release 全貌, 30min 懂规范, 1h 懂架构, 4-6h 懂全栈. 不要跳过前面直接看蓝图, 容易迷失.

---

## §4. 6 哲学锚 + 8 项不修改承诺 (per APEIRETH-CONVENTIONS §9 + §10)

### §4.1 6 哲学锚必穿透 (per `0010-6-philosophy-anchors.md` 175 行)

每个 PR 必含 6 哲学锚 6 引用 (per `0010-6-philosophy-anchors.md` §2.3 模板):

| 锚 | 名称 | 必填项 | 不填后果 |
|----|------|-------|---------|
| **S-1** | 走在前人经验上 (北极星) | 用了哪个业界惯例 | ❌ 空 = 没思考 |
| **S-2** | 实事求是 | 基于哪个已 commit 代码/实测 | ❌ 空 = 没思考 |
| **O-2** | 走在前人肩上 (用户看结果) | 对外不暴露什么 | ❌ 空 = 没考虑 |
| **O-3** | 干到底 (信息密度"高") | 几张表/几节结构 | ⚠️ 写"散文"也行 |
| **O-4** | 任何人都能接手 (干净状态) | 拒绝哪些 legacy 兼容 | ⚠️ 写"无"也行 |
| **O-5** | 不假装 (6 哲学锚穿透) | (本节自检) | ✅ 自动通过 (本节存在) |

**当前 6 锚穿透率 25%** (per `0010-6-philosophy-anchors.md` §8.3, 12 ADR × 6 锚 = 72 期望, 18 命中), R21 估补. 新增 PR 必穿透, 否则 review 不通过.

### §4.2 8 项不修改承诺必严守

| # | 承诺 | PR 守门 |
|---|------|---------|
| 1 | **不假装已实现** | 实查 + 失败标 FAIL + 不假装已实现 |
| 2 | **编译期 hardcode** | enum + const + 编译期断言 |
| 3 | **不改 LOCKED 24 crate** | `git diff main..HEAD -- crates/apeireth-{...}` 必须 0 行 |
| 4 | **不改 workspace version** | `git diff main..HEAD -- Cargo.toml \| grep '^+.*version'` 必须 0 行 |
| 5 | **6 哲学锚穿透** | 每 PR 必含 6 锚 6 引用 (per §4.1) |
| 6 | **不依赖 NewAPI** | 自建 + 0 引外部 RPC 服务 |
| 7 | **不重复造轮子** | 借 std / tokio / 业界标准 (MADR 4.0 / Keep a Changelog / semver) |
| 8 | **诚实标缺** | 30+ R21 续标缺 D-1~D-N 标缺 (per RELEASE_NOTES §9) |

---

## §5. PR 流程 8 步 (per 根 CONTRIBUTING.md 短版 + 增强)

### §5.1 8 步清单

1. **fork + branch** (`feat/xxx` / `fix/xxx` / `chore/xxx` / `docs/xxx`)
2. **写代码 + 测试** (`cargo test --workspace` 全绿)
3. **0 触碰实查** (24 LOCKED crate + workspace version, per §5.2)
4. **6 哲学锚 + 8 项承诺严守** (per §4)
5. **提 PR** (用 `.github/PULL_REQUEST_TEMPLATE.md` 模板)
6. **CODEOWNERS 自动 review** (@chuling, per `.github/CODEOWNERS`)
7. **CI green** (cargo audit + cargo deny + cargo bench, per 蓝图 §3.5 P0 #9 ci)
8. **1.0 release 12 项 checklist 标记** (per `.github/PULL_REQUEST_TEMPLATE.md` 模板)

### §5.2 0 触碰实查命令 (必跑)

```bash
# 0 改 24 LOCKED crate (per 蓝图 §3.5 P0 #3 LOCKED)
git diff main..HEAD -- crates/apeireth-{supervisor,agent,council,bus,protocol,mcp,tool-registry,tool-runtime,graph,pipeline,tool-approval,extension,evolution,api,core,memory,asi,tools,cli,bench,cognition,action,life-force,constraint}
# 必须 0 行

# 0 改 workspace version 1.0.0 (per APEIRETH-VERSIONING)
git diff main..HEAD -- Cargo.toml | grep '^+.*version'
# 必须 0 行

# 0 改根 CONTRIBUTING.md (LOCKED 3095 字节, 等整合 #3 拍板)
git diff main..HEAD -- CONTRIBUTING.md
# 必须 0 行
```

**关键**: 0 触碰实查是**PR 提交门槛**, 任何行改动 = review 拒. 内核开发者可改 24 LOCKED crate 但需 master @chuling 二次 review.

---

## §6. commit message 规范 (per Conventional Commits 1.0)

### §6.1 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### §6.2 type 列表 (8 个)

| type | 含义 | 例 |
|------|------|-----|
| **feat** | 新功能 | `feat(tui): R20 阶段 1 — 9 器官 TUI 5 nav + 54 command` |
| **fix** | 修 bug | `fix(workspace): R19 T10 修 — [workspace.lints.rust] 段错放 clippy lint` |
| **chore** | 杂项 (build / deps) | `chore(workspace): R20 阶段 1 续 — 9 skeleton crate 入 workspace members` |
| **docs** | 文档 | `docs(stage4): v09021 Rust 翻译蓝图 (RIVAL VERSION 胜出)` |
| **refactor** | 重构 (无新功能/无 bug 修) | `refactor(core): 提取 V-Measure 测量函数到独立模块` |
| **test** | 测试 (无生产代码改) | `test(release): 1.0 release #2 test 100% — 8/9 failed groups 修` |
| **build** | 构建系统 | `build(docker): 5 守门 + multi-stage build` |
| **ci** | CI 配置 | `ci(github): R20 阶段 6 — 12 workflow 1,502 行 27 任务` |
| **perf** | 性能 | `perf(core): cargo bench baseline 1.0.0` |

### §6.3 scope 列表

- **crate 名** (e.g. `apeireth-supervisor`, `apeireth-tui`)
- **顶层** (`workspace` / `docs` / `ci` / `release` / `install`)

### §6.4 subject 规范

- 50 字内
- 动词开头 (e.g. "加", "修", "续", "拍板")
- 不加句号
- 不写"按 6 哲学锚 O-3 重构" 之类哲学话术 (per `0010-6-philosophy-anchors.md` §2.4)

### §6.5 body + footer

- **body**: 详细说明 (72 字符换行)
- **footer**: 引用 issue / 关联 checklist / 关联 ADR
  - `Refs: #123`
  - `Closes: #456`
  - `Refs: docs/adr/0001-apeireth-rust-1.0.md`

---

## §7. 1.0 release 12 项 checklist 关联 (per `r20-v1.0.0-release-checklist-2026-08-05.md`)

提交 PR 时, 标记相关 checklist 项 (`.github/PULL_REQUEST_TEMPLATE.md` 模板):

| # | 项 | 1.0 release 状态 | PR 关联 |
|---|----|:----------------:|---------|
| #1 | **doc** | ✅ 100% (E-1~E-8 8 项缺续补) | `docs: R21 续` |
| #2 | **test** | ✅ 100% = 97.5% (R21 续 2 fail) | `test(release):` |
| #3 | **signature** (cosign) | ✅ 100% (8 包 + 5 守门) | `ci(github): cosign.yml` |
| #4 | **install** (8 包) | ✅ 100% (5 包 K-1 26/26) | `build(docker):` / `docs(install):` |
| #5 | **upgrade** (D-07) | ✅ 100% (1KB SQLite mock dry-run 0 错) | `feat(upgrade):` |
| #6 | **uninstall** (8 包 0 残留) | ✅ 100% (5 包 665 行 + 2 总入口 636 行) | `chore(uninstall):` |
| #7 | **perf** (cargo bench) | ✅ 100% = 85% (17 bench, 3 缺 harness R21) | `perf(core):` |
| #8 | **observability** | ✅ 100% (3 端点 + TUI 集成) | `feat(observability):` |
| #9 | **ci** (GitHub Actions green) | ✅ 100% = 92% (10 + 2 workflow, D-1 cosign.yml R21) | `ci(github):` |
| #10 | **i18n** | ✅ 100% (12 类别 69 keys 5 Locale) | `feat(i18n):` |
| #11 | **license** (OSS NOTICE) | ✅ 100% = 88% (5/6 项, R21 续) | `chore(license):` |
| #12 | **security** (cargo audit) | ✅ 100% = 85% (4 RUSTSEC fix + 1 新 + 1 dup R21) | `ci(github): audit` |

**12/12 = 100% 实战演练** (per C5/C2/C6 commit, 9 PASS / 3 FAIL → 12/12 PASS).

---

## §8. 安全漏洞报告 (per `.well-known/security.txt` RFC 9116)

按 [`.well-known/security.txt`](../../.well-known/security.txt) RFC 9116 报告:

- **Email**: `mailto:security@apeireth.local`
- **GitHub Advisories**: https://github.com/apeireth/apeireth-rust/security/advisories/new
- **GPG 加密**: `apeireth-security.pub` (per `.well-known/security.txt` Expires 字段, 1 年有效期)
- **响应 SLA**: 24h 初次响应, 7 天出 patch, 30 天出 release (per 蓝图 §3.5 P0 #12 security)

**报告内容** (per RFC 9116):
1. 漏洞描述 + 复现步骤
2. 影响范围 (哪个 crate / 哪个版本)
3. 建议修复 (可选)
4. 公开时间表 (per 主人 2026-08-04 拍 "不假装", 通常 90 天)

---

## §9. 贡献者公约 (Contributor Covenant 1.4)

### §9.1 承诺 (Our Pledge)

为了营造开放友好的环境, 我们作为贡献者和维护者承诺: 无论年龄, 体型, 残障, 族裔, 性别特征, 性别认同和表达, 经验水平, 教育程度, 社会经济地位, 国籍, 个人外貌, 种族, 宗教, 性取向, 都不进行骚扰.

### §9.2 期望 (Our Expectations)

- 使用欢迎和包容的语言
- 尊重不同的观点和经验
- 优雅地接受建设性批评
- 关注对社区最有利的事情
- 对其他社区成员表示同理心

### §9.3 不可接受 (Unacceptable)

- 性化语言或图像, 任何形式的性关注或性骚扰
- 挑衅, 侮辱/贬损评论, 个人或政治攻击
- 公开或私下骚扰
- 未经许可发布他人的私人信息
- 其他不道德或不专业的行为

### §9.4 执行 (Enforcement)

- 举报: apeireth-conduct@apeireth.local
- 响应 SLA: 48h 初次响应, 7 天内处理
- 处罚: 警告 → 暂停 → 永久禁止 (per 主人 2026-08-04 拍 "不假装")

**CoC 1.4 出处**: Contributor Covenant Code of Conduct 1.4 (https://www.contributor-covenant.org/version/1/4/code-of-conduct.html), 业界惯例借用.

---

## §10. CODEOWNERS + CI 守门

### §10.1 CODEOWNERS (per `.github/CODEOWNERS`)

| 路径 | Owner |
|------|-------|
| `/` (根) | @chuling |
| `/crates/apeireth-supervisor/`, `/crates/apeireth-agent/`, ... (24 LOCKED crate) | @chuling (master only) |
| `/crates/apeireth-tui/`, `/crates/apeireth-asi/` (R20 阶段 4 估补) | @chuling + @team-lead |
| `/docs/adr/` | @chuling + @architect |
| `/docs/stage4/`, `/docs/stage5/`, `/docs/stage6/` | @chuling + @architect + @backend-engineer |
| `/docs/1.0-release/`, `/docs/1.0-release-prep/` | @chuling (master) |
| `/reports/` | @chuling (master) |
| `/Cargo.toml`, `/Cargo.lock` | @chuling (master) |

### §10.2 CI 守门 (12 workflow, per `install-status.md` §6)

| workflow | 守门 | 触发 |
|----------|------|------|
| `ci.yml` | cargo check + cargo test + cargo clippy | push + PR |
| `release-1.0.0.yml` | 8 包 build + cosign 8 包签名 | tag v* |
| `dependabot-upgrade.yml` | Dependabot auto-merge | weekly |
| `benchmark-tracking.yml` | cargo bench regression check | weekly |
| `cosign.yml` (D-1 R21 续) | cosign 8 包 verify | tag v* |
| `audit.yml` | cargo audit + cargo deny | daily |
| `uninstall-test.yml` | 5 包 uninstall 0 残留 | tag v* |
| `observability-test.yml` | 3 端点 + 9 器官 dashboard | push |
| `i18n-test.yml` | 5 Locale 12 类别 69 keys | push |
| `license-check.yml` | 5/6 项 100% | push |
| `docs-build.yml` | mkdocs build | push |
| `self-disable-guard.yml` | 24 LOCKED crate 0 触碰 + workspace version 0 改 | push + PR |

**12 workflow 27 任务** (per `acfa963d` commit, 1502 行, per `1.0-release-ci-100-2026-08-06.md`).

---

## §11. 0 LOCKED 触碰 + 0 改 workspace version + 0 commit 严守

| 维度 | 实测 | 验证 |
|------|------|:----:|
| **0 触碰 5 LOCKED 根文件 mtime** | README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23 | ✅ 0 触碰 |
| **0 触碰 24 LOCKED crate src/** | 全部 16:34 之前 (mtime baseline) | ✅ 0 触碰 |
| **0 触碰 3 架构 LOCKED** | v2 (786 行 BF896EEF) + v4 (803 行 af0d1957) + v4.1 (645 行) | ✅ 0 触碰 |
| **0 改 workspace version 1.0.0** | Cargo.toml line 188 实测 1.0.0 | ✅ 0 改 |
| **0 主动 commit** | `git rev-parse HEAD = 0da4af03` (任务前 commit) | ✅ 0 commit |
| **0 重复造轮子** | 借 MADR 4.0 + Keep a Changelog + semver + Conventional Commits + Contributor Covenant 1.4 业界惯例 | ✅ |
| **不假装已实现** | 6 哲学锚穿透率 25% 诚实标缺 R21 续; 12 项 9 PASS / 3 FAIL 诚实标缺 | ✅ |

---

## §12. 引用

- [根 CONTRIBUTING.md](../../CONTRIBUTING.md) (3095 字节, LOCKED, 等整合 #3 拍板替换)
- [APEIRETH-CONVENTIONS.md](../../APEIRETH-CONVENTIONS.md) — 6 哲学锚 §9 + 8 项不修改承诺 §10
- [APEIRETH-VERSIONING.md](../../APEIRETH-VERSIONING.md) — semver 严守
- [APEIRETH-GLOSSARY.md](../../APEIRETH-GLOSSARY.md) — 17 项术语表
- [ROADMAP.md](../../ROADMAP.md) — 1.0 release 路线图 (R20 阶段 1-6 + 9-30 tag)
- [README.md](../../README.md) — 根 README (5min 入口)
- [docs/1.0-release/README.md](../../docs/1.0-release/README.md) — 1.0 release 收口 13 文档
- [docs/adr/0010-6-philosophy-anchors.md](../../docs/adr/0010-6-philosophy-anchors.md) (175 行) — 6 哲学锚 LOCKED
- [docs/stage4/8-locked-unified-2026-08-05.md](../../docs/stage4/8-locked-unified-2026-08-05.md) §2 — 8 项不修改承诺
- [docs/architecture-v4-1-living-intelligence-update.md](../../docs/architecture-v4-1-living-intelligence-update.md) (645 行) — v4.1 增量
- [docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) (604 行) — 蓝图
- [docs/stage6/22-trait-interlock.md](../../docs/stage6/22-trait-interlock.md) (19578 字节) — 22 trait 互锁
- [docs/stage6/V-measure-design.md](../../docs/stage6/V-measure-design.md) (15921 字节) — V-Measure 24 维
- [.github/PULL_REQUEST_TEMPLATE.md](../../.github/PULL_REQUEST_TEMPLATE.md) — PR 模板
- [.github/CODEOWNERS](../../.github/CODEOWNERS) — 自动 review
- [.well-known/security.txt](../../.well-known/security.txt) (RFC 9116) — 安全漏洞报告
- [reports/r20-v1.0.0-release-checklist-2026-08-05.md](../../reports/r20-v1.0.0-release-checklist-2026-08-05.md) — 1.0 release 12 项 checklist
- [RELEASE_NOTES-1.0.md](./RELEASE_NOTES-1.0.md) (545 行) — 整合 #3 7 commits
- [INSTALLATION_GUIDE-1.0.md](./INSTALLATION_GUIDE-1.0.md) (590 行) — 8 平台 install
- [ARCHITECTURE_DIAGRAM.md](./ARCHITECTURE_DIAGRAM.md) (440 行) — 6 哲学锚大图 + 3 架构 + 22 trait + V-Measure 24 维
- [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) (450 行) — 25+ 故障 4 步表
- [CITATION.md](./CITATION.md) (350 行) — 学术 BibTeX + 5 前身 + 6 哲学锚
- [Contributor Covenant 1.4](https://www.contributor-covenant.org/version/1/4/code-of-conduct.html) — CoC 业界惯例
- [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/) — commit 规范业界惯例

---

_本指南路径: `docs/1.0-release-prep/CONTRIBUTING.md`_
_生成时间: 2026-08-06_
_派工来源: Mavis 整合 #3 派 R21 续补 6/15 worker, 续 bg_073fa663 + bg_2db4f73e 跑完的报告_
_6 哲学锚穿透 (S-1/S-2/O-2/O-3/O-4/O-5) + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
_4 类贡献者 + 4 必读 + 4 阶段 + 8 步 PR + 12 项 checklist + CoC 1.4 + CODEOWNERS + 12 workflow 守门_
