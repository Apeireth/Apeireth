#![no_main]

//! fuzz: **search 面（本地检索工具）** —— 「字面量 / 大小写不敏感 / 确定性序 / 凭据路径跳过」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/search.rs`
//! （`SearchTool::invoke` + `search_path` / `search_file` / 命中计数与列号语义）。
//!
//! ## 为什么值得 fuzz
//!
//! - **吃不受控输入**：检索的**查询串**由模型生成（不可信）；被检索的**文件内容**
//!   来自工作区（可能完全不可信，含任意字节 / 超长行 / 二进制 / 多字节字符）。
//! - **有强承诺**（模块头自己写死的）：
//!   1. **local, literal, case-insensitive** —— 「**does not use regex**」；
//!   2. **deterministic (path, line, text) order**；
//!   3. **Known credential and key paths are skipped**（安全承诺：密钥面不进检索面）。
//!
//! ## 判据纪律（每条断言都从上面三条推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **字面量**：查询里含正则元字符时，只匹配**字面**文本；「长得像正则」的那份**不得**被匹配 | does not use regex |
//! | O2 | **大小写不敏感**：大写查询必须命中小写内容 | case-insensitive |
//! | O3 | **确定性**：两次相同调用 ⇒ 输出**逐字节相同**；且结果按 (path, line, text) 有序 | deterministic order |
//! | O4 | **凭据路径跳过**：敏感面文件名（`.env` / `id_rsa` / `creds.json` / `secret` / `.ssh`…）**不得**出现在结果里 | credential paths skipped |
//! | O5 | **越界路径拒绝**：`path` 指向根外 ⇒ **拒绝**（不是"搜不到"，是**拒**） | 路径级拒绝 |
//! | O6 | **列号语义**：`column` 是**折行后**的 **1-based** 列（ASCII 输入下折行不改变长度 ⇒ 可精确断言） | `count_literal_hits` 的契约 |
//!
//! ## 诚实边界
//!
//! - **命中文本会被脱敏**（`redact_secret_field_values`）⇒ 本 target **不断言 `text` 的内容**
//!   （只断言它是字符串）—— 那是**另一个面**（脱敏）的承诺。
//! - 二进制文件 / 超大文件**跳过**语义不深测（模块自己有界读与跳过逻辑）。
//! - **多字节字符**的列号按**字节**计 ⇒ 本 target 的列号断言**只用 ASCII 内容**。
//! - 不测 `max_results` 截断语义（那属另一条承诺，本 target 不碰）。

use std::fs;
use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::search::SearchTool;

/// 源文件名池：**刻意混入敏感面名字**（好压 O4）。
const NAME_POOL: &[&str] = &[
    "notes.md",
    "code.rs",
    ".env",
    "id_rsa",
    "creds.json",
    "secret",
    "config.toml",
];

/// 敏感目录名（`is_sensitive_directory` 认的那几个）。
const SENSITIVE_DIRS: &[&str] = &[".ssh", ".aws", ".gnupg"];

/// 无依赖的极简 `block_on`（抄 `credentials.rs` 的同款：**不引 futures/tokio**，
/// 因为本工作区的纪律是 **0 引新外部 dep**；`SearchTool::invoke` 内没有挂起点，
/// 一次 poll 即完成）。
struct NoopWaker;

impl std::task::Wake for NoopWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWaker));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(value) => return value,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn invoke(tool: &SearchTool, arguments: Value) -> Result<Value, String> {
    let call = ToolCall {
        id: "call_search_fuzz".to_string(),
        name: "search".to_string(),
        arguments,
    };
    match block_on(tool.invoke(&call)).outcome {
        ToolOutcome::Ok { value } => Ok(value),
        ToolOutcome::Error { message, .. } => Err(message),
    }
}

fuzz_target!(|data: &[u8]| {
    let root = std::env::temp_dir().join(format!(
        "apeireth-search-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64))
    ));
    let _ = fs::create_dir_all(&root);

    // ── 查询串：刻意含**正则元字符**（O1 的核心）
    //    取输入里的可打印 ASCII 片段做前缀/后缀，中间夹一个 `.*`。
    let printable: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .collect();
    let prefix: String = printable
        .iter()
        .take(3)
        .map(|b| *b as char)
        .collect();
    let suffix: String = printable
        .iter()
        .skip(3)
        .take(3)
        .map(|b| *b as char)
        .collect();
    if prefix.is_empty() || suffix.is_empty() {
        // 构不出查询串 ⇒ 本次输入无意义（**不伪造期望**）。
        let _ = fs::remove_dir_all(&root);
        return;
    }
    let query = format!("{prefix}.*{suffix}");

    // ── 语料：三个文件
    //    ① `literal.txt`  **含**查询串的字面文本 ⇒ 必须被命中；
    //    ② `regexish.txt` 含 `prefix` + `xxx` + `suffix`（**长得像正则能匹配**）⇒ **不得**被命中；
    //    ③ `sensitive/*`  敏感面文件 ⇒ **不得**出现在结果里。
    let literal_path = root.join("literal.txt");
    let regexish_path = root.join("regexish.txt");
    fs::write(&literal_path, format!("head\n{query}\ntail\n")).expect("write literal");
    fs::write(&regexish_path, format!("head\n{prefix}xxx{suffix}\ntail\n")).expect("write regexish");
    for dir in SENSITIVE_DIRS {
        let dir_path = root.join(dir);
        let _ = fs::create_dir_all(&dir_path);
        let _ = fs::write(dir_path.join("payload.txt"), format!("{query}\n"));
    }
    for name in NAME_POOL {
        let _ = fs::write(root.join(name), format!("{query}\n"));
    }

    let tool = SearchTool::new(&root);

    // ── O1 字面量 + O3 确定性：跑两次，输出必须逐字节相同
    let first = invoke(&tool, json!({ "query": query })).expect("search 应成功");
    let second = invoke(&tool, json!({ "query": query })).expect("search 应成功");
    assert_eq!(
        first, second,
        "确定性：两次相同调用输出必须逐字节相同"
    );

    let matches = first["matches"]
        .as_array()
        .expect("matches 是数组")
        .clone();

    // ── O1：只有含**字面**查询串的那个文件可被命中
    for entry in &matches {
        let path = entry["path"].as_str().unwrap_or_default();
        assert!(
            !path.ends_with("regexish.txt"),
            "字面量：查询 {query:?} 不得匹配「长得像正则」的那份（{path}）"
        );
    }
    assert!(
        matches
            .iter()
            .any(|entry| entry["path"].as_str().unwrap_or_default().ends_with("literal.txt")),
        "字面量：含查询串字面文本的文件必须被命中（查询={query:?}）"
    );

    // ── O3：结果必须按 (path, line, text) 有序
    let keys: Vec<(String, u64, String)> = matches
        .iter()
        .map(|entry| {
            (
                entry["path"].as_str().unwrap_or_default().to_string(),
                entry["line"].as_u64().unwrap_or_default(),
                entry["text"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "确定性序：结果必须按 (path, line, text) 有序");

    // ── O2 大小写不敏感：大写查询必须命中同一批文件
    let upper = invoke(&tool, json!({ "query": query.to_uppercase() })).expect("search 应成功");
    let upper_matches = upper["matches"].as_array().expect("matches 是数组").clone();
    let literal_hits = |list: &[Value]| {
        list.iter()
            .filter(|entry| {
                entry["path"]
                    .as_str()
                    .unwrap_or_default()
                    .ends_with("literal.txt")
            })
            .count()
    };
    assert!(
        literal_hits(&upper_matches) == literal_hits(&matches) && literal_hits(&matches) >= 1,
        "大小写不敏感：大写查询的命中数必须与小写一致（upper={} lower={}）",
        literal_hits(&upper_matches),
        literal_hits(&matches)
    );

    // ── O4 凭据路径跳过：敏感面文件/目录不得出现在结果里
    for entry in &matches {
        let path = entry["path"].as_str().unwrap_or_default().to_ascii_lowercase();
        for name in [".env", "id_rsa", "creds.json", "secret"] {
            assert!(
                !path.contains(&format!("/{name}")),
                "凭据路径跳过：结果里出现了敏感面 {name}（{path}）"
            );
        }
        for dir in SENSITIVE_DIRS {
            assert!(
                !path.contains(&format!("/{dir}/")),
                "凭据路径跳过：结果里出现了敏感目录 {dir}（{path}）"
            );
        }
    }

    // ── O6 列号语义：`column` 是折行后的 1-based 列
    //    `literal.txt` 的命中行是第 2 行（"head" 是第 1 行）⇒ 命中从第 1 列开始。
    if let Some(entry) = matches
        .iter()
        .find(|entry| entry["path"].as_str().unwrap_or_default().ends_with("literal.txt"))
    {
        assert_eq!(
            entry["line"].as_u64(),
            Some(2),
            "命中行号应为 2（head 是第 1 行）: {entry}"
        );
        assert_eq!(
            entry["column"].as_u64(),
            Some(1),
            "命中列号应为 1（折行后的 1-based 字节列）: {entry}"
        );
        assert!(
            entry["occurrences"].as_u64().unwrap_or_default() >= 1,
            "命中次数必须 ≥1: {entry}"
        );
        assert!(
            entry["text"].is_string(),
            "text 必须是字符串（**本 target 不断言其内容** —— 命中文本会被脱敏）"
        );
    }

    // ── O5 越界路径拒绝：`path` 指向根外 ⇒ 拒绝（不是"搜不到"）
    let outside = invoke(
        &tool,
        json!({ "query": query, "path": "/etc" }),
    );
    assert!(
        outside.is_err(),
        "越界路径必须被拒绝（不是「搜不到」而是拒）: {:?}",
        outside
    );

    let _ = fs::remove_dir_all(&root);
    let _ = PathBuf::from(&root);
});
