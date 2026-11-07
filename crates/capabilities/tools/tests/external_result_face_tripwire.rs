//! `ext.*` 族（外部工具适配族）结果面凭据形状绊线 —— 批次 2c 接线回归测试。
//!
//! 覆盖面: 本族 10 个外部工具适配器的结果面（**证据信封**）接上凭据形状绊线。
//!
//! 一次接线（本族**唯一**的结果面出口）覆盖四个字符串面：
//!   · `details` 的**递归逐字符串叶子**（键名 / 数字 / 布尔 / null / 结构不动）；
//!   · 信封**顶层字符串字段**（`claim` / `tool` / `tool_version` / `target` /
//!     `scope` / `observed_at`）；
//!   · `evidence[]` 的字符串项（`kind` / `source` / `digest` / `note`）；
//!   · `limitations[]`。
//!
//! 层序沿用批次 1 裁决: **形状层 → 字段值层**。
//!
//! 验收条目 (逐条对应):
//!   ① `details` 里的密钥形状串 ⇒ 结果无明文 + 带遮形标签
//!   ② **JSON 结构完好**: 键名 / 键序 / 数字 / 布尔 / null / 数组长度 / 嵌套层级全不变
//!   ③ 遮形后的信封**仍是合法 JSON 信封** (可反序列化回 `Evidence`)
//!   ④ **原样回显面**（大块原文, 长度接近真实量级）里的密钥形状串被遮, 且**其余
//!     内容仍可读**（不是整块被清空）
//!   ⑤ **顶层字符串字段**（含 `evidence[]` / `limitations[]`）里的密钥形状串也被遮
//!   ⑥ **干净夹具逐字节全等**（零命中 ⇒ 与接线前的序列化结果完全相同, 且不挂侧车）
//!   ⑦ 命中**可观测**（`credential_tripwire` 侧车存在且 `triggered: true`）
//!   ⑧ 接线回归守卫: 本族只有**一个**结果面出口, 且它过本族的遮形助手
//!
//! 夹具全是**固定内容**（写死的证据信封 + 写死的原文块）⇒ 不赌概率。
//! 凭据串一律是**合成**的固定串, 不是任何真实凭据。
//!
//! ⚠️ 本文件**不依赖任何外部工具二进制**（本机未必装）⇒ 只测族型的遮形与挂载,
//! 不去跑真工具。

use std::fs;
use std::path::Path;

use apeireth_tools_canonical::external::tripwire::{
    redact_credential_faces, sanitized_evidence_value,
};
use apeireth_tools_canonical::recon::{Evidence, EvidenceItem};
use serde_json::{json, Value};

/// 形状族夹具: 一枚**合成**的裸密钥串（非真实凭据; 前缀后 ≥ 20 字符 ⇒ 命中形状族）。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnopqrstuv";

/// 形状层夹具: 形状层的遮形标签（内核既有标签，批次 1 同款）。
const SHAPE_LABEL: &str = "[REDACTED_OPENAI_KEY]";

/// 干净夹具: 一段普通可打印串（**不得**被遮 —— 证明只换命中的叶子）。
const CLEAN_TEXT: &str = "harmless printable text";

/// 原样回显面的**真实量级**: 规则扫描族把工具 stdout 的**前 8192 字**整段塞进
/// `details` 的一枚字符串叶子 ⇒ 夹具按同一量级造（字符数一致）。
const RAW_ECHO_CHARS: usize = 8192;

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
        "ext.demo",
        "external",
        "/workspace/sample.bin",
        "workspace",
        "one rule matched, 3 metadata field(s), no credential shape",
    )
    .with_details(json!({
        "count": 3,
        "ratio": 0.5,
        "magic": null,
        "entries": [{"offset": 0, "description": CLEAN_TEXT}],
        "embedded": [],
    }))
    .with_evidence(EvidenceItem::new("file", "sample.bin").with_digest("sha256:00"))
    .with_limitation("delegated to an external binary; output is text-parsed")
}

/// 原样回显面夹具: 一块**真实量级**（8192 字）的原文，凭据形状串埋在**中段**。
///
/// ⚠️ 块内除凭据串外全是普通可打印文本 ⇒ 「其余内容仍可读」这条断言才有意义
/// （不是整块被清空）。
fn raw_echo_blob() -> String {
    let head = "RULE-SCAN-HEAD\n";
    let middle = "rule matched at 0x00001234 ";
    let tail = "\nRULE-SCAN-TAIL\n";
    let filler = "rule line matched at 0x0000dead with no credential shape\n";
    // 全部是 ASCII ⇒ 字节数 == 字符数 ⇒ 按字节切分安全。
    let fixed = head.len() + middle.len() + FAKE_KEY.len() + 1 + tail.len();
    let room = RAW_ECHO_CHARS - fixed;
    let repeats = room / filler.len();
    let mut blob = String::with_capacity(RAW_ECHO_CHARS);
    blob.push_str(head);
    blob.push_str(middle);
    blob.push_str(FAKE_KEY);
    blob.push('\n');
    for _ in 0..repeats {
        blob.push_str(filler);
    }
    // 余数用填充行的一段补齐 ⇒ 块长**精确**等于真实量级。
    blob.push_str(&filler[..room - repeats * filler.len()]);
    blob.push_str(tail);
    blob
}

/// 原样回显面的信封夹具: `details` 里那一枚大叶子就是工具 stdout 的整段原文。
fn raw_echo_evidence() -> Evidence {
    Evidence::new(
        "ext.demo",
        "external",
        "/workspace/sample.bin",
        "workspace",
        "1 rule(s) matched in /workspace/sample.bin",
    )
    .with_details(json!({
        "matches": ["rule_demo"],
        "raw": raw_echo_blob(),
    }))
}

// ── ① `details` 里的密钥形状串: 无明文 + 带遮形标签 ──────────────────────────

#[test]
fn a_key_shape_inside_details_leaves_no_plaintext_and_carries_the_shape_label() {
    let evidence = Evidence::new(
        "ext.demo",
        "external",
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
        "ext.demo",
        "external",
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
        "ext.demo",
        "external",
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

// ── ④ 原样回显面: 凭据形状串被遮, 其余内容仍可读 ────────────────────────────

#[test]
fn the_raw_echo_face_is_masked_and_the_rest_stays_readable() {
    let blob = raw_echo_blob();

    // 夹具前提: 原文里**确实**带着凭据形状串（否则下面的断言是空转）。
    assert!(
        blob.contains(FAKE_KEY),
        "夹具前提: 原样回显面必须真的带着凭据形状串"
    );
    assert_eq!(
        blob.chars().count(),
        RAW_ECHO_CHARS,
        "夹具前提: 原文块必须与真实量级一致"
    );

    let evidence = raw_echo_evidence();
    let value = sanitized_evidence_value(&evidence);
    let masked = value["details"]["raw"]
        .as_str()
        .expect("原样回显面仍是一枚字符串叶子")
        .to_string();

    // ① 凭据形状串无明文。
    assert!(!masked.contains(FAKE_KEY), "原样回显面不得留明文密钥");
    // ② 带遮形标签。
    assert!(
        masked.contains(SHAPE_LABEL),
        "原样回显面必须带形状层遮形标签"
    );
    // ③ 其余内容仍可读（头尾标记与中段上下文都还在 ⇒ 不是整块被清空）。
    assert!(
        masked.contains("RULE-SCAN-HEAD"),
        "原样回显面的开头必须仍可读"
    );
    assert!(
        masked.contains("RULE-SCAN-TAIL"),
        "原样回显面的结尾必须仍可读"
    );
    assert!(
        masked.contains("rule matched at 0x00001234"),
        "凭据串所在那一行的上下文必须仍可读"
    );
    // ④ 整块没被清空（长度只因遮形标签而略缩）。
    let shrunk = blob.chars().count() - masked.chars().count();
    assert!(shrunk < 64, "原样回显面只应因遮形而略缩, 实缩 {shrunk} 字");
    assert!(
        masked.chars().count() > RAW_ECHO_CHARS / 2,
        "原样回显面不得被整块清空"
    );
    // ⑤ 其余叶子逐字节不变（`matches` 没被误伤）。
    assert_eq!(value["details"]["matches"], json!(["rule_demo"]));
}

// ── ⑤ 顶层字符串字段 ─────────────────────────────────────────────────────────

#[test]
fn top_level_string_faces_are_masked_too() {
    let mut evidence = Evidence::new(
        "ext.demo",
        "external",
        "/workspace/sample.bin",
        "workspace",
        "fixture claim",
    )
    .with_evidence(
        EvidenceItem::new("file", FAKE_KEY)
            .with_digest(FAKE_KEY)
            .with_note(format!("note carries {FAKE_KEY}")),
    )
    .with_limitation(format!("limitation carries {FAKE_KEY}"));

    // 六个顶层字符串字段逐个埋串。
    evidence.claim = format!("claim carries {FAKE_KEY}");
    evidence.tool = FAKE_KEY.to_string();
    evidence.tool_version = FAKE_KEY.to_string();
    evidence.target = FAKE_KEY.to_string();
    evidence.scope = FAKE_KEY.to_string();
    evidence.observed_at = FAKE_KEY.to_string();

    let value = sanitized_evidence_value(&evidence);
    let rendered = value.to_string();

    assert!(
        !rendered.contains(FAKE_KEY),
        "顶层字符串面不得留明文: {rendered}"
    );
    assert_eq!(
        value["claim"],
        json!(format!("claim carries {SHAPE_LABEL}"))
    );
    assert_eq!(value["tool"], json!(SHAPE_LABEL));
    assert_eq!(value["tool_version"], json!(SHAPE_LABEL));
    assert_eq!(value["target"], json!(SHAPE_LABEL));
    assert_eq!(value["scope"], json!(SHAPE_LABEL));
    assert_eq!(value["observed_at"], json!(SHAPE_LABEL));
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

// ── ⑦ 命中可观测 ─────────────────────────────────────────────────────────────

#[test]
fn a_hit_is_observable_through_the_tripwire_sidecar() {
    let evidence = Evidence::new(
        "ext.demo",
        "external",
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
    // 侧车挂在**信封同级键**（不进 `details`）。
    assert!(
        value["details"].get("credential_tripwire").is_none(),
        "侧车不得进 details: {value}"
    );
}

// ── ⑧ 接线回归守卫: 本族只有**一个**结果面出口 ─────────────────────────────────

/// 本族 10 个适配器必须都经**同一个**结果面出口（凭据绊线只接在那儿）⇒ 将来新增
/// 适配器若绕过出口自己造结果，绊线就会漏 ⇒ 这里把「唯一出口」钉死。
#[test]
fn the_family_has_exactly_one_result_face_exit() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/external.rs");
    let text = fs::read_to_string(&source).expect("读得到族文件");

    assert_eq!(
        text.matches("ToolResult::ok(").count(),
        1,
        "本族必须只有**一个**结果面出口（绊线接在那儿）"
    );
    assert!(
        text.contains("tripwire::sanitized_evidence_value"),
        "唯一出口必须过本族的遮形助手"
    );
    assert_eq!(
        text.matches("ok_evidence(call,").count(),
        10,
        "本族 10 个适配器都必须经同一个结果面出口"
    );
}

/// 本族的接线实现落在**族子模块**里（族文件在尺寸棘轮基线上 ⇒ 实现落子模块）⇒
/// 且它必须走**共享助手**（形状层文本面 + JSON 叶子面）⇒ 守卫钉住这两点。
#[test]
fn the_family_tripwire_module_runs_on_the_shared_helpers() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/external/tripwire.rs");
    let text = fs::read_to_string(&source).expect("读得到族子模块");

    assert!(
        text.contains("scan_json_string_leaves"),
        "族子模块必须走共享的 JSON 叶子扫描助手"
    );
    assert!(
        text.contains("scan_result_face"),
        "族子模块必须走共享的文本面扫描助手"
    );
}
