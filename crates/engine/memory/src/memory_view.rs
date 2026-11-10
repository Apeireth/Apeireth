//! `apeireth-memory::memory_view` — 记忆纵深体系的**读侧契约**（设计稿 §5 ①②）。
//!
//! 设计稿：`docs/01-architecture/memory-knowledge-depth-system-spec.md` §5
//! 「三个接口契约（让未来算法插得上）」。
//!
//! # 这个模块解决什么问题
//!
//! owner 的目标之一是「不管未来出了什么算法方案，我们都能够用好用」。
//! 要让**未来的算法插得上**，前提是**接口要比今天的检索窄，比今天的模型宽**：
//! 算法能拿到的东西足够表达它的判断，但**不许直接碰库** —— 碰库 = 绑死存储 = 换不动。
//! 于是本模块只定两个契约：
//!
//! - ① **读契约** [`MemoryView`]：算法**只能**通过它读记录；
//! - ② **写契约**（标注）[`MemoryAnnotation`]：算法**只能**产出标注。
//!
//! # 两条铁律落在**类型**上（不是落在注释上）
//!
//! 1. **权限在契约里，不在算法里** —— 可见性判定由 [`MemoryView::visible_to`] 与
//!    [`scope_visible`] 承担；算法代码里**不该**出现"我能不能看这条"的分支。
//! 2. **算法只能产标注，永不回写原始层** —— [`MemoryAnnotation`] 是**纯数据**：
//!    它**没有**任何写原始记录的方法，本模块也**不提供**"把标注写回库"的函数。
//!    因此"回写"在**类型层面**就不可能发生，而不是靠约定或纪律。
//!
//! # 确定性纪律（为什么值得单独写一段）
//!
//! - 存储与索引一律用 [`BTreeMap`]（**不用** `HashMap`）⇒ 无哈希迭代序依赖；
//! - [`MemoryView::by_time`] 按 **`(timestamp_ms, id)` 升序** ⇒ 同一时间戳也有确定顺序；
//! - [`MemoryView::related`] 按 **id 升序** ⇒ 同一批记录两次调用逐字节相等；
//! - 「标签词」切分（[`tokenize`]）是**纯函数**：CJK bigram + 词，**不引分词依赖**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、契约、内存实现与测试齐备（**库级**实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0** 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在单元/集成测试里跑过，未在真实记忆库上验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::{BTreeMap, BTreeSet};

// ============================================
// ① 读契约：算法只能通过它读，不许直接碰库
// ============================================

/// 记录 id（**新类型**，不与既有 `String` id 混用 ⇒ 将来换存储不用改算法）。
///
/// 为什么要有这层新类型：算法侧如果直接拿 `String` 当 id，将来换存储（换 id 形态）
/// 就得改算法代码；用新类型把「id」这件事圈起来，算法侧只依赖**本契约**。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordId(pub String);

impl RecordId {
    /// 造一个 id（任何能变成 `String` 的东西都行）。
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// 取出 id 文本（只读 ⇒ 不能拿它去反查库）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 一条记录在**读侧**的形态（**刻意比库里的 `Episode` 窄**：
/// 算法只该看到「内容 + 时间 + 出处 + 可见范围」，不该看到内部实现细节）。
///
/// 为什么刻意窄：读侧形态越窄，将来算法越容易被换掉；库里的东西会变，
/// 契约里的东西不能变（设计稿 §5「接口要比今天的检索窄」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// 记录 id。
    pub id: RecordId,
    /// 正文内容。
    pub content: String,
    /// 时间戳（毫秒）。
    pub timestamp_ms: i64,
    /// 出处链（原始来源 → 谁写的 → 何时）。
    pub provenance: ProvenanceChain,
    /// 可见范围（权限**在契约里**，不在算法里）。
    pub scope: Scope,
}

/// 出处链（可追溯：这条记忆从哪来、被谁改过）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProvenanceChain {
    /// 逐级出处（**原始来源在前**）。
    pub steps: Vec<ProvenanceStep>,
}

impl ProvenanceChain {
    /// 链长（0 = 无出处，**空链是合法状态**，不是错误）。
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// 是否空链。
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// 出处链上的一步。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceStep {
    /// 这一步的说明（如 "用户原话" / "由 x 模型摘要" / "人工订正"）。
    pub note: String,
    /// 这一步发生的时间（毫秒）。
    pub at_ms: i64,
    /// 这一步的来源标识（如 provider id / user id）。
    pub source: String,
}

/// 可见范围（**与 `crate::scope::MemoryScope` 语义对齐**，但**独立类型**
/// ⇒ 契约不绑死在具体枚举上，将来可换）。
///
/// 为什么要独立类型：读契约是**算法侧**依赖的东西，不能因为存储层把枚举改了
/// 就连累算法；两套语义对齐但互不绑死，换存储时契约不动。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// 全局共享（安装内所有用户/人设可见）。
    Global,
    /// 某个用户可见。
    User { user_id: String },
    /// 某个用户下的某个人设可见。
    Persona { persona_id: String, user_id: String },
    /// 某个项目可见。
    Project { project_id: String },
    /// 某次会话可见。
    Session { session_id: String },
}

/// 读契约：**算法只能通过它读，不许直接碰库**。
///
/// # 为什么要有它
///
/// 「不许直接碰库」如果只写在文档里，早晚会破；写成 trait，算法侧就只能拿到
/// `&dyn MemoryView` —— 想碰库就得先拿到库对象，那属于**越权接线**，一眼可见。
///
/// # 五条方法的语义（逐条对齐设计稿 §5 ①）
///
/// - [`MemoryView::record`]：按 id 取一条，**没有就是 `None`**（不 panic）；
/// - [`MemoryView::by_time`]：时间窗内的记录（**闭区间**，按 `(timestamp_ms, id)` 升序）；
/// - [`MemoryView::related`]：与某条相关的记录（**确定性**：按 id 升序）；
/// - [`MemoryView::provenance`]：出处链（**没有这条记录 ⇒ 返回空链**，不 panic）；
/// - [`MemoryView::visible_to`]：可见性判定（**权限在契约里，不在算法里**）。
pub trait MemoryView: Send + Sync {
    /// 按 id 取一条（未命中 ⇒ `None`）。
    fn record(&self, id: &RecordId) -> Option<&Record>;

    /// 时间窗内的记录（`from_ms` 闭区间左端、`to_ms` 闭区间右端；**按时间升序**）。
    ///
    /// 同一时间戳的记录按 **id 升序**接在后面 ⇒ 结果**全序**、可复现。
    /// `from_ms > to_ms` ⇒ 返回**空**（不 panic）。
    fn by_time(&self, from_ms: i64, to_ms: i64) -> Vec<&Record>;

    /// 与某条相关的记录（**确定性**：按 id 升序）。
    fn related(&self, id: &RecordId) -> Vec<&Record>;

    /// 出处链（没有这条记录 ⇒ 返回**空链**，不 panic）。
    fn provenance(&self, id: &RecordId) -> ProvenanceChain;

    /// 这个**视图**对某个范围是否可用（**fail-closed**：视图内**每条**都可见才 true）。
    ///
    /// ⚠️ 设计稿 §5 ① 的签名**没有记录 id** ⇒ 它只能回答「视图级」的门
    /// ⇒ 逐条口径用 [`MemoryView::record_visible_to`]。
    fn visible_to(&self, scope: &Scope) -> bool;

    /// 这**条**记录对**已授权范围列表**可见吗（**权限在契约里，不在算法里**）。
    ///
    /// # 为什么是**列表**（2026-10-26 总指挥裁决）
    ///
    /// canonical `crate::scope::MemoryScope::is_visible_in` 收的就是**列表**
    /// ⇒ 「人设代理既要读用户原始记忆、又要读自己的人设记忆」靠**传两个范围**表达
    /// ⇒ **不需要**在契约里自造「人设 ⊃ 用户」的可见性阶梯
    /// （自造阶梯 ⇒ 与 canonical 两套口径 ⇒ 权限判定分叉 ⇒ 安全风险）。
    ///
    /// 判定口径见 [`scope_visible`]（**与 canonical 逐条对齐**）。
    fn record_visible_to(&self, id: &RecordId, granted_scopes: &[Scope]) -> bool;
}

/// 可见性判定的**唯一真源**（记录 scope × **已授权范围列表** ⇒ 可见否）。
///
/// # 口径：**与 canonical `crate::scope::MemoryScope::is_visible_in` 逐条对齐**
///
/// | 记录 scope | 可见的授权 scope |
/// |---|---|
/// | `Global` | `Global` |
/// | `User { u }` | `User { 同 u }` |
/// | `Persona { p, u }` | `Persona { 同 p 且同 u }` |
/// | `Project { p }` | `Project { 同 p }` |
/// | `Session { s }` | `Session { 同 s }` |
///
/// # 为什么是**列表**而不是单一 scope（2026-10-26 总指挥裁决）
///
/// canonical 的 `is_visible_in(&self, visible_scopes: &[Self])` 收的就是**列表**
/// ⇒ 「人设代理既要读**用户原始记忆**、又要读**自己的人设记忆**」靠
/// **传两个范围**表达（`&[User { u }, Persona { p, u }]`）
/// ⇒ **不需要**在契约里自造「人设 ⊃ 用户」的可见性阶梯
/// （自造阶梯 ⇒ 与 canonical **两套口径** ⇒ 权限判定分叉 ⇒ 安全风险）。
///
/// ⚠️ 跨类**一律不可见**（`Global` 查询也**看不到** `User` 记录）——
/// 这正是 canonical 的口径（`is_visible_in` 只在**同类**之间比）。
/// ⇒ 要「全量视角」就把 `Global` **同时**授权给调用方（列表里带上它）。
///
/// ⚠️ 「全局记录对所有人可见」这条**不在**canonical 里 ⇒ 本契约也不加
/// （加了就与生产两套口径 ⇒ 记忆可能被越权看到）。
pub fn scope_visible(record_scope: &Scope, granted_scopes: &[Scope]) -> bool {
    granted_scopes
        .iter()
        .any(|granted| match (record_scope, granted) {
            (Scope::Global, Scope::Global) => true,
            (Scope::User { user_id: a }, Scope::User { user_id: b }) => a == b,
            (
                Scope::Persona {
                    persona_id: a_persona,
                    user_id: a_user,
                },
                Scope::Persona {
                    persona_id: b_persona,
                    user_id: b_user,
                },
            ) => a_persona == b_persona && a_user == b_user,
            (Scope::Project { project_id: a }, Scope::Project { project_id: b }) => a == b,
            (Scope::Session { session_id: a }, Scope::Session { session_id: b }) => a == b,
            // 跨类 ⇒ 不可见（不 panic）。
            _ => false,
        })
}

/// 单一授权范围的便捷判定（= [`scope_visible`] 传一个元素的列表）。
pub fn scope_visible_in(record_scope: &Scope, granted_scope: &Scope) -> bool {
    scope_visible(record_scope, std::slice::from_ref(granted_scope))
}

// ============================================
// ② 写契约：算法只能产「标注」，永不回写原始层
// ============================================

/// 标注种类（**算法只能产标注，永不回写原始层**）。
///
/// 为什么要有 `Ord`：标注集合要能进 [`BTreeMap`]（按种类分桶、逐桶比对），
/// 顺序必须**稳定**（跨机器同序），不能靠枚举声明顺序的"碰巧"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnnotationKind {
    /// 关联（这两条有关系）。
    Association,
    /// 标签（打标）。
    Tag,
    /// 分数（某种打分）。
    Score,
    /// 建议权重（可重算的**建议**，不是真值）。
    SuggestedWeight,
}

/// 标注载荷。
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationPayload {
    /// 关联到另一条记录。
    Association { target: RecordId, strength: f64 },
    /// 一个标签。
    Tag { label: String },
    /// 一个分数（0.0 ~ 1.0；**出界要 clamp**，不 panic）。
    Score { value: f64 },
    /// 建议权重（0.0 ~ 1.0）。
    SuggestedWeight { value: f64 },
}

impl AnnotationPayload {
    /// 造一个分数载荷（**自动 clamp 到 `[0.0, 1.0]`**，不 panic）。
    pub fn score(value: f64) -> Self {
        Self::Score {
            value: clamp_unit(value),
        }
    }

    /// 造一个建议权重载荷（**自动 clamp 到 `[0.0, 1.0]`**，不 panic）。
    pub fn suggested_weight(value: f64) -> Self {
        Self::SuggestedWeight {
            value: clamp_unit(value),
        }
    }

    /// 造一个标签载荷。
    pub fn tag(label: impl Into<String>) -> Self {
        Self::Tag {
            label: label.into(),
        }
    }

    /// 造一个关联载荷（**强度不设上下界** —— 设计稿没给强度定义量纲，
    /// 强行 clamp 会把"强度大于 1"的合法用法悄悄改小 ⇒ 保持原值）。
    pub fn association(target: RecordId, strength: f64) -> Self {
        Self::Association { target, strength }
    }

    /// 返回**已 clamp** 的副本（`Score` / `SuggestedWeight` 落到 `[0.0, 1.0]`）。
    ///
    /// 为什么要留这个函数：枚举变体的字段是 `pub` 的 ⇒ 调用方**能**直接构造越界值。
    /// 本函数 + [`MemoryAnnotation::new`] 给出**安全出口**：
    /// 越界值进库之前会被夹住，而不是在下游炸掉。
    pub fn clamped(&self) -> Self {
        match self {
            Self::Score { value } => Self::score(*value),
            Self::SuggestedWeight { value } => Self::suggested_weight(*value),
            Self::Association { target, strength } => Self::Association {
                target: target.clone(),
                strength: *strength,
            },
            Self::Tag { label } => Self::Tag {
                label: label.clone(),
            },
        }
    }
}

/// 把一个数值夹到 `[0.0, 1.0]`；**NaN ⇒ 0.0**（不 panic，也不把 NaN 传下去）。
fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// 算法戳（**哪套算法、哪个版本** ⇒ 百年后要能重算/追责）。
///
/// 三个字段合起来是**追责三件套**：谁（`algo_id`）、哪一版（`version`）、
/// 用的什么参数（`params_digest`）。少了第三个，重算就复现不出同样的结果。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlgoStamp {
    /// 算法 id（如 `betti_hole_detector`）。
    pub algo_id: String,
    /// 算法版本（字符串 ⇒ 语义化版本或内容摘要都行）。
    pub version: String,
    /// 产出时的算法参数摘要（**可复现的关键**）。
    pub params_digest: String,
}

/// 标注（**派生数据**：可全量重算，不改原始层）。
///
/// # 为什么它**不能**回写原始层
///
/// 本类型**没有任何**写原始记录的方法 —— 没有 `apply`、没有 `write_back`、
/// 没有 `&mut Record` 入口。本模块也**不提供**「把标注落回库」的函数。
/// 于是"算法回写 D0~D3"这件事在**类型层面**就不可能发生：
/// 想回写，只能另开一个口子，而那个口子必须过一次显式评审（设计稿 §3.5 铁律二）。
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAnnotation {
    /// 标注种类。
    pub kind: AnnotationKind,
    /// 被标注的记录。
    pub target: RecordId,
    /// 标注载荷。
    pub payload: AnnotationPayload,
    /// 算法戳（哪套算法、哪个版本）。
    pub algo: AlgoStamp,
}

impl MemoryAnnotation {
    /// 造一条标注（**载荷越界自动 clamp**，不 panic）。
    pub fn new(
        kind: AnnotationKind,
        target: RecordId,
        payload: AnnotationPayload,
        algo: AlgoStamp,
    ) -> Self {
        Self {
            kind,
            target,
            payload: payload.clamped(),
            algo,
        }
    }
}

// ============================================
// 标签词切分（纯函数，不引分词依赖）
// ============================================

/// 「标签词」切分：**CJK bigram + 词**（不引分词依赖）。
///
/// # 口径
///
/// - **CJK 连续块** ⇒ 切成 **bigram**（相邻两字一个词）；只有 1 个字时保留该字
///   （否则单字内容会一个标签都没有）；
/// - **非 CJK 的字母数字连续块** ⇒ 切成**一个词**（整块小写化 ⇒ 大小写不敏感）；
/// - 其余字符（标点、空白等）⇒ **分隔符**，切断两种块。
///
/// # 为什么用 bigram 而不是分词器
///
/// ① 不引新依赖（工程纪律）；② 分词器对「同一段文字切法不同」的版本漂移敏感，
/// 而本模块的口径必须**跨机器一致**（设计稿 §6.4「比稳定分不比单次分」）。
///
/// # 例子
///
/// `tokenize("记忆系统 v2 设计")` ⇒ `["记忆", "忆系", "系统", "v2", "设计"]`
/// （"记忆系统" 是 4 个字的 CJK 块 ⇒ 3 个 bigram；"v2" 是词；"设计" 是 2 字块 ⇒ 1 个 bigram）。
#[must_use]
pub fn tokenize(content: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut word: String = String::new();
    let mut cjk: Vec<char> = Vec::new();

    for ch in content.chars() {
        if is_cjk(ch) {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word).to_lowercase());
            }
            cjk.push(ch);
        } else if ch.is_alphanumeric() {
            if !cjk.is_empty() {
                push_cjk(&mut tokens, &mut cjk);
            }
            word.push(ch);
        } else {
            // 分隔符：切断两种块。
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word).to_lowercase());
            }
            if !cjk.is_empty() {
                push_cjk(&mut tokens, &mut cjk);
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word.to_lowercase());
    }
    if !cjk.is_empty() {
        push_cjk(&mut tokens, &mut cjk);
    }
    tokens
}

/// 把一个 CJK 块切成 bigram（块长为 1 时保留单字）。
fn push_cjk(tokens: &mut Vec<String>, run: &mut Vec<char>) {
    if run.len() == 1 {
        tokens.push(run[0].to_string());
    } else {
        for pair in run.windows(2) {
            tokens.push(pair.iter().collect());
        }
    }
    run.clear();
}

/// 是否 CJK 文字（假名 / CJK 统一表意文字及其扩展）。
///
/// 为什么不用「非 ASCII 字母」那种粗判：那会把希腊文、西里尔文也当成 CJK
/// 切 bigram（假名与汉字才是不分词连写的）。
fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x3040..=0x30FF).contains(&code)        // 平假名 / 片假名
        || (0x3400..=0x4DBF).contains(&code)   // CJK 扩展 A
        || (0x4E00..=0x9FFF).contains(&code)   // CJK 统一表意文字（基本区）
        || (0xF900..=0xFAFF).contains(&code)   // CJK 兼容表意文字
        || (0x20000..=0x3FFFF).contains(&code) // 扩展 B 及以后（第二、三平面）
}

// ============================================
// 可装进内存的读侧实现（供算法离线跑批用）
// ============================================

/// 一个**可装进内存**的读侧实现（供算法离线跑批用；生产可另接库实现）。
///
/// # 口径
///
/// * 存储用 [`BTreeMap`] ⇒ **确定性**（不用 `HashMap` 迭代序）；
/// * [`MemoryView::by_time`] **按 `(timestamp_ms, id)` 升序** ⇒ 同一时间戳的也有确定顺序；
/// * [`MemoryView::related`] = 与目标**共享至少一个标签词**的其它记录（见 [`tokenize`]）
///   ⇒ **确定性**（按 id 升序）；「标签词」= 内容里的 CJK bigram + 词（**不引分词依赖**）。
///
/// # `visible_to` 的口径（⚠️ 设计稿签名没有记录 id ⇒ 这里必须说清）
///
/// 设计稿 §5 ① 的签名是 `visible_to(&self, scope: &Scope) -> bool` —— **只有 scope**。
/// 本实现取**全称口径**：视图内**每一条**记录都对 scope 可见才返回 `true`
/// （空视图按空真返回 `true`）。理由：这条方法回答的是「**整个视图**能不能用」，
/// 取全称才是**失败即关门**（fail-closed）的用法；取存在量化会让"混着不同 scope 的视图"
/// 在只有一条记录可见时也放行 ⇒ 越权风险。
///
/// **逐条**口径请用 [`InMemoryMemoryView::record_visible_to`] 或 [`scope_visible`]；
/// 要"这个 scope 能看到的全部记录"请用 [`InMemoryMemoryView::visible_records`]。
#[derive(Debug, Clone)]
pub struct InMemoryMemoryView {
    records: BTreeMap<RecordId, Record>,
    /// 标签词的倒排索引（`词 → 记录 id 列表`，**列表按 id 升序**）。
    index: BTreeMap<String, Vec<RecordId>>,
}

impl InMemoryMemoryView {
    /// 用一批记录建视图（**建倒排索引**）。
    ///
    /// 同一批记录建两次 ⇒ 结果**逐字节相等**（无哈希序依赖）。
    #[must_use]
    pub fn new(records: Vec<Record>) -> Self {
        let mut view = Self {
            records: BTreeMap::new(),
            index: BTreeMap::new(),
        };
        for record in records {
            view.insert(record);
        }
        view
    }

    /// 追加一条记录（**重建受影响的倒排项**）。
    ///
    /// 同 id 重复插入 ⇒ **覆盖**旧记录并**重建**它的倒排项
    /// （旧标签词不再指向它 ⇒ 不会留下"幽灵命中"）。
    pub fn insert(&mut self, record: Record) {
        let id = record.id.clone();
        // 先摘掉该 id 的旧倒排项（覆盖语义 ⇒ 旧标签必须一起消失）。
        self.drop_postings(&id);
        for token in tokenize(&record.content) {
            let postings = self.index.entry(token).or_default();
            // 保持 posting 列表按 id 升序 ⇒ 查询结果天然有序。
            let position = postings.partition_point(|existing| existing < &id);
            postings.insert(position, id.clone());
        }
        self.records.insert(id, record);
    }

    /// 记录条数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 是否空视图。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// **逐条**可见性判定：这条记录对**已授权范围列表**可见吗
    /// （记录不存在 ⇒ `false`，不 panic）。
    ///
    /// ⚠️ **列表口径**（与 canonical `is_visible_in` 对齐）⇒
    /// 「人设代理既要读用户原始记忆、又要读自己的人设记忆」⇒ 传
    /// `&[Scope::User { user_id }, Scope::Persona { persona_id, user_id }]`。
    #[must_use]
    pub fn record_visible_to(&self, id: &RecordId, granted_scopes: &[Scope]) -> bool {
        self.records
            .get(id)
            .is_some_and(|record| scope_visible(&record.scope, granted_scopes))
    }

    /// 对**已授权范围列表**可见的全部记录（按 id 升序）。
    #[must_use]
    pub fn visible_records(&self, granted_scopes: &[Scope]) -> Vec<&Record> {
        self.records
            .values()
            .filter(|record| scope_visible(&record.scope, granted_scopes))
            .collect()
    }

    /// 摘掉某条记录在倒排索引里的全部 posting（**顺带清掉空 posting 列表**）。
    fn drop_postings(&mut self, id: &RecordId) {
        self.index.retain(|_, postings| {
            postings.retain(|posting| posting != id);
            !postings.is_empty()
        });
    }
}

impl MemoryView for InMemoryMemoryView {
    fn record(&self, id: &RecordId) -> Option<&Record> {
        self.records.get(id)
    }

    fn by_time(&self, from_ms: i64, to_ms: i64) -> Vec<&Record> {
        // 倒挂窗口（from > to）⇒ 空（不 panic）。
        if from_ms > to_ms {
            return Vec::new();
        }
        let mut out: Vec<&Record> = self
            .records
            .values()
            .filter(|record| record.timestamp_ms >= from_ms && record.timestamp_ms <= to_ms)
            .collect();
        // 全序：(timestamp_ms, id) ⇒ 同一时间戳也有确定顺序。
        out.sort_by(|left, right| {
            left.timestamp_ms
                .cmp(&right.timestamp_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        out
    }

    fn related(&self, id: &RecordId) -> Vec<&Record> {
        let Some(target) = self.records.get(id) else {
            // 目标不存在 ⇒ 没有相关（不 panic）。
            return Vec::new();
        };
        // 共享至少一个标签词的其它记录。
        let mut related_ids: BTreeSet<RecordId> = BTreeSet::new();
        for token in tokenize(&target.content) {
            if let Some(postings) = self.index.get(&token) {
                for posting in postings {
                    related_ids.insert(posting.clone());
                }
            }
        }
        related_ids.remove(id);
        // BTreeSet ⇒ 按 id 升序（确定性）。
        related_ids
            .into_iter()
            .filter_map(|related_id| self.records.get(&related_id))
            .collect()
    }

    fn provenance(&self, id: &RecordId) -> ProvenanceChain {
        // 没有这条记录 ⇒ 空链（不 panic）。
        self.records
            .get(id)
            .map(|record| record.provenance.clone())
            .unwrap_or_default()
    }

    fn record_visible_to(&self, id: &RecordId, granted_scopes: &[Scope]) -> bool {
        InMemoryMemoryView::record_visible_to(self, id, granted_scopes)
    }

    fn visible_to(&self, scope: &Scope) -> bool {
        // 全称口径（理由见本类型文档）：每条可见才放行整个视图。
        self.records
            .values()
            .all(|record| scope_visible_in(&record.scope, scope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, content: &str, timestamp_ms: i64) -> Record {
        Record {
            id: RecordId::new(id),
            content: content.to_string(),
            timestamp_ms,
            provenance: ProvenanceChain::default(),
            scope: Scope::Global,
        }
    }

    #[test]
    fn in_memory_view_reads_back_records() {
        let view = InMemoryMemoryView::new(vec![record("r1", "记忆归档", 100)]);

        assert_eq!(view.len(), 1);
        assert!(!view.is_empty());
        assert_eq!(
            view.record(&RecordId::new("r1")).map(|r| r.id.as_str()),
            Some("r1")
        );
        assert!(view.record(&RecordId::new("nope")).is_none());
    }

    #[test]
    fn tokenize_splits_cjk_bigrams_and_words() {
        assert_eq!(
            tokenize("记忆系统 v2 设计"),
            vec!["记忆", "忆系", "系统", "v2", "设计"]
        );
        // 单字块保留单字（否则单字内容没有标签）。
        assert_eq!(tokenize("猫"), vec!["猫"]);
        // 分隔符切断两种块。
        assert_eq!(tokenize("a, 猫"), vec!["a", "猫"]);
    }

    #[test]
    fn scope_visible_matches_the_canonical_ladder() {
        // ⚠️ **与 canonical `crate::scope::MemoryScope::is_visible_in` 逐条对齐**：
        // **只同类相等**才可见；跨类**一律不可见**（连 `Global` 查询也看不到 `User` 记录）。
        let user = Scope::User {
            user_id: "u1".into(),
        };
        let persona = Scope::Persona {
            persona_id: "p1".into(),
            user_id: "u1".into(),
        };
        // 同类相等 ⇒ 可见。
        assert!(scope_visible_in(&Scope::Global, &Scope::Global));
        assert!(scope_visible_in(&user, &user));
        assert!(scope_visible_in(&persona, &persona));
        // 异 user ⇒ 不可见。
        assert!(!scope_visible_in(
            &Scope::User {
                user_id: "u2".into()
            },
            &user
        ));
        // **跨类不可见**（canonical 口径；「全局记录对所有人可见」不在 canonical 里 ⇒ 本契约也不加）。
        assert!(!scope_visible_in(&user, &Scope::Global));
        assert!(!scope_visible_in(&Scope::Global, &user));
        assert!(!scope_visible_in(&persona, &user));
        assert!(!scope_visible_in(&user, &persona));
    }

    #[test]
    fn granted_scope_list_expresses_delegation() {
        // 「人设代理既要读用户原始记忆、又要读自己的人设记忆」
        // ⇒ **靠列表**表达（不是靠自造阶梯）⇒ 这就是 canonical 的做法。
        let user = Scope::User {
            user_id: "u1".into(),
        };
        let persona = Scope::Persona {
            persona_id: "p1".into(),
            user_id: "u1".into(),
        };
        let granted = [user.clone(), persona.clone()];
        assert!(scope_visible(&user, &granted), "用户原始记忆：可见");
        assert!(scope_visible(&persona, &granted), "自己的人设记忆：可见");
        let other_user = Scope::User {
            user_id: "u2".into(),
        };
        assert!(!scope_visible(&other_user, &granted), "别人的记忆：不可见");
    }

    #[test]
    fn annotation_payload_clamps_out_of_range_scores() {
        let clamped = AnnotationPayload::score(1.7).clamped();
        assert_eq!(clamped, AnnotationPayload::Score { value: 1.0 });
        let clamped = AnnotationPayload::score(-0.3).clamped();
        assert_eq!(clamped, AnnotationPayload::Score { value: 0.0 });
        // NaN 不往下传（不 panic）。
        let clamped = AnnotationPayload::score(f64::NAN).clamped();
        assert_eq!(clamped, AnnotationPayload::Score { value: 0.0 });
    }
}
