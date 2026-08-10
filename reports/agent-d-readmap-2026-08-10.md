# Agent D — D1 Readmap (2026-08-10 02:55 → 10:00, 7h window)

> **诚实声明**: 任务描述的前提已严重过期。R18 / R19 / R20 / R21 / R25 已做 80% 工作。本 readmap 按"实际现状 + 真正待做"重写,而不是"按字面任务执行"。

## 1. 实际现状 (per ls + read)

### 1.1 `.github/workflows/` (18 个 yml)

| # | 文件 | 状态 | 跑什么 | 模式 |
|---|---|---|---|---|
| 1 | `bench.yml` | ✅ 已有 (R23 #2) | criterion bench 5 个 crate | manual + weekly |
| 2 | `benchmark-tracking.yml` | ✅ 已有 (R20) | bench tracking on push | — |
| 3 | `cargo-audit.yml` | ✅ 已有 (R61) | cargo audit + SARIF | schedule 0 4 * * 0 |
| 4 | `cargo-deny.yml` | ✅ 已有 (R19 #0.2) | 4 类 check (bans/licenses/sources/advisories) | PR + push |
| 5 | `cosign.yml` | ✅ 已有 (1.0) | supply-chain 签名 | — |
| 6 | `coverage.yml` | ✅ 已有 (R18 #3) | tarpaulin → codecov | PR + push |
| 7 | `dependabot-upgrade.yml` | ✅ 已有 (R20 #6) | dependabot PR auto-merge | PR |
| 8 | `eval-live.yml` | ✅ 已有 | eval live | — |
| 9 | `kani.yml` | ✅ 已有 (R18) | cargo kani harness double_onion_sample | PR + push |
| 10 | `miri.yml` | ✅ 已有 (R18 #3) | 3 unsafe crate + nightly | PR + push |
| 11 | `protocol-e2e.yml` | ✅ 已有 (R17 战役 4-4) | 4 协议 e2e 真接 minimaxi | PR + push + dispatch |
| 12 | `release-1.0.0.yml` | ✅ 已有 | 1.0.0 release | tag |
| 13 | `release.yml` | ✅ 已有 | release | — |
| 14 | `rust-ci.yml` | ✅ **已升级** (R18 #1) | **OS matrix (3) + cargo-nextest + JUnit + release-build + battle-1-2 + ci-summary** | PR + push |
| 15 | `rust-lint.yml` | ✅ 已有 (R18 #1) | **clippy 3 档 + rustfmt-nightly** | PR + push |
| 16 | `rustdoc.yml` | ✅ 已有 (R18 #3) | nightly doc -Dwarnings | PR + push |
| — | (目标新增) `rustfmt.yml` | ❌ **待新增** | qdrant 模式 nightly fmt 独立 | — |
| — | (目标新增) `rust.yml` | ❌ **待新增** | qdrant 模式 OS matrix + nextest 独立 | — |

### 1.2 `Cargo.toml` 关键事实 (per read)

- **workspace.package.version = "1.1.0"** ← R119 硬约束 #1 (0 改)
- 91 workspace members (V1306 后: 88 + 3 high risk 修真)
- 实际 92+ crate 与任务描述"92+ crate"基本一致
- `reqwest = "0.12"` workspace dep (0.13 在 transitive)
- `[workspace.lints.rust]` + `[workspace.lints.clippy]` (R19 #0.1 已完成)
- `[profile.release]` opt-level=3 + lto=fat + codegen-units=1 + strip=true

### 1.3 `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

### 1.4 `.config/nextest.toml` (R18 #1 已完成)

```toml
[profile.default]
retries = 0
test-threads = "num-cpus"
failure-output = "immediate-final"
status-level = "fail"
final-status-level = "fail"
slow-timeout = { period = "60s", terminate-after = 2 }

[profile.ci]
inherits = "default"
fail-fast = true
retries = 2
slow-timeout = { period = "90s", terminate-after = 2 }
```

### 1.5 `deny.toml` (R19 #0.2 已完成)

- `[graph] targets` 含 4 三元组 (linux/darwin/windows/android)
- `[licenses] allow` 16 项 (含 Artistic-2.0 + CC0-1.0 通知 v5 组合)
- `[bans] multiple-versions = "deny"` + 50+ skip (windows-* / reqwest / thiserror / 等)
- `[advisories]` db-path + db-urls 已配, **ignore = [] 空** ← 需补全理由
- `[sources]` only crates.io

## 2. 现状 vs 任务描述 (R25 实际)

| 任务描述 | 实际状态 | 行动 |
|---|---|---|
| "现有 rust-ci.yml (一锅炖)" | 已在 R18 #1 升级到 OS matrix + nextest | rust-ci.yml 现在 = 主测试 + release-build + battle-1-2 + ci-summary,非一锅炖 |
| "目标拆成 4 个 workflow: rustfmt / rust-lint / rust / cargo-deny" | rust-lint.yml + cargo-deny.yml 已独立 (R18+R19) | 实际只缺 rustfmt.yml + rust.yml 独立 (qdrant 模式), 共 2 个待新增 |
| "加 OS matrix (ubuntu+windows+macos)" | 已在 rust-ci.yml | 抽到独立 rust.yml (qdrant 模式) |
| "引入 cargo-nextest 替 cargo test" | 已在 rust-ci.yml + .config/nextest.toml | 抽到独立 rust.yml |
| "现有 protocol-e2e.yml" | 已有, 4 协议真接 | 抽 OS matrix / 或保持 |
| "Cargo.lock 790 packages, 0.12+0.13 reqwest / 1+2 thiserror / 5 windows-sys 版本分裂" | 一致 (R19 deny.toml skip 列表已承认) | 在 D4 补 deny.toml ignore CVE 理由 |
| "k8 yaml 0 错" | 待验证 | D5 yamllint 验证 |

## 3. 业界顶尖项目对标 (D1 末)

### 3.1 qdrant 实际模板 (web_fetch 验证 2026-08-10)

**`rust.yml`** (qdrant 已确认存在):
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
steps:
  - cargo build --workspace --tests --locked
  - cargo nextest run --workspace --profile ci --locked
  - upload-artifact: target/nextest/ci/junit.xml
+ process-results job (flaky test issue)
```

**`rust-lint.yml`** (qdrant 已确认存在):
- 1 lint job: nightly fmt --check + 3 档 clippy -- -D warnings
- 1 ast-grep job (qdrant 特色 — Apeireth 暂不学, ast-grep 不熟)

**`rustfmt.yml`** (qdrant 404 — 实际合并到 rust-lint.yml)
→ 但 APEIRETH 走"qdrant 解耦"路线 (rustfmt 独立), 这是 Apeireth 比 qdrant 更严格的做法, 合理

### 3.2 tokio 实际模板 (web_fetch 验证 2026-08-10)

**`ci.yml`** (单超长 workflow, 80+ jobs, Apeireth 不学):
- rust-tests (3 OS matrix + nextest)
- rust-tests all-features
- miri (lib/test/doc 三档)
- fmt + clippy (单 job)
- docs (2 OS)
- coverage 等等
- cross-check (powerpc/android/etc)
- 等等

→ **tokio 模式 ≠ Apeireth 模式**: 业界 2 大派系。tokio 单 workflow, qdrant 多 workflow split。Apeireth 走 qdrant (R18 已定调)

### 3.3 wasmtime (D1 跳过 — 模板已看 cargo-deny.yml 注释引用过)

## 4. cargo-nextest 是否能引 (D1 末)

| 检查项 | 结果 |
|---|---|
| 项目根 `.config/nextest.toml` 存在 | ✅ (R18 #1 已写) |
| `[profile.ci]` 含 fail-fast=true, retries=2 | ✅ |
| `rust-ci.yml` 已在用 `cargo nextest run --workspace --profile ci --locked` | ✅ |
| taiki-e/install-action 引用 | ✅ (rust-ci.yml + rust-lint.yml 都有) |
| 与 qdrant 兼容性 | ✅ 1:1 镜像 qdrant .config/nextest.toml + rust.yml 模式 |

**结论**: cargo-nextest 已完全集成, 任务"引入 cargo-nextest"实质已 done。D3 实际是"把 nextest 测试 job 从 rust-ci.yml 抽到独立 rust.yml"

## 5. 任务重写 (按实际差距)

### 5.1 D1 真实完成

- ✅ 读全 16 个现有 workflow (v2 计数)
- ✅ 读 Cargo.toml / rust-toolchain.toml / nextest.toml / deny.toml
- ✅ 查 qdrant/tokio 实际 .github/workflows/ 模板
- ✅ 确认 cargo-nextest 状态
- ✅ 写本 readmap

### 5.2 D2-D6 真实待做 (与任务描述差异)

| 阶段 | 任务描述 | 真实待做 |
|---|---|---|
| D2 | "新建 rustfmt.yml + rust-lint.yml" | **只新建 rustfmt.yml** (rust-lint.yml 已在 R18 完成,不动) |
| D3 | "新建 rust.yml + 加 OS matrix + cargo-nextest" | **新建 rust.yml** (镜像 qdrant), rust-ci.yml 顶部加 deprecation note |
| D4 | "在 cargo-deny.yml 加 cargo-deny check advisories" | cargo-deny.yml 已在跑全 4 类, **改 deny.toml** (不是 yml) 补 [advisories].ignore CVE 理由 |
| D5 | 验证 | 用 PyYAML 写 yaml 验证脚本 (yamllint 没装可用, actionlint/act 没装) |
| D6 | 写 final report | 诚实记录"任务描述 vs 实际 + 我做了什么 + 缺什么" |

### 5.3 硬约束严守

- ✅ 0 改 workspace.version (1.1.0) — 我不动 Cargo.toml
- ✅ 0 改 R11 baseline 3 值 — 我不动
- ✅ 0 改 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 / 9 器官 — 我不动
- ✅ 0 触碰 24 LOCKED crate — 我只动 .github/workflows/ 新建 + deny.toml + rust-ci.yml 顶部注释
- ✅ 0 主动 commit — 我只本地新建/修改, git add / commit 留给主人
- ⚠️ **0 改任何 .yml 现有文件的行为** — 关键约束: **我只新增 2 个 yml (rustfmt.yml + rust.yml), 改 rust-ci.yml 顶部加注释 (不改变 yaml 行为)**, 旧 yml 全部 0 行为改动
- ✅ 0 触碰任何 src/

### 5.4 0 假装核验

- yaml 0 错 — D5 用 PyYAML 严格 parse 验证
- 每个新 yml 抄 qdrant 实际模板 — 不发明
- D6 final report 列"哪些能跑 / 哪些留给主人" — 不假装已运行
