#![no_main]

//! fuzz: **fetch 出网面 + image_ops 图像面** —— 本仓此前 **0 覆盖** 的两个面。
//!
//! 被测面（仓内源文件）：
//! - `crates/capabilities/tools/src/fetch.rs`（Fetch 工具的 URL 闸 / 冻结载荷）
//! - `crates/capabilities/tools/src/fetch/html_text.rs`（HTML → 纯文本 / 链接 / 标题）
//! - `crates/capabilities/tools/src/fetch/accessibility.rs`（HTML → 无障碍树 / 快照）
//! - `crates/capabilities/tools/src/fetch/rate_limit.rs`（每主机滑窗限流）
//! - `crates/capabilities/tools/src/fetch/response_cache.rs`（TTL + LRU + 字节预算缓存）
//! - `crates/capabilities/tools/src/image_ops.rs`（图像操作面：词表 / 参数 / 读面 / 炸弹图三层 / 写面）
//!
//! ## 为什么值得 fuzz（docs/02-guides/fuzz-target-authoring.md §1）
//!
//! - **吃不受控输入**：fetch 的 URL 来自 LLM 生成参数；fetch 抽的 HTML 来自**被抓取的
//!   远端页面**（完全不可信）；image_ops 读的是工作区里的图片文件（内容可被构造）。
//! - **有强承诺**：fetch 声明了 scheme 白名单 / userinfo 拒 / 长度上限 / 片段不下发；
//!   image_ops 声明了**闭合操作词表**、**三层炸弹图上限**、**格式白名单**、
//!   **工作区限域（词法层 + 现实层）**、**路径脱敏**。
//! - **接线了**：`tool.fetch` 与 `tool.image_ops` 都是注册式工具能力。
//!
//! ## 判据纪律（每条断言都能从被测代码自己写明的契约推出来）
//!
//! 本 target **只**用「被测函数自己写明的契约」当判据；**不自造期望**。
//! 典型反例（已在本仓出现 9 次「以为发现缺陷、最后证明是自己断言写错」）在本 target 里
//! 被显式规避：
//!
//! - `extract_text` 的输出**允许**含 `<` / `>`（`&lt;` / `&gt;` 会被解码）⇒ 不写
//!   「输出不含尖括号」这种自造断言（docs/02-guides/fuzz-target-authoring.md §2.1 的原例）；
//! - `fetch` **不**做 IP 分类（`fetch.rs:17`「does not duplicate IP classification」）⇒
//!   不写「fetch 必须拒掉 `http://2130706433/`」；地址分类归 `egress` 面
//!   （该面已有 `fuzz/fuzz_targets/egress.rs` 覆盖，本 target 不重复造）。
//! - `extract_links` 的链接**文本**可能被截在多字节字符中间（`from_utf8_lossy` 替换）⇒
//!   只断言**链接 URL** 是输入子串（其两端必然落在 ASCII 边界上），不断言文本。
//!
//! ## 网络纪律（不发真请求）
//!
//! - fetch 面只碰**纯函数层**：`FetchTool::freeze_invocation`（只读参数 + 归一化 URL，
//!   **不触网**）、HTML 抽取、无障碍树、限流器、缓存。
//! - `ControlledEgress::get*` **一律不调用**（那需要真网络）。
//! - image_ops 面的端到端跑在**本次运行独有的临时工作区**里（无网络、无共享目录）。
//!
//! ## 诚实边界（未覆盖的部分）
//!
//! - 真网络路径（DNS 解析 / 逐跳重定向重验 / TLS / 代理策略 / 响应体限流）**未覆盖**：
//!   `egress` 面另有 target 覆盖判定层，但**逐跳重验的端到端链路本 target 没跑**。
//! - `image_ops` 的 32 MiB 输入字节上限（`MAX_INPUT_BYTES`）**未覆盖**（需要 32 MiB 文件，
//!   fuzz 成本不划算）；覆盖的是解码期维度/像素上限与解码后像素复核。
//! - `layers_composite` / `psd_read_info` 的**像素/图层**路径未覆盖（需要第二张图 / PSD 夹具），
//!   覆盖的是它们的**参数校验层**。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use libfuzzer_sys::fuzz_target;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::content_feed::store::validate_feed_url;
use apeireth_tools_canonical::egress::{EgressDestination, EgressPolicy};
use apeireth_tools_canonical::fetch::accessibility::{
    AccessibilityNode, AccessibilityTree, MAX_SNAPSHOT_DEPTH, MAX_TREE_DEPTH, NodeRole,
    extract_tree,
};
use apeireth_tools_canonical::fetch::html_text::{
    HtmlExtractError, extract_links, extract_text, extract_title,
};
use apeireth_tools_canonical::fetch::rate_limit::RateLimiter;
use apeireth_tools_canonical::fetch::response_cache::ResponseCache;
use apeireth_tools_canonical::fetch::{FETCH_URL_MAX_BYTES, FetchConfig, FetchTool};
use apeireth_tools_canonical::image_ops::{
    ImageFormatKind, ImageOperation, ImageOpsActivity, ImageOpsParams, ImageOpsTool, MAX_IMAGE_DIM,
    MAX_PIPELINE_STEPS, MAX_PSD_LAYER_NAMES, MAX_WATERMARK_SCALE, MAX_WATERMARK_TEXT_CHARS,
    OPERATION_VOCABULARY, OperationSpec, SUPPORTED_FORMATS,
};
use apeireth_tools_canonical::observed_gate::ObservedGate;

// ─────────────────────────────── 输入游标 ───────────────────────────────

/// 字节游标：只按字节切（永不在多字节字符中间切），故无 char 边界风险。
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> &'a [u8] {
        let start = self.pos.min(self.data.len());
        let end = (start + n).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }

    fn u8(&mut self) -> u8 {
        self.take(1).first().copied().unwrap_or(0)
    }

    fn u16(&mut self) -> u16 {
        let bytes = self.take(2);
        let mut buf = [0u8; 2];
        for (slot, value) in buf.iter_mut().zip(bytes.iter()) {
            *slot = *value;
        }
        u16::from_le_bytes(buf)
    }
}

// ───────────────────────────── 面 1: fetch URL 闸 ─────────────────────────────

/// 取 URL 的 scheme（不含冒号）；无 scheme 返回 `None`。
///
/// 口径与 WHATWG URL 的 scheme 语法一致：`[a-zA-Z][a-zA-Z0-9+\-.]*` 后紧跟 `:`。
/// 前导 C0 控制符与空格会被 URL 解析器忽略，故先裁掉。
fn scheme_of(raw: &str) -> Option<&str> {
    let trimmed = raw.trim_start_matches(|c: char| c <= ' ');
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let mut index = 1usize;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric()
            || bytes[index] == b'+'
            || bytes[index] == b'-'
            || bytes[index] == b'.')
    {
        index += 1;
    }
    if index < bytes.len() && bytes[index] == b':' {
        Some(&trimmed[..index])
    } else {
        None
    }
}

/// 取 `scheme://authority` 形态 URL 的 authority；非该形态返回 `None`。
fn authority_of(raw: &str) -> Option<&str> {
    let trimmed = raw.trim_start_matches(|c: char| c <= ' ');
    let rest = trimmed.split_once("://")?.1;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// fetch 面：URL 闸 + 冻结载荷契约（**不触网**）。
///
/// 契约出处（`crates/capabilities/tools/src/fetch.rs`）：
/// - `:41-42` `FETCH_URL_MAX_BYTES` = 8 KiB，「Maximum accepted URL length in bytes.」
/// - `:240-245` 空 URL 必拒（帧含 `must not be empty`）
/// - `:246-255` 超长 URL 必拒（帧含 `the maximum is 8192 bytes`）
/// - `:262-271` scheme 只认 http/https（帧含 `only http and https are supported`）
/// - `:272-278` userinfo 必拒（帧含 `userinfo`）
/// - `:279-284` 必须带主机
/// - `:286-288`「Fragments are never sent over HTTP. Strip before freezing/execution」
/// - `:221-229` 只认 `url` 一个参数
/// - `:45` 冻结方法恒为 `GET`
fn fetch_url_face(data: &[u8]) {
    let raw = String::from_utf8_lossy(data);
    let tool = FetchTool::new(FetchConfig::deny_all());
    let call = ToolCall {
        id: "fuzz_fetch".to_string(),
        name: "fetch".to_string(),
        arguments: serde_json::json!({ "url": raw.as_ref() }),
    };

    match tool.freeze_invocation(&call) {
        Ok(Some(frozen)) => {
            let payload = frozen.payload;
            assert_eq!(
                payload.get("method").and_then(|v| v.as_str()),
                Some("GET"),
                "冻结载荷的方法必须恒为 GET (fetch.rs:45)"
            );
            let url = payload
                .get("url")
                .and_then(|v| v.as_str())
                .expect("冻结载荷必须带 url 字符串 (fetch.rs:302-311)");

            // 契约：片段永不下发（fetch.rs:286-288）。
            assert!(
                !url.contains('#'),
                "冻结 URL 不得含片段 (fetch.rs:286-288): {url}"
            );
            // 契约：scheme 只认 http/https（fetch.rs:262-271）。
            let scheme = scheme_of(url).unwrap_or("");
            assert!(
                scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"),
                "冻结 URL 的 scheme 必须是 http/https (fetch.rs:262-271): {url}"
            );
            // 契约：必须有主机、不得带 userinfo（fetch.rs:272-284）。
            let authority = authority_of(url).expect("冻结 URL 必须是 scheme://authority 形态");
            assert!(
                !authority.is_empty(),
                "冻结 URL 必须带主机 (fetch.rs:279-284): {url}"
            );
            assert!(
                !authority.contains('@'),
                "冻结 URL 不得带 userinfo (fetch.rs:272-278): {url}"
            );

            // 交接面：冻结的 URL 必须能被受控出口解析成目的地。
            // URL 解析器借用同 crate 的公开 API `validate_feed_url`（与 fetch 同一套
            // URL 口径），避免在 harness 里重写一遍 URL 解析（同义反复）。
            if let Ok(parsed) = validate_feed_url(url, &EgressPolicy::Unrestricted) {
                let destination = EgressDestination::parse(&parsed)
                    .expect("冻结的 URL 必须是受控出口可解析的目的地 (egress.rs:293-315)");
                assert!(
                    destination.scheme == "http" || destination.scheme == "https",
                    "目的地 scheme 必须仍是 http/https (egress.rs:294-296)"
                );
                assert!(
                    !destination.host.is_empty(),
                    "目的地必须有主机 (egress.rs:302-304)"
                );
            }
        }
        Ok(None) => panic!("fetch 必须冻结出调用载荷 (fetch.rs:613-624)"),
        Err(result) => {
            let text = result.render();
            // 契约优先级与实现顺序一致（fetch.rs:239-290：长度 → **解析** →
            // scheme → userinfo → 主机）。解析在 scheme 之前 ⇒ 「URL 根本解析
            // 不了」时帧里是解析理由，不是 scheme 理由。
            //
            // 「能不能解析 / scheme 合不合规」用同 crate 的 content_feed 面
            // （`validate_feed_url` 与 fetch 同一套 URL 口径）作判据：它的拒绝
            // 文案按理由分类，可据此判定 fetch 该落在哪一条理由上。
            if raw.len() > FETCH_URL_MAX_BYTES {
                assert!(
                    text.contains("maximum"),
                    "超长 URL 的拒绝帧必须说明上限 (fetch.rs:246-255): {text}"
                );
            } else if raw.is_empty() {
                assert!(
                    text.contains("must not be empty"),
                    "空 URL 的拒绝帧必须说明非空要求 (fetch.rs:240-245): {text}"
                );
            } else {
                match validate_feed_url(&raw, &EgressPolicy::Unrestricted) {
                    Err(message) if message.starts_with("unsupported feed url scheme") => {
                        assert!(
                            text.contains("scheme"),
                            "非 http(s) scheme 的拒绝帧必须点名 scheme (fetch.rs:262-271): {text}"
                        );
                    }
                    Err(message) if message.starts_with("invalid feed url:") => {
                        assert!(
                            text.contains("invalid fetch url"),
                            "解析不了的 URL 的拒绝帧必须说明解析失败 (fetch.rs:257-260): {text}"
                        );
                    }
                    Err(message) if message.contains("userinfo") => {
                        assert!(
                            text.contains("userinfo"),
                            "带 userinfo 的拒绝帧必须点名 userinfo (fetch.rs:272-278): {text}"
                        );
                    }
                    Err(message) if message.contains("must include a host") => {
                        assert!(
                            text.contains("host"),
                            "缺主机的拒绝帧必须点名 host (fetch.rs:279-284): {text}"
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    // 契约：只认 `url` 一个参数（fetch.rs:221-229）。
    let extra = ToolCall {
        id: "fuzz_fetch".to_string(),
        name: "fetch".to_string(),
        arguments: serde_json::json!({ "url": raw.as_ref(), "method": "POST" }),
    };
    let rejected = tool
        .freeze_invocation(&extra)
        .err()
        .expect("多余参数必须拒绝 (fetch.rs:221-229)");
    assert!(
        rejected.render().contains("unknown fetch parameter"),
        "多余参数的拒绝帧必须点名未知参数 (fetch.rs:221-229): {}",
        rejected.render()
    );
}

// ───────────────────────────── 面 2: HTML 文本抽取 ─────────────────────────────

/// 造一个只含字母数字的标记串（可安全放进 `<script>` 体与普通文本里）。
fn marker(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        let c = (b'A' + (byte % 26)) as char;
        out.push(c);
    }
    if out.is_empty() {
        out.push('X');
    }
    out
}

/// fetch/html_text 面：纯文本 / 链接 / 标题。
///
/// 契约出处（`crates/capabilities/tools/src/fetch/html_text.rs`）：
/// - `:1-8` 模块头：「script/style content is skipped, block-level tags become
///   newlines, named and numeric HTML entities are decoded, and text is
///   whitespace-collapsed」
/// - `:29-33` 空输入（trim 后）⇒ `Empty`
/// - `:128-133` 输出是空白折叠后的（`split_whitespace().join(" ")`）
/// - `:136-137`「Extract `(url, link_text)` pairs from `href="..."` anchors.」
/// - `:167-168`「Extract the `<title>` element content, when present.」
fn html_text_face(data: &[u8]) {
    let html = String::from_utf8_lossy(data);

    // ① 纯文本契约
    match extract_text(&html) {
        Ok(text) => {
            assert!(
                !text.is_empty(),
                "Ok 的抽取文本不得为空 (html_text.rs:128-133)"
            );
            assert_eq!(
                text.trim(),
                text,
                "抽取文本不得有首尾空白 (空白折叠契约 html_text.rs:1-8): {text:?}"
            );
            assert!(
                !text.contains("  "),
                "抽取文本不得有连续空白 (空白折叠契约 html_text.rs:1-8): {text:?}"
            );
            for ch in text.chars() {
                assert!(
                    !ch.is_whitespace() || ch == ' ',
                    "抽取文本的空白只能是单空格 (html_text.rs:128-133): {text:?}"
                );
            }
        }
        Err(HtmlExtractError::Empty) => assert!(
            html.trim().is_empty(),
            "Empty 只在输入 trim 后为空时出现 (html_text.rs:31-33)"
        ),
        Err(HtmlExtractError::NoText) => assert!(
            !html.trim().is_empty(),
            "NoText 不得在输入 trim 后为空时出现 (html_text.rs:31-33)"
        ),
    }

    // ② 链接契约：链接 URL 必须是输入的子串。
    //    `href="` 与收尾 `"` 都是 ASCII ⇒ 切片两端必落在 char 边界上 ⇒
    //    `from_utf8_lossy` 不会引入替换字符 ⇒ URL 必是原样子串。
    for (url, _text) in extract_links(&html) {
        assert!(
            html.contains(url.as_str()),
            "抽出的链接 URL 必须是输入的子串 (html_text.rs:136-137): {url:?}"
        );
    }

    // ③ 标题契约：`<title>` 内容必须来自输入。
    //    注意：标题切片的**收尾**边界可能落在多字节字符中间（文档末尾没有
    //    `</title>` 时，收尾点 = len - 8），此时 `from_utf8_lossy` 产出替换字符
    //    U+FFFD ⇒ 允许「不是原文子串」。模块头已声明病态标记会被误解析
    //    （html_text.rs:5-8），故这不是库缺陷，是断言要按实现口径写。
    if let Some(title) = extract_title(&html) {
        assert!(
            html.contains(title.as_str()) || title.contains('\u{FFFD}'),
            "抽出的标题必须来自输入 (html_text.rs:167-168): {title:?}"
        );
        assert!(
            !title.contains("</title>"),
            "标题内容不得含闭合标签 (html_text.rs:167-168): {title:?}"
        );
    }

    // ④ script/style 内容跳过契约（模块头 html_text.rs:5-6）。
    //    用两个**必然不同**的标记：体内一个、体外一个；并做 harness 自检
    //    （体外的标记必须出现在输出里 ⇒ 断言不是空转）。
    let inside = format!("{}IN", marker(&data[..data.len().min(8)]));
    let outside = format!("{}OUT", marker(&data[data.len().saturating_sub(8)..]));
    let script_html = format!("<script>{inside}</script><style>{inside}</style><p>{outside}</p>");
    let text = extract_text(&script_html).unwrap_or_default();
    assert!(
        !text.contains(inside.as_str()),
        "script/style 内容必须被跳过 (html_text.rs:5-6): {text:?}"
    );
    assert!(
        text.contains(outside.as_str()),
        "harness 自检：script 外的文本必须存活 (否则上面的断言是空转): {text:?}"
    );
}

// ───────────────────────────── 面 3: 无障碍树 ─────────────────────────────

fn is_interactive_role(role: &NodeRole) -> bool {
    matches!(
        role,
        NodeRole::Link
            | NodeRole::Button
            | NodeRole::Textbox
            | NodeRole::Checkbox
            | NodeRole::Radio
            | NodeRole::Combobox
    )
}

/// fetch/accessibility 面：无障碍树 + 快照。
///
/// 契约出处（`crates/capabilities/tools/src/fetch/accessibility.rs`）：
/// - `:15-19` 解析期树深上限 `MAX_TREE_DEPTH`，超出整棵丢弃并置 `truncated`
/// - `:21-23` 渲染期深度上限 `MAX_SNAPSHOT_DEPTH`
/// - `:101-104` `truncated` 标记语义：「解析或渲染因深度上限截断过子树」
/// - `:129-131` 快照 = 2 空格缩进的层级文本
/// - `:140-150` 渲染超深即截断并标记
/// - `:174-176` `find_by_ref` 按 ref id 找回节点
/// - `:181-183` `interactive_refs` = 交互角色节点、文档顺序
fn accessibility_face(data: &[u8]) {
    let html = String::from_utf8_lossy(data);
    let tree = extract_tree(&html);

    // ① 子节点索引必须自洽。
    for node in &tree.nodes {
        for child in &node.children {
            assert!(
                *child < tree.nodes.len(),
                "children 索引越界（指向不存在的节点）"
            );
        }
    }

    // ② ref id 必须唯一，且 `find_by_ref` 必须能找回该节点。
    let mut seen: HashSet<&str> = HashSet::new();
    for node in &tree.nodes {
        if let Some(ref_id) = node.ref_id.as_deref() {
            assert!(
                seen.insert(ref_id),
                "ref id 必须唯一 (accessibility.rs:84-85, :440-444): {ref_id}"
            );
            let found = tree
                .find_by_ref(ref_id)
                .expect("find_by_ref 必须能按 ref id 找回节点 (accessibility.rs:174-176)");
            assert_eq!(
                found.tag, node.tag,
                "find_by_ref 必须找回同一个节点 (accessibility.rs:174-176): {ref_id}"
            );
        }
    }

    // ③ `interactive_refs` 必须与节点表一致（交互角色 + 有 ref id + 文档顺序）。
    let expected: Vec<(String, NodeRole, String)> = tree
        .nodes
        .iter()
        .filter(|node| is_interactive_role(&node.role) && node.ref_id.is_some())
        .map(|node| {
            (
                node.ref_id.clone().expect("filtered"),
                node.role.clone(),
                node.name.clone(),
            )
        })
        .collect();
    assert_eq!(
        tree.interactive_refs(),
        expected,
        "interactive_refs 必须与节点表一致且按文档顺序 (accessibility.rs:181-183)"
    );

    // ④ 快照深度必须有界（渲染期上限 accessibility.rs:140-150）。
    let snapshot = tree.to_snapshot();
    let max_depth = snapshot
        .lines()
        .map(|line| line.len() - line.trim_start_matches(' ').len())
        .max()
        .unwrap_or(0)
        / 2;
    assert!(
        max_depth <= MAX_SNAPSHOT_DEPTH,
        "快照深度必须 ≤ MAX_SNAPSHOT_DEPTH ({MAX_SNAPSHOT_DEPTH})，实得 {max_depth}"
    );

    // ⑤ 渲染期截断标记 ⇔ `truncated` 标记（accessibility.rs:101-104）。
    if snapshot.contains("truncated at depth") {
        assert!(
            tree.truncated,
            "快照出现渲染截断标记时 truncated 必须置位 (accessibility.rs:101-104)"
        );
    }
}

// ───────────────────────────── 面 4: 限流 + 缓存 ─────────────────────────────

/// fetch/rate_limit + fetch/response_cache 面。
///
/// 契约出处：
/// - `rate_limit.rs:44-45`「True when one more request to `host` is allowed right now.」
/// - `rate_limit.rs:59-63` 记录一次请求；超窗的旧时间戳先被裁掉
/// - `rate_limit.rs:79-81`「How long until the next request to `host` is allowed;
///   `None` when it is allowed immediately」
/// - `rate_limit.rs:96-98` 主机计数；`:100-102` 窗口内请求计数
/// - `rate_limit.rs:36-39` 上限夹到 ≥1
/// - `response_cache.rs:114-117`「An expired entry is evicted and counted as an
///   eviction plus a miss.」
/// - `response_cache.rs:146-149` 写入后 LRU 驱逐
/// - `response_cache.rs:216-221`「字节预算至少保留 1 条」
fn scheduling_face(data: &[u8]) {
    let mut cursor = Cursor::new(data);
    let limit = 1 + (cursor.u8() % 8) as usize;
    let host = format!("host-{}.example", cursor.u16());

    // 限流器：窗口取 1 小时 ⇒ 无时间抖动（不 sleep）。
    let mut limiter = RateLimiter::with_limit(limit, Duration::from_secs(3600));
    for _ in 0..limit {
        limiter.record(&host);
    }
    assert!(
        !limiter.check(&host),
        "窗口内记录数达上限后必须拒绝 (rate_limit.rs:44-45)"
    );
    assert_eq!(
        limiter.count(&host),
        limit,
        "窗口内计数必须等于记录数 (rate_limit.rs:100-102)"
    );
    assert_eq!(limiter.hosts(), 1, "只跟踪了一个主机 (rate_limit.rs:96-98)");
    assert!(
        limiter.check("other-host.example"),
        "不同主机必须相互独立 (rate_limit.rs:3-7)"
    );
    assert!(
        limiter.wait_time(&host).is_some(),
        "被拒时必须给出退避时长 (rate_limit.rs:79-81)"
    );
    assert!(
        limiter.clear_host(&host),
        "clear_host 必须报告此前是否跟踪过 (rate_limit.rs:118-121)"
    );
    assert!(limiter.check(&host), "清除计数后必须重新放行");
    assert_eq!(limiter.hosts(), 0, "清除后不得再跟踪该主机");

    // 上限 0 被夹到 1（rate_limit.rs:36-39）。
    let mut clamped = RateLimiter::with_limit(0, Duration::from_secs(3600));
    clamped.record(&host);
    assert!(
        !clamped.check(&host),
        "上限 0 必须被夹到 1 (rate_limit.rs:36-39)"
    );

    // 缓存：条目数上限 + LRU。
    let cache = ResponseCache::new(Duration::from_secs(3600)).with_max_entries(2);
    cache.put("k1", serde_json::json!(1));
    cache.put("k2", serde_json::json!(2));
    cache.put("k3", serde_json::json!(3));
    assert!(
        cache.stats().size <= 2,
        "条目数必须受 max_entries 约束 (response_cache.rs:21-22, :222-233)"
    );
    assert!(
        cache.get("k1").is_none(),
        "最久未用的条目必须被 LRU 淘汰 (response_cache.rs:146-149)"
    );
    assert!(
        cache.get("k3").is_some(),
        "最新写入的条目必须存活 (response_cache.rs:146-149)"
    );
    let hits_before = cache.stats().hits;
    cache.get("k3");
    assert_eq!(
        cache.stats().hits,
        hits_before + 1,
        "命中必须计数 (response_cache.rs:128-134)"
    );
    let misses_before = cache.stats().misses;
    assert!(cache.get("absent").is_none());
    assert_eq!(
        cache.stats().misses,
        misses_before + 1,
        "未命中必须计数 (response_cache.rs:142)"
    );

    // 过期条目：驱逐 + 计一次 miss（response_cache.rs:114-117）。
    let expiring = ResponseCache::new(Duration::ZERO);
    expiring.put("k", serde_json::json!(1));
    assert!(
        expiring.get("k").is_none(),
        "过期条目不得返回 (response_cache.rs:114-117)"
    );
    assert_eq!(
        expiring.stats().evictions,
        1,
        "过期条目必须计一次驱逐 (response_cache.rs:114-117)"
    );
    assert_eq!(
        expiring.stats().misses,
        1,
        "过期条目必须计一次未命中 (response_cache.rs:114-117)"
    );
    assert_eq!(expiring.stats().size, 0, "过期条目必须被驱逐");

    // 字节预算：至少保留 1 条，且不无界增长（response_cache.rs:216-221）。
    let payload = serde_json::json!({ "payload": "y".repeat(4096) });
    let one = serde_json::to_vec(&payload).map(|b| b.len()).unwrap_or(0);
    let bounded = ResponseCache::new(Duration::from_secs(3600))
        .with_max_entries(64)
        .with_max_total_bytes(one * 2);
    bounded.put("a", payload.clone());
    bounded.put("b", payload.clone());
    bounded.put("c", payload.clone());
    assert!(
        bounded.stats().size <= 2,
        "字节预算必须约束条目数 (response_cache.rs:24-25, :222-233)"
    );
    assert!(
        bounded.stats().size >= 1,
        "字节预算至少保留一条 (response_cache.rs:218-221)"
    );
}

// ───────────────────────────── 面 5: image_ops 参数面 ─────────────────────────────

/// 每个操作的参数集合（逐操作闭合格）。
///
/// 出处：`image_ops.rs` 的 `require_exact` 调用点 —— `:347`(resize)、`:356`(crop)、
/// `:369`(rotate)、`:381`(flip)、`:396`(format_convert)、`:407`(brightness)、
/// `:420`(contrast)、`:429`(grayscale)、`:433`(blur)、`:439`(sharpen)、
/// `:454`(watermark_text)、`:484`(layers_composite)、`:508`(psd_read_info)。
const GOLDEN_FIELDS: [(&str, &[&str]); 13] = [
    ("resize", &["width", "height"]),
    ("crop", &["x", "y", "width", "height"]),
    ("rotate", &["angle_deg"]),
    ("flip", &["axis"]),
    ("format_convert", &["format"]),
    ("brightness", &["amount"]),
    ("contrast", &["factor"]),
    ("grayscale", &[]),
    ("blur", &["sigma"]),
    ("sharpen", &["sigma", "threshold"]),
    ("watermark_text", &["text", "x", "y", "scale"]),
    ("layers_composite", &["overlay", "x", "y", "opacity"]),
    ("psd_read_info", &["path"]),
];

fn golden_fields(op: &str) -> Option<&'static [&'static str]> {
    GOLDEN_FIELDS
        .iter()
        .find(|(name, _)| *name == op)
        .map(|(_, fields)| *fields)
}

/// `OperationSpec` 里「出现过」的字段名（镜像 `image_ops.rs:601-652` 的
/// `present_fields`，但只读公开字段，不调用被测私有函数）。
fn present_fields(spec: &OperationSpec) -> Vec<&'static str> {
    let mut present = Vec::new();
    if spec.width.is_some() {
        present.push("width");
    }
    if spec.height.is_some() {
        present.push("height");
    }
    if spec.x.is_some() {
        present.push("x");
    }
    if spec.y.is_some() {
        present.push("y");
    }
    if spec.angle_deg.is_some() {
        present.push("angle_deg");
    }
    if spec.axis.is_some() {
        present.push("axis");
    }
    if spec.format.is_some() {
        present.push("format");
    }
    if spec.amount.is_some() {
        present.push("amount");
    }
    if spec.factor.is_some() {
        present.push("factor");
    }
    if spec.sigma.is_some() {
        present.push("sigma");
    }
    if spec.threshold.is_some() {
        present.push("threshold");
    }
    if spec.text.is_some() {
        present.push("text");
    }
    if spec.scale.is_some() {
        present.push("scale");
    }
    if spec.overlay.is_some() {
        present.push("overlay");
    }
    if spec.opacity.is_some() {
        present.push("opacity");
    }
    if spec.path.is_some() {
        present.push("path");
    }
    present
}

/// 构造一份 `OperationSpec`：参数值来自 fuzz 字节。
///
/// `valid = true` 时各参数落在契约范围内（走放行路径）；`valid = false` 时
/// 每个参数各取一个越界/畸形值（走拒绝路径）。两版消耗同样的字节序列。
fn build_spec(cursor: &mut Cursor, op: &str, valid: bool) -> OperationSpec {
    let mut spec = OperationSpec {
        op: op.to_string(),
        ..Default::default()
    };

    let b0 = cursor.u8();
    let b1 = cursor.u8();
    let b2 = cursor.u8();
    let b3 = cursor.u8();
    let b4 = cursor.u8();
    let b5 = cursor.u8();
    let b6 = cursor.u8();
    let b7 = cursor.u8();
    let w0 = cursor.u16();
    let w1 = cursor.u16();
    let raw_text = cursor.take(1 + (b7 as usize % 8)).to_vec();

    // 可打印 ASCII 文本（水印文字契约：1..=64 字符且只收 0x20..=0x7E）。
    let mut text = String::new();
    for byte in raw_text {
        text.push((b' ' + (byte % 95)) as char);
    }
    if text.is_empty() {
        text.push('A');
    }

    spec.width = Some(if valid {
        1 + (b0 as i64 % 512)
    } else {
        i64::MAX
    });
    spec.height = Some(if valid { 1 + (b1 as i64 % 512) } else { 0 });
    spec.x = Some(if valid { b2 as i64 % 512 } else { -1 });
    spec.y = Some(if valid { b3 as i64 % 512 } else { i64::MAX });
    spec.angle_deg = Some(if valid {
        [90i64, 180, 270][(b4 % 3) as usize]
    } else {
        45
    });
    spec.axis = Some(if valid {
        ["horizontal", "vertical"][(b5 % 2) as usize].to_string()
    } else {
        "sideways".to_string()
    });
    spec.format = Some(if valid {
        ["png", "jpeg", "jpg", "webp", "bmp"][(b6 % 5) as usize].to_string()
    } else {
        "gif".to_string()
    });
    spec.amount = Some(if valid { b0 as i64 % 511 - 255 } else { 256 });
    spec.factor = Some(if valid {
        (w0 as f64 % 301.0) / 100.0
    } else {
        f64::NAN
    });
    spec.sigma = Some(if valid {
        0.5 + (w1 as f64 % 320.0) / 10.0
    } else {
        0.0
    });
    spec.threshold = Some(if valid { b1 as i64 % 256 } else { 256 });
    spec.text = Some(if valid { text } else { String::new() });
    spec.scale = Some(if valid {
        1 + (b2 as i64 % MAX_WATERMARK_SCALE as i64)
    } else {
        MAX_WATERMARK_SCALE as i64 + 1
    });
    spec.overlay = Some(if valid {
        "overlay.png".to_string()
    } else {
        String::new()
    });
    spec.opacity = Some(if valid {
        (w0 as f64 % 101.0) / 100.0
    } else {
        f64::INFINITY
    });
    spec.path = Some(if valid {
        "layer.psd".to_string()
    } else {
        String::new()
    });
    spec
}

/// image_ops 参数面：闭合词表 + 逐参数范围。
///
/// 契约出处（`crates/capabilities/tools/src/image_ops.rs`）：
/// - `:7-12`「操作词表 (闭合)……表外操作名与逐操作多余/缺失参数一律**拒绝即帧**」
/// - `:340-342`「词表外名 / 缺参 / 多余参 / 越界值一律拒绝即帧; 校验先于任何 IO」
/// - `:525-534` 边长 1..=MAX_IMAGE_DIM；`:536-545` 坐标 0..=MAX_IMAGE_DIM
/// - `:547-555` sigma 有限且 0<σ<=32
/// - `:368-378` rotate 只认 90/180/270；`:380-393` flip 只认 horizontal/vertical
/// - `:406-416` brightness -255..=255；`:418-427` contrast 0.0..=3.0 有限
/// - `:438-451` sharpen threshold 0..=255
/// - `:453-481` watermark 文字 1..=64 字符且只收可打印 ASCII；scale 1..=8
/// - `:483-506` layers_composite opacity 0.0..=1.0 有限；overlay 非空
/// - `:507-516` psd_read_info path 非空
fn image_ops_params_face(data: &[u8]) {
    let mut cursor = Cursor::new(data);
    let vocab_index = (cursor.u8() as usize) % (OPERATION_VOCABULARY.len() + 2);
    let op_name = if vocab_index < OPERATION_VOCABULARY.len() {
        OPERATION_VOCABULARY[vocab_index].to_string()
    } else {
        // 表外操作名（另加大小写/空白变体，验证归一化口径）。
        ["RESIZE", "  resize  ", "resize2", "rotate ", "Rotate"]
            [(vocab_index - OPERATION_VOCABULARY.len()) as usize]
            .to_string()
    };
    let in_vocab = OPERATION_VOCABULARY
        .iter()
        .any(|name| *name == op_name.trim().to_ascii_lowercase());

    for valid in [true, false] {
        let spec = build_spec(&mut cursor, &op_name, valid);
        let parsed = ImageOperation::parse(&spec);
        match parsed {
            Ok(operation) => {
                assert!(
                    in_vocab,
                    "表外操作名必须拒绝即帧 (image_ops.rs:7-12): {op_name}"
                );
                assert_eq!(
                    operation.name(),
                    op_name.trim().to_ascii_lowercase(),
                    "解析出的操作名必须是词表名 (image_ops.rs:321-337)"
                );
                let golden = golden_fields(operation.name()).expect("词表内操作必须有参数集合");
                let mut present = present_fields(&spec);
                present.sort_unstable();
                let mut expected: Vec<&str> = golden.to_vec();
                expected.sort_unstable();
                assert_eq!(
                    present, expected,
                    "解析成功时给出的字段必须恰好是该操作的字段集 (image_ops.rs:654-678)"
                );
                assert_operation_ranges(&operation);
            }
            Err(error) => {
                assert_eq!(
                    error.code(),
                    "image_ops.param_rejected",
                    "参数面拒绝必须是 param_rejected (image_ops.rs:726-727, :752-766): {error:?}"
                );
            }
        }
    }

    // 契约：表外操作名必拒（image_ops.rs:7-12, :517-520）。
    if !in_vocab {
        let spec = build_spec(&mut cursor, &op_name, true);
        assert!(
            ImageOperation::parse(&spec).is_err(),
            "表外操作名必须拒绝即帧 (image_ops.rs:7-12): {op_name}"
        );
    }

    // 契约：多余参数必拒（image_ops.rs:654-665）。
    let spec = build_spec(&mut cursor, &op_name, true);
    let normalized_op = op_name.trim().to_ascii_lowercase();
    let golden = golden_fields(normalized_op.as_str()).unwrap_or(&[]);
    let extra_field = [
        "width",
        "height",
        "x",
        "y",
        "angle_deg",
        "axis",
        "format",
        "amount",
        "factor",
        "sigma",
        "threshold",
        "text",
        "scale",
        "overlay",
        "opacity",
        "path",
    ]
    .into_iter()
    .find(|field| !golden.contains(field))
    .expect("总有不在参数集里的字段");
    let mut extra = spec.clone();
    match extra_field {
        "width" => extra.width = Some(1),
        "height" => extra.height = Some(1),
        "x" => extra.x = Some(0),
        "y" => extra.y = Some(0),
        "angle_deg" => extra.angle_deg = Some(90),
        "axis" => extra.axis = Some("horizontal".to_string()),
        "format" => extra.format = Some("png".to_string()),
        "amount" => extra.amount = Some(0),
        "factor" => extra.factor = Some(1.0),
        "sigma" => extra.sigma = Some(1.0),
        "threshold" => extra.threshold = Some(0),
        "text" => extra.text = Some("x".to_string()),
        "scale" => extra.scale = Some(1),
        "overlay" => extra.overlay = Some("o.png".to_string()),
        "opacity" => extra.opacity = Some(1.0),
        "path" => extra.path = Some("p.psd".to_string()),
        _ => unreachable!(),
    }
    assert!(
        ImageOperation::parse(&extra).is_err(),
        "多余参数必须拒绝即帧 (image_ops.rs:654-665): 多余字段 {extra_field}"
    );

    // 契约：缺参必拒（image_ops.rs:666-677）—— 只对有必填参数的操作做。
    if !golden.is_empty() {
        let mut missing = spec.clone();
        missing.width = None;
        missing.height = None;
        missing.x = None;
        missing.y = None;
        missing.angle_deg = None;
        missing.axis = None;
        missing.format = None;
        missing.amount = None;
        missing.factor = None;
        missing.sigma = None;
        missing.threshold = None;
        missing.text = None;
        missing.scale = None;
        missing.overlay = None;
        missing.opacity = None;
        missing.path = None;
        assert!(
            ImageOperation::parse(&missing).is_err(),
            "缺参数必须拒绝即帧 (image_ops.rs:666-677): {op_name}"
        );
    }

    // 契约：格式白名单（image_ops.rs:150-151, :181-190, :192-195）。
    for name in ["png", "PNG", " jpeg ", "jpg", "webp", "bmp"] {
        let kind = ImageFormatKind::from_name(name)
            .unwrap_or_else(|| panic!("白名单成员必须可解析: {name}"));
        assert!(
            SUPPORTED_FORMATS.contains(&kind.name()),
            "格式名必须落在白名单内 (image_ops.rs:150-151): {name}"
        );
        assert_eq!(
            ImageFormatKind::from_extension(name),
            Some(kind),
            "扩展名解析必须与名字解析一致 (image_ops.rs:192-195): {name}"
        );
        assert_eq!(
            ImageFormatKind::from_name(kind.name()),
            Some(kind),
            "白名单成员的名字必须是稳定拼写 (image_ops.rs:171-179): {name}"
        );
    }
    for name in ["gif", "tiff", "svg", "heic", "pdf", "jpg2", "PNG2"] {
        assert!(
            ImageFormatKind::from_name(name).is_none(),
            "白名单外格式名必须回 None (image_ops.rs:181-190): {name}"
        );
    }
}

/// 解析成功后的逐参数范围断言（出处见 `image_ops_params_face` 的契约表）。
fn assert_operation_ranges(operation: &ImageOperation) {
    match operation {
        ImageOperation::Resize { width, height } => {
            assert!(
                (1..=MAX_IMAGE_DIM).contains(width) && (1..=MAX_IMAGE_DIM).contains(height),
                "resize 边长必须落在 1..=MAX_IMAGE_DIM (image_ops.rs:525-534): {width}x{height}"
            );
        }
        ImageOperation::Crop {
            x,
            y,
            width,
            height,
        } => {
            assert!(
                (0..=MAX_IMAGE_DIM).contains(x) && (0..=MAX_IMAGE_DIM).contains(y),
                "crop 坐标必须落在 0..=MAX_IMAGE_DIM (image_ops.rs:536-545): {x},{y}"
            );
            assert!(
                (1..=MAX_IMAGE_DIM).contains(width) && (1..=MAX_IMAGE_DIM).contains(height),
                "crop 边长必须落在 1..=MAX_IMAGE_DIM (image_ops.rs:525-534): {width}x{height}"
            );
        }
        ImageOperation::Rotate { angle_deg } => {
            assert!(
                matches!(angle_deg, 90 | 180 | 270),
                "rotate 角度只认 90/180/270 (image_ops.rs:368-378): {angle_deg}"
            );
        }
        ImageOperation::Flip { axis } => {
            let _ = axis;
        }
        ImageOperation::FormatConvert { format } => {
            assert!(
                SUPPORTED_FORMATS.contains(&format.name()),
                "format_convert 目标格式必须落在白名单 (image_ops.rs:150-151): {format:?}"
            );
        }
        ImageOperation::Brightness { amount } => {
            assert!(
                (-255..=255).contains(amount),
                "brightness 增量必须落在 -255..=255 (image_ops.rs:406-416): {amount}"
            );
        }
        ImageOperation::Contrast { factor } => {
            assert!(
                factor.is_finite() && (0.0..=3.0).contains(factor),
                "contrast 因子必须是 0.0..=3.0 的有限数 (image_ops.rs:418-427): {factor}"
            );
        }
        ImageOperation::Grayscale => {}
        ImageOperation::Blur { sigma } => {
            assert!(
                sigma.is_finite() && *sigma > 0.0 && *sigma <= 32.0,
                "blur sigma 必须是 0<σ<=32 的有限数 (image_ops.rs:547-555): {sigma}"
            );
        }
        ImageOperation::Sharpen { sigma, threshold } => {
            assert!(
                sigma.is_finite() && *sigma > 0.0 && *sigma <= 32.0,
                "sharpen sigma 必须是 0<σ<=32 的有限数 (image_ops.rs:547-555): {sigma}"
            );
            assert!(
                (0..=255).contains(threshold),
                "sharpen threshold 必须落在 0..=255 (image_ops.rs:438-451): {threshold}"
            );
        }
        ImageOperation::WatermarkText { text, x, y, scale } => {
            let chars = text.chars().count();
            assert!(
                (1..=MAX_WATERMARK_TEXT_CHARS).contains(&chars),
                "水印文字长度必须落在 1..=MAX_WATERMARK_TEXT_CHARS (image_ops.rs:453-465): {chars}"
            );
            assert!(
                text.chars().all(|c| (' '..='~').contains(&c)),
                "水印文字只收可打印 ASCII (image_ops.rs:462-467): {text:?}"
            );
            assert!(
                (0..=MAX_IMAGE_DIM).contains(x) && (0..=MAX_IMAGE_DIM).contains(y),
                "水印坐标必须落在 0..=MAX_IMAGE_DIM (image_ops.rs:536-545): {x},{y}"
            );
            assert!(
                (1..=MAX_WATERMARK_SCALE).contains(scale),
                "水印放大倍数必须落在 1..=MAX_WATERMARK_SCALE (image_ops.rs:471-475): {scale}"
            );
        }
        ImageOperation::LayersComposite {
            overlay,
            x,
            y,
            opacity,
        } => {
            assert!(
                !overlay.trim().is_empty(),
                "叠加图路径不得为空 (image_ops.rs:485-490): {overlay:?}"
            );
            assert!(
                (0..=MAX_IMAGE_DIM).contains(x) && (0..=MAX_IMAGE_DIM).contains(y),
                "叠加坐标必须落在 0..=MAX_IMAGE_DIM (image_ops.rs:536-545): {x},{y}"
            );
            assert!(
                opacity.is_finite() && (0.0..=1.0).contains(opacity),
                "不透明度必须是 0.0..=1.0 的有限数 (image_ops.rs:494-499): {opacity}"
            );
        }
        ImageOperation::PsdReadInfo { path } => {
            assert!(
                !path.trim().is_empty(),
                "PSD 路径不得为空 (image_ops.rs:508-515): {path:?}"
            );
        }
    }
}

// ───────────────────────────── 面 6: image_ops 端到端 ─────────────────────────────

/// 本次运行独有的临时工作区（不写共享目录 ⇒ 不会与上一轮遗留物撞车）。
fn workspace() -> &'static Path {
    static WORKSPACE: OnceLock<PathBuf> = OnceLock::new();
    WORKSPACE.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-fuzz-fetch-image-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("create fuzz workspace");
        dir
    })
}

/// 工作区**外面**的落点（用于符号链接越界用例）。
#[cfg(unix)]
fn outside_dir() -> &'static Path {
    static OUTSIDE: OnceLock<PathBuf> = OnceLock::new();
    OUTSIDE.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-fuzz-outside-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("create outside dir");
        dir
    })
}

fn crc32(data: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 != 0 {
                    0xEDB8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *slot = value;
        }
        table
    });
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc = table[((crc ^ *byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in data {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// zlib（deflate stored 块）—— 不引外部依赖的最小实现。
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01];
    let mut offset = 0usize;
    loop {
        let end = (offset + 65535).min(data.len());
        let block = &data[offset..end];
        let last = end == data.len();
        out.push(if last { 0x01 } else { 0x00 });
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
        offset = end;
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// 造一张 PNG：声明尺寸与实际扫描线可以不一致（炸弹图用例需要）。
fn build_png(
    declared_width: u32,
    declared_height: u32,
    raw_scanlines: &[u8],
    text: Option<(&str, &str)>,
) -> Vec<u8> {
    let mut out = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&declared_width.to_be_bytes());
    ihdr.extend_from_slice(&declared_height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type = RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    png_chunk(&mut out, b"IHDR", &ihdr);
    if let Some((keyword, value)) = text {
        let mut payload = Vec::new();
        payload.extend_from_slice(keyword.as_bytes());
        payload.push(0);
        payload.extend_from_slice(value.as_bytes());
        png_chunk(&mut out, b"tEXt", &payload);
    }
    png_chunk(&mut out, b"IDAT", &zlib_stored(raw_scanlines));
    png_chunk(&mut out, b"IEND", &[]);
    out
}

/// 由像素数据造扫描线（filter byte 0 + RGBA8）。
fn scanlines(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0);
        for column in 0..stride {
            let index = (row * stride + column) % pixels.len().max(1);
            raw.push(if pixels.is_empty() { 0 } else { pixels[index] });
        }
    }
    raw
}

/// image_ops 端到端：读面 / 解码上限 / 操作管道 / 输出格式裁定 / 写面。
///
/// 契约出处（`crates/capabilities/tools/src/image_ops.rs`）：
/// - `:21-23`「输入字节上限 + 解码期维度/分配上限 (解码器 Limits) + 解码后像素上限
///   MAX_IMAGE_PIXELS, 三层各自拒绝即帧」
/// - `:24-25`「输入按内容嗅探、输出按目标格式, 双侧都只认 png/jpeg/webp/bmp」
/// - `:1263-1264`「输出扩展名若映射到已知图像格式却与目标不符 = 拒绝即帧」
/// - `:1213-1230` 读面按**内容**嗅探（扩展名不参与输入格式判定）
/// - `:1032-1038` 工作区限域：词法层 → canonicalize 现实层 → root 包含 → 敏感面拒
/// - `:43-47` 脱敏口径：结果帧只回工作区相对拼写，工作区根绝对拼写不出帧
fn image_ops_run_face(data: &[u8]) {
    let root = workspace();
    let mut cursor = Cursor::new(data);

    let width = 1 + (cursor.u8() % 16) as u32;
    let height = 1 + (cursor.u8() % 16) as u32;
    let pixels = cursor.take(64).to_vec();
    let raw = scanlines(width, height, &pixels);
    let secret = "F0ZZ-SECRET-MARKER-8A2C";
    let png = build_png(width, height, &raw, Some(("Comment", secret)));

    let input_path = root.join("in.bin");
    if fs::write(&input_path, &png).is_err() {
        return;
    }
    let output_path = root.join("out.png");
    let _ = fs::remove_file(&output_path);

    let tool = ImageOpsTool::new(
        root,
        Arc::new(ObservedGate::new()),
        Arc::new(ImageOpsActivity::new()),
    );

    let target_width = 1 + (cursor.u8() % 8) as i64;
    let target_height = 1 + (cursor.u8() % 8) as i64;
    let params = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations: vec![OperationSpec {
            op: "resize".to_string(),
            width: Some(target_width),
            height: Some(target_height),
            ..Default::default()
        }],
    };

    match tool.run(&params) {
        Ok(report) => {
            // 输入格式按**内容**嗅探（输入文件叫 in.bin，不是图像扩展名）。
            assert_eq!(
                report.input_format,
                ImageFormatKind::Png,
                "输入格式必须按内容嗅探 (image_ops.rs:24-25, :1213-1230)"
            );
            assert_eq!(
                report.input_size,
                (width, height),
                "输入尺寸必须等于解码结果 (image_ops.rs:1391)"
            );
            assert_eq!(
                report.output_format,
                ImageFormatKind::Png,
                "输出扩展名 .png ⇒ 输出格式 png (image_ops.rs:1263-1311)"
            );
            assert_eq!(
                report.output_size,
                (target_width as u32, target_height as u32),
                "输出尺寸必须等于 resize 目标 (image_ops.rs:1656-1678)"
            );
            assert_eq!(
                report.operations,
                vec!["resize".to_string()],
                "汇报必须如实列出应用的操作 (image_ops.rs:1443-1446)"
            );
            assert!(
                report.bytes_written > 0,
                "落盘字节数必须为正 (image_ops.rs:1448)"
            );
            // 脱敏口径：只回工作区相对拼写。
            assert_eq!(
                report.input, "in.bin",
                "汇报只回相对拼写 (image_ops.rs:43-44)"
            );
            assert_eq!(
                report.output, "out.png",
                "汇报只回相对拼写 (image_ops.rs:43-44)"
            );
            let root_text = root.to_string_lossy().to_string();
            let json = report.to_json().to_string();
            assert!(
                !json.contains(root_text.as_str()),
                "结果帧不得含工作区根的绝对拼写 (image_ops.rs:43-45): {json}"
            );

            // 输出必须是**重新编码**的 PNG ⇒ 不得夹带输入的元数据明文。
            let written = fs::read(&output_path).unwrap_or_default();
            assert!(
                written.len() > 8
                    && written[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
                "输出必须是 PNG 字节流 (image_ops.rs:24-25)"
            );
            assert!(
                !String::from_utf8_lossy(&written).contains(secret),
                "输出不得夹带输入元数据里的明文 (输出是重编码, image_ops.rs:1313-1360)"
            );
        }
        Err(error) => {
            panic!("合法输入 + 合法操作必须成功: {error:?}");
        }
    }

    let _ = fs::remove_file(&output_path);

    // 契约：逐操作语义（管道顺序应用，任一步拒绝即帧 ⇒ 全无产出）。
    // 用 fuzz 派生的像素类操作跑一遍端到端（image_ops.rs:1393-1397）。
    let op_pool = [
        "resize",
        "crop",
        "rotate",
        "flip",
        "format_convert",
        "brightness",
        "contrast",
        "grayscale",
        "blur",
        "sharpen",
        "watermark_text",
    ];
    let op_count = 1 + (cursor.u8() % 3) as usize;
    let mut operations = Vec::new();
    for _ in 0..op_count {
        let name = op_pool[(cursor.u8() as usize) % op_pool.len()];
        operations.push(build_spec(&mut cursor, name, true));
    }
    let pipeline = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations,
    };
    match tool.run(&pipeline) {
        Ok(report) => {
            let expected: Vec<String> = report.operations.clone();
            assert_eq!(
                report.operations.len(),
                op_count,
                "汇报必须如实列出应用的操作 (image_ops.rs:1443-1446): {expected:?}"
            );
            assert_eq!(
                report.output_format,
                ImageFormatKind::Png,
                "输出扩展名 .png ⇒ 输出格式 png (image_ops.rs:1263-1311)"
            );
            let written = fs::read(&output_path).unwrap_or_default();
            assert!(
                written.len() > 8
                    && written[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
                "输出必须是 PNG 字节流 (image_ops.rs:24-25)"
            );
            assert!(
                !String::from_utf8_lossy(&written).contains(secret),
                "输出不得夹带输入元数据里的明文 (输出是重编码, image_ops.rs:1313-1360)"
            );
            let root_text = root.to_string_lossy().to_string();
            assert!(
                !report.to_json().to_string().contains(root_text.as_str()),
                "结果帧不得含工作区根的绝对拼写 (image_ops.rs:43-45)"
            );
        }
        Err(error) => {
            // 合法输入下的拒绝只可能是几何越界 / 水印字框越界 / 扩展名不符 /
            // 分配上限 —— 都在已声明的帧类里；且帧不得回显绝对路径。
            assert!(
                matches!(
                    error.code(),
                    "image_ops.param_rejected"
                        | "image_ops.limit_exceeded"
                        | "image_ops.format_rejected"
                ),
                "像素管道的拒绝帧类必须在已声明集合内 (image_ops.rs:719-748): {error:?}"
            );
            let root_text = root.to_string_lossy().to_string();
            assert!(
                !error.frame().contains(root_text.as_str()),
                "拒绝帧不得回显工作区根的绝对拼写 (image_ops.rs:45-46): {}",
                error.frame()
            );
        }
    }
    let _ = fs::remove_file(&output_path);

    // 契约：输出扩展名与目标格式不符 ⇒ 拒绝即帧（image_ops.rs:1263-1264）。
    let mismatch = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations: vec![OperationSpec {
            op: "format_convert".to_string(),
            format: Some("jpeg".to_string()),
            ..Default::default()
        }],
    };
    let error = tool
        .run(&mismatch)
        .expect_err("输出扩展名与目标格式不符必须拒绝即帧 (image_ops.rs:1263-1264)");
    assert_eq!(
        error.code(),
        "image_ops.param_rejected",
        "扩展名/格式不符必须是 param_rejected (image_ops.rs:1286-1294): {error:?}"
    );
    let _ = fs::remove_file(&output_path);

    // 契约：解码期维度上限（image_ops.rs:21-23, :1232-1251）。
    let bomb_dimension = build_png(MAX_IMAGE_DIM + 1, MAX_IMAGE_DIM + 1, &raw, None);
    fs::write(root.join("bomb.png"), &bomb_dimension).ok();
    let bomb = ImageOpsParams {
        input: "bomb.png".to_string(),
        output: "out.png".to_string(),
        operations: vec![OperationSpec {
            op: "grayscale".to_string(),
            ..Default::default()
        }],
    };
    let error = tool
        .run(&bomb)
        .expect_err("超尺寸图必须被解码期上限拒绝 (image_ops.rs:21-23)");
    assert_eq!(
        error.code(),
        "image_ops.limit_exceeded",
        "超尺寸图必须是 limit_exceeded (image_ops.rs:1247-1249): {error:?}"
    );
    let _ = fs::remove_file(&output_path);
    let _ = fs::remove_file(root.join("bomb.png"));

    // 契约：解码期像素上限 + 压缩炸弹启发式（image_ops.rs:21-23）。
    let bomb_ratio = build_png(4096, 4096, &raw, None);
    fs::write(root.join("bomb2.png"), &bomb_ratio).ok();
    let bomb = ImageOpsParams {
        input: "bomb2.png".to_string(),
        output: "out.png".to_string(),
        operations: vec![OperationSpec {
            op: "grayscale".to_string(),
            ..Default::default()
        }],
    };
    let error = tool
        .run(&bomb)
        .expect_err("压缩炸弹必须被解码期上限拒绝 (image_ops.rs:21-23)");
    assert_eq!(
        error.code(),
        "image_ops.limit_exceeded",
        "压缩炸弹必须是 limit_exceeded (image_ops.rs:1247-1249): {error:?}"
    );
    let _ = fs::remove_file(&output_path);
    let _ = fs::remove_file(root.join("bomb2.png"));

    // 契约：`psd_read_info` 探针（image_ops.rs:1849-1887）：解析失败**拒绝即帧**
    // （decode_failed），不得 panic。分层文档解析面此前仓内 0 fuzz 覆盖
    // （`crates/engine/image/codecs/fuzz/` 只覆盖像素编解码），这里借工具面补上。
    let psd_bytes = cursor.take(512).to_vec();
    fs::write(root.join("layer.psd"), &psd_bytes).ok();
    let psd = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations: vec![OperationSpec {
            op: "psd_read_info".to_string(),
            path: Some("layer.psd".to_string()),
            ..Default::default()
        }],
    };
    match tool.run(&psd) {
        Ok(report) => {
            assert_eq!(
                report.psd_infos.len(),
                1,
                "psd 探针必须产出恰好一条 (image_ops.rs:1394, :1444-1449)"
            );
            let info = &report.psd_infos[0];
            assert_eq!(
                info.path, "layer.psd",
                "psd 探针只回工作区相对拼写 (image_ops.rs:810, :872)"
            );
            assert!(
                info.layer_names.len() <= MAX_PSD_LAYER_NAMES,
                "图层名列表必须受上限约束 (image_ops.rs:131, :829)"
            );
            assert_eq!(
                info.layer_names_capped,
                info.layer_count > MAX_PSD_LAYER_NAMES,
                "截断标记必须与图层数一致 (image_ops.rs:832, :885)"
            );
        }
        Err(error) => {
            assert_eq!(
                error.code(),
                "image_ops.decode_failed",
                "PSD 解析失败必须是 decode_failed (image_ops.rs:1861-1862): {error:?}"
            );
        }
    }
    let _ = fs::remove_file(&output_path);
    let _ = fs::remove_file(root.join("layer.psd"));

    // 契约：工作区限域的现实层 —— 工作区内的符号链接指向工作区外 ⇒ 拒绝
    // （image_ops.rs:1032-1038 的 canonicalize 现实层 + root 包含检查）。
    // 符号链接是 POSIX 语义 ⇒ 该用例只在 unix 跑（非 unix 上没有符号链接可建）。
    #[cfg(unix)]
    {
        let outside_file = outside_dir().join("secret.png");
        fs::write(&outside_file, &png).ok();
        let link = root.join("link.png");
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(&outside_file, &link).ok();
        let escape = ImageOpsParams {
            input: "link.png".to_string(),
            output: "out.png".to_string(),
            operations: vec![OperationSpec {
                op: "grayscale".to_string(),
                ..Default::default()
            }],
        };
        let error = tool
            .run(&escape)
            .expect_err("工作区内的符号链接指向工作区外必须被拒 (image_ops.rs:1032-1038)");
        assert_eq!(
            error.code(),
            "image_ops.outside_workspace",
            "符号链接越界必须是 outside_workspace (image_ops.rs:1076-1078): {error:?}"
        );
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(&link);
    }

    // 契约：管道步数上限（image_ops.rs:1375-1379）与空管道拒绝（:1370-1374）。
    let too_many = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations: (0..=MAX_PIPELINE_STEPS)
            .map(|_| OperationSpec {
                op: "grayscale".to_string(),
                ..Default::default()
            })
            .collect(),
    };
    let error = tool
        .run(&too_many)
        .expect_err("超步数管道必须拒绝 (image_ops.rs:1375-1379)");
    assert_eq!(
        error.code(),
        "image_ops.param_rejected",
        "超步数管道必须是 param_rejected (image_ops.rs:1375-1379): {error:?}"
    );
    let empty_pipeline = ImageOpsParams {
        input: "in.bin".to_string(),
        output: "out.png".to_string(),
        operations: Vec::new(),
    };
    let error = tool
        .run(&empty_pipeline)
        .expect_err("空管道必须拒绝 (image_ops.rs:1370-1374)");
    assert_eq!(
        error.code(),
        "image_ops.param_rejected",
        "空管道必须是 param_rejected (image_ops.rs:1370-1374): {error:?}"
    );
    // 收尾: 临时工作区里不留本轮产物（目录本身是本次运行独有的，见 `workspace()`）。
    let _ = fs::remove_file(&input_path);
}

// ───────────────────────────── 自检（harness 自身的正确性） ─────────────────────────────

/// 用固定输入验证判据不是空转（降级路径 + 已知形态）。
fn self_check() {
    // fetch：已知形态的判定。
    let tool = FetchTool::new(FetchConfig::deny_all());
    let freeze = |url: &str| {
        tool.freeze_invocation(&ToolCall {
            id: "self".to_string(),
            name: "fetch".to_string(),
            arguments: serde_json::json!({ "url": url }),
        })
    };
    assert!(
        freeze("https://example.com/").is_ok(),
        "自检：普通 https 必须放行"
    );
    assert!(
        freeze("http://example.com/").is_ok(),
        "自检：普通 http 必须放行"
    );
    assert!(
        freeze("file:///etc/passwd").is_err(),
        "自检：file 协议必须被拒"
    );
    assert!(
        freeze("http://ok.com@evil.com/").is_err(),
        "自检：userinfo 混淆必须被拒"
    );
    assert!(
        freeze("http://[::1]/").is_ok(),
        "自检：IPv6 字面量是合法 URL 形态"
    );
    assert!(freeze("").is_err(), "自检：空 URL 必须被拒");
    let long = format!("https://example.com/{}", "a".repeat(FETCH_URL_MAX_BYTES));
    assert!(freeze(&long).is_err(), "自检：超长 URL 必须被拒");

    // 声明契约（fetch.rs:196-213；与仓内测试 fetch.rs:678-685 同口径）。
    let declaration = tool.declaration();
    let parameters = declaration.parameters;
    let url_schema = parameters
        .get("properties")
        .and_then(|properties| properties.get("url"))
        .expect("声明必须描述 url 参数 (fetch.rs:198-205)");
    assert_eq!(
        url_schema.get("maxLength").and_then(|v| v.as_u64()),
        Some(FETCH_URL_MAX_BYTES as u64),
        "自检：声明的 maxLength 必须等于 FETCH_URL_MAX_BYTES (fetch.rs:203)"
    );
    assert_eq!(
        url_schema.get("minLength").and_then(|v| v.as_u64()),
        Some(1),
        "自检：声明的 minLength 必须是 1 (fetch.rs:202)"
    );
    assert_eq!(
        parameters
            .get("additionalProperties")
            .and_then(|v| v.as_bool()),
        Some(false),
        "自检：声明必须禁止额外参数 (fetch.rs:208)"
    );
    let required = parameters
        .get("required")
        .and_then(|v| v.as_array())
        .expect("声明必须列出必填字段 (fetch.rs:207)");
    assert_eq!(
        required.len(),
        1,
        "自检：声明的必填字段必须只有 url (fetch.rs:207)"
    );

    // 无障碍树：深嵌套必须被标记截断（accessibility.rs:15-19）。
    let mut deep = String::new();
    for _ in 0..(MAX_TREE_DEPTH + 64) {
        deep.push_str("<div>");
    }
    deep.push_str("deep");
    for _ in 0..(MAX_TREE_DEPTH + 64) {
        deep.push_str("</div>");
    }
    let tree = extract_tree(&deep);
    assert!(
        tree.truncated,
        "自检：超深嵌套必须置 truncated (accessibility.rs:15-19)"
    );
    assert!(
        tree.nodes.len() <= MAX_TREE_DEPTH + 64,
        "自检：节点数不得随嵌套深度线性爆炸 (accessibility.rs:15-19): {}",
        tree.nodes.len()
    );

    // 无障碍树：手工构造的病态深树必须被渲染上限截断（accessibility.rs:140-150）。
    let mut hand_built = AccessibilityTree::default();
    hand_built.nodes.push(AccessibilityNode {
        role: NodeRole::Document,
        name: String::new(),
        ref_id: None,
        children: vec![1],
        tag: "#document".to_string(),
        attrs: std::collections::HashMap::new(),
    });
    hand_built.root = 0;
    let mut parent = 0usize;
    for index in 1..=5_000usize {
        hand_built.nodes.push(AccessibilityNode {
            role: NodeRole::Generic,
            name: format!("n{index}"),
            ref_id: None,
            children: Vec::new(),
            tag: "div".to_string(),
            attrs: std::collections::HashMap::new(),
        });
        let node = hand_built.nodes.len() - 1;
        hand_built.nodes[parent].children.push(node);
        parent = node;
    }
    let snapshot = hand_built.to_snapshot();
    assert!(
        snapshot.contains("truncated at depth"),
        "自检：手工病态树必须被渲染上限截断 (accessibility.rs:140-150)"
    );
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        self_check();
    }
    fetch_url_face(data);
    html_text_face(data);
    accessibility_face(data);
    scheduling_face(data);
    image_ops_params_face(data);
    image_ops_run_face(data);
});
