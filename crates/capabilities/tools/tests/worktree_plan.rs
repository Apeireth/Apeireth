//! 只读改单工具面 (`tool.worktree_plan`) 的集成测试。
//!
//! 被测面: `crates/capabilities/tools/src/worktree_plan.rs` (crate
//! `apeireth-tools-canonical`)。改单是 `tool.worktree_apply` 的**输入合同** ——
//! 改单格式一旦漂移, 下游落盘拍的交叉核对会静默失配, 所以本文件把「改单的
//! 形状与拒绝面」钉死:
//!
//! - 正常路径: 合法改单被正确解析并产出结构化改单报告 (工具面 + 纯函数面);
//! - 确定性: 同输入两次解析逐字段一致, 改单任一要素变化必改 plan_id;
//! - 拒绝面: 空目标 / 空文件集 / 空路径 / 空预期 / 动作词表越界 / 工作区外
//!   路径 / 重复目标 / 敏感面 / 参数面越界 —— 一律**明确拒绝** (错误即帧,
//!   不 panic, 不静默放行);
//! - 只读保证: 计划拍**零落盘** (工作区逐字节不动, 不多出任何文件);
//! - 0 装诚实: 报告只说「已产改单」, 不谎称「已落盘」。
//!
//! 断言口径: 钉**行为**不钉字面量快照 —— 报告结构 (字段存在 / 类型 / 计数 /
//! 顺序) 与拒绝帧的**语义关键词**为主; 具体措辞可在演进中调整, 但语义关键词
//! 与输出合同的字段形状是合同面, 变更须同步本文件 (演进时在注释里注明理由)。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::{
    change_order_refusal, validated_change_order, worktree_plan_capability, ActionDistribution,
    WorktreeChangeAction, WorktreeChangeOrder, WorktreeFileChange, WorktreePlanBoundaryGuard,
    WorktreePlanTool, WORKTREE_PLAN_CAPABILITY_ID, WORKTREE_PLAN_TOOL_NAME,
};

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

fn call(arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call-plan-1".to_string(),
        name: WORKTREE_PLAN_TOOL_NAME.to_string(),
        arguments,
    }
}

fn expect_value(result: &ToolResult) -> serde_json::Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("期望成功却得错误帧: {message}"),
    }
}

fn expect_error(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("期望错误帧却得成功: {value}"),
    }
}

/// 改单参数面 (`goal` + 逐文件 `{path, action, expect}`)。
fn order_args(goal: &str, files: &[(&str, &str, &str)]) -> serde_json::Value {
    serde_json::json!({
        "goal": goal,
        "files": files
            .iter()
            .map(|(path, action, expect)| serde_json::json!({
                "path": path,
                "action": action,
                "expect": expect,
            }))
            .collect::<Vec<_>>(),
    })
}

/// 工作区内容快照 (相对路径 → 内容): 零落盘断言的量尺。
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .expect("relative")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read_to_string(&path).expect("read file"),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// 工作区文件条目集合 (相对路径): 「不多出任何文件」的量尺 (只看名字不看内容)。
fn entries(root: &Path) -> BTreeSet<String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .expect("relative")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

// ---------------------------------------------------------------------------
// 正常路径
// ---------------------------------------------------------------------------

/// ① 正常路径 (工具面): 合法改单被解析成结构化改单报告 —— 状态、计数、动作
/// 分布、逐文件声明 (顺序 + 三字段) 全部相符。
#[tokio::test]
async fn plan_produces_a_structured_change_order() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());
    let arguments = order_args(
        "多文件改: 更新 a, 删除 b, 新建 c",
        &[
            ("a.txt", "update", "内容更新为新语义"),
            ("b.txt", "delete", "移除废弃件"),
            ("c.txt", "add", "新建承接件"),
        ],
    );

    let value = expect_value(&tool.invoke(&call(arguments)).await);

    // 状态面: 计划拍只产改单, 状态词是 planned。
    assert_eq!(value["status"], "planned");
    // 计数与分布: 结构断言 (不钉字面量快照)。
    assert_eq!(value["file_count"], 3);
    assert_eq!(
        value["action_distribution"],
        serde_json::json!({ "add": 1, "update": 1, "delete": 1 })
    );
    // 逐文件声明: 顺序即改单顺序, 三字段逐项相符。
    let files = value["files"].as_array().expect("files 数组");
    assert_eq!(files.len(), 3);
    for (index, (path, action)) in [("a.txt", "update"), ("b.txt", "delete"), ("c.txt", "add")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(files[index]["path"], path);
        assert_eq!(files[index]["action"], action);
        assert!(
            files[index]["expect"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "每个文件必须带非空预期: {files:?}"
        );
    }
    // goal 原样回显 (改单是任务级合同, 下游按它核对补丁语义)。
    assert_eq!(value["goal"], "多文件改: 更新 a, 删除 b, 新建 c");
}

/// ①附 正常路径 (纯函数面): `validated_change_order` 解析出改单, 计数 / 分布 /
/// 动作标签 / 身份摘要的公开契约一致 —— 工具面与纯函数面不许各说各话。
#[test]
fn validated_change_order_parses_a_legal_change_order() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let arguments = order_args(
        "三件套",
        &[
            ("a.txt", "update", "改"),
            ("b.txt", "delete", "删"),
            ("c.txt", "add", "建"),
        ],
    );

    let order: WorktreeChangeOrder =
        validated_change_order(workspace.path(), &arguments).expect("合法改单必须被接受");

    assert_eq!(order.goal, "三件套");
    assert_eq!(order.file_count(), 3);
    assert_eq!(
        order.action_distribution(),
        ActionDistribution {
            add: 1,
            update: 1,
            delete: 1
        }
    );
    // 动作词表的稳定标签 (改单 / 补丁交叉核对同文)。
    assert_eq!(WorktreeChangeAction::Add.as_str(), "add");
    assert_eq!(WorktreeChangeAction::Update.as_str(), "update");
    assert_eq!(WorktreeChangeAction::Delete.as_str(), "delete");
    // 改单形状: 逐文件三字段。
    assert_eq!(
        order.files[0],
        WorktreeFileChange {
            path: "a.txt".to_string(),
            action: WorktreeChangeAction::Update,
            expect: "改".to_string(),
        }
    );
    // 纯判定面: 合法改单不产拒绝文案。
    assert!(
        change_order_refusal(workspace.path(), &order).is_none(),
        "合法改单不该被拒绝"
    );
    // 身份摘要: FNV-1a 64 hex (非密码学, 只做同一改单配对标识)。
    let plan_id = order.plan_id();
    assert_eq!(plan_id.len(), 16, "plan_id 是 FNV-1a 64 hex: {plan_id}");
    assert!(
        plan_id
            .chars()
            .all(|character| character.is_ascii_hexdigit()),
        "plan_id 必须是 hex: {plan_id}"
    );
}

// ---------------------------------------------------------------------------
// 确定性
// ---------------------------------------------------------------------------

/// ② 确定性: 同输入两次解析结果逐字段一致; 改单任一要素 (目标 / 路径 / 动作 /
/// 预期) 变化, 身份摘要必变 —— 摘要跟声明走, 不跟调用次序走。
#[tokio::test]
async fn plan_is_deterministic_for_the_same_change_order() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());
    let files = [("a.txt", "update", "改"), ("b.txt", "add", "建")];
    let arguments = order_args("确定性演示", &files);

    let first = expect_value(&tool.invoke(&call(arguments.clone())).await);
    let second = expect_value(&tool.invoke(&call(arguments.clone())).await);
    assert_eq!(first, second, "同输入两次解析必须逐字段一致");

    // 纯函数面同样确定。
    let left = validated_change_order(workspace.path(), &arguments).expect("合法改单");
    let right = validated_change_order(workspace.path(), &arguments).expect("合法改单");
    assert_eq!(left, right, "同输入两次解析出的改单必须相等");
    assert_eq!(left.plan_id(), right.plan_id(), "同改单必须同 id");

    // 要素变化 → 摘要变化 (逐项改, 每次只动一处)。
    let variants: Vec<(&str, &str, Vec<(&str, &str, &str)>)> = vec![
        ("换目标", "确定性演示二", files.to_vec()),
        (
            "换路径",
            "确定性演示",
            vec![("a.txt", "update", "改"), ("b2.txt", "add", "建")],
        ),
        (
            "换动作",
            "确定性演示",
            vec![("a.txt", "delete", "改"), ("b.txt", "add", "建")],
        ),
        (
            "换预期",
            "确定性演示",
            vec![("a.txt", "update", "改二"), ("b.txt", "add", "建")],
        ),
    ];
    let baseline = left.plan_id();
    for (label, goal, variant) in variants {
        let changed = validated_change_order(workspace.path(), &order_args(goal, &variant))
            .expect("合法改单");
        assert_ne!(
            changed.plan_id(),
            baseline,
            "改单 {label} 变了, 身份摘要必须变"
        );
    }
}

// ---------------------------------------------------------------------------
// 拒绝面: 明确拒绝, 不 panic
// ---------------------------------------------------------------------------

/// ③ 空任务描述 ⇒ 明确拒绝 (错误即帧): 空串与纯空白都拒; 拒绝帧不可重试
/// (同一份改单重试无意义), 且工作区零写入。
#[tokio::test]
async fn plan_refuses_an_empty_goal() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());
    let before = snapshot(workspace.path());

    for goal in ["", "   ", "\n\t "] {
        let result = tool
            .invoke(&call(order_args(goal, &[("a.txt", "add", "建")])))
            .await;
        let message = expect_error(&result);
        assert!(
            message.contains("任务目标"),
            "空目标必须被明确拒绝: {message}"
        );
        assert!(
            !result.outcome.is_retryable(),
            "改单内容面拒绝不可重试: {message}"
        );
    }
    assert_eq!(snapshot(workspace.path()), before, "拒绝路径必须零写入");
}

/// ④ 空文件列表 ⇒ 明确拒绝: 改单至少要有一条文件声明 (「什么都不改」不是改单)。
#[tokio::test]
async fn plan_refuses_an_empty_file_list() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    let message = expect_error(&tool.invoke(&call(order_args("空集", &[]))).await);
    assert!(
        message.contains("至少声明一个文件"),
        "空文件列表必须被明确拒绝: {message}"
    );
}

/// ⑤ 空路径 (空路径段) ⇒ 明确拒绝: 空串与纯空白路径都拒, 且拒绝帧点名
/// 「路径为空」而不是笼统报错。
#[tokio::test]
async fn plan_refuses_an_empty_target_path() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    for path in ["", "   ", "\t"] {
        let message = expect_error(
            &tool
                .invoke(&call(order_args("空路径", &[(path, "add", "建")])))
                .await,
        );
        assert!(
            message.contains("路径为空"),
            "空路径必须被明确拒绝: {message}"
        );
    }
}

/// ⑥ 工作区外路径 ⇒ 明确拒绝: 绝对路径 (含盘符 / UNC 前缀) 与 `..` 组件都拒
/// —— 改单只能落在工作区相对路径上。
#[tokio::test]
async fn plan_refuses_paths_that_escape_the_workspace() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    // 绝对路径: 用系统临时目录拼一个真实绝对路径 (跨平台稳定)。
    let absolute = std::env::temp_dir()
        .join("apeireth-plan-escape.txt")
        .to_string_lossy()
        .into_owned();
    let cases = [
        (absolute.as_str(), "禁止绝对路径"),
        ("../escape.txt", "禁止的路径组件"),
        ("dir/../../escape.txt", "禁止的路径组件"),
        ("..", "禁止的路径组件"),
    ];
    for (path, needle) in cases {
        let message = expect_error(
            &tool
                .invoke(&call(order_args("越界", &[(path, "update", "改")])))
                .await,
        );
        assert!(
            message.contains(needle),
            "越界路径 {path:?} 必须被拒: {message}"
        );
    }
}

/// ⑦ 动作词表越界 ⇒ 明确拒绝: 词表只有 add / update / delete 三项, 拼写
/// (大小写) 与近义词都算越界 —— 词表漂移会让下游补丁交叉核对失配。
#[tokio::test]
async fn plan_refuses_actions_outside_the_vocabulary() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    for action in ["modify", "create", "remove", "ADD", "Update", ""] {
        let message = expect_error(
            &tool
                .invoke(&call(order_args("词表越界", &[("a.txt", action, "改")])))
                .await,
        );
        assert!(
            message.contains("invalid worktree_plan parameters"),
            "词表外动作 {action:?} 必须被拒: {message}"
        );
    }

    // 词表内三动作全放行 (词表边界是三值, 不是单值)。
    for action in ["add", "update", "delete"] {
        let value = expect_value(
            &tool
                .invoke(&call(order_args("词表内", &[("a.txt", action, "改")])))
                .await,
        );
        assert_eq!(value["files"][0]["action"], action);
    }
}

/// ⑧ 重复目标 ⇒ 明确拒绝: 同一目标不得声明两次; 判定按**归一后的路径键**
/// (`./` 前缀与首尾空白归一), 所以换个写法也逃不掉。
#[tokio::test]
async fn plan_refuses_duplicate_targets() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    let cases = [
        // 同字面重复。
        vec![("a.txt", "update", "改"), ("a.txt", "delete", "删")],
        // `./` 前缀写法归一后同目标。
        vec![("a.txt", "update", "改"), ("./a.txt", "delete", "删")],
        // 首尾空白归一后同目标。
        vec![("a.txt", "update", "改"), ("  a.txt  ", "delete", "删")],
    ];
    for files in cases {
        let message = expect_error(&tool.invoke(&call(order_args("重复", &files))).await);
        assert!(
            message.contains("重复声明"),
            "重复目标必须被明确拒绝: {message}"
        );
    }

    // 边界: 不同目录下的同名文件**不是**重复目标 (归一键含目录部分)。
    let value = expect_value(
        &tool
            .invoke(&call(order_args(
                "同名不同目录",
                &[("dir/a.txt", "update", "改"), ("a.txt", "add", "建")],
            )))
            .await,
    );
    assert_eq!(value["file_count"], 2);
}

/// ⑨ 敏感面 ⇒ 明确拒绝: 凭据 / 密钥面 (含嵌套目录里的密钥件) 一律拒, 且
/// 安全面判定**先于**内容面判定 (敏感面 + 空预期同现时报的是敏感面)。
#[tokio::test]
async fn plan_refuses_credential_surfaces() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    for path in [
        ".env",
        "keys/id_rsa",
        "config/secrets.json",
        "certs/server.pem",
        ".ssh/config",
    ] {
        let message = expect_error(
            &tool
                .invoke(&call(order_args("敏感面", &[(path, "update", "改")])))
                .await,
        );
        assert!(
            message.contains("credential surface"),
            "敏感面 {path:?} 必须被拒: {message}"
        );
    }

    // 判定顺序: 敏感面先于空预期 (安全面优先, 不让内容面把安全面盖掉)。
    let message = expect_error(
        &tool
            .invoke(&call(order_args("敏感面", &[(".env", "update", "  ")])))
            .await,
    );
    assert!(
        message.contains("credential surface"),
        "安全面判定必须优先于内容面: {message}"
    );
}

/// ⑩ 空预期 ⇒ 明确拒绝: 没有预期的改动不进改单 (预期非空是人读合同)。
#[tokio::test]
async fn plan_refuses_an_empty_expectation() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    for expect in ["", "   ", "\n"] {
        let message = expect_error(
            &tool
                .invoke(&call(order_args("空预期", &[("a.txt", "update", expect)])))
                .await,
        );
        assert!(
            message.contains("预期结果"),
            "空预期必须被明确拒绝: {message}"
        );
    }
}

/// ⑪ 参数面封闭: 未知字段 / 缺字段 / 类型错一律错误帧 (错误即帧, 不静默
/// 忽略未知字段 —— 静默忽略会让模型以为声明生效了)。
#[tokio::test]
async fn plan_parameters_are_a_closed_surface() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    let cases = [
        // 顶层未知字段。
        serde_json::json!({
            "goal": "越界字段",
            "files": [{ "path": "a.txt", "action": "add", "expect": "建" }],
            "extra": 1,
        }),
        // 文件项未知字段。
        serde_json::json!({
            "goal": "越界字段",
            "files": [{ "path": "a.txt", "action": "add", "expect": "建", "mode": "x" }],
        }),
        // 缺 goal。
        serde_json::json!({
            "files": [{ "path": "a.txt", "action": "add", "expect": "建" }],
        }),
        // 缺 files。
        serde_json::json!({ "goal": "缺文件集" }),
        // 缺文件项字段 (expect)。
        serde_json::json!({
            "goal": "缺预期",
            "files": [{ "path": "a.txt", "action": "add" }],
        }),
        // 类型错 (files 非数组)。
        serde_json::json!({ "goal": "类型错", "files": "a.txt" }),
    ];
    for arguments in cases {
        let message = expect_error(&tool.invoke(&call(arguments)).await);
        assert!(
            message.contains("invalid worktree_plan parameters"),
            "参数面越界必须错误帧: {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// 只读保证: 零落盘
// ---------------------------------------------------------------------------

/// ⑫ 只读保证 (零落盘): 改单里声明了新建 / 修改 / 删除, 计划拍跑完之后工作区
/// 逐字节不动, **不多出任何文件** —— 计划不落盘是本工具的硬承诺。
#[tokio::test]
async fn plan_never_touches_the_workspace() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "kept a").expect("seed a");
    std::fs::create_dir_all(workspace.path().join("dir")).expect("seed dir");
    std::fs::write(workspace.path().join("dir/b.txt"), "kept b").expect("seed b");
    let before_content = snapshot(workspace.path());
    let before_entries = entries(workspace.path());

    let tool = WorktreePlanTool::new(workspace.path());
    let arguments = order_args(
        "只产改单, 不落盘",
        &[
            ("a.txt", "update", "改"),
            ("dir/b.txt", "delete", "删"),
            ("new.txt", "add", "建"),
        ],
    );
    expect_value(&tool.invoke(&call(arguments)).await);

    assert_eq!(
        snapshot(workspace.path()),
        before_content,
        "计划拍必须零落盘: 任何文件的增删改都算破戒"
    );
    assert_eq!(
        entries(workspace.path()),
        before_entries,
        "计划拍不得在作区多出任何文件"
    );
    assert!(
        !workspace.path().join("new.txt").exists(),
        "声明为新建的目标在工作区里必须仍不存在"
    );
    assert!(
        workspace.path().join("dir/b.txt").exists(),
        "声明为删除的目标在工作区里必须仍在 (删除是落盘拍的活)"
    );
}

// ---------------------------------------------------------------------------
// 0 装诚实: 报告不谎称已落盘
// ---------------------------------------------------------------------------

/// ⑬ 0 装诚实: 计划拍的报告只说「已产改单」, 不谎称「已落盘」—— 状态词是
/// planned, 且**不带**落盘侧的账目字段 (落地清单 / 验证账目); 改单摘要说的是
/// 「改单」不是「已落地」。工具声明里也写明只读零审批。
#[tokio::test]
async fn plan_report_never_claims_the_work_was_applied() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());
    let arguments = order_args(
        "诚实演示",
        &[("a.txt", "update", "改"), ("c.txt", "add", "建")],
    );

    let value = expect_value(&tool.invoke(&call(arguments.clone())).await);

    // 状态词: planned (不是 applied)。
    assert_eq!(value["status"], "planned");
    assert_ne!(value["status"], "applied");
    // 落盘侧账目字段一律不许出现 (那是落盘拍的报告形状)。
    for landing_field in [
        "applied",
        "files_added",
        "files_updated",
        "files_deleted",
        "verification",
    ] {
        assert!(
            value.get(landing_field).is_none(),
            "计划报告不得带落盘侧字段 {landing_field}: {value}"
        );
    }

    // 改单摘要说的是「改单」, 不是「已落地」。
    let order = validated_change_order(workspace.path(), &arguments).expect("合法改单");
    let summary = order.summary();
    assert!(summary.contains("改单摘要"), "{summary}");
    assert!(!summary.contains("已落盘"), "{summary}");
    assert!(!summary.contains("已落地"), "{summary}");

    // 工具声明写明只读零审批 (模型侧看到的就是这段话)。
    let description = tool
        .declaration()
        .description
        .clone()
        .expect("计划拍描述必须在册");
    assert!(
        description.contains("read-only") && description.contains("needs no approval"),
        "计划拍声明必须写明只读零审批: {description}"
    );
}

// ---------------------------------------------------------------------------
// 治理面: 边界 Guard / 五段流水线
// ---------------------------------------------------------------------------

/// ⑭ 边界 Guard 与工具同判定: 同一份改单, 工具拒 ⇒ Guard 也拒, 且拒绝文案
/// 同文 (预检与 Guard 不各说各话); 合法改单 Guard 无异议; Guard 只管自己
/// 的能力, 不越界拦别人的调用; 参数面解析失败不归 Guard (那是工具的解析帧)。
#[test]
fn plan_boundary_guard_agrees_with_the_tool() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let guard = WorktreePlanBoundaryGuard::new(workspace.path());
    let capability = CapabilityId::new(WORKTREE_PLAN_CAPABILITY_ID).expect("能力 id");

    // 越界改单: Guard 拒。
    let refused = order_args("越界", &[("../escape.txt", "update", "改")]);
    let refusal = guard
        .deny(&ToolGuardRequest::new(
            &capability,
            WORKTREE_PLAN_TOOL_NAME,
            &refused,
        ))
        .expect("越界改单必须被边界 Guard 拒");
    assert!(
        refusal.contains("禁止的路径组件"),
        "Guard 拒绝帧必须点名病灶: {refusal}"
    );

    // 合法改单: Guard 无异议 (不是放行, 是不拒)。
    let legal = order_args("合规", &[("a.txt", "add", "建")]);
    assert!(
        guard
            .deny(&ToolGuardRequest::new(
                &capability,
                WORKTREE_PLAN_TOOL_NAME,
                &legal
            ))
            .is_none(),
        "合规改单不该被边界 Guard 拒"
    );

    // Guard 只管自己的能力: 别的能力 id 一律不拦。
    let other = CapabilityId::new("tool.git_write").expect("能力 id");
    assert!(
        guard
            .deny(&ToolGuardRequest::new(&other, "git_write", &refused))
            .is_none(),
        "边界 Guard 不得越界拦别的能力"
    );

    // 参数面解析失败不归 Guard (解析失败是工具自己的错误帧)。
    let malformed = serde_json::json!({ "goal": "坏参数" });
    assert!(
        guard
            .deny(&ToolGuardRequest::new(
                &capability,
                WORKTREE_PLAN_TOOL_NAME,
                &malformed
            ))
            .is_none(),
        "解析失败不该由边界 Guard 背"
    );
}

/// ⑮ 五段流水线: 走完整流水线 (只读档零审批) 后, 产出仍满足声明的输出归一
/// 合同 (六字段); 越界改单在流水线里被拒, 且工作区零写入。
///
/// 演进注: 输出合同的六字段形状是**冻结**面 (改形状要同步下游消费方), 所以
/// 这里断言「字段齐 + 类型对」, 不钉具体文案。
#[tokio::test]
async fn plan_pipeline_keeps_the_frozen_output_contract() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let capability = worktree_plan_capability(workspace.path());

    // 合法改单: 流水线放行 (零审批 —— 无 ask 档钩子), 输出满足声明合同。
    let value = expect_value(
        &capability
            .invoke(&call(order_args(
                "流水线演示",
                &[("a.txt", "update", "改"), ("b.txt", "add", "建")],
            )))
            .await,
    );
    WorktreePlanTool::output_schema()
        .validate(&value)
        .expect("计划报告必须满足声明的输出归一合同");
    assert_eq!(value["status"], "planned");
    assert_eq!(value["file_count"], 2);

    // 越界改单: 流水线拒 (拒绝帧仍带病灶文案), 工作区零写入。
    let message = expect_error(
        &capability
            .invoke(&call(order_args(
                "越界",
                &[("../escape.txt", "update", "改")],
            )))
            .await,
    );
    assert!(
        message.contains("禁止的路径组件"),
        "流水线拒绝帧必须点名病灶: {message}"
    );
    assert!(
        entries(workspace.path()).is_empty(),
        "被拒的流水线调用不得留下任何文件"
    );
}

/// ⑯ 声明词表 == 实现词表: 模型侧 JSON Schema 里写的动作枚举必须与解析器
/// 真正接受的三项一致 —— 声明与实现漂移会让模型拿到「声明能过、实跑被拒」的
/// 改单。
#[test]
fn declared_action_vocabulary_matches_the_accepted_one() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let declaration = WorktreePlanTool::new(workspace.path()).declaration();

    let declared = declaration.parameters["properties"]["files"]["items"]["properties"]["action"]
        ["enum"]
        .as_array()
        .expect("动作词表必须是数组");
    let declared_labels: Vec<&str> = declared
        .iter()
        .map(|value| value.as_str().expect("词表项必须是字符串"))
        .collect();
    assert_eq!(
        declared_labels,
        vec!["add", "update", "delete"],
        "声明词表必须与实现词表同文"
    );

    // 声明里写明的必填项也要与解析器一致 (缺项必须是错误帧)。
    let required = declaration.parameters["properties"]["files"]["items"]["required"]
        .as_array()
        .expect("required 必须是数组");
    let required_labels: Vec<&str> = required
        .iter()
        .map(|value| value.as_str().expect("required 项必须是字符串"))
        .collect();
    assert_eq!(required_labels, vec!["path", "action", "expect"]);
}
