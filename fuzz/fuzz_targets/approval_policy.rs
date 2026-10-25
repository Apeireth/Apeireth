#![no_main]

//! fuzz: **审批分档判定面** —— `approval_policy`（分档）+ `approval_family`（族归一）。
//!
//! ## 这个面为什么值得 fuzz
//!
//! 审批分档 = 给定「能力名 + JSON 参数 + 规则配置」，决定这次调用落在**放行 / 要人批 /
//! 拒**哪一档。**误放行 = 高危动作绕过审批；误拒 = 可用性事故。**
//! E-P2 把命令级名单从「逐字命令」升级为「语义族匹配」，族词表与变体匹配规则全部收在
//! 治理层 `approval_family`（**单一来源**），`approval_policy` 按族分档。
//!
//! 本仓历史已修的同族真缺陷（本次 fuzz 的重点参照）：
//!
//! * `crates/capabilities/tools/src/write_intent.rs`：原地改写特判手写 `w == "sed"`，
//!   **绕过**治理层的「可执行后缀」剥离 ⇒ `sed.exe -i` 漏放，而同面的族词路（`rm.exe`）
//!   能抓 ⇒ **面内不一致**（修法：复用治理层单一来源 `strip_executable_suffix`）；
//! * `crates/capabilities/tools/src/external.rs` + `recon_tool.rs`：typed 参数用
//!   `and_then(as_X).unwrap_or(default)` ⇒「没给」与「给了畸形值」混为一谈
//!   ⇒ 畸形值静默回落成默认值（fail-open）。
//!
//! ⇒ 本 target 的判据重心：**同族多条路径口径一致**（词面 / 归属面 / 触达面 / 分档面）、
//! **别名与主形同权**、**族级条目对同族所有形态生效**、**跨族不串**、
//! **畸形参数不静默回落**、**任何输入不 panic**。
//!
//! ## 被 fuzz 的 API（全部在 `apeireth_governance` 内）
//!
//! * `approval_policy`：`extract_commands` / `parse_approval_entry` / `is_high_risk` /
//!   `frequency_count` / `best_approval_match` / `ParsedApprovalEntry::target_for` /
//!   `ApprovalPolicyEngine::evaluate`；
//! * `approval_family`：`word_families` / `command_families` / `touched_families` /
//!   `member_calls` / `member_family` / `command_words` / `strip_executable_suffix` /
//!   `family_ids_for_arguments` / `interpret_recorded_command`。
//!
//! ## 契约来源（断言逐条指回下表 —— **不自行发明期望值**）
//!
//! | # | 契约 | 出处（文件:行 + 原文片段） |
//! |---|---|---|
//! | C1 | 变体防护：短形态逐字 / 前缀缩写 ≥5 字符 / 可执行后缀剥离 / 大小写不敏感 | `approval_family.rs:20-26`「**短形态**：短别名词表内逐字命中（`rm` / `rd` / `del` / `ri` …）；**前缀**：成员词头前缀命中（`Delete*` / `Write*` …）+ 命令名缩写命中（族词前缀 ≥5 字符，`Remove-I` ≡ `Remove-Item`；短于此不收，挡 `rem` / `set` / `stat` 类短词误伤）+ 可执行后缀剥离（`.exe` / `.cmd` / `.bat` / `.com` / `.ps1`）；**大小写**：一律 ASCII 不敏感。」 |
//! | C2 | 精确命中优先于缩写命中 | `approval_family.rs:26-28`「**精确命中优先于缩写命中**（`start` 精确判进程族，不因 `Start-Transcript` 缩写面误入文件写入族）。」 |
//! | C3 | 可执行后缀只剥一层 | `approval_family.rs:61-62`「可执行后缀（变体防护：`rm.exe` ≡ `rm`）。只剥一层。」 |
//! | C4 | 成员词头前缀命中（`::成员` / `.成员(` 两种形态） | `approval_family.rs:157-159`「脚本引擎成员调用面词头 → 族（大小写不敏感前缀命中）。覆盖 `::成员`（静态调用 / 方法组）与 `.成员(`（实例调用）两种形态；」 |
//! | C5 | 提词：词字符 = 字母数字 + `_` / `-` / `.`（其余是词边界） | `approval_family.rs:346-347`「提词：词字符 = 字母数字 + `_` / `-` / `.`（命令名与参数各成一词；文件名带点不拆词，`file.md` 不会误命中 `md`）。」 |
//! | C6 | 归属面 = 段首命令动词 + 成员调用（段内提到族词不算归属） | `approval_family.rs:13-16`「**归属面** [`attributed_families`]（授权对象用）：只认**段首命令动词**与**成员调用**——"段内提到某族词"不算授权归属（`echo del is a word` 不会拿到删除族授权），宁可归属不足（重批）不白拿授权；」 |
//! | C7 | 触达面 = 词面任意命中 + 成员调用（保守，宁可误拒不漏放） | `approval_family.rs:17-18`「**触达面** [`touched_families`]（审批门用）：词面任意命中 + 成员调用——宁可误拒不漏放（与写意图扫描同一保守口径，误拒面见 `write_intent.rs`）。」 |
//! | C8 | 参数面：走 `command` / `command1..N` 提取，逐条命令取**归属面**族并合并 | `approval_family.rs:467-470`「调用参数 → 授权族（族 id 词表序去重）：走 `command` / `command1..N` 提取（与命令级名单同一提取口径），逐条命令取归属面族并合并。空 = 非命令面调用（授权对象 = 操作本身，逐操作授权）。」 |
//! | C9 | 存量逐字记录 = **迁移生效**（命令 → 族映射后生效），词表外退回逐字对象 | `approval_family.rs:495-498`「存量逐字审批记录的**迁移解释**（E-P2 兼容口径，二选一钉死）：**迁移生效** —— 老记录的命令 → 族映射后生效（不是失效重批）。命令词表外（无法归族）→ 退回逐字命令对象（既有精确匹配语义）。」 |
//! | C10 | 族级条目对**触达面**相交的命令生效（保守触达、宁拒不漏） | `approval_policy.rs:202-204`「族级匹配（E-P2）：族对象条目对**触达面**相交的命令生效（保守触达、宁拒不漏——`echo del is a word` 也命中删除族条目，误拒面同写意图口径）；存量逐字条目按迁移语义归族后走同一族级匹配，词表外命令退回逐字精确匹配。」 |
//! | C11 | 名单条目对象三态：`Tool` → 能力；`Tool:family:<族id>` → 族；`Tool:<命令>` → 迁移语义按族 | `approval_policy.rs:96-103`「* `Tool` → [`ApprovalTarget::Capability`]（整个能力）；* `Tool:family:<族id>` → [`ApprovalTarget::Families`]（族级对象；未知族 id **不给族级语义**——退回逐字对象，宁可无效不放宽）；* `Tool:<命令>`（存量逐字记录）→ **迁移语义 = 按族解释**」 |
//! | C12 | 最佳匹配：命令/族级（specificity 2）压过工具级（1）；同档位静默条目胜出 | `approval_policy.rs:199-200`「Best `approvalList` match: command/family-level (specificity 2) beats tool-level (specificity 1); at equal specificity a silent entry wins.」 |
//! | C13 | 判定顺序：Blacklist → Trust → Frequency → Risk → ApprovalList → Whitelist，首个终止匹配胜出 | `approval_policy.rs:301-302`「Evaluate in baseline order: Blacklist → Trust → Frequency → Risk → ApprovalList → Whitelist. First terminal match wins.」 |
//! | C14 | 命令提取：`command` 在前，`command1..N` 按数字升序 | `approval_policy.rs:119`「Extract `command` then `command1..N` (numeric ascending) from JSON args.」 |
//! | C15 | 条目解析：空串 / 只有静默后缀的串**跳过** | `approval_policy.rs:151`「Parse one approval-list entry. Empty / suffix-only strings are skipped.」 |
//! | C16 | 风险前缀 = 前缀匹配（大小写不敏感） | `approval_policy.rs:173`「Prefix match used by the baseline RiskRule (`system.exec`, `file_write`, …).」 + `approval_policy.rs:439-443` 测试「risk_prefix_is_case_insensitive」 |
//! | C17 | 频率面：同能力、窗口内 `(now - window, now]` 的记录计数 | `approval_policy.rs:181`「Count same-capability records inside `(now - window, now]`.」 |
//! | C18 | 词表外命令退回逐字精确匹配（既有语义不破） | `approval_policy.rs:596-598`「**E-P2 边界**：词表外的存量逐字条目退回逐字精确匹配（既有语义不破）；未知族 id 的显式族条目**不给族级语义**（宁可无效不放宽，族 id 由 `CommandFamily` 词表 API 给，杜绝手写打错字）。」 |
//!
//! ## 已知**不**断言的边界（避免自造期望值 → 假阳性）
//!
//! * **族外词**（`deltree` / `git` / `python -c` …）：族词表是**冻结面**
//!   （`approval_family.rs:37-40`「审批族冻结词表…新族必须显式入表并补测试」），
//!   族外词按设计不入表 ⇒ **不断言「必须命中」**，只断言「不得被族条目误挡」。
//! * **频率窗口左端点**：文档写 `(now - window, now]`（左开），实现与 donor 移植源
//!   （`legacy/donor/apeireth-tool-approval/src/rule.rs:248` `r.timestamp_ms >= now_ms - self.window_ms as i64`）
//!   一致为**左闭** ⇒ 左端点归属不唯一，**不断言**该边界点；只断言窗口内/外/未来/异能力四个 unambigous 方向。
//! * **`command0` / 超大序号键**：`extract_commands` 的实现口径是「`command` + ASCII 数字且能解析为 u64」
//!   （`approval_policy.rs:135-138`），超出 u64 的键被静默跳过 —— 文档未写清该边界，**不断言**。

use apeireth_governance::approval_family::{
    command_families, command_words, family_ids_for_arguments, interpret_recorded_command,
    member_calls, member_family, strip_executable_suffix, touched_families, word_families,
    CommandFamily, MemberCallForm, MEMBER_PREFIX_FAMILIES, MIN_ABBREVIATION_CHARS,
};
use apeireth_governance::approval_policy::{
    best_approval_match, extract_commands, frequency_count, is_high_risk, parse_approval_entry,
    ApprovalPolicyEngine, CallRecord, ParsedApprovalEntry, APPROVAL_TIMEOUT_MS,
};
use apeireth_governance::Decision;
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};
use std::sync::OnceLock;

// ─────────────────────────────────────────────────────────────────────────────
// 冻结面副本（库里的表若被改动，这里会炸 —— 冻结面是契约的一部分）
// ─────────────────────────────────────────────────────────────────────────────

/// 可执行后缀（C1 / C3；`approval_family.rs:62`）。
const EXEC_SUFFIXES: [&str; 5] = [".exe", ".cmd", ".bat", ".com", ".ps1"];

/// 段分隔符（C5 / C6 / C7；`approval_family.rs:65`）。
const SEGMENT_SEPARATORS: [char; 9] = ['|', '&', ';', '\n', '\r', '(', ')', '{', '}'];

/// 缩写最短字符数（C1；`approval_family.rs:59`）。
const ABBREV_MIN: usize = 5;

/// 成员词头表副本（C4；`approval_family.rs:160-177`）。
const MEMBER_PREFIX_TABLE: [(CommandFamily, &str); 16] = [
    (CommandFamily::Delete, "delete"),
    (CommandFamily::MoveCopy, "move"),
    (CommandFamily::MoveCopy, "copy"),
    (CommandFamily::MoveCopy, "rename"),
    (CommandFamily::Create, "create"),
    (CommandFamily::WriteContent, "write"),
    (CommandFamily::WriteContent, "append"),
    (CommandFamily::WriteContent, "replace"),
    (CommandFamily::WriteContent, "set"),
    (CommandFamily::WriteContent, "encrypt"),
    (CommandFamily::WriteContent, "decrypt"),
    (CommandFamily::Network, "download"),
    (CommandFamily::Network, "upload"),
    (CommandFamily::Process, "kill"),
    (CommandFamily::Process, "start"),
    (CommandFamily::Process, "stop"),
];

/// 静默拒绝后缀（`approval_policy.rs:48`）。
const SILENT_SUFFIX: &str = "::SilentReject";

/// 族级条目标记（`approval_policy.rs:68`）。
const FAMILY_MARKER: &str = "family:";

/// 分档判定的能力名（**刻意选成非高危前缀**，好让分档断言只落在名单面上）。
const CAP: &str = "tool.shell";

// ─────────────────────────────────────────────────────────────────────────────
// 文档口径模型（按 C1..C7 重写一遍，与实现对照 = 面内一致性检查）
// ─────────────────────────────────────────────────────────────────────────────

/// 剥一层可执行后缀（C3）。
fn model_strip_suffix(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    for suffix in EXEC_SUFFIXES {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return word[..stripped.len()].to_string();
        }
    }
    word.to_string()
}

/// 精确命中的族（C2）：词（剥后缀 + ASCII 小写）逐字等于某族词。
fn exact_families(word: &str) -> Vec<CommandFamily> {
    let normalized = model_strip_suffix(word).to_ascii_lowercase();
    CommandFamily::ALL
        .iter()
        .copied()
        .filter(|family| {
            family
                .words()
                .iter()
                .any(|family_word| family_word.to_ascii_lowercase() == normalized)
        })
        .collect()
}

/// 命令词 → 族（C1 + C2）：精确命中优先，否则收 ≥5 字符前缀缩写。
fn model_word_families(word: &str) -> Vec<CommandFamily> {
    let normalized = model_strip_suffix(word).to_ascii_lowercase();
    let exact: Vec<CommandFamily> = CommandFamily::ALL
        .iter()
        .copied()
        .filter(|family| {
            family
                .words()
                .iter()
                .any(|family_word| family_word.to_ascii_lowercase() == normalized)
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    if normalized.chars().count() < ABBREV_MIN {
        return Vec::new();
    }
    CommandFamily::ALL
        .iter()
        .copied()
        .filter(|family| {
            family
                .words()
                .iter()
                .any(|family_word| family_word.to_ascii_lowercase().starts_with(&normalized))
        })
        .collect()
}

/// 提词（C5）：词字符 = 字母数字 + `_` / `-` / `.`，其余是词边界。
fn model_command_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in segment.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            current.push(ch);
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// 成员名（`approval_family.rs:364-380`）：ASCII 字母/下划线开头，ASCII 字母数字/下划线续。
fn model_ident_at(chars: &[char], start: usize) -> Option<String> {
    let first = *chars.get(start)?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let mut end = start + 1;
    while let Some(&ch) = chars.get(end) {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end += 1;
        } else {
            break;
        }
    }
    Some(chars[start..end].iter().collect())
}

/// 成员调用（C4）：`::成员`（静态）与 `.成员(`（实例），左到右按出现序。
fn model_member_calls(command: &str) -> Vec<(String, MemberCallForm)> {
    let chars: Vec<char> = command.chars().collect();
    let mut calls = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' && chars.get(i + 1) == Some(&':') {
            let mut j = i + 2;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if let Some(name) = model_ident_at(&chars, j) {
                calls.push((name, MemberCallForm::Static));
            }
            i += 2;
            continue;
        }
        if chars[i] == '.' {
            if let Some(name) = model_ident_at(&chars, i + 1) {
                let mut k = i + 1 + name.chars().count();
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if chars.get(k) == Some(&'(') {
                    calls.push((name, MemberCallForm::Instance));
                }
            }
        }
        i += 1;
    }
    calls
}

/// 成员名 → 族（C4）：成员词头表前缀命中（大小写不敏感），首个命中者胜出。
fn model_member_family(name: &str) -> Option<CommandFamily> {
    let lower = name.to_ascii_lowercase();
    MEMBER_PREFIX_TABLE
        .iter()
        .find(|(_, prefix)| lower.starts_with(prefix))
        .map(|(family, _)| *family)
}

/// 触达面模型（C7）：词面任意命中 + 成员调用。
fn model_touched(command: &str) -> Vec<CommandFamily> {
    let mut families: Vec<CommandFamily> = Vec::new();
    for (name, _) in model_member_calls(command) {
        if let Some(family) = model_member_family(&name) {
            push_family(&mut families, family);
        }
    }
    for segment in split_segments(command) {
        for word in model_command_words(&segment) {
            for family in model_word_families(&word) {
                push_family(&mut families, family);
            }
        }
    }
    sort_families(&mut families);
    families
}

/// 归属面模型（C6）：段首命令动词 + 成员调用。
fn model_attributed(command: &str) -> Vec<CommandFamily> {
    let mut families: Vec<CommandFamily> = Vec::new();
    for (name, _) in model_member_calls(command) {
        if let Some(family) = model_member_family(&name) {
            push_family(&mut families, family);
        }
    }
    for segment in split_segments(command) {
        if let Some(head) = model_command_words(&segment).into_iter().next() {
            for family in model_word_families(&head) {
                push_family(&mut families, family);
            }
        }
    }
    sort_families(&mut families);
    families
}

/// 按段分隔符切段（C5 / C6 / C7）。
fn split_segments(command: &str) -> Vec<String> {
    command
        .split(|c: char| SEGMENT_SEPARATORS.contains(&c))
        .map(str::to_string)
        .collect()
}

fn push_family(families: &mut Vec<CommandFamily>, family: CommandFamily) {
    if !families.contains(&family) {
        families.push(family);
    }
}

fn sort_families(families: &mut [CommandFamily]) {
    families.sort_by_key(|family| {
        CommandFamily::ALL
            .iter()
            .position(|candidate| candidate == family)
            .unwrap_or(usize::MAX)
    });
}

fn family_ids(families: &[CommandFamily]) -> Vec<String> {
    families.iter().map(|family| family.id().to_string()).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// 条目面模型（C11 / C12 / C15）
// ─────────────────────────────────────────────────────────────────────────────

/// 条目对象模型（C11）。
#[derive(Debug)]
enum ModelTarget {
    Capability,
    Families(Vec<CommandFamily>),
    Command(String),
}

/// 条目解析模型（C15）：空串 / 只有静默后缀的串跳过。
fn model_parse_entry(entry: &str) -> Option<(String, String, bool)> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }
    let silent = trimmed.ends_with(SILENT_SUFFIX);
    let base = if silent {
        trimmed[..trimmed.len() - SILENT_SUFFIX.len()].trim()
    } else {
        trimmed
    };
    if base.is_empty() {
        return None;
    }
    Some((trimmed.to_string(), base.to_string(), silent))
}

/// 条目对象模型（C11 + C9）。
fn model_target_for(base: &str, capability: &str) -> Option<ModelTarget> {
    if base == capability {
        return Some(ModelTarget::Capability);
    }
    let rest = base.strip_prefix(capability)?.strip_prefix(':')?;
    if let Some(id) = rest.strip_prefix(FAMILY_MARKER) {
        return Some(match CommandFamily::from_id(id) {
            Some(family) => ModelTarget::Families(vec![family]),
            None => ModelTarget::Command(rest.to_string()),
        });
    }
    let families = command_families(rest);
    if families.is_empty() {
        Some(ModelTarget::Command(rest.to_string()))
    } else {
        Some(ModelTarget::Families(families))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 词表矩阵（同族形态 / 跨族隔离）
// ─────────────────────────────────────────────────────────────────────────────

/// 每个族的一条**只触达该族**的命令（跨族隔离检查用；由模型算出，不手写期望）。
fn solo_command(family: CommandFamily) -> Option<String> {
    CommandFamily::ALL
        .iter()
        .copied()
        .find(|candidate| *candidate == family)
        .and_then(|family| {
            family.words().iter().find_map(|word| {
                let candidate = (*word).to_string();
                let touched = model_touched(&candidate);
                if touched.len() == 1 && touched[0] == family {
                    Some(candidate)
                } else {
                    None
                }
            })
        })
}

/// 同族形态矩阵（C1 / C2 / C6 / C7 / C10）：
/// 同族的不同形态必须落到**同一档**（别名与主形同权），且族级条目对所有形态生效。
fn same_family_matrix() -> &'static [(CommandFamily, &'static [&'static str])] {
    static MATRIX: OnceLock<Vec<(CommandFamily, &'static [&'static str])>> = OnceLock::new();
    MATRIX.get_or_init(|| {
        vec![
            (
                CommandFamily::Delete,
                &[
                    "rm x.txt",
                    "rm.exe x.txt",
                    "del x.txt",
                    "del.cmd x.txt",
                    "rd x_dir",
                    "ri x.txt",
                    "Remove-Item x.txt",
                    "Remove-I x.txt",
                    "REMOVE-ITEM.EXE x.txt",
                    "remove-item.cmd x.txt",
                    "REMOVE-ITEM x.txt",
                    "[System.IO.File]::Delete('x.txt')",
                    "[system.io.file]::delete('x.txt')",
                    "([System.IO.FileInfo]'x.txt').Delete()",
                    "$fs.DeleteFile('x.txt')",
                ],
            ),
            (
                CommandFamily::MoveCopy,
                &[
                    "Move-Item a b",
                    "mv a b",
                    "ren a b",
                    "Rename-Item a b",
                    "cpi a b",
                    "cp a b",
                    "copy a b",
                    "move a b",
                    "MOVE-ITEM.EXE a b",
                    "$obj.CopyTo('x')",
                ],
            ),
            (
                CommandFamily::Create,
                &[
                    "mkdir d",
                    "md d",
                    "touch f",
                    "New-Item f",
                    "New-TemporaryFile f",
                    "install f",
                    "ni f",
                    "[System.IO.Directory]::Create('d')",
                    "$obj.Create('x')",
                ],
            ),
            (
                CommandFamily::WriteContent,
                &[
                    "Set-Content x.txt hello",
                    "Add-Content x.txt hello",
                    "Out-File x.txt",
                    "Clear-Content x.txt",
                    "Set-Item x.txt hello",
                    "Export-Csv x.csv",
                    "tee x.txt",
                    "dd if=a of=b",
                    "truncate x.txt",
                    "[System.IO.File]::WriteAllText('x','y')",
                    "[System.IO.File]::AppendAllText('x','y')",
                    "$obj.Append('x')",
                ],
            ),
            (
                CommandFamily::Network,
                &[
                    "curl http://example.invalid",
                    "wget http://example.invalid",
                    "Invoke-WebRequest http://example.invalid",
                    "Invoke-RestMethod http://example.invalid",
                    "Start-BitsTransfer http://example.invalid",
                    "bitsadmin /transfer x",
                    "ftp h",
                    "sftp h",
                    "scp a b",
                    "ssh h",
                    "rsync a b",
                    "nc h 1",
                    "ncat h 1",
                    "telnet h",
                    "iwr http://example.invalid",
                ],
            ),
            (
                CommandFamily::Process,
                &[
                    "start notepad",
                    "stop proc",
                    "kill 1",
                    "taskkill /PID 1",
                    "wmic x",
                    "nssm install x",
                    "Start-Process notepad",
                    "Stop-Process x",
                    "Invoke-Expression cmd",
                    "iex cmd",
                ],
            ),
        ]
    })
    .as_slice()
}

/// 一次性自检：族词变体矩阵 + 同族形态矩阵 + 跨族隔离。
///
/// 断言全部来自 C1 / C2 / C3 / C10 / C11 的文档承诺，**期望值由冻结词表算出**，
/// 不是手写常量。
fn run_once_matrix() {
    // 冻结面副本与库内表一致（族词表与成员词头表都是冻结面）。
    assert_eq!(
        MEMBER_PREFIX_FAMILIES.len(),
        MEMBER_PREFIX_TABLE.len(),
        "成员词头表长度漂移（冻结面）"
    );
    for (index, (family, prefix)) in MEMBER_PREFIX_FAMILIES.iter().enumerate() {
        assert_eq!(
            *family,
            MEMBER_PREFIX_TABLE[index].0,
            "成员词头表族漂移（冻结面）: {prefix}"
        );
        assert_eq!(
            *prefix,
            MEMBER_PREFIX_TABLE[index].1,
            "成员词头表词头漂移（冻结面）: {prefix}"
        );
    }
    assert_eq!(
        MIN_ABBREVIATION_CHARS, ABBREV_MIN,
        "缩写最短字符数漂移（冻结面）"
    );

    // C1 / C2 / C3：族词的大小写 / 可执行后缀 / ≥5 字符前缀缩写变体。
    for family in CommandFamily::ALL {
        for word in family.words() {
            let forms = vec![
                (*word).to_string(),
                word.to_ascii_lowercase(),
                word.to_ascii_uppercase(),
                format!("{word}.exe"),
                format!("{word}.cmd"),
                format!("{word}.bat"),
                format!("{word}.com"),
                format!("{word}.ps1"),
            ];
            for form in forms {
                let hit = word_families(&form);
                assert!(
                    hit.contains(&family),
                    "族词变体必须同族命中: 族={} 形态={form:?} 命中={hit:?}",
                    family.id()
                );
                assert_eq!(
                    hit,
                    model_word_families(&form),
                    "族词变体判定与文档口径不一致: 族={} 形态={form:?}",
                    family.id()
                );
            }
            // ≥5 字符前缀缩写（C1）；精确命中优先（C2）；短于此不收（C1 的
            // 「挡 `rem` / `set` / `stat` 类短词误伤」条款）。这里**穷举**每个
            // 族词的 1..=len 前缀与前缀+后缀组合，期望值由文档口径模型算出。
            for take in 1..=word.chars().count() {
                let prefix: String = word.chars().take(take).collect();
                for form in [
                    prefix.clone(),
                    format!("{prefix}.exe"),
                    format!("{prefix}.cmd"),
                    format!("{prefix}.bat"),
                    format!("{prefix}.com"),
                    format!("{prefix}.ps1"),
                ] {
                    let hit = word_families(&form);
                    assert_eq!(
                        hit,
                        model_word_families(&form),
                        "前缀缩写判定与文档口径不一致（≥5 字符才收 / 精确优先）: 族={} 形态={form:?}",
                        family.id()
                    );
                    let exact = exact_families(&prefix);
                    if exact.is_empty() {
                        if take >= ABBREV_MIN {
                            // 缩写命中族词所在族（该族词即以该前缀开头 ⇒ 同族）。
                            assert!(
                                hit.contains(&family),
                                "≥5 字符前缀缩写必须同族命中: 族={} 形态={form:?} 命中={hit:?}",
                                family.id()
                            );
                        }
                    } else {
                        // C2：精确命中优先于缩写命中。
                        assert_eq!(
                            hit, exact,
                            "精确命中必须优先于缩写命中: 形态={form:?} 精确族={:?}",
                            family_ids(&exact)
                        );
                    }
                }
            }
        }
    }

    // C10 / C6 / C7：族级条目对同族所有形态生效（同族不同形态同档）。
    for (family, forms) in same_family_matrix() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.approval_list.push(ParsedApprovalEntry::family_entry(
            CAP,
            *family,
            false,
        ));
        for &form in forms.iter() {
            let args = json!({ "command": form });
            let (decision, detail) = engine.evaluate(CAP, &args, &[], 0);
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "族级条目必须挡住同族所有形态（别名与主形同权）: 族={} 形态={form:?} 判定={decision:?}",
                family.id()
            );
            assert_eq!(
                detail.matched_family,
                vec![family.id().to_string()],
                "命中族标注必须逐字等于族 id: 族={} 形态={form:?}",
                family.id()
            );
            assert_eq!(detail.matched_rule.as_deref(), Some("approval_list"));
        }
    }

    // C10：跨族不串（批删除族 ≠ 批网络族）。
    for family in CommandFamily::ALL {
        let Some(solo) = solo_command(family) else {
            continue;
        };
        for other in CommandFamily::ALL.iter().copied().filter(|f| *f != family) {
            let mut engine = ApprovalPolicyEngine::new();
            engine.approval_list.push(ParsedApprovalEntry::family_entry(
                CAP,
                other,
                false,
            ));
            let args = json!({ "command": solo });
            let (decision, detail) = engine.evaluate(CAP, &args, &[], 0);
            assert!(
                !matches!(decision, Decision::RequireApproval { .. }),
                "族级条目不得越族挡人: 条目族={} 命令只触达={} 命令={solo:?} 判定={decision:?}",
                other.id(),
                family.id()
            );
            assert!(
                detail.matched_family.is_empty(),
                "越族命中不得回传族标注: {detail:?}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 词面 / 命令面检查
// ─────────────────────────────────────────────────────────────────────────────

/// 词面检查（C1 / C2 / C3 / C4）。
fn check_word_surface(word: &str) {
    // C3：只剥一层可执行后缀。
    assert_eq!(
        strip_executable_suffix(word),
        model_strip_suffix(word),
        "可执行后缀剥离口径不一致: {word:?}"
    );
    // C1 + C2：词 → 族。
    let hit = word_families(word);
    assert_eq!(
        hit,
        model_word_families(word),
        "词 → 族 判定与文档口径不一致（变体 / 精确优先）: {word:?}"
    );
    assert!(
        hit.iter().all(|family| CommandFamily::ALL.contains(family)),
        "命中的族必须在冻结词表内: {word:?} => {hit:?}"
    );
    // 确定性。
    assert_eq!(word_families(word), hit, "同一输入两次判定不一致: {word:?}");

    // C4：成员词头 → 族。
    let member = member_family(word);
    assert_eq!(
        member,
        model_member_family(word),
        "成员词头 → 族 判定与文档口径不一致: {word:?}"
    );
}

/// 命令面检查（C4 / C5 / C6 / C7）。
fn check_command_surface(command: &str) {
    // C5：提词。
    for segment in split_segments(command) {
        assert_eq!(
            command_words(&segment),
            model_command_words(&segment),
            "提词口径不一致: 段={segment:?}"
        );
    }
    // C4：成员调用。
    let calls = member_calls(command);
    let model_calls = model_member_calls(command);
    assert_eq!(
        calls.len(),
        model_calls.len(),
        "成员调用条数不一致: {command:?}"
    );
    for (call, (name, form)) in calls.iter().zip(model_calls.iter()) {
        assert_eq!(&call.name, name, "成员名不一致: {command:?}");
        assert_eq!(call.form, *form, "成员调用形态不一致: {command:?}");
    }
    // C7：触达面。
    let touched = touched_families(command);
    assert_eq!(
        touched,
        model_touched(command),
        "触达面判定与文档口径不一致（词面任意命中 + 成员调用）: {command:?}"
    );
    // C6：归属面。
    let attributed = command_families(command);
    assert_eq!(
        attributed,
        model_attributed(command),
        "归属面判定与文档口径不一致（段首动词 + 成员调用）: {command:?}"
    );
    // C6 ⊆ C7：触达面是归属面的超集（保守口径，宁可误拒不漏放）。
    for family in &attributed {
        assert!(
            touched.contains(family),
            "触达面必须覆盖归属面（保守口径）: {command:?} 归属={:?} 触达={:?}",
            family_ids(&attributed),
            family_ids(&touched)
        );
    }
    // 确定性。
    assert_eq!(
        touched_families(command),
        touched,
        "同一输入两次判定不一致: {command:?}"
    );
    assert_eq!(
        command_families(command),
        attributed,
        "同一输入两次判定不一致: {command:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 分档面检查（C10 / C11 / C12 / C13）
// ─────────────────────────────────────────────────────────────────────────────

/// 族级条目 × 命令形态：命中 ⟺ 触达面相交（C10）。
fn check_family_entry_gate(family: CommandFamily, command: &str) {
    let mut engine = ApprovalPolicyEngine::new();
    engine.approval_list.push(ParsedApprovalEntry::family_entry(
        CAP,
        family,
        false,
    ));
    let args = json!({ "command": command });
    let (decision, detail) = engine.evaluate(CAP, &args, &[], 0);
    let touched = touched_families(command);
    if touched.contains(&family) {
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "族级条目必须挡住触达同族的命令: 族={} 命令={command:?} 触达={:?} 判定={decision:?}",
            family.id(),
            family_ids(&touched)
        );
        assert_eq!(detail.matched_rule.as_deref(), Some("approval_list"));
        assert_eq!(
            detail.matched_family,
            vec![family.id().to_string()],
            "命中族标注必须逐字等于族 id: 命令={command:?}"
        );
        // 族级命中必须回传被挡下的命令与审批时限。
        assert_eq!(
            detail.matched_command.as_deref(),
            Some(command),
            "族级命中必须回传被挡下的命令: 命令={command:?}"
        );
        assert_eq!(
            detail.timeout_ms,
            Some(APPROVAL_TIMEOUT_MS),
            "要求审批的档位必须带审批时限: 命令={command:?}"
        );
        assert!(
            decision
                .reason()
                .is_some_and(|reason| !reason.trim().is_empty()),
            "要求审批必须给出可读理由: 命令={command:?}"
        );
    } else {
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "族级条目不得挡下不触达该族的命令: 族={} 命令={command:?} 触达={:?} 判定={decision:?}",
            family.id(),
            family_ids(&touched)
        );
        assert!(
            detail.matched_family.is_empty(),
            "非族级命中不得回传族标注: {detail:?}"
        );
    }
    // 确定性（C13 的可复核面）。
    let (decision2, detail2) = engine.evaluate(CAP, &args, &[], 0);
    assert_eq!(decision, decision2, "同一输入两次判定不一致: {command:?}");
    assert_eq!(detail, detail2, "同一输入两次判定不一致: {command:?}");
}

/// 六族条目齐备时的分档：命中 ⟺ 至少一族触达（C10）。
fn check_all_family_entries(command: &str) {
    let mut engine = ApprovalPolicyEngine::new();
    for family in CommandFamily::ALL {
        engine.approval_list.push(ParsedApprovalEntry::family_entry(
            CAP,
            family,
            false,
        ));
    }
    let args = json!({ "command": command });
    let (decision, detail) = engine.evaluate(CAP, &args, &[], 0);
    let touched = touched_families(command);
    if touched.is_empty() {
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "族外命令不得被族条目挡下（族词表是冻结面，族外词按设计不入表）: 命令={command:?} 判定={decision:?}"
        );
        assert!(detail.matched_family.is_empty(), "{detail:?}");
    } else {
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "触达任一族即必须审批: 命令={command:?} 触达={:?} 判定={decision:?}",
            family_ids(&touched)
        );
        assert_eq!(detail.matched_rule.as_deref(), Some("approval_list"));
        assert!(
            !detail.matched_family.is_empty(),
            "族级命中必须回传族标注: {detail:?}"
        );
        for id in &detail.matched_family {
            assert!(
                touched
                    .iter()
                    .any(|family| family.id() == id),
                "回传的族必须在触达面内: 命令={command:?} 回传={id} 触达={:?}",
                family_ids(&touched)
            );
        }
    }
}

/// 条目面：解析 / 对象 / 最佳匹配 / 分档（C11 / C12 / C15 / C18 / C9）。
fn check_entry_surface(raw_entry: &str, capability: &str, command: &str) {
    // C15：解析模型。
    let parsed = parse_approval_entry(raw_entry);
    let model = model_parse_entry(raw_entry);
    match (&parsed, &model) {
        (None, None) => {}
        (Some(parsed), Some((raw, base, silent))) => {
            assert_eq!(parsed.raw, *raw, "条目原文口径不一致: {raw_entry:?}");
            assert_eq!(parsed.base, *base, "条目基底口径不一致: {raw_entry:?}");
            assert_eq!(parsed.silent, *silent, "静默后缀口径不一致: {raw_entry:?}");
        }
        _ => panic!("条目解析口径不一致: {raw_entry:?} impl={parsed:?} model={model:?}"),
    }

    let Some(parsed) = parsed else {
        return;
    };

    // C11：条目对象模型。
    let target = parsed.target_for(capability);
    let model_target = model_target_for(&parsed.base, capability);
    match (&target, &model_target) {
        (None, None) => {}
        (Some(target), Some(model_target)) => {
            match (target, model_target) {
                (
                    apeireth_governance::approval_family::ApprovalTarget::Capability,
                    ModelTarget::Capability,
                ) => {}
                (
                    apeireth_governance::approval_family::ApprovalTarget::Families(families),
                    ModelTarget::Families(model_families),
                ) => {
                    assert_eq!(
                        family_ids(families),
                        family_ids(model_families),
                        "族级条目对象口径不一致: {raw_entry:?}"
                    );
                }
                (
                    apeireth_governance::approval_family::ApprovalTarget::Command(text),
                    ModelTarget::Command(model_text),
                ) => {
                    assert_eq!(
                        text, model_text,
                        "逐字条目对象口径不一致: {raw_entry:?}"
                    );
                }
                _ => panic!("条目对象口径不一致: {raw_entry:?} impl={target:?} model={model_target:?}"),
            }
        }
        _ => panic!("条目对象口径不一致: {raw_entry:?} impl={target:?} model={model_target:?}"),
    }

    // C12：最佳匹配（单条目模型：能力级 → 命中且无命令；族/逐字级 → 命中且回传命令）。
    let commands = vec![command.to_string()];
    let matched = best_approval_match(std::slice::from_ref(&parsed), capability, &commands);
    match &model_target {
        None => assert!(
            matched.is_none(),
            "条目不适用于该能力时不得命中: {raw_entry:?} 能力={capability}"
        ),
        Some(ModelTarget::Capability) => {
            let (entry, matched_command) = matched.unwrap_or_else(|| {
                panic!("能力级条目必须命中: {raw_entry:?} 能力={capability}")
            });
            assert_eq!(entry.base, parsed.base);
            assert!(matched_command.is_none(), "能力级命中不得回传命令");
        }
        Some(ModelTarget::Families(families)) => {
            let touched = touched_families(command);
            let hit = families.iter().any(|family| touched.contains(family));
            if hit {
                let (entry, matched_command) = matched.unwrap_or_else(|| {
                    panic!("族级条目命中触达同族的命令: {raw_entry:?} 命令={command:?}")
                });
                assert_eq!(entry.base, parsed.base);
                assert_eq!(matched_command.as_deref(), Some(command));
            } else {
                assert!(
                    matched.is_none(),
                    "族级条目不得命中不触达该族的命令: {raw_entry:?} 命令={command:?}"
                );
            }
        }
        Some(ModelTarget::Command(text)) => {
            if command == text {
                let (entry, matched_command) = matched.unwrap_or_else(|| {
                    panic!("逐字条目必须命中逐字相同的命令: {raw_entry:?}")
                });
                assert_eq!(entry.base, parsed.base);
                assert_eq!(matched_command.as_deref(), Some(command));
            } else {
                assert!(
                    matched.is_none(),
                    "逐字条目不得扩权到别的命令: {raw_entry:?} 命令={command:?}"
                );
            }
        }
    }

    // C18：未知族 id 不给族级语义（不得挡住真族命令）。
    if parsed.base.contains(FAMILY_MARKER) {
        let unknown = parsed
            .base
            .split_once(FAMILY_MARKER)
            .map(|(_, id)| id.to_string())
            .unwrap_or_default();
        if CommandFamily::from_id(&unknown).is_none() {
            let (decision, detail) = single_entry_decision(&parsed, capability, command);
            assert!(
                !matches!(decision, Decision::RequireApproval { .. }),
                "未知族 id 不得获得族级语义: 条目={raw_entry:?} 命令={command:?} 判定={decision:?}"
            );
            assert!(detail.matched_family.is_empty(), "{detail:?}");
        }
    }
}

/// 单条目在给定能力/命令下的判定（供条目面复用）。
fn single_entry_decision(
    entry: &ParsedApprovalEntry,
    capability: &str,
    command: &str,
) -> (Decision, apeireth_governance::PolicyMatch) {
    let mut engine = ApprovalPolicyEngine::new();
    engine.approval_list.push(entry.clone());
    let args = json!({ "command": command });
    engine.evaluate(capability, &args, &[], 0)
}

/// 存量逐字条目的迁移语义（C9 / C10）：
/// 存量条目 `Tool:<命令>` 归族后**必须**与族级条目同权（不是失效重批）。
fn check_legacy_migration(command: &str, legacy_command: &str) {
    let attributed = command_families(legacy_command);
    if attributed.is_empty() {
        // C18：词表外退回逐字精确匹配（既有语义不破）。
        let mut engine = ApprovalPolicyEngine::new();
        engine
            .approval_list
            .push(parse_approval_entry(&format!("{CAP}:{legacy_command}")).unwrap());
        let args = json!({ "command": command });
        let (decision, _) = engine.evaluate(CAP, &args, &[], 0);
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "词表外逐字条目不得扩权到别的命令: 条目={legacy_command:?} 命令={command:?}"
        );
        return;
    }
    // 迁移生效：存量条目归到的族，必须对该族的所有形态生效。
    for family in &attributed {
        let mut engine = ApprovalPolicyEngine::new();
        engine
            .approval_list
            .push(parse_approval_entry(&format!("{CAP}:{legacy_command}")).unwrap());
        let args = json!({ "command": command });
        let (decision, _detail) = engine.evaluate(CAP, &args, &[], 0);
        let touched = touched_families(command);
        if touched.contains(family) {
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "存量条目迁移为族对象后必须挡住同族形态: 条目={legacy_command:?} 族={} 命令={command:?} 判定={decision:?}",
                family.id()
            );
        }
    }
}

/// 判定顺序（C13）：首个终止匹配胜出。
fn check_rule_order(capability: &str, args: &Value, history: &[CallRecord], now_ms: i64) {
    let mut engine = ApprovalPolicyEngine::new();
    engine.blacklist.insert("tool.blocked".into());
    engine.trusted.insert("tool.trusted".into());
    engine.whitelist.insert("tool.safe".into());
    engine
        .approval_list
        .push(ParsedApprovalEntry::family_entry(CAP, CommandFamily::Delete, false));

    let (decision, detail) = engine.evaluate(capability, args, history, now_ms);
    let expected = expected_rule(&engine, capability, args, history, now_ms);
    assert_eq!(
        detail.matched_rule.as_deref(),
        expected,
        "判定顺序与文档口径不一致（Blacklist → Trust → Frequency → Risk → ApprovalList → Whitelist）: 能力={capability} 参数={args} 规则={:?}",
        detail.matched_rule
    );
    // 分档映射：命中规则 → 档位。
    let kind = decision.label();
    let expected_kind = match expected {
        Some("blacklist") | Some("frequency") => "deny",
        Some("risk") | Some("approval_list") => "require_approval",
        _ => "allow",
    };
    assert_eq!(
        kind, expected_kind,
        "命中规则与档位不一致: 规则={expected:?} 判定={decision:?}"
    );
    // 审批时限：只有要求人批的档位带时限（风险档 / 名单档）。
    match expected {
        Some("risk") | Some("approval_list") => assert_eq!(
            detail.timeout_ms,
            Some(APPROVAL_TIMEOUT_MS),
            "要求人批的档位必须带审批时限: 规则={expected:?}"
        ),
        _ => assert_eq!(
            detail.timeout_ms, None,
            "非人批档位不得带审批时限: 规则={expected:?}"
        ),
    }
    // 拒绝 / 要求人批都必须给出可读理由（`Decision::reason` 文档：理由要能被读到的人看懂）。
    if !decision.is_allowed() {
        assert!(
            decision
                .reason()
                .is_some_and(|reason| !reason.trim().is_empty()),
            "非放行判定必须给出理由: 规则={expected:?} 判定={decision:?}"
        );
    }
    // 确定性。
    let (decision2, detail2) = engine.evaluate(capability, args, history, now_ms);
    assert_eq!(decision, decision2, "同一输入两次判定不一致");
    assert_eq!(detail, detail2, "同一输入两次判定不一致");
}

/// 文档口径的判定顺序（C13）。
fn expected_rule(
    engine: &ApprovalPolicyEngine,
    capability: &str,
    args: &Value,
    history: &[CallRecord],
    now_ms: i64,
) -> Option<&'static str> {
    if engine.blacklist.contains(capability) {
        return Some("blacklist");
    }
    if engine.trusted.contains(capability) {
        return Some("trust");
    }
    let count = frequency_count(capability, history, now_ms, engine.frequency_window_ms);
    if engine.frequency_max_calls > 0 && count + 1 >= engine.frequency_max_calls {
        return Some("frequency");
    }
    let prefixes: Vec<&str> = engine.high_risk_prefixes.iter().map(String::as_str).collect();
    if is_high_risk(capability, &prefixes) {
        return Some("risk");
    }
    if !engine.approval_list.is_empty() {
        let commands = extract_commands(args);
        if best_approval_match(&engine.approval_list, capability, &commands).is_some() {
            return Some("approval_list");
        }
    }
    if engine.whitelist.contains(capability) {
        return Some("whitelist");
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// 参数面 / 频率面检查（C8 / C14 / C17）
// ─────────────────────────────────────────────────────────────────────────────

/// 参数面（C8 / C14）：畸形 / 空 / 超长 / Unicode 参数。
fn check_args_surface(args: &Value) {
    let ids = family_ids_for_arguments(args);
    // C8：空 = 非命令面调用（授权对象 = 操作本身）。
    if args.as_object().is_none() {
        assert!(
            ids.is_empty(),
            "非对象参数没有族（授权对象 = 操作本身）: {args}"
        );
    }
    // 族 id 必在冻结词表内、词表序、不重复。
    let mut seen: Vec<&str> = Vec::new();
    let mut last_index = 0usize;
    for id in &ids {
        let family = CommandFamily::ALL
            .iter()
            .copied()
            .find(|family| family.id() == *id)
            .unwrap_or_else(|| panic!("族 id 不在冻结词表内: {id}"));
        assert!(!seen.contains(&id.as_str()), "族 id 必须去重: {ids:?}");
        seen.push(id);
        let index = CommandFamily::ALL
            .iter()
            .position(|candidate| *candidate == family)
            .unwrap();
        assert!(
            index >= last_index,
            "族 id 必须按词表序输出: {ids:?}"
        );
        last_index = index;
    }
    // C8：逐条命令取**归属面**族并合并 —— 结果必须等于归属面并集。
    let mut expected: Vec<CommandFamily> = Vec::new();
    for command in extract_commands(args) {
        for family in command_families(&command) {
            push_family(&mut expected, family);
        }
    }
    sort_families(&mut expected);
    assert_eq!(
        ids,
        family_ids(&expected),
        "参数 → 族 与文档口径不一致（逐条命令取归属面族并合并）: {args}"
    );
    // 确定性。
    assert_eq!(
        family_ids_for_arguments(args),
        ids,
        "同一输入两次判定不一致: {args}"
    );
}

/// 命令提取（C14）：`command` 在前，`command1..N` 按数字升序。
fn check_extract_commands(args: &Value) {
    let commands = extract_commands(args);
    // 每条命令都必须是非空的已 trim 串。
    for command in &commands {
        assert!(
            !command.trim().is_empty(),
            "提取出的命令不得是空白串: {args}"
        );
        assert_eq!(command, command.trim(), "提取出的命令必须已 trim: {args}");
    }
    // `command` 键优先。
    if let Some(text) = args.get("command").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            assert_eq!(
                commands.first().map(String::as_str),
                Some(trimmed),
                "`command` 键必须排在最前: {args}"
            );
        }
    }
    // `commandN` 的数字必须非降序。
    let mut last_index: Option<u64> = None;
    for command in &commands {
        let Some(object) = args.as_object() else {
            break;
        };
        let mut index = None;
        for (key, value) in object {
            if value.as_str().map(str::trim) != Some(command.as_str()) {
                continue;
            }
            let Some(rest) = key.strip_prefix("command") else {
                continue;
            };
            if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            if let Ok(parsed) = rest.parse::<u64>() {
                index = Some(parsed);
                break;
            }
        }
        let Some(index) = index else {
            continue;
        };
        if let Some(previous) = last_index {
            assert!(
                index >= previous,
                "`command1..N` 必须按数字升序: {args} => {commands:?}"
            );
        }
        last_index = Some(index);
    }
    // 确定性。
    assert_eq!(
        extract_commands(args),
        commands,
        "同一输入两次判定不一致: {args}"
    );
}

/// 频率面（C17）：同能力 + 窗口内才计数。
fn check_frequency_surface(capability: &str, now_ms: i64, window_ms: u64, offsets: &[i64]) {
    let history: Vec<CallRecord> = offsets
        .iter()
        .map(|offset| CallRecord::new(capability.to_string(), now_ms.saturating_add(*offset)))
        .collect();
    let count = frequency_count(capability, &history, now_ms, window_ms);
    // 窗口内（严格内部，避开左端点归属争议）+ 同能力 ⇒ 计入。
    let inside = offsets
        .iter()
        .filter(|offset| **offset <= 0 && now_ms.saturating_add(**offset) > now_ms.saturating_sub(window_ms as i64))
        .count() as u32;
    assert_eq!(
        count, inside,
        "频率计数必须只算窗口内的同能力记录: 能力={capability} 偏移={offsets:?}"
    );
    // 未来的记录不计入。
    let future: Vec<CallRecord> = offsets
        .iter()
        .map(|offset| CallRecord::new(capability.to_string(), now_ms.saturating_add(*offset)))
        .chain(std::iter::once(CallRecord::new(
            capability.to_string(),
            now_ms.saturating_add(1),
        )))
        .collect();
    assert_eq!(
        frequency_count(capability, &future, now_ms, window_ms),
        count,
        "未来的记录不得计入频率窗口: 能力={capability}"
    );
    // 异能力不计入。
    let other: Vec<CallRecord> = history
        .iter()
        .cloned()
        .map(|record| CallRecord::new("tool.other", record.timestamp_ms))
        .collect();
    assert_eq!(
        frequency_count(capability, &other, now_ms, window_ms),
        0,
        "异能力记录不得计入频率窗口: 能力={capability}"
    );
    // 确定性。
    assert_eq!(
        frequency_count(capability, &history, now_ms, window_ms),
        count,
        "同一输入两次判定不一致"
    );
}

/// 风险前缀面（C16）：前缀匹配 + 大小写不敏感。
fn check_risk_surface(capability: &str) {
    let prefixes = apeireth_governance::approval_policy::DEFAULT_HIGH_RISK_PREFIXES;
    let hit = is_high_risk(capability, &prefixes);
    let expected = prefixes.iter().any(|prefix| {
        capability
            .to_lowercase()
            .starts_with(&prefix.to_lowercase())
    });
    assert_eq!(
        hit, expected,
        "风险前缀判定口径不一致: {capability:?}"
    );
    // ASCII 能力名的大小写不敏感（C16）。
    if capability.is_ascii() {
        assert_eq!(
            is_high_risk(&capability.to_ascii_uppercase(), &prefixes),
            hit,
            "风险前缀判定必须大小写不敏感: {capability:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 结构化驱动
// ─────────────────────────────────────────────────────────────────────────────

/// 字节流游标（把 fuzz 字节当选择器用）。
struct Stream<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Stream<'a> {
    fn byte(&mut self) -> u8 {
        if self.data.is_empty() {
            return 0;
        }
        let byte = self.data[self.position % self.data.len()];
        self.position = self.position.wrapping_add(1);
        byte
    }

    fn pick<'b, T>(&mut self, items: &'b [T]) -> &'b T {
        &items[self.byte() as usize % items.len()]
    }
}

/// 非族命令池（族外词按设计不入表 ⇒ 不得被族条目误挡）。
const NON_FAMILY_COMMANDS: [&str; 10] = [
    "git status",
    "ls -la",
    "echo hi",
    "cat x.txt",
    "pwd",
    "whoami",
    "grep foo bar",
    "npm test",
    "cargo build",
    "python -c print(1)",
];

/// 词表外命令（用于 C18 的逐字语义检查）。
const OUT_OF_VOCAB_COMMANDS: [&str; 4] = [
    "run-report --weekly",
    "deploy --env prod",
    "some-unknown-tool --flag",
    "printf hello",
];

/// 结构化命令池：族词形态 + 文档同族孪生 + 非族命令 + 分隔符变体。
fn command_pool() -> &'static Vec<String> {
    static POOL: OnceLock<Vec<String>> = OnceLock::new();
    POOL.get_or_init(|| {
        let mut pool: Vec<String> = Vec::new();
        for (_, forms) in same_family_matrix() {
            for form in forms.iter() {
                pool.push((*form).to_string());
            }
        }
        for family in CommandFamily::ALL {
            for word in family.words() {
                pool.push((*word).to_string());
                pool.push(word.to_ascii_uppercase());
                pool.push(format!("{word}.exe"));
                pool.push(format!("{word}.cmd"));
                if word.chars().count() >= ABBREV_MIN {
                    pool.push(word.chars().take(ABBREV_MIN).collect());
                }
            }
        }
        for command in NON_FAMILY_COMMANDS {
            pool.push(command.to_string());
        }
        for command in OUT_OF_VOCAB_COMMANDS {
            pool.push(command.to_string());
        }
        // 显式误拒面（C7 / C10）：提到族词也触达。
        pool.push("echo del is a word".to_string());
        pool.push("findstr \"::Delete\" notes.md".to_string());
        // 复合命令：跨族组合。
        pool.push("del a.txt && curl http://example.invalid".to_string());
        pool.push("echo hi | rm x".to_string());
        pool
    })
}

/// 条目模板：族级 / 存量逐字 / 未知族 id / 工具级 / 静默后缀 / 能力不匹配。
fn entry_pool() -> &'static Vec<String> {
    static POOL: OnceLock<Vec<String>> = OnceLock::new();
    POOL.get_or_init(|| {
        let mut pool: Vec<String> = Vec::new();
        for family in CommandFamily::ALL {
            pool.push(format!("{CAP}:family:{}", family.id()));
            pool.push(format!("{CAP}:family:{}{SILENT_SUFFIX}", family.id()));
        }
        pool.push(format!("{CAP}:family:不存在族"));
        pool.push(format!("{CAP}:family:{SILENT_SUFFIX}"));
        pool.push(CAP.to_string());
        pool.push(format!("{CAP}{SILENT_SUFFIX}"));
        for command in ["rm -rf /", "Remove-Item x.txt", "run-report --weekly", "ls"] {
            pool.push(format!("{CAP}:{command}"));
            pool.push(format!("{CAP}:{command}{SILENT_SUFFIX}"));
        }
        pool.push("other.tool:rm".to_string());
        pool.push("tool.shell.extra:rm".to_string());
        pool
    })
}

/// 参数面形态：畸形 / 空 / 超长 / Unicode。
fn args_pool() -> &'static Vec<Value> {
    static POOL: OnceLock<Vec<Value>> = OnceLock::new();
    POOL.get_or_init(|| {
        vec![
            json!({ "command": "rm -rf /" }),
            json!({ "command": "  rm -rf /  " }),
            json!({ "command": "" }),
            json!({ "command": "   " }),
            json!({ "command": 42 }),
            json!({ "command": null }),
            json!({ "command": ["rm", "-rf"] }),
            json!({ "command": { "nested": "rm" } }),
            json!({ "command": true }),
            json!({ "command1": "rm x" }),
            json!({ "command2": "rm x", "command10": "curl http://example.invalid" }),
            json!({ "command": "rm x", "command1": "curl http://example.invalid" }),
            json!({ "command0": "rm x" }),
            json!({ "operation": "update", "path": "a.txt" }),
            json!({}),
            json!([]),
            json!(null),
            json!("rm x"),
            json!(42),
            json!({ "command": "删除 命令 🗑️ rm" }),
            json!({ "command": "ＲＭ　x" }),
            json!({ "command": "rm\u{0}x" }),
            json!({ "command": "x".repeat(4096) }),
            json!({ "command": format!("rm {}", "x".repeat(2048)) }),
        ]
    })
}

fuzz_target!(|data: &[u8]| {
    // ① 一次性自检：冻结面 + 族词变体矩阵 + 同族形态矩阵 + 跨族隔离。
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(run_once_matrix);

    // ② 原始输入面：任意 UTF-8 当词 / 当命令。
    if let Ok(text) = std::str::from_utf8(data) {
        check_word_surface(text);
        check_command_surface(text);
        // 前缀变体（更短的命令面）。
        let head: String = text.chars().take(24).collect();
        check_command_surface(&head);
    }

    // ③ 结构化输入面：字节流选 token，拼出形态已知的输入。
    let mut stream = Stream { data, position: 0 };
    let pool = command_pool();

    // ③-1 分档面：族条目 × 命令形态。
    let family = *stream.pick(&CommandFamily::ALL);
    let command = stream.pick(pool).clone();
    check_family_entry_gate(family, &command);

    // ③-2 分档面：六族条目齐备。
    let command = stream.pick(pool).clone();
    check_all_family_entries(&command);

    // ③-3 条目面：条目模板 × 命令。
    let raw_entry = stream.pick(entry_pool()).clone();
    let command = stream.pick(pool).clone();
    check_entry_surface(&raw_entry, CAP, &command);
    check_entry_surface(&raw_entry, "other.tool", &command);

    // ③-4 存量逐字条目的迁移语义。
    let legacy = stream.pick(pool).clone();
    let command = stream.pick(pool).clone();
    check_legacy_migration(&command, &legacy);

    // ③-5 参数面。
    let args = stream.pick(args_pool()).clone();
    check_args_surface(&args);
    check_extract_commands(&args);

    // ③-6 判定顺序。
    let capability = *stream.pick(&[
        "tool.blocked",
        "tool.trusted",
        "tool.safe",
        CAP,
        "system.exec",
        "file_write",
        "network.fetch",
        "calculator",
    ]);
    let args = stream.pick(args_pool()).clone();
    let now_ms = stream.byte() as i64;
    check_rule_order(capability, &args, &[], now_ms);

    // ③-7 频率面。
    // 偏移量一律取**偶数增量**，保证记录永远落在窗口左端点（`now - window`）的
    // 两侧而不是**端点上** —— 端点归属在文档里写的是半开区间 `(now - window, now]`
    // 而实现与 donor 移植源一致取左闭，属文档措辞歧义，**不断言**该端点。
    let offsets: Vec<i64> = [0i64, -1, -59_999, -60_001, -120_000, 1]
        .iter()
        .map(|offset| *offset + 2 * i64::from(stream.byte()))
        .collect();
    check_frequency_surface(capability, now_ms, 60_000, &offsets);

    // ③-8 风险前缀面。
    let capability = stream.pick(pool).clone();
    check_risk_surface(&capability);

    // ③-9 存量记录解释对象（迁移语义，C9）。
    let legacy = stream.pick(pool).clone();
    let target = interpret_recorded_command(&legacy);
    let attributed = command_families(&legacy);
    if attributed.is_empty() {
        assert!(
            matches!(target, apeireth_governance::approval_family::ApprovalTarget::Command(ref text) if text == &legacy),
            "词表外命令必须退回逐字对象: {legacy:?} => {target:?}"
        );
    } else {
        assert!(
            matches!(target, apeireth_governance::approval_family::ApprovalTarget::Families(ref families) if family_ids(families) == family_ids(&attributed)),
            "存量记录必须按族解释（迁移生效）: {legacy:?} => {target:?}"
        );
    }
});
