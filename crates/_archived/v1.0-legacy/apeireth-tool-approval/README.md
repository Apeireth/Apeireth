# apeireth-tool-approval

> Apeireth R17 战役 2-3: 工具审批 (5 规则 + 5 分钟窗口 + fuzzy matching 集成, 借鉴自 toolApprovalManager.js (origin: open-source))

Apeireth 1.0 工作区 crate。src 模块: approval_bridge / decision / fuzzy_bridge / history / lib / manager / organ_kani_proofs / rule / rule_trait。测试数(单测标注): 85 (lib 5 + decision 12 + fuzzy 7 + history 3 + manager 12 + organ_kani 10 + rule 36)。注: 旧 README 列 "rule_trait" 但漏 "rule" — src 实有 9 个 .rs 文件 (代码 src/lib.rs:78-86).

## 文档

- 架构: [docs/01-architecture/architecture.md](../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../docs/03-reference/crates.md)
