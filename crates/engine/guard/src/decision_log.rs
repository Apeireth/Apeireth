//! 守卫判定的 **append-only 落盘面**（2026-10-25 接电）。
//!
//! # 这个模块补的是什么缺口
//!
//! `apeireth nightwatch` 自己报过一条 advisory gap：
//!
//! > audit chain 未持久化供闲时读取 —— no-degrade 复盘段空转 (接线项)
//!
//! 深查后确认比字面更严重：守卫的判定（`decision` / `risk_score` / `reasons`）
//! 只存在 `BehaviorChainGuardHook::recent_events` 的
//! `Mutex<VecDeque<GuardEventDto>>` 里 —— **进程重启即散**。
//!
//! 也就是说：判定算得出来、面板看得到（当次进程内），但**决策没有落盘面**。
//! 在 ADR-0014「记录 ≠ 负责」的语境下这是真问题 —— 事后无法追责、
//! 无法复盘「风险是否静默降级」，因为记录本身不存在。
//!
//! # 设计边界（0 装）
//!
//! - **只记录、不决策**：本模块写盘失败**绝不影响**守卫结论
//!   （调用方吞掉错误并计数，判定照常返回）。审计面不该成为可用性风险。
//! - **append-only**：一行一条 `GuardEventDto`，永不改写历史。
//!   坏行**读作不存在**（与 `egress_ledger` / `governance-audit.jsonl` 同纪律），
//!   不因为一行损坏就丢掉整份历史。
//! - **不自动执行任何东西**：落盘只是让事件可被闲时读取与复盘。
//!
//! # 风险档位映射（本批**显式定义**，不是捡来的）
//!
//! 守夜人的 no-degrade 复盘按 `governance::risk` 的**闭合词表**比较风险档
//! （`info|low|medium|high|critical|nuclear`），而守卫只给 `risk_score: f64`。
//! 本仓此前**没有** score → 档的映射（`grep` 全仓为证），所以这里显式定义一份，
//! 并写进测试钉死：任何输出都必须能被 `governance::risk::risk_rank` 识别
//! （`rank >= 0`），否则 no-degrade 比较会静默失效。
//!
//! 这是**只用于 report-only 复盘的观测口径**，不回写任何判定。

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::introspection::GuardEventDto;

/// 守卫判定落盘文件名（放在 `APEIRETH_DATA_DIR` 下）。
pub const GUARD_DECISION_LOG_FILE: &str = "guard-decisions.jsonl";

/// 读取时默认最多回多少条（防把整份历史读进内存）。
pub const DEFAULT_READ_LIMIT: usize = 512;

/// 守卫判定的 append-only JSONL 账。
///
/// `Clone` 是廉价共享（内部 `Arc<Mutex<..>>` 语义由调用方用 `Arc` 包一层即可），
/// 这里不实现 `Clone`，需要共享就 `Arc<GuardDecisionLog>`。
pub struct GuardDecisionLog {
    path: PathBuf,
    /// 写盘互斥（同进程内多线程 append 不交错）。
    write_lock: Mutex<()>,
}

impl std::fmt::Debug for GuardDecisionLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuardDecisionLog")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl GuardDecisionLog {
    /// 打开（必要时创建父目录）指定路径的判定账。
    ///
    /// 只创建目录、不创建文件 —— 文件在第一次 `append` 时出现
    /// （「没有判定发生过」不该留下一个空文件，那会让人以为审计面在跑）。
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("创建审计目录失败 ({}): {error}", parent.display()))?;
            }
        }
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    /// 账本路径（自述面 / 测试用）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一条判定。
    ///
    /// **写盘失败不影响守卫结论**：调用方应把 `Err` 计入指标后继续。
    pub fn append(&self, event: &GuardEventDto) -> Result<(), String> {
        let line =
            serde_json::to_string(event).map_err(|error| format!("判定事件序列化失败: {error}"))?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "判定账写锁中毒".to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("打开判定账失败 ({}): {error}", self.path.display()))?;
        writeln!(file, "{line}").map_err(|error| format!("写判定账失败: {error}"))
    }

    /// 读**最近** `limit` 条判定（按写入顺序，最老的在前）。
    ///
    /// - 文件不存在 ⇒ 空（不是错误：说明还没有判定发生过）；
    /// - 坏行 ⇒ 跳过（与 `egress_ledger` 同纪律：一行坏不毁整份历史）。
    pub fn read_recent(&self, limit: usize) -> Result<Vec<GuardEventDto>, String> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("读判定账失败: {error}")),
        };
        let mut events: Vec<GuardEventDto> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<GuardEventDto>(line).ok())
            .collect();
        if limit > 0 && events.len() > limit {
            // 保留最近的 limit 条（尾部）
            events.drain(..events.len() - limit);
        }
        Ok(events)
    }

    /// 账里现存多少条可解析判定（自述面 / 测试用）。
    pub fn len(&self) -> Result<usize, String> {
        Ok(self.read_recent(0)?.len())
    }

    /// 是否为空（无判定落盘）。
    pub fn is_empty(&self) -> Result<bool, String> {
        Ok(self.len()? == 0)
    }
}

/// `risk_score: f64` → `governance::risk` 闭合词表的档位标签。
///
/// **本仓唯一一处 score→档映射**（此前不存在）。分箱是均等的 0.2 步长，
/// 最后一档收窄到 0.95 以便留出 `nuclear` 的极高档位。
///
/// 不变式（有测试钉死）：**任何输入的输出都必须被
/// [`apeireth_governance::risk::risk_rank`] 识别**（rank >= 0）。
/// 否则守夜人的 no-degrade 比较会静默失效（全部 rank = -1 ⇒ 永不触发）。
pub fn risk_label_for_score(score: f64) -> &'static str {
    // NaN 归到最低档而不是最高档：不确定**不该**冒充「极高风险」，
    // 也不该冒充「安全」—— 归 `info`（rank 0）让 no-degrade 比较按「无信号」处理。
    if score.is_nan() || score < 0.2 {
        return "info";
    }
    if score < 0.4 {
        return "low";
    }
    if score < 0.6 {
        return "medium";
    }
    if score < 0.8 {
        return "high";
    }
    if score < 0.95 {
        return "critical";
    }
    "nuclear"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::GuardStage;

    fn event(score: f64, decision: &str) -> GuardEventDto {
        GuardEventDto {
            timestamp_ms: 1_700_000_000_000,
            session_id: "s".to_string(),
            trace_id: "t".to_string(),
            round: 1,
            capability_id: "tool.shell".to_string(),
            stage: GuardStage::FastGuard,
            decision: decision.to_string(),
            risk_score: score,
            reasons: vec!["why".to_string()],
            evidence: vec!["ev".to_string()],
            containment: None,
        }
    }

    #[test]
    fn append_then_read_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log =
            GuardDecisionLog::open(dir.path().join(GUARD_DECISION_LOG_FILE)).expect("open log");
        assert!(log.is_empty().expect("empty"), "新账应为空");

        log.append(&event(0.1, "allow")).expect("append allow");
        log.append(&event(0.9, "deny")).expect("append deny");

        let events = log.read_recent(10).expect("read");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].decision, "allow");
        assert_eq!(events[1].decision, "deny");
        assert_eq!(events[1].risk_score, 0.9);
    }

    #[test]
    fn read_recent_keeps_the_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = GuardDecisionLog::open(dir.path().join("d.jsonl")).expect("open");
        for index in 0..10 {
            log.append(&event(0.1 * index as f64, "allow"))
                .expect("append");
        }
        let tail = log.read_recent(3).expect("read");
        assert_eq!(tail.len(), 3, "必须只回最近 3 条");
        // 最近三条是 index 7/8/9 ⇒ score 0.7/0.8/0.9
        assert!((tail[2].risk_score - 0.9).abs() < 1e-9, "{tail:?}");
    }

    #[test]
    fn corrupt_lines_are_skipped_not_fatal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("d.jsonl");
        let log = GuardDecisionLog::open(&path).expect("open");
        log.append(&event(0.5, "allow")).expect("append");
        // 手工插入一行坏数据（模拟半行写入 / 磁盘损坏）
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open for corrupt");
            writeln!(file, "{{ not json at all").expect("write corrupt");
        }
        log.append(&event(0.6, "deny")).expect("append");

        let events = log.read_recent(10).expect("read");
        assert_eq!(
            events.len(),
            2,
            "坏行应被跳过而不是毁掉整份历史: {events:?}"
        );
    }

    #[test]
    fn missing_file_reads_as_empty_not_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = GuardDecisionLog::open(dir.path().join("never-written.jsonl")).expect("open");
        assert!(log.read_recent(10).expect("read").is_empty());
    }

    /// 风险档映射的**硬不变式**：输出必须全部落在 `governance::risk` 的闭合词表内。
    ///
    /// 这条是「守夜人 no-degrade 段不静默失效」的守卫 ——
    /// 若哪个标签不在词表里，`risk_rank` 返回 -1，两两比较永远不触发降级。
    #[test]
    fn risk_labels_are_always_in_the_closed_vocabulary() {
        let probes = [
            f64::NAN,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            0.19,
            0.2,
            0.39,
            0.4,
            0.59,
            0.6,
            0.79,
            0.8,
            0.94,
            0.95,
            0.99,
            1.0,
            f64::INFINITY,
        ];
        for score in probes {
            let label = risk_label_for_score(score);
            assert!(
                apeireth_governance::risk::risk_rank(label) >= 0,
                "score={score} 映射出的 `{label}` 不在 governance::risk 闭合词表内"
            );
        }
    }

    /// 映射必须**单调不减**（分数越高档越高），且能覆盖全部 6 档。
    #[test]
    fn risk_labels_are_monotone_and_cover_every_tier() {
        let mut seen: Vec<&'static str> = Vec::new();
        let mut last_rank = -1;
        for step in 0..=100 {
            let score = step as f64 / 100.0;
            let label = risk_label_for_score(score);
            let rank = apeireth_governance::risk::risk_rank(label);
            assert!(rank >= last_rank, "score={score} 出现降档: {label}");
            last_rank = rank;
            if seen.last() != Some(&label) {
                seen.push(label);
            }
        }
        assert_eq!(
            seen,
            vec!["info", "low", "medium", "high", "critical", "nuclear"],
            "6 档必须按序全部覆盖"
        );
    }

    /// NaN **不冒充**最高风险（不确定性 ≠ 极高风险）。
    #[test]
    fn nan_does_not_pose_as_nuclear() {
        assert_eq!(risk_label_for_score(f64::NAN), "info");
    }
}
