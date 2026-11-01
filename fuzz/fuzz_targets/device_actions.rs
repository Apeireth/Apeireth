#![no_main]

//! fuzz: **物理动作参数闸**（`device_control::validate_action_params`）。
//!
//! **为什么这个面最值得 fuzz**：它是 `unlock_door` / `physical_delete` /
//! `ignite` 这类**真实世界动作**的准入闸。被绕过 = 有人能对物理设备下达
//! 未声明的参数（例如给"开门"塞额外字段，或把亮度塞到 101）。
//!
//! **契约断言（取自该函数自己的语义）**：
//! 该函数的实现是「**键集闭合**校验 + 值域校验」——
//! `expect_keys(map, expected)` 要求 `map` 的键集**恰好等于** `expected`
//! （多一个少一个都拒）。因此可断言：
//!
//! 1. **返回 `Ok` ⇒ params 的键集恰好等于该动作声明的那一组**（不多不少）；
//! 2. **返回 `Ok` ⇒ 值满足声明的形状**（整数型在给定区间内 / 枚举在集合内 /
//!    非空字符串、正数）；
//! 3. 未知动作必须返回 `UnknownAction`，不得返回 `Ok`；
//! 4. 任何输入都不得 panic。
//!
//! 这四条合起来等价于「这道闸没有放行任何未声明的参数」——
//! 正是物理动作闸该有的性质。

use apeireth_tools_canonical::device::device_control::validate_action_params;
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value};

/// 各动作**声明**的键集（逐字取自实现里的 `expect_keys` 调用）。
/// 键集闭合 ⇒ 返回 Ok 时实际键集必须与此**完全相等**。
const DECLARED_KEYS: &[(&str, &[&str])] = &[
    ("power_on", &[]),
    ("power_off", &[]),
    ("start", &[]),
    ("stop", &[]),
    ("ignite", &[]),
    ("unlock_door", &[]),
    ("physical_delete", &[]),
    ("set_brightness", &["level"]),
    ("set_temperature", &["celsius"]),
    ("set_mode", &["mode"]),
    ("set_timer", &["seconds"]),
    ("purchase", &["item", "amount", "currency"]),
    ("place_order", &["item", "quantity", "amount", "currency"]),
];

/// `set_mode` 的枚举值（取自 `require_str_in` 的调用参数）。
const MODES: [&str; 3] = ["cool", "heat", "fan"];

fuzz_target!(|data: &[u8]| {
    // 任意 JSON 值都试；非 UTF-8 直接跳过（JSON 一定是 UTF-8）。
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };

    for (action, declared) in DECLARED_KEYS {
        if validate_action_params(action, &value).is_err() {
            continue;
        }

        // ── 断言 1: 键集闭合 —— Ok 时实际键集必须恰好等于声明的键集 ──────────
        // `null` 与 `{}` 等价（`params_map` 把 Null 当空 map）。
        let empty = Map::new();
        let map = if value.is_null() {
            &empty
        } else {
            value.as_object().unwrap_or_else(|| {
                panic!("validate_action_params 对非对象 params 返回了 Ok: action={action} value={value:?}")
            })
        };
        let mut actual: Vec<&str> = map.keys().map(String::as_str).collect();
        actual.sort_unstable();
        let mut wanted: Vec<&str> = declared.to_vec();
        wanted.sort_unstable();
        assert_eq!(
            actual, wanted,
            "放行了未声明的键集: action={action} actual={actual:?} declared={wanted:?}"
        );

        // ── 断言 2: 值域 —— Ok 时值必须满足声明的形状 ────────────────────────
        for (key, v) in map {
            let ok_shape = match *action {
                "set_brightness" => {
                    key == "level" && v.as_i64().is_some_and(|n| (0..=100).contains(&n))
                }
                "set_temperature" => {
                    key == "celsius" && v.as_i64().is_some_and(|n| (16..=30).contains(&n))
                }
                "set_mode" => key == "mode" && v.as_str().is_some_and(|s| MODES.contains(&s)),
                "set_timer" => {
                    key == "seconds" && v.as_i64().is_some_and(|n| (1..=3600).contains(&n))
                }
                "purchase" | "place_order" => match key.as_str() {
                    "item" | "currency" => v.as_str().is_some_and(|s| !s.is_empty()),
                    "amount" => v.as_f64().is_some_and(|n| n > 0.0),
                    "quantity" => v.as_i64().is_some_and(|n| (1..=99).contains(&n)),
                    _ => false,
                },
                // 无参数动作: 键集已在上面的断言里钉死为空, 不会走到这里。
                _ => false,
            };
            assert!(
                ok_shape,
                "放行了不符声明的值: action={action} key={key} value={v:?}"
            );
        }
    }

    // ── 断言 3: 未知动作不得放行 ─────────────────────────────────────────────
    // 取输入里的一段文本当作动作名（跳过已声明的名字）。
    if let Ok(text) = std::str::from_utf8(data) {
        let candidate = text.trim();
        let known = DECLARED_KEYS.iter().any(|(name, _)| *name == candidate);
        if !known && validate_action_params(candidate, &value).is_ok() {
            panic!("未知动作被放行: action={candidate:?}");
        }
    }
});
