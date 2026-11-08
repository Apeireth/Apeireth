//! F7 `repo_map` 结果面凭据形状绊线（M1）的回归测试。
//!
//! # 本族为什么需要
//!
//! `repo_map` 回显**紧凑代码大纲**（排名后的**符号签名**，函数体省略成 `...`）⇒ ⇒
//! 签名里的**默认值**可能带硬编码凭据 ⇒ ⇒ 它是**凭据进模型面**的路径。
//!
//! # 面区分（本族关键）
//!
//! | 面 | 字段 | 处置 |
//! |---|---|---|
//! | **结构化摘要面** | `scanned_files` / `skipped_oversize` / `skipped_over_count` / `truncated` | **一字不动** |
//! | **大纲文本面** | `map`（符号签名，含默认值） | 过形状层（M1） |
//! | **说明文本面** | `note` | 过形状层（M1） |
//!
//! ⚠️ **键名不动**：JSON 叶子扫描只换**字符串叶子**的内容 ⇒ ⇒ `map` / `note` 这两个
//! **键名本身**不会被改 ⇒ ⇒ 结构与键集合保持不变。

use std::fs;
use std::path::PathBuf;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::repo_map_tool::{RepoMapConfig, RepoMapTool};
use serde_json::json;

/// 一枚**形状合法**的密钥串（**合成**，非真实凭据）：`sk-` + ≥20 字节。
const FAKE_KEY: &str = "sk-repo-map-fixture-abcdefghijklmnop";

/// 建一个只含一个源码文件的临时仓库。
///
/// ⚠️ **夹具的凭据放在「文件名」里**（不是文件内容）—— 这是**实测出来的真实暴露面**：
/// `repo_map` 回显的是**紧凑代码大纲**（**符号签名**，函数体省略成 `...`）
/// ⇒ ⇒ **函数体里的字面量不进结果面** ⇒ ⇒ ⇒ **只有路径行会回显**（`## <路径>`）
/// ⇒ ⇒ ⇒ 凭据形状出现在**路径**里时才会进模型面 ⇒ ⇒ 这才是本族要遮的面。
fn fixture_repo(dir: &PathBuf) {
    fs::create_dir_all(dir).expect("临时目录");
    fs::write(
        dir.join(format!("{FAKE_KEY}.rs")),
        "pub fn connect(api_key: &str) -> u32 {\n    let _ = api_key;\n    0\n}\n",
    )
    .expect("写夹具");
}

fn call_for(dir: &PathBuf) -> ToolCall {
    ToolCall {
        id: "call-repo-map".to_string(),
        name: "repo_map".to_string(),
        arguments: json!({
            "path": dir.display().to_string(),
        }),
    }
}

/// 从结果帧取**成功**的值（失败即帧 ⇒ 真因如实）。
fn ok_value(result: &ToolResult) -> serde_json::Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("工具必须成功, 不得是错误帧: {message}"),
    }
}

/// 结果面里的字符串叶子（递归收集）。
fn string_leaves(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => out.push(text.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                string_leaves(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, item) in map {
                string_leaves(item, out);
            }
        }
        _ => {}
    }
}

#[test]
fn a_credential_shape_in_the_outline_is_masked_and_no_plaintext_survives() {
    let dir = std::env::temp_dir().join(format!("repo-map-tripwire-{}", std::process::id()));
    fixture_repo(&dir);
    let tool = RepoMapTool::new(RepoMapConfig::new(dir.clone()));

    let result = futures::executor::block_on(tool.invoke(&call_for(&dir)));
    let value = ok_value(&result);

    let mut leaves = Vec::new();
    string_leaves(&value, &mut leaves);
    let joined = leaves.join("\n");

    assert!(
        !joined.contains(FAKE_KEY),
        "合成凭据明文留在结果面: {joined}"
    );
    assert!(
        joined.contains("[REDACTED_OPENAI_KEY]"),
        "结果面没有形状遮形标签: {joined}"
    );
    // 附带事实：**符号签名只到名字与参数名** ⇒ ⇒ 参数名里带凭据形状也会被遮
    // （这是同一面，故一并断言它确实进了结果面再被遮）。
    assert!(
        !joined.contains(&FAKE_KEY[..12]),
        "路径里的凭据前缀都没被遮: {joined}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_structured_summary_faces_are_not_harmed_by_the_tripwire() {
    let dir = std::env::temp_dir().join(format!("repo-map-summary-{}", std::process::id()));
    fixture_repo(&dir);
    let tool = RepoMapTool::new(RepoMapConfig::new(dir.clone()));

    let result = futures::executor::block_on(tool.invoke(&call_for(&dir)));
    let value = ok_value(&result);

    // 结构化摘要面的四个字段必须**逐字**保持其类型与含义。
    assert!(
        value
            .get("scanned_files")
            .and_then(|v| v.as_u64())
            .is_some(),
        "scanned_files 不是数字: {value}"
    );
    assert!(
        value
            .get("skipped_oversize")
            .and_then(|v| v.as_u64())
            .is_some(),
        "skipped_oversize 不是数字: {value}"
    );
    assert!(
        value
            .get("skipped_over_count")
            .and_then(|v| v.as_u64())
            .is_some(),
        "skipped_over_count 不是数字: {value}"
    );
    assert!(
        value.get("truncated").and_then(|v| v.as_bool()).is_some(),
        "truncated 不是布尔: {value}"
    );
    // 键集合不得多出/少掉除侧车以外的键。
    let mut keys: Vec<String> = value
        .as_object()
        .expect("对象结果面")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    let expected = vec![
        "map",
        "note",
        "scanned_files",
        "skipped_over_count",
        "skipped_oversize",
        "truncated",
    ];
    let mut actual: Vec<String> = keys
        .iter()
        .filter(|key| key.as_str() != "credential_tripwire")
        .cloned()
        .collect();
    actual.sort();
    assert_eq!(actual, expected, "结果面键集合被改了: {value}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_clean_repo_produces_a_result_face_without_a_tripwire_sidecar() {
    let dir = std::env::temp_dir().join(format!("repo-map-clean-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("临时目录");
    fs::write(
        dir.join("clean.rs"),
        "pub fn plain(value: u32) -> u32 {\n    value + 1\n}\n",
    )
    .expect("写夹具");
    let tool = RepoMapTool::new(RepoMapConfig::new(dir.clone()));

    let result = futures::executor::block_on(tool.invoke(&call_for(&dir)));
    let value = ok_value(&result);

    assert!(
        value.get("credential_tripwire").is_none(),
        "零命中却挂了侧车: {value}"
    );
    let mut leaves = Vec::new();
    string_leaves(&value, &mut leaves);
    assert!(
        !leaves.join("\n").contains("REDACTED"),
        "零命中却有遮形痕迹: {value}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_hit_is_observable_through_the_sidecar() {
    let dir = std::env::temp_dir().join(format!("repo-map-sidecar-{}", std::process::id()));
    fixture_repo(&dir);
    let tool = RepoMapTool::new(RepoMapConfig::new(dir.clone()));

    let result = futures::executor::block_on(tool.invoke(&call_for(&dir)));
    let value = ok_value(&result);

    let sidecar = value.get("credential_tripwire").expect("命中必须有侧车");
    assert_eq!(
        sidecar.get("triggered").and_then(|v| v.as_bool()),
        Some(true),
        "侧车未标记命中: {value}"
    );
    let kinds = sidecar
        .get("leaked_kinds")
        .and_then(|v| v.as_array())
        .expect("侧车缺 leaked_kinds");
    assert!(
        kinds.iter().any(|k| k.as_str() == Some("OpenAiKey")),
        "侧车族名不符: {value}"
    );

    let _ = fs::remove_dir_all(&dir);
}
