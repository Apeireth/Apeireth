# R268: Self-Disable 实战触发链 Kani proofs 扩展

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 给 `SelfDisableGuard` 加 4 Kani proofs + 5 cargo mirrors, 覆盖 disarm/pass/serialization invariants

---

## §1 背景

R253 已有 3 Kani proofs + 4 cargo mirrors:
1. proof_no_revoke_when_triggered (record 只增不改)
2. proof_armed_blocks_all_violations (armed=true 必记录)
3. proof_no_path_disarm_when_triggered (disarm/rearm 不消除触发状态)

R258 Tier A 候选: Self-Disable Kani 实战触发链 = ★★★★. R253 已覆盖"理论", R268 加"实战".

---

## §2 新增 4 properties

### Property 4: disarmed guard never records anything

```rust
#[cfg(kani)]
#[kani::proof]
fn proof_disarmed_blocks_all_triggers() {
    let mut guard = SelfDisableGuard::new();
    guard.disarm();
    assert!(!guard.is_armed);
    let _ = guard.check_no_degrade("high", "low", "ctx", 0);
    let _ = guard.check_no_patch("principle_keys_count", 0, "ctx", 1);
    let _ = guard.check_no_bypass("master", false, "ctx", 2);
    let _ = guard.check_no_reverse("x", "ctx", 3);
    let _ = guard.check_no_hide("w1", "ctx", 4);
    assert_eq!(guard.record_count(), 0);
}
```

### Property 5: rearm after disarm restores armed=true

```rust
guard.disarm();
assert!(!guard.is_armed);
guard.rearm();
assert!(guard.is_armed);
// 重新 armed 后, 触发应记录
let _ = guard.check_no_degrade("high", "low", "ctx", 0);
assert!(guard.has_triggered());
```

### Property 6: pass path never increments record_count

```rust
// check_no_degrade with same risk_level = Pass (not violation)
let _ = guard.check_no_degrade("high", "high", "ctx", 0);
let _ = guard.check_no_degrade("low", "high", "ctx", 1);  // upgrade OK
let _ = guard.check_no_degrade("medium", "medium", "ctx", 2);
assert_eq!(guard.record_count(), before);
assert!(!guard.has_triggered());
```

### Property 7: trigger_id uniqueness across many triggers

```rust
let mut ids = std::collections::HashSet::new();
for i in 0..5 {
    let r = guard.check_no_degrade("high", "low", "ctx", i);
    if let Triggered(rec) = r {
        assert!(!ids.contains(&rec.trigger_id));
        ids.insert(rec.trigger_id);
    }
}
assert_eq!(ids.len(), 5);
```

### Property 8 (cargo-only): 5 大 mechanism IDs are stable (1, 2, 3, 4, 5)

```rust
assert_eq!(trigger_no_degrade().mechanism_id(), 1);
assert_eq!(trigger_no_patch().mechanism_id(), 2);
assert_eq!(trigger_no_bypass().mechanism_id(), 3);
assert_eq!(trigger_no_reverse().mechanism_id(), 4);
assert_eq!(trigger_no_hide().mechanism_id(), 5);
```

---

## §3 测试 (5 cargo mirrors + 4 Kani proofs)

- r268_01_disarmed_blocks_all_triggers (cargo + Kani)
- r268_02_rearm_restores_armed (cargo + Kani)
- r268_03_pass_path_no_record (cargo + Kani)
- r268_04_trigger_id_uniqueness (cargo + Kani)
- r268_05_mechanism_ids_are_stable (cargo only, 5 mechanism_id invariants)

**9 tests pass** in kani_proofs mod (R253 旧 4 + R268 新 5).

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借 Kani CBMC 模型检测, 自实现 5 大机制 + 实战触发链
- **S-2 实事求是**: Kani proofs 是形式化验证, cargo mirrors 跑同样代码做确定性断言
- **O-1 安全优先**: disarm 后 0 record, rearm 后恢复, 不可逆 — 这是"NoReverse"机制的形式化证明
- **O-3 干到底**: 4 Kani proofs + 5 cargo tests + 5 mechanism IDs invariant
- **O-5 不假装**: pass path 显式测 0 record (而非"测了armed=false的检查就够了")

---

## §5 后续

R270.1 = guard_endpoint.rs (api crate) 真接 sovereignty::SelfDisableGuard
- 当前 V2Guard 是简化策略 (nuclear → deny / 系统路径 → deny / tool.invoke → allow)
- 后续可加 `Arc<Mutex<SelfDisableGuard>>` 字段, check 时调 guard.check_no_bypass(...) 产生真 record
- 加 e2e test: check("/etc/passwd") → SelfDisableGuard record_count += 1
