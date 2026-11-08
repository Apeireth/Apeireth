//! **默认关** —— 域注册表的装配判据.
//!
//! 域注册表（按会话选域）**默认不装** ⇒ 外部工具桥保持「每工具一域」、出网闸
//! 保持自己的域 ⇒ 行为与本批接线前**逐字节一致**.
//!
//! # 判据 = **既有登记名**（不新增 `APEIRETH_*` 名）
//!
//! 判据是 `APEIRETH_ENABLE_EGRESS_GATE`（出网闸开关）：闸开着才有一个「需要
//! 跨面互认」的会话 ⇒ 才装注册表 ⇒ **不新增** `APEIRETH_*` 名 ⇒ 架构契约的
//! env 棘轮不动。

use apeireth_runtime_assembly::egress_gate::{
    egress_gate_enabled_from_value, install_shared_session_registry_if_enabled,
    shared_session_domain, shared_session_registry, EGRESS_GATE_ENV,
};

/// The criterion is a **pure function** ⇒ directly testable (no env touching).
#[test]
fn the_criterion_is_the_already_registered_egress_gate_switch() {
    for off in ["", "  ", "0", "false", "FALSE"] {
        assert!(
            !egress_gate_enabled_from_value(off),
            "{off:?} ⇒ 出网闸关 ⇒ 域注册表不装"
        );
    }
    for on in ["1", "{\"version\":1}"] {
        assert!(
            egress_gate_enabled_from_value(on),
            "{on:?} ⇒ 出网闸开 ⇒ 域注册表可装"
        );
    }
}

/// The switch **off** ⇒ **no** shared registry ⇒ nothing is installed ⇒ the two
/// faces keep their own domains (byte-for-byte the behaviour before this
/// batch).
#[test]
fn the_shared_registry_is_not_installed_while_the_gate_is_off() {
    // 测试进程里可能已被别处设置（env 是全局状态）⇒ 先读一眼再断言.
    let raw = std::env::var(EGRESS_GATE_ENV).ok();
    if !raw
        .as_deref()
        .map(egress_gate_enabled_from_value)
        .unwrap_or(false)
    {
        assert!(
            install_shared_session_registry_if_enabled().is_none(),
            "闸关 ⇒ 不装注册表 ⇒ 两面各持各的域（逐字节不变）"
        );
        assert!(shared_session_registry().is_none(), "闸关 ⇒ 共享槽是空的");
        assert!(
            shared_session_domain().is_none(),
            "闸关 ⇒ 没有共享固定域（逐字节不变）"
        );
    }
}

/// The switch **on** ⇒ the registry **is** installed ⇒ and it is **one**
/// instance (the first install wins) ⇒ the gate and the external tool bridge
/// take the same one ⇒ cross-family, same domain.
#[test]
fn the_shared_registry_is_one_instance_for_the_whole_process() {
    let raw = std::env::var(EGRESS_GATE_ENV).ok();
    if !raw
        .as_deref()
        .map(egress_gate_enabled_from_value)
        .unwrap_or(false)
    {
        // 闸关 ⇒ 本进程不装（上面那条已断言）⇒ 这里不重复断言「已装」。
        return;
    }
    let first = install_shared_session_registry_if_enabled().expect("闸开 ⇒ 装注册表");
    let second = install_shared_session_registry_if_enabled().expect("闸开 ⇒ 装注册表（幂等）");
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "闸开 ⇒ 注册表是**同一份** ⇒ 两面同域"
    );
    assert_eq!(
        shared_session_registry().map(|registry| std::sync::Arc::ptr_eq(&registry, &first)),
        Some(true),
        "共享槽里就是那一份"
    );
}
