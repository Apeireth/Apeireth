//! 命令审批族（语义族）词表与族归一 —— 命令级审批的单一来源（E-P2）。
//!
//! 病灶（威胁模型 G2）：命令级审批名单逐字精确匹配 —— 批一次 `Remove-Item x`，
//! 换个等价写法（`[System.IO.File]::Delete` / `rm` / 变体）重放就逃出名单 =
//! 白拿授权，孪生 API 形态不同权。整改口径：**审批族归一** —— 命令→族，
//! 审批/授权记录的对象是**族**（批"删除族" = 该族所有形态生效），
//! 同族换形重放被同一族授权覆盖（不重批），跨族仍逐族审批（批删除≠批网络）。
//!
//! 本模块是族词表的**单一来源**：写意图扫描（tools 侧 `write_intent.rs`）的
//! 族词表与本表**同源引用**（同一份族词、同一命中标签、同一变体匹配规则），
//! 两处族词一致由 `approval_family_parity` 测试钉死。
//!
//! 两个判定面（同一词表，两个方向各自 fail-closed）：
//! * **归属面** [`attributed_families`]（授权对象用）：只认**段首命令动词**与
//!   **成员调用**——"段内提到某族词"不算授权归属（`echo del is a word` 不会
//!   拿到删除族授权），宁可归属不足（重批）不白拿授权；
//! * **触达面** [`touched_families`]（审批门用）：词面任意命中 + 成员调用——
//!   宁可误拒不漏放（与写意图扫描同一保守口径，误拒面见 `write_intent.rs`）。
//!
//! 变体防护（沿 git_write 变形防护经验——短形态 / 前缀 / 大小写）：
//! * **短形态**：短别名词表内逐字命中（`rm` / `rd` / `del` / `ri` …）；
//! * **前缀**：成员词头前缀命中（`Delete*` / `Write*` …）+ 命令名缩写命中
//!   （族词前缀 ≥5 字符，`Remove-I` ≡ `Remove-Item`；短于此不收，挡
//!   `rem` / `set` / `stat` 类短词误伤）+ 可执行后缀剥离（`.exe` / `.cmd` /
//!   `.bat` / `.com` / `.ps1`）；
//! * **大小写**：一律 ASCII 不敏感。
//! **精确命中优先于缩写命中**（`start` 精确判进程族，不因 `Start-Transcript`
//! 缩写面误入文件写入族）。
//!
//! 边界（如实，与 `write_intent.rs` 白名单注释同源口径）：词法面不解析引号/
//! 上下文，"只提到命令词/调用形态文本"会被触达面判到（明示误拒面）；
//! 动态成员名/反射调用、编码负载、解释器内写入等词法外形态不入族（由审批链 +
//! 沙箱兜底，见 G3 对象级判定）。

use serde_json::Value;

/// 审批族冻结词表。批一族 = 该族所有形态生效；跨族逐族审批。
///
/// 顺序即判定优先级（词组多族共现时先报低位序族）；`ALL` 与 `id` /
/// `hit_label` / 词表构成冻结面，新族必须显式入表并补测试。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandFamily {
    /// 删除族：删文件/删目录（命令面 + 静态/实例成员调用面同族）。
    Delete,
    /// 移动 / 复制 / 改名族。
    MoveCopy,
    /// 建目录 / 建文件族。
    Create,
    /// 文件写入族（改内容 / 追加 / 覆盖输出 / 原地截断）。
    WriteContent,
    /// 网络族：出网读/传（下载、抓取、套接字）。
    Network,
    /// 进程族：起停进程 / 执行代码。
    Process,
}

/// 命令族变体匹配的最短缩写长度：族词前缀命中只收 ≥5 字符的缩写，
/// 挡 `rem` / `set` / `stat` 类短词误伤（精确命中不受此限）。
pub const MIN_ABBREVIATION_CHARS: usize = 5;

/// 可执行后缀（变体防护：`rm.exe` ≡ `rm`）。只剥一层。
const EXECUTABLE_SUFFIXES: [&str; 5] = [".exe", ".cmd", ".bat", ".com", ".ps1"];

/// 段分隔符：管道 / 条件链 / 顺序链 / 分组括号 / 换行 —— 逐段扫描的切点。
pub const SEGMENT_SEPARATORS: [char; 9] = ['|', '&', ';', '\n', '\r', '(', ')', '{', '}'];

/// 删除族族词（短形态别名 + 类比命令面；大小写不敏感）。
pub const DELETE_FAMILY_WORDS: &[&str] = &[
    "del",
    "erase",
    "rd",
    "rmdir",
    "rm",
    "unlink",
    "shred",
    "ri",
    "Remove-Item",
];

/// 移动 / 复制 / 改名族族词。
pub const MOVE_COPY_FAMILY_WORDS: &[&str] = &[
    "move",
    "copy",
    "xcopy",
    "robocopy",
    "ren",
    "rename",
    "mv",
    "cp",
    "ln",
    "mi",
    "cpi",
    "Move-Item",
    "Copy-Item",
    "Rename-Item",
];

/// 建目录 / 建文件族族词。
pub const CREATE_FAMILY_WORDS: &[&str] = &[
    "mkdir",
    "md",
    "touch",
    "New-Item",
    "New-TemporaryFile",
    "install",
    "ni",
];

/// 文件写入族族词（改内容 / 追加 / 覆盖输出 / 原地截断）。
pub const WRITE_CONTENT_FAMILY_WORDS: &[&str] = &[
    "tee",
    "dd",
    "truncate",
    "Set-Content",
    "Add-Content",
    "Out-File",
    "Clear-Content",
    "Set-Item",
    "Export-Csv",
    "Export-Clixml",
    "Start-Transcript",
];

/// 网络族族词（出网读/传）。
pub const NETWORK_FAMILY_WORDS: &[&str] = &[
    "curl",
    "wget",
    "Invoke-WebRequest",
    "Invoke-RestMethod",
    "Start-BitsTransfer",
    "bitsadmin",
    "ftp",
    "sftp",
    "scp",
    "ssh",
    "rsync",
    "nc",
    "ncat",
    "telnet",
    "iwr",
];

/// 进程族族词（起停进程 / 执行代码）。
pub const PROCESS_FAMILY_WORDS: &[&str] = &[
    "start",
    "stop",
    "kill",
    "taskkill",
    "wmic",
    "nssm",
    "Start-Process",
    "Stop-Process",
    "Invoke-Expression",
    "iex",
];

/// 脚本引擎成员调用面词头 → 族（大小写不敏感前缀命中）。
/// 覆盖 `::成员`（静态调用 / 方法组）与 `.成员(`（实例调用）两种形态；
/// 变更成员命中，只读成员（`Read*` / `Get*` / `Exists` 等）不在表内。
pub const MEMBER_PREFIX_FAMILIES: &[(CommandFamily, &str)] = &[
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

/// 成员调用的形态：`::成员`（静态调用 / 方法组）与 `.成员(`（实例调用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberCallForm {
    /// `::成员` 静态调用 / 方法组。
    Static,
    /// `.成员(` 实例调用。
    Instance,
}

/// 命令文本里扫到的一个成员调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCall {
    /// 成员名（原样大小写）。
    pub name: String,
    /// 调用形态。
    pub form: MemberCallForm,
}

impl CommandFamily {
    /// 冻结词表全集（判定优先级序）。
    pub const ALL: [Self; 6] = [
        Self::Delete,
        Self::MoveCopy,
        Self::Create,
        Self::WriteContent,
        Self::Network,
        Self::Process,
    ];

    /// 族 id（人读族名，审批记录/审批卡/名单条目共用同一词）。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Delete => "删除族",
            Self::MoveCopy => "移动/复制/改名族",
            Self::Create => "建目录/建文件族",
            Self::WriteContent => "文件写入族",
            Self::Network => "网络族",
            Self::Process => "进程族",
        }
    }

    /// 写意图命中面标签（`族id + "命令"`，与历史拒绝帧逐字相同）。
    pub const fn hit_label(self) -> &'static str {
        match self {
            Self::Delete => "删除族命令",
            Self::MoveCopy => "移动/复制/改名族命令",
            Self::Create => "建目录/建文件族命令",
            Self::WriteContent => "文件写入族命令",
            Self::Network => "网络族命令",
            Self::Process => "进程族命令",
        }
    }

    /// 是否写族（写意图总闸的执法面只覆盖这四族；网络/进程族不属写意图）。
    pub const fn is_write_family(self) -> bool {
        matches!(
            self,
            Self::Delete | Self::MoveCopy | Self::Create | Self::WriteContent
        )
    }

    /// 该族的族词表。
    pub const fn words(self) -> &'static [&'static str] {
        match self {
            Self::Delete => DELETE_FAMILY_WORDS,
            Self::MoveCopy => MOVE_COPY_FAMILY_WORDS,
            Self::Create => CREATE_FAMILY_WORDS,
            Self::WriteContent => WRITE_CONTENT_FAMILY_WORDS,
            Self::Network => NETWORK_FAMILY_WORDS,
            Self::Process => PROCESS_FAMILY_WORDS,
        }
    }

    /// 族 id → 族（冻结词表精确匹配；未知 id 返回 `None`）。
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|family| family.id() == id)
    }

    /// 命令词是否判入本族（变体匹配：短形态逐字 / 前缀缩写 ≥5 字符 /
    /// 可执行后缀剥离 / 大小写不敏感；精确命中优先于缩写命中由
    /// [`word_families`] 统一裁决）。
    pub fn matches_word(self, word: &str) -> bool {
        !word_families_of_kind(word, self, MatchKind::Exact).is_empty()
            || !word_families_of_kind(word, self, MatchKind::Abbreviation).is_empty()
    }
}

/// 词法匹配的两种容忍度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchKind {
    /// 精确命中（含大小写不敏感 + 可执行后缀剥离）。
    Exact,
    /// 前缀缩写命中（族词前缀 ≥5 字符）。
    Abbreviation,
}

/// 剥掉一层可执行后缀（变体防护：`rm.exe` ≡ `rm`）。
///
/// **公开**（2026-10-25）：写意图面的「原地改写特判」（`sed -i` / `tar -x`）
/// 原先手写 `w == "sed"`，**绕过了**本函数的剥离 ⇒ `sed.exe -i` 漏放，
/// 而同面的族词路（`rm.exe`）能抓 ⇒ **面内不一致**。
/// 本面文档承诺「变体匹配（…**可执行后缀**）全部引自治理层单一来源」，
/// 故特判必须复用同一入口，而不是各写各的。
pub fn strip_executable_suffix(word: &str) -> &str {
    let lower = word.to_ascii_lowercase();
    for suffix in EXECUTABLE_SUFFIXES {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return &word[..stripped.len()];
        }
    }
    word
}

/// 单族单向命中判定。
fn word_hit_of_kind(word: &str, family_word: &str, kind: MatchKind) -> bool {
    let word = strip_executable_suffix(word).to_ascii_lowercase();
    let family_word = family_word.to_ascii_lowercase();
    match kind {
        MatchKind::Exact => word == family_word,
        MatchKind::Abbreviation => {
            word.chars().count() >= MIN_ABBREVIATION_CHARS && family_word.starts_with(&word)
        }
    }
}

/// 某一匹配向度下命中的族（多族可共现）。
fn word_families_of_kind(word: &str, family: CommandFamily, kind: MatchKind) -> Vec<CommandFamily> {
    if family
        .words()
        .iter()
        .any(|family_word| word_hit_of_kind(word, family_word, kind))
    {
        vec![family]
    } else {
        Vec::new()
    }
}

/// 命令词 → 族（变体匹配）。**精确命中优先于缩写命中**：有精确命中时
/// 只按精确族计（`start` 精确判进程族，不因 `Start-Transcript` 缩写面
/// 误入文件写入族）；无精确命中时才收 ≥5 字符缩写。
pub fn word_families(word: &str) -> Vec<CommandFamily> {
    let mut exact = Vec::new();
    let mut abbreviations = Vec::new();
    for family in CommandFamily::ALL {
        if !word_families_of_kind(word, family, MatchKind::Exact).is_empty() {
            exact.push(family);
        } else if !word_families_of_kind(word, family, MatchKind::Abbreviation).is_empty() {
            abbreviations.push(family);
        }
    }
    if exact.is_empty() {
        abbreviations
    } else {
        exact
    }
}

/// 成员名 → 族（大小写不敏感前缀命中；只读成员不命中）。
pub fn member_family(member: &str) -> Option<CommandFamily> {
    let lower = member.to_ascii_lowercase();
    MEMBER_PREFIX_FAMILIES
        .iter()
        .find(|(_, prefix)| lower.starts_with(prefix))
        .map(|(family, _)| *family)
}

/// 提词：词字符 = 字母数字 + `_` / `-` / `.`（命令名与参数各成一词；
/// 文件名带点不拆词，`file.md` 不会误命中 `md`）。
pub fn command_words(segment: &str) -> Vec<String> {
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

/// 从 `start` 起提一个成员名：字母/下划线开头、字母数字/下划线续。
/// 非标识符开头（如 `::30` 的数字、`::(` 等）返回 `None`。
fn member_name_at(chars: &[char], start: usize) -> Option<String> {
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

/// 整条命令面扫描成员调用（`::成员` 静态 / `.成员(` 实例两种形态，
/// 左到右按出现序）。成员调用可跨段分隔符，故整条命令面整体扫描。
pub fn member_calls(command: &str) -> Vec<MemberCall> {
    let chars: Vec<char> = command.chars().collect();
    let mut calls = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' && chars.get(i + 1) == Some(&':') {
            let mut j = i + 2;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if let Some(name) = member_name_at(&chars, j) {
                calls.push(MemberCall {
                    name,
                    form: MemberCallForm::Static,
                });
            }
            i += 2;
            continue;
        }
        if chars[i] == '.' {
            if let Some(name) = member_name_at(&chars, i + 1) {
                let mut k = i + 1 + name.chars().count();
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if chars.get(k) == Some(&'(') {
                    calls.push(MemberCall {
                        name,
                        form: MemberCallForm::Instance,
                    });
                }
            }
        }
        i += 1;
    }
    calls
}

/// **族归一函数（单一来源）**：命令 → 族（归属面）。
///
/// 授权对象用：只认**段首命令动词**（每段第一个词）与**成员调用**。
/// 段内参数里"提到"的族词不算归属（`echo del is a word` 不拿删除族授权）；
/// 归属不足是安全向（重批即可），不白拿授权。
pub fn command_families(command: &str) -> Vec<CommandFamily> {
    let mut families = Vec::new();
    for call in member_calls(command) {
        if let Some(family) = member_family(&call.name) {
            push_unique(&mut families, family);
        }
    }
    for segment in command.split(|c| SEGMENT_SEPARATORS.contains(&c)) {
        if let Some(head) = command_words(segment).into_iter().next() {
            for family in word_families(&head) {
                push_unique(&mut families, family);
            }
        }
    }
    sort_families(&mut families);
    families
}

/// 命令 → 族（触达面，保守）：词面任意命中 + 成员调用。
///
/// 审批门用：宁可误拒不漏放（`echo del is a word` 也判触达删除族，
/// 与写意图扫描同一误拒口径）。
pub fn touched_families(command: &str) -> Vec<CommandFamily> {
    let mut families = Vec::new();
    for call in member_calls(command) {
        if let Some(family) = member_family(&call.name) {
            push_unique(&mut families, family);
        }
    }
    for segment in command.split(|c| SEGMENT_SEPARATORS.contains(&c)) {
        for word in command_words(segment) {
            for family in word_families(&word) {
                push_unique(&mut families, family);
            }
        }
    }
    sort_families(&mut families);
    families
}

/// 调用参数 → 授权族（族 id 词表序去重）：走 `command` / `command1..N`
/// 提取（与命令级名单同一提取口径），逐条命令取归属面族并合并。
/// 空 = 非命令面调用（授权对象 = 操作本身，逐操作授权）。
pub fn family_ids_for_arguments(arguments: &Value) -> Vec<String> {
    let mut families = Vec::new();
    for command in crate::approval_policy::extract_commands(arguments) {
        for family in command_families(&command) {
            push_unique(&mut families, family);
        }
    }
    sort_families(&mut families);
    families
        .iter()
        .map(|family| family.id().to_string())
        .collect()
}

/// 一条审批/授权记录的对象（E-P2 族归一）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalTarget {
    /// 整个能力（tool 级记录）。
    Capability,
    /// 族对象：单族 = 批"该族" = 该族所有形态生效；多族 = 该族组合形态。
    Families(Vec<CommandFamily>),
    /// 逐字命令对象（词表外命令的精确语义退路，存量记录兼容面）。
    Command(String),
}

/// 存量逐字审批记录的**迁移解释**（E-P2 兼容口径，二选一钉死）：
/// **迁移生效** —— 老记录的命令 → 族映射后生效（不是失效重批）。
/// 命令词表外（无法归族）→ 退回逐字命令对象（既有精确匹配语义）。
pub fn interpret_recorded_command(command: &str) -> ApprovalTarget {
    let families = command_families(command);
    if families.is_empty() {
        ApprovalTarget::Command(command.to_string())
    } else {
        ApprovalTarget::Families(families)
    }
}

fn push_unique(families: &mut Vec<CommandFamily>, family: CommandFamily) {
    if !families.contains(&family) {
        families.push(family);
    }
}

fn sort_families(families: &mut Vec<CommandFamily>) {
    families.sort_by_key(|family| {
        CommandFamily::ALL
            .iter()
            .position(|candidate| candidate == family)
            .unwrap_or(usize::MAX)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ids(families: &[CommandFamily]) -> Vec<&str> {
        families.iter().map(|family| family.id()).collect()
    }

    /// ① 族归一矩阵·同族多形态：命令面族词、短别名、静态调用、实例调用
    /// 全部归入同一族。
    #[test]
    fn family_normalization_keeps_all_forms_of_a_family_together() {
        for command in [
            "Remove-Item x.txt",
            "rm x.txt",
            "del x.txt",
            "rd x_dir",
            "ri x.txt",
            "shred x.txt",
            "[System.IO.File]::Delete('x.txt')",
            "[System.IO.Directory]::Delete('x_dir', $true)",
            "([System.IO.FileInfo]'x.txt').Delete()",
            "$fs.DeleteFile('x.txt')",
        ] {
            assert_eq!(
                ids(&command_families(command)),
                vec!["删除族"],
                "{command} 必须归删除族"
            );
        }
        for command in [
            "Set-Content x.txt hello",
            "tee x.txt",
            "truncate x.txt",
            "[System.IO.File]::WriteAllText('x.txt','hello')",
            "[System.IO.File]::AppendAllText('x.txt','hello')",
        ] {
            assert_eq!(
                ids(&command_families(command)),
                vec!["文件写入族"],
                "{command} 必须归文件写入族"
            );
        }
        for command in [
            "Move-Item a b",
            "mv a b",
            "ren a b",
            "Rename-Item a b",
            "cpi a b",
        ] {
            assert_eq!(
                ids(&command_families(command)),
                vec!["移动/复制/改名族"],
                "{command} 必须归移动/复制/改名族"
            );
        }
        for command in [
            "mkdir d",
            "New-Item f",
            "[System.IO.Directory]::Create('d')",
        ] {
            assert_eq!(
                ids(&command_families(command)),
                vec!["建目录/建文件族"],
                "{command} 必须归建目录/建文件族"
            );
        }
    }

    /// ①附 族归一矩阵·跨族不串：删除 / 写入 / 网络 / 进程互不串族，
    /// 复合命令按触达族组合入账。
    #[test]
    fn family_normalization_never_crosses_families() {
        for (command, expected) in [
            ("Remove-Item x.txt", vec!["删除族"]),
            ("curl http://example.invalid", vec!["网络族"]),
            ("Invoke-WebRequest http://example.invalid", vec!["网络族"]),
            ("taskkill /PID 1", vec!["进程族"]),
            ("Start-Process notepad", vec!["进程族"]),
        ] {
            assert_eq!(ids(&command_families(command)), expected, "{command}");
            // 单族命令绝不携带别族。
            for family in command_families(command) {
                assert_eq!(
                    vec![family],
                    command_families(command),
                    "{command} 只能归一族"
                );
            }
        }
        // 复合命令：两族组合，不吞并成单族。
        assert_eq!(
            ids(&command_families(
                "del a.txt && curl http://example.invalid"
            )),
            vec!["删除族", "网络族"],
            "跨族复合命令必须两族都入账（逐族审批的前提）"
        );
    }

    /// ② 变体防护（短束 / 前缀 / 大小写）：短别名、命令缩写、可执行后缀、
    /// 大小写变体一律判同族（换形重放逃不出族）。
    #[test]
    fn variant_forms_short_aliases_prefix_abbreviations_and_case_stay_in_family() {
        for variant in [
            // 短束：短别名词表内逐字命中。
            "rm x.txt",
            "rd x_dir",
            "del x.txt",
            "ri x.txt",
            // 前缀：命令名缩写（≥5 字符）与可执行后缀变体。
            "Remove-I x.txt",
            "Remove x.txt",
            "REMOVE-ITEM.EXE x.txt",
            "remove-item.cmd x.txt",
            // 大小写。
            "REMOVE-ITEM x.txt",
            "Remove-item x.txt",
            // 成员词头前缀 + 大小写 + 实例形态。
            "[system.io.file]::delete('x.txt')",
            "$fs.DeleteFolder('x_dir')",
            "([System.IO.FileInfo]'x.txt').Delete ()",
        ] {
            assert_eq!(
                ids(&command_families(variant)),
                vec!["删除族"],
                "{variant} 变体必须判删除族"
            );
        }
        // 触达面同口径（审批门不被变体绕过）。
        for variant in [
            "Remove-I x.txt",
            "REMOVE-ITEM.EXE x.txt",
            "$fs.DeleteFile('x')",
        ] {
            assert_eq!(ids(&touched_families(variant)), vec!["删除族"], "{variant}");
        }
    }

    /// ③ 精确命中优先于缩写命中 + 只读命令零归属：
    /// `start` 精确判进程族（不因 `Start-Transcript` 缩写面入文件写入族）；
    /// 只读命令/只读成员调用不归族。
    #[test]
    fn exact_hits_win_and_read_only_forms_attribute_no_family() {
        assert_eq!(ids(&command_families("start notepad")), vec!["进程族"]);
        assert_eq!(ids(&command_families("stat file.txt")), Vec::<&str>::new());
        assert_eq!(
            ids(&command_families("rem this is a comment")),
            Vec::<&str>::new()
        );
        for command in [
            "type x.txt",
            "cat x.txt",
            "git status",
            "echo hi",
            "[System.IO.File]::ReadAllText('x.txt')",
            "[System.IO.File]::Exists('x.txt')",
        ] {
            assert_eq!(
                command_families(command),
                Vec::<CommandFamily>::new(),
                "{command} 只读命令不得归族"
            );
        }
    }

    /// ④ 归属面与触达面的方向分工：段内"提到"族词不构成授权归属
    /// （不白拿授权），但触达面照判（宁误拒不漏放）。
    #[test]
    fn attribution_ignores_mentions_while_the_gate_still_catches_them() {
        assert_eq!(
            command_families("echo del is a word"),
            Vec::<CommandFamily>::new(),
            "只提到命令词不得构成授权归属"
        );
        assert_eq!(
            ids(&touched_families("echo del is a word")),
            vec!["删除族"],
            "触达面按保守口径判到（误拒面，明示）"
        );
    }

    /// ⑤ 词表冻结面：六族齐全、族 id 唯一、命中面标签 = 族 id + "命令"、
    /// 族词不跨族重复（跨族不串的词法前提）、成员词头只指向在表族。
    #[test]
    fn family_vocabulary_is_frozen_and_disjoint() {
        assert_eq!(CommandFamily::ALL.len(), 6);
        for family in CommandFamily::ALL {
            assert!(!family.words().is_empty(), "{} 词表不能为空", family.id());
            assert_eq!(family.hit_label(), format!("{}命令", family.id()));
            assert_eq!(CommandFamily::from_id(family.id()), Some(family));
        }
        for (index, family) in CommandFamily::ALL.iter().enumerate() {
            for other in &CommandFamily::ALL[index + 1..] {
                assert_ne!(family.id(), other.id(), "族 id 必须唯一");
                for word in family.words() {
                    assert!(
                        !other.words().iter().any(|other_word| other_word == word),
                        "族词 {word} 不得跨族重复（{} / {}）",
                        family.id(),
                        other.id()
                    );
                }
            }
        }
        for (family, prefix) in MEMBER_PREFIX_FAMILIES {
            assert!(!prefix.is_empty(), "成员词头不能为空");
            assert!(
                CommandFamily::ALL.contains(family),
                "成员词头必须指向在表族"
            );
        }
    }

    /// ⑥ 存量逐字记录迁移语义（二选一钉死 = **迁移生效**）：老记录的命令
    /// → 族映射后生效；词表外命令退回逐字对象，不静默扩权。
    #[test]
    fn legacy_verbatim_records_migrate_by_family_interpretation() {
        assert_eq!(
            interpret_recorded_command("Remove-Item x.txt"),
            ApprovalTarget::Families(vec![CommandFamily::Delete]),
            "存量记录按族解释后生效（迁移生效，不失效重批）"
        );
        assert_eq!(
            interpret_recorded_command("[System.IO.File]::Delete('y.txt')"),
            ApprovalTarget::Families(vec![CommandFamily::Delete]),
            "孪生形态的存量记录解释到同一族"
        );
        assert_eq!(
            interpret_recorded_command("some-unknown-tool --flag"),
            ApprovalTarget::Command("some-unknown-tool --flag".to_string()),
            "词表外命令退回逐字对象（不静默扩权）"
        );
        // 参数面同源提取：command / command1..N 都入族账。
        assert_eq!(
            family_ids_for_arguments(&json!({
                "command": "Remove-Item a.txt",
                "command2": "curl http://example.invalid"
            })),
            vec!["删除族", "网络族"]
        );
        assert_eq!(
            family_ids_for_arguments(&json!({ "operation": "update", "path": "a.txt" })),
            Vec::<String>::new(),
            "非命令面调用无族（授权对象 = 操作本身）"
        );
    }
}
