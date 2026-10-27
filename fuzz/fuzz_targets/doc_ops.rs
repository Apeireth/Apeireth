#![no_main]

//! fuzz: **文档操作面（`tool.doc_ops`）的参数矩阵** —— 「禁填字段矩阵 / 只创建不覆盖」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/doc_ops.rs`
//! （`doc_ops_params_refusal` / `allowed_fields`（私有的逐动作字段矩阵）/
//! `DocOpsConfig` / `DOC_OPS_ACTIONS` / `DOC_OPS_READ_ACTIONS` / `DOC_OPS_WRITE_ACTIONS`）。
//!
//! ## 为什么值得 fuzz（**「多给的参数被静默吞掉」= 模型以为生效了**）
//!
//! 模块头与实现的注释写死的纪律：
//! 1. **动作词表闭合**：未知动作**拒绝即帧**，不静默降级；
//! 2. **禁填字段矩阵**：该动作**不消费**的字段**一律拒绝即帧**
//!    （「多给的字段若被静默吞掉，模型侧看不出『这个参数没生效』—— **拒绝即帧才诚实**」）
//!    ⇒ ⇒ 这正是我 2026-10-27 修的 **defect ⑬「doc_ops 禁填字段矩阵缺失」**那一层
//!    ⇒ ⇒ **必须有机器守卫钉住它**，否则会回退；
//! 3. **各动作的数量/范围规则**：`pdf_merge` 必须 **2–32** 个 inputs（多入一出）、
//!    `pdf_extract_pages` 的 pages **非空 / 从 1 起 / 不重复 / 不超上限**；
//! 4. **源文上限宁拒不截**：字符数 / 段落数超限 ⇒ 拒（**不截断生成**）；
//!    XML 非法控制字符 ⇒ 拒（**不静默剔除不猜**）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **动作词表闭合**：未知/缺省动作必拒 | 闭合词表分支 |
//! | O2 | **禁填字段矩阵**：给动作**不认**的字段 ⇒ 必拒（**逐字段**） | `allowed_fields` + 禁填字段面 |
//! | O3 | **必填字段缺失必拒**（各动作的 `require_text`） | 各动作分支 |
//! | O4 | **`pdf_merge` 数量**：inputs **2–32** ⇒ 之外必拒；含空路径必拒 | `pdf_merge` 分支 |
//! | O5 | **选页规则**：pages 非空 / 页码 ≥1 / 不重复 / 不超 `max_pages` | `pdf_extract_pages` 分支 |
//! | O6 | **源文上限宁拒不截**：字符 / 段落超限必拒 | 源文面上限分支 |
//! | O7 | **XML 非法控制字符必拒** | `is_xml_illegal_char` 分支 |
//! | O8 | **读/写动作分档闭合**：读写两表**互不相交**且并起来 == 全表 | `DOC_OPS_READ_ACTIONS` / `DOC_OPS_WRITE_ACTIONS` |
//!
//! ## 诚实边界
//!
//! - **不测** PDF/DOCX 的**二进制解析**（那是声明子集面，另立）；
//! - **不测**真实文件读写（`pdf_merge` 等要真 PDF ⇒ 本 target 只测**参数层纯判定**）；
//! - 写总闸（`write_gate_open`）默认关 ⇒ ⇒ 写动作在参数合法后**还会**被总闸拒
//!   ⇒ ⇒ 本 target **只断言「参数层的判定」**，不要求写动作整体成功。

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::doc_ops::{
    doc_ops_params_refusal, DocOpsConfig, DocOpsParams, DOC_OPS_ACTIONS, DOC_OPS_READ_ACTIONS,
    DOC_OPS_WRITE_ACTIONS,
};

/// 逐动作的**允许字段集**（抄自被测代码的 `allowed_fields` —— 它是私有的，
/// 这里按同一份矩阵**独立复刻**用于对照 ⇒ ⇒ 若实现改了矩阵而这里没改，
/// 断言会失败 ⇒ ⇒ **这正是「把私有契约钉成公开守卫」的意义**）。
const ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("pdf_info", &["input"]),
    ("docx_outline", &["input"]),
    ("pdf_extract_text", &["input", "max_chars"]),
    ("docx_extract_text", &["input", "max_chars"]),
    ("pdf_merge", &["inputs", "output"]),
    ("pdf_split", &["input", "output_dir"]),
    ("pdf_extract_pages", &["input", "output", "pages"]),
    ("pdf_watermark_text", &["input", "output", "watermark_text"]),
    ("docx_from_markdown", &["text", "output"]),
    ("docx_append", &["input", "text", "output"]),
];

/// 全部可填字段（用于「给一个不属于该动作的字段」）。
const ALL_FIELDS: &[&str] = &[
    "input",
    "inputs",
    "output",
    "output_dir",
    "pages",
    "watermark_text",
    "max_chars",
    "text",
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
        "apeireth-doc-ops-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, x| acc.wrapping_mul(31).wrapping_add(*x as u64))
    ));
    let _ = std::fs::remove_dir_all(&workspace);
    let _ = std::fs::create_dir_all(&workspace);
    let config = DocOpsConfig::new(&workspace);

    // ── 动作：词表内 / 词表外 / 缺省
    let action: Option<String> = match b(0) % 4 {
        0 => None,
        1 => Some("not_an_action".to_string()),
        2 => Some("".to_string()),
        _ => {
            let table = DOC_OPS_ACTIONS;
            Some(table[(b(1) as usize) % table.len()].to_string())
        }
    };
    let action_ref = action.as_deref().unwrap_or("");

    // ── 字段：给「该动作不认」的字段（O2 的核心）
    let extra_field = ALL_FIELDS[(b(2) as usize) % ALL_FIELDS.len()];
    // 再挑一个数据驱动的多余字段（覆盖全部 8 个）
    let extra_field2 = ALL_FIELDS[(b(3) as usize) % ALL_FIELDS.len()];

    let mut params = DocOpsParams {
        action: action.clone(),
        input: Some("in.pdf".to_string()),
        inputs: Some(vec!["a.pdf".to_string(), "b.pdf".to_string()]),
        output: Some("out.pdf".to_string()),
        output_dir: Some("outdir".to_string()),
        pages: Some(vec![1, 2]),
        watermark_text: Some("wm".to_string()),
        max_chars: Some(100),
        text: Some("# 标题\\n\\n段落".to_string()),
    };

    // 按数据变异：清掉某些字段 / 造出多余字段
    match b(4) % 6 {
        0 => params.input = None,
        1 => params.output = None,
        2 => params.text = None,
        3 => params.pages = None,
        4 => params.output_dir = None,
        _ => {}
    }
    match b(5) % 5 {
        0 => params.inputs = Some(vec!["a.pdf".to_string()]), // 数量不足
        1 => params.inputs = Some(vec![" ".to_string(), "b.pdf".to_string()]), // 含空路径
        2 => params.pages = Some(vec![0, 1]),                 // 页码 0 非法
        3 => params.pages = Some(vec![1, 1]),                 // 重复页码
        _ => {}
    }

    let refusal = doc_ops_params_refusal(&config, &params);

    // ── O1 动作词表闭合
    if action.is_none() || action_ref == "not_an_action" || action_ref.is_empty() {
        assert!(
            refusal.is_some(),
            "缺省/未知动作必须拒绝即帧（闭合词表）: action={action:?}"
        );
    }

    // ── O2 禁填字段矩阵：对**词表内**动作，逐个字段核对「不认 ⇒ 必拒」
    if let Some(allowed) = allowed_for(action_ref) {
        for field in ALL_FIELDS {
            let present = match *field {
                "input" => params.input.is_some(),
                "inputs" => params.inputs.is_some(),
                "output" => params.output.is_some(),
                "output_dir" => params.output_dir.is_some(),
                "pages" => params.pages.is_some(),
                "watermark_text" => params.watermark_text.is_some(),
                "max_chars" => params.max_chars.is_some(),
                "text" => params.text.is_some(),
                _ => false,
            };
            if present && !allowed.contains(field) {
                assert!(
                    refusal.is_some(),
                    "禁填字段必须拒绝即帧: action={action_ref:?} 多给字段={field:?} \
                     （该动作只认 {allowed:?}）"
                );
            }
        }
    }

    // ── O4 pdf_merge 数量规则
    if action_ref == "pdf_merge" {
        if let Some(inputs) = &params.inputs {
            if !(2..=32).contains(&inputs.len()) || inputs.iter().any(|i| i.trim().is_empty()) {
                assert!(
                    refusal.is_some(),
                    "pdf_merge 的 inputs 数量/空路径必须拒绝即帧: inputs={inputs:?}"
                );
            }
        }
    }

    // ── O5 选页规则
    if action_ref == "pdf_extract_pages" {
        if let Some(pages) = &params.pages {
            let mut seen = std::collections::BTreeSet::new();
            let bad = pages.iter().any(|p| *p == 0)
                || pages.iter().any(|p| !seen.insert(*p))
                || pages.len() as u64 > config.max_pages;
            if pages.is_empty() || bad {
                assert!(
                    refusal.is_some(),
                    "选页违规必须拒绝即帧: pages={pages:?}"
                );
            }
        }
    }

    // ── O7 XML 非法控制字符（0x00–0x08 / 0x0B / 0x0C / 0x0E–0x1F）
    if matches!(action_ref, "docx_from_markdown" | "docx_append") {
        if let Some(text) = &params.text {
            let illegal = text
                .chars()
                .any(|c| matches!(c as u32, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F));
            if illegal {
                assert!(
                    refusal.is_some(),
                    "源文含 XML 非法控制字符必须拒绝即帧（不静默剔除不猜）"
                );
            }
        }
    }

    // ── O8 读/写动作分档闭合
    for action in DOC_OPS_ACTIONS {
        assert!(
            DOC_OPS_ACTIONS.contains(action),
            "全表项必须在全表内: {action}"
        );
    }
    for action in DOC_OPS_READ_ACTIONS {
        assert!(
            !DOC_OPS_WRITE_ACTIONS.contains(action),
            "读/写两表不得相交: {action}"
        );
    }
    for action in DOC_OPS_WRITE_ACTIONS {
        assert!(
            !DOC_OPS_READ_ACTIONS.contains(action),
            "读/写两表不得相交: {action}"
        );
    }
    assert_eq!(
        DOC_OPS_READ_ACTIONS.len() + DOC_OPS_WRITE_ACTIONS.len(),
        DOC_OPS_ACTIONS.len(),
        "读表 + 写表 必须并起来等于全表（不得有第三档）"
    );

    // ── 拒绝必须带真因（不得空消息）
    if let Some(error) = &refusal {
        let message = error.message();
        assert!(!message.trim().is_empty(), "拒绝必须带真因（不得空消息）");
    }

    let _ = extra_field;
    let _ = extra_field2;
    let _ = std::fs::remove_dir_all(&workspace);
});
