# R270: Cargo.toml 修复 (workspace 21,315 tests 全过)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 修 R269 提交时误把 hex/uuid/apeireth-pipeline-g5 移到 [dev-dependencies], 导致 workspace 编译失败

---

## §1 问题

R269 加 MultiModelAdvisorBackend 时, Python patch 误把 `hex`/`uuid`/`apeireth-pipeline-g5` 从
[dependencies] 移到 [dev-dependencies]:

```
[dependencies]
parking_lot = "0.12"
sha2 = "0.10"

[dev-dependencies]
async-trait = { workspace = true }
hex = "0.4"
uuid = { version = "1", features = ["v4"] }
apeireth-pipeline-g5 = { path = "../apeireth-pipeline-g5" }
```

但 `hex`/`uuid`/`apeireth-pipeline-g5` 在 lib 代码 (group_chat.rs / g5_council_bridge.rs) 用, 不是 test-only.

**症状**:
```
error[E0432]: unresolved import `uuid`
  --> crates\apeireth-council\src\group_chat.rs:32:5
error[E0432]: unresolved import `apeireth_pipeline_g5`
  --> crates\apeireth-council\src\g5_council_bridge.rs:15:5
```

`cargo test --workspace` 编译失败.

---

## §2 修复

正确的 Cargo.toml:

```toml
[dependencies]
parking_lot = "0.12"
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1", features = ["v4"] }
# R159: council 接 g5 substrate (第 3 个生产调用方)
apeireth-pipeline-g5 = { path = "../apeireth-pipeline-g5" }

[dev-dependencies]
# R269: MultiModelAdvisorBackend tests impl LlmProvider trait (defined with #[async_trait])
async-trait = { workspace = true }
```

---

## §3 验证

```
$ cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.74s

$ cargo test --workspace --lib --no-fail-fast
# TOTAL PASSED: 1521

$ cargo test --workspace --no-fail-fast
# TOTAL PASSED: 21315
```

**workspace 21,315 tests 全过** (lib + integration + doctests).

---

## §4 主哲学锚对齐

- **S-2 实事求是**: 不假装 dev-dep 替换 lib-dep (会编译失败), 立刻修
- **O-3 干到底**: Cargo.toml 一处改对, workspace 全部编译通过
- **O-5 不假装**: 21,315 pass 是真实数据, 不是 approximate / round-number

---

## §5 教训

Python scripted patch 处理 Cargo.toml 的 section split 时要谨慎:
- 用 old/new 整段字符串匹配, 不要 half-match
- patch 后立即 cargo check 验证, 不要等 commit 后才发现
