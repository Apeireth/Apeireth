//! 创作工具面 K3 (`tool.sheet_ops`) 行为钉子: 六操作行为 (read/summary/filter/
//! sort/aggregate/formula) + 导出写面 (export_csv) 全部钉在**真 fixture CSV** 上;
//! filter/sort/aggregate 边界 (空表 / 类型混 / NaN 语义) / 行列字节上限拒 /
//! 创建类原子写 / 敏感面拒 / 越界不回显路径 / 写总闸 fail-closed / 开关
//! fail-closed / 审批档与边界 Guard 各一枚。口径钉死处都断言精确值, 演进须改测试。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use std::path::{Path, PathBuf};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use serde_json::{json, Value};

use apeireth_tools_canonical::sheet_ops::{
    is_sheet_ops_write_action, sheet_ops_enabled_from_env, sheet_ops_params_refusal,
    SheetOpsApprovalHook, SheetOpsBoundaryGuard, SheetOpsConfig, SheetOpsError, SheetOpsParams,
    SheetOpsRunner, SheetOpsTool, ACTION_AGGREGATE, ACTION_EXPORT_CSV, ACTION_FILTER,
    ACTION_FORMULA, ACTION_READ, ACTION_SORT, ACTION_SUMMARY, SHEET_OPS_CAPABILITY_ID,
    SHEET_OPS_ENABLE_ENV, SHEET_OPS_TOOL_NAME,
};

// ---------------------------------------------------------------------------
// 夹具 (真 CSV 文件: tests/fixtures/sheet_ops/*.csv)
// ---------------------------------------------------------------------------

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sheet_ops")
        .join(name)
}

/// 建一个工作区临时目录并拷入夹具 (写面测试不脏仓库)。
fn workspace_with(names: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for name in names {
        std::fs::copy(fixture_path(name), dir.path().join(name)).expect("copy fixture");
    }
    dir
}

fn runner(root: &Path) -> SheetOpsRunner {
    SheetOpsRunner::new(SheetOpsConfig::new(root).with_write_gate(true))
}

fn call(runner: &SheetOpsRunner, args: Value) -> Result<Value, SheetOpsError> {
    let params: SheetOpsParams = serde_json::from_value(args).expect("valid params shape");
    runner.run(&params)
}

fn frame_of(error: &SheetOpsError) -> String {
    error.frame()
}

fn rows_of(frame: &Value) -> Vec<Vec<String>> {
    frame["rows"]
        .as_array()
        .expect("rows array")
        .iter()
        .map(|row| {
            row.as_array()
                .expect("row array")
                .iter()
                .map(|cell| cell.as_str().expect("cell string").to_string())
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 六操作行为钉 (真 fixture CSV)
// ---------------------------------------------------------------------------

/// ① `read`: 表头 + 行预览 + 显式 truncated (不冒充全量)。
#[test]
fn read_returns_columns_rows_and_explicit_truncation() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_READ, "source": "sales.csv", "offset": 0, "limit": 2}),
    )
    .expect("read ok");
    assert_eq!(frame["action"], ACTION_READ);
    assert_eq!(frame["row_count"], 3);
    assert_eq!(frame["returned"], 2);
    assert_eq!(frame["truncated"], true);
    assert_eq!(frame["columns"], json!(["name", "qty", "price", "note"]));
    assert_eq!(
        rows_of(&frame),
        vec![
            vec!["apple", "3", "1.25", "red"],
            vec!["pear", "10", "0.5", "fresh"],
        ]
    );

    let frame = call(
        &runner,
        json!({"action": ACTION_READ, "source": "sales.csv", "offset": 2, "limit": 100}),
    )
    .expect("read ok");
    assert_eq!(frame["returned"], 1);
    assert_eq!(frame["truncated"], false);

    // offset 越界不回空帧冒充: 拒绝即帧。
    let error = call(
        &runner,
        json!({"action": ACTION_READ, "source": "sales.csv", "offset": 99}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.param_rejected"),
        "{}",
        frame_of(&error)
    );
}

/// ② `summary`: 行列数 / 列名 / 类型猜测钉死 (int/float/text/mixed/empty;
/// `NaN`/`inf` 词面按文本, 永不入数值轴)。
#[test]
fn summary_reports_shape_and_pinned_type_guesses() {
    let dir = workspace_with(&["sales.csv", "mixed.csv", "header_only.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_SUMMARY, "source": "sales.csv"}),
    )
    .expect("summary ok");
    assert_eq!(frame["row_count"], 3);
    assert_eq!(frame["column_count"], 4);
    assert_eq!(frame["columns"][0]["type"], "text");
    assert_eq!(frame["columns"][1]["type"], "int");
    assert_eq!(frame["columns"][2]["type"], "float");
    assert_eq!(frame["columns"][3]["type"], "text");
    assert_eq!(frame["columns"][1]["empty_cells"], 0);

    let frame = call(
        &runner,
        json!({"action": ACTION_SUMMARY, "source": "mixed.csv"}),
    )
    .expect("summary ok");
    assert_eq!(frame["columns"][0]["type"], "text");
    assert_eq!(
        frame["columns"][1]["type"], "mixed",
        "NaN 是文本 → 数值/文本混型"
    );
    assert_eq!(frame["columns"][1]["empty_cells"], 1);

    let frame = call(
        &runner,
        json!({"action": ACTION_SUMMARY, "source": "header_only.csv"}),
    )
    .expect("summary ok");
    assert_eq!(frame["row_count"], 0);
    assert_eq!(frame["columns"][0]["type"], "empty");
    assert_eq!(frame["columns"][0]["empty_cells"], 0);
}

/// ③ `filter`: eq 数值轴/文本轴双语义 + 空值恒不匹配 (钉死)。
#[test]
fn filter_eq_is_numeric_aware_or_textual_and_empties_never_match() {
    let dir = workspace_with(&["sales.csv", "blank_cells.csv"]);
    let runner = runner(dir.path());

    // 双侧数值 → 数值相等 ("3" == "3.0")。
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "qty",
               "predicate": "eq", "value": "3.0"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1);
    assert_eq!(rows_of(&frame)[0][0], "apple");

    // 否则文本相等。
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "note",
               "predicate": "eq", "value": "fresh"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1);
    assert_eq!(rows_of(&frame)[0][0], "pear");

    // 空值恒不匹配 (eq 与 ne 同口径, 空单元不进结果)。
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "blank_cells.csv", "column": "value",
               "predicate": "eq", "value": "1"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1);
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "blank_cells.csv", "column": "value",
               "predicate": "ne", "value": "1"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1, "空单元不匹配 ne (只 z 行)");
    assert_eq!(rows_of(&frame)[0][0], "z");

    // contains = 文本包含。
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "name",
               "predicate": "contains", "value": "ea"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1);
    assert_eq!(rows_of(&frame)[0][0], "pear");
}

/// ④ `filter` 边界: 数值谓词遇类型混/非数值拒即帧, value 非数值词面拒即帧。
#[test]
fn filter_numeric_predicates_refuse_mixed_columns() {
    let dir = workspace_with(&["mixed.csv", "sales.csv"]);
    let runner = runner(dir.path());

    let error = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "mixed.csv", "column": "score",
               "predicate": "gt", "value": "0"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.type_mixed"),
        "{}",
        frame_of(&error)
    );

    let error = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "mixed.csv", "column": "item",
               "predicate": "lt", "value": "2"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.type_mixed"),
        "{}",
        frame_of(&error)
    );

    // 数值列 + 非数值比较值 = 参数拒 (与类型混分帧)。
    let error = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "qty",
               "predicate": "gt", "value": "abc"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.param_rejected"),
        "{}",
        frame_of(&error)
    );

    // NaN 词面是文本: eq 文本比较命中, 不当数值。
    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "mixed.csv", "column": "score",
               "predicate": "eq", "value": "NaN"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 1);
    assert_eq!(rows_of(&frame)[0][0], "c");
}

/// ⑤ `sort`: 数值列数值序 (10 > 3), 文本列码点序; 稳定排序。
#[test]
fn sort_orders_numbers_numerically_and_text_by_codepoint() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_SORT, "source": "sales.csv", "column": "qty", "order": "asc"}),
    )
    .expect("sort ok");
    let names: Vec<String> = rows_of(&frame).iter().map(|row| row[0].clone()).collect();
    assert_eq!(names, vec!["plum", "apple", "pear"], "数值序: 2 < 3 < 10");

    let frame = call(
        &runner,
        json!({"action": ACTION_SORT, "source": "sales.csv", "column": "qty", "order": "desc"}),
    )
    .expect("sort ok");
    let names: Vec<String> = rows_of(&frame).iter().map(|row| row[0].clone()).collect();
    assert_eq!(names, vec!["pear", "apple", "plum"]);

    let frame = call(
        &runner,
        json!({"action": ACTION_SORT, "source": "sales.csv", "column": "name", "order": "asc"}),
    )
    .expect("sort ok");
    let names: Vec<String> = rows_of(&frame).iter().map(|row| row[0].clone()).collect();
    assert_eq!(names, vec!["apple", "pear", "plum"], "文本列码点序");
}

/// ⑥ `sort` 边界: 空值恒排最后 (asc/desc 同), 类型混拒绝即帧。
#[test]
fn sort_pins_empty_cells_last_and_refuses_mixed_columns() {
    let dir = workspace_with(&["blank_cells.csv", "mixed.csv"]);
    let runner = runner(dir.path());

    for (order, expected) in [
        ("asc", vec!["x", "z", "y", "w"]),
        ("desc", vec!["z", "x", "y", "w"]),
    ] {
        let frame = call(
            &runner,
            json!({"action": ACTION_SORT, "source": "blank_cells.csv", "column": "value",
                   "order": order}),
        )
        .expect("sort ok");
        let labels: Vec<String> = rows_of(&frame).iter().map(|row| row[0].clone()).collect();
        assert_eq!(labels, expected, "空值恒排最后 ({order}), 空行保持原相对序");
    }

    let error = call(
        &runner,
        json!({"action": ACTION_SORT, "source": "mixed.csv", "column": "score"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.type_mixed"),
        "{}",
        frame_of(&error)
    );
}

/// ⑦ `aggregate`: f64 如实 (0.1+0.2 不静默取整), min/max/avg/count 各钉真值。
#[test]
fn aggregate_reports_f64_truthfully_without_rounding() {
    let dir = workspace_with(&["decimals.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "decimals.csv", "column": "x",
               "function": "sum"}),
    )
    .expect("aggregate ok");
    let sum = frame["value"].as_f64().expect("numeric value");
    assert_eq!(sum, 0.1f64 + 0.2f64, "浮点如实: 顺序求和的 f64 真值");
    assert_ne!(
        format!("{sum}"),
        "0.3",
        "不静默取整: 真值是 0.30000000000000004"
    );
    assert_eq!(frame["count"], 2);

    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "decimals.csv", "column": "x",
               "function": "avg"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["value"].as_f64(), Some((0.1f64 + 0.2f64) / 2.0));

    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "decimals.csv", "column": "x",
               "function": "min"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["value"].as_f64(), Some(0.1));
    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "decimals.csv", "column": "x",
               "function": "max"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["value"].as_f64(), Some(0.2));
}

/// ⑧ `aggregate` 边界: count 只数非空; 空聚合 sum=0/count=0 如实, avg/min/max
/// 空集拒绝即帧 (不冒充 0)。
#[test]
fn aggregate_count_and_empty_semantics_are_pinned() {
    let dir = workspace_with(&["blank_cells.csv", "header_only.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "blank_cells.csv", "column": "value",
               "function": "count"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["count"], 2);
    assert_eq!(frame["value"], 2);

    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "blank_cells.csv", "column": "value",
               "function": "sum"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["count"], 2);
    assert_eq!(frame["value"].as_f64(), Some(4.0), "空单元缺位不参与");

    // 仅表头 = 空集。
    let frame = call(
        &runner,
        json!({"action": ACTION_AGGREGATE, "source": "header_only.csv", "column": "a",
               "function": "sum"}),
    )
    .expect("aggregate ok");
    assert_eq!(frame["count"], 0);
    assert_eq!(
        frame["value"].as_f64(),
        Some(0.0),
        "sum(空集)=0 如实带 count:0"
    );

    for function in ["avg", "min", "max"] {
        let error = call(
            &runner,
            json!({"action": ACTION_AGGREGATE, "source": "header_only.csv", "column": "a",
                   "function": function}),
        )
        .unwrap_err();
        assert!(
            frame_of(&error).contains("sheet_ops.aggregate_empty"),
            "{function}: {}",
            frame_of(&error)
        );
    }
}

/// ⑨ `aggregate` 边界: 非空非数值单元 (含 `NaN` 词面) 类型混拒绝即帧, 不静默跳行。
#[test]
fn aggregate_refuses_non_numeric_cells() {
    let dir = workspace_with(&["mixed.csv"]);
    let runner = runner(dir.path());
    for function in ["sum", "avg", "min", "max"] {
        let error = call(
            &runner,
            json!({"action": ACTION_AGGREGATE, "source": "mixed.csv", "column": "score",
                   "function": function}),
        )
        .unwrap_err();
        assert!(
            frame_of(&error).contains("sheet_ops.type_mixed"),
            "{function}: {}",
            frame_of(&error)
        );
    }
}

/// ⑩ `formula`: 计算列追加 (原文不改), f64 最短往返文本 (0.1+0.2 →
/// 0.30000000000000004)。
#[test]
fn formula_appends_computed_column_with_roundtrip_repr() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "sales.csv", "expression": "price * qty",
               "new_column": "total"}),
    )
    .expect("formula ok");
    assert_eq!(frame["new_column"], "total");
    let rows = rows_of(&frame);
    assert_eq!(rows[0], vec!["apple", "3", "1.25", "red", "3.75"]);
    assert_eq!(rows[1], vec!["pear", "10", "0.5", "fresh", "5"]);
    assert_eq!(rows[2], vec!["plum", "2", "3.4", "sweet", "6.8"]);
    assert_eq!(frame["columns"][4], "total");

    let frame = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "sales.csv", "expression": "0.1 + 0.2",
               "new_column": "k"}),
    )
    .expect("formula ok");
    for row in rows_of(&frame) {
        assert_eq!(row[4], "0.30000000000000004", "浮点如实, 不静默取整");
    }
}

/// ⑪ `formula` 边界: 除零 / 未知列 / 非数值单元 / 重名列全部拒绝即帧
/// (NaN/inf 零产出)。
#[test]
fn formula_refuses_division_by_zero_unknown_columns_and_bad_cells() {
    let dir = workspace_with(&["sales.csv", "mixed.csv"]);
    let runner = runner(dir.path());

    let error = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "sales.csv", "expression": "qty / 0",
               "new_column": "t"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.formula_error"),
        "{}",
        frame_of(&error)
    );
    assert!(frame_of(&error).contains("除零"), "{}", frame_of(&error));

    let error = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "sales.csv", "expression": "nope + 1",
               "new_column": "t"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.formula_error"),
        "{}",
        frame_of(&error)
    );

    let error = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "mixed.csv", "expression": "score * 2",
               "new_column": "t"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.formula_error"),
        "{}",
        frame_of(&error)
    );

    let error = call(
        &runner,
        json!({"action": ACTION_FORMULA, "source": "sales.csv", "expression": "1 + 1",
               "new_column": "qty"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.param_rejected"),
        "{}",
        frame_of(&error)
    );
}

/// ⑫ 空表 (仅表头) 全动作口径: 0 行不是错, 读/筛/排/计算列都如实回 0 行。
#[test]
fn header_only_tables_yield_zero_rows_not_errors() {
    let dir = workspace_with(&["header_only.csv"]);
    let runner = runner(dir.path());

    for args in [
        json!({"action": ACTION_READ, "source": "header_only.csv"}),
        json!({"action": ACTION_FILTER, "source": "header_only.csv", "column": "a",
               "predicate": "eq", "value": "1"}),
        json!({"action": ACTION_SORT, "source": "header_only.csv", "column": "a"}),
        json!({"action": ACTION_FORMULA, "source": "header_only.csv", "expression": "a + 1",
               "new_column": "t"}),
    ] {
        let frame = call(&runner, args).expect("0 行不是错");
        assert_eq!(frame["row_count"], 0, "{frame}");
    }
}

// ---------------------------------------------------------------------------
// 受控面 (上限拒 / 原子写 / 越界 / 敏感面 / 写总闸 / 开关)
// ---------------------------------------------------------------------------

/// ⑬ 防炸弹表: 字节/数据行/列数三层上限各自拒绝即帧 (宁拒不截)。
#[test]
fn row_col_byte_caps_reject_bomb_tables() {
    let dir = workspace_with(&["sales.csv"]);
    let tiny_bytes = SheetOpsRunner::new(
        SheetOpsConfig::new(dir.path())
            .with_write_gate(true)
            .with_limits(10, 1_000, 1_000, 1_000),
    );
    let error = call(
        &tiny_bytes,
        json!({"action": ACTION_READ, "source": "sales.csv"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.limit_exceeded"),
        "{}",
        frame_of(&error)
    );

    let tiny_rows = SheetOpsRunner::new(
        SheetOpsConfig::new(dir.path())
            .with_write_gate(true)
            .with_limits(100_000, 2, 1_000, 1_000),
    );
    let error = call(
        &tiny_rows,
        json!({"action": ACTION_READ, "source": "sales.csv"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.limit_exceeded"),
        "{}",
        frame_of(&error)
    );

    let tiny_cols = SheetOpsRunner::new(
        SheetOpsConfig::new(dir.path())
            .with_write_gate(true)
            .with_limits(100_000, 1_000, 2, 1_000),
    );
    let error = call(
        &tiny_cols,
        json!({"action": ACTION_READ, "source": "sales.csv"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.limit_exceeded"),
        "{}",
        frame_of(&error)
    );
}

/// ⑭ `export_csv`: 规范 CSV 再序列化 + 创建类原子写 (只创建不覆盖, 目标原状)。
#[test]
fn export_csv_writes_canonical_csv_and_never_overwrites() {
    let dir = workspace_with(&["sales.csv", "quoted.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_EXPORT_CSV, "source": "sales.csv", "output": "out.csv"}),
    )
    .expect("export ok");
    assert_eq!(frame["output"], "out.csv");
    assert_eq!(frame["rows_written"], 3);
    let written = std::fs::read_to_string(dir.path().join("out.csv")).expect("read out");
    assert_eq!(
        written,
        "name,qty,price,note\napple,3,1.25,red\npear,10,0.5,fresh\nplum,2,3.4,sweet\n"
    );

    // 引号方言归一 (""转义 / 内嵌逗号)。
    let frame = call(
        &runner,
        json!({"action": ACTION_EXPORT_CSV, "source": "quoted.csv", "output": "q.csv"}),
    )
    .expect("export ok");
    assert_eq!(frame["rows_written"], 2);
    let written = std::fs::read_to_string(dir.path().join("q.csv")).expect("read q");
    assert_eq!(
        written,
        "city,pop\n\"Paris, FR\",100\n\"He said \"\"hi\"\"\",200\n"
    );

    // 只创建不覆盖: 目标已存在拒绝即帧, 文件原状。
    let error = call(
        &runner,
        json!({"action": ACTION_EXPORT_CSV, "source": "sales.csv", "output": "out.csv"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("sheet_ops.output_exists"),
        "{}",
        frame_of(&error)
    );
    let after = std::fs::read_to_string(dir.path().join("out.csv")).expect("read out");
    assert!(after.starts_with("name,qty,price,note"), "目标文件原状");
}

/// ⑮ 变换动作写面: 带 `output` 落新文件 (rows_written 如实); 不带 = 内存预览
/// (超预览上限显式 truncated)。
#[test]
fn transform_actions_write_only_with_output_and_preview_with_truncation() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = runner(dir.path());

    let frame = call(
        &runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "qty",
               "predicate": "eq", "value": "3", "output": "filtered.csv"}),
    )
    .expect("filter+write ok");
    assert_eq!(frame["output"], "filtered.csv");
    assert_eq!(frame["rows_written"], 1);
    assert_eq!(frame["truncated"], false);
    let written = std::fs::read_to_string(dir.path().join("filtered.csv")).expect("read");
    assert_eq!(written, "name,qty,price,note\napple,3,1.25,red\n");

    // 预览截断显式标注 (不冒充全量)。
    let preview_runner = SheetOpsRunner::new(
        SheetOpsConfig::new(dir.path())
            .with_write_gate(true)
            .with_limits(100_000, 1_000, 1_000, 1),
    );
    let frame = call(
        &preview_runner,
        json!({"action": ACTION_FILTER, "source": "sales.csv", "column": "qty",
               "predicate": "gt", "value": "0"}),
    )
    .expect("filter ok");
    assert_eq!(frame["matched"], 3);
    assert_eq!(frame["returned"], 1);
    assert_eq!(frame["truncated"], true);
}

/// ⑯ 工作区限域: 越界/穿越拒绝即帧且**不回显路径** (脱敏口径)。
#[test]
fn workspace_scoping_refusals_never_echo_paths() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = runner(dir.path());
    let root_spelling = dir.path().to_string_lossy().to_string();

    for source in [
        "../escape.csv".to_string(),
        std::env::temp_dir()
            .join("outside.csv")
            .to_string_lossy()
            .to_string(),
    ] {
        let error = call(&runner, json!({"action": ACTION_READ, "source": source}))
            .expect_err("越界必须拒");
        let frame = frame_of(&error);
        assert!(
            frame.contains("sheet_ops.outside_workspace")
                || frame.contains("sheet_ops.path_rejected"),
            "{frame}"
        );
        assert!(!frame.contains("escape.csv"), "路径不回显: {frame}");
        assert!(!frame.contains(&root_spelling), "根拼写不回显: {frame}");
    }
}

/// ⑰ 敏感面拒: 凭据面路径读写都拒, 拒绝信息即帧 (`pipeline.pre_deny` 语义)。
#[test]
fn sensitive_surface_is_refused_with_credential_frame() {
    let dir = workspace_with(&["sales.csv"]);
    std::fs::write(dir.path().join("credentials.csv"), "a\n1\n").expect("fixture");
    let runner = runner(dir.path());

    let error = call(
        &runner,
        json!({"action": ACTION_READ, "source": "credentials.csv"}),
    )
    .unwrap_err();
    let frame = frame_of(&error);
    assert!(frame.contains("pipeline.pre_deny"), "{frame}");
    assert!(!frame.contains("1\n"), "凭据内容不入帧: {frame}");

    let error = call(
        &runner,
        json!({"action": ACTION_EXPORT_CSV, "source": "sales.csv",
               "output": "credentials.csv"}),
    )
    .unwrap_err();
    assert!(
        frame_of(&error).contains("pipeline.pre_deny"),
        "{}",
        frame_of(&error)
    );
}

/// ⑱ 写总闸 (宪法级 `file_write` 唯一写总闸): 关 = 拒写不拒读 (fail-closed)。
#[test]
fn write_gate_fails_closed_but_reads_pass() {
    let dir = workspace_with(&["sales.csv"]);
    let runner = SheetOpsRunner::new(SheetOpsConfig::new(dir.path()).with_write_gate(false));

    call(
        &runner,
        json!({"action": ACTION_READ, "source": "sales.csv"}),
    )
    .expect("读面不受写总闸约束");

    for args in [
        json!({"action": ACTION_EXPORT_CSV, "source": "sales.csv", "output": "o.csv"}),
        json!({"action": ACTION_SORT, "source": "sales.csv", "column": "qty",
               "output": "s.csv"}),
    ] {
        let error = call(&runner, args).expect_err("写面必须被拒");
        assert!(
            frame_of(&error).contains("sheet_ops.write_gate_closed"),
            "{}",
            frame_of(&error)
        );
    }
}

/// ⑲ 开关 fail-closed: `APEIRETH_ENABLE_SHEET_OPS` 只认显式 "1"。
#[test]
fn sheet_ops_switch_fails_closed_on_anything_but_one() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::remove_var(SHEET_OPS_ENABLE_ENV);
    assert!(!sheet_ops_enabled_from_env(), "未设 = 关");
    assert!(
        SheetOpsConfig::from_env(".").is_none(),
        "未设 = 不注册 (fail-closed)"
    );
    for value in ["0", "true", "yes", "on", " 2 ", "11", "1x"] {
        std::env::set_var(SHEET_OPS_ENABLE_ENV, value);
        assert!(!sheet_ops_enabled_from_env(), "must refuse {value:?}");
    }
    std::env::set_var(SHEET_OPS_ENABLE_ENV, "1");
    assert!(sheet_ops_enabled_from_env());
    assert!(SheetOpsConfig::from_env(".").is_some());
    std::env::remove_var(SHEET_OPS_ENABLE_ENV);
}

// ---------------------------------------------------------------------------
// 治理面 (审批档 / 边界 Guard / 参数矩阵)
// ---------------------------------------------------------------------------

/// ⑳ 审批档: 写面每次人批 (无自动放行档), 纯读放行, 词表外 fail-closed。
#[tokio::test]
async fn approval_hook_stops_writes_at_human_approval_and_passes_reads() {
    fn dispatch(args: Value) -> GovernanceRequest<'static> {
        let capability: &'static CapabilityId = Box::leak(Box::new(
            CapabilityId::new(SHEET_OPS_CAPABILITY_ID).expect("capability id"),
        ));
        let arguments: &'static Value = Box::leak(Box::new(args));
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        )
    }
    let hook = SheetOpsApprovalHook::new();

    for args in [
        json!({"action": ACTION_EXPORT_CSV, "source": "t.csv", "output": "o.csv"}),
        json!({"action": ACTION_FILTER, "source": "t.csv", "column": "a",
               "predicate": "eq", "value": "1", "output": "o.csv"}),
    ] {
        let decision = hook.evaluate(&dispatch(args)).await;
        let Decision::RequireApproval { reason } = decision else {
            panic!("写面必须每次人批: {decision:?}");
        };
        assert!(reason.contains("每次写面都停人工审批"), "{reason}");
    }

    for args in [
        json!({"action": ACTION_READ, "source": "t.csv"}),
        json!({"action": ACTION_SUMMARY, "source": "t.csv"}),
        json!({"action": ACTION_AGGREGATE, "source": "t.csv", "column": "a",
               "function": "sum"}),
        json!({"action": ACTION_FILTER, "source": "t.csv", "column": "a",
               "predicate": "eq", "value": "1"}),
    ] {
        let decision = hook.evaluate(&dispatch(args)).await;
        assert!(
            matches!(decision, Decision::Allow),
            "纯读应放行: {decision:?}"
        );
    }

    let decision = hook
        .evaluate(&dispatch(json!({"action": "rm_rf", "source": "t.csv"})))
        .await;
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "词表外必须 fail-closed 停审批: {decision:?}"
    );
}

/// ㉑ 边界 Guard 单调只拒不放: 敏感面/越界在流水线门禁就被拒, 正常读放行。
#[test]
fn boundary_guard_denies_before_execution() {
    let dir = workspace_with(&["sales.csv"]);
    std::fs::write(dir.path().join("credentials.csv"), "a\n1\n").expect("fixture");
    let guard = SheetOpsBoundaryGuard::new(SheetOpsConfig::new(dir.path()));
    let capability = CapabilityId::new(SHEET_OPS_CAPABILITY_ID).expect("capability id");

    let sensitive = json!({"action": ACTION_READ, "source": "credentials.csv"});
    let request = ToolGuardRequest::new(&capability, SHEET_OPS_TOOL_NAME, &sensitive);
    assert!(guard.deny(&request).is_some(), "敏感面必须被边界 Guard 拒");

    let escaping = json!({"action": ACTION_READ, "source": "../escape.csv"});
    let request = ToolGuardRequest::new(&capability, SHEET_OPS_TOOL_NAME, &escaping);
    assert!(guard.deny(&request).is_some(), "越界必须被边界 Guard 拒");

    let good = json!({"action": ACTION_READ, "source": "sales.csv"});
    let request = ToolGuardRequest::new(&capability, SHEET_OPS_TOOL_NAME, &good);
    assert!(
        guard.deny(&request).is_none(),
        "正常读不拦 (执行面另有语义拒)"
    );
}

/// ㉒ 参数矩阵: 未知动作 / 多余参数拒绝即帧; 写面判定钉死。
#[test]
fn parameter_matrix_rejects_unknown_actions_and_extra_fields() {
    let dir = workspace_with(&["sales.csv"]);
    let cfg = SheetOpsConfig::new(dir.path()).with_write_gate(true);

    let unknown = SheetOpsParams {
        action: Some("pivot".to_string()),
        source: Some("sales.csv".to_string()),
        ..SheetOpsParams::default()
    };
    let error = sheet_ops_params_refusal(&cfg, &unknown).expect("未知动作必须拒");
    assert!(
        error.frame().contains("sheet_ops.param_rejected"),
        "{}",
        error.frame()
    );

    let extra = SheetOpsParams {
        action: Some(ACTION_SUMMARY.to_string()),
        source: Some("sales.csv".to_string()),
        output: Some("o.csv".to_string()),
        ..SheetOpsParams::default()
    };
    let error = sheet_ops_params_refusal(&cfg, &extra).expect("多余参数必须拒");
    assert!(error.frame().contains("output"), "{}", error.frame());

    assert!(is_sheet_ops_write_action(ACTION_EXPORT_CSV, false));
    assert!(!is_sheet_ops_write_action(ACTION_SORT, false));
    assert!(is_sheet_ops_write_action(ACTION_SORT, true));
}

/// ㉓ 工具面 (声明名 `sheet_ops`): 成功回结构化帧, 坏参数回稳定拒绝帧。
#[tokio::test]
async fn tool_face_frames_successes_and_rejections() {
    let dir = workspace_with(&["sales.csv"]);
    let tool = SheetOpsTool::new(SheetOpsConfig::new(dir.path()).with_write_gate(true));

    let ok = tool
        .invoke(&ToolCall {
            id: "call_1".into(),
            name: SHEET_OPS_TOOL_NAME.into(),
            arguments: json!({"action": ACTION_SUMMARY, "source": "sales.csv"}),
        })
        .await;
    assert!(ok.is_ok(), "{}", ok.render());
    let ToolOutcome::Ok { value } = ok.outcome else {
        panic!("summary 应成功");
    };
    assert_eq!(value["action"], ACTION_SUMMARY);
    assert_eq!(value["status"], "ok");

    let bad = tool
        .invoke(&ToolCall {
            id: "call_2".into(),
            name: SHEET_OPS_TOOL_NAME.into(),
            arguments: json!({"action": "drop_table", "source": "sales.csv"}),
        })
        .await;
    assert!(!bad.is_ok());
    assert!(
        bad.render().contains("sheet_ops.param_rejected"),
        "{}",
        bad.render()
    );
}
