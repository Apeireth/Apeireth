//! Code content search: regex + Aho-Corasick multi-pattern.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use aho_corasick::AhoCorasick;
use regex::Regex;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("regex: `{0}`")]
    Regex(#[from] regex::Error),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("aho-corasick: `{0}`")]
    AhoCorasick(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// Plain string search (substring match)
    Literal,
    /// Regular expression (regex syntax)
    Regex,
    /// Multi-pattern Aho-Corasick (OR of multiple literals)
    MultiPattern,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub word_boundary: bool,
    pub max_results: usize,
    /// Lines of context to include before/after match
    pub context_lines: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            word_boundary: false,
            max_results: 1000,
            context_lines: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
    /// 0-indexed match position in the line
    pub match_start: usize,
    pub match_end: usize,
}

pub struct CodeSearcher;

impl CodeSearcher {
    pub fn new() -> Self {
        Self
    }

    /// Search a single file's contents.
    pub fn search_file(
        &self,
        path: &Path,
        kind: SearchKind,
        pattern: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchMatch>, SearchError> {
        let content = std::fs::read_to_string(path)?;
        let file = path.to_string_lossy().to_string();
        Ok(match kind {
            SearchKind::Literal => self.search_literal(&content, &file, pattern, options)?,
            SearchKind::Regex => self.search_regex(&content, &file, pattern, options)?,
            SearchKind::MultiPattern => {
                let patterns: Vec<&str> = pattern.split('|').collect();
                self.search_multi(&content, &file, &patterns, options)?
            }
        })
    }

    /// Literal substring search.
    pub fn search_literal(
        &self,
        content: &str,
        file: &str,
        pattern: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchMatch>, SearchError> {
        let mut matches = Vec::new();
        let search_content = if options.case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };
        let search_pattern = if options.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        for (line_idx, line) in content.lines().enumerate() {
            let search_line = if options.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_pattern) {
                let abs = start + pos;
                if options.word_boundary
                    && !is_word_boundary(content, line_idx, abs, search_pattern.len())
                {
                    start = abs + 1;
                    continue;
                }
                matches.push(SearchMatch {
                    file: file.to_string(),
                    line: line_idx + 1,
                    column: abs + 1,
                    text: line.to_string(),
                    match_start: abs,
                    match_end: abs + search_pattern.len(),
                });
                if matches.len() >= options.max_results {
                    return Ok(matches);
                }
                start = abs + search_pattern.len().max(1);
            }
            let _ = search_content;
        }
        Ok(matches)
    }

    /// Regex search.
    pub fn search_regex(
        &self,
        content: &str,
        file: &str,
        pattern: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchMatch>, SearchError> {
        let mut builder = Regex::new(pattern)?;
        if !options.case_sensitive {
            builder = Regex::new(&format!("(?i){}", pattern))?;
        }
        let re = builder;
        let mut matches = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            for m in re.find_iter(line) {
                let start = m.start();
                let end = m.end();
                if options.word_boundary && !is_word_boundary(content, line_idx, start, end - start)
                {
                    continue;
                }
                matches.push(SearchMatch {
                    file: file.to_string(),
                    line: line_idx + 1,
                    column: start + 1,
                    text: line.to_string(),
                    match_start: start,
                    match_end: end,
                });
                if matches.len() >= options.max_results {
                    return Ok(matches);
                }
            }
        }
        Ok(matches)
    }

    /// Multi-pattern Aho-Corasick (OR of literals).
    pub fn search_multi(
        &self,
        content: &str,
        file: &str,
        patterns: &[&str],
        options: &SearchOptions,
    ) -> Result<Vec<SearchMatch>, SearchError> {
        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(!options.case_sensitive)
            .build(patterns)
            .map_err(|e| SearchError::AhoCorasick(e.to_string()))?;
        let mut matches = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            for m in ac.find_iter(line) {
                let pat = patterns[m.pattern().as_usize()];
                let start = m.start();
                let end = m.end();
                if options.word_boundary && !is_word_boundary(content, line_idx, start, end - start)
                {
                    continue;
                }
                matches.push(SearchMatch {
                    file: file.to_string(),
                    line: line_idx + 1,
                    column: start + 1,
                    text: line.to_string(),
                    match_start: start,
                    match_end: end,
                });
                matches.last_mut().unwrap().text = line.to_string();
                let _ = pat;
                if matches.len() >= options.max_results {
                    return Ok(matches);
                }
            }
        }
        Ok(matches)
    }
}

fn is_word_boundary(content: &str, _line_idx: usize, start: usize, len: usize) -> bool {
    let before = content[..start].chars().last();
    let after = content[start + len..].chars().next();
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    let before_ok = before.map(|c| !is_word_char(c)).unwrap_or(true);
    let after_ok = after.map(|c| !is_word_char(c)).unwrap_or(true);
    before_ok && after_ok
}

impl Default for CodeSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn literal_search_finds_matches() {
        let content = "fn hello() {}\nfn world() {}\n";
        let s = CodeSearcher::new();
        let m = s
            .search_literal(content, "test.rs", "hello", &SearchOptions::default())
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].line, 1);
    }

    #[test]
    fn case_insensitive_literal() {
        let content = "Hello World\nHELLO AGAIN\n";
        let s = CodeSearcher::new();
        let opts = SearchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let m = s
            .search_literal(content, "test.rs", "hello", &opts)
            .unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn regex_search() {
        let content = "let x = 1;\nlet y = 2;\nconst z = 3;\n";
        let s = CodeSearcher::new();
        let m = s
            .search_regex(content, "test.rs", r"^let\s+\w+", &SearchOptions::default())
            .unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn multi_pattern_search() {
        let content = "TODO: fix this\nFIXME: and this\nNOTE: also this\n";
        let s = CodeSearcher::new();
        let m = s
            .search_multi(
                content,
                "test.rs",
                &["TODO", "FIXME"],
                &SearchOptions::default(),
            )
            .unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn word_boundary() {
        let content = "fn helper() {}\nhelpers = [];\n";
        let s = CodeSearcher::new();
        let opts = SearchOptions {
            word_boundary: true,
            ..Default::default()
        };
        let m = s
            .search_literal(content, "test.rs", "helper", &opts)
            .unwrap();
        assert_eq!(m.len(), 1, "should match `helper` but not `helpers`");
    }

    #[test]
    fn max_results_caps() {
        let content = "match\n".repeat(100);
        let s = CodeSearcher::new();
        let opts = SearchOptions {
            max_results: 5,
            ..Default::default()
        };
        let m = s
            .search_literal(&content, "test.rs", "match", &opts)
            .unwrap();
        assert_eq!(m.len(), 5);
    }

    #[test]
    fn search_file_real() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("test.rs");
        fs::write(&p, "fn main() {}\nfn helper() {}\n").unwrap();
        let s = CodeSearcher::new();
        let m = s
            .search_file(&p, SearchKind::Literal, "fn", &SearchOptions::default())
            .unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn invalid_regex_errors() {
        let content = "test";
        let s = CodeSearcher::new();
        let r = s.search_regex(content, "x.rs", "(", &SearchOptions::default());
        assert!(r.is_err());
    }
}
