//! `doc_ops` 族的**结果面**工具: 帧读取 + M1 凭据形状绊线 (形状层遮形).
//!
//! # 本族结果帧的形状 (混合面)
//!
//! `doc_ops` 的结果帧 = **结构化摘要** + **文档正文** 两部分混在一起:
//!
//! | 面 | 承载物 | 处置 |
//! |---|---|---|
//! | 摘要面 | 页数 / 字符数 / 段落数 / 标题数 / 字节数 / 逐页尺寸 / 备注 | **一字不动** |
//! | 正文面 | 逐页文本 / Markdown 转出 / 标题正文 / 水印文字 | **过形状层 (M1)** |
//!
//! ⚠️ **只遮正文面是本族最容易做错的地方**: 摘要面若跟着一起变形 (页数变成遮形
//! 标签), 输出就**不可读了** (统计失真) ⇒ ⇒ 摘要面**逐字保留**.
//!
//! # 为什么正文面必须过形状层
//!
//! 本族读的是**用户文档** ⇒ 文档里可能带**硬编码密钥** ⇒ ⇒ 这是「凭据进模型面」
//! 的路径 ⇒ ⇒ 正文面必须就地遮形 (命中**不改判、不拒执行**, 只遮形).
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
//! **路径面与参数回显面不遮** (`input` / `output` / `output_dir` / `files` / `notes`).
//! 理由: 它们是**位置与本族自产文本**, 不是文档正文 ⇒ 遮了会动到「摘要面」的
//! 可读性; 且**敏感面路径另有拒读闸** (`sensitive_path` 族词表) ⇒ ⇒ 若 owner 认为
//! 「路径里也可能有硬编码密钥」, 那是**另一批**的裁决点 (本批不擅自扩面).

use serde_json::Value;

use crate::guardrail::{attach_credential_tripwire, scan_json_string_leaves, LeakedCredentialKind};

use super::{
    ACTION_DOCX_EXTRACT_TEXT, ACTION_DOCX_OUTLINE, ACTION_PDF_EXTRACT_TEXT,
    ACTION_PDF_WATERMARK_TEXT,
};

/// 本族结果帧的**正文键** (逐动作): 只有这些键承载「文档正文」⇒ 只有它们过形状层.
///
/// ⚠️ **摘要面不在此列** ⇒ ⇒ 页数 / 字符数 / 段落数 / 标题数 / 字节数 / 逐页尺寸 /
/// 备注等**统计与结构**字段**逐字不动**。
fn body_keys(action: &str) -> &'static [&'static str] {
    match action {
        // 逐页文本 (`{"page": n, "text": "…"}`): 只换 `text` 叶子, `page` 数字不动。
        ACTION_PDF_EXTRACT_TEXT => &["pages"],
        // DOCX 正文 → Markdown 转出 (整段正文)。
        ACTION_DOCX_EXTRACT_TEXT => &["markdown"],
        // 标题层级大纲 (`{"level": n, "text": "…"}`): 同上, 只换 `text` 叶子。
        ACTION_DOCX_OUTLINE => &["outline"],
        // 写入的水印文字 (用户正文, 随帧回显)。
        ACTION_PDF_WATERMARK_TEXT => &["watermark"],
        // 其余动作的结果帧**不含文档正文** (只有摘要 / 路径 / 计数) ⇒ 一律不动。
        _ => &[],
    }
}

/// 走一遍 M1 形状层 (**只遮正文面**) ⇒ 终稿结果帧。
///
/// 零命中 ⇒ **原帧逐字节不变** (那一支不重建、不挂侧车);
/// 命中 ⇒ 正文面就地遮形 + 顶栏挂 `credential_tripwire` 侧车。
pub(crate) fn mask_body(frame: Value, action: &str) -> Value {
    let mut frame = frame;
    let mut kinds: Vec<LeakedCredentialKind> = Vec::new();
    for key in body_keys(action) {
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

/// 从帧里读处理页数 (u64 = 页数; 数组 = 返回页行数)。
pub(super) fn frame_pages_touched(frame: &Value) -> u64 {
    match frame.get("pages") {
        Some(Value::Number(number)) => number.as_u64().unwrap_or(0),
        Some(Value::Array(items)) => items.len() as u64,
        _ => 0,
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
    const FAKE_KEY: &str = "sk-doc-ops-fixture-abcdefghijklmnop";

    /// 正文面里的凭据形状 ⇒ **就地遮形** + **顶栏挂侧车**。
    #[test]
    fn the_body_face_masks_a_credential_shape_and_keeps_the_summary_intact() {
        let frame = serde_json::json!({
            "pages": [{"page": 1, "text": format!("内含 {FAKE_KEY}")}],
            "page_count": 3,
            "char_count": 42,
            "output": "/tmp/out.txt",
        });
        let masked = super::mask_body(frame, ACTION_PDF_EXTRACT_TEXT);

        let text = masked["pages"][0]["text"].as_str().expect("正文仍是字符串");
        assert!(!text.contains(FAKE_KEY), "明文留在正文面: {text}");
        assert!(
            text.contains("[REDACTED_OPENAI_KEY]"),
            "正文面无遮形标签: {text}"
        );
        // 摘要面逐字不动 (页号 / 计数 / 路径)。
        assert_eq!(masked["pages"][0]["page"].as_u64(), Some(1), "页号被改了");
        assert_eq!(masked["page_count"].as_u64(), Some(3), "页数被改了");
        assert_eq!(masked["char_count"].as_u64(), Some(42), "字符数被改了");
        assert_eq!(
            masked["output"].as_str(),
            Some("/tmp/out.txt"),
            "路径面被改了"
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
            "pages": [{"page": 1, "text": "普通正文, 没有凭据"}],
            "page_count": 1,
        });
        let before = frame.to_string();
        let masked = super::mask_body(frame, ACTION_PDF_EXTRACT_TEXT);
        assert_eq!(masked.to_string(), before, "零命中却变了");
        assert!(
            masked.get("credential_tripwire").is_none(),
            "零命中却挂了侧车"
        );
    }
}
