//! shell 命令写意图词法扫描 —— 文件写入总闸（file_write）的 shell 侧执行面。
//!
//! 病灶（内测实锤）：文件写入开关此前只拦受控写文件工具（补丁式写入），
//! shell 的 `echo xxx > file` 照样落盘 —— 开关名与权力不符（摆设开关）。
//! 整改口径：**文件写入只有唯一总闸**，shell 命令执行前先做写意图词法扫描，
//! 命中且总闸未开 → 拒绝即帧（`pipeline.pre_deny`，文案明示同受一闸）；
//! 总闸开 → shell 行为不变（仍走既有 guardrail / 审批链 / 风险映射）。
//!
//! 扫描口径：**保守优先，宁可误拒不漏放**。
//! 命中面四族：
//! 1. 重定向面：`>` / `>>` / `N>` / `&>` / `<>` 一律视为落盘意图；
//! 2. 命令面：删除 / 移动 / 复制 / 改名 / 建目录族与文件写入族（逐词命中）；
//! 3. 脚本引擎等值命令面：与上述操作等值的命令面（大小写不敏感）；
//! 4. 脚本引擎静态调用面：`::变更成员` / `.变更成员(` 形态（`Delete` / `Write*` /
//!    `Create*` / `Move*` / `Copy*` / `Append*` / `Replace` / `Set*` 等变更成员），
//!    与命令面族词**同族同权**——`[System.IO.File]::Delete` 与 `Remove-Item` /
//!    `del` 判入同一删除族，同一总闸、同一命中族名、同一审批档。
//!
//! **族词表同源（E-P2）**：本面的族词 / 成员词头 / 命中标签 / 变体匹配
//! （大小写 / 前缀缩写 / 可执行后缀）全部引自治理层单一来源
//! `apeireth_governance::approval_family`（命令审批族词表），两处族词一致
//! 由本文件的同源断言测试钉死；写总闸只执法**写四族**（网络族 / 进程族
//! 不属写意图，词表同源但不入本闸）。
//!
//! 边界（白名单与误拒口径逐条写在 [`mask_handle_duplication`] 的注释里，
//! 写意图扫描矩阵测试锁住该注释与两侧边界）。

use apeireth_governance::approval_family::{
    command_words, member_calls, word_families, CommandFamily, MemberCallForm, SEGMENT_SEPARATORS,
};

/// 命中的写意图面（人读描述，进入拒绝帧文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteIntentHit {
    /// 人读面描述，例如 `重定向 ">"` / `删除族命令 "del"`。
    pub surface: String,
}

/// 写四族（写总闸执法面，词表序 = 命中优先级）：网络族 / 进程族同源在表
/// 但不入写闸。
const WRITE_FAMILIES: [CommandFamily; 4] = [
    CommandFamily::Delete,
    CommandFamily::MoveCopy,
    CommandFamily::Create,
    CommandFamily::WriteContent,
];

/// 扫描一条 shell 命令的写意图。命中返回拒绝面；纯读命令返回 `None`。
pub fn scan_shell_write_intent(command: &str) -> Option<WriteIntentHit> {
    let masked = mask_handle_duplication(command);
    if let Some(surface) = scan_redirections(&masked) {
        return Some(WriteIntentHit { surface });
    }
    if let Some(hit) = scan_static_member_calls(&masked) {
        return Some(hit);
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
/// * **脚本引擎静态调用面同族同权**：`::变更成员` / `.变更成员(`（如
///   `[System.IO.File]::Delete` / `[System.IO.Directory]::Delete` /
///   `([System.IO.FileInfo]'x').Delete()`）与命令面族词判入同一族（命中面
///   族名逐字相同），`Delete*`/`Write*`/`Create*`/`Move*`/`Copy*`/`Append*`/
///   `Replace*`/`Set*` 等变更成员词头命中，只读成员（`Read*`/`Get*`/`Exists`
///   等）不命中 —— 两条删除路径同受一闸、同一审批档；
/// * **已知误拒边界**（词法面不解析引号/上下文，宁可误拒不漏放）：
///   引号内的 `>`（`echo "a > b"`）、"只提到命令词"（`echo del is a word`）
///   与"只提到调用形态文本"（`findstr "::Delete" notes.md`）都会被拒 ——
///   提到与执行无法词法区分，属明示的误拒面；
/// * **已知漏放边界**（不在这层词法面内，由 shell 审批链 + 沙箱兑底）：
///   解释器内任意写入（脚本/语言运行时自写文件）、编码负载
///   （如 `-EncodedCommand` 形态）、解包类工具的写盘、动态成员名/反射调用
///   （成员名经字符串拼接或变量拼出、`GetMethod(...).Invoke(...)` 形态）。
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

/// 脚本引擎静态调用面：`::变更成员`（静态调用 / 方法组）与 `.变更成员(`
/// （实例调用）两种形态，按治理层成员词头表判族，
/// 与命令面族词同族同权（`[System.IO.File]::Delete` ≡ `Remove-Item`）。
/// 成员名大小写不敏感；`::` 后允许空白（宁可误拒不漏放），`.成员` 后要求
/// 调用括号（文件名带点的 `.txt` / `.md` 等不误命中）。成员调用形态可跨
/// 段分隔符（`(` `)` 本身就是切段符），故整条命令面整体扫描。只执法写四族
/// （网络族 / 进程族成员不判写意图）。
fn scan_static_member_calls(command: &str) -> Option<WriteIntentHit> {
    for call in member_calls(command) {
        let Some(family) = apeireth_governance::approval_family::member_family(&call.name) else {
            continue;
        };
        if !family.is_write_family() {
            continue;
        }
        let surface_call = match call.form {
            MemberCallForm::Static => format!("::{}", call.name),
            MemberCallForm::Instance => format!(".{}()", call.name),
        };
        return Some(WriteIntentHit {
            surface: format!("{} {:?}", family.hit_label(), surface_call),
        });
    }
    None
}

/// 逐段扫描命令面：切段 → 提词 → 族词命中即拒；外加两条原地改写特判。
/// 族词判定走治理层 [`word_families`]（精确命中优先于缩写命中，含大小写 /
/// 前缀缩写 / 可执行后缀变体口径），只执法写四族。
fn scan_segments_for_write_commands(command: &str) -> Option<WriteIntentHit> {
    for segment in command.split(|c| SEGMENT_SEPARATORS.contains(&c)) {
        let words = command_words(segment);
        for word in &words {
            let families = word_families(word);
            for family in WRITE_FAMILIES {
                if families.contains(&family) {
                    return Some(WriteIntentHit {
                        surface: format!("{} {:?}", family.hit_label(), word),
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

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_governance::approval_family::{
        member_family, touched_families, word_families, MEMBER_PREFIX_FAMILIES,
    };

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

    /// ② 删除/移动/复制/改名/建目录族 + 脚本引擎等值命令面 + 静态调用面矩阵。
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
            // 脚本引擎静态调用面（删除族/写入族同权）。
            "[System.IO.File]::Delete('file.txt')",
            "[System.IO.Directory]::Delete('old_dir')",
            "([System.IO.FileInfo]'file.txt').Delete()",
            "[System.IO.File]::WriteAllText('file.txt','hello')",
            "[System.IO.File]::AppendAllText('file.txt','hello')",
            "[System.IO.Directory]::Create('new_dir')",
            "[System.IO.File]::Move('a.txt','b.txt')",
            "[System.IO.File]::Copy('a.txt','b.txt')",
        ] {
            let hit = scan_shell_write_intent(command)
                .unwrap_or_else(|| panic!("{command} 必须判写意图"));
            assert!(!hit.surface.is_empty(), "{command} 命中面不能为空");
        }
    }

    /// 命中面族名（命中面文案 = `族名 空格 引号命中词`，族名不含空格）。
    fn family_label(surface: &str) -> &str {
        surface.split(' ').next().unwrap_or(surface)
    }

    /// ②附1 删除对称性：两条删除路径（命令面族词与静态调用面成员调用）命中
    /// 同一族名；写入/建/移动族同理 —— 同族同权的词法前提。
    #[test]
    fn static_call_member_forms_join_the_same_family_as_their_command_forms() {
        for (baseline, variant, expected) in [
            (
                "Remove-Item file.txt",
                "[System.IO.File]::Delete('file.txt')",
                "删除族命令",
            ),
            (
                "del file.txt",
                "[System.IO.Directory]::Delete('file.txt')",
                "删除族命令",
            ),
            (
                "rd old_dir",
                "([System.IO.FileInfo]'file.txt').Delete()",
                "删除族命令",
            ),
            (
                "rm file.txt",
                "[System.IO.FileInfo]::new('file.txt').Delete()",
                "删除族命令",
            ),
            (
                "Set-Content file.txt hello",
                "[System.IO.File]::WriteAllText('file.txt','hello')",
                "文件写入族命令",
            ),
            (
                "Add-Content file.txt hello",
                "[System.IO.File]::AppendAllText('file.txt','hello')",
                "文件写入族命令",
            ),
            (
                "Out-File file.txt",
                "[System.IO.File]::Replace('a.txt','b.txt','c.txt')",
                "文件写入族命令",
            ),
            (
                "Clear-Content file.txt",
                "[System.IO.File]::SetAttributes('file.txt','Hidden')",
                "文件写入族命令",
            ),
            (
                "mkdir new_dir",
                "[System.IO.Directory]::Create('new_dir')",
                "建目录/建文件族命令",
            ),
            (
                "New-Item file.txt",
                "[System.IO.File]::Create('file.txt')",
                "建目录/建文件族命令",
            ),
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
        ] {
            let base = scan_shell_write_intent(baseline)
                .unwrap_or_else(|| panic!("{baseline} 必须判写意图"));
            let variant_hit = scan_shell_write_intent(variant)
                .unwrap_or_else(|| panic!("{variant} 必须判写意图"));
            assert_eq!(
                family_label(&base.surface),
                expected,
                "{baseline}: {}",
                base.surface
            );
            assert_eq!(
                family_label(&variant_hit.surface),
                expected,
                "{variant}: {}",
                variant_hit.surface
            );
        }
    }

    /// ②附2 删除族静态调用面形态矩阵：大小写 / 空白 / 实例构造 / 成员词头
    /// 变体一律判入删除族（保守优先，宁可误拒不漏放）。
    #[test]
    fn delete_member_call_variants_all_join_the_delete_family() {
        for variant in [
            "[System.IO.File]::Delete('file.txt')",
            "[system.io.file]::delete('file.txt')",
            "[System.IO.File]::  Delete('file.txt')",
            "[System.IO.Directory]::Delete('old_dir', $true)",
            "([System.IO.FileInfo]'file.txt').Delete()",
            "([System.IO.FileInfo]'file.txt').Delete ()",
            "[System.IO.FileInfo]::new('file.txt').Delete()",
            "$fs.DeleteFile('file.txt')",
            "$dir.DeleteFolder('old_dir')",
        ] {
            let hit = scan_shell_write_intent(variant)
                .unwrap_or_else(|| panic!("{variant} 必须判删除族写意图"));
            assert_eq!(
                family_label(&hit.surface),
                "删除族命令",
                "{variant}: {}",
                hit.surface
            );
        }
    }

    /// ②附3 只读成员调用零误伤：`Read*`/`Get*`/`Exists`/`OpenText` 等不判写意图。
    #[test]
    fn read_only_member_calls_are_not_write_intent() {
        for command in [
            "[System.IO.File]::ReadAllText('file.txt')",
            "[System.IO.File]::Exists('file.txt')",
            "[System.IO.Directory]::GetFiles('new_dir')",
            "([System.IO.FileInfo]'file.txt').OpenText()",
            "$fs.get_Length()",
        ] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 只读成员调用不得判写意图"
            );
        }
    }

    /// ②附4 非写族成员调用不入写闸（词表同源但网络/进程族不属写意图）：
    /// `Download*` / `Kill*` / `Start*` 成员不判写意图。
    #[test]
    fn non_write_family_member_calls_stay_out_of_the_write_gate() {
        for command in [
            "[System.Net.WebClient]::DownloadFile('http://example.invalid','x')",
            "$proc.Kill()",
            "$proc.Start()",
        ] {
            assert_eq!(
                scan_shell_write_intent(command),
                None,
                "{command} 非写族成员调用不得判写意图"
            );
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
        // 静态调用面口径（同族同权 + 误拒/漏放边界）同样逐条有注释行，
        // 改口径必改注释+测试（本测试即锁）。
        assert!(
            source.contains("脚本引擎静态调用面同族同权"),
            "write_intent.rs 必须有注释行解释静态调用面同族同权口径"
        );
        assert!(
            source.contains("只提到调用形态文本"),
            "write_intent.rs 必须有注释行解释静态调用面的误拒边界"
        );
        assert!(
            source.contains("动态成员名/反射调用"),
            "write_intent.rs 必须有注释行解释静态调用面的漏放边界"
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

    /// ⑤ 写意图族词表同源断言（两处族词一致，E-P2）：写意图扫描命中面族名
    /// 与治理层族归一（`touched_families` 触达面）逐一相符——同一份族词、
    /// 同一命中标签、同一变体匹配，任何一族的词表漂移都会在此断掉。
    #[test]
    fn write_intent_family_vocabulary_agrees_with_the_governance_source() {
        // 词表同源：写四族每个族词（含短形态/长形态）在写意图面命中本族标签，
        // 在治理层归一本族。
        for family in WRITE_FAMILIES {
            for word in family.words() {
                let hit =
                    scan_shell_write_intent(word).unwrap_or_else(|| panic!("{word} 必须判写意图"));
                assert_eq!(
                    family_label(&hit.surface),
                    family.hit_label(),
                    "族词 {word} 的命中标签必须与治理层族 {} 一致",
                    family.id()
                );
                assert!(
                    touched_families(word).contains(&family),
                    "族词 {word} 在治理层必须归 {} 族",
                    family.id()
                );
                assert!(
                    word_families(word).contains(&family),
                    "族词 {word} 的族归一必须落在 {} 族",
                    family.id()
                );
            }
        }
        // 成员词头同源：写族成员词头两种调用形态命中本族标签，治理层同族。
        for (family, prefix) in MEMBER_PREFIX_FAMILIES {
            if !family.is_write_family() {
                // 非写族词头同源在表，但不入写闸（上一测试钉边界）。
                assert!(
                    member_family(&format!("{prefix}X")) == Some(*family),
                    "成员词头 {prefix} 在治理层必须归 {} 族",
                    family.id()
                );
                continue;
            }
            for call in [
                format!("[Type]::{prefix}X('y')"),
                format!("$obj.{prefix}X('y')"),
            ] {
                let hit =
                    scan_shell_write_intent(&call).unwrap_or_else(|| panic!("{call} 必须判写意图"));
                assert_eq!(
                    family_label(&hit.surface),
                    family.hit_label(),
                    "成员词头 {prefix} 的命中标签必须与治理层族 {} 一致",
                    family.id()
                );
            }
        }
    }
}
