//! `git_write` 的**词法面**（段切分 / 提词 / 逐段分级）—— 从 `git_write.rs`
//! **纯搬移**过来。
//!
//! 存在理由：`git_write.rs` 在 size 棘轮基线上（`arch-baseline.txt`，0 容差）
//! ⇒ 任何净增都会让 `arch_guard` 变红；棘轮文件**不归我改** ⇒ 正路是**让本文件净减**。
//!
//! `src/git_write.rs` + `src/git_write/parse.rs` 是 Rust 2018 的标准模块布局
//! ⇒ `mod parse;` 自动解析 ⇒ **不需要**改 `lib.rs`（别人的文件）。
//!
//! # 操作数路径面（2026-11-01 安全审计 P3-7 修）
//!
//! `git commit -F <路径>` / `git tag -F <路径>`（长形 `--file`）的**操作数是文件
//! 路径** ⇒ 原先**只置 `message_source` 标志、完全不校验路径** ⇒ 工作区外的文件
//! （`/etc/passwd` / `~/.aws/credentials`）能被读进提交/附注对象。
//!
//! 本模块的分工：
//!
//! - [`grade_commit`] / [`grade_tag`]：**捕获** `-F` / `--file` 的操作数（路径）；
//! - [`check_message_file_operands`]: 按**工作区根**做**包含性校验**（口径照抄
//!   `filesystem.rs` 的 `resolve_contained`：词法层 guardrail → canonicalize →
//!   `starts_with(root)` → 凭据面排除）⇒ 越界即拒绝即帧。

use std::fs;
use std::path::{Path, PathBuf};

use super::{refuse_l3, refuse_morph, refuse_offlist, GitWriteTier};

/// 段切分: 只认 `&&` / `;` / 换行为段分隔; shell 面 (管道/重定向/插值/分组)
/// 拒绝即帧 (本工具不经 shell, 不给"看起来像 shell"的幻觉空间)。
pub(super) fn split_command_segments(command: &str) -> Result<Vec<String>, String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some(open) => {
                current.push(ch);
                if ch == open {
                    quote = None;
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                ';' | '\n' | '\r' => segments.push(std::mem::take(&mut current)),
                '&' => {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        segments.push(std::mem::take(&mut current));
                    } else {
                        return Err(refuse_morph(
                            "单个 & (后台/句柄面) 不入本工具: 段分隔只认 && 与 ;",
                        ));
                    }
                }
                '|' | '<' | '>' => {
                    return Err(refuse_morph(
                        "管道/重定向面不入本工具: 命令按结构化参数执行 git, 不经 shell",
                    ))
                }
                '`' | '$' | '(' | ')' | '{' | '}' => {
                    return Err(refuse_morph(format!(
                        "shell 元字符 {ch:?} 不入本工具: 不做插值/分组/展开 (引号内按字面参数)"
                    )))
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err(refuse_morph("引号未闭合: 命令文本无法按参数面解析"));
    }
    segments.push(current);
    Ok(segments)
}

/// 段内提词: 引号成组去壳; 引号内字面保留 (无转义/无插值); 空引号仍是词。
pub(super) fn tokenize_segment(segment: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for ch in segment.chars() {
        match quote {
            Some(open) => {
                if ch == open {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    started = true;
                }
                whitespace if whitespace.is_whitespace() => {
                    if started || !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                _ => {
                    current.push(ch);
                    started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err(refuse_morph("引号未闭合: 命令文本无法按参数面解析"));
    }
    if started || !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

/// 长选项基名 (`--message=x` → `--message`)。
pub(super) fn option_base(token: &str) -> &str {
    match token.find('=') {
        Some(index) => &token[..index],
        None => token,
    }
}

/// 长选项前缀重叠命中 (变形防护): git 允许无歧义缩写, 故 `--ha` 是 `--hard`
/// 的同面变体, `--force-with-lease=x` 与 `--force` 同族 —— 两侧前缀重叠都算。
pub(super) fn long_option_overlaps(base: &str, pattern: &str) -> bool {
    base.starts_with(pattern) || pattern.starts_with(base)
}

/// 短选项束的字符面 (`-qf` → `q`,`f`); 长选项与裸 `-` 不算束。
pub(super) fn short_bundle(token: &str) -> Option<Vec<char>> {
    if token.starts_with("--") || token == "-" {
        return None;
    }
    token
        .strip_prefix('-')
        .map(|rest| rest.chars().collect::<Vec<char>>())
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

/// commit 的分级 (含**消息文件操作数**的捕获)。
///
/// `message_files` 出参: `-F` / `--file` 的**文件路径操作数**。原先这些操作数
/// **完全不受约束** ⇒ `git commit -F /etc/passwd` / `--file=/etc/passwd` 能把
/// **工作区外**的文件读进提交对象 ⇒ 现在由根感知分级面 (见 `git_write.rs` 的
/// `classify_git_write_under`) 按工作区根做包含性校验。
pub(super) fn grade_commit(
    args: &[String],
    message_files: &mut Vec<String>,
) -> Result<GitWriteTier, String> {
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
                // ⚠️ `--file` 的操作数是**文件路径**（长选项形）: git 允许无歧义缩写
                // （`--fil` ≡ `--file`）⇒ 命中前缀重叠即捕获。原先**只置标志**、
                // 路径**完全不受约束** ⇒ 工作区外的文件能进提交对象 ⇒ 捕获后交
                // 根感知分级面按工作区根做包含性校验（越界即拒绝即帧）。
                // 捕获时机: 必须**先于**下面的 `matches!` 分支（那个分支只在
                // 精确拼写 `--file` 时把 `index` 推进到操作数词; 缩写形 `--fil`
                // 不会推进 ⇒ 操作数一律按「选项的下一个词」取 `args[index + 1]`）。
                if long_option_overlaps(base, "--file") {
                    if let Some((_, value)) = token.split_once('=') {
                        message_files.push(value.to_string());
                    } else if let Some(value) = args.get(index + 1) {
                        message_files.push(value.clone());
                    }
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
            let bundle: Vec<char> = token[1..].chars().collect();
            let mut position = 0usize;
            while position < bundle.len() {
                let ch = bundle[position];
                position += 1;
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
                    // ⚠️ `-F` 的操作数是**文件路径**: 值可**附着**（`-F/etc/passwd`）
                    // 或**另起一词**（`-F /etc/passwd`）⇒ 两种拼写都捕获。原先
                    // 只置标志、路径完全不受约束 ⇒ 同上, 捕获后交根感知分级面校验。
                    if ch == 'F' {
                        let attached: String = bundle[position..].iter().collect();
                        if attached.is_empty() {
                            if let Some(value) = args.get(index + 1) {
                                message_files.push(value.clone());
                                index += 1;
                            }
                        } else {
                            message_files.push(attached);
                        }
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

/// tag 的分级 (含**附注文件操作数**的捕获)。
///
/// `message_files` 出参: `-F` / `--file` 的**文件路径操作数**（同 `grade_commit`:
/// 原先**完全不受约束** ⇒ `git tag -F ~/.aws/credentials` 能把工作区外的凭据文件
/// 读进附注对象 ⇒ 捕获后交根感知分级面按工作区根做包含性校验）。
pub(super) fn grade_tag(
    args: &[String],
    message_files: &mut Vec<String>,
) -> Result<GitWriteTier, String> {
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
                // ⚠️ `--file` 的操作数是**文件路径**（长选项形）⇒ 捕获（原先只置
                // 标志、路径完全不受约束 ⇒ 同 `grade_commit`, 交根感知分级面校验）。
                // 捕获时机: 必须**先于**下面的 `match` 分支（那个分支只在精确拼写
                // `--file` 时把 `index` 推进到操作数词）⇒ 操作数一律按「选项的下一个
                // 词」取 `args[index + 1]`。
                if long_option_overlaps(base, "--file") {
                    if let Some((_, value)) = token.split_once('=') {
                        message_files.push(value.to_string());
                    } else if let Some(value) = args.get(index + 1) {
                        message_files.push(value.clone());
                    }
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
            let bundle = short_bundle(token).unwrap_or_default();
            let mut position = 0usize;
            while position < bundle.len() {
                let ch = bundle[position];
                position += 1;
                match ch {
                    'd' | 'D' => {
                        return Err(refuse_l3("tag -d = 引用删除族 (与 update-ref -d 同权)"))
                    }
                    'f' => return Err(refuse_l3("tag -f = 强制覆盖既有 tag 引用")),
                    's' | 'u' => return Err(refuse_offlist("签名/密钥面不入本工具")),
                    'a' | 'q' => {}
                    'm' => break,
                    'F' => {
                        // ⚠️ `-F` 的操作数是**文件路径**（附着或另起一词）⇒ 捕获,
                        // 交根感知分级面按工作区根做包含性校验（越界即拒绝即帧）。
                        let attached: String = bundle[position..].iter().collect();
                        if attached.is_empty() {
                            if let Some(value) = args.get(index + 1) {
                                message_files.push(value.clone());
                                index += 1;
                            }
                        } else {
                            message_files.push(attached);
                        }
                        break;
                    }
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

/// 消息/附注**文件操作数**的包含性校验: 解析后必须落在**工作区根内**, 且不在
/// 凭据面上。
///
/// 口径照抄 `filesystem.rs` 的 `resolve_contained`（词法层 guardrail →
/// canonicalize → `starts_with(root)` → 凭据面排除）:
/// - **原先错在哪**: `-F` / `--file` 的操作数**完全不受约束** ⇒
///   `git commit -F /etc/passwd` / `git tag -F ~/.aws/credentials` 能把**工作区外**
///   的文件读进提交/附注对象;
/// - **现在为什么这样**: 按同一口径判越界 ⇒ 越界即拒绝即帧（不放行到执行面）;
///   凭据面（`sensitive_path` 的形状黑名单）同样 fail-closed。
pub(super) fn check_message_file_operands(root: &Path, operands: &[String]) -> Result<(), String> {
    if operands.is_empty() {
        // 无文件操作数 ⇒ 无需解析根 (工作区根不存在时也不该误伤普通命令)。
        return Ok(());
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("工作区根不可解析 ({}): {error}", root.display()))?;
    for requested in operands {
        if requested.trim().is_empty() {
            return Err("消息/附注文件路径必须非空".to_string());
        }
        let candidate = if Path::new(requested).is_absolute() {
            PathBuf::from(requested)
        } else {
            canonical_root.join(requested)
        };
        // 词法层 (相对穿越 / 绝对敏感系统路径) —— 与 `resolve_contained` 同口径。
        crate::guardrail::ToolGuardrail::verify_path_access(&canonical_root, &candidate)
            .map_err(|error| error.to_string())?;
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!(
                "消息/附注文件不可解析 (不存在或不可读): {}: {error}",
                candidate.display()
            )
        })?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "消息/附注文件 {} 落在工作区根之外",
                candidate.display()
            ));
        }
        // 凭据面排除 (fail-closed): 根内的凭据/密钥文件同样不可进提交/附注对象。
        if crate::sensitive_path::is_sensitive_path(&canonical_root, &canonical) {
            return Err(crate::sensitive_path::credential_surface_refusal());
        }
    }
    Ok(())
}
