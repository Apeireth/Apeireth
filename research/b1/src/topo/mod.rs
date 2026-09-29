//! 拓扑模块: 逐字节 vendored 引擎源码 + 漂移检测测试.
//!
//! 协议 §7.2 主方案: 将 crates/engine/memory 的两个纯 std+serde 模块 vendor 到此,
//! 避免 path-dep apeireth-memory 触发 workspace 版本解析失败 (tokio/rusqlite/aes-gcm 重链)。
//! 每个文件的漂移检测测试在运行时读取 `../../crates/engine/memory/src/<file>`
//! 与 vendor 副本做 SHA-256 比对, 保证「评的是引擎真实代码」。

pub mod betti_hole_detector;
pub mod kuramoto_resonance;
