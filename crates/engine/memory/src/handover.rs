//! `apeireth-memory::handover` — **handover 演练**（「换人接手」的可验证版本）。
//!
//! # 「handover」到底是什么
//!
//! 不是「写份文档」，而是：**把库交给一个不认识它的人**，对方只靠**包里带的说明**
//! 就能（a）读懂这些数据是什么、（b）知道现在处于什么状态、（c）知道接下来该做什么、
//! （d）知道**哪些东西不许碰**。
//!
//! ⇒ 本模块把它变成**机器可验**的：产出一个 **handover 包**（数据 + 说明 + 状态 + 下一步 + 红线），
//! 并提供**一个「接手者」只能靠包内信息完成任务的演练**。
//!
//! # 为什么不是「写份文档」
//!
//! 文档只能被**读**，不能被**验** ⇒ 三个月后没人说得清「当时的库到底什么样」，
//! 更说不清「当时的说法有没有夸大」。⇒ 本模块的判据全是**机器可判**的：
//!
//! * 字段说明**覆盖包内数据的全部字段**（漏一个字段 ⇒ 判失败）；
//! * 状态里的条数**与归档物里的条数相等**（不等 ⇒ 判失败 ⇒ 接手者不会被误导）；
//! * 每条下一步都有**可跑的验收判据**（没判据 ⇒ 判失败 ⇒ 那不叫下一步）；
//! * 红线里**必须含 P1 的「不删原始证据」**（缺 ⇒ 判失败）。
//!
//! # 为什么包要**自带归档物**（而不是只给路径）
//!
//! 真实场景里「交接」经常发生在**原库已经没了 / 拿不到**的时候（机器坏了、换人没权限、
//! 公司注销）。⇒ 包里直接带 [`OfflineArchive`]（数据本体 + 自说明 + 可独立校验）
//! ⇒ 接手者**不依赖原库**也能读、能核对、能接着干。
//!
//! # B4 的位置（别越界宣称）
//!
//! 设计稿 §8 **B4** = 脱机归档（**保原文**）+ 迁移回滚 + **handover 演练**。
//! 本模块做的是 B4 的**第三块砖**（handover 演练）；
//! **迁移回滚**那一块**尚无实现** ⇒ **不宣称 B4 已完成**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27 首版）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：包 + 演练 + 报告齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（无 CLI、无定时任务、生产 0 调用点）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：已在**真实语料**（《紅樓夢》120 回）上跑过对照，
//!   但**未在生产记忆库上实测**（本机无生产库）⇒ 不宣称已验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use crate::offline_archive::{verify_archive, OfflineArchive};

/// handover 包的格式版本（**改了结构就要升** ⇒ 接手者能判断这份包能不能读）。
pub const HANDOVER_PACKAGE_VERSION: &str = "apeireth-handover/v1";

/// P1 红线的**规范措辞**（演练按**逐字**判定）。
///
/// **为什么逐字**：红线必须**机器可检** ⇒ 换词就判失败，是**故意的**：
/// 一条「差不多意思」的红线，在真出事的时候等于没有。
pub const P1_RED_LINE_MARKER: &str = "不删原始证据";

/// 包内数据的**全部字段**（⇒ 演练的「字段说明覆盖」判据）。
///
/// # 口径
///
/// * **记录的字段**（[`crate::offline_archive::ArchivedEpisode`]）12 个；
/// * **链头的字段**（[`crate::offline_archive::ArchivedChainHead`]）4 个；
/// * **清单的标量字段**（[`crate::offline_archive::ArchiveManifest`]）6 个；
/// * `field_docs` **不在表里** —— 它**就是**那张说明表自己，说明表不需要说明自己
///   （自指 ⇒ 无限 regress）。
pub const REQUIRED_FIELD_DOCS: &[&str] = &[
    // 记录的字段
    "id",
    "continuity_id",
    "session_id",
    "timestamp",
    "role",
    "content",
    "valid_from_ms",
    "valid_until_ms",
    "created_ms",
    "provenance",
    "chain_hash",
    "chain_prev_hash",
    // 链头的字段
    "subject_id",
    "last_hash",
    "entry_count",
    "updated_at_ms",
    // 清单的标量字段
    "format_version",
    "created_at_ms",
    "episode_count",
    "chain_head_count",
    "unmanaged_count",
    "content_digest",
];

/// handover 包里的**数据引用**（**不复制大对象** ⇒ 包小、可传）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DataRef {
    /// 什么数据（如「记忆导出（JSONL）」）。
    pub what: String,
    /// 在哪（如文件路径或「见附件」）。
    pub where_: String,
    /// **怎么读它**（一行命令或格式说明）⇒ 接手者**不用猜**。
    pub how_to_read: String,
    /// **它的可信度**（如「保原文，已独立校验」/「未纳管 N 条」）。
    pub trust: String,
}

/// **当前状态**（接手者**必须知道现在怎么样**）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CurrentState {
    /// 库路径。
    pub db_path: String,
    /// 记录条数。
    pub episode_count: usize,
    /// 主体数（链数）。
    pub subject_count: usize,
    /// **未纳管**条数（**如实** ⇒ 接手者知道账不完整）。
    pub unmanaged_count: usize,
    /// **当前用的检索算法**（若有）。
    pub current_algorithm: Option<String>,
    /// 已知的**问题**（如「recall@10 = 0.2983，瓶颈是排序」）⇒ **不隐瞒坏消息**。
    pub known_issues: Vec<String>,
}

/// **下一步**（接手者**马上能做的事**）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NextStep {
    /// 做什么。
    pub what: String,
    /// 为什么（**接上下文**用的）。
    pub why: String,
    /// 怎么验收（**可跑的**命令或判据）。
    pub acceptance: String,
    /// 大致工作量（`1` = 几小时；`3` = 几天）。
    pub cost: u8,
}

/// **红线**（**不许碰**的东西）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedLine {
    /// 不许做什么。
    pub forbidden: String,
    /// 为什么。
    pub because: String,
}

/// 一个完整的 **handover 包**。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HandoverPackage {
    /// 包的版本（改了结构就要升）。
    pub version: String,
    /// 生成时刻（epoch 毫秒；**注入** ⇒ 确定性）。
    pub created_at_ms: i64,
    /// 数据引用清单。
    pub data: Vec<DataRef>,
    /// 当前状态。
    pub state: CurrentState,
    /// 下一步清单。
    pub next_steps: Vec<NextStep>,
    /// 红线清单。
    pub red_lines: Vec<RedLine>,
    /// **归档快照**（数据本体 ⇒ 接手者**不依赖原库**也能读）。
    pub archive: OfflineArchive,
}

/// 造一个 handover 包。
///
/// # 口径
///
/// * `known_issues` **不许空着好看** —— 调用方给什么就写什么（**不粉饰**）；
/// * `red_lines` **必须**包含仓库既有的硬约束（见 [`default_red_lines`]）
///   ⇒ 本函数**直接**用 [`default_red_lines`] 填（调用方没得选，这是纪律）；
/// * **确定性**：`created_at_ms` 注入 ⇒ 同输入 ⇒ 逐字相等。
///
/// # 数据引用是怎么来的
///
/// 本函数**不接受** `data` 参数 ⇒ 它**从归档物与状态里推导**出两条数据引用：
///
/// 1. **包内归档物**（随包走 ⇒ 不依赖原库）；
/// 2. **原始记忆库**（`state.db_path` ⇒ 权威层还在时可直接查）。
///
/// 这样做的理由：handover 包的价值就在于「**对方不用问我们**」⇒ 数据清单必须**自带**。
pub fn build_package(
    archive: OfflineArchive,
    state: CurrentState,
    next_steps: Vec<NextStep>,
    created_at_ms: i64,
) -> HandoverPackage {
    let data = default_data_refs(&state, &archive);
    HandoverPackage {
        version: HANDOVER_PACKAGE_VERSION.to_string(),
        created_at_ms,
        data,
        state,
        next_steps,
        red_lines: default_red_lines(),
        archive,
    }
}

/// **默认红线**（仓库既有的硬约束，**每份包都必须带上**）。
///
/// # 内容（照本仓纪律）
///
/// * **不删原始证据**（P1：append-only）；
/// * **不改已固化的基线**（准确度不许掉 ⇒ 基线是地板）；
/// * **不把「库级已实现」写成「生产已接线」**（四级口径）；
/// * **不猜着读数据**（未知格式显式拒）。
///
/// # 为什么是这四条
///
/// 前两条是**数据面**的（数据与基线是接手者的地基）；
/// 后两条是**说法面**的（接手者最容易被「已实现 / 已上线」这类话误导 ⇒ 真出事时他去动生产路径）。
pub fn default_red_lines() -> Vec<RedLine> {
    vec![
        RedLine {
            forbidden: P1_RED_LINE_MARKER.to_string(),
            because: "原则 P1（数据保真优先）：原始证据（原文+结构+时间+出处+关系+上下文+访问史）永久保留；\
                      向量等派生层可丢可重建，原始证据删了就回不来。"
                .to_string(),
        },
        RedLine {
            forbidden: "不改基线（= 不改已固化的基线：准确度不许掉 ⇒ 基线是地板）".to_string(),
            because: "已固化的准确度基线是**地板**：改了它 ⇒ 后面再也分不清「改动导致掉分」还是「本来就低」\
                      ⇒ 基线只许追加（留历史），不许改写。"
                .to_string(),
        },
        RedLine {
            forbidden: "口径不混写：不把「库级已实现」写成「生产已接线」（四级口径）".to_string(),
            because: "四级口径（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED）各是一级；\
                      混写 ⇒ 接手者以为已经上线 ⇒ 去改生产路径 ⇒ 出事故。"
                .to_string(),
        },
        RedLine {
            forbidden: "不猜着读数据（未知格式显式拒）".to_string(),
            because: "原则 P4（格式演进）：schema 版本化 + 未知版本显式拒；猜着读 ⇒ 把别的格式当自己的 \
                      ⇒ 静默写坏数据。"
                .to_string(),
        },
    ]
}

/// **接手者演练**：一个**只拿到包**（没有原库、没有上下文）的接手者，能不能完成任务？
///
/// # 口径（**这是本批的核心**）
///
/// * 演练检查**四件事**（每件都能自动判定）：
///   ① **能读懂**：包里的字段说明非空、且覆盖了包内数据的**全部**字段；
///   ② **知道现状**：`state.episode_count` 与归档里的条数**一致**；
///   ③ **知道下一步**：`next_steps` 非空，且**每条都有可跑的验收判据**；
///   ④ **知道红线**：`red_lines` 非空，且包含 P1 的「不删原始证据」；
/// * 返回每项的通过与否 + 人话说明。
///
/// # 返回
///
/// `(全部通过?, 逐项说明)`
///
/// 逐项说明**恒为 4 条**（一项一条，顺序固定）⇒ 接手者能直接看到「哪一项没过、为什么」。
///
/// # 为什么①里还查「数据引用」
///
/// 「能读懂」不只是「字段名有解释」，还包括「**东西在哪、怎么读**」——
/// 数据引用里的 [`DataRef::how_to_read`] 就是干这个的 ⇒ 缺了它，接手者第一件事就得来问人。
pub fn drill_handover(package: &HandoverPackage) -> (bool, Vec<String>) {
    let mut notes: Vec<String> = Vec::new();
    let mut all_passed = true;

    // ① 能读懂：字段说明覆盖全部字段 + 每条数据引用都说清了「是什么/在哪/怎么读/可信度」。
    let mut problems: Vec<String> = Vec::new();
    let docs = &package.archive.manifest.field_docs;
    if docs.is_empty() {
        problems.push("字段说明表是空的".to_string());
    }
    for field in REQUIRED_FIELD_DOCS {
        match docs.get(*field) {
            None => problems.push(format!("字段 `{field}` 没有说明")),
            Some(text) if text.trim().is_empty() => {
                problems.push(format!("字段 `{field}` 的说明是空串"))
            }
            Some(_) => {}
        }
    }
    if package.data.is_empty() {
        problems.push("数据引用清单是空的（接手者不知道数据在哪）".to_string());
    }
    for (index, item) in package.data.iter().enumerate() {
        let mut missing: Vec<&str> = Vec::new();
        if item.what.trim().is_empty() {
            missing.push("是什么");
        }
        if item.where_.trim().is_empty() {
            missing.push("在哪");
        }
        if item.how_to_read.trim().is_empty() {
            missing.push("怎么读");
        }
        if item.trust.trim().is_empty() {
            missing.push("可信度");
        }
        if !missing.is_empty() {
            problems.push(format!(
                "第 {} 条数据引用缺「{}」",
                index + 1,
                missing.join("、")
            ));
        }
    }
    if problems.is_empty() {
        notes.push(format!(
            "✅ ① 能读懂：字段说明覆盖包内数据的全部 {} 个字段；{} 条数据引用都写清了「是什么/在哪/怎么读/可信度」",
            REQUIRED_FIELD_DOCS.len(),
            package.data.len()
        ));
    } else {
        all_passed = false;
        notes.push(format!("❌ ① 能读懂：{}", problems.join("；")));
    }

    // ② 知道现状：状态里的条数必须与归档物里的条数**一致**（不一致 ⇒ 接手者被误导）。
    let archived = package.archive.episodes.len();
    if package.state.episode_count == archived {
        notes.push(format!(
            "✅ ② 知道现状：状态里的记录条数 {} == 包内归档条数 {}",
            package.state.episode_count, archived
        ));
    } else {
        all_passed = false;
        notes.push(format!(
            "❌ ② 知道现状：状态里的记录条数 {} ≠ 包内归档条数 {}（账实不符 ⇒ 接手者会被误导）",
            package.state.episode_count, archived
        ));
    }

    // ③ 知道下一步：非空，且每条都有**可跑的验收判据**（没判据的不算下一步）。
    if package.next_steps.is_empty() {
        all_passed = false;
        notes.push("❌ ③ 知道下一步：下一步清单是空的（接手者不知道该干什么）".to_string());
    } else {
        let mut broken: Vec<String> = Vec::new();
        for (index, step) in package.next_steps.iter().enumerate() {
            let mut missing: Vec<&str> = Vec::new();
            if step.what.trim().is_empty() {
                missing.push("做什么");
            }
            if step.acceptance.trim().is_empty() {
                missing.push("验收判据");
            }
            if !missing.is_empty() {
                broken.push(format!("第 {} 条（缺{}）", index + 1, missing.join("、")));
            }
        }
        if broken.is_empty() {
            notes.push(format!(
                "✅ ③ 知道下一步：{} 条下一步，每条都有可跑的验收判据",
                package.next_steps.len()
            ));
        } else {
            all_passed = false;
            notes.push(format!(
                "❌ ③ 知道下一步：{} —— 没验收判据的下一步不算下一步",
                broken.join("；")
            ));
        }
    }

    // ④ 知道红线：非空，且含 P1 的「不删原始证据」。
    if package.red_lines.is_empty() {
        all_passed = false;
        notes.push("❌ ④ 知道红线：红线清单是空的（接手者不知道什么不许碰）".to_string());
    } else if !package
        .red_lines
        .iter()
        .any(|line| line.forbidden.contains(P1_RED_LINE_MARKER))
    {
        all_passed = false;
        notes.push(format!(
            "❌ ④ 知道红线：{} 条红线，但**缺 P1 的「{}」**",
            package.red_lines.len(),
            P1_RED_LINE_MARKER
        ));
    } else {
        notes.push(format!(
            "✅ ④ 知道红线：{} 条红线，含 P1 的「{}」",
            package.red_lines.len(),
            P1_RED_LINE_MARKER
        ));
    }

    (all_passed, notes)
}

/// 人话报告（包的摘要 ⇒ 接手者**先看这个**）。
///
/// # 口径
///
/// * **如实**：已知问题一条不藏（没有就明说「没给」⇒ **不代填**）；
/// * **可 grep**：条数、条数、条数都在明面上（谁接手都能一眼看到账）。
pub fn render_package_summary(package: &HandoverPackage) -> String {
    let state = &package.state;
    let mut out = String::new();

    out.push_str("════════ handover 包摘要 ════════\n");
    out.push_str(&format!("版本：{}\n", package.version));
    out.push_str(&format!(
        "生成时刻：{} ms（epoch 毫秒；注入 ⇒ 可复现）\n",
        package.created_at_ms
    ));

    out.push_str("\n── 当前状态 ──\n");
    out.push_str(&format!("库路径：{}\n", state.db_path));
    out.push_str(&format!(
        "记录条数：{}　主体数：{}　未纳管：{}\n",
        state.episode_count, state.subject_count, state.unmanaged_count
    ));
    out.push_str(&format!(
        "当前检索算法：{}\n",
        state
            .current_algorithm
            .as_deref()
            .unwrap_or("未设定（走默认路径）")
    ));
    out.push_str(&format!("已知问题（{} 条）：\n", state.known_issues.len()));
    if state.known_issues.is_empty() {
        out.push_str("  （调用方没给已知问题 ⇒ **不代填**：接手者须自己查，别当成「没问题」）\n");
    } else {
        for (index, issue) in state.known_issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", index + 1, issue));
        }
    }

    out.push_str(&format!("\n── 数据引用（{} 条）──\n", package.data.len()));
    for item in &package.data {
        out.push_str(&format!("  · {}\n", item.what));
        out.push_str(&format!("    在哪：{}\n", item.where_));
        out.push_str(&format!("    怎么读：{}\n", item.how_to_read));
        out.push_str(&format!("    可信度：{}\n", item.trust));
    }

    out.push_str(&format!(
        "\n── 下一步（{} 条）──\n",
        package.next_steps.len()
    ));
    for (index, step) in package.next_steps.iter().enumerate() {
        out.push_str(&format!(
            "  {}. [成本 {}] {}\n",
            index + 1,
            step.cost,
            step.what
        ));
        out.push_str(&format!("     为什么：{}\n", step.why));
        out.push_str(&format!("     验收：{}\n", step.acceptance));
    }

    out.push_str(&format!("\n── 红线（{} 条）──\n", package.red_lines.len()));
    for (index, line) in package.red_lines.iter().enumerate() {
        out.push_str(&format!("  {}. 不许：{}\n", index + 1, line.forbidden));
        out.push_str(&format!("     因为：{}\n", line.because));
    }

    out.push_str("\n── 包内归档物（随包走 ⇒ 不依赖原库）──\n");
    let manifest = &package.archive.manifest;
    out.push_str(&format!("格式版本：{}\n", manifest.format_version));
    out.push_str(&format!(
        "记录条数：{}　链头条数：{}　未纳管：{}\n",
        manifest.episode_count, manifest.chain_head_count, manifest.unmanaged_count
    ));
    out.push_str(&format!("内容哈希：{}\n", manifest.content_digest));
    out.push_str(&format!(
        "字段说明：{} 条（自说明 ⇒ 随物走）\n",
        manifest.field_docs.len()
    ));

    out
}

/// **默认数据引用**（从归档物与状态里推导 ⇒ 不用调用方手写）。
///
/// # 为什么自己推导
///
/// handover 包的价值在「**对方不用问我们**」⇒ 数据清单必须自带；
/// 而「怎么读」「可信度」两栏能从归档物本身算出（`to_jsonl` 的用法 + 独立校验的结果）
/// ⇒ 让调用方手写只会写出「见附件」这种**没有信息量**的话。
fn default_data_refs(state: &CurrentState, archive: &OfflineArchive) -> Vec<DataRef> {
    // 独立校验的**实测**结果写进「可信度」⇒ 接手者不必自己先验一遍才知道能不能信。
    let verified = match verify_archive(archive) {
        Ok(()) => "已独立校验通过（条数 + 内容哈希 + 链账核对）".to_string(),
        Err(reason) => format!("⚠️ 独立校验**不通过**：{reason}（接手者须先查清再往下走）"),
    };

    vec![
        DataRef {
            what: "记忆导出（JSONL：第 1 行是 manifest，其后逐条记录 + 逐条链头）".to_string(),
            where_: "本包的 archive 字段（**随包走** ⇒ 原库没了也能读）".to_string(),
            how_to_read:
                "apeireth_memory::offline_archive::to_jsonl(&package.archive) ⇒ 逐行 JSON；\
                 或直接读 package.archive.episodes / package.archive.chain_heads"
                    .to_string(),
            trust: format!(
                "保原文（逐字导出：不摘要 / 不截断 / 不改写）；{verified}；未纳管 {} 条（如实标出，不在哈希链保护范围内）",
                archive.manifest.unmanaged_count
            ),
        },
        DataRef {
            what: "原始记忆库（权威层 D0）".to_string(),
            where_: state.db_path.clone(),
            how_to_read:
                "sqlite3 <库路径> \"SELECT id, continuity_id, session_id, timestamp, role, content, chain_hash \
                 FROM episodes ORDER BY timestamp, id\"（**只读**打开）"
                    .to_string(),
            trust: "权威层：派生层（D1–D5）可丢可重建；此库若已损坏，以包内归档物为准（保原文 + 可独立校验）"
                .to_string(),
        },
    ]
}
