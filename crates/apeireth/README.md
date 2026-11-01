# apeireth

**Apeireth 引擎门面 crate** —— 一个依赖拿到引擎核心模块。

> ⚠️ **这是门面，不是新能力。** 本 crate 只做 `pub use` 重导出：零业务逻辑、零新 trait、
> 零新注册表。它存在的理由是**入口**（占住 crates.io 裸名 + 省掉手写多行依赖），
> 不是提供超出各子 crate 的任何功能。能力四级口径见
> [`docs/03-reference/capabilities-matrix.md`](../../docs/03-reference/capabilities-matrix.md)。

## 安装

```bash
cargo add apeireth
```

当前版本为预发布 `2.0.0-rc.4`，`cargo add` 会写入 `apeireth = "2.0.0-rc.4"`
（预发布版本在 semver 中默认精确匹配，不会漂移）。

## 用法

```rust
// 默认导出（4 个核心模块）
use apeireth::{core, protocol, runtime, tools};

// 可选模块按 feature 打开
use apeireth::{memory, tools};
```

```toml
# 打开可选模块
apeireth = { version = "2.0.0-rc.4", features = ["tools", "memory"] }
```

## 导出表

| 门面路径 | 实际 crate | 默认 |
|---|---|---|
| `apeireth::core` | `apeireth-core` | ✅ |
| `apeireth::protocol` | `apeireth-protocol` | ✅ |
| `apeireth::runtime` | `apeireth-runtime` | ✅ |
| `apeireth::memory` | `apeireth-memory` | feature `memory` |
| `apeireth::guard` | `apeireth-guard` | feature `guard` |
| `apeireth::orchestration` | `apeireth-orchestration` | feature `orchestration` |
| `apeireth::tools` | `apeireth-tools-canonical` | feature `tools`（传递依赖重：**+160 包**） |

**不在门面内**：`apeireth-organ` / `apeireth-image-core` —— 需要时直接依赖对应 crate。

## 依赖树规模（实测）

| 依赖写法 | `Cargo.lock` 包数（实测 2026-10-31） |
|---|---|
| `apeireth`（默认） | **92**（= 91 个依赖 + 门面本体 1 个） |
| `apeireth` + `features = ["tools"]` | 252 |

`tools` 的传递依赖会带进图像家族 11 个 crate 及其编解码/色彩/几何依赖链（**+160 包**），
故默认关闭；需要工具层时显式开。

## 纪律

- **门面不新增能力**：本 crate 里出现任何非 `pub use` 的实质逻辑即为违规（ADR-0019）。
- **导出集合只增不减**：删除模块即门面 breaking。
- **测试数字不在此抄写**：单一真源 [docs/03-reference/test-baseline.md](../../docs/03-reference/test-baseline.md)。

## 许可

Apache-2.0 OR MIT。第三方归属见仓库根 `NOTICE` / `OSS_NOTICE.md` / `THIRD-PARTY-NOTICES.md`。

## 决策真源

- [ADR-0019（门面 crate + 裸名占位）](../../docs/adr/0019-apeireth-facade-crate.md)
- [ADR-0017（引擎仓 / 产品仓分离）](../../docs/adr/0017-engine-product-repo-split.md)
