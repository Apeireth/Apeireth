//! 泊位磁贴面 —— 工作区根限定下的目录列举与磁贴写盘（批次推进 + 安全批 B3）。
//!
//! 契约（I-starport-builtins.md「泊位」）：
//!   1. 只读列举 —— `list_workspace_dir` 仅读目录元数据，不写不删不建；
//!   2. 诚实报错 —— 空路径/不存在/非目录/权限拒绝/条目读取失败一律如实报
//!      （fs_err 错误自带路径，仓库纪律 clippy 禁 std::fs）；
//!   3. 照实列举 —— 隐藏文件照实列出（不滤不猜）；符号链接不穿越（按条目
//!      自身形态标注）；目录 size_bytes 恒 0（不冒充文件大小）；
//!   4. 展示序 —— 目录优先 → 名称大小写不敏感升序 → 同名大小写稳定序。
//!
//! 工作区根限定（安全批 B3，隐患分析 #4）：
//!   5. 权限边界 = 已登记工作区根 —— 前端传入的路径只是请求、不是边界；
//!      基路径 canonicalize 后必须是登记根自身或其后代（`..` 归一、符号链接/
//!      目录联接等重解析点解析成最终形态后再比祖先），根外一律拒绝；
//!   6. fail-closed —— 根未登记 / 根不可用 / 规范化失败 / 越出根，一律拒绝并如实报；
//!   7. 接线点 —— 四个 `#[tauri::command]` 从 `State<Arc<BackendSupervisor>>`
//!      读登记根（lib.rs `.manage` 注入的唯一真源），纯函数层做全部校验与
//!      文件操作（本层可单测）。已知残余边界：校验与文件操作之间无句柄级
//!      原子绑定（TOCTOU 窗口），见批次报告。
//!
//! 测试：`cargo test`（排序纯函数 + 真临时目录往返 + 根外/逃逸对抗用例）。

use crate::backend_supervisor::BackendSupervisor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

/// 目录条目（照实形态：名称 / 是否目录 / 大小）。
#[derive(Debug, serde::Serialize)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

/// 目录列举结果。
#[derive(Debug, serde::Serialize)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirEntryInfo>,
}

/// 展示序纯函数：目录优先 → 名称大小写不敏感升序 → 原名稳定序。
fn sort_entries(entries: &mut [DirEntryInfo]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

// ---------------------------------------------------------------------------
// 工作区根限定（安全批 B3）：登记根解析 + 路径钉根（全部 fail-closed）。
// ---------------------------------------------------------------------------

/// 登记根解析（fail-closed）：空登记 = 未登记，拒绝；登记值必须是现存目录。
///
/// 登记源是后端监督器状态里唯一的工作区根（`set_workspace_dir` 校验并持久化、
/// 启动时从 app-data 恢复），不是前端随请求传入的任何路径。
pub fn resolve_registered_root(registered: &str) -> Result<PathBuf, String> {
    let trimmed = registered.trim();
    if trimmed.is_empty() {
        return Err("工作区根未登记，文件操作一律拒绝（fail-closed）".to_string());
    }
    let root = PathBuf::from(trimmed);
    if !root.is_dir() {
        return Err(format!("登记的工作区根不可用（如实报）：{trimmed}"));
    }
    Ok(root)
}

/// 把请求路径钉进登记根内（fail-closed）：两侧都 canonicalize 后比祖先。
///
/// 比较发生在**最终形态**上：`..` 归一、符号链接/目录联接等重解析点全部解析
/// 之后才比，所以词法上跳不出（`..`）、经链接跳不出（symlink/junction 逃逸）、
/// 同前缀兄弟目录（`ws` vs `ws2`）也骗不过分量级比较。返回规范形态，后续
/// 文件操作只作用于规范形态。
fn contain_within_root(root: &Path, requested: &Path) -> Result<PathBuf, String> {
    let canon_root = fs_err::canonicalize(root)
        .map_err(|error| format!("登记根不可规范化（如实报）：{error}"))?;
    let canon = fs_err::canonicalize(requested)
        .map_err(|error| format!("路径不可规范化（如实报）：{error}"))?;
    if !canon.starts_with(&canon_root) {
        return Err(format!(
            "路径越出已登记工作区根，拒绝（fail-closed）：{}",
            requested.display()
        ));
    }
    Ok(canon)
}

/// 写盘基目录解析：空/非目录如实拒 + 根内钉定。
fn resolve_base_in_root(root: &Path, base: &str) -> Result<PathBuf, String> {
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return Err(format!("基目录不存在或非目录：{trimmed}"));
    }
    let contained = contain_within_root(root, Path::new(trimmed))?;
    if !contained.is_dir() {
        return Err(format!("基目录不存在或非目录：{trimmed}"));
    }
    Ok(contained)
}

/// 只读列举一个目录（权限/形态错误如实报）；`root` = 已登记工作区根。
pub fn list_workspace_dir_in_root(root: &Path, path: &str) -> Result<DirListing, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("路径为空".to_string());
    }
    let dir = contain_within_root(root, Path::new(trimmed))?;
    let metadata =
        fs_err::metadata(&dir).map_err(|error| format!("路径不可读（如实报）：{error}"))?;
    if !metadata.is_dir() {
        return Err(format!("非目录拒绝：{trimmed}"));
    }
    let read = fs_err::read_dir(&dir).map_err(|error| format!("列举失败（如实报）：{error}"))?;
    let mut entries = Vec::new();
    for entry in read {
        let entry = entry.map_err(|error| format!("读取条目失败（如实报）：{error}"))?;
        // 条目自身形态（不穿越符号链接）；隐藏条目照实列。
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取条目形态失败（如实报）：{error}"))?;
        let is_dir = file_type.is_dir();
        let size_bytes = if is_dir {
            // 目录大小不冒充文件大小（0 装）。
            0
        } else {
            fs_err::symlink_metadata(entry.path())
                .map(|meta| meta.len())
                .unwrap_or(0)
        };
        entries.push(DirEntryInfo {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir,
            size_bytes,
        });
    }
    sort_entries(&mut entries);
    Ok(DirListing {
        path: trimmed.to_string(),
        entries,
    })
}

// ---------------------------------------------------------------------------
// 磁贴写盘（泊位进阶件）：建目录 / 改名 / 删条目——用户直操面（同终端口径）。
// 0 装：条目名含分隔符/控制字符/`.`/`..` 拒绝；rename 源必须在、目标必须不在；
// delete 目录非空拒绝（防误删，递归删除为增强位）；错误带路径（fs_err 纪律）。
// 基路径一律先经 contain_within_root 钉进登记根（安全批 B3）。
// ---------------------------------------------------------------------------

/// 条目名单层校验（非空、无分隔符、无控制字符、非 `.`/`..` 相对分量）。
fn validate_entry_name(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("条目名不能为空".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.chars().any(char::is_control) {
        return Err("条目名含路径分隔符或控制字符，拒绝".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("条目名是相对路径分量（. 或 ..），拒绝".to_string());
    }
    Ok(trimmed)
}

/// 建一层目录（已存在拒绝；返回建成路径）；`root` = 已登记工作区根。
pub fn create_dir_entry_in_root(root: &Path, base: &str, name: &str) -> Result<String, String> {
    let base_dir = resolve_base_in_root(root, base)?;
    let clean = validate_entry_name(name)?;
    let target = base_dir.join(clean);
    if target.exists() {
        return Err(format!("条目已存在：{}", target.display()));
    }
    fs_err::create_dir(&target).map_err(|error| format!("建目录失败（如实报）：{error}"))?;
    Ok(target.display().to_string())
}

/// 同目录改名（源必须在、目标必须不在）；`root` = 已登记工作区根。
pub fn rename_entry_in_root(
    root: &Path,
    base: &str,
    from: &str,
    to: &str,
) -> Result<String, String> {
    let base_dir = resolve_base_in_root(root, base)?;
    let from_clean = validate_entry_name(from)?;
    let to_clean = validate_entry_name(to)?;
    let source = base_dir.join(from_clean);
    let target = base_dir.join(to_clean);
    if !source.exists() {
        return Err(format!("源条目不存在：{}", source.display()));
    }
    if target.exists() {
        return Err(format!("目标已存在（不覆盖）：{}", target.display()));
    }
    fs_err::rename(&source, &target).map_err(|error| format!("改名失败（如实报）：{error}"))?;
    Ok(target.display().to_string())
}

/// 删条目（文件直删；空目录可删；非空目录拒绝——防误删）；`root` = 已登记工作区根。
pub fn delete_entry_in_root(root: &Path, base: &str, name: &str) -> Result<String, String> {
    let base_dir = resolve_base_in_root(root, base)?;
    let clean = validate_entry_name(name)?;
    let target = base_dir.join(clean);
    let meta =
        fs_err::metadata(&target).map_err(|error| format!("读元数据失败（如实报）：{error}"))?;
    if meta.is_dir() {
        let mut children =
            fs_err::read_dir(&target).map_err(|error| format!("读目录失败（如实报）：{error}"))?;
        if children.next().is_some() {
            return Err(format!(
                "目录非空，拒绝删除（防误删）：{}",
                target.display()
            ));
        }
        fs_err::remove_dir(&target).map_err(|error| format!("删目录失败（如实报）：{error}"))?;
    } else {
        fs_err::remove_file(&target).map_err(|error| format!("删文件失败（如实报）：{error}"))?;
    }
    Ok(target.display().to_string())
}

// ---------------------------------------------------------------------------
// 命令层（接线点）：登记根取自后端监督器状态（lib.rs `.manage` 注入的唯一真源，
// 即 set_workspace_dir 登记、启动恢复的那份），前端传入的路径不再是权限边界。
// 根未登记/校验失败一律拒绝（fail-closed），错误如实报。IPC 参数名不变
// （State 由 Tauri 注入，不占 invoke 参数）。
// ---------------------------------------------------------------------------

/// 只读列举（命令面）：登记根内才列举。
#[tauri::command]
pub async fn list_workspace_dir(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    path: String,
) -> Result<DirListing, String> {
    let root = resolve_registered_root(&supervisor.get_workspace_dir().await)?;
    list_workspace_dir_in_root(&root, &path)
}

/// 建目录（命令面）：登记根内才建。
#[tauri::command]
pub async fn create_dir_entry(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    path: String,
    name: String,
) -> Result<String, String> {
    let root = resolve_registered_root(&supervisor.get_workspace_dir().await)?;
    create_dir_entry_in_root(&root, &path, &name)
}

/// 同目录改名（命令面）：登记根内才改。
#[tauri::command]
pub async fn rename_entry(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    path: String,
    from: String,
    to: String,
) -> Result<String, String> {
    let root = resolve_registered_root(&supervisor.get_workspace_dir().await)?;
    rename_entry_in_root(&root, &path, &from, &to)
}

/// 删条目（命令面）：登记根内才删。
#[tauri::command]
pub async fn delete_entry(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    path: String,
    name: String,
) -> Result<String, String> {
    let root = resolve_registered_root(&supervisor.get_workspace_dir().await)?;
    delete_entry_in_root(&root, &path, &name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> DirEntryInfo {
        DirEntryInfo {
            name: name.to_string(),
            is_dir,
            size_bytes: if is_dir { 0 } else { 1 },
        }
    }

    /// 测试临时目录（先清残留后建，进程 id + 标签唯一名）。
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("starport-{tag}-{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("建测试临时目录");
        dir
    }

    /// 建目录链接（Windows 语义下可行的写法）：符号链接需特权，标准用户可建
    /// 目录联接（junction）；先试符号链接，无特权退 mklink /J。
    #[cfg(windows)]
    fn make_dir_link(link: &Path, target: &Path) -> bool {
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .is_ok_and(|status| status.success());
        made && fs_err::symlink_metadata(link).is_ok()
    }

    #[cfg(not(windows))]
    fn make_dir_link(link: &Path, target: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[test]
    fn sort_puts_dirs_first_then_names() {
        let mut entries = vec![
            entry("zeta.txt", false),
            entry("Beta", true),
            entry("alpha.txt", false),
            entry("Alpha", true),
        ];
        sort_entries(&mut entries);
        let names: Vec<&str> = entries.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Beta", "alpha.txt", "zeta.txt"]);
    }

    #[test]
    fn sort_is_stable_on_case_ties() {
        let mut entries = vec![
            entry("abc", false),
            entry("ABC", false),
            entry("aBc", false),
        ];
        sort_entries(&mut entries);
        let names: Vec<&str> = entries.iter().map(|item| item.name.as_str()).collect();
        // 大小写不敏感同组：原名升序稳定（ABC < aBc < abc）。
        assert_eq!(names, vec!["ABC", "aBc", "abc"]);
    }

    #[test]
    fn unregistered_root_fails_closed() {
        let empty = resolve_registered_root("").unwrap_err();
        assert!(empty.contains("未登记"), "空登记必须按未登记拒绝：{empty}");
        let blank = resolve_registered_root("   ").unwrap_err();
        assert!(
            blank.contains("未登记"),
            "空白登记必须按未登记拒绝：{blank}"
        );

        let missing = std::env::temp_dir().join(format!("starport-no-root-{}", std::process::id()));
        assert!(
            resolve_registered_root(&missing.to_string_lossy()).is_err(),
            "登记根不可用必须拒绝"
        );

        let dir = scratch("root");
        assert_eq!(
            resolve_registered_root(&dir.to_string_lossy()).unwrap(),
            dir,
            "现存登记根必须照常放行"
        );
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn listing_rejects_empty_and_non_dir() {
        let dir = scratch("list-nondir");
        assert!(
            list_workspace_dir_in_root(&dir, "   ").is_err(),
            "空路径必须拒绝"
        );

        let file_path = dir.join("plain.txt");
        fs_err::write(&file_path, "x").unwrap();
        let error = list_workspace_dir_in_root(&dir, &file_path.to_string_lossy()).unwrap_err();
        assert!(error.contains("非目录"), "非目录必须如实拒绝：{error}");

        let missing = dir.join("no-such-subdir");
        assert!(
            list_workspace_dir_in_root(&dir, &missing.to_string_lossy()).is_err(),
            "不存在必须拒绝"
        );
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn listing_reports_real_shape() {
        let dir = scratch("list-shape");
        let sub = dir.join("sub");
        fs_err::create_dir_all(&sub).unwrap();
        fs_err::write(dir.join("b.txt"), "12345").unwrap();
        fs_err::write(dir.join(".hidden"), "xy").unwrap();
        fs_err::write(sub.join("inner.txt"), "z").unwrap();

        let listing = list_workspace_dir_in_root(&dir, &dir.to_string_lossy()).unwrap();
        let by_name: std::collections::HashMap<&str, &DirEntryInfo> = listing
            .entries
            .iter()
            .map(|item| (item.name.as_str(), item))
            .collect();

        // 目录优先 + 名称序：sub 在最前（唯一目录）。
        assert_eq!(listing.entries[0].name, "sub");
        assert!(by_name["sub"].is_dir);
        assert_eq!(by_name["sub"].size_bytes, 0, "目录大小不冒充");
        assert_eq!(by_name["b.txt"].size_bytes, 5);
        assert!(by_name.contains_key(".hidden"), "隐藏条目照实列");
        assert!(!by_name[".hidden"].is_dir);
        assert!(!by_name.contains_key("inner.txt"), "不递归下钻");
        assert_eq!(listing.path, dir.to_string_lossy());

        // 根内子目录照常列举（限定不误伤）。
        let sub_listing = list_workspace_dir_in_root(&dir, &sub.to_string_lossy()).unwrap();
        assert_eq!(sub_listing.entries.len(), 1);
        assert_eq!(sub_listing.entries[0].name, "inner.txt");

        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn write_ops_lifecycle_and_zero_fab() {
        let dir = scratch("tiles-write");
        let base = dir.to_string_lossy().into_owned();

        // 建 → 改名 → 删 全链（根内正常写盘不受限定影响）。
        let created = create_dir_entry_in_root(&dir, &base, "docs").unwrap();
        assert!(Path::new(&created).is_dir(), "目录真建成");
        let renamed = rename_entry_in_root(&dir, &base, "docs", "notes").unwrap();
        assert!(Path::new(&renamed).is_dir(), "改名真生效");
        assert!(delete_entry_in_root(&dir, &base, "notes")
            .unwrap()
            .ends_with("notes"));

        // 文件删除。
        fs_err::write(dir.join("a.txt"), b"x").unwrap();
        assert!(delete_entry_in_root(&dir, &base, "a.txt").is_ok());

        // 0 装边界（既有防线保留）。
        assert!(
            create_dir_entry_in_root(&dir, &base, "x/y").is_err(),
            "分隔符拒绝"
        );
        assert!(
            create_dir_entry_in_root(&dir, &base, " ").is_err(),
            "空名拒绝"
        );
        assert!(
            create_dir_entry_in_root(&dir, &base, "a\tb").is_err(),
            "控制字符拒绝"
        );
        assert!(
            create_dir_entry_in_root(&dir, "", "n").is_err(),
            "空基目录拒绝"
        );
        assert!(
            rename_entry_in_root(&dir, &base, "ghost", "x").is_err(),
            "源必须在"
        );
        create_dir_entry_in_root(&dir, &base, "keep").unwrap();
        create_dir_entry_in_root(&dir, &base, "again").unwrap();
        assert!(
            create_dir_entry_in_root(&dir, &base, "keep").is_err(),
            "已存在拒绝"
        );
        assert!(
            rename_entry_in_root(&dir, &base, "again", "keep").is_err(),
            "目标已存在不覆盖"
        );

        // 非空目录删除拒绝（防误删）。
        fs_err::write(dir.join("keep/inner.txt"), b"x").unwrap();
        assert!(
            delete_entry_in_root(&dir, &base, "keep").is_err(),
            "非空目录拒绝"
        );

        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn entry_name_traversal_components_rejected() {
        let dir = scratch("tiles-traversal");
        let base = dir.to_string_lossy().into_owned();
        for bad in [".", ".."] {
            assert!(
                create_dir_entry_in_root(&dir, &base, bad).is_err(),
                "{bad} 作条目名建目录必须拒绝"
            );
            assert!(
                rename_entry_in_root(&dir, &base, bad, "x").is_err(),
                "{bad} 作改名源必须拒绝"
            );
            assert!(
                rename_entry_in_root(&dir, &base, "x", bad).is_err(),
                "{bad} 作改名目标必须拒绝"
            );
            assert!(
                delete_entry_in_root(&dir, &base, bad).is_err(),
                "{bad} 作删除目标必须拒绝"
            );
        }
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn outside_root_rejected_for_every_op() {
        let tmp = scratch("contain");
        let root = tmp.join("ws");
        // 与根同前缀的兄弟目录：字符串前缀比对会误放行，必须按路径分量比。
        let sibling = tmp.join("ws2");
        let outside = tmp.join("other");
        for dir in [&root, &sibling, &outside] {
            fs_err::create_dir_all(dir).unwrap();
        }
        fs_err::write(sibling.join("s.txt"), b"x").unwrap();
        fs_err::write(outside.join("o.txt"), b"x").unwrap();

        let sibling_s = sibling.to_string_lossy().into_owned();
        let outside_s = outside.to_string_lossy().into_owned();
        for base in [&sibling_s, &outside_s] {
            assert!(
                list_workspace_dir_in_root(&root, base)
                    .unwrap_err()
                    .contains("越出"),
                "根外列举必须拒：{base}"
            );
            assert!(
                create_dir_entry_in_root(&root, base, "n")
                    .unwrap_err()
                    .contains("越出"),
                "根外建目录必须拒：{base}"
            );
            assert!(
                rename_entry_in_root(&root, base, "a", "b")
                    .unwrap_err()
                    .contains("越出"),
                "根外改名必须拒：{base}"
            );
            assert!(
                delete_entry_in_root(&root, base, "a")
                    .unwrap_err()
                    .contains("越出"),
                "根外删除必须拒：{base}"
            );
        }

        // 词法逃逸：`..` 上跳后落根外，规范化后仍须拒。
        let lex_escape = root.join("..").join("ws2");
        assert!(
            create_dir_entry_in_root(&root, &lex_escape.to_string_lossy(), "n")
                .unwrap_err()
                .contains("越出"),
            "词法 .. 逃逸必须拒"
        );
        let up = root.join("..");
        assert!(
            list_workspace_dir_in_root(&root, &up.to_string_lossy())
                .unwrap_err()
                .contains("越出"),
            "根的父目录必须拒"
        );

        // 根内一切照常（限定不误伤）。
        let sub = root.join("sub");
        fs_err::create_dir_all(&sub).unwrap();
        assert!(list_workspace_dir_in_root(&root, &sub.to_string_lossy()).is_ok());
        create_dir_entry_in_root(&root, &sub.to_string_lossy(), "leaf").unwrap();
        rename_entry_in_root(&root, &sub.to_string_lossy(), "leaf", "leaf2").unwrap();
        delete_entry_in_root(&root, &sub.to_string_lossy(), "leaf2").unwrap();

        // 根外内容零触碰。
        assert!(sibling.join("s.txt").is_file(), "根外兄弟目录必须零触碰");
        assert!(outside.join("o.txt").is_file(), "根外目录必须零触碰");
        let _ = fs_err::remove_dir_all(&tmp);
    }

    #[test]
    fn link_escape_base_rejected_in_root_link_allowed() {
        let tmp = scratch("link-escape");
        let root = tmp.join("ws");
        let outside = tmp.join("outside");
        let real = root.join("real");
        for dir in [&root, &outside, &real] {
            fs_err::create_dir_all(dir).unwrap();
        }
        fs_err::write(outside.join("secret.txt"), b"x").unwrap();

        let escape = root.join("escape");
        if !make_dir_link(&escape, &outside) {
            // 0 装：不假绿——本环境无链接创建权限时显式跳过并自报；
            // 同一拒止路径（canonicalize 后比祖先）由规范化逃逸用例无条件钉住。
            eprintln!(
                "[skip] 本环境无法创建目录链接（符号链接/联接），链接逃逸断言跳过；拒止路径由 outside_root_rejected_for_every_op 钉住"
            );
            let _ = fs_err::remove_dir_all(&tmp);
            return;
        }

        let escape_s = escape.to_string_lossy().into_owned();
        assert!(
            list_workspace_dir_in_root(&root, &escape_s)
                .unwrap_err()
                .contains("越出"),
            "经链接逃逸的列举必须拒"
        );
        assert!(
            create_dir_entry_in_root(&root, &escape_s, "n")
                .unwrap_err()
                .contains("越出"),
            "经链接逃逸的建目录必须拒"
        );
        assert!(
            rename_entry_in_root(&root, &escape_s, "a", "b")
                .unwrap_err()
                .contains("越出"),
            "经链接逃逸的改名必须拒"
        );
        assert!(
            delete_entry_in_root(&root, &escape_s, "a")
                .unwrap_err()
                .contains("越出"),
            "经链接逃逸的删除必须拒"
        );

        // 根内链接：重解析后仍在根内，不误伤。
        let inner = root.join("inner-link");
        assert!(make_dir_link(&inner, &real), "根内链接创建失败");
        assert!(
            list_workspace_dir_in_root(&root, &inner.to_string_lossy()).is_ok(),
            "重解析后仍在根内的链接必须放行"
        );

        // 链接条目自身按条目处理、根外内容零触碰：escape 指向非空根外目录，
        // 删除被既有「目录非空」防线拒绝（fail-closed），根外内容必须原样。
        assert!(
            delete_entry_in_root(&root, &root.to_string_lossy(), "escape").is_err(),
            "指向非空根外目录的链接条目删除必须拒（fail-closed）"
        );
        assert!(
            outside.join("secret.txt").is_file(),
            "链接目标（根外内容）必须零触碰"
        );

        let _ = fs_err::remove_dir_all(&tmp);
    }
}
