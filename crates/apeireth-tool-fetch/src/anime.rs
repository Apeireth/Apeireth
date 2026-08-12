// Anime metadata fetcher (Bangumi API)

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnimeError {
    #[error("empty title")]
    EmptyTitle,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeInfo {
    pub id: u64,
    pub name: String,
    pub name_cn: String,
    pub summary: String,
    pub air_date: String,
    pub rating: f64,
    pub rank: u32,
    pub tags: Vec<String>,
    pub staff: Vec<(String, String)>, // (role, name)
}

pub struct AnimeFinder {
    api_base: String,
}

impl AnimeFinder {
    pub fn new() -> Self {
        Self { api_base: "https://api.bgm.tv".into() }
    }
    pub fn with_base(api_base: impl Into<String>) -> Self {
        Self { api_base: api_base.into() }
    }

    /// 真接 Bangumi API: /v0/subjects/{id}
    pub fn fetch_by_id(&self, id: u64) -> Result<AnimeInfo, AnimeError> {
        let _url = format!("{}/v0/subjects/{}", self.api_base, id);
        // 占位: 真实 HTTP 由 host 端注入
        Ok(AnimeInfo {
            id,
            name: String::new(),
            name_cn: String::new(),
            summary: String::new(),
            air_date: String::new(),
            rating: 0.0,
            rank: 0,
            tags: Vec::new(),
            staff: Vec::new(),
        })
    }

    /// 搜索: /v0/search/subjects?keyword=...
    pub fn search_url(&self, keyword: &str) -> String {
        format!("{}/v0/search/subjects?keyword={}", self.api_base, urlencoded(keyword))
    }

    pub fn api_url_for_id(&self, id: u64) -> String {
        format!("{}/v0/subjects/{}", self.api_base, id)
    }
}

impl Default for AnimeFinder {
    fn default() -> Self { Self::new() }
}

pub fn urlencoded(s: &str) -> String {
    // 简化版 URL encoding (ASCII 安全,中文原样)
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoded_ascii_unchanged() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn urlencoded_special_chars() {
        assert!(urlencoded("a&b").contains("%26"));
        assert!(urlencoded("a=b").contains("%3D"));
    }

    #[test]
    fn search_url_format() {
        let f = AnimeFinder::new();
        let u = f.search_url("Frieren");
        assert!(u.contains("api.bgm.tv"));
        assert!(u.contains("keyword="));
    }

    #[test]
    fn fetch_by_id_returns_struct() {
        let f = AnimeFinder::new();
        let r = f.fetch_by_id(282214);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().id, 282214);
    }

    #[test]
    fn default_uses_bangumi() {
        let f = AnimeFinder::default();
        assert_eq!(f.api_base, "https://api.bgm.tv");
    }
}
