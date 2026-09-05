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
- **循环展开上界 (2026-09-05 CI 实测)**: Kani 将 String 字节缓冲当符号长度,
  HashMap SipHash `Hasher::write` 循环无限展开 (实测 1900+ 迭代 × 2s 卡死)。
  canonical 文件的 3 个 harness 均带 `#[kani::unwind(32)]` (仅 cfg(kani) 生效):
  真实执行用具体短键 ("a1"), 展开深度 ≤ 2 字节 + 桶遍历, 32 覆盖全部真实执行;
  超出上界的符号路径不检查 —— 属**有界模型检查**口径, 与 TLC 全状态穷举互相印证。
