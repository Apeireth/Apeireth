//! R251 -- in-process pure Rust pattern matcher (no ast-grep CLI dep).
//!
//! Provides [`PurePatternSearcher`] implementing the same [`AstSearcher`] trait
//! as [`crate::ast_grep::AstGrepSearcher`] but with 0 external binary dep.
//!
//! **Use case**: agent runs in restricted env (Windows no ast-grep install, container
//!   without apt-get, sandboxed CI). Code-search still works via regex/literal match.
//!
//! **Honest** (O-5):
//! - Not real AST awareness (only matches text patterns, not syntax trees)
//! - But covers ~80% of common searches (literal + regex)
//! - For full AST, fall back to `AstGrepSearcher` when ast-grep binary is installed
//!
//! 0 触碰: new module, `AstSearcher` trait already in `ast_grep.rs`.

#![allow(missing_docs)]

use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};
use std::sync::Arc;

use regex::Regex;
// Deserialize not needed directly (AstGrepMatch has it via ast_grep)
use thiserror::Error;
use walkdir::WalkDir;

use super::ast_grep::{AstGrepError, AstGrepMatch, AstSearcher};
// ============================================================================
// 错误
// ============================================================================

#[derive(Debug, Error)]
pub enum PurePatternError {
    #[error("regex compile failed: {0}")]
    RegexCompile(#[from] regex::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
}

impl From<PurePatternError> for AstGrepError {
    fn from(e: PurePatternError) -> Self {
        match e {
            PurePatternError::Io(io) => AstGrepError::Io(io),
            other => AstGrepError::SpawnFailed(other.to_string()),
        }
    }
}

// ============================================================================
// Pattern type
// ============================================================================

#[derive(Debug, Clone)]
pub enum PurePatternKind {
    Literal(String),
    Regex(Arc<Regex>),
}

#[derive(Debug, Clone)]
pub struct PurePattern {
    pub source: String,
    pub kind: PurePatternKind,
}

impl PurePattern {
    pub fn literal(text: impl Into<String>) -> Self {
        let s: String = text.into();
        Self {
            source: s.clone(),
            kind: PurePatternKind::Literal(s),
        }
    }

    pub fn regex(pat: impl AsRef<str>) -> Result<Self, PurePatternError> {
        let s = pat.as_ref();
        let re = Regex::new(s)?;
        Ok(Self {
            source: s.to_string(),
            kind: PurePatternKind::Regex(Arc::new(re)),
        })
    }

    pub fn auto(pat: impl AsRef<str>) -> Result<Self, PurePatternError> {
        let s = pat.as_ref();
        let has_meta = s.chars().any(|c| {
            matches!(
                c,
                '*' | '+' | '?' | '|' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '\\'
            )
        });
        if has_meta {
            Self::regex(s)
        } else {
            Ok(Self::literal(s))
        }
    }
}
// ============================================================================
// PurePatternSearcher
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct PurePatternSearcher {
    pub extension_filter: Option<String>,
    pub max_file_size: u64,
    pub follow_symlinks: bool,
}

impl PurePatternSearcher {
    pub fn new() -> Self {
        Self {
            extension_filter: None,
            max_file_size: 1_048_576,
            follow_symlinks: false,
        }
    }

    pub fn with_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension_filter = Some(ext.into());
        self
    }

    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    pub fn search_content(content: &str, pattern: &PurePattern) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        match &pattern.kind {
            PurePatternKind::Literal(needle) => {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(needle.as_str()) {
                        out.push((i + 1, line.to_string()));
                    }
                }
            }
            PurePatternKind::Regex(re) => {
                for (i, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        out.push((i + 1, line.to_string()));
                    }
                }
            }
        }
        out
    }

    pub fn search_file(
        &self,
        path: &Path,
        pattern: &PurePattern,
    ) -> Result<Vec<AstGrepMatch>, PurePatternError> {
        let metadata = fs::metadata(path)?;
        if metadata.len() > self.max_file_size {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let line_matches = Self::search_content(&content, pattern);
        Ok(line_matches
            .into_iter()
            .map(|(line, text)| AstGrepMatch {
                file: path.to_path_buf(),
                start_line: line,
                end_line: line,
                text,
                rule_id: None,
            })
            .collect())
    }

    pub fn search_dir(
        &self,
        root: &Path,
        pattern: &PurePattern,
    ) -> Result<Vec<AstGrepMatch>, PurePatternError> {
        let mut results = Vec::new();
        let walker = WalkDir::new(root).follow_links(self.follow_symlinks);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = &self.extension_filter {
                let path_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if path_ext != ext {
                    continue;
                }
            }
            match self.search_file(path, pattern) {
                Ok(matches) => results.extend(matches),
                Err(_) => continue,
            }
        }
        Ok(results)
    }
}

impl AstSearcher for PurePatternSearcher {
    fn search(
        &self,
        root: &Path,
        pattern: &str,
        _lang: Option<&str>,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError> {
        let pat = PurePattern::auto(pattern).map_err(AstGrepError::from)?;
        self.search_dir(root, &pat).map_err(AstGrepError::from)
    }

    fn search_with_rule(
        &self,
        _root: &Path,
        _rule_file: &Path,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError> {
        Err(AstGrepError::SpawnFailed("PurePatternSearcher does not support YAML rules; install ast-grep binary or use a code-level rule parser".to_string()))
    }
}
// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("apeireth_pure_pattern_tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn r251_01_literal_pattern_matches_substring() {
        let p = PurePattern::literal("foo");
        let matches = PurePatternSearcher::search_content("hello foo\nbar foo\nbaz", &p);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].0, 1);
        assert_eq!(matches[1].0, 2);
    }

    #[test]
    fn r251_02_regex_pattern_matches() {
        let p = PurePattern::regex(r"fn\s+(\w+)").unwrap();
        let content = "fn foo() {}\nfn bar() {}\nstruct Baz {}";
        let matches = PurePatternSearcher::search_content(content, &p);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].1, "fn foo() {}");
    }

    #[test]
    fn r251_03_auto_detects_literal_vs_regex() {
        let lit = PurePattern::auto("plain_text").unwrap();
        assert!(matches!(lit.kind, PurePatternKind::Literal(_)));
        let re = PurePattern::auto(r"^fn\s+").unwrap();
        assert!(matches!(re.kind, PurePatternKind::Regex(_)));
    }

    #[test]
    fn r251_04_search_file_returns_ast_grep_match_shaped_results() {
        let path = tmp_file("r251_04.txt", "line 1 hello\nline 2 world\nline 3 hello\n");
        let p = PurePattern::literal("hello");
        let searcher = PurePatternSearcher::new();
        let matches = searcher.search_file(&path, &p).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start_line, 1);
        assert_eq!(matches[0].end_line, 1);
        assert_eq!(matches[1].start_line, 3);
        assert!(matches[0].text.contains("hello"));
    }

    #[test]
    fn r251_05_search_dir_extension_filter() {
        let dir = std::env::temp_dir().join("apeireth_pure_pattern_dir_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.join("b.txt"), "fn b() {}").unwrap();
        fs::write(dir.join("c.rs"), "fn c() {}").unwrap();

        let searcher = PurePatternSearcher::new().with_extension("rs");
        let p = PurePattern::regex(r"fn \w+").unwrap();
        let matches = searcher.search_dir(&dir, &p).unwrap();
        assert_eq!(matches.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn r251_06_search_with_rule_returns_unsupported_error() {
        let s = PurePatternSearcher::new();
        let result = s.search_with_rule(Path::new("."), Path::new("rule.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn r251_07_search_via_ast_searcher_trait() {
        let dir = std::env::temp_dir().join("apeireth_pure_pattern_trait_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "fn hello() {}\nfn world() {}").unwrap();

        let searcher = PurePatternSearcher::new().with_extension("rs");
        let matches = AstSearcher::search(&searcher, &dir, "hello", None).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].text.contains("hello"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn r251_08_max_file_size_skips_large_files() {
        let dir = std::env::temp_dir().join("apeireth_pure_pattern_size_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.txt");
        let content: String = "x".repeat(100);
        fs::write(&path, &content).unwrap();
        let searcher = PurePatternSearcher::new().with_max_file_size(10);
        let p = PurePattern::literal("x");
        let matches = searcher.search_file(&path, &p).unwrap();
        assert_eq!(matches.len(), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
