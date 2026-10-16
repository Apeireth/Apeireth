//! B1 拓扑记忆实验确定性骨架 (协议 Phase 0/1, 0 LLM)。
//!
//! 用途: 会话切分 → 构图 → vendored Betti 环启发式分析 → 诊断落盘,
//! 为 Phase 2-4 (LLM 缺口标注 / 提问生成 / 共振跨域 + judge) 提供确定性的输入层。
//! 骨架不含任何 LLM 调用 (dry-run 打印 0 条调用清单, LLM 阶段留 TODO 占位)。

pub mod embed;
pub mod graph;
pub mod log;
pub mod session_split;
pub mod topo;

pub use embed::{EmbeddingProvider, InjectedEmbeddingProvider, l2_normalize};
pub use graph::{activation_energy, build_nodes, truncate_nodes, NodeInput};
pub use session_split::{split_sessions, split_sessions_str, SessionSlice, SessionTurn};
pub use topo::betti_hole_detector::{BettiHoleDetector, BettiTopologicalReport, ManifoldConceptNode};
pub use topo::kuramoto_resonance::{KuramotoOscillator, KuramotoResonanceEngine};
