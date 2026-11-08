//! 深度维 (调用树深度 / 推理深度): 会话级账本 + 共享句柄 + 嵌套作用域守卫。
//!
//! # 口径 (逐字)
//!
//! * **深度 = 运行时调用树读数** —— 既有读数, 不新造: 顶层回合 = 0
//!   ([`super::super::module::InvocationContext::depth`]), 隔离侧车调用 = 1
//!   (spawner 的 `parent_depth + 1`)。
//! * **可达深度 ≤ 1** —— 存量结构里 `InvocationContext` 只有 `user_turn()`
//!   (depth = 0) ⇒ 侧车调用的子层**恒为 1**; 且
//!   [`super::super::module::DEFAULT_MAX_INVOCATION_DEPTH`] = 1 ⇒ 运行时自己在
//!   depth 2 就拒 ⇒ **上限 > 1 不可达** (配了也不触发)。⚠️ **本维不是有效的
//!   深度防护**: 它报的是运行时调用树读数, 不是"思考深度"本身。
//! * **只留一套机制** —— 喂深度只走**绝对**深度
//!   ([`DepthQuota::observe_depth`]); 相对进/出层 ([`DepthQuota::enter`] /
//!   [`DepthQuota::exit`]) **禁止**与绝对深度混用: 两套口径可脱钩 (相对口径
//!   只抬自己的计数, 绝对口径只抬峰值) ⇒ 上限可被"分摊到多条路径"绕过。
//!   生产接线只用绝对深度。
//! * **峰值单调不降** (只由绝对深度观测抬升, 出栈不洗白); **当前层随嵌套
//!   作用域回落** —— 作用域守卫 [`DepthScope`] 是 RAII ⇒ 早退 / `?` / panic
//!   都不抬高当前层 (不必靠自觉配对)。
//! * **反序列化只接受上限**: 计数从 0 起 (调用树在进程重启后本就不存在)
//!   ⇒ 改会话存储**伪造不出**计数 (会话存储 = 可信面, 但伪造计数无效)。

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{QuotaDimension, QuotaExceeded};

/// Largest session call-tree depth a configuration value is clamped to.
///
/// 家族钳制锚点与 [`super::MAX_QUOTA_TOKENS`] / [`super::MAX_QUOTA_TOOL_STEPS`] /
/// [`super::MAX_QUOTA_COST_MICROS`] 同口径 (u32::MAX 层): 深度计数是嵌套层数
/// (u32), 越界值钳到锚点, 不发明更深的"预算"。
pub const MAX_QUOTA_DEPTH: u64 = u32::MAX as u64;

/// 深度维上限的**宽松**解析 (配额家族同纪律: 非法值 = 不设限)。
///
/// ⚠️ **非生产路径**: 深度维打错的代价是"思考深度无上限" (fail-open), 故生产
/// 走 [`parse_quota_max_depth_strict`] (非法值明确报错, 绝不静默放行)。
/// 保留本函数只为与配额家族同形 (其余三维同口径)。
pub fn parse_quota_max_depth(raw: Option<&str>) -> Option<u64> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(MAX_QUOTA_DEPTH))
}

/// 深度维上限的**严格**解析 (生产路径)。
///
/// * 未设 / 空 / 全空白 ⇒ `Ok(None)` = 不设限 (opt-in 默认, **不是**错误);
/// * 正整数 ⇒ `Ok(Some(n))`, 越界钳到 [`MAX_QUOTA_DEPTH`] (两端一致: 超大值
///   钳到锚点, **不**因解析失败而变成"不设限");
/// * 0 / 负数 / 非数字 / 小数 ⇒ `Err` (**明确报错**, 绝不静默放行成"无上限")。
pub fn parse_quota_max_depth_strict(raw: Option<&str>) -> Result<Option<u64>, String> {
    let Some(text) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let value: u64 = text
        .parse::<u64>()
        .map_err(|_| format!("depth quota ceiling must be a positive integer, got {text:?}"))?;
    if value == 0 {
        return Err("depth quota ceiling must be a positive integer, got 0".to_string());
    }
    Ok(Some(value.min(MAX_QUOTA_DEPTH)))
}

/// 深度维计量器 (会话级账本的值形态) —— 真计量, 不是占位。
///
/// * 已耗 = **峰值深度** (单调不降: 出栈不洗白已经越限的树);
/// * 耗尽 = `peak_depth >= max_depth` (与 token / 步数 / 花费同读法:
///   恰好等于上限即耗尽);
/// * 上限 = `max_depth` = 允许嵌套到的最大层数 (`None` = 不设限)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DepthQuota {
    /// 深度上限 (可嵌套的最大层数); `None` = 不设限 (opt-in 默认)。
    max_depth: Option<u64>,
    /// 当前占用的层 (嵌套作用域内; 出作用域回落)。
    current_depth: u32,
    /// 本会话已达的最深层数 (高水位, 单调不降)。
    peak_depth: u32,
}

impl DepthQuota {
    /// 不设限的深度计量器 (默认: 深度维是 opt-in)。
    pub const fn unlimited() -> Self {
        Self {
            max_depth: None,
            current_depth: 0,
            peak_depth: 0,
        }
    }

    /// 带上限的深度计量器 (`None` = 不设限)。
    ///
    /// 注: 上限 `0` 只可能由直接构造得到 (配置解析把 0 读成"不设限" 或
    /// **报错**, 见 [`parse_quota_max_depth_strict`]), 语义是"不容许任何嵌套
    /// 层" —— 顶层回合即到顶。
    pub const fn with_max_depth(max_depth: Option<u64>) -> Self {
        Self {
            max_depth,
            current_depth: 0,
            peak_depth: 0,
        }
    }

    /// 深度上限 (`None` = 不设限)。
    pub const fn max_depth(&self) -> Option<u64> {
        self.max_depth
    }

    /// 当前占用的层 (嵌套作用域内; 出作用域回落; 永不为负)。
    pub const fn current_depth(&self) -> u32 {
        self.current_depth
    }

    /// 峰值深度 (高水位, 单调不降) —— 自述面的"已耗"取它。
    pub const fn peak_depth(&self) -> u32 {
        self.peak_depth
    }

    /// 已耗口径 = 峰值深度 (自述面直接用这个数)。
    pub fn consumed(&self) -> u64 {
        u64::from(self.peak_depth)
    }

    /// 自述读数 (接线点: 自述面的深度维格子填这份真值, 不再是 null)。
    pub const fn reading(&self) -> DepthQuotaReading {
        DepthQuotaReading {
            limit: self.max_depth,
            current_depth: self.current_depth,
            peak_depth: self.peak_depth,
        }
    }

    /// 耗尽事实 (`None` = 未到顶)。
    ///
    /// 读法与其它三维一致: 已耗 (`peak_depth`) >= 上限 (`max_depth`) 即耗尽,
    /// 事实带真数字 (上限 / 已耗), 从不编造。
    pub fn check(&self) -> Option<QuotaExceeded> {
        let limit = self.max_depth?;
        if u64::from(self.peak_depth) >= limit {
            Some(QuotaExceeded {
                dimension: QuotaDimension::Depth,
                limit,
                consumed: u64::from(self.peak_depth),
            })
        } else {
            None
        }
    }

    /// 观测一个**绝对**深度 (生产接线唯一的喂入口)。
    ///
    /// 绝对 = 既有调用树读数给出的层数 (顶层 0 / 侧车 1), 不是"又深了几层"。
    /// 峰值单调不降; 当前层取"当前占用的最深一层" (嵌套作用域的守卫负责回落)。
    /// 返回抬升后的峰值。
    pub fn observe_depth(&mut self, depth: u32) -> u32 {
        self.peak_depth = self.peak_depth.max(depth);
        self.current_depth = self.current_depth.max(depth);
        self.peak_depth
    }

    /// 相对进层 (相对口径 —— **禁止**与绝对深度混用; 见模块头契约)。
    ///
    /// 只留给相对口径的单元面; 生产接线不走这里。
    pub fn enter(&mut self) -> Result<u32, QuotaExceeded> {
        if let Some(exceeded) = self.check() {
            return Err(exceeded);
        }
        self.current_depth = self.current_depth.saturating_add(1);
        self.peak_depth = self.peak_depth.max(self.current_depth);
        Ok(self.current_depth)
    }

    /// 相对出层 (相对口径 —— **禁止**与绝对深度混用; 见模块头契约)。
    ///
    /// 只留给相对口径的单元面; 生产接线不走这里 (生产走 [`DepthScope`] 守卫)。
    pub fn exit(&mut self) -> u32 {
        self.current_depth = self.current_depth.saturating_sub(1);
        self.current_depth
    }
}

/// 深度维账本的**持久化形态**: 只带上限。
///
/// 计数不落盘 —— 调用树在进程重启后本就不存在, 计数从 0 起才是事实;
/// 也因此**改会话存储伪造不出计数** (伪造的计数字段被丢弃)。
#[derive(Serialize, Deserialize)]
struct PersistedDepth {
    /// 深度上限 (`None` = 不设限)。
    #[serde(default)]
    max_depth: Option<u64>,
    /// 旧形态的当前层 (被丢弃: 计数不可伪造)。
    #[serde(default)]
    current_depth: Option<u32>,
    /// 旧形态的峰值 (被丢弃: 计数不可伪造)。
    #[serde(default)]
    peak_depth: Option<u32>,
}

impl Serialize for DepthQuota {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PersistedDepth {
            max_depth: self.max_depth,
            current_depth: None,
            peak_depth: None,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DepthQuota {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let persisted = PersistedDepth::deserialize(deserializer)?;
        // 计数一律丢弃 (从 0 起): 会话存储里的深度计数**不是事实**。
        let _ = (persisted.current_depth, persisted.peak_depth);
        Ok(Self {
            max_depth: persisted.max_depth,
            current_depth: 0,
            peak_depth: 0,
        })
    }
}

/// 深度维的自述读数 (接线点)。
///
/// 自述面的深度维格子要的是**真值**: 上限 + 已耗 (峰值深度) —— 不再是
/// null / "暂无接口"。本结构就是那次读数的形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepthQuotaReading {
    /// 深度上限 (`None` = 不设限)。
    pub limit: Option<u64>,
    /// 当前占用的层 (嵌套作用域内; 出作用域回落)。
    pub current_depth: u32,
    /// 峰值深度 (单调不降) —— 自述面的"已耗"取它 (与上限的差即余量)。
    pub peak_depth: u32,
}

impl DepthQuotaReading {
    /// 已耗口径 = 峰值深度 (自述面的 `consumed` 直接用这个数)。
    pub fn consumed(&self) -> u64 {
        u64::from(self.peak_depth)
    }
}

/// 深度维的会话级账本**句柄** (共享)。
///
/// 为什么是句柄而不是裸值: 嵌套侧车调用在**另一个执行上下文**里跑 (子代理
/// 自己的回合), 父子两处必须记**同一本账** —— 否则父回合的检查点看不到子
/// 调用吃掉的深度 (上限可被绕过)。共享的是计数, 不是第二本账。
#[derive(Debug, Clone, Default)]
pub struct DepthQuotaHandle {
    inner: Arc<Mutex<DepthQuota>>,
}

impl DepthQuotaHandle {
    /// 包一份账本。
    pub fn new(quota: DepthQuota) -> Self {
        Self {
            inner: Arc::new(Mutex::new(quota)),
        }
    }

    /// 带上限的账本句柄 (`None` = 不设限)。
    pub fn with_max_depth(max_depth: Option<u64>) -> Self {
        Self::new(DepthQuota::with_max_depth(max_depth))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, DepthQuota> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// 快照 (读一次账本)。
    pub fn read(&self) -> DepthQuota {
        *self.lock()
    }

    /// 绝对深度观测 (生产接线唯一的喂入口; 见模块头契约)。
    pub fn observe_depth(&self, depth: u32) -> u32 {
        self.lock().observe_depth(depth)
    }

    /// 进入一个嵌套作用域: 观测**绝对**深度 + 占住当前层。
    ///
    /// 到顶 ⇒ `Err` = 既有配额拒绝语义的事实 (调用方据此拒这次嵌套);
    /// 放行 ⇒ 返回守卫, 守卫 drop 即回落当前层 (RAII ⇒ 早退也不抬高)。
    pub fn scope(&self, level: u32) -> Result<DepthScope, QuotaExceeded> {
        let mut quota = self.lock();
        if let Some(exceeded) = quota.check() {
            return Err(exceeded);
        }
        let previous = quota.current_depth;
        quota.observe_depth(level);
        Ok(DepthScope {
            handle: self.clone(),
            previous,
        })
    }

    /// 耗尽事实 (`None` = 未到顶)。
    pub fn check(&self) -> Option<QuotaExceeded> {
        self.lock().check()
    }

    /// 自述读数 (接线点)。
    pub fn reading(&self) -> DepthQuotaReading {
        self.lock().reading()
    }

    /// 已耗 (峰值深度)。
    pub fn consumed(&self) -> u64 {
        self.lock().consumed()
    }

    /// 深度上限 (`None` = 不设限)。
    pub fn max_depth(&self) -> Option<u64> {
        self.lock().max_depth
    }

    /// 当前占用的层。
    pub fn current_depth(&self) -> u32 {
        self.lock().current_depth
    }

    /// 峰值深度。
    pub fn peak_depth(&self) -> u32 {
        self.lock().peak_depth
    }

    /// 绑上限 (上限随计量器: 账本自己带上限)。
    pub fn set_max_depth(&self, max_depth: Option<u64>) {
        self.lock().max_depth = max_depth;
    }

    /// 回落当前层 (作用域守卫专用)。
    fn restore_current(&self, previous: u32) {
        self.lock().current_depth = previous;
    }
}

/// 句柄落盘 = 账本落盘 (只带上限; 计数从 0 起 ⇒ 伪造不出计数)。
impl Serialize for DepthQuotaHandle {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DepthQuotaHandle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(DepthQuota::deserialize(deserializer)?))
    }
}

/// 嵌套作用域守卫 (RAII): drop 即回落当前层。
///
/// 把"必须配对"从自觉变成结构保证 —— 早退 / `?` / panic 都不抬高当前层。
#[derive(Debug)]
pub struct DepthScope {
    handle: DepthQuotaHandle,
    previous: u32,
}

impl Drop for DepthScope {
    fn drop(&mut self) {
        self.handle.restore_current(self.previous);
    }
}
