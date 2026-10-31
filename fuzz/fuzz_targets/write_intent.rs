#![no_main]

//! fuzz: **shell 写意图词法扫描**（`scan_shell_write_intent`）—— 文件写入唯一总闸的 shell 侧执行面。
//!
//! 靶子：`apeireth_tools_canonical::write_intent::scan_shell_write_intent`
//! （`crates/capabilities/tools/src/write_intent.rs:49`）。
//!
//! ## 这个面为什么值得 fuzz
//!
//! 写总闸是「文件写入的唯一闸门」：shell 命令执行前先做写意图词法扫描，命中且闸未开
//! → 拒绝即帧（`write_intent.rs:3-7`「文件写入只有唯一总闸」）。这条词法面一旦被绕过
//! = 闸门形同虚设（shell 照样落盘）。
//!
//! 本仓历史已修过一次同类真缺陷：**命令守门硬编码空格分隔，可被 TAB / 换行绕过**
//! （`crates/capabilities/tools/src/guardrail.rs:249-256`
//! 「`netsh<TAB>set advfirewall x` → **放行** ❌ ← 绕过」）。所以本 target 的重心是
//! **分隔符变体**（空格 / TAB / 换行 / 回车 / 垂直制表 / 换页 / 全角空格 / 连续空白 /
//! 行首行尾空白）与**等价写法**（`>` / `>>` / `2>` / `&>` / `>|` / `<>` / 引号内 `>`）。
//!
//! ## 契约来源（断言逐条取自被测函数自己的文档注释 —— **不自行发明期望值**）
//!
//! | # | 契约 | 出处（文件:行 + 原文片段） |
//! |---|---|---|
//! | C1 | 命中返回拒绝面；纯读命令返回 `None` | `write_intent.rs:48`「扫描一条 shell 命令的写意图。命中返回拒绝面；纯读命令返回 `None`。」 |
//! | C2 | 扫描口径：保守优先，宁可误拒不漏放 | `write_intent.rs:9`「扫描口径：**保守优先，宁可误拒不漏放**。」 |
//! | C3 | 重定向面：`>` / `>>` / `N>` / `&>` / `<>` 一律视为落盘意图 | `write_intent.rs:11`「重定向面：`>` / `>>` / `N>` / `&>` / `<>` 一律视为落盘意图；」 |
//! | C4 | 命令面：删除 / 移动 / 复制 / 改名 / 建目录族与文件写入族（逐词命中） | `write_intent.rs:12`「命令面：删除 / 移动 / 复制 / 改名 / 建目录族与文件写入族（逐词命中）；」 |
//! | C5 | 静态调用面：`::变更成员` / `.变更成员(` 与命令面族词**同族同权** | `write_intent.rs:14-17`「脚本引擎静态调用面：`::变更成员` / `.变更成员(` 形态 … 与命令面族词**同族同权**」 |
//! | C6 | 写总闸只执法**写四族**（网络族 / 进程族不属写意图） | `write_intent.rs:22-23`「写总闸只执法**写四族**（网络族 / 进程族不属写意图，词表同源但不入本闸）。」 |
//! | C7 | 白名单：句柄复制 `N>&M`（两头单个数字、紧邻无空格）抹成空格后**不判写意图** | `write_intent.rs:60`「白名单：句柄复制 `N>&M`（两头单个数字、紧邻无空格）抹成空格后不判写意图。」 |
//! | C8 | **不收带空格的变体**（`2>& 1`）：词法面不解析 shell 变体，**命中即拒** | `write_intent.rs:66-67`「**不收带空格的变体**（`2>& 1`）：词法面不解析 shell 变体，命中即拒——宁可误拒不漏放；」 |
//! | C9 | **`N>&M` 之外的 `>` 一律命中** | `write_intent.rs:68`「**`N>&M` 之外的 `>` 一律命中**：`cmd > f 2>&1` 的 `> f` 照样拒；」 |
//! | C10 | 已知误拒边界：引号内的 `>` / 只提到命令词 / 只提到调用形态文本 **都会被拒** | `write_intent.rs:74-77`「**已知误拒边界**（词法面不解析引号/上下文，宁可误拒不漏放）：引号内的 `>`（`echo "a > b"`）、"只提到命令词"（`echo del is a word`）与"只提到调用形态文本"（`findstr "::Delete" notes.md`）都会被拒」 |
//! | C11 | 提词：词字符 = 字母数字 + `_` / `-` / `.`（其余字符都是词边界） | `approval_family.rs:340-341`「提词：词字符 = 字母数字 + `_` / `-` / `.`（命令名与参数各成一词；文件名带点不拆词，`file.md` 不会误命中 `md`）。」 |
//! | C12 | 命中面标签 = 族 id + "命令" | `approval_family.rs:220`「写意图命中面标签（`族id + "命令"`，与历史拒绝帧逐字相同）。」 |
//! | C13 | 段分隔符：管道 / 条件链 / 顺序链 / 分组括号 / 换行（逐段扫描的切点） | `approval_family.rs:65` |
//!
//! ## 断言清单（每条都指回上表某一条）
//!
//! **① 通用不变量（任意 UTF-8 输入）**
//! * 命中面非空（人读描述进入拒绝帧文案；`write_intent.rs:35`）；
//! * 命中面族名标签属冻结词表，且**不得**是网络族 / 进程族（C6 / C12）；
//! * 确定性：同一输入两次调用结果相同。
//!
//! **② 命令面（逐词命中，C4 + C2）**
//! * 输入里出现写族词（治理层 `command_words` + `word_families` 口径）→ **必须**判写意图；
//! * 无 `>` / 无 `&`（掩码不改字面）时，命中面族名标签**逐字**等于治理层 `hit_label`，
//!   且**没有**写族词 / 成员调用 / sed-tar 特判时**必须**返回 `None`（C1 零误伤）。
//!
//! **③ 静态调用面（C5 + C2）**
//! * 输入里出现写族成员调用（`::变更成员` / `.变更成员(`）→ **必须**判写意图。
//!
//! **④ 重定向面（C3 + C9 + C10）**
//! * 含 `>` 且不含句柄复制形态 → **必须**判重定向面；
//! * 引号内的 `>` → **必须**判（明示误拒边界 C10）。
//!
//! **⑤ 白名单（C7 + C8）**
//! * `N>&M` 单独出现 → **不得**判写意图；
//! * `2>& 1`（带空格变体）→ **必须**判（C8）。
//!
//! **⑥ 分隔符变体（C4 + C11 + C13）**：写族词与前后文的分隔符换成
//! 空格 / TAB / 换行 / 回车 / 垂直制表 / 换页 / 全角空格 / 连续空白 / 行首行尾空白
//! → **必须**判写意图，且族名标签逐字正确。
//!
//! ## 已知**不**断言的边界（避免自造期望值 → 假阳性）
//!
//! * `<<` heredoc：文档的重定向面清单（C3）未含 `<<`，故不作断言（观察到当前不判写意图）；
//! * 相邻句柄复制链（`3>&2>&1`）：文档未写清该链的切词口径（贪心非重叠 vs 全覆盖），
//!   只断言「命中面必为重定向面」，不断言命中与否；
//! * `sed --in-place` / `tar --extract` 等价 flag 与 `.exe` 后缀变体：文档的原地改写特判
//!   只写明 `sed -i` / `tar -x`（`write_intent.rs:161`），故不作断言。

use apeireth_governance::approval_family::{
    CommandFamily, command_words, member_calls, member_family, word_families,
};
use apeireth_tools_canonical::write_intent::scan_shell_write_intent;
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

/// 写四族（执法面 = 删除 / 移动复制改名 / 建目录建文件 / 文件写入；网络族与进程族
/// 不属写意图 —— `write_intent.rs:22-23`）。顺序 = 命中优先级（`write_intent.rs:39-40`）。
const WRITE_FAMILIES: [CommandFamily; 4] = [
    CommandFamily::Delete,
    CommandFamily::MoveCopy,
    CommandFamily::Create,
    CommandFamily::WriteContent,
];

/// 重定向面命中标签（`write_intent.rs:107` `重定向 '>'` / `:110` `读写重定向 "<>"`）。
const REDIRECT_LABELS: [&str; 2] = ["重定向", "读写重定向"];

/// 固定命中标签（原地改写 / 归档解开两个特判面 —— `write_intent.rs:166` / `:171`）。
const FIXED_LABELS: [&str; 2] = ["原地改写命令", "归档解开命令"];

/// 非写族标签（网络族 / 进程族）—— 不得出现在写意图命中面（C6）。
const FOREIGN_LABELS: [&str; 2] = ["网络族命令", "进程族命令"];

/// 纯读命令词（**不得**命中任何族：既不是族词，也不是 ≥5 字符的族词前缀，
/// 也不带可执行后缀 —— 逐条对过治理层词表）。
const BENIGN: [&str; 24] = [
    "echo", "cat", "ls", "git", "status", "type", "pwd", "dir", "findstr", "sort", "more",
    "printf", "grep", "whoami", "env", "uname", "date", "head", "tail", "wc", "ps", "id", "df",
    "help",
];

/// 分隔符变体：POSIX 空白六件套 + 非 ASCII 空白 + 连续空白 + 段分隔符（C11 / C13）。
const SEPARATORS: [&str; 20] = [
    " ", "\t", "\n", "\r", "\u{b}", "\u{c}", "\u{a0}", "\u{3000}", "  ", "\t\t", " \t ", "\r\n",
    "\n\t", "|", "||", "&", "&&", ";", "()", "{ }",
];

/// 行首 / 行尾空白变体（C11：非词字符都是词边界）。
const EDGE_WHITESPACE: [&str; 6] = [" ", "\t", "\n", "  ", "\r\n", "\u{3000}"];

/// 重定向形态（文档 C3 列出的形态 + `>|` 强制覆盖形态）。
const REDIRECTS: [&str; 10] = [">", ">>", "2>", "2>>", "&>", "&>>", ">|", "<>", "1>", "3>"];

/// 句柄复制形态（白名单 C7：两头单个数字、紧邻无空格）。
const HANDLE_DUPS: [&str; 4] = ["2>&1", "1>&2", "3>&1", "0>&1"];

/// 带空格的句柄复制变体（C8：不收，命中即拒）。
const SPACED_HANDLE_DUP: [&str; 2] = ["2>& 1", "2> &1"];

/// 静态 / 实例成员调用（写族，C5）。
const MEMBER_CASES: [(&str, CommandFamily); 10] = [
    ("[System.IO.File]::Delete('x')", CommandFamily::Delete),
    ("[System.IO.Directory]::Delete('x')", CommandFamily::Delete),
    ("([System.IO.FileInfo]'x').Delete()", CommandFamily::Delete),
    ("$fs.DeleteFile('x')", CommandFamily::Delete),
    ("[System.IO.File]::Move('a','b')", CommandFamily::MoveCopy),
    ("$obj.CopyTo('x')", CommandFamily::MoveCopy),
    ("[System.IO.Directory]::Create('x')", CommandFamily::Create),
    ("$obj.Create('x')", CommandFamily::Create),
    (
        "[System.IO.File]::WriteAllText('x','y')",
        CommandFamily::WriteContent,
    ),
    ("$obj.Append('x')", CommandFamily::WriteContent),
];

/// 只读成员调用（`Read*` / `Get*` / `Exists` / `OpenText` 等 —— `write_intent.rs:72-73`
/// 「只读成员（`Read*`/`Get*`/`Exists` 等）不命中」）。
const READ_ONLY_MEMBERS: [&str; 5] = [
    "[System.IO.File]::ReadAllText('x')",
    "[System.IO.File]::Exists('x')",
    "[System.IO.Directory]::GetFiles('x')",
    "([System.IO.FileInfo]'x').OpenText()",
    "$fs.get_Length()",
];

/// 一个写族词形与其应归的族（词形取自治理层词表 + 变体形态）。
struct WordCase {
    text: String,
    family: CommandFamily,
}

/// 写族词形集合：族词原形 / 大小写变体 / 可执行后缀变体 / ≥5 字符前缀缩写变体。
/// 变体口径全部引自治理层单一来源（`approval_family.rs:20-28` 变体防护）。
fn word_cases() -> &'static Vec<WordCase> {
    static CASES: OnceLock<Vec<WordCase>> = OnceLock::new();
    CASES.get_or_init(|| {
        let mut cases: Vec<WordCase> = Vec::new();
        for family in WRITE_FAMILIES {
            for word in family.words() {
                let mut variants: Vec<String> = vec![
                    (*word).to_string(),
                    word.to_ascii_lowercase(),
                    word.to_ascii_uppercase(),
                    format!("{word}.exe"),
                ];
                // 前缀缩写变体：族词前缀 ≥5 字符才收（`approval_family.rs:57-59`）。
                if word.chars().count() > 5 {
                    variants.push(word.chars().take(5).collect());
                }
                for text in variants {
                    let families = word_families(&text);
                    // 期望族 = 治理层归一结果里第一个写族（与实现的判定顺序同口径）。
                    let Some(hit) = WRITE_FAMILIES.iter().find(|f| families.contains(f)) else {
                        continue;
                    };
                    cases.push(WordCase { text, family: *hit });
                }
            }
        }
        cases
    })
}

/// 命中面族名标签（命中面文案 = `族名 空格 引号命中词`，族名不含空格）。
fn family_label(surface: &str) -> &str {
    surface.split(' ').next().unwrap_or(surface)
}

/// 冻结命中标签全集（写四族族标签 + 重定向面 + 两个特判面）。
fn all_labels() -> Vec<&'static str> {
    let mut labels: Vec<&'static str> = REDIRECT_LABELS.to_vec();
    labels.extend(FIXED_LABELS.iter().copied());
    labels.extend(WRITE_FAMILIES.iter().map(|family| family.hit_label()));
    labels
}

/// 输入里第一个命中写族的词对应的族（治理层提词 + 族归一口径，C4）。
fn first_write_family_word(text: &str) -> Option<CommandFamily> {
    for word in command_words(text) {
        let families = word_families(&word);
        if let Some(family) = WRITE_FAMILIES.iter().find(|f| families.contains(f)) {
            return Some(*family);
        }
    }
    None
}

/// 输入里是否出现写族成员调用（`::变更成员` / `.变更成员(`，C5）。
fn has_write_member_call(text: &str) -> bool {
    member_calls(text)
        .iter()
        .any(|call| member_family(&call.name).is_some_and(|family| family.is_write_family()))
}

/// 输入里是否存在句柄复制形态 `数字 '>' '&' 数字`（白名单 C7）。
fn has_handle_duplication(text: &str) -> bool {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(3)).any(|index| {
        bytes[index].is_ascii_digit()
            && bytes[index + 1] == b'>'
            && bytes[index + 2] == b'&'
            && bytes[index + 3].is_ascii_digit()
    })
}

/// 输入里是否出现 `sed` / `tar` 词（原地改写特判面 —— `write_intent.rs:161-172`）。
fn has_sed_or_tar_word(text: &str) -> bool {
    command_words(text).iter().any(|word| {
        let lower = word.to_ascii_lowercase();
        lower == "sed" || lower == "tar"
    })
}

/// 结构化输入面的形态。
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// 纯读命令（零误伤）。
    ReadOnly,
    /// 写族词 + 分隔符变体（必须命中该族）。
    FamilyWord,
    /// 写族词被引号包住（明示误拒面 C10：只提到命令词也会被拒）。
    FamilyWordQuoted,
    /// 重定向形态（必须命中重定向面）。
    Redirect,
    /// 引号内的重定向（明示误拒面 C10）。
    RedirectQuoted,
    /// 句柄复制白名单（不得判写意图，C7）。
    HandleDup,
    /// 带空格的句柄复制变体（必须判，C8）。
    SpacedHandleDup,
    /// 相邻句柄复制链（不断言命中与否，只断言命中面必为重定向面）。
    HandleDupChain,
    /// 写族成员调用（必须命中该族，C5）。
    MemberCall,
    /// 只读成员调用（不得判写意图）。
    ReadOnlyMember,
}

/// 结构化输入面的期望结果。
enum Expect {
    /// 不得判写意图。
    NoHit,
    /// 必须判写意图，且族名标签逐字相等。
    HitLabel(&'static str),
    /// 必须判写意图，且命中面属重定向面。
    HitRedirect,
    /// 命中则命中面必属重定向面（不断言命中与否）。
    MaybeRedirect,
}

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

/// 按形态拼出一条命令（token 全来自上面的表 → 期望结果已知）。
fn build(stream: &mut Stream<'_>, shape: Shape) -> (String, Expect) {
    let mut command = String::new();
    let expect = match shape {
        Shape::ReadOnly => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::NoHit
        }
        Shape::FamilyWord => {
            let case = stream.pick(word_cases());
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(&case.text);
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitLabel(case.family.hit_label())
        }
        Shape::FamilyWordQuoted => {
            let case = stream.pick(word_cases());
            command.push_str(stream.pick(&BENIGN));
            command.push('"');
            command.push_str(&case.text);
            command.push('"');
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitLabel(case.family.hit_label())
        }
        Shape::Redirect => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&REDIRECTS));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitRedirect
        }
        Shape::RedirectQuoted => {
            command.push_str(stream.pick(&BENIGN));
            command.push('"');
            command.push('a');
            command.push_str(stream.pick(&REDIRECTS));
            command.push('b');
            command.push('"');
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitRedirect
        }
        Shape::HandleDup => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&HANDLE_DUPS));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::NoHit
        }
        Shape::SpacedHandleDup => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&SPACED_HANDLE_DUP));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitRedirect
        }
        Shape::HandleDupChain => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str("3>&2>&1");
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::MaybeRedirect
        }
        Shape::MemberCall => {
            let (text, family) = stream.pick(&MEMBER_CASES);
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(text);
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::HitLabel(family.hit_label())
        }
        Shape::ReadOnlyMember => {
            command.push_str(stream.pick(&BENIGN));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&READ_ONLY_MEMBERS));
            command.push_str(stream.pick(&SEPARATORS));
            command.push_str(stream.pick(&BENIGN));
            Expect::NoHit
        }
    };
    // 行首 / 行尾空白变体（C11：空白是词边界，不改变判定）。
    if stream.byte() % 2 == 0 {
        command.insert_str(0, stream.pick(&EDGE_WHITESPACE));
    }
    if stream.byte() % 2 == 0 {
        command.push_str(stream.pick(&EDGE_WHITESPACE));
    }
    (command, expect)
}

/// 原始输入面：任意 UTF-8 命令 → 通用不变量 + 契约断言。
fn check_raw(text: &str) {
    let hit = scan_shell_write_intent(text);

    // ① 通用不变量
    if let Some(hit) = &hit {
        assert!(
            !hit.surface.is_empty(),
            "命中面不能为空（人读描述进入拒绝帧文案）: {text:?}"
        );
        let label = family_label(&hit.surface);
        assert!(
            all_labels().contains(&label),
            "命中面族名标签不在冻结词表内: {text:?} => {}",
            hit.surface
        );
        assert!(
            !FOREIGN_LABELS.contains(&label),
            "网络族/进程族不属写意图，不得入写闸: {text:?} => {}",
            hit.surface
        );
    }
    // 确定性
    assert_eq!(
        scan_shell_write_intent(text),
        hit,
        "同一输入两次调用结果不同: {text:?}"
    );

    // ② 命令面：出现写族词 → 必须判写意图（C4 + C2）
    if let Some(family) = first_write_family_word(text) {
        assert!(
            hit.is_some(),
            "命令面出现写族词却未判写意图（逐词命中 · 宁可误拒不漏放）: {text:?} 族={:?}",
            family.id()
        );
    }

    // ③ 静态调用面：出现写族成员调用 → 必须判写意图（C5 + C2）
    if has_write_member_call(text) {
        assert!(
            hit.is_some(),
            "静态调用面出现变更成员却未判写意图（同族同权 · 宁可误拒不漏放）: {text:?}"
        );
    }

    // ④ 重定向面：含 '>' 且不含句柄复制形态 → 必须判重定向面（C3 + C9）
    if text.contains('>') && !has_handle_duplication(text) {
        let hit = hit.as_ref().unwrap_or_else(|| {
            panic!("命令含 '>' 却未判重定向写意图（重定向面一律视为落盘意图）: {text:?}")
        });
        assert!(
            REDIRECT_LABELS.contains(&family_label(&hit.surface)),
            "含 '>' 的命令命中面必须是重定向面: {text:?} => {}",
            hit.surface
        );
    }

    // ⑤ 精确性：无 '>' / 无 '&'（句柄复制掩码不改字面）且无成员调用、无 sed-tar 特判
    //    时，命中面必须逐字等于治理层族标签；没有写族词时必须返回 None（C1 零误伤）。
    if !text.contains('>')
        && !text.contains('&')
        && member_calls(text).is_empty()
        && !has_sed_or_tar_word(text)
    {
        match (&hit, first_write_family_word(text)) {
            (None, None) => {}
            (Some(hit), Some(family)) => assert_eq!(
                family_label(&hit.surface),
                family.hit_label(),
                "命中面族标签必须与治理层族标签逐字相同: {text:?} => {}",
                hit.surface
            ),
            (Some(hit), None) => panic!(
                "纯读命令被误判写意图（命中返回拒绝面；纯读命令返回 None）: {text:?} => {}",
                hit.surface
            ),
            (None, Some(family)) => panic!(
                "写族词未被判写意图（逐词命中）: {text:?} 族={:?}",
                family.id()
            ),
        }
    }
}

/// 结构化输入面：形态已知的命令 → 形态级契约断言。
fn check_shaped(command: &str, expect: &Expect) {
    let hit = scan_shell_write_intent(command);
    match expect {
        Expect::NoHit => assert!(
            hit.is_none(),
            "纯读/白名单命令被误判写意图: {command:?} => {hit:?}"
        ),
        Expect::HitLabel(label) => {
            let hit = hit.unwrap_or_else(|| panic!("必须判写意图: {command:?}"));
            assert_eq!(
                family_label(&hit.surface),
                *label,
                "命中面族标签不符: {command:?} => {}",
                hit.surface
            );
        }
        Expect::HitRedirect => {
            let hit = hit.unwrap_or_else(|| panic!("重定向面必须判写意图: {command:?}"));
            assert!(
                REDIRECT_LABELS.contains(&family_label(&hit.surface)),
                "命中面必须是重定向面: {command:?} => {}",
                hit.surface
            );
        }
        Expect::MaybeRedirect => {
            if let Some(hit) = hit {
                assert!(
                    REDIRECT_LABELS.contains(&family_label(&hit.surface)),
                    "句柄复制链的命中面只能是重定向面: {command:?} => {}",
                    hit.surface
                );
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // ── ① 原始输入面：任意字节当命令扫 ────────────────────────────────
    if let Ok(text) = std::str::from_utf8(data) {
        check_raw(text);
    }

    // ── ② 结构化输入面：字节流选 token，拼出形态已知的命令 ──────────────
    const SHAPES: [Shape; 10] = [
        Shape::ReadOnly,
        Shape::FamilyWord,
        Shape::FamilyWordQuoted,
        Shape::Redirect,
        Shape::RedirectQuoted,
        Shape::HandleDup,
        Shape::SpacedHandleDup,
        Shape::HandleDupChain,
        Shape::MemberCall,
        Shape::ReadOnlyMember,
    ];
    let mut stream = Stream { data, position: 0 };
    let shape = *stream.pick(&SHAPES);
    let (command, expect) = build(&mut stream, shape);
    check_shaped(&command, &expect);
});
