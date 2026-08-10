//! R33-5: Conditional Edge — LangGraph `add_conditional_edges` 字段级借鉴
//!
//! **目标**: 给 `apeireth-graph` 加 LangGraph 风格的 conditional edge — 跑完 source
//! 节点后, 调 `condition(&state) -> label` 决定下一节点. 跟 R32-2 `apeireth-pipeline::tool_loop`
//! 的 `should_continue` 1:1 (per LangGraph `add_conditional_edges("tool_call", should_continue)`).
//!
//! **LangGraph 真代码借鉴** (`langgraph/graph/state.py:StateGraph.add_conditional_edges`):
//! - `path_map: dict[label, NodeId]` — condition 返 label, 查 path_map 得 target
//! - `then: Optional[NodeId]` — condition 不在 path_map 时 fallback
//! - 终止条件: condition 返 `END` 字符串 / cycle 检测
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `Graph` / `Node` / `Edge` / `Executor` 现有方法 (新增字段 + 新增 method)
//! - 0 改 `State` / `FinalState` / `NodeOutput` / `Checkpoint` (复用 1:1)
//! - 0 引入 async-runtime 重依赖 (条件闭包 sync, 跟 Node::run 一致)
//! - 0 引入 `unsafe` (workspace `#![deny(unsafe_code)]` 继承)
//!
//! **设计**:
//! ```text
//! Graph:
//!   nodes: BTreeMap<NodeId, Box<dyn Node>>     // 不变
//!   edges: Vec<Edge>                             // 不变 (DAG-only 边)
//!   conditional_edges: Vec<ConditionalEdge>      // 新增: 条件边列表
//!
//! ConditionalEdge:
//!   from: NodeId                                  // source node
//!   path_map: BTreeMap<String, NodeId>            // label -> target
//!   default: Option<NodeId>                       // condition 不在 path_map 时 fallback
//!   condition: Arc<dyn Fn(&State) -> String>      // 状态查 label
//!
//! Executor::execute():
//!   1. 跑所有 DAG-only 节点 (topological order, 走 edges 路径)
//!   2. 每跑完 conditional source 节点, 调 condition(state) 拿 label
//!   3. 查 path_map[label] -> target, 跑 target
//!   4. 没命中 -> 走 default 或终止 (LangGraph END 借鉴)
//!   5. cycle 检测: visited set, 已 visited 节点不再跑
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{GraphError, NodeId, Result, State};

/// LangGraph `END` sentinel: condition 返这个字符串时, 走 default 路径 (per LangGraph 1:1).
/// 我们用 `"__end__"` 避免跟用户 label 冲突.
pub const END_LABEL: &str = "__end__";

/// Conditional Edge — `from` 节点跑完后, 调 `condition` 决定下一节点.
///
/// **构造**: 用 `Graph::add_conditional_edge(from, path_map, default, condition)` 1:1
/// 跟 LangGraph `add_conditional_edges` 对齐.
#[derive(Clone)]
pub struct ConditionalEdge {
    /// source 节点 id (条件边起点)
    pub from: NodeId,
    /// condition 返 label -> target 节点 id 映射 (per LangGraph `path_map`)
    pub path_map: BTreeMap<String, NodeId>,
    /// condition 返 label 不在 path_map 时 fallback (per LangGraph `then`)
    pub default: Option<NodeId>,
    /// 状态查 label 闭包 (sync, 跟 Node::run 一致; 闭包可捕获外部状态)
    pub condition: Arc<dyn Fn(&State) -> String + Send + Sync>,
}

impl std::fmt::Debug for ConditionalEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalEdge")
            .field("from", &self.from)
            .field("path_map", &self.path_map)
            .field("default", &self.default)
            .field("condition", &"<closure>")
            .finish()
    }
}

/// Conditional edge 执行结果 (per conditional edge 调用一次).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalDecision {
    /// source 节点 id
    pub from: NodeId,
    /// condition 返的 label
    pub label: String,
    /// 走的 target 节点 id (None = 终止 / per LangGraph END)
    pub target: Option<NodeId>,
    /// 走的路径: "path_map" / "default" / "end" / "missing"
    pub path_kind: String,
}

impl ConditionalEdge {
    /// 跑条件闭包 + 查 path_map 拿 target.
    ///
    /// **返回**: `ConditionalDecision { from, label, target, path_kind }`.
    /// `target = None` 表示终止 (condition 返 `"__end__"` 或 label 不在 path_map 且无 default).
    pub fn decide(&self, state: &State) -> ConditionalDecision {
        let label = (self.condition)(state);
        if label == END_LABEL {
            return ConditionalDecision {
                from: self.from.clone(),
                label,
                target: None,
                path_kind: "end".to_string(),
            };
        }
        if let Some(target) = self.path_map.get(&label) {
            return ConditionalDecision {
                from: self.from.clone(),
                label,
                target: Some(target.clone()),
                path_kind: "path_map".to_string(),
            };
        }
        ConditionalDecision {
            from: self.from.clone(),
            label,
            target: self.default.clone(),
            path_kind: if self.default.is_some() {
                "default".to_string()
            } else {
                "missing".to_string()
            },
        }
    }
}

/// Conditional edge 评估错误 (cycle / missing target).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionalError {
    /// Conditional target 节点不在 graph 中
    MissingTarget {
        from: NodeId,
        target: NodeId,
    },
    /// Conditional cycle 检测 (target 已在 execution set)
    Cycle {
        from: NodeId,
        target: NodeId,
    },
}

impl std::fmt::Display for ConditionalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTarget { from, target } => {
                write!(f, "conditional edge from `{from}` targets missing node `{target}`")
            }
            Self::Cycle { from, target } => {
                write!(f, "conditional edge from `{from}` to `{target}` would create a cycle")
            }
        }
    }
}

impl std::error::Error for ConditionalError {}

impl From<ConditionalError> for GraphError {
    fn from(err: ConditionalError) -> Self {
        match err {
            ConditionalError::MissingTarget { from: _, target } => GraphError::MissingNode(target),
            ConditionalError::Cycle { from, target } => GraphError::Cycle {
                nodes: vec![from, target],
            },
        }
    }
}

// ============================================================
// Unit tests (no net) — 借鉴 LangGraph conditional edge 1:1 行为
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_state() -> State {
        State::new()
    }

    fn state_with(key: &str, value: &str) -> State {
        State::with(key, value)
    }

    #[test]
    fn end_label_terminates() {
        // 借鉴 LangGraph: condition 返 "END" 走 default (None) → 终止
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map: BTreeMap::new(),
            default: None,
            condition: Arc::new(|_| END_LABEL.to_string()),
        };
        let d = edge.decide(&empty_state());
        assert_eq!(d.label, END_LABEL);
        assert_eq!(d.target, None);
        assert_eq!(d.path_kind, "end");
    }

    #[test]
    fn path_map_routes_to_mapped_target() {
        // 借鉴 LangGraph: condition 返 "yes" 走 path_map["yes"] → target
        let mut path_map = BTreeMap::new();
        path_map.insert("yes".to_string(), "approve_node".to_string());
        path_map.insert("no".to_string(), "reject_node".to_string());
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map,
            default: None,
            condition: Arc::new(|state| {
                if state.get("ok").and_then(|v| v.as_str()) == Some("yes") {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }
            }),
        };
        let s_yes = state_with("ok", "yes");
        let d_yes = edge.decide(&s_yes);
        assert_eq!(d_yes.target, Some("approve_node".to_string()));
        assert_eq!(d_yes.path_kind, "path_map");

        let s_no = state_with("ok", "no");
        let d_no = edge.decide(&s_no);
        assert_eq!(d_no.target, Some("reject_node".to_string()));
        assert_eq!(d_no.path_kind, "path_map");
    }

    #[test]
    fn missing_label_falls_back_to_default() {
        // 借鉴 LangGraph: condition 返 label 不在 path_map → 走 default
        let mut path_map = BTreeMap::new();
        path_map.insert("known".to_string(), "target".to_string());
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map,
            default: Some("fallback".to_string()),
            condition: Arc::new(|_| "unknown_label".to_string()),
        };
        let d = edge.decide(&empty_state());
        assert_eq!(d.target, Some("fallback".to_string()));
        assert_eq!(d.path_kind, "default");
    }

    #[test]
    fn missing_label_no_default_terminates() {
        // 借鉴 LangGraph: label 不在 path_map 且 default=None → 终止 (target=None)
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map: BTreeMap::new(),
            default: None,
            condition: Arc::new(|_| "anything".to_string()),
        };
        let d = edge.decide(&empty_state());
        assert_eq!(d.target, None);
        assert_eq!(d.path_kind, "missing");
    }

    #[test]
    fn decision_serializes_to_json() {
        let mut path_map = BTreeMap::new();
        path_map.insert("approve".to_string(), "next".to_string());
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map,
            default: None,
            condition: Arc::new(|_| "approve".to_string()),
        };
        let d = edge.decide(&empty_state());
        let json = serde_json::to_string(&d).unwrap();
        let back: ConditionalDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn conditional_error_displays_readably() {
        let err = ConditionalError::MissingTarget {
            from: "a".to_string(),
            target: "missing".to_string(),
        };
        assert!(err.to_string().contains("a"));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn conditional_error_converts_to_graph_error() {
        let err = ConditionalError::Cycle {
            from: "a".to_string(),
            target: "b".to_string(),
        };
        let g: GraphError = err.into();
        assert!(matches!(g, GraphError::Cycle { .. }));
    }

    #[test]
    fn end_label_excludes_user_path_map() {
        // 用户 path_map 不会跟 END_LABEL 冲突 (除非显式用 "__end__")
        let mut path_map = BTreeMap::new();
        path_map.insert("__end__".to_string(), "should_not_reach".to_string());
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map,
            default: None,
            condition: Arc::new(|_| END_LABEL.to_string()),
        };
        let d = edge.decide(&empty_state());
        // END_LABEL 优先, 不查 path_map
        assert_eq!(d.target, None);
        assert_eq!(d.path_kind, "end");
    }

    #[test]
    fn condition_can_capture_external_state() {
        // 借鉴 LangGraph: condition 闭包可捕获外部 state
        let counter = Arc::new(std::sync::Mutex::new(0u32));
        let counter_for_cond = Arc::clone(&counter);
        let edge = ConditionalEdge {
            from: "src".to_string(),
            path_map: BTreeMap::new(),
            default: Some("end".to_string()),
            condition: Arc::new(move |_| {
                let mut c = counter_for_cond.lock().unwrap();
                *c += 1;
                if *c >= 3 {
                    END_LABEL.to_string()
                } else {
                    "loop".to_string()
                }
            }),
        };
        // 模拟多轮调用
        let s = state_with("x", "y");
        assert_eq!(edge.decide(&s).target, Some("end".to_string()));
        // 1st call: counter=1, label="loop" → path_map 空 → default=Some("end")
        // (default 是 target, 不是 END; 这里 test 验证闭包捕获 ok)
        let _ = edge.decide(&s);  // counter=2
        let d3 = edge.decide(&s);  // counter=3, label=END → target=None
        assert_eq!(d3.target, None);
        assert_eq!(*counter.lock().unwrap(), 3);
    }
}
