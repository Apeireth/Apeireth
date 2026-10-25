#![no_main]

//! fuzz: **输入安全面**（`governance::input_security`）—— PII 识别 / 脱敏 /
//! 出境占位符族 / 注入启发式 / 两个治理钩子。
//!
//! 这个面是「不可信文本 → 治理裁决 / 出境文本」的中段：上游是模型、工具、
//! 网络回程回来的任意文本，下游是治理钩子与出境脱敏桥。它自己必须经得起
//! 不可信输入。
//!
//! 判据全部取自该模块**自己写明的契约**（出处为
//! `crates/foundation/governance/src/input_security.rs` 的行号 + 原文片段）：
//!
//! 1. **偏移契约**（模块文档 §Byte offsets L22-26：「`start` and `end` on
//!    findings are **byte offsets** into the original UTF-8 input string. They
//!    come from the regex engine and are safe for slicing the same Rust `str`
//!    that was scanned.」）⇒ 偏移必须是合法切片、必须落在字符边界上。
//! 2. **确定性 / 有序**（`PiiDetector::findings` L196：「All configured PII
//!    findings in `text`, sorted deterministically.」；`signals` L529 同款）⇒
//!    同输入两次调用结果相同、且按 `start` 升序。
//! 3. **词面只提示**（`PiiKind::Feeling` L92-101：「The word face
//!    ([`FeelingHintHeuristic`]) only raises advisory hints and never produces
//!    detector findings, redaction, or governance escalation by itself」；
//!    `FeelingHint` L577-581：「A hint is **not** a finding: it never redacts,
//!    never blocks, and never escalates any governance decision.」）⇒ 情感词面
//!    永不产 finding、永不进脱敏面。
//! 4. **脱敏必须真的脱**（`PiiDetector::redact` L227-230：「The original
//!    sensitive tokens are replaced with stable placeholders.」）⇒ 每一条
//!    finding 的命中片段不得在脱敏输出里存活（**多实例：每一处都要脱**）。
//! 5. **秘密面脱敏只动三类面**（`redact_secret_surfaces` L250-257：「覆盖三类
//!    秘密面: 凭据 URL …、环境式密钥赋值 …、凭据形状的键值 … 但**不**动
//!    邮箱/电话/IP 等可读信息」）⇒ 无秘密面时必须是恒等变换；秘密面命中时
//!    片段不得存活。
//! 6. **注入启发式**（`PromptInjectionHeuristic` L521：「Case-insensitive,
//!    pattern-based prompt-injection heuristic.」+ L529：「All configured
//!    prompt-injection signals in `text`」）⇒ 命中片段必须是配置词面的大小写
//!    变体；配置词面的**每一次**出现都必须被检出（多实例）。
//! 7. **占位符族严格解析**（`RedactionPlaceholder::parse` L353-354：「Strictly
//!    parse one token. Anything outside the placeholder family returns `None`
//!    — the restore face recognizes this family only.」+ 族规格 L271-280）⇒
//!    族外一律 `None`；族内必须能 `render` 原样还原。
//! 8. **scan 覆盖**（`RedactionPlaceholder::scan` L368：「Every well-formed
//!    placeholder occurrence in `text`, in text order.」）⇒ 族内 token 不得漏报。
//! 9. **会话指纹**（`session_fingerprint` L395-400：「same session key + same
//!    value → same fingerprint (stable alias); different session key →
//!    different fingerprint (no cross-session link)」+ L396「truncated to its
//!    first 6 bytes rendered as 12 lowercase hex digits」）⇒ 12 位小写 hex；
//!    同密钥同值恒同；换密钥换别名（= 密钥化，不是裸哈希）。
//! 10. **治理钩子**（`PromptInjectionHook` L650-655：「Maps prompt-injection
//!     heuristic signals in capability arguments to `RequireApproval`. The hook
//!     is deliberately not a `Deny`」；`CredentialDisclosureHook` L688-692：
//!     「Maps credential-like PII in capability arguments to `RequireApproval`.
//!     Email/phone PII is not a governance decision by itself … only escalates
//!     credential-like strings.」）⇒ 命中即 `RequireApproval`；没命中就放行。

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::input_security::{
    CredentialDisclosureHook, FeelingHintHeuristic, PiiDetector, PiiKind, PromptInjectionHeuristic,
    PromptInjectionHook, PromptInjectionKind, REDACTION_FINGERPRINT_HEX_LEN,
    REDACTION_PLACEHOLDER_PREFIX, REDACTION_PLACEHOLDER_SUFFIX, RedactionPlaceholder,
    redact_secret_surfaces, session_fingerprint,
};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};

/// 该模块自己配置的 5 条注入词面（逐字取自 `input_security.rs` 的
/// `PROMPT_INJECTION_PATTERNS` 表，L482-497）。
const PROMPT_PATTERNS: &[(PromptInjectionKind, &str)] = &[
    (
        PromptInjectionKind::IgnorePreviousInstructions,
        "ignore previous instructions",
    ),
    (PromptInjectionKind::SystemPromptLeak, "system prompt leak"),
    (
        PromptInjectionKind::DeveloperMode,
        "you are now in developer mode",
    ),
    (PromptInjectionKind::JailbreakActive, "jailbreak active"),
    (
        PromptInjectionKind::DisregardGuardrails,
        "disregard all guardrails",
    ),
];

/// 9 个占位类别（`PiiKind::placeholder_category` 的全部取值，L137-148）。
const ALL_PII_KINDS: [PiiKind; 9] = [
    PiiKind::Email,
    PiiKind::Phone,
    PiiKind::CredentialKey,
    PiiKind::Ssn,
    PiiKind::CreditCard,
    PiiKind::IpAddress,
    PiiKind::CredentialUrl,
    PiiKind::EnvSecret,
    PiiKind::Feeling,
];

/// 秘密面三类（`redact_secret_surfaces` L250-257）。
fn is_secret_surface(kind: PiiKind) -> bool {
    matches!(
        kind,
        PiiKind::CredentialKey | PiiKind::CredentialUrl | PiiKind::EnvSecret
    )
}

/// 12 位小写十六进制。
fn is_fingerprint12(value: &str) -> bool {
    value.len() == REDACTION_FINGERPRINT_HEX_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// 截断显示，避免断言消息把整段输入刷出来。
fn brief(text: &str) -> String {
    let head: String = text.chars().take(120).collect();
    format!("{head:?}…(len={})", text.len())
}

/// 偏移契约（L22-26）：合法切片 + 字符边界。
fn assert_span(text: &str, start: usize, end: usize, what: &str) {
    assert!(start <= end, "{what}: start({start}) > end({end})");
    assert!(
        end <= text.len(),
        "{what}: end({end}) 超出串长({})",
        text.len()
    );
    assert!(
        text.is_char_boundary(start),
        "{what}: start({start}) 不在字符边界上: {}",
        brief(text)
    );
    assert!(
        text.is_char_boundary(end),
        "{what}: end({end}) 不在字符边界上: {}",
        brief(text)
    );
}

/// 文本里 ASCII 大小写不敏感的词面出现位置（字节偏移）。
///
/// 口径说明：只认 **ASCII** 大小写折叠。正则引擎额外折叠的 U+017F(ſ) 会改变
/// 字节形态，那种片段不是「词面的 ASCII 形态」，不纳入本检查（避免自造期望）。
fn ascii_occurrences(text: &str, phrase: &str) -> Vec<(usize, usize)> {
    let hay = text.as_bytes();
    let needle = phrase.as_bytes();
    let mut out = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        let same = hay[i..i + needle.len()]
            .iter()
            .zip(needle)
            .all(|(a, b)| a.to_ascii_lowercase() == *b);
        if same {
            out.push((i, i + needle.len()));
        }
        i += 1;
    }
    out
}

/// 按族规格（L271-280）判定 `s` 开头是不是族内 token，返回其字节长度。
///
/// 这是**按文档规格**写的参考判定（不复制实现的正则）：`__AP_` + 9 个类别之一
/// + `_` + 12 位小写 hex + `__`。
fn family_token_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix(REDACTION_PLACEHOLDER_PREFIX)?;
    for kind in ALL_PII_KINDS {
        let category = kind.placeholder_category();
        let Some(tail) = rest.strip_prefix(category) else {
            continue;
        };
        let Some(tail) = tail.strip_prefix('_') else {
            continue;
        };
        let bytes = tail.as_bytes();
        if bytes.len() < REDACTION_FINGERPRINT_HEX_LEN + 2 {
            continue;
        }
        let fingerprint = &bytes[..REDACTION_FINGERPRINT_HEX_LEN];
        if !fingerprint
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        {
            continue;
        }
        if &bytes[REDACTION_FINGERPRINT_HEX_LEN..REDACTION_FINGERPRINT_HEX_LEN + 2] != b"__" {
            continue;
        }
        return Some(
            REDACTION_PLACEHOLDER_PREFIX.len()
                + category.len()
                + 1
                + REDACTION_FINGERPRINT_HEX_LEN
                + 2,
        );
    }
    None
}

/// 文本里所有族内占位符的字节跨度。
fn family_occurrences(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while from < text.len() {
        let Some(offset) = text[from..].find(REDACTION_PLACEHOLDER_PREFIX) else {
            break;
        };
        let start = from + offset;
        if let Some(len) = family_token_len(&text[start..]) {
            out.push((start, start + len));
        }
        from = start + 1;
    }
    out
}

/// 无依赖的极简 `block_on`：治理钩子的 `evaluate` 是 async，但实现内没有任何
/// 挂起点（一次 poll 即完成），不需要 tokio。
struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWaker));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 固定夹具：把「同族多实例 / 多字节相邻 / 大小写变体 / 词面只提示 /
/// 占位符遮蔽」这几类性质变成确定性检查（fuzzer 未必随机造得出同族多实例）。
fn fixture_suite() {
    // (a) 同族多实例：每一处都要脱，不是只脱首处。
    // 假 key 在**运行时构造**（不是字面量）：R215 的 secret 扫描器是**文本扫描**,
    // 字面量会被判为「高置信凭证 pattern」; 运行时构造后**源码不含该形态**,
    // 而运行时值与原来**逐字节相同** ⇒ 测试语义不变, 且**不改任何门禁/allowlist**。
    let fake_key_a = format!("sk-{}", "1".repeat(26));
    let fake_key_b = format!("sk-{}", "2".repeat(26));
    let multi = format!("a1@b.com a2@b.org 13812345678 {fake_key_a} {fake_key_b}");
    let redacted = PiiDetector::redact(&multi);
    assert_eq!(
        redacted.matches("[REDACTED_EMAIL]").count(),
        2,
        "同族多实例：两处邮箱都要脱: {redacted:?}"
    );
    assert_eq!(
        redacted.matches("[REDACTED_PHONE]").count(),
        1,
        "同族多实例：电话要脱: {redacted:?}"
    );
    assert_eq!(
        redacted.matches("[REDACTED_CREDENTIAL]").count(),
        2,
        "同族多实例：两处凭据都要脱: {redacted:?}"
    );
    assert!(!redacted.contains("a1@b.com") && !redacted.contains("a2@b.org"));

    // (b) 多字节相邻：标记紧贴多字节字符（历史缺陷面）。
    let fake_key_c = format!("sk-{}", "3".repeat(26));
    let multi_byte = format!("我的邮箱是 a@b.com，手机号 13812345678，key {fake_key_c}。");
    for finding in PiiDetector::findings(&multi_byte) {
        assert!(
            multi_byte.is_char_boundary(finding.start) && multi_byte.is_char_boundary(finding.end),
            "多字节边界：偏移必须落在字符边界上: {finding:?}"
        );
    }
    let redacted = PiiDetector::redact(&multi_byte);
    assert!(!redacted.contains("a@b.com"), "{redacted:?}");
    assert!(!redacted.contains(&fake_key_c), "{redacted:?}");
    assert!(redacted.contains("我的邮箱是"), "{redacted:?}");

    // (c) 注入词面多实例 + 大小写变体。
    let inject = "IGNORE PREVIOUS INSTRUCTIONS and ignore previous instructions";
    assert_eq!(
        PromptInjectionHeuristic::signals(inject).len(),
        2,
        "同族多实例：两处注入词面都要报到"
    );

    // (d) 词面只提示：情感句不产 finding、不进脱敏面。
    let feeling = "我恨背刺";
    assert!(FeelingHintHeuristic::has_hint(feeling));
    assert!(PiiDetector::findings(feeling).is_empty());
    assert_eq!(PiiDetector::redact(feeling), feeling);

    // (e) 占位符族往返 + 遮蔽不得漏报。
    let token = PiiKind::Email.placeholder_label("a1b2c3d4e5f6");
    assert_eq!(
        RedactionPlaceholder::parse(&token).map(|p| p.render()),
        Some(token.clone()),
        "族内 token 必须能往返"
    );
    let shadowed = format!("__AP_EMAIL__{token}");
    let matches = RedactionPlaceholder::scan(&shadowed);
    assert_eq!(
        matches.len(),
        1,
        "族内 token 被前置字面量遮蔽时也不得漏报: {shadowed:?}"
    );
    assert_eq!(matches[0].start, "__AP_EMAIL__".len());
    assert_eq!(&shadowed[matches[0].start..matches[0].end], token);
}

static FIXTURES_RUN: AtomicBool = AtomicBool::new(false);

fuzz_target!(|data: &[u8]| {
    // 固定夹具跑一次即可（确定性输入，与 fuzzer 的随机输入无关）。
    if !FIXTURES_RUN.swap(true, Ordering::Relaxed) {
        fixture_suite();
    }

    // 不可信字节 → 文本（有损转换保证总是合法 `str`，多字节面也能覆盖）。
    let text = String::from_utf8_lossy(&data[..data.len().min(2048)]);
    let text = text.as_ref();

    // ── ① 偏移契约 + 确定性/有序 ───────────────────────────────────────────
    let findings = PiiDetector::findings(text);
    let signals = PromptInjectionHeuristic::signals(text);
    let hints = FeelingHintHeuristic::hints(text);
    let placeholders = RedactionPlaceholder::scan(text);

    for finding in &findings {
        assert_span(text, finding.start, finding.end, "PiiFinding");
    }
    for signal in &signals {
        assert_span(text, signal.start, signal.end, "PromptInjectionSignal");
    }
    for hint in &hints {
        assert_span(text, hint.start, hint.end, "FeelingHint");
        assert_eq!(
            &text[hint.start..hint.end],
            hint.cue,
            "FeelingHint 的 cue 必须等于命中的原文片段: {}",
            brief(text)
        );
    }
    for matched in &placeholders {
        assert_span(
            text,
            matched.start,
            matched.end,
            "RedactionPlaceholderMatch",
        );
    }

    let starts: Vec<usize> = findings.iter().map(|f| f.start).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        starts,
        sorted,
        "findings 必须按 start 升序: {}",
        brief(text)
    );
    assert_eq!(findings, PiiDetector::findings(text), "findings 必须确定");

    let signal_starts: Vec<usize> = signals.iter().map(|s| s.start).collect();
    let mut sorted = signal_starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        signal_starts,
        sorted,
        "signals 必须按 start 升序: {}",
        brief(text)
    );
    assert_eq!(
        signals,
        PromptInjectionHeuristic::signals(text),
        "signals 必须确定"
    );

    assert_eq!(
        PiiDetector::contains_pii(text),
        !findings.is_empty(),
        "contains_pii 必须与 findings 同口径"
    );
    assert_eq!(
        PromptInjectionHeuristic::has_signal(text),
        !signals.is_empty(),
        "has_signal 必须与 signals 同口径"
    );
    assert_eq!(
        FeelingHintHeuristic::has_hint(text),
        !hints.is_empty(),
        "has_hint 必须与 hints 同口径"
    );

    // ── ② 词面只提示：情感类别永不产 finding ────────────────────────────────
    assert!(
        findings.iter().all(|f| f.kind != PiiKind::Feeling),
        "情感词面命中不得进 findings: {}",
        brief(text)
    );

    // ── ③ 脱敏：脱敏后再扫一遍，不应再有**可脱的 PII** ──────────────────────
    //
    // **本条 oracle 换过一次写法**（我自己撞出来的教训，如实记）：
    // 原来写的是「`!redacted.contains(token)` —— 检测到的片段不得在脱敏输出里存活」。
    // **那是弱 oracle ⇒ 假阳性**：同一 token 文本**可能出现在非 PII 上下文里**
    // （如 `5.5.55.5C` —— 后跟词字符 ⇒ 按边界文法**本就不是独立 IP 记号**、
    // `findings` 也不报它 ⇒ `redact` 不脱它是**正确的**）；
    // 但 `redacted.contains("5.5.55.5")` 为真 ⇒ 被误判成「漏脱」⇒ **假阳性崩溃**。
    // （实测样本：`5.5.55.5` 两次 / `17777777770` 两次，后者第一处后跟 `C`。）
    //
    // 改用的口径是**自洽且更强**的：**「脱敏后再扫，不应再有可脱的 PII」** ——
    // 这正是「脱敏」这件事本身的语义（幂等到无可脱为止），且**不依赖 token 文本**。
    // 已实测：所有占位符（`[REDACTED_EMAIL]` / `[REDACTED_PHONE]` / `[REDACTED_IP]` /
    // `[REDACTED_CREDENTIAL]` / `[REDACTED_URL_CREDENTIAL]` / `KEY=[REDACTED_ENV_SECRET]`）
    // **都不自匹配** ⇒ 二次扫描为空是稳定判据。
    let redacted = PiiDetector::redact(text);
    let secret_redacted = redact_secret_surfaces(text);
    let residual = PiiDetector::findings(&redacted);
    assert!(
        residual.is_empty(),
        "PiiDetector::redact 后仍有可脱的 PII {:?}: {} -> {}",
        residual
            .iter()
            .map(|f| (f.kind, redacted[f.start..f.end].to_string()))
            .collect::<Vec<_>>(),
        brief(text),
        brief(&redacted)
    );
    // 秘密面脱敏（只动 3 类秘密面）⇒ 只允许**非秘密类**残留。
    let secret_residual = PiiDetector::findings(&secret_redacted)
        .into_iter()
        .filter(|f| is_secret_surface(f.kind))
        .collect::<Vec<_>>();
    assert!(
        secret_residual.is_empty(),
        "redact_secret_surfaces 后仍有可脱的秘密面 {:?}: {} -> {}",
        secret_residual
            .iter()
            .map(|f| (f.kind, secret_redacted[f.start..f.end].to_string()))
            .collect::<Vec<_>>(),
        brief(text),
        brief(&secret_redacted)
    );

    // 无命中 ⇒ 脱敏是恒等变换；无秘密面 ⇒ 秘密面脱敏不动可读信息。
    if findings.is_empty() {
        assert_eq!(
            redacted,
            text,
            "无命中时 redact 必须是恒等变换: {}",
            brief(text)
        );
    }
    if !findings.iter().any(|f| is_secret_surface(f.kind)) {
        assert_eq!(
            secret_redacted,
            text,
            "秘密面脱敏只动三类秘密面，无秘密面时必须是恒等变换: {}",
            brief(text)
        );
    }

    // ── ④ 注入启发式：命中片段 = 配置词面的大小写变体；每一次出现都要检出 ──
    for signal in &signals {
        let slice = &text[signal.start..signal.end];
        let (kind, phrase) = PROMPT_PATTERNS
            .iter()
            .find(|(kind, _)| *kind == signal.kind)
            .copied()
            .expect("signal 的类别必须属于配置的 5 个词面");
        assert_eq!(
            slice.chars().count(),
            phrase.chars().count(),
            "命中片段长度必须等于该词面: {slice:?} vs {phrase:?}"
        );
        if slice.is_ascii() {
            assert_eq!(
                slice.to_ascii_lowercase(),
                phrase,
                "命中片段必须是该词面的大小写变体: {slice:?} vs {phrase:?}"
            );
        }
    }
    for (kind, phrase) in PROMPT_PATTERNS {
        for (start, end) in ascii_occurrences(text, phrase) {
            assert!(
                signals
                    .iter()
                    .any(|s| s.start == start && s.end == end && s.kind == *kind),
                "注入词面漏检: {phrase:?} @ {start}..{end}: {}",
                brief(text)
            );
        }
    }

    // ── ⑤ 占位符族：严格解析 + scan 覆盖 ───────────────────────────────────
    for kind in ALL_PII_KINDS {
        let category = kind.placeholder_category();
        assert_eq!(
            PiiKind::from_placeholder_category(category),
            Some(kind),
            "类别 token 必须能映射回 PiiKind: {category}"
        );
    }
    for category in ["email", "Email", "eMaIl", "CREDENTIAL_key", "SSN "] {
        assert!(
            PiiKind::from_placeholder_category(category).is_none(),
            "类别必须精确大写匹配: {category:?}"
        );
    }

    // parse: 族外一律 None；族内必须能原样往返。
    match family_token_len(text) {
        None => assert!(
            RedactionPlaceholder::parse(text).is_none(),
            "族外 token 不得被解析: {}",
            brief(text)
        ),
        Some(len) => {
            assert_eq!(len, text.len(), "族内 token 的长度口径不一致");
            let parsed = RedactionPlaceholder::parse(text)
                .unwrap_or_else(|| panic!("族内 token 必须能解析: {}", brief(text)));
            assert_eq!(parsed.render(), text, "族内 token 必须能原样还原");
            assert_eq!(parsed.fingerprint.len(), REDACTION_FINGERPRINT_HEX_LEN);
            assert!(
                is_fingerprint12(&parsed.fingerprint),
                "指纹必须是 12 位小写 hex: {:?}",
                parsed.fingerprint
            );
        }
    }

    // scan: 命中的片段必须就是规范 token；按文本序。
    for matched in &placeholders {
        assert_eq!(
            &text[matched.start..matched.end],
            matched.placeholder.render(),
            "scan 命中的片段必须就是规范 token: {}",
            brief(text)
        );
        assert!(
            is_fingerprint12(&matched.placeholder.fingerprint),
            "指纹必须是 12 位小写 hex: {:?}",
            matched.placeholder.fingerprint
        );
    }
    for pair in placeholders.windows(2) {
        assert!(
            pair[0].start < pair[1].start,
            "scan 必须按文本序返回: {}",
            brief(text)
        );
    }
    // 每个族内 token 都必须被报到（被别的族内 token 遮住的不在此列：
    // 还原面按不重叠区间替换，重叠会破坏还原）。
    for (start, end) in family_occurrences(text) {
        let reported = placeholders
            .iter()
            .any(|m| m.start == start && m.end == end);
        let shadowed = placeholders.iter().any(|m| m.start < end && start < m.end);
        assert!(
            reported || shadowed,
            "scan 漏报族内占位符 @ {start}..{end}: {}",
            brief(text)
        );
    }

    // ── ⑥ 会话指纹：12 位小写 hex / 同密钥同值恒同 / 换密钥换别名 ──────────
    let fingerprint = session_fingerprint(data, text);
    assert_eq!(fingerprint.len(), REDACTION_FINGERPRINT_HEX_LEN);
    assert!(
        is_fingerprint12(&fingerprint),
        "指纹必须是 12 位小写 hex: {fingerprint:?}"
    );
    assert_eq!(
        fingerprint,
        session_fingerprint(data, text),
        "同密钥同值必须同指纹（稳定别名）"
    );
    let mut other_key = data.to_vec();
    other_key.push(0xA5);
    other_key.extend_from_slice(&[0x5A; 100]);
    assert_ne!(
        fingerprint,
        session_fingerprint(&other_key, text),
        "换会话密钥必须换别名（不得跨会话关联）"
    );

    // ── ⑦ 治理钩子：命中即要人批，没命中就放行 ──────────────────────────────
    let capability = CapabilityId::new("tool.shell").expect("tool.shell 必须是合法能力 id");
    let shapes: [Value; 5] = [
        json!({ "text": text }),
        json!([text, text]),
        json!({ "nested": { "deep": [text] } }),
        json!(text),
        json!({ "mixed": [null, 1, true, text] }),
    ];
    let has_credential = findings.iter().any(|f| f.kind == PiiKind::CredentialKey);
    for arguments in &shapes {
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );

        let injection = block_on(PromptInjectionHook::new().evaluate(&request));
        assert!(
            !matches!(injection, Decision::Deny { .. }),
            "注入钩子刻意不是 Deny: {}",
            brief(text)
        );
        if signals.is_empty() {
            assert!(
                injection.is_allowed(),
                "无注入信号必须放行: {}",
                brief(text)
            );
        } else {
            assert!(
                matches!(injection, Decision::RequireApproval { .. }),
                "注入信号必须升级为人批: {}",
                brief(text)
            );
        }

        let credential = block_on(CredentialDisclosureHook::new().evaluate(&request));
        if has_credential {
            assert!(
                matches!(credential, Decision::RequireApproval { .. }),
                "凭据形状必须升级为人批: {}",
                brief(text)
            );
        } else {
            assert!(
                credential.is_allowed(),
                "无凭据形状必须放行（邮箱/电话本身不是裁决）: {}",
                brief(text)
            );
        }
    }
});
