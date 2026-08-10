<!--
  PR Template — Apeireth (R26+ 工程化补全)
  模式: tokio/qdrant 业界惯例 (markdown checklist 1:1)
  参考:
    - https://github.com/tokio-rs/tokio/blob/master/.github/PULL_REQUEST_TEMPLATE.md
    - https://github.com/qdrant/qdrant/blob/master/.github/PULL_REQUEST_TEMPLATE.md
  硬约束 (per CONTRIBUTING.md §0 触碰实查):
    - 0 触碰 24 LOCKED crate
    - 0 改 workspace.version (1.1.0 semver 严守)
    - 0 改 R11 baseline 3 值 (V1141 / V1131 / V1136, in `apeireth-asi/src/lib.rs`)
    - cargo test pass (cargo test --workspace 全绿)
    - 0 假装 (不假装已实现, skeleton 标 ⏳)
-->

## Description

<!-- 1-3 句话讲清楚这个 PR 干了什么. 引用 issue: Fixes #123 / Closes #456 -->

## Motivation & Context

<!-- 为什么需要这个改动? 引用 6 哲学 anchor (per docs/conventions/09-anchor.md) 哪一项?
     - S-1 北极星导向
     - S-2 实事求是
     - O-2 走在前人肩上
     - O-3 干到底
     - O-4 任何人都能接手
     - O-5 不假装 -->

## Type of change

<!-- 删掉不适用的项, 保留 1 项 -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] CI / tooling update
- [ ] Refactor (no functional change)

## R26+ 5 项硬约束 (per [CONTRIBUTING.md §0 触碰实查](https://github.com/apeireth/apeireth-rust/blob/main/CONTRIBUTING.md))

<!-- 任务要的 5 项 1:1, 提交前必跑实查 -->

- [ ] **0 触碰 24 LOCKED crate** (mtime 实查, 24 个 crate: apeireth-{supervisor, agent, council, bus, protocol, mcp, tool-registry, tool-runtime, graph, pipeline, tool-approval, extension, evolution, api, core, memory, asi, tools, cli, bench, cognition, action, life-force, constraint})
- [ ] **0 改 workspace.version** (1.1.0 semver 严守, per `docs/versioning/`)
- [ ] **0 改 R11 baseline 3 值** (V1141 / V1131 / V1136, in `apeireth-asi/src/lib.rs`)
- [ ] **cargo test pass** (`cargo test --workspace` 全绿 + `cargo test --doc` 全绿)
- [ ] **0 假装** (skeleton 标 ⏳, 不假装已实现, 编译期 hardcode 守门)

## 测试 (per CONTRIBUTING.md §PR 流程 第 2 步)

- [ ] `cargo check --workspace` 0 error
- [ ] `cargo test --workspace` 全绿
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` 0 warning (或已说明豁免)
- [ ] `cargo fmt --all -- --check` 0 diff (或已说明豁免)
- [ ] 0 引 NewAPI (per 8 项不修改承诺)
- [ ] 0 重复造轮子 (复用 std / tokio / 业界标准)

## 文档 (per CONTRIBUTING.md §PR 流程 第 4 步)

- [ ] 涉及代码改动 → `CHANGELOG.md` 顶部 `## [Unreleased]` 段加 1 行
- [ ] 涉及路线图变更 → `ROADMAP.md` 按状态标记 (✅ / ⏳ / ❌)
- [ ] 涉及 4 关 / 12 键 / 5 重守门 / 24 维 / 9 器官 / 6 哲学锚 → 同步 `docs/conventions/` + `docs/glossary/`
- [ ] 涉及新 public API → 加 `///` rustdoc (per `cargo doc` 通过)
- [ ] 涉及新 submodule → 加 `mod.rs` 顶部 30+ 行守门 (per 主人偏好 O-4)

## 6 哲学 anchor 穿透 (per `docs/conventions/09-anchor.md`)

<!-- 每个 PR 必含的 6 段, 删掉不适用的项 -->

- [ ] **S-1** 走在前人经验上 (北极星): 1:1 翻译 v0.9.21 商业版? 0 重设计?
- [ ] **S-2** 实事求是: 实查 5 决策点 (loc / files / deps / hardcode / 0 触碰)?
- [ ] **O-2** 走在前人肩上: 复用 std / tokio / 业界标准?
- [ ] **O-3** 干到底: 测试 N/N passed + cargo check 0 error?
- [ ] **O-4** 任何人都能接手: doc 顶部 30+ 行 + 编译期 hardcode 守门?
- [ ] **O-5** 不假装: skeleton 标 ⏳ + 不假装已实现?

## 8 项不修改承诺 (per `docs/conventions/10-locked.md`)

<!-- 8 项 1:1, 0 触碰 严守 -->

- [ ] 不假装已实现
- [ ] 编译期 hardcode
- [ ] 不改 LOCKED 24 crate
- [ ] 不改 workspace version
- [ ] 6 哲学 anchor 穿透
- [ ] 不依赖 NewAPI
- [ ] 不重复造轮子
- [ ] 诚实标缺

## 1.0 release 12 项 checklist (per [CONTRIBUTING.md §1.0 release 12 项 checklist](https://github.com/apeireth/apeireth-rust/blob/main/CONTRIBUTING.md))

<!-- 仅在 1.0 release 收尾时勾选, 日常 PR 留空 -->

- [ ] #1 doc
- [ ] #2 test
- [ ] #3 signature (cosign)
- [ ] #4 install (8 包)
- [ ] #5 upgrade (D-07 1 次迁移)
- [ ] #6 uninstall (8 包 0 残留)
- [ ] #7 perf (cargo bench)
- [ ] #8 observability
- [ ] #9 ci (GitHub Actions green)
- [ ] #10 i18n
- [ ] #11 license (OSS NOTICE)
- [ ] #12 security (cargo audit)

## Checklist (PR 提交前自查)

<!-- 提交前必跑 -->

- [ ] 上面 R26+ 5 项硬约束全勾
- [ ] 上面测试段 6 项全勾
- [ ] 上面文档段 5 项对应勾
- [ ] 上面 6 哲学 anchor 段 6 项对应勾
- [ ] 上面 8 项不修改承诺段 8 项对应勾
- [ ] 跟现有 18 workflow yml + 18 LOCKED crate 0 冲突
- [ ] 跟 A / B / C / D-2 / D-3 并行任务 0 冲突 (per `reports/agent-a2-final-2026-08-10.md`)
- [ ] commit message 规范: `<type>(<scope>): <subject>`, 例: `feat(workspace): R26 阶段 — dependabot + ISSUE_TEMPLATE 补全`
- [ ] CODEOWNERS 自动 review (per CONTRIBUTING.md §PR 流程 第 6 步)
- [ ] 本地 `cargo test --workspace` 跑过, JUnit 上传 (per rust.yml 1.0)
- [ ] PR title 简明 (50 字内, 动词开头, 不加句号)

## Related issues

<!-- 链接相关 issue / discussion / docs -->

Closes #
Relates to #
Refs docs/

## Breaking changes

<!-- 如果有 breaking change, 详细说明 + migration path -->

- **API 变更**: ...
- **数据迁移**: ...
- **配置文件变更**: ...
- **依赖升级**: ...

---

> 本模板 per tokio/qdrant 业界惯例 + APEIRETH-CONVENTIONS §10 8 项不修改承诺 + CONTRIBUTING.md §0 触碰实查.
> 替代 R20 阶段 1-6 模板 (R20 已于 2026-08-05 收官, 5 段砍到 6 段: 5 硬约束 + 测试 + 文档 + 6 哲学 + 8 项承诺 + 12 项 checklist).
