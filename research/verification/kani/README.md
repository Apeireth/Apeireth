# Kani mirror crate —— Phase 5 审批状态机机器证明

- **零复制**: `src/lib.rs` 用 `#[path]` 直接包含 canonical 源文件
  `crates/engine/runtime/src/canonical/research_approval_sm.rs`, 验证的永远是同一份代码。
- **为何需要**: workspace 声明 rustc 1.97; Kani 0.67 (crates.io 最新) 自带 nightly 1.93;
  直接 `cargo kani -p apeireth-runtime` 被 rust-version 检查拒绝。本 crate 独立
  (root workspace 已 exclude research/), 只编译状态机 + serde。
- **运行**: `cargo kani --manifest-path research/verification/kani/Cargo.toml`
  (CI: `.github/workflows/kani.yml`, ubuntu runner)。
- **同步**: 无需同步 —— 指向同一文件。canonical 文件改名/移动时改 `lib.rs` 的 path 即可。
- **验证目标**: 3 个 `#[kani::proof]` (终态锁 / executed 单调至多一次 / crash 恢复 InvC)。
