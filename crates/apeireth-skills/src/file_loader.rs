//! R63: File-based Skill Descriptor Loader — 借鉴 VCP `vcptoolbox/modules` 真扫目录
//!
//! **目标**: 跟 in-memory DescriptorRegistry 互补 — 从文件系统目录扫 .json 描述符,
//! 自动 load 到 DescriptorRegistry。借鉴 VCP `vcptoolbox/modules/<name>/index.js` 5 字段
//! metadata + LangChain `hub.load("owner/repo")` 仓库式 descriptor loading 模式.
//!
//! **借鉴锚 (S-1)**:
//! - VCP `vcptoolbox/modules/<name>/index.js` (R33-1 conventions_scanner 同源思路): 真实目录扫描
//! - LangChain Hub: `load("langchain/...")` 仓库式 descriptor loading
//! - Anthropic Skills CLI `~/.claude/skills/<name>/SKILL.md`: 文件路径即 ID
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 Skill (R23 LOCKED 4 字段)
//! - 0 改 SkillDescriptor / DescriptorRegistry (R36 batch 后续 7 字段)
//! - 0 改 workspace 1.0.0 / 24 LOCKED crate
//! - 0 引入异步 I/O (sync std::fs + walkdir, 跟 descriptor in-memory 一致)
//!
//! **JSON 描述符 schema** (单文件 1 个 SkillDescriptor):
//! ```json
//! {
//!   "id": "summarize-text",
//!   "version": "1.0.0",
//!   "description": "...",
//!   "tags": ["summarize", "text"],
//!   "source": "vcptoolbox",
//!   "input_example": "{\"text\":\"...\"}",
//!   "output_example": "{\"summary\":\"...\"}"
//! }
//! ```
//!
//! **目录约定** (per Anthropic Skills CLI + VCP 双借鉴):
//! - `<base_dir>/<id>/descriptor.json` — 每个 skill 1 个子目录 + 1 个 descriptor.json
//! - 也支持扁平: `<base_dir>/<id>.json` — 直接 1 个 .json 文件
//! - 两种 layout 自动 fallback (扁平优先)

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::descriptor::{DescriptorRegistry, SkillDescriptor};
use crate::{Skill, SkillResult};

/// 默认 descriptor 文件名 (per Anthropic Skills CLI `SKILL.md` 借鉴, JSON 化)
pub const DESCRIPTOR_FILE: &str = "descriptor.json";

/// 单个 .json layout 的最大 size 限制 (防 OOM, 256 KiB 远大于实际 descriptor < 4 KiB)
pub const MAX_DESCRIPTOR_BYTES: u64 = 256 * 1024;

/// 目录扫到的每个 descriptor 路径 entry (给 reporting / cache invalidation 用)
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDescriptor {
    /// 文件绝对路径
    pub path: PathBuf,
    /// layout 类型 (NestedDir / FlatFile)
    pub layout: DescriptorLayout,
    /// loaded descriptor (parse OK 后填 None)
    pub descriptor: Option<SkillDescriptor>,
    /// parse error (any stage fail, 不 panic, 记录给 reporting)
    pub error: Option<String>,
}

/// 目录 layout 类型 (借鉴 VCP 双 layout 兼容)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorLayout {
    /// `<base>/<id>/descriptor.json` (Anthropic Skills 风格)
    NestedDir,
    /// `<base>/<id>.json` (扁平)
    FlatFile,
}

/// 递归扫 base_dir, 找出所有 descriptor 路径 (扁平 + 嵌套两种 layout)
pub fn discover_descriptor_paths(base_dir: impl AsRef<Path>) -> SkillResult<Vec<PathBuf>> {
    let base = base_dir.as_ref();
    if !base.exists() {
        return Err(crate::SkillError::UnknownSkill(
            format!("base_dir 不存在: {}", base.display()),
        ));
    }
    if !base.is_dir() {
        return Err(crate::SkillError::UnknownSkill(
            format!("base_dir 不是目录: {}", base.display()),
        ));
    }
    let mut paths = Vec::new();
    for entry in WalkDir::new(base)
        .max_depth(4) // 限制深度防环
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else { continue };
        // Nested layout: <base>/<id>/descriptor.json
        if name == DESCRIPTOR_FILE {
            paths.push(p.to_path_buf());
            continue;
        }
        // Flat layout: <base>/<id>.json (且父目录就是 base, 不是 nested 子目录)
        if name.ends_with(".json") && p.parent() == Some(base) {
            paths.push(p.to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
}

/// 推断 layout 类型
pub fn detect_layout(path: impl AsRef<Path>, base_dir: impl AsRef<Path>) -> DescriptorLayout {
    let p = path.as_ref();
    let base = base_dir.as_ref();
    if p.file_name().and_then(|s| s.to_str()) == Some(DESCRIPTOR_FILE) {
        // 检查父目录的父目录是不是 base (即 nested 一层)
        if let Some(parent) = p.parent() {
            if parent.parent() == Some(base) {
                return DescriptorLayout::NestedDir;
            }
        }
    }
    DescriptorLayout::FlatFile
}

/// 从单个 .json 文件 load 一个 SkillDescriptor (含 Skill 派生)
pub fn load_one(path: impl AsRef<Path>) -> SkillResult<(Skill, SkillDescriptor)> {
    let p = path.as_ref();
    let meta = fs::metadata(p).map_err(|e| {
        crate::SkillError::UnknownSkill(format!("stat {}: {}", p.display(), e))
    })?;
    if meta.len() > MAX_DESCRIPTOR_BYTES {
        return Err(crate::SkillError::UnknownSkill(format!(
            "descriptor 太大: {} ({} > {})",
            p.display(),
            meta.len(),
            MAX_DESCRIPTOR_BYTES
        )));
    }
    let text = fs::read_to_string(p).map_err(|e| {
        crate::SkillError::UnknownSkill(format!("read {}: {}", p.display(), e))
    })?;
    let descriptor: SkillDescriptor = serde_json::from_str(&text).map_err(|e| {
        crate::SkillError::UnknownSkill(format!("parse {}: {}", p.display(), e))
    })?;
    descriptor.validate_for_loader()?;
    let skill = Skill::new(
        descriptor.id.clone(),
        descriptor.version.clone(),
        descriptor.input_example.clone(),
        descriptor.output_example.clone(),
    );
    skill.validate()?;
    Ok((skill, descriptor))
}

/// 给 SkillDescriptor 加一个 loader-specific validator (复用 registry 的 validate 路径)
impl SkillDescriptor {
    fn validate_for_loader(&self) -> SkillResult<()> {
        if self.id.is_empty() {
            return Err(crate::SkillError::EmptyId("(no id)".into()));
        }
        if !crate::is_valid_id(&self.id) {
            return Err(crate::SkillError::InvalidIdFormat(self.id.clone()));
        }
        if self.version.is_empty() {
            return Err(crate::SkillError::InvalidVersion("(no version)".into()));
        }
        // semver 3-segment
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
            return Err(crate::SkillError::InvalidVersion(self.version.clone()));
        }
        Ok(())
    }
}

/// 扫 base_dir 并 load 所有 descriptor 到一个新 DescriptorRegistry
///
/// 返回 (registry, loaded_entries). loaded_entries 包含 parse error 记录, 给 reporting 用.
pub fn load_registry_from_dir(
    base_dir: impl AsRef<Path>,
) -> SkillResult<(DescriptorRegistry, Vec<LoadedDescriptor>)> {
    let base = base_dir.as_ref().to_path_buf();
    let paths = discover_descriptor_paths(&base)?;
    let mut registry = DescriptorRegistry::new();
    let mut entries = Vec::with_capacity(paths.len());

    for path in paths {
        let layout = detect_layout(&path, &base);
        match load_one(&path) {
            Ok((skill, descriptor)) => {
                // register_pair 内部 sort, duplicate id 会 reject
                let reg_result = registry.register_pair(skill, descriptor.clone());
                entries.push(LoadedDescriptor {
                    path: path.clone(),
                    layout,
                    descriptor: if reg_result.is_ok() { Some(descriptor) } else { None },
                    error: reg_result.err().map(|e| e.to_string()),
                });
            }
            Err(e) => {
                entries.push(LoadedDescriptor {
                    path: path.clone(),
                    layout,
                    descriptor: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    Ok((registry, entries))
}

/// Markdown 报告生成 (给 reporting / TUI 显示用)
pub fn report_to_markdown(entries: &[LoadedDescriptor]) -> String {
    let mut out = String::new();
    out.push_str("# Skills File Loader Report\n\n");
    let total = entries.len();
    let loaded = entries.iter().filter(|e| e.descriptor.is_some()).count();
    let failed = total - loaded;
    out.push_str(&format!("- Total: {}\n- Loaded: {}\n- Failed: {}\n\n", total, loaded, failed));
    out.push_str("| Path | Layout | Status | Error |\n");
    out.push_str("|------|--------|--------|-------|\n");
    for e in entries {
        let status = if e.descriptor.is_some() { "OK" } else { "FAIL" };
        let id = e.descriptor.as_ref().map(|d| d.id.as_str()).unwrap_or("-");
        let err = e.error.as_deref().unwrap_or("-");
        out.push_str(&format!(
            "| `{}` | {:?} | {} (id={}) | {} |\n",
            e.path.display(),
            e.layout,
            status,
            id,
            err,
        ));
    }
    out
}

// ============================================================
// 单元测试 (用 tempfile 保证 isolated 目录)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_desc(dir: &Path, id: &str, version: &str, tags: &[&str]) {
        let nested = dir.join(id);
        fs::create_dir_all(&nested).unwrap();
        let tags_json = tags
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            r#"{{"id":"{id}","version":"{version}","description":"{id} skill","tags":[{tags_json}],"source":"test","input_example":"{{}}","output_example":"{{}}"}}"#
        );
        let mut f = fs::File::create(nested.join(DESCRIPTOR_FILE)).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    fn write_flat(dir: &Path, id: &str, version: &str) {
        let tags_json = "\"alpha\",\"beta\"";
        let json = format!(
            r#"{{"id":"{id}","version":"{version}","description":"{id} skill","tags":[{tags_json}],"source":"test","input_example":"{{}}","output_example":"{{}}"}}"#
        );
        let mut f = fs::File::create(dir.join(format!("{id}.json"))).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    #[test]
    fn discover_nested_layout() {
        let dir = tempdir();
        write_desc(&dir, "summarize-text", "1.0.0", &["alpha"]);
        write_desc(&dir, "summarize-html", "1.5.0", &["beta"]);
        let paths = discover_descriptor_paths(&dir).unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn discover_flat_layout() {
        let dir = tempdir();
        write_flat(&dir, "a-skill", "1.0.0");
        write_flat(&dir, "b-skill", "2.0.0");
        let paths = discover_descriptor_paths(&dir).unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn detect_layout_nested_vs_flat() {
        let dir = tempdir();
        write_desc(&dir, "x", "1.0.0", &[]);
        let nested_path = dir.join("x").join(DESCRIPTOR_FILE);
        assert_eq!(
            detect_layout(&nested_path, &dir),
            DescriptorLayout::NestedDir
        );
        write_flat(&dir, "y", "1.0.0");
        let flat_path = dir.join("y.json");
        assert_eq!(detect_layout(&flat_path, &dir), DescriptorLayout::FlatFile);
    }

    #[test]
    fn load_one_round_trip() {
        let dir = tempdir();
        write_desc(&dir, "load-one", "1.0.0", &["t1"]);
        let path = dir.join("load-one").join(DESCRIPTOR_FILE);
        let (skill, descriptor) = load_one(&path).unwrap();
        assert_eq!(skill.id, "load-one");
        assert_eq!(descriptor.version, "1.0.0");
        assert!(descriptor.has_tag("t1"));
    }

    #[test]
    fn load_one_rejects_invalid_id() {
        let dir = tempdir();
        let nested = dir.join("Bad_ID");
        fs::create_dir_all(&nested).unwrap();
        let json = r#"{"id":"Bad_ID","version":"1.0.0","description":"x","tags":[],"source":"t","input_example":"{}","output_example":"{}"}"#;
        fs::write(nested.join(DESCRIPTOR_FILE), json).unwrap();
        let path = nested.join(DESCRIPTOR_FILE);
        let result = load_one(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_one_rejects_invalid_version() {
        let dir = tempdir();
        let nested = dir.join("badver");
        fs::create_dir_all(&nested).unwrap();
        let json = r#"{"id":"badver","version":"1.0","description":"x","tags":[],"source":"t","input_example":"{}","output_example":"{}"}"#;
        fs::write(nested.join(DESCRIPTOR_FILE), json).unwrap();
        let path = nested.join(DESCRIPTOR_FILE);
        let result = load_one(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_registry_from_dir_mixed() {
        let dir = tempdir();
        write_desc(&dir, "nested-skill", "1.0.0", &["nested"]);
        write_flat(&dir, "flat-skill", "2.0.0");
        let (reg, entries) = load_registry_from_dir(&dir).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(reg.len(), 2);
        assert!(reg.descriptor("nested-skill").is_some());
        assert!(reg.descriptor("flat-skill").is_some());
    }

    #[test]
    fn load_registry_from_dir_partial_failure() {
        let dir = tempdir();
        write_desc(&dir, "good-skill", "1.0.0", &[]);
        // 写一个 invalid descriptor
        let nested = dir.join("bad-skill");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join(DESCRIPTOR_FILE), "not json").unwrap();
        let (reg, entries) = load_registry_from_dir(&dir).unwrap();
        assert_eq!(entries.len(), 2);
        // 1 个 OK + 1 个 FAIL
        assert_eq!(reg.len(), 1);
        let fail = entries.iter().filter(|e| e.descriptor.is_none()).count();
        assert_eq!(fail, 1);
    }

    #[test]
    fn discover_nonexistent_dir_returns_error() {
        let dir = std::env::temp_dir().join("__apeireth_does_not_exist__");
        let result = discover_descriptor_paths(&dir);
        assert!(result.is_err());
    }

    #[test]
    fn report_to_markdown_basic() {
        let dir = tempdir();
        write_desc(&dir, "a", "1.0.0", &[]);
        let (_, entries) = load_registry_from_dir(&dir).unwrap();
        let md = report_to_markdown(&entries);
        assert!(md.contains("Total: 1"));
        assert!(md.contains("Loaded: 1"));
        assert!(md.contains("Failed: 0"));
        assert!(md.contains("`"));
    }

    // 简易 tempfile 替代 (避免加 tempfile dep)
    fn tempdir() -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("apeireth-skills-test-{}-{}", pid, nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }
}


