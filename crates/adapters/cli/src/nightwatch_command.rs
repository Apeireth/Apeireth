//! 守夜人 Nightwatch CLI 面 (report-only, 显式命令即授权)。
//!
//! 从 `lib.rs` 拆出的**真拆分** (2026-10-25): 装配根文件已达尺寸零余量上限
//! (`scripts/arch-baseline.txt` 钉死), 新增接线必须配套拆出真实单元, 不允许
//! 调基线抵扣。本模块承载两个守夜人命令:
//! - [`dispatch_nightwatch`]: 单次复盘 (五件组合审计 → 报告落 `<data>/nightwatch/`);
//! - [`dispatch_nightwatch_watch`]: 后台守护 (双闸: 活动闸 + 冷却闸)。
//!
//! 两者都**只读 + report-only**: 不阻塞、不批准、不改回合行为。

use std::sync::Arc;

use super::{cognitive_db_path, default_panel_data_dir, guard_decision_wiring};

pub async fn dispatch_nightwatch(session: Option<String>, limit: usize) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::nightwatch::{EpisodeSnapshot, NightwatchInputs};
    use apeireth_storage::SqliteConnectionPool;

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let pool = Arc::new(
        SqliteConnectionPool::open(cognitive_db_path())
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let episodes = match &session {
        Some(session_id) => backend
            .recent_episodes(session_id, limit)
            .map_err(|error| format!("read recent episodes failed: {error}"))?,
        None => Vec::new(),
    };
    let snapshots: Vec<EpisodeSnapshot> = episodes
        .iter()
        .map(|episode| EpisodeSnapshot {
            id: episode.id.clone(),
            session: session.clone().unwrap_or_else(|| "unknown".to_string()),
            role: episode.role.clone(),
            content: episode.content.clone(),
        })
        .collect();

    let inputs = NightwatchInputs {
        episodes: snapshots,
        // 守卫判定账（append-only）：映射口径见 `guard_decision_wiring`（含 score→档 定义）。
        audit: guard_decision_wiring::nightwatch_audit_snapshots(
            guard_decision_wiring::NIGHTWATCH_AUDIT_READ_LIMIT,
        ),
        // 装配根无条件接上判定落盘（`attach_decision_log` 恒被调用）⇒ 恒 true；
        // 文件不存在只说明「还没有判定记录」（按存在性判断会误报）。
        audit_source_wired: true,
        ..Default::default()
    };
    let report = apeireth_runtime_assembly::canonical::nightwatch_audit(&inputs);
    let report_dir = default_panel_data_dir().join("nightwatch");
    let path = apeireth_runtime_assembly::canonical::write_nightwatch_report(&report_dir, &report)
        .map_err(|error| format!("nightwatch report write failed: {error}"))?;

    let mut text = format!(
        "守夜复盘完成 (report-only, 不阻塞不批准):\n{}\n报告: {}\n",
        report.summary,
        path.display()
    );
    for finding in &report.findings {
        text.push_str(&format!(
            " - [{}] {:?}: {}\n",
            finding.severity, finding.area, finding.detail
        ));
    }
    for gap in &report.advisory_gaps {
        text.push_str(&format!(" - [gap] {gap}\n"));
    }
    Ok(text)
}

/// **守夜人后台守护** (2026-10-10, 设计闭环最后一块): `nightwatch --watch`
/// = 显式授权的长跑循环。**只在用户空闲时复盘** (双闸, 纯状态机):
/// ① 活动闸: 距最近活动 ≥ `idle_secs` —— 活动信号 = 认知库 episode 时间戳
///    (用户说话 = 新 episode; `--session` 指定观测会话);
/// ② 冷却闸: 距上次复盘 ≥ `cooldown_secs` (复盘不刷屏)。
/// 轮询间隔 `interval_secs`; 复盘复用单次审计路径 (同 DB 读 + 报告落盘)。
/// Ctrl-C 退出 (信号即终止; 报告已落盘, 无中间态)。
pub async fn dispatch_nightwatch_watch(
    session: Option<String>,
    limit: usize,
    idle_secs: i64,
    cooldown_secs: i64,
    interval_secs: u64,
) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::{IdleGate, NightwatchIdleScheduler};
    use apeireth_storage::SqliteConnectionPool;
    use std::time::{SystemTime, UNIX_EPOCH};

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let pool = Arc::new(
        SqliteConnectionPool::open(cognitive_db_path())
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let now_ms = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    };
    let mut scheduler = NightwatchIdleScheduler::new(idle_secs, cooldown_secs, now_ms());
    let mut runs = 0usize;

    loop {
        let now = now_ms();
        // 活动探测: 观测会话最近一条 episode 的时间戳 (epoch 秒 → 毫秒)。
        if let Some(session_id) = &session {
            if let Ok(episodes) = backend.recent_episodes(session_id, 1) {
                if let Some(latest) = episodes.first() {
                    scheduler.note_activity(latest.timestamp.saturating_mul(1000));
                }
            }
        }

        match scheduler.should_run(now) {
            IdleGate::IdleOk => {
                scheduler.mark_ran(now);
                runs += 1;
                let summary = dispatch_nightwatch(session.clone(), limit).await?;
                println!("[nightwatch --watch] 第 {runs} 次空闲复盘:\n{summary}");
            }
            IdleGate::ActiveSoon { wait_secs } => {
                eprintln!("[nightwatch --watch] 用户仍活跃, {wait_secs}s 后再判");
            }
            IdleGate::RecentlyRan { wait_secs } => {
                eprintln!("[nightwatch --watch] 冷却中, {wait_secs}s 后再判");
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs.max(1))).await;
    }
}
