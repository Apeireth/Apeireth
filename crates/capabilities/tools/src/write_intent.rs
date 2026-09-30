//! shell 命令写意图词法扫描 —— 文件写入总闸（file_write）的 shell 侧执行面。
//!
//! 病灶（内测实锤）：文件写入开关此前只拦受控写文件工具（补丁式写入），
//! shell 的 `echo xxx > file` 照样落盘 —— 开关名与权力不符（摆设开关）。
//! 整改口径：**文件写入只有唯一总闸**，shell 命令执行前先做写意图词法扫描，
//! 命中且总闸未开 → 拒绝即帧（`pipeline.pre_deny`，文案明示同受一闸）；
//! 总闸开 → shell 行为不变（仍走既有 guardrail / 审批链 / 风险映射）。
//!
//! 扫描口径：**保守优先，宁可误拒不漏放**。
//! 命中面三族：
//! 1. 重定向面：`>` / `>>` / `N>` / `&>` / `<>` 一律视为落盘意图；
//! 2. 命令面：删除 / 移动 / 复制 / 改名 / 建目录族与文件写入族（逐词命中）；
//! 3. 脚本引擎等值命令面：与上述操作等值的命令面（大小写不敏感）。
//!
//! 边界（白名单与误拒口径逐条写在 [`mask_handle_duplication`] 的注释里，
//! 写意图扫描矩阵测试锁住该注释与两侧边界）。

/// 命中的写意图面（人读描述，进入拒绝帧文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteIntentHit {
    /// 人读面描述，例如 `重定向 ">"` / `删除族命令 "del"`。
    pub surface: String,
}

/// 删除族：删文件/删目录（cmd 内建与类比命令面）。
const DELETE_FAMILY: &[&str] = &[
    "del",
    "erase",
    "rd",
    "rmdir",
    "rm",
    "unlink",
    "shred",
    "Remove-Item",
];

/// 移动 / 复制 / 改名族。
const MOVE_COPY_FAMILY: &[&str] = &[
    "move",
    "copy",
    "xcopy",
    "robocopy",
    "ren",
    "rename",
    "mv",
    "cp",
    "ln",
    "Move-Item",
    "Copy-Item",
];

/// 建目录 / 建文件族。
const CREATE_FAMILY: &[&str] = &[
    "mkdir",
    "md",
    "touch",
    "New-Item",
    "New-TemporaryFile",
    "install",
];

/// 文件写入族（改内容 / 追加 / 覆盖输出 / 原地截断）。
const WRITE_CONTENT_FAMILY: &[&str] = &[
    "tee",
    "dd",
    "truncate",
    "Set-Content",
    "Add-Content",
    "Out-File",
    "Clear-Content",
    "Set-Item",
    "Rename-Item",
    "Export-Csv",
    "Export-Clixml",
    "Start-Transcript",
];

/// 各族的面名（拒绝帧里的人读标签）。
const FAMILY_LABELS: [(&str, &[&str]); 4] = [
    ("删除族命令", DELETE_FAMILY),
    ("移动/复制/改名族命令", MOVE_COPY_FAMILY),
    ("建目录/建文件族命令", CREATE_FAMILY),
    ("文件写入族命令", WRITE_CONTENT_FAMILY),
];

/// 段分隔符：管道 / 条件链 / 顺序链 / 分组括号 / 换行 —— 逐段扫描的切点。
/// `&` 的句柄复制形态已在 [`mask_handle_duplication`] 里抹掉，此处可安全切。
const SEGMENT_SEPARATORS: [char; 9] = ['|', '&', ';', '\n', '\r', '(', ')', '{', '}'];

/// 扫描一条 shell 命令的写意图。命中返回拒绝面；纯读命令返回 `None`。
pub fn scan_shell_write_intent(command: &str) -> Option<WriteIntentHit> {
    let masked = mask_handle_duplication(command);
    if let Some(surface) = scan_redirections(&masked) {
        return Some(WriteIntentHit { surface });
    }
    scan_segments_for_write_commands(&masked)
}

/// 白名单：句柄复制 `N>&M`（两头单个数字、紧邻无空格）抹成空格后不判写意图。
///
/// 边界与误拒口径（写意图扫描矩阵测试锁住本注释，改口径必改注释+测试）：
/// * **收进白名单的只有句柄复制**：`cargo build 2>&1` / `echo hi 1>&2` 只把
///   标准流并到另一个句柄，不落盘，不是写意图；
/// * **不收带空格的变体**（`2>& 1`）：词法面不解析 shell 变体，命中即拒——
///   宁可误拒不漏放；
/// * **`N>&M` 之外的 `>` 一律命中**：`cmd > f 2>&1` 的 `> f` 照样拒；
/// * **已知误拒边界**（词法面不解析引号/上下文，宁可误拒不漏放）：
///   引号内的 `>`（`echo "a > b"`）与"只提到命令词"（`echo del is a word`）
///   都会被拒 —— 提到与执行无法词法区分，属明示的误拒面；
/// * **已知漏放边界**（不在这层词法面内，由 shell 审批链 + 沙箱兑底）：
///   解释器内任意写入（脚本/语言运行时自写文件）、编码负载
///   （如 `-EncodedCommand` 形态）、解包类工具的写盘。
fn mask_handle_duplication(command: &str) -> String {
    let bytes = command.as_bytes();
    let mut masked = command.to_string().into_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i].is_ascii_digit()
            && bytes[i + 1] == b'>'
            && bytes[i + 2] == b'&'
            && bytes[i + 3].is_ascii_digit()
        {
            masked[i + 1] = b' ';
            masked[i + 2] = b' ';
            i += 4;
        } else {
            i += 1;
        }
    }
    String::from_utf8(masked).unwrap_or_else(|_| command.to_string())
}

/// 重定向面：任何未被白名单抹掉的 `>`（含 `>>` / `N>` / `&>`）与读写开档 `<>`。
fn scan_redirections(command: &str) -> Option<String> {
    let chars: Vec<char> = command.chars().collect();
    for (idx, ch) in chars.iter().enumerate() {
        match ch {
            '>' => return Some(format!("重定向 {ch:?}")),
            '<' => {
                if chars.get(idx + 1) == Some(&'>') {
                    return Some("读写重定向 \"<>\"".to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// 逐段扫描命令面：切段 → 提词 → 族词命中即拒；外加两条原地改写特判。
fn scan_segments_for_write_commands(command: &str) -> Option<WriteIntentHit> {
    for segment in command.split(|c| SEGMENT_SEPARATORS.contains(&c)) {
        let words = command_words(segment);
        for (label, family) in FAMILY_LABELS {
            for word in &words {
                if family.iter().any(|cmd| word.eq_ignore_ascii_case(cmd)) {
                    return Some(WriteIntentHit {
                        surface: format!("{label} {word:?}"),
                    });
                }
            }
        }
        // 原地改写特判：`sed -i …` / `tar -x…` 落盘改写文件，词组命中即拒
        // （词法面只看词组同段共现，误拒口径同上注释）。
        let lower: Vec<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();
        if lower.iter().any(|w| w == "sed") && lower.iter().any(|w| w.starts_with("-i")) {
            return Some(WriteIntentHit {
                surface: "原地改写命令 \"sed -i\"".to_string(),
            });
        }
        if lower.iter().any(|w| w == "tar") && lower.iter().any(|w| w.starts_with("-x")) {
            return Some(WriteIntentHit {
                surface: "归档解开命令 \"tar -x\"".to_string(),
            });
        }
    }
    None
}

/// 提词：词字符 = 字母数字 + `_` / `-` / `.`（命令名与参数各成一词；
/// 文件名带点不拆词，`file.md` 不会误命中 `md`）。
fn command_words(segment: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// ① 重定向面矩阵：各形态重定向都判写意图。
    #[test]
    fn redirection_surfaces_are_write_intent() {
        for command in [
            "echo xxx > file.txt",
            "echo xxx >> file.txt",
            "echo xxx 1> file.txt",
            "echo xxx 2>> file.txt",
            "type nul > file.txt",
            "cmd &> file.txt",
            "cmd >file.txt",
            "cmd <> file.txt",
            "sort < in.txt > out.txt",
        ] {
            let hit = scan_shell_write_intent(command)
                .unwrap_or_else(|| panic!("{command} 必须判写意图"));
            assert!(
                hit.surface.contains("重定向"),
                "{command} 命中面应为重定向：{}",
                hit.surface
            );
        }
    }

    /// ② 删除/移动/复制/改名/建目录族 + 脚本引擎等值命令面矩阵。
    #[test]
    fn deletion_move_and_script_engine_families_are_write_intent() {
        for command in [
            "del file.txt",
            "erase file.txt",
            "move a.txt b.txt",
            "copy a.txt b.txt",
            "rename a.txt b.txt",
            "ren a.txt b.txt",
            "mkdir new_dir",
            "md new_dir",
            "rd old_dir",
            "rmdir old_dir",
            "Remove-Item file.txt",
            "Move-Item a.txt b.txt",
            "Copy-Item a.txt b.txt",
            "Rename-Item a.txt b.txt",
            "New-Item file.txt",
            "Set-Content file.txt hello",
            "Add-Content file.txt hello",
            "Out-File file.txt",
            "Clear-Content file.txt",
            "rm file.txt",
            "mv a.txt b.txt",
            "cp a.txt b.txt",
            "mkdir -p new_dir",
            "touch file.txt",
            "tee file.txt",
            "sed -i s/a/b/ file.txt",
            "tar -xzf bundle.tgz",
        ] {
            let hit = scan_shell_write_intent(command)
                .unwrap_or_else(|| panic!("{command} 必须判写意图"));
            assert!(!hit.surface.is_empty(), "{command} 命中面不能为空");
        }
    }

    /// ③ 管道/条件链逐段扫描：写意图在任何一段都要扫到。
    #[test]
    fn pipe_and_chain_segments_are_scanned_segment_by_segment() {
        for command in [
            "echo a | tee file.txt",
            "echo a && del file.txt",
            "type a.txt & del b.txt",
            "cd dir; mkdir sub",
            "echo ok || rd dir",
            "if exist a.txt (del a.txt)",
            "for %i in (a.txt) do del %i",
            "echo a\r\ndel file.txt",
        ] {
            let hit = scan_shell_write_intent(command)
                .unwrap_or_else(|| panic!("{command} 逐段扫描必须命中写意图"));
            assert!(!hit.surface.is_empty(), "{command} 命中面不能为空");
        }
        // 纯读链条不误拒（不含写意图面）。
        for command in [
            "echo hi | findstr hi",
            "dir | more",
            "type a.txt | sort",
            "cd dir && dir",
        ] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 纯读链条不得误拒"
            );
        }
    }

    /// ④ 白名单只收句柄复制 + 误拒边界有注释行解释（口径改动必改注释）。
    #[test]
    fn handle_duplication_whitelist_and_false_deny_boundary_are_commented() {
        // 白名单：句柄复制不判写意图。
        for command in [
            "cargo build 2>&1",
            "echo hi 1>&2",
            "pytest -q 2>&1",
            "gcc main.c -o main 2>&1",
        ] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 句柄复制不落盘，不得判写意图"
            );
        }
        // 白名单之外照拒：句柄复制掩盖不了真正的重定向。
        assert!(scan_shell_write_intent("cmd > f.txt 2>&1").is_some());
        assert!(scan_shell_write_intent("cmd 2>&1 > f.txt").is_some());
        // 明示的误拒面（宁可误拒不漏放）也要真的拒：
        assert!(scan_shell_write_intent("echo \"a > b\"").is_some());
        assert!(scan_shell_write_intent("echo del is a word").is_some());

        // 注释行解释边界：白名单边界 + 误拒口径 + 漏放兜底三段都要在源码里。
        let source = include_str!("write_intent.rs");
        assert!(
            source.contains("收进白名单的只有句柄复制"),
            "write_intent.rs 必须有注释行解释白名单边界"
        );
        assert!(
            source.contains("已知误拒边界"),
            "write_intent.rs 必须有注释行解释误拒边界"
        );
        assert!(
            source.contains("已知漏放边界"),
            "write_intent.rs 必须有注释行解释漏放边界（兑底层）"
        );
    }

    /// 只读命令零误伤（总闸关时行为如常的词法前提）。
    #[test]
    fn read_only_commands_are_not_write_intent() {
        for command in [
            "echo hi",
            "cd",
            "pwd",
            "type file.txt",
            "cat file.txt",
            "git status",
            "cargo test --workspace",
            "dir /w",
            "findstr /? ",
        ] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 只读命令不得判写意图"
            );
        }
    }
}
