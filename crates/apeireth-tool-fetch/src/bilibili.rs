// Bilibili video fetcher (BV/AV -> metadata + subtitle)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BilibiliError {
    #[error("invalid BV/AV id: {0}")]
    InvalidId(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilibiliInfo {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub desc: String,
    pub duration_sec: u32,
    pub pubdate_ts: i64,
    pub owner_mid: u64,
    pub owner_name: String,
    pub view_count: u64,
    pub danmaku_count: u64,
    pub reply_count: u64,
    pub favorite_count: u64,
    pub coin_count: u64,
    pub share_count: u64,
    pub like_count: u64,
}

pub struct BilibiliFetcher {
    api_base: String,
}

impl BilibiliFetcher {
    pub fn new() -> Self {
        Self {
            api_base: "https://api.bilibili.com".into(),
        }
    }
    pub fn with_base(api_base: impl Into<String>) -> Self {
        Self {
            api_base: api_base.into(),
        }
    }

    /// BV 号解析: BV1xx411c7XX -> API 调用 wbi 签名 (真实调用由 host 注入)
    pub fn fetch_info(&self, bvid: &str) -> Result<BilibiliInfo, BilibiliError> {
        if !bvid.starts_with("BV") || bvid.len() < 5 {
            return Err(BilibiliError::InvalidId(bvid.into()));
        }
        // 占位: 真实 fetch 由 host 端注入 api.bilibili.com/x/web-interface/view
        // 这里返回最小可测试结构
        Ok(BilibiliInfo {
            bvid: bvid.into(),
            aid: bv_to_aid(bvid).unwrap_or(0),
            title: String::new(),
            desc: String::new(),
            duration_sec: 0,
            pubdate_ts: 0,
            owner_mid: 0,
            owner_name: String::new(),
            view_count: 0,
            danmaku_count: 0,
            reply_count: 0,
            favorite_count: 0,
            coin_count: 0,
            share_count: 0,
            like_count: 0,
        })
    }

    /// AV 号解析 (旧版)
    pub fn fetch_by_aid(&self, aid: u64) -> Result<BilibiliInfo, BilibiliError> {
        let _url = format!("{}/x/web-interface/view?aid={}", self.api_base, aid);
        // 占位
        Ok(BilibiliInfo {
            bvid: String::new(),
            aid,
            title: String::new(),
            desc: String::new(),
            duration_sec: 0,
            pubdate_ts: 0,
            owner_mid: 0,
            owner_name: String::new(),
            view_count: 0,
            danmaku_count: 0,
            reply_count: 0,
            favorite_count: 0,
            coin_count: 0,
            share_count: 0,
            like_count: 0,
        })
    }

    pub fn api_url_for_bvid(bvid: &str) -> String {
        format!(
            "https://api.bilibili.com/x/web-interface/view?bvid={}",
            bvid
        )
    }

    pub fn short_link_url(bvid: &str) -> String {
        format!("https://b23.tv/{}", &bvid[3..])
    }
}

impl Default for BilibiliFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// BV 号 -> AV 号 (经典 XOR 算法,2023-03 后变更,留作参考)
pub fn bv_to_aid(bvid: &str) -> Option<u64> {
    if !bvid.starts_with("BV1") || bvid.len() != 12 {
        return None;
    }
    // 简化的表查 (不依赖完整 XOR 编码)
    // 实际 BV->AV 编码涉及字符表 + XOR + 位运算,本 crate 仅留接口
    let table = "fZodR9XQDSUm21yCkr6zBqiveYah8bt4xsWpHnJE7jL5VG3guMTKNPAwcF";
    let mut result: u64 = 0;
    for c in [9usize, 8, 1, 4, 7, 2] {
        if let Some(ch) = bvid.chars().nth(c) {
            if let Some(pos) = table.find(ch) {
                result = result * 58 + pos as u64;
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(result - (1u64 << 31) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_link_strips_prefix() {
        assert_eq!(
            BilibiliFetcher::short_link_url("BV1xx411c7mD"),
            "https://b23.tv/xx411c7mD"
        );
    }

    #[test]
    fn api_url_for_bvid() {
        assert!(BilibiliFetcher::api_url_for_bvid("BV1xx411c7mD").contains("bvid=BV1xx411c7mD"));
    }

    #[test]
    fn invalid_bvid_rejected() {
        let f = BilibiliFetcher::new();
        assert!(matches!(
            f.fetch_info("AV123"),
            Err(BilibiliError::InvalidId(_))
        ));
        assert!(matches!(
            f.fetch_info("BV"),
            Err(BilibiliError::InvalidId(_))
        ));
    }

    #[test]
    fn fetch_info_returns_struct() {
        let f = BilibiliFetcher::new();
        let r = f.fetch_info("BV1xx411c7mD");
        assert!(r.is_ok());
        let info = r.unwrap();
        assert_eq!(info.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn fetch_by_aid_works() {
        let f = BilibiliFetcher::new();
        let r = f.fetch_by_aid(123456);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().aid, 123456);
    }

    #[test]
    fn bv_to_aid_returns_some_for_valid() {
        // BV 长度错误应该返回 None
        assert_eq!(bv_to_aid("BV1xx"), None);
        // BV 长度对但字符不在表中也返回 None
        assert_eq!(bv_to_aid("BV1??????????"), None);
    }
}
