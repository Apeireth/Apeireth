#![no_main]

//! fuzz: **git 写分级面** (`tool.git_write`) —— 参数注入 / 路径逃逸 / 危险动作越权。
//!
//! 面的选择依据 (`docs/02-guides/fuzz-target-authoring.md` §1):
//! 1. **吃不受控输入**: `command` 文本由模型产出 (可被提示注入操纵) → 是;
//! 2. **有强承诺**: 分级词表 / 变形防护 / 拒绝即帧 / L2 永人批 → 是;
//! 3. **接线了吗**: 生产工具面 (`tool.git_write`, 干活能力轴 P1) → 是。
//!
//! # 被 fuzz 的 API (全是产品公开面, 无一自造)
//!
//! | API | 契约出处 (`crates/capabilities/tools/src/git_write.rs`) |
//! |-----|------------------------------------------------------|
//! | `classify_git_write` | 模块文档 §1-§5 (L1-§5) + L179-214 |
//! | `git_write_plan_for_arguments` | L216-223 |
//! | `git_write_human_approval_floor` | L225-238 |
//! | `GitWriteBoundaryGuard::deny` | L1207-1240 |
//! | `GitWriteTierApprovalHook::evaluate` | L1242-1309 |
//! | `GitWriteTool::invoke` | L1108-1151 (拒绝即帧必须落到工具面) |
//!
//! # 断言纪律 (本项目历史 8 次「自造期望 = 假阳性」的教训)
//!
//! 每条断言都从被测函数**自己写明的契约**推出, 出处写在断言旁。本 target
//! **不发明期望值**: 期望值只有三种来源 ——
//! (a) 模块文档 / 交付记录 (`docs/04-internal/backend-enhancement/04-delivery/
//! git-write-tiering-delivery.md` §2 分级词表全表) 的原文;
//! (b) 被测文件里的**稳定前缀常量**与**分级词表表**;
//! (c) 被测函数注释里写明的类别名 (如「交互补丁面」「本工具不开编辑器面」)。
//!
//! # 覆盖面
//!
//! - **参数注入面**: `-` 开头的选项注入 (`--upload-pack=` / `--exec=` / `-c
//!   core.pager=` / 分支名以 `-` 开头)、长选项前缀重叠 (`--ha` ≡ `--hard`)、
//!   短选项束逐字符 (`-qf` 里的 `f`)、同一张脸的**长短形别名** (`-p` vs
//!   `--interactive`、`-c` vs `-e/--edit`)、`--` 分隔缺失;
//! - **路径面**: `../`、绝对路径、`.git` 内路径 (`.git/config`、`.git/hooks/`)
//!   作为**槽位载荷**进入分级面;
//! - **语义面**: 空提交信息、只有空白、超长提交信息、含换行/制表符/NUL 的
//!   提交信息与分支名、空分支名、Unicode;
//! - **shell 面**: 管道 / 重定向 / `$` / 反引号 / 分组括号 / 单 `&`。
//!
//! # 已知缺陷 (本 target 钉住, 产品侧已修)
//!
//! `git commit` 的两张脸存在**长短形别名绕过**:
//! - `-e` / `--edit` 打开编辑器 —— 违反「本工具不开编辑器面」(L504 / 交付记录 §2);
//! - `--interactive` 是 `-p` (交互补丁面) 的长形 —— 短形被拒、长形放行。
//!
//! 见 `crates/capabilities/tools/tests/git_write_grading.rs` 的回归测试。

use std::sync::Arc;
use std::sync::OnceLock;
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::{
    GIT_WRITE_CAPABILITY_ID, GIT_WRITE_TOOL_NAME, GitWriteBoundaryGuard, GitWritePlan,
    GitWriteReleaseTier, GitWriteTier, GitWriteTierApprovalHook, GitWriteTool, GradedSegment,
    REFUSAL_L3_PREFIX, REFUSAL_MORPH_PREFIX, REFUSAL_OFFLIST_PREFIX, classify_git_write,
    git_write_human_approval_floor, git_write_plan_for_arguments,
};
use libfuzzer_sys::fuzz_target;

// ---------------------------------------------------------------------------
// 契约常量 (逐条抄自被测文件, 不自造)
// ---------------------------------------------------------------------------

/// 分级词表内的子命令 (L1 五项 + L2 三项; `git_write.rs` L414-437 的白名单)。
const WHITELIST_SUBCOMMANDS: &[&str] = &[
    "add", "commit", "branch", "switch", "tag", "push", "fetch", "remote",
];

/// 子命令前**唯一**放行的 git 全局面 (模块文档 §2: 「子命令前的 git 全局面只放行
/// `--no-pager`/`--paginate`」, L20-23)。
const ALLOWED_GLOBAL_OPTIONS: &[&str] = &["--no-pager", "--paginate"];

/// 拒绝帧的三类稳定前缀 (L69-76)。
const REFUSAL_PREFIXES: [&str; 3] = [
    REFUSAL_L3_PREFIX,
    REFUSAL_OFFLIST_PREFIX,
    REFUSAL_MORPH_PREFIX,
];

/// 一个子命令的**选项面契约** (交付记录 §2 分级词表全表的「允许形态」与
/// 「升级为更严档 / 拒绝即帧的变体」两列)。
struct OptionFamily {
    /// 词表写明「只认」的长选项基名 (精确匹配, 不含 `=` 之后的部分)。
    allowed_long: &'static [&'static str],
    /// 词表写明「只认」的短选项字符。
    allowed_short: &'static [char],
    /// 值参数短选项 (遇之即止: 余下字符是值, 实现 L482-497)。
    value_shorts: &'static [char],
    /// 值参数长选项 (吃掉下一个 token; 实现里 `index += 1` 的那几处)。
    value_long: &'static [&'static str],
    /// 拒绝族: 长选项 → 帧前缀 (前缀重叠命中, 实现 L350-353)。
    denied_long: &'static [(&'static str, &'static str)],
    /// 拒绝族: 短选项字符 → 帧前缀。
    denied_short: &'static [(char, &'static str)],
    /// **同一张脸的长短形别名**: 必须被拒, 但帧前缀不钉死 (只钉「必须拒」)。
    denied_long_alias: &'static [&'static str],
    /// 同上 (短形)。
    denied_short_alias: &'static [char],
    /// 词表是否**封闭**: 未列选项一律拒 (branch/switch/tag 封闭;
    /// commit/push/fetch/add/remote 不封闭 —— 交付记录 §2 逐行写明)。
    closed: bool,
}

/// `git commit`: L1 可逆, 必带消息, 不开编辑器面 (交付记录 §2 commit 行)。
const COMMIT_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &[
        "--message",
        "--file",
        "--reuse-message",
        "--fixup",
        "--squash",
    ],
    allowed_short: &[],
    value_shorts: &['m', 'F', 'C', 't'],
    value_long: &[
        "--message",
        "--file",
        "--reuse-message",
        "--fixup",
        "--squash",
    ],
    // `--amend` 族 → L3 改史; `--no-verify` → 变形面; `--sign`/`--gpg-sign` → 签名面。
    denied_long: &[
        ("--amend", REFUSAL_L3_PREFIX),
        ("--no-verify", REFUSAL_MORPH_PREFIX),
        ("--sign", REFUSAL_OFFLIST_PREFIX),
        ("--gpg-sign", REFUSAL_OFFLIST_PREFIX),
    ],
    // `n`(--no-verify 绕钩子) / `s`·`S`(签名密钥面) / `c`(开编辑器) / `p`(交互补丁面)。
    denied_short: &[
        ('n', REFUSAL_MORPH_PREFIX),
        ('s', REFUSAL_OFFLIST_PREFIX),
        ('S', REFUSAL_OFFLIST_PREFIX),
        ('c', REFUSAL_OFFLIST_PREFIX),
        ('p', REFUSAL_OFFLIST_PREFIX),
    ],
    // 「本工具不开编辑器面」(L504): `-e`/`--edit` 是同一张脸。
    // 「交互补丁面」(L443-449): `--interactive` 是 `-p` 的长形。
    denied_long_alias: &["--edit", "--interactive"],
    denied_short_alias: &['e'],
    closed: false,
};

/// `git branch` (建): 位置参 1-2 个; 选项只认 `-t/--track/--no-track/-q/--quiet/--`。
const BRANCH_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &["--track", "--no-track", "--quiet"],
    allowed_short: &['t', 'q'],
    value_shorts: &[],
    value_long: &["--track", "--no-track", "--quiet"],
    denied_long: &[
        ("--delete", REFUSAL_L3_PREFIX),
        ("--force", REFUSAL_L3_PREFIX),
    ],
    denied_short: &[
        ('d', REFUSAL_L3_PREFIX),
        ('D', REFUSAL_L3_PREFIX),
        ('f', REFUSAL_L3_PREFIX),
    ],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: true,
};

/// `git switch` (切): 1 个分支名, 或 `-c/--create <new>`。
const SWITCH_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &["--create", "--track", "--no-track", "--quiet"],
    allowed_short: &['t', 'q'],
    value_shorts: &['c'],
    value_long: &["--create"],
    denied_long: &[
        ("--force-create", REFUSAL_L3_PREFIX),
        ("--discard-changes", REFUSAL_L3_PREFIX),
        ("--detach", REFUSAL_OFFLIST_PREFIX),
    ],
    denied_short: &[('C', REFUSAL_L3_PREFIX), ('d', REFUSAL_OFFLIST_PREFIX)],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: true,
};

/// `git tag` (本地建): 名字 [commit]。
const TAG_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &["--annotate", "--quiet", "--message", "--file"],
    allowed_short: &['a', 'q'],
    value_shorts: &['m', 'F'],
    value_long: &["--message", "--file"],
    denied_long: &[
        ("--delete", REFUSAL_L3_PREFIX),
        ("--force", REFUSAL_L3_PREFIX),
        ("--sign", REFUSAL_OFFLIST_PREFIX),
        ("--local-user", REFUSAL_OFFLIST_PREFIX),
    ],
    denied_short: &[
        ('d', REFUSAL_L3_PREFIX),
        ('D', REFUSAL_L3_PREFIX),
        ('f', REFUSAL_L3_PREFIX),
        ('s', REFUSAL_OFFLIST_PREFIX),
        ('u', REFUSAL_OFFLIST_PREFIX),
    ],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: true,
};

/// `git push`: 非强推推送 (L2 出网, 永远人工审批)。
const PUSH_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &[],
    allowed_short: &[],
    value_shorts: &[],
    value_long: &[],
    denied_long: &[
        ("--force", REFUSAL_L3_PREFIX),
        ("--delete", REFUSAL_L3_PREFIX),
        ("--mirror", REFUSAL_L3_PREFIX),
        ("--exec", REFUSAL_L3_PREFIX),
        ("--receive-pack", REFUSAL_L3_PREFIX),
        ("--no-verify", REFUSAL_MORPH_PREFIX),
        ("--signed", REFUSAL_OFFLIST_PREFIX),
        ("--gpg-sign", REFUSAL_OFFLIST_PREFIX),
        ("--push-option", REFUSAL_MORPH_PREFIX),
    ],
    denied_short: &[
        ('f', REFUSAL_L3_PREFIX),
        ('d', REFUSAL_L3_PREFIX),
        ('o', REFUSAL_L3_PREFIX),
    ],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: false,
};

/// `git fetch`: 常规抓取 (L2 出网)。
const FETCH_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &[],
    allowed_short: &[],
    value_shorts: &[],
    value_long: &[],
    denied_long: &[
        ("--force", REFUSAL_L3_PREFIX),
        ("--upload-pack", REFUSAL_L3_PREFIX),
        ("--prune", REFUSAL_L3_PREFIX),
    ],
    denied_short: &[('f', REFUSAL_L3_PREFIX)],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: false,
};

/// `git remote`: 只开 add / rename / set-url / update。
const REMOTE_FAMILY: OptionFamily = OptionFamily {
    allowed_long: &[],
    allowed_short: &[],
    value_shorts: &[],
    value_long: &[],
    denied_long: &[("--mirror", REFUSAL_L3_PREFIX)],
    denied_short: &[],
    denied_long_alias: &[],
    denied_short_alias: &[],
    closed: false,
};

// ---------------------------------------------------------------------------
// 槽位模板 (第一个字节选模板, 其余是槽位载荷)
// ---------------------------------------------------------------------------

const MODE_RAW: usize = 0;
const MODE_ADD_PATH: usize = 1;
const MODE_COMMIT_MESSAGE: usize = 2;
const MODE_BRANCH_NAME: usize = 3;
const MODE_TAG_NAME: usize = 4;
const MODE_SWITCH_NAME: usize = 5;
const MODE_PUSH: usize = 6;
const MODE_FETCH: usize = 7;
const MODE_REMOTE_URL: usize = 8;
const MODE_GLOBAL_OPTION: usize = 9;
const MODE_ADD_OPTION: usize = 10;
const MODE_COMMIT_OPTION: usize = 11;
const MODE_PUSH_OPTION: usize = 12;
const MODE_BRANCH_TWO: usize = 13;
const MODE_SWITCH_CREATE: usize = 14;
const MODE_ADD_AFTER_DASHDASH: usize = 15;
const MODE_PUSH_AFTER_DASHDASH: usize = 16;
const MODE_FETCH_REFSPEC: usize = 17;
const MODE_REMOTE_VERB: usize = 18;
const MODE_COUNT: usize = 19;

/// 按模板拼出命令面。模板里除载荷外全是**词表内的固定词**, 保证载荷落进
/// 我们想测的那张脸。
fn build_command(mode: usize, payload: &str) -> String {
    match mode {
        MODE_RAW => payload.to_string(),
        MODE_ADD_PATH => format!("git add {payload}"),
        MODE_COMMIT_MESSAGE => format!("git commit -m {payload}"),
        MODE_BRANCH_NAME => format!("git branch {payload}"),
        MODE_TAG_NAME => format!("git tag {payload}"),
        MODE_SWITCH_NAME => format!("git switch {payload}"),
        MODE_PUSH => format!("git push {payload}"),
        MODE_FETCH => format!("git fetch {payload}"),
        MODE_REMOTE_URL => format!("git remote add origin {payload}"),
        MODE_GLOBAL_OPTION => format!("git {payload} add a.txt"),
        MODE_ADD_OPTION => format!("git add a.txt {payload}"),
        MODE_COMMIT_OPTION => format!("git commit -m msg {payload}"),
        MODE_PUSH_OPTION => format!("git push origin main {payload}"),
        MODE_BRANCH_TWO => {
            let (left, right) = split_half(payload);
            format!("git branch {left} {right}")
        }
        MODE_SWITCH_CREATE => format!("git switch -c {payload}"),
        MODE_ADD_AFTER_DASHDASH => format!("git add -- {payload}"),
        MODE_PUSH_AFTER_DASHDASH => format!("git push origin -- {payload}"),
        MODE_FETCH_REFSPEC => format!("git fetch origin {payload}"),
        MODE_REMOTE_VERB => format!("git remote {payload}"),
        _ => payload.to_string(),
    }
}

/// 在**字符边界**上对半切 (不切多字节字符中间 —— fuzz 规范 §3.4)。
fn split_half(text: &str) -> (&str, &str) {
    let middle = text
        .char_indices()
        .nth(text.chars().count() / 2)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    text.split_at(middle)
}

/// 载荷是否「简单」: 没有引号、没有段分隔符、没有 shell 元字符 —— 此时载荷的
/// 词切分与 quoting 上下文完全可预测, 槽位特化断言才成立。
fn is_simple_payload(payload: &str) -> bool {
    !payload.chars().any(|ch| {
        matches!(
            ch,
            '\'' | '"'
                | ';'
                | '\n'
                | '\r'
                | '&'
                | '|'
                | '<'
                | '>'
                | '`'
                | '$'
                | '('
                | ')'
                | '{'
                | '}'
        )
    })
}

// ---------------------------------------------------------------------------
// 断言工具
// ---------------------------------------------------------------------------

/// 长选项前缀重叠命中 (与实现 `long_option_overlaps` 同口径; 模块文档 §2:
/// 「长选项按**前缀重叠**命中」)。
fn long_overlaps(base: &str, pattern: &str) -> bool {
    base.starts_with(pattern) || pattern.starts_with(base)
}

/// 拒绝帧必须带三类稳定前缀之一 (L69-76: 「稳定前缀」)。
fn refusal_prefix_of(reason: &str) -> Option<&'static str> {
    REFUSAL_PREFIXES
        .into_iter()
        .find(|prefix| reason.starts_with(prefix))
}

/// 断言「必须被拒」+ 帧前缀。
fn expect_refused(plan: &GitWritePlan, prefix: &str, token: &str, command: &str) {
    match plan {
        GitWritePlan::Refused { reason } => assert!(
            reason.starts_with(prefix),
            "拒绝帧前缀不对: {reason:?} (期望 {prefix:?}; token={token:?}; command={command:?})"
        ),
        GitWritePlan::Graded { tier, .. } => {
            panic!("危险面被放行: token={token:?} command={command:?} tier={tier:?}")
        }
    }
}

/// 断言「必须被拒」(不钉帧前缀 —— 同一张脸的长短形别名, 帧种类不钉死)。
fn expect_refused_any(plan: &GitWritePlan, token: &str, command: &str) {
    match plan {
        GitWritePlan::Refused { .. } => {}
        GitWritePlan::Graded { tier, .. } => {
            panic!("同一张脸的别名被放行: token={token:?} command={command:?} tier={tier:?}")
        }
    }
}

/// 断言「必须是 L2 出网档」。
fn expect_graded_l2(plan: &GitWritePlan, command: &str) {
    match plan {
        GitWritePlan::Graded { tier, .. } => assert_eq!(
            *tier,
            GitWriteTier::NetworkEgress,
            "远端面必须是 L2 出网档: {command:?}"
        ),
        GitWritePlan::Refused { reason } => {
            panic!("远端面被拒 (应放行到人工审批): {reason:?} ({command:?})")
        }
    }
}

// ---------------------------------------------------------------------------
// 契约断言 (逐条带出处)
// ---------------------------------------------------------------------------

/// ① 分级计划的形状契约 (模块文档 §1/§2 + L139-214)。
fn assert_plan_contract(command: &str, plan: &GitWritePlan) {
    match plan {
        GitWritePlan::Refused { reason } => {
            // 契约: 三类拒绝帧各带稳定前缀 (L69-76)。
            let prefix = refusal_prefix_of(reason)
                .unwrap_or_else(|| panic!("拒绝帧缺稳定前缀: {reason:?} (command={command:?})"));
            // 契约: 帧必须说明真因 (前缀 + ':' + 细节; L167-177 的三处构造)。
            assert!(
                reason.len() > prefix.len() && reason[prefix.len()..].starts_with(':'),
                "拒绝帧必须说明真因: {reason:?} (command={command:?})"
            );
        }
        GitWritePlan::Graded { tier, segments } => {
            // 契约: L3 毁灭级「拒绝即帧, 无工具面」(模块文档 L9) ⇒ 可执行计划里
            // 不得出现 L3。
            assert!(
                *tier != GitWriteTier::Destructive,
                "L3 不得出现在可执行计划 (L3 无工具面): {command:?}"
            );
            assert!(!segments.is_empty(), "可执行计划必须有段: {command:?}");
            // 契约: 组合命令逐段分级, 整体取**最严**档 (模块文档 §1: 「组合命令
            // 逐段分级, 整体取最严档 (一段 L2 = 整次 L2)」)。
            let strictest = segments
                .iter()
                .fold(GitWriteTier::ReversibleLocal, |acc, segment| {
                    acc.strictest(segment.tier)
                });
            assert_eq!(
                *tier, strictest,
                "整体分级必须取最严档: {command:?} (整体={tier:?} 逐段={strictest:?})"
            );
            for segment in segments {
                assert!(
                    segment.tier != GitWriteTier::Destructive,
                    "L3 段不得出现在可执行计划: {command:?}"
                );
                assert_segment_argv(command, segment);
            }
        }
    }
}

/// ② 每段 argv 的形状契约 (L144: 「实际执行 argv (git 之后的参数面)」;
/// L20-23: 子命令前的 git 全局面只放行 `--no-pager`/`--paginate`;
/// L414-437: 子命令词表白名单)。
fn assert_segment_argv(command: &str, segment: &GradedSegment) {
    let argv = &segment.argv;
    assert!(!argv.is_empty(), "argv 不得为空: {command:?}");
    assert_ne!(
        argv[0], "git",
        "argv 是「git 之后的参数面」, 首元不得是 git 本身: {command:?}"
    );
    let subcommand = argv
        .iter()
        .position(|token| !token.starts_with('-'))
        .unwrap_or_else(|| panic!("argv 必须含子命令: {command:?}"));
    for token in &argv[..subcommand] {
        assert!(
            ALLOWED_GLOBAL_OPTIONS.contains(&token.as_str()),
            "未列 git 全局选项被放行 (只放行 --no-pager/--paginate): {token:?} ({command:?})"
        );
    }
    assert!(
        WHITELIST_SUBCOMMANDS.contains(&argv[subcommand].as_str()),
        "词表外子命令被放行: {} ({command:?})",
        argv[subcommand]
    );
}

/// ③ 治理面一致性契约 (参数面分级 / L2 地板 / 单调边界 Guard / 分级风险钩子)。
fn assert_governance_contract(command: &str, plan: &GitWritePlan) {
    let arguments = serde_json::json!({ "command": command });

    // 契约 (L216-223): 参数面分级与命令面分级同源。
    assert_eq!(
        git_write_plan_for_arguments(&arguments),
        plan.clone(),
        "参数面分级与命令面分级不一致: {command:?}"
    );

    // 契约 (L225-238): 「命中 L2 (出网) 即地板生效」; 「L3/词表外本来就拒绝即帧,
    // 不需要地板」⇒ 被拒计划不踩地板, L1 不踩地板。
    let expect_floor = matches!(
        plan,
        GitWritePlan::Graded {
            tier: GitWriteTier::NetworkEgress,
            ..
        }
    );
    assert_eq!(
        git_write_human_approval_floor(&arguments),
        expect_floor,
        "L2 人工审批地板判定与分级不一致: {command:?}"
    );

    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).expect("valid capability id");

    // 契约 (L1207-1240): 单调边界 Guard 只拒不放 —— 拒的正是被拒计划, 帧同源。
    let guard = GitWriteBoundaryGuard::new();
    let request = ToolGuardRequest {
        capability: &capability,
        tool_name: GIT_WRITE_TOOL_NAME,
        arguments: &arguments,
    };
    match plan {
        GitWritePlan::Refused { reason } => assert_eq!(
            guard.deny(&request).as_deref(),
            Some(reason.as_str()),
            "边界 Guard 必须拒被拒计划且帧同源: {command:?}"
        ),
        GitWritePlan::Graded { .. } => assert!(
            guard.deny(&request).is_none(),
            "边界 Guard 不得拦词表内分级命令: {command:?}"
        ),
    }

    // 契约 (L1242-1309): 分级风险映射 —— L1 按档放行/审批, L2 永远审批,
    // L3/词表外/变形面拒绝即帧。
    let (session, trace) = request_ids();
    let governance = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        session,
        trace,
        1,
    );
    let default_hook =
        GitWriteTierApprovalHook::new(GitWriteReleaseTier::RequireApprovalEveryWrite);
    let auto_hook = GitWriteTierApprovalHook::new(GitWriteReleaseTier::AutoPassReversibleL1);
    let default_decision = block_on(default_hook.evaluate(&governance));
    let auto_decision = block_on(auto_hook.evaluate(&governance));
    match plan {
        GitWritePlan::Refused { reason } => {
            assert!(
                matches!(default_decision, Decision::Deny { .. }),
                "拒绝即帧必须落成 Deny (不进审批): {command:?}"
            );
            assert!(
                matches!(auto_decision, Decision::Deny { .. }),
                "拒绝即帧在自动放行档也必须落成 Deny: {command:?}"
            );
            let Decision::Deny {
                reason: deny_reason,
            } = &default_decision
            else {
                unreachable!()
            };
            let prefix = refusal_prefix_of(reason).unwrap_or_default();
            assert!(
                deny_reason.contains(prefix),
                "拒绝帧必须进治理面 (理由带真因): {deny_reason:?} ({command:?})"
            );
        }
        GitWritePlan::Graded { tier, .. } => match tier {
            GitWriteTier::ReversibleLocal => {
                assert!(
                    matches!(default_decision, Decision::RequireApproval { .. }),
                    "L1 默认档必须停人工审批: {command:?}"
                );
                assert!(
                    matches!(auto_decision, Decision::Allow),
                    "L1 「自动放行 L1」档必须放行: {command:?}"
                );
            }
            GitWriteTier::NetworkEgress => {
                assert!(
                    matches!(default_decision, Decision::RequireApproval { .. }),
                    "L2 必须停人工审批: {command:?}"
                );
                assert!(
                    matches!(auto_decision, Decision::RequireApproval { .. }),
                    "L2 永不因预设/设置放行: {command:?}"
                );
                // 契约 (L34-35): 理由文本带**分级标注**进审批卡。
                let Decision::RequireApproval { reason } = &default_decision else {
                    unreachable!()
                };
                assert!(reason.contains("L2"), "审批理由必须带分级标注: {reason:?}");
                assert!(
                    reason.contains("never auto-passes"),
                    "L2 审批理由必须写明永不自动放行: {reason:?}"
                );
            }
            GitWriteTier::Destructive => {
                panic!("L3 不得出现在可执行计划: {command:?}")
            }
        },
    }
}

/// ④ 工具面契约 (L1108-1151 + L160-164): 拒绝即帧必须**落到工具面**,
/// 且拒绝发生在**执行之前** (不进审批、不执行 ⇒ 无 git 副作用)。
fn assert_tool_surface(command: &str, plan: &GitWritePlan) {
    let GitWritePlan::Refused { reason } = plan else {
        // 放行命令要真起 git 进程 (需要 tokio 运行时): 本 target 不引新依赖,
        // 放行面的执行语义由 crate 侧 cargo test 覆盖。
        return;
    };
    let tool = GitWriteTool::new("/nonexistent/fuzz/git-write-root");
    let call = ToolCall {
        id: "fuzz-git-write".to_string(),
        name: GIT_WRITE_TOOL_NAME.to_string(),
        arguments: serde_json::json!({ "command": command }),
    };
    let result = block_on(tool.invoke(&call));
    assert!(!result.is_ok(), "拒绝即帧必须落成错误帧: {command:?}");
    let rendered = result.render();
    assert!(
        rendered.contains(reason.as_str()),
        "错误帧必须带拒绝真因: {rendered}"
    );
    assert!(
        !rendered.contains("执行中断"),
        "拒绝必须发生在执行之前 (不得是执行失败帧 ⇒ 无 git 副作用): {rendered}"
    );
}

/// ⑤ 槽位特化断言: 载荷落进某张脸时, 该脸的契约必须仍然成立。
fn assert_slot_rules(mode: usize, payload: &str, command: &str, plan: &GitWritePlan) {
    // 引号会改写 quoting 上下文 ⇒ 下面的「未加引号」类断言只在命令面无引号时
    // 成立 (引号内字面量按参数保留, 不做插值 —— 模块文档 §2)。
    let quoted = command.contains('\'') || command.contains('"');
    if !quoted {
        // 契约 (L275-284): 管道/重定向/插值/分组 元字符一律拒绝即帧 (变形面)。
        if command
            .chars()
            .any(|ch| matches!(ch, '|' | '<' | '>' | '`' | '$' | '(' | ')' | '{' | '}'))
        {
            expect_refused(plan, REFUSAL_MORPH_PREFIX, command, command);
        }
        // 契约 (L264-273): 段分隔只认 `&&` / `;` / 换行 —— 分隔字符绝不进 token。
        if command.contains('&') {
            assert_no_token_contains(plan, &['&'], command);
        }
        if command.contains(';') || command.contains('\n') || command.contains('\r') {
            assert_no_token_contains(plan, &[';', '\n', '\r'], command);
        }
    }

    if !is_simple_payload(payload) {
        return;
    }
    let tokens: Vec<&str> = payload.split_whitespace().collect();

    match mode {
        // 契约 (L20-23): 子命令前的 git 全局面只放行 --no-pager/--paginate。
        // 只有**紧跟 `git` 的那一串 `-` 开头 token**才是全局选项位; 之后的
        // 非选项 token 就是子命令位 (由通用 argv 断言管)。
        MODE_GLOBAL_OPTION => {
            for token in &tokens {
                if !token.starts_with('-') {
                    break;
                }
                if matches!(plan, GitWritePlan::Refused { .. }) {
                    expect_refused_any(plan, token, command);
                    break;
                }
                if *token == "--" {
                    // `--` 是「选项到此为止」的分隔符: 它让子命令位落到 `--` 上,
                    // 帧是词表外 (不是变形面) —— 只钉「必须被拒」。
                    expect_refused_any(plan, token, command);
                    break;
                }
                if !ALLOWED_GLOBAL_OPTIONS.contains(token) {
                    expect_refused(plan, REFUSAL_MORPH_PREFIX, token, command);
                }
            }
        }
        MODE_COMMIT_OPTION => assert_option_family(&COMMIT_FAMILY, &tokens, command, plan),
        MODE_BRANCH_NAME => assert_option_family(&BRANCH_FAMILY, &tokens, command, plan),
        MODE_BRANCH_TWO => {
            // 命令面把载荷对半切成了两个位置参 ⇒ 选项面按**整条 token 序列**走
            // (`--` 分隔符跨两半生效: 分隔符之后是位置参, 不再查选项)。
            let (left, right) = split_half(payload);
            let mut all: Vec<&str> = left.split_whitespace().collect();
            all.extend(right.split_whitespace());
            assert_option_family(&BRANCH_FAMILY, &all, command, plan);
        }
        MODE_SWITCH_NAME | MODE_SWITCH_CREATE => {
            assert_option_family(&SWITCH_FAMILY, &tokens, command, plan)
        }
        MODE_TAG_NAME => assert_option_family(&TAG_FAMILY, &tokens, command, plan),
        MODE_PUSH | MODE_PUSH_OPTION => {
            // `--` 之后全是 refspec (实现 L757-760)。
            let mut after_dashdash = false;
            for token in &tokens {
                if *token == "--" {
                    after_dashdash = true;
                    continue;
                }
                if !after_dashdash && token.starts_with('-') && token.len() > 1 {
                    assert_option_family(&PUSH_FAMILY, std::slice::from_ref(token), command, plan);
                } else {
                    assert_push_refspec(token, command, plan);
                }
            }
        }
        MODE_FETCH | MODE_FETCH_REFSPEC => {
            // `--` 之后全是 refspec (实现 L834-837)。
            let mut after_dashdash = false;
            for token in &tokens {
                if *token == "--" {
                    after_dashdash = true;
                    continue;
                }
                if !after_dashdash && token.starts_with('-') && token.len() > 1 {
                    assert_option_family(&FETCH_FAMILY, std::slice::from_ref(token), command, plan);
                } else {
                    assert_fetch_refspec(token, command, plan);
                }
            }
        }
        MODE_PUSH_AFTER_DASHDASH => {
            // `--` 之后全是 refspec (实现 L757-760)。
            for token in &tokens {
                assert_push_refspec(token, command, plan);
            }
        }
        MODE_REMOTE_VERB => {
            // 契约 (L905-909): `remote --mirror` = 推送即删除的镜像面 → L3。
            for token in tokens.iter().filter(|token| token.starts_with('-')) {
                assert_option_family(&REMOTE_FAMILY, std::slice::from_ref(token), command, plan);
            }
            assert_remote_verb(&tokens, command, plan);
        }
        _ => {}
    }
}

/// 选项面契约检查 (按该子命令的「允许形态 / 拒绝族」表逐 token 走)。
fn assert_option_family(
    family: &OptionFamily,
    tokens: &[&str],
    command: &str,
    plan: &GitWritePlan,
) {
    // 计划**此前**已被拒 (别的 token / 别的检查先触发): 帧的前缀归属不明 ⇒
    // 只钉「被拒」, 不钉前缀。
    if matches!(plan, GitWritePlan::Refused { .. }) {
        expect_refused_any(plan, tokens.first().copied().unwrap_or(""), command);
        return;
    }
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        index += 1;
        if token == "--" {
            // `--` 之后是位置参 (git 与分级同口径)。
            break;
        }
        let base = token.split('=').next().unwrap_or(token);
        if base.starts_with("--") {
            let mut refused = false;
            for (pattern, prefix) in family.denied_long {
                if long_overlaps(base, pattern) {
                    expect_refused(plan, prefix, token, command);
                    refused = true;
                    break;
                }
            }
            if refused {
                return;
            }
            for alias in family.denied_long_alias {
                if base == *alias {
                    expect_refused_any(plan, token, command);
                    return;
                }
            }
            if family.allowed_long.contains(&base) {
                if family.value_long.contains(&base) && !token.contains('=') {
                    index += 1;
                }
                continue;
            }
            if family.closed {
                expect_refused(plan, REFUSAL_OFFLIST_PREFIX, token, command);
                return;
            }
            continue;
        }
        if token.starts_with('-') && token.len() > 1 {
            for ch in token.chars().skip(1) {
                let mut refused = false;
                for (denied, prefix) in family.denied_short {
                    if ch == *denied {
                        expect_refused(plan, prefix, token, command);
                        refused = true;
                        break;
                    }
                }
                if refused {
                    return;
                }
                if family.denied_short_alias.contains(&ch) {
                    expect_refused_any(plan, token, command);
                    return;
                }
                if family.value_shorts.contains(&ch) {
                    break;
                }
                if !family.allowed_short.contains(&ch) && family.closed {
                    expect_refused(plan, REFUSAL_OFFLIST_PREFIX, token, command);
                    return;
                }
            }
        }
    }
}

/// push refspec 变形面 (L806-821): `+` 前缀 = 强推变体; 空侧 `:` = 远端引用删除变体。
fn assert_push_refspec(token: &str, command: &str, plan: &GitWritePlan) {
    if matches!(plan, GitWritePlan::Refused { .. }) {
        expect_refused_any(plan, token, command);
        return;
    }
    if token.starts_with('+') {
        expect_refused(plan, REFUSAL_L3_PREFIX, token, command);
        return;
    }
    if let Some((source, destination)) = token.split_once(':') {
        if source.is_empty() || destination.is_empty() {
            expect_refused(plan, REFUSAL_L3_PREFIX, token, command);
        }
    }
}

/// fetch refspec 变形面 (L866-884): 空侧 `:` = 引用删除; 目的地是本地分支/标签
/// = 本地引用覆盖变体。
fn assert_fetch_refspec(token: &str, command: &str, plan: &GitWritePlan) {
    if matches!(plan, GitWritePlan::Refused { .. }) {
        expect_refused_any(plan, token, command);
        return;
    }
    let Some((source, destination)) = token.split_once(':') else {
        return;
    };
    if source.is_empty() || destination.is_empty() {
        expect_refused(plan, REFUSAL_L3_PREFIX, token, command);
        return;
    }
    if destination.starts_with("refs/heads/")
        || destination.starts_with("refs/tags/")
        || destination == "HEAD"
    {
        expect_refused(plan, REFUSAL_L3_PREFIX, token, command);
    }
}

/// remote 动词面 (L887-937): 只开 add/rename/set-url/update; remove/rm/prune → L3。
fn assert_remote_verb(tokens: &[&str], command: &str, plan: &GitWritePlan) {
    if matches!(plan, GitWritePlan::Refused { .. }) {
        expect_refused_any(plan, tokens.first().copied().unwrap_or(""), command);
        return;
    }
    let Some(verb) = tokens
        .iter()
        .find(|token| !token.starts_with('-'))
        .map(|token| token.to_ascii_lowercase())
    else {
        return;
    };
    match verb.as_str() {
        "remove" | "rm" | "prune" => expect_refused(plan, REFUSAL_L3_PREFIX, &verb, command),
        "add" | "rename" | "set-url" => {
            let positionals = tokens
                .iter()
                .filter(|token| !token.starts_with('-'))
                .count()
                .saturating_sub(1);
            if positionals >= 2 {
                expect_graded_l2(plan, command);
            } else {
                expect_refused(plan, REFUSAL_OFFLIST_PREFIX, &verb, command);
            }
        }
        "update" => expect_graded_l2(plan, command),
        _ => expect_refused(plan, REFUSAL_OFFLIST_PREFIX, &verb, command),
    }
}

/// 段分隔字符绝不进放行 token。
fn assert_no_token_contains(plan: &GitWritePlan, chars: &[char], command: &str) {
    let GitWritePlan::Graded { segments, .. } = plan else {
        return;
    };
    for segment in segments {
        for token in &segment.argv {
            for ch in chars {
                assert!(
                    !token.contains(*ch),
                    "段分隔/句柄字符 {ch:?} 进了放行 token: {token:?} ({command:?})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 固定攻击向量 (覆盖面清单; 走与 fuzz 输入完全相同的断言)
// ---------------------------------------------------------------------------

const ATTACK_BATTERY: &[(usize, &str)] = &[
    // 参数注入面: 以 `-` 开头的选项注入 / 远端命令执行面。
    (MODE_RAW, "git fetch --upload-pack=touch /tmp/pwn"),
    (MODE_RAW, "git fetch --upload-pack touch"),
    (MODE_RAW, "git fetch --upload-pack=sh"),
    (MODE_RAW, "git push --exec=touch /tmp/pwn origin main"),
    (
        MODE_RAW,
        "git push --receive-pack=touch /tmp/pwn origin main",
    ),
    (MODE_RAW, "git -c core.pager=cat add a.txt"),
    (MODE_RAW, "git -ccore.pager=cat add a.txt"),
    (MODE_RAW, "git --config-env=PATH=/tmp/evil add a.txt"),
    (MODE_RAW, "git --exec-path=/tmp/evil add a.txt"),
    (MODE_RAW, "git --git-dir=/tmp/evil add a.txt"),
    (MODE_RAW, "git --work-tree=/tmp/evil add a.txt"),
    (MODE_RAW, "git --namespace=other add a.txt"),
    (MODE_RAW, "git -C /elsewhere commit -m x"),
    (MODE_RAW, "git --no-pager commit -m x"),
    (MODE_RAW, "git --paginate commit -m x"),
    (MODE_GLOBAL_OPTION, "-c core.pager=cat"),
    (MODE_GLOBAL_OPTION, "--git-dir=/tmp/evil"),
    (MODE_GLOBAL_OPTION, "--exec-path=/tmp/evil"),
    (MODE_GLOBAL_OPTION, "--no-pager"),
    (MODE_GLOBAL_OPTION, "--no-pager --paginate"),
    (MODE_GLOBAL_OPTION, "--no-pager -c x=1"),
    // L3 毁灭族 (含变形)。
    (MODE_RAW, "git reset --hard"),
    (MODE_RAW, "git reset --ha"),
    (MODE_RAW, "git reset --hard=true"),
    (MODE_RAW, "git clean -f"),
    (MODE_RAW, "git clean -fdx"),
    (MODE_RAW, "git rebase -i HEAD~2"),
    (MODE_RAW, "git update-ref -d refs/heads/x"),
    (MODE_RAW, "git gc"),
    (MODE_RAW, "git reflog expire"),
    (MODE_RAW, "git replace a b"),
    (MODE_RAW, "git commit --amend"),
    (MODE_RAW, "git commit --am"),
    (MODE_RAW, "git commit -m x --amend"),
    (MODE_BRANCH_NAME, "-d"),
    (MODE_BRANCH_NAME, "-D"),
    (MODE_BRANCH_NAME, "--delete"),
    (MODE_BRANCH_NAME, "-f"),
    (MODE_BRANCH_NAME, "--force"),
    (MODE_SWITCH_NAME, "-C"),
    (MODE_SWITCH_NAME, "--force-create"),
    (MODE_SWITCH_NAME, "--discard-changes"),
    (MODE_SWITCH_NAME, "-d"),
    (MODE_TAG_NAME, "-f"),
    (MODE_TAG_NAME, "--force"),
    (MODE_TAG_NAME, "-d"),
    (MODE_TAG_NAME, "--delete"),
    (MODE_TAG_NAME, "-s"),
    (MODE_TAG_NAME, "--sign"),
    (MODE_PUSH, "--force"),
    (MODE_PUSH, "--force-with-lease=refs/heads/main"),
    (MODE_PUSH, "--delete"),
    (MODE_PUSH, "--mirror"),
    (MODE_PUSH, "--exec=touch /tmp/pwn"),
    (MODE_PUSH, "--receive-pack=touch /tmp/pwn"),
    (MODE_PUSH, "--no-verify"),
    (MODE_PUSH, "--signed"),
    (MODE_PUSH, "--push-option=x"),
    (MODE_PUSH, "-qf"),
    (MODE_PUSH, "+main"),
    (MODE_PUSH, ":main"),
    (MODE_PUSH, "main:"),
    (MODE_PUSH_AFTER_DASHDASH, "--force"),
    (MODE_FETCH, "--force"),
    (MODE_FETCH, "--upload-pack=sh"),
    (MODE_FETCH, "--prune"),
    (MODE_FETCH, "-f"),
    (MODE_FETCH_REFSPEC, "main:refs/heads/main"),
    (MODE_FETCH_REFSPEC, "main:"),
    (MODE_FETCH_REFSPEC, ":main"),
    (MODE_REMOTE_VERB, "remove origin"),
    (MODE_REMOTE_VERB, "rm origin"),
    (MODE_REMOTE_VERB, "prune origin"),
    (MODE_REMOTE_VERB, "add origin https://host.example/x.git"),
    (MODE_REMOTE_VERB, "add origin"),
    (MODE_REMOTE_VERB, "show origin"),
    (MODE_REMOTE_VERB, "--mirror"),
    // 词表外。
    (MODE_RAW, "git status"),
    (MODE_RAW, "git checkout main"),
    (MODE_RAW, "git checkout -- file.txt"),
    (MODE_RAW, "git restore ."),
    (MODE_RAW, "git merge feature"),
    (MODE_RAW, "git config user.name x"),
    (MODE_RAW, "git clone https://host.example/repo.git"),
    (MODE_RAW, "echo hi"),
    (MODE_RAW, "git commit"),
    (MODE_RAW, "git commit -a"),
    (MODE_RAW, "git branch"),
    (MODE_RAW, "git tag"),
    // shell 面。
    (MODE_RAW, "git add a.txt | tee log"),
    (MODE_RAW, "git add a.txt > out.txt"),
    (MODE_RAW, "git add a.txt >> out.txt"),
    (MODE_RAW, "git add a.txt < in.txt"),
    (MODE_RAW, "git add a.txt &"),
    (MODE_RAW, "git commit -m $(boom)"),
    (MODE_RAW, "git commit -m `boom`"),
    (MODE_RAW, "git commit -m \"a;b\""),
    (MODE_RAW, "git commit -m 'unclosed"),
    // 分支名以 `-` 开头 / `--` 分隔。
    (MODE_RAW, "git branch -x"),
    (MODE_RAW, "git branch -- -x"),
    (MODE_RAW, "git switch -x"),
    (MODE_RAW, "git switch -- -c"),
    (MODE_RAW, "git tag -x"),
    (MODE_RAW, "git tag -- -f"),
    (MODE_RAW, "git push -- --force"),
    // 路径面。
    (MODE_RAW, "git add ../outside.txt"),
    (MODE_RAW, "git add /etc/passwd"),
    (MODE_RAW, "git add .git/config"),
    (MODE_RAW, "git add .git/hooks/pre-commit"),
    (MODE_ADD_PATH, "../outside.txt"),
    (MODE_ADD_PATH, "/etc/passwd"),
    (MODE_ADD_PATH, ".git/config"),
    (MODE_ADD_PATH, ".git/hooks/pre-commit"),
    (MODE_ADD_AFTER_DASHDASH, "../outside.txt"),
    (MODE_ADD_AFTER_DASHDASH, ".git/config"),
    // 语义面。
    (MODE_RAW, "git commit -m \"\""),
    (MODE_RAW, "git commit -m \"   \""),
    (MODE_RAW, "git commit -m \"\n\n\""),
    (MODE_RAW, "git commit -m \"a\tb\""),
    (MODE_RAW, "git commit -m \"nul\0byte\""),
    (MODE_RAW, "git commit --file=/etc/passwd"),
    (MODE_RAW, "git commit --template=/etc/passwd"),
    (MODE_RAW, "git commit --allow-empty-message -m \"\""),
    (MODE_RAW, "git commit --reuse-message=HEAD"),
    (MODE_RAW, "git commit -C HEAD"),
    // Unicode。
    (MODE_RAW, "git branch 特性分支"),
    (MODE_RAW, "git commit -m \"提交信息\""),
    (MODE_RAW, "git add 文件.txt"),
    (MODE_RAW, "git tag v1.0-版本"),
    // 编辑器面 / 交互补丁面 (长短形别名)。
    (MODE_COMMIT_OPTION, "-e"),
    (MODE_COMMIT_OPTION, "--edit"),
    (MODE_COMMIT_OPTION, "--interactive"),
    (MODE_COMMIT_OPTION, "-p"),
    (MODE_COMMIT_OPTION, "-n"),
    (MODE_COMMIT_OPTION, "-s"),
    (MODE_COMMIT_OPTION, "-S"),
    (MODE_COMMIT_OPTION, "--no-verify"),
    (MODE_COMMIT_OPTION, "--sign"),
    (MODE_COMMIT_OPTION, "--gpg-sign"),
    (MODE_COMMIT_OPTION, "--amend"),
    // 组合段。
    (MODE_RAW, "git add a.txt && git commit -m x"),
    (MODE_RAW, "git add a.txt && git push origin main"),
    (MODE_RAW, "git add a.txt && git reset --hard"),
    (MODE_RAW, "git commit -m x ; git fetch origin"),
    (MODE_RAW, "git add a.txt\ngit commit -m x"),
    (MODE_RAW, "git add a.txt ; ; git commit -m x"),
    // 凭据 URL (脱敏面: 凭据不入命令面/审批理由)。
    (
        MODE_RAW,
        "git remote add origin https://user:tok123456@host.example/repo.git",
    ),
    (
        MODE_RAW,
        "git push https://user:tok123456@host.example/repo.git main",
    ),
];

// ---------------------------------------------------------------------------
// 工具
// ---------------------------------------------------------------------------

/// 极简执行器: 本 target 只跑**同步可完结**的 future (拒绝帧路径与治理判定),
/// 不需要 tokio 运行时 (不给 fuzz 子 workspace 引入新依赖)。
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;
    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 治理判定用的 id (一次生成, 全程复用)。
fn request_ids() -> (SessionId, TraceId) {
    static IDS: OnceLock<(SessionId, TraceId)> = OnceLock::new();
    *IDS.get_or_init(|| (SessionId::new(), TraceId::new()))
}

/// 一条命令面的全部契约断言。
fn check_command(mode: usize, payload: &str, command: &str) {
    let plan = classify_git_write(command);

    // 确定性: 同一命令两次分级必须一致 (拒绝即帧的帧也一致)。
    assert_eq!(
        classify_git_write(command),
        plan.clone(),
        "同一命令两次分级不一致: {command:?}"
    );

    assert_plan_contract(command, &plan);
    assert_governance_contract(command, &plan);
    assert_tool_surface(command, &plan);
    assert_slot_rules(mode, payload, command, &plan);
}

fuzz_target!(|data: &[u8]| {
    // 固定攻击向量: 与 fuzz 输入走完全相同的断言 (覆盖面清单可复核)。
    static BATTERY: std::sync::Once = std::sync::Once::new();
    BATTERY.call_once(|| {
        for (mode, payload) in ATTACK_BATTERY {
            let command = build_command(*mode, payload);
            check_command(*mode, payload, &command);
        }
    });

    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let mut chars = text.chars();
    let mode = (chars.next().expect("non-empty") as usize) % MODE_COUNT;
    let payload: String = chars.collect();
    let command = build_command(mode, &payload);
    check_command(mode, &payload, &command);
});
