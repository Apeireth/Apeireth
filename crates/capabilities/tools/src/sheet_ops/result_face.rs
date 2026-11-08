//! `sheet_ops` 族的**结果面**工具: 帧构造 + M1 凭据形状绊线 (形状层遮形).
//!
//! # 本族结果帧的形状 (混合面)
//!
//! `sheet_ops` 的结果帧 = **结构化摘要** + **表格正文** 两部分混在一起:
//!
//! | 面 | 承载物 | 处置 |
//! |---|---|---|
//! | 摘要面 | 行数 / 列数 / 命中数 / 聚合值 / 字节数 / 截断标记 | **一字不动** |
//! | 正文面 | 单元格正文 (`rows`) + 表头名 (`columns` / `column`) | **过形状层 (M1)** |
//!
//! ⚠️ **只遮正文面是本族最容易做错的地方**: 摘要面若跟着一起变形 (行数变成遮形
//! 标签), 输出就**不可读了** (统计失真) ⇒ ⇒ 摘要面**逐字保留**.
//!
//! # 「什么算正文」的判定口径 (可复核, 不含糊)
//!
//! **正文 = 从用户表格里读出来的内容** (含它的回显):
//!
//! * `rows` = 单元格正文;
//! * `columns` = 表头名 (表头也是文件内容);
//! * `column` = 被筛 / 被排 / 被聚合的**列名回显** ⇒ **必须与 `columns` 同口径**
//!   —— 否则同一个表头字符串「一处遮、一处不遮」⇒ ⇒ 那就是漏.
//!
//! **不是正文** (一律不动): 路径 (`source` / `output`) / 参数回显 (`predicate` /
//! `order` / `value` / `expression` / `new_column` / `function`) / 本族自产的固定
//! 文本 (`action` / `status` / `notes` / `input_format` / `output_format` /
//! `sheet_name` / `unsupported`) / **统计数字** (`row_count` / `column_count` /
//! `offset` / `returned` / `truncated` / `matched` / `count` / `rows_written` /
//! `bytes_written` / `cells_written` / `formula_cells`).
//!
//! # 为什么正文面必须过形状层
//!
//! 本族读的是**用户表格** ⇒ 单元格 / 表头里可能带**硬编码密钥** ⇒ ⇒ 这是「凭据进
//! 模型面」的路径 ⇒ ⇒ 正文面必须就地遮形 (命中**不改判、不拒执行**, 只遮形).
//!
//! # 口径
//!
//! * **形状层 (M1)**: 族判据与族标签的**唯一真源**在治理层
//!   (`apeireth_governance::credential_sweep`) ⇒ ⇒ 本面**不自造**判据, 一律走
//!   `crate::guardrail` 的共享助手 (`scan_json_string_leaves` /
//!   `attach_credential_tripwire`);
//! * **不接字段层 (M4)**: 本族接线前**没有**字段层 ⇒ 接了会引入**新的可见变形**
//!   ⇒ ⇒ 与 `repo` 族同口径 (只走形状层);
//! * **层序**: 形状层 → 字段层 ⇒ 本批**只有**形状层 ⇒ 无反序;
//! * **零命中 ⇒ 结果帧逐字节不变** (那一支**不重建**、**不挂侧车**);
//! * **命中 ⇒ 顶栏挂 `credential_tripwire` 侧车** (`triggered` + `leaked_kinds`).
//!
//! # 已知边界 (如实登记, 不粉饰)
//!
//! **路径面与参数回显面不遮** (`source` / `output` / `value` / `expression` …).
//! 理由: 它们是**位置与用户自填词**, 不是表格正文 ⇒ 遮了会动到摘要面的可读性;
//! 且**敏感面路径另有拒读闸** (`sensitive_path` 族词表) ⇒ ⇒ 若 owner 认为
//! 「参数回显里也可能有硬编码密钥」, 那是**另一批**的裁决点 (本批不擅自扩面).

use serde_json::{json, Value};

use crate::guardrail::{attach_credential_tripwire, scan_json_string_leaves, LeakedCredentialKind};

use super::{SheetInputFormat, SheetLoadMeta};

/// 本族结果帧的**正文键**: 单元格正文 + 表头名 (含列名回显).
///
/// ⚠️ **摘要面不在此列** ⇒ ⇒ 行数 / 列数 / 命中数 / 聚合值 / 字节数 / 截断标记等
/// **统计字段逐字不动**。
fn body_keys() -> &'static [&'static str] {
    &["rows", "columns", "column"]
}

/// 走一遍 M1 形状层 (**只遮正文面**) ⇒ 终稿结果帧。
///
/// 零命中 ⇒ **原帧逐字节不变** (那一支不重建、不挂侧车);
/// 命中 ⇒ 正文面就地遮形 + 顶栏挂 `credential_tripwire` 侧车。
pub(crate) fn mask_body(frame: Value, _action: &str) -> Value {
    let mut frame = frame;
    let mut kinds: Vec<LeakedCredentialKind> = Vec::new();
    for key in body_keys() {
        let Some(sub) = frame.get_mut(*key) else {
            continue;
        };
        // 形状层只换**字符串叶子**: 键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级
        // 一律不动 (摘要面因此天然完好)。
        let (scanned, sub_kinds) = scan_json_string_leaves(sub);
        if sub_kinds.is_empty() {
            // 零命中 ⇒ **不重建**这一支 (原值逐字节保留)。
            continue;
        }
        *sub = scanned;
        kinds.extend(sub_kinds);
    }
    if kinds.is_empty() {
        return frame;
    }
    attach_credential_tripwire(&mut frame, &kinds);
    frame
}

/// `json!` 对象 → map (帧字段增补用)。
pub(super) fn into_map(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// 帧随行: 输入格式恒注 (`input_format`); xlsx 另注 `formula_cells` 计数,
/// 有公式格时带缓存值口径注记 (CSV 恒 0, 不注)。
pub(super) fn annotate_sheet_meta(
    frame: &mut serde_json::Map<String, Value>,
    meta: &SheetLoadMeta,
) {
    frame.insert(
        "input_format".to_string(),
        Value::String(meta.format.name().to_string()),
    );
    if meta.format == SheetInputFormat::Xlsx {
        frame.insert("formula_cells".to_string(), json!(meta.formula_cells));
        if meta.formula_cells > 0 {
            frame.insert(
                "notes".to_string(),
                json!([format!(
                    "公式格 {} 个: 一律取缓存值未重算 (共享公式不展开, 如实标注口径)",
                    meta.formula_cells
                )]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 测试 (形状层真源在治理层; 本面只做「面」的编排)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── 结果面凭据形状绊线 (M1) ────────────────────────────────

    /// 一枚**形状合法**的密钥串 (形状夹具, 不是真凭据): `sk-` + >=20 字节。
    const FAKE_KEY: &str = "sk-sheet-ops-fixture-abcdefghijklmnop";

    /// 正文面 (行 / 列 / 单列) 里的凭据形状 ⇒ **就地遮形** + **顶栏挂侧车**。
    #[test]
    fn the_body_face_masks_a_credential_shape_and_keeps_the_summary_intact() {
        let frame = serde_json::json!({
            "rows": [["单元格", format!("{FAKE_KEY}")]],
            "row_count": 1,
            "col_count": 2,
            "output": "/tmp/out.csv",
        });
        let masked = super::mask_body(frame, "read");

        let cell = masked["rows"][0][1].as_str().expect("单元格仍是字符串");
        assert!(!cell.contains(FAKE_KEY), "明文留在正文面: {cell}");
        assert!(
            cell.contains("[REDACTED_OPENAI_KEY]"),
            "正文面无遮形标签: {cell}"
        );
        // 摘要面逐字不动 (计数 / 路径 / 其它单元格)。
        assert_eq!(masked["row_count"].as_u64(), Some(1), "行数被改了");
        assert_eq!(masked["col_count"].as_u64(), Some(2), "列数被改了");
        assert_eq!(
            masked["output"].as_str(),
            Some("/tmp/out.csv"),
            "路径面被改了"
        );
        assert_eq!(
            masked["rows"][0][0].as_str(),
            Some("单元格"),
            "干净单元格被改了"
        );
        // 命中可观测。
        let sidecar = masked.get("credential_tripwire").expect("命中必须有侧车");
        assert_eq!(
            sidecar.get("triggered").and_then(|v| v.as_bool()),
            Some(true),
            "侧车未标记命中"
        );
    }

    /// **干净帧 ⇒ 逐字节全等** (零命中 ⇒ 不重建那一支、不挂侧车)。
    #[test]
    fn a_clean_body_face_is_byte_identical_and_carries_no_sidecar() {
        let frame = serde_json::json!({
            "rows": [["表头", "普通内容"]],
            "row_count": 1,
        });
        let before = frame.to_string();
        let masked = super::mask_body(frame, "read");
        assert_eq!(masked.to_string(), before, "零命中却变了");
        assert!(
            masked.get("credential_tripwire").is_none(),
            "零命中却挂了侧车"
        );
    }
}
