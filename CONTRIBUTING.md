# 贡献指南 (Apeireth 贡献与开发公约)
# Contributing Guide & Developer Certificate of Origin (DCO)

> Apeireth 是基于纯 Safe Rust 构建的 AGI 操作系统微内核与认知生命体框架。贡献前请阅读哲学与设计原则 —— [docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md)（9 哲学锚 / 三洋葱 / 0 装 PASS）。

---

## 📜 开发者原创性声明与双向许可契约 (DCO Equivalent Contribution Policy)

为了在保护项目独立性与知识产权的同时避免繁琐的提交标记（无需强制配置 sign-off 机器人或每次手工敲 `-s`），Apeireth 遵循 **Apache 2.0 §5 与行业公认的 “Inbound = Outbound” 双向许可原则**：

> **贡献者承诺与授权保证（等同于 DCO 1.1 效力）**：
> 
> 当您向 Apeireth 项目提交任何 Pull Request、Issue、Patch 或代码贡献时，即视为您不可撤销地确认并同意：
> 
> 1. **原创性与合法权利**：该贡献由您独立创作，或者您拥有合法授权将该贡献按照本项目的开源协议提交给本项目；
> 2. **双重开源授权**：该贡献自动以 **MIT OR Apache-2.0 双协议** 进行授权，受本项目根目录下 [`LICENSE-MIT`](LICENSE-MIT) 与 [`LICENSE-APACHE`](LICENSE-APACHE) 的完整条款约束，不附带任何额外限制或专有保留条件；
> 3. **公开性与防纠纷**：您知悉并同意本项目的贡献记录、Commit 历史及代码内容均处于公开发布状态。

---

## 📚 核心文档索引

- [docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md) — 9 大哲学锚（S-1~S-3 核心安全, O-1~O-6 长期共生）
- [docs/01-architecture/architecture.md](docs/01-architecture/architecture.md) — 16-Crate 单向微内核拓扑图
- [docs/01-architecture/security.md](docs/01-architecture/security.md) — 零信任三洋葱物理沙箱安全模型
- [docs/03-reference/crates.md](docs/03-reference/crates.md) — 16 个核心 Crate 职责速查
- [DISCLAIMER.md](DISCLAIMER.md) — 法律免责与合规声明
- [ROADMAP.md](ROADMAP.md) — 路线图与演进规划

---

## ⚙️ 提交前本地质量门禁

在向分支发起 PR 或提交前，必须确保本地 100% 通过以下三道硬质门禁：

```bash
cargo check --workspace --all-targets                       # 16-Crate 编译全 Target 干净
cargo clippy --workspace --all-targets --all-features -- -D warnings # 0 错误 0 警告
cargo test --workspace                                      # 2012+ 单元/集成测试全绿
cargo fmt --all --check                                     # Rustfmt 代码风格严格对齐
pwsh scripts/secret-scan.ps1 -Mode scan-all                 # 凭据与安全扫描 0 泄露
```

---

## 🛡️ 0 装 PASS 纪律（核心守则）

1. **未实现 = 显式标注**：绝不允许假装 mock、伪造 pass 或空壳桩；
2. **Safe Rust 铁律**：除 OS 级进程绑定所必需的底层 FFI 孤岛外，全仓严禁任何未受审 `unsafe` 代码；
3. **不可变脊柱资产**：严守 9 哲学锚、13 键决策缓存、自禁用保护与 R11 常数，严禁架构漂移。

---

## ⚖️ 开源协议

Apeireth 采用 **[MIT OR Apache-2.0](LICENSE)** 双重开源协议。
详情请参见 [LICENSE-MIT](LICENSE-MIT) 与 [LICENSE-APACHE](LICENSE-APACHE)。
