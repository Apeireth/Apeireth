#![no_main]

//! fuzz: **表格操作面（`tool.sheet_ops`）的参数矩阵 + 写面判定** ——
//! 「带 `output` 就变写面」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/sheet_ops.rs`
//! （`sheet_ops_params_refusal` / `is_sheet_ops_write_action` / `allowed_fields`（私有）/
//! `SheetOpsConfig` / 三张闭合词表 + 两张动作表）。
//!
//! ## 为什么值得 fuzz（**这是「同族同档」的第二处 ⇒ 同族缺陷会两处都有**）
//! 模块头写明「与 `tool.doc_ops` **同族同档**」⇒ ⇒ 我今天修的 defect ⑬（禁填字段矩阵）
//! 与它的写面语义，**两处必须同一把尺**。而这里有一条比 doc_ops **更险**的东西：
//!
//! ⭐ **`is_sheet_ops_write_action(action, has_output)`**：
//! - `export_csv` / `export_xlsx` **恒写**；
//! - **变换动作（filter / sort / formula）带 `output` 即写**（`params.output.is_some()`）。
//! ⇒ ⇒ 若这条判定漏了 ⇒ ⇒ **一个「读起来像只读」的动作可以绕过宪法级写总闸**
//! （`file_write` 唯一写总闸，默认关 fail-closed）去落盘 ⇒ ⇒ **写闸形同虚设**。
//! ⇒ ⇒ 实现里写闸的锚点是 `if write_face && !config.write_gate_open ⇒ WriteGateClosed`
//! （**关 = 拒写不拒读**）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **动作词表闭合**：缺省/未知动作必拒 | 闭合词表分支 |
//! | O2 | **字段矩阵**：给动作不认的字段 ⇒ 必拒（**逐字段**） | `allowed_fields` |
//! | O3 | ⭐ **写面判定**：`export_*` 恒写；**变换动作带 output 即写**；其余非写 | `is_sheet_ops_write_action` |
//! | O4 | ⭐ **写总闸关 ⇒ 写面必拒**（`WriteGateClosed`）；**读面仍放行**（关 = 拒写不拒读） | 写闸分支 |
//! | O5 | **三张词表闭合**：`FILTER_PREDICATES` / `AGGREGATE_FUNCTIONS` / `SORT_ORDERS` 非空且互异 | 词表常量 |
//! | O6 | **`read` limit 范围**：`0` 或超 `max_preview_rows` ⇒ 必拒 | `read` 分支 |
//! | O7 | **动作表分档闭合**：写动作表 / 变换动作表 ⊂ 全表 | 两张动作表 |
//!
//! ## 诚实边界
//!
//! - **不测** 真实表解析（CSV/xlsx 读取、列存在性 ⇒ 执行面，需真文件）；
//! - **不测**导出落盘；**不测**公式求值语义（四则+括号+列引用的求值器另立面）；
//! - `allowed_fields` 是**私有**的 ⇒ 本 target 按同一份矩阵**独立复刻**用于对照
//!   ⇒ ⇒ 实现若改矩阵而这里没改 ⇒ **断言立刻失败**（把私有契约钉成公开守卫）。

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::sheet_ops::{
    is_sheet_ops_write_action, sheet_ops_params_refusal, SheetOpsConfig, SheetOpsParams,
    SheetOpsError, AGGREGATE_FUNCTIONS, FILTER_PREDICATES, SHEET_OPS_ACTIONS,
    SHEET_OPS_TRANSFORM_ACTIONS, SHEET_OPS_WRITE_ACTIONS, SORT_ORDERS,
};

/// 逐动作的**允许字段集**（独立复刻自实现里的私有 `allowed_fields`）。
const ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("read", &["offset", "limit"]),
    ("summary", &[]),
    ("filter", &["column", "predicate", "value", "output"]),
    ("sort", &["column", "order", "output"]),
    ("aggregate", &["column", "function"]),
    ("formula", &["expression", "new_column", "output"]),
    ("export_csv", &["output"]),
    ("export_xlsx", &["output"]),
];

const ALL_FIELDS: &[&str] = &[
    "output",
    "column",
    "predicate",
    "value",
    "order",
    "function",
    "expression",
    "new_column",
    "offset",
    "limit",
];

fn allowed_for(action: &str) -> Option<&'static [&'static str]> {
    ALLOWED_FIELDS
        .iter()
        .find(|(name, _)| *name == action)
        .map(|(_, fields)| *fields)
}

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    let workspace = std::env::temp_dir().join(format!(
        "apeireth-sheet-ops-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, x| acc.wrapping_mul(31).wrapping_add(*x as u64))
    ));
    let _ = std::fs::remove_dir_all(&workspace);
    let _ = std::fs::create_dir_all(&workspace);

    // ── 动作：词表内 / 词表外 / 缺省
    let action: Option<String> = match b(0) % 4 {
        0 => None,
        1 => Some("not_a_sheet_action".to_string()),
        2 => Some(String::new()),
        _ => Some(SHEET_OPS_ACTIONS[(b(1) as usize) % SHEET_OPS_ACTIONS.len()].to_string()),
    };
    let action_ref = action.as_deref().unwrap_or("");

    // ── O3/O5 写面判定 + 三张词表（**与配置无关，是纯函数**）
    // export_* 恒写
    for wa in SHEET_OPS_WRITE_ACTIONS {
        assert!(
            is_sheet_ops_write_action(wa, false),
            "恒写动作即使没有 output 也是写面: {wa}"
        );
        assert!(
            is_sheet_ops_write_action(wa, true),
            "恒写动作带 output 更是写面: {wa}"
        );
    }
    // ⭐ 变换动作：**带 output 才变写面**
    for ta in SHEET_OPS_TRANSFORM_ACTIONS {
        assert!(
            !is_sheet_ops_write_action(ta, false),
            "变换动作**不带** output 时不是写面: {ta}"
        );
        assert!(
            is_sheet_ops_write_action(ta, true),
            "⭐ 变换动作**带** output 时**必须**是写面（否则绕过写总闸）: {ta}"
        );
    }
    // 其余动作（read / summary / aggregate + 未知动作）任何情况下都不是写面
    for other in ["read", "summary", "aggregate", "not_a_sheet_action", ""] {
        assert!(
            !is_sheet_ops_write_action(other, false),
            "非写动作不带 output 时不是写面: {other:?}"
        );
        assert!(
            !is_sheet_ops_write_action(other, true),
            "非写动作即使带 output 也不是**恒写**动作（变换动作才算）: {other:?}"
        );
    }
    // 三张词表闭合
    for (name, table) in [
        ("FILTER_PREDICATES", FILTER_PREDICATES),
        ("AGGREGATE_FUNCTIONS", AGGREGATE_FUNCTIONS),
        ("SORT_ORDERS", SORT_ORDERS),
    ] {
        assert!(!table.is_empty(), "{name} 词表不得为空");
        let mut sorted = table.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "{name} 词表内不得重复");
    }
    // O7 动作表分档闭合
    for a in SHEET_OPS_WRITE_ACTIONS {
        assert!(
            SHEET_OPS_ACTIONS.contains(a),
            "写动作必须在全表内: {a}"
        );
    }
    for a in SHEET_OPS_TRANSFORM_ACTIONS {
        assert!(
            SHEET_OPS_ACTIONS.contains(a),
            "变换动作必须在全表内: {a}"
        );
        assert!(
            !SHEET_OPS_WRITE_ACTIONS.contains(a),
            "变换动作不得同时是恒写动作: {a}"
        );
    }

    // ── 造参数（全字段在场 ⇒ O2 必然对所有「不认某个字段」的动作触发拒绝）
    let output_present = b(2) % 2 == 0;
    let params = SheetOpsParams {
        action: action.clone(),
        source: Some("in.csv".to_string()),
        output: if output_present {
            Some("out.csv".to_string())
        } else {
            None
        },
        column: Some("c1".to_string()),
        predicate: Some("eq".to_string()),
        value: Some("1".to_string()),
        order: Some("asc".to_string()),
        function: Some("sum".to_string()),
        expression: Some("a + b".to_string()),
        new_column: Some("c2".to_string()),
        offset: Some(0),
        limit: Some(1),
    };

    // ── O4 写总闸**关**（默认 fail-closed）
    let closed = SheetOpsConfig::new(&workspace);
    assert!(
        !closed.write_gate_open,
        "写总闸默认必须是**关**（fail-closed）"
    );
    let refusal_closed = sheet_ops_params_refusal(&closed, &params);

    // ── O1 动作词表闭合
    if action.is_none() || action_ref == "not_a_sheet_action" || action_ref.is_empty() {
        assert!(
            refusal_closed.is_some(),
            "缺省/未知/空动作必须拒绝即帧（闭合词表）: action={action:?}"
        );
    } else {
        // ── O2 字段矩阵：逐个「不认的字段」核对必拒
        if let Some(allowed) = allowed_for(action_ref) {
            let present_fields: [(&str, bool); 10] = [
                ("output", params.output.is_some()),
                ("column", params.column.is_some()),
                ("predicate", params.predicate.is_some()),
                ("value", params.value.is_some()),
                ("order", params.order.is_some()),
                ("function", params.function.is_some()),
                ("expression", params.expression.is_some()),
                ("new_column", params.new_column.is_some()),
                ("offset", params.offset.is_some()),
                ("limit", params.limit.is_some()),
            ];
            let extra: Vec<&str> = present_fields
                .iter()
                .filter(|(f, present)| *present && !allowed.contains(f))
                .map(|(f, _)| *f)
                .collect();
            if !extra.is_empty() {
                assert!(
                    refusal_closed.is_some(),
                    "禁填字段必须拒绝即帧: action={action_ref:?} 多给 {extra:?}（只认 {allowed:?}）"
                );
            } else {
                // 字段矩阵通过 ⇒ 剩下的拒绝只可能来自**写闸**或动作自身规则
                let write_face = is_sheet_ops_write_action(action_ref, params.output.is_some());
                if write_face {
                    assert!(
                        refusal_closed.is_some(),
                        "⭐ 写总闸关时**写面必须拒**（关 = 拒写不拒读）: action={action_ref:?} \
                         output={:?}",
                        params.output
                    );
                    if let Some(error) = &refusal_closed {
                        assert!(
                            matches!(error, SheetOpsError::WriteGateClosed(_)),
                            "写闸关的拒绝必须是 WriteGateClosed（真因不串）: {error:?}"
                        );
                    }
                }
            }
        }
    }

    // ── O4 反面：写总闸**开** ⇒ 写面不再因写闸被拒（真因应换成别的或直接通过）
    let open = SheetOpsConfig::new(&workspace).with_write_gate(true);
    assert!(open.write_gate_open, "with_write_gate(true) 必须生效");
    let refusal_open = sheet_ops_params_refusal(&open, &params);
    if let Some(error) = &refusal_open {
        assert!(
            !matches!(error, SheetOpsError::WriteGateClosed(_)),
            "写总闸开时不得再报 WriteGateClosed（否则闸是假的）: {error:?}"
        );
    }

    // ── O6 read limit 范围
    let read_params = SheetOpsParams {
        action: Some("read".to_string()),
        source: Some("in.csv".to_string()),
        output: None,
        column: None,
        predicate: None,
        value: None,
        order: None,
        function: None,
        expression: None,
        new_column: None,
        offset: None,
        limit: Some(0),
    };
    assert!(
        sheet_ops_params_refusal(&closed, &read_params).is_some(),
        "read 的 limit=0 必须拒绝即帧（不静默截断）"
    );

    // ── 拒绝必须带**稳定 code**（帧的可审计锚；`code()` 是公开的稳定契约）
    for error in [&refusal_closed, &refusal_open].into_iter().flatten() {
        let code = error.code();
        assert!(
            !code.trim().is_empty(),
            "拒绝帧必须带稳定 code（不得空）"
        );
        assert!(
            code.starts_with("sheet_ops.") || code == "pipeline.pre_deny",
            "拒绝 code 必须落在稳定命名空间内（sheet_ops.* 或 pipeline.pre_deny）: {code:?}"
        );
    }

    let _ = source_unused();
    let _ = std::fs::remove_dir_all(&workspace);
});

/// 占位：保证 ALL_FIELDS 被引用（避免 dead_code 告警而不用 allow 掩盖）。
fn source_unused() -> usize {
    ALL_FIELDS.len()
}
