//! `apeireth-memory::injection_renderer` — 注入契约（设计稿 §5 ③）。
//!
//! # 这个契约解决什么
//!
//! **检索语义不变，输出形态可换。**
//! 今天的注入形态是「一段带反幻觉规则的文本」（`memory_injection`），
//! 但**算法**（如 D4 的模式层）可能需要另一种形态：
//! 「只给结论不给原文」「结构化成条目」「按层级分组」…
//! ⇒ 契约要**窄于今天的实现、宽于今天的形态**。
//!
//! # 契约的三块（窄在证据、宽在形态）
//!
//! - [`Evidence`]：**注入的最小单位**（内容 + 出处 + 相关度）—— 刻意比检索侧的
//!   `MemoryCandidate` 窄（`layer` / `scope` / `provenance` 全套是**检索侧**的事）；
//! - [`Budget`]：注入预算（**字符数**，不是 token 数 —— 本仓没有分词器，
//!   用字符数是**可复现**的口径）；
//! - [`Injection`]：渲染产物（**枚举** ⇒ 形态可换，不是一个固定字符串）。
//!
//! # 预算纪律（三条铁律，三个渲染器**共用一个装配器** ⇒ 不会各写各的、不会漂移）
//!
//! 1. **按 `relevance` 降序装入**（同分按 `id` 升序）⇒ **确定性**（同输入两次逐字相等）；
//! 2. **装到预算就停** ⇒ 产物**永远**不超预算（`Injection::char_len() ≤ budget.max_chars`）；
//! 3. **截断必须如实标注** ⇒ 被截断的内容带「…（已截断）」，**不静默截断**。
//!
//! ⚠️ **装不下就说装不下**：预算装不下时返回 [`Injection::Empty`]（带原因），
//! **不许硬塞超预算**（超预算 = 上下文被撑爆 = 上游的预算口径失效）。
//! 唯一例外：**第一条**证据就放不下 ⇒ `Empty { reason: "第一条证据就超出预算" }`
//! （此时「装不下」比「装一条残缺的」更诚实：残缺条目会被模型当成完整事实）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、契约、三个可换实现与测试齐备（**库级**实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0** 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在单元/集成测试里跑过，未在真实记忆库上验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::cmp::{Ordering, Reverse};
use std::collections::BTreeMap;

/// 一条证据（**注入的最小单位**）。
///
/// ⚠️ **刻意比 `MemoryCandidate` 窄**：注入只需要「内容 + 出处 + 一点权重」，
/// 不需要 `layer` / `scope` / `provenance` 全套（那些是**检索侧**的事）。
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    /// 记录 id（**可追溯**：模型看到的东西要能指回原文）。
    pub id: String,
    /// 证据正文（**渲染器拿它做形态变换**）。
    pub text: String,
    /// 出处说明（如「用户原话」「2026-10-25 的提交」「检索命中」）。
    pub attribution: String,
    /// 相关度（0.0 ~ 1.0；出界 clamp，不 panic）。
    pub relevance: f64,
}

impl Evidence {
    /// 构造（`relevance` 出界 clamp：1.7 ⇒ 1.0、-0.3 ⇒ 0.0、NaN ⇒ 0.0）。
    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        attribution: impl Into<String>,
        relevance: f64,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            attribution: attribution.into(),
            relevance: clamp_unit(relevance),
        }
    }
}

/// 注入预算（**字符数**，不是 token 数 —— 本仓没有分词器，
/// 用字符数是**可复现**的口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// 最多可用字符数。
    pub max_chars: usize,
    /// 最多几条证据（`0` = 不限条数，只受字符预算约束）。
    pub max_items: usize,
}

impl Budget {
    /// 构造（`max_items = 0` ⇒ 不限条数）。
    pub fn new(max_chars: usize, max_items: usize) -> Self {
        Self {
            max_chars,
            max_items,
        }
    }
}

/// 渲染产物（**形态可换** ⇒ 用枚举，不是一个固定字符串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Injection {
    /// 一段纯文本（今天的形态）。
    Plain { text: String },
    /// 结构化条目（每条一行「内容 —— 出处」）。
    Bulleted { lines: Vec<String> },
    /// 只给结论（**不贴原文** ⇒ 适合「算法已经提炼过」的场景）。
    Conclusions { items: Vec<String> },
    /// 渲染器拒绝渲染（如预算为 0、或没有证据）。
    Empty { reason: String },
}

impl Injection {
    /// 渲染出的**字符数**（用来核对「有没有超预算」）。
    ///
    /// # 口径（**三个形态同一口径** ⇒ 体量可比）
    ///
    /// - [`Injection::Plain`]：文本长度（行间的 `\n` 也在里面）；
    /// - [`Injection::Bulleted`] / [`Injection::Conclusions`]：各条目长度之和
    ///   **加条目之间的换行分隔**（按 `\n` 连接计，与 `Plain` 一致）；
    /// - [`Injection::Empty`]：**0**（没有注入任何内容；`reason` 是诊断文本，
    ///   不占注入预算）。
    #[must_use]
    pub fn char_len(&self) -> usize {
        match self {
            Self::Plain { text } => text.chars().count(),
            Self::Bulleted { lines } => joined_len(lines),
            Self::Conclusions { items } => joined_len(items),
            Self::Empty { .. } => 0,
        }
    }

    /// 渲染出的**条目数**（**证据条目**，不是文本行数）。
    ///
    /// ⚠️ `Plain` 的**末尾反幻觉提示行不算证据条目**（它是规则，不是一条记忆）
    /// ⇒ 口径见 [`plain_item_count`]。
    #[must_use]
    pub fn item_count(&self) -> usize {
        match self {
            Self::Plain { text } => plain_item_count(text),
            Self::Bulleted { lines } => lines.len(),
            Self::Conclusions { items } => items.len(),
            Self::Empty { .. } => 0,
        }
    }
}

/// 注入契约：**检索语义不变，输出形态可换**。
pub trait InjectionRenderer {
    /// 渲染一批证据。
    fn render(&self, evidence: &[Evidence], budget: Budget) -> Injection;
}

// ============================================
// 三个可换的渲染器（证明「形态可换」不是空话）
// ============================================

/// 反幻觉提示（[`PlainRenderer`] 默认形态的尾巴）。
///
/// 口径沿用今天的注入（`memory_injection`）：**闭世界** ——
/// 说话只能基于注入条目，不许声称记得别的东西。
pub const ANTI_HALLUCINATION_NOTE: &str =
    "规则: 说话只能基于以上条目; 不确定就说「我猜」; 禁止说「我记得我们以前聊过」— 那是编造。";

/// 截断标注（**如实告知**：被截断的内容必须带这个标记，不静默截断）。
pub const TRUNCATION_MARKER: &str = "…（已截断）";

/// 纯文本形态（今天的形态）：每条证据一行「`[出处] 内容`」，末尾可带反幻觉提示。
///
/// # 为什么提示是**开关**而不是写死
///
/// 今天的注入（`memory_injection`）**总是**带反幻觉规则；但「只给结论」这类形态
/// 往往由**算法已经提炼过**的内容组成 ⇒ 再套一层规则是浪费预算。
/// ⇒ 策略留在**渲染器**上，契约只承诺「产物长什么样」——**形态可换 ⇒ 策略也可换**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlainRenderer {
    /// 是否在末尾追加反幻觉提示。
    include_note: bool,
}

impl Default for PlainRenderer {
    /// 默认形态 = [`PlainRenderer::new`]（**带**提示，与今天的注入一致）。
    fn default() -> Self {
        Self::new()
    }
}

impl PlainRenderer {
    /// 默认形态：**带**反幻觉提示（与今天的注入一致）。
    #[must_use]
    pub fn new() -> Self {
        Self { include_note: true }
    }

    /// 不带反幻觉提示（纯文本形态）。
    #[must_use]
    pub fn without_note() -> Self {
        Self {
            include_note: false,
        }
    }
}

impl InjectionRenderer for PlainRenderer {
    fn render(&self, evidence: &[Evidence], budget: Budget) -> Injection {
        // 提示放**末尾** ⇒ 先把它的空间预留出来（否则截断到最后才发现装不下）。
        let note_cost = if self.include_note {
            ANTI_HALLUCINATION_NOTE.chars().count() + 1
        } else {
            0
        };
        // ⚠️ 提示**整体**装不下 ⇒ **整体省略**（**不截断提示本身**：
        // 半条规则比没规则更危险 —— 模型可能照着半句编）。
        let include_note = self.include_note && note_cost <= budget.max_chars;
        let reserved = if include_note { note_cost } else { 0 };

        match assemble(&rank_by_relevance(evidence), budget, reserved, plain_line) {
            Ok(items) => {
                let mut text = items.join("\n");
                if include_note {
                    text.push('\n');
                    text.push_str(ANTI_HALLUCINATION_NOTE);
                }
                Injection::Plain { text }
            }
            Err(failure) => Injection::Empty {
                reason: failure.reason().to_string(),
            },
        }
    }
}

/// **生产形态**：编号证据清单 + 反幻觉规则（**逐字复刻今天的注入文本**）。
///
/// # 为什么要有这个渲染器（2026-10-30 · B2 接线）
///
/// 设计稿 §5 ③ 把注入上提为契约（[`InjectionRenderer`]），但**生产路径没走它**：
/// `context_compiler` 直接调 `memory_injection::build_memory_injection_with_budget`
/// ⇒ 契约是「库级」的，不是「生产级」的。
///
/// ⇒ 本渲染器 = **契约与生产之间的那层适配**，且必须**逐字等价**于今天的输出
/// ⇒ ⇒ 接线当天**行为零变化**（有测试钉住，见 `tests/injection_renderer_production.rs`）。
///
/// # 与 [`PlainRenderer`] 的三处**实测差异**（别混用）
///
/// | | 生产 / 本渲染器 | [`PlainRenderer`] |
/// |---|---|---|
/// | 标题行 | **有**（`[记忆证据 — …]`）| 无 |
/// | 条目行 | `1. 内容`（**编号**）| `[出处] 内容` |
/// | 规则措辞 | 「以上**编号**条目」 | 「以上条目」 |
///
/// # ⚠️ **不重排**（与其它三个渲染器不同）
///
/// [`PlainRenderer`] 等按 `relevance` 重排（`rank_by_relevance`）；
/// 而**生产顺序 = 检索顺序**（排序由检索层负责，渲染层再排一次会**改掉检索的语义**）。
/// ⇒ ⇒ 本渲染器**保持输入顺序** ⇒ 这是它能逐字等价的前提之一。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberedEvidenceRenderer {
    /// 是否在末尾追加反幻觉规则（生产默认**带**）。
    include_note: bool,
    /// **单条**证据的字符上限（生产默认 `120` = `EVIDENCE_MAX_CHARS`）。
    per_entry_chars: usize,
}

impl Default for NumberedEvidenceRenderer {
    /// 默认 = 与今天的生产注入**逐字一致**（带规则 · 单条 120 字）。
    fn default() -> Self {
        Self::new()
    }
}

impl NumberedEvidenceRenderer {
    /// 默认形态：带规则 · 单条 120 字（**与今天逐字一致**）。
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_note: true,
            per_entry_chars: crate::memory_injection::EVIDENCE_MAX_CHARS,
        }
    }

    /// 不带反幻觉规则。
    #[must_use]
    pub fn without_note() -> Self {
        Self {
            include_note: false,
            per_entry_chars: crate::memory_injection::EVIDENCE_MAX_CHARS,
        }
    }

    /// 指定**单条**证据的字符上限。
    ///
    /// ⚠️ `0` ⇒ **截成空串**（与生产 `chars().take(0)` **逐字一致**）
    /// ⇒ 「不截断」请用 `usize::MAX`。
    #[must_use]
    pub fn with_per_entry_chars(mut self, per_entry_chars: usize) -> Self {
        self.per_entry_chars = per_entry_chars;
        self
    }
}

impl InjectionRenderer for NumberedEvidenceRenderer {
    fn render(&self, evidence: &[Evidence], budget: Budget) -> Injection {
        if evidence.is_empty() {
            return Injection::Empty {
                reason: "没有证据".to_string(),
            };
        }
        if budget.max_chars == 0 {
            return Injection::Empty {
                reason: "预算为 0".to_string(),
            };
        }
        // ⚠️ **不自行裁剪到预算**：裁剪要连带回退「选中回执」
        // （`selected_candidate_ids` / `selected_attribution`）
        // ⇒ 那是**调用方**的事（它才知道哪几行是记忆正文行）
        // ⇒ 本渲染器只负责**逐字复刻格式**。
        let mut text = String::from(EVIDENCE_BLOCK_HEADER);
        for (index, item) in evidence.iter().enumerate() {
            // ⚠️ `per_entry_chars = 0` ⇒ **截成空串**（与生产 `chars().take(0)` 一致）
            // ⇒ **不是**「不截断」⇒ 「不截断」用 `usize::MAX`。
            let body = item
                .text
                .chars()
                .take(self.per_entry_chars)
                .collect::<String>();
            text.push_str(&format!("{}. {}\n", index + 1, body));
        }
        if self.include_note {
            text.push_str(NUMBERED_RULE_NOTE);
        }
        Injection::Plain { text }
    }
}

/// 生产注入的**标题行**（逐字复刻 `memory_injection::build_memory_injection_with_budget`）。
pub const EVIDENCE_BLOCK_HEADER: &str =
    "[记忆证据 — 你只知道以下条目, 不要声称记得列表之外的任何对话]\n";

/// 生产注入的**反幻觉规则**（逐字复刻）。
///
/// ⚠️ 与 [`ANTI_HALLUCINATION_NOTE`] **差一个「编号」二字**
/// ⇒ 生产文本说的是「以上**编号**条目」（因为条目确实带编号）
/// ⇒ ⇒ 两个常量**必须分开**，否则会把今天的文本改掉。
pub const NUMBERED_RULE_NOTE: &str =
    "规则: 说话只能基于以上编号条目; 不确定就说「我猜」; 禁止说「我记得我们以前聊过」— 那是编造。";

/// 结构化条目形态：每条一行「内容 —— 出处」。
///
/// # 为什么行里**不带**项目符号
///
/// 形态名承诺的是「**每条一行**」这个结构，不承诺具体符号
/// ⇒ 调用方可以自己加编号 / 圆点 / 缩进（**形态可换**是本契约的主张，
/// 渲染器不该把符号焊死在内容里）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BulletedRenderer;

impl InjectionRenderer for BulletedRenderer {
    fn render(&self, evidence: &[Evidence], budget: Budget) -> Injection {
        match assemble(&rank_by_relevance(evidence), budget, 0, bulleted_line) {
            Ok(lines) => Injection::Bulleted { lines },
            Err(failure) => Injection::Empty {
                reason: failure.reason().to_string(),
            },
        }
    }
}

/// 只给结论的形态：每条证据**取首句**（不贴全文）⇒ 同预算下能装**更多条**。
///
/// # 为什么是「首句」而不是「摘要」
///
/// 契约层**不做语义提炼**（提炼是算法的事，见 D4 的模式层）：
/// 首句是**纯字符串操作** ⇒ 确定性、可复现、零依赖；
/// 真要摘要 ⇒ 换一个渲染器实现即可（**形态可换**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConclusionsRenderer;

impl InjectionRenderer for ConclusionsRenderer {
    fn render(&self, evidence: &[Evidence], budget: Budget) -> Injection {
        match assemble(&rank_by_relevance(evidence), budget, 0, conclusion_line) {
            Ok(items) => Injection::Conclusions { items },
            Err(failure) => Injection::Empty {
                reason: failure.reason().to_string(),
            },
        }
    }
}

// ============================================
// 装配（三个渲染器共用 ⇒ 预算纪律只有一份实现）
// ============================================

/// 装配失败的**种类**（各渲染器把它翻成自己的话术）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssemblyFailure {
    /// 没有证据。
    NoEvidence,
    /// 预算为 0。
    ZeroBudget,
    /// 固定部分（反幻觉提示）占满了预算 ⇒ 装不下证据。
    ReservedBudget,
    /// 第一条证据就超出预算。
    FirstItemTooLong,
}

impl AssemblyFailure {
    /// 失败原因（**如实说装不下**，不静默降级）。
    fn reason(&self) -> &'static str {
        match self {
            Self::NoEvidence => "没有证据可渲染",
            Self::ZeroBudget => "预算为 0，装不下任何证据",
            Self::ReservedBudget => "预算已被反幻觉提示占满，装不下证据",
            Self::FirstItemTooLong => "第一条证据就超出预算",
        }
    }
}

/// 通用装配器：把**已排序**的证据按预算装进产物。
///
/// # 三条纪律落在**这一个函数**里
///
/// 1. 按序装入，装不下就**停**（不跳过、不硬塞）；
/// 2. 第一条就装不下 ⇒ **如实说装不下**（[`AssemblyFailure::FirstItemTooLong`]），
///    不硬塞超预算（超预算 = 上下文被撑爆 = 上游预算口径失效）；
/// 3. 中途装不下 ⇒ 把**剩下的空间用满**并标注「已截断」（不静默截断）。
///
/// # 参数
///
/// - `ranked`：**已按相关性排序**的证据（[`rank_by_relevance`]）；
/// - `reserved`：固定部分预留的字符数（如 `Plain` 的反幻觉提示）；
/// - `render_item`：单条证据 → 该形态的文本（**形态差异只在这一个函数里**）。
fn assemble(
    ranked: &[&Evidence],
    budget: Budget,
    reserved: usize,
    render_item: impl Fn(&Evidence) -> String,
) -> Result<Vec<String>, AssemblyFailure> {
    if ranked.is_empty() {
        return Err(AssemblyFailure::NoEvidence);
    }
    if budget.max_chars == 0 {
        return Err(AssemblyFailure::ZeroBudget);
    }
    // 固定部分（提示）已经占满预算 ⇒ 装不下证据。
    let available = budget.max_chars.saturating_sub(reserved);
    if available == 0 {
        return Err(AssemblyFailure::ReservedBudget);
    }

    let mut items: Vec<String> = Vec::new();
    let mut used = 0usize;
    for item in ranked {
        // 条数上限（`0` = 不限条数，只受字符预算约束）。
        if budget.max_items != 0 && items.len() >= budget.max_items {
            break;
        }
        let text = render_item(item);
        if text.is_empty() {
            // 空条目不入产物（例：结论形态下原文为空 ⇒ 不产出一条空行）。
            continue;
        }
        // 条目之间的换行分隔（`\n`）：第一个条目不带。
        let separator = usize::from(!items.is_empty());
        let cost = text.chars().count() + separator;
        if used + cost <= available {
            items.push(text);
            used += cost;
            continue;
        }
        // 装不下这条 ⇒ 该停了。
        if items.is_empty() {
            // 第一条就装不下 ⇒ **如实说装不下**（不硬塞超预算）。
            return Err(AssemblyFailure::FirstItemTooLong);
        }
        // 截断：把剩下的空间**用满**（内容前缀 + 截断标注），不静默截断。
        let remaining = available - used;
        let room = remaining.saturating_sub(separator);
        let marker_len = TRUNCATION_MARKER.chars().count();
        if room < marker_len {
            // 连标注都放不下 ⇒ **放弃这条碎片**（宁可不装，也不超预算、也不静默截断）。
            break;
        }
        let prefix: String = text.chars().take(room - marker_len).collect();
        items.push(format!("{prefix}{TRUNCATION_MARKER}"));
        break;
    }
    Ok(items)
}

// ============================================
// 排序与单条渲染（形态差异都收在这一段）
// ============================================

/// 按 **`relevance` 降序** 装入（同分按 **id 升序**）⇒ **确定性**。
///
/// # 为什么用 [`BTreeMap`] 分桶，而不是直接 `sort`
///
/// ① 桶键是**全序**（[`OrderedRelevance`]）⇒ 同分的证据落进**同一个**桶，
///    不会出现「同分被拆成两堆、顺序反直觉」；
/// ② 桶内按 `id` 升序 ⇒ 同分证据的顺序**只由 id 决定**（与输入顺序无关）
///    ⇒ 同一批证据两次渲染**逐字相等**。
///
/// ⚠️ 排序用的是**构造口径**（[`clamp_unit`] 之后的值）⇒ 直接给 `pub` 字段塞
/// `NaN` / `1.7` 也不会把 NaN 排到最前、也不会让越界值影响名次。
fn rank_by_relevance(evidence: &[Evidence]) -> Vec<&Evidence> {
    let mut buckets: BTreeMap<Reverse<OrderedRelevance>, Vec<&Evidence>> = BTreeMap::new();
    for item in evidence {
        buckets
            .entry(Reverse(OrderedRelevance(clamp_unit(item.relevance))))
            .or_default()
            .push(item);
    }
    let mut ranked = Vec::with_capacity(evidence.len());
    for (_, mut bucket) in buckets {
        bucket.sort_by(|left, right| left.id.cmp(&right.id));
        ranked.extend(bucket);
    }
    ranked
}

/// 相关度的**全序**包装（`f64` 本身不是 `Ord`）。
///
/// `total_cmp` 给出**确定**的全序（连 NaN 也有确定位置）⇒ 排序结果永远可复现。
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedRelevance(f64);

impl Eq for OrderedRelevance {}

impl PartialOrd for OrderedRelevance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedRelevance {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// 纯文本形态的单条渲染：`[出处] 内容`。
fn plain_line(item: &Evidence) -> String {
    format!("[{}] {}", item.attribution, item.text)
}

/// 结构化条目的单条渲染：`内容 —— 出处`。
fn bulleted_line(item: &Evidence) -> String {
    format!("{} —— {}", item.text, item.attribution)
}

/// 结论形态的单条渲染：**首句**（不贴全文）。
fn conclusion_line(item: &Evidence) -> String {
    first_sentence(&item.text)
}

/// 取**首句**（结论形态用）：在第一个句末标点（`。！？!?`）处截断（**含**标点）。
///
/// # 为什么**不含**半角句点 `.`
///
/// 版本号 / 小数 / 缩写里到处是点（`v2.0`、`0.75`、`e.g.`）⇒ 按点断句会把一句
/// 切成半句（"取 v2" 反而丢信息）。中文/英文的句末标点（`。！？!?`）足够可靠。
fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    for (index, ch) in trimmed.char_indices() {
        if matches!(ch, '。' | '！' | '？' | '!' | '?') {
            return trimmed[..index + ch.len_utf8()].trim().to_string();
        }
    }
    trimmed.to_string()
}

// ============================================
// 小工具（纯函数）
// ============================================

/// 把相关度夹到 `[0.0, 1.0]`；**NaN ⇒ 0.0**（不 panic，也不把 NaN 传下去）。
fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        return 0.0;
    }
    let clamped = value.clamp(0.0, 1.0);
    // `-0.0` 归一成 `+0.0`：`-0.0 == 0.0` 成立，但 `total_cmp` 把两者判成不同
    // ⇒ 不归一的话，同分的证据会被分到两个桶里（顺序反直觉）。
    if clamped == 0.0 {
        0.0
    } else {
        clamped
    }
}

/// 一组条目按 `\n` 连接后的字符数（**与 `Plain` 的口径一致** ⇒ 形态之间可比）。
fn joined_len(parts: &[String]) -> usize {
    parts.iter().map(|part| part.chars().count()).sum::<usize>() + parts.len().saturating_sub(1)
}

/// 纯文本形态的**证据条目数**（**不含**末尾的反幻觉提示行）。
///
/// 口径：按 `\n` 数行；**末行恰好等于** [`ANTI_HALLUCINATION_NOTE`] 时扣掉一行
/// （提示是**规则**，不是一条记忆）。
fn plain_item_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut count = text.split('\n').count();
    if text.rsplit('\n').next() == Some(ANTI_HALLUCINATION_NOTE) {
        count = count.saturating_sub(1);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevance_is_clamped_on_construction() {
        assert_eq!(Evidence::new("a", "甲", "用户原话", 1.7).relevance, 1.0);
        assert_eq!(Evidence::new("a", "甲", "用户原话", -0.3).relevance, 0.0);
        assert_eq!(
            Evidence::new("a", "甲", "用户原话", f64::NAN).relevance,
            0.0
        );
    }

    #[test]
    fn first_sentence_stops_at_the_sentence_mark() {
        assert_eq!(first_sentence("第一句。第二句。"), "第一句。");
        assert_eq!(first_sentence("第一句！第二句"), "第一句！");
        // 没有句末标点 ⇒ 整段（不截半句）。
        assert_eq!(first_sentence("没有句号的一段"), "没有句号的一段");
        // 半角句点**不**断句（版本号 / 小数）。
        assert_eq!(first_sentence("版本 v2.0 的说明"), "版本 v2.0 的说明");
    }

    #[test]
    fn ranking_is_deterministic_for_equal_relevance() {
        let evidence = vec![
            Evidence::new("r2", "乙", "出处", 0.5),
            Evidence::new("r1", "甲", "出处", 0.5),
            Evidence::new("r3", "丙", "出处", 0.9),
        ];
        let ranked = rank_by_relevance(&evidence);
        let ids: Vec<&str> = ranked.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, vec!["r3", "r1", "r2"]);
    }
}
