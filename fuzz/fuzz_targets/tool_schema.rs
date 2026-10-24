//! fuzz: 工具参数/输出的**契约校验面**（`OutputSchema::validate`）。
//!
//! **面**：这个校验器判定 LLM 回传的结构化参数是否符合工具声明的形状。
//! 输入完全是**模型生成的、不可信的 JSON** —— 校验器本身若不稳，
//! 整个工具链的准入判定就不可信。
//!
//! **判据（契约断言，不只是不 panic）**：
//! `validate` 返回 `Ok` 时，被声明为 required 的每个字段**必须存在且类型匹配**。
//! 这是它自己的语义，写成断言才能抓「放行了不该放行的值」。
#![no_main]

use apeireth_tools_canonical::exec_pipeline::contract::{OutputSchema, SchemaKind};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };

    // 覆盖全部 5 种 SchemaKind：单字段 schema 逐一钉
    let kinds = [
        SchemaKind::String,
        SchemaKind::Number,
        SchemaKind::Boolean,
        SchemaKind::Object,
        SchemaKind::Array,
    ];
    for kind in kinds {
        let schema = OutputSchema::new().require("f", kind);
        if schema.validate(&value).is_ok() {
            let obj = value.as_object().expect("Ok 时必须是结构化对象");
            let got = obj.get("f").expect("Ok 时 required 字段必须存在");
            assert!(
                kind.matches(got),
                "validate 放行了类型不匹配的字段: kind={kind:?} value={got:?}"
            );
        }
    }

    // 组合 schema：多字段 + 混合类型
    let multi = OutputSchema::new()
        .require("a", SchemaKind::String)
        .require("b", SchemaKind::Number)
        .require("c", SchemaKind::Array);
    if multi.validate(&value).is_ok() {
        let obj = value.as_object().expect("Ok 时必须是结构化对象");
        for (name, kind) in [("a", SchemaKind::String), ("b", SchemaKind::Number), ("c", SchemaKind::Array)] {
            let got = obj.get(name).unwrap_or_else(|| panic!("Ok 时 required 字段 {name} 必须存在"));
            assert!(kind.matches(got), "validate 放行了 {name} 的类型不匹配: value={got:?}");
        }
    }
});
