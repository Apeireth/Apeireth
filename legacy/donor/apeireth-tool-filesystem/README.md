# apeireth-tool-filesystem

> Apeireth R137: filesystem extension (sandbox + atomic write + fsnotify + file lock + compat). 5 真实现模块 (sandbox / atomic / watch / lock / compat) + enhanced (EnhancedFileOps 总入口) + register (N17/TP2 统一注册件). **doc parsing 已声明移除** (lib.rs:47-48: parse.rs 从未存在 → clippy E0583, 落地时恢复, feature gated deps lopdf/docx-rs/calamine 仍保留). 旧 README "doc parsing" 系 R137 设计稿残影, src/ 实际无 parse.rs.

apeireth-tool-filesystem 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
