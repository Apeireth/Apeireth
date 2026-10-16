//! 对象级判定测试 (E-P3 · 威胁模型 G3「沙箱对象级判定」)。
//!
//! 钉住的行为:
//! 1. **判定矩阵**: 四动作 (创建/修改/删除/枚举) × 域分区 (域内合规/敏感/域外);
//! 2. **形态无关**: 同对象同动作, API 形态不同 (档案 API / 句柄 API / 子进程命令面 /
//!    静态调用面) 判定逐一相同;
//! 3. **词法放行但对象越界的兜底**: 词法面明示的漏放形态 (编码负载) 落地对象
//!    越界 → 失败即帧 + `sandbox.object_violation` 审计;
//! 4. **审计行**: 对象/动作/进程/域/理由逐项在行;
//! 5. **契约挂载点**: fake policy 接入 (请求面+事实面都被征询), 策略链第一个
//!    拒绝即拒绝;
//! 6. **执行前后状态对比**: 只认窗口内对象事实 (含并发安全)。

use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::{
    scan_shell_write_intent, FsObjectStateProbe, MemoryObjectAuditSink, ObjectAccessJudge,
    ObjectAccessPolicy, ObjectAccessRequest, ObjectAction, ObjectChange, ObjectDomain,
    ObjectGuardError, ObjectPolicyChain, ObjectSubject, ObjectVerdict, ObjectViolation, ObjectZone,
    PipelineFailure, ShellTool, ToolGuardrail, TrustedShellConfig, WorkspaceObjectPolicy,
    OBJECT_VIOLATION_AUDIT_EVENT,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 假策略 (契约挂载点的接入形态, 星港线深望智能体按此实现 ObjectAccessPolicy)
// ---------------------------------------------------------------------------

/// 记录征询次数、按对象名拒绝的假策略。
struct RecordingPolicy {
    deny_object: &'static str,
    calls: Mutex<usize>,
}

impl ObjectAccessPolicy for RecordingPolicy {
    fn name(&self) -> &str {
        "recording_object_policy"
    }

    fn decide(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        *self
            .calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
        if request.object.ends_with(self.deny_object) {
            ObjectVerdict::deny("fake policy forbids this object")
        } else {
            ObjectVerdict::Allow
        }
    }
}

/// 恒放行策略 (策略链对照)。
struct AlwaysAllow;

impl ObjectAccessPolicy for AlwaysAllow {
    fn name(&self) -> &str {
        "always_allow"
    }

    fn decide(&self, _request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        ObjectVerdict::Allow
    }
}

/// 恒拒绝策略 (策略链对照)。
struct AlwaysDeny;

impl ObjectAccessPolicy for AlwaysDeny {
    fn name(&self) -> &str {
        "always_deny"
    }

    fn decide(&self, _request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        ObjectVerdict::deny("always deny")
    }
}

/// 事实清单: (动作, 对象名), 按差分键序。
fn facts(report: &apeireth_tools_canonical::ObjectGuardReport) -> Vec<(ObjectAction, String)> {
    report
        .changes()
        .iter()
        .map(|change: &ObjectChange| {
            let name = change
                .object
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            (change.action, name)
        })
        .collect()
}

fn call_shell(command: String) -> ToolCall {
    ToolCall {
        id: "call_object_judgment".into(),
        name: "shell".into(),
        arguments: json!({ "command": command }),
    }
}

async fn invoke_shell(
    tool: &ShellTool,
    command: String,
) -> apeireth_protocol::canonical::ToolResult {
    tool.invoke(&call_shell(command)).await
}

// ---------------------------------------------------------------------------
// 1. 判定矩阵: 四动作 × 越界/合规
// ---------------------------------------------------------------------------

/// 对象判定矩阵: 四动作 × (域内合规 / 敏感对象 / 域外对象) —— 合规放行,
/// 敏感与域外一律拒 (fail-closed), 与动作种类无关。
#[test]
fn object_decision_matrix_covers_four_actions_across_zones() {
    let base = tempfile::tempdir().unwrap();
    let workspace = base.path().join("ws");
    std::fs::create_dir_all(workspace.join(".ssh")).unwrap();
    let domain = ObjectDomain::new(workspace.clone());
    let subject = ObjectSubject::new("matrix-process");
    let policy = WorkspaceObjectPolicy;

    let compliant = workspace.join("notes.txt");
    let sensitive_file = workspace.join(".env");
    let sensitive_nested = workspace.join(".ssh").join("id_rsa");
    let outside = base.path().join("outside.txt");

    for action in ObjectAction::ALL {
        for (label, object, expected_allow) in [
            ("compliant", compliant.as_path(), true),
            ("sensitive-file", sensitive_file.as_path(), false),
            ("sensitive-nested", sensitive_nested.as_path(), false),
            ("outside", outside.as_path(), false),
        ] {
            let verdict = policy.decide(&domain.request(&subject, object, action));
            assert_eq!(
                verdict.is_allow(),
                expected_allow,
                "{label} x {}: {verdict:?}",
                action.as_str()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 执行前后状态对比 (事实判定)
// ---------------------------------------------------------------------------

/// 事实判定矩阵: 窗口内创建/修改/删除逐一对账, 未动对象零入账。
#[test]
fn state_diff_covers_create_modify_delete_and_keeps_untouched_objects() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join("del.txt"), "x").unwrap();
    std::fs::write(ws.path().join("keep.txt"), "x").unwrap();
    std::fs::write(ws.path().join("mod.txt"), "x").unwrap();

    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("diff-process");
    let ((), report) = judge
        .guard(subject, ObjectDomain::new(ws.path().to_path_buf()), || {
            std::fs::remove_file(ws.path().join("del.txt")).unwrap();
            std::fs::write(ws.path().join("mod.txt"), "longer-content").unwrap();
            std::fs::write(ws.path().join("new.txt"), "y").unwrap();
        })
        .unwrap();

    assert_eq!(
        facts(&report),
        [
            (ObjectAction::Delete, "del.txt".to_string()),
            (ObjectAction::Modify, "mod.txt".to_string()),
            (ObjectAction::Create, "new.txt".to_string()),
        ]
    );
    assert!(report.complete());
    assert!(!report.is_failure(), "合规对象变更不得判失败");
}

/// 状态对比正确性: 只认窗口内对象事实 —— 开窗前既有对象不动不入账, 核验后
/// 的变更不入账, 窗口内的修改是 Modify 不是 Create。
#[test]
fn window_comparison_attributes_only_in_window_object_changes() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join("pre.txt"), "before").unwrap();

    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("window-process");
    let window = judge
        .open(subject, ObjectDomain::new(ws.path().to_path_buf()))
        .unwrap();

    std::fs::write(ws.path().join("pre.txt"), "changed-in-window").unwrap();
    std::fs::write(ws.path().join("mid.txt"), "created-in-window").unwrap();

    let report = window.verify();
    assert_eq!(
        facts(&report),
        [
            (ObjectAction::Create, "mid.txt".to_string()),
            (ObjectAction::Modify, "pre.txt".to_string()),
        ]
    );

    std::fs::write(ws.path().join("post.txt"), "after").unwrap();
    assert_eq!(
        report.changes().len(),
        2,
        "核验后的变更不得倒灌进已关的窗口"
    );
}

/// 并发安全: 共享判定器 + 不相交对象域并发执行 —— 各窗各账, 零串扰, 审计
/// 一行不丢 (快照即值, 判定器无共享可变态)。
#[test]
fn concurrent_object_judgments_stay_independent_and_keep_every_audit_row() {
    let judge = ObjectAccessJudge::with_defaults();
    let judge_ref = &judge;
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let ws = tempfile::tempdir().unwrap();
                std::fs::write(ws.path().join(".env"), "SECRET=1").unwrap();
                let domain = ObjectDomain::new(ws.path().to_path_buf());
                let subject = ObjectSubject::new(format!("concurrent-process-{i}"));
                scope.spawn(move || {
                    let ((), report) = judge_ref
                        .guard(subject, domain, || {
                            std::fs::remove_file(ws.path().join(".env")).unwrap();
                        })
                        .unwrap();
                    (ws, report)
                })
            })
            .collect();

        let mut expected_objects = Vec::new();
        for handle in handles {
            let (ws, report) = handle.join().expect("并发判定不得 panic");
            assert_eq!(report.violations().len(), 1, "{report:?}");
            let violation = &report.violations()[0];
            assert!(
                violation.object.starts_with(ws.path()),
                "并发窗口不得串扰: {:?}",
                violation.object
            );
            expected_objects.push(violation.object.clone());
        }

        let rows = judge.audit_rows();
        assert_eq!(rows.len(), 4, "每条违规一行审计, 不丢行: {rows:?}");
        for object in &expected_objects {
            let needle = object.display().to_string();
            assert!(
                rows.iter().any(|row| row.contains(&needle)),
                "审计行必须逐条可对账: {needle} in {rows:?}"
            );
        }
    });
}

// ---------------------------------------------------------------------------
// 3. 形态无关: 同对象同判定, API 形态不同结果一致
// ---------------------------------------------------------------------------

/// 形态无关断言: 同一对象同一动作, 分别经档案 API / 句柄 API / 子进程命令面
/// 落地 —— 对象判定 (事实集 + 裁决) 逐一相同。
#[test]
fn same_object_same_verdict_across_api_forms() {
    use std::io::Write as _;

    let ws = tempfile::tempdir().unwrap();
    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("form-agnostic-process");
    let domain = ObjectDomain::new(ws.path().to_path_buf());

    // 创建: 形态 A = 档案 API 整写; 形态 B = 句柄 API 落字。
    let create_a = judge
        .guard(subject.clone(), domain.clone(), || {
            std::fs::write(ws.path().join("probe.txt"), "v1").unwrap();
        })
        .unwrap()
        .1;
    std::fs::remove_file(ws.path().join("probe.txt")).unwrap();
    let create_b = judge
        .guard(subject.clone(), domain.clone(), || {
            let mut file = std::fs::File::create(ws.path().join("probe.txt")).unwrap();
            file.write_all(b"v1").unwrap();
        })
        .unwrap()
        .1;
    assert_eq!(
        create_a, create_b,
        "同一对象同一创建事实, 形态不同判定必须逐一相同"
    );
    assert_eq!(create_a.changes()[0].action, ObjectAction::Create);

    // 修改: 形态 A = 整写; 形态 B = 开档截断写。
    let modify_a = judge
        .guard(subject.clone(), domain.clone(), || {
            std::fs::write(ws.path().join("probe.txt"), "v2-longer").unwrap();
        })
        .unwrap()
        .1;
    std::fs::write(ws.path().join("probe.txt"), "v1").unwrap();
    let modify_b = judge
        .guard(subject.clone(), domain.clone(), || {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(ws.path().join("probe.txt"))
                .unwrap();
            file.write_all(b"v2-longer").unwrap();
        })
        .unwrap()
        .1;
    assert_eq!(
        modify_a, modify_b,
        "同一对象同一修改事实, 形态不同判定必须一致"
    );
    assert_eq!(modify_a.changes()[0].action, ObjectAction::Modify);

    // 删除: 形态 A = 进程内档案 API; 形态 B = 子进程命令面 (形态无关对照)。
    std::fs::write(ws.path().join("victim.txt"), "v1").unwrap();
    let delete_a = judge
        .guard(subject.clone(), domain.clone(), || {
            std::fs::remove_file(ws.path().join("victim.txt")).unwrap();
        })
        .unwrap()
        .1;
    std::fs::write(ws.path().join("victim.txt"), "v1").unwrap();
    let delete_b = judge
        .guard(subject, domain, || {
            #[cfg(windows)]
            let removed = std::process::Command::new("cmd")
                .args(["/C", "del", "victim.txt"])
                .current_dir(ws.path())
                .status()
                .unwrap()
                .success();
            #[cfg(not(windows))]
            let removed = std::process::Command::new("rm")
                .arg("victim.txt")
                .current_dir(ws.path())
                .status()
                .unwrap()
                .success();
            assert!(removed, "子进程删除夹具必须成功");
        })
        .unwrap()
        .1;
    assert_eq!(
        delete_a, delete_b,
        "同一对象同一删除事实, 进程内外形态判定必须一致"
    );
    assert_eq!(delete_a.changes()[0].action, ObjectAction::Delete);
}

// ---------------------------------------------------------------------------
// 4-5. 词法放行但对象越界的兜底 (词法面明示漏放边界: 编码负载)
// ---------------------------------------------------------------------------

/// `powershell -EncodedCommand` 载荷 `[System.IO.File]::Delete('.env')` 的
/// UTF-16LE base64 (词法面看不到成员调用形态)。
#[cfg(windows)]
const ENCODED_DELETE_ENV: &str =
    "WwBTAHkAcwB0AGUAbQAuAEkATwAuAEYAaQBsAGUAXQA6ADoARABlAGwAZQB0AGUAKAAnAC4AZQBuAHYAJwApAA==";

/// 同形态载荷 `[System.IO.File]::WriteAllText('probe.txt','ok')`。
#[cfg(windows)]
const ENCODED_WRITE_PROBE: &str =
    "WwBTAHkAcwB0AGUAbQAuAEkATwAuAEYAaQBsAGUAXQA6ADoAVwByAGkAdABlAEEAbABsAFQAZQB4AHQAKAAnAHAAcgBvAGIAZQAuAHQAeAB0ACcALAAnAG8AawAnACkA";

/// 词法放行 (含守门放行) 但对象越界 → 对象判定兜底: 失败即帧 + 审计行
/// (对象/动作/进程)。这是「换 API 换权力」的实弹兜底用例: 编码负载是词法面
/// 注释里明示的漏放边界, 删除的还是敏感对象 `.env`。
#[cfg(windows)]
#[tokio::test]
async fn lexical_pass_but_object_violation_is_fenced_with_frame_and_audit() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join(".env"), "SECRET=1").unwrap();
    let command =
        format!("powershell -NoProfile -NonInteractive -EncodedCommand {ENCODED_DELETE_ENV}");

    // 词法面 + 守门面都放行 (漏放边界实证) —— 对象判定是唯一兜底。
    assert!(
        scan_shell_write_intent(&command).is_none(),
        "词法面必须放行该漏放形态, 才是兜底实证: {command}"
    );
    assert!(
        ToolGuardrail::verify_shell_command(&command).is_ok(),
        "守门面必须放行该漏放形态: {command}"
    );

    let tool = ShellTool::new(TrustedShellConfig::new(ws.path().to_path_buf()));
    assert!(!tool.config().file_write, "总闸关: 本例正是词法漏放形态");
    let result = invoke_shell(&tool, command).await;
    assert!(!result.is_ok(), "对象越界必须失败即帧: {}", result.render());

    let rendered = result.render();
    assert!(rendered.contains("pipeline.post_block"), "{rendered}");
    assert!(rendered.contains("object violation"), "{rendered}");
    assert!(rendered.contains(".env"), "{rendered}");
    assert!(rendered.contains("action=delete"), "{rendered}");
    assert!(rendered.contains("process="), "{rendered}");

    let rows = tool.object_judge().audit_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(
        rows[0].starts_with(OBJECT_VIOLATION_AUDIT_EVENT),
        "审计行事件名 = {} : {}",
        OBJECT_VIOLATION_AUDIT_EVENT,
        rows[0]
    );
    assert!(rows[0].contains("object="), "{}", rows[0]);
    assert!(rows[0].contains("action=delete"), "{}", rows[0]);
    assert!(rows[0].contains("process="), "{}", rows[0]);
}

/// 对照面: 同一漏放形态下的**合规**对象变更 (域内普通对象) 结果零变化 ——
/// 对象判定只拦越界, 不误伤; 对象审计计数随结果可见。
#[cfg(windows)]
#[tokio::test]
async fn compliant_lexical_pass_keeps_the_result_and_the_object_audit_wall() {
    let ws = tempfile::tempdir().unwrap();
    let command =
        format!("powershell -NoProfile -NonInteractive -EncodedCommand {ENCODED_WRITE_PROBE}");
    assert!(scan_shell_write_intent(&command).is_none(), "{command}");

    let tool = ShellTool::new(TrustedShellConfig::new(ws.path().to_path_buf()));
    let result = invoke_shell(&tool, command).await;
    assert!(result.is_ok(), "{}", result.render());
    assert!(ws.path().join("probe.txt").exists(), "对象事实: 已创建");

    let value = match result.outcome {
        ToolOutcome::Ok { value } => value,
        other => panic!("expected ok outcome, got {other:?}"),
    };
    assert_eq!(value["object_audit"]["changes"], json!(1), "{value}");
    assert_eq!(value["object_audit"]["violations"], json!(0), "{value}");
    assert_eq!(
        value["object_audit"]["coverage"],
        json!("complete"),
        "{value}"
    );
    assert!(
        tool.object_judge().audit_rows().is_empty(),
        "合规执行零审计行: {:?}",
        tool.object_judge().audit_rows()
    );
}

// ---------------------------------------------------------------------------
// 6. 审计行: 对象/动作/进程/域/理由
// ---------------------------------------------------------------------------

/// 越界即审计: `sandbox.object_violation` 行逐项带对象/动作/进程/域/理由,
/// 失败帧走闭合词表 (`pipeline.post_block`) 且理由串同样可对账。
#[test]
fn violation_audit_rows_name_object_action_and_process() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join(".env"), "SECRET=1").unwrap();

    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("test-process.exe");
    let ((), report) = judge
        .guard(subject, ObjectDomain::new(ws.path().to_path_buf()), || {
            std::fs::remove_file(ws.path().join(".env")).unwrap();
        })
        .unwrap();

    assert!(report.is_violation());
    assert_eq!(report.violations().len(), 1);
    let violation: &ObjectViolation = &report.violations()[0];
    assert_eq!(violation.action, ObjectAction::Delete);
    assert_eq!(violation.zone, ObjectZone::Sensitive);
    assert_eq!(violation.process, "test-process.exe");
    assert!(violation.object.ends_with(".env"), "{violation:?}");

    let rows = judge.audit_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(
        rows[0].starts_with(OBJECT_VIOLATION_AUDIT_EVENT),
        "{}",
        rows[0]
    );
    assert!(rows[0].contains("object="), "{}", rows[0]);
    assert!(rows[0].contains("action=delete"), "{}", rows[0]);
    assert!(rows[0].contains("process=test-process.exe"), "{}", rows[0]);
    assert!(rows[0].contains("zone=sensitive"), "{}", rows[0]);
    assert!(rows[0].contains("reason="), "{}", rows[0]);

    let failure: PipelineFailure = report.failure().expect("违规必须出帧");
    assert_eq!(failure.code(), "pipeline.post_block");
    let message = failure.message();
    assert!(message.contains("object violation"), "{message}");
    assert!(message.contains("action=delete"), "{message}");
    assert!(message.contains("process=test-process.exe"), "{message}");
}

// ---------------------------------------------------------------------------
// 7. 契约挂载点: fake policy 接入 + 策略链
// ---------------------------------------------------------------------------

/// 挂载点实证: 假策略在请求面与事实面都被逐次征询、其裁决生效; 策略链
/// 第一个拒绝即拒绝 (叠加不覆盖)。
#[test]
fn fake_policy_mount_decides_and_chain_keeps_first_deny_wins() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join("locked.txt"), "v1").unwrap();

    let policy = Arc::new(RecordingPolicy {
        deny_object: "locked.txt",
        calls: Mutex::new(0),
    });
    let judge = ObjectAccessJudge::new(
        Arc::clone(&policy) as Arc<dyn ObjectAccessPolicy>,
        Arc::new(FsObjectStateProbe::new()),
        Arc::new(MemoryObjectAuditSink::new()),
    );
    let subject = ObjectSubject::new("fake-mount-process");
    let domain = ObjectDomain::new(ws.path().to_path_buf());

    // 请求面: 每次判定都征询假策略。
    assert!(judge
        .check(&domain.request(&subject, &ws.path().join("free.txt"), ObjectAction::Modify))
        .is_allow());
    assert!(!judge
        .check(&domain.request(
            &subject,
            &ws.path().join("locked.txt"),
            ObjectAction::Modify
        ))
        .is_allow());
    assert_eq!(
        *policy
            .calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        2,
        "挂载点必须逐次征询策略"
    );

    // 事实面: 窗口内动了假策略的禁改对象 → 违规理由是策略原话。
    let ((), report) = judge
        .guard(subject.clone(), domain.clone(), || {
            std::fs::write(ws.path().join("locked.txt"), "v2-longer").unwrap();
        })
        .unwrap();
    assert_eq!(report.violations().len(), 1, "{report:?}");
    assert_eq!(
        report.violations()[0].reason,
        "fake policy forbids this object",
        "违规理由必须是策略原话"
    );
    assert_eq!(judge.audit_rows().len(), 1);

    // 策略链: 全放行才放行, 任一拒绝即拒绝 (两种顺序都拒)。
    let allow = Arc::new(AlwaysAllow) as Arc<dyn ObjectAccessPolicy>;
    let deny = Arc::new(AlwaysDeny) as Arc<dyn ObjectAccessPolicy>;
    let free = ws.path().join("free.txt");
    let request = domain.request(&subject, &free, ObjectAction::Enumerate);
    assert!(ObjectPolicyChain::new(vec![Arc::clone(&allow)])
        .decide(&request)
        .is_allow());
    assert!(
        !ObjectPolicyChain::new(vec![Arc::clone(&allow), Arc::clone(&deny)])
            .decide(&request)
            .is_allow()
    );
    assert!(
        !ObjectPolicyChain::new(vec![Arc::clone(&deny), Arc::clone(&allow)])
            .decide(&request)
            .is_allow()
    );
}

/// 生产面挂载点: shell 执行面挂自定义判定器后按其对象策略判定 —— 同一条
/// 命令默认判定器合规放行, 挂载禁改对象后失败即帧 + 审计。
#[cfg(windows)]
#[tokio::test]
async fn shell_mounted_object_policy_replaces_the_default_face() {
    let ws = tempfile::tempdir().unwrap();
    let command =
        format!("powershell -NoProfile -NonInteractive -EncodedCommand {ENCODED_WRITE_PROBE}");

    let judge = Arc::new(ObjectAccessJudge::new(
        Arc::new(RecordingPolicy {
            deny_object: "probe.txt",
            calls: Mutex::new(0),
        }),
        Arc::new(FsObjectStateProbe::new()),
        Arc::new(MemoryObjectAuditSink::new()),
    ));
    let mounted = ShellTool::new(
        TrustedShellConfig::new(ws.path().to_path_buf()).with_object_judgment(Arc::clone(&judge)),
    );
    let result = invoke_shell(&mounted, command.clone()).await;
    assert!(
        !result.is_ok(),
        "挂载策略禁改对象必须拦下: {}",
        result.render()
    );
    assert!(
        result.render().contains("object violation"),
        "{}",
        result.render()
    );
    assert_eq!(judge.audit_rows().len(), 1, "{:?}", judge.audit_rows());

    // 对照: 同命令同形态, 默认判定器下合规放行 (零帧零审计行)。
    let ws_default = tempfile::tempdir().unwrap();
    let default_tool = ShellTool::new(TrustedShellConfig::new(ws_default.path().to_path_buf()));
    let ok = invoke_shell(&default_tool, command).await;
    assert!(ok.is_ok(), "{}", ok.render());
    assert!(default_tool.object_judge().audit_rows().is_empty());
}

// ---------------------------------------------------------------------------
// 10. 枚举: 请求时判定 + 零事实痕迹 (如实边界)
// ---------------------------------------------------------------------------

/// 枚举四动作之一: 走请求时判定面 (敏感/域外拒、域内放); 事实面零痕迹 ——
/// 只读执行不动对象, 差分为空、不判失败 (如实边界, 不假装事实判定能看见枚举)。
#[test]
fn enumerate_is_judged_on_the_request_face_and_leaves_no_fact_trace() {
    let ws = tempfile::tempdir().unwrap();
    std::fs::write(ws.path().join(".env"), "SECRET=1").unwrap();
    std::fs::write(ws.path().join("notes.txt"), "n").unwrap();

    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("enumerate-process");
    let domain = ObjectDomain::new(ws.path().to_path_buf());

    assert!(
        !judge
            .check(&domain.request(&subject, &ws.path().join(".env"), ObjectAction::Enumerate))
            .is_allow(),
        "敏感面枚举必须拒"
    );
    assert!(
        !judge
            .check(&domain.request(
                &subject,
                &ws.path().parent().unwrap().join("out.txt"),
                ObjectAction::Enumerate
            ))
            .is_allow(),
        "域外枚举必须拒"
    );
    assert!(judge
        .check(&domain.request(
            &subject,
            &ws.path().join("notes.txt"),
            ObjectAction::Enumerate
        ))
        .is_allow());

    let (entries, report) = judge
        .guard(subject, domain, || {
            std::fs::read_dir(ws.path()).unwrap().count()
        })
        .unwrap();
    assert!(entries > 0);
    assert!(report.changes().is_empty(), "枚举零状态痕迹: {report:?}");
    assert!(!report.is_failure());
}

// ---------------------------------------------------------------------------
// 12. fail-closed: 判定窗开不出 = 执行不启动
// ---------------------------------------------------------------------------

/// 判定窗开不出 (域根不可读) → fail-closed: 受控执行**不启动**, 走
/// [`ObjectGuardError`] 通道报真因, 不静默降级。
#[test]
fn unobservable_domain_fails_closed_without_running_the_payload() {
    let base = tempfile::tempdir().unwrap();
    let gone = base.path().join("never-created");
    let judge = ObjectAccessJudge::with_defaults();
    let subject = ObjectSubject::new("fail-closed-process");

    let ran = std::cell::Cell::new(false);
    let outcome = judge.guard(subject, ObjectDomain::new(gone), || ran.set(true));
    assert!(
        matches!(outcome, Err(ObjectGuardError::Unobservable(_))),
        "判定窗开不出必须 fail-closed: {outcome:?}"
    );
    assert!(!ran.get(), "判定窗开不出 = 受控执行不得启动");
}

// ---------------------------------------------------------------------------
// 纯路径形状对照 (域界判定的词法口径)
// ---------------------------------------------------------------------------

/// 域界判定: 拼写对不上域根的形态按域外拒 (fail-closed), 不猜、不放;
/// 词法折叠后回落域内的形态按域内判 (与观测门禁同一归一口径)。
#[test]
fn domain_boundary_misspellings_fail_closed_to_outside() {
    let domain = ObjectDomain::new(Path::new("root"));
    assert_eq!(
        domain.zone_of(Path::new("rootish/notes.txt")),
        ObjectZone::OutsideWorkspace
    );
    assert_eq!(
        domain.zone_of(Path::new("root/../escape.txt")),
        ObjectZone::OutsideWorkspace,
        "折叠后落到域外的对象按域外拒"
    );
    assert_eq!(
        domain.zone_of(Path::new("root/sub/../notes.txt")),
        ObjectZone::InsideWorkspace,
        "折叠后回落域内的对象按域内判 (词法归一口径, 与观测门禁一致)"
    );
}
