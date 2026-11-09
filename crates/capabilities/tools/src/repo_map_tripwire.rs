//! F7 `repo_map` 结果面凭据形状绊线（M1）的**验收测试**。
//!
//! # 为什么单独一个文件
//!
//! 接线落在族文件 `repo_map_tool.rs`（族文件里才有族名 ⇒ 接线守卫的族枚举
//! 认得出本族）；本文件只放**验收判据**，与族文件的实现分开 ⇒ ⇒ 实现与
//! 判据各自可读。
//!
//! ⚠️ **本文件不得写出内核扫描助手的名字面量** —— 接线守卫按**文件所在族**
//! 枚举接线方 ⇒ ⇒ 本文件一旦写出那些字面量，守卫的族枚举会多出一族
//! `repo_map_tripwire` ⇒ ⇒ 守卫红、且报错看不出真因（同 `evidence_faces`
//! 那条显式断言的教训）⇒ ⇒ 本文件只用**族文件的公开面**做断言。
//!
//! # 本族的面区分（**实测**，2026-11-09）
//!
//! `repo_map` 回显的是**紧凑代码大纲**：生成器只保留**符号名 + 行号 + 路径**，
//! 函数体一律省略成 `...` ⇒ ⇒ **源码正文（函数体里的字面量）根本不进结果面**
//! ⇒ ⇒ 本族**没有**「原文片段面」可遮 ⇒ ⇒ 真正可能带凭据形状的仓库派生
//! 文本只有两处：**路径行**（`## <路径>`）与**符号名**。
//!
//! ⇒ ⇒ 形状层（只认凭据形状）照扫即可 ⇒ ⇒ 普通标识符 / 普通文件名不会被遮
//! ⇒ ⇒ 本文件的核心判据是「**只有凭据形状的那几段变了，其余逐字不变**」。
//!
//! # 判据来源
//!
//! * **接线前真值** = 直调口 [`RepoMapTool::generate`]（库 API，**未接绊线**）
//!   ⇒ ⇒ 工具面与它逐字节相同 = 「零命中 ⇒ 逐字节全等」。
//! * **明文边界** = 合成串（**非真实凭据**），形状合法（`sk-` + ≥20 字符 /
//!   `ghp_` + ≥36 字符）⇒ ⇒ 命中是真的，不赌概率。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use serde_json::{json, Value};

use super::{RepoMapConfig, RepoMapTool, REPO_MAP_DEFAULT_TOKEN_BUDGET};

/// 合成凭据 · **文件名**（形状合法：`sk-` + ≥20 字符；**非真实凭据**）。
///
/// ⚠️ 本族结果面**不回显源码正文** ⇒ ⇒ 凭据形状要进结果面，只能落在
/// **路径**或**符号名**上 ⇒ ⇒ 夹具因此把凭据放在**文件名**里。
const FAKE_KEY_FILE: &str = "sk-repo-map-fixture-abcdefghijklmnop.rs";

/// 合成凭据 · **符号名**（形状合法：`ghp_` + ≥36 字符；**非真实凭据**）。
const FAKE_PAT_SYMBOL: &str = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";

/// 合成凭据 · **源码正文**（形状合法：`sk-` + ≥20 字符；**非真实凭据**）。
const FAKE_KEY_BODY: &str = "sk-repo-map-fixture-abcdefghijklmnop";

/// 源码正文里的**普通**标记串（用来钉住「正文不进结果面」这一现状）。
const BODY_MARKER: &str = "body_marker_8f21c4";

/// 形状层的两个遮形标签（内核既有口径，测试按字面比对）。
const LABEL_OPENAI: &str = "[REDACTED_OPENAI_KEY]";
const LABEL_GITHUB: &str = "[REDACTED_GITHUB_PAT]";

/// 建一个**固定夹具**的仓库（确定性：文件集合与内容都是写死的）。
///
/// 四个文件各占一条通道：
///
/// | 文件 | 通道 | 期望 |
/// |---|---|---|
/// | `src/clean.rs` | 普通摘要面 | 逐字不变 |
/// | `src/<凭据形状名>.rs` | **路径行** | 遮形 |
/// | `src/keystore.rs` | **符号名** | 遮形（且同行其余部分存活） |
/// | `src/plain.rs` | **源码正文** | 不进结果面（现状） |
fn fixture_repo(root: &Path) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("建目录");
    fs::write(
        src.join("clean.rs"),
        "pub fn clean_symbol() {}\npub fn second_symbol() {}\n",
    )
    .expect("写普通文件");
    fs::write(
        src.join(FAKE_KEY_FILE),
        "pub fn credential_shaped_file_name() {}\n",
    )
    .expect("写凭据形状文件名的文件");
    fs::write(
        src.join("keystore.rs"),
        format!("pub fn {FAKE_PAT_SYMBOL}() {{}}\n"),
    )
    .expect("写凭据形状符号名的文件");
    fs::write(
        src.join("plain.rs"),
        format!(
            "pub fn read_api_key() -> &'static str {{\n    \"{FAKE_KEY_BODY}\"\n}}\n\
             const _MARKER: &str = \"{BODY_MARKER}\";\n"
        ),
    )
    .expect("写正文含凭据的文件");
}

/// 调一次工具面（**走 `ToolCapability::invoke`**，即生产路径）。
fn invoke(tool: &RepoMapTool, path: &str) -> ToolResult {
    let call = ToolCall {
        id: "call-repo-map-tripwire".to_string(),
        name: "repo_map".to_string(),
        arguments: json!({ "path": path }),
    };
    futures::executor::block_on(tool.invoke(&call))
}

/// 取成功帧的值（失败即帧 ⇒ 真因如实）。
fn ok_value(result: &ToolResult) -> Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("工具必须成功, 不得是错误帧: {message}"),
    }
}

/// 取失败帧的文案（成功即帧 ⇒ 真因如实）。
fn error_message(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("必须是错误帧: {value}"),
    }
}

/// 工具面的**大纲文本叶子**。
fn map_text(value: &Value) -> &str {
    value
        .get("map")
        .and_then(Value::as_str)
        .expect("结果面必须有 map 文本叶子")
}

// ── 判据 ④：零命中 ⇒ 与接线前**逐字节全等** ──────────────────────────────────

/// **零命中 ⇒ 输出与接线前完全相同**（含「不挂侧车」）。
///
/// 接线前的真值 = 直调口 `generate()`（库 API 未接绊线）⇒ ⇒ 逐字节比对。
#[test]
fn a_clean_repo_yields_a_result_face_byte_identical_to_the_unwired_outcome() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("src")).expect("建目录");
    fs::write(root.join("src/clean.rs"), "pub fn plain_symbol() {}\n").expect("写夹具");

    let tool = RepoMapTool::new(RepoMapConfig::new(root));
    let unwired = tool
        .generate(".", &[], REPO_MAP_DEFAULT_TOKEN_BUDGET)
        .expect("直调口");
    let unwired_value = serde_json::to_value(&unwired).expect("序列化");

    let value = ok_value(&invoke(&tool, "."));

    assert_eq!(
        serde_json::to_string(&value).expect("序列化"),
        serde_json::to_string(&unwired_value).expect("序列化"),
        "零命中 ⇒ 工具面必须与接线前逐字节相同"
    );
    assert!(
        value.get("credential_tripwire").is_none(),
        "零命中不得挂侧车: {value}"
    );
}

// ── 判据 ③：摘要面逐字不变（**本族关键判据**） ───────────────────────────────

/// **只有凭据形状的那几段变了，其余逐字不变**。
///
/// 这是本族最关键的一条：证明形状层**只遮凭据形状**、没有把可读性毁掉
/// ⇒ ⇒ 文件名 / 行号 / 符号名 / 大纲语法**逐字存活**。
#[test]
fn only_the_credential_spans_change_the_outline_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fixture_repo(root);

    let tool = RepoMapTool::new(RepoMapConfig::new(root));
    let unwired_map = tool
        .generate(".", &[], REPO_MAP_DEFAULT_TOKEN_BUDGET)
        .expect("直调口")
        .map;

    let value = ok_value(&invoke(&tool, "."));
    let wired_map = map_text(&value);

    let expected = unwired_map
        .replace(FAKE_KEY_FILE, LABEL_OPENAI)
        .replace(FAKE_PAT_SYMBOL, LABEL_GITHUB);
    assert_eq!(
        wired_map, expected,
        "只有凭据形状的那几段应变; 其余必须逐字不变"
    );

    // 摘要面的**普通文本**逐字存活（文件名 / 行号 / 符号名 / 大纲语法）。
    for needle in [
        "clean.rs",
        "L1: [fn] clean_symbol ...",
        "L2: [fn] second_symbol ...",
        "# Repository Map (Ranked Code Signatures)",
        "## ",
    ] {
        assert!(wired_map.contains(needle), "摘要面被改了: {wired_map}");
    }
    // 符号名那一行：**只换掉凭据形状本身**，同行其余部分（行号/类型/省略号）存活。
    assert!(
        wired_map.contains(&format!("L1: [fn] {LABEL_GITHUB} ...")),
        "符号名那一行被整行抹掉了: {wired_map}"
    );
}

// ── 判据 ① + ⑤：命中 ⇒ 无明文 + 可观测 ──────────────────────────────────────

/// **命中 ⇒ 结果面无明文 + 侧车可观测**（`triggered: true` + 族名）。
#[test]
fn a_credential_shape_is_masked_and_the_hit_is_observable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fixture_repo(root);

    let tool = RepoMapTool::new(RepoMapConfig::new(root));
    let value = ok_value(&invoke(&tool, "."));
    let wired_map = map_text(&value);

    // ① 无明文：路径行、符号名、源码正文三路都不得留明文。
    assert!(
        !wired_map.contains(FAKE_KEY_FILE),
        "凭据明文留在结果面: {wired_map}"
    );
    assert!(
        !wired_map.contains(FAKE_PAT_SYMBOL),
        "凭据明文留在结果面: {wired_map}"
    );
    assert!(
        !wired_map.contains(FAKE_KEY_BODY),
        "源码正文里的明文不得出现在结果面: {wired_map}"
    );
    // 遮形标签到位。
    assert!(wired_map.contains(LABEL_OPENAI), "缺遮形标签: {wired_map}");
    assert!(wired_map.contains(LABEL_GITHUB), "缺遮形标签: {wired_map}");

    // ⑤ 命中可观测：侧车挂在顶栏，`triggered: true` + 族名。
    let sidecar = value.get("credential_tripwire").expect("命中必须有侧车");
    assert_eq!(
        sidecar.get("triggered").and_then(Value::as_bool),
        Some(true),
        "侧车未标记命中: {value}"
    );
    let kinds = sidecar
        .get("leaked_kinds")
        .and_then(Value::as_array)
        .expect("侧车缺 leaked_kinds");
    assert!(
        kinds.iter().any(|kind| kind == "OpenAiKey"),
        "侧车族名不符: {value}"
    );
    assert!(
        kinds.iter().any(|kind| kind == "GitHubPat"),
        "侧车族名不符: {value}"
    );
}

/// **源码正文不进结果面**（本族现状的钉子）。
///
/// ⚠️ 这条断言红的含义**不是**「绊线坏了」，而是「**暴露面分析要重做**」——
/// 本族今天的暴露面结论建立在「结果面只回显路径 + 符号名」上；一旦生成器
/// 改成回显源码正文 ⇒ ⇒ 暴露面结论作废 ⇒ ⇒ 必须重新评估（并同步改本断言）。
#[test]
fn source_bodies_do_not_reach_the_result_face() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fixture_repo(root);

    let tool = RepoMapTool::new(RepoMapConfig::new(root));
    let value = ok_value(&invoke(&tool, "."));
    let wired_map = map_text(&value);

    assert!(
        !wired_map.contains(BODY_MARKER),
        "源码正文进了结果面 ⇒ 本族的暴露面结论作废, 需重新评估: {wired_map}"
    );
}

// ── 判据 ②：结构完好（键名 / 数字 / 布尔 / null / 结构） ──────────────────────

/// **命中时结构仍完好**：键集合只多一个侧车键；数字 / 布尔 / null 一字不动。
#[test]
fn the_structured_faces_keep_their_shape_while_a_credential_is_masked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fixture_repo(root);

    let tool = RepoMapTool::new(RepoMapConfig::new(root));
    let unwired_value = serde_json::to_value(
        &tool
            .generate(".", &[], REPO_MAP_DEFAULT_TOKEN_BUDGET)
            .expect("直调口"),
    )
    .expect("序列化");
    let value = ok_value(&invoke(&tool, "."));

    // 键集合：本夹具**有命中** ⇒ ⇒ 键集合必须只多**一个**侧车键（不多不少、不改名）。
    let keys_of = |value: &Value| -> BTreeSet<String> {
        value
            .as_object()
            .expect("对象结果面")
            .keys()
            .cloned()
            .collect()
    };
    let mut expected_keys = keys_of(&unwired_value);
    expected_keys.insert("credential_tripwire".to_string());
    assert_eq!(
        keys_of(&value),
        expected_keys,
        "命中时键集合必须只多一个侧车键: {value}"
    );

    // 计数 / 布尔 / null：逐字不变。
    for key in ["scanned_files", "skipped_oversize", "skipped_over_count"] {
        assert_eq!(value[key], unwired_value[key], "{key} 被改了: {value}");
    }
    assert_eq!(value["truncated"], unwired_value["truncated"], "布尔被改了");
    assert_eq!(value["note"], unwired_value["note"], "note 被改了: {value}");
}

// ── 失败面：裸文本帧（路径回显 ⇒ 同样要扫） ──────────────────────────────────

/// **失败面也要扫**：错误文案**回显路径** ⇒ ⇒ 凭据形状的路径不得明文出境。
#[test]
fn the_error_face_masks_a_credential_shaped_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 用**规范化的绝对路径**当工作区根 ⇒ 请求路径一定落在根内（不走越界分支）。
    let root = dir.path().canonicalize().expect("规范化");
    let tool = RepoMapTool::new(RepoMapConfig::new(root.clone()));

    let dirty = root.join(FAKE_KEY_BODY);
    let message = error_message(&invoke(&tool, &dirty.display().to_string()));

    assert!(
        !message.contains(FAKE_KEY_BODY),
        "凭据明文留在失败面: {message}"
    );
    assert!(
        message.contains(LABEL_OPENAI),
        "失败面缺遮形标签: {message}"
    );
    // 命中可观测：末尾一行 JSON 注解（裸文本面挂不住结构化侧车）。
    assert!(
        message.contains("\"credential_tripwire\""),
        "失败面缺可观测标记: {message}"
    );
    assert!(
        message.contains("\"triggered\":true"),
        "失败面缺可观测标记: {message}"
    );
}

/// **失败面零命中 ⇒ 逐字节不变**（不挂任何注解）。
#[test]
fn a_clean_error_face_is_byte_identical_to_the_pre_wiring_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("规范化");
    let tool = RepoMapTool::new(RepoMapConfig::new(root.clone()));

    let missing = root.join("no-such-directory");
    let message = error_message(&invoke(&tool, &missing.display().to_string()));

    assert_eq!(
        message,
        format!("路径不可用: {} 不是目录", missing.display()),
        "零命中 ⇒ 失败面必须与接线前逐字节相同"
    );
    assert!(
        !message.contains("credential_tripwire"),
        "零命中不得挂注解: {message}"
    );
}
