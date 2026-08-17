# release-tools

发布期供应链验证的工程化锚点（TP20-S5 塞缝批）。

## 它做什么

把「发布前必须做的安全检查」从散落的 shell 脚本里抽到 first-class crate：

- 共享 `workspace = true` 的版本号，编译期硬编码 `1.0.0`
- 供 `.cargo/vet.toml` 当 publisher / third-party audits 的索引根
- CI `release.yml` 的 `security-and-sbom` job 引用 `release_tools::VERSION` 做对账

## 它不做什么

- **零运行时逻辑**：本 crate 不导出可执行能力，只暴露 3 个常量
- **不取代工具链**：`cargo vet` / `cargo audit` / `cargo deny` / `cargo-cyclonedx`
  还是外部命令，本 crate 只是「文档期/编译期锚点」
- **不接在线 CVE DB**：遵循「不假装」（TP11 哲学），audit 失败时阻塞，不静默

## 文件

| 文件 | 用途 |
|---|---|
| `Cargo.toml` | workspace member，零依赖 |
| `src/lib.rs` | 3 个常量 + 3 个编译期测试 |
| `tests/` (暂无) | vet 审计配置覆盖在 `.cargo/vet.toml` |

## 引用方式

```rust
use release_tools::{VERSION, SBOM_FILENAME, CYCLONEDX_SPEC_VERSION};

assert_eq!(VERSION, "1.0.0");
assert_eq!(SBOM_FILENAME, "cyclonedx-sbom.json");
```

## 验收

- `cargo build -p release-tools` → 编译通过（< 1s）
- `cargo test -p release-tools` → 3 个测试全过
- `cargo vet --manifest-path crates/release-tools/Cargo.toml` → 0 失败

详见项目根 `reports/22ac8801-622b-4fcc-b061-c9beb6c54ca1-devops_engineer2-report.md`。