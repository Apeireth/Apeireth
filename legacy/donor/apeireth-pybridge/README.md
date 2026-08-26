# apeireth-pybridge

> Apeireth PyO3 桥 (Python 3.13.14 <-> Rust) — ADR 0007 compat-components-layer + ADR 0008 feature-gating-pybridge (default 0 pyo3, `--features python-ext` 才编 pyo3). R129-4/5/6/18 ASI Python 整合 Stage 4-7 (D1-D4 自治 + G1-G4 治理 + K1-K4 守护 + I1-I7 跨模块) + R220 async_wrapper (tokio::spawn_blocking 包装 sync Python, 0 引 pyo3-asyncio) + R128 Stage 2/3 集成验证. 32 src 模块 (含 asi_modules + bridge + bridge_pool + type_convert + 7 stage7_i* + 4 governance + 4 guardianship + 4 self_loop + organ_kani_proofs). 测试数 (#[test]): 593 in-src + 561 集成 (Stage 2/3/4/5/6/7 跨 build 严守).

apeireth-pybridge 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
