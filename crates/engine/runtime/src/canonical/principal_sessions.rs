//! 「**主体 → 会话**」归属映射的**端口**（2026-11-01 安全审计 P1 收口）。
//!
//! # 为什么端口在 runtime
//!
//! 归属映射要写进**记忆库**（`apeireth-memory`），而：
//! * `apeireth-gateway`（请求面）**不依赖** memory；
//! * `apeireth-runtime` 被 gateway / runtime-assembly / CLI **都依赖**
//!   ⇒ ⇒ 端口放 runtime ⇒ **无循环依赖**，且三方都能用。
//!
//! ⇒ 实现（持有 memory store 的那个）放**装配层**（CLI）注入 ⇒ 端口与实现分离。
//!
//! # 主体语义（**与 memory 侧一致**）
//!
//! `principal_id` = **人 / 账户级**标识（域面 `DomainUserId` = 令牌绑定的 `user_id`）。
//! ⇒ **不是**客户端署名键（令牌名 / `anonymous` / `unattributed`）
//! ⇒ ⇒ 调用方**只应在能解析出真实 `user_id` 时**登记 ⇒
//! **无主体 ⇒ 不登记**（宁可「归属不可判定」，也不要错归属 ⇒ 误删风险）。
//!
//! # 失败口径
//!
//! 写失败 ⇒ **不阻断主流程**（这是**旁路留痕**：主体忘了 ⇒ episode 归不到主体
//! ⇒ 那一维的「被遗忘权」不生效 ⇒ 由 [`crate::canonical::universal_forget`]
//! 那一侧的 `AttributionStatus` 把缺口**显式暴露**给自述面）。
//! ⇒ ⇒ 调用方**记审计**，不抛错。
//!
//! # 槽位模型（2026-11-05 修：进程级**单槽** ⇒ **全局默认槽 + 作用域内槽覆盖**）
//!
//! 旧模型是**进程级单槽**：一个 `static RECORDER: RwLock<Option<..>>`
//! ⇒ ⇒ 并行用例**互相顶替** / 一个用例的 `Drop` **误杀**另一个用例的写面 / 串数据。
//!
//! 新模型**分两层**（读侧**先作用域、后默认**）：
//!
//! | 层 | 载体 | 谁用 | 语义 |
//! |---|---|---|---|
//! | **全局默认槽** | 进程级 `static RwLock` | **生产** | 跨线程、跨请求、装一次 |
//! | **作用域内槽** | **线程局部**栈 | **测试** | 每个用例 push/pop 自己那层 |
//!
//! **读侧规则**：**先看作用域内栈**（栈顶）⇒ ⇒ 没有才**回落到全局默认槽**。
//!
//! ⇒ ⇒ **生产行为完全不变**（生产永不带作用域 ⇒ 恒命中全局默认槽）。
//! ⇒ ⇒ **测试并行不再互相顶替**（各用例各推各的层）。
//!
//! # 为什么全局默认槽必须是**进程级**（**不能**改成线程局部）
//!
//! 生产的**读侧**在 **HTTP handler** 里（`apeireth-gateway` 的 `post(native_chat)`
//! ⇒ **tokio worker 线程**），而**装入侧**在**启动线程**（CLI 装配）
//! ⇒ ⇒ **两者不是同一个线程** ⇒ ⇒ 写面必须**跨线程可见**
//! ⇒ ⇒ 若把槽整体改成线程局部 ⇒ handler 线程读不到 ⇒ **归属被静默丢弃**
//! （不报错、只是不记 ⇒ 最坏的一类故障）。
//! ⇒ ⇒ 故：**作用域内槽只作测试覆盖**，**生产恒走全局默认槽**。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::ThreadId;

/// 归属映射的写面（实现方持有 memory store）。
pub trait PrincipalSessionRecorder: Send + Sync {
    /// 登记一条「主体 → 会话」映射（**幂等**）。
    ///
    /// `principal_id` / `session_id` 为空 ⇒ 实现方**必须拒**（fail-closed）。
    fn record(&self, principal_id: &str, session_id: &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// 层一：**全局默认槽**（进程级）—— 生产用这个
// ---------------------------------------------------------------------------

/// 进程级的**默认**写面槽（**未装入 = 不登记** ⇒ 零行为变化）。
///
/// 用 `RwLock<Option<..>>`（不是 `OnceLock`）⇒ **可换指针**：
/// 装配面重装时以**最后一次**为准（与 `live_policy` 的共享槽语义一致）。
///
/// ⚠️ **必须进程级**（理由见模块头「为什么全局默认槽必须是进程级」）：
/// 装入在**启动线程**、读在 **tokio worker 线程** ⇒ 线程局部 ⇒ 读不到。
static DEFAULT_RECORDER: RwLock<Option<Arc<dyn PrincipalSessionRecorder>>> = RwLock::new(None);

// ---------------------------------------------------------------------------
// 层二：**作用域内槽**（线程局部栈）—— 测试用这个
// ---------------------------------------------------------------------------

/// 作用域内槽的一层。
struct ScopeLayer {
    /// 本层装入的写面。
    recorder: Arc<dyn PrincipalSessionRecorder>,
    /// 本层**是否仍然活着**（跨线程 Drop 时把本层标记为失效 ⇒ 读侧跳过它）。
    alive: AtomicBool,
}

// **线程局部**的作用域内槽（栈）。
//
// ⚠️ **只作测试覆盖用** —— 生产**恒走全局默认槽**（见模块头）。
// ⚠️ **线程局部 ⇒ 作用域不跨线程**：作用域内的读必须发生在**推入的同一线程**上。
// ⚠️ 注：`thread_local!` 块**不是 item** ⇒ ⇒ rustdoc **不认** `///`（clippy: unused doc comment）
// ⇒ ⇒ 这里用普通注释（内容与模块头的说明一致 ⇒ 不丢信息）。
thread_local! {
    static SCOPED_RECORDERS: RefCell<Vec<Arc<ScopeLayer>>> =
        const { RefCell::new(Vec::new()) };
}

/// 作用域守卫（RAII）。
///
/// # Drop 语义（**本修复的要害**）
///
/// * **只弹自己那一层** ⇒ ⇒ **不清全局默认槽** ⇒ ⇒ 一个用例结束**不会误杀**
///   另一个用例（或生产装配）的写面；
/// * 本层带**身份**（`Arc::ptr_eq` 核对）⇒ ⇒ 即使**乱序 Drop**，也只摘自己那层，
///   不会摘掉别人的层；
/// * **跨线程 Drop**（例如 async 任务迁移了线程）⇒ **不碰别的线程的栈**
///   （那会误弹别人的层）⇒ 只把本层标记为**失效** ⇒ 原线程后续读**跳过**它。
#[must_use = "作用域守卫立刻被丢弃 ⇒ 作用域等于没进 ⇒ 请把它绑定到变量上"]
pub struct ScopeGuard {
    /// 本层（身份核对用）。
    layer: Arc<ScopeLayer>,
    /// 推入时的栈深度（1 = 最底层）。
    depth: usize,
    /// 推入时的线程（跨线程 Drop 时据此判断「能不能碰栈」）。
    owner: ThreadId,
}

impl ScopeGuard {
    /// 本作用域装的写面。
    pub fn recorder(&self) -> &Arc<dyn PrincipalSessionRecorder> {
        &self.layer.recorder
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        // 无论在哪个线程 Drop ⇒ 本层先**失效**（读侧立刻不再认它）。
        self.layer.alive.store(false, Ordering::Release);
        if std::thread::current().id() != self.owner {
            // ⚠️ **跨线程 Drop**：不能碰**别的线程**的栈（那会误弹别人的层）
            // ⇒ ⇒ 原线程上留下一个「已失效」的层 ⇒ 读侧会跳过它（见 `scoped_recorder`）。
            return;
        }
        SCOPED_RECORDERS.with(|stack| {
            let mut stack = stack.borrow_mut();
            let index = self.depth.saturating_sub(1);
            if index < stack.len() && Arc::ptr_eq(&stack[index], &self.layer) {
                stack.remove(index);
                return;
            }
            // 兜底：**乱序 Drop** ⇒ 深度可能已被别人弹出而错位 ⇒ 按身份摘自己那层。
            if let Some(found) = stack.iter().position(|item| Arc::ptr_eq(item, &self.layer)) {
                stack.remove(found);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// 装入 / 卸下
// ---------------------------------------------------------------------------

/// 装入写面实现（装配层调用；重复装入以**最后一次**为准）。
///
/// ⚠️ 装的是**全局默认槽** ⇒ ⇒ **跨线程可见** ⇒ ⇒ 生产读侧（tokio worker 线程）读得到。
pub fn install_principal_session_recorder(recorder: Arc<dyn PrincipalSessionRecorder>) {
    let mut slot = DEFAULT_RECORDER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(recorder);
}

/// 卸下**全局默认槽**的写面实现（回到「未装入 ⇒ 不登记」）。
///
/// # 为什么需要
///
/// 写面是**进程级**槽 ⇒ 测试里装入后**必须能卸下** ⇒ 否则
/// **同进程内其它用例会继承这个假写面**（跨用例污染 ⇒ 顺序敏感 ⇒ 偶发红）。
///
/// ⇒ 装配面正常流程**不需要**它（进程生命周期内装一次即可）⇒
/// ⇒ 本函数是给**测试隔离**与**热重装**用的。
///
/// # 口径（**显式化**）
///
/// 本函数清的是**全局默认槽** ⇒ ⇒ **作用域内槽不受影响**
/// （作用域内槽由 [`ScopeGuard`] 的 Drop 按层释放）。
pub fn clear_default_principal_session_recorder() {
    let mut slot = DEFAULT_RECORDER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = None;
}

/// 卸下写面实现（**旧名薄壳** ⇒ 转发到 [`clear_default_principal_session_recorder`]）。
///
/// # 弃用理由
///
/// 旧名**看不出**清的是「默认槽」还是「作用域槽」⇒ ⇒ 显式化。
/// ⇒ ⇒ 本函数**只**清**全局默认槽**（作用域内槽**不受影响**）。
#[deprecated(
    note = "改名以显式化语义：清的是「全局默认槽」。请改用 clear_default_principal_session_recorder()"
)]
pub fn clear_principal_session_recorder() {
    clear_default_principal_session_recorder();
}

/// 进入一个**作用域**（**线程局部** ⇒ 作用域内槽覆盖全局默认槽）。
///
/// # 用法
///
/// ```ignore
/// let _scope = principal_sessions::enter_scope(fake_recorder);
/// // ⇒ 本线程内的读命中 `fake_recorder`
/// // ⇒ `_scope` 被 Drop 时**只弹这一层** ⇒ 不动全局默认槽
/// ```
///
/// # 边界（**诚实口径**）
///
/// * **线程局部** ⇒ ⇒ 作用域**不跨线程**：在别的线程读 ⇒ 读不到这一层
///   ⇒ ⇒ 回落**全局默认槽**（生产读侧恒走这一条 ⇒ 行为不变）。
/// * async 任务在 `multi_thread` 运行时里**可能迁移线程** ⇒ ⇒ 别把守卫跨
///   `.await` 长期持有（`#[tokio::test]` 默认 current_thread ⇒ 不迁移）。
pub fn enter_scope(recorder: Arc<dyn PrincipalSessionRecorder>) -> ScopeGuard {
    let layer = Arc::new(ScopeLayer {
        recorder,
        alive: AtomicBool::new(true),
    });
    let depth = SCOPED_RECORDERS.with(|stack| {
        let mut stack = stack.borrow_mut();
        stack.push(Arc::clone(&layer));
        stack.len()
    });
    ScopeGuard {
        layer,
        depth,
        owner: std::thread::current().id(),
    }
}

// ---------------------------------------------------------------------------
// 读侧
// ---------------------------------------------------------------------------

/// **作用域内槽**的写面（栈顶；无作用域 ⇒ `None`）。
///
/// ⇒ ⇒ **只看作用域** ⇒ ⇒ 用来断言「全局默认槽没被动过」。
pub fn scoped_recorder() -> Option<Arc<dyn PrincipalSessionRecorder>> {
    SCOPED_RECORDERS.with(|stack| {
        let stack = stack.borrow();
        stack
            .iter()
            .rev()
            .find(|layer| layer.alive.load(Ordering::Acquire))
            .map(|layer| layer.recorder.clone())
    })
}

/// **全局默认槽**的写面（`None` = 未装入）。
pub fn default_recorder() -> Option<Arc<dyn PrincipalSessionRecorder>> {
    let slot = DEFAULT_RECORDER
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.clone()
}

/// 当前生效的写面（**先作用域内槽 ⇒ 没有才回落全局默认槽**）。
///
/// * **生产**（永不带作用域）⇒ ⇒ 恒命中**全局默认槽** ⇒ ⇒ **行为完全不变**。
/// * **测试**（带作用域）⇒ ⇒ 命中**作用域内槽** ⇒ ⇒ 并行用例互不顶替。
pub fn current_recorder() -> Option<Arc<dyn PrincipalSessionRecorder>> {
    scoped_recorder().or_else(default_recorder)
}

/// 登记一条「主体 → 会话」映射（**旁路**：失败不抛错，只返回结果）。
///
/// * `principal_id` 或 `session_id` 为空 ⇒ **不登记**（返回 `Ok(false)`；
///   理由：**空主体会把所有人的 episode 归到同一主体** ⇒ 误删风险）。
/// * 未装入写面 ⇒ **不登记**（返回 `Ok(false)`）。
/// * 写失败 ⇒ 返回 `Err`（真因如实）⇒ 调用方**记审计**，**不阻断主流程**。
pub fn record_principal_session(principal_id: &str, session_id: &str) -> Result<bool, String> {
    let principal_id = principal_id.trim();
    let session_id = session_id.trim();
    if principal_id.is_empty() || session_id.is_empty() {
        return Ok(false);
    }
    let Some(recorder) = current_recorder() else {
        return Ok(false);
    };
    recorder.record(principal_id, session_id)?;
    Ok(true)
}
