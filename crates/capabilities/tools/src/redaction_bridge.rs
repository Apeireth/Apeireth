//! 出境脱敏双向闸 (egress redaction bridge).
//!
//! 脱敏不是一次性预处理, 是**出境必扫、落地才还原**的双向闸:
//!
//! 1. **占位不可逆指纹**: 真值出境前换成 `__AP_<类别>_<指纹12>__` (占位格式
//!    规格见 [`apeireth_governance::input_security`] 的占位族原语); 指纹 =
//!    HMAC-SHA256(会话随机密钥, 真值) 截断 12 位 —— 同会话同值恒同占位
//!    (模型可跨轮关联), 跨会话换别名, 对出境面不可逆。
//! 2. **还原双闸**: 占位符→真值**只在受控回程**发生
//!    ([`RedactionBridge::restore_for_local`], 模型结果带回的占位符在本地
//!    落盘前还原), 且**只还原本会话登记簿登记过的指纹**; 未登记占位符
//!    不还原、照实留痕。出境面 ([`RedactionBridge::redact_for_egress`] /
//!    [`RedactionBridge::rescan_history`]) 永不吐回真值。
//! 3. **历史回扫**: 工具轨迹/会话历史以真值形态存储, 明文可能来自早年轮次
//!    (历史搭车) —— 每次出境前对历史段逐段回扫
//!    ([`RedactionBridge::rescan_history`]), 真值面(检测器命中 + 本会话已
//!    登记值的裸/变体形态)一律换占位。
//! 4. **脱敏账本**: 每次替换/还原一条 [`LedgerEntry`] (类别/指纹/时间/还原
//!    与否), **零真值入账**; [`RedactionBridge::ledger_jsonl`] /
//!    [`RedactionBridge::persist_ledger`] 落 `redaction.jsonl`。
//!
//! # 双闸时序
//!
//! ```text
//!  出境 (每一轮)                                   受控回程 (本地落盘前)
//!  ─────────────                                  ─────────────────────
//!  历史段 [工具轨迹/会话历史, 存真值]               模型结果 (携带占位符)
//!    │ ① 历史回扫 rescan_history                     │ ④ 还原 restore_for_local
//!    │    真值面 → __AP_<类>_<fp>__                  │    闸一: 只在受控回程还原
//!  当前文本                                         │    闸二: fp ∈ 本会话登记簿
//!    │ ② 出境脱敏 redact_for_egress                  │      ├ 命中   → 还原真值
//!    │    真值 → 占位符 + 登记簿 + 账本               │      └ 未登记 → 照实留痕
//!    │ ③ (可选) 既有凭据绊线补单向标签                │         (账本记 skipped)
//!    ▼                                              ▼
//!  出境文本 (只见占位族 [+ 单向标签])               本地文本 (真值) + 账本
//! ```
//!
//! # 变体绕过防御面 (0 装: 只挡声明的族)
//!
//! 历史里除了真值本体, 还可能出现真值的**变体形态**。对**本会话登记簿中
//! 已登记的值**, 回扫/出境扫额外清扫以下形态并换回同一占位符:
//!
//! * V1 裸形态 (逐字);
//! * V2 base64 (标准字母表, 带/不带 `=` 填充);
//! * V3 百分号编码 (大写/小写 hex);
//! * V4 十六进制串 (大写/小写);
//! * V5 拆分串 (真值字符间插空白/零宽字符 —— 归一影子串扫描)。
//!
//! **已知边界 (不声称防御)**: 未登记值的变体形态、多层编码链 (base64 的
//! base64)、字符集转写 (全角/音译)、值内分隔符替换。这些留给出境面检测器
//! 与后续阶段, 不在本件声称的防御面内。
//!
//! # 情感隐私类别 (拍板 ㉓ `FEELING`)
//!
//! `__AP_FEELING_<指纹12>__` 是占位族的第 9 个类别 (类别原语见
//! [`apeireth_governance::input_security`])。**识别=登记簿驱动**: 情感隐私值
//! 只经显式登记进入本会话登记簿 (调用方标注, 入口
//! [`RedactionBridge::register_feeling`] / [`RedactionBridge::register_value`]),
//! 之后与其余类别同待遇 —— 出境/回扫换占位、变体清扫、受控回程按登记簿还原。
//! 词面 (`我很爱` / `我恨` / `我害怕` 类自述情感句) **只登记不强拦**: 词面
//! 提示在治理面只作辅助提示, 不自动脱敏、不拦截、不升级裁决。
//!
//! # 接线状态 (0 装)
//!
//! 本件是机制面: [`crate::guardrail::ToolGuardrail::sweep_output_for_egress`]
//! 已把工具输出的后置出站面接上双层扫; 网关/补全链路的出境挂接与会话历史
//! 存储面的接入不在本战场, 未接线。

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::{
    session_fingerprint, PiiDetector, PiiKind, RedactionPlaceholder,
};
use serde::{Deserialize, Serialize};

/// 脱敏账本默认文件名 (JSONL, 一行一条 [`LedgerEntry`])。
pub const REDACTION_LEDGER_FILE: &str = "redaction.jsonl";

/// 变体形态扫描的最小真值长度 (字节) —— 过短的值不展开变体形态, 防误伤。
const MIN_VARIANT_VALUE_BYTES: usize = 4;

/// 事件发生在哪一面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionSurface {
    /// 当前轮出境文本。
    Egress,
    /// 会话历史/工具轨迹段的出境前回扫。
    HistoryRescan,
    /// 受控回程 (本地落盘前) 的还原面。
    LocalRestore,
    /// 调用方显式登记 (预置已知秘密)。
    ExplicitRegistration,
}

impl RedactionSurface {
    /// Stable label for the ledger.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Egress => "egress",
            Self::HistoryRescan => "history_rescan",
            Self::LocalRestore => "local_restore",
            Self::ExplicitRegistration => "explicit_registration",
        }
    }
}

/// 账本事件: 替换还是还原。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEvent {
    /// 真值 → 占位符。
    Redacted,
    /// 占位符 → 真值 (受控回程, 已登记)。
    Restored,
    /// 未登记占位符: 不还原、照实留痕。
    RestoreSkippedUnregistered,
}

impl LedgerEvent {
    /// Stable label for the ledger.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Redacted => "redacted",
            Self::Restored => "restored",
            Self::RestoreSkippedUnregistered => "restore_skipped_unregistered",
        }
    }
}

/// 脱敏账本条目 —— **零真值入账**: 只有类别/指纹/时间/还原与否与面标签。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Unix 毫秒时间戳。
    pub ts_unix_ms: u64,
    /// PII 类别标签 (与 [`PiiKind::label`] 同口径)。
    pub category: String,
    /// 12 位小写十六进制会话指纹。
    pub fingerprint: String,
    /// 替换/还原事件。
    pub event: LedgerEvent,
    /// 还原与否 (替换面恒 false)。
    pub restored: bool,
    /// 事件面。
    pub surface: RedactionSurface,
}

/// 一次脱敏替换命中 (不含真值)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionHit {
    pub kind: PiiKind,
    pub fingerprint: String,
    pub surface: RedactionSurface,
}

/// 出境脱敏结果: 替换后的文本 + 命中清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressRedaction {
    pub text: String,
    pub hits: Vec<RedactionHit>,
}

/// 历史回扫结果 (逐段就地替换)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRescanReport {
    pub segments_swept: usize,
    pub hits: Vec<RedactionHit>,
}

/// 受控回程还原结果。
///
/// `text` 是给本地落盘面的真值文本 —— 这是全库唯一允许携带真值的出口,
/// 故 `Debug` 手写实现只报指纹与长度, 不打印文本 (日志面永不落明文)。
pub struct RestoreReport {
    pub text: String,
    /// 已还原占位符的指纹 (登记簿命中)。
    pub restored: Vec<String>,
    /// 未登记占位符的指纹 (照实留痕, 未还原)。
    pub unrestored: Vec<String>,
}

impl fmt::Debug for RestoreReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestoreReport")
            .field("text_len", &self.text.len())
            .field("restored", &self.restored)
            .field("unrestored", &self.unrestored)
            .finish()
    }
}

/// 本会话登记簿条目 (真值驻留处 —— `Debug` 永不打印)。
struct RegistryEntry {
    kind: PiiKind,
    value: String,
}

impl fmt::Debug for RegistryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegistryEntry")
            .field("kind", &self.kind)
            .field("value_len", &self.value.len())
            .finish()
    }
}

/// 出境脱敏双向闸: 会话随机密钥 + 本会话登记簿 + 脱敏账本。
///
/// 一个实例 = 一个会话的脱敏域: 跨实例指纹不可关联, 登记簿互不认账
/// (跨会话占位符在任何实例里都还原不了)。
pub struct RedactionBridge {
    session_key: [u8; 32],
    /// 指纹 → 登记条目 (真值)。
    registry: HashMap<String, RegistryEntry>,
    /// (类别, 真值) → 指纹 (同值同类恒同占位的去重索引)。
    value_index: HashMap<(String, String), String>,
    ledger: Vec<LedgerEntry>,
}

impl fmt::Debug for RedactionBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedactionBridge")
            .field("session_key", &"<redacted>")
            .field("registered", &self.registry.len())
            .field("ledger_entries", &self.ledger.len())
            .finish()
    }
}

impl Default for RedactionBridge {
    fn default() -> Self {
        Self::new_session()
    }
}

impl RedactionBridge {
    /// 新会话脱敏域: 32 字节随机会话密钥 (uuid v4 熵源, 每次调用新域)。
    pub fn new_session() -> Self {
        Self::with_session_key(fresh_session_key())
    }

    /// 以显式会话密钥建域 (跨进程续接同一脱敏域时由调用方保管密钥)。
    pub fn with_session_key(session_key: [u8; 32]) -> Self {
        Self {
            session_key,
            registry: HashMap::new(),
            value_index: HashMap::new(),
            ledger: Vec::new(),
        }
    }

    /// 显式登记一个真值 (预置已知秘密, 供后续出境/回扫换占位与变体清扫)。
    pub fn register_value(&mut self, kind: PiiKind, value: &str) -> RedactionPlaceholder {
        let fingerprint = self.register(kind, value);
        self.ledger.push(LedgerEntry {
            ts_unix_ms: now_unix_ms(),
            category: kind.label().to_string(),
            fingerprint: fingerprint.clone(),
            event: LedgerEvent::Redacted,
            restored: false,
            surface: RedactionSurface::ExplicitRegistration,
        });
        RedactionPlaceholder::new(kind, &fingerprint)
    }

    /// 显式登记一个情感隐私值 (拍板 ㉓ `FEELING` 类别面)。
    ///
    /// 识别=登记簿驱动的登记口: 调用方标注这是自述情感值, 本会话登记簿记下
    /// 后, 该值的裸/变体形态在出境与历史回扫面换 `__AP_FEELING_<指纹12>__`,
    /// 受控回程按登记簿还原。词面启发式 (自述情感句) 只作辅助提示, 是否登记
    /// 由调用方裁量 —— 未登记 = 不识别、不脱敏 (只登记不强拦)。
    pub fn register_feeling(&mut self, value: &str) -> RedactionPlaceholder {
        self.register_value(PiiKind::Feeling, value)
    }

    /// 出境脱敏: 真值面 → 占位符 (检测器命中 + 已登记值的裸/变体形态),
    /// 同步登记簿与账本。输出只含占位族, 永不含真值。
    pub fn redact_for_egress(&mut self, text: &str) -> EgressRedaction {
        self.sweep(text, RedactionSurface::Egress)
    }

    /// 历史回扫 (堵历史搭车): 工具轨迹/会话历史段在**每次出境前**逐段回扫,
    /// 早年留下的明文真值面就地换占位。返回逐段命中统计。
    pub fn rescan_history(&mut self, segments: &mut [String]) -> HistoryRescanReport {
        let mut hits = Vec::new();
        let mut segments_swept = 0usize;
        for segment in segments.iter_mut() {
            let redaction = self.sweep(segment, RedactionSurface::HistoryRescan);
            *segment = redaction.text;
            hits.extend(redaction.hits);
            segments_swept += 1;
        }
        HistoryRescanReport {
            segments_swept,
            hits,
        }
    }

    /// 受控回程还原 (**双闸**):
    ///
    /// * 闸一: 还原只在此面发生 —— 模型结果带回的占位符在本地落盘前还原;
    /// * 闸二: 只还原本会话登记簿登记过的指纹, 未登记占位符**不还原、
    ///   照实留痕** (文本原样保留, 账本记 skipped)。
    pub fn restore_for_local(&mut self, text: &str) -> RestoreReport {
        let matches = RedactionPlaceholder::scan(text);
        let mut out = text.to_string();
        let mut restored = Vec::new();
        let mut unrestored = Vec::new();
        for matched in matches.iter().rev() {
            let fingerprint = matched.placeholder.fingerprint.clone();
            match self.registry.get(&fingerprint) {
                Some(entry) => {
                    out.replace_range(matched.start..matched.end, &entry.value);
                    restored.push(fingerprint.clone());
                    self.ledger.push(LedgerEntry {
                        ts_unix_ms: now_unix_ms(),
                        category: matched.placeholder.kind.label().to_string(),
                        fingerprint,
                        event: LedgerEvent::Restored,
                        restored: true,
                        surface: RedactionSurface::LocalRestore,
                    });
                }
                None => {
                    unrestored.push(fingerprint.clone());
                    self.ledger.push(LedgerEntry {
                        ts_unix_ms: now_unix_ms(),
                        category: matched.placeholder.kind.label().to_string(),
                        fingerprint,
                        event: LedgerEvent::RestoreSkippedUnregistered,
                        restored: false,
                        surface: RedactionSurface::LocalRestore,
                    });
                }
            }
        }
        restored.reverse();
        unrestored.reverse();
        RestoreReport {
            text: out,
            restored,
            unrestored,
        }
    }

    /// 账本条目 (只读)。
    pub fn ledger_entries(&self) -> &[LedgerEntry] {
        &self.ledger
    }

    /// 账本 JSONL (一行一条, 零真值)。
    pub fn ledger_jsonl(&self) -> String {
        let mut out = String::new();
        for entry in &self.ledger {
            out.push_str(
                &serde_json::to_string(entry).expect("ledger entry serialization cannot fail"),
            );
            out.push('\n');
        }
        out
    }

    /// 追加账本到 `redaction.jsonl` (append-only, 零真值)。
    pub fn persist_ledger(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write_all(self.ledger_jsonl().as_bytes())
    }

    /// 登记簿登记数。
    pub fn registered_values(&self) -> usize {
        self.registry.len()
    }

    /// 登记一个真值, 返回会话指纹 (已登记则复用同一指纹 —— 同值恒同占位)。
    fn register(&mut self, kind: PiiKind, value: &str) -> String {
        let index_key = (kind.label().to_string(), value.to_string());
        if let Some(fingerprint) = self.value_index.get(&index_key) {
            return fingerprint.clone();
        }
        let fingerprint = session_fingerprint(&self.session_key, value);
        self.registry
            .entry(fingerprint.clone())
            .or_insert_with(|| RegistryEntry {
                kind,
                value: value.to_string(),
            });
        self.value_index.insert(index_key, fingerprint.clone());
        fingerprint
    }

    /// 一面的清扫核心: 保护既有占位符 → PII 检测命中 → 已登记值变体形态。
    /// 三类候选按优先级去重重叠后右起替换。
    fn sweep(&mut self, text: &str, surface: RedactionSurface) -> EgressRedaction {
        struct Candidate {
            start: usize,
            end: usize,
            kind: PiiKind,
            fingerprint: String,
        }

        // 既有占位符是最高保护层: 绝不二次替换 (幂等固定点的来源)。
        let mut taken: Vec<(usize, usize)> = RedactionPlaceholder::scan(text)
            .iter()
            .map(|matched| (matched.start, matched.end))
            .collect();
        let mut accepted: Vec<Candidate> = Vec::new();

        for finding in PiiDetector::findings(text) {
            if overlaps_any(finding.start, finding.end, &taken) {
                continue;
            }
            let value = &text[finding.start..finding.end];
            let fingerprint = self.register(finding.kind, value);
            taken.push((finding.start, finding.end));
            accepted.push(Candidate {
                start: finding.start,
                end: finding.end,
                kind: finding.kind,
                fingerprint,
            });
        }

        // 已登记值的变体形态清扫 (含本轮刚登记的值)。快照按指纹定序,
        // 跨值重叠命中取舍与 HashMap 迭代序无关 (确定性承诺)。
        let mut snapshot: Vec<(PiiKind, String, String)> = self
            .registry
            .iter()
            .map(|(fingerprint, entry)| (entry.kind, entry.value.clone(), fingerprint.clone()))
            .collect();
        snapshot.sort_by(|left, right| left.2.cmp(&right.2));
        for (kind, value, fingerprint) in &snapshot {
            for (start, end) in variant_ranges(text, value) {
                if overlaps_any(start, end, &taken) {
                    continue;
                }
                taken.push((start, end));
                accepted.push(Candidate {
                    start,
                    end,
                    kind: *kind,
                    fingerprint: fingerprint.clone(),
                });
            }
        }

        accepted.sort_by_key(|candidate| candidate.start);
        let mut out = text.to_string();
        let mut hits = Vec::new();
        for candidate in accepted.iter().rev() {
            let placeholder = RedactionPlaceholder::new(candidate.kind, &candidate.fingerprint);
            out.replace_range(candidate.start..candidate.end, &placeholder.render());
            self.ledger.push(LedgerEntry {
                ts_unix_ms: now_unix_ms(),
                category: candidate.kind.label().to_string(),
                fingerprint: candidate.fingerprint.clone(),
                event: LedgerEvent::Redacted,
                restored: false,
                surface,
            });
            hits.push(RedactionHit {
                kind: candidate.kind,
                fingerprint: candidate.fingerprint.clone(),
                surface,
            });
        }
        hits.reverse();
        EgressRedaction { text: out, hits }
    }
}

fn overlaps_any(start: usize, end: usize, taken: &[(usize, usize)]) -> bool {
    taken
        .iter()
        .any(|&(taken_start, taken_end)| start < taken_end && taken_start < end)
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// 32 字节会话随机密钥 (uuid v4 熵源 × 2, 零新外部依赖)。
fn fresh_session_key() -> [u8; 32] {
    let first = SessionId::new().as_uuid();
    let second = SessionId::new().as_uuid();
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(first.as_bytes());
    key[16..].copy_from_slice(second.as_bytes());
    key
}

// ---------------------------------------------------------------------------
// 变体形态 (只对已登记值展开; 边界见模块文档「已知边界」)
// ---------------------------------------------------------------------------

/// 一个真值在文本中的全部变体形态命中区间 (byte offsets, 可重叠, 未去重)。
fn variant_ranges(text: &str, value: &str) -> Vec<(usize, usize)> {
    let mut ranges = substring_ranges(text, value);
    if value.len() < MIN_VARIANT_VALUE_BYTES {
        return ranges;
    }
    let value_bytes = value.as_bytes();
    for needle in [
        base64_std(value_bytes),
        base64_std_nopad(value_bytes),
        percent_encode(value_bytes),
        percent_encode_lower(value_bytes),
        hex_lower(value_bytes),
        hex_upper(value_bytes),
    ] {
        if needle.len() >= MIN_VARIANT_VALUE_BYTES {
            ranges.extend(substring_ranges(text, &needle));
        }
    }
    ranges.extend(split_form_ranges(text, value));
    ranges
}

fn substring_ranges(text: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    text.match_indices(needle)
        .map(|(start, matched)| (start, start + matched.len()))
        .collect()
}

/// V5 拆分串: 真值字符间插空白/零宽字符 —— 归一影子串扫描 (两侧同规则归一)。
fn split_form_ranges(text: &str, value: &str) -> Vec<(usize, usize)> {
    let shadow = NormalizedShadow::new(text);
    let needle: String = value.chars().filter(|c| !is_soft_separator(*c)).collect();
    if needle.len() < MIN_VARIANT_VALUE_BYTES {
        return Vec::new();
    }
    shadow
        .norm
        .match_indices(&needle)
        .filter_map(|(start, matched)| {
            let end = start + matched.len();
            let orig_start = *shadow.orig_start.get(start)?;
            let orig_end = *shadow.orig_end.get(end - 1)?;
            Some((orig_start, orig_end))
        })
        .collect()
}

/// 归一影子串: 去掉软分隔符后的文本 + 每个归一字节到原文字节区间的映射。
struct NormalizedShadow {
    norm: String,
    /// norm 第 i 字节在原文中的字符起始字节偏移。
    orig_start: Vec<usize>,
    /// norm 第 i 字节所属原文字符的结束字节偏移。
    orig_end: Vec<usize>,
}

impl NormalizedShadow {
    fn new(text: &str) -> Self {
        let mut norm = String::with_capacity(text.len());
        let mut orig_start = Vec::with_capacity(text.len());
        let mut orig_end = Vec::with_capacity(text.len());
        for (index, character) in text.char_indices() {
            if is_soft_separator(character) {
                continue;
            }
            let char_end = index + character.len_utf8();
            for _ in 0..character.len_utf8() {
                norm.push(character);
                orig_start.push(index);
                orig_end.push(char_end);
            }
        }
        Self {
            norm,
            orig_start,
            orig_end,
        }
    }
}

/// 软分隔符: 空白族 + 零宽族 (拆分串绕过常用的插入字符)。
fn is_soft_separator(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
        )
}

fn base64_std(bytes: &[u8]) -> String {
    let mut out = base64_std_nopad(bytes);
    while out.len() % 4 != 0 {
        out.push('=');
    }
    out
}

fn base64_std_nopad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(char::from(ALPHABET[(triple >> 18) as usize & 0x3F]));
        out.push(char::from(ALPHABET[(triple >> 12) as usize & 0x3F]));
        if chunk.len() > 1 {
            out.push(char::from(ALPHABET[(triple >> 6) as usize & 0x3F]));
        }
        if chunk.len() > 2 {
            out.push(char::from(ALPHABET[triple as usize & 0x3F]));
        }
    }
    out
}

fn percent_encode(bytes: &[u8]) -> String {
    percent_encode_with(bytes, |byte| format!("%{byte:02X}"))
}

fn percent_encode_lower(bytes: &[u8]) -> String {
    percent_encode_with(bytes, |byte| format!("%{byte:02x}"))
}

fn percent_encode_with(bytes: &[u8], encode: impl Fn(u8) -> String) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&encode(byte));
        }
    }
    out
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_governance::input_security::{PiiDetector, RedactionPlaceholder};

    const SECRET: &str = "hunter2-plain-value";
    const EMAIL: &str = "test.user@example.com";
    const CREDENTIAL: &str = "sk-1234567890abcdef1234567890";

    fn bridge(key_seed: u8) -> RedactionBridge {
        RedactionBridge::with_session_key([key_seed; 32])
    }

    /// 占位确定性: 同会话同值恒同占位, 脱敏幂等 (输出是固定点)。
    #[test]
    fn placeholder_determinism_same_value_same_alias_and_fixed_point() {
        let mut bridge = bridge(1);
        let first = bridge.redact_for_egress(&format!("a {EMAIL} b"));
        let second = bridge.redact_for_egress(&format!("c {EMAIL} d"));
        let alias = format!("__AP_EMAIL_{}__", session_fingerprint(&[1; 32], EMAIL));
        assert_eq!(first.text, format!("a {alias} b"));
        assert_eq!(second.text, format!("c {alias} d"));
        // 幂等: 对已脱敏文本再扫一遍是固定点。
        let again = bridge.redact_for_egress(&first.text);
        assert_eq!(again.text, first.text, "脱敏输出必须是固定点");
        assert!(
            again.hits.is_empty(),
            "固定点不应再报命中: {:?}",
            again.hits
        );
    }

    /// 占位确定性: 跨会话换别名, 且别名互不还原。
    #[test]
    fn placeholder_aliases_differ_across_sessions() {
        let mut session_a = bridge(1);
        let mut session_b = bridge(2);
        let token_a = session_a.register_value(PiiKind::Email, EMAIL);
        let token_b = session_b.register_value(PiiKind::Email, EMAIL);
        assert_ne!(token_a, token_b, "跨会话必须换别名");
        // 跨会话登记簿互不认账。
        let report = session_b.restore_for_local(&token_a.render());
        assert_eq!(report.text, token_a.render());
        assert_eq!(report.unrestored, vec![token_a.fingerprint.clone()]);
    }

    /// 不可逆: 占位符只携带 类别 + 12 位截断指纹, 无登记簿/密钥无法反推真值。
    #[test]
    fn placeholder_is_irreversible_without_registry() {
        let mut owner = bridge(1);
        let token = owner.register_value(PiiKind::CredentialKey, CREDENTIAL);
        let rendered = token.render();
        assert!(!rendered.contains(CREDENTIAL), "{rendered}");
        assert_eq!(
            RedactionPlaceholder::parse(&rendered).map(|p| (p.kind, p.fingerprint)),
            Some((PiiKind::CredentialKey, token.fingerprint.clone()))
        );
        // 全新会话域 (无登记簿) 看着同一个占位符也还原不出真值。
        let mut fresh = bridge(9);
        let report = fresh.restore_for_local(&rendered);
        assert_eq!(report.text, rendered);
        assert!(report.restored.is_empty());
    }

    /// 还原双闸: 已登记还原, 未登记不还原照实留痕。
    #[test]
    fn restore_double_gate_registered_restores_unregistered_traced() {
        let mut bridge = bridge(3);
        let known = bridge.register_value(PiiKind::EnvSecret, SECRET);
        let unknown = RedactionPlaceholder::new(PiiKind::Email, "deadbeef1234");
        let text = format!("known {} unknown {}", known.render(), unknown.render());
        let report = bridge.restore_for_local(&text);
        assert_eq!(
            report.text,
            format!("known {SECRET} unknown {}", unknown.render()),
            "未登记占位符必须照实保留"
        );
        assert_eq!(report.restored, vec![known.fingerprint.clone()]);
        assert_eq!(report.unrestored, vec![unknown.fingerprint.clone()]);
        // 照实留痕: 账本记 skipped 且还原与否=false。
        let skipped: Vec<&LedgerEntry> = bridge
            .ledger_entries()
            .iter()
            .filter(|entry| entry.event == LedgerEvent::RestoreSkippedUnregistered)
            .collect();
        assert_eq!(skipped.len(), 1);
        assert!(!skipped[0].restored);
        assert_eq!(skipped[0].fingerprint, unknown.fingerprint);
    }

    /// 双闸之一: 出境面永不吐真值, 还原只在受控回程; 往返保真。
    #[test]
    fn restore_happens_only_on_local_surface_with_exact_roundtrip() {
        let mut bridge = bridge(4);
        let original = format!("Contact {EMAIL}; key {CREDENTIAL}; tel 13812345678.");
        let egress = bridge.redact_for_egress(&original);
        assert!(!egress.text.contains(EMAIL), "{}", egress.text);
        assert!(!egress.text.contains(CREDENTIAL), "{}", egress.text);
        assert!(!egress.text.contains("13812345678"), "{}", egress.text);
        // 受控回程还原 = 整段保真往返。
        let report = bridge.restore_for_local(&egress.text);
        assert_eq!(report.text, original, "登记值往返必须逐字保真");
        assert_eq!(report.restored.len(), 3);
        assert!(report.unrestored.is_empty());
    }

    /// 历史回扫: 早年留下的明文真值面在每次出境前被拦 (堵历史搭车)。
    #[test]
    fn history_rescan_blocks_legacy_plaintext_before_every_egress() {
        let mut bridge = bridge(5);
        let mut history = vec![
            "2026-01-01 tool output: 联系 test.user@example.com".to_string(),
            "2026-02-02 tool output: key sk-1234567890abcdef1234567890".to_string(),
        ];
        let report = bridge.rescan_history(&mut history);
        assert_eq!(report.segments_swept, 2);
        assert!(report
            .hits
            .iter()
            .all(|hit| hit.surface == RedactionSurface::HistoryRescan));
        for segment in &history {
            assert!(!segment.contains(EMAIL), "{segment}");
            assert!(!segment.contains(CREDENTIAL), "{segment}");
        }
        assert!(history[0].contains("__AP_EMAIL_"), "{}", history[0]);
        assert!(
            history[1].contains("__AP_CREDENTIAL_KEY_"),
            "{}",
            history[1]
        );
        // 每次出境前都回扫: 重复回扫幂等, 明文永不复活。
        let before = history.clone();
        bridge.rescan_history(&mut history);
        assert_eq!(history, before);
    }

    /// 变体绕过 (编码): base64 / 百分号 / 十六进制形态的历史搭车全部被拦。
    #[test]
    fn encoding_variant_bypass_defended_in_history() {
        let mut bridge = bridge(6);
        let token = bridge.register_value(PiiKind::EnvSecret, SECRET);
        let bytes = SECRET.as_bytes();
        let smuggled = format!(
            "b64 {} b64np {} pct {} pctl {} hex {} hexu {}",
            base64_std(bytes),
            base64_std_nopad(bytes),
            percent_encode(bytes),
            percent_encode_lower(bytes),
            hex_lower(bytes),
            hex_upper(bytes),
        );
        let mut history = vec![smuggled];
        bridge.rescan_history(&mut history);
        for encoded in [
            base64_std(bytes),
            base64_std_nopad(bytes),
            percent_encode(bytes),
            percent_encode_lower(bytes),
            hex_lower(bytes),
            hex_upper(bytes),
        ] {
            assert!(!history[0].contains(&encoded), "编码变体漏网: {encoded}");
        }
        assert_eq!(
            history[0].matches(&token.render()).count(),
            6,
            "{}",
            history[0]
        );
    }

    /// 变体绕过 (拆分串): 字符间插空白/零宽字符的真值同样被拦。
    #[test]
    fn split_string_bypass_defended_in_history() {
        let mut bridge = bridge(7);
        let token = bridge.register_value(PiiKind::CredentialKey, CREDENTIAL);
        let spaced = CREDENTIAL
            .chars()
            .map(|c| format!("{c} "))
            .collect::<String>()
            .trim()
            .to_string();
        let zero_width = CREDENTIAL.replace("sk-", "sk\u{200B}-\u{200B}");
        let mut history = vec![format!("split [{spaced}] zero-width [{zero_width}]")];
        bridge.rescan_history(&mut history);
        assert!(!history[0].contains(&spaced), "{}", history[0]);
        assert!(!history[0].contains(&zero_width), "{}", history[0]);
        assert_eq!(
            history[0].matches(&token.render()).count(),
            2,
            "{}",
            history[0]
        );
    }

    /// 变体形态辅助函数对齐公共向量 (base64 RFC 4648)。
    #[test]
    fn variant_encoding_helpers_match_known_vectors() {
        assert_eq!(base64_std(b"Man"), "TWFu");
        assert_eq!(base64_std(b"Ma"), "TWE=");
        assert_eq!(base64_std_nopad(b"Man"), "TWFu");
        assert_eq!(percent_encode(b"a@b"), "a%40b");
        assert_eq!(hex_lower(b"\xab\xcd"), "abcd");
        assert_eq!(hex_upper(b"\xab\xcd"), "ABCD");
    }

    /// 脱敏账本: 类别/指纹/时间/还原与否齐全, 零真值入账。
    #[test]
    fn ledger_records_events_with_zero_true_values() {
        let mut bridge = bridge(8);
        let token = bridge.register_value(PiiKind::EnvSecret, SECRET);
        let egress = bridge.redact_for_egress(&format!("mail {EMAIL}"));
        bridge.restore_for_local(&format!("{} {}", egress.text, token.render()));
        bridge.restore_for_local("__AP_EMAIL_deadbeef1234__");

        let jsonl = bridge.ledger_jsonl();
        for needle in [SECRET, EMAIL, CREDENTIAL] {
            assert!(!jsonl.contains(needle), "账本零真值: {needle}\n{jsonl}");
        }
        let mut events = Vec::new();
        for line in jsonl.lines() {
            let entry: serde_json::Value =
                serde_json::from_str(line).unwrap_or_else(|e| panic!("JSONL 行: {e}"));
            for key in [
                "ts_unix_ms",
                "category",
                "fingerprint",
                "event",
                "restored",
                "surface",
            ] {
                assert!(entry.get(key).is_some(), "缺字段 {key}: {line}");
            }
            events.push(entry["event"].clone());
        }
        assert!(events.contains(&serde_json::json!("redacted")));
        assert!(events.contains(&serde_json::json!("restored")));
        assert!(events.contains(&serde_json::json!("restore_skipped_unregistered")));
    }

    /// 账本落盘: redaction.jsonl 追加写, 零真值, 重复持久化逐条追加。
    #[test]
    fn ledger_persists_to_redaction_jsonl_without_plaintext() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(REDACTION_LEDGER_FILE);
        let mut bridge = bridge(10);
        bridge.register_value(PiiKind::EnvSecret, SECRET);
        let entry_count = bridge.ledger_entries().len();
        bridge.persist_ledger(&path).expect("persist");
        bridge.persist_ledger(&path).expect("persist");
        let written = std::fs::read_to_string(&path).expect("read ledger");
        assert_eq!(written.lines().count(), entry_count * 2);
        assert!(!written.contains(SECRET), "账本零真值");
    }

    /// 与既有脱敏共存: 单向 `[REDACTED_*]` 标签不属占位族, 永不还原, 零回归。
    #[test]
    fn coexists_with_legacy_one_way_redaction() {
        let legacy = PiiDetector::redact(&format!("mail {EMAIL}"));
        assert!(legacy.contains("[REDACTED_EMAIL]"), "{legacy}");
        let mut bridge = bridge(11);
        let egress = bridge.redact_for_egress(&legacy);
        assert!(egress.text.contains("[REDACTED_EMAIL]"), "{}", egress.text);
        let report = bridge.restore_for_local(&egress.text);
        assert!(
            report.text.contains("[REDACTED_EMAIL]"),
            "单向标签永不可还原: {}",
            report.text
        );
        assert!(!report.text.contains(EMAIL), "{}", report.text);
    }

    /// 既有脱敏输出直接经双向闸往返也不丢不改 (只换真值面, 不动单向标签)。
    #[test]
    fn legacy_redaction_labels_pass_through_bridge_unchanged() {
        let mut bridge = bridge(12);
        let input = "key sk-1234567890abcdef1234567890 gone -> [REDACTED_CREDENTIAL] stays";
        let egress = bridge.redact_for_egress(input);
        assert!(
            egress.text.ends_with("-> [REDACTED_CREDENTIAL] stays"),
            "{}",
            egress.text
        );
    }

    /// 显式登记: 同值恒同占位, 后续出境裸形态换同一占位。
    #[test]
    fn explicit_registration_preseeds_alias_for_later_egress() {
        let mut bridge = bridge(13);
        let first = bridge.register_value(PiiKind::EnvSecret, SECRET);
        let second = bridge.register_value(PiiKind::EnvSecret, SECRET);
        assert_eq!(first, second);
        let egress = bridge.redact_for_egress(&format!("run echo {SECRET}"));
        assert_eq!(egress.text, format!("run echo {}", first.render()));
        assert_eq!(bridge.registered_values(), 1);
    }

    /// 零明文留痕: Debug 面 (桥/还原报告/登记簿) 永不打印真值。
    #[test]
    fn debug_output_never_leaks_true_values() {
        let mut bridge = bridge(14);
        let token = bridge.register_value(PiiKind::Email, EMAIL);
        let report = bridge.restore_for_local(&token.render());
        let debug = format!("{bridge:?} {report:?}");
        assert!(!debug.contains(EMAIL), "{debug}");
        assert!(!debug.contains('@'), "{debug}");
        assert!(debug.contains("RestoreReport"), "{debug}");
    }

    /// 出境双层扫 (guardrail 接线面): 占位族 + 既有绊线单向标签共存。
    #[test]
    fn guardrail_combined_sweep_covers_both_families() {
        use crate::guardrail::ToolGuardrail;
        let mut bridge = bridge(15);
        let raw = format!(
            "mail {EMAIL}\npassword = {SECRET}\n-----BEGIN RSA PRIVATE KEY----- MIIEowIBAAKCAQEA0... -----END RSA PRIVATE KEY-----"
        );
        let result = ToolGuardrail::sweep_output_for_egress(&raw, &mut bridge);
        let final_text = &result.tripwire.sanitized_output;
        assert!(final_text.contains("__AP_EMAIL_"), "{final_text}");
        assert!(final_text.contains("__AP_ENV_SECRET_"), "{final_text}");
        assert!(
            final_text.contains("[REDACTED_PEM_PRIVATE_KEY]"),
            "{final_text}"
        );
        assert!(!final_text.contains(EMAIL), "{final_text}");
        assert!(!final_text.contains(SECRET), "{final_text}");
        assert!(result
            .tripwire
            .leaked_kinds
            .contains(&crate::guardrail::LeakedCredentialKind::PemPrivateKey));
        assert!(!result.redaction.hits.is_empty());
        // 受控回程: 占位族可还原, 单向标签保持脱敏。
        let report = bridge.restore_for_local(final_text);
        assert!(report.text.contains(EMAIL), "{}", report.text);
        assert!(
            !report.text.contains("MIIEowIBAAKCAQEA0"),
            "{}",
            report.text
        );
    }

    /// FEELING 情感隐私类别 (拍板 ㉓): 识别=登记簿驱动 —— 词面命中不脱敏;
    /// 显式登记后换 `__AP_FEELING_<指纹12>__`, 受控回程按登记簿还原往返。
    #[test]
    fn feeling_is_registry_driven_and_roundtrips_through_the_bridge() {
        const FEELING_VALUE: &str = "背刺";
        let mut bridge = bridge(16);
        let spoken = "我恨背刺, 真的恨";
        // 未登记 = 不识别、不脱敏 (词面只登记不强拦, 文本原样)。
        let untouched = bridge.redact_for_egress(spoken);
        assert_eq!(untouched.text, spoken);
        assert!(untouched.hits.is_empty(), "{:?}", untouched.hits);
        // 显式登记 (调用方标注 FEELING) → 出境换 FEELING 占位。
        let token = bridge.register_feeling(FEELING_VALUE);
        assert_eq!(token.kind, PiiKind::Feeling);
        assert!(
            token.render().starts_with("__AP_FEELING_"),
            "{}",
            token.render()
        );
        let egress = bridge.redact_for_egress(spoken);
        assert!(!egress.text.contains(FEELING_VALUE), "{}", egress.text);
        assert!(egress.text.contains(&token.render()), "{}", egress.text);
        assert!(egress.hits.iter().all(|hit| hit.kind == PiiKind::Feeling));
        // 受控回程往返: 登记簿命中 → 还原真值。
        let report = bridge.restore_for_local(&egress.text);
        assert_eq!(report.text, spoken);
        assert_eq!(report.restored, vec![token.fingerprint.clone()]);
    }
}
