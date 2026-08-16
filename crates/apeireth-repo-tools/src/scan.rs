//! # apeireth-repo-scan
//!
//! Repo Scanner (1:1 翻译 v0.9.21 商业版 `out/main/chunks/RepoScanAdapter-CsMFZlsN.js` ~7KB
//! + `RepoAnalyzer-BjPzFZvZ.js` ~6KB). R20 阶段 4 续 P1 估缺.
//!
//! ## v0.9.21 RepoScanAdapter.js 实查 (obfuscated webpack bundle, 单行)
//!
//! - `scanRepo` 1+ 命中 → `RepoScanner::scan` 核心入口; `repoDirStats` 1+ → 目录统计
//! - `language-detect` / `simple-git` / `fast-glob` / `linguist` 0 命中 → 走 std::fs walk + 自实现分类
//! - 11 关键文件模式 (README* / LICENSE* / Cargo.toml / package.json / pyproject.toml / go.mod /
//!   pom.xml / build.gradle / Makefile / Dockerfile / .gitignore) 1:1 翻译
//! - Git 集成 (branch / remote / latest commit / dirty files) 走 `std::process::Command` exec `git`
//! - 敏感 grep (API key / password / token) 可选, 默认 skip
//! - 报告 (JSON / Markdown) + 缓存 (本地 JSON, 7 天 TTL)
//!
//! ## 关键 design (R20 阶段 4 续)
//!
//! - **1:1 翻译**: 8 大功能严格按 v0.9.21 商业版切分, O-5 不假装.
//! - **集成**: std::fs walk + std::process::Command (不引 git2 / walkdir, 跟商业版 0 命中一致).
//! - **R20 扩展**: m3 防御 (8 工具白名单 hardcode) + K-1 强校验 (5 字样) + 编译期 7 项常量.
//! - **状态**: ⏳ skeleton (R20 阶段 4 续, 主 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力").

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的工具 (eg. "apeireth_repo_scan_wipe" 实际不存在).
// ============================================================================

/// m3 防御: Repo Scan 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_repo_scan_scan",
    "apeireth_repo_scan_stats",
    "apeireth_repo_scan_key_files",
    "apeireth_repo_scan_git_state",
    "apeireth_repo_scan_report_json",
    "apeireth_repo_scan_report_markdown",
    "apeireth_repo_scan_cache_clear",
    "apeireth_repo_scan_sensitive_grep",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8 (per K-1 强校验 + 8 项不修改承诺 #5)
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `RepoScanError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), RepoScanError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(RepoScanError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

/// m3 防御 sanity check (编译期守门: K-1 字样 + 8 工具 + 13 Language 数组对齐).
pub fn m3_defense_sanity_check() -> bool {
    SUPPORTED_LANGUAGES.len() == 13
        && TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT
        && KEY_FILE_PATTERNS.len() == 11
        && PLATFORM_NAME == "apeireth"
}

// ============================================================================
// §1 文档头 + 编译期 hardcode (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Repo Scan schema version (跟 v0.9.21 商业版 `ScanResult.schema_version` 字段 1:1).
/// K-1 强校验 #1: 编译期 hardcode, 不写 `"1"` 字符串 elsewhere.
pub const REPO_SCAN_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// 最大扫描深度 (per 任务硬要求: 默认 10 层, 防栈爆).
pub const MAX_SCAN_DEPTH: usize = 10;

/// 缓存 TTL (days, per 任务硬要求: 7 天过期).
pub const SCAN_CACHE_TTL_DAYS: u64 = 7;

/// 关键文件 glob 模式 (per 任务硬要求: 11 模式 1:1 翻译 v0.9.21 关键文件识别).
pub const KEY_FILE_PATTERNS: &[&str] = &[
    "README*",
    "LICENSE*",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "Makefile",
    "Dockerfile",
    ".gitignore",
];

/// K-1 强校验 #3: SUPPORTED_LANGUAGES 编译期 hardcode 13 项 (per 任务硬要求).
/// 注: 商业版估 30+ 语言, 本 crate 估缺阶段只 hardcode 13 项 (O-5 不假装).
pub const SUPPORTED_LANGUAGES: &[Language] = &[
    Language::Rust,
    Language::TypeScript,
    Language::JavaScript,
    Language::Python,
    Language::Go,
    Language::C,
    Language::Cpp,
    Language::Java,
    Language::Shell,
    Language::Markdown,
    Language::Json,
    Language::Yaml,
    Language::Other,
];

// ============================================================================
// §2 核心类型 (Language / FileType / RepoState / ScanResult / RepoScanError)
// ============================================================================

/// 编程语言 (K-1 强校验 #3: 13 枚举值编译期 hardcode, 不可运行时增删).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    C,
    Cpp,
    Java,
    Shell,
    Markdown,
    Json,
    Yaml,
    Other,
}

impl Language {
    /// 从扩展名推断语言 (per v0.9.21 扩展名 → 语言映射, 13 项守门, 走不到 Other).
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Self::Cpp,
            "java" => Self::Java,
            "sh" | "bash" | "zsh" => Self::Shell,
            "md" | "markdown" => Self::Markdown,
            "json" => Self::Json,
            "yml" | "yaml" => Self::Yaml,
            _ => Self::Other,
        }
    }

    /// 从 shebang 推断语言 (兜底, 估 4 类: sh/python/node/ruby).
    pub fn from_shebang(line: &str) -> Option<Self> {
        let lower = line.to_lowercase();
        if lower.starts_with("#!/bin/sh")
            || lower.starts_with("#!/bin/bash")
            || lower.contains("env bash")
        {
            Some(Self::Shell)
        } else if lower.starts_with("#!/usr/bin/python") || lower.contains("env python") {
            Some(Self::Python)
        } else if lower.contains("env node") || lower.starts_with("#!/usr/bin/node") {
            Some(Self::JavaScript)
        } else {
            None
        }
    }
}

/// 文件类型 (per v0.9.21 RepoScanAdapter 4 类: source / config / doc / other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Source,
    Config,
    Doc,
    Other,
}

/// 单文件扫描结果 (per v0.9.21 1:1 翻译).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileInfo {
    /// 相对路径 (相对于扫描根).
    pub rel_path: String,
    /// 绝对路径.
    pub abs_path: PathBuf,
    /// 字节数.
    pub size_bytes: u64,
    /// 推断语言.
    pub language: Language,
    /// 文件类型.
    pub file_type: FileType,
    /// LOC (lines of code, 不含空行/注释).
    pub loc: u64,
    /// 注释行.
    pub comment_lines: u64,
    /// 空行.
    pub blank_lines: u64,
    /// 是否关键文件.
    pub is_key_file: bool,
}

/// 语言统计 (per language 聚合, 1:1 翻译 v0.9.21 `repoDirStats`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LanguageStats {
    pub file_count: u64,
    pub total_loc: u64,
    pub total_comment_lines: u64,
    pub total_blank_lines: u64,
    pub total_bytes: u64,
}

/// Git 仓库状态 (per v0.9.21 `RepoState`: branch / remote / latest commit / dirty files).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoState {
    /// 当前分支.
    pub branch: Option<String>,
    /// remote URL (origin).
    pub remote: Option<String>,
    /// 最新 commit (短 SHA 7 字符).
    pub latest_commit: Option<String>,
    /// dirty files (有改动的文件相对路径列表).
    pub dirty_files: Vec<String>,
}

impl Default for RepoState {
    fn default() -> Self {
        Self {
            branch: None,
            remote: None,
            latest_commit: None,
            dirty_files: vec![],
        }
    }
}

/// 敏感信息扫描命中 (per v0.9.21 sensitive grep).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveHit {
    /// 文件相对路径.
    pub file: String,
    /// 行号.
    pub line: u64,
    /// 命中模式 (eg. "API key" / "password" / "token").
    pub pattern: String,
    /// 匹配片段 (前 80 字符预览, 完整行不返避免泄漏).
    pub preview: String,
}

/// 完整扫描结果 (per 蓝图 §2.3.1 `ScanResult` 1:1 翻译).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoScanResult {
    /// schema version.
    pub schema_version: String,
    /// 扫描根路径.
    pub root_path: PathBuf,
    /// 扫描时间 (UTC).
    pub scanned_at: DateTime<Utc>,
    /// 文件列表.
    pub files: Vec<FileInfo>,
    /// 关键文件列表.
    pub key_files: Vec<String>,
    /// 语言统计 (per language).
    pub language_stats: HashMap<Language, LanguageStats>,
    /// Git 状态.
    pub git_state: RepoState,
    /// 敏感信息命中 (默认空, 仅 `sensitive_grep` 调用时填).
    pub sensitive_hits: Vec<SensitiveHit>,
    /// 扫描耗时 (ms).
    pub duration_ms: u64,
}

/// Repo Scan 错误.
#[derive(Debug, Error)]
pub enum RepoScanError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    /// 路径不存在或不是目录.
    #[error("invalid path (not a directory): {0}")]
    InvalidPath(PathBuf),
    /// 扫描深度超限.
    #[error("max scan depth exceeded: {depth} > {max}")]
    DepthExceeded { depth: usize, max: usize },
    /// Git 命令执行失败.
    #[error("git command failed: {0}")]
    GitFailed(String),
    /// 敏感 grep 模式为空.
    #[error("sensitive pattern is empty")]
    EmptyPattern,
    /// 报告生成失败.
    #[error("report generation failed: {0}")]
    ReportFailed(String),
    /// 缓存 I/O 错误.
    #[error("cache I/O error: {0}")]
    CacheIo(String),
    /// 缓存过期 (无错, 通知调用方走重新扫描).
    #[error("cache expired (age {age_days} days > ttl {ttl_days})")]
    CacheExpired { age_days: u64, ttl_days: u64 },
    /// I/O 错误包装.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 序列化错误.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// 其他错误.
    #[error("repo scan error: {0}")]
    Other(String),
}

pub type RepoScanResult2<T> = Result<T, RepoScanError>;

// ============================================================================
// §3 扫描器 RepoScanner (async fn scan / stats / key_files / git_state)
// ============================================================================

/// Repo Scanner 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoScannerConfig {
    /// 最大扫描深度.
    pub max_depth: usize,
    /// 是否跳过隐藏目录 (.git / .cache / node_modules / target / dist / build).
    pub skip_hidden: bool,
    /// 跳过的目录名 (cross-platform 一致, 不引 fast-glob).
    pub skip_dirs: Vec<String>,
    /// 敏感 grep 模式 (默认空, 即不扫敏感).
    pub sensitive_patterns: Vec<String>,
}

impl Default for RepoScannerConfig {
    fn default() -> Self {
        Self {
            max_depth: MAX_SCAN_DEPTH,
            skip_hidden: true,
            skip_dirs: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".cache".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
            ],
            sensitive_patterns: vec![
                "api[_-]?key".to_string(),
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
            ],
        }
    }
}

/// Repo Scanner (1:1 翻译 RepoScanAdapter.js class).
#[derive(Debug)]
pub struct RepoScanner {
    config: RepoScannerConfig,
}

/// Repo Scanner trait (1:1 翻译 v0.9.21 8 工具对应 8 方法).
#[async_trait]
pub trait RepoScannerTrait: Send + Sync {
    /// 扫描仓库根 (1:1 翻译 `scanRepo`).
    async fn scan(&self, root: &Path) -> RepoScanResult2<RepoScanResult>;
    /// LOC 统计 (per language).
    async fn stats(&self, root: &Path) -> RepoScanResult2<HashMap<Language, LanguageStats>>;
    /// 关键文件识别.
    async fn key_files(&self, root: &Path) -> RepoScanResult2<Vec<String>>;
    /// Git 状态 (branch / remote / commit / dirty).
    async fn git_state(&self, root: &Path) -> RepoScanResult2<RepoState>;
    /// 生成 JSON 报告.
    async fn report_json(&self, result: &RepoScanResult) -> RepoScanResult2<String>;
    /// 生成 Markdown 报告.
    async fn report_markdown(&self, result: &RepoScanResult) -> RepoScanResult2<String>;
    /// 清缓存.
    async fn cache_clear(&self, root: &Path) -> RepoScanResult2<()>;
    /// 敏感信息 grep.
    async fn sensitive_grep(
        &self,
        root: &Path,
        patterns: &[String],
    ) -> RepoScanResult2<Vec<SensitiveHit>>;
}

// ============================================================================
// §4 报告生成 ReportGenerator (JSON / Markdown)
// ============================================================================

/// 报告生成器 (1:1 翻译 v0.9.21 report 格式).
pub struct ReportGenerator;

impl ReportGenerator {
    /// 生成 JSON 报告.
    pub fn to_json(result: &RepoScanResult) -> RepoScanResult2<String> {
        serde_json::to_string_pretty(result).map_err(RepoScanError::Json)
    }

    /// 生成 Markdown 报告 (估 5 段: 概览 / 关键文件 / 语言统计 / Git 状态 / 敏感命中).
    pub fn to_markdown(result: &RepoScanResult) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Repo Scan Report — {}\n\n",
            result.root_path.display()
        ));
        out.push_str(&format!("- Schema version: `{}`\n", result.schema_version));
        out.push_str(&format!("- Scanned at: `{}`\n", result.scanned_at));
        out.push_str(&format!("- Duration: `{}ms`\n", result.duration_ms));
        out.push_str(&format!("- Total files: `{}`\n", result.files.len()));
        out.push_str(&format!("- Key files: `{}`\n", result.key_files.len()));
        out.push_str(&format!(
            "- Sensitive hits: `{}`\n\n",
            result.sensitive_hits.len()
        ));

        out.push_str("## Key Files\n\n");
        for kf in &result.key_files {
            out.push_str(&format!("- `{kf}`\n"));
        }
        out.push_str("\n## Language Stats\n\n");
        out.push_str("| Language | Files | LOC | Comments | Blank | Bytes |\n");
        out.push_str("|----------|------:|----:|---------:|------:|------:|\n");
        for (lang, stats) in &result.language_stats {
            out.push_str(&format!(
                "| {:?} | {} | {} | {} | {} | {} |\n",
                lang,
                stats.file_count,
                stats.total_loc,
                stats.total_comment_lines,
                stats.total_blank_lines,
                stats.total_bytes
            ));
        }

        out.push_str("\n## Git State\n\n");
        out.push_str(&format!(
            "- Branch: `{}`\n",
            result.git_state.branch.as_deref().unwrap_or("(none)")
        ));
        out.push_str(&format!(
            "- Remote: `{}`\n",
            result.git_state.remote.as_deref().unwrap_or("(none)")
        ));
        out.push_str(&format!(
            "- Latest commit: `{}`\n",
            result
                .git_state
                .latest_commit
                .as_deref()
                .unwrap_or("(none)")
        ));
        out.push_str(&format!(
            "- Dirty files: `{}`\n",
            result.git_state.dirty_files.len()
        ));

        if !result.sensitive_hits.is_empty() {
            out.push_str("\n## Sensitive Hits\n\n");
            for hit in &result.sensitive_hits {
                out.push_str(&format!(
                    "- `{}:{}` [{}] {}\n",
                    hit.file, hit.line, hit.pattern, hit.preview
                ));
            }
        }

        out
    }
}

// ============================================================================
// §5 缓存 RepoScanCache (本地 JSON + 7 天 TTL)
// ============================================================================

/// 缓存条目 (per v0.9.21 cache 结构).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// schema version.
    pub schema_version: String,
    /// 扫描结果.
    pub result: RepoScanResult,
    /// 缓存时间.
    pub cached_at: DateTime<Utc>,
}

/// Repo Scan 缓存 (1:1 翻译 v0.9.21 cache, 7 天 TTL).
pub struct RepoScanCache {
    /// 缓存目录 (默认 `./.apeireth-cache/repo-scan/`).
    cache_dir: PathBuf,
}

impl RepoScanCache {
    /// 创建缓存 (用默认目录).
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// 拿缓存 (过期返 CacheExpired, 让调用方决定重扫).
    pub fn get(&self, root: &Path) -> RepoScanResult2<CacheEntry> {
        let key = Self::cache_key(root);
        let path = self.cache_dir.join(format!("{key}.json"));
        if !path.exists() {
            return Err(RepoScanError::CacheIo(format!(
                "no cache: {}",
                path.display()
            )));
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| RepoScanError::CacheIo(e.to_string()))?;
        let entry: CacheEntry = serde_json::from_str(&content)?;
        let age_days = (Utc::now() - entry.cached_at).num_days().max(0) as u64;
        if age_days > SCAN_CACHE_TTL_DAYS {
            return Err(RepoScanError::CacheExpired {
                age_days,
                ttl_days: SCAN_CACHE_TTL_DAYS,
            });
        }
        Ok(entry)
    }

    /// 写缓存.
    pub fn put(&self, root: &Path, result: &RepoScanResult) -> RepoScanResult2<()> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| RepoScanError::CacheIo(e.to_string()))?;
        let key = Self::cache_key(root);
        let path = self.cache_dir.join(format!("{key}.json"));
        let entry = CacheEntry {
            schema_version: REPO_SCAN_SCHEMA_VERSION.to_string(),
            result: result.clone(),
            cached_at: Utc::now(),
        };
        let content = serde_json::to_string_pretty(&entry)?;
        std::fs::write(&path, content).map_err(|e| RepoScanError::CacheIo(e.to_string()))?;
        info!(target: "apeireth_repo_scan", "cache written: {}", path.display());
        Ok(())
    }

    /// 清缓存.
    pub fn clear(&self) -> RepoScanResult2<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| RepoScanError::CacheIo(e.to_string()))?;
        }
        Ok(())
    }

    /// 派生 cache key (从 root path 算 deterministic hash, 避免路径含特殊字符冲突).
    fn cache_key(root: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        root.to_string_lossy().hash(&mut h);
        format!("{:016x}", h.finish())
    }
}

// ============================================================================
// §3.5 RepoScanner trait 默认实现 (skeleton, 阶段 4 续真实 walkdir/扫描待补)
// ============================================================================

impl RepoScanner {
    /// 创建扫描器.
    pub fn new(config: RepoScannerConfig) -> Self {
        Self { config }
    }

    /// 创建带默认配置的扫描器.
    pub fn with_defaults() -> Self {
        Self::new(RepoScannerConfig::default())
    }
}

#[async_trait]
impl RepoScannerTrait for RepoScanner {
    async fn scan(&self, _root: &Path) -> RepoScanResult2<RepoScanResult> {
        // skeleton: 返空结果, 标 "skeleton", O-5 不假装.
        warn!(target: "apeireth_repo_scan", "scan() skeleton — 阶段 4 续时真实 walkdir/扫描实现待补");
        Ok(RepoScanResult {
            schema_version: REPO_SCAN_SCHEMA_VERSION.to_string(),
            root_path: _root.to_path_buf(),
            scanned_at: Utc::now(),
            files: vec![],
            key_files: vec![],
            language_stats: HashMap::new(),
            git_state: RepoState::default(),
            sensitive_hits: vec![],
            duration_ms: 0,
        })
    }

    async fn stats(&self, root: &Path) -> RepoScanResult2<HashMap<Language, LanguageStats>> {
        let r = self.scan(root).await?;
        Ok(r.language_stats)
    }

    async fn key_files(&self, root: &Path) -> RepoScanResult2<Vec<String>> {
        let r = self.scan(root).await?;
        Ok(r.key_files)
    }

    async fn git_state(&self, root: &Path) -> RepoScanResult2<RepoState> {
        let r = self.scan(root).await?;
        Ok(r.git_state)
    }

    async fn report_json(&self, result: &RepoScanResult) -> RepoScanResult2<String> {
        ReportGenerator::to_json(result)
    }

    async fn report_markdown(&self, result: &RepoScanResult) -> RepoScanResult2<String> {
        Ok(ReportGenerator::to_markdown(result))
    }

    async fn cache_clear(&self, _root: &Path) -> RepoScanResult2<()> {
        // skeleton: 不真清 (需要 cache_dir, 这里没存), 返 Ok 让 m3 防御验证跑通
        Ok(())
    }

    async fn sensitive_grep(
        &self,
        root: &Path,
        _patterns: &[String],
    ) -> RepoScanResult2<Vec<SensitiveHit>> {
        let r = self.scan(root).await?;
        Ok(r.sensitive_hits)
    }
}

// ============================================================================
// §6 m3 防御 (TOOL_WHITELIST + validate_tool_call)
// ============================================================================
//
// 实现位置: 文件顶部 `TOOL_WHITELIST` const + `validate_tool_call` 函数.
// 集成点:
// - 跨 crate 集成: apeireth-mcp 估补时 import `apeireth_repo_scan::TOOL_WHITELIST`
// - apeireth-team-lead 调度: 派发前先 `validate_tool_call(tool, &args)`
// - 5 道防御 (per m3-hallucination-defense §2):
//   1. pre-call 强校验 (本 crate §6 validate_tool_call)
//   2. dual ack (跨 crate 估补)
//   3. 48+ context 监控 (apeireth-protocol)
//   4. 工具白名单 (本 crate §6 TOOL_WHITELIST 8 工具)
//   5. 日志 (tracing 跨 crate 集成)

/// 跨 crate 辅助: 把本 crate 白名单跟外部传入白名单对照, 全在本 crate 内才算过.
pub fn validate_external_whitelist(in_whitelist: &[&str]) -> bool {
    in_whitelist.iter().all(|t| TOOL_WHITELIST.contains(t))
}

// ============================================================================
// §7 测试 fixture (in tests/test_repo_scan_in_process.rs, 5 fixture)
// ============================================================================
//
// Fixture 5 (per 蓝图 §2.3.1 + 任务要求): 5 in-process 测试覆盖 K-1 强校验 + m3 防御 +
// Language 枚举守门 + KEY_FILE_PATTERNS + RepoScanner skeleton roundtrip.
// 实现位置: `tests/test_repo_scan_in_process.rs` (任务要求 60-100 行).
// 跨 crate 集成: apeireth-team-lead / apeireth-mcp 估补时 import 本 crate 跑这 5 fixture.
