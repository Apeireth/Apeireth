# apeireth-host

> Apeireth host infrastructure facade: secure keyring and cross-platform machine identity

apeireth-host 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (4 src 文件 + machine_id 子模块 / 17 测试 + 2 Kani proof)

- `src/lib.rs` — facade 入口 re-export + machine_id 子模块装配
- `src/keyring.rs` — 平台 keyring 后端 (keyring 3.6, apple-native/windows-native/sync-secret-service features) + 12 测试
- `src/machine_id/` — 跨平台 machine identity 派生 (mod + linux/darwin/win/bsd/provider, cfg-gated per target)
- `src/organ_kani_proofs.rs` — host organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
