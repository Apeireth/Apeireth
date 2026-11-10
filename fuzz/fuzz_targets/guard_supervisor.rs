#![no_main]

//! fuzz: 三个「闸」面 —— 前置护栏 / 读前观测闸 / 子进程监管。
//!
//! 三个面都是**闸**：一洞 = 「该拒的放过了」。本 target 的判据**全部**取自被测
//! 模块自己写明的契约（出处逐条标在断言旁的 `src:` 注释里），不发明期望。
//!
//! ## 面 ① 护栏 (`crates/capabilities/tools/src/guardrail.rs`)
//!
//! - **前置路径闸** `ToolGuardrail::verify_path_access`
//!   （`src: guardrail.rs:177-229`）：
//!   - "1. 拦截明显的路径穿越特征"：`path_str.contains("../") ||
//!     path_str.contains("..\\") || path_str == ".."` ⇒ `PathTraversal`
//!     （`src: guardrail.rs:184-190`）
//!   - "2. 拦截绝对敏感路径"：`forbidden_prefixes` 前缀命中 ⇒ `PathTraversal`
//!     （`src: guardrail.rs:192-212`）
//!   - "3. 若为绝对路径，必须位于 workspace_root 范围内"：越界 ⇒ `OutsideWorkspace`
//!     （`src: guardrail.rs:214-226`）
//!   ⇒ 断言全是**单向 fail-closed**：命中禁止形态 ⇒ 必须拒。
//!
//! - **后置凭据绊线** `ToolGuardrail::scan_and_sanitize_output`
//!   （`src: guardrail.rs:365-571`）：
//!   - 硬不变量："保证**脱敏后的输出不再含 `PRIVATE KEY-----`**"（`src: guardrail.rs:449-457`）
//!   - PEM 整块收口："只换头部会把 base64 私钥体留在输出里 —— 确证级泄漏债,
//!     整块收口"（`src: guardrail.rs:420-424`）⇒ **私钥体不得留在输出里**。
//!   - 同族多枚全带走："反复脱敏同族**全部**匹配, 而不只是第一处"（`src: guardrail.rs:316-327`
//!     `redact_all`）⇒ 两枚同族凭证的载荷都不得留存。
//!   - 占位符族豁免："占位符族豁免: 受控占位符不是明文口令 —— 单向截断会毁掉
//!     受控回程的还原锚点"（`src: guardrail.rs:528-537`）⇒ `password=<占位符>`
//!     的占位符必须存活。
//!
//! - **出境双扫** `ToolGuardrail::sweep_output_for_egress`（`src: guardrail.rs:573-589`）：
//!   "顺序固定: ① 出境占位族脱敏 (真值 → `__AP_<类>_<指纹12>__`, 登记入簿,
//!   受控回程可还原) ② 既有凭据绊线" ⇒ 真值不得出境 + 占位锚点必须存活。
//!
//! ## 面 ② 观测闸 (`crates/capabilities/tools/src/observed_gate.rs`)
//!
//! `ObservedGate::check_write` 的判定是纯函数（`src: observed_gate.rs:269-272`
//! "全部判定方法是纯函数 (内存映射查询), 不做任何文件系统 IO"），契约逐条写在
//! 方法文档里（`src: observed_gate.rs:333-345`）：
//! - 覆盖类：未观测 ⇒ `NotObserved`；观测为不存在 ⇒ `ObservedMissing`；
//!   携带 `replace_if_version` 时必须等于观测版本（钥匙陈旧/伪造即拒），且写入端
//!   探查的现状必须等于观测版本（不等 = 外部改动）⇒ `VersionMismatch`。
//! - 创建类：`create_if_absent` 钥匙 + 目标已出现 ⇒ `TargetAppeared`；无「已观测为
//!   不存在」证据且目标已存在 ⇒ `CreateTargetExists`。
//! ⇒ 断言按**文档规则**逐条重算（oracle = 文档，不是实现的复制），并额外钉住
//! 「过期观测（现状 ≠ 观测版本）必须拒」这条 fail-closed 底线。
//!
//! ## 面 ③ 子进程监管 (`crates/capabilities/tools/src/std_sub_supervisor.rs`)
//!
//! ⚠️ 本面**只测纯判定层**（`on_child_exit` 的策略决策 + 重启限额 +
//! 未知 child 拒绝）。`start` / `stop` 会**真的起进程 / 杀进程**（`src:
//! std_sub_supervisor.rs:74-88` / `165-188`），本 target **不覆盖**（见交付报告的
//! 「验证边界清单」）。
//!
//! - 策略决策（`src: std_sub_supervisor.rs:213-253`）：`OneForOne` / `RestForOne`
//!   "任何 exit 都重启"；`Transient` "正常退出 (Normal) 不重启; 异常才重启"。
//! - 重启限额（`src: std_sub_supervisor.rs:56-57` "v1 默认: 5 次 / 60 秒" +
//!   `src: std_sub_supervisor.rs:116-125`）：窗口内第 6 次 ⇒ `RestartLimitExceeded`。
//! - 未知 child ⇒ `UnknownChild`（`src: std_sub_supervisor.rs:201-206`）。
//!
//! ## 刻意不造的期望（自造期望 = 假阳性；本项目历史已栽 9 次）
//!
//! 1. **`ExitReason::Normal { code: 非 0 }`**：生产上不可达（`classify_exit` 只在
//!    `code == 0` 时给 `Normal`，非零一律 `Abnormal`，`src: std_sub_supervisor.rs:99-103`），
//!    且「Transient 按变体判（不重启）还是按 code 判（重启）」两种读法在文档里
//!    都说得通 ⇒ 本 target **不作断言**（只跳过这一格）。
//! 2. **观测闸「已观测为不存在 + 目标已出现 + 无 `create_if_absent` 钥匙」**：
//!    文档规则 3 明写"无「已观测为不存在」证据**且**目标已存在 ⇒ `CreateTargetExists`"
//!    （`src: observed_gate.rs:345`）—— 有「已观测为不存在」证据时**不在**拒的理由里
//!    ⇒ 属**契约明示的放行口径**，本 target 按文档断言放行，不当成缺陷。
//! 3. **`verify_path_access` 对「不存在的工作区外绝对路径」放行**：`canonicalize`
//!    失败时规则 3 整段跳过（`src: guardrail.rs:215-226` 的 `if let (Ok, Ok)`）。
//!    文档只承诺"若为绝对路径，必须位于 workspace_root 范围内"，**没有**承诺
//!    「不存在也必须拒」⇒ 不作断言（记入交付报告的边界清单）。
//! 4. **守卫闸的模糊匹配口径**：`find_jwt_span` / `find_bearer_span` 的形状判据
//!    （`src: guardrail.rs:82-170`）自带"保守: 宁可漏判也不误伤"的取舍 ⇒ 只钉
//!    「命中了必须带走」与「脱敏后不得残留」，不钉"某形状必须命中"。

use std::path::Path;
use std::sync::Once;
use std::time::{Duration, UNIX_EPOCH};

use apeireth_governance::input_security::PiiKind;
use apeireth_tools_canonical::guardrail::{LeakedCredentialKind, PreCallGuardError, ToolGuardrail};
use apeireth_tools_canonical::observed_gate::{
    ExclusionList, FileVersion, GateDenial, ObservationState, ObservedGate, WriteIntent, WriteKind,
};
use apeireth_tools_canonical::redaction_bridge::RedactionBridge;
use apeireth_tools_canonical::std_sub_supervisor::StdSubSupervisor;
use apeireth_tools_canonical::supervisor::{
    ChildSpec, ExitReason, RestartDecision, RestartStrategy, SubSupervisor, SubSupervisorKind,
    SupervisorError,
};
use libfuzzer_sys::fuzz_target;

/// 护栏的敏感路径前缀表（逐字取自 `src: guardrail.rs:194-203`）。
const FORBIDDEN_PREFIXES: [&str; 8] = [
    "/etc/shadow",
    "/etc/passwd",
    "/etc/sudoers",
    "/root",
    "/var/run",
    "/dev",
    "c:\\windows\\system32",
    "c:\\windows\\system",
];

/// 凭据族：前缀 + 接受长度区间（逐字取自 `src: guardrail.rs:387-479` 的
/// `redact_all` 调用点：接受条件都是 **token 全长**）。
struct CredentialFamily {
    name: &'static str,
    prefix: &'static str,
    min_len: usize,
    max_len: Option<usize>,
}

const FAMILIES: [CredentialFamily; 4] = [
    CredentialFamily {
        name: "OpenAI",
        prefix: "sk-",
        min_len: 20,
        max_len: None,
    },
    CredentialFamily {
        name: "AWS",
        prefix: "AKIA",
        min_len: 16,
        max_len: Some(32),
    },
    CredentialFamily {
        name: "GitHub",
        prefix: "ghp_",
        min_len: 36,
        max_len: None,
    },
    CredentialFamily {
        name: "Slack",
        prefix: "xoxb-",
        min_len: 20,
        max_len: None,
    },
];

// ===========================================================================
// 通用小工具
// ===========================================================================

/// 把任意字节映射成一个 ASCII 字母数字（**非空白 / 非引号** ⇒ 各族的 token
/// 定界符不会把载荷截断）。
fn alnum_char(byte: u8) -> char {
    let idx = (byte as usize) % 62;
    if idx < 26 {
        (b'A' + idx as u8) as char
    } else if idx < 52 {
        (b'a' + (idx - 26) as u8) as char
    } else {
        (b'0' + (idx - 52) as u8) as char
    }
}

/// 运行时构造的字母数字载荷（长度上限 `max_len`）。
fn alnum_payload(bytes: &[u8], max_len: usize) -> String {
    bytes.iter().take(max_len).map(|b| alnum_char(*b)).collect()
}

/// 构造版本指纹。
fn version(len: u64, tag: u64) -> FileVersion {
    FileVersion {
        len,
        mtime: UNIX_EPOCH + Duration::from_secs(tag),
    }
}

// ===========================================================================
// 面 ① 护栏
// ===========================================================================

/// 前置路径闸（`src: guardrail.rs:177-229`）。
fn mode_path_gate(payload: &[u8]) {
    let text = String::from_utf8_lossy(payload);
    let requested = Path::new(text.as_ref());
    // 工作区根取一个**不存在**的路径 ⇒ 规则 3 的 `canonicalize` 必然失败 ⇒
    // 本 mode 只验规则 1 / 规则 2（规则 3 由 `fixed_contracts` 用真实路径验）。
    let root = Path::new("/__fuzz_absent_workspace_root__");
    let verdict = ToolGuardrail::verify_path_access(root, requested);
    let path_str = requested.to_string_lossy();

    // 规则 1（`src: guardrail.rs:184-190`）：穿越特征 ⇒ 必须 `PathTraversal`。
    if path_str.contains("../") || path_str.contains("..\\") || path_str == ".." {
        assert!(
            matches!(verdict, Err(PreCallGuardError::PathTraversal(_))),
            "路径穿越形态被放行: {path_str:?} => {verdict:?}"
        );
    }

    // 规则 2（`src: guardrail.rs:192-212`）：敏感前缀 ⇒ 必须 `PathTraversal`。
    let lower = path_str.to_lowercase();
    if FORBIDDEN_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        assert!(
            matches!(verdict, Err(PreCallGuardError::PathTraversal(_))),
            "敏感系统路径被放行: {path_str:?} => {verdict:?}"
        );
    }
}

/// 一次性的、与随机输入无关的契约检查（`Once` 触发，避免每轮重复 IO）。
fn fixed_contracts() {
    // 规则 3（`src: guardrail.rs:214-226`）：绝对路径越出工作区 ⇒ `OutsideWorkspace`。
    // 只在候选路径**真实存在**且**确实不在工作区内**时断言（harness 侧去歧义：
    // 免得环境本身（TMPDIR=/usr 之类）造出假阳性）。
    let root = std::env::temp_dir();
    let Ok(canonical_root) = root.canonicalize() else {
        return;
    };
    if canonical_root.parent().is_none() {
        // 工作区根就是文件系统根 ⇒ 无边界可言, 不验。
        return;
    }
    for candidate in ["/etc", "/usr", "/bin", "/var", "/proc", "/sbin", "/lib"] {
        let path = Path::new(candidate);
        if !path.is_absolute() || !path.exists() {
            continue;
        }
        let Ok(canonical_target) = path.canonicalize() else {
            continue;
        };
        if canonical_target.starts_with(&canonical_root) {
            continue;
        }
        let verdict = ToolGuardrail::verify_path_access(&root, path);
        assert!(
            matches!(verdict, Err(PreCallGuardError::OutsideWorkspace(_))),
            "工作区外的绝对路径被放行: {candidate} => {verdict:?}"
        );
        return;
    }
}

/// 后置凭据绊线（`src: guardrail.rs:365-571`）。
fn mode_tripwire(payload: &[u8]) {
    let raw = String::from_utf8_lossy(payload);

    // ① 硬不变量（`src: guardrail.rs:449-457`）：脱敏后的输出不得再含
    //    "PRIVATE KEY-----"。
    let res = ToolGuardrail::scan_and_sanitize_output(&raw);
    assert!(
        !res.sanitized_output.contains("PRIVATE KEY-----"),
        "绊线输出仍含裸私钥标记: raw={raw:?} out={:?}",
        res.sanitized_output
    );

    // ② PEM 私钥体不得留存（`src: guardrail.rs:420-424` 整块收口）。
    check_pem_body_not_surviving(&raw);

    // ③ 前置**其它**块（证书等）形态：专门喂「块首/块尾锚点错配」的通道。
    check_pem_after_other_block(payload);

    // ④ 同族多枚全带走（`src: guardrail.rs:316-327`）。
    for family in FAMILIES.iter() {
        check_family_all_instances(family, payload);
    }

    // ⑤ 占位符族豁免（`src: guardrail.rs:528-537`）：`password=<占位符>` 的值
    //    不是明文口令 ⇒ 占位符必须原样存活（还原锚点不可被单向截断毁掉）。
    let mut bridge = RedactionBridge::with_session_key([0x5a; 32]);
    let placeholder = bridge.register_value(PiiKind::CredentialKey, "zephyrquartz-anchor");
    let token = placeholder.render();
    let text = format!("password={token}\n下一行完好");
    let res2 = ToolGuardrail::scan_and_sanitize_output(&text);
    assert!(
        res2.sanitized_output.contains(&token),
        "占位符被当明文口令截断（还原锚点被毁）: out={:?}",
        res2.sanitized_output
    );
    assert!(
        !res2
            .leaked_kinds
            .contains(&LeakedCredentialKind::PlaintextPassword),
        "占位符不该被报成明文口令: {res2:?}"
    );
}

/// 同族多枚必须全部带走（`src: guardrail.rs:316-327` `redact_all`）。
fn check_family_all_instances(family: &CredentialFamily, payload: &[u8]) {
    // 载荷长度取**恰好落在接受区间下界** ⇒ 同时钉住长度边界（`token.len() >= N`）。
    let need = family.min_len.saturating_sub(family.prefix.len());
    let cap = family
        .max_len
        .map(|max| max.saturating_sub(family.prefix.len()))
        .unwrap_or(need);
    if cap < need {
        return;
    }
    let mut first = alnum_payload(payload, need);
    while first.len() < need {
        first.push('A');
    }
    let mut second = alnum_payload(&payload[payload.len() / 2..], need);
    while second.len() < need {
        second.push('B');
    }
    let token_a = format!("{}{}", family.prefix, first);
    let token_b = format!("{}{}", family.prefix, second);
    if token_a.len() < family.min_len
        || token_a.len() > family.max_len.unwrap_or(usize::MAX)
        || token_b.len() < family.min_len
        || token_b.len() > family.max_len.unwrap_or(usize::MAX)
    {
        return;
    }
    let text = format!("前段 {token_a} 中段 {token_b} 尾段");
    let res = ToolGuardrail::scan_and_sanitize_output(&text);
    assert!(
        !res.sanitized_output.contains(&first),
        "{} 族第一枚凭证仍在输出里: {text:?} => {:?}",
        family.name,
        res.sanitized_output
    );
    assert!(
        !res.sanitized_output.contains(&second),
        "{} 族第二枚凭证仍在输出里: {text:?} => {:?}",
        family.name,
        res.sanitized_output
    );
}

/// PEM 私钥体不得留在脱敏输出里（`src: guardrail.rs:420-424`）。
fn check_pem_body_not_surviving(raw: &str) {
    let Some((body_start, body_end)) = pem_body_span(raw) else {
        return;
    };
    let body = raw[body_start..body_end].trim_matches(|c: char| c == '\r' || c == '\n');
    // 载荷太短 ⇒ 无区分度（harness 侧跳过, 免得撞词假阳性）。
    if body.len() < 8 {
        return;
    }
    // 载荷在块外也出现 ⇒ 撞词, 跳过（harness 侧去歧义）。
    let outside = format!("{}{}", &raw[..body_start], &raw[body_end..]);
    if outside.contains(body) {
        return;
    }
    let res = ToolGuardrail::scan_and_sanitize_output(raw);
    assert!(
        !res.sanitized_output.contains(body),
        "PEM 私钥体留在输出里（整块收口违约）: raw={raw:?} => out={:?}",
        res.sanitized_output
    );
}

/// 构造「前置其它块 + 私钥块」形态：`src: guardrail.rs:426-448` 的 PEM 扫描以
/// **全文第一个 `-----BEGIN`** 当块首、以**全文第一个 `-----END` 之后的
/// `KEY-----`** 当块尾 ⇒ 前置的证书块会把自己的 END 行锚成私钥块的块尾 ⇒
/// 私钥体留在输出里。本 mode 专门喂这个形态。
fn check_pem_after_other_block(payload: &[u8]) {
    let cert_body = alnum_payload(payload, 12);
    let key_body = alnum_payload(&payload[payload.len() / 2..], 24);
    if cert_body.len() < 8 || key_body.len() < 8 {
        return;
    }
    let raw = format!(
        "-----BEGIN CERTIFICATE-----\n{cert_body}\n-----END CERTIFICATE-----\n\
         -----BEGIN RSA PRIVATE KEY-----\n{key_body}\n-----END RSA PRIVATE KEY-----"
    );
    check_pem_body_not_surviving(&raw);
}

/// 解析**完整** PEM 私钥块的**体**区间 `(body_start, body_end)`。
///
/// 头 = 同一行内的 `-----BEGIN ... PRIVATE KEY-----`；尾 = 其后 `-----END` 所在行
/// 里的 `KEY-----`。不是完整块 ⇒ `None`。
fn pem_body_span(text: &str) -> Option<(usize, usize)> {
    const MARKER: &str = "PRIVATE KEY-----";
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(MARKER) {
        let marker = from + rel;
        let line_start = text[..marker].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let Some(begin_rel) = text[line_start..marker].rfind("-----BEGIN") else {
            from = marker + MARKER.len();
            continue;
        };
        let header_start = line_start + begin_rel;
        let header_end = marker + MARKER.len();
        // 头与 `PRIVATE KEY-----` 之间不得出现 `-----END`（那就不是本块的头）。
        if text[header_start..header_end].contains("-----END") {
            from = marker + MARKER.len();
            continue;
        }
        let Some(end_rel) = text[header_end..].find("-----END") else {
            return None;
        };
        let footer_at = header_end + end_rel;
        let footer_line_end = text[footer_at..]
            .find('\n')
            .map(|i| footer_at + i)
            .unwrap_or(text.len());
        if !text[footer_at..footer_line_end].contains("KEY-----") {
            return None;
        }
        return Some((header_end, footer_at));
    }
    None
}

/// 出境双扫（`src: guardrail.rs:573-589`）。
fn mode_egress_sweep(payload: &[u8]) {
    // 登记一个**非凭据形状**的真值（免得被 PII 检测器抢先认领 ⇒ 占位符可预期）。
    let mut bridge = RedactionBridge::with_session_key([0x5a; 32]);
    let secret = format!("zephyrquartz-{}", alnum_payload(payload, 8));
    let placeholder = bridge.register_value(PiiKind::CredentialKey, &secret);
    let token = placeholder.render();
    let text = format!("前段 {secret} 中段 {token} 尾段");
    let swept = ToolGuardrail::sweep_output_for_egress(&text, &mut bridge);
    let out = &swept.tripwire.sanitized_output;
    assert!(!out.contains(&secret), "真值仍出境: {out:?}");
    assert!(
        out.contains(&token),
        "占位锚点被破坏（受控回程不可还原）: {out:?}"
    );

    // 凭据形状的真值: 运行时构造（源码里**不出现** `sk-` + 20+ 位字面量）。
    let credential = format!("sk-{}", "1".repeat(26));
    let mut bridge2 = RedactionBridge::with_session_key([0x5a; 32]);
    let swept2 =
        ToolGuardrail::sweep_output_for_egress(&format!("头 {credential} 尾"), &mut bridge2);
    assert!(
        !swept2.tripwire.sanitized_output.contains(&credential),
        "凭据真值仍出境: {:?}",
        swept2.tripwire.sanitized_output
    );
}

// ===========================================================================
// 面 ② 观测闸
// ===========================================================================

/// 观测闸判定（`src: observed_gate.rs:333-389`）。
fn mode_observed_gate(payload: &[u8]) {
    let byte = |index: usize| payload.get(index).copied().unwrap_or(0);
    let observed = version(u64::from(byte(0)), u64::from(byte(1)));
    // 必定与 `observed` 不同的另一个版本（逐字段异或 ⇒ 不相等）。
    let other = version(u64::from(byte(0) ^ 0x5a), u64::from(byte(1) ^ 0xa5));

    let path = Path::new("notes.txt");
    let gate = ObservedGate::new();
    match byte(2) % 3 {
        0 => {} // 未观测
        1 => gate.observe_present(path, observed),
        _ => gate.observe_missing(path),
    }
    let keyed = match byte(3) % 3 {
        0 => None,
        1 => Some(observed),
        _ => Some(other),
    };
    let current = match byte(4) % 3 {
        0 => None,
        1 => Some(observed),
        _ => Some(other),
    };
    let kind = if byte(5) % 2 == 0 {
        WriteKind::Replace
    } else {
        WriteKind::Create
    };
    let create_if_absent = byte(6) % 2 == 0;

    let intent = WriteIntent {
        path,
        kind,
        replace_if_version: keyed,
        create_if_absent,
        current,
    };
    let state = gate.state(path);
    let verdict = gate.check_write(&intent);

    // oracle = 文档规则（`src: observed_gate.rs:333-345`），不是实现的复制。
    let expected: Result<(), GateDenial> = match kind {
        WriteKind::Replace => match state {
            ObservationState::Unobserved => Err(GateDenial::NotObserved(path.to_path_buf())),
            ObservationState::Missing => Err(GateDenial::ObservedMissing(path.to_path_buf())),
            ObservationState::Present(seen) => {
                let keyed_stale = keyed.is_some_and(|k| k != seen);
                let current_changed = current != Some(seen);
                if keyed_stale || current_changed {
                    Err(GateDenial::VersionMismatch {
                        path: path.to_path_buf(),
                        keyed: seen,
                        found: current,
                    })
                } else {
                    Ok(())
                }
            }
        },
        WriteKind::Create => {
            if create_if_absent && current.is_some() {
                Err(GateDenial::TargetAppeared(path.to_path_buf()))
            } else if state != ObservationState::Missing && current.is_some() {
                Err(GateDenial::CreateTargetExists(path.to_path_buf()))
            } else {
                Ok(())
            }
        }
    };

    match (&verdict, &expected) {
        (Ok(()), Ok(())) => {}
        (Err(got), Err(want)) => assert!(
            std::mem::discriminant(got) == std::mem::discriminant(want),
            "门禁拒绝理由与文档规则不符: got={got:?} want={want:?} intent={intent:?} state={state:?}"
        ),
        (got, want) => panic!(
            "门禁判定与文档规则不符: got={got:?} want={want:?} intent={intent:?} state={state:?}"
        ),
    }

    // fail-closed 底线（独立于上表的显式断言）：
    // ① 覆盖类 + 未观测 ⇒ 必须拒（"未读不得覆盖写"）。
    if kind == WriteKind::Replace && state == ObservationState::Unobserved {
        assert!(verdict.is_err(), "盲写被放行: {intent:?}");
    }
    // ② 覆盖类 + 观测版本 ≠ 现状 ⇒ 必须拒（"过期观测放行了已变的状态"）。
    if kind == WriteKind::Replace {
        if let ObservationState::Present(seen) = state {
            if current != Some(seen) {
                assert!(
                    verdict.is_err(),
                    "过期观测放行了已变的状态: {intent:?} state={state:?}"
                );
            }
        }
    }
    // ③ 覆盖类 + 观测版本 == 现状（钥匙不冲突）⇒ 必须放行（"读后可写"）。
    if kind == WriteKind::Replace {
        if let ObservationState::Present(seen) = state {
            if current == Some(seen) && keyed.is_none_or(|k| k == seen) {
                assert!(
                    verdict.is_ok(),
                    "读后同版本写被拒: {intent:?} state={state:?} => {verdict:?}"
                );
            }
        }
    }
}

/// 排除清单（`src: observed_gate.rs:162-230`）：`*` 匹配任意片段, 其余逐字面。
fn mode_exclusion_list(payload: &[u8]) {
    let split = payload.len() / 2;
    let (pat_bytes, path_bytes) = payload.split_at(split);
    let pattern = build_pattern(pat_bytes);
    let path = build_path(path_bytes);
    if pattern.is_empty() || path.is_empty() {
        return;
    }
    // 词法归一只在**无歧义域**内重算（相对路径、无 `..`、无 `\`）——
    // 否则 oracle 自己的归一口径与实现不一致就成了假阳性源（本仓踩过）。
    let key = oracle_path_key(&path);
    let expected = glob_match(&pattern, &key);
    let list = ExclusionList::new([pattern.clone()]);
    let actual = list.matches(Path::new(&path));
    assert_eq!(
        actual, expected,
        "排除清单判定与契约背离: pattern={pattern:?} path={path:?} key={key:?}"
    );

    // 排除清单透明（`src: observed_gate.rs:162-166` "命中路径完全透明"）：
    // 命中 ⇒ 不记录、不判定 ⇒ 未观测的覆盖写也放行。
    let gate = ObservedGate::with_exclusions(ExclusionList::new([pattern.clone()]));
    if actual {
        let intent = WriteIntent {
            path: Path::new(&path),
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
            current: None,
        };
        assert!(
            gate.check_write(&intent).is_ok(),
            "命中排除清单的路径未被透明放行: pattern={pattern:?} path={path:?}"
        );
        gate.observe_present(Path::new(&path), version(1, 1));
        assert_eq!(
            gate.state(Path::new(&path)),
            ObservationState::Unobserved,
            "排除清单路径不该被记录观测: pattern={pattern:?} path={path:?}"
        );
    }
}

/// 从字节构造排除模式（`*` 或字母数字）。
fn build_pattern(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(12)
        .map(|b| if *b % 3 == 0 { '*' } else { alnum_char(*b) })
        .collect()
}

/// 从字节构造相对路径（字母数字 / `/` / `.`；首字符不许是 `/`）。
fn build_path(bytes: &[u8]) -> String {
    let mut out: String = bytes
        .iter()
        .take(16)
        .map(|b| match *b % 7 {
            0 => '/',
            1 => '.',
            _ => alnum_char(*b),
        })
        .collect();
    if out.starts_with('/') {
        out.remove(0);
    }
    // `..` 组件的折叠语义由 `path_key` 定（`src: observed_gate.rs:249-253`），
    // 本 oracle 不重算它 ⇒ 直接跳过这类路径（harness 侧去歧义）。
    if out.contains("..") {
        return String::new();
    }
    out
}

/// oracle 版的词法归一（`src: observed_gate.rs:245-267` `path_key`）：
/// 去 `.` 组件、空组件，统一 `/` 分隔。
fn oracle_path_key(path: &str) -> String {
    // 口径**必须**与产品的 `path_key`（`src: observed_gate.rs:245-266`）一致。
    //
    // ⚠️ 我第一版用 `split('/').filter(非空)` ⇒ **丢掉了绝对路径的前导 `/`**
    // ⇒ 键从 `/www9` 变成 `www9` ⇒ 与实现的口径不同 ⇒ **假阳性**
    //   （实测: pattern=`www***` path=`/www9` ⇒ oracle 说该匹配、实现说不匹配）。
    //   产品按**键**判定（`pattern_matches(pattern, &key)`）⇒ 绝对路径的键带前导 `/`
    //   ⇒ pattern `www***` 不以 `/` 开头 ⇒ **不匹配是正确行为**。
    //
    // ⇒ 这里照抄实现的**结构语义**：`Path::components()` 走一遍
    // （`CurDir` 丢弃、`ParentDir` 回退一级、其余 push ⇒ 绝对路径**保留**前导 `/`）
    // + `\` 归一为 `/` + Windows verbatim 前缀剥离 + Windows 大小写折叠。
    use std::path::{Component, Path, PathBuf};
    let mut joined = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                joined.pop();
            }
            other => joined.push(other.as_os_str()),
        }
    }
    let key = joined.to_string_lossy().replace('\\', "/");
    let key = if let Some(rest) = key.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = key.strip_prefix("//?/") {
        rest.to_string()
    } else {
        key
    };
    if cfg!(windows) {
        // Windows 折叠大小写（`src: observed_gate.rs:264-265`）。
        key.to_lowercase()
    } else {
        key
    }
}

/// oracle 版的最小通配匹配（`src: observed_gate.rs:200-230`）：
/// `*` 匹配任意片段, 其余逐字面。用**经典回溯两指针**实现（与被测代码的
/// 「分段 + 首次出现」实现路径不同 ⇒ 不是同义反复）。
fn glob_match(pattern: &str, key: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let k: Vec<char> = key.chars().collect();
    let mut pi = 0usize;
    let mut ki = 0usize;
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ki < k.len() {
        if pi < p.len() && p[pi] == k[ki] {
            pi += 1;
            ki += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ki;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ki = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

// ===========================================================================
// 面 ③ 子进程监管（只测纯判定层）
// ===========================================================================

/// 子进程监管的策略决策 + 重启限额（`src: std_sub_supervisor.rs:140-254`）。
fn mode_sub_supervisor(payload: &[u8]) {
    let byte = |index: usize| payload.get(index).copied().unwrap_or(0);
    let strategy = match byte(0) % 3 {
        0 => RestartStrategy::OneForOne,
        1 => RestartStrategy::RestForOne,
        _ => RestartStrategy::Transient,
    };
    let reason = build_reason(byte(1), byte(2));
    let id = format!("child-{}", byte(3));

    let mut spec = ChildSpec::new(id.clone(), "/bin/true");
    spec.restart = strategy;
    let mut supervisor = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec.clone()]);

    // 未知 child ⇒ `UnknownChild`（`src: std_sub_supervisor.rs:201-206`）。
    let unknown_id = format!("{id}-absent");
    match supervisor.on_child_exit(&unknown_id, reason.clone()) {
        Err(SupervisorError::UnknownChild(_)) => {}
        other => panic!("未知 child 未报 UnknownChild: {other:?}"),
    }

    // 策略决策（`src: std_sub_supervisor.rs:213-253`）。
    let decision = supervisor.on_child_exit(&id, reason.clone());
    let expected: Result<RestartDecision, SupervisorError> = match strategy {
        // "OneForOne: 任何 exit 都重启" / "RestForOne: 任何 exit 都重启"。
        RestartStrategy::OneForOne | RestartStrategy::RestForOne => Ok(RestartDecision::RestartNow),
        // "Transient: 正常退出 (Normal) 不重启; 异常才重启"。
        RestartStrategy::Transient => match &reason {
            ExitReason::Normal { code: 0 } => Ok(RestartDecision::DoNotRestart),
            // `Normal { code: 非 0 }` 生产不可达 + 两种读法不等价 ⇒ 不作断言。
            ExitReason::Normal { .. } => return,
            _ => Ok(RestartDecision::RestartNow),
        },
    };
    match (decision, expected) {
        (Ok(got), Ok(want)) => assert_eq!(
            got, want,
            "重启决策与策略契约不符: strategy={strategy:?} reason={reason:?}"
        ),
        (got, want) => panic!(
            "重启决策与策略契约不符: got={got:?} want={want:?} strategy={strategy:?} reason={reason:?}"
        ),
    }

    // 重启限额（`src: std_sub_supervisor.rs:56-57` "v1 默认: 5 次 / 60 秒"）。
    // 独立 supervisor ⇒ 计数从零起。
    let mut limited = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
    for round in 1..=5 {
        let decision = limited.on_child_exit(&id, ExitReason::Abnormal { code: 1 });
        assert!(
            matches!(decision, Ok(RestartDecision::RestartNow)),
            "窗口内第 {round} 次重启应当允许: {decision:?}"
        );
    }
    let over = limited.on_child_exit(&id, ExitReason::Abnormal { code: 1 });
    assert!(
        matches!(over, Err(SupervisorError::RestartLimitExceeded(_))),
        "窗口内第 6 次重启必须被限额拒: {over:?}"
    );
}

/// 由字节构造退出原因。
fn build_reason(kind: u8, payload: u8) -> ExitReason {
    match kind % 7 {
        0 => ExitReason::Normal { code: 0 },
        // 非零 Normal: 生产不可达 ⇒ 判定侧跳过（见文件头「刻意不造的期望」）。
        1 => ExitReason::Normal {
            code: i32::from(payload) + 1,
        },
        2 => ExitReason::Abnormal {
            code: i32::from(payload),
        },
        3 => ExitReason::Signaled {
            signal: i32::from(payload),
        },
        4 => ExitReason::Killed,
        5 => ExitReason::HealthCheckFailed,
        _ => ExitReason::StartTimeout,
    }
}

// ===========================================================================
// 入口
// ===========================================================================

fuzz_target!(|data: &[u8]| {
    // 一次性契约检查（与随机输入无关的部分：真实路径的工作区边界）。
    static FIXED: Once = Once::new();
    FIXED.call_once(fixed_contracts);

    let Some(&mode) = data.first() else {
        return;
    };
    let payload = &data[1..];
    match mode % 6 {
        0 => mode_path_gate(payload),
        1 => mode_tripwire(payload),
        2 => mode_egress_sweep(payload),
        3 => mode_observed_gate(payload),
        4 => mode_exclusion_list(payload),
        _ => mode_sub_supervisor(payload),
    }
});
