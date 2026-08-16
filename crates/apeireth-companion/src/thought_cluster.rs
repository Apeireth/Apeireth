//! `apeireth-companion::thought_cluster` — 思维簇管理 + 元自学习读取口 (backlog N4).
//!
//! VCP ThoughtClusterManager (research/source/vcptoolbox/Plugin/ThoughtClusterManager) 吸收:
//! AI 的思考链不是对话副产品, 而是 AI 自己维护的"思考文件"——按主题聚簇落盘,
//! 反思/做梦可回读它们做"思考的再思考"(元自学习).
//!
//! 真实机制 (0 假装):
//! - 簇 = root 下以「簇」结尾的目录; 条目 = `{YYYY-MM-DD}-{seq:03}.md` (日期由注入时钟决定, 可测)
//! - [`ThoughtClusterManager`]: create_file / list_clusters / read_cluster /
//!   register_chain / read_chain / edit_file / search — 全部确定性 (目录/文件名字典序)
//! - 链 = `meta_thinking_chains.json` 里 `{"chains": {链名: [簇, ...]}}` (与 VCP 格式一致)
//! - [`ThoughtClusterReader`]: 反思/做梦消费思维簇的统一 trait 口
//!   (reflection.rs `with_thought_reader` / dream.rs `with_thought_reader` 注入)
//!
//! 0 装 PASS 标注 (诚实):
//! - 写入侧 (AI 自己决定写/改哪个簇) 需要 LLM 在部署层经工具调用驱动 — 本 lib 只提供
//!   create_file/edit_file 机制口, **未注册工具** (ToolBridge 接线属工具任务包, 不在本边界)
//! - 聚簇归类 = 调用方显式指定簇名 (VCP 同款); 语义自动聚类不做 (可接 embedding, 未假装)

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::clock::Clock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 簇目录后缀 (VCP 约定: 以「簇」结尾的目录才是思维簇).
pub const CLUSTER_SUFFIX: &str = "簇";
/// 链注册表文件名 (与 VCP meta_thinking_chains.json 对齐).
pub const META_CHAINS_FILE: &str = "meta_thinking_chains.json";
/// edit_file 目标文本最短字符数 (VCP 规则: < 15 拒绝, 防误伤).
pub const MIN_EDIT_TARGET_CHARS: usize = 15;

/// 思维簇错误 (非法输入显式拒绝, 不 panic).
#[derive(Debug, Error)]
pub enum ThoughtClusterError {
    #[error("非法簇名: {0} (须非空、不含路径分隔符、以「{CLUSTER_SUFFIX}」结尾)")]
    InvalidName(String),
    #[error("内容为空")]
    EmptyContent,
    #[error("编辑目标文本过短: {0} 字符 (< {MIN_EDIT_TARGET_CHARS})")]
    TargetTooShort(usize),
    #[error("未找到包含目标文本的文件")]
    NotFound,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 簇内一个思考文件 (name = 文件名, 如 `2026-08-16-001.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThoughtFile {
    pub name: String,
    pub content: String,
}

/// 链注册表 (meta_thinking_chains.json): 链名 → 簇名列表.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MetaChains {
    #[serde(default)]
    chains: BTreeMap<String, Vec<String>>,
}

/// 思维簇读取口 (元自学习消费侧): 反思/做梦经此回读 AI 自己的历史思考链.
///
/// 全部 infallible (失败/不存在 → 空), 消费方本就按"诚实降级"设计.
pub trait ThoughtClusterReader: Send + Sync {
    /// 列出全部簇名 (字典序, 确定性).
    fn clusters(&self) -> Vec<String>;
    /// 读一个簇的全部思考文件 (按文件名字典序 = 时间序); 不存在 → 空.
    fn read_cluster(&self, name: &str) -> Vec<ThoughtFile>;
    /// 读一条链 (链 = 一组簇): 返回 `簇/文件名` 形式的 ThoughtFile, 按簇名/文件名序; 链不存在 → 空.
    fn read_chain(&self, name: &str) -> Vec<ThoughtFile>;
}

/// 思维簇管理器: AI 思维链文件按主题聚簇落盘 (文件形态, 按日归档).
pub struct ThoughtClusterManager {
    root: PathBuf,
    clock: Arc<dyn Clock>,
}

impl ThoughtClusterManager {
    /// root = 思维簇根目录 (注入, 如 `<memory_path>/thought_clusters`); clock 注入 → 可测.
    pub fn new(root: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self { root: root.into(), clock }
    }

    /// 簇名规范化: 去空白; 拒绝空名/路径分隔符/`..`(防目录穿越)/非「簇」结尾.
    fn normalize_name(name: &str) -> Result<String, ThoughtClusterError> {
        let cleaned: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        let bad = cleaned.is_empty()
            || !cleaned.ends_with(CLUSTER_SUFFIX)
            || cleaned.contains('/')
            || cleaned.contains('\\')
            || cleaned.contains("..");
        if bad {
            return Err(ThoughtClusterError::InvalidName(name.to_string()));
        }
        Ok(cleaned)
    }

    fn cluster_dir(&self, name: &str) -> Result<PathBuf, ThoughtClusterError> {
        Ok(self.root.join(Self::normalize_name(name)?))
    }

    /// 新建一个思考文件落盘; 文件名 = `{日期}-{当日序号}.md` (时钟注入 → 确定性).
    /// 簇目录不存在自动创建. 空内容 → Err.
    pub fn create_file(&self, cluster: &str, content: &str) -> Result<PathBuf, ThoughtClusterError> {
        if content.trim().is_empty() {
            return Err(ThoughtClusterError::EmptyContent);
        }
        let dir = self.cluster_dir(cluster)?;
        std::fs::create_dir_all(&dir)?;
        let date = self.clock.now().format("%Y-%m-%d").to_string();
        // 当日序号 = 同日期前缀文件数 + 1 (扫目录得, 崩溃重入也不覆盖)
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

    /// 列出全部簇目录名 (字典序); root 不存在 → 空 (空簇场景不报错).
    pub fn list_clusters(&self) -> Result<Vec<String>, ThoughtClusterError> {
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

    /// 读一个簇的全部思考文件 (字典序 = 时间序, .md/.txt); 簇不存在 → 空; 非法名 → Err.
    pub fn read_cluster(&self, name: &str) -> Result<Vec<ThoughtFile>, ThoughtClusterError> {
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
            out.push(ThoughtFile { name, content });
        }
        Ok(out)
    }

    fn load_chains(&self) -> Result<MetaChains, ThoughtClusterError> {
        let path = self.root.join(META_CHAINS_FILE);
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(MetaChains::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// 注册一条链 (链名 → 簇列表) 到 meta_thinking_chains.json (BTreeMap → 序列化确定).
    pub fn register_chain(&self, chain: &str, clusters: &[String]) -> Result<(), ThoughtClusterError> {
        if chain.trim().is_empty() {
            return Err(ThoughtClusterError::InvalidName(chain.to_string()));
        }
        for c in clusters {
            Self::normalize_name(c)?; // 链成员也必须是合法簇名
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

    /// 读一条链: 链内各簇的思考文件, ThoughtFile.name = `簇/文件` (簇名序 → 文件名序, 确定).
    pub fn read_chain(&self, chain: &str) -> Result<Vec<ThoughtFile>, ThoughtClusterError> {
        let meta = self.load_chains()?;
        let Some(clusters) = meta.chains.get(chain.trim()) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for c in clusters {
            for f in self.read_cluster(c)? {
                out.push(ThoughtFile {
                    name: format!("{c}/{}", f.name),
                    content: f.content,
                });
            }
        }
        Ok(out)
    }

    /// 编辑思考文件: 在指定簇 (None = 全部簇, 字典序) 中找第一个包含 target 的文件,
    /// 替换第一处出现并写回. target < 15 字符 → Err (防误伤, VCP 规则).
    pub fn edit_file(
        &self,
        cluster: Option<&str>,
        target: &str,
        replacement: &str,
    ) -> Result<PathBuf, ThoughtClusterError> {
        let n = target.chars().count();
        if n < MIN_EDIT_TARGET_CHARS {
            return Err(ThoughtClusterError::TargetTooShort(n));
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
        Err(ThoughtClusterError::NotFound)
    }

    /// 检索: 全部簇中内容含 query 的文件 ((簇, 文件, 命中次数), 簇名序 → 文件名序, 确定).
    /// 空 query → 空结果.
    pub fn search(&self, query: &str) -> Result<Vec<(String, String, usize)>, ThoughtClusterError> {
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

impl ThoughtClusterReader for ThoughtClusterManager {
    fn clusters(&self) -> Vec<String> {
        self.list_clusters().unwrap_or_default()
    }

    fn read_cluster(&self, name: &str) -> Vec<ThoughtFile> {
        ThoughtClusterManager::read_cluster(self, name).unwrap_or_default()
    }

    fn read_chain(&self, name: &str) -> Vec<ThoughtFile> {
        ThoughtClusterManager::read_chain(self, name).unwrap_or_default()
    }
}

/// 测试用临时 root (uuid 隔离, 用完删除).
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;

    fn vclock() -> VirtualClock {
        VirtualClock::new(chrono::Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap())
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("apeireth-tcm-test-{}", uuid::Uuid::new_v4()))
    }

    fn mgr(root: &Path) -> ThoughtClusterManager {
        ThoughtClusterManager::new(root.to_path_buf(), Arc::new(vclock()))
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn create_file_names_are_date_seq_deterministic() {
        let root = temp_root();
        let m = mgr(&root);
        let p1 = m.create_file("前思维簇", "【思考模块：模块A】\n触发条件: x").unwrap();
        let p2 = m.create_file("前思维簇", "第二条思考").unwrap();
        assert_eq!(p1.file_name().unwrap(), "2026-08-16-001.md");
        assert_eq!(p2.file_name().unwrap(), "2026-08-16-002.md", "同日序号递增");
        assert!(p2.starts_with(root.join("前思维簇")));
        // 另一簇独立编号
        let p3 = m.create_file("反思簇", "反思一条").unwrap();
        assert_eq!(p3.file_name().unwrap(), "2026-08-16-001.md");
        cleanup(&root);
    }

    #[test]
    fn invalid_inputs_rejected() {
        let root = temp_root();
        let m = mgr(&root);
        assert!(matches!(m.create_file("名字没后缀", "x"), Err(ThoughtClusterError::InvalidName(_))));
        // 纯空白名去空白后为空 → 非法
        assert!(matches!(m.create_file("   ", "x"), Err(ThoughtClusterError::InvalidName(_))));
        assert!(matches!(m.create_file("../逃逸簇", "x"), Err(ThoughtClusterError::InvalidName(_))));
        assert!(matches!(m.create_file("好簇", "  "), Err(ThoughtClusterError::EmptyContent)));
        // 编辑目标过短 (< 15 字符)
        assert!(matches!(m.edit_file(None, "短文本", "新"), Err(ThoughtClusterError::TargetTooShort(3))));
        // 非法文件内容读回也应拒绝
        m.create_file("正簇", "一条足够长的思考内容，用于后续编辑测试。").unwrap();
        assert!(matches!(m.edit_file(Some("bad name"), "一条足够长的思考内容，用于后续编辑测试。", "x"), Err(ThoughtClusterError::InvalidName(_))));
        cleanup(&root);
    }

    #[test]
    fn list_clusters_sorted_and_empty_root_ok() {
        let root = temp_root();
        let m = mgr(&root);
        assert_eq!(m.list_clusters().unwrap(), Vec::<String>::new(), "root 不存在 = 空, 不报错");
        m.create_file("乙簇", "b").unwrap();
        m.create_file("甲簇", "a").unwrap();
        std::fs::create_dir_all(root.join("不是簇的目录")).unwrap();
        std::fs::write(root.join("散文件.md"), "x").unwrap();
        // 字典序: 乙(U+4E59) < 甲(U+7532); 非簇目录与散文件不计入
        assert_eq!(m.list_clusters().unwrap(), vec!["乙簇".to_string(), "甲簇".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn read_cluster_sorted_empty_and_missing() {
        let root = temp_root();
        let m = mgr(&root);
        assert!(m.read_cluster("不存在簇").unwrap().is_empty(), "不存在 → 空");
        m.create_file("思考簇", "第一条").unwrap();
        m.create_file("思考簇", "第二条").unwrap();
        std::fs::write(root.join("思考簇").join("README.md"), "手册").unwrap();
        std::fs::write(root.join("思考簇").join("ignore.bin"), "非文本").unwrap();
        let files = m.read_cluster("思考簇").unwrap();
        let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["2026-08-16-001.md", "2026-08-16-002.md", "README.md"]);
        // 空簇 (目录在但无文件)
        std::fs::create_dir_all(root.join("空簇")).unwrap();
        assert!(m.read_cluster("空簇").unwrap().is_empty());
        cleanup(&root);
    }

    #[test]
    fn chains_register_and_read_deterministic() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("前思维簇", "前置思考链内容").unwrap();
        m.create_file("后思维簇", "后置思考链内容").unwrap();
        // 注册时乱序 + 重复 → 落盘排序去重 (确定)
        m.register_chain("coding", &["后思维簇".into(), "前思维簇".into(), "后思维簇".into()])
            .unwrap();
        let files = m.read_chain("coding").unwrap();
        let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["前思维簇/2026-08-16-001.md", "后思维簇/2026-08-16-001.md"]);
        // 链不存在 → 空; 非法簇名入链 → Err; 空链名 → Err
        assert!(m.read_chain("nope").unwrap().is_empty());
        assert!(m.register_chain("bad", &["无后缀".into()]).is_err());
        assert!(m.register_chain("  ", &["前思维簇".into()]).is_err());
        cleanup(&root);
    }

    #[test]
    fn edit_file_replaces_first_occurrence_across_clusters() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("甲簇", "旧思考内容甲，一共十五个字以上。").unwrap();
        m.create_file("乙簇", "无关内容").unwrap();
        let target = "旧思考内容甲，一共十五个字以上。";
        let path = m.edit_file(None, target, "新思考内容甲（已修订）。").unwrap();
        assert!(path.starts_with(root.join("甲簇")));
        let edited = std::fs::read_to_string(&path).unwrap();
        assert!(edited.contains("新思考内容甲（已修订）。"));
        assert!(!edited.contains(target));
        // 找不到 → NotFound
        assert!(matches!(
            m.edit_file(None, "这段文本根本不存在于任何簇文件里面", "x"),
            Err(ThoughtClusterError::NotFound)
        ));
        cleanup(&root);
    }

    #[test]
    fn search_is_deterministic_and_counts_hits() {
        let root = temp_root();
        let m = mgr(&root);
        m.create_file("乙簇", "元自学习 元自学习").unwrap();
        m.create_file("甲簇", "元自学习 一次").unwrap();
        m.create_file("甲簇", "无关文件").unwrap();
        let hits = m.search("元自学习").unwrap();
        assert_eq!(hits.len(), 2, "两文件命中");
        // 簇名字典序: 乙(U+4E59) < 甲(U+7532)
        assert_eq!(hits[0].0, "乙簇");
        assert_eq!(hits[0].2, 2, "命中次数");
        assert_eq!(hits[1].0, "甲簇");
        assert_eq!(hits[1].2, 1);
        assert!(m.search("").unwrap().is_empty(), "空 query → 空");
        assert!(m.search("不存在词").unwrap().is_empty());
        cleanup(&root);
    }

    #[test]
    fn reader_trait_impl_matches_manager() {
        let root = temp_root();
        let m = mgr(&root);
        let r: Arc<dyn ThoughtClusterReader> = Arc::new(ThoughtClusterManager::new(root.clone(), Arc::new(vclock())));
        m.create_file("思簇", "思考一条").unwrap();
        m.register_chain("c", &["思簇".into()]).unwrap();
        assert_eq!(r.clusters(), vec!["思簇".to_string()]);
        assert_eq!(r.read_cluster("思簇").len(), 1);
        assert_eq!(r.read_cluster("没有簇").len(), 0);
        assert_eq!(r.read_chain("c").len(), 1);
        assert!(r.read_chain("没有链").is_empty());
        cleanup(&root);
    }
}
