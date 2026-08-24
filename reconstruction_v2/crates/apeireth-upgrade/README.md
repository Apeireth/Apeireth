# apeireth-upgrade

> Apeireth 升级器官 (A15 落点 — R14 Phase 5 OTA 升级 + sandbox-validator + 5 重治理)

apeireth-upgrade 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (13 src 文件 / 157 测试 + 2 Kani proof + 34 集成)

- `src/lib.rs` — 升级器官 facade 入口 + 6 测试
- `src/ota.rs` — OTA 3 阶段 (Download/Verify/Apply) + 38 测试
- `src/sandbox.rs` — sandbox-validator (manifest hash 校验) + 5 测试
- `src/self_update.rs` — 自更新主循环 (per OTA Apply 阶段) + 10 测试
- `src/rollback.rs` — 失败回滚路径 (前 snapshot 恢复) + 10 测试
- `src/manifest.rs` — 升级 manifest (version/diff/signature) + 6 测试
- `src/multisig.rs` — 多人审批门 (per 5 重治理) + 15 测试
- `src/council.rs` — 升级 → council 7 advisor 提交 + 11 测试
- `src/governance.rs` — 5 重治理入口 (per 借鉴) + 4 测试
- `src/intent.rs` — 升级意图解析 (per apeireth-perception) + 9 测试
- `src/monitor.rs` — 升级运行时 monitor + 17 测试
- `src/cross_crate.rs` — round10-10 跨 crate 集成入口 + 21 测试
- `src/organ_kani_proofs.rs` — upgrade organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/integration_7_stages.rs` (10) + `tests/integration_round10_10_cross_crate.rs` (16) + `tests/integration_round10_sandbox_rollback.rs` (8)
- 例: `examples/upgrade_demo.rs`
