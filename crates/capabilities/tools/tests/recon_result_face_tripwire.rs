//! `re.*` 族（recon 族）结果面凭据形状绊线 —— 批次 2 接线回归测试。
//!
//! 覆盖面: 本族 9 个逆向工程工具的结果面（证据信封）接上凭据形状绊线。
//!
//! 一次接线（[`recon_tool`](apeireth_tools_canonical::recon_tool) 里**唯一**的结果面
//! 出口）覆盖四个字符串面：
//!   · `details` 的**递归逐字符串叶子**（键名 / 数字 / 布尔 / null / 结构不动）；
//!   · 信封**顶层字符串字段**（`claim` / `target` / `scope` / `observed_at` …）；
//!   · `evidence[]` 的字符串项（`kind` / `source` / `digest` / `note`）；
//!   · `limitations[]`。
//!
//! 层序沿用批次 1 裁决: **形状层 → 字段值层**。
//!
//! 验收条目 (逐条对应):
//!   ① `details` 里的密钥形状串 ⇒ 结果无明文 + 带遮形标签
//!   ② **JSON 结构完好**: 键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级全不变
//!   ③ 遮形后的信封**仍是合法 JSON 信封** (可反序列化回 `Evidence` 且 `PartialEq` 成立)
//!   ④ `re.pack` 的 `details.strings`（二进制可打印字符串全量提取）里的密钥样式串被遮
//!   ⑤ **顶层字符串字段**（`claim` / `target` / `note` / `limitations[]`）里的密钥样式串也被遮
//!   ⑥ **干净夹具逐字节全等**（零命中 ⇒ 与接线前的序列化结果完全相同, 且不挂侧车）
//!   ⑦ 命中**可观测**（`credential_tripwire` 侧车存在且 `triggered: true`）
//!
//! 夹具全是**固定内容**（临时目录里写死的字节序列 / 写死的证据信封）⇒ 不赌概率。
//! 凭据串一律是**合成**的固定串，不是任何真实凭据。

use std::fs;
use std::path::Path;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::recon::{
    redact_credential_faces, sanitized_evidence_value, Evidence, EvidenceItem,
};
use apeireth_tools_canonical::recon_tool::ReconPackTool;
use serde_json::{json, Value};
use tempfile::tempdir;

/// 形状族夹具: 一枚**合成**的裸密钥串（非真实凭据; 长度 ≥ 20 ⇒ 命中形状族）。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

/// 形状族夹具: 形状层的遮形标签（内核既有标签，批次 1 同款）。
const SHAPE_LABEL: &str = "[REDACTED_OPENAI_KEY]";

/// 干净夹具: 一段普通可打印串（**不得**被遮 —— 证明只换命中的叶子）。
const CLEAN_TEXT: &str = "harmless printable text";

/// 结构比对: 键名 / 键序 / 数字 / 布尔 / null / 数组长度 / 嵌套层级必须完全一致；
/// **只有字符串叶子的内容允许不同**。
fn assert_same_structure(before: &Value, after: &Value, path: &str) {
    match (before, after) {
        (Value::Object(before_map), Value::Object(after_map)) => {
            let before_keys: Vec<&String> = before_map.keys().collect();
            let after_keys: Vec<&String> = after_map.keys().collect();
            assert_eq!(before_keys, after_keys, "键名/键序必须不变 @{path}");
            for (key, before_value) in before_map {
                let after_value = after_map
                    .get(key)
                    .unwrap_or_else(|| panic!("键 {key} 必须还在 @{path}"));
                assert_same_structure(before_value, after_value, &format!("{path}.{key}"));
            }
        }
        (Value::Array(before_items), Value::Array(after_items)) => {
            assert_eq!(
                before_items.len(),
                after_items.len(),
                "数组长度必须不变 @{path}"
            );
            for (index, (before_item, after_item)) in
                before_items.iter().zip(after_items.iter()).enumerate()
            {
                assert_same_structure(before_item, after_item, &format!("{path}[{index}]"));
            }
        }
        // 字符串叶子: 内容允许变（这正是遮形要动的那一层）。
        (Value::String(_), Value::String(_)) => {}
        (before_value, after_value) => assert_eq!(
            before_value, after_value,
            "非字符串叶子必须逐字节不变 @{path}"
        ),
    }
}

/// 带密钥叶子的 `details` 夹具: 数字 / 布尔 / null / 数组 / 嵌套全都有。
fn fixture_details() -> Value {
    json!({
        "headline": "plain text",
        "count": 42,
        "ratio": 1.5,
        "flag": true,
        "nothing": null,
        "nested": {
            "deep": {
                "secret_slot": FAKE_KEY,
                "n": 7,
                "flag": false,
            },
            "list": [1, 2, 3],
        },
        "items": [
            {"name": "alpha", "weight": 1},
            {"name": "beta", "weight": 2},
        ],
    })
}

/// 干净信封夹具: 全是**不命中**的内容（含数字 / 布尔 / null / 嵌套）。
fn clean_evidence() -> Evidence {
    Evidence::new(
        "re.pack",
        "0.1.0",
        "/workspace/sample.bin",
        "workspace",
        "size 3 bytes, entropy 0.00 bit/byte, 1 strings, magic none, packed hint none",
    )
    .with_details(json!({
        "size": 3,
        "entropy_bits": 0.0,
        "magic": null,
        "strings": [{"offset": 0, "text": CLEAN_TEXT, "encoding": "ascii"}],
        "embedded": [],
    }))
    .with_evidence(EvidenceItem::new("file", "sample.bin").with_digest("sha256:00"))
    .with_limitation("static inspection only; the target is never executed")
}

/// 真二进制夹具: 魔数 + 非可打印字节 + 可打印串 + **密钥样式串**。
///
/// ⚠️ 必须是**真二进制**（不是文本文件）⇒ 字符串提取器才会把它当「二进制里的
/// 可打印串」整段提取出来（这正是 `re.pack` 的 `details.strings` 语义）。
fn binary_fixture(include_key: bool) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&[0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00]);
    bytes.extend_from_slice(&[0x00, 0x90, 0xff, 0xfe, 0x01]);
    bytes.extend_from_slice(CLEAN_TEXT.as_bytes());
    bytes.push(0x00);
    if include_key {
        bytes.extend_from_slice(FAKE_KEY.as_bytes());
        bytes.push(0x00);
    }
    bytes.extend_from_slice(&[0xff, 0xfe, 0x00]);
    bytes.extend_from_slice(b"tail bytes");
    bytes
}

async fn invoke_pack(root: &Path, path: &str) -> Value {
    let tool = ReconPackTool::new(root.to_path_buf());
    let call = ToolCall {
        id: "call_pack".to_string(),
        name: "re.pack".to_string(),
        arguments: json!({ "path": path }),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "re.pack 失败: {}", result.render());
    let ToolOutcome::Ok { value } = result.outcome else {
        panic!("re.pack 必须成功: {result:?}");
    };
    value
}

// ── ① `details` 里的密钥形状串: 无明文 + 带遮形标签 ──────────────────────────

#[test]
fn a_key_shape_inside_details_leaves_no_plaintext_and_carries_the_shape_label() {
    let evidence = Evidence::new(
        "re.pack",
        "0.1.0",
        "/workspace/sample.bin",
        "workspace",
        "fixture claim",
    )
    .with_details(fixture_details());

    let value = sanitized_evidence_value(&evidence);
    let rendered = value.to_string();

    assert!(
        !rendered.contains(FAKE_KEY),
        "明文密钥不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains(SHAPE_LABEL),
        "必须带形状层遮形标签: {rendered}"
    );
    assert_eq!(
        value["details"]["nested"]["deep"]["secret_slot"],
        json!(SHAPE_LABEL),
        "被遮的叶子必须换成遮形标签: {rendered}"
    );
}

// ── ② JSON 结构完好 ──────────────────────────────────────────────────────────

#[test]
fn masking_keeps_the_json_structure_intact() {
    let evidence = Evidence::new(
        "re.pack",
        "0.1.0",
        "/workspace/sample.bin",
        "workspace",
        "fixture claim",
    )
    .with_details(fixture_details());

    let value = sanitized_evidence_value(&evidence);
    let before = fixture_details();
    let after = &value["details"];

    assert_same_structure(&before, after, "details");

    // 键名不变。
    for key in [
        "headline", "count", "ratio", "flag", "nothing", "nested", "items",
    ] {
        assert!(after.get(key).is_some(), "键 {key} 必须还在");
    }
    // 数字 / 布尔 / null 不变。
    assert_eq!(after["count"], json!(42));
    assert_eq!(after["ratio"], json!(1.5));
    assert_eq!(after["flag"], json!(true));
    assert!(after["nothing"].is_null());
    assert_eq!(after["nested"]["deep"]["n"], json!(7));
    assert_eq!(after["nested"]["deep"]["flag"], json!(false));
    // 数组长度不变。
    assert_eq!(after["nested"]["list"].as_array().map(Vec::len), Some(3));
    assert_eq!(after["items"].as_array().map(Vec::len), Some(2));
    // 嵌套层级不变（叶子深度不变）。
    assert!(after["nested"]["deep"]["secret_slot"].is_string());
    // 未命中的叶子逐字节不变。
    assert_eq!(after["headline"], json!("plain text"));
    assert_eq!(after["items"][0]["name"], json!("alpha"));
}

// ── ③ 遮形后的信封仍是合法 JSON 信封 ────────────────────────────────────────

#[test]
fn the_masked_envelope_is_still_a_valid_evidence_json() {
    let evidence = Evidence::new(
        "re.pack",
        "0.1.0",
        "/workspace/sample.bin",
        "workspace",
        "fixture claim",
    )
    .with_details(fixture_details());

    let (masked, kinds) = redact_credential_faces(&evidence);
    let rendered = serde_json::to_string(&masked).expect("序列化");
    let back: Evidence = serde_json::from_str(&rendered).expect("遮形后仍是合法证据信封");

    assert_eq!(back, masked, "信封往返必须成立");
    assert!(!kinds.is_empty(), "命中必须报族");

    // PartialEq: 除**被遮形的叶子**外, 遮形后的信封与原信封完全相等。
    let mut expected = evidence.clone();
    expected.details["nested"]["deep"]["secret_slot"] = json!(SHAPE_LABEL);
    assert_eq!(masked, expected, "除被遮形的叶子外必须与原信封相等");
}

// ── ④ `re.pack` 的 `details.strings` ─────────────────────────────────────────

#[tokio::test]
async fn re_pack_masks_a_key_shaped_string_in_the_extracted_strings() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("fixture.bin"), binary_fixture(true)).unwrap();

    let value = invoke_pack(dir.path(), "fixture.bin").await;
    let rendered = value.to_string();

    assert!(
        !rendered.contains(FAKE_KEY),
        "二进制里硬编码的密钥不得原样回显: {rendered}"
    );
    assert!(
        rendered.contains(SHAPE_LABEL),
        "必须带形状层遮形标签: {rendered}"
    );

    // 提取到的字符串: 密钥那一段被遮, 其余两段逐字节不变, 偏移不变。
    let strings = value["details"]["strings"].as_array().expect("字符串数组");
    assert_eq!(strings.len(), 3, "字符串条数不变: {rendered}");
    assert_eq!(strings[0]["text"], json!(CLEAN_TEXT));
    assert_eq!(strings[1]["text"], json!(SHAPE_LABEL));
    assert_eq!(strings[2]["text"], json!("tail bytes"));
    assert_eq!(strings[0]["offset"], json!(13));
    assert_eq!(strings[1]["offset"], json!(37));
    assert_eq!(strings[2]["offset"], json!(70));
}

// ── ⑤ 顶层字符串字段 ─────────────────────────────────────────────────────────

#[test]
fn top_level_string_faces_are_masked_too() {
    let evidence = Evidence::new(
        "re.pack",
        "0.1.0",
        FAKE_KEY,
        "workspace",
        format!("claim carries {FAKE_KEY}"),
    )
    .with_evidence(
        EvidenceItem::new("file", FAKE_KEY)
            .with_digest(FAKE_KEY)
            .with_note(format!("note carries {FAKE_KEY}")),
    )
    .with_limitation(format!("limitation carries {FAKE_KEY}"));

    let value = sanitized_evidence_value(&evidence);
    let rendered = value.to_string();

    assert!(
        !rendered.contains(FAKE_KEY),
        "顶层字符串面不得留明文: {rendered}"
    );
    assert_eq!(value["target"], json!(SHAPE_LABEL));
    assert_eq!(
        value["claim"],
        json!(format!("claim carries {SHAPE_LABEL}"))
    );
    assert_eq!(value["evidence"][0]["source"], json!(SHAPE_LABEL));
    assert_eq!(value["evidence"][0]["digest"], json!(SHAPE_LABEL));
    assert_eq!(
        value["evidence"][0]["note"],
        json!(format!("note carries {SHAPE_LABEL}"))
    );
    assert_eq!(
        value["limitations"][0],
        json!(format!("limitation carries {SHAPE_LABEL}"))
    );
}

// ── ⑥ 干净夹具逐字节全等 ─────────────────────────────────────────────────────

#[test]
fn a_clean_envelope_is_byte_identical_and_carries_no_sidecar() {
    let evidence = clean_evidence();

    let value = sanitized_evidence_value(&evidence);
    let plain = serde_json::to_value(&evidence).expect("序列化");

    assert_eq!(
        value.to_string(),
        plain.to_string(),
        "零命中必须与接线前逐字节全等"
    );
    assert!(
        value.get("credential_tripwire").is_none(),
        "零命中不得挂侧车: {value}"
    );
}

#[tokio::test]
async fn a_clean_re_pack_result_carries_no_sidecar_and_no_visible_change() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("clean.bin"), binary_fixture(false)).unwrap();

    let value = invoke_pack(dir.path(), "clean.bin").await;
    let rendered = value.to_string();

    assert!(
        !rendered.contains("credential_tripwire"),
        "零命中不得挂侧车: {rendered}"
    );
    // 信封的键集合与接线前一致（`evidence` / `limitations` 非空 ⇒ 两键都在）。
    let mut keys: Vec<String> = value.as_object().expect("对象").keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "claim",
            "details",
            "evidence",
            "limitations",
            "observed_at",
            "scope",
            "target",
            "tool",
            "tool_version",
        ],
        "零命中时信封的形状与字节都不变"
    );
    // 提取到的字符串逐字节不变。
    let strings = value["details"]["strings"].as_array().expect("字符串数组");
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[0]["text"], json!(CLEAN_TEXT));
    assert_eq!(strings[1]["text"], json!("tail bytes"));
}

// ── ⑦ 命中可观测 ─────────────────────────────────────────────────────────────

#[test]
fn a_hit_is_observable_through_the_tripwire_sidecar() {
    let evidence = Evidence::new(
        "re.pack",
        "0.1.0",
        "/workspace/sample.bin",
        "workspace",
        "fixture claim",
    )
    .with_details(fixture_details());

    let value = sanitized_evidence_value(&evidence);
    let sidecar = value
        .get("credential_tripwire")
        .expect("命中必须挂可观测侧车");

    assert_eq!(sidecar["triggered"], json!(true));
    let kinds = sidecar["leaked_kinds"].as_array().expect("命中族清单");
    assert!(
        kinds.iter().any(|kind| kind == "OpenAiKey"),
        "侧车必须报出形状族: {kinds:?}"
    );
}

// ── 接线回归守卫: 本族只有**一个**结果面出口 ─────────────────────────────────

/// 本族 9 个工具必须都经**同一个**结果面出口（凭据绊线只接在那儿）⇒ 将来新增
/// 工具若绕过出口自己造结果，绊线就会漏 ⇒ 这里把「唯一出口」钉死。
#[test]
fn the_family_has_exactly_one_result_face_exit() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/recon_tool.rs");
    let text = fs::read_to_string(&source).expect("读得到 recon_tool.rs");

    assert_eq!(
        text.matches("ToolResult::ok(").count(),
        1,
        "本族必须只有**一个**结果面出口（绊线接在那儿）"
    );
    assert!(
        text.contains("sanitized_evidence_value"),
        "唯一出口必须过本族的遮形助手"
    );
}
