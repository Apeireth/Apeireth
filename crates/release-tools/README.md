# release-tools

> release-tools — TP20-S5 塞缝批: 发布期供应链验证 (cargo vet/audit/deny + CycloneDX SBOM) 的工程化载体

release-tools 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (1 src 文件 / 3 测试)

- `src/lib.rs` — 发布期工具链载体 (0 运行时代码, 仅供 cargo vet 的 publisher / third-party audits 引用, 让 .cargo/vet.toml 的 [[package]] 索引有根; 编译期 < 1s) + 3 测试
