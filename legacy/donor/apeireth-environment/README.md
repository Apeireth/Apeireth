# apeireth-environment

> Apeireth terminal sandboxes (R173 / Stage2 §3): 6 backend - Local/Docker/SSH/Daytona/Modal/Singularity

apeireth-environment 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (2 src 文件 / 13 测试 + 2 Kani proof)

- `src/lib.rs` — Environment trait + 6 backend (Local/Docker/SSH/Daytona/Modal/Singularity) + Local/Docker/SSH 真实现 + 远程 stub + 8 测试
- `src/organ_kani_proofs.rs` — R177 environment organ Kani proofs (5 测试 + 2 `#[kani::proof]`)
