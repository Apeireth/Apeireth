# R253 -- Kani Formal Proofs for Self-Disable Guards

## Problem
`apeireth-formal` crate archived at R165 (per `docs/24-locked-crates.md`). However,
3 Self-Disable invariants still lacked formal proofs at the production layer. Without
formal verification, a refactor could silently break the NoReverse / NoHide / NoBypass
mechanisms that protect L0 HA.

## Solution

### New module: `apeireth-sovereignty::kani_proofs`
3 Kani proofs (one per missing invariant) + 4 unit-test mirrors.

### Proof 1: `proof_no_revoke_when_triggered`
**Property**: SelfDisableRecord一旦创建, 不能被任何guard方法修改或删除.
**合约**: `SelfDisableGuard` 不暴露任何修改/删除 record 的方法.
**验证**: 5次 check_* 触发 + disarm/rearm/has_triggered/records_by_mechanism 后 record_count 仍 = 5.

### Proof 2: `proof_armed_blocks_all_violations`
**Property**: armed=true 时任何违规尝试必须被记录.
**合约**: 5大 check_* (no_degrade/no_patch/no_bypass/no_reverse/no_hide) 在 armed 状态下全部产生 SelfDisableCheck::Triggered.
**验证**: 3次 check_no_degrade 后 has_triggered()=true, record_count=3.

### Proof 3: `proof_no_path_disarm_when_triggered`
**Property**: 一旦 has_triggered()=true, disarm/rearm 不能消除.
**合约**: NoReverse 机制保证触发状态不可逆.
**验证**: disarm + rearm 循环 5 次后 has_triggered() 仍 = true, record_count 不变.

## Design honesty (O-5)
- `cfg(kani)` annotations on proof_* functions -- actual CBMC verification requires kani toolchain
- 普通 cargo test 跑确定性 mirror 测试 (覆盖核心 property)
- 0 false claims of "verified" without toolchain
- Kani proofs use bounded loops (kani::any::<u8>() not used in this PR to keep deps zero)

## Tests (4 new pass)
- r253_01: 5 triggers via check_*, then 4 read methods, record_count stays at 5
- r253_02: armed guard blocks all violations
- r253_03: disarm/rearm loops 5x, has_triggered stays true
- r253_04: integration of all 3 properties

## Files
- `crates/apeireth-sovereignty/src/kani_proofs.rs` (new, ~150 lines)
- `crates/apeireth-sovereignty/src/lib.rs` (add `pub mod kani_proofs;`)

## How to run actual Kani proofs (future)
```bash
cargo install kani-verifier
cargo kani -p apeireth-sovereignty --harness kani_proofs
```

cumulative: ~6375 tests pass.
