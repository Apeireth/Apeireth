//! `apeireth-memory::swap_flow` — 算法切换流程（**零风险上线 / 可回退**）。
//!
//! # 三步流程（**不许跳步**）
//!
//! ① **影子双跑**（[`crate::shadow_run::run_shadow`]）⇒ 候选算法在**影子**里跑，
//!    结果**不外泄**；
//! ② **裁决**（[`crate::shadow_run::judge_swap`]）⇒ 只有 `Approved` / `ApprovedWithGain`
//!    才允许进入第 ③ 步；
//! ③ **切换 + 归档**：写「**当前用哪个算法**」的指针 + 把评测报告**归档**（落盘）。
//!
//! # `Swap cost` 是什么
//!
//! **换算法要付出多少**（设计稿 L4 指标）。本模块度量**三样**：
//! * **时间成本**：从「决定换」到「切换完成」经过的毫秒；
//! * **风险成本**：切换前跑了几轮影子、样本多少题（**样本越少 ⇒ 风险越高**）；
//! * **回退成本**：回退需要多久（**回退必须比切换更快** ⇒ 否则不许换）。
//!
//! # 三道闸（**不许跳**）
//!
//! 1. **裁决闸**：`Rejected` ⇒ 不许换（没证据 ≠ 没变差）；
//! 2. **回退速度闸**：`rollback_ms >= switch_ms` ⇒ 不许换
//!    （**回退不比切换快 ⇒「可回退」是假的** ⇒ 零风险上线这句话就不成立）；
//! 3. **样本充分闸**：`shadow_runs >= 2`（一轮 ⇒ 只是运气）且 `shadow_cases >= 10`
//!    （题太少 ⇒ 结论没意义）。
//!
//! ⚠️ 三道闸**全过**才允许真的切指针 ⇒ 任何一道不过 ⇒ **什么都不做**
//! （指针逐字不变、**不落盘**、如实返回 `switched = false` + 人话理由）。
//!
//! # `Swap cost` 的**毫秒**是模型估算，不是 wall-clock 实测（诚实交代）
//!
//! 契约要求 `now_ms` **注入**（不许在函数里读系统时钟）且归档内容**逐字可复现**
//! ⇒ 实测耗时**进不了归档**（同一份输入两次跑，墙钟耗时必然不同 ⇒ 归档就不可能逐字相同）。
//! ⇒ 本模块用**确定性成本模型**给出毫秒数：
//!
//! ```text
//! switch_ms   = shadow_runs × shadow_cases × 每条影子样本等效毫秒 + 切换落盘等效毫秒
//! rollback_ms = 回退落盘等效毫秒
//! ```
//!
//! * 「等影子证据」计入切换成本：轮数×题数越大 ⇒ 等得越久（但**同时风险越低**
//!   ⇒ 这正是 L4「可塑性」的取舍：时间成本与风险成本是一对，必须一起看）；
//! * 回退**不重跑影子**、报告也更精简 ⇒ `rollback_ms` 恒小于 `switch_ms`
//!   ⇒ 本模型下「回退必须比切换更快」结构性成立；保留这道闸是为了将来把
//!   **实测耗时**换进来时**不必改口径**（闸的位置、判定方向都不变）。
//!
//! # 归档报告格式（JSON，**不覆盖旧报告**）
//!
//! 文件名 = `{kind}-{时间戳ms}-g{代次}-to-{目标算法}.json`
//! （`kind` ∈ `swap` / `rollback`）⇒ **带时间戳 + 代次 + 目标** ⇒ 同一次切换
//! 不会覆盖上一次报告，且**文件名按序排 ⇒ 天然时序**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、三步流程、`Swap cost` 度量、归档、测试齐备（**库级实现**，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0 调用点**
//!   （「影子双跑 ⇒ 裁决 ⇒ 切换」要在生产里同时跑两套检索 ⇒ 那是**下批**的接线活）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：尚未在真实卷子 + 真实检索器上跑过
//!   （公开基准数据未到位 ⇒ 只有单测里的假检索器跑过）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::shadow_run::{SwapDecision, SwapVerdict};

/// 影子轮数的**下限**（`shadow_runs >= 2`）。
///
/// # 为什么是 2 而不是 1
///
/// 一轮影子 ⇒ 只看到一个样本 ⇒ 那叫**运气**不叫**证据**（§6.4「比稳定分不比单次分」）。
pub const MIN_SHADOW_RUNS: usize = 2;

/// 影子样本题数的**下限**（`shadow_cases >= 10`）。
///
/// # 为什么是 10
///
/// 题太少 ⇒ 结论的波动比差异还大 ⇒ 换算法这件事本身的**风险**比收益大
/// （换错了要回退，回退还要时间 ⇒ 见 [`SwapCost`]）。
pub const MIN_SHADOW_CASES: usize = 10;

/// 归档报告的 **schema 标识**（版本化 ⇒ 将来改格式能认出来）。
pub const ARCHIVE_SCHEMA: &str = "apeireth.swap_archive.v1";

/// 每条影子样本的**等效等待毫秒**（成本模型单位）。
///
/// # 为什么是 1
///
/// 这是**模型单位**不是实测值 ⇒ 取 1 是为了让 `switch_ms` 的量级直接等于
/// 「证据条数 + 落盘开销」⇒ 读报告的人一眼能看出「这次切换等了多久的证据」。
const SHADOW_EVIDENCE_MS_PER_CASE: u64 = 1;

/// 切换落盘的**等效毫秒**（写指针 + 写**完整**报告：含影子证据与裁决）。
const SWITCH_WRITE_MS: u64 = 5;

/// 回退落盘的**等效毫秒**（写指针 + 写**精简**报告：回退不重跑影子 ⇒ 没有证据要写）。
const ROLLBACK_WRITE_MS: u64 = 1;

/// **算法指针**（当前生产用哪个算法）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgoPointer {
    /// 当前算法 id。
    pub current: String,
    /// 上一次算法 id（**回退目标** ⇒ `None` = 从没换过）。
    pub previous: Option<String>,
    /// 切换时刻（毫秒）。
    pub switched_at_ms: i64,
    /// 切换序号（第几次换 ⇒ 审计用）。
    pub generation: u64,
}

/// **切换计划**（裁决通过后才生成）。
#[derive(Debug, Clone, PartialEq)]
pub struct SwapPlan {
    /// 从谁换到谁。
    pub from: String,
    /// 到谁。
    pub to: String,
    /// 支撑这次切换的裁决（**必须原样带进归档** ⇒ 事后可查「凭什么换的」）。
    pub verdict: SwapVerdict,
    /// 影子轮数（跑了几轮才下决心）。
    pub shadow_runs: usize,
    /// 影子样本题数。
    pub shadow_cases: usize,
}

/// **`Swap cost`**（L4 指标）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwapCost {
    /// 切换耗时（毫秒）。
    pub switch_ms: u64,
    /// **回退耗时**（毫秒）⇒ 必须 **< switch_ms**（回退必须更快）。
    pub rollback_ms: u64,
    /// 影子轮数（**样本充分性**的度量）。
    pub shadow_runs: usize,
    /// 影子样本题数。
    pub shadow_cases: usize,
}

impl SwapCost {
    /// 是否**允许**这次切换（照设计稿 §6.4 + L4）。
    ///
    /// # 口径
    ///
    /// * 裁决必须是 `Approved` / `ApprovedWithGain`（**Rejected ⇒ 不许换**）；
    /// * **回退必须比切换快**（`rollback_ms < switch_ms`）⇒ 否则「可回退」是假的；
    /// * **样本充分**：`shadow_runs >= 2`（一轮 ⇒ 只是运气）且
    ///   `shadow_cases >= 10`（题太少 ⇒ 结论没意义）。
    ///
    /// # 闸的顺序（为什么是这个顺序）
    ///
    /// 裁决 → 回退速度 → 样本充分：
    /// * **先裁决**：裁决是「证据结论」，结论不成立时其它指标没资格说话；
    /// * **再回退速度**：「可回退」是**零风险上线**的前提 ⇒ 前提不成立 ⇒ 后面再准也不换；
    /// * **最后样本充分**：样本不够是**证据不足**（比结论更靠后的问题）。
    pub fn allows_swap(&self, decision: SwapDecision) -> bool {
        // --- 闸 1：裁决 ---
        if !matches!(
            decision,
            SwapDecision::Approved | SwapDecision::ApprovedWithGain
        ) {
            return false;
        }
        // --- 闸 2：回退必须比切换快（否则「可回退」是假的） ---
        if self.rollback_ms >= self.switch_ms {
            return false;
        }
        // --- 闸 3：样本充分 ---
        if self.shadow_runs < MIN_SHADOW_RUNS || self.shadow_cases < MIN_SHADOW_CASES {
            return false;
        }
        true
    }
}

/// 一次**完整切换**的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct SwapOutcome {
    /// 切换后的指针。
    pub pointer: AlgoPointer,
    /// 度量出来的 `Swap cost`。
    pub cost: SwapCost,
    /// 归档报告的**落盘路径**（**没落盘 ⇒ 空串** ⇒ 见 [`execute_swap`] 的「什么都不做」口径）。
    pub archive_path: String,
    /// **有没有真切换**（裁决不通过 ⇒ `false`，**且指针不变**）。
    pub switched: bool,
    /// 人话说明。
    pub detail: String,
}

/// 执行一次**切换**（裁决不通过 ⇒ **什么都不做**，如实返回 `switched = false`）。
///
/// # 口径
///
/// * `now_ms` **注入** ⇒ 确定性（**不许**在函数里读系统时钟）；
/// * 成功后：`pointer.current = plan.to`、`pointer.previous = plan.from`（⇒ **可回退**）、
///   `generation += 1`；
/// * **归档**：把 `plan` + `cost` + 裁决理由写成 **JSON** 落盘（文件名带时间戳与序号
///   ⇒ **不覆盖旧报告**）；
/// * ⚠️ **回退**：另给 [`rollback`]（**必须**比切换快，否则 `allows_swap` 就不通过）。
///
/// # 「什么都不做」的**确切含义**
///
/// 裁决闸 / 回退速度闸 / 样本充分闸**任一不过** ⇒：
///
/// * **指针逐字不变**（`current` / `previous` / `generation` / `switched_at_ms` 全不变）；
/// * **不落盘**（`archive_path` = 空串 ⇒ 目录里不会多出任何文件）；
/// * 返回 `switched = false` + `detail` 里写清**是哪道闸**拦下的 + 裁决原话。
///
/// # 什么情况返回 `Err`（**调用方的错**，不是策略拒绝）
///
/// * 归档目录不存在 / 不是目录 ⇒ 不许静默跳过归档（评测报告必须留痕）；
/// * `plan.from` 与 `pointer.current` 不一致 ⇒ **计划过期**（影子跑的是另一个现役算法）
///   ⇒ 不许换（零风险的前提是「比的就是现在在用的那个」）；
/// * 落盘失败（IO 错误）⇒ **指针不动**（不许出现「切了但没留痕」）。
pub fn execute_swap(
    plan: &SwapPlan,
    pointer: &mut AlgoPointer,
    archive_dir: &Path,
    now_ms: i64,
) -> Result<SwapOutcome, String> {
    // --- 0. 归档目录必须先在（不许自动建 ⇒ 免得把「路径打错」变成静默成功） ---
    ensure_archive_dir(archive_dir)?;

    // --- 1. 计划必须与现在的指针对得上（计划过期 ⇒ 不许换） ---
    if plan.from != pointer.current {
        return Err(format!(
            "切换计划过期：计划写的是从「{}」换到「{}」，但当前现役算法是「{}」\
             ⇒ 不许换（零风险的前提是「比的就是现在在用的那个」）",
            plan.from, plan.to, pointer.current
        ));
    }

    // --- 2. 三道闸 ---
    let cost = plan_cost(plan);
    if let Some(detail) = gate_detail(&cost, plan.verdict.decision, &plan.verdict.reason) {
        return Ok(SwapOutcome {
            pointer: pointer.clone(),
            cost,
            archive_path: String::new(),
            switched: false,
            detail,
        });
    }

    // --- 3. 算新指针（先算后写 ⇒ 写失败时指针还没动） ---
    let next_pointer = AlgoPointer {
        current: plan.to.clone(),
        previous: Some(plan.from.clone()),
        switched_at_ms: now_ms,
        generation: pointer.generation.saturating_add(1),
    };
    let detail = format!(
        "已切换：{} → {}（第 {} 次切换，切换耗时 {}ms，回退耗时 {}ms ⇒ 回退更快 ⇒ 可回退）；\
         回退目标 = {}",
        plan.from, plan.to, next_pointer.generation, cost.switch_ms, cost.rollback_ms, plan.from
    );

    // --- 4. 归档（先落盘再改指针 ⇒ 落盘失败 ⇒ 指针不动） ---
    let file_name = archive_file_name("swap", now_ms, next_pointer.generation, &plan.to);
    let report = ArchiveReport {
        schema: ARCHIVE_SCHEMA,
        kind: "swap",
        archived_at_ms: now_ms,
        plan: ArchivePlan {
            from: plan.from.clone(),
            to: plan.to.clone(),
            shadow_runs: plan.shadow_runs,
            shadow_cases: plan.shadow_cases,
        },
        verdict: Some(ArchiveVerdict::from(&plan.verdict)),
        cost: ArchiveCost::from(&cost),
        pointer_before: ArchivePointer::from(&*pointer),
        pointer_after: ArchivePointer::from(&next_pointer),
        detail: detail.clone(),
    };
    let archive_path = write_archive(archive_dir, &file_name, &report)?;

    // --- 5. 真正切换 ---
    *pointer = next_pointer;

    Ok(SwapOutcome {
        pointer: pointer.clone(),
        cost,
        archive_path,
        switched: true,
        detail,
    })
}

/// **回退**（回到 `pointer.previous`）。
///
/// # 口径
///
/// * `previous` 为 `None` ⇒ `Err`（**没得回退** ⇒ 如实报错，不假装成功）；
/// * 回退**也归档**（一份 `kind = "rollback"` 的报告）⇒ **回退也要留痕**；
/// * 回退**不改变** `generation`（它不是又一次「新选择」⇒ 而是**撤销**）
///   ⇒ 但 `previous` 与 `current` **对调**（⇒ 再回退一次能回去 ⇒ 不丢历史）。
///
/// # 为什么回退不重跑影子
///
/// 回退是**撤销**不是**新选择** ⇒ 不需要新证据（证据只用来支持「换」）
/// ⇒ 回退**必须比切换快**（否则出事时最贵的一步是「撤回来」⇒ 零风险上线是假的）。
pub fn rollback(
    pointer: &mut AlgoPointer,
    archive_dir: &Path,
    now_ms: i64,
) -> Result<SwapOutcome, String> {
    // --- 0. 归档目录必须先在 ---
    ensure_archive_dir(archive_dir)?;

    // --- 1. 没得回退 ⇒ 如实报错（不假装成功） ---
    let Some(target) = pointer.previous.clone() else {
        return Err(format!(
            "没有可回退的目标：当前算法「{}」是初始算法（`previous` = None）⇒ 回退无路可走",
            pointer.current
        ));
    };

    // --- 2. 算新指针（对调 current/previous ⇒ 再回退一次能回去；generation 不变） ---
    let next_pointer = AlgoPointer {
        current: target.clone(),
        previous: Some(pointer.current.clone()),
        switched_at_ms: now_ms,
        generation: pointer.generation,
    };
    let cost = SwapCost {
        // 回退不重跑影子 ⇒ 影子证据维度记 0；`switch_ms` 记的是
        // 「此刻再换一次（不带新证据）的等效耗时」⇒ 报告里始终满足「回退更快」。
        switch_ms: switch_cost_ms(0, 0),
        rollback_ms: ROLLBACK_WRITE_MS,
        shadow_runs: 0,
        shadow_cases: 0,
    };
    let detail = format!(
        "已回退：{} → {}（撤销，generation 不变 = {}；回退耗时 {}ms < 切换耗时 {}ms ⇒ 可再回退）",
        pointer.current, target, pointer.generation, cost.rollback_ms, cost.switch_ms
    );

    // --- 3. 归档（回退也要留痕） ---
    let file_name = archive_file_name("rollback", now_ms, pointer.generation, &target);
    let report = ArchiveReport {
        schema: ARCHIVE_SCHEMA,
        kind: "rollback",
        archived_at_ms: now_ms,
        plan: ArchivePlan {
            from: pointer.current.clone(),
            to: target.clone(),
            shadow_runs: 0,
            shadow_cases: 0,
        },
        // 回退没有裁决（裁决是为「换」准备的）⇒ 显式记 null ⇒ 不假装有裁决。
        verdict: None,
        cost: ArchiveCost::from(&cost),
        pointer_before: ArchivePointer::from(&*pointer),
        pointer_after: ArchivePointer::from(&next_pointer),
        detail: detail.clone(),
    };
    let archive_path = write_archive(archive_dir, &file_name, &report)?;

    // --- 4. 真正回退 ---
    *pointer = next_pointer;

    Ok(SwapOutcome {
        pointer: pointer.clone(),
        cost,
        archive_path,
        switched: true,
        detail,
    })
}

/// 列出已归档的报告（**按文件名序** ⇒ 确定性）。
///
/// # 口径
///
/// * 只列**文件**（子目录不算）且只列 `.json`（归档目录专用于报告）；
/// * 返回**文件名**（不是全路径 ⇒ 调用方自己拼目录）；
/// * **按文件名序**（字节序 ⇒ 跨平台一致）⇒ 文件名带时间戳 ⇒ 排序即**时序**；
/// * 目录不存在 ⇒ `Err`（**不 panic**）。
pub fn list_archives(archive_dir: &Path) -> Result<Vec<String>, String> {
    ensure_archive_dir(archive_dir)?;
    let entries = fs::read_dir(archive_dir)
        .map_err(|e| format!("归档目录读取失败（{}）：{e}", archive_dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("归档目录项读取失败（{}）：{e}", archive_dir.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("归档目录项类型读取失败（{}）：{e}", entry.path().display()))?;
        if file_type.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                names.push(name);
            }
        }
    }
    names.sort();
    Ok(names)
}

// ============================================
// 内部工具
// ============================================

/// 三道闸的**人话说明**（`None` = 全过 ⇒ 允许切）。
///
/// # 为什么把「哪道闸拦下的」写进返回值
///
/// 不写 ⇒ 事后只看得到「没切」不知道**为什么**没切 ⇒ 下次还是犯同样的错。
fn gate_detail(cost: &SwapCost, decision: SwapDecision, reason: &str) -> Option<String> {
    if decision == SwapDecision::Rejected {
        return Some(format!(
            "未切换：裁决 = Rejected ⇒ 不许换；裁决原话：{reason}"
        ));
    }
    if cost.rollback_ms >= cost.switch_ms {
        return Some(format!(
            "未切换：回退耗时 {}ms 不快于切换耗时 {}ms ⇒「可回退」是假的 ⇒ 不许换",
            cost.rollback_ms, cost.switch_ms
        ));
    }
    if cost.shadow_runs < MIN_SHADOW_RUNS {
        return Some(format!(
            "未切换：影子只跑了 {} 轮（< {}）⇒ 样本不足（一轮只是运气）⇒ 不许换",
            cost.shadow_runs, MIN_SHADOW_RUNS
        ));
    }
    if cost.shadow_cases < MIN_SHADOW_CASES {
        return Some(format!(
            "未切换：影子样本只有 {} 题（< {}）⇒ 结论没意义 ⇒ 不许换",
            cost.shadow_cases, MIN_SHADOW_CASES
        ));
    }
    None
}

/// 切换成本（时间维度）= 等影子证据的时间 + 切换落盘的时间。
fn switch_cost_ms(shadow_runs: usize, shadow_cases: usize) -> u64 {
    let evidence = u64::try_from(shadow_runs)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::try_from(shadow_cases).unwrap_or(u64::MAX))
        .saturating_mul(SHADOW_EVIDENCE_MS_PER_CASE);
    evidence.saturating_add(SWITCH_WRITE_MS)
}

/// 计划 ⇒ `Swap cost`（样本维度直接来自计划 ⇒ 归档里能对上「凭什么说样本够」）。
fn plan_cost(plan: &SwapPlan) -> SwapCost {
    SwapCost {
        switch_ms: switch_cost_ms(plan.shadow_runs, plan.shadow_cases),
        rollback_ms: ROLLBACK_WRITE_MS,
        shadow_runs: plan.shadow_runs,
        shadow_cases: plan.shadow_cases,
    }
}

/// 归档文件名（**带时间戳与代次** ⇒ 不覆盖旧报告）。
fn archive_file_name(kind: &str, now_ms: i64, generation: u64, target: &str) -> String {
    format!(
        "{kind}-{now_ms}-g{generation}-to-{}.json",
        sanitize_id(target)
    )
}

/// 算法 id ⇒ 文件名安全片段（**不许**路径穿越 ⇒ 不许出现 `/` `..` 之类）。
fn sanitize_id(id: &str) -> String {
    let mut out = String::new();
    for ch in id.chars().take(64) {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out == "." || out == ".." {
        out.push('_');
    }
    out
}

/// 归档目录必须**已经存在**（不许自动建 ⇒ 免得把「路径打错」变成静默成功）。
fn ensure_archive_dir(archive_dir: &Path) -> Result<(), String> {
    if !archive_dir.is_dir() {
        return Err(format!(
            "归档目录不存在或不是目录（{}）⇒ 报错（评测报告必须留痕 ⇒ 不许静默跳过归档）",
            archive_dir.display()
        ));
    }
    Ok(())
}

/// 落盘一份归档报告（**pretty JSON + 末尾换行** ⇒ 人能读、工具能 diff）。
fn write_archive(
    archive_dir: &Path,
    file_name: &str,
    report: &ArchiveReport,
) -> Result<String, String> {
    let path = archive_dir.join(file_name);
    let mut body = serde_json::to_string_pretty(report)
        .map_err(|e| format!("归档报告序列化失败（{}）：{e}", path.display()))?;
    body.push('\n');
    fs::write(&path, body.as_bytes())
        .map_err(|e| format!("归档报告落盘失败（{}）：{e}", path.display()))?;
    Ok(path.display().to_string())
}

// ============================================
// 归档报告的**可序列化镜像**
//
// 为什么不用 `SwapVerdict` / `SwapDecision` 直接序列化：
// `shadow_run.rs` 已定稿（不许改）⇒ 那边的类型**没有** `Serialize` 派生
// ⇒ 在这里做一份**字段一一对应**的镜像（口径只有一处：镜像字段名与原字段同名）。
// ============================================

/// 归档报告本体。
#[derive(Debug, Serialize)]
struct ArchiveReport {
    /// schema 版本（格式变了能认出来）。
    schema: &'static str,
    /// `swap`（切换）/ `rollback`（回退）。
    kind: &'static str,
    /// 归档时刻（毫秒，注入值）。
    archived_at_ms: i64,
    /// 这次切换的计划。
    plan: ArchivePlan,
    /// 裁决（回退没有裁决 ⇒ `null`）。
    verdict: Option<ArchiveVerdict>,
    /// 度量出来的 `Swap cost`。
    cost: ArchiveCost,
    /// 切换前的指针。
    pointer_before: ArchivePointer,
    /// 切换后的指针。
    pointer_after: ArchivePointer,
    /// 人话说明。
    detail: String,
}

/// 归档里的计划（与 [`SwapPlan`] 字段同名）。
#[derive(Debug, Serialize)]
struct ArchivePlan {
    from: String,
    to: String,
    shadow_runs: usize,
    shadow_cases: usize,
}

/// 归档里的裁决（与 [`SwapVerdict`] 字段同名）。
#[derive(Debug, Serialize)]
struct ArchiveVerdict {
    decision: &'static str,
    champion_score: Option<f64>,
    challenger_score: Option<f64>,
    champion_tokens: Option<usize>,
    challenger_tokens: Option<usize>,
    reason: String,
}

impl From<&SwapVerdict> for ArchiveVerdict {
    fn from(verdict: &SwapVerdict) -> Self {
        ArchiveVerdict {
            decision: decision_name(verdict.decision),
            champion_score: verdict.champion_score,
            challenger_score: verdict.challenger_score,
            champion_tokens: verdict.champion_tokens,
            challenger_tokens: verdict.challenger_tokens,
            reason: verdict.reason.clone(),
        }
    }
}

/// 裁决枚举 ⇒ 归档里的**稳定字符串**（不依赖 `Debug` 格式 ⇒ 格式变了报告还能读）。
fn decision_name(decision: SwapDecision) -> &'static str {
    match decision {
        SwapDecision::Approved => "Approved",
        SwapDecision::ApprovedWithGain => "ApprovedWithGain",
        SwapDecision::Rejected => "Rejected",
    }
}

/// 归档里的 `Swap cost`（与 [`SwapCost`] 字段同名）。
#[derive(Debug, Serialize)]
struct ArchiveCost {
    switch_ms: u64,
    rollback_ms: u64,
    shadow_runs: usize,
    shadow_cases: usize,
}

impl From<&SwapCost> for ArchiveCost {
    fn from(cost: &SwapCost) -> Self {
        ArchiveCost {
            switch_ms: cost.switch_ms,
            rollback_ms: cost.rollback_ms,
            shadow_runs: cost.shadow_runs,
            shadow_cases: cost.shadow_cases,
        }
    }
}

/// 归档里的算法指针（与 [`AlgoPointer`] 字段同名）。
#[derive(Debug, Serialize)]
struct ArchivePointer {
    current: String,
    previous: Option<String>,
    switched_at_ms: i64,
    generation: u64,
}

impl From<&AlgoPointer> for ArchivePointer {
    fn from(pointer: &AlgoPointer) -> Self {
        ArchivePointer {
            current: pointer.current.clone(),
            previous: pointer.previous.clone(),
            switched_at_ms: pointer.switched_at_ms,
            generation: pointer.generation,
        }
    }
}
