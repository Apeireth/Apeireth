//! 回合级取消注册表 (后端合同 C 批 · C2-2 段)。
//!
//! 目标形态:
//! - 回合开跑时以 `(session_id, request_id)` 注册一个取消令牌; 工具派发前
//!   ([crate::canonical::module_host] 的 `bind_admitted_tool`) 把该令牌作为父
//!   取消源接到本次调用的 `ToolCancellation` 上;
//! - `stop(session, request)` 取消运行时持有的令牌 ⇒ 已派发/后续派发的工具在
//!   dispatcher 的取消检查处被阻断 (只阻断**后续工具派发与提示工具**, 已发生的
//!   副作用不撤销);
//! - 第一次 stop 返回 [`StopOutcome::Stopped`] 并把摘要留在有界队列里 ⇒ 后续
//!   重复 stop 返回同一摘要 ([`StopOutcome::AlreadyStopped`]), 即幂等应答;
//! - 未知键返回 [`StopOutcome::NotFound`] (由网关映射 404)。
//!
//! 不做什么 (诚实边界):
//! - 注册表是**进程内内存**: 进程重启即失, 多进程不共享;
//! - 只做取消信号, 不做终帧发布 (C1 帧族的 `stopped` 终帧归 C2-3);
//! - 不杀死已经在执行的同步代码, 也不回滚已完成副作用。
//!
//! 进程内注册表是唯一事实: [`StopSummary`] 只由第一次 stop 冻结, 重复 stop
//! 原样回放同一份摘要 (不重新读取可变状态)。

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard, OnceLock};

use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_plugin::ToolCancellation;
use serde::Serialize;

/// 已停止摘要的保留条数 (有界队列, 淘汰最老者)。
pub const RETAINED_STOP_SUMMARIES: usize = 512;

/// 停止终态 (wire 字面量固定为 `stopped`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopState {
    /// 该回合已被显式停止。
    Stopped,
}

/// 第一次 stop 冻结的摘要 (重复 stop 原样回放同一份)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StopSummary {
    /// 会话 id。
    pub session_id: SessionId,
    /// 运行时请求 id (与流式 SSE chunk id 同源)。
    pub request_id: RequestId,
    /// 终态 (恒为 `stopped`)。
    pub state: StopState,
    /// 停止只阻断后续工具派发: 已发生的副作用不撤销 ⇒ 恒为 `true`。
    pub effects_may_be_incomplete: bool,
}

/// stop 的三态结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// 本次 stop 命中了在飞回合 (摘要由本次冻结)。
    Stopped(StopSummary),
    /// 同一键此前已 stop 过 (幂等回放同一摘要)。
    AlreadyStopped(StopSummary),
    /// 没有该键的运行中回合 (也没有历史摘要)。
    NotFound,
}

struct Entry {
    token: ToolCancellation,
    generation: u64,
}

#[derive(Default)]
struct Registry {
    running: HashMap<(SessionId, RequestId), Entry>,
    stopped: VecDeque<((SessionId, RequestId), StopSummary)>,
    next_generation: u64,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

/// 进程内注册表只有一把锁; 锁中毒不改变事实, 取回内层继续。
fn lock_registry() -> MutexGuard<'static, Registry> {
    registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 注册一个回合的取消令牌, 返回随回合存活的作用域守卫。
///
/// 同键重复注册会替换旧条目 (旧守卫因 generation 不匹配而不再删除新条目),
/// 并丢弃该键上陈旧的停止摘要 ⇒ id 复用时不会拿到上一回合的摘要。
#[must_use]
pub fn register(session: SessionId, request: RequestId) -> TurnGuard {
    let mut registry = lock_registry();
    registry.next_generation = registry.next_generation.wrapping_add(1);
    let generation = registry.next_generation;
    let key = (session, request);
    registry.stopped.retain(|(existing, _)| existing != &key);
    registry.running.insert(
        key,
        Entry {
            token: ToolCancellation::default(),
            generation,
        },
    );
    TurnGuard {
        session,
        request,
        generation,
    }
}

/// 为一次工具派发构造取消令牌: 在父取消源之外接上该回合的取消令牌。
///
/// 回合未注册时等价于只接父取消源 (与 [`ToolCancellation::joined`] 语义一致)。
#[must_use]
pub fn cancellation_for(
    session: SessionId,
    request: RequestId,
    parents: Vec<ToolCancellation>,
) -> ToolCancellation {
    let mut parents = parents;
    {
        let registry = lock_registry();
        if let Some(entry) = registry.running.get(&(session, request)) {
            parents.push(entry.token.clone());
        }
    }
    ToolCancellation::joined(parents)
}

/// 停止一个在飞回合; 同键重复调用幂等回放第一次的摘要。
///
/// 已发生的副作用不撤销, 只阻断后续工具派发与提示工具。
#[must_use]
pub fn stop(session: SessionId, request: RequestId) -> StopOutcome {
    let mut registry = lock_registry();
    let key = (session, request);
    if let Some((_, summary)) = registry
        .stopped
        .iter()
        .find(|(existing, _)| existing == &key)
    {
        return StopOutcome::AlreadyStopped(summary.clone());
    }
    let Some(entry) = registry.running.get(&key) else {
        return StopOutcome::NotFound;
    };
    entry.token.cancel();
    let summary = StopSummary {
        session_id: session,
        request_id: request,
        state: StopState::Stopped,
        effects_may_be_incomplete: true,
    };
    registry.stopped.push_back((key, summary.clone()));
    while registry.stopped.len() > RETAINED_STOP_SUMMARIES {
        registry.stopped.pop_front();
    }
    StopOutcome::Stopped(summary)
}

/// 回合作用域守卫: 回合结束时注销运行中条目 (停止摘要保留, 供幂等回放)。
#[derive(Debug)]
pub struct TurnGuard {
    session: SessionId,
    request: RequestId,
    generation: u64,
}

impl Drop for TurnGuard {
    fn drop(&mut self) {
        let mut registry = lock_registry();
        let key = (self.session, self.request);
        if registry
            .running
            .get(&key)
            .is_some_and(|entry| entry.generation == self.generation)
        {
            registry.running.remove(&key);
        }
    }
}
