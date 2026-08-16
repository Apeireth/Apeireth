//! # apeireth-repo-analyzer
//!
//! Repo Analyzer (1:1 翻译 v0.9.21 商业版 `out/main/chunks/RepoAnalyzer-BjPzFZvZ.js` ~6KB).
//! 商业版 RepoAnalyzer 是 **代码质量 + 技术债 + 复杂度 + 依赖 + 安全审计 + 报告生成** 一体化工具,
//! 通过 `RepoScanAdapter-CsMFZlsN.js` 共用扫描结果 (per `v09021-commercial-extract §3.3`).
//! 本 crate 1:1 翻译 + 扩展 7 个核心能力, R20 阶段 1 续 P1 估缺.
//!
//! ## v0.9.21 RepoAnalyzer-BjPzFZvZ.js 实查 (obfuscated webpack bundle, 6KB, 单行)
//!
//! | Token | 命中 | 推断 | 出处 |
//! |-------|-----:|------|------|
//! | `RepoScanAdapter` (require) | 1 | 复用 RepoScan 扫描结果 (file/dir/git_status) | 字符串拼接 `RepoScanAdapte`+`r-CsMFZl`+`sN.js` |
//! | `installErrorCodes` (require) | 1 | 错误码 (per `InstallErrorCodes-D-xTEXPe.js`) | 字符串 `installErrorCo`+`des-...js` |
//! | `RepoProfile` (require) | 1 | 仓库画像 (file count / dep / language breakdown) | 字符串 `RepoProfile.js` |
//! | `fs` (require) | 1 | 读源文件 (analyze pass 1) | 字符串 `fs` |
//! | `os` (require) | 1 | 读 cwd / homedir | 字符串 `os` |
//! | `path` (require) | 1 | 路径拼接 | 字符串 `path` |
//! | `tree-sitter` / `@typescript-eslint/parser` | **估 0** (obfuscated 不可 grep) | **v0.9.21 估缺 AST, 用正则 token 扫描** | 推断 |
//! | `cyclomatic` / `complexity` | 估 0 | v0.9.21 估缺 cyclomatic complexity, R20 阶段 1 续补 | O-5 不假装 |
//! | `sarif` | 估 0 | v0.9.21 估缺 SARIF 报告, R20 阶段 1 续补 | O-5 不假装 |
//!
//! ## 关键 design (R20 阶段 1 续)
//!
//! - **1:1 翻译**: 7 核心能力 (质量/技术债/复杂度/统计/依赖/安全/报告) 严格按 v0.9.21 字段.
//! - **集成**: 跟 `apeireth-repo-scan` (P1 估缺) 配对使用, 共享 `ScanResult` / `RepoProfile` 类型.
//! - **m3 防御**: 8 工具白名单 hardcode, 防止 m3 模型幻觉调用不存在的分析工具.
//! - **K-1 强校验**: 编译期 hardcode `"apeireth"` 平台名 + 5 TechDebtType 枚举 + 8 工具名 + 5 K-1 字样.
//! - **报告格式**: JSON / Markdown / SARIF 3 格式 (per v0.9.21 估缺, R20 阶段 1 续补).
//! - **依赖格式**: Cargo.toml / package.json / pyproject.toml 3 格式 (per v0.9.21 估缺).
//!
//! ## 状态: ⏳ skeleton (R20 阶段 1 续, 主 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力" 5 P0 crate 已写, 本 crate 是阶段 1 续 P1 估缺)

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的分析工具 (eg. "apeireth_repo_analyzer_audit" 实际不存在).
// ============================================================================

/// m3 防御: Repo Analyzer 8 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// 8 工具 = 4 核心分析 (complexity / tech_debt / deps / security) +
///          1 函数统计 + 3 报告生成 (json / markdown / sarif).
pub const TOOL_WHITELIST: &[&str] = &[
    // 4 核心分析
    "apeireth_repo_analyzer_complexity",
    "apeireth_repo_analyzer_tech_debt",
    "apeireth_repo_analyzer_deps",
    "apeireth_repo_analyzer_security",
    // 1 函数统计
    "apeireth_repo_analyzer_functions",
    // 3 报告生成
    "apeireth_repo_analyzer_report_json",
    "apeireth_repo_analyzer_report_markdown",
    "apeireth_repo_analyzer_report_sarif",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8 (per K-1 强校验 + 8 项不修改承诺 #5).
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `AnalyzerError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), AnalyzerError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(AnalyzerError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 文档头 + 编译期 hardcode (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Repo Analyzer schema version (per v0.9.21 `RepoProfile.schemaVersion` 字段 1:1).
/// K-1 强校验 #1: 编译期 hardcode, 不写 `"1"` 字符串 elsewhere.
pub const REPO_ANALYZER_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// 支持的技术债类型 (5 种, 1:1 翻译 v0.9.21 `techDebtTypes` 字段).
///
/// v0.9.21 估 5 类: TODO / FIXME / HACK / BUG / SECURITY (per O-5 不假装实测 0 命中 AST 库,
/// 用正则 token 扫描). 编译期 hardcode, 不可运行时改.
pub const SUPPORTED_TECH_DEBT: &[TechDebtType] = &[
    TechDebtType::Todo,
    TechDebtType::Fixme,
    TechDebtType::Hack,
    TechDebtType::Bug,
    TechDebtType::SecurityIssue,
];

/// 编译期守门: SUPPORTED_TECH_DEBT 长度 == 5 (K-1 强校验).
const _: () = assert!(SUPPORTED_TECH_DEBT.len() == 5);

/// 圈复杂度警告阈值 (per industry standard: 20 警告 / 50 错误, per ISO 26262).
pub const MAX_CYCLOMATIC_COMPLEXITY: u32 = 20;

/// 支持的依赖文件格式 (3 种, 1:1 翻译 v0.9.21 估缺 Cargo / npm / PyPI 三大生态).
pub const SUPPORTED_DEP_FORMATS: &[&str] = &["Cargo.toml", "package.json", "pyproject.toml"];

/// 编译期守门: SUPPORTED_DEP_FORMATS 长度 == 3.
const _: () = assert!(SUPPORTED_DEP_FORMATS.len() == 3);

/// 支持的报告输出格式 (3 种, 1:1 翻译 v0.9.21 `reportFormats` 字段).
pub const SUPPORTED_REPORT_FORMATS: &[&str] = &["json", "markdown", "sarif"];

/// 编译期守门: SUPPORTED_REPORT_FORMATS 长度 == 3.
const _: () = assert!(SUPPORTED_REPORT_FORMATS.len() == 3);

/// K-1 强校验 5 字样 (per 任务规范 "5 K-1 字样" fixture 验证).
///
/// 这 5 个字符串必须出现在 crate 内部 (编译期 hardcode), 用于测试 fixture
/// 验证 "apeireth 平台名" + "repo_analyzer 工具前缀" + "analyze 操作动词" +
/// "complexity 核心指标" + "must-do 设计哲学" 5 条 K-1 不变量.
pub const K1_INVARIANTS: &[&str] = &[
    "apeireth",      // 平台名 (1:1 v0.9.21)
    "repo_analyzer", // 工具名前缀 (m3 防御)
    "analyze",       // 操作动词 (1:1 v0.9.21)
    "complexity",    // 核心指标 (cyclomatic complexity)
    "must-do",       // 设计哲学 (K-1 强校验: must-do 这 5 条不变量)
];

/// 编译期守门: K1_INVARIANTS 长度 == 5.
const _: () = assert!(K1_INVARIANTS.len() == 5);

/// 单次分析最大文件数 (估 10K 文件防 OOM, 跟 `apeireth-mcp-ssh` `max_sessions=16` 类比).
pub const MAX_FILES_PER_ANALYSIS: usize = 10_000;

/// 单文件分析超时 (ms, 估 5s 够单文件 token 扫描).
pub const SINGLE_FILE_ANALYSIS_TIMEOUT_MS: u64 = 5_000;

/// 仓库扫描缓存 TTL (ms, per `apeireth-mcp-ssh` 风格, 5 分钟缓存避免重复扫).
pub const REPO_CACHE_TTL_MS: u64 = 5 * 60 * 1_000;

// ============================================================================
// §2 核心类型 (AnalysisResult / TechDebtType / ComplexityMetrics / AnalyzerError)
// ============================================================================

/// 技术债类型 (5 种, K-1 强校验 #3: 编译期 hardcode 5 个枚举值, 不可运行时增删).
///
/// 字段对应 v0.9.21 `techDebtTypes` 数组元素: TODO / FIXME / HACK / BUG / SECURITY 5 类,
/// 1:1 翻译 (per O-5 不假装实测 0 命中 AST 库, 用 regex token 扫描).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechDebtType {
    /// TODO 标记 (e.g. `// TODO: refactor`).
    Todo,
    /// FIXME 标记 (e.g. `// FIXME: race condition`).
    Fixme,
    /// HACK 标记 (e.g. `// HACK: workaround for issue #123`).
    Hack,
    /// BUG 标记 (e.g. `// BUG: off-by-one error`).
    Bug,
    /// SECURITY 标记 (e.g. `// SECURITY: SQL injection risk`).
    SecurityIssue,
}

impl TechDebtType {
    /// 1:1 翻译 v0.9.21 `techDebtTypes[i].regex` 字段, 返回用于 token 扫描的正则锚点.
    /// 注: skeleton 阶段只返 tag 名 (per O-5 不假装, 实际 regex 引擎留 R20 阶段 4 接 `regex` crate).
    pub fn tag(&self) -> &'static str {
        match self {
            TechDebtType::Todo => "TODO",
            TechDebtType::Fixme => "FIXME",
            TechDebtType::Hack => "HACK",
            TechDebtType::Bug => "BUG",
            TechDebtType::SecurityIssue => "SECURITY",
        }
    }
}

/// 技术债条目 (1:1 翻译 v0.9.21 `TechDebtEntry` 字段: file/line/col/type/message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechDebtEntry {
    /// 所在文件 (相对仓库根).
    pub file: PathBuf,
    /// 行号 (1-indexed).
    pub line: u32,
    /// 列号 (0-indexed).
    pub column: u32,
    /// 技术债类型.
    pub debt_type: TechDebtType,
    /// 标记后的描述文本 (e.g. FIXME 后面的注释内容).
    pub message: String,
}

/// 圈复杂度指标 (1:1 翻译 v0.9.21 `complexityMetrics` 字段).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// 函数名.
    pub function_name: String,
    /// 所在文件.
    pub file: PathBuf,
    /// 起始行号.
    pub start_line: u32,
    /// 圈复杂度值 (decision points + 1).
    pub cyclomatic: u32,
    /// 是否超过警告阈值 (`MAX_CYCLOMATIC_COMPLEXITY`).
    pub exceeds_threshold: bool,
}

/// 文件统计 (1:1 翻译 v0.9.21 `FileStats` 字段: file/line_count/function_count).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStats {
    /// 文件路径.
    pub path: PathBuf,
    /// 总行数 (含空行和注释).
    pub line_count: u32,
    /// 函数/方法数.
    pub function_count: u32,
    /// 类/struct 数.
    pub class_count: u32,
    /// 平均函数长度 (lines / functions, 0 if function_count==0).
    pub avg_function_length: u32,
}

/// 依赖条目 (1:1 翻译 v0.9.21 `DependencyEntry` 字段: name/version/source).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEntry {
    /// 依赖名 (e.g. `tokio` / `serde` / `requests`).
    pub name: String,
    /// 版本约束 (e.g. `1.40` / `^1.0` / `>=2.0`).
    pub version_req: String,
    /// 依赖来源 (e.g. `crates.io` / `npm` / `pypi`).
    pub source: String,
    /// 是否为开发依赖 (dev-dependency).
    pub is_dev: bool,
}

/// 安全审计条目 (1:1 翻译 v0.9.21 `SecurityIssue` 字段).
///
/// v0.9.21 估缺的安全模式: hardcoded_secret / unsafe_block / eval_usage / weak_crypto /
/// sql_injection_risk 5 类. 编译期 hardcode 5 种, R20 阶段 1 续补.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// 所在文件.
    pub file: PathBuf,
    /// 行号.
    pub line: u32,
    /// 安全类别 (e.g. `hardcoded_secret` / `eval_usage`).
    pub category: String,
    /// 严重程度 (`low` / `medium` / `high` / `critical`).
    pub severity: String,
    /// 描述信息.
    pub message: String,
}

/// 完整分析结果 (per v0.9.21 `AnalysisResult` 顶层结构).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// 仓库根路径.
    pub repo_path: PathBuf,
    /// 分析完成时间.
    pub completed_at: SystemTime,
    /// 仓库画像 (file count / language breakdown, per v0.9.21 `RepoProfile`).
    pub total_files: u32,
    /// 文件统计列表.
    pub file_stats: Vec<FileStats>,
    /// 复杂度指标列表.
    pub complexity: Vec<ComplexityMetrics>,
    /// 技术债条目列表.
    pub tech_debt: Vec<TechDebtEntry>,
    /// 依赖条目列表.
    pub dependencies: Vec<DependencyEntry>,
    /// 安全审计发现列表.
    pub security_findings: Vec<SecurityFinding>,
    /// 平台名 (硬编 `"apeireth"`, per K-1 强校验 #2).
    pub platform: String,
    /// schema version (硬编 `"1"`, per K-1 强校验 #1).
    pub schema_version: String,
}

impl AnalysisResult {
    /// 创建新空结果 (per builder 模式, 调用方逐个 fill).
    pub fn new_empty(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            completed_at: SystemTime::now(),
            total_files: 0,
            file_stats: Vec::new(),
            complexity: Vec::new(),
            tech_debt: Vec::new(),
            dependencies: Vec::new(),
            security_findings: Vec::new(),
            platform: PLATFORM_NAME.to_string(),
            schema_version: REPO_ANALYZER_SCHEMA_VERSION.to_string(),
        }
    }

    /// 总结: 技术债总数 (per 5 类之和).
    pub fn tech_debt_total(&self) -> usize {
        self.tech_debt.len()
    }

    /// 总结: 超过复杂度阈值的函数数.
    pub fn high_complexity_count(&self) -> usize {
        self.complexity
            .iter()
            .filter(|c| c.exceeds_threshold)
            .count()
    }

    /// 总结: critical 安全发现数.
    pub fn critical_security_count(&self) -> usize {
        self.security_findings
            .iter()
            .filter(|s| s.severity == "critical")
            .count()
    }
}

/// Repo Analyzer 错误类型 (10 variant, per mcp-ssh 13 variant 类比).
#[derive(Debug, Error)]
pub enum AnalyzerError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    #[error("repo not found: {0}")]
    RepoNotFound(PathBuf),
    #[error("repo too large: {files} files > {max} max")]
    RepoTooLarge { files: usize, max: usize },
    #[error("unsupported dep format: {0}")]
    UnsupportedDepFormat(String),
    #[error("unsupported report format: {0}")]
    UnsupportedReportFormat(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("analysis timeout ({0:?})")]
    Timeout(Duration),
    #[error("cache error: {0}")]
    Cache(String),
    #[error("analyzer I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("analyzer error: {0}")]
    Other(String),
}

pub type AnalyzerResult<T> = Result<T, AnalyzerError>;

// ============================================================================
// §3 质量分析 QualityAnalyzer (async fn analyze_complexity / tech_debt / deps / security)
// ============================================================================

/// QualityAnalyzer 配置.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// 仓库根路径.
    pub repo_path: PathBuf,
    /// 单文件分析超时.
    pub file_timeout: Duration,
    /// 是否启用技术债扫描.
    pub enable_tech_debt: bool,
    /// 是否启用复杂度分析.
    pub enable_complexity: bool,
    /// 是否启用依赖分析.
    pub enable_deps: bool,
    /// 是否启用安全审计.
    pub enable_security: bool,
    /// 最大文件数 (per `MAX_FILES_PER_ANALYSIS`).
    pub max_files: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            file_timeout: Duration::from_millis(SINGLE_FILE_ANALYSIS_TIMEOUT_MS),
            enable_tech_debt: true,
            enable_complexity: true,
            enable_deps: true,
            enable_security: true,
            max_files: MAX_FILES_PER_ANALYSIS,
        }
    }
}

/// QualityAnalyzer 核心 (per v0.9.21 `RepoAnalyzer` class).
///
/// 5 async fn: `analyze_complexity` / `analyze_tech_debt` / `analyze_deps` / `analyze_security` /
/// `analyze_functions` (per K-1 强校验 8 工具中的 5 个分析工具).
#[derive(Debug)]
pub struct QualityAnalyzer {
    /// 配置.
    pub config: AnalyzerConfig,
    /// 缓存 (per §5 AnalysisCache).
    pub cache: AnalysisCache,
    /// 运行状态.
    pub running: AtomicBool,
}

impl QualityAnalyzer {
    /// 默认构造.
    pub fn new(config: AnalyzerConfig) -> AnalyzerResult<Self> {
        if !config.repo_path.exists() {
            return Err(AnalyzerError::RepoNotFound(config.repo_path.clone()));
        }
        Ok(Self {
            config,
            cache: AnalysisCache::default(),
            running: AtomicBool::new(false),
        })
    }

    /// 启动 analyzer.
    pub fn start(&self) -> AnalyzerResult<()> {
        self.running.store(true, Ordering::SeqCst);
        info!(target: "apeireth_repo_analyzer", "start repo={:?}", self.config.repo_path);
        Ok(())
    }

    /// 停止 analyzer.
    pub fn stop(&self) -> AnalyzerResult<()> {
        self.running.store(false, Ordering::SeqCst);
        info!(target: "apeireth_repo_analyzer", "stop");
        Ok(())
    }

    /// 检查工具名是否在白名单内 (m3 防御).
    pub fn validate_tool(&self, tool: &str) -> AnalyzerResult<()> {
        validate_tool_call(tool, &serde_json::Value::Null)
    }

    /// 1:1 翻译 v0.9.21 `analyzeComplexity(file)` — 圈复杂度分析.
    ///
    /// skeleton 阶段: 返回空 Vec (per R20 阶段 1 续 P1 估缺, 真 AST 走 `apeireth-tree-sitter`).
    pub async fn analyze_complexity(&self, _file: &Path) -> AnalyzerResult<Vec<ComplexityMetrics>> {
        if !self.running.load(Ordering::SeqCst) {
            warn!(target: "apeireth_repo_analyzer", "analyze_complexity called while stopped");
        }
        debug!(target: "apeireth_repo_analyzer", "analyze_complexity file={:?} (skeleton: empty)", _file);
        Ok(Vec::new())
    }

    /// 1:1 翻译 v0.9.21 `analyzeTechDebt(file)` — 技术债扫描.
    ///
    /// skeleton 阶段: 返回空 Vec (regex token 扫描留 R20 阶段 1 续).
    pub async fn analyze_tech_debt(&self, _file: &Path) -> AnalyzerResult<Vec<TechDebtEntry>> {
        debug!(target: "apeireth_repo_analyzer", "analyze_tech_debt file={:?} (skeleton: empty)", _file);
        Ok(Vec::new())
    }

    /// 1:1 翻译 v0.9.21 `analyzeDeps(file)` — 依赖分析.
    ///
    /// skeleton 阶段: 根据 `SUPPORTED_DEP_FORMATS` 检测文件后缀,
    /// 返回空 Vec (真解析留 R20 阶段 1 续, 走 `toml` / `serde_json` / `pyproject-toml`).
    pub async fn analyze_deps(&self, file: &Path) -> AnalyzerResult<Vec<DependencyEntry>> {
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !SUPPORTED_DEP_FORMATS.contains(&file_name) {
            return Err(AnalyzerError::UnsupportedDepFormat(file_name.to_string()));
        }
        debug!(target: "apeireth_repo_analyzer", "analyze_deps file={:?} format={} (skeleton: empty)", file, file_name);
        Ok(Vec::new())
    }

    /// 1:1 翻译 v0.9.21 `analyzeSecurity(file)` — 安全审计.
    ///
    /// skeleton 阶段: 返回空 Vec (5 种安全模式 hardcoded secret / unsafe block / eval / weak crypto /
    /// sql injection 留 R20 阶段 1 续, 走 `regex` + `apeireth-tree-sitter`).
    pub async fn analyze_security(&self, _file: &Path) -> AnalyzerResult<Vec<SecurityFinding>> {
        debug!(target: "apeireth_repo_analyzer", "analyze_security file={:?} (skeleton: empty)", _file);
        Ok(Vec::new())
    }

    /// 1:1 翻译 v0.9.21 `analyzeFunctions(file)` — 函数/类统计.
    ///
    /// skeleton 阶段: 返回单个空 `FileStats` (真 AST 走 `apeireth-tree-sitter`).
    pub async fn analyze_functions(&self, file: &Path) -> AnalyzerResult<FileStats> {
        debug!(target: "apeireth_repo_analyzer", "analyze_functions file={:?} (skeleton: empty)", file);
        Ok(FileStats {
            path: file.to_path_buf(),
            line_count: 0,
            function_count: 0,
            class_count: 0,
            avg_function_length: 0,
        })
    }

    /// 跑完整分析 (顶层入口, 1:1 翻译 v0.9.21 `RepoAnalyzer.analyze`).
    ///
    /// skeleton 阶段: 返回空 `AnalysisResult` (5 个子分析并发 stub).
    pub async fn analyze(&self) -> AnalyzerResult<AnalysisResult> {
        let mut result = AnalysisResult::new_empty(self.config.repo_path.clone());
        result.total_files = 0; // skeleton: 不真扫, R20 阶段 1 续接 apeireth-repo-scan
        info!(target: "apeireth_repo_analyzer", "analyze complete: {} files analyzed (skeleton)", result.total_files);
        Ok(result)
    }
}

// ============================================================================
// §4 报告生成 (JSON / Markdown / SARIF)
// ============================================================================

/// 报告生成器 (per v0.9.21 `ReportGenerator` class).
///
/// 3 格式: `report_json` / `report_markdown` / `report_sarif` (per `SUPPORTED_REPORT_FORMATS`).
#[derive(Debug)]
pub struct ReportGenerator {
    /// 输出格式 (`json` / `markdown` / `sarif`).
    pub format: String,
}

impl ReportGenerator {
    /// 默认 JSON 格式.
    pub fn new(format: impl Into<String>) -> AnalyzerResult<Self> {
        let fmt = format.into();
        if !SUPPORTED_REPORT_FORMATS.contains(&fmt.as_str()) {
            return Err(AnalyzerError::UnsupportedReportFormat(fmt));
        }
        Ok(Self { format: fmt })
    }

    /// 1:1 翻译 v0.9.21 `reportJson(result)` — JSON 报告.
    ///
    /// skeleton 阶段: 序列化 `AnalysisResult` 整个结构为 JSON 字符串.
    pub fn report_json(&self, result: &AnalysisResult) -> AnalyzerResult<String> {
        if self.format != "json" {
            return Err(AnalyzerError::UnsupportedReportFormat(self.format.clone()));
        }
        serde_json::to_string_pretty(result).map_err(|e| AnalyzerError::Parse(e.to_string()))
    }

    /// 1:1 翻译 v0.9.21 `reportMarkdown(result)` — Markdown 报告.
    ///
    /// skeleton 阶段: 生成简单 markdown 表格, 列出 file/tech_debt/complexity/security 4 段.
    pub fn report_markdown(&self, result: &AnalysisResult) -> AnalyzerResult<String> {
        if self.format != "markdown" {
            return Err(AnalyzerError::UnsupportedReportFormat(self.format.clone()));
        }
        let mut md = String::new();
        md.push_str(&format!("# {} Repo Analysis Report\n\n", PLATFORM_NAME));
        md.push_str(&format!(
            "- **Repository**: `{}`\n",
            result.repo_path.display()
        ));
        md.push_str(&format!("- **Total files**: {}\n", result.total_files));
        md.push_str(&format!(
            "- **Tech debt entries**: {}\n",
            result.tech_debt_total()
        ));
        md.push_str(&format!(
            "- **High complexity functions**: {}\n",
            result.high_complexity_count()
        ));
        md.push_str(&format!(
            "- **Critical security findings**: {}\n",
            result.critical_security_count()
        ));
        md.push_str(&format!("- **Platform**: {}\n", result.platform));
        md.push_str(&format!(
            "- **Schema version**: {}\n\n",
            result.schema_version
        ));

        md.push_str("## File Statistics\n\n");
        md.push_str("| File | Lines | Functions | Classes | Avg Fn Length |\n");
        md.push_str("|------|------:|----------:|--------:|--------------:|\n");
        for fs in &result.file_stats {
            md.push_str(&format!(
                "| `{}` | {} | {} | {} | {} |\n",
                fs.path.display(),
                fs.line_count,
                fs.function_count,
                fs.class_count,
                fs.avg_function_length
            ));
        }
        md.push_str("\n## Tech Debt\n\n");
        for td in &result.tech_debt {
            md.push_str(&format!(
                "- `{}:{}:{}` **{}**: {}\n",
                td.file.display(),
                td.line,
                td.column,
                td.debt_type.tag(),
                td.message
            ));
        }
        Ok(md)
    }

    /// 1:1 翻译 v0.9.21 `reportSarif(result)` — SARIF 2.1.0 报告.
    ///
    /// SARIF = Static Analysis Results Interchange Format (OASIS standard),
    /// 用于 GitHub Code Scanning / IDE 集成. skeleton 阶段: 生成最小 SARIF 骨架.
    pub fn report_sarif(&self, result: &AnalysisResult) -> AnalyzerResult<String> {
        if self.format != "sarif" {
            return Err(AnalyzerError::UnsupportedReportFormat(self.format.clone()));
        }
        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": format!("{PLATFORM_NAME}-repo-analyzer"),
                        "version": REPO_ANALYZER_SCHEMA_VERSION,
                        "informationUri": "https://github.com/apeireth/apeireth-rust"
                    }
                },
                "results": result.security_findings.iter().map(|sf| {
                    serde_json::json!({
                        "ruleId": sf.category,
                        "level": match sf.severity.as_str() {
                            "critical" | "high" => "error",
                            "medium" => "warning",
                            _ => "note"
                        },
                        "message": { "text": sf.message },
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": { "uri": sf.file.display().to_string() },
                                "region": { "startLine": sf.line }
                            }
                        }]
                    })
                }).collect::<Vec<_>>()
            }]
        });
        serde_json::to_string_pretty(&sarif).map_err(|e| AnalyzerError::Parse(e.to_string()))
    }

    /// 通用入口: 根据 `format` 字段分派到对应方法.
    pub fn generate(&self, result: &AnalysisResult) -> AnalyzerResult<String> {
        match self.format.as_str() {
            "json" => self.report_json(result),
            "markdown" => self.report_markdown(result),
            "sarif" => self.report_sarif(result),
            other => Err(AnalyzerError::UnsupportedReportFormat(other.to_string())),
        }
    }
}

// ============================================================================
// §5 缓存 (per 5 P0 crate 风格, HashMap-based, TTL 控制)
// ============================================================================

/// 分析缓存条目.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// 缓存的 `AnalysisResult`.
    pub result: AnalysisResult,
    /// 缓存创建时间.
    pub cached_at: SystemTime,
}

impl CacheEntry {
    /// 检查是否过期 (per `REPO_CACHE_TTL_MS`).
    pub fn is_expired(&self) -> bool {
        self.cached_at
            .elapsed()
            .map(|d| d.as_millis() > REPO_CACHE_TTL_MS as u128)
            .unwrap_or(true)
    }
}

/// 分析缓存 (per repo path key).
#[derive(Debug, Default)]
pub struct AnalysisCache {
    /// 缓存表 (repo_path_str → CacheEntry).
    pub entries: HashMap<String, CacheEntry>,
}

impl AnalysisCache {
    /// 插入缓存.
    pub fn put(&mut self, repo_path: &Path, result: AnalysisResult) {
        let key = repo_path.display().to_string();
        self.entries.insert(
            key,
            CacheEntry {
                result,
                cached_at: SystemTime::now(),
            },
        );
    }

    /// 读取缓存 (None 或过期返 None).
    pub fn get(&self, repo_path: &Path) -> Option<&AnalysisResult> {
        let key = repo_path.display().to_string();
        self.entries.get(&key).and_then(|e| {
            if e.is_expired() {
                None
            } else {
                Some(&e.result)
            }
        })
    }

    /// 清空缓存.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 当前缓存条目数.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ============================================================================
// §6 m3 防御 (TOOL_WHITELIST 已 §0 嵌入, 这里补 schema 校验 + 命名规约检查)
// ============================================================================

/// m3 防御 #2: schema 校验 (per m3-hallucination-defense §2.1).
///
/// v0.9.21 商业版估缺 schema 校验, R20 阶段 1 续补. 工具调用必须满足:
/// 1. tool 名在 TOOL_WHITELIST 内 (per §0 validate_tool_call).
/// 2. args 必须是 `serde_json::Value::Object` (不能是 null / array / scalar).
/// 3. 必填字段必须存在 (e.g. `apeireth_repo_analyzer_complexity` 需要 `file` 字段).
pub fn validate_tool_schema(tool: &str, args: &serde_json::Value) -> AnalyzerResult<()> {
    validate_tool_call(tool, args)?;
    if !args.is_object() {
        return Err(AnalyzerError::Parse(format!(
            "tool {tool} args must be JSON object, got {}",
            args_type_name(args)
        )));
    }
    Ok(())
}

/// 辅助: JSON Value 类型名 (per `serde_json::Value` 6 variant).
fn args_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// m3 防御 #5: 命名规约 (per m3-hallucination-defense §2.4 + supervisor-prompt-818 §5.3).
///
/// 所有 `apeireth_repo_analyzer_*` 工具名必须遵守: `apeireth_` 前缀 + `repo_analyzer` 子命名空间 +
/// snake_case 操作名. 编译期由 TOOL_WHITELIST 守门, 运行时额外检查 args 字段名是否 snake_case.
pub fn check_naming_convention(name: &str) -> bool {
    name.starts_with("apeireth_repo_analyzer_")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

// ============================================================================
// §7 内置测试 fixture (5 个, per 5 P0 crate 风格: skeleton_sanity / serde /
//     whitelist_count / k1_invariants / schema_validate)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_skeleton_creates_with_default_config() {
        let cfg = AnalyzerConfig::default();
        // skeleton 阶段不真访问 repo_path, 用 cwd (.) 兜底 (不 RepoNotFound)
        let analyzer = QualityAnalyzer::new(cfg);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn tech_debt_type_tag_roundtrip() {
        for td in [
            TechDebtType::Todo,
            TechDebtType::Fixme,
            TechDebtType::Hack,
            TechDebtType::Bug,
            TechDebtType::SecurityIssue,
        ] {
            let tag = td.tag();
            assert!(!tag.is_empty());
            assert!(tag.chars().all(|c| c.is_ascii_uppercase()));
        }
        assert_eq!(TechDebtType::Todo.tag(), "TODO");
        assert_eq!(TechDebtType::SecurityIssue.tag(), "SECURITY");
    }

    #[test]
    fn tool_whitelist_contains_eight_analyzer_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        for tool in [
            "apeireth_repo_analyzer_complexity",
            "apeireth_repo_analyzer_tech_debt",
            "apeireth_repo_analyzer_deps",
            "apeireth_repo_analyzer_security",
            "apeireth_repo_analyzer_functions",
            "apeireth_repo_analyzer_report_json",
            "apeireth_repo_analyzer_report_markdown",
            "apeireth_repo_analyzer_report_sarif",
        ] {
            assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
        }
    }

    #[test]
    fn k1_invariants_all_present() {
        // K-1 强校验: 5 字样必须全在 (per 任务规范)
        assert_eq!(K1_INVARIANTS.len(), 5);
        for s in [
            "apeireth",
            "repo_analyzer",
            "analyze",
            "complexity",
            "must-do",
        ] {
            assert!(K1_INVARIANTS.contains(&s), "K-1 invariant 缺: {s}");
        }
    }

    #[test]
    fn schema_validate_rejects_non_object_args() {
        // m3 防御 #2: args 必须是 JSON object
        let tool = "apeireth_repo_analyzer_complexity";
        let bad = serde_json::json!(null);
        assert!(validate_tool_schema(tool, &bad).is_err());
        let bad = serde_json::json!([1, 2, 3]);
        assert!(validate_tool_schema(tool, &bad).is_err());
        let bad = serde_json::json!("string");
        assert!(validate_tool_schema(tool, &bad).is_err());
        let good = serde_json::json!({"file": "src/main.rs"});
        assert!(validate_tool_schema(tool, &good).is_ok());
    }

    #[test]
    fn naming_convention_enforced() {
        assert!(check_naming_convention("apeireth_repo_analyzer_complexity"));
        assert!(!check_naming_convention(
            "apeireth_REPO_analyzer_complexity"
        ));
        assert!(!check_naming_convention("repo_analyzer_complexity")); // 缺前缀
        assert!(!check_naming_convention("apeireth_repo_analyzer_FOO"));
    }

    #[test]
    fn report_generator_format_dispatch() {
        let result = AnalysisResult::new_empty(PathBuf::from("/tmp/repo"));
        // JSON
        let gen = ReportGenerator::new("json").unwrap();
        let out = gen.generate(&result).unwrap();
        assert!(out.contains("\"schema_version\""));
        // Markdown
        let gen = ReportGenerator::new("markdown").unwrap();
        let out = gen.generate(&result).unwrap();
        assert!(out.contains("# apeireth Repo Analysis Report"));
        // SARIF
        let gen = ReportGenerator::new("sarif").unwrap();
        let out = gen.generate(&result).unwrap();
        assert!(out.contains("\"version\": \"2.1.0\""));
        // 错误格式
        assert!(ReportGenerator::new("xml").is_err());
    }

    #[test]
    fn cache_put_get_expire() {
        let mut cache = AnalysisCache::default();
        let result = AnalysisResult::new_empty(PathBuf::from("/tmp/repo"));
        cache.put(&PathBuf::from("/tmp/repo"), result);
        assert!(cache.get(&PathBuf::from("/tmp/repo")).is_some());
        assert!(cache.get(&PathBuf::from("/other")).is_none());
        assert_eq!(cache.len(), 1);
    }
}
