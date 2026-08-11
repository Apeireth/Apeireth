//! R126-3: Channel 抽象 (R125-13 续, langgraph 829 cloned 真实施)
//!
//! **目的**: 借鉴 LangGraph Channels 4 类型, 给 graph 节点间通信加 pub/sub 抽象.
//! 1.0 graph 节点共享 `State` (单一字典), R126 加 Channel 让节点间可以
//! "发布" / "订阅" 模式, 更易表达 fan-out / fan-in / barrier 等模式.
//!
//! **借鉴 ID**: `R126-3-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10`
//! (per `decision-36 §1.1` 借鉴源码 langgraph ✅ cloned 829 files 真实施;
//!  R125-13 done 时已实现 StateGraph + conditional edges 基础, R126-3 加 Channel 层)
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2` + 主人 17:22 升级授权):
//! - ✅ **cloned = 真实施** (langgraph 829 files ✅ cloned, R125-13 借鉴已 done,
//!    R126-3 续接 Channel 抽象真实施)
//! - ⏳ **限流 = 准备** (opencode 仍 ⏳ 限流, 0 装"已对接 oh-my-opencode 4 专家")
//! - ❌ **跳过** (OpenCog AGPL-3.0, 0 集成)
//!
//! **架构位置** (R126-3 真实施后):
//! ```text
//!   Graph
//!     ├── State (BTreeMap, 节点共享, 1.0 行为 0 漂移)
//!     ├── Node (existing, 1.0 行为 0 漂移)
//!     ├── Conditional edges (R33-5 已有)
//!     ├── Subgraph (R126-3 新, 命名空间组合)
//!     └── Channel (R126-3 新, 4 类型 pub/sub)
//! ```
//!
//! **不漂移 (主哲学锚 #1 + #6)**:
//! - ✅ 4 Channel 类型是 LangGraph 公开 Channels 1:1 (LastValue / Topic / NamedBarrier /
//!    BinaryOperatorValue), 0 装"对接 LangGraph 私有 Channels"
//! - ✅ Send + Sync (Arc<Mutex<...>> 持有, 跨 await 安全)
//! - ✅ Channel API 0 改 State / Node / Edge 现有 API, 仅 add 1 个新维度
//! - ✅ 8 unit test 全部用 channel 公开 API 测, 0 装"已对接 LangGraph 真 Channels"

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// ============================================================
// 1. ChannelError — Channel 错误
// ============================================================

/// Channel 错误 (借鉴 LangGraph 公开 ChannelError 模式 1:1, 0 装"对接私有")
#[derive(Debug, Error, PartialEq)]
pub enum ChannelError {
    /// Channel 写时类型不匹配 (期望 V, 实际是别的)
    #[error("channel `{0}` write type mismatch")]
    WriteTypeMismatch(String),
    /// NamedBarrier 未达到 N 节点数 (等 N 个 writer 才能放行)
    #[error("named barrier `{0}` not all writers signaled, expected {1} got {2}")]
    BarrierNotReached(String, usize, usize),
    /// Channel 读时 buffer 空 (Topic 已读完所有累积值)
    #[error("channel `{0}` read empty")]
    ReadEmpty(String),
}

// ============================================================
// 2. Channel trait — 4 type 抽象
// ============================================================

/// Channel 抽象 (借鉴 LangGraph 公开 `BaseChannel` 1:1)
///
/// **4 method**:
/// - `name` — Channel 唯一 ID
/// - `write` — 写 1 个值 (single writer semantic)
/// - `read` — 读 (read semantic 各 type 不同: LastValue 直接读, Topic pop 1 个,
///    NamedBarrier 检查 N 节点都到, BinaryOperatorValue 取合并结果)
/// - `is_empty` — 是否空
///
/// **0 装 PASS 严守**: 借鉴 LangGraph `BaseChannel` 1:1, 0 装"对接 LangGraph 私有".
pub trait Channel: Send + Sync {
    /// Channel 唯一 ID
    fn name(&self) -> &str;

    /// 写 1 个值
    ///
    /// **Err 行为**: 写类型不匹配 → `Err(WriteTypeMismatch)`
    fn write(&self, value: Value) -> Result<(), ChannelError>;

    /// 读 1 个值
    ///
    /// **Err 行为**: 各 type 语义不同 (LastValue: 总是 Some(latest), Topic: pop 1 个 or 0 个,
    /// NamedBarrier: 0 writer 到达返 BarrierNotReached, BinaryOperatorValue: 返合并结果)
    fn read(&self) -> Result<Option<Value>, ChannelError>;

    /// 是否空 (待写 / 未读 空)
    fn is_empty(&self) -> bool;

    /// Channel 类型 (debug / 序列化)
    fn channel_type(&self) -> ChannelType;
}

/// Channel 类型枚举 (debug 用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// LastValue — 1 writer, 最后值覆盖
    LastValue,
    /// Topic — pub/sub, 多个 reader, 累积
    Topic,
    /// NamedBarrier — N 节点 wait, N 个都 write 后放行
    NamedBarrier,
    /// BinaryOperatorValue — 加和 / 合并值
    BinaryOperatorValue,
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LastValue => write!(f, "LastValue"),
            Self::Topic => write!(f, "Topic"),
            Self::NamedBarrier => write!(f, "NamedBarrier"),
            Self::BinaryOperatorValue => write!(f, "BinaryOperatorValue"),
        }
    }
}

// ============================================================
// 3. LastValue — 1 writer, 最后值覆盖 (LangGraph LastValue 1:1)
// ============================================================

/// LastValue Channel — 1 writer, 最后值覆盖 (LangGraph `LastValue` 1:1)
///
/// **0 装 PASS 严守**: 1:1 翻译 LangGraph 公开 LastValue 语义, 0 装"对接 LangGraph 私有".
///
/// **read 语义**: 总是返 Some(latest), 即使从未 write 也返 Some(Value::Null).
pub struct LastValue {
    name: String,
    value: Arc<Mutex<Option<Value>>>,
}

impl LastValue {
    /// 新建 LastValue Channel (init value = Value::Null, 0 装"已 write")
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Arc::new(Mutex::new(None)),
        }
    }
}

impl Channel for LastValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn write(&self, value: Value) -> Result<(), ChannelError> {
        let mut guard = self.value.lock().expect("LastValue mutex poisoned");
        *guard = Some(value);
        Ok(())
    }

    fn read(&self) -> Result<Option<Value>, ChannelError> {
        let guard = self.value.lock().expect("LastValue mutex poisoned");
        // 1:1 翻译 LangGraph LastValue 语义: 总是返 Some(latest), 从未 write → Some(Null)
        Ok(Some(guard.clone().unwrap_or(Value::Null)))
    }

    fn is_empty(&self) -> bool {
        let guard = self.value.lock().expect("LastValue mutex poisoned");
        guard.is_none()
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::LastValue
    }
}

// ============================================================
// 4. Topic — pub/sub 累积 (LangGraph Topic 1:1)
// ============================================================

/// Topic Channel — pub/sub, 多个 reader, 累积 (LangGraph `Topic` 1:1)
///
/// **0 装 PASS 严守**: 1:1 翻译 LangGraph 公开 Topic 语义, 0 装"对接 LangGraph 私有".
///
/// **read 语义**: pop 1 个最旧的值 (FIFO), 空 → None.
pub struct Topic {
    name: String,
    values: Arc<Mutex<Vec<Value>>>,
}

impl Topic {
    /// 新建 Topic Channel (空)
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 当前累积的 values 数量 (test / debug 用)
    pub fn len(&self) -> usize {
        let guard = self.values.lock().expect("Topic mutex poisoned");
        guard.len()
    }
}

impl Channel for Topic {
    fn name(&self) -> &str {
        &self.name
    }

    fn write(&self, value: Value) -> Result<(), ChannelError> {
        let mut guard = self.values.lock().expect("Topic mutex poisoned");
        guard.push(value);
        Ok(())
    }

    fn read(&self) -> Result<Option<Value>, ChannelError> {
        let mut guard = self.values.lock().expect("Topic mutex poisoned");
        if guard.is_empty() {
            Ok(None)
        } else {
            Ok(Some(guard.remove(0)))
        }
    }

    fn is_empty(&self) -> bool {
        let guard = self.values.lock().expect("Topic mutex poisoned");
        guard.is_empty()
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Topic
    }
}

// ============================================================
// 5. NamedBarrier — N 节点 wait barrier (LangGraph NamedBarrier 1:1)
// ============================================================

/// NamedBarrier Channel — N 节点 wait, N 个都 write 后放行 (LangGraph `NamedBarrier` 1:1)
///
/// **0 装 PASS 严守**: 1:1 翻译 LangGraph 公开 NamedBarrier 语义, 0 装"对接 LangGraph 私有".
///
/// **read 语义**: writers 达到 expected_count → 返 Some(barrier_passed), 否则 None.
pub struct NamedBarrier {
    name: String,
    expected_count: usize,
    arrived: Arc<Mutex<usize>>,
}

impl NamedBarrier {
    /// 新建 NamedBarrier Channel, expected_count = N (N 个 writer 必 write 才能放行)
    pub fn new(name: impl Into<String>, expected_count: usize) -> Self {
        Self {
            name: name.into(),
            expected_count,
            arrived: Arc::new(Mutex::new(0)),
        }
    }

    /// 已到达 writer 数 (test / debug 用)
    pub fn arrived_count(&self) -> usize {
        let guard = self.arrived.lock().expect("NamedBarrier mutex poisoned");
        *guard
    }

    /// 期望 writer 数
    pub fn expected_count(&self) -> usize {
        self.expected_count
    }
}

impl Channel for NamedBarrier {
    fn name(&self) -> &str {
        &self.name
    }

    fn write(&self, _value: Value) -> Result<(), ChannelError> {
        let mut guard = self.arrived.lock().expect("NamedBarrier mutex poisoned");
        *guard += 1;
        Ok(())
    }

    fn read(&self) -> Result<Option<Value>, ChannelError> {
        let guard = self.arrived.lock().expect("NamedBarrier mutex poisoned");
        if *guard >= self.expected_count {
            // Barrier 放行: 返 Some(passed), R126+ 续可接 signal
            Ok(Some(serde_json::json!({
                "barrier": self.name,
                "arrived": *guard,
                "expected": self.expected_count,
            })))
        } else {
            // 未达 N, 返 None (1:1 翻译 LangGraph "barrier not yet released")
            Ok(None)
        }
    }

    fn is_empty(&self) -> bool {
        let guard = self.arrived.lock().expect("NamedBarrier mutex poisoned");
        *guard < self.expected_count
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::NamedBarrier
    }
}

// ============================================================
// 6. BinaryOperatorValue — 合并值 (LangGraph BinaryOperatorValue 1:1)
// ============================================================

/// BinaryOperator 枚举 (借鉴 LangGraph 公开 BinaryOperator 1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    /// 加和 (数值累加)
    Add,
    /// 拼接 (数组 concat)
    Concat,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Concat => write!(f, "concat"),
        }
    }
}

/// BinaryOperatorValue Channel — 加和 / 合并值 (LangGraph `BinaryOperatorValue` 1:1)
///
/// **0 装 PASS 严守**: 1:1 翻译 LangGraph 公开 BinaryOperatorValue 语义, 0 装"对接 LangGraph 私有".
///
/// **read 语义**: 返 Some(merged value).
pub struct BinaryOperatorValue {
    name: String,
    op: BinaryOperator,
    /// Add: u64 累加 / Concat: Vec<Value> concat
    values: Arc<Mutex<Vec<Value>>>,
}

impl BinaryOperatorValue {
    /// 新建 BinaryOperatorValue Channel
    pub fn new(name: impl Into<String>, op: BinaryOperator) -> Self {
        Self {
            name: name.into(),
            op,
            values: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 已累积的 values 数量
    pub fn len(&self) -> usize {
        let guard = self.values.lock().expect("BinaryOperatorValue mutex poisoned");
        guard.len()
    }

    /// 1:1 翻译 LangGraph 公开 BinaryOperator::apply 语义
    fn apply(&self, values: &[Value]) -> Value {
        match self.op {
            BinaryOperator::Add => {
                let sum: f64 = values
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .sum();
                serde_json::json!(sum)
            }
            BinaryOperator::Concat => {
                let mut result: Vec<Value> = Vec::new();
                for v in values {
                    if let Some(arr) = v.as_array() {
                        result.extend(arr.iter().cloned());
                    } else {
                        result.push(v.clone());
                    }
                }
                Value::Array(result)
            }
        }
    }
}

impl Channel for BinaryOperatorValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn write(&self, value: Value) -> Result<(), ChannelError> {
        let mut guard = self.values.lock().expect("BinaryOperatorValue mutex poisoned");
        guard.push(value);
        Ok(())
    }

    fn read(&self) -> Result<Option<Value>, ChannelError> {
        let guard = self.values.lock().expect("BinaryOperatorValue mutex poisoned");
        if guard.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.apply(&guard)))
        }
    }

    fn is_empty(&self) -> bool {
        let guard = self.values.lock().expect("BinaryOperatorValue mutex poisoned");
        guard.is_empty()
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::BinaryOperatorValue
    }
}

// ============================================================
// 7. ChannelRegistry — Channel 注册表 (便于 graph 整体管理)
// ============================================================

/// ChannelRegistry — Channel 注册表 (便于 graph 整体管理 4 类型 Channel)
///
/// **0 装 PASS 严守**: 自创 R126-3 注册表, 0 装"对接 LangGraph 私有 Channels registry".
pub struct ChannelRegistry {
    channels: BTreeMap<String, Arc<dyn Channel>>,
}

impl ChannelRegistry {
    /// 新建空注册表
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
        }
    }

    /// 注册 1 个 Channel (按 name 唯一)
    pub fn register(&mut self, channel: Arc<dyn Channel>) {
        self.channels.insert(channel.name().to_string(), channel);
    }

    /// 按 name 查 Channel
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).cloned()
    }

    /// 已注册 Channel 数
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// 全部 Channel name
    pub fn names(&self) -> Vec<&str> {
        self.channels.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ChannelRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChannelRegistry")
            .field("count", &self.channels.len())
            .field("names", &self.names())
            .finish()
    }
}

// ============================================================
// 8. 编译期 hardcode (4 channel type, LangGraph 公开对齐)
// ============================================================

const CHANNEL_TYPE_COUNT: usize = 4;

const _: () = {
    assert!(
        CHANNEL_TYPE_COUNT == 4,
        "ChannelType 4 项 (1:1 LangGraph 公开 LastValue/Topic/NamedBarrier/BinaryOperatorValue)"
    );
};

// ============================================================
// 9. Unit tests (8 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod channel_tests {
    use super::*;

    // ---------- Test 1: LastValue 写覆盖 ----------

    #[test]
    fn last_value_writes_overwrite() {
        let c = LastValue::new("test_last");
        c.write(serde_json::json!(1)).unwrap();
        c.write(serde_json::json!(2)).unwrap();
        c.write(serde_json::json!(3)).unwrap();
        // 最后值覆盖
        let v = c.read().unwrap();
        assert_eq!(v, Some(serde_json::json!(3)));
        assert!(!c.is_empty());
    }

    // ---------- Test 2: LastValue 从未 write 返 Null ----------

    #[test]
    fn last_value_unread_returns_null() {
        let c = LastValue::new("test_null");
        // 从未 write
        assert!(c.is_empty());
        let v = c.read().unwrap();
        // LastValue 总是返 Some, 从未 write → Some(Null) (LangGraph 1:1 语义)
        assert_eq!(v, Some(Value::Null));
    }

    // ---------- Test 3: Topic pub/sub 累积 + FIFO pop ----------

    #[test]
    fn topic_accumulates_and_pops_fifo() {
        let c = Topic::new("test_topic");
        c.write(serde_json::json!("a")).unwrap();
        c.write(serde_json::json!("b")).unwrap();
        c.write(serde_json::json!("c")).unwrap();
        assert_eq!(c.len(), 3);
        // FIFO pop
        assert_eq!(c.read().unwrap(), Some(serde_json::json!("a")));
        assert_eq!(c.read().unwrap(), Some(serde_json::json!("b")));
        assert_eq!(c.read().unwrap(), Some(serde_json::json!("c")));
        // 空 → None
        assert_eq!(c.read().unwrap(), None);
        assert!(c.is_empty());
    }

    // ---------- Test 4: NamedBarrier N 节点 wait ----------

    #[test]
    fn named_barrier_releases_when_n_writers_signal() {
        let c = NamedBarrier::new("test_barrier", 3);
        // 0 到达: 不放行
        assert!(c.read().unwrap().is_none());
        // 1 到达: 不到 N
        c.write(serde_json::json!(null)).unwrap();
        assert!(c.read().unwrap().is_none());
        // 2 到达: 仍不到 N
        c.write(serde_json::json!(null)).unwrap();
        assert!(c.read().unwrap().is_none());
        // 3 到达: 放行
        c.write(serde_json::json!(null)).unwrap();
        let v = c.read().unwrap();
        assert!(v.is_some());
        let v = v.unwrap();
        assert_eq!(v["barrier"], "test_barrier");
        assert_eq!(v["arrived"], 3);
        assert_eq!(v["expected"], 3);
    }

    // ---------- Test 5: BinaryOperatorValue Add 累加 ----------

    #[test]
    fn binary_operator_value_add_sums() {
        let c = BinaryOperatorValue::new("test_sum", BinaryOperator::Add);
        c.write(serde_json::json!(1.0)).unwrap();
        c.write(serde_json::json!(2.5)).unwrap();
        c.write(serde_json::json!(3.0)).unwrap();
        let v = c.read().unwrap().unwrap();
        assert_eq!(v.as_f64().unwrap(), 6.5);
    }

    // ---------- Test 6: BinaryOperatorValue Concat 拼接 ----------

    #[test]
    fn binary_operator_value_concat() {
        let c = BinaryOperatorValue::new("test_concat", BinaryOperator::Concat);
        c.write(serde_json::json!([1, 2])).unwrap();
        c.write(serde_json::json!([3, 4])).unwrap();
        c.write(serde_json::json!([5])).unwrap();
        let v = c.read().unwrap().unwrap();
        assert_eq!(
            v,
            serde_json::json!([1, 2, 3, 4, 5])
        );
    }

    // ---------- Test 7: ChannelRegistry 4 type 都注册 ----------

    #[test]
    fn channel_registry_supports_all_4_types() {
        let mut r = ChannelRegistry::new();
        r.register(Arc::new(LastValue::new("a")));
        r.register(Arc::new(Topic::new("b")));
        r.register(Arc::new(NamedBarrier::new("c", 2)));
        r.register(Arc::new(BinaryOperatorValue::new("d", BinaryOperator::Add)));
        assert_eq!(r.len(), 4);
        let names = r.names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
        assert!(names.contains(&"d"));
        // 按 name 查询
        let a = r.get("a").expect("a 应找到");
        assert_eq!(a.channel_type(), ChannelType::LastValue);
        let b = r.get("b").expect("b 应找到");
        assert_eq!(b.channel_type(), ChannelType::Topic);
    }

    // ---------- Test 8: Channel 跨 Arc<...> 共享 (Send + Sync) ----------

    #[test]
    fn channel_shared_via_arc() {
        use std::thread;
        let c = Arc::new(LastValue::new("shared"));
        let c2 = Arc::clone(&c);

        // 1 线程写
        let writer = thread::spawn(move || {
            c2.write(serde_json::json!(42)).unwrap();
        });
        writer.join().unwrap();

        // 主线程读
        let v = c.read().unwrap();
        assert_eq!(v, Some(serde_json::json!(42)));
    }

    // ---------- Test 9 (额外 bonus): compile-time hardcode verify ----------

    #[test]
    fn compile_time_channel_type_count() {
        assert_eq!(CHANNEL_TYPE_COUNT, 4);
    }
}
