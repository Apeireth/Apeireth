<!--
  PR Template — Apeireth (R26+ 工程化补全)
  模式: tokio/qdrant 业界惯例 (markdown checklist 1:1)
  参考:
    - https://github.com/tokio-rs/tokio/blob/master/.github/PULL_REQUEST_TEMPLATE.md
    - https://github.com/qdrant/qdrant/blob/master/.github/PULL_REQUEST_TEMPLATE.md
  硬约束 (per CONTRIBUTING.md §0 触碰实查 + rust.yml hard-walls job):
    - 0 触碰受锁源码（**清单与数字见下方「自动生成」块** ⇒ 不要在此手写）
    - 0 改 workspace.version（**当前值见下方「自动生成」块**）
    - 0 改 R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
    - R11 baseline 在 `legacy/donor/apeireth-asi/tests/integration_r_measure.rs` (编译期 const)
    - cargo test pass (cargo test --workspace 全绿)
    - 0 假装 (不假装已实现, skeleton 标 ⏳)
-->

<!-- BEGIN:auto-facts -->
<!-- 本块由 `scripts/gen_gate_manifest.py` 从 `docs/architecture/contract.toml` **生成** ⇒ **请勿手改** -->
<!-- 手改会被 `scripts/gen_gate_manifest.py --check` 拦下（守卫会红） -->

### 仓库事实（**自动生成** · 真源 = `docs/architecture/contract.toml`）

- **workspace.version**：`2.0.0-rc.4`（契约 `1.0` · 状态 `adjudicated`）
- **workspace 成员**：`30`（产品 crate `29`）
- **受锁源码（0 触碰 · CI hard-walls 实际清单）**：
  - `crates/foundation/core/src/eight_anchors.rs`
  - `crates/foundation/core/src/onion.rs`
  - `crates/foundation/core/src/philosophy.rs`
- **哲学锚**：**9** 条（`docs/01-architecture/philosophy.md`）
- **workflow**：**42** 个（`.github/workflows/*.yml`）
- **crate**：**0** 个（`crates/*/Cargo.toml`）
- **门禁登记表**：`docs/architecture/contract.toml` 的 `[ci_gates]`（共 **34** 条 · required **32** 条）
  - 本地可跑（`27` 条）：`docs-truth` · `doc-class` · `doc-caliber` · `doc-numbers` · `doc-paths` · `docs-links` · `batch-status` · `claims-verify` · `arch-strict` · `arch-doc` · `crate-budget` · `dead-code` · `unwired` · `unwired-groups` · `stale-binaries` · `mod-targets` · `guard-wiring` · `quality-metrics` · `golden-baseline` · `golden-no-leak` · `internal-golden` · `d4-ledger` · `wiring-report` · `verify-ledger` · `toolchain-pin` · `gate-manifest` · `self-status-vocab`
  - 完整清单与命令：**不要在这里复制**，改看契约 `[ci_gates]`（`scripts/harness.py plan` 会按变更范围列出该跑哪几条）

### 硬约束（**数字全部来自上面那块，不要在此手写**）

- [ ] **0 触碰受锁源码**（清单见上，CI `hard-walls` 逐文件 word-diff 守）
- [ ] **0 改 `workspace.version`**（除非带 release 拍板证据，见 `docs/versioning/`）
- [ ] `cargo test --workspace` 全绿 + `cargo test --doc` 全绿
- [ ] **0 假装**（skeleton 标 ⏳，不假装已实现，编译期 hardcode 守门）
<!-- END:auto-facts -->

## Description

<!-- 1-3 句话讲清楚这个 PR 干了什么. 引用 issue: Fixes #123 / Closes #456 -->

## Motivation & Context

<!-- 为什么需要这个改动? 引用 6 哲学 anchor (per ../docs/archive/conventions/09-anchor.md) 哪一项?
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

## R26+ 5 项硬约束 (per [CONTRIBUTING.md §0 触碰实查](https://github.com/Apeireth/Apeireth/blob/main/CONTRIBUTING.md))

<!-- 任务要的 5 项 1:1, 提交前必跑实查 -->

- [ ] **0 触碰受锁源码**（**逐个文件清单见上方「自动生成」块** ⇒ CI `hard-walls` 按该清单 word-diff 守；此处**不手写清单**，避免副本过期）
- [ ] **0 改 `workspace.version`**（**当前值见上方「自动生成」块**；改动须带 release 拍板证据，per `docs/versioning/`）
- [ ] **0 改 R11 baseline 3 值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, in `legacy/donor/apeireth-asi/tests/integration_r_measure.rs`)
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
- [ ] 涉及 4 关 / 12 键 / 5 重守门 / 24 维 / 9 器官 / **哲学锚（数量见上方自动块）** → 同步 `docs/conventions/` + `docs/glossary/`
- [ ] 涉及新 public API → 加 `///` rustdoc (per `cargo doc` 通过)
- [ ] 涉及新 submodule → 加 `mod.rs` 顶部 30+ 行守门 (per 主人偏好 O-4)

## 6 哲学 anchor 穿透 (per `docs/conventions/09-anchor.md`)

<!-- 每个 PR 必含的 6 段, 删掉不适用的项 -->

- [ ] **S-1** 走在前人经验上 (北极星): 严守既定设计口径? 0 发明性重设计?
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

## 1.0 release 12 项 checklist (per [scripts/release-1.0-checklist.sh](https://github.com/Apeireth/Apeireth/blob/main/scripts/release-1.0-checklist.sh))

<!-- 仅在 1.0 release 收尾时勾选, 日常 PR 留空 -->

- [ ] #1 doc (README + CHANGELOG + 4 docs 站)
- [ ] #2 test (cargo test --workspace 0 fail)
- [ ] #3 security (5 守门 + cargo audit + cargo deny)
- [ ] #4 install (8 包 dry-run install 0 错)
- [ ] #5 upgrade (D-07 1 次迁移脚本 dry-run 0 错)
- [ ] #6 uninstall (apt remove / dnf remove / brew uninstall 0 残留)
- [ ] #7 perf (cargo bench baseline 0 regression, P95 < 2s)
- [ ] #8 observability (tracing + Prometheus metrics endpoint 200)
- [ ] #9 ci (GitHub Actions green, 5 守门 + 7 matrix)
- [ ] #10 i18n (中英文档 0 missing)
- [ ] #11 license (Apache 2.0 + NOTICE + 第三方 LICENSE)
- [ ] #12 signature (8 形态签名: deb.gpg / rpm.gpg / brew / scoop / tarball / image.cosign / git.tag.gpg / crates.io.token)

## Checklist (PR 提交前自查)

<!-- 提交前必跑 -->

- [ ] 上面 R26+ 5 项硬约束全勾
- [ ] 上面测试段 6 项全勾
- [ ] 上面文档段 5 项对应勾
- [ ] 上面 6 哲学 anchor 段 6 项对应勾
- [ ] 上面 8 项不修改承诺段 8 项对应勾
- [ ] 跟现有 workflow 与受锁源码 0 冲突（**清单见上方自动块**）
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
