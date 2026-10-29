//! 响应式运行期错误类型。

use std::fmt;

use crate::graph::NodeId;

/// 响应式运行期的错误类型。
///
/// 错误设计原则（设计文档 §4.4）：报错必须**自带人名** —— 节点可通过
/// [`crate::Runtime::set_label`] 挂标签，报错路径里带上标签才可定位。
///
/// ## 失败显式化（不静默降级）
///
/// 每一种失败都有对应的 `Err` 出口；「无条件」变体（[`crate::Signal::set`] 等）
/// **不 panic**，而是把错误记录到运行期错误槽（[`crate::Runtime::take_flush_error`]
/// 可取走）⇒ wasm 侧 panic = 整页 abort，引擎不得把用户的坏数据升级成页面崩溃。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReactiveError {
    /// 求值环：派生 A 读派生 B，派生 B 又读回 A。
    CycleDetected {
        /// 环上的节点路径（首尾为同一节点）。
        path: Vec<NodeId>,
    },
    /// 单次 flush 内某节点的执行次数超过**兜底上限**（自激循环的最后一道防线）。
    ///
    /// 注意：自激循环的**主**检测手段是「同一次 flush 内同一 effect 被自己
    /// （或其下游）的写入反复标脏」⇒ 那种情况报 [`ReactiveError::CycleDetected`]；
    /// 本变体只在兜底上限被撞到时出现（说明因果链判定漏了某条路径）。
    FlushCycle {
        /// 超出上限的节点。
        node: NodeId,
    },
    /// 派生求值过程中发生写操作（派生必须是纯函数）。
    MutationDuringDerivation {
        /// 发生写入的派生节点。
        node: NodeId,
    },
    /// 作用域已销毁后再使用。
    DisposedScope,
    /// 节点（派生）已被销毁后再使用。
    DisposedNode {
        /// 已销毁的节点。
        node: NodeId,
    },
    /// 读改写（`update`）期间该信号被**再次写入** ⇒ 本次读改写被拒绝。
    ///
    /// 为什么拒绝而不是覆盖：读改写闭包拿到的是**旧值快照**，闭包期间发生的
    /// 另一次写入会被闭包的返回值**静默覆盖** ⇒ 那是一次丢更新。宁可显式报错。
    ConcurrentWrite {
        /// 发生并发写的信号节点。
        node: NodeId,
    },
}

impl fmt::Display for ReactiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReactiveError::CycleDetected { path } => {
                write!(f, "CycleDetected: ")?;
                for (idx, node) in path.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " -> ")?;
                    }
                    write!(f, "{node}")?;
                }
                Ok(())
            }
            ReactiveError::FlushCycle { node } => {
                write!(
                    f,
                    "FlushCycle: 节点 {node} 在单次 flush 内执行次数超兜底上限"
                )
            }
            ReactiveError::MutationDuringDerivation { node } => {
                write!(
                    f,
                    "MutationDuringDerivation: 派生节点 {node} 求值期间发生写操作"
                )
            }
            ReactiveError::DisposedScope => write!(f, "DisposedScope: 作用域已销毁"),
            ReactiveError::DisposedNode { node } => {
                write!(f, "DisposedNode: 节点 {node} 已被销毁")
            }
            ReactiveError::ConcurrentWrite { node } => {
                write!(
                    f,
                    "ConcurrentWrite: 读改写期间节点 {node} 被再次写入，本次读改写被拒绝"
                )
            }
        }
    }
}

impl std::error::Error for ReactiveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_error_message_lists_the_path() {
        let err = ReactiveError::CycleDetected {
            path: vec![NodeId(1), NodeId(2), NodeId(1)],
        };
        assert_eq!(err.to_string(), "CycleDetected: #1 -> #2 -> #1");
    }

    #[test]
    fn other_errors_render_readable_messages() {
        assert_eq!(
            ReactiveError::FlushCycle { node: NodeId(3) }.to_string(),
            "FlushCycle: 节点 #3 在单次 flush 内执行次数超兜底上限"
        );
        assert_eq!(
            ReactiveError::MutationDuringDerivation { node: NodeId(4) }.to_string(),
            "MutationDuringDerivation: 派生节点 #4 求值期间发生写操作"
        );
        assert_eq!(
            ReactiveError::DisposedScope.to_string(),
            "DisposedScope: 作用域已销毁"
        );
        assert_eq!(
            ReactiveError::DisposedNode { node: NodeId(5) }.to_string(),
            "DisposedNode: 节点 #5 已被销毁"
        );
        assert_eq!(
            ReactiveError::ConcurrentWrite { node: NodeId(6) }.to_string(),
            "ConcurrentWrite: 读改写期间节点 #6 被再次写入，本次读改写被拒绝"
        );
    }
}
