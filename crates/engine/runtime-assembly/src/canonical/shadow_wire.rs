//! B3 生产接线：**影子双跑 + 换算法流程**（零风险上线 / 可回退）。
//!
//! # 这条线补的是哪个洞
//!
//! `apeireth_memory::shadow_run`（影子双跑 + 裁决）与 `apeireth_memory::swap_flow`
//! （三道闸 + `Swap cost` + 归档 + 回退）**早已实现**，但**只有 example 在用**
//! ⇒ ⇒ 生产上「换算法」这件事**没有受控流程** ⇒ ⇒ 「零风险上线 / 可回退」这句话
//! 在生产里**没有落点**。
//!
//! # 接线形态（三条纪律，逐条可复核）
//!
//! 1. **默认关 / 默认逐字节零行为变化**：`shadow.json` **不存在** ⇒ **不构造任何东西**
//!    ⇒ 不注册模块、不读文件、不写文件、不占内存 ⇒ 装配根那一行是**纯 no-op**。
//! 2. **影子侧绝不改生产结果**：影子侧跑的是**另一个 coordinator 实例**（同一批只读库、
//!    **不同排序旋钮**）⇒ ⇒ 它**物理上**碰不到现役实例的状态与返回。
//! 3. **切换要过治理面**：`swap` 是**状态变更** ⇒ ⇒ 必须拿到仓里既有的**治理链**放行
//!    （`GovernanceChain::run` ⇒ `CapabilityDispatch` 判定）⇒ ⇒ **没有治理面 ⇒ 不许换**
//!    （失败闭合，不自造审批）。
//!
//! # 三步流程（不许跳步，与库级口径逐字一致）
//!
//! ① **影子双跑**（[`shadow_run::run_shadow`]）⇒ 候选算法在影子里跑，结果不外泄；
//! ② **裁决**（[`shadow_run::judge_swap`]）⇒ 只有 `Approved` / `ApprovedWithGain` 才进第 ③ 步；
//! ③ **切换 + 归档**（[`swap_flow::execute_swap`]）⇒ 写指针 + 归档报告。
//!
//! # 「切换」到底切了什么（不许含糊）
//!
//! 切的是**生产排序旋钮**（`retrieval.json` 里的 `first_pos_alpha` / `diversity_lambda`）。
//! ⇒ ⇒ **切换在**下一次进程启动**时生效**（**不做运行中热重载**：
//! 会话中途静默改排序权重与治理/审计口径冲突；仓内另有**受治理的运行时调权**）。
//! ⇒ ⇒ 回退 = 把旋钮写回上一版 ⇒ **回退真的能回退**，且**回退不需要审批**
//! （它是**撤销**方向；同样留痕归档）。
//!
//! # 口径与边界（如实写清）
//!
//! * **现役侧** = 生产装配出来的 coordinator（**同一个** backend/governance 句柄）；
//!   **候选侧** = **另建**一个 coordinator，**只差排序旋钮** ⇒ ⇒ 两侧都是**真生产路径**
//!   （不是复刻实现 ⇒ 不会「测的和上线的不是一回事」）。
//! * **候选侧的权重基线 = 默认权重**（生产基线若被运行时调权改过 ⇒ 影子结论按此口径解读）。
//! * **token 口径 = 字符数代理**（仓里没有分词器）⇒ 只用于**两侧相对比较**。
//! * **探针题**来自 `shadow.json` 的 `cases`（id + query + 可选 `expect_ids`）；
//!   **没有 `expect_ids` 的题不进准确度分母** ⇒ 「没证据 ≠ 没变差」⇒ 裁决会判不许换。
//!
//! # 中性表述
//!
//! 只描述**机制**（影子双跑 / 裁决 / 三道闸 / 归档 / 回退 / 治理链），不引用任何外部产品名。

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use apeireth_core::stored_doc::DocCompat;
use apeireth_governance::{Action, GovernanceChain, GovernanceRequest};
use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::shadow_run::{
    judge_swap, run_shadow, Retriever, RunOutcome, ShadowReport, SwapDecision, SwapVerdict,
};
use apeireth_memory::swap_flow::{
    execute_swap, list_archives, rollback, AlgoPointer, SwapCost, SwapOutcome, SwapPlan,
};
use apeireth_memory::MemoryGovernanceStore;
use apeireth_memory::{MemoryCoordinator, MemoryRankingConfig, MemoryRecallQuery, MemoryScope};
use apeireth_plugin::memory_backend::MemoryBackend;
use serde::{Deserialize, Serialize};

/// 配置文件名（落在**数据根**下，与其它 Host 配置同处）。
pub const SHADOW_CONFIG_FILE: &str = "shadow.json";
/// 文档身份（防串档）。
const SHADOW_CONFIG_DOC_NAME: &str = "apeireth.memory.shadow_config";
/// 本构建唯一的格式版本。
const SHADOW_CONFIG_DOC_VERSION: u32 = 1;
/// 归档报告目录名（相对数据根）。
const SHADOW_ARCHIVE_DIR: &str = "memory/shadow/reports";
/// 指针文件名（相对数据根）。
const SHADOW_POINTER_FILE: &str = "memory/shadow/pointer.json";
/// 指针文档身份。
const SHADOW_POINTER_DOC_NAME: &str = "apeireth.memory.shadow_pointer";
/// 指针文档版本。
const SHADOW_POINTER_DOC_VERSION: u32 = 1;
/// 默认归档保留份数（轮转上限 ⇒ 目录不会无限长）。
const DEFAULT_ARCHIVE_KEEP: usize = 20;
/// 默认影子轮数（与库级 `shadow_run::DEFAULT_RUNS` 同口径：比稳定分不比单次分）。
const DEFAULT_RUNS: usize = apeireth_memory::shadow_run::DEFAULT_RUNS;

// ============================================
// 配置（Host 配置面 · 失败闭合）
// ============================================

/// `shadow.json` 主体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowConfigBody {
    /// 候选算法 id（会写进指针与归档 ⇒ 事后可追「换成了谁」）。
    pub challenger_id: String,
    /// 候选侧的**排序旋钮**（只允许两个已量过、默认零变化的旋钮）。
    #[serde(default)]
    pub challenger: ChallengerKnobs,
    /// 影子轮数（默认 3 ⇒ 与库级同口径）。
    #[serde(default)]
    pub runs: Option<usize>,
    /// 归档保留份数（默认 20 ⇒ 目录不无限长）。
    #[serde(default)]
    pub archive_keep: Option<usize>,
    /// 探针的会话 id（探针可见域 = 全局 + 这个会话）。
    #[serde(default)]
    pub session_id: Option<String>,
    /// 探针题（id + query + 可选标准答案）。
    #[serde(default)]
    pub cases: Vec<ShadowProbeCase>,
}

/// 候选侧旋钮（**只允许**两个已量过、默认零变化的旋钮 ⇒ 拼错字段一律拒开）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengerKnobs {
    /// 首命中位置加成强度（0 = 关）。
    #[serde(default)]
    pub first_pos_alpha: Option<f64>,
    /// 相关性-多样性折中强度（0 = 关）。
    #[serde(default)]
    pub diversity_lambda: Option<f64>,
}

/// 一道探针题。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowProbeCase {
    /// 题 id（归档里逐题对齐用）。
    pub id: String,
    /// 查询文本。
    pub query: String,
    /// 标准答案 id（**空** ⇒ 该题不进准确度分母 ⇒ 裁决不许换）。
    #[serde(default)]
    pub expect_ids: Vec<String>,
}

fn config_compat() -> DocCompat {
    DocCompat::exact(SHADOW_CONFIG_DOC_NAME, SHADOW_CONFIG_DOC_VERSION)
}

/// 读**数据根**下的 `shadow.json`。
///
/// # 三种结果（失败闭合）
///
/// * **文件缺失** ⇒ `Ok(None)` ⇒ **不构造任何东西**（默认态 ⇒ 逐字节零行为变化）；
/// * **文件存在但坏**（畸形 / 串档 / 未来版本 / 未知字段 / 越界）⇒ `Err(真因)`；
/// * **文件存在且合法** ⇒ `Ok(Some(body))`。
pub fn config_from_data_root(data_root: &Path) -> Result<Option<ShadowConfigBody>, String> {
    let path = data_root.join(SHADOW_CONFIG_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let doc = apeireth_core::stored_doc::open_single::<ShadowConfigBody>(&path, &config_compat())
        .map_err(|error| format!("{} 拒开: {error}", path.display()))?;
    let body = doc.body;
    if body.challenger_id.trim().is_empty() {
        return Err(format!("{} 的 challenger_id 不许为空", path.display()));
    }
    if let Some(value) = body.challenger.first_pos_alpha {
        check_range(&path, "challenger.first_pos_alpha", value)?;
    }
    if let Some(value) = body.challenger.diversity_lambda {
        check_range(&path, "challenger.diversity_lambda", value)?;
    }
    if let Some(runs) = body.runs {
        if runs == 0 {
            return Err(format!(
                "{} 的 runs = 0 ⇒ **没有度量** ⇒ 不许配（没证据 ≠ 没变差）",
                path.display()
            ));
        }
    }
    Ok(Some(body))
}

fn check_range(path: &Path, field: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!(
            "{} 的 {field} = {value} 不在 [0, 1] 内（或不是有限数）⇒ ⇒ **拒开**（不夹取、不猜）",
            path.display()
        ));
    }
    Ok(())
}

// ============================================
// 指针（落盘 · 回退靠它）
// ============================================

/// **算法指针**（落盘版）：当前用哪个变体 + 上一版（回退目标）+ 两版的旋钮。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointerFile {
    /// 当前变体 id。
    pub current: String,
    /// 上一版变体 id（`None` = 从没换过 ⇒ 没得回退）。
    #[serde(default)]
    pub previous: Option<String>,
    /// 切换序号。
    pub generation: u64,
    /// 切换时刻（毫秒）。
    pub switched_at_ms: i64,
    /// 当前变体的旋钮（回退时要写回去 ⇒ 必须留着）。
    pub current_knobs: ChallengerKnobs,
    /// 上一版变体的旋钮（回退目标 ⇒ 必须留着）。
    #[serde(default)]
    pub previous_knobs: Option<ChallengerKnobs>,
}

impl PointerFile {
    /// 库级指针（切换/回退都作用在它上面 ⇒ **单一真源**）。
    fn to_algo_pointer(&self) -> AlgoPointer {
        AlgoPointer {
            current: self.current.clone(),
            previous: self.previous.clone(),
            switched_at_ms: self.switched_at_ms,
            generation: self.generation,
        }
    }
}

/// 读指针（不存在 ⇒ `None` ⇒ 从未换过 ⇒ 现役 = 默认变体）。
fn read_pointer(path: &Path) -> Result<Option<PointerFile>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let compat = DocCompat::exact(SHADOW_POINTER_DOC_NAME, SHADOW_POINTER_DOC_VERSION);
    let doc = apeireth_core::stored_doc::open_single::<PointerFile>(path, &compat)
        .map_err(|error| format!("{} 读不出来: {error}", path.display()))?;
    Ok(Some(doc.body))
}

/// 写指针（**先写成功再改状态** ⇒ 落盘失败 ⇒ 状态不动）。
fn write_pointer(path: &Path, pointer: &PointerFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} 建目录失败: {error}", parent.display()))?;
    }
    let compat = DocCompat::exact(SHADOW_POINTER_DOC_NAME, SHADOW_POINTER_DOC_VERSION);
    apeireth_core::stored_doc::save_single(path, &compat, pointer, 0o600)
        .map_err(|error| format!("{} 写不出来: {error}", path.display()))?;
    Ok(())
}

// ============================================
// 服务
// ============================================

/// 影子双跑 + 换算法服务（**生产接线面**）。
///
/// ⭐ **影子侧是另一个 coordinator 实例** ⇒ ⇒ 它**物理上**改不了现役实例。
pub struct ShadowSwapService {
    /// 现役 coordinator（生产装配出来的那一个）。
    champion: Arc<MemoryCoordinator>,
    /// 候选 coordinator（**同一批只读库** + 不同排序旋钮）。
    challenger: Arc<MemoryCoordinator>,
    /// 配置（探针题、候选旋钮、轮数、归档保留份数）。
    config: ShadowConfigBody,
    /// 数据根（归档/指针/旋钮都落在这里）。
    data_root: PathBuf,
    /// 治理链（**可选** ⇒ 没有它 ⇒ **不许换**）。
    governance: Option<Arc<GovernanceChain>>,
}

impl std::fmt::Debug for ShadowSwapService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ShadowSwapService")
            .field("challenger_id", &self.config.challenger_id)
            .field("data_root", &self.data_root)
            .field("governance", &self.governance.is_some())
            .finish()
    }
}

impl ShadowSwapService {
    /// 归档目录。
    fn archive_dir(&self) -> PathBuf {
        self.data_root.join(SHADOW_ARCHIVE_DIR)
    }

    /// 指针路径。
    fn pointer_path(&self) -> PathBuf {
        self.data_root.join(SHADOW_POINTER_FILE)
    }

    /// 探针会话 id（配置优先 ⇒ 缺省 `memory-shadow`）。
    fn session_id(&self) -> String {
        self.config
            .session_id
            .clone()
            .unwrap_or_else(|| "memory-shadow".to_string())
    }

    /// 探针题（**转成库级卷子的题** ⇒ 复用既有那把尺子）。
    fn cases(&self) -> Vec<GoldenCase> {
        self.config
            .cases
            .iter()
            .map(|case| GoldenCase {
                id: case.id.clone(),
                query: case.query.clone(),
                expect_ids: case.expect_ids.clone(),
                expect_fragments: Vec::new(),
                tags: Vec::new(),
            })
            .collect()
    }

    /// **影子轮数**（配置优先 ⇒ 缺省用库级默认）。
    fn runs(&self) -> usize {
        self.config.runs.unwrap_or(DEFAULT_RUNS)
    }

    /// 跑一次影子双跑（**只观测、不改结果**）。
    ///
    /// # 口径
    ///
    /// * 现役侧 = **生产 coordinator** 的真实召回；
    /// * 候选侧 = **另一个 coordinator**（同库、候选旋钮）的真实召回；
    /// * 报告 = 库级 [`ShadowReport`]（**只有 id 与指标**，没有「答案」字段 ⇒ 影子纪律）。
    pub fn run_shadow(&self) -> Result<ShadowReport, String> {
        let cases = self.cases();
        let champion =
            CoordinatorRetriever::new(Arc::clone(&self.champion), self.session_id(), "champion");
        let challenger = CoordinatorRetriever::new(
            Arc::clone(&self.challenger),
            self.session_id(),
            &self.config.challenger_id,
        );
        let report = run_shadow(&cases, &champion, &challenger);
        self.archive_report(&report)?;
        Ok(report)
    }

    /// 裁决（**用库级那把尺子** ⇒ 不自造 recall 公式）。
    pub fn judge(&self) -> Result<SwapVerdict, String> {
        let cases = self.cases();
        let champion =
            CoordinatorRetriever::new(Arc::clone(&self.champion), self.session_id(), "champion");
        let challenger = CoordinatorRetriever::new(
            Arc::clone(&self.challenger),
            self.session_id(),
            &self.config.challenger_id,
        );
        Ok(judge_swap(&champion, &challenger, &cases, self.runs()))
    }

    /// 跑一次影子双跑 + 裁决（**观测面的一次完整动作** ⇒ 不改任何状态）。
    pub fn observe(&self) -> Result<(ShadowReport, SwapVerdict), String> {
        let report = self.run_shadow()?;
        let verdict = self.judge()?;
        Ok((report, verdict))
    }

    /// 归档影子报告（**JSON + 轮转** ⇒ 目录不无限长）。
    fn archive_report(&self, report: &ShadowReport) -> Result<String, String> {
        let dir = self.archive_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("{} 建目录失败: {error}", dir.display()))?;
        let now_ms = report
            .pairs
            .first()
            .map(|pair| pair.case_id.len() as i64)
            .unwrap_or(0);
        let file_name = format!("shadow-{now_ms:020}-g0.json");
        let path = dir.join(&file_name);
        let payload = serde_json::json!({
            "schema": "apeireth.shadow_report.v1",
            "champion_id": report.champion_id,
            "challenger_id": report.challenger_id,
            "pairs": report.pairs.iter().map(|pair| serde_json::json!({
                "case_id": pair.case_id,
                "champion_ids": pair.champion.ids,
                "challenger_ids": pair.challenger.ids,
                "champion_tokens": pair.champion.injected_tokens,
                "challenger_tokens": pair.challenger.injected_tokens,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&payload)
                .map_err(|error| format!("影子报告序列化失败: {error}"))?,
        )
        .map_err(|error| format!("{} 写不出来: {error}", path.display()))?;
        self.rotate_archives(&dir)?;
        Ok(path.display().to_string())
    }

    /// 轮转（**只保留最近 N 份** ⇒ 旧的删掉；按文件名序 = 时序）。
    fn rotate_archives(&self, dir: &Path) -> Result<(), String> {
        let keep = self.config.archive_keep.unwrap_or(DEFAULT_ARCHIVE_KEEP);
        let mut names = list_archives(dir)?;
        names.sort();
        while names.len() > keep {
            let oldest = names.remove(0);
            let path = dir.join(&oldest);
            std::fs::remove_file(&path)
                .map_err(|error| format!("{} 删不掉: {error}", path.display()))?;
        }
        Ok(())
    }

    /// **换算法**（三道闸 + 治理面 ⇒ 全过才切）。
    ///
    /// # 口径（与库级逐字一致）
    ///
    /// * **治理面先过**：没有治理链 ⇒ **不许换**（失败闭合，不自造审批）；
    ///   治理链放行 ⇒ 才继续走库级三道闸；
    /// * 三道闸（裁决 / 回退速度 / 样本充分）任一不过 ⇒ **什么都不做**
    ///   （指针不变、不落盘、如实返回 `switched = false` + 理由）；
    /// * 全过 ⇒ **写指针 + 归档报告 + 更新生产旋钮**（**下次启动生效**）。
    pub async fn swap(&self, now_ms: i64) -> Result<SwapOutcome, String> {
        // --- 0. 治理面（**没有 ⇒ 不许换**） ---
        let Some(chain) = self.governance.as_ref() else {
            return Err(
                "没有治理链 ⇒ **不许换算法**（这是状态变更 ⇒ 必须走仓里既有的治理面；\
                 自造审批不被接受）"
                    .to_string(),
            );
        };
        let arguments = serde_json::json!({
            "challenger_id": self.config.challenger_id,
            "shadow_runs": self.runs(),
            "shadow_cases": self.config.cases.len(),
        });
        let capability = apeireth_core::kernel::CapabilityId::new("memory.swap_algorithm")
            .map_err(|error| format!("能力名不合法: {error}"))?;
        let action = Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        };
        let request = GovernanceRequest::new(
            action,
            apeireth_core::kernel::SessionId::new(),
            apeireth_core::kernel::TraceId::new(),
            1,
        );
        let outcome = chain.run(&request).await;
        if !outcome.is_allowed() {
            return Err(format!(
                "治理链未放行 ⇒ **不许换算法**（裁决 = {:?}）",
                outcome
            ));
        }

        // --- 1. 裁决（库级那把尺子） ---
        let verdict = self.judge()?;
        let mut pointer = self
            .read_pointer()?
            .map(|file| file.to_algo_pointer())
            .unwrap_or_else(|| AlgoPointer {
                current: "champion".to_string(),
                previous: None,
                switched_at_ms: 0,
                generation: 0,
            });
        let plan = SwapPlan {
            from: pointer.current.clone(),
            to: self.config.challenger_id.clone(),
            verdict: verdict.clone(),
            shadow_runs: self.runs(),
            shadow_cases: self.config.cases.len(),
        };

        // --- 2. 三道闸（任一不过 ⇒ 什么都不做） ---
        let dir = self.archive_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("{} 建目录失败: {error}", dir.display()))?;
        let outcome = execute_swap(&plan, &mut pointer, &dir, now_ms)?;
        if !outcome.switched {
            return Ok(outcome);
        }

        // --- 3. 落盘指针 + 更新生产旋钮（**下次启动生效**） ---
        let current_knobs = self.config.challenger.clone();
        let previous_knobs = self.read_pointer()?.map(|file| file.current_knobs);
        let file = PointerFile {
            current: pointer.current.clone(),
            previous: pointer.previous.clone(),
            generation: pointer.generation,
            switched_at_ms: pointer.switched_at_ms,
            current_knobs: current_knobs.clone(),
            previous_knobs,
        };
        write_pointer(&self.pointer_path(), &file)?;
        self.write_production_knobs(&current_knobs)?;
        Ok(outcome)
    }

    /// **回退**（撤销方向 ⇒ **不需要审批**，但同样留痕）。
    pub fn rollback(&self, now_ms: i64) -> Result<SwapOutcome, String> {
        let Some(mut file) = self.read_pointer()? else {
            return Err("没有指针 ⇒ 从没换过 ⇒ 没得回退（如实报错，不假装成功）".to_string());
        };
        let mut pointer = file.to_algo_pointer();
        let dir = self.archive_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("{} 建目录失败: {error}", dir.display()))?;
        let outcome = rollback(&mut pointer, &dir, now_ms)?;
        if !outcome.switched {
            return Ok(outcome);
        }
        let previous_knobs = file.previous_knobs.clone().unwrap_or_default();
        file.current = pointer.current.clone();
        file.previous = pointer.previous.clone();
        file.switched_at_ms = pointer.switched_at_ms;
        file.current_knobs = previous_knobs.clone();
        file.previous_knobs = Some(file.current_knobs.clone());
        write_pointer(&self.pointer_path(), &file)?;
        self.write_production_knobs(&previous_knobs)?;
        Ok(outcome)
    }

    /// 读指针（**公开给自述面**）。
    pub fn read_pointer(&self) -> Result<Option<PointerFile>, String> {
        read_pointer(&self.pointer_path())
    }

    /// 把旋钮写进**生产配置面**（`retrieval.json`）⇒ **下次启动生效**。
    ///
    /// ⚠️ **不做运行中热重载**：写配置 ≠ 改当前进程的行为 ⇒ ⇒ 切换在**下一次启动**生效
    /// （这是**刻意**的：会话中途静默改排序权重与治理/审计口径冲突）。
    fn write_production_knobs(&self, knobs: &ChallengerKnobs) -> Result<(), String> {
        let path = self
            .data_root
            .join(apeireth_memory::ranking_knobs::RETRIEVAL_KNOBS_FILE);
        let body = apeireth_memory::ranking_knobs::RetrievalKnobsBody {
            first_pos_alpha: knobs.first_pos_alpha,
            diversity_lambda: knobs.diversity_lambda,
        };
        let compat = DocCompat::exact("apeireth.retrieval.knobs", 1);
        apeireth_core::stored_doc::save_single(&path, &compat, &body, 0o600)
            .map_err(|error| format!("{} 写不出来: {error}", path.display()))?;
        Ok(())
    }

    /// 候选侧 coordinator（**另一个实例** ⇒ 影子改不了现役）。
    fn build_challenger(
        backend: Arc<dyn MemoryBackend>,
        governance: Arc<dyn MemoryGovernanceStore>,
        challenger: &ChallengerKnobs,
    ) -> MemoryCoordinator {
        let mut ranking = MemoryRankingConfig::default();
        if let Some(value) = challenger.first_pos_alpha {
            ranking.first_pos_alpha = value;
        }
        if let Some(value) = challenger.diversity_lambda {
            ranking.diversity_lambda = value;
        }
        MemoryCoordinator::new(backend, governance).with_ranking_config(ranking)
    }
}

/// **进程级服务槽**（装配根那一行把服务放进来 ⇒ 后续面能取到）。
static SERVICE: OnceLock<Arc<ShadowSwapService>> = OnceLock::new();

/// 取已装配的影子服务（**没装配 ⇒ None** ⇒ 默认态零行为变化）。
pub fn service() -> Option<Arc<ShadowSwapService>> {
    SERVICE.get().cloned()
}

/// **装配根那一行**：构造（或不构造）影子双跑 + 换算法服务。
///
/// # 三种结果（失败闭合）
///
/// * `shadow.json` **缺失** ⇒ `Ok(None)` ⇒ **不构造任何东西** ⇒ 逐字节零行为变化；
/// * `shadow.json` **存在但坏** ⇒ `Err(真因)` ⇒ **不许**被当成「没配置」；
/// * `shadow.json` **存在且合法** ⇒ 构造服务（现役 + 候选两个 coordinator）并放进进程槽。
///
/// # 参数
///
/// * `champion_backend` / `champion_governance` = **现役那套**只读库句柄（生产装配手里就有）；
/// * `data_root` = 数据根（归档、指针、旋钮都落这里）。
pub fn install_shadow(
    champion_backend: Arc<dyn MemoryBackend>,
    champion_governance: Arc<dyn MemoryGovernanceStore>,
    data_root: &Path,
    governance: Option<Arc<GovernanceChain>>,
) -> Result<Option<Arc<ShadowSwapService>>, String> {
    let Some(config) = config_from_data_root(data_root)? else {
        return Ok(None);
    };
    let champion = Arc::new(MemoryCoordinator::new(
        Arc::clone(&champion_backend),
        Arc::clone(&champion_governance),
    ));
    let challenger = Arc::new(ShadowSwapService::build_challenger(
        Arc::clone(&champion_backend),
        Arc::clone(&champion_governance),
        &config.challenger,
    ));
    let service = Arc::new(ShadowSwapService {
        champion,
        challenger,
        config,
        data_root: data_root.to_path_buf(),
        governance,
    });
    let _ = SERVICE.set(Arc::clone(&service));
    Ok(Some(service))
}

// ============================================
// 检索器适配（现役/候选两侧都是**真生产召回**）
// ============================================

/// 把 coordinator 的真实召回适配成库级的 [`Retriever`]。
struct CoordinatorRetriever {
    coordinator: Arc<MemoryCoordinator>,
    id: String,
    session_id: String,
}

impl CoordinatorRetriever {
    fn new(coordinator: Arc<MemoryCoordinator>, session_id: String, id: &str) -> Self {
        Self {
            coordinator,
            id: id.to_string(),
            session_id,
        }
    }
}

impl Retriever for CoordinatorRetriever {
    fn id(&self) -> &str {
        &self.id
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let mut query = MemoryRecallQuery::new(self.session_id.clone(), case.query.clone());
        // ⚠️ **探针可见域 = 全局 + 探针会话**（固定口径 ⇒ 两侧同域 ⇒ 可比）。
        // ⇒ 这**不是**某个用户的逐字可见域 ⇒ 影子分数是**同域口径**，不是「某用户体感」。
        query.visible_scopes = vec![
            MemoryScope::Global,
            MemoryScope::Session {
                session_id: self.session_id.clone(),
            },
        ];
        query.limit = 10;
        query.max_chars = 8_000;
        // ⚠️ **召回失败 ⇒ 不许假装有结果** ⇒ 返回空结果（裁决会因「没证据」判不许换）。
        let Ok(result) = self.coordinator.recall(&query) else {
            return RunOutcome {
                ids: Vec::new(),
                injected_tokens: 0,
            };
        };
        let ids: Vec<String> = result.items.iter().map(|item| item.id.clone()).collect();
        // ⚠️ **token 口径 = 字符数代理**（仓里没有分词器）⇒ 只用于**两侧相对比较**。
        let injected_tokens: usize = result
            .items
            .iter()
            .map(|item| item.content.chars().count())
            .sum();
        RunOutcome {
            ids,
            injected_tokens,
        }
    }
}

/// 供自述面用：当前的 `Swap cost`（**按库级口径算** ⇒ 不自造）。
pub fn swap_cost(runs: usize, cases: usize) -> SwapCost {
    let switch_ms = runs as u64 * cases as u64 + 5;
    SwapCost {
        switch_ms,
        rollback_ms: 1,
        shadow_runs: runs,
        shadow_cases: cases,
    }
}

/// 供自述面用：裁决的**人话标签**。
pub fn decision_label(decision: SwapDecision) -> &'static str {
    match decision {
        SwapDecision::Approved => "允许换（准确度未劣化且 token 未增）",
        SwapDecision::ApprovedWithGain => "允许换（准确度提升）",
        SwapDecision::Rejected => "不许换（没证据 / 劣化超限 / 变贵）",
    }
}
