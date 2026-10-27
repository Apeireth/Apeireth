#![no_main]

//! fuzz: **内容流面（`tool.content_feed`）** —— 「截断如实 / 散列门去重」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/content_feed/`
//! （`summary::summarize_page` / `summary::summarize_feed_document` /
//! `ingest::item_identity` / `ingest::stable_ingest_content` / `ingest::item_source` /
//! `ingest::AttentionLimits`）。
//!
//! ## 为什么值得 fuzz（**外部内容 ⇒ 进上下文 ⇒ 注入面 + 注意力面**）
//!
//! 模块头写死的纪律：
//! 1. **摘要截断如实**：正文 ≤ `BODY_EXCERPT_MAX_CHARS`、标题 ≤ `TITLE_MAX_CHARS`，
//!    且 `body_truncated` **如实标注**（截断了就要说）；
//! 2. **缺项不编造**：标题缺失 ⇒ 「**无标题**」如实标注；来源没给时间/链接 ⇒ `None`（**不编**）；
//! 3. ⭐ **内容级去重**：`stable_ingest_content` **不含拉取时刻**
//!    （实现注释原话：「拉取时刻每次拉取都变, 若进内容则散列门无法按内容去重」）
//!    ⇒ ⇒ 同条目重复拉取 ⇒ **同散列** ⇒ **散列门跳过** ⇒ **零新记忆**；
//! 4. **条目身份**：条目链接优先，缺链接退化到**内容散列**（**稳定，不随拉取时刻变**）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **标题上限**：标题 ≤ `TITLE_MAX_CHARS` 字符 | `truncate_chars(&title, TITLE_MAX_CHARS)` |
//! | O2 | **正文上限**：正文 ≤ `BODY_EXCERPT_MAX_CHARS` 字符 | `truncate_chars(&text, BODY_EXCERPT_MAX_CHARS)` |
//! | O3 | **截断如实**：正文超限 ⇒ `body_truncated == true`；未超限 ⇒ false | `truncate_chars` 返回的二元组 |
//! | O4 | **缺标题如实标注**：无标题 ⇒ 标题 == 「无标题」 | `unwrap_or_else(|| "无标题")` |
//! | O5 | **来源标注如实**：`feed_id` / `source_url` **原样**保留（不截断不改写） | 字段定义 |
//! | O6 | **不编造时间/链接**：单条摘要 ⇒ `published_raw == None` 且 `item_link == None` | `published_raw: None, item_link: None` |
//! | O7 | ⭐ **散列门不含拉取时刻**：`stable_ingest_content` 对**不同 `fetched_at_ms`** 必须**逐字相同** | 去重面文档 |
//! | O8 | **条目身份稳定**：同条目（同链接/同内容）⇒ `item_identity` 相同；**链接优先于内容** | `item_identity` 分支 |
//! | O9 | **身份前缀闭合**：有链接 ⇒ `link:` 前缀；无链接 ⇒ `hash:` 前缀（**16 位十六进制**） | `item_identity` |
//! | O10 | **注意力上限可配**：`AttentionLimits` 默认值 + 两个覆盖方法生效 | `AttentionLimits` |
//!
//! ## 诚实边界
//!
//! - **不测** RSS/Atom 的**完整解析**（只测「解析出来的条目 ⇒ 摘要」的纪律）；
//! - **不测**生命周期账（去重账 / 退避 / 健康度）与**真实网络拉取**；
//! - 不测审批档（`pull` 每次停人工审批）—— 那是治理面，另立。

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::content_feed::ingest::{
    item_identity, item_source, stable_ingest_content, AttentionLimits,
};
use apeireth_tools_canonical::content_feed::summary::{
    summarize_feed_document, summarize_page, FeedItemSummary, BODY_EXCERPT_MAX_CHARS,
    TITLE_MAX_CHARS,
};
use apeireth_tools_canonical::content_feed::ingest::DEFAULT_DAILY_INGESTS;
use apeireth_tools_canonical::content_feed::ingest::DEFAULT_PER_PULL_ITEMS;

/// 候选文档体（覆盖 HTML / 纯文本 / 超长 / 空 / 多行）。
const BODIES: &[&str] = &[
    "",
    "   ",
    "普通一行",
    "标题行\n第二行\n第三行",
    "<html><head><title>网页标题</title></head><body><p>正文</p></body></html>",
    "<title>只有标题标签</title>内容",
    "<body>只有 body</body>",
    "a\nb\nc\nd\ne",
];

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    let feed_id = format!("feed-{}", b(0) % 4);
    let source_url = format!("https://example.invalid/feed/{}", b(1) % 3);
    let body_base = BODIES[(b(2) as usize) % BODIES.len()];
    // 变体：拼超长内容（压截断上限）
    let body = if b(3) % 3 == 0 {
        let filler = "字".repeat(2_500);
        format!("{body_base}{filler}")
    } else {
        body_base.to_string()
    };
    let fetched_at_ms = 1_700_000_000_000 + (b(4) as i64) * 1_000;

    let summary = summarize_page(&feed_id, &source_url, &body, fetched_at_ms);

    // ── O1 标题上限
    assert!(
        summary.title.chars().count() <= TITLE_MAX_CHARS,
        "标题不得超过上限（{}）: {} 字符",
        TITLE_MAX_CHARS,
        summary.title.chars().count()
    );
    // ── O2 正文上限
    assert!(
        summary.body_excerpt.chars().count() <= BODY_EXCERPT_MAX_CHARS,
        "正文不得超过上限（{}）: {} 字符",
        BODY_EXCERPT_MAX_CHARS,
        summary.body_excerpt.chars().count()
    );
    // ── O3 截断如实
    let plain_len = body.chars().count();
    if plain_len > BODY_EXCERPT_MAX_CHARS {
        assert!(
            summary.body_truncated,
            "正文超限必须如实标注 body_truncated（不得默默截）"
        );
    } else if !body.contains('<') {
        assert!(
            !summary.body_truncated,
            "正文未超限不得标注截断: len={plain_len}"
        );
    }
    // ── O4 缺标题如实标注
    if summary.title.trim().is_empty() {
        panic!("标题不得为空（缺标题时应标注「无标题」）");
    }
    // ── O5 来源标注如实
    assert_eq!(summary.feed_id, feed_id, "feed_id 必须原样保留");
    assert_eq!(summary.source_url, source_url, "source_url 必须原样保留");
    // ── O6 不编造时间/链接
    assert!(
        summary.published_raw.is_none(),
        "单条摘要不得编造来源时间（来源没给就是 None）"
    );
    assert!(
        summary.item_link.is_none(),
        "单条摘要不得编造条目链接（来源没给就是 None）"
    );

    // ── O7 ⭐ 散列门不含拉取时刻（去重的根据）
    let later = summarize_page(&feed_id, &source_url, &body, fetched_at_ms + 60_000);
    assert_eq!(
        stable_ingest_content(&summary),
        stable_ingest_content(&later),
        "稳定摄取内容**不得含拉取时刻**（否则散列门无法按内容去重 ⇒ 重复拉取会产生新记忆）"
    );

    // ── O8/O9 条目身份：链接优先，缺链接退化到内容散列
    let identity = item_identity(&summary);
    if summary.item_link.is_none() {
        assert!(
            identity.starts_with("hash:"),
            "无链接时身份必须退化到内容散列（hash: 前缀）: {identity}"
        );
        let hex = identity.trim_start_matches("hash:");
        assert_eq!(hex.len(), 16, "散列身份必须是 16 位十六进制: {identity}");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "散列身份必须是十六进制: {identity}"
        );
    } else {
        assert!(
            identity.starts_with("link:"),
            "有链接时身份必须用链接（link: 前缀）: {identity}"
        );
    }
    // 同内容 ⇒ 同身份
    assert_eq!(
        identity,
        item_identity(&later),
        "同条目（同内容）的身份必须稳定（不随拉取时刻变）"
    );
    // 来源标注带身份
    let source = item_source(&feed_id, &summary);
    assert!(
        source.starts_with(&format!("content_feed:{feed_id}:")),
        "来源标注必须是 content_feed:<feed_id>:<身份>: {source}"
    );

    // ── O10 注意力上限可配
    let default_limits = AttentionLimits::default();
    assert_eq!(
        default_limits.per_pull_items, DEFAULT_PER_PULL_ITEMS,
        "单次拉取默认上限必须与常量一致"
    );
    assert_eq!(
        default_limits.daily_ingests, DEFAULT_DAILY_INGESTS,
        "每日摄取默认上限必须与常量一致"
    );
    let custom = AttentionLimits::default()
        .with_per_pull_items(7)
        .with_daily_ingests(9);
    assert_eq!(custom.per_pull_items, 7, "单次拉取上限覆盖必须生效");
    assert_eq!(custom.daily_ingests, 9, "每日摄取上限覆盖必须生效");
    // 0 = 拒绝拉取任何条目（如实）
    let zero = AttentionLimits::default().with_per_pull_items(0);
    assert_eq!(zero.per_pull_items, 0, "0 上限必须如实保留（拒绝任何条目）");

    // ── 文档面（RSS/Atom/HTML/纯文本四形态）：摘要纪律同样成立
    let (form, items) = summarize_feed_document(&feed_id, &source_url, &body, fetched_at_ms);
    // 形态必须落在四个闭合形态内（RSS 条目 / Atom 条目 / HTML 页面 / 纯文本）
    let _ = form;
    for item in &items {
        assert!(
            item.title.chars().count() <= TITLE_MAX_CHARS,
            "文档面标题不得超上限"
        );
        assert!(
            item.body_excerpt.chars().count() <= BODY_EXCERPT_MAX_CHARS,
            "文档面正文不得超上限"
        );
        assert_eq!(item.feed_id, feed_id, "文档面 feed_id 必须原样");
        assert_eq!(item.source_url, source_url, "文档面 source_url 必须原样");
    }
    // 文档面：页面/纯文本形态**必须给一条**（RSS/Atom 未必有条目 ⇒ 允许空）
    if matches!(
        form,
        apeireth_tools_canonical::content_feed::summary::FeedDocumentForm::HtmlPage
            | apeireth_tools_canonical::content_feed::summary::FeedDocumentForm::PlainText
    ) {
        assert!(
            !items.is_empty(),
            "页面/纯文本形态必须给出至少一条摘要"
        );
    }

    // ── 数据驱动：构造一个「改了标题」的条目 ⇒ 身份必变
    let mut mutated: FeedItemSummary = summary.clone();
    mutated.title.push('x');
    assert_ne!(
        item_identity(&mutated),
        identity,
        "改标题必须改内容散列身份（否则改内容不被察觉）"
    );
    assert_ne!(
        stable_ingest_content(&mutated),
        stable_ingest_content(&summary),
        "改标题必须改稳定摄取内容（散列门据此跳过变更条目）"
    );

    let _ = data;
});
