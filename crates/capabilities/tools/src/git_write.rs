//! git 写分级工具面 (`tool.git_write`, 声明名 `git_write`) —— 干活能力轴 P1。
//!
//! 仓库写操作不再「一刀切无工具」, 而是**分级词表 + 分级语义**:
//!
//! | 级 | 词表 (写意图族) | 风险语义 |
//! |----|------------------|----------|
//! | L1 可逆 | `git add` / `git commit` (带消息) / `git branch` 建·`git switch` 切 / `git tag` 本地建 | 默认 **RequireApproval** (每次人批); 设置可开「自动放行 L1」档 |
//! | L2 出网 | `git push` (非强推) / `git fetch` / `git remote` add·rename·set-url·update | **永远人工审批** —— 无自动放行档, 任何预设/设置都不得越人 |
//! | L3 毁灭 | `reset --hard` / `clean -f` / 强推 / 改史 (`commit --amend`·`rebase`) / 引用删除 (`update-ref -d`·`branch -d`·`tag -d`) | **拒绝即帧** —— 无工具面, 不给幻觉空间 (工具描述同文) |
//!
//! # 执行面 (词法分级 + 受控通道)
//!
//! 1. **命令白名单词法分级**: 命令文本必须是 git 调用段 (`git` 起词), 段间只
//!    认 `&&` / `;` / 换行; 管道、重定向、`$`/反引号等 shell 面一律拒绝即帧
//!    (本工具按结构化参数执行 git, **不经 shell**)。组合命令逐段分级, 整体
//!    取**最严**档 (一段 L2 = 整次 L2; 一段 L3 = 整次拒绝即帧)。
//! 2. **参数变形防护**: 长选项按**前缀重叠**命中 (`--ha` ≡ `--hard` 缩写,
//!    `--force-with-lease=x` 撞 `--force` 族), 短选项束逐字符查
//!    (`-qf` 里的 `f` 也算强推); refspec 面查 `+` (强推变体) 与空侧 `:`
//!    (引用删除变体)。子命令前的 git 全局面只放行 `--no-pager`/`--paginate`,
//!    `-c`/`--config-env`/`--exec-path`/`--git-dir`/`--work-tree` 等与任何
//!    未列全局选项拒绝即帧 (别名注入/执行路径劫持无处藏)。
//! 3. **受控通道执行**: 执行走既有 [`ProcessExecutor`] 安全容器 —— 结构化
//!    argv (无 shell 解析)、`git -C <工作区根>` 限域、`ProcessLimits::default()`
//!    超时 (到期归 `timeout.*` 族)、输出按上限截断归一。
//! 4. **原子性如实语义**: 段是**顺序落地**的, 中段失败不谎称回滚 —— 错误帧
//!    逐段报告「已落地段保留 / 失败段 / 失败因」, staged 残留照实留存。
//! 5. **脱敏**: 命令面/输出/审批理由里的远端 URL 凭据一律走
//!    [`redact_secret_surfaces`] —— 凭据不入日志、不入审批卡。
//!
//! # 治理接线 (与本地审批面板 / IM 审批卡同链)
//!
//! - [`GitWriteTierApprovalHook`]: 分级风险映射 (L1 按档位放行或审批,
//!   L2 永远审批, L3/词表外拒绝即帧), 理由文本带**分级标注**进审批卡;
//! - [`GitWriteRiskMappingHook`] + [`GitWriteBoundaryGuard`]: 工具面五段流水
//!   线的授权映射与单调边界 (拒绝不可被后续阶段翻回放行);
//! - 会话预设 (`read_only` 拒 / `standard` 审批 / `full` L1 放行·L2 照审批)
//!   由组装根的 [`apeireth_governance`] 预设钩子承载, L2 永不因预设放行。

// ── **词法面**（段切分 / 提词）外置 —— 纯搬移, 不改逻辑。
//
// 存在理由: 本文件在 size 棘轮基线上（`arch-baseline.txt`, 0 容差）⇒ 任何净增都会让
// `arch_guard` 变红。棘轮文件**不归我改** ⇒ 正路是**让本文件净减**。
// `src/git_write.rs` + `src/git_write/parse.rs` 是 Rust 2018 标准模块布局
// ⇒ `mod parse;` **自动解析**, **不需要**改 `lib.rs`（别人的文件）。
mod parse;
use parse::{split_command_segments, tokenize_segment};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::deadline::TimeoutErrorCode;
use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    redact_secret_surfaces, Action, Decision, GovernanceHook, GovernanceRequest, PermissionPolicy,
    ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};
use crate::process::{ProcessExecutor, ProcessLimits, ProcessRequest};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const GIT_WRITE_CAPABILITY_ID: &str = "tool.git_write";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const GIT_WRITE_TOOL_NAME: &str = "git_write";

/// 生产执行时限 (毫秒): 每段 git 子进程的墙钟上限, 到期归 `timeout.*`。
pub const GIT_WRITE_TIMEOUT_MS: u64 = 30_000;

/// L3 毁灭级拒绝帧的稳定前缀 (拒绝即帧, 无工具面)。
pub const REFUSAL_L3_PREFIX: &str = "L3 毁灭级拒绝即帧 (无工具面)";

/// 词表外拒绝帧的稳定前缀 (白名单外子命令/形态, 同样无工具面)。
pub const REFUSAL_OFFLIST_PREFIX: &str = "词表外拒绝即帧 (无工具面)";

/// 变形面拒绝帧的稳定前缀 (选项变形 / shell 面 / 危险全局选项)。
pub const REFUSAL_MORPH_PREFIX: &str = "变形面拒绝即帧 (无工具面)";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const GIT_WRITE_TOOL_DESCRIPTION: &str = "Run one graded git write command \
 against the repository at the workspace root. The command text is git \
 invocations only, optionally joined with `&&`, `;` or newlines; every segment \
 is graded and the whole call takes the strictest tier. L1 reversible-local \
 (git add; git commit with an explicit message; git branch create / git \
 switch; git tag create) requires human approval by default and may be \
 auto-passed only by an explicit settings tier. L2 network-egress (git push \
 without force, git fetch, git remote add/rename/set-url/update) always \
 requires a human decision - no preset or setting auto-passes L2. L3 \
 destructive operations (history rewrites such as commit --amend or rebase, \
 forced pushes, ref deletions such as branch -d or update-ref -d, reset --hard, \
 clean -f) are refused as error frames and deliberately have no tool surface. \
 Dangerous git global options (-c, --config-env, --exec-path, --git-dir, \
 --work-tree, or any unlisted global option) are refused. Pipes, redirections \
 and shell metacharacters are refused: commands run as structured git \
 arguments inside the workspace root under a timeout, segment by segment, and \
 a mid-sequence failure reports exactly which segments already landed (no \
 rollback claim). Credential URLs in commands, output and approval payloads \
 are redacted.";

/// 写意图分级 (词表三档)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GitWriteTier {
    /// L1 可逆: 本地仓库可逆写 (add / commit / branch 建·switch 切 / tag 本地)。
    ReversibleLocal = 1,
    /// L2 出网: 远端交互 (push / fetch / remote 配置) —— 永远人工审批。
    NetworkEgress = 2,
    /// L3 毁灭: 改史 / 强推 / 引用删除 —— 拒绝即帧, 无工具面。
    Destructive = 3,
}

impl GitWriteTier {
    /// 稳定分级标签 (审批卡分级标注 / 报告字段同文)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReversibleLocal => "L1",
            Self::NetworkEgress => "L2",
            Self::Destructive => "L3",
        }
    }

    /// 分级面的人读名 (风险语义)。
    pub const fn surface(self) -> &'static str {
        match self {
            Self::ReversibleLocal => "reversible-local",
            Self::NetworkEgress => "network-egress",
            Self::Destructive => "destructive",
        }
    }

    /// 组合命令取最严: 高位覆盖低位。
    pub const fn strictest(self, other: Self) -> Self {
        if (other as u8) > (self as u8) {
            other
        } else {
            self
        }
    }
}

/// 一段已分级、可执行的 git 写命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradedSegment {
    /// 人读命令面 (已脱敏)。
    pub display: String,
    /// 实际执行 argv (git 之后的参数面; 执行时由 [`GitWriteTool`] 前置 `-C` 限域)。
    pub argv: Vec<String>,
    /// 该段分级。
    pub tier: GitWriteTier,
}

/// 一次 git 写调用的分级判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWritePlan {
    /// 白名单内: 可执行, 审批档位由治理层按分级决定。
    Graded {
        /// 整次调用的分级 (逐段取最严)。
        tier: GitWriteTier,
        /// 逐段执行面。
        segments: Vec<GradedSegment>,
    },
    /// 拒绝即帧: L3 毁灭级 / 词表外 / 变形面 (无工具面, 不进审批、不执行)。
    Refused {
        /// 拒绝帧全文 (含稳定前缀)。
        reason: String,
    },
}

fn refuse_l3(detail: impl AsRef<str>) -> String {
    format!("{REFUSAL_L3_PREFIX}: {}", detail.as_ref())
}

fn refuse_offlist(detail: impl AsRef<str>) -> String {
    format!("{REFUSAL_OFFLIST_PREFIX}: {}", detail.as_ref())
}

fn refuse_morph(detail: impl AsRef<str>) -> String {
    format!("{REFUSAL_MORPH_PREFIX}: {}", detail.as_ref())
}

/// 分级一次 git 写调用 (词表 + 变形防护 + 逐段取最严)。
pub fn classify_git_write(command: &str) -> GitWritePlan {
    let segments_text = match split_command_segments(command) {
        Ok(segments) => segments,
        Err(reason) => return GitWritePlan::Refused { reason },
    };
    if segments_text.is_empty() {
        return GitWritePlan::Refused {
            reason: refuse_offlist("空命令: 本工具只执行分级词表内的 git 写命令"),
        };
    }
    let mut segments = Vec::new();
    let mut tier = GitWriteTier::ReversibleLocal;
    for text in &segments_text {
        let tokens = match tokenize_segment(text) {
            Ok(tokens) => tokens,
            Err(reason) => return GitWritePlan::Refused { reason },
        };
        if tokens.is_empty() {
            continue;
        }
        match grade_segment(text, &tokens) {
            Ok(segment) => {
                tier = tier.strictest(segment.tier);
                segments.push(segment);
            }
            Err(reason) => return GitWritePlan::Refused { reason },
        }
    }
    if segments.is_empty() {
        return GitWritePlan::Refused {
            reason: refuse_offlist("空命令: 本工具只执行分级词表内的 git 写命令"),
        };
    }
    GitWritePlan::Graded { tier, segments }
}

/// 从调用参数分级 (参数名 [`GIT_WRITE_TOOL_NAME`] 的 `command` 面)。
pub fn git_write_plan_for_arguments(arguments: &serde_json::Value) -> GitWritePlan {
    let command = arguments
        .get("command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    classify_git_write(command)
}

/// 「L2 永不因预设放行」地板判定: 命中 L2 (出网) 即地板生效。
///
/// 会话预设的 `full` 放行改写与 `approval_remember` 免审记忆都不得越过它 ——
/// 出网写每次都停在人工审批 (本地面板 / IM 审批卡同链)。L3/词表外本来就
/// 拒绝即帧, 不需要地板。
pub fn git_write_human_approval_floor(arguments: &serde_json::Value) -> bool {
    matches!(
        git_write_plan_for_arguments(arguments),
        GitWritePlan::Graded {
            tier: GitWriteTier::NetworkEgress,
            ..
        }
    )
}

// ---------------------------------------------------------------------------
// 词法面: 段切分 / 提词
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 词法面: 逐段分级 (词表 + 变形防护)
// ---------------------------------------------------------------------------

/// 长选项基名 (`--message=x` → `--message`)。
fn option_base(token: &str) -> &str {
    match token.find('=') {
        Some(index) => &token[..index],
        None => token,
    }
}

/// 长选项前缀重叠命中 (变形防护): git 允许无歧义缩写, 故 `--ha` 是 `--hard`
/// 的同面变体, `--force-with-lease=x` 与 `--force` 同族 —— 两侧前缀重叠都算。
fn long_option_overlaps(base: &str, pattern: &str) -> bool {
    base.starts_with(pattern) || pattern.starts_with(base)
}

/// 短选项束的字符面 (`-qf` → `q`,`f`); 长选项与裸 `-` 不算束。
fn short_bundle(token: &str) -> Option<Vec<char>> {
    if token.starts_with("--") || token == "-" {
        return None;
    }
    token
        .strip_prefix('-')
        .map(|rest| rest.chars().collect::<Vec<char>>())
}

/// 逐段分级: 词表白名单 + 危险全局选项拒 + 变形防护。
fn grade_segment(text: &str, tokens: &[String]) -> Result<GradedSegment, String> {
    let Some(first) = tokens.first() else {
        return Err(refuse_offlist(
            "空命令: 本工具只执行分级词表内的 git 写命令",
        ));
    };
    if !first.eq_ignore_ascii_case("git") {
        return Err(refuse_offlist(format!(
            "非 git 白名单段 {first:?}: 本工具只执行分级词表内的 git 写命令"
        )));
    }

    // 子命令前的 git 全局面: 只放行 --no-pager / --paginate。-c (别名/配置注入)、
    // --exec-path (执行路径劫持)、--git-dir/--work-tree (仓库逃逸) 等一律拒。
    let mut index = 1;
    while index < tokens.len() && tokens[index].starts_with('-') {
        if tokens[index] == "--" {
            break;
        }
        match tokens[index].as_str() {
            "--no-pager" | "--paginate" => index += 1,
            other => {
                return Err(refuse_morph(format!(
                    "危险/未知 git 全局选项 {other:?} (子命令前): -c / --config-env / \
                     --exec-path / --git-dir / --work-tree 族与任何未列全局选项拒绝即帧"
                )))
            }
        }
    }
    let subcommand = tokens
        .get(index)
        .ok_or_else(|| refuse_offlist("缺 git 子命令: 只有分级词表内的写子命令可执行"))?;
    let rest = &tokens[index + 1..];

    let tier = grade_subcommand(&subcommand.to_ascii_lowercase(), rest)?;

    let mut argv: Vec<String> = tokens[1..].to_vec();
    argv[index - 1] = subcommand.to_ascii_lowercase();
    Ok(GradedSegment {
        display: redact_secret_surfaces(text.trim()),
        argv,
        tier,
    })
}

/// 子命令词表分级。
fn grade_subcommand(subcommand: &str, args: &[String]) -> Result<GitWriteTier, String> {
    match subcommand {
        "add" => Ok(GitWriteTier::ReversibleLocal),
        "commit" => grade_commit(args),
        "branch" => grade_branch(args),
        "switch" => grade_switch(args),
        "tag" => grade_tag(args),
        "push" => grade_push(args),
        "fetch" => grade_fetch(args),
        "remote" => grade_remote(args),
        // L3 毁灭族: 改史 / 强制覆盖 / 引用删除 / 清理 —— 拒绝即帧, 无工具面。
        "reset" | "clean" | "rebase" | "update-ref" | "filter-branch" | "filter-repo" | "gc"
        | "prune" | "reflog" | "replace" => Err(refuse_l3(format!(
            "git {subcommand} 属毁灭族 (改史/强制覆盖/引用删除/清理): 与 reset --hard / \
             clean -f / update-ref -d 同权, 永不提供工具面"
        ))),
        "checkout" | "restore" => Err(refuse_offlist(format!(
            "git {subcommand} 的路径丢弃面不入本工具: 切分支走 git switch (L1), \
             丢弃工作区变更属 L3 毁灭族 (无工具面)"
        ))),
        other => Err(refuse_offlist(format!(
            "git {other} 不在分级词表: 白名单 = L1 (add/commit/branch/switch/tag) + \
             L2 (push/fetch/remote); 其余写面拒绝即帧"
        ))),
    }
}

/// 值参数短选项: 遇上即"余下字符是值, 下一词是值"。
const COMMIT_VALUE_SHORTS: &[char] = &['m', 'F', 'C', 't'];

/// commit 短选项束的拒绝字符: `n`(--no-verify 绕钩子) / `s`·`S`(签名密钥面) /
/// `c`·**`e`**(**编辑器面**) / `p`(交互补丁面)。
///
/// ⚠️ `e`（`--edit` 的短形）此前**漏在表外** ⇒ `git commit -m x -e` / `-em x`
/// 被**静默放过**（`e` 既不在拒绝表、也不在值参数表 ⇒ 短选项束扫描时**被忽略**）
/// ⇒ 违反本文件写死的承诺「**本工具不开编辑器面**」。
const COMMIT_DENIED_SHORTS: &[char] = &['n', 's', 'S', 'c', 'e', 'p'];

fn grade_commit(args: &[String]) -> Result<GitWriteTier, String> {
    let mut message_source = false;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            break;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if long_option_overlaps(base, "--amend") {
                    return Err(refuse_l3(
                        "commit --amend 改写既有提交 = 改史族 (与 rebase 改史同权)",
                    ));
                }
                if long_option_overlaps(base, "--no-verify") {
                    return Err(refuse_morph("commit --no-verify 绕过仓库钩子 = 变形绕过面"));
                }
                if long_option_overlaps(base, "--edit") {
                    // `--edit` = 在编辑器里改提交消息 ⇒ **编辑器面**（长选项形）。
                    // 与短选项 `-e` 同一族（此前长选项分支**根本没检查**它 ⇒ 放行）。
                    return Err(refuse_offlist("commit --edit 开编辑器: 本工具不开编辑器面"));
                }
                if long_option_overlaps(base, "--patch")
                    || long_option_overlaps(base, "--interactive")
                {
                    // ⚠️ **交互补丁面（长选项形）**（2026-10-26 修，缺陷 ㉑）：
                    // 拒绝表只挡了短选项 `-p`，而 git 自己的 usage 行写着
                    // `用法：git commit [-a | --interactive | --patch] …`
                    // ⇒ **`--patch` / `--interactive` 与 `-p` 是同一张脸**
                    // ⇒ 放行它们 = **别名绕过**（fuzz 实测：
                    // `git commit -m msg --interactive` 被放行成 ReversibleLocal）。
                    return Err(refuse_offlist(
                        "commit --patch/--interactive 交互补丁面: 本工具不开交互面",
                    ));
                }
                if long_option_overlaps(base, "--reedit-message") {
                    // ⚠️ **复用并编辑面（长选项形）**（2026-10-26 修，缺陷 ㉑）：
                    // `-c` 早已在拒绝表里，而它的长形 `--reedit-message`
                    // （复用既有提交的 message **并进编辑器改**）此前**没被检查**
                    // ⇒ 同脸不同形 ⇒ 别名绕过。
                    return Err(refuse_offlist(
                        "commit --reedit-message 复用并编辑面: 本工具不开编辑器面",
                    ));
                }
                if long_option_overlaps(base, "--sign") || long_option_overlaps(base, "--gpg-sign")
                {
                    return Err(refuse_offlist("签名/密钥面不入本工具"));
                }
                if matches!(
                    base,
                    "--message" | "--file" | "--reuse-message" | "--fixup" | "--squash"
                ) {
                    message_source = true;
                    if !token.contains('=') {
                        index += 1;
                    }
                }
                index += 1;
                continue;
            }
            // 短选项束: 逐字符查拒绝面; 遇值参数字符即止 (余下是值)。
            for ch in token[1..].chars() {
                if COMMIT_DENIED_SHORTS.contains(&ch) {
                    return Err(match ch {
                        'n' => refuse_morph("commit -n/--no-verify 绕过仓库钩子 = 变形绕过面"),
                        'c' | 'e' => refuse_offlist("commit -c/-e 开编辑器: 本工具不开编辑器面"),
                        _ => refuse_offlist("签名/密钥面不入本工具"),
                    });
                }
                if COMMIT_VALUE_SHORTS.contains(&ch) {
                    if ch != 't' {
                        message_source = true;
                    }
                    break;
                }
            }
            index += 1;
            continue;
        }
        index += 1;
    }
    if !message_source {
        return Err(refuse_offlist(
            "commit 必须显式带消息 (-m/-F/--message/--file/--reuse-message): 本工具不开编辑器面",
        ));
    }
    Ok(GitWriteTier::ReversibleLocal)
}

/// branch 只开**建**形态 (切走 switch): 位置参 = 名字 [起点]。
fn grade_branch(args: &[String]) -> Result<GitWriteTier, String> {
    let mut positionals = 0usize;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            positionals += args.len() - index - 1;
            break;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if long_option_overlaps(base, "--delete") {
                    return Err(refuse_l3(
                        "branch -d/-D = 引用删除族 (与 update-ref -d 同权)",
                    ));
                }
                if long_option_overlaps(base, "--force") {
                    return Err(refuse_l3("branch -f = 强制覆盖分支引用"));
                }
                if matches!(base, "--track" | "--no-track" | "--quiet") {
                    index += 1;
                    continue;
                }
                return Err(refuse_offlist(
                    "branch 只开建/切形态: 建 = git branch <name> [起点], 切 = git switch <name>; \
                     列表/改名/删除/强制面拒绝即帧",
                ));
            }
            for ch in short_bundle(token).unwrap_or_default() {
                match ch {
                    'd' | 'D' => {
                        return Err(refuse_l3(
                            "branch -d/-D = 引用删除族 (与 update-ref -d 同权)",
                        ))
                    }
                    'f' => return Err(refuse_l3("branch -f = 强制覆盖分支引用")),
                    't' | 'q' => {}
                    _ => {
                        return Err(refuse_offlist(
                            "branch 只开建/切形态: 建 = git branch <name> [起点], 切 = git switch <name>; \
                             列表/改名/删除/强制面拒绝即帧",
                        ))
                    }
                }
            }
            index += 1;
            continue;
        }
        positionals += 1;
        index += 1;
    }
    if !(1..=2).contains(&positionals) {
        return Err(refuse_offlist(
            "branch 建形态要求 1-2 个位置参 (名字 [起点])",
        ));
    }
    Ok(GitWriteTier::ReversibleLocal)
}

/// switch 只开**切**形态 (含 `-c` 建并切)。
fn grade_switch(args: &[String]) -> Result<GitWriteTier, String> {
    let mut positionals = 0usize;
    let mut created = false;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            positionals += args.len() - index - 1;
            break;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if long_option_overlaps(base, "--force-create") {
                    return Err(refuse_l3("switch --force-create 重置既有分支 = 强制覆盖族"));
                }
                if long_option_overlaps(base, "--discard-changes") {
                    return Err(refuse_l3(
                        "switch --discard-changes 丢弃工作区变更 = 毁灭族",
                    ));
                }
                if long_option_overlaps(base, "--detach") {
                    return Err(refuse_offlist("switch --detach 游离面不入本工具"));
                }
                match base {
                    "--create" => {
                        created = true;
                        if !token.contains('=') {
                            index += 1;
                        }
                    }
                    "--track" | "--no-track" | "--quiet" => {}
                    _ => {
                        return Err(refuse_offlist(
                            "switch 只开切/建并切形态: git switch <branch> 或 git switch -c <new>",
                        ))
                    }
                }
                index += 1;
                continue;
            }
            for ch in short_bundle(token).unwrap_or_default() {
                match ch {
                    'C' => return Err(refuse_l3("switch -C 强制创建会重置既有分支 = 强制覆盖族")),
                    'd' => return Err(refuse_offlist("switch -d/--detach 游离面不入本工具")),
                    'c' => {
                        created = true;
                        break;
                    }
                    't' | 'q' => {}
                    _ => {
                        return Err(refuse_offlist(
                            "switch 只开切/建并切形态: git switch <branch> 或 git switch -c <new>",
                        ))
                    }
                }
            }
            index += 1;
            continue;
        }
        positionals += 1;
        index += 1;
    }
    if !created && positionals != 1 {
        return Err(refuse_offlist(
            "switch 切形态要求 1 个分支名 (或 -c <new> 建并切)",
        ));
    }
    Ok(GitWriteTier::ReversibleLocal)
}

/// tag 只开**本地创建**形态 (名字 [commit])。
fn grade_tag(args: &[String]) -> Result<GitWriteTier, String> {
    let mut positionals = 0usize;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            positionals += args.len() - index - 1;
            break;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if long_option_overlaps(base, "--delete") {
                    return Err(refuse_l3(
                        "tag --delete = 引用删除族 (与 update-ref -d 同权)",
                    ));
                }
                if long_option_overlaps(base, "--force") {
                    return Err(refuse_l3("tag --force = 强制覆盖既有 tag 引用"));
                }
                if long_option_overlaps(base, "--sign")
                    || long_option_overlaps(base, "--local-user")
                {
                    return Err(refuse_offlist("签名/密钥面不入本工具"));
                }
                match base {
                    "--annotate" | "--quiet" => {}
                    "--message" | "--file" => {
                        if !token.contains('=') {
                            index += 1;
                        }
                    }
                    _ => {
                        return Err(refuse_offlist(
                            "tag 只开本地创建形态: git tag <name> [commit] (-a/-m/-F 附注可选)",
                        ))
                    }
                }
                index += 1;
                continue;
            }
            for ch in short_bundle(token).unwrap_or_default() {
                match ch {
                    'd' | 'D' => {
                        return Err(refuse_l3("tag -d = 引用删除族 (与 update-ref -d 同权)"))
                    }
                    'f' => return Err(refuse_l3("tag -f = 强制覆盖既有 tag 引用")),
                    's' | 'u' => return Err(refuse_offlist("签名/密钥面不入本工具")),
                    'a' | 'q' => {}
                    'm' | 'F' => break,
                    _ => {
                        return Err(refuse_offlist(
                            "tag 只开本地创建形态: git tag <name> [commit] (-a/-m/-F 附注可选)",
                        ))
                    }
                }
            }
            index += 1;
            continue;
        }
        positionals += 1;
        index += 1;
    }
    if !(1..=2).contains(&positionals) {
        return Err(refuse_offlist(
            "tag 建形态要求 1-2 个位置参 (名字 [commit])",
        ));
    }
    Ok(GitWriteTier::ReversibleLocal)
}

/// push 的拒绝长选项族 (前缀重叠, 变形同拒)。
const PUSH_DENIED_L3_LONGS: &[&str] = &[
    "--force", // 含 --force-with-lease / --force-if-includes 族 (强推)
    "--delete",
    "--mirror",
    "--exec",
    "--receive-pack",
];
/// push 的拒绝短选项字符: `f`(强推) / `d`(删除) / `o`(服务端钩子选项面)。
const PUSH_DENIED_SHORTS: &[char] = &['f', 'd', 'o'];

fn grade_push(args: &[String]) -> Result<GitWriteTier, String> {
    let mut refspecs: Vec<&str> = Vec::new();
    let mut seen_double_dash = false;
    for token in args {
        if seen_double_dash {
            refspecs.push(token);
            continue;
        }
        if token == "--" {
            seen_double_dash = true;
            continue;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if let Some(pattern) = PUSH_DENIED_L3_LONGS
                    .iter()
                    .find(|pattern| long_option_overlaps(base, pattern))
                {
                    return Err(refuse_l3(format!(
                        "push {base:?} 属毁灭/绕过面 ({pattern} 族): 强推·远端删除·远端命令执行永不提供工具面"
                    )));
                }
                if long_option_overlaps(base, "--no-verify") {
                    return Err(refuse_morph(
                        "push --no-verify 绕过 pre-push 钩子 = 变形绕过面",
                    ));
                }
                if long_option_overlaps(base, "--signed")
                    || long_option_overlaps(base, "--gpg-sign")
                {
                    return Err(refuse_offlist("签名/密钥面不入本工具"));
                }
                if long_option_overlaps(base, "--push-option") {
                    return Err(refuse_morph(
                        "push --push-option 服务端钩子选项面不入本工具",
                    ));
                }
                continue;
            }
            for ch in short_bundle(token).unwrap_or_default() {
                if PUSH_DENIED_SHORTS.contains(&ch) {
                    return Err(refuse_l3(format!(
                        "push 短选项 {ch:?} 属毁灭面: 强推·远端删除·服务端钩子选项永不提供工具面"
                    )));
                }
            }
            continue;
        }
        refspecs.push(token);
    }
    for token in refspecs {
        check_push_refspec(token)?;
    }
    Ok(GitWriteTier::NetworkEgress)
}

/// push refspec 变形面: `+x` = 强推变体; 空侧 `:` = 远端引用删除变体。
fn check_push_refspec(token: &str) -> Result<(), String> {
    if token.starts_with('+') {
        return Err(refuse_l3(format!(
            "push refspec {token:?} 的 + 前缀 = 强推变体 (与 --force 同权)"
        )));
    }
    if let Some((source, destination)) = token.split_once(':') {
        if source.is_empty() || destination.is_empty() {
            return Err(refuse_l3(format!(
                "push refspec {token:?} 的空侧 = 远端引用删除变体 (与 --delete 同权)"
            )));
        }
    }
    Ok(())
}

/// fetch 的拒绝长选项族: 强制覆盖 / 远端命令执行 / 引用删除 (--prune 族)。
const FETCH_DENIED_L3_LONGS: &[&str] = &["--force", "--upload-pack", "--prune"];

fn grade_fetch(args: &[String]) -> Result<GitWriteTier, String> {
    let mut refspecs: Vec<&str> = Vec::new();
    let mut seen_double_dash = false;
    for token in args {
        if seen_double_dash {
            refspecs.push(token);
            continue;
        }
        if token == "--" {
            seen_double_dash = true;
            continue;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") {
                if let Some(pattern) = FETCH_DENIED_L3_LONGS
                    .iter()
                    .find(|pattern| long_option_overlaps(base, pattern))
                {
                    return Err(refuse_l3(format!(
                        "fetch {base:?} 属毁灭/绕过面 ({pattern} 族): 强制覆盖·远端命令执行·引用删除永不提供工具面"
                    )));
                }
                continue;
            }
            for ch in short_bundle(token).unwrap_or_default() {
                if ch == 'f' {
                    return Err(refuse_l3("fetch -f 强制覆盖远端跟踪引用 = 毁灭族"));
                }
            }
            continue;
        }
        refspecs.push(token);
    }
    for token in refspecs {
        check_fetch_refspec(token)?;
    }
    Ok(GitWriteTier::NetworkEgress)
}

/// fetch refspec 变形面: 空侧 `:` = 引用删除; 目的地是本地分支/标签 = 覆盖本地引用。
fn check_fetch_refspec(token: &str) -> Result<(), String> {
    let Some((source, destination)) = token.split_once(':') else {
        return Ok(());
    };
    if source.is_empty() || destination.is_empty() {
        return Err(refuse_l3(format!(
            "fetch refspec {token:?} 的空侧 = 引用删除变体 (与 update-ref -d 同权)"
        )));
    }
    if destination.starts_with("refs/heads/")
        || destination.starts_with("refs/tags/")
        || destination == "HEAD"
    {
        return Err(refuse_l3(format!(
            "fetch refspec {token:?} 的目的地是本地分支/标签 = 本地引用覆盖变体"
        )));
    }
    Ok(())
}

/// remote 只开远端操作面: add / rename / set-url / update。
fn grade_remote(args: &[String]) -> Result<GitWriteTier, String> {
    let mut verb: Option<String> = None;
    let mut positionals_after_verb = 0usize;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            for rest in &args[index + 1..] {
                if verb.is_none() {
                    verb = Some(rest.to_ascii_lowercase());
                } else {
                    positionals_after_verb += 1;
                }
            }
            break;
        }
        if token.starts_with('-') && token.len() > 1 {
            let base = option_base(token);
            if base.starts_with("--") && long_option_overlaps(base, "--mirror") {
                return Err(refuse_l3(
                    "remote --mirror 配置的是推送即删除的镜像面 = 毁灭族",
                ));
            }
            index += 1;
            continue;
        }
        if verb.is_none() {
            verb = Some(token.to_ascii_lowercase());
        } else {
            positionals_after_verb += 1;
        }
        index += 1;
    }
    match verb.as_deref() {
        Some("remove") | Some("rm") | Some("prune") => Err(refuse_l3(
            "remote remove/prune 删除远端或其跟踪引用 = 引用删除族 (与 update-ref -d 同权)",
        )),
        Some("add") | Some("rename") | Some("set-url") => {
            if positionals_after_verb < 2 {
                return Err(refuse_offlist(
                    "remote add/rename/set-url 要求完整位置参 (如 add <name> <url>)",
                ));
            }
            Ok(GitWriteTier::NetworkEgress)
        }
        Some("update") => Ok(GitWriteTier::NetworkEgress),
        _ => Err(refuse_offlist(
            "remote 只开远端操作面: add / rename / set-url / update (查询走 repo 工具)",
        )),
    }
}

// ---------------------------------------------------------------------------
// 执行面: 受控通道 (结构化 argv + 工作区限域 + 超时 + 输出归一)
// ---------------------------------------------------------------------------

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitWriteParams {
    /// 一条 git 写命令文本 (段间可用 `&&` / `;` / 换行)。
    pub command: String,
}

/// 一段执行结果的如实账目。
struct SegmentOutcome {
    display: String,
    output: String,
}

/// 执行分级计划: 逐段顺序落地, 中段失败如实上报 (无回滚承诺)。
fn execute_plan(root: &Path, segments: &[GradedSegment]) -> Result<String, String> {
    let executor = ProcessExecutor::new();
    let mut outcomes: Vec<SegmentOutcome> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let mut args = vec![
            std::ffi::OsString::from("-C"),
            root.as_os_str().to_os_string(),
        ];
        args.extend(segment.argv.iter().map(std::ffi::OsString::from));
        let request = ProcessRequest::new("git")
            .with_args(args)
            .with_working_directory(root)
            .with_limits(ProcessLimits::default());
        let result = executor
            .execute(&request)
            .map_err(|error| execution_failure_frame(segments, index, &format!("IO: {error}")))?;
        let mut output = String::new();
        if !result.stdout.is_empty() {
            output.push_str(&String::from_utf8_lossy(&result.stdout));
        }
        if !result.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&String::from_utf8_lossy(&result.stderr));
        }
        let output = redact_secret_surfaces(&output);
        if result.timed_out() {
            return Err(execution_failure_frame(
                segments,
                index,
                &format!(
                    "{} (段执行超过 {}ms)",
                    TimeoutErrorCode::DeadlineExpired.as_str(),
                    GIT_WRITE_TIMEOUT_MS
                ),
            ));
        }
        outcomes.push(SegmentOutcome {
            display: segment.display.clone(),
            output,
        });
        if !result.success() {
            let code = result
                .exit_code()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into());
            let detail = outcomes
                .last()
                .map(|outcome| outcome.output.as_str())
                .filter(|text| !text.is_empty())
                .unwrap_or("(无输出)");
            return Err(execution_failure_frame(
                segments,
                index,
                &format!("exit {code}: {detail}"),
            ));
        }
    }
    Ok(outcomes
        .into_iter()
        .map(|outcome| {
            if outcome.output.is_empty() {
                format!("$ {}", outcome.display)
            } else {
                format!("$ {}\n{}", outcome.display, outcome.output)
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// 失败帧: 如实报告已落地段 / 失败段 / 失败因 —— 不谎称回滚。
fn execution_failure_frame(
    segments: &[GradedSegment],
    failed_index: usize,
    detail: &str,
) -> String {
    let total = segments.len();
    let landed: Vec<&str> = segments[..failed_index]
        .iter()
        .map(|segment| segment.display.as_str())
        .collect();
    let landed_text = if landed.is_empty() {
        "无 (第一段即失败)".to_string()
    } else {
        format!("{} (顺序落地, 保留)", landed.join(" ; "))
    };
    format!(
        "git_write 执行中断 (如实上报, 无回滚承诺): 第 {}/{total} 段失败 ({}) — {detail}; \
         已落地段: {landed_text}",
        failed_index + 1,
        segments[failed_index].display
    )
}

/// 受控 git 写工具 (`tool.git_write`, 声明名 `git_write`)。
pub struct GitWriteTool {
    id: CapabilityId,
    root: PathBuf,
}

impl GitWriteTool {
    /// 绑定工作区根 (执行限域: `git -C <root>` + 进程工作目录同根)。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new(GIT_WRITE_CAPABILITY_ID).expect("valid capability id"),
            root: root.into(),
        }
    }

    /// 输出归一合同: 成功报告的形状冻结为四字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("tier", SchemaKind::String)
            .require("commands", SchemaKind::Array)
            .require("output", SchemaKind::String)
    }
}

#[async_trait]
impl ToolCapability for GitWriteTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "One graded git write command (segments joined with && or ;), e.g. `git add src/main.rs && git commit -m \"change\"`."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(GIT_WRITE_TOOL_NAME)
            .with_description(GIT_WRITE_TOOL_DESCRIPTION)
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = GIT_WRITE_TOOL_NAME;
        let params: GitWriteParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid git_write parameters: {error}"),
                )
                .with_name(name)
            }
        };
        let plan = classify_git_write(&params.command);
        let GitWritePlan::Graded { tier, segments } = plan else {
            let reason = match plan {
                GitWritePlan::Refused { reason } => reason,
                GitWritePlan::Graded { .. } => unreachable!("matched above"),
            };
            return ToolResult::permanent_error(&call.id, reason).with_name(name);
        };

        let root = self.root.clone();
        let segments_for_run = segments.clone();
        match tokio::task::spawn_blocking(move || execute_plan(&root, &segments_for_run)).await {
            Ok(Ok(output)) => ToolResult::ok(
                &call.id,
                serde_json::json!({
                    "status": "executed",
                    "tier": tier.label(),
                    "commands": segments
                        .iter()
                        .map(|segment| segment.display.as_str())
                        .collect::<Vec<_>>(),
                    "output": output,
                }),
            )
            .with_name(name),
            Ok(Err(failure)) => ToolResult::permanent_error(&call.id, failure).with_name(name),
            Err(error) => {
                ToolResult::permanent_error(&call.id, format!("git_write task failed: {error}"))
                    .with_name(name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 治理面: 授权映射 / 单调边界 / 分级风险映射
// ---------------------------------------------------------------------------

/// git 写风险档位 (设置档): 默认每次人批; 自动放行档只放 L1。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitWriteReleaseTier {
    /// 默认档: L1 / L2 都停人工审批 (L2 永远如此)。
    RequireApprovalEveryWrite,
    /// 「自动放行 L1」档: 只放可逆本地写; L2 照旧永远人工审批。
    AutoPassReversibleL1,
}

/// 授权映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与文件写入的风险映射同一语义: 本钩子只回答「这次调用是否有授权」,
/// 分级风险档位由 [`GitWriteTierApprovalHook`] 承载 (同一条审批链)。
pub struct GitWriteRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl GitWriteRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for GitWriteRiskMappingHook {
    fn name(&self) -> &str {
        "git_write_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(request.capability.as_str());
        match decision {
            Decision::Allow => PreVerdict::Allow,
            Decision::RequireApproval { reason } => PreVerdict::Ask { reason },
            Decision::Deny { .. } => PreVerdict::Ask {
                reason: format!(
                    "git-write tool {} sits at the require-approval level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): L3 毁灭级 / 词表外 / 变形面拒绝即帧。
///
/// 自动放行档放行的调用也照穿本 Guard —— 任何后续阶段都无法把这里的拒绝
/// 翻回放行 (单调性)。
pub struct GitWriteBoundaryGuard;

impl GitWriteBoundaryGuard {
    /// 空构造 (无状态 Guard)。
    pub fn new() -> Self {
        Self
    }
}

impl Default for GitWriteBoundaryGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard for GitWriteBoundaryGuard {
    fn name(&self) -> &str {
        "git_write_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != GIT_WRITE_CAPABILITY_ID {
            return None;
        }
        match git_write_plan_for_arguments(request.arguments) {
            GitWritePlan::Refused { reason } => Some(reason),
            GitWritePlan::Graded { .. } => None,
        }
    }
}

/// 分级风险映射 (治理层, 与本地审批面板 / IM 审批卡同链)。
///
/// - L1 可逆本地写: 默认档停 [`Decision::RequireApproval`] (理由带分级标注);
///   「自动放行 L1」档放行;
/// - L2 出网写: **永远** [`Decision::RequireApproval`] —— 无自动放行档;
/// - L3 毁灭级 / 词表外 / 变形面: [`Decision::deny`] 拒绝即帧 (无工具面)。
pub struct GitWriteTierApprovalHook {
    release: GitWriteReleaseTier,
}

impl GitWriteTierApprovalHook {
    /// 按设置档构造分级风险映射。
    pub fn new(release: GitWriteReleaseTier) -> Self {
        Self { release }
    }
}

#[async_trait]
impl GovernanceHook for GitWriteTierApprovalHook {
    fn name(&self) -> &str {
        "git_write_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != GIT_WRITE_CAPABILITY_ID {
            return Decision::Allow;
        }
        match git_write_plan_for_arguments(arguments) {
            GitWritePlan::Refused { reason } => Decision::deny(reason),
            GitWritePlan::Graded { tier, segments } => {
                let display = segments
                    .iter()
                    .map(|segment| segment.display.as_str())
                    .collect::<Vec<_>>()
                    .join(" ; ");
                match tier {
                    GitWriteTier::ReversibleLocal => match self.release {
                        GitWriteReleaseTier::AutoPassReversibleL1 => Decision::Allow,
                        GitWriteReleaseTier::RequireApprovalEveryWrite => {
                            Decision::require_approval(format!(
                                "git write tier L1 ({}): {display} — reversible local \
                                     repository write, human approval required by default \
                                     (auto-pass only via the explicit L1 settings tier)",
                                tier.surface()
                            ))
                        }
                    },
                    GitWriteTier::NetworkEgress => Decision::require_approval(format!(
                        "git write tier L2 ({}): {display} — network-bound repository write, \
                         always requires a human decision; L2 never auto-passes on any preset \
                         or setting",
                        tier.surface()
                    )),
                    GitWriteTier::Destructive => {
                        Decision::deny(refuse_l3("毁灭级永不进审批: 拒绝即帧 (无工具面)"))
                    }
                }
            }
        }
    }
}

/// 生产授权播种: 已注册的 git 写工具即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 分级风险档位不在工具策略里 —— 由 [`GitWriteTierApprovalHook`] 承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_git_write_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(apeireth_governance::Permission::ExecuteTool(
            GIT_WRITE_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装生产五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn git_write_pipeline(
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(GitWriteRiskMappingHook::new(policy)))
            .with_guard(Arc::new(GitWriteBoundaryGuard::new()))
            .with_around(
                AroundPolicy::new().with_timeout(GIT_WRITE_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(GitWriteTool::output_schema()),
    )
}

/// 组装生产工具面: 分级词表 + 五段流水线 (模型侧一个能力)。
pub fn git_write_capability(
    root: impl Into<PathBuf>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    let tool = Arc::new(GitWriteTool::new(root));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        git_write_pipeline(policy),
    ))
}
