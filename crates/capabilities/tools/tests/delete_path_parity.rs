//! 删除/写入路径同权对称性测试。
//!
//! 同一条 shell 通道里同一操作的两条 API 路径 —— 命令面族词/等值命令面
//! （如 `Remove-Item` / `del`）与脚本引擎静态调用面（如
//! `[System.IO.File]::Delete('x')` / `[System.IO.Directory]::Delete('x')` /
//! `([System.IO.FileInfo]'x').Delete()` / `[System.IO.File]::WriteAllText(...)`，
//! PowerShell/.NET 成员调用形态）—— 必须同族同权，在三层逐一相等：
//!
//! (a) 写意图扫描：命中与否 + 命中面族名逐字相同；
//! (b) 总闸关（默认 fail-closed）：shell 面拒绝即帧 —— 同一 `pipeline.pre_deny`
//!     帧、同一总闸文案、同一命中族名；freeze→execute 时差复核同理；
//! (c) 总闸开：冻结形状逐一相同（载荷仅脚本文本不同），审批档判定
//!     （规则引擎风险档 / 命令族守门 / 生产能力级审批）逐一相同。

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    parse_approval_entry, ApprovalPolicyEngine, Permission, PermissionPolicy,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::{
    scan_shell_write_intent, CommandFamilyGateHook, PreExecuteHook, PreExecuteRequest, PreVerdict,
    RiskLevelGateHook, ShellTool, ToolGuardrail, TrustedShellConfig,
};
use serde_json::json;

/// 对称性对照表：(命令面基线, 静态调用面变体, 期望命中族名)。
/// 基线取命令面族词/等值命令面，变体取同一操作的静态/实例成员调用形态。
const PARITY_PAIRS: &[(&str, &str, &str)] = &[
    // 删除族：两条删除路径必须同族同权。
    (
        "Remove-Item x.txt",
        "[System.IO.File]::Delete('x.txt')",
        "删除族命令",
    ),
    (
        "del x.txt",
        "[System.IO.Directory]::Delete('x.txt')",
        "删除族命令",
    ),
    (
        "rd x_dir",
        "([System.IO.FileInfo]'x_dir').Delete()",
        "删除族命令",
    ),
    (
        "rmdir x_dir",
        "[System.IO.Directory]::Delete('x_dir', $true)",
        "删除族命令",
    ),
    (
        "rm x.txt",
        "[System.IO.FileInfo]::new('x.txt').Delete()",
        "删除族命令",
    ),
    // 文件写入族。
    (
        "Set-Content x.txt hello",
        "[System.IO.File]::WriteAllText('x.txt','hello')",
        "文件写入族命令",
    ),
    (
        "Add-Content x.txt hello",
        "[System.IO.File]::AppendAllText('x.txt','hello')",
        "文件写入族命令",
    ),
    (
        "tee x.txt",
        "[System.IO.File]::WriteAllBytes('x.txt', $bytes)",
        "文件写入族命令",
    ),
    (
        "Out-File x.txt",
        "[System.IO.File]::Replace('x.txt','y.txt','z.txt')",
        "文件写入族命令",
    ),
    (
        "Clear-Content x.txt",
        "[System.IO.File]::SetAttributes('x.txt','Hidden')",
        "文件写入族命令",
    ),
    // 建目录/建文件族。
    (
        "mkdir x_dir",
        "[System.IO.Directory]::Create('x_dir')",
        "建目录/建文件族命令",
    ),
    (
        "New-Item x.txt",
        "[System.IO.File]::Create('x.txt')",
        "建目录/建文件族命令",
    ),
    // 移动/复制/改名族。
    (
        "Move-Item a.txt b.txt",
        "[System.IO.File]::Move('a.txt','b.txt')",
        "移动/复制/改名族命令",
    ),
    (
        "Copy-Item a.txt b.txt",
        "[System.IO.File]::Copy('a.txt','b.txt')",
        "移动/复制/改名族命令",
    ),
];

/// 只读对照表：命令面读法与静态调用面读成员（`Read*`/`Get*`/`Exists`/`Open*`）。
const READ_PAIRS: &[(&str, &str)] = &[
    ("type x.txt", "[System.IO.File]::ReadAllText('x.txt')"),
    ("dir", "[System.IO.Directory]::GetFiles('x_dir')"),
    (
        "findstr hi x.txt",
        "([System.IO.FileInfo]'x.txt').OpenText()",
    ),
    ("echo hi", "[System.IO.File]::Exists('x.txt')"),
];

/// 命中面族名（命中面文案 = `族名 空格 引号命中词`，族名不含空格）。
fn family_label(surface: &str) -> &str {
    surface.split(' ').next().unwrap_or(surface)
}

fn call_for(command: &str) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: "shell".into(),
        arguments: json!({ "command": command }),
    }
}

/// 总闸关：冻结必须拒绝即帧，返回拒绝帧渲染文本。
fn gate_closed_freeze_err(tool: &ShellTool, command: &str) -> String {
    match tool.freeze_invocation(&call_for(command)) {
        Err(result) => result.render(),
        Ok(_) => panic!("{command} 总闸关时写意图必须拒绝即帧"),
    }
}

/// 总闸开：冻结载荷（脚本逐字携带）。
fn gate_open_frozen_payload(tool: &ShellTool, command: &str) -> serde_json::Value {
    tool.freeze_invocation(&call_for(command))
        .expect("总闸开时冻结必须放行（行为不变）")
        .expect("总闸开时冻结必须有载荷")
        .payload
}

/// 风险档规则引擎判定（每次全新钩子：调用频度历史不跨对照污染）。
fn risk_verdict(
    engine: &ApprovalPolicyEngine,
    capability: &CapabilityId,
    command: &str,
) -> PreVerdict {
    let hook = RiskLevelGateHook::new(engine.clone());
    hook.pre_verdict(&PreExecuteRequest::new(&call_for(command), capability))
}

/// 命令族守门层判定。
fn command_family_verdict(capability: &CapabilityId, command: &str) -> PreVerdict {
    CommandFamilyGateHook::new()
        .pre_verdict(&PreExecuteRequest::new(&call_for(command), capability))
}

/// (a) 写意图扫描：两条路径命中面族名逐一相同，且等于期望族名。
#[test]
fn write_intent_scan_is_symmetric_across_command_and_static_call_forms() {
    for &(baseline, variant, expected) in PARITY_PAIRS {
        let base_hit =
            scan_shell_write_intent(baseline).unwrap_or_else(|| panic!("{baseline} 必须判写意图"));
        let variant_hit =
            scan_shell_write_intent(variant).unwrap_or_else(|| panic!("{variant} 必须判写意图"));
        assert_eq!(
            family_label(&base_hit.surface),
            expected,
            "{baseline}: {}",
            base_hit.surface
        );
        assert_eq!(
            family_label(&variant_hit.surface),
            expected,
            "{variant}: {}",
            variant_hit.surface
        );
    }
}

/// (a) 读侧对称：只读命令与只读成员调用都不判写意图，且在总闸关的 shell
/// 面照常冻结（读命令零误伤）。
#[test]
fn read_only_member_calls_stay_out_of_scan_and_gate() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
    assert!(!tool.config().file_write, "文件写入总闸默认必须关");
    for &(baseline, variant) in READ_PAIRS {
        for command in [baseline, variant] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 只读命令/只读成员调用不得判写意图"
            );
            assert!(
                tool.freeze_invocation(&call_for(command))
                    .unwrap()
                    .is_some(),
                "{command} 只读命令必须照常冻结"
            );
        }
    }
}

/// (b) 总闸关：两条路径在 shell 面拒绝即帧逐一相同 —— 同一 `pipeline.pre_deny`
/// 帧、同一总闸文案、同一命中族名；拒绝即帧 = 命令未执行，文件不得被删。
#[test]
fn gate_closed_pre_deny_frame_is_symmetric_across_command_and_static_call_forms() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("x.txt"), "keep").unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
    assert!(!tool.config().file_write, "文件写入总闸默认必须关");

    for &(baseline, variant, expected) in PARITY_PAIRS {
        let base_render = gate_closed_freeze_err(&tool, baseline);
        let variant_render = gate_closed_freeze_err(&tool, variant);
        for render in [&base_render, &variant_render] {
            assert!(render.contains("pipeline.pre_deny"), "{render}");
            assert!(
                render.contains("文件写入开关未开——shell 写命令受同一总闸管辖"),
                "{render}"
            );
        }
        assert!(base_render.contains(expected), "{baseline}: {base_render}");
        assert!(
            variant_render.contains(expected),
            "{variant}: {variant_render}"
        );
    }
    assert!(
        tmp.path().join("x.txt").exists(),
        "拒绝即帧 = 命令未执行, 文件不得被删"
    );
}

/// (b) freeze→execute 时差复核：开闸冻结的两条路径载荷，关闸后执行面拒绝
/// 即帧逐一相同（fail-closed），文件不得被删。
#[tokio::test]
async fn gate_close_recheck_is_symmetric_for_frozen_payloads_of_both_forms() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("x.txt"), "keep").unwrap();
    let open_tool =
        ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()).with_file_write(true));
    let closed_tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    for &(baseline, variant, _expected) in PARITY_PAIRS {
        for command in [baseline, variant] {
            let call = call_for(command);
            let frozen = open_tool.freeze_invocation(&call).unwrap().unwrap();
            let result = closed_tool.invoke_frozen(&call, Some(&frozen)).await;
            assert!(!result.is_ok(), "{command} 关闸后的冻结载荷必须拒绝");
            let rendered = result.render();
            assert!(rendered.contains("pipeline.pre_deny"), "{rendered}");
            assert!(
                rendered.contains("文件写入开关未开——shell 写命令受同一总闸管辖"),
                "{rendered}"
            );
        }
    }
    assert!(
        tmp.path().join("x.txt").exists(),
        "拒绝即帧 = 命令未执行, 文件不得被删"
    );
}

/// (c) 总闸开：两条路径的冻结形状逐一相同（载荷仅脚本文本不同），同一能力
/// 面同一审批链入口；脚本逐字携带。
#[test]
fn gate_open_freeze_shape_is_symmetric_across_command_and_static_call_forms() {
    let tmp = tempfile::tempdir().unwrap();
    let tool =
        ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()).with_file_write(true));
    assert!(tool.config().file_write);

    for &(baseline, variant, _expected) in PARITY_PAIRS {
        let base_payload = gate_open_frozen_payload(&tool, baseline);
        let variant_payload = gate_open_frozen_payload(&tool, variant);
        for key in [
            "version",
            "shell_executable",
            "workspace_root",
            "cwd",
            "timeout_ms",
            "max_stdout_bytes",
            "max_stderr_bytes",
            "environment",
            "isolation",
        ] {
            assert_eq!(
                base_payload[key], variant_payload[key],
                "{baseline} vs {variant}: 冻结字段 {key} 必须逐一相同"
            );
        }
        for (payload, command) in [(&base_payload, baseline), (&variant_payload, variant)] {
            let script = payload["shell_args"]
                .as_array()
                .and_then(|args| args.last())
                .and_then(|arg| arg.as_str())
                .unwrap_or_default();
            assert_eq!(script, command, "冻结载荷原样携带脚本");
        }
    }
}

/// (c) 总闸开：审批档判定逐一相同 —— 同一策略下两条路径在风险档规则引擎
/// 与命令族守门两层拿到逐一相同的判定（普通删除/写入族同为放行至审批链档）。
#[test]
fn gate_open_approval_tier_is_symmetric_across_command_and_static_call_forms() {
    let capability = CapabilityId::new("tool.shell").unwrap();

    let mut tool_level_approval = ApprovalPolicyEngine::new();
    tool_level_approval
        .approval_list
        .push(parse_approval_entry("tool.shell").unwrap());

    let mut blacklist = ApprovalPolicyEngine::new();
    blacklist.blacklist.insert("tool.shell".to_string());

    let policies = [
        ("default", ApprovalPolicyEngine::new()),
        ("tool_level_approval", tool_level_approval),
        ("blacklist", blacklist),
    ];

    for &(baseline, variant, _expected) in PARITY_PAIRS {
        for (policy_name, policy) in &policies {
            let base_verdict = risk_verdict(policy, &capability, baseline);
            let variant_verdict = risk_verdict(policy, &capability, variant);
            assert_eq!(
                base_verdict, variant_verdict,
                "{baseline} vs {variant} 在 {policy_name} 策略下审批档判定必须逐一相同"
            );
        }
        let base_family = command_family_verdict(&capability, baseline);
        let variant_family = command_family_verdict(&capability, variant);
        assert_eq!(
            base_family, variant_family,
            "{baseline} vs {variant} 命令族守门判定必须逐一相同"
        );
    }
}

/// (c) 命令族守门（guardrail 破坏面）同档锁定：普通删除/写入族两条路径在
/// 守门层同为放行档（硬拒只留给灾难字面量与系统配置写动词）；灾难作用域的
/// 两条 API 路径也同档 —— 灾难字面量窄清单不扩面，同权由写意图总闸 +
/// shell 审批链统一兜住。
#[test]
fn guardrail_verdicts_are_symmetric_across_command_and_static_call_forms() {
    for &(baseline, variant, _expected) in PARITY_PAIRS {
        assert_eq!(
            ToolGuardrail::verify_shell_command(baseline),
            ToolGuardrail::verify_shell_command(variant),
            "{baseline} vs {variant} 在守门层必须同档"
        );
        assert!(
            ToolGuardrail::verify_shell_command(baseline).is_ok(),
            "{baseline} 普通删除/写入族应放行至审批链"
        );
    }
    // 灾难作用域的两条 API 路径同档（同为放行至审批链，由审批人与沙箱兜底）。
    assert_eq!(
        ToolGuardrail::verify_shell_command("Remove-Item -Recurse -Force C:\\"),
        ToolGuardrail::verify_shell_command("[System.IO.Directory]::Delete('C:\\', $true)"),
    );
}

/// (c) 生产审批档（能力级授权面）：shell 以能力为单位要求人工审批，与命令
/// 文本无关 —— 两条删除路径在生产档结构性同档。
#[test]
fn production_capability_level_approval_is_symmetric_for_both_forms() {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.shell".to_string()));
    policy.require_approval_for("tool.shell");

    for &(baseline, variant, _expected) in PARITY_PAIRS {
        let base = PreVerdict::from(policy.decision_for_capability("tool.shell"));
        let variant_verdict = PreVerdict::from(policy.decision_for_capability("tool.shell"));
        assert_eq!(
            base, variant_verdict,
            "{baseline} vs {variant} 生产审批档判定必须逐一相同"
        );
        assert!(
            matches!(base, PreVerdict::Ask { .. }),
            "{baseline} 生产档应停在人工审批"
        );
    }
}

/// 边界实证：规则引擎的**命令级**审批名单是逐字精确匹配（该引擎既有语义），
/// 按 API 形态逐一登记是运维口径；生产接线走能力级（见上一测试），两条删除
/// 路径在生产档逐一相同。命令面族归一属规则引擎层（governance）语义变更，
/// 不在本层改动面内 —— 此处把边界钉在测试上，改口径必改本测试。
#[test]
fn command_level_approval_entries_stay_exact_text_boundary() {
    let capability = CapabilityId::new("tool.shell").unwrap();
    let mut engine = ApprovalPolicyEngine::new();
    engine
        .approval_list
        .push(parse_approval_entry("tool.shell:Remove-Item x.txt").unwrap());

    let pinned = risk_verdict(&engine, &capability, "Remove-Item x.txt");
    let twin = risk_verdict(&engine, &capability, "[System.IO.File]::Delete('x.txt')");
    assert!(
        matches!(&pinned, PreVerdict::Ask { reason } if reason.contains("approval list matched")),
        "{pinned:?}"
    );
    assert_eq!(
        twin,
        PreVerdict::Allow,
        "未逐字登记的孪生形态不命中命令级名单（精确匹配边界）"
    );
}
