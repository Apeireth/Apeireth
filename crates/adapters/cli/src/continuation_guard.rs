//! 回合级**中断检测**（`orchestration::continuation` 的 CLI 接线，安全子集）。
//!
//! # 为什么只做「检测」不做「自动续跑」
//!
//! `ContinuationSnapshot` 的原始设计是「断点续跑」。但回合在**工具调用中途**
//! 崩溃时，续跑意味着**重放该调用** —— 写文件/发消息这类副作用会**执行两次**。
//! `ContinuationStore::consume` 的「一次性销毁」只防重复**消费快照**，不防重复
//! **执行副作用**。因此自动续跑属产品语义决策（自动 vs 报告后人工决定），
//! 不在这批擅自接上。
//!
//! 本模块做**确定能做的那一半**：回合开始前落快照、正常结束即删；若下次同会话
//! 启动时快照还在 ⇒ **说明上次回合没跑完**（崩溃 / 强杀 / 断电），如实告诉用户。
//!
//! # 三条纪律
//!
//! 1. **不存正文**：快照里 `messages` 恒为空 —— 检测中断只需要「这个会话有一轮没
//!    收尾」，不需要用户说了什么。少落一份正文 = 少一份泄漏面。
//! 2. **不自动重放**：只报「上次中断」，要不要重试由用户决定。
//! 3. **报告即清**：报过一次就删快照（同一中断不反复打扰）。
//!
//! # 只用 `load` + `delete`，不用 `consume`
//!
//! `consume` 会留下「已消费声明」，而本轮快照 id 是**按会话固定**的
//! （`turn-<session>`）⇒ 声明会把后续回合的 `save` 永久挡成 `AlreadyConsumed`。
//! 故用 `load` + `delete`（语义上等价于「读后清」，不写声明）。

use apeireth_orchestration::continuation::{
    ContinuationSnapshot, ContinuationStore, ContinuationStoreError, FileContinuationStore,
};

/// 快照目录名（数据根下）。
const CONTINUATION_DIR: &str = "continuations";

/// 默认快照存储（`<数据根>/continuations`）。
pub fn default_store() -> FileContinuationStore {
    FileContinuationStore::new(apeireth_cli::default_panel_data_dir().join(CONTINUATION_DIR))
}

/// 快照 id（按会话固定：一个会话同时只可能有**一个**未收尾回合）。
fn snapshot_id(session: &str) -> String {
    format!("turn-{session}")
}

/// 一次已开启的回合（Drop 不自动清理 —— 崩溃才该留下痕迹）。
#[derive(Debug)]
pub struct OpenTurn {
    /// 快照 id（收尾时按它删）。
    pub id: String,
}

/// 一次「上次中断」的告知（**已消费**该快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterruptedNotice {
    /// 可直接打印的提示行。
    pub text: String,
    /// **被中断那轮**的序号（≥1）。调用方据此推下一轮序号（`turn + 1`）。
    ///
    /// 真值来自被删快照的 `turn` 字段（不重算、不猜测）⇒ 提示语里的
    /// 「第 N 轮」是**真值**，不是恒 1。
    pub turn: u64,
}

/// 检查并**消费**「上次中断」的通知（报一次即清）。返回可打印的提示行。
///
/// 无中断（`NotFound`）= `None`；存储异常 = 如实报错字符串（不吞、不假装正常）。
pub fn take_interrupted_notice(
    store: &FileContinuationStore,
    session: &str,
    now_ms: i64,
) -> Option<InterruptedNotice> {
    let id = snapshot_id(session);
    match store.load(&id) {
        Ok(snapshot) => {
            let _ = store.delete(&id);
            let age_secs = now_ms.saturating_sub(snapshot.saved_at_epoch_ms) / 1_000;
            Some(InterruptedNotice {
                text: format!(
                    "[续行] 上次回合未正常结束（第 {} 轮，约 {} 秒前）—— 可能崩溃或被中断；\
                     本次是新回合，不会自动重放上次的工具调用",
                    snapshot.turn, age_secs
                ),
                turn: snapshot.turn,
            })
        }
        Err(ContinuationStoreError::NotFound { .. }) => None,
        Err(error) => Some(InterruptedNotice {
            text: format!("[续行] 快照读取异常（如实报告，不影响本轮）: {error}"),
            // 读取异常 ⇒ 拿不到真轮号 ⇒ 0（调用方据此回落到第 1 轮，不猜）。
            turn: 0,
        }),
    }
}

/// 回合开始：落快照（不存正文），返回句柄供正常结束时删除。
pub fn begin_turn(
    store: &FileContinuationStore,
    session: &str,
    now_ms: i64,
    turn: u64,
) -> Result<OpenTurn, String> {
    let id = snapshot_id(session);
    let snapshot = ContinuationSnapshot::new(
        session.to_string(),
        Vec::new(), // 纪律 ①：不存正文
        None,       // None = 常规轮次断点（非工具调用中断）
        now_ms,
        turn,
    );
    let mut snapshot = snapshot;
    snapshot.id = id.clone();
    store
        .save(&snapshot)
        .map_err(|error| format!("续行快照落盘失败: {error}"))?;
    Ok(OpenTurn { id })
}

/// 回合正常收尾（成功 / 业务错误 / 挂起待审批都算「没崩」）：删快照。
pub fn finish_turn(store: &FileContinuationStore, open: &OpenTurn) {
    let _ = store.delete(&open.id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> FileContinuationStore {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-cont-guard-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        FileContinuationStore::new(dir)
    }

    /// 正常收尾 ⇒ 不留快照 ⇒ 下次启动不报中断。
    #[test]
    fn finished_turn_leaves_no_snapshot() {
        let store = scratch("finish");
        let open = begin_turn(&store, "s-1", 1_000, 1).expect("落快照");
        finish_turn(&store, &open);
        assert!(take_interrupted_notice(&store, "s-1", 2_000).is_none());
    }

    /// 未收尾（模拟崩溃/强杀）⇒ 下次启动报「上次回合未正常结束」，且**只报一次**。
    #[test]
    fn unfinished_turn_is_reported_once() {
        let store = scratch("crash");
        let _open = begin_turn(&store, "s-2", 1_000, 3).expect("落快照");
        // 不调用 finish_turn ⇒ 相当于进程在回合中途消失
        let notice = take_interrupted_notice(&store, "s-2", 61_000).expect("应报中断");
        assert!(notice.text.contains("第 3 轮"), "{notice:?}");
        assert!(notice.text.contains("不会自动重放"), "{notice:?}");
        // 报过一次即清（同一中断不反复打扰）
        assert!(take_interrupted_notice(&store, "s-2", 62_000).is_none());
    }

    /// 报过中断后，**同会话可以继续落新快照**（id 固定但不被消费声明挡死）。
    #[test]
    fn session_can_start_a_new_turn_after_interruption_report() {
        let store = scratch("resume");
        let _crashed = begin_turn(&store, "s-3", 1_000, 1).expect("落快照");
        assert!(take_interrupted_notice(&store, "s-3", 2_000).is_some());
        let next = begin_turn(&store, "s-3", 3_000, 2);
        assert!(next.is_ok(), "中断报过之后必须能开新回合: {next:?}");
        finish_turn(&store, &next.expect("open"));
        assert!(take_interrupted_notice(&store, "s-3", 4_000).is_none());
    }

    /// 快照里**不存正文**（纪律 ①）：用户说了什么不应该因中断检测落盘。
    #[test]
    fn snapshot_carries_no_message_content() {
        let store = scratch("privacy");
        let open = begin_turn(&store, "s-4", 1_000, 1).expect("落快照");
        let snapshot = store.load(&open.id).expect("读回");
        assert!(
            snapshot.messages.is_empty(),
            "不得存正文: {:?}",
            snapshot.messages
        );
        assert!(snapshot.pending_tool_call.is_none());
    }
}
