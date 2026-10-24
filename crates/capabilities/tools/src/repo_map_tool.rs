//! 代码地图工具面 (`tool.repo_map`, 声明名 `repo_map`)。
//!
//! # 这个工具做什么
//!
//! 把一个代码目录压成**紧凑的符号地图**（Aider 式的 `repo map`）：
//! 遍历源文件 → 抽 AST 级符号签名 → 建跨文件依赖图 → Personalized PageRank
//! 排序（当前关注的文件权重更高）→ 在 **token 预算内**输出排名后的代码大纲
//! （函数体一律省略成 `...`）。
//!
//! 用途是给模型一个「这个仓库长什么样」的骨架，而不是把整个仓库塞进上下文。
//!
//! # 0 假装边界
//!
//! - **只读**：不写任何文件、不改任何状态；
//! - **工作区限域**：请求路径必须落在 `workspace_root` 内（词法层 + 现实层双检，
//!   与 `doc_ops` 同一把尺），`..` 逃逸与绝对路径越界一律拒绝即帧；
//! - **有预算**：文件数上限、单文件字节上限、token 预算三重封顶 ——
//!   撞上限时**如实报告截断**（`truncated` + 原因），不静默少扫；
//! - **不强解析**：符号提取是逐行的结构化文本规则（不是编译器级 AST），
//!   所以宏生成的符号、跨行签名等会漏 —— 这是**已知局限**，写在工具描述里，
//!   不假装是全精度索引。
//!
//! # 实测成本（本仓真数据）
//!
//! | 扫描面 | 文件数 | 用时 |
//! |---|---|---|
//! | `crates/engine/memory` | 152 | **0.26s** |
//! | `crates`（全仓 30 crate） | 1319 | **2.1s** |
//!
//! 这两行来之不易：接线时发现建图与 PageRank **两处都是平方级**，
//! 152 文件要 **9.5s**、全仓要 **10.8s**（一个工具卡十秒 = 不可用）。
//! 已修（token 索引建边 + 推式 PageRank，见 `repo_map::RepoDependencyGraph`
//! 的注释），并加了复杂度回归守卫测试。

use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};

use crate::repo_map::{RepoDependencyGraph, RepoMapGenerator, SymbolParser, SymbolTag};

/// 能力 id（注册用命名空间）。
pub const REPO_MAP_CAPABILITY_ID: &str = "tool.repo_map";
/// 模型可见的工具名（短，因为每次请求都要花 token）。
pub const REPO_MAP_TOOL_NAME: &str = "repo_map";

/// 默认 token 预算（≈ 输出字符数 / 4）。
pub const REPO_MAP_DEFAULT_TOKEN_BUDGET: usize = 1_500;
/// token 预算上限（防模型传个天文数字把上下文撑爆）。
pub const REPO_MAP_MAX_TOKEN_BUDGET: usize = 16_000;
/// 单次扫描的文件数上限。
pub const REPO_MAP_MAX_FILES: usize = 2_000;
/// 单文件字节上限（超了跳过并计数，不静默）。
pub const REPO_MAP_MAX_FILE_BYTES: u64 = 256 * 1024;
/// PageRank 阻尼系数（Aider 同款 0.85）。
const REPO_MAP_DAMPING: f64 = 0.85;
/// PageRank 最大迭代次数。
const REPO_MAP_MAX_ITERATIONS: usize = 100;
/// PageRank 收敛容差。
const REPO_MAP_TOLERANCE: f64 = 1e-6;

/// 扫描时跳过的目录名（构建产物、版本库、依赖缓存）。
const SKIP_DIRS: [&str; 12] = [
    "target",
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "dist",
    "build",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".apeireth",
];

/// 工具配置（工作区限域根 + 三重预算）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMapConfig {
    /// 工作区根（扫描限域）。
    pub workspace_root: PathBuf,
    /// 文件数上限。
    pub max_files: usize,
    /// 单文件字节上限。
    pub max_file_bytes: u64,
}

impl RepoMapConfig {
    /// 工作区根 + 默认预算。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            max_files: REPO_MAP_MAX_FILES,
            max_file_bytes: REPO_MAP_MAX_FILE_BYTES,
        }
    }

    /// 覆盖预算（测试/宿主微调；命名即承诺：值即生效权力）。
    pub fn with_limits(mut self, max_files: usize, max_file_bytes: u64) -> Self {
        self.max_files = max_files;
        self.max_file_bytes = max_file_bytes;
        self
    }
}

/// 一次生成的结构化结果。
///
/// `map` 是模型真正要看的紧凑大纲；其余字段是**诚实账**（扫了多少、跳了多少、
/// 为什么截断），便于调用方判断这份地图可不可信。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMapOutcome {
    /// 紧凑代码大纲（排名后的符号签名，函数体省略成 `...`）。
    pub map: String,
    /// 实际纳入依赖图并成功解析的文件数。
    pub scanned_files: usize,
    /// 因超单文件字节上限被跳过的文件数。
    pub skipped_oversize: usize,
    /// 因超文件总数上限未纳入的文件数。
    pub skipped_over_count: usize,
    /// 是否发生了任何截断（`map` 只含排名最靠前的那部分）。
    pub truncated: bool,
    /// 截断/跳过的可读说明（无截断时为空）。
    pub note: Option<String>,
}

/// 工具错误（拒绝即帧，真因如实）。
#[derive(Debug)]
enum RepoMapError {
    /// 参数非法。
    InvalidParam(String),
    /// 路径越出工作区。
    OutsideWorkspace(String),
    /// 路径不存在或不是目录。
    NotFound(String),
    /// IO 失败。
    Io(String),
}

impl RepoMapError {
    fn message(&self) -> String {
        match self {
            Self::InvalidParam(detail) => format!("参数非法: {detail}"),
            Self::OutsideWorkspace(detail) => {
                format!("路径越出工作区（本工具只读工作区内路径）: {detail}")
            }
            Self::NotFound(detail) => format!("路径不可用: {detail}"),
            Self::Io(detail) => format!("IO 失败: {detail}"),
        }
    }
}

/// 代码地图工具（`tool.repo_map`，声明名 `repo_map`）。
pub struct RepoMapTool {
    id: CapabilityId,
    config: RepoMapConfig,
}

impl RepoMapTool {
    /// 绑定配置（工作区限域根 + 预算）。
    pub fn new(config: RepoMapConfig) -> Self {
        Self {
            id: CapabilityId::new(REPO_MAP_CAPABILITY_ID).expect("valid capability id"),
            config,
        }
    }

    /// 执行面（测试直调口，与工具面同一真源）。
    pub fn generate(
        &self,
        requested: &str,
        focus: &[String],
        token_budget: usize,
    ) -> Result<RepoMapOutcome, String> {
        self.generate_inner(requested, focus, token_budget)
            .map_err(|error| error.message())
    }

    fn generate_inner(
        &self,
        requested: &str,
        focus: &[String],
        token_budget: usize,
    ) -> Result<RepoMapOutcome, RepoMapError> {
        let root = resolve_contained(&self.config.workspace_root, requested)?;
        if !root.is_dir() {
            return Err(RepoMapError::NotFound(format!(
                "{} 不是目录",
                root.display()
            )));
        }
        // focus 文件同样要限域（不然可以把工作区外的文件拉进排名）
        let focus_files: Vec<PathBuf> = focus
            .iter()
            .filter(|item| !item.trim().is_empty())
            .filter_map(|item| {
                resolve_contained(&self.config.workspace_root, item)
                    .ok()
                    .filter(|path| path.is_file())
            })
            .collect();

        let scan = scan_sources(&root, self.config.max_files, self.config.max_file_bytes)?;

        let file_tags: Vec<(PathBuf, Vec<SymbolTag>)> = scan
            .files
            .iter()
            .map(|(path, content)| (path.clone(), SymbolParser::parse_file(path, content)))
            .collect();
        let file_contents: Vec<(PathBuf, String)> = scan.files.clone();

        let graph = RepoDependencyGraph::build(&file_tags, &file_contents);
        let ranks = graph.compute_personalized_pagerank(
            &focus_files,
            REPO_MAP_DAMPING,
            REPO_MAP_MAX_ITERATIONS,
            REPO_MAP_TOLERANCE,
        );
        let mut ranked: Vec<(PathBuf, f64)> = ranks.into_iter().collect();
        // 排名降序；同分按路径升序（确定性输出，便于对账与测试）
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });

        let budget = token_budget.clamp(1, REPO_MAP_MAX_TOKEN_BUDGET);
        let map = RepoMapGenerator::new(budget).generate_map(&file_tags, &ranked);

        let mut notes: Vec<String> = Vec::new();
        if scan.skipped_oversize > 0 {
            notes.push(format!(
                "{} 个文件超单文件上限（{} 字节）已跳过",
                scan.skipped_oversize, self.config.max_file_bytes
            ));
        }
        if scan.skipped_over_count > 0 {
            notes.push(format!(
                "{} 个文件因超文件总数上限（{}）未纳入",
                scan.skipped_over_count, self.config.max_files
            ));
        }
        let truncated = !notes.is_empty() || ranked.len() > parsed_file_count(&map);
        if truncated && notes.is_empty() {
            notes.push(format!(
                "输出受 {} token 预算限制，只含排名最靠前的那部分文件",
                budget
            ));
        }
        Ok(RepoMapOutcome {
            map,
            scanned_files: file_contents.len(),
            skipped_oversize: scan.skipped_oversize,
            skipped_over_count: scan.skipped_over_count,
            truncated,
            note: if notes.is_empty() {
                None
            } else {
                Some(notes.join("；"))
            },
        })
    }
}

/// 地图里实际出现的文件数（用于判断是否被 token 预算截断）。
fn parsed_file_count(map: &str) -> usize {
    map.lines().filter(|line| line.starts_with("## ")).count()
}

/// 一次扫描结果。
struct ScanOutcome {
    /// 成功读入的文件（路径, 内容）。
    files: Vec<(PathBuf, String)>,
    /// 因超单文件字节上限跳过的数量。
    skipped_oversize: usize,
    /// 因超文件总数上限未纳入的数量。
    skipped_over_count: usize,
}

/// 递归扫描目录下的源文件（确定性顺序，跳构建产物目录）。
fn scan_sources(
    root: &Path,
    max_files: usize,
    max_file_bytes: u64,
) -> Result<ScanOutcome, RepoMapError> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut skipped_oversize = 0usize;
    let mut skipped_over_count = 0usize;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| RepoMapError::Io(format!("{}: {error}", directory.display())))?;
        // 同一层的条目按名字排序 ⇒ 遍历顺序确定（不受文件系统顺序影响）
        let mut children: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|error| RepoMapError::Io(format!("{}: {error}", directory.display())))?;
            children.push(entry.path());
        }
        children.sort();
        // 逆序入栈，出栈时正好是升序
        for path in children.into_iter().rev() {
            let Ok(file_type) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            // 跳过符号链接（防环 + 防越域）
            if file_type.file_type().is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                if SKIP_DIRS.contains(&name) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !is_source_file(&path) {
                continue;
            }
            if files.len() >= max_files {
                skipped_over_count += 1;
                continue;
            }
            if file_type.len() > max_file_bytes {
                skipped_oversize += 1;
                continue;
            }
            // 非 UTF-8（如二进制误标扩展名）静默跳过：不是源文件
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            files.push((path, content));
        }
    }

    // 最终按路径排序，保证 generate_map 的输入顺序确定
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(ScanOutcome {
        files,
        skipped_oversize,
        skipped_over_count,
    })
}

/// 是否算源文件（只认 `parse_file` 能识别的那些语言）。
fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go")
    )
}

/// 把请求路径解析进工作区根并证明包含（词法层 + 现实层双检）。
///
/// 与 `doc_ops::resolve_contained` 同一把尺：先做**词法层**归一（拒绝 `..`
/// 逃逸），再对已存在的路径做 **canonicalize** 比对（防符号链接绕过）。
fn resolve_contained(root: &Path, requested: &str) -> Result<PathBuf, RepoMapError> {
    if requested.trim().is_empty() {
        return Err(RepoMapError::InvalidParam("路径不能为空".to_string()));
    }
    let root = root
        .canonicalize()
        .map_err(|error| RepoMapError::Io(format!("工作区根不可用: {error}")))?;
    let requested_path = Path::new(requested);
    let candidate = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        root.join(requested_path)
    };

    // 词法层：逐个组件折叠，遇到越界即拒
    let mut lexical = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::ParentDir => {
                if !lexical.pop() {
                    return Err(RepoMapError::OutsideWorkspace(requested.to_string()));
                }
            }
            Component::CurDir => {}
            other => lexical.push(other.as_os_str()),
        }
    }

    // 现实层：路径存在则 canonicalize 后必须仍在根内（防符号链接）
    match lexical.canonicalize() {
        Ok(real) => {
            if real.starts_with(&root) || real == root {
                Ok(real)
            } else {
                Err(RepoMapError::OutsideWorkspace(requested.to_string()))
            }
        }
        Err(_) => {
            // 不存在：直接用词法归一结果，但仍要证明它在根内
            if lexical.starts_with(&root) {
                Ok(lexical)
            } else {
                Err(RepoMapError::OutsideWorkspace(requested.to_string()))
            }
        }
    }
}

#[async_trait]
impl ToolCapability for RepoMapTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要生成地图的目录（工作区内，相对或绝对）。默认 \".\" 表示工作区根。"
                },
                "focus": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "当前关注的文件（工作区内）。这些文件在排名里权重更高 —— 用于「我正改这几个文件，给我相关上下文」。"
                },
                "token_budget": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": REPO_MAP_MAX_TOKEN_BUDGET,
                    "description": "输出大纲的 token 预算（默认 1500，上限 16000）。超预算时只保留排名最靠前的文件，并在 note 里如实说明。"
                }
            },
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new(REPO_MAP_TOOL_NAME)
            .with_description(
                "生成代码地图: 抽取源文件符号签名 (函数/结构体/枚举/trait/类)、\
                 建跨文件依赖图、按 Personalized PageRank 排序 (focus 文件权重更高)，\
                 在 token 预算内输出紧凑大纲 (函数体省略成 `...`)。\
                 支持 Rust / Python / TypeScript / JavaScript / Go。\
                 只读、工作区限域。已知局限: 符号提取是逐行规则不是编译器级 AST，\
                 宏生成符号与跨行签名会漏 —— 它是「骨架概览」，不是全精度索引。",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let requested = call
            .arguments
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(".")
            .to_string();
        let focus: Vec<String> = call
            .arguments
            .get("focus")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let token_budget = call
            .arguments
            .get("token_budget")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(REPO_MAP_DEFAULT_TOKEN_BUDGET);

        match self.generate_inner(&requested, &focus, token_budget) {
            Ok(outcome) => match serde_json::to_value(&outcome) {
                Ok(value) => ToolResult::ok(&call.id, value).with_name(REPO_MAP_TOOL_NAME),
                Err(error) => {
                    ToolResult::permanent_error(&call.id, format!("结果序列化失败: {error}"))
                        .with_name(REPO_MAP_TOOL_NAME)
                }
            },
            Err(error) => {
                ToolResult::permanent_error(&call.id, error.message()).with_name(REPO_MAP_TOOL_NAME)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create dirs");
        }
        std::fs::write(path, content).expect("write file");
    }

    #[test]
    fn generates_a_map_with_symbols_and_skips_build_dirs() {
        let dir = workspace();
        let root = dir.path();
        write(
            root,
            "src/lib.rs",
            "pub struct Engine;\npub fn start_engine() {}\n",
        );
        write(root, "src/helper.py", "def helper_fn():\n    pass\n");
        // 构建产物目录必须跳过
        write(root, "target/debug/junk.rs", "pub fn junk() {}\n");
        // 非源码文件不扫
        write(root, "README.md", "# 文档\n");

        let tool = RepoMapTool::new(RepoMapConfig::new(root));
        let outcome = tool
            .generate(".", &[], REPO_MAP_DEFAULT_TOKEN_BUDGET)
            .expect("map");
        assert!(outcome.map.contains("Engine"), "{}", outcome.map);
        assert!(outcome.map.contains("start_engine"), "{}", outcome.map);
        assert!(outcome.map.contains("helper_fn"), "{}", outcome.map);
        assert!(
            !outcome.map.contains("junk"),
            "target/ 必须跳过: {}",
            outcome.map
        );
        assert_eq!(outcome.scanned_files, 2);
        assert!(!outcome.truncated);
    }

    #[test]
    fn rejects_paths_outside_the_workspace() {
        let dir = workspace();
        let root = dir.path();
        write(root, "src/lib.rs", "pub fn inside() {}\n");
        let tool = RepoMapTool::new(RepoMapConfig::new(root));

        let error = tool
            .generate("../../..", &[], 1_500)
            .expect_err("越界必须拒绝");
        assert!(error.contains("越出工作区"), "{error}");

        let error = tool
            .generate("/etc", &[], 1_500)
            .expect_err("绝对路径越界必须拒绝");
        assert!(error.contains("越出工作区"), "{error}");
    }

    #[test]
    fn focus_files_are_ranked_higher() {
        let dir = workspace();
        let root = dir.path();
        write(root, "src/a.rs", "pub fn alpha() {}\n");
        write(root, "src/b.rs", "pub fn beta() {}\n");
        let tool = RepoMapTool::new(RepoMapConfig::new(root));
        let outcome = tool
            .generate(
                ".",
                &["src/b.rs".to_string()],
                REPO_MAP_DEFAULT_TOKEN_BUDGET,
            )
            .expect("map");
        let beta_at = outcome.map.find("beta").expect("beta 应在图里");
        let alpha_at = outcome.map.find("alpha").expect("alpha 应在图里");
        assert!(
            beta_at < alpha_at,
            "focus 文件应排更前: beta@{beta_at} alpha@{alpha_at}\n{}",
            outcome.map
        );
    }

    #[test]
    fn oversized_files_are_reported_not_silently_dropped() {
        let dir = workspace();
        let root = dir.path();
        write(root, "src/small.rs", "pub fn small() {}\n");
        write(root, "src/big.rs", &"pub fn big() {}\n".repeat(200));
        let tool = RepoMapTool::new(RepoMapConfig::new(root).with_limits(100, 64));
        let outcome = tool
            .generate(".", &[], REPO_MAP_DEFAULT_TOKEN_BUDGET)
            .expect("map");
        assert_eq!(outcome.skipped_oversize, 1, "{outcome:?}");
        assert!(outcome.truncated, "跳过必须算截断: {outcome:?}");
        assert!(
            outcome
                .note
                .as_deref()
                .is_some_and(|note| note.contains("超单文件上限")),
            "{outcome:?}"
        );
    }

    #[test]
    fn missing_directory_is_a_clear_error() {
        let dir = workspace();
        let tool = RepoMapTool::new(RepoMapConfig::new(dir.path()));
        let error = tool
            .generate("does-not-exist", &[], 1_500)
            .expect_err("应报错");
        assert!(
            error.contains("不可用") || error.contains("不是目录"),
            "{error}"
        );
    }

    #[test]
    fn output_is_deterministic() {
        let dir = workspace();
        let root = dir.path();
        for index in 0..6 {
            write(
                root,
                &format!("src/mod{index}.rs"),
                &format!("pub fn function_{index}() {{}}\n"),
            );
        }
        let tool = RepoMapTool::new(RepoMapConfig::new(root));
        let first = tool.generate(".", &[], 800).expect("map").map;
        let second = tool.generate(".", &[], 800).expect("map").map;
        assert_eq!(first, second, "同输入必须同输出（便于对账）");
    }
}
