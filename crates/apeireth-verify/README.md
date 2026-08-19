# apeireth-verify

> Apeireth cross-crate regression verification mechanism

apeireth-verify 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (3 src 文件 / 47 测试 + 2 Kani proof + 14 集成)

- `src/lib.rs` — 跨 crate regression facade + 28 测试
- `src/const_proofs.rs` — 编译期 hardcode 守门 (per LOCKED crate 入口签名降级后 3 不可变脊柱: Self-Disable / L0 HA / 13 键 verdict cache) + 14 测试
- `src/organ_kani_proofs.rs` — verify organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/cross_crate_smoke.rs` (1) + `tests/macro_smoke.rs` (1) + `tests/stage6_22_interlock.rs` (10)
- 例: `examples/walk_all_crates.rs`
