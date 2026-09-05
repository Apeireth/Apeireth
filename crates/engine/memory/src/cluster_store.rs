//! `apeireth-memory::cluster_store` — 思维簇管理与元自学习读取口 (N4 / 认知长程聚类).
//!
//! AI 的思考链不是对话的消耗性副产品，而是 AI 自主维护的“思考文件”：
//! 按主题聚簇落盘 (`{YYYY-MM-DD}-{seq:03}.md`)，在反思 (Reflection) 与做梦 (Dreaming) 周期中
//! 经由统一的只读接口 [`ClusterReader`] 回读，实现“思考的再思考”(元自学习).
//!
//! ## 核心机制
//! - 簇目录: 根目录下以「簇」结尾的目录；
//! - 条目文件名: `{YYYY-MM-DD}-{seq:03}.md` (由时钟与目录扫描决定，严格确定性与崩溃重入幂等)；
//! - 链式注册表: `meta_thinking_chains.json` 维护 `{"chains": {链名: [簇, ...]}}`；
//! - 安全防御: 路径穿越严格拦截 (`..`, `/`, `\\`)，编辑操作目标文本需 $\ge 15$ 字符防误伤；
//! - 纯 Safe Rust 零未定义行为，0 外部不可信 C-FFI 依赖。

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::clock::Clock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 簇目录后缀 (以「簇」结尾的目录识别为合法思维簇).
pub const CLUSTER_SUFFIX: &str = "簇";

/// 链注册表文件名 (meta_thinking_chains.json).
pub const META_CHAINS_FILE: &str = "meta_thinking_chains.json";

/// edit_file 目标文本最短字符数 (防误伤防御阈值).
pub const MIN_EDIT_TARGET_CHARS: usize = 15;

/// 思维簇错误定义.
#[derive(Debug, Error)]
pub enum ClusterStoreError {
    #[error("非法簇名: {0} (须非空、不含路径分隔符、不含 '..' 且以「{CLUSTER_SUFFIX}」结尾)")]
    InvalidName(String),
    #[error("内容为空")]
    EmptyContent,
    #[error("编辑目标文本过短: {0} 字符 (< {MIN_EDIT_TARGET_CHARS})")]
    TargetTooShort(usize),
    #[error("未找到包含目标文本的文件")]
    NotFound,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 簇内思考文件载荷.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterFile {
    pub name: String,
    pub content: String,
}

/// 链注册表数据模型.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MetaChains {
    #[serde(default)]
    chains: BTreeMap<String, Vec<String>>,
}

/// 思维簇统一读取接口 (元自学习消费侧).
///
/// 采用无异常降级语义 (文件或簇不存在时返回空列表).
pub trait ClusterReader: Send + Sync {
    /// 列出全部簇名 (字典序排序).
    fn clusters(&self) -> Vec<String>;
    /// 读取指定簇的全部思考文件 (按文件名字典序 = 时间序).
    fn read_cluster(&self, name: &str) -> Vec<ClusterFile>;
    /// 读取一条链 (链 = 一组簇): 返回 `簇/文件名` 形式的思考文件.
    fn read_chain(&self, name: &str) -> Vec<ClusterFile>;
}

/// 思维簇文件管理器.
pub struct ClusterStore {
    root: PathBuf,
    clock: Arc<dyn Clock>,
}

impl ClusterStore {
    /// root = 思维簇根目录；clock = 可注入时钟抽象.
    pub fn new(root: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self {
            root: root.into(),
            clock,
        }
    }

    /// 簇名规范化与安全防穿越校验.
    fn normalize_name(name: &str) -> Result<String, ClusterStoreError> {
        let cleaned: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        let bad = cleaned.is_empty()
            || !cleaned.ends_with(CLUSTER_SUFFIX)
            || cleaned.contains('/')
            || cleaned.contains('\\')
            || cleaned.contains("..");
        if bad {
            return Err(ClusterStoreError::InvalidName(name.to_string()));
        }
        Ok(cleaned)
    }

    fn cluster_dir(&self, name: &str) -> Result<PathBuf, ClusterStoreError> {
        Ok(self.root.join(Self::normalize_name(name)?))
    }

    /// 创建一个思考文件落盘；文件名 = `{日期}-{当日序号:03}.md`.
    pub fn create_file(&self, cluster: &str, content: &str) -> Result<PathBuf, ClusterStoreError> {
        if content.trim().is_empty() {
            return Err(ClusterStoreError::EmptyContent);
        }
        let dir = self.cluster_dir(cluster)?;
        std::fs::create_dir_all(&dir)?;
        let date = self.clock.now().format("%Y-%m-%d").to_string();
        let seq = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{date}-"))
            })
            .count()
            + 1;
        let path = dir.join(format!("{date}-{seq:03}.md"));
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// 列出全部合法簇目录名 (字典序升序).
    pub fn list_clusters(&self) -> Result<Vec<String>, ClusterStoreError> {
        let rd = match std::fs::read_dir(&self.root) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(CLUSTER_SUFFIX))
            .collect();
        names.sort();
        Ok(names)
    }

    /// 读取指定簇的全部思考文件 (.md / .txt).
    pub fn read_cluster(&self, name: &str) -> Result<Vec<ClusterFile>, ClusterStoreError> {
        let dir = self.cluster_dir(name)?;
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |t| t.is_file()))
            .map(|e| e.path())
            .filter(|p| {
                matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("md") | Some("txt")
                )
            })
            .collect();
        files.sort();
        let mut out = Vec::new();
        for p in files {
            let name = p
                .file_name()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            let content = std::fs::read_to_string(&p)?;
            out.push(ClusterFile { name, content });
        }
        Ok(out)
    }

    fn load_chains(&self) -> Result<MetaChains, ClusterStoreError> {
        let path = self.root.join(META_CHAINS_FILE);
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(MetaChains::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// 注册一条思维链 (链名 $\to$ 簇名列表) 至 `meta_thinking_chains.json`.
    pub fn register_chain(
        &self,
        chain: &str,
        clusters: &[String],
    ) -> Result<(), ClusterStoreError> {
        if chain.trim().is_empty() {
            return Err(ClusterStoreError::InvalidName(chain.to_string()));
        }
        for c in clusters {
            Self::normalize_name(c)?;
        }
        std::fs::create_dir_all(&self.root)?;
        let mut meta = self.load_chains()?;
        let mut sorted = clusters.to_vec();
        sorted.sort();
        sorted.dedup();
        meta.chains.insert(chain.trim().to_string(), sorted);
        std::fs::write(
            self.root.join(META_CHAINS_FILE),
            serde_json::to_string_pretty(&meta)?,
        )?;
        Ok(())
    }

    /// 读取一条思维链下全部簇的所有思考文件.
    pub fn read_chain(&self, chain: &str) -> Result<Vec<ClusterFile>, ClusterStoreError> {
        let meta = self.load_chains()?;
        let Some(clusters) = meta.chains.get(chain.trim()) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for c in clusters {
            for f in self.read_cluster(c)? {
                out.push(ClusterFile {
                    name: format!("{c}/{}", f.name),
                    content: f.content,
                });
            }
        }
        Ok(out)
    }

    /// 安全编辑思考文件 (替换首处匹配的目标文本，目标文本长度必须 $\ge 15$ 字符).
    pub fn edit_file(
        &self,
        cluster: Option<&str>,
        target: &str,
        replacement: &str,
    ) -> Result<PathBuf, ClusterStoreError> {
        let n = target.chars().count();
        if n < MIN_EDIT_TARGET_CHARS {
            return Err(ClusterStoreError::TargetTooShort(n));
        }
        let dirs = match cluster {
            Some(c) => vec![self.cluster_dir(c)?],
            None => self
                .list_clusters()?
                .iter()
                .map(|c| self.root.join(c))
                .collect(),
        };
        for dir in dirs {
            let dir_name = dir
                .file_name()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            for f in self.read_cluster(&dir_name)? {
                if f.content.contains(target) {
                    let new = f.content.replacen(target, replacement, 1);
                    let path = dir.join(&f.name);
                    std::fs::write(&path, new)?;
                    return Ok(path);
                }
            }
        }
        Err(ClusterStoreError::NotFound)
    }

    /// 全局检索 (查找所有内容中包含指定 query 的文件).
    pub fn search(&self, query: &str) -> Result<Vec<(String, String, usize)>, ClusterStoreError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for c in self.list_clusters()? {
            for f in self.read_cluster(&c)? {
                let hits = f.content.matches(query).count();
                if hits > 0 {
                    out.push((c.clone(), f.name, hits));
                }
            }
        }
        Ok(out)
    }
}

impl ClusterReader for ClusterStore {
    fn clusters(&self) -> Vec<String> {
        self.list_clusters().unwrap_or_default()
    }

    fn read_cluster(&self, name: &str) -> Vec<ClusterFile> {
        ClusterStore::read_cluster(self, name).unwrap_or_default()
    }

    fn read_chain(&self, name: &str) -> Vec<ClusterFile> {
        ClusterStore::read_chain(self, name).unwrap_or_default()
    }
}

// ============================================================
// 内存版 Reader (测试与瞬态运行时使用)
// ============================================================

/// 纯内存实现的思维簇读取器 (供无磁盘/瞬态场景或轻量测试直接使用).
#[derive(Debug, Clone, Default)]
pub struct InMemoryClusterReader {
    clusters: BTreeMap<String, Vec<ClusterFile>>,
    chains: BTreeMap<String, Vec<String>>,
}

impl InMemoryClusterReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_file(&mut self, cluster: impl Into<String>, file: ClusterFile) {
        self.clusters.entry(cluster.into()).or_default().push(file);
    }

    pub fn register_chain(&mut self, chain: impl Into<String>, clusters: Vec<String>) {
        self.chains.insert(chain.into(), clusters);
    }
}

impl ClusterReader for InMemoryClusterReader {
    fn clusters(&self) -> Vec<String> {
        self.clusters.keys().cloned().collect()
    }

    fn read_cluster(&self, name: &str) -> Vec<ClusterFile> {
        self.clusters.get(name).cloned().unwrap_or_default()
    }

    fn read_chain(&self, name: &str) -> Vec<ClusterFile> {
        let Some(clusters) = self.chains.get(name) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for c in clusters {
            if let Some(files) = self.clusters.get(c) {
                for f in files {
                    out.push(ClusterFile {
                        name: format!("{c}/{}", f.name),
                        content: f.content.clone(),
                    });
                }
            }
        }
        out
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;
    use std::path::Path;

    fn vclock() -> VirtualClock {
        VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
                .single()
                .unwrap(),
        )
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("apeireth-tcm-test-{}", uuid::Uuid::new_v4()))
    }

    fn mgr(root: &Path) -> ClusterStore {
        ClusterStore::new(root.to_path_buf(), Arc::new(vclock()))
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn create_file_names_are_date_seq_deterministic() {
        let root = temp_root();
        let m = mgr(&root);
        let p1 = m
            .create_file("前思维簇", "【思考模块：模块A】\n触发条件: x")
            .unwrap();
        let p2 = m.create_file("前思维簇", "第二条思考").unwrap();
        assert_eq!(p1.file_name().unwrap(), "2026-08-16-001.md");
        assert_eq!(p2.file_name().unwrap(), "2026-08-16-002.md");
        assert!(p2.starts_with(root.join("前思维簇")));

        let p3 = m.create_file("反思簇", "反思一条").unwrap();
        assert_eq!(p3.file_name().unwrap(), "2026-08-16-001.md");
        cleanup(&root);
    }

    #[test]
    fn invalid_cluster_names_rejected() {
        let root = temp_root();
        let m = mgr(&root);
        assert!(m.create_file("", "x").is_err());
        assert!(m.create_file("未加后缀", "x").is_err());
        assert!(m.create_file("../逃逸簇", "x").is_err());
        assert!(m.create_file("a/b簇", "x").is_err());
        assert!(m.create_file("前思维簇", "   ").is_err());
        cleanup(&root);
    }

    #[test]
    fn list_clusters_and_read_cluster_deterministic() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("乙簇", "乙内容").unwrap();
        m.create_file("甲簇", "甲内容").unwrap();

        let clusters = m.list_clusters().unwrap();
        assert_eq!(clusters, vec!["乙簇", "甲簇"]);

        let files = m.read_cluster("甲簇").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].content, "甲内容");
        cleanup(&root);
    }

    #[test]
    fn chain_registration_and_reading_works() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("甲簇", "甲1").unwrap();
        m.create_file("乙簇", "乙1").unwrap();

        m.register_chain("主线思考", &["乙簇".into(), "甲簇".into()])
            .unwrap();
        let files = m.read_chain("主线思考").unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "乙簇/2026-08-16-001.md");
        assert_eq!(files[1].name, "甲簇/2026-08-16-001.md");
        cleanup(&root);
    }

    #[test]
    fn edit_file_replaces_target_with_safety_boundary() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file(
            "测试簇",
            "这是第一行说明文本。这是一段足够长的目标文本需要被替换。这是最后一行。",
        )
        .unwrap();

        // 过短拒绝 (< 15 字符)
        assert!(m.edit_file(Some("测试簇"), "短文本", "新文本").is_err());

        let target = "这是一段足够长的目标文本需要被替换。";
        let path = m
            .edit_file(Some("测试簇"), target, "【已替换的新内容】")
            .unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("【已替换的新内容】"));
        assert!(!content.contains(target));
        cleanup(&root);
    }

    #[test]
    fn search_finds_matching_occurrences() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("甲簇", "关键字出现 关键字出现").unwrap();
        m.create_file("乙簇", "这里也有关键字").unwrap();

        let hits = m.search("关键字").unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].2, 1); // 乙簇 1 次
        assert_eq!(hits[1].2, 2); // 甲簇 2 次
        cleanup(&root);
    }

    #[test]
    fn in_memory_reader_works_identically() {
        let mut reader = InMemoryClusterReader::new();
        reader.insert_file(
            "内存簇",
            ClusterFile {
                name: "test.md".into(),
                content: "内存内容".into(),
            },
        );
        reader.register_chain("测试链", vec!["内存簇".into()]);

        assert_eq!(reader.clusters(), vec!["内存簇"]);
        let files = reader.read_cluster("内存簇");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].content, "内存内容");

        let chain_files = reader.read_chain("测试链");
        assert_eq!(chain_files.len(), 1);
        assert_eq!(chain_files[0].name, "内存簇/test.md");
    }
}
