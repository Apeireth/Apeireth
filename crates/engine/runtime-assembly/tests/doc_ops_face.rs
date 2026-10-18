//! 创作工具面 W2 · 文档操作 (`tool.doc_ops`) 的装配级回归:
//! 注册即闸 (开关默认关 fail-closed) / 写总闸注入 (拒写不拒读) / 自述创作节
//! 同源同账 (工具面与自述同一计数器) / 名册 `doc_ops` 行按实际注册条件取值。
//!
//! 装配走真 [`ProductionModules`] 生产组装根; 配置注入才注册 (注册即闸),
//! 名册 `doc_ops` 行按实际注册条件取值。真 fixture = 手写 PDF 字节 (与工具面
//! 测试同口径的独立 fixture 生成)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::{DocOpsConfig, DOC_OPS_CAPABILITY_ID};
use serde_json::{json, Value};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

/// 与生产同款的最小装配: 记忆槽关闭, 工具槽默认开。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-doc-ops-face-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试工作区");
    dir
}

/// 手写 PDF fixture (单页单行, 真 PDF 字节)。
fn fixture_pdf() -> Vec<u8> {
    let content = "BT\n/F1 12 Tf\n72 720 Td\n(hello) Tj\nET\n";
    let objects: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                .to_string(),
        ),
        (
            4,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            ),
        ),
        (
            5,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_string(),
        ),
    ];
    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize; 6];
    for (num, body) in &objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_at = out.len();
    out.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &offsets[1..] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );
    out
}

fn capability<'a>(modules: &'a ProductionModules, id: &str) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

async fn invoke(tool: &Arc<dyn ToolCapability>, arguments: Value) -> Value {
    let call = ToolCall {
        id: "call_doc_ops".into(),
        name: "doc_ops".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    serde_json::from_str(&result.render()).expect("structured json")
}

async fn invoke_raw(tool: &Arc<dyn ToolCapability>, arguments: Value) -> (bool, String) {
    let call = ToolCall {
        id: "call_doc_ops".into(),
        name: "doc_ops".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    (result.is_ok(), result.render())
}

/// ① 注册即闸 + 开关 fail-closed: 默认配置 (配置落位缺席) = 工具不注册、
/// 名册 `doc_ops` 行照实 false; 落位注入 = 注册 + 名册真值 true。
#[test]
fn the_switch_is_fail_closed_and_registration_follows_the_slot() {
    assert!(
        DocOpsConfig::from_env(".").is_none(),
        "APEIRETH_ENABLE_DOC_OPS 未开必须不装配 (fail-closed)"
    );
    let modules =
        ProductionModules::build(base_config(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, DOC_OPS_CAPABILITY_ID).is_none(),
        "默认必须不注册文档操作面"
    );
    let roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "doc_ops")
        .expect("名册必须有 doc_ops 行");
    assert!(!row.enabled, "默认关 = 显式未接线");

    // 落位注入 (开关开的装配结果) = 注册 + 名册真值 true (注册即闸的正向面)。
    let root = temp_root("register");
    let mut config = base_config();
    config.doc_ops = Some(DocOpsConfig::new(&root));
    let modules =
        ProductionModules::build(config.clone(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, DOC_OPS_CAPABILITY_ID).is_some(),
        "落位注入必须注册文档操作面"
    );
    let roster = roster_from_config(&config, true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "doc_ops")
        .expect("名册必须有 doc_ops 行");
    assert!(row.enabled, "注册即闸的正向面必须照实 true");
}

/// ② 真读穿全流水线 + 自述创作节同源同账: 工具面一次 pdf_info 之后,
/// `tool.self_status` 创作节必须报同一计数器的真值 (只计数不回路径)。
#[tokio::test]
async fn a_real_read_shows_up_in_the_creation_section_of_self_status() {
    let root = temp_root("creation");
    std::fs::write(root.join("sample.pdf"), fixture_pdf()).unwrap();
    let mut config = base_config();
    config.doc_ops = Some(DocOpsConfig::new(&root));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();

    let doc_tool = capability(&modules, DOC_OPS_CAPABILITY_ID).expect("文档操作面已注册");
    let frame = invoke(
        doc_tool,
        json!({"action": "pdf_info", "input": "sample.pdf"}),
    )
    .await;
    assert_eq!(frame["status"], "ok");
    assert_eq!(frame["pages"], 1);

    let status_tool = capability(&modules, "tool.self_status").expect("自述面在场");
    let status = invoke(status_tool, json!({})).await;
    assert_eq!(status["doc_ops"]["calls"], 1);
    assert_eq!(status["doc_ops"]["pages_touched"], 1);
    assert_eq!(
        status["doc_ops"]["by_operation"][0]["operation"],
        "pdf_info"
    );
    assert_eq!(status["doc_ops"]["by_operation"][0]["count"], 1);
    assert!(
        !status["doc_ops"].to_string().contains(".pdf"),
        "创作节零路径零文件名: {}",
        status["doc_ops"]
    );
}

/// ③ 写总闸 (宪法级 `file_write` 唯一写总闸) 注入: 关 = 拒写不拒读 (拒绝即帧
/// 说明写总闸), 开 = 写面正常 (创建类原子写)。
#[tokio::test]
async fn the_write_gate_refuses_writes_but_never_reads() {
    let root = temp_root("gate");
    std::fs::write(root.join("sample.pdf"), fixture_pdf()).unwrap();
    let mut config = base_config();
    config.doc_ops = Some(DocOpsConfig::new(&root));
    // `file_write` 默认 false = 写总闸关。
    let modules = ProductionModules::build(
        config.clone(),
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let doc_tool = capability(&modules, DOC_OPS_CAPABILITY_ID).expect("文档操作面已注册");
    let frame = invoke(
        doc_tool,
        json!({"action": "pdf_info", "input": "sample.pdf"}),
    )
    .await;
    assert_eq!(frame["status"], "ok", "读面不受写总闸影响");
    let (ok, rendered) = invoke_raw(
        doc_tool,
        json!({
            "action": "pdf_extract_pages",
            "input": "sample.pdf",
            "output": "out.pdf",
            "pages": [1]
        }),
    )
    .await;
    assert!(!ok, "写总闸关必须拒写");
    assert!(
        rendered.contains("唯一写总闸"),
        "拒绝帧必须说明写总闸: {rendered}"
    );

    // 写总闸开 = 写面正常 (创建类原子写)。
    let mut config = base_config();
    config.doc_ops = Some(DocOpsConfig::new(&root));
    config.file_write = true;
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let doc_tool = capability(&modules, DOC_OPS_CAPABILITY_ID).expect("文档操作面已注册");
    let frame = invoke(
        doc_tool,
        json!({
            "action": "pdf_extract_pages",
            "input": "sample.pdf",
            "output": "out.pdf",
            "pages": [1]
        }),
    )
    .await;
    assert_eq!(frame["status"], "ok", "写总闸开必须放行写面");
    assert!(root.join("out.pdf").exists(), "输出必须真落盘");
}
