//! `doc_ops` (F9) + `sheet_ops` (F10) 两族的**结果面凭据形状绊线 (M1)** 行为钉子.
//!
//! # 覆盖面
//!
//! 两族读的是**用户文档 / 表格** ⇒ 文档里可能带**硬编码密钥** ⇒ ⇒ 这是「凭据进
//! 模型面」的路径 ⇒ ⇒ 两族的结果面必须过**形状层 (M1)**.
//!
//! 验收判据 (逐条钉死):
//!
//! ① 含 `sk-` + ≥20 字符的**正文** ⇒ 结果**无明文**且含 `[REDACTED_OPENAI_KEY]`;
//! ② **结构完好** ⇒ 键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级全不变;
//! ③ **摘要面不被误伤** ⇒ 统计类字段**逐字不变**（本批的关键判据）;
//! ④ **干净夹具逐字节全等** ⇒ 零命中 ⇒ 输出与接线前**完全相同**且**不挂侧车**;
//! ⑤ 命中**可观测** ⇒ 侧车存在且 `triggered: true`;
//! ⑥ **两族各自一条**（本文件两族各一组, 不许只测一族）.
//!
//! # 本族特有的坑（最容易做错的地方）
//!
//! 两族的结果帧 = **结构化摘要**（页数 / 行数 / 单元格数 / 操作统计）+ **正文**
//! （文档文本 / 单元格内容）⇒ ⇒ **只遮正文**, 摘要面**一字不动** ⇒ ⇒ 断言 ③ 的
//! 做法是: **把两帧的正文键摘掉后逐字比对** ⇒ 摘掉的部分若还相同 ⇒ 摘要面逐字
//! 未变; 摘掉的部分若变了 ⇒ 摘要面被误伤.
//!
//! # 确定性
//!
//! 固定夹具 (真 CSV / 真手写 PDF 字节), **不赌概率**: 期望帧是**手写常量**
//! ⇒ 逐字节可比; 结构指纹 (键名 / 类型 / 长度 / 层级) 单独可比.

use std::path::Path;

use serde_json::{json, Value};

use apeireth_tools_canonical::doc_ops::{
    DocOpsConfig, DocOpsParams, DocOpsRunner, ACTION_PDF_EXTRACT_TEXT,
};
use apeireth_tools_canonical::sheet_ops::{
    SheetOpsConfig, SheetOpsParams, SheetOpsRunner, ACTION_READ,
};

// ---------------------------------------------------------------------------
// 夹具 (真字节; 不用生产写端 —— 防循环自证)
// ---------------------------------------------------------------------------

/// 正文里的**假凭据** (形状: `sk-` + 24 字符 ⇒ ≥20 ⇒ 命中 `SkPrefixedKey` 族)。
///
/// ⚠️ 这是**形状夹具**, 不是真凭据: 它只用来触发形状层判据。
const FAKE_KEY: &str = "sk-0123456789abcdefghijkl";

/// 形状层的**遮形标签** (治理层既有字符串; 本测试不自造标签)。
const REDACTED: &str = "[REDACTED_OPENAI_KEY]";

/// 干净正文: 与 [`FAKE_KEY`] **等长** (25 字符) ⇒ 两份文档的统计值逐字相同
/// ⇒ ⇒ 摘要面可直接逐字比对 (字符预算等统计项不因正文内容而变)。
const CLEAN_KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaa";

/// 手写单页 PDF (真字节): 逐行 `Tj` 显示文本。
fn fixture_pdf(lines: &[&str]) -> Vec<u8> {
    let mut body = String::from("BT\n/F1 12 Tf\n72 720 Td\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            body.push_str("0 -14 Td\n");
        }
        body.push('(');
        body.push_str(line);
        body.push_str(") Tj\n");
    }
    body.push_str("ET\n");
    let objects: Vec<(u32, Vec<u8>)> = vec![
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
               /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{body}endstream", body.len()).into_bytes(),
        ),
        (
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        ),
    ];
    let count = objects.len();
    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize; count + 1];
    for (num, body_bytes) in &objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body_bytes);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", count + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets[1..] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            count + 1
        )
        .as_bytes(),
    );
    out
}

/// 建一个工作区临时目录并写入夹具 (真文件; 只写临时目录)。
fn workspace_with_pdf(line: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("doc.pdf"), fixture_pdf(&[line])).expect("写 PDF 夹具");
    dir
}

/// 建一个工作区临时目录并写入 CSV 夹具 (真文件; 只写临时目录)。
fn workspace_with_csv(content: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("table.csv"), content).expect("写 CSV 夹具");
    dir
}

// ---------------------------------------------------------------------------
// 断言助手
// ---------------------------------------------------------------------------

/// 值的**结构指纹**: 键名 / 类型 / 数组长度 / 嵌套层级 —— **不含任何字面量**。
///
/// ⇒ 正文被遮形后长度会变, 但结构指纹**不变** ⇒ 结构完好 (判据 ②) 可逐字比对。
fn shape(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => format!("bool({value})"),
        Value::Number(number) => format!("num({})", number.as_f64().unwrap_or_default()),
        Value::String(_) => "str".to_string(),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(shape).collect();
            format!("arr[{}]({})", items.len(), inner.join(","))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .map(|(key, value)| format!("{key}:{}", shape(value)))
                .collect();
            format!("obj{{{}}}", inner.join(","))
        }
    }
}

/// 摘掉若干键 (正文键 + 侧车键) ⇒ 余下的就是**摘要面**。
fn without_keys(frame: &Value, keys: &[&str]) -> Value {
    let mut copy = frame.clone();
    if let Value::Object(map) = &mut copy {
        for key in keys {
            map.remove(*key);
        }
    }
    copy
}

/// 侧车口径断言 (判据 ⑤): 侧车存在 + `triggered: true` + 命中族含形状族。
fn assert_tripwire_visible(frame: &Value, expected_kind: &str) {
    let sidecar = frame
        .get("credential_tripwire")
        .unwrap_or_else(|| panic!("命中必须挂 credential_tripwire 侧车: {frame}"));
    assert_eq!(
        sidecar.get("triggered").and_then(Value::as_bool),
        Some(true),
        "侧车 triggered 必须为 true: {frame}"
    );
    let kinds = sidecar
        .get("leaked_kinds")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("侧车必须带 leaked_kinds: {frame}"));
    assert!(
        kinds
            .iter()
            .any(|kind| kind.as_str() == Some(expected_kind)),
        "侧车必须报形状族 {expected_kind}: {frame}"
    );
}

// ---------------------------------------------------------------------------
// F10 `sheet_ops`: 单元格正文过形状层, 摘要面逐字不动
// ---------------------------------------------------------------------------

/// 干净表格 (两行两列; 表头与脏表**同构** ⇒ 摘要面可比)。
const CLEAN_CSV: &str = "id,note\n1,alpha\n2,beta\n";

/// 脏表格: **第二列的单元格**里带形状凭据 (表头与统计项与干净表同构)。
const DIRTY_CSV: &str = "id,note\n1,sk-0123456789abcdefghijkl\n2,plain\n";

fn sheet_runner(root: &Path) -> SheetOpsRunner {
    SheetOpsRunner::new(SheetOpsConfig::new(root).with_write_gate(true))
}

fn sheet_read(runner: &SheetOpsRunner) -> Value {
    let params: SheetOpsParams =
        serde_json::from_value(json!({"action": ACTION_READ, "source": "table.csv"}))
            .expect("valid params shape");
    runner.run(&params).expect("read 成功")
}

#[test]
fn sheet_ops_body_credential_is_masked_while_summary_stays_byte_identical() {
    let clean_dir = workspace_with_csv(CLEAN_CSV);
    let dirty_dir = workspace_with_csv(DIRTY_CSV);
    let clean = sheet_read(&sheet_runner(clean_dir.path()));
    let dirty = sheet_read(&sheet_runner(dirty_dir.path()));

    // ── ① 正文里的形状凭据: 无明文 + 有遮形标签 ────────────────────────────
    let dirty_text = serde_json::to_string(&dirty).expect("序列化");
    assert!(
        !dirty_text.contains(FAKE_KEY),
        "命中后正文不得留明文: {dirty_text}"
    );
    assert!(
        dirty_text.contains(REDACTED),
        "命中后必须含遮形标签 {REDACTED}: {dirty_text}"
    );

    // ── ② 结构完好: 键名 / 类型 / 数组长度 / 嵌套层级全不变 ──────────────────
    assert_eq!(
        shape(&without_keys(&dirty, &["credential_tripwire"])),
        shape(&clean),
        "遮形只许换字符串叶子: 结构 (键名/类型/数组长度/嵌套层级) 必须不变"
    );
    let rows = dirty
        .get("rows")
        .and_then(Value::as_array)
        .expect("rows 是数组");
    assert_eq!(rows.len(), 2, "行数不得变");
    let first = rows[0].as_array().expect("行是数组");
    assert_eq!(first.len(), 2, "列数不得变");
    assert_eq!(
        first[1].as_str(),
        Some(REDACTED),
        "命中的单元格必须是遮形标签"
    );
    assert_eq!(first[0].as_str(), Some("1"), "同行的其它单元格不得被改");
    assert_eq!(
        rows[1].as_array().expect("行是数组")[1].as_str(),
        Some("plain"),
        "未命中的单元格必须逐字保留"
    );

    // ── ③ 摘要面逐字不变 (本批关键判据) ─────────────────────────────────────
    let clean_summary = without_keys(&clean, &["rows"]);
    let dirty_summary = without_keys(&dirty, &["rows", "credential_tripwire"]);
    assert_eq!(
        serde_json::to_string(&dirty_summary).expect("序列化"),
        serde_json::to_string(&clean_summary).expect("序列化"),
        "摘要面必须逐字不变 (行数/列数/表头/截断标记/输入格式等)"
    );
    assert_eq!(
        dirty.get("row_count").and_then(Value::as_u64),
        Some(2),
        "行数统计不得变"
    );
    assert_eq!(
        dirty.get("truncated").and_then(Value::as_bool),
        Some(false),
        "截断标记不得变"
    );

    // ── ⑤ 命中可观测 ──────────────────────────────────────────────────────
    assert_tripwire_visible(&dirty, "OpenAiKey");
}

#[test]
fn sheet_ops_clean_fixture_is_byte_identical_and_carries_no_sidecar() {
    // ── ④ 干净夹具: 整帧逐字节等于接线前的形状, 且不挂侧车 ───────────────────
    let dir = workspace_with_csv(CLEAN_CSV);
    let frame = sheet_read(&sheet_runner(dir.path()));
    let expected = json!({
        "action": ACTION_READ,
        "status": "ok",
        "source": "table.csv",
        "columns": ["id", "note"],
        "rows": [["1", "alpha"], ["2", "beta"]],
        "row_count": 2,
        "offset": 0,
        "returned": 2,
        "truncated": false,
        "input_format": "csv",
    });
    assert_eq!(
        serde_json::to_string(&frame).expect("序列化"),
        serde_json::to_string(&expected).expect("序列化"),
        "干净夹具的结果帧必须与接线前逐字节相同"
    );
    assert!(
        frame.get("credential_tripwire").is_none(),
        "零命中不得挂侧车: {frame}"
    );
}

// ---------------------------------------------------------------------------
// F9 `doc_ops`: 文档正文过形状层, 摘要面逐字不动
// ---------------------------------------------------------------------------

fn doc_runner(root: &Path) -> DocOpsRunner {
    DocOpsRunner::new(DocOpsConfig::new(root).with_write_gate(true))
}

fn doc_extract_text(runner: &DocOpsRunner) -> Value {
    let params: DocOpsParams =
        serde_json::from_value(json!({"action": ACTION_PDF_EXTRACT_TEXT, "input": "doc.pdf"}))
            .expect("valid params shape");
    runner.run(&params).expect("pdf_extract_text 成功")
}

#[test]
fn doc_ops_body_credential_is_masked_while_summary_stays_byte_identical() {
    let clean_dir = workspace_with_pdf(CLEAN_KEY);
    let dirty_dir = workspace_with_pdf(FAKE_KEY);
    let clean = doc_extract_text(&doc_runner(clean_dir.path()));
    let dirty = doc_extract_text(&doc_runner(dirty_dir.path()));

    // ── ① 正文里的形状凭据: 无明文 + 有遮形标签 ────────────────────────────
    let dirty_text = serde_json::to_string(&dirty).expect("序列化");
    assert!(
        !dirty_text.contains(FAKE_KEY),
        "命中后正文不得留明文: {dirty_text}"
    );
    assert!(
        dirty_text.contains(REDACTED),
        "命中后必须含遮形标签 {REDACTED}: {dirty_text}"
    );

    // ── ② 结构完好: 键名 / 类型 / 数组长度 / 嵌套层级全不变 ──────────────────
    assert_eq!(
        shape(&without_keys(&dirty, &["credential_tripwire"])),
        shape(&clean),
        "遮形只许换字符串叶子: 结构 (键名/类型/数组长度/嵌套层级) 必须不变"
    );
    let pages = dirty
        .get("pages")
        .and_then(Value::as_array)
        .expect("pages 是数组");
    assert_eq!(pages.len(), 1, "页数不得变");
    let page = pages[0].as_object().expect("页是对象");
    assert_eq!(page.len(), 2, "页对象的键数不得变");
    assert!(
        page.get("page").and_then(Value::as_u64) == Some(1),
        "页码必须仍是数字 1"
    );
    assert!(
        page.get("text").and_then(Value::as_str) == Some(REDACTED),
        "命中的正文必须是遮形标签"
    );

    // ── ③ 摘要面逐字不变 (本批关键判据) ─────────────────────────────────────
    let clean_summary = without_keys(&clean, &["pages"]);
    let dirty_summary = without_keys(&dirty, &["pages", "credential_tripwire"]);
    assert_eq!(
        serde_json::to_string(&dirty_summary).expect("序列化"),
        serde_json::to_string(&clean_summary).expect("序列化"),
        "摘要面必须逐字不变 (字符预算/页数/截断标记/备注等)"
    );
    assert_eq!(
        dirty.get("chars").and_then(Value::as_u64),
        Some(CLEAN_KEY.len() as u64),
        "字符预算必须仍是**原文**长度 (未被遮形后的长度改写 ⇒ 摘要面未被重算)"
    );

    // ── ⑤ 命中可观测 ──────────────────────────────────────────────────────
    assert_tripwire_visible(&dirty, "OpenAiKey");
}

#[test]
fn doc_ops_clean_fixture_is_byte_identical_and_carries_no_sidecar() {
    // ── ④ 干净夹具: 整帧逐字节等于接线前的形状, 且不挂侧车 ───────────────────
    let dir = workspace_with_pdf(CLEAN_KEY);
    let frame = doc_extract_text(&doc_runner(dir.path()));
    let expected = json!({
        "action": ACTION_PDF_EXTRACT_TEXT,
        "status": "ok",
        "input": "doc.pdf",
        "pages": [{"page": 1, "text": CLEAN_KEY}],
        "chars": CLEAN_KEY.len(),
        "truncated": false,
        "notes": [],
    });
    assert_eq!(
        serde_json::to_string(&frame).expect("序列化"),
        serde_json::to_string(&expected).expect("序列化"),
        "干净夹具的结果帧必须与接线前逐字节相同"
    );
    assert!(
        frame.get("credential_tripwire").is_none(),
        "零命中不得挂侧车: {frame}"
    );
}
