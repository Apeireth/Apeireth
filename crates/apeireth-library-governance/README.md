# apeireth-library-governance

> Apeireth Library Stage 5 governance — policy framework + formal verification + cross-crate consistency (R127 P5-2, per decision-33 §1.4 + decision-55 §2.3). 三大模块: strategy (5 policy + 3 action + DecisionTree, clap 725 derive 借鉴) + verification (6 invariant + 6 harness + 8 boundary, Kani 4502 形式化模型借鉴) + consistency (5 check + 5 API lock + 编译期 hardcode, Kani proofs 借鉴) + formal_proof (P8-2 Stage 5.1 深化, Kani Invariant trait + ProofHarness + Stage5Token/LockedSignature POD 类型). src 模块 7 个 (lib + consistency + formal_proof + invariants + organ_kani_proofs + strategy + verification). 测试数 (#[test]): 112 in-src + 52 集成. 8 硬墙严守 (24 LOCKED → R148 仅保 3 不可变脊柱, per decision-130).

apeireth-library-governance 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
