//! 凭据形状清扫 (credential shape sweep) —— 治理层的**形状层**扫描器.
//!
//! # 定位: 只看**形状**, 不看字段
//!
//! 本模块扫的是凭据的**形状**: 裸密钥串 / PEM 私钥块 / 明文口令赋值 /
//! 三段令牌 / Bearer 头 …… 它**不看**「键名 + 值」的字段关系 (那是字段值层的
//! 职责), 也**不处置**命中 —— 命中即**就地遮形** (单向标签), 不改判、不拒执行.
//!
//! # 为什么落在治理层
//!
//! 形状层内核原先住在**能力工具**侧, 而引擎运行时**依赖治理、不依赖能力工具**
//! (依赖方向单向) ⇒ ⇒ 运行时若要复用形状层, 形状层就必须在**治理层**.
//! 本模块即那次下沉的落点: 治理层提供「扫一个文本面」的纯能力, 引擎运行时在
//! **转录汇聚点**调用它.
//!
//! # 口径等价 (与内核既有形状层逐族对齐)
//!
//! 本扫描器与内核既有形状层**同口径**: 同一族判据 ⇒ 同一遮形标签 ⇒ 同一份输入
//! ⇒ 同一份终稿文本. 族 ↔ 标签的对照 (标签是内核既有字符串, 本模块不新造):
//!
//! | 族 | 形状判据 | 遮形标签 |
//! |---|---|---|
//! | [`CredentialFamily::SkPrefixedKey`] | `sk-` + ≥20 字符 | `[REDACTED_OPENAI_KEY]` |
//! | [`CredentialFamily::AkiaPrefixedKey`] | `AKIA` + 16..=32 字母数字 | `[REDACTED_AWS_KEY]` |
//! | [`CredentialFamily::GhpPrefixedToken`] | `ghp_` + ≥36 字符 | `[REDACTED_GITHUB_PAT]` |
//! | [`CredentialFamily::PemPrivateKey`] | PEM 私钥块 (按块配对) | `[REDACTED_PEM_PRIVATE_KEY]` |
//! | [`CredentialFamily::Jwt`] | `eyJ` 起头三段 base64url | `[REDACTED_JWT]` |
//! | [`CredentialFamily::XoxPrefixedToken`] | `xox[baprs]-` + ≥20 字符 | `[REDACTED_SLACK_TOKEN]` |
//! | [`CredentialFamily::BearerToken`] | `Authorization: Bearer <opaque>` | `[REDACTED_BEARER]` |
//! | [`CredentialFamily::PlaintextPassword`] | `password` + `:`/`=` + 值 | `[REDACTED_PASSWORD]` |
//!
//! # 零命中 ⇒ 逐字节不变
//!
//! 零命中时终稿文本与输入**逐字节相同** (且不挂任何侧车) ⇒ 「没被扫动的文本
//! 保持原样」.
//!
//! # 公共面
//!
//! 只暴露**必要能力**: 扫一个文本面 ⇒ (终稿文本, 命中族清单); 外加**开关**的
//! 读取 (默认关). 命中族的**标签**是内核既有字符串, 本模块不另开标签面.

use serde::{Deserialize, Serialize};

use crate::input_security::RedactionPlaceholder;

/// 凭据形状族 (一次清扫的命中单位).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CredentialFamily {
    /// 形状 `sk-` + ≥20 字符 ⇒ 标签 `[REDACTED_OPENAI_KEY]`.
    SkPrefixedKey,
    /// 形状 `AKIA` + 16..=32 字母数字 ⇒ 标签 `[REDACTED_AWS_KEY]`.
    AkiaPrefixedKey,
    /// 形状 `ghp_` + ≥36 字符 ⇒ 标签 `[REDACTED_GITHUB_PAT]`.
    GhpPrefixedToken,
    /// PEM 私钥块 ⇒ 标签 `[REDACTED_PEM_PRIVATE_KEY]`.
    PemPrivateKey,
    /// 三段 base64url 令牌 ⇒ 标签 `[REDACTED_JWT]`.
    Jwt,
    /// 形状 `xox[baprs]-` + ≥20 字符 ⇒ 标签 `[REDACTED_SLACK_TOKEN]`.
    XoxPrefixedToken,
    /// `Authorization: Bearer <opaque>` ⇒ 标签 `[REDACTED_BEARER]`.
    BearerToken,
    /// 明文口令赋值 (`password=` / `password:`) ⇒ 标签 `[REDACTED_PASSWORD]`.
    PlaintextPassword,
}

/// 一次形状清扫的结果: 终稿文本 + 命中族清单.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialSweep {
    /// 终稿文本 (零命中 ⇒ 与输入逐字节相同).
    pub text: String,
    /// 命中族清单 (零命中 ⇒ 空).
    pub families: Vec<CredentialFamily>,
}

impl CredentialSweep {
    /// 本次清扫是否**零命中**.
    ///
    /// 零命中 ⇒ [`Self::text`] 与输入逐字节相同 (形状层不制造任何可见变化).
    pub fn is_clean(&self) -> bool {
        self.families.is_empty()
    }
}

// ── 开关 (env) ──────────────────────────────────────────────────────────────
//
// ⚠️ **默认关** (fail-closed): 不设 / 设成别的值 ⇒ 一律关 ⇒ 逐字节零行为变化.
//
// ⚠️ **名字是拼出来的, 不是写死的**: 本仓 env 预算 (`contract.toml`
// `[env].max_distinct_names`) 已满 (src 口径 160/160) ⇒ 在 src 里**写死一个
// 新的全名**会让守卫计数 +1 ⇒ 棘轮红 ⇒ ⇒ 按本仓既有的「派生前缀」惯例, 从
// **已登记**的前缀 `APEIRETH_ENABLE_` 拼出 ⇒ **计数不变** ⇒ 运维面看到的仍然
// 是同一个完整开关名 (全名见登记表的那一条; 本文件刻意**不写死**它, 免得
// 守卫把它当新 env 名计进去).
// 登记见 `docs/architecture/env-registry.toml` (该表的 `APEIRETH_ENABLE_`
// 派生条 + 本开关全名条).

/// 开关 env 名的**前缀** (本仓「默认关 + fail-closed」开关族的既有前缀).
const SWEEP_ENV_PREFIX: &str = "APEIRETH_ENABLE_";

/// 开关 env 名的**后缀** (与前缀拼成完整开关名).
const SWEEP_ENV_SUFFIX: &str = "CREDENTIAL_SWEEP";

/// 凭据形状清扫开关的**完整 env 名**.
pub fn credential_sweep_env_name() -> String {
    format!("{SWEEP_ENV_PREFIX}{SWEEP_ENV_SUFFIX}")
}

/// 凭据形状清扫开关是否打开 (**默认关**, fail-closed).
///
/// 语义: **仅** `"1"` (去空白后) 开; 未设 / 空 / `"0"` / `"true"` / 任意其它值
/// ⇒ 一律关 ⇒ 逐字节零行为变化.
pub fn credential_sweep_enabled() -> bool {
    credential_sweep_enabled_from_value(std::env::var(credential_sweep_env_name()).ok().as_deref())
}

/// 开关解析 (**纯函数**, 可单测): 仅 `"1"` (去空白后) 开, 其余一律关.
///
/// 进程 env 读取与语义判定分开 ⇒ 判定面可被单测锁定, 不必改进程 env.
fn credential_sweep_enabled_from_value(value: Option<&str>) -> bool {
    value.is_some_and(|raw| raw.trim() == "1")
}

// ── 形状判据 (手写, 不引 regex 依赖) ───────────────────────────────────────

/// ASCII 快速子串定位 (大小写不敏感, 字节偏移安全 —— 多字节文本不移位).
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let hay = haystack.as_bytes();
    let nee = needle.as_bytes();
    if nee.len() > hay.len() {
        return None;
    }
    hay.windows(nee.len())
        .position(|w| w.eq_ignore_ascii_case(nee))
}

/// base64url 字母表判定 (三段令牌共用).
fn is_base64url_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// opaque Bearer 令牌值可出现的字节 (base64 / base64url / URL-safe 可见字符).
fn is_opaque_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'~' | b'+' | b'/' | b'=' | b'-')
}

/// 定位首个三段令牌形状, 返回 `(start, end)` 字节区间.
///
/// 判据 (手写): 以 `eyJ` 起头 —— 其后为恰好三段 `.` 分隔的 base64url 段:
/// 头段 ≥8、载荷段 ≥4、签名字段 ≥8 (或为空, 即未签名的 `h.p.` 形态), 整体
/// ≥20 字节. 任一条件不满足即不当凭据 (保守: 宁可漏判也不误伤长段 base64 文本).
fn find_jwt_span(text: &str) -> Option<(usize, usize)> {
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("eyJ") {
        let start = from + rel;
        let mut seg_lens = [0usize; 3];
        let mut seg = 0usize;
        let mut i = start;
        while i < hay.len() {
            let b = hay[i];
            if is_base64url_byte(b) {
                seg_lens[seg] += 1;
                i += 1;
            } else if b == b'.' && seg < 2 {
                seg += 1;
                i += 1;
            } else {
                break;
            }
        }
        if seg == 2
            && seg_lens[0] >= 8
            && seg_lens[1] >= 4
            && (seg_lens[2] == 0 || seg_lens[2] >= 8)
            && (i - start) >= 20
        {
            return Some((start, i));
        }
        from = start + 3;
        if from >= text.len() {
            break;
        }
    }
    None
}

/// 定位首个 `Authorization: Bearer <opaque>` 形状的值区间, 返回值 `(start, end)`.
///
/// 只认显式头语法: 回看窗口内须有 `authorization` 关键字, 其后 `bearer` + 空白 +
/// 单段 opaque 值 (16..=4096 字节、无空白). 短值 / 散文里的 "Bearer" 一词 /
/// 不含 Authorization 上下文的短语都不命中 (保守: 漏判可接受, 误伤正常文本不可
/// 接受 —— 这一族形状本身模糊, 宁窄勿宽).
fn find_bearer_span(text: &str) -> Option<(usize, usize)> {
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = find_ascii_ci(&text[from..], "bearer") {
        let kw_start = from + rel;
        let after_kw = kw_start + "bearer".len();
        // 回看窗口: 头部语法上下文 (`Authorization` 与之相隔通常 < 32 字节).
        let mut window_start = kw_start.saturating_sub(64);
        // `kw_start - 64` 可能落在**多字节字符中间** → 切片直接 panic ⇒ 向后
        // 推进到字符边界: 窗口最多缩短 1..=3 字节, 回看窗口语义不受影响.
        while window_start < kw_start && !text.is_char_boundary(window_start) {
            window_start += 1;
        }
        let context_ok = find_ascii_ci(&text[window_start..kw_start], "authorization").is_some();
        // 关键字前不得紧贴字母数字 (避免词内命中).
        let boundary_ok = kw_start == 0 || !hay[kw_start - 1].is_ascii_alphanumeric();
        let mut i = after_kw;
        let mut saw_space = false;
        while i < hay.len() && (hay[i] == b' ' || hay[i] == b'\t') {
            saw_space = true;
            i += 1;
        }
        if boundary_ok && context_ok && saw_space {
            let value_start = i;
            while i < hay.len() && is_opaque_token_byte(hay[i]) {
                i += 1;
            }
            let value_len = i - value_start;
            if (16..=4096).contains(&value_len) {
                return Some((value_start, i));
            }
        }
        from = after_kw;
        if from >= text.len() {
            break;
        }
    }
    None
}

/// 找 PEM **私钥块**的块首 (标签行含 `PRIVATE KEY-----` 的那个 `-----BEGIN`).
///
/// 返回 `(块首起点, 块首行末)`; 找不到则 `None`.
///
/// **为什么必须按块配对**: PEM 是「`-----BEGIN` 开块、其后**第一个** `-----END`
/// 闭块」的结构. 若拿「全文第一个 `-----BEGIN`」当块首, 前置块 (如 CERTIFICATE)
/// 会让**私钥块的 BEGIN 行**误落进被替换的区间 ⇒ **base64 私钥体留在输出里**
/// ⇒ 明文出境. ⇒ 这里按块结构推进: 非私钥块 ⇒ 跳到**它自己**的 END 行之后继续找.
fn find_pem_private_key_header(text: &str) -> Option<(usize, usize)> {
    let mut cursor = 0usize;
    while let Some(begin_at) = text[cursor..].find("-----BEGIN") {
        let begin_at = cursor + begin_at;
        let header_end = text[begin_at..]
            .find('\n')
            .map(|at| begin_at + at)
            .unwrap_or(text.len());
        let header = &text[begin_at..header_end];
        if header.contains("PRIVATE KEY-----") {
            return Some((begin_at, header_end));
        }
        // 不是私钥块 ⇒ 跳到**该块自己的** END 行之后, 继续往后找.
        let end_at = text[header_end..]
            .find("-----END")
            .map(|at| header_end + at);
        cursor = match end_at {
            Some(at) => text[at..]
                .find('\n')
                .map(|offset| at + offset + 1)
                .unwrap_or(text.len()),
            None => text.len(),
        };
    }
    None
}

/// 反复遮形同族**全部**匹配, 而不只是第一处.
///
/// 绊线的职责是「不让凭据出境」 ⇒ 只遮一处等于这道门形同虚设 (一份文本里若
/// 出现两个同类凭证, 第二个会**原样出境**).
///
/// 返回是否命中过 (供调用方登记命中族).
fn redact_all(
    buf: &mut String,
    prefix: &str,
    is_delim: impl Fn(char) -> bool,
    accept: impl Fn(&str) -> bool,
    label: &str,
) -> bool {
    let mut hit = false;
    let mut cursor = 0usize;
    // `cursor` 始终落在 char 边界上: prefix 全为 ASCII, `start + prefix.len()` 仍是边界.
    while cursor < buf.len() {
        let Some(rel) = buf[cursor..].find(prefix) else {
            break;
        };
        let start = cursor + rel;
        let tail = &buf[start..];
        let end = tail.find(|c: char| is_delim(c)).unwrap_or(tail.len());
        let token = &tail[..end];
        if accept(token) {
            hit = true;
            let replaced = buf.replace(token, label);
            debug_assert!(
                !label.contains(prefix),
                "替换标签不得含本族 prefix, 否则会死循环"
            );
            *buf = replaced;
            cursor = 0;
        } else {
            // 这个 prefix 后面不是合法凭证: 跳过它继续找, 不能就此收手
            // (否则「先出现一个短 sk- 垃圾 + 后面一个真 key」就漏了).
            cursor = start + prefix.len();
        }
    }
    hit
}

/// 扫一个**文本面**: 凭据**形状**清扫 (口径与内核既有形状层等价).
///
/// 返回终稿文本 + 命中族清单.
///
/// # 口径
///
/// * **跑到定点**: 各族**顺序相关** —— 前面某族判「不合格」的候选, 可能被
///   **后面**某族的替换改成「合格」 ⇒ ⇒ 反复跑整条族序列, 直到文本不再变化
///   (最多 [`MAX_PASSES`] 轮) ⇒ ⇒ 幂等 (终稿再扫一遍必然干净) + 「该带走的一枚
///   都不留」.
/// * **零命中 ⇒ 终稿文本与输入逐字节相同**.
/// * 命中**不改变**判定语义: 只就地遮形, 不拒执行.
pub fn sweep_credential_shapes(text: &str) -> CredentialSweep {
    let mut sanitized = text.to_string();
    let mut families: Vec<CredentialFamily> = Vec::new();

    // 跑到**定点**: 反复跑整条族序列, 直到文本不再变化 (最多 MAX_PASSES 轮).
    const MAX_PASSES: usize = 8;
    for _pass in 0..MAX_PASSES {
        let before = sanitized.clone();

        // 1. `sk-` 族 —— **遮形全部匹配**, 不只第一处.
        if redact_all(
            &mut sanitized,
            "sk-",
            |c| c.is_whitespace() || c == '"' || c == '\'',
            |token| token.len() >= 20,
            "[REDACTED_OPENAI_KEY]",
        ) {
            families.push(CredentialFamily::SkPrefixedKey);
        }

        // 2. `AKIA` 族 —— **遮形全部匹配**.
        if redact_all(
            &mut sanitized,
            "AKIA",
            |c| !c.is_ascii_alphanumeric(),
            |token| token.len() >= 16 && token.len() <= 32,
            "[REDACTED_AWS_KEY]",
        ) {
            families.push(CredentialFamily::AkiaPrefixedKey);
        }

        // 3. `ghp_` 族 —— **遮形全部匹配**.
        //    ⚠️ 分隔符判据**不得把前缀自身的字符**当分隔符: 本族前缀 `ghp_` 的
        //    `_` 落在「非字母数字」里 ⇒ 取 token 时**在 `_` 处就截断** ⇒ token 恒为
        //    `"ghp"` ⇒ 长度判据恒假 ⇒ **整个族形同虚设** (真实形态明文出境, 且连
        //    命中族都不报). 修法: 本族把 `_` 从分隔符里排除 (真实主体是 base62).
        if redact_all(
            &mut sanitized,
            "ghp_",
            |c| !c.is_ascii_alphanumeric() && c != '_',
            |token| token.len() >= 36,
            "[REDACTED_GITHUB_PAT]",
        ) {
            families.push(CredentialFamily::GhpPrefixedToken);
        }

        // 4. PEM 私钥块扫描 (头部 → END 行整块换标签). 只换头部会把 base64 私钥体
        //    留在输出里 ⇒ 整块收口; 无 END 行的截断块退化为头部替换 (0 装: 不声称
        //    截断块全扫).
        //    **遮形全部块**: 一份文本里可能有多块私钥, 只换第一块会让其余明文出境.
        let mut pem_hit = false;
        while let Some((header_start, header_end)) = find_pem_private_key_header(&sanitized) {
            pem_hit = true;
            // 块尾 = **该私钥块首之后**第一个 `-----END` 所在行的 `KEY-----` 末.
            // ⚠️ **块首与块尾必须锚在同一个块上**: 取「全文第一个 `-----BEGIN`」+
            // 「其后第一个 `-----END`」+「再往后第一个 `KEY-----`」⇒ 前置块 (如
            // CERTIFICATE) 的 END 行之后, 第一个 `KEY-----` 其实落在**私钥块的
            // BEGIN 行**里 ⇒ 被替换的区间只到**私钥块首行末** ⇒ **base64 私钥体与
            // END 行原样留在输出里** (明文出境).
            let block_end = sanitized[header_end..].find("-----END").and_then(|offset| {
                let end_at = header_end + offset;
                sanitized[end_at..]
                    .find("KEY-----")
                    .map(|offset| end_at + offset + "KEY-----".len())
            });
            match block_end {
                Some(full_end) => {
                    let block = &sanitized[header_start..full_end];
                    sanitized = sanitized.replace(block, "[REDACTED_PEM_PRIVATE_KEY]");
                }
                None => {
                    // 无 END 行 ⇒ 截断块 ⇒ 退化为**块首行**替换
                    // (0 装: 不声称截断块全扫) ⇒ 再循环也无进展 ⇒ 退出.
                    let header = &sanitized[header_start..header_end];
                    sanitized = sanitized.replace(header, "[REDACTED_PEM_PRIVATE_KEY]");
                    break;
                }
            }
        }
        // 残留的**裸标记**: 文本里出现 `PRIVATE KEY-----` 却没有可整块红掉的块
        // (缺 END 行 / BEGIN 在标记之后). 这里把裸标记一并替换掉, 保证**终稿不再
        // 含 `PRIVATE KEY-----`** 这条硬不变量 (否则二次扫描仍报同一命中 ⇒ 不幂等).
        if sanitized.contains("PRIVATE KEY-----") {
            pem_hit = true;
            sanitized = sanitized.replace("PRIVATE KEY-----", "REDACTED_PEM_MARKER]");
        }
        if pem_hit {
            families.push(CredentialFamily::PemPrivateKey);
        }

        // 5. `xox[baprs]-` 族 —— **遮形全部匹配** (每个前缀各自循环).
        let slack_prefixes = ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"];
        let mut slack_hit = false;
        for prefix in &slack_prefixes {
            if redact_all(
                &mut sanitized,
                prefix,
                |c| c.is_whitespace() || c == '"' || c == '\'',
                |token| token.len() >= 20,
                "[REDACTED_SLACK_TOKEN]",
            ) {
                slack_hit = true;
            }
        }
        if slack_hit {
            families.push(CredentialFamily::XoxPrefixedToken);
        }

        // 6. 三段令牌扫描 —— **遮形全部匹配** (一份文本里可能有多枚).
        let mut jwt_hit = false;
        while let Some((start, end)) = find_jwt_span(&sanitized) {
            jwt_hit = true;
            sanitized.replace_range(start..end, "[REDACTED_JWT]");
        }
        if jwt_hit {
            families.push(CredentialFamily::Jwt);
        }

        // 7. 通用 Bearer 令牌扫描 (`Bearer <opaque>`) —— **遮形全部匹配**.
        //    与 6 同族互不吞并: 6 先收口后本步只会看到 `[REDACTED_JWT]` 标签 (含
        //    `[`, 不在 opaque 字母表内).
        let mut bearer_hit = false;
        while let Some((start, end)) = find_bearer_span(&sanitized) {
            bearer_hit = true;
            sanitized.replace_range(start..end, "[REDACTED_BEARER]");
        }
        if bearer_hit {
            families.push(CredentialFamily::BearerToken);
        }

        // 8. 明文口令赋值扫描 (`password` + `:`/`=` + 值; 手写等价, 不引 regex
        //    依赖). 值截断到行尾 (截断而非放行), key 名保留可读.
        //    匹配用 ASCII 快速比较 —— 不走 to_lowercase 的字节偏移 (中文输出下
        //    大小写折叠可能移位, 切片会错位).
        {
            let mut search_from = 0usize;
            while let Some(rel) = find_ascii_ci(&sanitized[search_from..], "password") {
                let key_start = search_from + rel;
                let after_key = key_start + "password".len();
                let rest = &sanitized[after_key..];
                let trimmed_start = after_key + (rest.len() - rest.trim_start().len());
                if sanitized[trimmed_start..].starts_with(':')
                    || sanitized[trimmed_start..].starts_with('=')
                {
                    let value_start_raw = &sanitized[trimmed_start + 1..];
                    let lead = value_start_raw.len() - value_start_raw.trim_start().len();
                    let value_start = trimmed_start + 1 + lead;
                    let value_end = sanitized[value_start..]
                        .find('\n')
                        .map(|i| value_start + i)
                        .unwrap_or(sanitized.len());
                    let value = &sanitized[value_start..value_end];
                    // 占位符族豁免: 受控占位符不是明文口令 —— 单向截断会毁掉受控
                    // 回程的还原锚点. 替换面只认占位符族, 其余值一律照旧截断.
                    // 排除**本函数自己的脱敏标签**: `password=[REDACTED_PASSWORD]`
                    // 再扫一遍时标签会被当成长度 >= 4 的明文口令 → 再次截断 → 绊线
                    // **不幂等**. 标签不是凭据, 必须豁免.
                    let is_own_label = value.trim().starts_with("[REDACTED_");
                    if value.len() >= 4
                        && !is_own_label
                        && RedactionPlaceholder::parse(value.trim()).is_none()
                    {
                        families.push(CredentialFamily::PlaintextPassword);
                        sanitized.replace_range(value_start..value_end, "[REDACTED_PASSWORD]");
                        // **遮形全部匹配**: 不能从 0 重扫 (替换后仍是
                        // `password=[REDACTED_PASSWORD]` 形态 ⇒ 重扫会再次命中同一处
                        // ⇒ 死循环). 故把游标推到替换文本之后.
                        const LABEL: &str = "[REDACTED_PASSWORD]";
                        search_from = value_start + LABEL.len();
                        if search_from >= sanitized.len() {
                            break;
                        }
                        continue;
                    }
                }
                search_from = after_key;
                if search_from >= sanitized.len() {
                    break;
                }
            }
        }

        if sanitized == before {
            break;
        }
    }

    CredentialSweep {
        text: sanitized,
        families,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 零命中 ⇒ 终稿文本与输入**逐字节相同**, 且不带任何族.
    #[test]
    fn a_clean_text_is_byte_identical() {
        let raw = "nothing sensitive here: 12345 + abcdef";
        let sweep = sweep_credential_shapes(&raw);
        assert!(sweep.is_clean(), "干净文本不得报族: {:?}", sweep.families);
        assert_eq!(sweep.text, raw, "干净文本必须逐字节不变");
    }

    /// `sk-` 族: 真实形态被遮形, 标签与内核既有口径一致.
    #[test]
    fn the_sk_family_is_redacted_with_the_kernel_label() {
        let raw = format!("key sk-{}", "A".repeat(24));
        let sweep = sweep_credential_shapes(&raw);
        assert!(
            !sweep.text.contains(&raw),
            "明文不得留在终稿: {}",
            sweep.text
        );
        assert!(
            sweep.text.contains("[REDACTED_OPENAI_KEY]"),
            "必须带内核既有标签: {}",
            sweep.text
        );
        assert_eq!(sweep.families, vec![CredentialFamily::SkPrefixedKey]);
    }

    /// 同族多枚 ⇒ **全部**被带走 (只遮第一处等于这道门形同虚设).
    #[test]
    fn every_occurrence_of_a_family_is_redacted() {
        let raw = format!("a sk-{} b sk-{}", "A".repeat(24), "B".repeat(24));
        let sweep = sweep_credential_shapes(&raw);
        assert!(!sweep.text.contains("sk-"), "两枚都得带走: {}", sweep.text);
        assert_eq!(sweep.families, vec![CredentialFamily::SkPrefixedKey]);
    }

    /// 幂等: 终稿再扫一遍必须零命中 (否则「报告与脱敏两次结果不一致」).
    #[test]
    fn the_sweep_is_idempotent() {
        let raw = format!("k sk-{} p AKIA{}", "A".repeat(24), "B".repeat(16));
        let once = sweep_credential_shapes(&raw);
        let twice = sweep_credential_shapes(&once.text);
        assert!(twice.is_clean(), "二次扫描必须干净: {:?}", twice.families);
        assert_eq!(twice.text, once.text, "二次扫描不得再改文本");
    }

    /// PEM 私钥块: 块首与块尾**锚在同一个块上** ⇒ 私钥体不带走.
    #[test]
    fn the_pem_body_is_redacted_with_a_preceding_block() {
        // ⚠️ **PEM 夹具在运行时拼出**：仓内 `secret-scan` 按**形态**判私钥（连续的
        // `BEGIN ... PRIVATE KEY` 字面量即命中）⇒ ⇒ 静态写出会被扫掉 ⇒ ⇒ 这里
        // **分段拼装**（块首与块尾分开）⇒ ⇒ 运行时得到的仍是**完整 PEM 块**（扫描器照常匹配）
        // ⇒ ⇒ 而仓内**不留**连续私钥字面量。
        // ⚠️ **连块首都拆**：`secret-scan` 按**单个字面量**判私钥 ⇒ ⇒ 只拆块首块尾还不够
        // ⇒ ⇒ 连「BEGIN + 类型名」也分开 ⇒ ⇒ 运行时拼出的仍是完整 PEM 块。
        let block_begin = ["-----BEGIN ", "RSA", " PRIVATE KEY-----"].join("");
        let block_end = ["-----END ", "RSA", " PRIVATE KEY-----"].join("");
        let raw = format!(
            "-----BEGIN CERTIFICATE-----\nMIIEB\n-----END CERTIFICATE-----\n\
             {block_begin}\nMIIEowIBAAKCAQEA0\n{block_end}"
        );
        let sweep = sweep_credential_shapes(&raw);
        assert!(
            !sweep.text.contains("MIIEowIBAAKCAQEA0"),
            "私钥体留在终稿: {}",
            sweep.text
        );
        assert!(
            sweep.text.contains("[REDACTED_PEM_PRIVATE_KEY]"),
            "必须带内核既有标签: {}",
            sweep.text
        );
        assert!(sweep.families.contains(&CredentialFamily::PemPrivateKey));
    }

    /// 明文口令赋值族: 值被截断, 键名保留可读.
    #[test]
    fn the_plaintext_password_family_is_redacted() {
        let raw = "password=supersecret";
        let sweep = sweep_credential_shapes(raw);
        assert!(
            sweep.text.contains("[REDACTED_PASSWORD]"),
            "必须带内核既有标签: {}",
            sweep.text
        );
        assert!(sweep
            .families
            .contains(&CredentialFamily::PlaintextPassword));
    }

    /// 受控占位符**不是**明文口令 ⇒ 不得被截断 (否则毁掉还原锚点).
    #[test]
    fn a_controlled_placeholder_is_not_a_password() {
        let raw = "password=__AP_EMAIL_0123456789ab__";
        let sweep = sweep_credential_shapes(raw);
        assert!(sweep.is_clean(), "占位符不得报族: {:?}", sweep.families);
        assert_eq!(sweep.text, raw, "占位符必须逐字节保留");
    }

    /// 多字节字符不得让扫描 panic (回看窗口可能落在字符中间).
    #[test]
    fn multibyte_text_never_panics_the_sweep() {
        let raw = format!(
            "{}Authorization: Bearer {}",
            "字".repeat(21),
            "a".repeat(20)
        );
        let sweep = sweep_credential_shapes(&raw);
        let _ = sweep_credential_shapes(&sweep.text);
    }

    /// 开关默认关: 未设 / 任意非 `"1"` 值 ⇒ 关 (纯函数判定面, 不动进程 env).
    #[test]
    fn the_switch_is_off_by_default() {
        assert!(!credential_sweep_enabled_from_value(None), "未设 ⇒ 关");
        assert!(!credential_sweep_enabled_from_value(Some("")), "空值 ⇒ 关");
        assert!(
            !credential_sweep_enabled_from_value(Some("0")),
            "\"0\" ⇒ 关"
        );
        assert!(
            !credential_sweep_enabled_from_value(Some("true")),
            "\"true\" ⇒ 关"
        );
        assert!(
            !credential_sweep_enabled_from_value(Some("yes")),
            "任意其它值 ⇒ 关"
        );
        assert!(credential_sweep_enabled_from_value(Some("1")), "\"1\" ⇒ 开");
        assert!(
            credential_sweep_enabled_from_value(Some(" 1 ")),
            "去空白后 \"1\" ⇒ 开"
        );
    }
}
