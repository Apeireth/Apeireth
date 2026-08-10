# 贡献指南 (Apeireth 1.0 release)

> **6 哲学 anchor** (per [`docs/conventions/09-anchor.md`](docs/conventions/09-anchor.md)) + **8 项不修改承诺** (per [`docs/conventions/10-locked.md`](docs/conventions/10-locked.md)) 必读.
>
> **R119-4d 状态 (2026-08-10)**: 顶层 APEIRETH-CONVENTIONS/VERSIONING/GLOSSARY 已下沉到 `docs/conventions/` + `docs/versioning/` + `docs/glossary/`。原意保留, 8 项不修改承诺形式撤销 6 项, 严守 2 项 (R11 baseline 三值 + workspace.version 1.1.0)。

## 必读

- [`docs/conventions/README.md`](docs/conventions/README.md) — 12 子规范系统 (命名空间/路径/ADR/成就/报告/Commit/状态/锚穿透/不修改承诺/Baseline/架构图/文档元信息/修正链)
- [`docs/versioning/README.md`](docs/versioning/README.md) — 7 子系统 (主代码/设计/修正链/R 周期/指标/基线/手册)
- [`docs/glossary/README.md`](docs/glossary/README.md) — 21 词条 (双洋葱/12 键/5 重守门/4 关/3 域/9 阶段/智囊团/电子环等)
- [`ROADMAP.md`](ROADMAP.md) — 时间线骨架 (R14 → R17 → R20 → R38 → R46-R62 → R63-R72 → R78-R113 → R114-R118)

## 0 触碰实查 (提交 PR 前必跑)

```bash
# 0 改 24 LOCKED crate
git diff main..HEAD -- crates/apeireth-{supervisor,agent,council,bus,protocol,mcp,tool-registry,tool-runtime,graph,pipeline,tool-approval,extension,evolution,api,core,memory,asi,tools,cli,bench,cognition,action,life-force,constraint}
# 必须 0 行

# 0 改 workspace version
git diff main..HEAD -- Cargo.toml | grep '^+.*version'
# 必须 0 行
```

## 6 哲学 anchor 必穿透

每个 PR 必含 §1 §2 §3 章节引用:
- §1 S-1 北极星导向: 1:1 翻译 v0.9.21 商业版? 0 重设计?
- §2 S-2 实事求是: 实查 5 决策点 (loc / files / deps / hardcode / 0 触碰)?
- §3 O-2 走在前人肩上: 复用 std / tokio / 业界标准?
- §4 O-3 干到底: 测试 N/N passed + cargo check 0 error?
- §5 O-4 任何人都能接手: doc 顶部 30+ 行 + 编译期 hardcode 守门?
- §6 O-5 不假装: skeleton 标 ⏳ + 不假装已实现?

## 8 项不修改承诺 必严守

- ✅ 不假装已实现
- ✅ 编译期 hardcode
- ✅ 不改 LOCKED 24 crate
- ✅ 不改 workspace version
- ✅ 6 哲学 anchor 穿透
- ✅ 不依赖 NewAPI
- ✅ 不重复造轮子
- ✅ 诚实标缺

## PR 流程

1. fork + branch (`feat/xxx` / `fix/xxx` / `chore/xxx` / `docs/xxx`)
2. 写代码 + 测试 (cargo test --workspace 全绿)
3. 0 触碰实查 (上面命令)
4. 6 哲学 anchor + 8 项承诺严守
5. 提 PR (用 `.github/PULL_REQUEST_TEMPLATE.md` 模板)
6. CODEOWNERS 自动 review (@chuling)
7. CI green (cargo audit + cargo deny + cargo bench)
8. 1.0 release 12 项 checklist 标记

## commit message 规范

```bash
# 格式: <type>(<scope>): <subject>
# type: feat / fix / chore / docs / refactor / test / build / ci / perf
# scope: crate 名 or 顶层 (workspace / docs / ci)
# subject: 50 字内, 动词开头, 不加句号

# 例:
feat(workspace): R20 阶段 1 — 5 P0 crate 加入 workspace members
fix(workspace): R19 T10 修 — [workspace.lints.rust] 段错放 clippy lint
chore(workspace): R20 阶段 1 续 — 9 skeleton crate 入 workspace members
docs(stage4): v09021 Rust 翻译蓝图 (RIVAL VERSION 胜出)
```

## 1.0 release 12 项 checklist

提交 PR 时, 标记相关 checklist 项 (`/PULL_REQUEST_TEMPLATE.md` 模板里):

- #1 doc
- #2 test
- #3 signature (cosign)
- #4 install (8 包)
- #5 upgrade (D-07 1 次迁移)
- #6 uninstall (8 包 0 残留)
- #7 perf (cargo bench)
- #8 observability
- #9 ci (GitHub Actions green)
- #10 i18n
- #11 license (OSS NOTICE)
- #12 security (cargo audit)

## 安全漏洞

按 `.well-known/security.txt` RFC 9116 报告:
- mailto:security@apeireth.local
- https://github.com/apeireth/apeireth-rust/security/advisories/new
