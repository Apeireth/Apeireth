//! `v1_tools/search_test` — search 4 测试函数 unit test library
//!
//! **目的**: 给 R20 阶段 4 (D-01 真接细节) search 4 actions + 5 K-1 强校验
//! 加 4 测试, 由 `tests/test_v1_tools_unit_in_process.rs` 通过 `#[path]` 注入.
//!
//! **4 测试函数** (per 任务规范):
//! 1. `search_4_actions_all_placeholder` — web/code/doc/image 4 actions e2e (全 NotImplemented)
//! 2. `search_k1_query_max_results` — K-1-1 query 非空 + K-1-2 max_results 1-100
//! 3. `search_k1_language_region` — K-1-3 language ISO 639 + K-1-4 region ISO 3166
//! 4. `search_k1_safe_search_and_errors` — K-1-5 safe_search enum + 错误路径
//!
//! **5 K-1 强校验** (per `search.rs` 头部 + 任务规范):
//! 1. **query 非空** — 拒空字符串
//! 2. **max_results 1-100** — 范围守门
//! 3. **language ISO 639** — 2 或 3 字母小写
//! 4. **region ISO 3166** — 2 字母大写
//! 5. **safe_search enum** — off / moderate / strict
//!
//! **不假装** (per O-5 不漂移 + 锚 #1):
//! - ✅ 4 actions 全部走 Tool trait dispatch 真跑
//! - ✅ 5 K-1 强校验真跑 (4 测试全覆盖)
//! - ✅ 4 actions 引擎层显式 NotImplemented (不假装已实现搜索)
//! - ✅ 占位返结构化 Err 含 query 反射 + 占位提示
//! - ✅ search "name" 用 "Search" (跟 web_search "WebSearch" 区分)
//!
//! **6 哲学锚穿透**:
//! - 锚 #1 不漂移: 4 actions 真接 dispatch + 5 K-1 真跑, 引擎显式 NotImplemented
//! - 锚 #2 编译期 hardcode: `SEARCH_ACTIONS_COUNT = 4` const assert
//! - 锚 #3 不引入 unsafe: `#![deny(unsafe_code)]` 继承
//! - 锚 #4 真值守门: 5 K-1 强校验输入守门
//! - 锚 #5 复用 mod.rs::invoke_by_name (通过 stub 满足 `pub use super::invoke_by_name`)
//! - 锚 #6 工程铁律: 5 K-1 强校验
//!
//! **8 项不修改承诺 (严守)**:
//! - ❌ 不改 LOCKED `search.rs` (本文件 0 触碰)
//! - ❌ 不改 LOCKED `mod.rs` (本文件用 `#[path]` 注入, 不动 mod.rs)
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方搜索引擎 (留 R21)
//! - ❌ 不假装已实现搜索 (显式 NotImplemented, 不假数据)
//! - ❌ 不破坏 24 LOCKED crate
//! - ❌ 不引 reqwest 之外 HTTP 客户端 (workspace reqwest 0.12 已用)

#![deny(unsafe_code)]

// ============================================================
// 通过 #[path] 注入 search 源文件
// ============================================================

/// **search 源** — 注入 `src/v1_tools/search.rs` 全部内容
/// (本 crate mod.rs LOCKED 未 declare search, 用 #[path] 绕开)
#[path = "search.rs"]
mod _search_src;

// ============================================================
// `pub use super::invoke_by_name as invoke;` 桩
// ============================================================
//
// search.rs 源文件 line 301: `pub use super::invoke_by_name as invoke;`
// (mod.rs LOCKED 包含 `pub async fn invoke_by_name(...)`, 本文件单独编译时
//  `super` 指向 `_search_test` 自身, 所以本模块内必须提供 `invoke_by_name`
//  让 search 源编译通过. 桩签名不需要跟真函数一致, 只要有这个名字即可.)

#[allow(dead_code)]
pub async fn invoke_by_name() -> Result<(), String> {
    Ok(())
}

// ============================================================
// 4 测试函数 (per 任务规范)
// ============================================================

/// **4 测试函数总入口** — 由 `tests/test_v1_tools_unit_in_process.rs` 注入后
/// 通过 `#[tokio::test]` 调每个入口.
pub mod entries {
    use super::_search_src::{
        parse_input, validate_language, validate_max_results, validate_region,
        validate_safe_search, SafeSearchLevel, SearchAction, SearchInput, SearchTool,
        SEARCH_ACTIONS, SEARCH_K1_CHECKS,
    };
    // Tool trait 必须 in scope, 否则 SearchTool::call 方法找不到
    use apeireth_tool_registry::Tool;
    use serde_json::json;

    /// 1. 4 actions e2e — 全部 NotImplemented 占位
    pub async fn search_4_actions_all_placeholder() {
        let s = SearchTool::new();
        for action in SEARCH_ACTIONS.iter() {
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
            // 占位: 引擎层 NotImplemented, 返 Err
            assert!(r.is_err(), "SearchTool {action} 当前应 NotImplemented");
            let err = r.unwrap_err();
            assert!(
                err.contains("not implemented")
                    || err.contains("SearchTool")
                    || err.contains("engine"),
                "err 应含 'not implemented' 提示, got: {err}"
            );
            // 校验 err 信息含 K-1 反射 (query / max_results / lang / region / safe_search)
            assert!(err.contains("rust async trait"), "err 应含 query 反射");
            assert!(err.contains("max_results=10"), "err 应含 max_results 反射");
        }
    }

    /// 2. K-1-1 query 非空 + K-1-2 max_results 1-100
    pub async fn search_k1_query_max_results() {
        // query 缺 → Err
        let r = parse_input(&json!({}));
        assert!(r.is_err(), "缺 query 应 Err");
        let err = r.unwrap_err();
        assert!(
            err.contains("query") || err.contains("missing"),
            "缺 query err: {err}"
        );

        // query 空 → Err (K-1-1)
        let r = parse_input(&json!({"query": ""}));
        assert!(r.is_err(), "空 query 应 Err (K-1-1)");
        let err = r.unwrap_err();
        assert!(
            err.contains("query") && err.contains("empty"),
            "空 query err: {err}"
        );

        // query 超长 → Err (> 1024)
        let long = "x".repeat(1025);
        let r = parse_input(&json!({"query": long}));
        assert!(r.is_err(), "超长 query 应 Err");

        // max_results=0 → Err (K-1-2 下界)
        assert!(validate_max_results(0).is_err(), "max_results=0 应 Err");
        // max_results=1 → Ok
        assert!(
            validate_max_results(1).is_ok(),
            "max_results=1 应 Ok (下界)"
        );
        // max_results=100 → Ok (上界)
        assert!(
            validate_max_results(100).is_ok(),
            "max_results=100 应 Ok (上界)"
        );
        // max_results=101 → Err (上界+1)
        assert!(validate_max_results(101).is_err(), "max_results=101 应 Err");
        // max_results=-1 → Err (负)
        assert!(validate_max_results(-1).is_err(), "max_results=-1 应 Err");

        // 合法: 走完 parse_input
        let r: SearchInput = parse_input(&json!({
            "query": "rust",
            "max_results": 50
        }))
        .expect("parse ok");
        assert_eq!(r.query, "rust");
        assert_eq!(r.max_results, 50);
    }

    /// 3. K-1-3 language ISO 639 + K-1-4 region ISO 3166
    pub async fn search_k1_language_region() {
        // language: 2 字母小写 ok
        assert!(validate_language("en").is_ok(), "language 'en' 应 Ok");
        assert!(validate_language("zh").is_ok(), "language 'zh' 应 Ok");
        // language: 3 字母小写 ok
        assert!(validate_language("eng").is_ok(), "language 'eng' 应 Ok");
        assert!(validate_language("zho").is_ok(), "language 'zho' 应 Ok");
        // language: 1 字母 Err
        assert!(
            validate_language("e").is_err(),
            "language 'e' 应 Err (len 1)"
        );
        // language: 4 字母 Err
        assert!(
            validate_language("enUS").is_err(),
            "language 'enUS' 应 Err (len 4)"
        );
        // language: 大写 Err (守 2 字母小写简化版)
        assert!(
            validate_language("EN").is_err(),
            "language 'EN' 应 Err (大写)"
        );
        // language: 含特殊字符 Err
        assert!(validate_language("e!").is_err(), "language 'e!' 应 Err");
        // language: 空 → Err
        assert!(validate_language("").is_err(), "空 language 应 Err");

        // region: 2 字母大写 ok
        assert!(validate_region("US").is_ok(), "region 'US' 应 Ok");
        assert!(validate_region("CN").is_ok(), "region 'CN' 应 Ok");
        assert!(validate_region("GB").is_ok(), "region 'GB' 应 Ok");
        // region: 1 字母 Err
        assert!(validate_region("U").is_err(), "region 'U' 应 Err (len 1)");
        // region: 3 字母 Err
        assert!(
            validate_region("USA").is_err(),
            "region 'USA' 应 Err (len 3)"
        );
        // region: 小写 Err
        assert!(validate_region("us").is_err(), "region 'us' 应 Err (小写)");
        // region: 含数字 Err
        assert!(
            validate_region("U1").is_err(),
            "region 'U1' 应 Err (含数字)"
        );
        // region: 空 → Err
        assert!(validate_region("").is_err(), "空 region 应 Err");

        // 合法: 走完 parse_input
        let r = parse_input(&json!({
            "query": "test",
            "language": "zh",
            "region": "CN"
        }))
        .expect("parse ok zh/CN");
        assert_eq!(r.language, "zh");
        assert_eq!(r.region, "CN");
    }

    /// 4. K-1-5 safe_search enum + 错误路径
    pub async fn search_k1_safe_search_and_errors() {
        // safe_search: 3 值 ok
        assert!(
            validate_safe_search("off").is_ok(),
            "safe_search 'off' 应 Ok"
        );
        assert!(
            validate_safe_search("moderate").is_ok(),
            "safe_search 'moderate' 应 Ok"
        );
        assert!(
            validate_safe_search("strict").is_ok(),
            "safe_search 'strict' 应 Ok"
        );
        // safe_search: 错值 Err
        assert!(
            validate_safe_search("none").is_err(),
            "safe_search 'none' 应 Err"
        );
        assert!(validate_safe_search("").is_err(), "空 safe_search 应 Err");
        assert!(
            validate_safe_search("OFF").is_err(),
            "safe_search 'OFF' 应 Err (大小写)"
        );

        // SafeSearchLevel 枚举值对齐
        assert_eq!(SafeSearchLevel::from_str("off"), Some(SafeSearchLevel::Off));
        assert_eq!(
            SafeSearchLevel::from_str("moderate"),
            Some(SafeSearchLevel::Moderate)
        );
        assert_eq!(
            SafeSearchLevel::from_str("strict"),
            Some(SafeSearchLevel::Strict)
        );
        assert_eq!(SafeSearchLevel::from_str("weird"), None);

        // SearchAction 4 值对齐
        assert_eq!(SearchAction::from_str("web"), Some(SearchAction::Web));
        assert_eq!(SearchAction::from_str("code"), Some(SearchAction::Code));
        assert_eq!(SearchAction::from_str("doc"), Some(SearchAction::Doc));
        assert_eq!(SearchAction::from_str("image"), Some(SearchAction::Image));
        assert_eq!(SearchAction::from_str("unknown"), None);
        assert_eq!(SearchAction::Web.as_str(), "web");
        assert_eq!(SearchAction::Image.as_str(), "image");

        // 错误路径: 缺 action / 错 action / 缺 query
        let s = SearchTool::new();
        let r = s.call(json!({})).await;
        assert!(r.is_err(), "缺 action 应 Err");
        assert!(r.unwrap_err().contains("action"), "err 应含 'action'");

        let r = s.call(json!({"action": "unknown"})).await;
        assert!(r.is_err(), "错 action 应 Err");
        let err = r.unwrap_err();
        assert!(
            err.contains("unknown") || err.contains("allowed"),
            "错 action err 应含提示: {err}"
        );

        let r = s.call(json!({"action": "web"})).await;
        assert!(r.is_err(), "缺 query 应 Err");
        assert!(r.unwrap_err().contains("query"), "err 应含 'query'");

        // 编译期 hardcode
        assert_eq!(SEARCH_ACTIONS.len(), 4, "4 actions");
        assert_eq!(SEARCH_K1_CHECKS.len(), 5, "5 K-1 强校验");
        assert_eq!(SEARCH_ACTIONS[0], "web");
        assert_eq!(SEARCH_ACTIONS[3], "image");
        assert_eq!(SEARCH_K1_CHECKS[0], "query_not_empty");
        assert_eq!(SEARCH_K1_CHECKS[4], "safe_search_enum");
    }
}
