#![no_main]

//! fuzz: **隐蔽爬虫的出网面**（`crates/capabilities/tools/src/stealth_crawler.rs`
//! 「Stealth Web & Multi-Modal Media Crawler」）+ 仓内**同族出网/取回面**
//! （受控 fetch 的 URL 判定、订阅源 URL 的敏感面拒绝、HTML 响应解析、
//! 响应缓存键隔离、按主机限流）。
//!
//! ## 为什么这个面值得 fuzz（docs/02-guides/fuzz-target-authoring.md §1）
//!
//! - **吃不受控输入**：爬虫正文/标题是**被抓页面的原文**（攻击者完全可控），
//!   目标 URL 由模型/运维给出；
//! - **有强承诺**：模块头自称「**Safety Boundaries**: Automatically encapsulates
//!   all harvested external HTML/text in `<<<[UNTRUSTED_CONTENT]>>>` defense
//!   envelopes **to neutralize prompt injections**」
//!   （`stealth_crawler.rs:10-11`）—— 这是**间接提示词注入**的正面防线；
//! - **接线状态**：`stealth_crawler` 目前**未接线**（全仓只有 `lib.rs:133` 的
//!   `pub mod` 与 `lib.rs:323` 的 re-export，零调用点）⇒ 属于「**上线前把边界
//!   验干净**」的那类面（规范 §1 第 3 问）；同族的**已接线**出网面
//!   （`fetch` / `egress` / `content_feed`）一并覆盖。
//!
//! ## 契约断言的出处（逐条对应被测代码自己写明的契约，**不自造期望**）
//!
//! | # | 断言 | 出处 |
//! |---|------|------|
//! | 1 | UA 必须取自 UA 池；池空 ⇒ 固定兜底串；同 URL ⇒ 同 UA（按 URL 哈希选取） | `stealth_crawler.rs:80-90`「Selects a pseudo-random User-Agent from the pool based on target URL hash.」+ `:82-84` 兜底分支 |
//! | 2 | 信封**结构**：`<<<[UNTRUSTED_CONTENT source="…"]>>>` … `<<<[/UNTRUSTED_CONTENT]>>>` | `stealth_crawler.rs:93-97` 实现 + 同文件单测 `:157` `contains("<<<[UNTRUSTED_CONTENT")` |
//! | 3 | **正文不得伪造边界**（提前闭合信封 = 提示词注入逃逸） | 模块头 `stealth_crawler.rs:10-11`「defense envelopes **to neutralize prompt injections**」+ 仓内**同一标记族**的规范实现 `untrusted_mark.rs:20-25`「将文本中出现的 `<<<[` 字面量安全替换为 `<<< [`，粉碎任何试图构造闭合标签实施逃逸的注入企图」+ 仓内**信封标准件** `untrusted_envelope.rs:231-238`「so a payload cannot close its own envelope and follow the closing marker with text that looks like it sits outside the boundary」 |
//! | 4 | **信源标注**在带引号的属性位里必须安全（无引号/无换行 ⇒ 不能伪造头部） | `untrusted_envelope.rs:240-247`「Render the source label safely inside a quoted marker attribute: boundary forgeries neutralized, quotes and line breaks flattened.」（仓内信封标准件口径） |
//! | 5 | `parse_scraped_document` 的结构化字段与代码自述一致（title 截断、`short_video` ⇒ 45s、tags） | `stealth_crawler.rs:112-126` 实现自述 |
//! | 6 | **script 标签口径大小写不敏感**（`<SCRIPT>` 与 `<script>` 同义） | 同 crate 同族实现 `fetch/html_text.rs:5-6`「script/style content is skipped」+ `:49-53`（先 `to_lowercase()` 再匹配）+ 仓内「同一套文本处理口径, 不造第二套」口径 `content_feed/summary.rs:16-17` |
//! | 7 | 受控 fetch 的 URL 判定：空/超长/非 http(s)/带凭据/无主机 ⇒ 拒；凭据拒绝**不回显口令**；片段剥除 | `fetch.rs:239-290`（逐条注释）+ 工具 schema 描述 `fetch.rs:204`「URL userinfo, unsupported schemes, and non-http(s) schemes are rejected.」+ 单测 `fetch.rs:687-784` |
//! | 8 | 订阅源 URL 的**敏感面拒绝**：凭据/非 http(s)/内网·回环·链路本地·未指定·广播 IP 字面量 ⇒ 拒；凭据拒绝不回显口令 | `content_feed/store.rs:12-18`「带凭据 (userinfo) 的 URL、非 http(s) 协议面、内网/回环/链路本地 IP 字面量一律**拒绝**」+ `:196-199`「拒绝原因**不回显** URL 本体 —— 凭据面拒绝时绝不把密码带回错误文本」+ `:242-256`（内网字面量拒绝，复用 `is_public_ip`） |
//! | 9 | **同一目标的不同写法 ⇒ 同一判定**（十进制/十六进制/八进制/缩写 IPv4、IPv6 括号式、IPv4-mapped IPv6） | 判据 8 的「内网地址拒绝 (字面量面)」+ `url` crate 的 WHATWG 主机解析（`Host::parse` → `ends_in_a_number` → `parse_ipv4addr`）——同一目标的十进制写法（`http://2130706433/` = `127.0.0.1`）不得绕过 |
//! | 10 | HTML 响应解析：空白折叠、script/style 正文跳过、实体解码、`href`/`<title>` 取值必须是原文子串 | `fetch/html_text.rs:1-11`（模块语义）+ `:104-123`（实体解码）+ `:137-165`/`:167-185`（links/title） |
//! | 11 | 响应缓存**键隔离**（键 = 精确的归一化请求 URL ⇒ 别的 URL 不得命中） | `fetch/response_cache.rs:14-15`「keys are the exact frozen normalized request URLs」 |
//! | 12 | 限流**按主机隔离**（一个主机吃掉配额不得影响另一个主机） | `fetch/rate_limit.rs:5-7`「each host has an independent window of request timestamps」 |
//!
//! ## 不做的事（诚实边界）
//!
//! - **不发真网络请求**：本 target 只碰**纯函数层**（URL 判定 / 解析 / 缓存 /
//!   限流）与**冻结期**判定（`freeze_invocation` 不触网）。需要真实 HTTP 回程的
//!   路径（**重定向逐跳重验**、DNS 解析后判定、响应体读取）**未覆盖**，见交付
//!   报告的「需要真网络而未覆盖」清单；
//! - **不引入新依赖**：URL 的可解析性用同 crate 的公开 API
//!   `content_feed::store::validate_feed_url` 造（不直接依赖 `url` crate）；
//! - 判定用**可判定的后置条件**（结果 == 拒 / 结果 == 允许 / 输出 == 期望形态），
//!   不用 `contains(某 token)` 判「没有泄漏」。
//!
//! ## 输入编码
//!
//! 第 1 字节 = 段号；其余按 `0x00` 切成若干字段（与仓内既有 target 同口径）。

use std::time::Duration;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::content_feed::store::validate_feed_url;
use apeireth_tools_canonical::egress::EgressPolicy;
use apeireth_tools_canonical::fetch::html_text::{HtmlExtractError, extract_links, extract_text, extract_title};
use apeireth_tools_canonical::fetch::rate_limit::RateLimiter;
use apeireth_tools_canonical::fetch::response_cache::ResponseCache;
use apeireth_tools_canonical::fetch::{FETCH_URL_MAX_BYTES, FetchConfig, FetchTool};
use apeireth_tools_canonical::stealth_crawler::{StealthBrowserConfig, StealthCrawlerEngine};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

// ───────────────────────────── 信封常量（被测代码自己的标记族） ─────────────────────────────

/// 信封开标记（`stealth_crawler.rs:95` 的 `<<<[UNTRUSTED_CONTENT`）。
const OPEN_TAG: &str = "<<<[UNTRUSTED_CONTENT";
/// 信封收标记（`stealth_crawler.rs:95` 的 `<<<[/UNTRUSTED_CONTENT]>>>`）。
const CLOSE_TAG: &str = "<<<[/UNTRUSTED_CONTENT]>>>";
/// 边界前缀（开标记与收标记共有；**正文里出现它就是伪造边界的企图**）。
const BOUNDARY_PREFIX: &str = "<<<[";
/// 信封头部前缀（含带引号的 `source=` 属性位）。
const HEADER_PREFIX: &str = "<<<[UNTRUSTED_CONTENT source=\"";

// ───────────────────────────── 极简输入工具 ─────────────────────────────

fn field<'a>(fields: &'a [&'a [u8]], index: usize) -> &'a [u8] {
    fields.get(index).copied().unwrap_or(&[])
}

/// 任意字节 → 合法 UTF-8 文本（非法字节按 U+FFFD 替换 ⇒ 永不 panic）。
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// 主机名形状的文本：空白/控制字符换成 `'x'`；其余（含 Unicode）原样保留。
fn host_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .map(|c| {
            if c.is_whitespace() || c.is_control() {
                'x'
            } else {
                c
            }
        })
        .collect()
}

/// 路径片段（只保留 URL 安全字符 ⇒ 拼出来的 URL 一定能被解析）。
fn path_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-._~!$&'()*+,;=:@/%".contains(c) {
                c
            } else {
                'x'
            }
        })
        .collect()
}

/// 口令形状：只保留字母数字（⇒ 放进 URL 的 userinfo 段不会被转义，改写形态可判）。
fn secret_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(24)
        .collect()
}

/// 信源标注是否「干净」（无引号/无换行/无边界前缀 ⇒ 允许逐字断言渲染结果）。
fn is_clean_label(source: &str) -> bool {
    !source.contains('"')
        && !source.contains('\r')
        && !source.contains('\n')
        && !source.contains(BOUNDARY_PREFIX)
}

/// 信封首行（头部）。
fn header_line(wrapped: &str) -> &str {
    wrapped.split('\n').next().unwrap_or("")
}

/// 信封正文区（首行之后、收标记之前的那一段文本）。
fn payload_region(wrapped: &str) -> &str {
    let start = wrapped.find('\n').map(|index| index + 1).unwrap_or(wrapped.len());
    let end = wrapped.len().saturating_sub(CLOSE_TAG.len() + 1);
    if start >= end {
        return "";
    }
    &wrapped[start..end]
}

// ───────────────────────── 判据 1：UA 选取 ─────────────────────────

/// 契约（`stealth_crawler.rs:80-90`）：
/// - 「Selects a pseudo-random User-Agent **from the pool** based on target URL
///   hash.」⇒ 返回值必须是池内元素；
/// - 池空 ⇒ 走 `:82-84` 的固定兜底串分支；
/// - 「based on target URL hash」⇒ 同 URL ⇒ 同 UA（纯函数）。
fn section_user_agent(fields: &[&[u8]]) {
    let url = text(field(fields, 0));

    // 池：由 fuzz 字段拼出（可为空 ⇒ 覆盖兜底分支）。
    let pool: Vec<String> = (1..=4)
        .map(|index| text(field(fields, index)))
        .filter(|entry| !entry.is_empty() || index_is_empty(fields, index))
        .collect();
    let pool: Vec<String> = if pool.is_empty() {
        Vec::new()
    } else {
        pool
    };

    let config = StealthBrowserConfig {
        user_agent_pool: pool.clone(),
        ..StealthBrowserConfig::default()
    };
    let crawler = StealthCrawlerEngine::new(config);

    let selected = crawler.select_user_agent(&url);

    if pool.is_empty() {
        // 池空 ⇒ 固定兜底串（`stealth_crawler.rs:82-84`）。
        assert_eq!(
            selected, "Apeireth-StealthCrawler/2.0",
            "UA 池为空时必须走固定兜底串: url={url:?}"
        );
    } else {
        assert!(
            pool.iter().any(|entry| entry == selected),
            "UA 必须取自 UA 池: url={url:?} selected={selected:?} pool={pool:?}"
        );
    }

    // 纯函数性：同 URL ⇒ 同 UA。
    assert_eq!(
        selected,
        crawler.select_user_agent(&url),
        "同一 URL 的 UA 必须稳定（按 URL 哈希选取）: url={url:?}"
    );
}

/// 字段是否本就存在（用于区分「空字段」与「不存在字段」—— 两者都算空池）。
fn index_is_empty(fields: &[&[u8]], index: usize) -> bool {
    field(fields, index).is_empty()
}

// ───────────────────────── 判据 2+3+4：信封 ─────────────────────────

/// 契约：
/// - 结构（`stealth_crawler.rs:93-97` + 同文件单测 `:157`）；
/// - 正文**不得伪造边界**（模块头 `:10-11` + 仓内同族规范 `untrusted_mark.rs:20-25`
///   + 信封标准件 `untrusted_envelope.rs:231-238`）；
/// - 信源标注在带引号的属性位里必须安全（`untrusted_envelope.rs:240-247`）。
fn assert_envelope_contract(wrapped: &str, source: &str, content: &str) {
    // ① 结构：固定的开/收标记。
    assert!(
        wrapped.starts_with(HEADER_PREFIX),
        "信封头部必须是固定形态 {HEADER_PREFIX:?}: wrapped={wrapped:?}"
    );
    assert!(
        wrapped.ends_with(CLOSE_TAG),
        "信封必须以收标记收尾: wrapped={wrapped:?}"
    );

    let header = header_line(wrapped);

    // ② 头部是**单行**：信源标注不得带换行（带换行 ⇒ 标记之后即越界）。
    assert!(
        header.ends_with("\"]>>>"),
        "信封头部必须是单行（信源标注不得含换行）: header={header:?}"
    );
    assert!(
        !header.contains('\r'),
        "信封头部不得含回车: header={header:?}"
    );

    // ③ 带引号的属性位只允许一对引号（信源标注不得含引号）。
    assert_eq!(
        header.matches('"').count(),
        2,
        "信源标注不得含引号（否则可伪造头部）: header={header:?}"
    );

    // ④ 正文/信源**不得伪造边界标记**（提前闭合信封 = 提示词注入逃逸）。
    assert_eq!(
        wrapped.matches(BOUNDARY_PREFIX).count(),
        2,
        "信封内只允许开/收两个边界标记（正文或信源伪造了边界）: wrapped={wrapped:?}"
    );
    assert_eq!(
        wrapped.matches(CLOSE_TAG).count(),
        1,
        "收标记只允许出现一次（正文提前闭合了信封）: wrapped={wrapped:?}"
    );

    // ⑤ 干净输入 ⇒ 渲染结果逐字可判（不臆造格式）。
    if is_clean_label(source) {
        assert_eq!(
            header,
            format!("{HEADER_PREFIX}{source}\"]>>>"),
            "干净信源的渲染形态必须逐字一致: source={source:?}"
        );
    }
    if !content.contains(BOUNDARY_PREFIX) {
        assert_eq!(
            payload_region(wrapped),
            content,
            "干净正文必须逐字落在信封正文区: content={content:?}"
        );
    }
}

/// 段 1：直接打 `wrap_untrusted_content`（信源 + 正文都是外部原文）。
fn section_envelope(fields: &[&[u8]]) {
    let source = text(field(fields, 0));
    let content = text(field(fields, 1));
    let wrapped = StealthCrawlerEngine::wrap_untrusted_content(&source, &content);
    assert_envelope_contract(&wrapped, &source, &content);
}

// ───────────────────── 判据 5+6：结构化抽取（抓页面原文） ─────────────────────

/// 契约：
/// - 结构化字段与实现自述一致（`stealth_crawler.rs:112-126`）；
/// - 信封性质同判据 2+3+4；
/// - script 标签口径**大小写不敏感**（同族实现 `fetch/html_text.rs:49-53` 先
///   小写再匹配；仓内「同一套文本处理口径, 不造第二套」口径
///   `content_feed/summary.rs:16-17`）。
fn section_document(fields: &[&[u8]]) {
    let url = text(field(fields, 0));
    let title = text(field(fields, 1));
    let body = text(field(fields, 2));
    let media_type = text(field(fields, 3));

    let crawler = StealthCrawlerEngine::default();
    let item = crawler.parse_scraped_document(&url, &title, &body, &media_type);

    // ① 结构化字段（实现自述）。
    assert_eq!(item.url, url, "url 字段必须原样带回");
    assert_eq!(item.title, title.trim(), "标题必须去空白");
    assert_eq!(
        item.media_duration_secs,
        if media_type == "short_video" {
            Some(45.0)
        } else {
            None
        },
        "时长只对 short_video 生效: media_type={media_type:?}"
    );
    assert_eq!(
        item.tags,
        vec!["web_curation".to_string(), media_type.clone()],
        "标签集合与实现自述一致: media_type={media_type:?}"
    );

    // ② 信封性质（正文是抓回来的原文 ⇒ 必须落在信封里且不能伪造边界）。
    assert_envelope_contract(&item.clean_markdown, &url, &body);

    // ③ script 标签口径：HTML 标签名大小写不敏感 ⇒ `<SCRIPT>` 与 `<script>` 同义。
    let payload = payload_region(&item.clean_markdown);
    assert!(
        !payload.to_ascii_lowercase().contains("<script"),
        "clean markdown 里不得残留 script 标签（大小写不敏感口径）: payload={payload:?}"
    );
}

// ───────────────────── 判据 7：受控 fetch 的 URL 判定（冻结期，不触网） ─────────────────────

/// 契约（`fetch.rs:239-290` + 工具 schema `fetch.rs:204` + 单测 `fetch.rs:687-784`）：
/// - 空 URL ⇒ 拒（`fetch.rs:240-245`「fetch url must not be empty」）；
/// - 超长 ⇒ 拒（`fetch.rs:246-255`，上限 `FETCH_URL_MAX_BYTES`）；
/// - 非 http(s) ⇒ 拒（`fetch.rs:262-271`，schema 描述 `:204` 同口径）；
/// - 带凭据 ⇒ 拒（`fetch.rs:272-278`「fetch url userinfo is not supported and
///   must not be sent」）**且不得回显口令**（单测 `fetch.rs:716-727`：
///   `!text.contains("secret")`）；
/// - 无主机 ⇒ 拒（`fetch.rs:279-284`）；
/// - 片段剥除（`fetch.rs:286-288`「Fragments are never sent over HTTP. Strip
///   before freezing/execution so the effective operation is the actual HTTP
///   request.」）。
fn section_fetch_url(fields: &[&[u8]]) {
    let shape = field(fields, 0).first().copied().unwrap_or(0);
    let tool = FetchTool::new(FetchConfig::deny_all());

    let raw = match shape % 6 {
        0 => text(field(fields, 1)),
        1 => {
            // 带凭据（口令在运行时构造，不落任何字面量密钥）。
            let secret = format!("pw{}", secret_text(field(fields, 2)));
            format!("https://user:{secret}@example.com/path")
        }
        2 => {
            let scheme = ["file", "ftp", "gopher", "data", "javascript", "ws"]
                [field(fields, 1).first().copied().unwrap_or(0) as usize % 6];
            format!("{scheme}://example.com/path")
        }
        3 => format!(
            "https://example.com/{}",
            "a".repeat(FETCH_URL_MAX_BYTES + 16)
        ),
        4 => String::new(),
        _ => {
            let host = host_text(field(fields, 1));
            let path = path_text(field(fields, 2));
            format!("https://{host}/{path}")
        }
    };

    let call = ToolCall {
        id: "fuzz-fetch".to_string(),
        name: "fetch".to_string(),
        arguments: json!({ "url": raw.clone() }),
    };
    let frozen = tool.freeze_invocation(&call);

    match shape % 6 {
        0 => {
            // 合法 URL ⇒ 必须被接受，且冻结的 URL 就是**归一化后**的 URL。
            let Ok(expected) = validate_feed_url(&raw, &EgressPolicy::Unrestricted) else {
                // 本身不可接受（解析失败/带凭据/无主机/非 http(s)）⇒ 冻结必须拒。
                let error = frozen.expect_err("不可接受的 URL 必须被拒");
                assert!(
                    !error.render().is_empty(),
                    "拒绝必须有原因文本: url={raw:?}"
                );
                return;
            };
            let frozen = frozen.expect("合法 URL 必须被接受").expect("必须冻结出调用");
            let frozen_url = frozen
                .payload
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            assert_eq!(
                frozen_url,
                expected.to_string(),
                "冻结的 URL 必须就是归一化后的 URL（片段剥除）: url={raw:?}"
            );
            assert!(
                !frozen_url.contains('#'),
                "片段不得进入冻结的请求: url={raw:?} frozen={frozen_url:?}"
            );
            assert!(
                !frozen_url.contains('@'),
                "凭据不得进入冻结的请求: url={raw:?} frozen={frozen_url:?}"
            );
        }
        1 => {
            let error = frozen.expect_err("带凭据的 URL 必须被拒");
            let rendered = error.render();
            assert!(
                rendered.contains("userinfo"),
                "凭据拒绝必须点明 userinfo: url={raw:?} rendered={rendered:?}"
            );
            let secret = raw
                .split("://")
                .nth(1)
                .and_then(|rest| rest.split('@').next())
                .and_then(|userinfo| userinfo.split_once(':'))
                .map(|(_, password)| password.to_string())
                .unwrap_or_default();
            assert!(
                !secret.is_empty() && !rendered.contains(&secret),
                "凭据拒绝不得回显口令: url={raw:?} rendered={rendered:?}"
            );
        }
        2 => {
            let error = frozen.expect_err("非 http(s) 的 URL 必须被拒");
            assert!(
                error.render().contains("scheme"),
                "协议面拒绝必须点明 scheme: url={raw:?} rendered={:?}",
                error.render()
            );
        }
        3 => {
            let error = frozen.expect_err("超长 URL 必须被拒");
            assert!(
                error.render().contains("maximum"),
                "超长拒绝必须点明上限: rendered={:?}",
                error.render()
            );
        }
        4 => {
            let error = frozen.expect_err("空 URL 必须被拒");
            assert!(
                error.render().contains("must not be empty"),
                "空 URL 拒绝必须点明原因: rendered={:?}",
                error.render()
            );
        }
        _ => {
            // 模糊 URL：判定层必须**要么接受要么拒**，不得 panic；接受时不得带凭据/片段。
            if let Ok(Some(frozen)) = frozen {
                let frozen_url = frozen
                    .payload
                    .get("url")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                assert!(
                    !frozen_url.contains('#') && !frozen_url.contains('@'),
                    "冻结的 URL 不得含片段或凭据: url={raw:?} frozen={frozen_url:?}"
                );
            }
        }
    }
}

// ───────────── 判据 8+9：订阅源 URL 的敏感面拒绝（出网面） ─────────────

/// 非公网 IP 字面量（各种书写形态）—— 契约：订阅表**一律拒绝**
/// （`content_feed/store.rs:12-18` + `:242-256`，复用 `is_public_ip` 口径）。
const NON_PUBLIC_LITERAL_FORMS: &[&str] = &[
    "http://127.0.0.1/feed",
    "http://127.1/feed",
    "http://2130706433/feed",
    "http://0x7f.0.0.1/feed",
    "http://0177.0.0.1/feed",
    "http://0x7f000001/feed",
    "http://017700000001/feed",
    "http://10.0.0.5/feed",
    "http://192.168.1.10/feed",
    "http://172.16.0.1/feed",
    "http://169.254.169.254/latest/meta-data",
    "http://0.0.0.0/feed",
    "http://255.255.255.255/feed",
    "http://[::1]/feed",
    "http://[::]/feed",
    "http://[fc00::1]/feed",
    "http://[fe80::1]/feed",
    "http://[::ffff:127.0.0.1]/feed",
    "http://[0:0:0:0:0:ffff:7f00:1]/feed",
];

/// 公网 IP 字面量 —— 契约：订阅表**必须接受**。
const PUBLIC_LITERAL_FORMS: &[&str] = &[
    "https://8.8.8.8/rss",
    "https://93.184.216.34/rss",
    "https://[2001:4860:4860::8888]/rss",
];

/// 非 http(s) 协议面 —— 契约：一律拒绝（`store.rs:211-216`）。
const REFUSED_SCHEMES: &[&str] = &["file", "ftp", "gopher", "data", "javascript", "ws"];

fn section_feed_url(fields: &[&[u8]]) {
    let shape = field(fields, 0).first().copied().unwrap_or(0) % 6;
    let policy = EgressPolicy::PublicInternetOnly;

    match shape {
        // ① 同一内网目标的各种书写形态 ⇒ **一律拒**（判据 9：写法不同、判定一致）。
        0 => {
            for url in NON_PUBLIC_LITERAL_FORMS {
                let error = validate_feed_url(url, &policy)
                    .err()
                    .unwrap_or_else(|| panic!("非公网 IP 字面量必须被拒: url={url}"));
                assert!(
                    error.contains("refused"),
                    "内网/回环/链路本地拒绝必须点明 refused: url={url} error={error}"
                );
            }
        }
        // ② 公网 IP 字面量 ⇒ 接受。
        1 => {
            for url in PUBLIC_LITERAL_FORMS {
                assert!(
                    validate_feed_url(url, &policy).is_ok(),
                    "公网 IP 字面量必须被接受: url={url}"
                );
            }
        }
        // ③ 凭据面 ⇒ 拒，且**不回显口令**。
        2 => {
            let secret = format!("pw{}", secret_text(field(fields, 1)));
            let url = format!("https://user:{secret}@example.com/rss");
            let error = validate_feed_url(&url, &policy)
                .err()
                .unwrap_or_else(|| panic!("带凭据的订阅 URL 必须被拒: url={url}"));
            assert!(
                !error.contains(&secret),
                "凭据拒绝不得回显口令: url={url} error={error}"
            );
        }
        // ④ 协议面 ⇒ 拒。
        3 => {
            for scheme in REFUSED_SCHEMES {
                let url = format!("{scheme}://example.com/rss");
                let error = validate_feed_url(&url, &policy)
                    .err()
                    .unwrap_or_else(|| panic!("非 http(s) 协议必须被拒: url={url}"));
                assert!(
                    error.contains("scheme") || error.contains("refused"),
                    "协议面拒绝必须点明原因: url={url} error={error}"
                );
            }
        }
        // ⑤ 边界输入：空 / 超长 / 片段剥除。
        _ => {
            let error = validate_feed_url("", &policy)
                .err()
                .unwrap_or_else(|| panic!("空订阅 URL 必须被拒"));
            assert!(
                error.contains("must not be empty"),
                "空 URL 拒绝必须点明原因: error={error}"
            );

            let long = format!("https://example.com/{}", "a".repeat(FETCH_URL_MAX_BYTES));
            let error = validate_feed_url(&long, &policy)
                .err()
                .unwrap_or_else(|| panic!("超长订阅 URL 必须被拒"));
            assert!(
                error.contains("maximum"),
                "超长拒绝必须点明上限: error={error}"
            );

            let normalized = validate_feed_url("https://example.com:8443/rss#frag", &policy)
                .expect("合法公网订阅 URL 必须被接受");
            assert_eq!(
                normalized.as_str(),
                "https://example.com:8443/rss",
                "订阅 URL 必须剥除片段"
            );
        }
    }
}

// ───────────────────── 判据 10：HTML 响应解析（取回的正文） ─────────────────────

/// script/style 正文标记（探针用；fuzz 输入里若出现同名串则跳过该断言）。
const SCRIPT_PROBE: &str = "ZQXSCRIPTPROBEBODY";
const STYLE_PROBE: &str = "ZXQSTYLEPROBEBODY";

fn section_html(fields: &[&[u8]]) {
    let raw = text(field(fields, 0));
    let html = format!("<script>{SCRIPT_PROBE}</script><style>{STYLE_PROBE}</style><p>{raw}</p>");

    // ① `extract_text`（`fetch/html_text.rs:30-134`）。
    match extract_text(&html) {
        Ok(text_out) => {
            // 契约：正文空白折叠（模块头 `:6-7`「text is whitespace-collapsed」）。
            assert_eq!(
                text_out,
                text_out.split_whitespace().collect::<Vec<_>>().join(" "),
                "正文必须是空白折叠后的形态: out={text_out:?}"
            );
            assert_eq!(text_out.trim(), text_out, "正文不得有首尾空白");
            // 契约：script/style 正文跳过（模块头 `:5-6`）。
            if !raw.contains(SCRIPT_PROBE) {
                assert!(
                    !text_out.contains(SCRIPT_PROBE),
                    "script 正文必须被跳过: out={text_out:?}"
                );
            }
            if !raw.contains(STYLE_PROBE) {
                assert!(
                    !text_out.contains(STYLE_PROBE),
                    "style 正文必须被跳过: out={text_out:?}"
                );
            }
        }
        Err(HtmlExtractError::Empty) => {
            assert!(
                html.trim().is_empty(),
                "Empty 只能来自空白输入: html={html:?}"
            );
        }
        Err(HtmlExtractError::NoText) => {
            assert!(
                !html.trim().is_empty(),
                "NoText 只能来自非空输入: html={html:?}"
            );
        }
    }

    // ② `extract_links`（`fetch/html_text.rs:137-165`）：每条链接的 URL 必须是
    //    原文里 `href="…"` 的取值（不得自造）。
    for (url, label) in extract_links(&html) {
        assert!(
            !url.contains('"'),
            "链接 URL 不得含引号（取值到第一个引号为止）: url={url:?}"
        );
        assert!(
            html.contains(&format!("href=\"{url}\"")),
            "链接 URL 必须来自原文的 href 取值: url={url:?}"
        );
        assert_eq!(
            label,
            label.split_whitespace().collect::<Vec<_>>().join(" "),
            "链接文字必须空白折叠: label={label:?}"
        );
    }

    // ③ `extract_title`（`fetch/html_text.rs:167-185`）：取值必须是原文子串。
    match extract_title(&html) {
        Some(title) => {
            assert!(
                html.contains("<title>"),
                "有标题 ⇒ 原文必须含 <title>: html={html:?}"
            );
            assert!(
                html.contains(&title),
                "标题必须是原文子串: title={title:?}"
            );
        }
        None => {
            assert!(
                !html.contains("<title>"),
                "无标题 ⇒ 原文不得含 <title>: html={html:?}"
            );
        }
    }
}

// ───────────────────── 判据 11：响应缓存的键隔离 ─────────────────────

/// 契约（`fetch/response_cache.rs:14-15`）：「keys are the exact frozen
/// normalized request URLs」⇒ 别的 URL 不得命中本 URL 的缓存。
fn section_cache(fields: &[&[u8]]) {
    let key = text(field(fields, 0));
    let other = text(field(fields, 1));
    let cache = ResponseCache::new(Duration::from_secs(60));
    let value = json!({ "body": key.clone() });

    cache.put(key.clone(), value.clone());
    assert_eq!(
        cache.get(&key),
        Some(value.clone()),
        "写入的 URL 必须命中自己的缓存: key={key:?}"
    );
    if key != other {
        assert!(
            cache.get(&other).is_none(),
            "别的 URL 不得命中本 URL 的缓存: key={key:?} other={other:?}"
        );
    }
}

// ───────────────────── 判据 12：限流的按主机隔离 ─────────────────────

/// 契约（`fetch/rate_limit.rs:5-7`）：「each host has an independent window of
/// request timestamps」⇒ 一个主机的配额不得影响另一个主机。
fn section_rate_limit(fields: &[&[u8]]) {
    let host = host_text(field(fields, 0));
    let other = host_text(field(fields, 1));
    let mut limiter = RateLimiter::with_limit(1, Duration::from_secs(60));

    assert!(limiter.check(&host), "首次请求必须放行: host={host:?}");
    limiter.record(&host);
    assert!(
        !limiter.check(&host),
        "配额用尽后必须限流: host={host:?}"
    );
    assert_eq!(limiter.count(&host), 1, "计数必须落在本主机: host={host:?}");
    if host != other {
        assert!(
            limiter.check(&other),
            "限流必须按主机隔离（别的主机不受影响）: host={host:?} other={other:?}"
        );
    }
}

// ───────────────────────────── 主入口 ─────────────────────────────

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let fields: Vec<&[u8]> = rest.split(|byte| *byte == 0x00).collect();
    match mode % 8 {
        0 => section_user_agent(&fields),
        1 => section_envelope(&fields),
        2 => section_document(&fields),
        3 => section_fetch_url(&fields),
        4 => section_feed_url(&fields),
        5 => section_html(&fields),
        6 => section_cache(&fields),
        _ => section_rate_limit(&fields),
    }
});
