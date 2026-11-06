#![no_main]

//! fuzz: **凭证面**（`apeireth-credentials`）—— 展示脱敏 / 长度脱敏 / 审计哈希 /
//! 服务名校验 / 审批闸门 / 错误面。
//!
//! 这个面是「秘密 ↔ 输出通道」的边界: 凭据明文从 keyring / 文件存储取出后, 要经过
//! **展示脱敏**（[`mask_for_display`]）、**长度脱敏**（[`redact_len`] /
//! [`SecretString`]）、**审计哈希**（[`name_hash`]）三道闸才允许出现在 UI / 日志里;
//! 取用侧还要过**审批门**（[`CredentialGate`]）。**一处算错 = 明文进日志**。
//!
//! 判据全部取自被测函数**自己写明的契约**（出处 = 文件:行号 + 原文片段）,
//! 不自造期望值; oracle 一律用**可判定的后置条件**, 不用「输出里是否包含某 token」
//! 这种在「秘密本身很短 / 是公共子串」时会假阳性的判据。
//!
//! ## 判据清单（逐条对应契约原文）
//!
//! 1. **展示脱敏的短串红线**（`secret.rs:76`：「**字符数** ≤ 4 时全掩, 不透任何字符。」
//!    —— 修复**前**的原文是「长度 ≤ 4 时全掩, 不透任何字符。」, 「长度」未写明是字节还是
//!    字符; 本 target 按**字符**读, 理由见下方判据 1 的口径说明）
//!    ⇒ 字符数 ≤ 4 的秘密, 脱敏串**不得含秘密的任何字符**。
//!    ⚠️ **2026-10-25 契约澄清（我改的）**：原文「长度 > 4 时保首 3 尾 3」在
//!    **5–6 字符**时窗口**覆盖整串** ⇒「掩中段」根本没发生 ⇒ 输出 = **明文 + 省略号**
//!    ⇒ 与短秘密同等对待，**也全掩**。⇒ 判据 2 的门槛因此是 **≥ 7 字符**。
//!    ⚠️ 口径: 「长度」按**字符**计 —— 因为该函数的保首/保尾窗口本身就是按字符取的
//!    （`secret.rs:88-96` 用 `chars().take(3)` / `chars().rev().take(3)`）; 若阈值按字节
//!    计, 则「≤4 全掩」这条红线对多字节秘密**形同虚设**（3 个汉字 = 9 字节 ⇒ 阈值被绕开,
//!    首尾窗口又覆盖整串 ⇒ 整条秘密原样进 UI）。
//! 2. **保首尾**（`secret.rs:74`：「对任意字符串做"保首尾、掩中段"的展示脱敏」;
//!    仓内既有断言 `secret.rs:147-149`）⇒ 字符数 ≥ 5 时, 脱敏串必须以**前 3 字符**开头、
//!    以**后 3 字符**结尾、且含省略号 `…`。
//! 3. **掩中段**（`secret.rs:74` 同句; 仓内既有断言 `secret.rs:150`：
//!    「中段不应透出」）⇒ 字符数 ≥ 7 时, 中段片段不得出现在脱敏串里。
//! 4. **长度脱敏只透露长度**（`secret.rs:67-72`：「按长度生成脱敏串: `[REDACTED len=<n>]`」/
//!    「只透露长度 (定位用元信息), 不透露任何明文字符」; `secret.rs:21`：
//!    「`Debug` / `Display` 输出恒为 `[REDACTED len=<字节数>]`, 不含明文」）⇒
//!    `redact_len` / `SecretString::redacted` / `SecretString` 的 `Debug`+`Display` /
//!    `SecretBuf` 的 `Debug`+`Display` 必须逐字等于 `[REDACTED len=<字节数>]`,
//!    且**不得含秘密中模板字母表以外的任何字符**。
//! 5. **服务名校验文法**（`store.rs:50-53`：「允许: 非空, 仅 `A-Za-z0-9 . _ -`,
//!    长度 ≤ 128。拒绝路径分隔符 (防穿越), 拒绝 `.` / `..` / 点开头」）⇒ 接受集合
//!    必须**恰好**等于该文法（不多放行、不少拒绝）。
//! 6. **错误面不带明文**（`error.rs:3`：「所有变体的 `Display` **只含服务名 / 路径等元信息**,
//!    绝不携带凭据明文」; `error.rs:37-49` 的 `Format` 变体自己写明
//!    「解析错误描述 (serde 消息, **不含明文**)」—— 修复**前**的原文; 修复后改为
//!    「错误类别 + 行列位置, 不含出错值原文」）⇒ 凭据表损坏时, 错误文本**不得**
//!    带出表里的值原文。
//! 7. **审批门 fail-closed**（`gate.rs:56-58`：「**fail-closed 默认门**: 高危一律拒绝」;
//!    `gate.rs:84-85`：「高危服务的 `get`/`set`/`delete` 先过 [`CredentialGate::decide`];
//!    未获放行 → [`CredentialsError::ApprovalRequired`] (不落明文)」）⇒ 默认高危名单
//!    上的服务, 读/写/删一律拒; 非高危放行; 高危判定必须**严格等于**名单成员
//!    （`gate.rs:46-48` 的默认实现语义）。
//! 8. **治理 hook 语义**（`hook.rs:130-135`：「非凭据动作 → Allow / 凭据动作但非高危服务 →
//!    Allow / 高危服务 + 会话内已批准 → Allow / 高危服务 + 未批准 → RequireApproval」）⇒
//!    高危未批准必须**挂起待批**（不是 Deny, 也不是放行）。
//! 9. **审计哈希**（`keyring.rs:287-296`：「**SHA-256(service) 前 16 hex** (64 bit 截断).
//!    单向, 同 service 同 hash」; `keyring.rs:298-307`：「**盐化名哈希**: `SHA-256(salt ‖ service)[:16] hex`」/
//!    「换 salt 即换 hash」; `keyring.rs:194-199`：审计条目**仅可含**元信息, 不得含明文）⇒
//!    16 位小写 hex、确定、同名同哈希、空盐 ≡ 无盐、换盐即换哈希、审计条目不含明文。
//! 10. **keyring 条目形状**（`keyring.rs:148-152`：服务不存在 → `UnknownService`; `set` 覆盖语义;
//!    `list` 只返服务名; **每次访问**写一条审计; `keyring.rs:772-776`：
//!    `MAX_SERVICE_NAME_LEN = 128` / `MAX_SECRET_LEN = 4096`）⇒ 超限必拒、审计条数
//!    与访问数一一对应、审计条目不含服务名原文。
//! 11. **resolver 口径**（`keyring_resolver.rs:29-34`：找到 → `Some`, 任何错误 → `None`,
//!    呈现恒打码）⇒ 命中/未命中口径正确、呈现不含明文。
//!
//! 另加: 任意字节 ⇒ 合法 `str`（含畸形 UTF-8、超长、空、零宽）⇒ 任何入口都不得 panic。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_credentials::error::CredentialsError;
use apeireth_credentials::gate::{
    CredentialGate, CredentialOp, DEFAULT_HIGH_RISK_SERVICES, DenyAllGate, GateDecision,
    GatedCredentialsStore,
};
use apeireth_credentials::hook::{ApprovalMemory, CredentialApprovalHook};
use apeireth_credentials::keyring::{
    AuditContext, AuditEvent, CountingAudit, InMemoryKeyring, KeyringBackend, KeyringError,
    MAX_SECRET_LEN, MAX_SERVICE_NAME_LEN, name_hash, name_hash_salted,
};
use apeireth_credentials::keyring_resolver::KeyringCredentialResolver;
use apeireth_credentials::secret::{SecretBuf, SecretString, mask_for_display, redact_len};
use apeireth_credentials::service_token::{
    SERVICE_TOKEN_SERVICE, SERVICE_TOKEN_USER, SERVICE_TOKEN_WINDOWS_TARGET,
};
use apeireth_credentials::{CredentialsStore, FileCredentialsStore, validate_service_name};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::CredentialResolver;
use libfuzzer_sys::fuzz_target;
use serde_json::json;

// ============================================================================
// 断言辅助
// ============================================================================

/// 断言消息里截断输入 (别把整段刷出来)。
fn brief(text: &str) -> String {
    let head: String = text.chars().take(80).collect();
    format!(
        "{head:?}…(chars={},bytes={})",
        text.chars().count(),
        text.len()
    )
}

/// `[REDACTED len=<n>]` 这条**固定文案**的字母表 (含数字位 —— 长度值本身是允许透出的元信息)。
const REDACT_TEMPLATE_CHARS: &str = "[]REDACTED len=";

/// 字符是否属于长度脱敏串的字母表。
fn is_template_char(c: char) -> bool {
    c.is_ascii_digit() || REDACT_TEMPLATE_CHARS.contains(c)
}

/// 把 ASCII 映射成 CJK 汉字（3 字节/字）—— 让「字节长度 ≠ 字符数」这一面必然被覆盖。
///
/// 理由: 随机字节几乎造不出合法多字节串, 不做这个映射, 多字节分支（脱敏阈值口径）
/// 基本碰不到。映射目标区间 `U+4E00..U+4E7F` 全是汉字, 不含 `*` / `…`,
/// 不会与脱敏串自身字母表自指。
fn expand_multibyte(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii() {
                char::from_u32(0x4E00 + (c as u32 % 0x80)).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// `mask_for_display` 的契约判据（`secret.rs:74-93`）。
///
/// 口径说明（两条, 都是为了**不造假阳性**）:
/// - 「长度」按**字符**计（阈值与保首/保尾窗口必须同一口径; 见文件头判据 1 的说明）。
/// - 秘密里若含脱敏串自身的字母表（`*` / `…`）, 「输出不得含秘密字符」会**自指**
///   （脱敏串必然含 `*`）⇒ 这类输入不参与该判据。
fn assert_mask_contract(secret: &str) {
    let masked = mask_for_display(secret);
    let chars: Vec<char> = secret.chars().collect();
    let n = chars.len();

    if secret.chars().any(|c| c == '*' || c == '…') {
        return;
    }

    // ── 判据 1: 长度 ≤ 4 ⇒ 全掩, 不透任何字符 ──────────────────────────────
    if n <= 4 {
        for &c in &chars {
            assert!(
                !masked.contains(c),
                "≤4 字符的秘密必须全掩, 脱敏串却泄出了字符 {c:?}: {masked:?} (秘密={})",
                brief(secret)
            );
        }
    }

    // ── 判据 1b: 5..=6 字符 ⇒ **首尾窗口覆盖全串** ⇒ 无中段可掩 ⇒ 全掩 ──────
    // 口径（2026-10-25 契约澄清）：原文写「长度 > 4 时保首 3 尾 3」，
    // 但首 3 + 尾 3 的窗口在 5–6 字符时**覆盖整串** ⇒「掩中段」根本没发生
    // ⇒ 输出 = **明文 + 省略号**（每个字符都露出来了）⇒ 与短秘密同等对待，**也全掩**。
    if (5..=6).contains(&n) {
        assert_eq!(
            masked,
            "*".repeat(n),
            "窗口覆盖全串（5..=6 字符）时必须全掩: {masked:?} (秘密={})",
            brief(secret)
        );
    }

    if n >= 7 {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[n - 3..].iter().collect();
        // ── 判据 2: 保首尾 + 省略号 ────────────────────────────────────────
        assert!(
            masked.starts_with(&head),
            "脱敏串必须保首 3 字符 {head:?}: {masked:?} (秘密={})",
            brief(secret)
        );
        assert!(
            masked.ends_with(&tail),
            "脱敏串必须保尾 3 字符 {tail:?}: {masked:?} (秘密={})",
            brief(secret)
        );
        assert!(
            masked.contains('…'),
            "脱敏串必须含省略号: {masked:?} (秘密={})",
            brief(secret)
        );
    }

    if n >= 7 {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[n - 3..].iter().collect();
        let middle: String = chars[3..n - 3].iter().collect();
        // ── 判据 3: 掩中段 ────────────────────────────────────────────────
        // 口径说明: 中段片段若与首/尾窗口同形, 它出现在脱敏串里是**首尾窗口**带来的,
        // 不构成「中段透出」⇒ 这类输入跳过本条（不制造自造期望）。
        if !head.contains(&middle) && !tail.contains(&middle) {
            assert!(
                !masked.contains(&middle),
                "中段不得透出: 中段={middle:?} 脱敏串={masked:?} (秘密={})",
                brief(secret)
            );
        }
    }
}

/// `redact_len` / `SecretString` / `SecretBuf` 的契约判据（`secret.rs:47-72`, `secret.rs:300-330`）。
fn assert_redacted_contract(secret: &str) {
    // ── 判据 4: 逐字等于 `[REDACTED len=<字节数>]` ──────────────────────────
    let expected = redact_len(secret.len());
    let s = SecretString::new(secret);
    assert_eq!(
        s.redacted(),
        expected,
        "SecretString::redacted 必须逐字等于脱敏串"
    );
    assert_eq!(
        format!("{s:?}"),
        expected,
        "SecretString 的 Debug 必须恒为脱敏串"
    );
    assert_eq!(
        format!("{s}"),
        expected,
        "SecretString 的 Display 必须恒为脱敏串"
    );

    let buf = SecretBuf::from_str(secret);
    assert_eq!(
        buf.redacted(),
        expected,
        "SecretBuf::redacted 必须逐字等于脱敏串"
    );
    assert_eq!(
        format!("{buf:?}"),
        expected,
        "SecretBuf 的 Debug 必须恒为脱敏串"
    );
    assert_eq!(
        format!("{buf}"),
        expected,
        "SecretBuf 的 Display 必须恒为脱敏串"
    );

    // ── 判据 4b: 不得含秘密中「模板字母表以外」的字符 ─────────────────────
    // 口径说明: 脱敏串的字符全在 `[REDACTED len=]` + 数字里（长度是**允许**透出的元信息）,
    // 故只有「表外字符」才是可判定的泄漏证据 —— 这条**不会**假阳性。
    for c in secret.chars() {
        if !is_template_char(c) {
            assert!(
                !expected.contains(c),
                "脱敏串泄出了秘密字符 {c:?}: {expected:?} (秘密={})",
                brief(secret)
            );
        }
    }
}

/// `validate_service_name` 的文法判据（`store.rs:50-53`）。
///
/// 判据: 接受集合**恰好**等于该函数自己文档化的文法 —— 多了 = 放行不该放行的,
/// 少了 = 误拒合法名。
fn expected_service_name_ok(service: &str) -> bool {
    !service.is_empty()
        && service.len() <= 128
        && !service.starts_with('.')
        && service
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn assert_service_name_contract(service: &str) {
    let ok = validate_service_name(service).is_ok();
    assert_eq!(
        ok,
        expected_service_name_ok(service),
        "服务名校验与文档文法不一致 (name={:?})",
        brief(service)
    );
    if !ok {
        // `store.rs:55-67`: 非法名一律 `InvalidServiceName`。
        let err = validate_service_name(service).expect_err("必须报错");
        assert!(
            matches!(err, CredentialsError::InvalidServiceName(_)),
            "非法服务名必须报 InvalidServiceName: {err:?}"
        );
    }
}

// ============================================================================
// 夹具（确定性检查 + 临时文件）
// ============================================================================

/// 假凭据: **运行时构造**（R215 secret 扫描器是文本扫描且扫全历史, 源码里不能出现
/// `sk-` + 20+ 位字母数字的字面量）。运行时值与字面量逐字节相同。
fn fake_secret() -> String {
    format!("sk-{}", "1".repeat(26))
}

/// 本次运行的独有临时目录（pid + 纳秒 ⇒ 不与别的运行撞车）。
fn fixture_dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-fuzz-credentials-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("fixture dir");
        dir
    })
}

/// 夹具用的**合法凭据表**（含一条普通凭据 + 一条高危凭据）。
fn valid_store_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = fixture_dir().join("valid.json");
        let store = FileCredentialsStore::new(&path).expect("store");
        store
            .set("openai", SecretString::new(fake_secret()))
            .expect("set");
        store
            .set("master", SecretString::new(fake_secret()))
            .expect("set");
        path
    })
}

/// 损坏的凭据表路径（内容由调用方现写, 每次覆盖）。
fn corrupt_store_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| fixture_dir().join("corrupt.json"))
}

/// 无依赖的极简 `block_on`: 治理 hook 的 `evaluate` 是 async, 但实现内没有任何挂起点
/// （一次 poll 即完成）, 不需要 tokio。
struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 确定性夹具: 把「随机输入未必造得出」的性质变成一次性检查。
fn fixture_suite() {
    // (a) 服务名校验文法边界（`store.rs:50-53` 文法 + 仓内既有断言 `store.rs:287-292`）。
    for (name, ok) in [
        ("openai", true),
        ("my-service.v2_prod", true),
        ("a", true),
        ("", false),
        (".", false),
        ("..", false),
        (".hidden", false),
        ("a/b", false),
        ("a\\b", false),
        ("a b", false),
        ("a:b", false),
        ("héllo", false),
    ] {
        assert_eq!(
            validate_service_name(name).is_ok(),
            ok,
            "服务名校验边界不对: {name:?}"
        );
    }
    assert!(
        validate_service_name(&"a".repeat(128)).is_ok(),
        "长度 128 应接受"
    );
    assert!(
        validate_service_name(&"a".repeat(129)).is_err(),
        "长度 129 应拒绝"
    );

    // (b) 闸门 fail-closed（`gate.rs:56-58`, `gate.rs:84-85`）。
    let store = FileCredentialsStore::new(valid_store_path()).expect("store");
    let gated = GatedCredentialsStore::new(store, DenyAllGate);
    let err = gated.get("master").expect_err("高危读取必须被拒");
    assert!(
        matches!(err, CredentialsError::ApprovalRequired { .. }),
        "高危未授权必须报 ApprovalRequired: {err:?}"
    );
    assert!(
        !format!("{err}").contains(&fake_secret()),
        "审批错误不得带出明文: {err}"
    );
    // 非高危放行（`gate.rs:58`:「非高危服务放行」）。
    let got = gated.get("openai").expect("普通凭据放行");
    assert_eq!(got.expose(), fake_secret());

    // (c) 损坏文件上的写必须**拒绝**且不得覆盖文件（`store.rs:332-352` 的既有语义:
    //     「文件损坏 (非 JSON) 时 set 必须报错拒绝, 不得"空表起步"把损坏文件里可能
    //     恢复的内容静默清掉」）。
    let corrupt = corrupt_store_path();
    std::fs::write(corrupt, "{ this is not json").expect("write");
    let before = std::fs::read(corrupt).expect("read");
    let corrupt_store = FileCredentialsStore::new(corrupt).expect("store");
    assert!(
        corrupt_store.set("svc", SecretString::new("v")).is_err(),
        "损坏文件上 set 必须拒绝"
    );
    assert_eq!(
        std::fs::read(corrupt).expect("read"),
        before,
        "损坏文件不得被静默覆盖"
    );

    // (d) 常驻服务令牌的三方同源锚（`service_token.rs:22-27`: 「keyring 的 Windows
    //     默认映射把 `Entry::new(service, user)` 写成目标名 `{user}.{service}`」）。
    assert_eq!(
        SERVICE_TOKEN_WINDOWS_TARGET,
        format!("{SERVICE_TOKEN_USER}.{SERVICE_TOKEN_SERVICE}")
    );
}

static FIXTURES_RUN: AtomicBool = AtomicBool::new(false);

// ============================================================================
// fuzz 主体
// ============================================================================

fuzz_target!(|data: &[u8]| {
    if !FIXTURES_RUN.swap(true, Ordering::Relaxed) {
        fixture_suite();
    }

    // 不可信字节 → 文本（两种口径: 严格 UTF-8 与有损替换; 再加一份「字节长度 ≠
    // 字符数」的展开版, 保证多字节分支必然被走到）。
    let bytes = &data[..data.len().min(1024)];
    let lossy = String::from_utf8_lossy(bytes);
    let text: &str = lossy.as_ref();
    let strict = std::str::from_utf8(bytes).unwrap_or(text);
    let multibyte = expand_multibyte(strict);

    // ── ① 展示脱敏面 `mask_for_display` ──────────────────────────────────
    for secret in [text, strict, multibyte.as_str()] {
        assert_mask_contract(secret);
    }

    // ── ② 长度脱敏面 `redact_len` / `SecretString` / `SecretBuf` ───────────
    for secret in [text, strict, multibyte.as_str()] {
        assert_redacted_contract(secret);
    }

    // ── ③ 服务名校验面 `validate_service_name` ───────────────────────────
    for name in [text, strict, multibyte.as_str()] {
        assert_service_name_contract(name);
    }

    // ── ④ 错误面: 凭据表损坏 ⇒ 错误文本不得带出表里的值原文 ───────────────
    //
    // 契约: `error.rs:3`「所有变体的 `Display` **只含服务名 / 路径等元信息**, 绝不携带
    // 凭据明文」; `error.rs:37-49` 的 `Format` 变体自己写明「解析错误描述 (serde 消息,
    // **不含明文**)」—— 这是**修复前**的原文（修复后改为「错误类别 + 行列位置, 不含出错值
    // 原文」, 因为 serde 的原始消息**确实会**带值）。凭据表的值 = 凭据明文
    // （`store.rs:72`:「单个 JSON 文件承载 `服务名 -> 明文` 映射」）。
    //
    // 取值口径: 用一个 19 位的整数当「值」—— serde 的类型不匹配错误会把**出错的值
    // 原文**写进消息（如 `invalid type: integer `987...`, expected a string`）;
    // 19 位数字不可能与消息里的行列号撞上 ⇒ 该判据不会假阳性。
    let value = format!(
        "{}",
        9_000_000_000_000_000_000u64 + (data.first().copied().unwrap_or(0) as u64 % 1_000_000_000)
    );
    let corrupt = corrupt_store_path();
    std::fs::write(corrupt, format!("{{\"openai\": {value}}}")).expect("write corrupt");
    let corrupt_store = FileCredentialsStore::new(corrupt).expect("store");
    let err = corrupt_store.get("openai").expect_err("损坏文件必须报错");
    assert!(
        matches!(err, CredentialsError::Format { .. }),
        "损坏凭据表必须报 Format: {err:?}"
    );
    let shown = format!("{err}");
    assert!(
        !shown.contains(&value),
        "错误文本带出了凭据表里的值原文: {shown}"
    );
    let debugged = format!("{err:?}");
    assert!(
        !debugged.contains(&value),
        "错误 Debug 带出了凭据表里的值原文: {debugged}"
    );

    // ── ⑤ 错误面: 未知名 ⇒ 错误文本只带服务名, 不带明文 ──────────────────
    //
    // 契约: `store.rs:34`「读取服务凭据; 服务不存在 → `CredentialsError::UnknownService`」;
    // `error.rs:13-15` 的变体只装**服务名**（元信息, 不是明文）。
    let store = FileCredentialsStore::new(valid_store_path()).expect("store");
    let err = store.get("no-such-service").expect_err("未知名必须报错");
    assert!(
        matches!(err, CredentialsError::UnknownService(_)),
        "未知名必须报 UnknownService: {err:?}"
    );
    let shown = format!("{err}");
    assert!(
        shown.contains("no-such-service"),
        "错误必须带服务名（元信息）: {shown}"
    );
    assert!(
        !shown.contains(&fake_secret()),
        "错误文本不得带出凭据明文: {shown}"
    );

    // ── ⑥ 闸门面: 高危必拒 / 非高危放行 / 高危判定不得放宽 ────────────────
    //
    // 契约: `gate.rs:56-58`「**fail-closed 默认门**: 高危一律拒绝」; `gate.rs:58`
    // 「非高危服务放行」; `gate.rs:46-48`「该服务是否属高危 (需审批)。默认按
    // `DEFAULT_HIGH_RISK_SERVICES`」⇒ 高危判定必须**严格等于**名单成员（不得大小写
    // 折叠 / 不得前缀匹配 / 不得白名单外扩）。
    assert_eq!(
        DenyAllGate.is_high_risk(text),
        DEFAULT_HIGH_RISK_SERVICES.contains(&text),
        "高危判定必须严格等于默认名单: {text:?}"
    );
    for svc in DEFAULT_HIGH_RISK_SERVICES {
        assert!(
            DenyAllGate.is_high_risk(svc),
            "默认高危名必须被判高危: {svc}"
        );
        for op in [
            CredentialOp::Read,
            CredentialOp::Write,
            CredentialOp::Delete,
        ] {
            assert_eq!(
                DenyAllGate.decide(svc, op),
                GateDecision::Deny,
                "默认门对高危服务必须拒: {svc} / {op:?}"
            );
        }
    }
    let store = FileCredentialsStore::new(valid_store_path()).expect("store");
    let gated = GatedCredentialsStore::new(store, DenyAllGate);
    for svc in DEFAULT_HIGH_RISK_SERVICES {
        let err = gated.get(svc).expect_err("高危读取必须被拒");
        assert!(
            matches!(err, CredentialsError::ApprovalRequired { .. }),
            "高危读取必须报 ApprovalRequired: {err:?}"
        );
        assert!(
            !format!("{err}").contains(&fake_secret()),
            "审批错误不得带出明文: {err}"
        );
        let err = gated
            .set(svc, SecretString::new(fake_secret()))
            .expect_err("高危写入必须被拒");
        assert!(
            matches!(err, CredentialsError::ApprovalRequired { .. }),
            "高危写入必须报 ApprovalRequired: {err:?}"
        );
        let err = gated.delete(svc).expect_err("高危删除必须被拒");
        assert!(
            matches!(err, CredentialsError::ApprovalRequired { .. }),
            "高危删除必须报 ApprovalRequired: {err:?}"
        );
    }
    // 非高危放行（`gate.rs:58`）。
    let got = gated.get("openai").expect("普通凭据放行");
    assert_eq!(got.expose(), fake_secret());
    assert!(
        !format!("{got:?}").contains(&fake_secret()),
        "取到的凭据呈现必须打码"
    );

    // ── ⑦ 治理 hook 面: 高危未批准 ⇒ 挂起待批 ───────────────────────────
    //
    // 契约: `hook.rs:130-135`「非凭据动作 → Allow / 凭据动作但非高危服务 → Allow /
    // 高危服务 + 会话内已批准 → Allow / 高危服务 + 未批准 → `RequireApproval`」;
    // `hook.rs:15-18`「不携带该字段的动作一律放行」。
    let memory = ApprovalMemory::new();
    let hook = CredentialApprovalHook::new(memory.clone());
    let capability = CapabilityId::new("credential.read").expect("capability id");
    let session = SessionId::new();
    for svc in ["master", "master_token", "master-token", text] {
        let arguments = json!({ "credential_service": svc, "credential_op": "read" });
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            session,
            TraceId::new(),
            1,
        );
        let decision = block_on(hook.evaluate(&request));
        if DEFAULT_HIGH_RISK_SERVICES.contains(&svc) {
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "高危未批准必须挂起待批（不是放行、也不是枪毙）: {decision:?} (service={svc:?})"
            );
        } else {
            assert!(
                decision.is_allowed(),
                "非高危凭据动作不得被本 hook 阻塞: {decision:?} (service={svc:?})"
            );
        }
    }
    // 无 `credential_service` 字段 ⇒ 一律放行（`hook.rs:15-18`）。
    let arguments = json!({ "command": "ls" });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        session,
        TraceId::new(),
        1,
    );
    assert!(
        block_on(hook.evaluate(&request)).is_allowed(),
        "非凭据动作必须放行"
    );
    // 人类批准回流 ⇒ 同会话同能力放行（`hook.rs:134`, `hook.rs:223-227`）。
    //
    // 口径说明（我自己踩的坑, 如实记）: 意图是按 **(会话, 能力)** 挂账的
    // （`hook.rs:82-88` 的 `intents: HashMap<(String, String), String>`）, 一个能力
    // **只记一条**意图 ⇒ 上面那个多服务循环会把意图覆盖掉。故这里用**独立的记忆/hook**
    // 单测一条高危服务, 不与循环混用。
    let approve_memory = ApprovalMemory::new();
    let approve_hook = CredentialApprovalHook::new(approve_memory.clone());
    let arguments = json!({ "credential_service": "master", "credential_op": "read" });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        session,
        TraceId::new(),
        1,
    );
    assert!(
        matches!(
            block_on(approve_hook.evaluate(&request)),
            Decision::RequireApproval { .. }
        ),
        "批准前必须挂起待批"
    );
    approve_hook.approval_resolved(&session, &capability);
    assert!(
        block_on(approve_hook.evaluate(&request)).is_allowed(),
        "已批准的会话必须放行"
    );

    // ── ⑧ 审计哈希面 `name_hash` / `name_hash_salted` / `AuditContext` ─────
    //
    // 契约: `keyring.rs:287-296`「**SHA-256(service) 前 16 hex** (64 bit 截断). 单向,
    // 同 service 同 hash」; `keyring.rs:298-307`「**盐化名哈希**: `SHA-256(salt ‖ service)[:16] hex`」
    // /「同 salt 内同 service 同 hash; 换 salt 即换 hash」; `keyring.rs:318`「无盐 (与
    // `name_hash` 同口径)」。
    let h = name_hash(text);
    assert_eq!(h.len(), 16, "name_hash 必须是 16 位 hex: {h}");
    assert!(
        h.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "name_hash 必须是 16 位小写 hex: {h}"
    );
    assert_eq!(h, name_hash(text), "同名必须同哈希（可关联）");
    assert_ne!(h, text, "name_hash 不得等于服务名原文");

    // 「不同名不同哈希」（抽样）: 取一个只差长后缀的兄弟名。
    //
    // 口径说明（如实记）: 契约**没有**明写单射性（`keyring.rs:287-296` 只写「单向, 同
    // service 同 hash」）; 本条按任务要求做抽样检查 —— 哈希是 64 bit 截断, 两个固定名
    // 撞哈希的概率 ~2^-64 ⇒ 不是断言口径问题, 撞了就是真撞了。
    let sibling = format!("{text}-sibling-0123456789");
    assert_ne!(
        name_hash(&sibling),
        h,
        "不同服务名必须不同哈希（审计可关联, 但不得混名）"
    );

    // 「不得包含可逆的服务名片段」: 哈希里不得出现服务名原文。
    //
    // 口径说明: 哈希只有 16 位 hex, 短名**天然**可能偶然出现在里面（16 位 hex 里出现
    // 一个 1 字符名的概率 ~64%）⇒ 只对**长度 ≥ 8** 的名字断言（概率 ~1e-9）,
    // 短名走上面的「不得等于原文」那条。仓内既有断言同款口径
    // （`keyring.rs:1566-1591` 用的是 "openai"/"anthropic"/"master-token" 这类长名）。
    if text.len() >= 8 {
        assert!(
            !h.contains(text),
            "name_hash 不得含服务名原文（不可逆）: {h} / {text:?}"
        );
    }

    // 空盐 ≡ 无盐（由文档公式 `SHA-256(salt ‖ service)` 直接推出）。
    assert_eq!(name_hash_salted(b"", text), h, "空盐必须与无盐同口径");
    // 换盐即换哈希（`keyring.rs:300`）。
    assert_ne!(
        name_hash_salted(b"fuzz-salt-a", text),
        h,
        "加盐后必须换哈希空间"
    );
    assert_ne!(
        name_hash_salted(b"fuzz-salt-a", text),
        name_hash_salted(b"fuzz-salt-b", text),
        "换盐即换哈希"
    );
    assert_eq!(
        name_hash_salted(b"fuzz-salt-a", text),
        name_hash_salted(b"fuzz-salt-a", text),
        "同 salt 内同 service 必须同哈希"
    );
    // `AuditContext` 口径（`keyring.rs:317-336`）。
    assert_eq!(
        AuditContext::new().hash(text),
        h,
        "无盐上下文必须与 name_hash 同口径"
    );
    assert_eq!(
        AuditContext::with_salt(b"fuzz-salt-a").hash(text),
        name_hash_salted(b"fuzz-salt-a", text),
        "有盐上下文必须与 name_hash_salted 同口径"
    );

    // ── ⑨ keyring 条目形状 + 审计条目 ────────────────────────────────────
    //
    // 契约: `keyring.rs:148-152`「服务不存在 → `UnknownService`; `set` 覆盖语义 (幂等);
    // `list` 只返服务名 (不含明文); **每次访问**通过 `AuditSink::record` 写一条审计
    // (含 `name_hash`, 不含明文)」; `keyring.rs:772-776` 的两个上限; `keyring.rs:194-199`
    // 「审计条目**不得含**凭据明文 … 仅可含: 时间戳 / event / `name_hash` / success /
    // backend 名」。
    let audit = Arc::new(CountingAudit::new());
    let keyring = InMemoryKeyring::new(audit.clone());
    let svc = "fuzzsvc";
    let secret = SecretBuf::from_str(&fake_secret());
    keyring.set(svc, &secret).expect("set");
    let got = keyring.get(svc).expect("get");
    assert_eq!(
        got.expose(),
        secret.expose(),
        "keyring 取回的值必须逐字节相同"
    );
    keyring.set(svc, &secret).expect("覆盖写");
    let names = keyring.list().expect("list");
    assert_eq!(names, vec![svc.to_string()], "list 只返服务名");
    let err = keyring.get("never-written").expect_err("未知名必须报错");
    assert!(
        matches!(err, KeyringError::UnknownService { .. }),
        "未知名必须报 UnknownService: {err:?}"
    );
    keyring.delete(svc).expect("delete");
    let err = keyring.get(svc).expect_err("删除后未知名");
    assert!(
        matches!(err, KeyringError::UnknownService { .. }),
        "删除后必须报 UnknownService: {err:?}"
    );

    // 上限（`keyring.rs:772-776`）。
    let long_name = "a".repeat(MAX_SERVICE_NAME_LEN + 1);
    assert!(
        matches!(
            keyring.get(&long_name),
            Err(KeyringError::ServiceNameTooLong { .. })
        ),
        "超长服务名必须拒"
    );
    let long_secret = SecretBuf::new(vec![b'x'; MAX_SECRET_LEN + 1]);
    assert!(
        matches!(
            keyring.set(svc, &long_secret),
            Err(KeyringError::SecretTooLong { .. })
        ),
        "超长凭据必须拒"
    );

    // 审计条目: 每次访问一条（7 次访问 ⇒ 7 条）, 且只含元信息。
    //
    // 口径说明（我自己踩的坑, 如实记）: 我第一版把次数写成 5 —— **数错了**（漏了覆盖写
    // 与删除后的那次 get）。库的行为是对的: **每次访问一条**; 而**超限**的
    // `get`/`set` 在写审计**之前**就返回错误 ⇒ 不产生条目（也算一次访问的失败，
    // 但按 `keyring.rs:1111-1167` 的实现, 校验失败路径不写审计）。
    let entries = audit.entries();
    assert_eq!(
        entries.len(),
        7,
        "每次访问必须写一条审计（set×2 + get + list + 未知名 get + delete + 删除后 get）: {entries:?}"
    );
    for entry in &entries {
        assert_eq!(entry.name_hash.len(), 16, "审计 name_hash 必须 16 位 hex");
        assert!(
            entry
                .name_hash
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "审计 name_hash 必须是小写 hex: {}",
            entry.name_hash
        );
        assert!(
            !entry.name_hash.contains(svc),
            "审计条目不得含服务名原文: {entry:?}"
        );
        assert!(
            !entry.name_hash.contains(&fake_secret()),
            "审计条目不得含凭据明文: {entry:?}"
        );
        assert!(
            matches!(
                entry.event,
                AuditEvent::Get | AuditEvent::Set | AuditEvent::Delete | AuditEvent::List
            ),
            "审计事件类别必须属于枚举: {entry:?}"
        );
    }

    // ── ⑩ resolver 面 ────────────────────────────────────────────────────
    //
    // 契约: `keyring_resolver.rs:29-34`「`resolve(name)` → `Ok(Secret(buf))` 转
    // `Some(Secret(string))`」/「任何 `KeyringError` … → `None`」; 呈现恒打码
    // （`keyring_resolver.rs:10`）。
    let backend = Arc::new(InMemoryKeyring::new(Arc::new(CountingAudit::new())));
    backend.set(svc, &secret).expect("set");
    let resolver = KeyringCredentialResolver::new(backend.clone());
    let resolved = resolver.resolve(svc).expect("命中");
    assert_eq!(resolved.expose(), fake_secret(), "resolver 必须取回明文");
    assert!(
        !format!("{resolved:?}").contains(&fake_secret()),
        "resolver 取到的凭据 Debug 必须打码"
    );
    assert!(
        !format!("{resolved}").contains(&fake_secret()),
        "resolver 取到的凭据 Display 必须打码"
    );
    assert!(
        resolver.resolve("never-written").is_none(),
        "未知名必须返 None"
    );
    assert!(
        resolver.resolve("bad/name").is_none(),
        "非法服务名必须返 None（不得 panic、不得放行）"
    );

    // ── ⑪ 空 / 零宽 / 超长输入不 panic ────────────────────────────────────
    let _ = mask_for_display("");
    let _ = mask_for_display(&"x".repeat(100_000));
    let _ = redact_len(0);
    let _ = redact_len(usize::MAX);
    let _ = validate_service_name("");
    let _ = name_hash("");
    let _ = name_hash_salted(&[], "");
});

/// 供集成测试复用: 临时目录清理（fuzz 进程被 kill 时不执行, 残留只在 /tmp）。
#[allow(dead_code)]
fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}
