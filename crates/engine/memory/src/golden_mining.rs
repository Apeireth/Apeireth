//! `apeireth-memory::golden_mining` — 从记忆记录挖考卷候选题目。
//!
//! # 为什么有这个模块
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §7.1 把「内部卷」定为：
//! **从本仓日记/记忆挖 2000 条候选 → 人工精标 300 条**，用来证明「我们自己的场景」好不好用
//! （外部公开卷是中文但场景不同）。
//! 「挖」这一步必须是**机器可复现**的：同一批记忆两次挖出两套题，卷子就没法比。
//! 本模块就是那条工序的机器部分：输入一批抽象记忆记录 ⇒ 输出「像人话的问题 + 该命中的记录 + 答案片段」。
//!
//! # 定位（只做粗筛，不做精标）
//!
//! - **只挖不标**：题目是否像人话、答案是否唯一，由 owner 精标时判（§8 B0-b）。
//! - 输入是抽象记录（[`MineRecord`]），不绑定 SQLite / 文件后端 ⇒ 纯函数、可测、不依赖真实数据。
//! - 输出**完全确定**：同输入两次 [`mine`] 逐字相等（无随机数、无哈希序依赖）。
//! - 只读：不写文件、不联网、不读环境变量。
//!
//! # 挖出来的题目长什么样
//!
//! | 题型 | 问法 | 标准答案（expect） |
//! |---|---|---|
//! | `keyword` | 关于{主体}，我们当时记了什么？ | 1 条记录 + 含该词的那一句 |
//! | `multi_record` | {主体A}和{主体B}这两件事是什么关系？ | 2 条记录，各一句 |
//! | `temporal` | {主体}是什么时候定下来的？ | 2 条记录（较晚那条必含，答案含时间） |
//! | `conflict` | {主体}后来是怎么改的？ | 2 条记录 |
//! | `decision` | {主体}这件事最后是怎么定的？ | 1 条记录 + 含决策词的那一句 |
//!
//! # 「主体」是什么（为什么不是直接用分词结果）
//!
//! 分词口径是 **CJK bigram**（无词典依赖 ⇒ 跨机器逐字一致），代价是切出来的常是碎片
//! （「召回延迟」⇒ `召回`/`回延`/`延迟`）。把碎片直接塞进问题会得到
//! 「关于回延，这两件事是什么关系？」这种**不像人话**的题。
//! 因此问题主体（`{主体}`）按下面顺序取：
//!
//! 1. 记录**标题**（人写的摘要，最像人话）；
//! 2. 标题里命中正文关键词池、且在标题中位置最靠前的词；
//! 3. 正文 top 关键词（无标题时的兜底）。
//!
//! 标题缺失时退回分词结果——这是本模块已知的质量上限，见交付说明。
//!
//! # 契约保证（测试逐条断言）
//!
//! 1. `expect_fragments[i]` **一定**是 `expect_ids[i]` 那条记录 `content` 的子串
//!    （否则模型答对了也无法判定「有据」）。
//! 2. 每条记录最多参与 `max_cases_per_record` 道题（防刷题）。
//! 3. 产出按 `(kind, id)` 排序 ⇒ 逐字可比。
//! 4. 挖不出就返回空 `Vec`，不报错、不 panic。
//!
//! # 状态: NOT IMPLEMENTED（新增未接线）
//!
//! 本模块**尚未接入任何生产路径**（无 CLI、无定时任务、无评估器调用点）：
//! 它只是把「挖题」这件事的能力做出来，等 owner 精标 + B0-b 接线批次再挂上去。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// 一条待挖矿的记忆记录（**抽象**，不绑定具体存储类型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineRecord {
    /// 记录 id（出现在 `expect_ids` 里，作为标准答案的指针）。
    pub id: String,
    /// 记录正文（可能多行）。
    pub content: String,
    /// 记录标题/首行摘要（可为空）。标题只参与「同主题」判定与问题主体的优选。
    pub title: Option<String>,
    /// 记录时间（毫秒时间戳）；用于挑「互相冲突/时间相邻」的记录。
    pub timestamp_ms: i64,
    /// 可选：这批记录的**同一会话/主题分组键**（同组 = 同一个话题 ⇒ 适合挖「同主题多版本」类问题）。
    pub thread_id: Option<String>,
}

/// 挖出的一条候选题目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinedCase {
    /// 候选 id（**确定性**生成：`gm-{kind}-{序号}`，无随机数）。
    pub id: String,
    /// 问题文本。
    pub query: String,
    /// 该命中的记录 id（**标准答案**：≥1 个）。按 `(时间, id)` 升序，便于跨批次比对。
    pub expect_ids: Vec<String>,
    /// 该命中的答案片段（原文子串；**必须真的出现在** `expect_ids[i]` 那条记录的 `content` 里）。
    ///
    /// 与 `expect_ids` **一一对应**：`expect_fragments[i]` 属于 `expect_ids[i]`。
    pub expect_fragments: Vec<String>,
    /// 题型标签：`keyword` / `multi_record` / `temporal` / `conflict` / `decision`。
    pub kind: String,
}

/// 挖掘配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiningConfig {
    /// 单题最多关联几条记录。
    pub max_records_per_case: usize,
    /// 单条记录最多出几道题（防刷题）。
    pub max_cases_per_record: usize,
    /// 最短正文长度（太短的记录不挖）。
    pub min_content_chars: usize,
    /// 判定「两个记录是同主题」的共享词最少个数。
    pub min_shared_terms: usize,
    /// 冲突型题目：两条记录时间差的上限（毫秒）—— 超过则不算「时间相邻」。
    pub temporal_window_ms: i64,
}

impl Default for MiningConfig {
    /// 3 条 / 2 题 / 16 字 / 2 个共享词 / 7 天。
    fn default() -> Self {
        Self {
            max_records_per_case: 3,
            max_cases_per_record: 2,
            min_content_chars: 16,
            min_shared_terms: 2,
            temporal_window_ms: 7 * 24 * 60 * 60 * 1000,
        }
    }
}

/// 答案片段最大长度（字）。
pub const FRAGMENT_MAX_CHARS: usize = 60;

/// 决策标记词（正文里命中**任一**即认为这条记录里有「拍板」）。
///
/// ⚠️ **必须同时收录繁体变体**（2026-10-26 实测）：
/// 原先只有简体（「决定/否决/改为/因为/通过」）⇒ 在**繁体语料**（《紅樓夢》《水滸傳》）
/// 上**永不命中** ⇒ `decision` 题型**大量漏判**
/// （实测两本小说的 decision 占比 110 vs 6 —— 那个悬殊**不是语料差异，是标记词表的锅**）。
/// ⇒ 每个简体标记词都补上繁体形；同形的（「裁定/原因/所以/拍板」）不重复列。
pub const DECISION_MARKERS: [&str; 14] = [
    // 简体
    "裁定", "决定", "因为", "原因", "所以", "改为", "否决", "通过", "拍板",
    // 繁体变体（简体表里**没有**的）
    "決定", "因為", "改為", "否決", "通過",
];

/// 每条记录参与的主题词池大小（内部用）：关键词优选要在这些词里挑。
const KEY_TERM_POOL: usize = 12;

/// 高频虚词字：出现在 CJK bigram 里 ⇒ 该 bigram 多半是**跨词边界的碎片**（「们需」「定把」），
/// 不是词 ⇒ 丢掉，否则挖出来的题目全是碎片词（不像人话）。
///
/// 代价：含这些字的真词也会被丢（「需要」「如果」「结果」）。这是**质量取舍**：
/// 宁可少几个召回好的词，也不要让题目变成「关于们需，我们当时记了什么？」。
const FUNCTION_CHARS: &str = "的了着过是在和与就都也而及或但却还又再更很太把被让使给对从向于并之其有无不要会能可说我你他她们这那些什么怎样呢吗吧啊呀嘛哪谁因所果如";

/// 中文数词（数量词的核心字）。
///
/// **为什么要单列**：纯 ASCII 数字的过滤（`is_ascii_digit` 全命中）拦不住
/// 「两千」「三次」「十毫秒」这类**中文数量词** ⇒ 会挖出
/// 「关于两千，我们当时记了什么？」这种**不像人话**的题
/// （**数量不是话题** —— 人不会「关于两千」地去回忆）。
const CHINESE_NUMERALS: &str = "零一二三四五六七八九十百千万亿两";

/// 量词/计量单位字（跟在数词后面 ⇒ 「数词+量词」= 数量短语）。
///
/// 收录的是**能独立构成数量短语**的字：个数（个/条/次/件…）、
/// 时间（年/月/日/时/分/秒/毫…）、度量（张/份/台/块/位…）。
const QUANTIFIER_CHARS: &str = "个条次件遍趟回下把张份台辆支块位名只年月日时分秒毫厘微周轮番度";

/// 停用词（整词命中才丢；与上面的虚词字过滤互补）。
const STOPWORDS: &[&str] = &[
    // 中文虚词 bigram（多数已被虚词字过滤兜住，这里只留残留的整词）
    "我们", "你们", "他们", "这个", "那个", "这些", "那些", "一个", "一些", "什么", "怎么", "这样",
    "那样", "就是", "不是", "但是", "因为", "所以", "如果", "虽然", "然后", "已经", "还是", "可以",
    "不能", "没有", "这里", "那里", "时候", "现在", "之后", "之前", "以及", "而且", "或者", "不过",
    "于是", "因此", "由于", "关于", "对于", "今天", "明天", "昨天", "自己", "大家", "东西", "事情",
    // 英文虚词
    "the", "and", "for", "that", "with", "this", "from", "have", "has", "had", "was", "were", "are",
    "but", "not", "you", "your", "our", "its", "his", "her", "they", "them", "their", "which",
    "what", "when", "where", "would", "could", "should", "been", "being", "than", "then", "also",
    "of", "is", "in", "it", "to", "as", "at", "by", "on", "or", "an", "be", "no", "so", "do", "we",
    "he", "my",
];

/// **泛称表**（数量词 + 称谓 + 泛指名词）。
///
/// # 为什么单独一张表（2026-10-26 实测）
///
/// 停用词表里没有这些 ⇒ 挖题时它们会当「问题主体」⇒ 题目长成
/// 「**一個**后来是怎么改的？」「**先生**后来是怎么改的？」——**不像人话**
/// （人会问「宋江后来怎么样」，不会问「一個后来怎么样」）。
///
/// ⚠️ **实测频率**（《水滸傳》）：`兩個` 1240 次、`一個` 1014 次
/// ⇒ 它们是**全篇最高频**的 bigram ⇒ 不滤掉就必然被选中。
const GENERIC_TERMS: &[&str] = &[
    // 数量泛称（繁体 + 简体）
    "一個",
    "兩個",
    "三個",
    "四個",
    "五個",
    "幾個",
    "許多",
    "很多",
    "一个",
    "两个",
    "三个",
    "几个",
    "许多",
    "很多",
    // 称谓泛称
    "先生",
    "夫人",
    "老爺",
    "老爷",
    "夫人",
    "公子",
    "姑娘",
    "大爺",
    "大爷",
    "和尚",
    "道人",
    "和尚",
    "小人",
    "在下",
    "奴家",
    "小二",
    // 泛指名词
    "哥哥",
    "兄弟",
    "頭領",
    "头领",
    "衆人",
    "众人",
    "眾人",
    "众人",
    "那人",
    "此人",
    "有人",
    "那裏",
    "這裏",
    "那里",
    "这里",
    "如何",
    "怎樣",
    "怎样",
    "爲何",
    "为什么",
    "甚麼",
    "什麼",
    "什么",
    "說道",
    "说道",
    "只見",
    "只见",
    "聽得",
    "听得",
    "去了",
    "了",
    // **虚词 bigram**（实测高频 ⇒ 不滤就一定被选中）：
    "笑道",
    "說著",
    "说着",
    "聽了",
    "听了",
    "起來",
    "起来",
    "出來",
    "出来",
    "來了",
    "来了",
    "不知",
    "也不",
    "我們",
    "我们",
    "他們",
    "他们",
    "你們",
    "你们",
    "咱們",
    "咱们",
    "那裡",
    "那里",
    "這裡",
    "這里",
    "這樣",
    "这样",
    "那樣",
    "那样",
    "如今",
    "知道",
    "聽見",
    "听见",
    "一面",
    "不得",
    "沒有",
    "没有",
    "東西",
    "东西",
    "就是",
    "告訴",
    "告诉",
    "進來",
    "进来",
    "回來",
    "回来",
    "丫頭",
    "丫头",
    "奶奶",
    "太太",
    "老爺",
    "老爷",
    "姐儿",
    "姐兒",
    "的人",
    "這個",
    "这个",
    "了說",
    "说了",
    "看著",
    "看着",
];

/// **实体词的最低出现次数**（长正文时用）。
///
/// # 为什么要有门槛（2026-10-26 实测）
///
/// bigram 切法必然产生**跨词边界的碎片**（「甄士隱」被切成「事隱」+「士隱」
/// ⇒ 「事隱」只出现 **1** 次 ⇒ 抽到它就会挖出「关于事隱，我们当时记了什么？」）。
///
/// 实测（《紅樓夢》）：实体「寶玉」**3832** 次、「賈母」1616、「黛玉」1321；
/// 碎片「事隱」**1** 次、「中既」3 次。
/// ⇒ **频次是最强的实体判据**。
const ENTITY_MIN_COUNT: usize = 8;

/// **短正文的频次门槛**（正文短到抽不出重复词时）。
///
/// # 为什么要两档（2026-10-26 实测）
///
/// 真实语料里两种正文长度差三个数量级：
/// * 小说一回 **4000+ 字** ⇒ 「寶玉」出现 3832 次 ⇒ 门槛 8 合理；
/// * 提交正文 / 短笔记 **几十字** ⇒ 任何词都只出现 1~3 次
///   ⇒ 门槛 8 会让**所有短记录都出不了题**（实测：短正文记录的 keyword 题全灭）。
///
/// ⇒ 按正文长度**自适应**：长正文用严格门槛（挡住碎片），短正文用宽松门槛
/// （短正文里本来就没有高频碎片可选 ⇒ 碎片风险本来就低）。
const ENTITY_MIN_COUNT_SHORT: usize = 1;

/// 短正文的判定阈值（字符数；低于它 ⇒ 用宽松门槛）。
const SHORT_BODY_CHARS: usize = 400;

/// 泛称集合（一次性构建 ⇒ 纯函数、线程安全）。
fn generic_set() -> &'static BTreeSet<&'static str> {
    static SET: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| GENERIC_TERMS.iter().copied().collect())
}

/// 停用词集合（一次性构建，之后只读 ⇒ 纯函数、线程安全）。
fn stopword_set() -> &'static BTreeSet<&'static str> {
    static SET: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| STOPWORDS.iter().copied().collect())
}

/// 中文分词口径：**连续 CJK 字符两两组合（bigram）** + ASCII 词（字母数字下划线，≥2 字符）。
/// 标点丢弃。返回**去重保序**的 token 列表。
///
/// # 为什么 bigram 而不是词典分词
///
/// 本仓不引入分词依赖，且要保证「跨机器、跨版本逐字一致」。
/// bigram 的口径是纯字符规则 ⇒ 结果只取决于输入字符串，无词典版本漂移。
/// 代价见 [`key_terms`] 的说明（碎片词由虚词字过滤兜底，题目主体另走标题优先）。
///
/// 标点同时是**边界**：CJK bigram 不会跨句号/逗号成词，
/// 于是「token 一定完整落在某一句里」⇒ 答案片段总能对上句子。
pub fn tokenize(text: &str) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();
    for tok in raw_tokens(text) {
        if seen.contains(&tok) {
            continue;
        }
        seen.insert(tok.clone());
        out.push(tok);
    }
    out
}

/// 从单条记录里提关键词（**确定性**：按频次降序 + 字典序；过滤停用词与过短 token）。
///
/// 过滤链（比「停用词 + 过短」多了两档，都是为了「像人话」）：
///
/// 1. 过短 token（< 2 字）；
/// 2. 停用词（整词）；
/// 3. 纯数字 token（`2024` 不配当问题关键词）；
/// 4. 含虚词字的 CJK bigram（跨词边界的碎片，如「们需」）；
/// 5. 中文数量短语（「两千」「三次」—— **数量不是话题**，否则出「关于两千…」这种假题）。
///
/// 英文 token 不做大小写折叠：契约口径是「按空格词」，保持原样最可预期。
pub fn key_terms(content: &str, max_terms: usize) -> Vec<String> {
    if max_terms == 0 {
        return Vec::new();
    }
    let counts = term_counts(content);
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    // 频次降序；同频次按字典序（`String` 的 `Ord` 是字节序，跨平台一致）。
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items
        .into_iter()
        .take(max_terms)
        .map(|(term, _)| term)
        .collect()
}

/// 挖掘主入口：**确定性**（同输入同输出，**禁止随机、禁止哈希序依赖**）。
///
/// # 出题优先级（决定了预算紧张时先牺牲谁）
///
/// 1. `keyword` —— 保证每条够长的记录至少有 1 题（**广度**兜底）；
/// 2. `decision` —— 单条记录就能答，信息密度高；
/// 3. `conflict` → 4. `temporal` → 5. `multi_record` —— 跨记录题（**深度**），
///    按**具体程度**排序：冲突（同时回答「改了没有」）> 时序（回答「什么时候」）
///    > 泛问（只回答「什么关系」）。
///
///    后果（有意为之）：预算只有 2 题/记录时，同一对记录留下的是**信息量最大**的那道题，
///    而不是「什么关系」这种谁都能答的泛问。
///    「同主题但相隔很久」（不在时间窗内）的两条只剩 `multi_record` 一类可挖。
///
/// # 其它保证
///
/// - 每条记录最多参与 [`MiningConfig::max_cases_per_record`] 道题；
/// - 单题最多关联 [`MiningConfig::max_records_per_case`] 条记录（< 2 时不产出跨记录题）；
/// - 正文短于 [`MiningConfig::min_content_chars`] 的记录**不挖**；
/// - 挖不出就返回空 `Vec`（不报错、不 panic）。
pub fn mine(records: &[MineRecord], config: MiningConfig) -> Vec<MinedCase> {
    let mut prepared: Vec<Prepared> = records
        .iter()
        .filter(|r| r.content.chars().count() >= config.min_content_chars)
        .map(Prepared::from_record)
        .collect();
    // 按 `(时间, id)` 归一：这样**输入顺序不影响产出**（同一批记忆换个顺序挖，卷子一样）。
    prepared.sort_by(|a, b| {
        a.timestamp_ms
            .cmp(&b.timestamp_ms)
            .then_with(|| a.id.cmp(&b.id))
    });

    let global_freq = global_term_freq(&prepared);

    let mut cases: Vec<MinedCase> = Vec::new();
    let mut usage: BTreeMap<String, usize> = BTreeMap::new();
    let mut seq: BTreeMap<String, usize> = BTreeMap::new();

    mine_keyword(&prepared, config, &mut cases, &mut usage, &mut seq);
    mine_decision(&prepared, config, &mut cases, &mut usage, &mut seq);
    if config.max_records_per_case >= 2 {
        // 顺序 = 具体程度（见文档「出题优先级」）：冲突 > 时序 > 泛问。
        mine_conflict(
            &prepared,
            config,
            &global_freq,
            &mut cases,
            &mut usage,
            &mut seq,
        );
        mine_temporal(
            &prepared,
            config,
            &global_freq,
            &mut cases,
            &mut usage,
            &mut seq,
        );
        mine_multi_record(
            &prepared,
            config,
            &global_freq,
            &mut cases,
            &mut usage,
            &mut seq,
        );
    }

    // 产出按 (kind, id) 排序 ⇒ 确定性 + 跨批次可比。
    cases.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.id.cmp(&b.id)));
    cases
}

// ============================================================
// 各题型
// ============================================================

/// 题型 1：对每条记录生成「关于{主体}，我们当时记了什么？」。
fn mine_keyword(
    prepared: &[Prepared],
    config: MiningConfig,
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
) {
    for p in prepared {
        if !budget_left(p, config, usage) {
            continue;
        }
        let Some(subject) = p.subject() else {
            continue;
        };
        let anchor = p.primary_keyword().unwrap_or_else(|| subject.clone());
        let Some(fragment) = p.fragment_for(&subject, &anchor) else {
            continue;
        };
        let case = MinedCase {
            id: next_id(seq, "keyword"),
            query: format!("关于{subject}，我们当时记了什么？"),
            expect_ids: vec![p.id.clone()],
            expect_fragments: vec![fragment],
            kind: "keyword".to_string(),
        };
        *usage.entry(p.id.clone()).or_insert(0) += 1;
        cases.push(case);
    }
}

/// 题型 2：正文命中决策标记词 ⇒「{主体}这件事最后是怎么定的？」。
fn mine_decision(
    prepared: &[Prepared],
    config: MiningConfig,
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
) {
    for p in prepared {
        if !budget_left(p, config, usage) {
            continue;
        }
        let Some((marker, sentence)) = p.decision_sentence() else {
            continue;
        };
        // ⚠️ **决策题的主体取自「决策句」**（2026-10-26 修正）：
        // 决策题问的是「**这句话里的那件事**最后怎么定的」
        // ⇒ 主体必须是**那句话里的词**，而不是整篇正文的高频词
        // （否则「裁定：限流阈值定在每秒两千次」的主体会是别处的词 ⇒ 答非所问）。
        // ⇒ 用决策句里的高频词（**该词必须出现在决策句里**）。
        let Some(subject) = p.sentence_subject(&sentence) else {
            continue;
        };
        let fragment = clip_around(&sentence, &marker, FRAGMENT_MAX_CHARS);
        let case = MinedCase {
            id: next_id(seq, "decision"),
            query: format!("{subject}这件事最后是怎么定的？"),
            expect_ids: vec![p.id.clone()],
            expect_fragments: vec![fragment],
            kind: "decision".to_string(),
        };
        *usage.entry(p.id.clone()).or_insert(0) += 1;
        cases.push(case);
    }
}

/// 题型 3：两条记录共享词足够 ⇒「{主体A}和{主体B}这两件事是什么关系？」。
fn mine_multi_record(
    prepared: &[Prepared],
    config: MiningConfig,
    global_freq: &BTreeMap<String, usize>,
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
) {
    for (i, a) in prepared.iter().enumerate() {
        for b in prepared.iter().skip(i + 1) {
            if !budget_left(a, config, usage) || !budget_left(b, config, usage) {
                continue;
            }
            let shared = shared_terms(a, b);
            if shared.len() < config.min_shared_terms {
                continue;
            }
            let ranked = rank_shared(&shared, a, b, global_freq);
            let anchor = pick_term(&ranked, &a.content_terms);
            let (subject_a, subject_b) = match (&a.subject(), &b.subject()) {
                (Some(sa), Some(sb)) if sa != sb => (sa.clone(), sb.clone()),
                _ => {
                    // 主体重复（同标题）⇒ 退回「共享词」口径，避免出现「X和X这两件事」。
                    let term_a = ranked.first().cloned();
                    let term_b = ranked.get(1).cloned().or_else(|| b.primary_keyword());
                    match (term_a, term_b) {
                        (Some(ta), Some(tb)) => (ta, tb),
                        _ => continue,
                    }
                }
            };
            let anchor_a = anchor.clone().unwrap_or_else(|| subject_a.clone());
            let anchor_b = anchor.unwrap_or_else(|| subject_b.clone());
            let Some(frag_a) = a.fragment_for(&subject_a, &anchor_a) else {
                continue;
            };
            let Some(frag_b) = b.fragment_for(&subject_b, &anchor_b) else {
                continue;
            };
            push_pair_case(
                cases,
                usage,
                seq,
                "multi_record",
                format!("{subject_a}和{subject_b}这两件事是什么关系？"),
                a,
                b,
                frag_a,
                frag_b,
            );
        }
    }
}

/// 题型 4：同 thread（或共享词足够）且时间差 ≤ 窗口 ⇒「{主体}是什么时候定下来的？」。
///
/// 主体取**较晚那条**（答案含时间 ⇒ 较晚那条必在答案里）。
fn mine_temporal(
    prepared: &[Prepared],
    config: MiningConfig,
    global_freq: &BTreeMap<String, usize>,
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
) {
    for (i, a) in prepared.iter().enumerate() {
        for b in prepared.iter().skip(i + 1) {
            if !budget_left(a, config, usage) || !budget_left(b, config, usage) {
                continue;
            }
            if !temporal_related(a, b, config) {
                continue;
            }
            let (earlier, later) = order_by_time(a, b);
            let Some(subject) = later.subject() else {
                continue;
            };
            let shared = shared_terms(a, b);
            let ranked = rank_shared(&shared, a, b, global_freq);
            let anchor = pick_term(&ranked, &earlier.content_terms)
                .or_else(|| later.primary_keyword())
                .unwrap_or_else(|| subject.clone());
            let Some(frag_early) = earlier.fragment_for(&subject, &anchor) else {
                continue;
            };
            let Some(frag_late) = later.fragment_for(&subject, &anchor) else {
                continue;
            };
            push_pair_case(
                cases,
                usage,
                seq,
                "temporal",
                format!("{subject}是什么时候定下来的？"),
                a,
                b,
                frag_early,
                frag_late,
            );
        }
    }
}

/// 题型 5：同 thread 或共享词强，但**内容不同**且时间相邻 ⇒「{主体}后来是怎么改的？」。
fn mine_conflict(
    prepared: &[Prepared],
    config: MiningConfig,
    global_freq: &BTreeMap<String, usize>,
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
) {
    for (i, a) in prepared.iter().enumerate() {
        for b in prepared.iter().skip(i + 1) {
            if !budget_left(a, config, usage) || !budget_left(b, config, usage) {
                continue;
            }
            if !temporal_related(a, b, config) {
                continue;
            }
            // 内容几乎相同 ⇒ 是同一条的重复，不是「冲突」。
            if !content_differs(a, b) {
                continue;
            }
            let (earlier, later) = order_by_time(a, b);
            let Some(subject) = later.subject() else {
                continue;
            };
            let shared = shared_terms(a, b);
            let ranked = rank_shared(&shared, a, b, global_freq);
            let anchor = pick_term(&ranked, &earlier.content_terms)
                .or_else(|| later.primary_keyword())
                .unwrap_or_else(|| subject.clone());
            let Some(frag_early) = earlier.fragment_for(&subject, &anchor) else {
                continue;
            };
            let Some(frag_late) = later.fragment_for(&subject, &anchor) else {
                continue;
            };
            push_pair_case(
                cases,
                usage,
                seq,
                "conflict",
                format!("{subject}后来是怎么改的？"),
                a,
                b,
                frag_early,
                frag_late,
            );
        }
    }
}

/// 跨记录题的落袋：排序 `expect_ids`（按时间、id）并同步 `expect_fragments`。
fn push_pair_case(
    cases: &mut Vec<MinedCase>,
    usage: &mut BTreeMap<String, usize>,
    seq: &mut BTreeMap<String, usize>,
    kind: &str,
    query: String,
    a: &Prepared,
    b: &Prepared,
    frag_a: String,
    frag_b: String,
) {
    let mut sides: Vec<(&Prepared, String)> = vec![(a, frag_a), (b, frag_b)];
    sides.sort_by(|x, y| {
        x.0.timestamp_ms
            .cmp(&y.0.timestamp_ms)
            .then_with(|| x.0.id.cmp(&y.0.id))
    });
    let expect_ids: Vec<String> = sides.iter().map(|(p, _)| p.id.clone()).collect();
    let expect_fragments: Vec<String> = sides.into_iter().map(|(_, f)| f).collect();
    for id in &expect_ids {
        *usage.entry(id.clone()).or_insert(0) += 1;
    }
    cases.push(MinedCase {
        id: next_id(seq, kind),
        query,
        expect_ids,
        expect_fragments,
        kind: kind.to_string(),
    });
}

// ============================================================
// 判定辅助
// ============================================================

/// 这条记录还有出题预算吗？
fn budget_left(p: &Prepared, config: MiningConfig, usage: &BTreeMap<String, usize>) -> bool {
    usage.get(&p.id).copied().unwrap_or(0) < config.max_cases_per_record
}

/// 两条记录是否「时间相邻且相关」（同 thread 或共享词足够）。
fn temporal_related(a: &Prepared, b: &Prepared, config: MiningConfig) -> bool {
    if config.temporal_window_ms < 0 {
        // 负窗口 = 禁止任何「时间相邻」（显式配置，不当作无限制）。
        return false;
    }
    let same_thread = same_thread(a, b);
    let shared = shared_terms(a, b);
    if !same_thread && shared.len() < config.min_shared_terms {
        return false;
    }
    a.timestamp_ms.abs_diff(b.timestamp_ms) <= config.temporal_window_ms as u64
}

/// 同 thread（同非空 thread_id）。
fn same_thread(a: &Prepared, b: &Prepared) -> bool {
    match (&a.thread_id, &b.thread_id) {
        (Some(x), Some(y)) => !x.is_empty() && x == y,
        _ => false,
    }
}

/// 两条记录的**内容**是否明显不同（相似度 < 0.5 ⇒ 可能是「同主题不同说法」）。
///
/// 相似度口径：共享词数 / 较短一侧的词数（重叠系数）。
/// 整数比较 `shared * 2 < min_len` 避免浮点误差（0.5 边界判为「相同」⇒ 不算冲突）。
fn content_differs(a: &Prepared, b: &Prepared) -> bool {
    // ⚠️ **必须用主张词（保留数量短语），不能用题面词**：
    // 「阈值定在每秒两千次」vs「阈值定在每秒五千次」**是冲突** ⇒ 数量短语是关键证据；
    // 用题面词会把「同主张的两条」误判成「冲突」（数量词被丢 ⇒ 相似度虚高 ⇒ 判定翻转）。
    let shared = a.claim_terms.intersection(&b.claim_terms).count();
    let min_len = a.claim_terms.len().min(b.claim_terms.len());
    shared.saturating_mul(2) < min_len
}

/// 两条记录共享的主题词（已按 `BTreeSet` 交集 ⇒ 字典序）。
fn shared_terms(a: &Prepared, b: &Prepared) -> Vec<String> {
    a.all_terms.intersection(&b.all_terms).cloned().collect()
}

/// 共享词排序：频次降序 → 标题位置升序 → 全局出现度降序 → 字典序。
///
/// 「标题位置」排前面的理由：标题是人写的摘要，**越靠前越是话题**
/// （`限流阈值定稿` ⇒ `限流` 在前、`阈值` 在后），比纯分词碎片更像人话。
fn rank_shared(
    shared: &[String],
    a: &Prepared,
    b: &Prepared,
    global_freq: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut items: Vec<(usize, usize, usize, &str)> = shared
        .iter()
        .map(|term| {
            let combined = a.content_counts.get(term).copied().unwrap_or(0)
                + b.content_counts.get(term).copied().unwrap_or(0);
            let title_pos = a
                .title_pos
                .get(term)
                .copied()
                .unwrap_or(usize::MAX)
                .min(b.title_pos.get(term).copied().unwrap_or(usize::MAX));
            let global = global_freq.get(term).copied().unwrap_or(0);
            (combined, title_pos, global, term.as_str())
        })
        .collect();
    items.sort_by(|x, y| {
        y.0.cmp(&x.0)
            .then_with(|| x.1.cmp(&y.1))
            .then_with(|| y.2.cmp(&x.2))
            .then_with(|| x.3.cmp(y.3))
    });
    items
        .into_iter()
        .map(|(_, _, _, term)| term.to_string())
        .collect()
}

/// 从候选词里挑第一个「确实出现在该记录正文里」的词（挑不到就用第一个）。
fn pick_term(candidates: &[String], content_terms: &BTreeSet<String>) -> Option<String> {
    candidates
        .iter()
        .find(|t| content_terms.contains(*t))
        .cloned()
        .or_else(|| candidates.first().cloned())
}

/// 按时间排（早的在前）。
fn order_by_time<'a>(a: &'a Prepared, b: &'a Prepared) -> (&'a Prepared, &'a Prepared) {
    if a.timestamp_ms <= b.timestamp_ms {
        (a, b)
    } else {
        (b, a)
    }
}

/// 每个主题词出现在多少条记录里（全局出现度，用于共享词排序）。
fn global_term_freq(prepared: &[Prepared]) -> BTreeMap<String, usize> {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for p in prepared {
        for term in &p.all_terms {
            *freq.entry(term.clone()).or_insert(0) += 1;
        }
    }
    freq
}

/// 候选 id：`gm-{kind}-{序号}`（序号按生成顺序，确定性、无随机数）。
fn next_id(seq: &mut BTreeMap<String, usize>, kind: &str) -> String {
    let counter = seq.entry(kind.to_string()).or_insert(0);
    *counter += 1;
    format!("gm-{kind}-{counter:04}")
}

// ============================================================
// 记录预处理
// ============================================================

/// 预处理后的记录（挖矿期用的索引结构）。
struct Prepared {
    id: String,
    timestamp_ms: i64,
    thread_id: Option<String>,
    /// 切好的句子（`content` 的子串，按句号/换行切）。
    sentences: Vec<String>,
    /// 正文主题词 → 频次。
    content_counts: BTreeMap<String, usize>,
    /// 正文主题词集合（**题面口径**��数量短语已丢）。
    content_terms: BTreeSet<String>,
    /// **主张词**集合（**相似度口径**：保留数量短语）⇒ 判断「是不是同一主张」用。
    claim_terms: BTreeSet<String>,
    /// 标题主题词 → 在标题中首次出现的字符位置。
    title_pos: BTreeMap<String, usize>,
    /// 标题 + 正文的主题词集合（「同主题」判定用）。
    all_terms: BTreeSet<String>,
    /// 正文 top 关键词（已过滤噪声）。
    keys: Vec<String>,
}

impl Prepared {
    fn from_record(record: &MineRecord) -> Self {
        let content_counts = term_counts(&record.content);
        let content_terms: BTreeSet<String> = content_counts.keys().cloned().collect();
        let claim_terms: BTreeSet<String> =
            claim_term_counts(&record.content).keys().cloned().collect();
        let title_pos = record
            .title
            .as_deref()
            .map(title_term_positions)
            .unwrap_or_default();
        let mut all_terms = content_terms.clone();
        all_terms.extend(title_pos.keys().cloned());
        Self {
            id: record.id.clone(),
            timestamp_ms: record.timestamp_ms,
            thread_id: record.thread_id.clone(),
            sentences: split_sentences(&record.content),
            content_counts,
            content_terms,
            claim_terms,
            title_pos,
            all_terms,
            keys: key_terms(&record.content, KEY_TERM_POOL),
        }
    }

    /// 这条记录的**主关键词**（正文 top 关键词里优先选标题也命中的那个）。
    fn primary_keyword(&self) -> Option<String> {
        if self.keys.is_empty() {
            return None;
        }
        for key in &self.keys {
            if self.title_pos.contains_key(key) {
                return Some(key.clone());
            }
        }
        Some(self.keys[0].clone())
    }

    /// **问题主体**：**正文里的实词优先**，标题只作次选。
    ///
    /// # 为什么改成「正文优先」（2026-10-26 实测修正）
    ///
    /// 原先用**标题**（提交主题 / 小说回目联）当主体 ⇒ 题目长成
    /// 「第三十二回　訴肺腑心迷活寶玉后来是怎么改的？」
    /// ⇒ 三个后果（**都实测过**）：
    /// ① **查询词不在正文里** ⇒ 检索只能靠 bigram 偶然命中；
    /// ② 注入按查询选窗口**必然选到无关处** ⇒ 证据族指标**测不出注入好坏**
    ///    （实测注「取开头」0.0210 vs 注「最相关片段」0.0015 ⇒ 方向是反的）；
    /// ③ 题面像「检索结果的目录名」，**不像人话**（人会问「寶玉和黛玉後來怎麼樣」）。
    ///
    /// ⇒ 主体改成**正文里真实出现的词**（人在正文里能看到的实体/关键词）
    /// ⇒ 这样查询与注入**指向同一片正文** ⇒ 检索与注入**联动**。
    ///
    /// ⚠️ **这不是泄漏**：泄漏指「**题面来源被逐字复制进可检索文本**」；
    /// 而「人话里的实体词**本来就在正文里**」⇒ 检索命中它是**正常的**
    /// （正如人问「宝玉怎么了」能检索到提到宝玉的章节）。
    /// **问题主体**：**必须来自正文**。
    ///
    /// # 为什么**只认正文**（2026-10-26 实测两次修正）
    ///
    /// ① 原先用**标题**（提交主题 / 小说回目联）⇒ 题面像「检索结果的目录名」，
    ///    且查询词**不在正文里** ⇒ 检索靠偶然命中、注入按查询选窗口**必然选错**
    ///    ⇒ 证据族指标**测不出注入好坏**。
    /// ② 改用标题的**回次部分**（「第三回」）⇒ 更糟：**回次不在正文里**
    ///    ⇒ 连答案片段都找不到（`fragment_for` 按主体找句子）⇒ 题是假的。
    ///
    /// ⇒ **主体只能来自正文**（人话里的实体**本来就在正文里**）
    /// ⇒ 找不到合格实体 ⇒ **返回 `None`** ⇒ 这条记录**不出题**
    /// （宁可少出题，不要出假题）。
    fn subject(&self) -> Option<String> {
        self.top_entity()
    }

    /// 这条记录的**实体频次门槛**（按正文长度自适应）。
    fn entity_min_count(&self) -> usize {
        let body_len = self
            .sentences
            .iter()
            .map(|s| s.chars().count())
            .sum::<usize>();
        if body_len < SHORT_BODY_CHARS {
            ENTITY_MIN_COUNT_SHORT
        } else {
            ENTITY_MIN_COUNT
        }
    }

    /// **某一句里**最像「实体」的词（**必须出现在这一句里**）。
    ///
    /// 用途：决策题的主体（决策句里的那个「事情」）。
    fn sentence_subject(&self, sentence: &str) -> Option<String> {
        let counts = term_counts(sentence);
        let mut best: Option<(&String, usize)> = None;
        for (term, count) in &counts {
            if term.chars().count() < 2
                || generic_set().contains(term.as_str())
                || stopword_set().contains(term.as_str())
                || !term.chars().all(|ch| ch.is_alphanumeric())
                || CHINESE_NUMERALS.contains(term.chars().next().unwrap_or('\0'))
            {
                continue;
            }
            match best {
                Some((best_term, best_count)) => {
                    if *count > best_count || (*count == best_count && term < best_term) {
                        best = Some((term, *count));
                    }
                }
                None => best = Some((term, *count)),
            }
        }
        best.map(|(term, _)| term.clone())
    }

    /// 正文里最像「实体」的那个词（**必须在正文里出现**）。
    ///
    /// 口径：在正文词池里挑**出现次数最多**的（并列按字典序 ⇒ 确定性）；
    /// 且**要求它在正文里以「词」形态出现**（不是只出现在标题里）。
    fn top_entity(&self) -> Option<String> {
        // ① 候选：正文词池里**不是泛称**、不是停用词、不是数量短语的词。
        let candidates: Vec<(&String, usize)> = self
            .content_counts
            .iter()
            .filter(|(term, count)| {
                term.chars().count() >= 2
                    && !generic_set().contains(term.as_str())
                    && !stopword_set().contains(term.as_str())
                    // ⚠️ **非字母数字的 token 一律不算实体**（2026-10-26 实测）：
                    // 古本的**分隔线**会被 OCR 成 `____` ⇒ 它是全篇高频词
                    // ⇒ 不滤掉就会挖出「____后来是怎么改的？」这种**假主体**。
                    && term.chars().all(|ch| ch.is_alphanumeric())
                    // ⚠️ **以数量字开头的 bigram 是跨词边界的碎片**（2026-10-26 实测）：
                    // 「一場好戲」被切成「一場」+「場好」⇒ 「一場」「一案」既在正文里
                    // （真的）**又是碎片** ⇒ 题面变成「关于一場，我们当时记了什么？」——不像人话。
                    // ⇒ 排除「首字是数量字」的 bigram（数字词不是话题）。
                    && !CHINESE_NUMERALS.contains(term.chars().next().unwrap_or('\0'))
                    // ⚠️ **频次门槛**：碎片（跨词边界的 bigram）通常只出现一两次
                    // ⇒ 低于门槛的词不配当主体（实测：碎片 1~3 次 vs 实体数百次）。
                    // ⇒ 门槛**按正文长度自适应**（短正文放宽 ⇒ 否则短记录全出不了题）。
                    && **count >= self.entity_min_count()
            })
            .map(|(term, count)| (term, *count))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        // ② 选「最像实体」的那个：
        //    **长度优先，其次频次**。
        //
        //    为什么长度优先（2026-10-26 实测）：bigram 切法会把「盧俊義」切成
        //    「盧俊」+「俊義」⇒ 只按频次会选到「俊義」（丢了姓）
        //    ⇒ 题目变成「俊義后来是怎么改的？」（半个人名）。
        //    而**长词**（≥3 字且高频）更可能是**完整人名/地名/术语**。
        //
        //    口径：先取**最长**（3 字优先于 2 字），同长度取**频次高**的，
        //    再并列按字典序 ⇒ **确定性**。
        let mut best: Option<(&String, usize, usize)> = None;
        for (term, count) in candidates {
            let len = term.chars().count();
            match best {
                Some((best_term, best_count, best_len)) => {
                    let better = len > best_len
                        || (len == best_len
                            && (count > best_count || (count == best_count && term < best_term)));
                    if better {
                        best = Some((term, count, len));
                    }
                }
                None => best = Some((term, count, len)),
            }
        }
        best.map(|(term, _, _)| term.clone())
    }

    /// 含决策标记词的那一句（取第一句命中的）。
    fn decision_sentence(&self) -> Option<(String, String)> {
        for sentence in &self.sentences {
            for marker in DECISION_MARKERS {
                if sentence.contains(marker) {
                    return Some((marker.to_string(), sentence.clone()));
                }
            }
        }
        None
    }

    /// 取一条**确实出现在正文里**的答案片段（≤ [`FRAGMENT_MAX_CHARS`] 字）。
    ///
    /// 优先取含 `subject` 的第一句；没有就退到含 `anchor` 的第一句；再没有就取第一句
    /// —— 三种情况都保证片段是 `content` 的子串。
    /// 取一条**确实出现在正文里**的答案片段（≤ [`FRAGMENT_MAX_CHARS`] 字）。
    ///
    /// ⚠️ **不做「取首句」兜底**（2026-10-26 口径修正）：
    /// 兜底会让**期望片段系统性地偏向记录开头** ⇒
    /// 「注开头 120 字」的注入口径天然占优 ⇒ **证据族指标测不出注入好坏**
    /// （实测：注「取开头」0.0210 vs 注「最相关片段」0.0015 ⇒ 方向是反的）。
    ///
    /// ⇒ **找不到真凭实据就返回 `None`**（这条题没有片段标注）
    /// ⇒ 与全仓纪律一致：「**不静默保留假的**」——
    /// 一个凭空取来的首句**不是证据**，只是让指标好看。
    fn fragment_for(&self, subject: &str, anchor: &str) -> Option<String> {
        let chosen = self
            .sentences
            .iter()
            .find(|sentence| sentence.contains(subject))
            .or_else(|| {
                self.sentences
                    .iter()
                    .find(|sentence| sentence.contains(anchor))
            })?;
        Some(clip_around(chosen, subject, FRAGMENT_MAX_CHARS))
    }
}

// ============================================================
// 文本处理
// ============================================================

/// 主题词计数（过滤噪声 token：停用词 / 纯数字 / 过短 / 含虚词字的碎片 bigram）。
fn term_counts(text: &str) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for tok in raw_tokens(text) {
        if is_noise_token(&tok) {
            continue;
        }
        *counts.entry(tok).or_insert(0) += 1;
    }
    counts
}

/// **主张词**计数（用于**相似度判断**：两条记录是不是在讲同一件事）。
///
/// 与 [`term_counts`] 的差别：**保留中文数量短语**（理由见 [`is_claim_noise_token`]）。
fn claim_term_counts(text: &str) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for tok in raw_tokens(text) {
        if is_claim_noise_token(&tok) {
            continue;
        }
        *counts.entry(tok).or_insert(0) += 1;
    }
    counts
}

/// 这个 token 是不是噪声（不进**题面**关键词池）。
fn is_noise_token(token: &str) -> bool {
    is_noise_token_impl(token, true)
}

/// 这个 token 是不是噪声（用于**相似度判断**的词池）。
///
/// ⚠️ **与 [`is_noise_token`] 的唯一差别 = 不丢中文数量短语**。
///
/// **为什么必须分开**：题面里「关于两千」不像人话 ⇒ 数量词该从**题面**里丢；
/// 但「阈值定在每秒两千次」vs「阈值定在每秒五千次」**恰恰是冲突**
/// ⇒ 数量词对「这两条是不是在讲同一主张」是**关键证据**，丢了就误判。
/// ⇒ 两种用途必须用两套词池。
fn is_claim_noise_token(token: &str) -> bool {
    is_noise_token_impl(token, false)
}

/// 噪声判定的共同实现。
///
/// `filter_quantities` = 是否丢掉中文数量短语（题面口径要丢，相似度口径不能丢）。
fn is_noise_token_impl(token: &str, filter_quantities: bool) -> bool {
    let len = token.chars().count();
    if len < 2 {
        return true;
    }
    if stopword_set().contains(token) {
        return true;
    }
    // 纯数字（年份、版本号）不当问题关键词。
    if token.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // 含虚词字的 CJK bigram = 跨词边界的碎片。
    if is_cjk_token(token) && token.chars().any(|c| FUNCTION_CHARS.contains(c)) {
        return true;
    }
    // 中文数量短语（**数量不是话题**）⇒ 丢掉，否则会挖出「关于两千……」这种不像人话的题。
    //
    // 两条规则，**刻意收窄**（只拦「纯数词」与「数词+量词」）：
    // * 纯数词：「两千」「三万」；
    // * 数词+量词：「三次」「十毫」。
    //
    // ⚠️ **为什么不是「含数词就丢」**：本仓的领域词汇大量是**编号标签**
    // （「五层」「六层」「D0–D5」「三维」）⇒ 那样会误伤真问题主体。
    // 这里只丢「读起来就是个数量」的 ⇒ 编号标签照常可当主题。
    if filter_quantities && is_cjk_token(token) {
        let mut chars = token.chars();
        let first = chars.next().unwrap_or('\0');
        let second = chars.next();
        let all_numerals = token.chars().all(|c| CHINESE_NUMERALS.contains(c));
        let numeral_plus_quantifier = CHINESE_NUMERALS.contains(first)
            && second.is_some_and(|c| QUANTIFIER_CHARS.contains(c));
        if all_numerals || numeral_plus_quantifier {
            return true;
        }
    }
    false
}

/// 是不是 CJK token（全部字符都是 CJK 汉字 ⇒ 视为 bigram 候选）。
fn is_cjk_token(token: &str) -> bool {
    !token.is_empty() && token.chars().all(is_cjk)
}

/// 标题里每个主题词首次出现的字符位置（标题是人写的摘要 ⇒ 位置越前越像话题）。
fn title_term_positions(title: &str) -> BTreeMap<String, usize> {
    let mut positions: BTreeMap<String, usize> = BTreeMap::new();
    for term in raw_tokens(title) {
        if is_noise_token(&term) {
            continue;
        }
        match title.find(&term) {
            Some(byte) => {
                positions
                    .entry(term)
                    .or_insert_with(|| title[..byte].chars().count());
            }
            None => {
                positions.entry(term).or_insert(usize::MAX);
            }
        }
    }
    positions
}

/// 切句：按句号/问号/叹号/分号/换行切；片段仍是 `content` 的子串。
fn split_sentences(content: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in content.chars() {
        if is_sentence_break(ch) {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

/// 句子边界字符。
fn is_sentence_break(ch: char) -> bool {
    matches!(
        ch,
        '。' | '！' | '？' | '!' | '?' | '；' | ';' | '\n' | '\r'
    )
}

/// 截断到 ≤ `max_chars` 字，并尽量让 `term` 落在窗口里（保证片段仍能回答问题）。
fn clip_around(sentence: &str, term: &str, max_chars: usize) -> String {
    let chars: Vec<char> = sentence.chars().collect();
    if chars.len() <= max_chars {
        return sentence.to_string();
    }
    let term_pos = find_char_pos(sentence, term);
    let start = match term_pos {
        Some(pos) => pos
            .saturating_sub(15)
            .min(chars.len().saturating_sub(max_chars)),
        None => 0,
    };
    let mut end = (start + max_chars).min(chars.len());
    if let Some(pos) = term_pos {
        let term_end = pos + term.chars().count();
        if end < term_end {
            end = term_end.min(chars.len());
        }
    }
    if end - start > max_chars {
        // 词本身超长 ⇒ 退化为前缀截断（仍 ≤ max_chars）。
        return chars[..max_chars].iter().collect();
    }
    chars[start..end].iter().collect()
}

/// 词在句子里首次出现的**字符下标**。
fn find_char_pos(sentence: &str, term: &str) -> Option<usize> {
    let byte_pos = sentence.find(term)?;
    Some(sentence[..byte_pos].chars().count())
}

/// 原始 token 流（**保留重复**，计数用）。
fn raw_tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if is_cjk(ch) {
            // 连续 CJK 段 ⇒ 两两组合成 bigram。
            let start = i;
            while i < chars.len() && is_cjk(chars[i]) {
                i += 1;
            }
            let run = &chars[start..i];
            if run.len() >= 2 {
                for window in run.windows(2) {
                    out.push(window.iter().collect());
                }
            }
        } else if is_word_char(ch) {
            // ASCII 词（字母数字下划线），≥2 字符才算。
            let start = i;
            while i < chars.len() && is_word_char(chars[i]) {
                i += 1;
            }
            if i - start >= 2 {
                out.push(chars[start..i].iter().collect());
            }
        } else {
            // 标点/空白：丢弃（同时是 token 边界）。
            i += 1;
        }
    }
    out
}

/// 是不是 CJK 汉字（基本区 + 扩展 A + 兼容区 + 扩展 B–F）。
fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x3400..=0x4DBF).contains(&c)
        || (0x4E00..=0x9FFF).contains(&c)
        || (0xF900..=0xFAFF).contains(&c)
        || (0x20000..=0x2FA1F).contains(&c)
}

/// 是不是 ASCII 词字符（字母、数字、下划线）。
fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
