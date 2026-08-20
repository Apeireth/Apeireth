# apeireth-extension

> apeireth-extension — 6 类扩展 (sync/async/static/service/messagePreprocessor/hybrid) + extension.toml 严格 schema + 审核后注册 + 沙盒 + 调用审计 (P28 round 5-03)

apeireth-extension 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。src 模块 9 个 + 1 子目录 plugins/: audit / error / lib / manifest / organ_kani_proofs / registry / sandbox / traits / types。测试数(单测标注): 36 (audit 8 + manifest 8 + organ_kani_proofs 5 + registry 2 + sandbox 9 + traits 4)。完整架构见 [docs/](../../docs/README.md)。
