//! 工具调用前置防御（Pre-Call Guard）与后置出站凭据绊线（Post-Call Tripwire）.
//!
//! 在能力执行前拦截路径穿越与高危 Shell 注入，在能力执行后扫描输出中的敏感凭据，
//! 阻断凭据外泄并防止被大模型长程记忆污染.
//! 出境脱敏双向闸接线面 [`ToolGuardrail::sweep_output_for_egress`]: 先占位族
//! 出境脱敏 (可还原), 再既有凭据绊线 (单向标签), 两族共存各零回归.
//!
//! # 接线面 (生产)
//!
//! [`EgressSweeper`] 把「脱敏域 (会话密钥 + 登记簿 + 账本)」与「既有凭据绊线」
//! 绑成一个可复用的清扫器: 接线点交一段文本进来, 拿终稿文本 + 两族真账出去.
//! 外部工具族（协议桥的远端结果面）接了这一道 —— 远端服务器回吐的内容是
//! **不可信源**, 它的凭据形状不得裸奔进模型上下文.
//!
//! 一次清扫 **不新增处置语义**: 逐字走
//! [`ToolGuardrail::sweep_output_for_egress`] —— 占位族脱敏 (真值 → 不可逆
//! 占位, 登记入簿) + 凭据绊线 (单向标签, 永不还原). 命中既不改判也不拒执行:
//! 文本被就地改写, 调用照常完成.

use crate::redaction_bridge::{EgressRedaction, LedgerEntry, RedactionBridge};
use apeireth_governance::input_security::RedactionPlaceholder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// 前置守门拦截错误.
#[derive(Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PreCallGuardError {
    #[error("检测到路径穿越或敏感系统路径访问: {0}")]
    PathTraversal(String),
    /// 目标路径落在工作区目录之外 —— 目录边界拒绝. 与词法攻击面/受保护
    /// 路径不同, 这类拒绝可经一次性升级 (见 `escalation` 模块) 就地引导.
    #[error("{0}")]
    OutsideWorkspace(String),
    #[error("检测到高危破坏性 Shell 注入或命令: {0}")]
    DangerousCommandInjection(String),
}

/// 泄露凭据类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeakedCredentialKind {
    OpenAiKey,
    AwsAccessKey,
    GitHubPat,
    PemPrivateKey,
    JwtToken,
    SlackToken,
    GenericBearer,
    /// 明文口令赋值 (`password=...` / `password: ...`, W1 §2.2 设计点名).
    PlaintextPassword,
}

/// 后置出站绊线扫描结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripwireScanResult {
    pub is_clean: bool,
    pub sanitized_output: String,
    pub leaked_kinds: Vec<LeakedCredentialKind>,
}

/// 出境双层扫结果: 占位族脱敏 (登记入簿, 受控回程可还原) + 既有凭据绊线
/// (PEM/口令赋值等形状, 单向标签) —— 两族在同一份输出上共存.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressSweepResult {
    pub redaction: EgressRedaction,
    pub tripwire: TripwireScanResult,
}

/// 工具执行守门员.
#[derive(Debug, Clone, Default)]
pub struct ToolGuardrail;

/// ASCII 快速子串定位 (大小写不敏感, 字节偏移安全 —— 多字节文本不移位)。
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let hay = haystack.as_bytes();
    let nee = needle.as_bytes();
    if nee.len() > hay.len() {
        return None;
    }
    hay.windows(nee.len())
        .position(|w| w.eq_ignore_ascii_case(nee))
}

/// base64url 字母表判定 (JWT 三段共用).
fn is_base64url_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// opaque Bearer 令牌值可出现的字节 (base64 / base64url / URL-safe 可见字符).
fn is_opaque_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'~' | b'+' | b'/' | b'=' | b'-')
}

/// 定位首个 JWT 形状令牌, 返回 `(start, end)` 字节区间.
///
/// 判据 (手写, 不引 regex 依赖): 以 `eyJ` 起头 (base64url 编码的 JSON 对象头,
/// 首字节 `{` + ASCII 字母必然落在这一前缀上) —— 其后为恰好三段 `.` 分隔的
/// base64url 段: 头段 ≥8、载荷段 ≥4、签名字段 ≥8 (或为空, 即 `alg=none` 的
/// 未签名 `h.p.` 形态), 整体 ≥20 字节. 任一条件不满足即不当凭据 (保守: 宁可
/// 漏判也不误伤长段 base64 文本)。
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
/// 单段 opaque 值 (16..=4096 字节、无空白)。短值 / 散文里的 "Bearer" 一词 /
/// 不含 Authorization 上下文的短语都不命中 (保守: 漏判可接受, 误伤正常文本不可
/// 接受 —— 这一族形状本身模糊, 宁窄勿宽)。
fn find_bearer_span(text: &str) -> Option<(usize, usize)> {
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = find_ascii_ci(&text[from..], "bearer") {
        let kw_start = from + rel;
        let after_kw = kw_start + "bearer".len();
        // 回看窗口: 头部语法上下文 (`Authorization` 与之相隔通常 < 32 字节)。
        let mut window_start = kw_start.saturating_sub(64);
        // `kw_start - 64` 可能落在**多字节字符中间** → 切片直接 panic
        // (fuzz 实测: 含多字节字符的输入报 `byte index ... is not a char boundary`)。
        // 向后推进到字符边界: 窗口最多缩短 1..=3 字节, 回看窗口语义不受影响。
        while window_start < kw_start && !text.is_char_boundary(window_start) {
            window_start += 1;
        }
        let context_ok = find_ascii_ci(&text[window_start..kw_start], "authorization").is_some();
        // 关键字前不得紧贴字母数字 (避免词内命中)。
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

/// 找 PEM **私钥块**的块首（标签行含 `PRIVATE KEY-----` 的那个 `-----BEGIN`）。
///
/// 返回 `(块首起点, 块首行末)`；找不到则 `None`。
///
/// **为什么必须按块配对**（2026-10-25 fuzz 发现）：PEM 是「`-----BEGIN` 开块、
/// 其后**第一个** `-----END` 闭块」的结构。若拿「全文第一个 `-----BEGIN`」当块首，
/// 前置块（如 CERTIFICATE）会让**私钥块的 BEGIN 行**误落进被替换的区间
/// ⇒ **base64 私钥体留在输出里 ⇒ 明文出境**。
///
/// ⇒ 这里按块结构推进：非私钥块 ⇒ 跳到**它自己**的 END 行之后继续找。
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
        // 不是私钥块 ⇒ 跳到**该块自己的** END 行之后, 继续往后找。
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

impl ToolGuardrail {
    pub fn new() -> Self {
        Self
    }

    /// 前置路径安全检查.
    pub fn verify_path_access(
        workspace_root: &Path,
        requested_path: &Path,
    ) -> Result<(), PreCallGuardError> {
        let path_str = requested_path.to_string_lossy();

        // 1. 拦截明显的路径穿越特征
        if path_str.contains("../") || path_str.contains("..\\") || path_str == ".." {
            return Err(PreCallGuardError::PathTraversal(format!(
                "禁止相对路径穿越: {}",
                path_str
            )));
        }

        // 2. 拦截绝对敏感路径
        let lower_path = path_str.to_lowercase();
        let forbidden_prefixes = [
            "/etc/shadow",
            "/etc/passwd",
            "/etc/sudoers",
            "/root",
            "/var/run",
            "/dev",
            "c:\\windows\\system32",
            "c:\\windows\\system",
        ];

        for prefix in &forbidden_prefixes {
            if lower_path.starts_with(prefix) {
                return Err(PreCallGuardError::PathTraversal(format!(
                    "禁止访问操作系统核心敏感路径: {}",
                    path_str
                )));
            }
        }

        // 3. 若为绝对路径，必须位于 workspace_root 范围内
        if requested_path.is_absolute() {
            if let (Ok(canonical_root), Ok(canonical_target)) =
                (workspace_root.canonicalize(), requested_path.canonicalize())
            {
                if !canonical_target.starts_with(&canonical_root) {
                    return Err(PreCallGuardError::OutsideWorkspace(format!(
                        "目标路径超出工作区边界: {}",
                        path_str
                    )));
                }
            }
        }

        Ok(())
    }

    /// 前置 Shell 命令防注入与破坏性检查.
    ///
    /// 删除族同权口径 (本层不改判, 理由在此写明): 破坏面是**窄词面灾难清单**
    /// (灾难形态字面量 + 系统配置写动词), 不做删除族全量拦截 —— 普通删除族
    /// (`del` / `Remove-Item` / `[System.IO.File]::Delete` 等静态调用面) 一律
    /// 同档放行, 经写意图总闸 + shell 审批链统一兜住; 两条删除 API 路径在本层
    /// 判定逐一相同 (同为放行档), 对称性由 tests/delete_path_parity.rs 锁定。
    /// 灾难字面量不扩面到 cmdlet / 静态调用形态的理由: 词面清单无法可靠判定
    /// 灾难作用域, 单边扩面会造成两条删除路径的审批档判定不一致 —— 恰是删除
    /// 同权要消除的权力差; 守门不是沙箱的替代, 灾难级作用域由总闸 + 人工审批
    /// + 沙箱三层兜住。
    pub fn verify_shell_command(command_line: &str) -> Result<(), PreCallGuardError> {
        let trimmed = command_line.trim();
        let lower = trimmed.to_lowercase();
        // 空白归一化后再做禁词匹配: **多空白折叠成单空格**。
        //
        // 为什么必须归一化 (2026-10-24 fuzz 发现的**真实绕过通道**):
        // shell 的参数分隔符可以是任意空白 —— 空格 / TAB / 换行 / 连续空格。
        // 下面两张禁词表里的项都是**用单空格书写**的 (`rm -rf /`、`netsh` + `" set"`),
        // 若按原样做子串匹配, 那么 `netsh<TAB>set advfirewall ...` 是一条**合法命令**
        // 却匹配不到 `" set"` (前导是 TAB 不是空格) → 直接绕过「高危系统配置修改」拦截。
        //
        // 实测(本机, 修复前):
        //   "netsh set advfirewall x"   → 拦截 ✅
        //   "netsh\tset advfirewall x"  → **放行** ❌   ← 绕过
        //   "netsh\nset advfirewall x"  → **放行** ❌   ← 绕过
        //
        // 归一化只会让判定**更严格**(禁词本身是单空格形态), 不会放宽任何原有拦截。
        // 诚实边界: 本函数仍是**子串启发式**, 不是沙箱 —— `net"sh" set`、变量拼接、
        // 绝对路径前缀等仍不在其覆盖范围; 真正的兜底是沙箱 + 审批 (见本函数上文)。
        let lower = lower.split_whitespace().collect::<Vec<_>>().join(" ");

        // 高危不可逆破坏性命令
        let forbidden_commands = [
            "rm -rf /",
            "rm -rf /*",
            "rmdir /s /q c:\\",
            "mkfs.",
            "dd if=",
            "format c:",
            ":(){ :|:& };:", // Fork 炸弹
            "shutdown -h now",
            "shutdown /s",
            "reboot",
            // W1 §2.3 系统修改命令 (2026-10-10 批): 注册表/服务/网络配置/属性隐藏。
            // 白名单否定式纵深 —— 守门不是沙箱的替代。
            "reg add",
            "reg delete",
            "reg import",
            "sc create",
            "sc config",
            "sc delete",
            "netsh set",
            "netsh add",
            "netsh delete",
            "netsh reset",
            "attrib +s",
            "attrib +h",
        ];

        for cmd in &forbidden_commands {
            if lower.contains(cmd) {
                return Err(PreCallGuardError::DangerousCommandInjection(format!(
                    "拦截高危系统破坏指令: {}",
                    cmd
                )));
            }
        }

        // netsh 写入类 (W1 §2.3): 单纯子串表抓不到 "netsh advfirewall set ..." 这种
        // 动词后置形态 —— "netsh" 出现且伴随写动词 (set/add/delete/reset/import) 即拦;
        // show/display 只读放行 (否定式保守, 误报可接受 —— 纵深不是沙箱的替代)。
        if lower.contains("netsh")
            && [" set", " add", " delete", " reset", " import"]
                .iter()
                .any(|verb| lower.contains(verb))
        {
            return Err(PreCallGuardError::DangerousCommandInjection(
                "拦截 netsh 写入类系统配置指令".to_string(),
            ));
        }

        Ok(())
    }

    /// 反复脱敏同族**全部**匹配, 而不只是第一处。
    ///
    /// 2026-10-24 fuzz 发现的**真实泄密路径**: 此前实现每族只处理第一处匹配 ——
    /// 一份工具输出里若出现两个同类凭证, 第二个会**原样出境**。实测修复前:
    /// ```text
    /// 输入  "xoxb-AAA... and xoxb-BBB..."
    /// 输出  "[REDACTED_SLACK_TOKEN] and xoxb-BBB..."   ← 第二个明文留下
    /// ```
    /// 三族（OpenAI / AWS / Slack）实测同样如此。
    /// 绊线的职责是「不让凭据出境」, 只脱敏一处等于这道门形同虚设。
    ///
    /// 返回是否命中过（供调用方登记 `leaked_kinds`）。
    #[allow(clippy::needless_pass_by_value)]
    fn redact_all(
        buf: &mut String,
        prefix: &str,
        is_delim: impl Fn(char) -> bool,
        accept: impl Fn(&str) -> bool,
        label: &str,
    ) -> bool {
        let mut hit = false;
        let mut cursor = 0usize;
        // `cursor` 始终落在 char 边界上: prefix 全为 ASCII, `start + prefix.len()` 仍是边界。
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
                // (否则「先出现一个短 sk- 垃圾 + 后面一个真 key」就漏了)。
                cursor = start + prefix.len();
            }
        }
        hit
    }

    /// 后置出站凭据绊线扫描与脱敏.
    pub fn scan_and_sanitize_output(raw_output: &str) -> TripwireScanResult {
        let mut sanitized = raw_output.to_string();
        let mut leaked_kinds = Vec::new();

        // ── 跑到**定点**: 反复跑整条族序列, 直到文本不再变化 (最多 MAX_PASSES 轮) ──
        //
        // 为什么必须定点 (2026-10-24 fuzz 发现的真实缺陷):
        // 各族**顺序相关** —— 前面某族判「不合格」的候选, 可能被**后面**某族的替换
        // 改成「合格」。实测:
        //   输入 `...sk-aPRIVATE KEY-----\t`
        //   ① OpenAI 族 (先跑): 候选 `sk-aPRIVATE` 长度 11 < 20 → **放行**
        //   ④ PEM 族: 把 `PRIVATE KEY-----` 换成 `REDACTED_PEM_MARKER]`
        //   ⇒ 同一处变成 `sk-aREDACTED_PEM_MARKER]` 长度 24 ≥ 20 → 二次扫描才报
        //   ⇒ 绊线**不幂等**: 报告与脱敏两次结果不一致
        //
        // 跑到定点同时解决两件事: ①幂等 (脱敏结果再扫一遍必然干净);
        // ②「该带走的一枚都不留」(不会因为族序漏掉任何一枚)。
        const MAX_PASSES: usize = 8;
        for _pass in 0..MAX_PASSES {
            let before = sanitized.clone();

            // 1. OpenAI Key 扫描 (sk-...) —— **脱敏全部匹配**, 不只第一处。
            if Self::redact_all(
                &mut sanitized,
                "sk-",
                |c| c.is_whitespace() || c == '"' || c == '\'',
                |token| token.len() >= 20,
                "[REDACTED_OPENAI_KEY]",
            ) {
                leaked_kinds.push(LeakedCredentialKind::OpenAiKey);
            }

            // 2. AWS Key 扫描 (AKIA...) —— **脱敏全部匹配**。
            if Self::redact_all(
                &mut sanitized,
                "AKIA",
                |c| !c.is_ascii_alphanumeric(),
                |token| token.len() >= 16 && token.len() <= 32,
                "[REDACTED_AWS_KEY]",
            ) {
                leaked_kinds.push(LeakedCredentialKind::AwsAccessKey);
            }

            // 3. GitHub PAT 扫描 (ghp_...) —— **脱敏全部匹配**。
            if Self::redact_all(
                &mut sanitized,
                "ghp_",
                // ⚠️ 分隔符判据**不得把前缀自身的字符**当分隔符
                // (2026-10-25 fuzz 发现, 真缺陷 ⑰):
                //   本族前缀 `ghp_` 的 `_` 落在「非字母数字」里 ⇒ `redact_all` 取 token 时
                //   **在 `_` 处就截断** ⇒ token 恒为 `"ghp"`（3 字符）⇒ `len >= 36` 恒假
                //   ⇒ **整个族形同虚设**: 真实形态 (`ghp_` + 36 = 40 字符) **明文出境**,
                //   且**连泄露族都不报**（`leaked_kinds` 为空 ⇒ 绊线也不响）。
                //   对照: 同面其余三族 (sk- / xoxb- / AKIA) 的分隔符判据要么只认空白与引号,
                //   要么前缀是纯字母数字 ⇒ **同一面内口径不一致**, 本族是唯一自相矛盾的那个。
                // 修法: 本族把 `_` 从分隔符里排除 (真实 PAT 主体是 base62, 不含 `_`)。
                |c| !c.is_ascii_alphanumeric() && c != '_',
                |token| token.len() >= 36,
                "[REDACTED_GITHUB_PAT]",
            ) {
                leaked_kinds.push(LeakedCredentialKind::GitHubPat);
            }

            // 4. PEM 私钥块扫描 (头部 → END 行整块换标签)。只换头部会把 base64
            //    私钥体留在输出里 —— 确证级泄漏债, 整块收口; 无 END 行的截断块
            //    退化为头部替换 (0 装: 不声称截断块全扫)。
            // **脱敏全部块**: 一份输出里可能有多块私钥, 只换第一块会让其余明文出境
            // (2026-10-24 fuzz 发现: 两块 PEM 只脱敏了第一块)。
            let mut pem_hit = false;
            while let Some((header_start, header_end)) = find_pem_private_key_header(&sanitized) {
                pem_hit = true;
                // 块尾 = **该私钥块首之后**第一个 `-----END` 所在行的 `KEY-----` 末。
                //
                // ⚠️ **块首与块尾必须锚在同一个块上** (2026-10-25 fuzz 发现):
                //   修复前取「全文第一个 `-----BEGIN`」+「其后第一个 `-----END`」+
                //   「再往后第一个 `KEY-----`」⇒ 前置块 (如 CERTIFICATE) 的 END 行之后,
                //   第一个 `KEY-----` 其实落在**私钥块的 BEGIN 行**里 ⇒ 被替换的区间
                //   只到**私钥块首行末** ⇒ **base64 私钥体与 END 行原样留在输出里**。
                //   实测修复前:
                //     "-----BEGIN CERTIFICATE-----\nX\n-----END CERTIFICATE-----\n
                //      -----BEGIN RSA PRIVATE KEY-----\nSECRET\n-----END RSA PRIVATE KEY-----"
                //     ⇒ 输出 "[REDACTED_PEM_PRIVATE_KEY]\nSECRET\n-----END RSA REDACTED_PEM_MARKER]"
                //     (SECRET = 私钥体, **明文出境**)。
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
                        // (0 装: 不声称截断块全扫) ⇒ 再循环也无进展 ⇒ 退出。
                        let header = &sanitized[header_start..header_end];
                        sanitized = sanitized.replace(header, "[REDACTED_PEM_PRIVATE_KEY]");
                        break;
                    }
                }
            }
            // 残留的**裸标记**: 输出里出现 `PRIVATE KEY-----` 却没有可整块红掉的块
            // (缺 END 行 / BEGIN 在标记之后 —— fuzz 实测的形态)。
            // 此前实现此处只「报告泄露」不「脱敏」: 二次扫描仍报同一泄露 → **绊线不幂等**,
            // 而且明文碎片其实一直留在输出里。这里把裸标记一并替换掉,
            // 保证**脱敏后的输出不再含 `PRIVATE KEY-----`** 这条硬不变量。
            if sanitized.contains("PRIVATE KEY-----") {
                pem_hit = true;
                sanitized = sanitized.replace("PRIVATE KEY-----", "REDACTED_PEM_MARKER]");
            }
            if pem_hit {
                leaked_kinds.push(LeakedCredentialKind::PemPrivateKey);
            }

            // 5. Slack Token 扫描 (xox[baprs]-...)
            let slack_prefixes = ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"];
            let mut slack_hit = false;
            for prefix in &slack_prefixes {
                // **脱敏全部匹配**（每个前缀各自循环）。
                if Self::redact_all(
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
                leaked_kinds.push(LeakedCredentialKind::SlackToken);
            }

            // 6. JWT 扫描 (header.payload.signature). 此前 `JwtToken` 变体只有声明、
            //    没有检测面 —— 命名即承诺的空槽, 本批补齐 (D18).
            // **脱敏全部匹配**: 一份输出里可能有多枚 JWT, 只换第一枚会让其余明文出境。
            let mut jwt_hit = false;
            while let Some((start, end)) = find_jwt_span(&sanitized) {
                jwt_hit = true;
                sanitized.replace_range(start..end, "[REDACTED_JWT]");
            }
            if jwt_hit {
                leaked_kinds.push(LeakedCredentialKind::JwtToken);
            }

            // 7. 通用 Bearer 令牌扫描 (`Bearer <opaque>`). 与 6 同属「声明了类别但没接
            //    检测」的空槽; 6 先收口后本步只会看到 `[REDACTED_JWT]` 标签 (含 `[`,
            //    不在 opaque 字母表内), 两族互不吞并。
            // **脱敏全部匹配**（同上：多枚 Bearer 令牌须逐一带走）。
            let mut bearer_hit = false;
            while let Some((start, end)) = find_bearer_span(&sanitized) {
                bearer_hit = true;
                sanitized.replace_range(start..end, "[REDACTED_BEARER]");
            }
            if bearer_hit {
                leaked_kinds.push(LeakedCredentialKind::GenericBearer);
            }

            // 8. 明文口令赋值扫描 (W1 §2.2 设计点名: `password\s*[:=]\s*\S+`; 手写等价,
            //    不引 regex 依赖)。值截断到行尾 (截断而非放行), key 名保留可读。
            //    匹配用 ASCII 快速比较 —— 不走 to_lowercase 的字节偏移 (中文输出下
            //    大小写折叠可能移位, 切片会错位)。
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
                        // 占位符族豁免: 受控占位符不是明文口令 —— 单向截断会毁掉
                        // 受控回程的还原锚点。替换面只认占位符族, 其余值一律照旧截断。
                        // 排除**本函数自己的脱敏标签**: `password=[REDACTED_PASSWORD]` 再扫一遍时
                        // 标签会被当成长度 >= 4 的明文口令 → 再次截断 → 绊线**不幂等**
                        // (二次扫描仍报 PlaintextPassword)。标签不是凭据, 必须豁免。
                        let is_own_label = value.trim().starts_with("[REDACTED_");
                        if value.len() >= 4
                            && !is_own_label
                            && RedactionPlaceholder::parse(value.trim()).is_none()
                        {
                            leaked_kinds.push(LeakedCredentialKind::PlaintextPassword);
                            sanitized.replace_range(value_start..value_end, "[REDACTED_PASSWORD]");
                            // **脱敏全部匹配**: 此前实现此处 `break`, 只处理第一处口令,
                            // 于是 `password=a\npassword=b` 的第二个值**原样出境**
                            // (2026-10-24 fuzz 发现)。
                            // 不能从 0 重扫: 替换后仍是 `password=[REDACTED_PASSWORD]` 形态,
                            // 重扫会再次命中同一处 → 死循环。故把游标推到替换文本之后。
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

        let is_clean = leaked_kinds.is_empty();
        TripwireScanResult {
            is_clean,
            sanitized_output: sanitized,
            leaked_kinds,
        }
    }

    /// 出境双层扫 (工具输出进入会话历史/出境面之前的脱敏收口).
    ///
    /// 顺序固定: ① 出境占位族脱敏 (真值 → `__AP_<类>_<指纹12>__`, 登记入簿,
    /// 受控回程可还原) ② 既有凭据绊线 (PEM/口令赋值等未被 PII 检测器覆盖的
    /// 凭据形状, 单向标签)。两族共存: 占位族在受控回程还原, 单向标签永远
    /// 保持脱敏。
    pub fn sweep_output_for_egress(
        raw_output: &str,
        bridge: &mut RedactionBridge,
    ) -> EgressSweepResult {
        let redaction = bridge.redact_for_egress(raw_output);
        let tripwire = Self::scan_and_sanitize_output(&redaction.text);
        EgressSweepResult {
            redaction,
            tripwire,
        }
    }
}

/// 一次清扫的真账: 终稿文本 + 两族命中.
///
/// 终稿文本 = 占位族脱敏 + 凭据绊线单向标签 **叠加** 后的那一份
/// （[`EgressSweepResult::tripwire`] 的 `sanitized_output`）; 占位族的中间
/// 文本留在 `redaction.text` 里, 供对照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressSweepOutcome {
    /// 两族命中真账.
    pub report: EgressSweepResult,
}

impl EgressSweepOutcome {
    /// 交给下游的终稿文本（占位族 + 单向标签叠加后）.
    pub fn text(&self) -> &str {
        &self.report.tripwire.sanitized_output
    }

    /// 本次清扫是否有命中（占位族或凭据绊线任一命中）.
    pub fn is_hit(&self) -> bool {
        !self.report.redaction.hits.is_empty() || !self.report.tripwire.leaked_kinds.is_empty()
    }
}

/// 出境清扫器: 一个脱敏域 + 既有凭据绊线的组合体.
///
/// 本类型 **不新增处置语义** —— 一次清扫逐字走
/// [`ToolGuardrail::sweep_output_for_egress`]（占位族脱敏 → 凭据绊线单向
/// 标签, 顺序固定）. 它只是把「脱敏域 (会话随机密钥 + 登记簿 + 账本)」与
/// 「绊线」绑在一起: 接线点交一段文本进来, 拿终稿文本 + 两族真账出去,
/// 不必各自手拼互斥量与账本入口.
///
/// 一个实例 = 一个脱敏域: 同域内同值恒同占位, 跨实例换别名.
#[derive(Debug, Default)]
pub struct EgressSweeper {
    bridge: RedactionBridge,
}

impl EgressSweeper {
    /// 新脱敏域 (会话随机密钥).
    pub fn new() -> Self {
        Self {
            bridge: RedactionBridge::new_session(),
        }
    }

    /// 以显式会话密钥建域（跨进程续接同一脱敏域时由调用方保管密钥）.
    pub fn with_session_key(session_key: [u8; 32]) -> Self {
        Self {
            bridge: RedactionBridge::with_session_key(session_key),
        }
    }

    /// 清扫一段文本: 占位族脱敏 → 凭据绊线（顺序固定, 不新增处置）.
    ///
    /// 干净文本 **逐字原样返回** —— 清扫面对「没有命中」零行为变更.
    pub fn sweep(&mut self, raw: &str) -> EgressSweepOutcome {
        EgressSweepOutcome {
            report: ToolGuardrail::sweep_output_for_egress(raw, &mut self.bridge),
        }
    }

    /// 脱敏账本条目（零真值: 只有类别 / 指纹 / 时间 / 还原与否）.
    pub fn ledger_entries(&self) -> &[LedgerEntry] {
        self.bridge.ledger_entries()
    }

    /// 脱敏账本 JSONL（零真值）.
    pub fn ledger_jsonl(&self) -> String {
        self.bridge.ledger_jsonl()
    }

    /// 登记簿里的真值数（脱敏域大小）.
    pub fn registered_values(&self) -> usize {
        self.bridge.registered_values()
    }
}

// ── 结果面凭据绊线 (共享助手) ───────────────────────────────────────────────
//
// 默认开的三个只读族 (`filesystem` / `search` / `repo`) 的**结果面**共用这一道.
// 两层覆盖面不同, 缺一不可:
//   · **形状层 (M1)**: [`ToolGuardrail::scan_and_sanitize_output`] —— 凭据的
//     **形状** (裸密钥串 / PEM 私钥块 / 明文口令赋值 …). 形状不是「值」的专利:
//     裸串与 PEM 块都不带字段名, 字段值层认不出它们.
//   · **字段值层 (M4)**: `crate::sensitive_path::redact_secret_field_values` ——
//     key / token / secret / password 等**字段名 + 值**形态; 字段值一律换成
//     `[redacted]`, 字段名与非敏感字段逐字节保留.
//
// **层序固定: 形状层 → 字段层** (批次 1 裁决):
//   ① 形状层先在**明文**上判定 ⇒ 命中是真的 (明文口令在明文上命中, 不是在
//      「已遮过的字段值」上误命中);
//   ② 字段层后接管**字段值** ⇒ 含口令赋值的内容, 终稿渲染与接线前**逐字节
//      相同** (可见标签仍是 `[redacted]`, 零可见变化).
//
// ⚠️ **桥接 (两层之间的标签交接)**: 字段层的实现把 `[` 开头的值当**容器值**
//    跳过 ⇒ 形状层的遮形标签 (`[REDACTED_PASSWORD]` 等) 落在字段值位时, 字段层
//    **不接管** ⇒ 可见标签会从 `[redacted]` 漂成形状层标签 (可见变化). 为保住
//    「零可见变化」, 两层之间加一道桥接: 形状层的标签先换成字段层认得的
//    **普通词占位** → 字段层照常接管该字段值 → 字段层**没接管**的占位再还原成
//    形状层标签 ⇒ 字段值归字段层, 裸形状归形状层.
//
// 纪律 (与内核与 shell 结果面同一套):
//   · **零命中** ⇒ 内容逐字节原样返回, **不挂任何侧车** ⇒ 干净答案的形状与
//     字节都不变 (「未被扫动的答案保持原样」);
//   · **命中** ⇒ 内容换成脱敏文本, 并挂上可观测的 `credential_tripwire` 标记
//     (与 shell 同名字段口径) ⇒ 命中不静默.

/// 形状层 (M1) 的遮形标签全集 (内核既有字符串) —— 桥接用.
///
/// 与 [`ToolGuardrail::scan_and_sanitize_output`] 写死的替换标签一一对应;
/// 少列一个 ⇒ 该族落在字段值位时字段层不接管 (可见标签漂移).
const SHAPE_LAYER_LABELS: &[&str] = &[
    "[REDACTED_OPENAI_KEY]",
    "[REDACTED_AWS_KEY]",
    "[REDACTED_GITHUB_PAT]",
    "[REDACTED_PEM_PRIVATE_KEY]",
    "[REDACTED_SLACK_TOKEN]",
    "[REDACTED_JWT]",
    "[REDACTED_BEARER]",
    "[REDACTED_PASSWORD]",
];

/// 桥接占位 (字段层认得的**普通词**形态).
///
/// 约束: 不含 `:` `=` (字段层的分隔符) 与 `,` `}` `]` `#` (字段层的切分标记),
/// 否则字段层对这一行的裁决会被占位本身带偏.
///
/// ⚠️ **另须避开 `APEIRETH_[A-Z0-9_]+` 形态**: 那是本仓 **env 开关命名空间**
/// (`scripts/arch_guard.py` 的 `ENV_RE` 会把它当**新增 env 开关**计数 ⇒ env 预算 161 > 160 ⇒
/// **棘轮会红**) ⇒ ⇒ 桥接占位是**内部标记**、不是 env ⇒ ⇒ 命名不得进入该命名空间.
fn bridge_placeholder(index: usize) -> String {
    format!("__TRIPWIRE_MASK_{index}__")
}

/// 形状层的遮形标签 → 桥接占位 (让字段层能看见这个值).
fn bridge_shape_labels(text: &str) -> String {
    let mut out = text.to_string();
    for (index, label) in SHAPE_LAYER_LABELS.iter().enumerate() {
        if out.contains(label) {
            out = out.replace(label, &bridge_placeholder(index));
        }
    }
    out
}

/// 桥接占位 → 形状层的遮形标签 (字段层**没接管**的那些还原).
fn restore_shape_labels(text: &str) -> String {
    let mut out = text.to_string();
    for (index, label) in SHAPE_LAYER_LABELS.iter().enumerate() {
        let placeholder = bridge_placeholder(index);
        if out.contains(&placeholder) {
            out = out.replace(&placeholder, label);
        }
    }
    out
}

/// 结果面扫描产物: 终稿文本 + 命中族清单.
#[derive(Debug, Clone)]
pub(crate) struct ResultFaceScan {
    /// 终稿文本 (零命中 ⇒ 与输入逐字节相同).
    pub(crate) text: String,
    /// 命中族清单 (零命中 ⇒ 空).
    pub(crate) leaked_kinds: Vec<LeakedCredentialKind>,
}

/// 扫一个**文本结果面**: 形状层 (M1) 过一遍.
///
/// 零命中 ⇒ 文本**逐字节**原样返回且不带任何族 (干净答案的形状与字节都不变).
pub(crate) fn scan_result_face(text: &str) -> ResultFaceScan {
    let scan = ToolGuardrail::scan_and_sanitize_output(text);
    if scan.is_clean {
        // 干净: 原样回传 (逐字节不变), 不带任何族清单。
        ResultFaceScan {
            text: text.to_string(),
            leaked_kinds: Vec::new(),
        }
    } else {
        ResultFaceScan {
            text: scan.sanitized_output,
            leaked_kinds: scan.leaked_kinds,
        }
    }
}

impl ResultFaceScan {
    /// 字段值层 (M4) 后过 —— 层序: 形状层 (M1) → 字段值层 (M4).
    ///
    /// 含字段名 + 值形态的内容, 终稿文本的可见标签与**接线前逐字节相同**
    /// (`[redacted]`), 而命中族仍按**明文**判定 (真命中, 不是对已遮内容的
    /// 误命中)。
    pub(crate) fn with_field_layer(self) -> Self {
        let bridged = bridge_shape_labels(&self.text);
        let redacted = crate::sensitive_path::redact_secret_field_values(&bridged);
        Self {
            text: restore_shape_labels(&redacted),
            leaked_kinds: self.leaked_kinds,
        }
    }
}

/// 命中侧车载荷: 与 shell 结果面同名字段口径 (`triggered` + `leaked_kinds`).
///
/// 内部细节: 只有下面两个「挂侧车」的入口用它 ⇒ 不单独开 crate 内的公共面。
fn credential_tripwire_sidecar(kinds: &[LeakedCredentialKind]) -> serde_json::Value {
    serde_json::json!({
        "triggered": true,
        "leaked_kinds": kinds,
    })
}

/// 把命中侧车挂到**对象**结果面上 (零命中不挂 ⇒ 干净答案形状不变)。
pub(crate) fn attach_credential_tripwire(
    value: &mut serde_json::Value,
    kinds: &[LeakedCredentialKind],
) {
    if kinds.is_empty() {
        return;
    }
    value["credential_tripwire"] = credential_tripwire_sidecar(kinds);
}

/// 把命中侧车挂到**字符串**结果面上: 该形状挂不住结构化侧车 (协议定的裸
/// 文本) ⇒ 侧车以一行 JSON 注解附在文本末尾 (同 MCP 结果面扫的纪律: 固定
/// 形状的面没有结构化侧车位, 命中只能靠改写后的标记显形; 这里再多给一行
/// 可机读的 `credential_tripwire` 载荷 ⇒ 命中不静默)。
///
/// 零命中 ⇒ 文本**逐字节**原样返回 (不挂任何标记)。
pub(crate) fn append_credential_tripwire(text: &str, kinds: &[LeakedCredentialKind]) -> String {
    if kinds.is_empty() {
        return text.to_string();
    }
    let marker = serde_json::json!({
        "credential_tripwire": credential_tripwire_sidecar(kinds),
    })
    .to_string();
    let mut out = String::with_capacity(text.len() + marker.len() + 2);
    out.push_str(text);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&marker);
    out.push('\n');
    out
}

/// 递归扫一个 JSON 值: 逐个**字符串叶子**过形状层扫描。
///
/// 返回 `(终稿值, 命中族清单)`。
///
/// # 为什么不能把整个 JSON `to_string` 后再塞回
///
/// 那样会把**结构**也一起改掉 (键序、空白、数字字面量形态都变) ⇒ 下游按
/// 结构读这个 JSON 的那一侧 (证据信封的 `details` 之类) 就读不到了。
/// ⇒ ⇒ 本助手**只替换字符串叶子的内容**, 其余一律原样保留。
///
/// # 口径 (与批次 1 的结果面助手同源)
///
/// * 只过**形状层** (凭据形状) —— 字段值层是「键 + 分隔符 + 值」的**文本**
///   判据, 一个孤立的 JSON 叶子值没有键上下文 ⇒ 字段层对它是空操作;
/// * **键名不动**、数字/布尔/null 不动、数组与对象的**结构**不动
///   (元素顺序、键集合、嵌套层级都不变);
/// * **零命中 ⇒ 返回值与输入完全相等** (逐字节等价: 零命中时**不重建**这棵树,
///   直接回传输入的克隆 ⇒ 序列化字节一致)。
pub(crate) fn scan_json_string_leaves(
    value: &serde_json::Value,
) -> (serde_json::Value, Vec<LeakedCredentialKind>) {
    let mut leaked_kinds: Vec<LeakedCredentialKind> = Vec::new();
    match scan_json_leaves(value, &mut leaked_kinds) {
        // 有叶子被替换 ⇒ 终稿值是重建后的那棵。
        Some(scanned) => (scanned, leaked_kinds),
        // 零命中 ⇒ **原值克隆** (不重建 ⇒ 键序/数字/布尔/null 逐字节不变)。
        None => (value.clone(), leaked_kinds),
    }
}

/// 递归扫 JSON 的**字符串叶子** (形状层)。
///
/// 返回 `Some(新值)` = 这一支有叶子被替换; `None` = 这一支**原样**
/// (调用方据此保留原值 ⇒ 零命中的整棵树不被重建 ⇒ 逐字节等价)。
fn scan_json_leaves(
    value: &serde_json::Value,
    leaked_kinds: &mut Vec<LeakedCredentialKind>,
) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::String(text) => {
            let scan = scan_result_face(text);
            if scan.leaked_kinds.is_empty() {
                None
            } else {
                leaked_kinds.extend(scan.leaked_kinds.iter().copied());
                Some(serde_json::Value::String(scan.text))
            }
        }
        serde_json::Value::Array(items) => {
            let mut changed = false;
            let mut scanned = Vec::with_capacity(items.len());
            for item in items {
                match scan_json_leaves(item, leaked_kinds) {
                    Some(replaced) => {
                        changed = true;
                        scanned.push(replaced);
                    }
                    None => scanned.push(item.clone()),
                }
            }
            if changed {
                Some(serde_json::Value::Array(scanned))
            } else {
                None
            }
        }
        serde_json::Value::Object(map) => {
            let mut changed = false;
            let mut scanned = serde_json::Map::new();
            for (key, item) in map {
                match scan_json_leaves(item, leaked_kinds) {
                    Some(replaced) => {
                        changed = true;
                        scanned.insert(key.clone(), replaced);
                    }
                    None => {
                        scanned.insert(key.clone(), item.clone());
                    }
                }
            }
            if changed {
                Some(serde_json::Value::Object(scanned))
            } else {
                None
            }
        }
        // 数字 / 布尔 / null: 不是文本面 ⇒ 原样。
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    /// GitHub PAT 族（`ghp_`）此前**整族失能** ⇒ 真实形态明文出境。
    ///
    /// 根因：分隔符判据用「非字母数字」，而本族**前缀 `ghp_` 的 `_` 本身**落在里面
    /// ⇒ 取 token 时**在 `_` 处截断** ⇒ token 恒为 `"ghp"` ⇒ `len >= 36` 恒假
    /// ⇒ 既不脱敏、**也不报泄露族**（绊线不响）。
    #[test]
    fn github_pat_family_is_actually_effective() {
        // 真实形态：ghp_ + 36 = 40 字符
        let real = format!("ghp_{}", "A".repeat(36));
        let res = ToolGuardrail::scan_and_sanitize_output(&real);
        assert!(
            !res.sanitized_output.contains(&real),
            "真实形态的 GitHub PAT 必须被脱敏: {}",
            res.sanitized_output
        );
        assert!(
            res.leaked_kinds.contains(&LeakedCredentialKind::GitHubPat),
            "必须报出 GitHub PAT 泄露: {:?}",
            res.leaked_kinds
        );
        // 前缀与主体之间不该被截断（这正是那个洞）。
        let embedded = format!("前段 {real} 中段 {real} 尾段");
        let res2 = ToolGuardrail::scan_and_sanitize_output(&embedded);
        assert!(
            !res2.sanitized_output.contains(&real),
            "两枚凭证都必须脱敏: {}",
            res2.sanitized_output
        );
        // 太短的**不得**被当成凭证（不误伤）：主体 20 位 ⇒ 总长 24 < 36
        let short = format!("ghp_{}", "A".repeat(20));
        let res3 = ToolGuardrail::scan_and_sanitize_output(&short);
        assert_eq!(res3.sanitized_output, short, "太短的不得被误伤");
    }

    /// PEM 私钥块的**块首与块尾必须锚在同一个块上**。
    ///
    /// 回归：前置块（CERTIFICATE / PUBLIC KEY）会让**私钥块的 BEGIN 行**误落进
    /// 被替换的区间 ⇒ **base64 私钥体原样留在输出里 ⇒ 明文出境**。
    #[test]
    fn pem_body_is_redacted_even_when_another_block_precedes_it() {
        for prefix in [
            "-----BEGIN CERTIFICATE-----\nMIIEB\n-----END CERTIFICATE-----\n",
            "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----\n",
        ] {
            let raw = format!(
                "{prefix}-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0\n-----END RSA PRIVATE KEY-----"
            );
            let res = ToolGuardrail::scan_and_sanitize_output(&raw);
            assert!(
                !res.sanitized_output.contains("MIIEowIBAAKCAQEA0"),
                "私钥体留在输出里: {}",
                res.sanitized_output
            );
            assert!(
                res.leaked_kinds
                    .contains(&LeakedCredentialKind::PemPrivateKey),
                "必须报出私钥泄露: {:?}",
                res.leaked_kinds
            );
            assert!(
                !res.sanitized_output.contains("PRIVATE KEY-----"),
                "脱敏后的输出不得再含 `PRIVATE KEY-----`: {}",
                res.sanitized_output
            );
            // 前置块**不该被误伤**（它不是私钥 ⇒ 保留原文）。
            assert!(
                res.sanitized_output.contains("-----BEGIN CERTIFICATE-----")
                    || res.sanitized_output.contains("-----BEGIN PUBLIC KEY-----"),
                "前置非私钥块不应被替换: {}",
                res.sanitized_output
            );
        }
    }
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pre_call_path_traversal() {
        let ws = PathBuf::from("c:/workspace/test");
        let bad = PathBuf::from("../etc/passwd");
        assert!(ToolGuardrail::verify_path_access(&ws, &bad).is_err());

        let forbidden = PathBuf::from("/etc/shadow");
        assert!(ToolGuardrail::verify_path_access(&ws, &forbidden).is_err());

        let safe = PathBuf::from("src/lib.rs");
        assert!(ToolGuardrail::verify_path_access(&ws, &safe).is_ok());
    }

    #[test]
    fn test_pre_call_dangerous_commands() {
        assert!(ToolGuardrail::verify_shell_command("rm -rf / --no-preserve-root").is_err());
        assert!(ToolGuardrail::verify_shell_command("dd if=/dev/zero of=/dev/sda").is_err());
        assert!(ToolGuardrail::verify_shell_command("cargo test --workspace").is_ok());
    }

    /// 多字节字符不得让绊线 panic (2026-10-24 fuzz 发现的真实崩溃)。
    ///
    /// `find_bearer_span` 的回看窗口起点是 `kw_start - 64`, 该字节偏移可能落在
    /// 多字节字符**中间** → `&text[window_start..kw_start]` 触发
    /// `byte index ... is not a char boundary` panic。
    #[test]
    fn multibyte_text_never_panics_the_tripwire() {
        // 构造: 前面若干三字节字符, 使其后 64 字节偏移正好切在字符中间
        let cases = [
            format!(
                "{}Authorization: Bearer {}",
                "字".repeat(21),
                "a".repeat(20)
            ),
            format!("{}Bearer {}", "汉".repeat(22), "b".repeat(24)),
            format!("{}bearer {}", "🙂".repeat(17), "c".repeat(18)),
            "字字字Bearer ddddddddddddddddddddd".to_string(),
        ];
        for raw in cases {
            // 不 panic 即通过; 是否命中凭据由形状决定, 这里不强制。
            let res = ToolGuardrail::scan_and_sanitize_output(&raw);
            let _ = ToolGuardrail::scan_and_sanitize_output(&res.sanitized_output);
        }
    }

    /// **同类凭据出现多处时必须全部脱敏** (2026-10-24 fuzz 发现的真实泄密路径)。
    ///
    /// 修复前每族只处理**第一处**匹配 —— 一份工具输出里若有两个同类凭证,
    /// 第二个**原样出境**。绊线的职责是「不让凭据出境」, 只脱敏一处等于没有这道门。
    #[test]
    fn multiple_credentials_of_same_kind_are_all_redacted() {
        let cases: [(&str, &str, &str); 5] = [
            (
                "两个 Slack token",
                "xoxb-1234567890abcdefghij and xoxb-9876543210zyxwvutsrq",
                "xoxb-",
            ),
            (
                "两个 OpenAI key",
                "sk-proj-aaaaaaaaaaaaaaaaaaaa and sk-proj-bbbbbbbbbbbbbbbbbbbb",
                "sk-proj-",
            ),
            (
                "两个 AWS key",
                "AKIAIOSFODNN7EXAMPLE12 and AKIAIOSFODNN7EXAMPLE34",
                "AKIA",
            ),
            (
                "两个 PEM 私钥块",
                "-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----\n\
                 -----BEGIN RSA PRIVATE KEY-----\nBBBB\n-----END RSA PRIVATE KEY-----",
                "AAAA",
            ),
            (
                "两处明文口令赋值",
                "password=firstpass\npassword=secondpass",
                "firstpass",
            ),
        ];
        for (label, raw, secret_fragment) in cases {
            let res = ToolGuardrail::scan_and_sanitize_output(raw);
            assert!(
                !res.sanitized_output.contains(secret_fragment),
                "{label}: 第二处凭据仍明文留在输出里: {:?}",
                res.sanitized_output
            );
            // 绊线必须**幂等**: 脱敏过的输出再扫一遍应当干净。
            let again = ToolGuardrail::scan_and_sanitize_output(&res.sanitized_output);
            assert!(
                again.is_clean,
                "{label}: 绊线不幂等, 二次扫描仍检出 {:?}: {:?}",
                again.leaked_kinds, res.sanitized_output
            );
        }
    }

    /// 空白不得成为绕过通道 (2026-10-24 fuzz 发现的真实绕过)。
    ///
    /// shell 的参数分隔符可以是任意空白; 修复前守门按原样匹配单空格书写的禁词,
    /// 于是 `netsh<TAB>set ...` 这条**合法命令**拦不住。下列三例修复前均放行。
    #[test]
    fn whitespace_variants_cannot_bypass_command_guard() {
        // 高危不可逆命令: 多空白形态
        for cmd in [
            "rm  -rf /",
            "rm\t-rf /",
            "rm\n-rf /",
            "dd  if=/dev/zero of=/dev/sda",
            "dd\tif=/dev/zero of=/dev/sda",
            "format  c:",
        ] {
            assert!(
                ToolGuardrail::verify_shell_command(cmd).is_err(),
                "多空白形态必须仍被拦: {cmd:?}"
            );
        }
        // netsh 写入类: 动词前的分隔符为 TAB / 换行 / 多空格
        for cmd in [
            "netsh set advfirewall x",
            "netsh  set advfirewall x",
            "netsh\tset advfirewall x",
            "netsh\nset advfirewall x",
            "netsh\t\tadd rule x",
            "netsh\nreset logfile",
        ] {
            assert!(
                ToolGuardrail::verify_shell_command(cmd).is_err(),
                "netsh 写入类多空白形态必须仍被拦: {cmd:?}"
            );
        }
        // 只读形态仍放行 (否定式保守不得扩大成误伤)
        assert!(ToolGuardrail::verify_shell_command("netsh show rule name=all").is_ok());
        assert!(ToolGuardrail::verify_shell_command("netsh  display  config").is_ok());
    }

    #[test]
    fn test_post_call_tripwire_sanitization() {
        let raw = "Server connected with key sk-proj-1234567890abcdef1234567890 and AKIAIOSFODNN7EXAMPLE12.";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::OpenAiKey));
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::AwsAccessKey));
        assert!(res.sanitized_output.contains("[REDACTED_OPENAI_KEY]"));
        assert!(res.sanitized_output.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn tripwire_redacts_plaintext_password_assignment() {
        // W1 §2.2 设计点名模式; 含中文上下文 (回归 to_lowercase 偏移隐患)。
        let raw = "数据库登录 ok\npassword = hunter2secret\n下一行完好";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::PlaintextPassword));
        assert!(!res.sanitized_output.contains("hunter2secret"), "值已截断");
        assert!(res.sanitized_output.contains("[REDACTED_PASSWORD]"));
        assert!(
            res.sanitized_output.contains("下一行完好"),
            "只截命中值到行尾"
        );
    }

    #[test]
    fn password_keyword_without_assignment_passes() {
        let res = ToolGuardrail::scan_and_sanitize_output("the password field is empty here");
        assert!(res.is_clean, "{res:?}");
    }

    #[test]
    fn system_modification_commands_are_guarded() {
        // W1 §2.3 扩展: 注册表/服务/网络配置/属性隐藏类系统修改命令。
        for cmd in [
            "reg add HKLM\\Software\\x /v y",
            "sc create evil binPath= x",
            "netsh advfirewall set allprofiles off",
            "attrib +s +h secret.txt",
        ] {
            assert!(
                ToolGuardrail::verify_shell_command(cmd).is_err(),
                "{cmd} 应被守门拦截"
            );
        }
        assert!(ToolGuardrail::verify_shell_command("netsh show all").is_ok());
        // 否定式保守语义: 含子串即拦 (纵深防线接受误报, 守门不是沙箱的替代)。
        assert!(ToolGuardrail::verify_shell_command("echo reg add is a phrase").is_err());
    }

    #[test]
    fn test_post_call_tripwire_pem_key() {
        let raw = "Output: -----BEGIN RSA PRIVATE KEY----- MIIEowIBAAKCAQEA0... -----END RSA PRIVATE KEY-----";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::PemPrivateKey));
        assert!(res.sanitized_output.contains("[REDACTED_PEM_PRIVATE_KEY]"));
    }

    /// PEM 整块收口: base64 私钥体不得留在输出 (只换头部 = 泄漏债, 已清)。
    #[test]
    fn pem_block_redaction_covers_key_body_not_just_header() {
        let raw = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0\n-----END RSA PRIVATE KEY-----\ntrailing";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(
            !res.sanitized_output.contains("MIIEowIBAAKCAQEA0"),
            "{}",
            res.sanitized_output
        );
        assert!(
            res.sanitized_output.contains("trailing"),
            "{}",
            res.sanitized_output
        );
        assert!(res.sanitized_output.contains("[REDACTED_PEM_PRIVATE_KEY]"));
    }

    /// 占位符族豁免: 受控占位符不是明文口令 (还原锚点不可被单向截断毁掉),
    /// 真口令照旧截断 —— 两头都钉。
    #[test]
    fn password_scan_keeps_placeholder_family_and_still_redacts_real_passwords() {
        let token = "__AP_ENV_SECRET_a1b2c3d4e5f6__";
        let res = ToolGuardrail::scan_and_sanitize_output(&format!("password = {token}\nok"));
        assert!(res.is_clean, "{res:?}");
        assert!(
            res.sanitized_output.contains(token),
            "{}",
            res.sanitized_output
        );

        let res2 = ToolGuardrail::scan_and_sanitize_output("password = realsecret\nok");
        assert!(!res2.is_clean);
        assert!(res2.sanitized_output.contains("[REDACTED_PASSWORD]"));
        assert!(!res2.sanitized_output.contains("realsecret"));
    }

    /// D18 收口: `JwtToken` 变体此前只有声明、零检测面 (命名即承诺的空槽)。
    #[test]
    fn jwt_token_is_detected_and_redacted() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                   eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ.\
                   SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let raw = format!("上游返回: Authorization: Bearer {jwt} (之后正常)");
        let res = ToolGuardrail::scan_and_sanitize_output(&raw);
        assert!(!res.is_clean);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(res.sanitized_output.contains("[REDACTED_JWT]"));
        assert!(!res.sanitized_output.contains("eyJ"));
        assert!(
            res.sanitized_output.contains("之后正常"),
            "非命中文段原样保留: {}",
            res.sanitized_output
        );
    }

    /// 形状不完整 (两段 / 签名过短) 或只是长 base64 文本 —— 不当 JWT。
    #[test]
    fn jwt_shape_requires_three_segments_and_long_enough_signature() {
        for raw in [
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0", // 只有两段
            "eyJhbGciOiJIUzI1NiJ9.abc.def",         // 段长不足
            "prefix eyJhbGciOiJIUzI1NiJ9 suffix",   // 单词形
            "no eyJ marker here",                   // 无前缀
        ] {
            let res = ToolGuardrail::scan_and_sanitize_output(raw);
            assert!(res.is_clean, "{raw} 不该被当 JWT: {res:?}");
        }
    }

    /// 未签名 `h.p.` 形态 (alg=none, 尾段为空) 也算 JWT 形状。
    #[test]
    fn unsigned_jwt_with_trailing_dot_is_detected() {
        let raw = "token=eyJhbGciOiJub25lIn0.eyJzdWIiOiJ4In0.";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(res.sanitized_output.contains("[REDACTED_JWT]"));
    }

    /// `GenericBearer` 同为空槽, 本批补齐: 只认 Authorization 头部上下文。
    #[test]
    fn authorization_bearer_token_is_detected_and_redacted() {
        let raw = "request headers: Authorization: Bearer opaqueTokenValue1234567890abc end";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::GenericBearer));
        assert!(res.sanitized_output.contains("[REDACTED_BEARER]"));
        assert!(!res
            .sanitized_output
            .contains("opaqueTokenValue1234567890abc"));
        assert!(res.sanitized_output.contains("end"));
    }

    /// 散文里的 "Bearer" / 无 Authorization 上下文 / 短值 —— 一律不误伤。
    #[test]
    fn bearer_word_without_header_context_is_not_flagged() {
        for raw in [
            "The bearer configuration mechanism is documented elsewhere.",
            "Bearer opaqueTokenValue1234567890abc",
            "Authorization: Bearer short",
            "authorization header explained",
        ] {
            let res = ToolGuardrail::scan_and_sanitize_output(raw);
            assert!(res.is_clean, "{raw} 不该被当 Bearer: {res:?}");
        }
    }

    /// 两族互不吞并: JWT 先收口, Bearer 步只看到 `[REDACTED_JWT]` 标签 (含 `[`
    /// 不在 opaque 字母表), 不重复计类。
    #[test]
    fn jwt_and_bearer_kinds_do_not_double_flag() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                   eyJzdWIiOiIxMjM0NTY3ODkwIn0.\
                   SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let raw = format!("Authorization: Bearer {jwt}");
        let res = ToolGuardrail::scan_and_sanitize_output(&raw);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(
            !res.leaked_kinds
                .contains(&LeakedCredentialKind::GenericBearer),
            "{res:?}"
        );
        assert!(!res.sanitized_output.contains("[REDACTED_BEARER]"));
    }

    // ── 结果面两层 (形状层 → 字段层) 的层序 ─────────────────────────────────

    /// 层序 (形状层 → 字段层): 含口令赋值的文本, 终稿标签与**接线前逐字节相同**
    /// (`[redacted]`), 而命中族按**明文**判定 (真命中, 不是对已遮内容的误命中).
    #[test]
    fn field_layer_runs_after_the_shape_layer_and_keeps_the_pre_wiring_label() {
        let scan =
            super::scan_result_face("password = hunter2-not-a-real-secret\n").with_field_layer();
        assert_eq!(
            scan.text, "password = [redacted]\n",
            "字段层须接管口令字段值 (可见标签与接线前一致)"
        );
        assert!(
            scan.leaked_kinds
                .contains(&LeakedCredentialKind::PlaintextPassword),
            "口令族须在明文上真命中: {:?}",
            scan.leaked_kinds
        );
    }

    /// 裸形状 (不在字段值位) 归形状层: 标签保持形状层的, 字段层不插手.
    #[test]
    fn bare_shapes_keep_the_shape_layer_label() {
        let scan = super::scan_result_face("plain line\nsk-0123456789abcdefghijklmnop\n");
        assert!(
            scan.text.contains("[REDACTED_OPENAI_KEY]"),
            "裸形状归形状层: {:?}",
            scan.text
        );
        assert!(scan.leaked_kinds.contains(&LeakedCredentialKind::OpenAiKey));
    }

    /// 字段值位里的形状: 字段层接管 (可见标签与接线前一致), 命中族仍按明文判定
    /// ⇒ 两层都出力, 谁都不漏.
    #[test]
    fn a_shape_inside_a_field_value_is_owned_by_the_field_layer() {
        let scan = super::scan_result_face("api_key = \"sk-0123456789abcdefghijklmnop\"\n")
            .with_field_layer();
        assert_eq!(
            scan.text, "api_key = \"[redacted]\"\n",
            "字段值位归字段层 (可见标签与接线前一致)"
        );
        assert!(scan.leaked_kinds.contains(&LeakedCredentialKind::OpenAiKey));
    }

    /// 零命中 ⇒ 逐字节不变, 且不带任何族 (干净答案的形状与字节都不变).
    #[test]
    fn a_clean_face_is_byte_identical() {
        let scan = super::scan_result_face("plain content line\n").with_field_layer();
        assert_eq!(scan.text, "plain content line\n");
        assert!(scan.leaked_kinds.is_empty());
    }

    /// 侧车口径: 零命中不挂; 命中才挂, 且载荷与 shell 结果面同名字段口径.
    #[test]
    fn the_sidecar_only_shows_up_on_a_hit() {
        let clean = super::scan_result_face("plain content line\n");
        let mut value = serde_json::json!({ "path": "." });
        super::attach_credential_tripwire(&mut value, &clean.leaked_kinds);
        assert!(
            value.get("credential_tripwire").is_none(),
            "零命中不得挂侧车: {value}"
        );
        assert_eq!(
            super::append_credential_tripwire(&clean.text, &clean.leaked_kinds),
            "plain content line\n"
        );

        let hit = super::scan_result_face("password = hunter2-not-a-real-secret\n");
        let mut value = serde_json::json!({ "path": "." });
        super::attach_credential_tripwire(&mut value, &hit.leaked_kinds);
        let sidecar = value.get("credential_tripwire").expect("命中必须挂侧车");
        assert_eq!(sidecar["triggered"], serde_json::json!(true));
        assert!(sidecar["leaked_kinds"]
            .as_array()
            .is_some_and(|kinds| kinds.contains(&serde_json::json!("PlaintextPassword"))));
        assert!(
            super::append_credential_tripwire(&hit.text, &hit.leaked_kinds)
                .contains("credential_tripwire")
        );
    }

    // ── JSON 字符串叶子扫描 (批次 2 的共享助手) ──────────────────────────────

    /// 形状族夹具: 一枚**合成**的裸密钥串 (非真实凭据; 长度 ≥ 20 ⇒ 命中形状族)。
    const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

    /// 断言两棵 JSON 树**结构相同**: 节点的种类一致、对象的**键集合**一致、
    /// 数组的长度一致 (递归) ⇒ 「只有字符串叶子的**内容**被换」这件事本身
    /// 可被独立验证 (不依赖逐字段肉眼比对)。
    fn assert_same_shape(left: &serde_json::Value, right: &serde_json::Value) {
        match (left, right) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                let a_keys: std::collections::BTreeSet<&String> = a.keys().collect();
                let b_keys: std::collections::BTreeSet<&String> = b.keys().collect();
                assert_eq!(a_keys, b_keys, "对象的键集合必须不变");
                for (key, value) in a {
                    assert_same_shape(value, &b[key]);
                }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "数组长度必须不变");
                for (x, y) in a.iter().zip(b.iter()) {
                    assert_same_shape(x, y);
                }
            }
            (serde_json::Value::Array(_), _) | (_, serde_json::Value::Array(_)) => {
                panic!("数组不得与其它种类互换")
            }
            (serde_json::Value::Object(_), _) | (_, serde_json::Value::Object(_)) => {
                panic!("对象不得与其它种类互换")
            }
            (serde_json::Value::Null, serde_json::Value::Null) => {}
            (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => assert_eq!(a, b),
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => assert_eq!(a, b),
            (serde_json::Value::String(_), serde_json::Value::String(_)) => {}
            _ => panic!("叶子的种类不得互换"),
        }
    }

    /// JSON 叶子扫描: **只换字符串叶子的内容** —— 键名、数字、布尔、null、
    /// 数组与对象的结构一律不动。
    #[test]
    fn json_leaf_scan_replaces_only_string_leaves() {
        let fixture = serde_json::json!({
            "plain": "clean text",
            "secret": FAKE_KEY,
            "count": 42,
            "ratio": 1.5,
            "flag": true,
            "nothing": null,
            "nested": {
                "inner": FAKE_KEY,
                "deeper": {
                    "leaf": ["clean", FAKE_KEY, 7],
                    "kept_number": 100,
                },
            },
            "list": [
                { "k": FAKE_KEY },
                { "k": "clean" },
            ],
        });

        let (scanned, kinds) = super::scan_json_string_leaves(&fixture);

        // 形状层命中族如实报出。
        assert!(
            kinds.contains(&LeakedCredentialKind::OpenAiKey),
            "形状层必须在明文上真命中: {kinds:?}"
        );

        // 只有字符串叶子被换。
        assert_eq!(scanned["plain"], serde_json::json!("clean text"));
        assert_eq!(
            scanned["secret"],
            serde_json::json!("[REDACTED_OPENAI_KEY]")
        );
        assert_eq!(
            scanned["nested"]["inner"],
            serde_json::json!("[REDACTED_OPENAI_KEY]")
        );
        assert_eq!(
            scanned["nested"]["deeper"]["leaf"][1],
            serde_json::json!("[REDACTED_OPENAI_KEY]")
        );
        assert_eq!(
            scanned["nested"]["deeper"]["leaf"][0],
            serde_json::json!("clean")
        );
        assert_eq!(
            scanned["list"][0]["k"],
            serde_json::json!("[REDACTED_OPENAI_KEY]")
        );
        assert_eq!(scanned["list"][1]["k"], serde_json::json!("clean"));

        // 数字 / 布尔 / null 一律不动。
        assert_eq!(scanned["count"], serde_json::json!(42));
        assert_eq!(scanned["ratio"], serde_json::json!(1.5));
        assert_eq!(scanned["flag"], serde_json::json!(true));
        assert_eq!(scanned["nothing"], serde_json::Value::Null);
        assert_eq!(scanned["nested"]["deeper"]["leaf"][2], serde_json::json!(7));
        assert_eq!(
            scanned["nested"]["deeper"]["kept_number"],
            serde_json::json!(100)
        );

        // 键名与结构不动。
        assert_same_shape(&fixture, &scanned);
        assert!(fixture["secret"].is_string(), "输入侧不得被就地改写");
        assert_eq!(fixture["secret"], serde_json::json!(FAKE_KEY));
    }

    /// 零命中 ⇒ 返回值与输入**完全相等** (逐字节等价: 序列化字节一致)。
    #[test]
    fn json_leaf_scan_is_byte_identical_when_clean() {
        let clean = serde_json::json!({
            "plain": "clean text",
            "count": 42,
            "ratio": 1.5,
            "flag": false,
            "nothing": null,
            "nested": { "inner": ["a", "b", 3] },
            "list": [{ "k": "v" }, { "k": "w" }],
        });

        let (scanned, kinds) = super::scan_json_string_leaves(&clean);

        assert!(kinds.is_empty(), "零命中不得报族: {kinds:?}");
        assert_eq!(scanned, clean, "零命中时返回值必须与输入完全相等");
        assert_eq!(
            serde_json::to_string(&scanned).unwrap(),
            serde_json::to_string(&clean).unwrap(),
            "零命中时序列化字节必须一致"
        );
    }
}
