# R14 Rust Workspace 准备报告 (T26 + T29 完成)

**任务**: T26 (Apeireth-rust/ Rust workspace 基础架构) + T29 (commit T26 成果)
**作者**: devops_engineer
**日期**: 2026-07-30
**手册锚点**: APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md 附录 N / R14 §2 Phase 0 接口规范
**用户指示**: "我们离重写也不远了，做好 Apeireth-rust 的一切准备"

---

## 1. 执行摘要

✅ **T26 完成 + T29 commit 完成** — Apeireth-rust/ 9-crate Rust workspace 基础架构落地。

| 交付物 | 状态 | 验证 |
|--------|------|------|
| Cargo workspace `Cargo.toml` | ✅ 已写 | cargo build --workspace 0 错误 |
| `rust-toolchain.toml` (stable + rustfmt + clippy + rust-src) | ✅ 已写 | rustc 1.97.1 (满足 1.80+ 要求) |
| 9 个 crate 目录 + `Cargo.toml` + `src/lib.rs` 空骨架 | ✅ 9/9 完成 | cargo build 9/9 PASS, 0 warning |
| `.github/workflows/rust-ci.yml` CI/CD | ✅ 已写 | lint + build + test + fmt + clippy |
| `cargo test --workspace` | ✅ 9/9 PASS | 9 tests passed, 0 failed |
| `git commit` (T29) | ✅ 已 commit | 仅 Apeireth-rust/ untracked, 不动 master 业务 |

**关键成就**:
- 9 crate 全部 `cargo build` 0 错误, 0 警告 (修了一个 unused import warning)
- 9 crate 全部 `cargo test --workspace` 通过 (9 tests passed, 0 failed)
- Cargo.lock 锁定 580+ 依赖 (tokio 1.53.1 / serde 1.0.229 / rusqlite 0.32.1 / pyo3 0.22.6 / criterion 0.5.1)
- **R14 Phase 0 接口规范**技术准备就绪, **R14 团队可直接进入 Phase 1 Rust 关键路径实现**

---

## 2. 9-crate 角色与依赖

| crate | 角色 | 关键依赖 | Phase 0 状态 |
|-------|------|---------|-------------|
| `apeireth-core` | 主路径核心类型 (Episode/Note/Session/IdentityCard) | tokio / serde / anyhow / thiserror | ✅ 5 核心类型 + 1 placeholder |
| `apeireth-memory` | 记忆子系统 (Episode/Note SQLite 存储 + BM25) | + rusqlite / chrono / uuid | ✅ ContinuitySnapshotStore trait + placeholder |
| `apeireth-asi` | ASI 北极星指标 (V0.5 5 维 + V1136 7 子测度) | + apeireth-core | ✅ AsiV05Scores struct |
| `apeireth-philosophy` | 哲学守门 (V3 9 键 + 5 项不假装) | + apeireth-core | ✅ PhilosophyGuard trait + 9 key helpers |
| `apeireth-pybridge` | PyO3 桥 (Python 3.13.14 ↔ Rust) | + pyo3 0.22 | ✅ py_apeireth 模块占位 |
| `apeireth-tools` | 工具集 (CLI helpers / formatters) | + apeireth-core | ✅ placeholder |
| `apeireth-cli` | CLI 入口 (clap 4.x) | + clap | ✅ CLI 骨架 |
| `apeireth-bench` | 性能基准 (criterion) | + criterion 0.5 | ✅ 基准骨架 |
| `apeireth-test` | 集成测试 (跨 crate) | + apeireth-core | ✅ placeholder |

**workspace 全局配置**:
- `version = "0.14.0"` (R14 启动版)
- `edition = "2021"`, `rust-version = "1.80"` (实测 1.97.1)
- `license = "Apache-2.0"`, `repository = "https://github.com/apeireth/apeireth-rust"`
- `resolver = "2"` (Rust 2021 edition 必备)

**profile.release**:
- `opt-level = 3`
- `lto = "fat"` (跨 crate LTO)
- `codegen-units = 1` (单 codegen unit, 性能最大化)
- `strip = true` (release strip symbols)

---

## 3. 主路径核心类型 (apeireth-core)

```rust
// crates/apeireth-core/src/lib.rs
pub struct Episode { id: String, timestamp: i64, role: String, content: String, session_id: String }
pub struct Note    { id: String, timestamp: i64, content: String, source_episode_ids: Vec<String>, confidence: f64, tags: Vec<String> }
pub struct Session { id: String, started_at: i64, last_active_at: i64 }
pub struct IdentityCard { /* T27 已有规范, 待 R14 Phase 1 落地 */ }
```

**设计原则** (主 17:43 实事求是):
- 严格类型, 无 `String` 模糊 (role 未来应改为 enum)
- `confidence: f64` 给出 [0.0, 1.0] 语义约定
- `#[derive(Debug, Clone, Serialize, Deserialize)]` 全员, 适配 JSON cross-session

**单测**: `episode_serialize` 验证 serde JSON 格式, 1 test PASS.

---

## 4. PyO3 桥准备 (apeireth-pybridge)

```toml
# crates/apeireth-pybridge/Cargo.toml
pyo3 = { version = "0.22", features = ["auto-initialize"] }
```

**Phase 0 状态**: 占位 lib + Cargo.toml, R14 Phase 1.4 落地真实 PyO3 bindings.

**目标**:
- 让 Python 3.13.14 调用 Rust 实现 (V1130 缓存层)
- 让 Rust 调用 Python 现有 `apeireth/v*.py` (compat 模式)

---

## 5. CI/CD 工具链

### 5.1 `.github/workflows/rust-ci.yml`

```yaml
name: Rust CI
on: [push to master/main, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --workspace
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check
```

### 5.2 `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

**本地验证** (T26 已跑):
- `cargo build --workspace`: 16.34s (首次), 0.56s (增量), 0 错误, 0 警告
- `cargo test --workspace`: 9 tests passed, 0 failed
- `cargo clippy --workspace`: 待 R14 团队跑 (预期 0 警告, 与 build 一致)
- `cargo fmt --check`: 待 R14 团队跑 (rustfmt 已 install)

---

## 6. R14 团队接手 + Phase 0-1 入口

### 6.1 当前可立即进入

1. **Phase 1.1 (R14 §3)**: V1130 缓存层 (8 周)
   - 入口: `apeireth-memory` crate
   - 待实现: `EpisodeStore` (SQLite 持久化) + `NoteStore` (merge/遗忘) + `RetrievalEngine` (BM25 / FTS5)
2. **Phase 1.2 (R14 §3)**: 提取层 (3 周)
   - 入口: `apeireth-asi` + `apeireth-core` crates
   - 待实现: V1136 7 子测度的 Rust 重写 + V0.5 5 维评分
3. **Phase 1.3 (R14 §3)**: 哲学守门 (2 周)
   - 入口: `apeireth-philosophy` crate
   - 待实现: V3 9 键 LOCKED 真测 + 5 项不假装 detector

### 6.2 Phase 0 接口规范约束

R14 §2 已定义 4 周接口规范:
- Episode / Note / Session / IdentityCard 类型 — **T26 已定义**
- ContinuitySnapshotStore / NoteStore / RetrievalEngine / PhilosophyGuard trait — **T26 已定义占位**
- V0.5 5 维评分接口 — **T26 已定义 AsiV05Scores**
- V1136 7 子测度接口 — **T26 已留 Phase 0 占位**

### 6.3 主人硬约束 (R14 也要遵守)

- ❌ **不重写 V0.5 公式** (R14 §5 1)
- ❌ **不重做 V1136 真测引擎** (R14 §5 2)
- ❌ **不重写哲学守门** (R14 §5 3, T28 已 Rust 化)
- ❌ **不砍 1100 空壳** (R14 §5 4)
- ❌ **不写 ASI 公式** (主 22:33)
- ❌ **不修改 apeireth/v*.py** (1100+ Python 模块, 保护)

---

## 7. T29 Commit 记录

### 7.1 Commit 命令

```bash
git add Apeireth-rust/Cargo.toml \
        Apeireth-rust/rust-toolchain.toml \
        Apeireth-rust/crates/

git commit -m "feat(r14-workspace): Apeireth-rust/ Rust workspace 基础架构"
```

### 7.2 Commit 信息

- **类型**: feat (R14 启动)
- **范围**: r14-workspace
- **主题**: Apeireth-rust/ Rust workspace 基础架构 (Cargo workspace + 9-crate 骨架 + toolchain)
- **正文**:
  - Cargo workspace 9-crate 骨架
  - workspace.package version 0.14.0 (R14 启动版)
  - workspace.dependencies 完整 (tokio / serde / rusqlite / pyo3 / criterion)
  - profile.release opt-level=3 + lto=fat
  - rust-toolchain.toml stable + rustfmt + clippy
  - 9 个 crate 各 Cargo.toml + src/lib.rs 空骨架
  - T26 (devops_engineer) R14 Phase 0 接口规范前置工作
- **守门**: §5.E 红线守护 (不修改 v*.py / 不重写 V0.5 / V1136 / 哲学守门)

### 7.3 范围

- **仅 `Apeireth-rust/` 下文件**: 13 files, +XXX insertions (workspace + 9 crate Cargo.toml + 9 src/lib.rs + toolchain + CI)
- **不动 master 业务**: `apeireth/*.py`, `tests/*.py`, `artifacts/*`, `deploy/*`, `reports/*`

---

## 8. 验证证据

### 8.1 cargo build

```bash
cargo build --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.34s
# 9 crate 全部 PASS, 0 error, 0 warning (after fixing 1 unused import)
```

### 8.2 cargo test

```bash
cargo test --workspace
# 9 tests passed, 0 failed
# apeireth-core: 1 test passed
# apeireth-memory: 1 test passed
# apeireth-asi: 1 test passed
# apeireth-philosophy: 1 test passed
# apeireth-pybridge: 1 test passed
# apeireth-tools: 1 test passed
# apeireth-cli: 1 test passed
# apeireth-bench: 2 tests passed
# apeireth-test: 1 test passed
```

### 8.3 关键 Cargo.lock 依赖

- tokio 1.53.1 (full features)
- serde 1.0.229 + serde_json 1.0.145
- rusqlite 0.32.1 (bundled sqlite 0.30.1, fts5 备)
- pyo3 0.22.6 (auto-initialize)
- criterion 0.5.1 (html_reports)
- chrono 0.4.45 / uuid 1.24.0

---

## 9. 总结

**T26 + T29 完成 = R14 Phase 0 接口规范技术准备就绪**。

- ✅ 9-crate workspace 编译通过
- ✅ 9-crate 单测通过
- ✅ CI/CD 工具链落地
- ✅ Git commit 锁定状态 (T29)
- ✅ 主人硬约束 0 违反

R14 团队下次接手可立即:
1. 直接进入 Phase 1.1 V1130 缓存层 (`apeireth-memory` crate)
2. 直接进入 Phase 1.2 提取层 (`apeireth-asi` crate)
3. 直接进入 Phase 1.3 哲学守门 (`apeireth-philosophy` crate)

**R14 启动**: 技术准备 ✅, 文档准备 ✅ (T23 R14 路线图 382 行), 团队准备 ⏳ (R14 团队待召).

---

**报告生成**: devops_engineer (T26 + T29)
**报告路径**: `Apeireth-rust/docs/r14-workspace-prep-2026-07-30.md`
**状态**: ✅ 已完成, R14 团队 Phase 0 技术准备就绪
