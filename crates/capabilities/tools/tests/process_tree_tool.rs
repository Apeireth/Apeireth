//! `tool.process_tree`（模型面工具面 · 能力创造线 W1 C-B1）回归。
//!
//! 覆盖用例：
//!
//! 1. **快照含本进程**（自根树 ⇒ 覆盖我们 spawn 出来的全部子进程）；
//! 2. **空树**（无子进程 ⇒ 快照只有根自己）；
//! 3. **表外动作拒绝即帧**（词表闭合）；
//! 4. **平台未实现显式报错**（不返回空快照冒充成功）；
//! 5. **取消非后代 pid 拒绝**（归属证明 fail-closed ⇒ 不越权杀别人的进程）；
//! 6. **取消需人批**（断言人批被触发；快照零审批；词表外 fail-closed 拒）；
//! 7. **对抗：伪 pid 拒绝且不误杀**（不存在的 pid ⇒ 拒绝即帧，别的进程照旧活着）；
//! 8. 取消真后代 ⇒ 子进程真的没了；重复取消不再产生副作用；
//! 9. 未知参数拒绝；取消本进程自身拒绝；冻结载荷绑定**确切目标 pid**。
//!
//! 全部用例都起**真进程**，不使用任何 mock 进程执行器。

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::process::{
    process_tree_support, PlatformKind, ProcessTreeError, ProcessTreeSupport,
};
use apeireth_tools_canonical::process_tree_tool::{
    process_tree_approval_reason, process_tree_error_message, process_tree_write_face_for_action,
    ProcessTreeApprovalHook, ProcessTreeTool, ProcessTreeWriteFace, ACTION_CANCEL, ACTION_SNAPSHOT,
    PROCESS_TREE_ACTIONS, PROCESS_TREE_CAPABILITY_ID, PROCESS_TREE_TOOL_NAME,
};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

/// 子进程测试用的 helper 可执行文件路径（与既有进程测试同款做法）。
fn helper() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_sandbox-test-child") {
        return PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps/
    dir.pop(); // <profile>/
    dir.push(if cfg!(windows) {
        "sandbox-test-child.exe"
    } else {
        "sandbox-test-child"
    });
    dir
}

/// 起一个子进程，测试结束必收尸（不留僵尸、不留挂着的 sleeper）。
struct ChildGuard(Child);

impl ChildGuard {
    fn pid(&self) -> u32 {
        self.0.id()
    }

    /// 等子进程**真的**退出（终止动作送达 ⇒ 进程被内核收掉需要一点时间）。
    fn wait_for_exit(&mut self, timeout_ms: u64) -> bool {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if self.0.try_wait().ok().flatten().is_some() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 起一个普通子进程（与本测试进程同组 ⇒ 属于本会话的受控树）。
fn spawn_child(args: &[&str]) -> ChildGuard {
    let child = Command::new(helper())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("helper child must spawn");
    ChildGuard(child)
}

/// 工具调用一帧。
fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: PROCESS_TREE_TOOL_NAME.into(),
        arguments,
    }
}

/// 工具面（无配置面：本面不需要任何端点或凭据）。
fn tool() -> ProcessTreeTool {
    ProcessTreeTool::new()
}

/// 成功帧的结构化值。
async fn ok_value(tool: &ProcessTreeTool, arguments: Value) -> Value {
    let result = tool.invoke(&call(arguments)).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("结构化 json")
}

/// 拒绝帧的结果（供需要断言「永久错误 / 不可重试」的用例用）。
async fn refused_result(tool: &ProcessTreeTool, arguments: Value) -> ToolResult {
    let result = tool.invoke(&call(arguments)).await;
    assert!(!result.is_ok(), "必须拒绝即帧: {}", result.render());
    result
}

/// 拒绝帧的文案。
async fn refused(tool: &ProcessTreeTool, arguments: Value) -> String {
    refused_result(tool, arguments).await.render()
}

/// 治理请求（能力派发）。
fn dispatch<'a>(capability: &'a CapabilityId, arguments: &'a Value) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
}

/// 进程是否还活着（探针）。
#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // SAFETY: 信号 0 只做存在性探测，不投递任何信号。
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// 派生一个**脱离本会话**的进程（double fork ⇒ 被会话初始进程收养）。
#[cfg(target_os = "linux")]
fn spawn_detached_sleeper(seconds: u64) -> u32 {
    // SAFETY: fork 之后子进程只调用异步信号安全的 libc 调用并 _exit 退出。
    unsafe {
        let mut fds = [0; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe must succeed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        let first = libc::fork();
        assert!(first >= 0, "fork must succeed");
        if first == 0 {
            libc::close(read_fd);
            libc::close(1);
            libc::close(2);
            let second = libc::fork();
            if second == 0 {
                libc::close(write_fd);
                sleep_in_child(seconds);
                libc::_exit(0);
            }
            let bytes = second.to_le_bytes();
            libc::write(write_fd, bytes.as_ptr().cast::<libc::c_void>(), bytes.len());
            libc::_exit(0);
        }

        libc::close(write_fd);
        let mut buffer = [0u8; 4];
        let mut filled = 0usize;
        while filled < buffer.len() {
            let read = libc::read(
                read_fd,
                buffer[filled..].as_mut_ptr().cast::<libc::c_void>(),
                buffer.len() - filled,
            );
            if read <= 0 {
                break;
            }
            filled += read as usize;
        }
        libc::close(read_fd);

        let mut status = 0;
        libc::waitpid(first, &mut status, 0);
        assert_eq!(filled, 4, "detached child pid must be readable");
        u32::from_le_bytes(buffer)
    }
}

/// 子进程里的睡眠（只用异步信号安全的调用）。
#[cfg(target_os = "linux")]
fn sleep_in_child(seconds: u64) {
    let mut remaining = libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: 0,
    };
    loop {
        let mut leftover = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = unsafe { libc::nanosleep(&remaining, &mut leftover) };
        if result == 0 {
            return;
        }
        remaining = leftover;
    }
}

// ---------------------------------------------------------------------------
// ① 快照含本进程
// ---------------------------------------------------------------------------

/// 自根树快照：根 = 本进程自身 ⇒ 快照**含本进程**，且能看到我们 spawn 的子进程。
#[tokio::test]
async fn snapshot_includes_the_current_process() {
    let child = spawn_child(&["sleep", "30"]);
    let value = ok_value(&tool(), json!({ "action": ACTION_SNAPSHOT })).await;

    assert_eq!(value["action"], ACTION_SNAPSHOT);
    assert_eq!(value["status"], "ok");
    let self_pid = std::process::id();
    assert_eq!(value["root_pid"], self_pid, "根必须是本进程自身");

    let entries = value["entries"].as_array().expect("entries 是数组");
    let root = entries
        .iter()
        .find(|entry| entry["pid"] == self_pid)
        .expect("快照必须含本进程");
    assert_eq!(root["parent_pid"].as_u64().is_some(), true);
    assert!(root["status"].is_string(), "每条成员都要有状态标签");
    assert!(
        root["group_containment"].is_string(),
        "每条成员都要有收容组归属标签"
    );

    // 我们 spawn 出来的子进程也必须在树里。
    assert!(
        entries
            .iter()
            .any(|entry| entry["pid"] == json!(child.pid())),
        "快照必须覆盖我们 spawn 的子进程"
    );
}

// ---------------------------------------------------------------------------
// ② 空树（无子进程）
// ---------------------------------------------------------------------------

/// 空树：一个没有子进程的后代 ⇒ 快照只有它自己（零后代）。
#[tokio::test]
async fn snapshot_of_a_childless_descendant_is_a_one_member_tree() {
    let child = spawn_child(&["sleep", "30"]);
    let value = ok_value(
        &tool(),
        json!({ "action": ACTION_SNAPSHOT, "pid": child.pid() }),
    )
    .await;

    assert_eq!(value["root_pid"], child.pid());
    assert_eq!(value["count"], 1, "无子进程的树只有一个成员");
    let entries = value["entries"].as_array().expect("entries 是数组");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["pid"], child.pid());
}

// ---------------------------------------------------------------------------
// ③ 表外动作拒绝即帧
// ---------------------------------------------------------------------------

/// 词表外动作（与动作缺失）⇒ 拒绝即帧，不猜、不静默放行。
#[tokio::test]
async fn unknown_action_is_refused_outright() {
    let tool = tool();
    for action in ["kill", "list", "SNAPSHOT", ""] {
        let message = refused(&tool, json!({ "action": action })).await;
        assert!(
            message.contains("词表") || message.contains("必须显式给 action"),
            "动作 {action:?} 必须被拒: {message}"
        );
    }

    // 动作缺失同样拒绝。
    let message = refused(&tool, json!({})).await;
    assert!(message.contains("必须显式给 action"), "{message}");

    // 词表常量本身必须闭合在这两个动作上。
    assert_eq!(PROCESS_TREE_ACTIONS, [ACTION_SNAPSHOT, ACTION_CANCEL]);
}

// ---------------------------------------------------------------------------
// ④ 平台未实现 ⇒ 显式报错（不返回空快照冒充成功）
// ---------------------------------------------------------------------------

/// 平台未实现 ⇒ 显式 `Unsupported` 错误帧；**绝不**返回空快照冒充成功。
#[tokio::test]
async fn unsupported_platform_is_reported_explicitly() {
    // ① 与宿主平台无关的判定：未实现平台的错误文案必须显式点出「未实现」。
    let message = process_tree_error_message(
        &ProcessTreeError::Unsupported {
            platform: PlatformKind::MacOs,
        },
        ACTION_SNAPSHOT,
    );
    assert!(message.contains("未实现"), "{message}");
    assert!(message.contains("macos"), "{message}");
    assert!(
        !message.contains("\"status\":\"ok\""),
        "未实现平台不得被写成成功: {message}"
    );

    // ② 真机口径：按平台支持档分支，已实现平台必须给出真树，未实现平台必须报错。
    match process_tree_support() {
        ProcessTreeSupport::Enumerable => {
            let value = ok_value(&tool(), json!({ "action": ACTION_SNAPSHOT })).await;
            assert_eq!(value["status"], "ok");
            assert_eq!(value["platform_support"], "enumerable");
            assert!(
                value["count"].as_u64().unwrap_or(0) >= 1,
                "已实现平台必须给出真树: {value}"
            );
        }
        ProcessTreeSupport::Unsupported => {
            let message = refused(&tool(), json!({ "action": ACTION_SNAPSHOT })).await;
            assert!(message.contains("未实现"), "{message}");
        }
    }
}

// ---------------------------------------------------------------------------
// ⑤ 取消非后代 pid ⇒ 拒绝（归属证明 fail-closed）
// ---------------------------------------------------------------------------

/// 取消一个**证不出后代关系**的 pid ⇒ 拒绝即帧，且**不动**那个进程。
#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancel_refuses_a_pid_outside_this_session() {
    let pid = spawn_detached_sleeper(30);

    let result = refused_result(&tool(), json!({ "action": ACTION_CANCEL, "pid": pid })).await;
    let message = result.render();
    assert!(
        message.contains("后代") || message.contains("拒绝"),
        "非后代 pid 必须被拒: {message}"
    );
    assert!(
        !result.outcome.is_retryable(),
        "归属证明失败 ⇒ 永久错误帧, 不得制造重试风暴: {message}"
    );
    assert!(
        process_alive(pid),
        "被拒的目标必须还活着 (不得跨会话越权终止)"
    );
}

/// 会话初始进程（pid 1）永远不是本进程的后代 ⇒ 取消它必须被拒。
#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancel_refuses_the_session_leader() {
    let message = refused(&tool(), json!({ "action": ACTION_CANCEL, "pid": 1 })).await;
    assert!(message.contains("后代"), "{message}");
    assert!(process_alive(1), "会话初始进程必须还活着");
}

// ---------------------------------------------------------------------------
// ⑥ 取消需人批（快照零审批 · 词表外 fail-closed 拒）
// ---------------------------------------------------------------------------

/// 治理档：**取消**每次都停人工审批；**快照**零审批；词表外 fail-closed 拒。
#[tokio::test]
async fn cancel_requires_human_approval_and_snapshot_does_not() {
    let hook = ProcessTreeApprovalHook::new();
    let capability = CapabilityId::new(PROCESS_TREE_CAPABILITY_ID).expect("capability id");

    // 写面：每次都停人工审批（无自动放行档）。
    let arguments = json!({ "action": ACTION_CANCEL, "pid": 4242 });
    let decision = hook.evaluate(&dispatch(&capability, &arguments)).await;
    let Decision::RequireApproval { reason } = decision else {
        panic!("cancel 必须停人工审批: {decision:?}");
    };
    assert!(reason.contains("逐次人批"), "{reason}");
    assert!(reason.contains("无自动放行档"), "{reason}");
    assert!(reason.contains("后代"), "{reason}");

    // 读面：零审批。
    let arguments = json!({ "action": ACTION_SNAPSHOT });
    let decision = hook.evaluate(&dispatch(&capability, &arguments)).await;
    assert_eq!(decision, Decision::Allow, "{decision:?}");

    // 词表外 / 动作缺失：fail-closed 拒绝（不静默放行）。
    for action in [None, Some("kill"), Some("list")] {
        let arguments = match action {
            Some(action) => json!({ "action": action }),
            None => json!({}),
        };
        let decision = hook.evaluate(&dispatch(&capability, &arguments)).await;
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "动作 {action:?} 必须 fail-closed 拒绝: {decision:?}"
        );
    }

    // 本钩子只管自己的面：其它能力一律放行（不越权误伤）。
    let other = CapabilityId::new("tool.care_schedule").expect("capability id");
    let arguments = json!({ "action": "care_add" });
    let decision = hook.evaluate(&dispatch(&other, &arguments)).await;
    assert_eq!(decision, Decision::Allow, "本钩子不得误伤别的面");

    // 审批理由文案与治理档一致（读面写面分别标注）。
    assert!(process_tree_approval_reason(Some(ACTION_CANCEL)).contains("逐次人批"));
    assert!(process_tree_approval_reason(Some(ACTION_SNAPSHOT)).contains("零审批"));
    assert_eq!(
        process_tree_write_face_for_action(ACTION_SNAPSHOT),
        Some(ProcessTreeWriteFace::ReadOnly)
    );
    assert_eq!(
        process_tree_write_face_for_action(ACTION_CANCEL),
        Some(ProcessTreeWriteFace::ProcessTermination)
    );
}

// ---------------------------------------------------------------------------
// ⑦ 对抗：伪 pid 拒绝且不误杀
// ---------------------------------------------------------------------------

/// 不存在的 pid（伪 pid）⇒ 拒绝即帧，**且不得误杀**任何进程。
#[tokio::test]
async fn cancel_of_a_fake_pid_is_refused_without_killing_anything() {
    let child = spawn_child(&["sleep", "30"]);
    let fake_pid = u32::MAX;

    let result = refused_result(&tool(), json!({ "action": ACTION_CANCEL, "pid": fake_pid })).await;
    let message = result.render();
    assert!(
        message.contains("不存在") || message.contains("拒绝"),
        "伪 pid 必须被拒: {message}"
    );
    assert!(
        !message.contains("\"status\":\"ok\""),
        "伪 pid 不得被报成成功: {message}"
    );
    assert!(
        !result.outcome.is_retryable(),
        "拒绝即帧 ⇒ 永久错误, 不得制造重试风暴: {message}"
    );

    // 不误杀：另一个真实子进程必须照旧活着。
    assert!(process_alive(child.pid()), "伪 pid 的取消不得误杀任何进程");
}

// ---------------------------------------------------------------------------
// ⑧ 取消真后代：子进程真的没了；重复取消不再产生副作用
// ---------------------------------------------------------------------------

/// 取消一棵**已证明是后代**的树 ⇒ 子进程真的被终止。
#[tokio::test]
async fn cancel_terminates_a_proven_descendant() {
    let mut child = spawn_child(&["sleep", "30"]);
    let pid = child.pid();

    let value = ok_value(&tool(), json!({ "action": ACTION_CANCEL, "pid": pid })).await;
    assert_eq!(value["action"], ACTION_CANCEL);
    assert_eq!(value["status"], "cancelled");
    assert_eq!(value["root_pid"], pid);
    assert_eq!(value["caused_termination"], true);
    let terminated = value["terminated"].as_array().expect("terminated 是数组");
    assert!(
        terminated.iter().any(|entry| *entry == json!(pid)),
        "被取消的根必须在终止清单里: {value}"
    );
    assert!(child.wait_for_exit(5_000), "被取消的子进程必须真的消失");

    // 重复取消：目标已不在 ⇒ 拒绝即帧（零副作用，如实报错而不是假装成功）。
    let message = refused(&tool(), json!({ "action": ACTION_CANCEL, "pid": pid })).await;
    assert!(
        message.contains("不存在") || message.contains("拒绝"),
        "重复取消必须如实报错: {message}"
    );
}

// ---------------------------------------------------------------------------
// ⑨ 参数边界 + 冻结载荷绑定确切目标
// ---------------------------------------------------------------------------

/// 未知参数拒绝（词表外的入参不许被静默忽略）。
#[tokio::test]
async fn unknown_parameter_is_refused() {
    let message = refused(&tool(), json!({ "action": ACTION_SNAPSHOT, "signal": 9 })).await;
    assert!(message.contains("未知进程树参数"), "{message}");
}

/// 取消本进程自身 ⇒ 拒绝（不许自我了断）。
#[tokio::test]
async fn cancel_of_the_current_process_is_refused() {
    let message = refused(
        &tool(),
        json!({ "action": ACTION_CANCEL, "pid": std::process::id() }),
    )
    .await;
    assert!(message.contains("本进程自身"), "{message}");
}

/// 冻结载荷绑定**确切的目标 pid** ⇒ 人工批准的操作 == 实际执行的操作。
#[tokio::test]
async fn frozen_invocation_binds_the_exact_target_pid() {
    let child = spawn_child(&["sleep", "30"]);
    let tool = tool();

    let frozen = tool
        .freeze_invocation(&call(
            json!({ "action": ACTION_CANCEL, "pid": child.pid() }),
        ))
        .expect("冻结必须成功")
        .expect("取消面必须给出冻结载荷");
    assert_eq!(frozen.payload["action"], ACTION_CANCEL);
    assert_eq!(frozen.payload["pid"], child.pid());
    assert_eq!(frozen.display["target_pid"], child.pid());

    // 非法目标 ⇒ **不生成待审批项**（免得让人去批一个无效操作）。
    let error = tool.freeze_invocation(&call(json!({ "action": ACTION_CANCEL, "pid": 1 })));
    assert!(error.is_err(), "非后代 pid 不得生成待审批的冻结载荷");
}
