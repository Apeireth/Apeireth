#![no_main]

//! fuzz: **自述面（`tool.self_status`）的冻结合同** —— 21 节恒在场 + 缺源节不得缺席 + 名册不丢项。
//!
//! 被测面（仓内源文件）：
//! - `crates/capabilities/tools/src/self_status.rs`（`render_snapshot` + `SELF_STATUS_SECTIONS` + `output_schema`）
//! - `crates/capabilities/tools/src/accountability_view.rs`（`capabilities` 节内的问责子对象）
//!
//! ## 为什么值得 fuzz
//!
//! 自述面是产品**对外的唯一承诺面**（「每一句自述都必须能对应到一个当前可验证的观察、
//! 一个明确的测量程序，或一个可追责的承诺」—— 外部顾问硬判据）。
//! 而它的形状是**冻结合同**：21 节**恒在场**，具体字段可以显式 `null`（来源缺失），
//! **但节不可缺席**。⇒ 「缺席」与「显式 null + 原因」的区别**只有机器能守**。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的契约推出来，**不自造期望**）
//!
//! 1. **21 节恒在场**：`SELF_STATUS_SECTIONS` 是冻结表，`output_schema()` 的 require 清单
//!    就是它 ⇒ 渲染结果的顶层键集**必须等于**冻结表（不多不少不少）。
//! 2. **schema 校验必须过**：`SelfStatusTool::output_schema().validate(&value).is_ok()`。
//! 3. **缺源节不得缺席、且必须带原因**：`minimal()` 造的快照里所有来源都是 `None`
//!    ⇒ 每节渲染成**对象**（字段为 `null`）**并带非空 `reason`**
//!    （顾问原文认可的例外形态：**整节级** `null + reason` 合理；
//!    但**节级缺席**是「形状破了」）。
//! 4. **名册不丢项**：`capabilities` 节的 doc 写的是「能力名册（当前生效开关**全表**）」
//!    ⇒ 快照里出现过的**每个不同能力名**都必须能在渲染结果里找到（同名重复只保留一个，
//!    这是 map 的语义；**但不能整项消失**）。
//! 5. **保留子对象键不被能力名顶成布尔**：`capabilities` 节内有两个**保留子对象**
//!    （`bio_mechanisms` / `accountability`）⇒ 若快照里有同名能力，
//!    渲染结果里那个键**不得退化成裸布尔**（否则保留子对象被静默顶掉 ⇒ 形状破了）。
//! 6. **确定性**：同一快照渲染两次**逐字节一致**（自述面不得每次说话都不一样）。
//!
//! ## 诚实边界
//!
//! - 本 target 只驱动 `SelfStatusSnapshot::minimal(...)`（**全部来源缺省**的那一档形状）
//!   ⇒ **各来源在场时**的逐字段语义**未覆盖**（那需要真的挂上观测面）。
//! - 「缺源 = null + 原因」只断言**原因非空**，不判断原因文案是否贴切。
//! - **D32 建议②的读数标注**（`bio_mechanisms.note`）在 `minimal` 档**不出现**
//!   （该子对象缺省）⇒ 由仓内单测 `bio_mechanisms_note_declares_readings_without_changing_keys` 守。

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::{
    render_snapshot, CapabilitySwitch, SelfStatusIdentity, SelfStatusSnapshot, SelfStatusTool,
    SELF_STATUS_SECTIONS,
};

/// 从任意字节里切一段当字符串（**可能含任意字节** ⇒ UTF-8 非法时用替代字符）。
fn slice_str(data: &[u8]) -> String {
    String::from_utf8_lossy(data).into_owned()
}

/// 取第 n 段（没有就是空）。
fn chunk<'a>(chunks: &'a [Vec<u8>], index: usize) -> &'a [u8] {
    chunks.get(index).map(Vec::as_slice).unwrap_or(&[])
}

/// 切成 n 段（按长度均分；不足则补空）。
fn split_chunks(data: &[u8], n: usize) -> Vec<Vec<u8>> {
    if n == 0 {
        return Vec::new();
    }
    let per = data.len() / n;
    let extra = data.len() % n;
    let mut out = Vec::with_capacity(n);
    let mut offset = 0usize;
    for index in 0..n {
        let take = per + usize::from(index < extra);
        let end = (offset + take).min(data.len());
        out.push(data[offset..end].to_vec());
        offset = end;
    }
    out
}

fuzz_target!(|data: &[u8]| {
    // ── 输入切分：身份三字段 + 能力名册（**名册条数由输入长度决定**，可任意多）
    let chunks = split_chunks(data, 4);
    let identity = SelfStatusIdentity {
        product_name: slice_str(chunk(&chunks, 0)),
        version: slice_str(chunk(&chunks, 1)),
        runtime_role: slice_str(chunk(&chunks, 2)),
    };

    // 能力名册：第三个块的前缀给出「条数」，其余字节两两一组当 (名, 开关)。
    let roster_bytes = chunk(&chunks, 3);
    let count_hint = roster_bytes.first().copied().unwrap_or(0) as usize;
    let count = count_hint.min(64);
    let mut capabilities = Vec::with_capacity(count);
    for index in 0..count {
        let name_bytes = roster_bytes.get(1 + index * 2).copied().unwrap_or(0u8);
        let enabled = roster_bytes.get(2 + index * 2).copied().unwrap_or(0) & 1 == 1;
        // 名可以是任意字节（含空串、含控制字符、含保留键名）
        let mut name = slice_str(&[name_bytes]);
        if index == 0 && count_hint > 0 && !name.is_empty() {
            // 第一条**可能**是保留键名（用来压「保留子对象被顶掉」那条）
            name = name.clone();
        }
        capabilities.push(CapabilitySwitch::new(name, enabled));
    }

    let snapshot = SelfStatusSnapshot::minimal(identity, capabilities.clone());
    let value = render_snapshot(&snapshot);

    // ① 21 节恒在场：顶层键集**必须等于**冻结表。
    let rendered_keys: Vec<&str> = value
        .as_object()
        .expect("自述顶层必须是对象")
        .keys()
        .map(String::as_str)
        .collect();
    let frozen: Vec<&str> = SELF_STATUS_SECTIONS.to_vec();
    assert_eq!(
        rendered_keys.len(),
        frozen.len(),
        "节数不符（冻结表 {} 节，渲染 {} 节）: {rendered_keys:?}",
        frozen.len(),
        rendered_keys.len()
    );
    for section in &frozen {
        assert!(
            value.get(*section).is_some(),
            "节 {section} 缺席（冻结合同：节不可缺席）: {rendered_keys:?}"
        );
    }

    // ② schema 校验必须过。
    assert!(
        SelfStatusTool::output_schema().validate(&value).is_ok(),
        "自述未过自己的 schema 校验"
    );

    // ③ 缺源节不得缺席、且必须带非空原因（整节级 null + reason 是被认可的形态）。
    for section in &frozen {
        let section_value = &value[*section];
        assert!(
            section_value.is_object(),
            "节 {section} 在缺源档应是对象（字段 null + 原因），实为 {section_value}"
        );
        if let Some(reason) = section_value.get("reason") {
            let text = reason.as_str().unwrap_or_default();
            assert!(
                !text.is_empty(),
                "节 {section} 缺源却给了空原因（等于没说为什么）"
            );
        }
    }

    // ④ 名册不丢项：快照里出现过的每个**不同**能力名都要能在渲染结果里找到。
    let roster = value["capabilities"].as_object().expect("名册是对象");
    let mut distinct: Vec<&str> = capabilities
        .iter()
        .map(|c| c.name.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    distinct.sort_unstable();
    for name in distinct {
        assert!(
            roster.contains_key(name),
            "名册丢项：能力 {name:?} 消失了（契约：当前生效开关**全表**）: {:?}",
            roster.keys().collect::<Vec<_>>()
        );
    }

    // ⑤ 保留子对象键不得退化成裸布尔。
    for reserved in ["bio_mechanisms", "accountability"] {
        if let Some(entry) = roster.get(reserved) {
            assert!(
                !entry.is_boolean(),
                "保留子对象 {reserved} 被同名能力顶成布尔（形状破了）: {entry}"
            );
        }
    }

    // ⑥ 确定性：同一快照渲染两次逐字节一致。
    let again = render_snapshot(&snapshot);
    assert_eq!(
        value, again,
        "同一快照两次渲染不一致（自述面不得每次说话都不一样）"
    );
});
