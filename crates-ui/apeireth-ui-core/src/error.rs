//! 响应式运行期错误类型。

use std::fmt;

use crate::graph::NodeId;

/// 响应式运行期的错误类型。
///
/// 错误设计原则（设计文档 §4.4）：报错必须**自带人名** —— 节点可通过
/// [`crate::Runtime::set_label`] 挂标签，报错路径里带上标签才可定位。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReactiveError {
    /// 求值环：派生 A 读派生 B，派生 B 又读回 A。
    CycleDetected {
        /// 环上的节点路径（首尾为同一节点）。
        path: Vec<NodeId>,
    },
    /// 单次 flush 内某节点的执行次数超过上限（自激循环兜底）。
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
                write!(f, "FlushCycle: 节点 {node} 在单次 flush 内执行次数超上限")
            }
            ReactiveError::MutationDuringDerivation { node } => {
                write!(
                    f,
                    "MutationDuringDerivation: 派生节点 {node} 求值期间发生写操作"
                )
            }
            ReactiveError::DisposedScope => write!(f, "DisposedScope: 作用域已销毁"),
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
            "FlushCycle: 节点 #3 在单次 flush 内执行次数超上限"
        );
        assert_eq!(
            ReactiveError::MutationDuringDerivation { node: NodeId(4) }.to_string(),
            "MutationDuringDerivation: 派生节点 #4 求值期间发生写操作"
        );
        assert_eq!(
            ReactiveError::DisposedScope.to_string(),
            "DisposedScope: 作用域已销毁"
        );
    }
}
