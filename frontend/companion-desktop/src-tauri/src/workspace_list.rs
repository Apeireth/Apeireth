//! 泊位磁贴面 —— 只读目录列举（批次推进）。
//!
//! 契约（I-starport-builtins.md「泊位」）：
//!   1. 只读列举 —— `list_workspace_dir` 仅读目录元数据，不写不删不建；
//!   2. 诚实报错 —— 空路径/不存在/非目录/权限拒绝/条目读取失败一律如实报
//!      （fs_err 错误自带路径，仓库纪律 clippy 禁 std::fs）；
//!   3. 照实列举 —— 隐藏文件照实列出（不滤不猜）；符号链接不穿越（按条目
//!      自身形态标注）；目录 size_bytes 恒 0（不冒充文件大小）；
//!   4. 展示序 —— 目录优先 → 名称大小写不敏感升序 → 同名大小写稳定序。
//!
//! 测试：`cargo test`（排序纯函数 + 真临时目录往返）。

use std::path::Path;

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

/// 只读列举一个目录（权限/形态错误如实报）。
#[tauri::command]
pub fn list_workspace_dir(path: String) -> Result<DirListing, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("路径为空".to_string());
    }
    let dir = Path::new(trimmed);
    let metadata =
        fs_err::metadata(dir).map_err(|error| format!("路径不可读（如实报）：{error}"))?;
    if !metadata.is_dir() {
        return Err(format!("非目录拒绝：{trimmed}"));
    }
    let read = fs_err::read_dir(dir).map_err(|error| format!("列举失败（如实报）：{error}"))?;
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
// 0 装：条目名含分隔符/控制字符拒绝；rename 源必须在、目标必须不在；
// delete 目录非空拒绝（防误删，递归删除为增强位）；错误带路径（fs_err 纪律）。
// ---------------------------------------------------------------------------

/// 条目名单层校验（非空、无分隔符、无控制字符）。
fn validate_entry_name(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("条目名不能为空".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.chars().any(char::is_control) {
        return Err("条目名含路径分隔符或控制字符，拒绝".to_string());
    }
    Ok(trimmed)
}

/// 建一层目录（已存在拒绝；返回建成路径）。
#[tauri::command]
pub fn create_dir_entry(path: String, name: String) -> Result<String, String> {
    let base = Path::new(path.trim());
    if path.trim().is_empty() || !base.is_dir() {
        return Err(format!("基目录不存在或非目录：{}", path.trim()));
    }
    let clean = validate_entry_name(&name)?;
    let target = base.join(clean);
    if target.exists() {
        return Err(format!("条目已存在：{}", target.display()));
    }
    fs_err::create_dir(&target).map_err(|error| format!("建目录失败（如实报）：{error}"))?;
    Ok(target.display().to_string())
}

/// 同目录改名（源必须在、目标必须不在）。
#[tauri::command]
pub fn rename_entry(path: String, from: String, to: String) -> Result<String, String> {
    let base = Path::new(path.trim());
    if path.trim().is_empty() || !base.is_dir() {
        return Err(format!("基目录不存在或非目录：{}", path.trim()));
    }
    let from_clean = validate_entry_name(&from)?;
    let to_clean = validate_entry_name(&to)?;
    let source = base.join(from_clean);
    let target = base.join(to_clean);
    if !source.exists() {
        return Err(format!("源条目不存在：{}", source.display()));
    }
    if target.exists() {
        return Err(format!("目标已存在（不覆盖）：{}", target.display()));
    }
    fs_err::rename(&source, &target).map_err(|error| format!("改名失败（如实报）：{error}"))?;
    Ok(target.display().to_string())
}

/// 删条目（文件直删；空目录可删；非空目录拒绝——防误删）。
#[tauri::command]
pub fn delete_entry(path: String, name: String) -> Result<String, String> {
    let base = Path::new(path.trim());
    if path.trim().is_empty() || !base.is_dir() {
        return Err(format!("基目录不存在或非目录：{}", path.trim()));
    }
    let clean = validate_entry_name(&name)?;
    let target = base.join(clean);
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
    fn listing_rejects_empty_and_non_dir() {
        assert!(
            list_workspace_dir("   ".to_string()).is_err(),
            "空路径必须拒绝"
        );

        let dir = std::env::temp_dir().join(format!("starport-list-nondir-{}", std::process::id()));
        fs_err::create_dir_all(&dir).unwrap();
        let file_path = dir.join("plain.txt");
        fs_err::write(&file_path, "x").unwrap();
        let error = list_workspace_dir(file_path.to_string_lossy().into_owned()).unwrap_err();
        assert!(error.contains("非目录"), "非目录必须如实拒绝：{error}");

        let missing = dir.join("no-such-subdir");
        assert!(
            list_workspace_dir(missing.to_string_lossy().into_owned()).is_err(),
            "不存在必须拒绝"
        );
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn listing_reports_real_shape() {
        let dir = std::env::temp_dir().join(format!("starport-list-shape-{}", std::process::id()));
        let sub = dir.join("sub");
        fs_err::create_dir_all(&sub).unwrap();
        fs_err::write(dir.join("b.txt"), "12345").unwrap();
        fs_err::write(dir.join(".hidden"), "xy").unwrap();
        fs_err::write(sub.join("inner.txt"), "z").unwrap();

        let listing = list_workspace_dir(dir.to_string_lossy().into_owned()).unwrap();
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

        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn write_ops_lifecycle_and_zero_fab() {
        let dir = std::env::temp_dir().join(format!("starport-tiles-write-{}", std::process::id()));
        fs_err::create_dir_all(&dir).unwrap();
        let base = dir.to_string_lossy().into_owned();

        // 建 → 改名 → 删 全链。
        let created = create_dir_entry(base.clone(), "docs".to_string()).unwrap();
        assert!(Path::new(&created).is_dir(), "目录真建成");
        let renamed = rename_entry(base.clone(), "docs".to_string(), "notes".to_string()).unwrap();
        assert!(Path::new(&renamed).is_dir(), "改名真生效");
        assert!(delete_entry(base.clone(), "notes".to_string())
            .unwrap()
            .ends_with("notes"));

        // 文件删除。
        fs_err::write(dir.join("a.txt"), b"x").unwrap();
        assert!(delete_entry(base.clone(), "a.txt".to_string()).is_ok());

        // 0 装边界。
        assert!(
            create_dir_entry(base.clone(), "x/y".to_string()).is_err(),
            "分隔符拒绝"
        );
        assert!(
            create_dir_entry(base.clone(), " ".to_string()).is_err(),
            "空名拒绝"
        );
        assert!(
            create_dir_entry(base.clone(), "a\tb".to_string()).is_err(),
            "控制字符拒绝"
        );
        assert!(
            create_dir_entry("".to_string(), "n".to_string()).is_err(),
            "空基目录拒绝"
        );
        assert!(
            rename_entry(base.clone(), "ghost".to_string(), "x".to_string()).is_err(),
            "源必须在"
        );
        create_dir_entry(base.clone(), "keep".to_string()).unwrap();
        create_dir_entry(base.clone(), "again".to_string()).unwrap();
        assert!(
            create_dir_entry(base.clone(), "keep".to_string()).is_err(),
            "已存在拒绝"
        );
        assert!(
            rename_entry(base.clone(), "again".to_string(), "keep".to_string()).is_err(),
            "目标已存在不覆盖"
        );

        // 非空目录删除拒绝（防误删）。
        fs_err::write(dir.join("keep/inner.txt"), b"x").unwrap();
        assert!(
            delete_entry(base.clone(), "keep".to_string()).is_err(),
            "非空目录拒绝"
        );

        let _ = fs_err::remove_dir_all(&dir);
    }
}
