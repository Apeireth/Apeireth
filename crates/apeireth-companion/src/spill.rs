//! `apeireth-companion::spill` — 工具结果溢出存储 (吸收 DeepSeek Harness spill 设计, Rust 重写).
//!
//! 问题: 工具输出可能超大 (grep/读文件/搜索), 直接塞回 LLM messages 撑爆上下文.
//! 方案: 超过阈值的结果溢出到**会话私有文件**, 返回给模型的是一条「路径 + 提示」,
//! 模型需要时用 FileOperator(read) 按需读取.
//!
//! 安全设计 (对齐 DSH spill-local):
//! - 目录: `<root>/<session-安全名>/<随机前缀>-<安全名>`, root 默认私有进程临时目录
//! - 独占写 `create_new` (wx): 已存在路径 (含 symlink) 直接失败, 防重定向/植入
//! - 文件名 sanitize: 去路径分隔符, 防 `..` 穿越
//! - 读取校验: 必须落在 root 内 (canonicalize 前缀检查)
//!
//! 0 假装: Windows 上 0700 权限位不生效, 依赖系统用户目录私有性 (与 DSH 同处理);
//! 分组用 session 名 (不引入 sha256 依赖).

use std::path::{Path, PathBuf};

/// 溢出阈值: 序列化结果超过该字符数 → spill (默认 2000).
pub const SPILL_THRESHOLD_CHARS: usize = 2000;

/// 会话私有溢出存储.
pub struct SpillStore {
    root: PathBuf,
}

/// 文件名 sanitize: 只保留安全段, 去路径分隔符与 `..` (不放行 `.`).
fn safe_segment(name: &str) -> String {
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

impl SpillStore {
    /// root 默认: 系统临时目录下私有随机子目录 (进程内, 其他用户不可预见).
    pub fn new_private() -> Self {
        let root = std::env::temp_dir().join(format!("apeireth-spill-{}", uuid::Uuid::new_v4()));
        Self { root }
    }

    /// 显式 root (生产可配; 建议给私有目录).
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 溢出写入: 返回落盘路径 (绝对). 独占写, 防 symlink 植入.
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
            &uuid::Uuid::new_v4().to_string()[..8],
            safe_segment(suggested_name)
        ));
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true); // wx: 已存在即失败
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

    /// 读取前校验: 路径必须解析在 root 内 (防越权读).
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
            std::fs::canonicalize(&p_abs).map_err(|e| format!("canonicalize 溢出路径失败: {e}"))?;
        if !p_c.starts_with(&root_c) {
            return Err(format!("溢出路径越界: {path}"));
        }
        std::fs::read_to_string(&p_c).map_err(|e| format!("读溢出文件失败: {e}"))
    }

    /// 判断某结果是否需要溢出.
    pub fn should_spill(serialized_len_chars: usize) -> bool {
        serialized_len_chars > SPILL_THRESHOLD_CHARS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("apeireth-spill-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn spill_writes_and_reads_back() {
        let store = SpillStore::with_root(tmp_root("rw"));
        let big = "x".repeat(3000);
        let path = store.spill("me", "search_results.txt", &big).unwrap();
        let read = store.read_within_root(&path).unwrap();
        assert_eq!(read.len(), 3000);
        // 分组目录存在
        assert!(Path::new(&path).parent().unwrap().exists());
    }

    #[test]
    fn spill_rejects_traversal_in_name() {
        let store = SpillStore::with_root(tmp_root("trav"));
        let path = store.spill("me", "../../evil.txt", "x").unwrap();
        let p = Path::new(&path);
        // sanitize 后不含 .. 分隔符, 且落在 root 内
        assert!(!path.contains(".."));
        assert!(p.starts_with(store.root()));
    }

    #[test]
    fn read_rejects_outside_root() {
        let store = SpillStore::with_root(tmp_root("outside"));
        let outside = std::env::temp_dir().join("apeireth-forbidden.txt");
        std::fs::write(&outside, "secret").unwrap();
        let r = store.read_within_root(&outside.to_string_lossy());
        assert!(r.is_err(), "越界读应被拒");
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn exclusive_write_fails_on_existing_path() {
        let store = SpillStore::with_root(tmp_root("excl"));
        let p1 = store.spill("me", "a.txt", "first").unwrap();
        // 同路径再写应失败 (wx) — 但随机前缀不同, 这里直接验证 create_new 语义:
        // 手工创建同路径文件
        std::fs::write(&p1, "taken").unwrap();
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        assert!(
            opts.open(&p1).is_err(),
            "已存在路径应拒绝 (防 symlink 植入)"
        );
    }

    #[test]
    fn threshold_judgement() {
        assert!(SpillStore::should_spill(2001));
        assert!(!SpillStore::should_spill(2000));
    }
}
