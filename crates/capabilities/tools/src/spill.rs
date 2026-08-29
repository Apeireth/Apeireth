//! `apeireth-tools-canonical::spill` — 工具结果溢出存储 (吸收 DeepSeek Harness spill 设计, Rust 重写).
//!
//! 问题: 工具输出可能超大 (grep/读大文件/代码检索), 直接塞入 LLM 上下文会导致上下文膨胀与腐烂.
//! 方案: 超过阈值的结果溢出到**会话私有文件**, 返回给模型的是一条「路径 + 摘要提示」,
//! 模型需要时用文件读取工具按需切片读取.
//!
//! **安全设计 (0 装严格边界)**:
//! - 目录隔离: `<root>/<session-安全名>/<随机前缀>-<安全名>`, root 默认私有进程临时目录
//! - 独占创建 `create_new` (wx): 已存在路径 (含 symlink) 直接失败, 防重定向与文件覆盖植入
//! - 文件名净化: 去除路径分隔符与 `..` 穿越符
//! - 读取校验: 路径必须严格解析在 root 目录内部 (`canonicalize` 祖先检查)
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决长会话大工具输出撑爆 Token 预算问题
//! 2. 系统: 放置在 `apeireth-tools-canonical`, 对所有内置工具输出提供统一溢出保护
//! 3. 架构: 纯 std::fs 零外部依赖, `#![deny(unsafe_code)]`

use std::path::{Path, PathBuf};

/// 溢出阈值: 序列化结果超过该字符数 → spill (默认 2000 字符).
pub const SPILL_THRESHOLD_CHARS: usize = 2000;

/// 会话私有溢出存储.
#[derive(Debug, Clone)]
pub struct SpillStore {
    root: PathBuf,
}

/// 文件名净化: 只保留字母数字与安全符号, 移除路径分隔符与 `..`.
pub fn safe_segment(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('_');
    if cleaned.is_empty() {
        "spill".to_string()
    } else {
        cleaned.chars().take(60).collect()
    }
}

impl Default for SpillStore {
    fn default() -> Self {
        Self::new_private()
    }
}

impl SpillStore {
    /// 构造系统临时目录下的私有随机子目录 (进程生命周期隔离).
    pub fn new_private() -> Self {
        let root = std::env::temp_dir().join(format!(
            "apeireth-spill-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        Self { root }
    }

    /// 显式指定 root 根目录.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 获取根目录路径引用.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 溢出写入: 返回落盘绝对路径. 独占写 `create_new`, 防 symlink 与植入.
    pub fn spill(
        &self,
        session_id: &str,
        suggested_name: &str,
        content: &str,
    ) -> Result<String, String> {
        let session_dir = self.root.join(safe_segment(session_id));
        std::fs::create_dir_all(&session_dir).map_err(|e| format!("创建溢出目录失败: {e}"))?;
        let file = session_dir.join(format!(
            "{}-{}",
            &std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis().to_string())
                .unwrap_or_else(|_| "0".into()),
            safe_segment(suggested_name)
        ));
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        let mut f = opts
            .open(&file)
            .map_err(|e| format!("独占写溢出文件失败: {e}"))?;
        use std::io::Write;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("写溢出内容失败: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600));
        }
        Ok(file.to_string_lossy().to_string())
    }

    /// 读取前校验: 路径必须严格在 root 内 (防目录穿越越权读).
    pub fn read_within_root(&self, path: &str) -> Result<String, String> {
        let p = Path::new(path);
        let root_c = std::fs::canonicalize(&self.root)
            .map_err(|e| format!("canonicalize root 失败: {e}"))?;
        let p_abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.root.join(p)
        };
        let p_c =
            std::fs::canonicalize(&p_abs).map_err(|e| format!("canonicalize target 失败: {e}"))?;
        if !p_c.starts_with(&root_c) {
            return Err("越权读取: 路径不在溢出根目录内".into());
        }
        std::fs::read_to_string(&p_c).map_err(|e| format!("读取溢出文件失败: {e}"))
    }

    /// 若内容超出阈值则溢出并返回提示文本, 否则原样返回.
    pub fn maybe_spill(
        &self,
        session_id: &str,
        tool_name: &str,
        content: &str,
        threshold: usize,
    ) -> String {
        if content.chars().count() > threshold {
            match self.spill(session_id, tool_name, content) {
                Ok(path) => {
                    let preview: String = content.chars().take(200).collect();
                    format!(
                        "[工具输出过大 (共 {} 字符), 已安全溢出落盘至: {}]\n预览内容:\n{}\n...",
                        content.chars().count(),
                        path,
                        preview
                    )
                }
                Err(_) => content.to_string(),
            }
        } else {
            content.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_segment_cleans_and_sanitizes() {
        assert_eq!(safe_segment("valid_name-123"), "valid_name-123");
        assert_eq!(safe_segment("../../../etc/passwd"), "etc_passwd");
        assert_eq!(safe_segment(""), "spill");
    }

    #[test]
    fn spill_and_read_within_root_works() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let content = "这是一段非常长的工具输出内容，需要溢出落盘保存。";
        let path = store.spill("session_1", "grep_tool", content).unwrap();

        let read_back = store.read_within_root(&path).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn read_outside_root_is_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let outside = std::env::temp_dir().join("some_other_file.txt");
        let _ = std::fs::write(&outside, "sensitive data");

        let err = store.read_within_root(&outside.to_string_lossy());
        assert!(err.is_err());
    }

    #[test]
    fn maybe_spill_triggers_above_threshold() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let big_content = "x".repeat(100);
        let small_content = "hello";

        let result_small = store.maybe_spill("s1", "cat", small_content, 50);
        assert_eq!(result_small, "hello");

        let result_big = store.maybe_spill("s1", "cat", &big_content, 50);
        assert!(result_big.contains("工具输出过大"));
        assert!(result_big.contains("已安全溢出落盘至"));
    }
}
