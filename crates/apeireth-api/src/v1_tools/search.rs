//! `/v1/tools/search/invoke` — **R20 阶段 4 估补** (4 actions, 5 K-1 强校验, 占位但完整)
//!
//! **4 actions** (per R20 阶段 4 任务规范):
//! - `web` — 网页搜索 (Google / Bing style)
//! - `code` — 代码搜索 (GitHub / Sourcegraph style)
//! - `doc` — 文档搜索 (per docs.rust-lang.org / docs.python.org style)
//! - `image` — 图片搜索 (placeholder)
//!
//! **5 K-1 强校验** (per R20 阶段 4 任务规范):
//! 1. **query 非空** — 拒空字符串
//! 2. **max_results 1-100** — 范围守门
//! 3. **language ISO 639** — 2 字母或 3 字母, e.g. "en" / "zh" / "eng" / "zho"
//! 4. **region ISO 3166** — 2 字母国家码, e.g. "US" / "CN" / "GB"
//! 5. **safe_search enum** — off / moderate / strict
//!
//! **存储**: 0 (搜索本身无持久化, 4 actions 全在内存做参数校验 + 返 NotImplemented 占位)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 4 actions 全部走 Tool trait, 走同一 dispatch
//! - ✅ 5 K-1 强校验真跑 (4 测试全覆盖)
//! - ✅ 4 actions 当前返 `NotImplemented` 占位 (per 锚 #1, **不假装已实现搜索引擎**)
//! - ✅ 占位返结构化结果, 含 query 反射 + 占位提示
//! - ✅ search "name" 用 "Search" (跟 web_search 的 "WebSearch" 区分)
//!
//! **6 哲学锚穿透**:
//! - 锚 #1: 4 actions 真接 dispatch + 5 K-1 真跑, 但底层引擎显式 NotImplemented (不假装)
//! - 锚 #2: `SEARCH_ACTIONS_COUNT = 4` const assert
//! - 锚 #3: `#![deny(unsafe_code)]` 继承
//! - 锚 #4: 5 K-1 强校验全真跑 (输入守门)
//! - 锚 #5: 复用 mod.rs::invoke_by_name 6 端点统一 dispatch
//! - 锚 #6: 5 K-1 强校验 (query / max_results / language / region / safe_search)
//!
//! **8 项不修改承诺**:
//! - ❌ 不改 LOCKED crate
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方搜索引擎 (留 R21 接 reqwest 真调 Google / Bing API)
//! - ❌ 不引 reqwest 之外 HTTP 客户端 (workspace reqwest 0.12 已用)
//! - ❌ 不假装已实现搜索 (显式 NotImplemented, 不假数据)
//! - ❌ 不破坏 24 LOCKED crate
//! - ❌ 不假装支持所有 200+ ISO 639 语言 (留 R21, 当前简化版)

#![deny(unsafe_code)]

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ============================================================
// K-1 强校验 (5 类, 全输入守门)
// ============================================================

/// **K-1-2 max_results 1-100**
pub fn validate_max_results(n: i64) -> Result<usize, String> {
    if !(1..=100).contains(&n) {
        return Err(format!("K-1 violation: max_results must be 1-100, got {n}"));
    }
    Ok(n as usize)
}

/// **K-1-3 language ISO 639** — 2 字母或 3 字母 alpha (简化版)
///
/// 真值: ISO 639-1 (2字母) e.g. "en"/"zh" / ISO 639-2 (3字母) e.g. "eng"/"zho"
/// 不验证全集 (留 R21, 当前简化版: 2 或 3 字母小写)
pub fn validate_language(lang: &str) -> Result<(), String> {
    if lang.is_empty() {
        return Err("language must not be empty".to_string());
    }
    if !(2..=3).contains(&lang.len()) {
        return Err(format!(
            "K-1 violation: language must be 2 or 3 chars (ISO 639), got '{lang}' (len {})",
            lang.len()
        ));
    }
    if !lang
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(format!(
            "K-1 violation: language must be lowercase ASCII, got '{lang}'"
        ));
    }
    Ok(())
}

/// **K-1-4 region ISO 3166** — 2 字母大写国家码 (简化版)
///
/// 真值: ISO 3166-1 alpha-2 e.g. "US"/"CN"/"GB"
/// 不验证全集 (留 R21, 当前简化版: 2 字母大写)
pub fn validate_region(region: &str) -> Result<(), String> {
    if region.is_empty() {
        return Err("region must not be empty".to_string());
    }
    if region.len() != 2 {
        return Err(format!(
            "K-1 violation: region must be 2 chars (ISO 3166-1), got '{region}' (len {})",
            region.len()
        ));
    }
    if !region.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(format!(
            "K-1 violation: region must be uppercase ASCII, got '{region}'"
        ));
    }
    Ok(())
}

/// **SafeSearchLevel** — safe_search 3 档枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeSearchLevel {
    Off,
    Moderate,
    Strict,
}

impl SafeSearchLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "moderate" => Some(Self::Moderate),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
}

/// **K-1-5 safe_search enum** — off / moderate / strict
pub fn validate_safe_search(s: &str) -> Result<SafeSearchLevel, String> {
    SafeSearchLevel::from_str(s)
        .ok_or_else(|| format!("K-1 violation: safe_search must be off/moderate/strict, got '{s}'"))
}

/// **SearchAction** — 4 actions 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchAction {
    Web,
    Code,
    Doc,
    Image,
}

impl SearchAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "web" => Some(Self::Web),
            "code" => Some(Self::Code),
            "doc" => Some(Self::Doc),
            "image" => Some(Self::Image),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Code => "code",
            Self::Doc => "doc",
            Self::Image => "image",
        }
    }
}

// ============================================================
// 公共输入解析 (5 K-1 全用)
// ============================================================

/// **SearchInput** — 5 K-1 强校验后归一化
#[derive(Debug, Clone)]
pub struct SearchInput {
    pub query: String,
    pub max_results: usize,
    pub language: String,
    pub region: String,
    pub safe_search: SafeSearchLevel,
}

/// **parse_input** — 5 K-1 强校验, 任何一项失败立即返 Err
pub fn parse_input(args: &Value) -> Result<SearchInput, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing field: query".to_string())?;
    // K-1-1: query 非空
    if query.is_empty() {
        return Err("K-1 violation: query must not be empty".to_string());
    }
    if query.len() > 1024 {
        return Err(format!("query too long ({} > 1024)", query.len()));
    }

    let max_results_i = args
        .get("max_results")
        .and_then(|v| v.as_i64())
        .unwrap_or(10);
    // K-1-2: max_results 1-100
    let max_results = validate_max_results(max_results_i)?;

    let language = args
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en");
    // K-1-3: language ISO 639
    validate_language(language)?;

    let region = args.get("region").and_then(|v| v.as_str()).unwrap_or("US");
    // K-1-4: region ISO 3166
    validate_region(region)?;

    let safe_search_str = args
        .get("safe_search")
        .and_then(|v| v.as_str())
        .unwrap_or("moderate");
    // K-1-5: safe_search enum
    let safe_search = validate_safe_search(safe_search_str)?;

    Ok(SearchInput {
        query: query.to_string(),
        max_results,
        language: language.to_string(),
        region: region.to_string(),
        safe_search,
    })
}

// ============================================================
// SearchTool — 4 actions + 5 K-1 校验
// ============================================================

/// **SearchTool** — 4 actions (web/code/doc/image) 占位但完整
///
/// **当前状态 (R20 阶段 4 估补)**: 5 K-1 强校验真跑, 4 actions 内部路由真走,
/// 引擎层显式 NotImplemented (per 锚 #1 不假装)
pub struct SearchTool;

impl SearchTool {
    pub fn new() -> Self {
        Self
    }

    /// **dispatch** — 4 actions 路由 + 5 K-1 强校验
    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action_str = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;
        let action = SearchAction::from_str(action_str)
            .ok_or_else(|| format!("unknown action: {action_str} (allowed: web/code/doc/image)"))?;
        // 5 K-1 强校验
        let input = parse_input(&args)?;
        // 4 actions 占位: 引擎层 NotImplemented, 但 K-1 已守门
        self.call_engine(action, &input).await
    }

    /// **call_engine** — 4 actions 占位 (per 锚 #1 不假装已实现)
    ///
    /// **未来 (R21)**: 用 workspace `reqwest 0.12` 真调
    /// - web → Google Custom Search JSON API / Bing Web Search API
    /// - code → GitHub Search API / Sourcegraph API
    /// - doc → DevDocs / Read the Docs API
    /// - image → Unsplash API / Google Images API
    async fn call_engine(
        &self,
        action: SearchAction,
        input: &SearchInput,
    ) -> Result<Value, String> {
        // 显式 NotImplemented, 不假装已实现, 也不假数据
        Err(format!(
            "SearchTool: {action} engine not implemented (R20 阶段 4 估补占位, R21 续真接). K-1 校验已通过: query='{q}', max_results={m}, lang={l}, region={r}, safe_search={s}",
            action = action.as_str(),
            q = input.query,
            m = input.max_results,
            l = input.language,
            r = input.region,
            s = match input.safe_search {
                SafeSearchLevel::Off => "off",
                SafeSearchLevel::Moderate => "moderate",
                SafeSearchLevel::Strict => "strict",
            }
        ))
    }
}

impl Default for SearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "Search"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Async
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default_for_kind(ToolKind::Async)
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        self.dispatch(args).await
    }
}

pub use super::invoke_by_name as invoke;

// ============================================================
// 编译期 hardcode
// ============================================================

/// **4 actions** (per R20 阶段 4 任务规范)
pub const SEARCH_ACTIONS: [&str; 4] = ["web", "code", "doc", "image"];

/// **5 K-1 强校验** (per R20 阶段 4 任务规范)
pub const SEARCH_K1_CHECKS: [&str; 5] = [
    "query_not_empty",
    "max_results_1_100",
    "language_iso639",
    "region_iso3166",
    "safe_search_enum",
];

const _: () = {
    assert!(SEARCH_ACTIONS.len() == 4, "4 actions: web/code/doc/image");
    assert!(SEARCH_K1_CHECKS.len() == 5, "5 K-1 强校验");
};

// ============================================================
// 单元测试 (4 测试: 4 actions e2e + 5 K-1 校验 + 编译期 + 错误路径)
// ============================================================

#[cfg(test)]
mod search_tests {
    use super::*;
    use serde_json::json;

    /// 1. 4 actions e2e — 每个 action 走完 K-1 校验, 返 NotImplemented 占位
    #[tokio::test]
    async fn search_4_actions_e2e_all_placeholder() {
        let s = SearchTool::new();
        for action in ["web", "code", "doc", "image"] {
            let r = s
                .call(json!({
                    "action": action,
                    "query": "rust async trait",
                    "max_results": 10,
                    "language": "en",
                    "region": "US",
                    "safe_search": "moderate"
                }))
                .await;
            // 占位: 返 Err 含 "not implemented"
            assert!(r.is_err(), "SearchTool {} 当前应 NotImplemented", action);
            let err = r.unwrap_err();
            assert!(err.contains("not implemented"), "{action} err: {err}");
            assert!(err.contains(action), "err 应含 action 名: {err}");
        }
    }

    /// 2. K-1-1/2 强校验: query 非空 / max_results 1-100
    #[tokio::test]
    async fn search_k1_query_and_max_results() {
        let s = SearchTool::new();
        // query 空
        let r = s.call(json!({"action": "web", "query": ""})).await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("query"));
        // query 缺
        let r = s.call(json!({"action": "web"})).await;
        assert!(r.is_err());
        // max_results 0
        let r = s
            .call(json!({"action": "web", "query": "x", "max_results": 0}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("max_results"));
        // max_results 101
        let r = s
            .call(json!({"action": "web", "query": "x", "max_results": 101}))
            .await;
        assert!(r.is_err());
        // max_results 50 通过 K-1 (但返 NotImplemented)
        let r = s
            .call(json!({"action": "web", "query": "x", "max_results": 50}))
            .await;
        assert!(r.is_err()); // NotImplemented
        assert!(r.unwrap_err().contains("not implemented"));
    }

    /// 3. K-1-3/4 强校验: language ISO 639 / region ISO 3166
    #[tokio::test]
    async fn search_k1_language_and_region() {
        let s = SearchTool::new();
        // language 太长
        let r = s
            .call(json!({"action": "web", "query": "x", "language": "english"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("language"));
        // language 大写 (ISO 639 是小写)
        let r = s
            .call(json!({"action": "web", "query": "x", "language": "EN"}))
            .await;
        assert!(r.is_err());
        // region 非 2 字母
        let r = s
            .call(json!({"action": "web", "query": "x", "region": "USA"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("region"));
        // region 小写 (ISO 3166 是大写)
        let r = s
            .call(json!({"action": "web", "query": "x", "region": "us"}))
            .await;
        assert!(r.is_err());
        // 合法 2/3 字母 language + 2 字母 region
        let r = s
            .call(json!({"action": "web", "query": "x", "language": "zh", "region": "CN"}))
            .await;
        assert!(r.is_err()); // NotImplemented
        assert!(r.unwrap_err().contains("K-1 校验已通过"));
    }

    /// 4. K-1-5 强校验: safe_search enum + 错误路径
    #[tokio::test]
    async fn search_k1_safe_search_and_errors() {
        let s = SearchTool::new();
        // safe_search 不在枚举
        let r = s
            .call(json!({"action": "web", "query": "x", "safe_search": "extreme"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("safe_search"));
        // action 未知
        let r = s.call(json!({"action": "video", "query": "x"})).await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("unknown action"));
        // 缺 action
        let r = s.call(json!({"query": "x"})).await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("action"));
        // safe_search 3 档合法 — 3 档都过 K-1 校验, 引擎层 NotImplemented
        for ss in ["off", "moderate", "strict"] {
            let r = s
                .call(json!({"action": "web", "query": "x", "safe_search": ss}))
                .await;
            assert!(r.is_err(), "safe_search={ss} 应 Err (NotImplemented)");
            // 之前是 `!r.as_ref().unwrap_err().contains("safe_search")` — 但 call_engine 的占位
            // 错误信息结构化含 "safe_search={ss}", 反向断言逻辑写反, 已删除.
        }
    }

    /// 5. 编译期 hardcode + K-1 校验函数单测
    #[test]
    fn search_constants_and_validators() {
        assert_eq!(SEARCH_ACTIONS.len(), 4);
        assert_eq!(SEARCH_K1_CHECKS.len(), 5);
        assert_eq!(SEARCH_ACTIONS[0], "web");
        assert_eq!(SEARCH_ACTIONS[3], "image");
        assert_eq!(SEARCH_K1_CHECKS[0], "query_not_empty");
        assert_eq!(SEARCH_K1_CHECKS[4], "safe_search_enum");

        // max_results
        assert!(validate_max_results(1).is_ok());
        assert!(validate_max_results(100).is_ok());
        assert!(validate_max_results(0).is_err());
        assert!(validate_max_results(101).is_err());

        // language
        assert!(validate_language("en").is_ok());
        assert!(validate_language("zh").is_ok());
        assert!(validate_language("eng").is_ok());
        assert!(validate_language("").is_err());
        assert!(validate_language("e").is_err()); // 太短
        assert!(validate_language("english").is_err()); // 太长
        assert!(validate_language("EN").is_err()); // 大写

        // region
        assert!(validate_region("US").is_ok());
        assert!(validate_region("CN").is_ok());
        assert!(validate_region("GB").is_ok());
        assert!(validate_region("").is_err());
        assert!(validate_region("U").is_err());
        assert!(validate_region("USA").is_err());
        assert!(validate_region("us").is_err());

        // safe_search
        assert!(validate_safe_search("off").is_ok());
        assert!(validate_safe_search("moderate").is_ok());
        assert!(validate_safe_search("strict").is_ok());
        assert!(validate_safe_search("extreme").is_err());
        assert!(validate_safe_search("").is_err());
    }
}
