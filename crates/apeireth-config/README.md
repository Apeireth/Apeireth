# apeireth-config

> apeireth-config — 强类型配置项 (R23 config 子模块)

apeireth-config 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (2 src 文件 / 22 测试 + 2 Kani proof)

- `src/lib.rs` — 强类型 Config 结构 (serde derive) + 17 测试
- `src/organ_kani_proofs.rs` — R177 config organ Kani proofs (5 测试 + 2 `#[kani::proof]`)
