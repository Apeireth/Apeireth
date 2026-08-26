//! Graph 共享 state + FinalState + NodeOutput.
//!
//! **State**: 异构 `serde_json::Value` 字典 (BTreeMap 保证deterministic order)
//! **NodeOutput**: 节点执行的最小输出 (id + touched keys + message)
//! **FinalState**: 一次完整执行的快照 (state + 节点输出 + 执行顺序)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Graph 共享 state = `String -> serde_json::Value`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    inner: BTreeMap<String, Value>,
}

impl State {
    /// 空 state.
    pub fn new() -> Self {
        Self::default()
    }

    /// 用 init 单 key 构造.
    pub fn with(key: impl Into<String>, value: impl Into<Value>) -> Self {
        let mut s = Self::new();
        s.insert(key, value.into());
        s
    }

    /// 写入 / 覆盖一个 key.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.inner.insert(key.into(), value)
    }

    /// 读一个 key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.inner.get(key)
    }

    /// 删除一个 key, 返回旧值.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.inner.remove(key)
    }

    /// 当前 key 数量.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 全部 key (BTreeMap 顺序).
    pub fn keys(&self) -> Vec<&str> {
        self.inner.keys().map(|s| s.as_str()).collect()
    }
}

/// 节点执行输出 (id + 写过的 keys + 可选 message).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeOutput {
    /// 节点 id
    pub node_id: String,
    /// 节点被修改过的 state key (按追加顺序)
    pub touched_keys: Vec<String>,
    /// 节点产出的 message (可选)
    pub message: Option<String>,
}

impl NodeOutput {
    /// 构造一个空 output (仅含 node_id).
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            node_id: id.into(),
            touched_keys: Vec::new(),
            message: None,
        }
    }

    /// 链式: 记录一次 state key 写入.
    pub fn touch(mut self, key: impl Into<String>) -> Self {
        self.touched_keys.push(key.into());
        self
    }

    /// 链式: 写入一条 message.
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
}

/// 一次 graph 完整执行的快照.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalState {
    /// 终态 state (执行完所有节点)
    pub state: State,
    /// 节点输出 (id -> output)
    pub outputs: BTreeMap<String, NodeOutput>,
    /// 实际执行顺序 (拓扑序)
    pub execution_order: Vec<String>,
}

impl FinalState {
    /// 读 state 的 key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.state.get(key)
    }

    /// 已执行节点数.
    pub fn executed_count(&self) -> usize {
        self.outputs.len()
    }

    /// 是否包含某节点 output.
    pub fn contains(&self, node_id: &str) -> bool {
        self.outputs.contains_key(node_id)
    }
}
