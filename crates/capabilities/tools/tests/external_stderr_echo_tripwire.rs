//! `ext.*` 族（外部工具适配族）**stderr 回显面**（失败帧）凭据形状绊线 —— 接线回归测试。
//!
//! # 这一族的面
//!
//! 上一批接的是本族的**成功面**（证据信封的四张字符串面）。本批接的是**第五张面**：
//! **stderr 回显面** —— 外部工具的 **stderr** 被拼进**错误帧**交给模型 ⇒ 工具自己吐出的
//! 凭据形状（目标里硬编码的密钥 / 工具配置里的令牌）会**原样回显**在错误帧里。
//!
//! 本族的 stderr 回显面有**两类**（外部工具把 stderr 交给我们的那两条路）：
//!
//! | 面 | 谁走 | 长度 | 越界面 |
//! |---|---|---|---|
//! | 截断面 | 多数适配器（`ensure_success`） | 前 **400 字**（既有合同） | 低 |
//! | 整段面 | 规则扫描族 / 元数据族 / 证书链族 | **不截断**（整段 stderr 原文） | **高** |
//!
//! **整段面是最凶的一面**（量级大、无上限）⇒ ⇒ 它必须**单独**被覆盖 —— 不能靠截断面的
//! 测试顺带证明（截断面的测试根本够不着那一段）。
//!
//! # 验收条目（逐条对应）
//!
//!   ① **本族 10 个适配器**逐个驱动：stderr 里的密钥形状串 ⇒ 错误帧**无明文** +
//!     带 `[REDACTED_OPENAI_KEY]`；
//!   ② **整段面（规则扫描族那一路）**：密钥串埋在**第 400 字之后**（截断面够不着的
//!     地方）⇒ 仍被遮 ⇒ 证明整段面**单独**接上了；
//!   ③ 错误帧**末尾的 JSON 注解**存在，且含 `"triggered": true` 与 `leaked_kinds`；
//!   ④ **干净 stderr ⇒ 错误帧逐字节全等**（**不挂注解**）⇒ 「仅命中才挂」的落点；
//!   ⑤ 错误帧的**既有语义不变**（错误文案主体 / 退出码 / 永久错误分类）；
//!   ⑥ 接线回归守卫：本族所有「stderr 进错误帧」的路都过本族的回显面绊线。
//!
//! # 夹具
//!
//! 夹具全是**固定内容** ⇒ 不赌概率；凭据串一律是**合成**的固定串（非真实凭据）。
//!
//! ⚠️ 本文件**不依赖任何外部工具二进制**（本机未必装）⇒ 用 `/bin/sh` 桩（假二进制）
//! 驱动：桩把固定夹具原样写到 stderr，再以固定退出码退出 ⇒ 确定性。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use serde_json::{json, Value};

use apeireth_tools_canonical::external::{
    ApktoolTool, BinwalkTool, ExifTool, GhidraTool, JadxTool, NmapTool, OpenSslTool, RadareTool,
    TsharkTool, YaraCliTool,
};
use apeireth_tools_canonical::recon::Scope;

// ────────────────────────────────────────────────────────────────────────────
// 夹具
// ────────────────────────────────────────────────────────────────────────────

/// 形状族夹具：一枚**合成**的裸密钥串（前缀后 ≥ 20 字符 ⇒ 命中形状族）。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnopqrstuv";

/// 形状层的遮形标签（内核既有标签）。
const SHAPE_LABEL: &str = "[REDACTED_OPENAI_KEY]";

/// 干净夹具：普通可打印文本（**不得**被遮 ⇒ 证明「零命中不挂注解」）。
const CLEAN_STDERR: &str = "scan aborted: target is not a regular file\n";

/// 截断面的既有窗口：前 **400 字**（先于本族接线存在的合同）。
const TRUNCATION_WINDOW: usize = 400;

/// 假二进制统一的退出码（非零 ⇒ 走失败帧；且**不等于 1** ⇒ 连容忍 1 的那两个适配器
/// 也会走失败帧）。
const STUB_EXIT_CODE: i32 = 2;

// ────────────────────────────────────────────────────────────────────────────
// helpers
// ────────────────────────────────────────────────────────────────────────────

/// 一个会被测试清理的暂存目录。
fn scratch_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("apeireth-ext-stderr-{tag}-"))
        .tempdir()
        .expect("scratch dir")
}

/// 只授权回环地址的 scope。
fn loopback_scope() -> Arc<Scope> {
    Arc::new(Scope {
        name: "loopback".into(),
        authorized: true,
        paths: Vec::new(),
        domains: Vec::new(),
        cidrs: vec!["127.0.0.1/32".into()],
    })
}

/// 一个带样例文件的工作区。
fn workspace_with_sample(scratch: &Path) -> PathBuf {
    let workspace = scratch.join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(workspace.join("sample.bin"), b"sample payload").expect("sample file");
    workspace
}

/// 一次工具调用。
fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_external_stderr".to_string(),
        name: name.to_string(),
        arguments,
    }
}

/// 失败结果的消息（成功时 panic）。
fn error_message(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("expected a failure, got {value}"),
    }
}

/// 造一个假二进制：把 `fixture` **原样**写到 stderr，再以 `code` 退出。
///
/// 桩不解释参数、不碰网络、不写文件 ⇒ 只把固定夹具搬进 stderr。
#[cfg(unix)]
fn failing_stub(dir: &Path, fixture: &str, code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let payload = dir.join("child-stderr.fixture");
    std::fs::write(&payload, fixture).expect("write stderr fixture");
    let script = dir.join("child-stub");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ncat '{}' >&2\nexit {code}\n", payload.display()),
    )
    .expect("write stub");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod stub");
    script
}

/// 造一段 stderr：密钥形状串的**起点**落在第 `pad` 个字符之后。
///
/// `pad` 之前全是普通文本（不含任何凭据形状）⇒ 起点位置**可精确断言**。
fn stderr_with_secret_after(pad: usize) -> String {
    let filler = "rule line matched at 0x0000dead with no credential shape\n";
    let mut blob = String::new();
    while blob.len() < pad {
        blob.push_str(filler);
    }
    blob.push_str(FAKE_KEY);
    blob.push('\n');
    blob
}

/// 一段**带上下文**的 stderr（秘密串前后都有普通文本）⇒ 用来证明遮形**只**换掉凭据形状，
/// 其余 stderr 原文仍在帧里。
fn stderr_with_context() -> String {
    format!("scan aborted: {FAKE_KEY}\nrule line matched at 0x0000dead\n")
}

/// 本族 10 个适配器，统一描述（全部指向同一个假二进制）。
struct AdapterCase {
    /// 模型面名字（`ext.<x>`）。
    name: &'static str,
    /// 适配器本体。
    tool: Arc<dyn ToolCapability>,
    /// 能通过适配器自身校验的参数。
    arguments: Value,
}

/// 本族 10 个适配器，全部指向 `binary`。
fn adapter_cases(binary: &Path, workspace: &Path, scope: Arc<Scope>) -> Vec<AdapterCase> {
    vec![
        AdapterCase {
            name: "ext.nmap",
            tool: Arc::new(NmapTool::new(Arc::clone(&scope)).with_binary(binary)),
            arguments: json!({"target": "127.0.0.1"}),
        },
        AdapterCase {
            name: "ext.exiftool",
            tool: Arc::new(ExifTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.tshark",
            tool: Arc::new(TsharkTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.yara",
            tool: Arc::new(YaraCliTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin", "rules": "rule demo { condition: true }"}),
        },
        AdapterCase {
            name: "ext.binwalk",
            tool: Arc::new(BinwalkTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.radare2",
            tool: Arc::new(RadareTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.jadx",
            tool: Arc::new(JadxTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.apktool",
            tool: Arc::new(ApktoolTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.ghidra",
            tool: Arc::new(GhidraTool::new(workspace).with_binary(binary)),
            arguments: json!({"path": "sample.bin"}),
        },
        AdapterCase {
            name: "ext.openssl",
            tool: Arc::new(OpenSslTool::new(Arc::clone(&scope)).with_binary(binary)),
            arguments: json!({"host": "127.0.0.1"}),
        },
    ]
}

// ────────────────────────────────────────────────────────────────────────────
// ① 本族 10 个适配器: stderr 里的密钥形状串 ⇒ 错误帧无明文 + 带遮形标签
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn every_adapter_masks_the_credential_shape_in_its_failure_frame() {
    let scratch = scratch_dir("mask-all");
    let workspace = workspace_with_sample(scratch.path());
    // 秘密串起点在第 0 字 ⇒ 落在**每一类**面的窗口内（截断面 400 字 / 整段面全段）。
    let stderr = stderr_with_secret_after(0);
    let binary = failing_stub(scratch.path(), &stderr, STUB_EXIT_CODE);

    let cases = adapter_cases(&binary, &workspace, loopback_scope());
    assert_eq!(cases.len(), 10, "本族必须仍是 10 个适配器");

    for case in cases {
        let result = case
            .tool
            .invoke(&call(case.name, case.arguments.clone()))
            .await;

        assert!(
            !result.is_ok(),
            "{} 必须失败（假二进制以 {STUB_EXIT_CODE} 退出）",
            case.name
        );
        let message = error_message(&result);
        assert!(
            !message.contains(FAKE_KEY),
            "{} 的错误帧不得留明文密钥: {message}",
            case.name
        );
        assert!(
            message.contains(SHAPE_LABEL),
            "{} 的错误帧必须带形状层遮形标签: {message}",
            case.name
        );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ② 整段面（规则扫描族那一路）: 400 字窗口之外的秘密串也被遮
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn the_whole_stderr_face_of_the_rule_scan_family_is_masked_too() {
    let scratch = scratch_dir("mask-whole");
    let workspace = workspace_with_sample(scratch.path());
    // 秘密串起点在 1024 字之后 ⇒ **落在截断面（400 字窗口）够不着的地方** ⇒
    // 只有整段面能遮它 ⇒ 这一条单独证明整段面接上了。
    let stderr = stderr_with_secret_after(1024);
    let secret_at = stderr.find(FAKE_KEY).expect("夹具里有密钥形状串");
    assert!(
        secret_at >= TRUNCATION_WINDOW,
        "夹具前提: 秘密串起点必须落在 {TRUNCATION_WINDOW} 字窗口之外, 实 {secret_at}"
    );

    let binary = failing_stub(scratch.path(), &stderr, STUB_EXIT_CODE);
    let tool = YaraCliTool::new(&workspace).with_binary(&binary);

    let result = tool
        .invoke(&call(
            "ext.yara",
            json!({"path": "sample.bin", "rules": "rule demo { condition: true }"}),
        ))
        .await;
    let message = error_message(&result);

    assert!(
        !message.contains(FAKE_KEY),
        "整段面不得留明文密钥: {message}"
    );
    assert!(
        message.contains(SHAPE_LABEL),
        "整段面必须带形状层遮形标签: {message}"
    );
    // 整段面**不截断** ⇒ 秘密串**之后**的普通文本仍在帧里（证明整段都进了帧）。
    assert!(
        message.contains("rule line matched at 0x0000dead"),
        "整段面必须把整段 stderr 带进帧: {message}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// ③ 错误帧末尾的 JSON 注解
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn a_hit_is_observable_through_the_trailing_json_annotation() {
    let scratch = scratch_dir("annotation");
    let workspace = workspace_with_sample(scratch.path());
    let stderr = stderr_with_secret_after(0);
    let binary = failing_stub(scratch.path(), &stderr, STUB_EXIT_CODE);
    let tool = BinwalkTool::new(&workspace).with_binary(&binary);

    let result = tool
        .invoke(&call("ext.binwalk", json!({"path": "sample.bin"})))
        .await;
    let message = error_message(&result);

    // 注解是**帧的最后一行**（裸文本面挂不住结构化侧车 ⇒ 末尾一行 JSON）。
    let last_line = message.lines().last().expect("失败帧非空").to_string();
    let annotation: Value =
        serde_json::from_str(&last_line).expect("帧末尾必须是可机读的 JSON 注解");
    let sidecar = annotation
        .get("credential_tripwire")
        .expect("注解必须带 credential_tripwire 侧车");
    assert_eq!(sidecar["triggered"], json!(true), "侧车必须报 triggered");
    let kinds = sidecar["leaked_kinds"]
        .as_array()
        .expect("侧车必须带命中族清单");
    assert!(
        kinds.iter().any(|kind| kind == "OpenAiKey"),
        "侧车必须报出形状族: {kinds:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// ④ 干净 stderr ⇒ 错误帧逐字节全等（不挂注解）
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn a_clean_stderr_leaves_the_failure_frame_byte_identical() {
    let scratch = scratch_dir("clean");
    let workspace = workspace_with_sample(scratch.path());
    let binary = failing_stub(scratch.path(), CLEAN_STDERR, STUB_EXIT_CODE);

    // 截断面（`ensure_success`）：帧 = `{binary} exited with {code:?}: {stderr}`。
    let tool = BinwalkTool::new(&workspace).with_binary(&binary);
    let result = tool
        .invoke(&call("ext.binwalk", json!({"path": "sample.bin"})))
        .await;
    let message = error_message(&result);
    let expected = format!(
        "{} exited with {:?}: {}",
        binary.display(),
        Some(STUB_EXIT_CODE),
        CLEAN_STDERR
    );
    assert_eq!(
        message, expected,
        "干净 stderr 的截断面错误帧必须逐字节全等"
    );
    assert!(
        !message.contains("credential_tripwire"),
        "干净 stderr 不得挂注解: {message}"
    );

    // 整段面（规则扫描族那一路）：帧 = `{tool} failed with {code:?}: {stderr}`。
    let tool = YaraCliTool::new(&workspace).with_binary(&binary);
    let result = tool
        .invoke(&call(
            "ext.yara",
            json!({"path": "sample.bin", "rules": "rule demo { condition: true }"}),
        ))
        .await;
    let message = error_message(&result);
    let expected = format!(
        "yara failed with {:?}: {}",
        Some(STUB_EXIT_CODE),
        CLEAN_STDERR
    );
    assert_eq!(
        message, expected,
        "干净 stderr 的整段面错误帧必须逐字节全等"
    );
    assert!(
        !message.contains("credential_tripwire"),
        "干净 stderr 不得挂注解: {message}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// ⑤ 既有语义不变
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn the_failure_frames_keep_their_existing_semantics() {
    let scratch = scratch_dir("semantics");
    let workspace = workspace_with_sample(scratch.path());
    let stderr = stderr_with_context();
    let binary = failing_stub(scratch.path(), &stderr, STUB_EXIT_CODE);

    // 截断面：文案主体与退出码不变（只换掉 stderr 里的凭据形状）。
    let tool = BinwalkTool::new(&workspace).with_binary(&binary);
    let result = tool
        .invoke(&call("ext.binwalk", json!({"path": "sample.bin"})))
        .await;
    assert!(
        !result.outcome.is_retryable(),
        "失败帧的分类不变: 仍是永久错误（不可重试）"
    );
    assert_eq!(
        result.name.as_deref(),
        Some("ext.binwalk"),
        "失败帧仍归属本适配器"
    );
    let message = error_message(&result);
    assert!(
        message.starts_with(&format!(
            "{} exited with Some({STUB_EXIT_CODE}): ",
            binary.display()
        )),
        "截断面的文案主体与退出码不变: {message}"
    );
    assert!(
        message.contains("scan aborted: ") && message.contains("rule line matched at 0x0000dead"),
        "遮形只换掉凭据形状 ⇒ 其余 stderr 原文仍在: {message}"
    );

    // 整段面：文案主体与退出码不变。
    let tool = YaraCliTool::new(&workspace).with_binary(&binary);
    let result = tool
        .invoke(&call(
            "ext.yara",
            json!({"path": "sample.bin", "rules": "rule demo { condition: true }"}),
        ))
        .await;
    assert!(
        !result.outcome.is_retryable(),
        "失败帧的分类不变: 仍是永久错误（不可重试）"
    );
    assert_eq!(
        result.name.as_deref(),
        Some("ext.yara"),
        "失败帧仍归属本适配器"
    );
    let message = error_message(&result);
    assert!(
        message.starts_with(&format!("yara failed with Some({STUB_EXIT_CODE}): ")),
        "整段面的文案主体与退出码不变: {message}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// ⑥ 接线回归守卫
// ────────────────────────────────────────────────────────────────────────────

/// 本族把 stderr 读进内存的地方**恰为两处**（截断面 + 整段面）⇒ 两处都必须过绊线。
///
/// 将来若有人再加一条「stderr 进错误帧」的路而绕过绊线，这条守卫会红。
#[test]
fn every_stderr_echo_path_runs_through_the_family_tripwire() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/external.rs");
    let text = std::fs::read_to_string(&source).expect("读得到族文件");

    assert_eq!(
        text.matches("String::from_utf8_lossy(&result.stderr)")
            .count(),
        2,
        "本族只应有两处把 stderr 读进内存（截断面 + 整段面）"
    );
    assert_eq!(
        text.matches("tripwire::scan_stderr_frame(").count(),
        2,
        "两处 stderr 回显面都必须过本族的回显面绊线"
    );
    assert_eq!(
        text.matches("ensure_success(&self.binary, &result)")
            .count(),
        7,
        "走截断面（ensure_success）的适配器应恰为 7 个"
    );
    assert_eq!(
        text.matches("failed_frame(\"").count(),
        3,
        "走整段面（failed_frame）的适配器应恰为 3 个"
    );
}

/// 本族的接线实现落在**族子模块**里，且它必须走**共享助手**（形状层文本面 + 末尾
/// 一行 JSON 注解）⇒ 守卫钉住这两点。
#[test]
fn the_family_tripwire_module_runs_on_the_shared_helpers() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/external/tripwire.rs");
    let text = std::fs::read_to_string(&source).expect("读得到族子模块");

    assert!(
        text.contains("scan_result_face"),
        "族子模块必须走共享的文本面扫描助手"
    );
    assert!(
        text.contains("append_credential_tripwire"),
        "族子模块必须走共享的末尾 JSON 注解助手"
    );
    assert!(
        text.contains("pub fn scan_stderr_frame"),
        "族子模块必须导出 stderr 回显面的接线入口"
    );
}
