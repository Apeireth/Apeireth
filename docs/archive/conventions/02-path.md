# 02 路径系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §2 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/02-path.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`Apeireth-rust/<path>/<file>`

## 顶层路径(R119 重建后)

| 路径 | 用途 | 状态 |
|---|---|---|
| `README.md` | 顶层入口 (3.9KB, R119-2 重建) | 🟢 |
| `CHANGELOG.md` | 顶层 release 索引 (2.7KB, R119-2 重建) | 🟢 |
| `ROADMAP.md` | 顶层时间线 (2.9KB, R119-2 重建) | 🟢 |
| `LICENSE` / `NOTICE` / `THIRD-PARTY-NOTICES.md` | 法律 (原样) | 🟢 |
| `CONTRIBUTING.md` / `INSTALL.md` / `SECURITY.md` | 顶层入口 (短, R119 保留) | 🟢 |
| `Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` / `clippy.toml` / `rustfmt.toml` / `deny.toml` / `CODEOWNERS` / `.gitignore` | 工程配置 (不动) | 🟢 |
| `_workspace/` | 施工人临时工作副本 (R119-1 留, .gitignore 忽略除 README) | 🟢 |

## docs/ 子目录(核验后,实际 24 子目录)

| 子目录 | 用途 | 状态 |
|---|---|---|
| `docs/conventions/` | 12 子规范系统 (R119-3a-1 拆) | 🟢 |
| `docs/versioning/` | 7 子系统版本号 (R119-3a-1 拆) | 🟢 |
| `docs/glossary/` | 21 词条术语表 (R119-3a-2 拆) | 🟢 |
| `docs/omnibus/` | 完整手册拆 (R119-3b 拆, 含 stage1-6/ design-v* / r11-baseline) | 🟢 |
| `docs/construction/` | 开工 / 收工 / 施工领导手册 (R119-3c 拆) | 🟢 |
| `docs/final-check/` | R14 末 / R54 / R70-R72 检查报告 (R119-3c 拆) | 🟢 |
| `docs/release/` | 各 release 详细 changelog + 索引 (R119-3c 建) | 🟢 |
| `docs/roadmap/` | 完整路线图 | 🟢 |
| `docs/adr/` | 架构决策记录 (12+4+21 = 37 文件) | 🟢 |
| `docs/api/` | API 端点 (HTTP + WebSocket) | 🟢 |
| `docs/sdk/` | SDK 集成 (Rust / Lark / LiveKit / Voice / Sandbox) | 🟢 |
| `docs/installation/` | 8 包安装 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) | 🟢 |
| `docs/ci/` | GitHub Actions CI 配置 | 🟢 |
| `docs/desktop/` | 桌面端 (Tauri stub) | 🟢 |
| `docs/security/` | 安全 (cosign / cargo audit / cargo deny / 5 守门) | 🟢 |
| `docs/research/` | 调研归档 (147 文件 / 2.2 MB) | 🟢 |
| `docs/v2-strategy/` | 5 战区战略 (8 文件) | 🟢 |
| `docs/r14-design/` | R14 周期产物 | 🟢 |
| `docs/stage1-6/` | 阶段 1-6 设计 (R11 LOCKED) | 🔒 LOCKED |
| `docs/1.0-release/` | 1.0 release 13 收口文档 | 🟢 |
| `docs/1.1-release/` | 1.1 release 索引 + 9 B-stage | 🟢 |
| `docs/1.0-release-prep/` | 1.0 release 续补 8 草稿 | 🟡 |
| `docs/license/` / `docs/licenses-3rdparty/` | 法律 (机械) | 🟢 |
| `docs/2.0-doc-system/` | 文档体系本身 (R119 留) | 🟢 |

## crates/ 子目录(核验后,实际 90+ crate)

| 路径 | 用途 | 数量 |
|---|---|---|
| `crates/<name>/` | crate 主目录 | 90+ |
| `crates/<name>/src/lib.rs` | crate 主代码 | — |
| `crates/<name>/src/*.rs` | crate 子模块 | — |
| `crates/<name>/tests/` | crate 集成测试 | — |
| `crates/<name>/examples/` | crate 示例 | — |
| `crates/<name>/Cargo.toml` | crate manifest | — |
| `crates/_v1306_backup/` | V1306 修真 backup (R119-1 已删, 现在 .gitignore 留) | — |
| `crates/apeireth-legacy/` | R11 Python 1100 模块归档 (保留) | — |
| `crates/_v1306_backup/` | V1306 修真 backup (R119-1 已删) | — |
| `crates/apeireth-memory/extensions/` | memory provider 子 crate | — |

## 路径核验

- ✅ crates/ 实际 90+ 个 crate (R20 阶段 1 加 14 + R36 删 5 + R119 累计), 24 LOCKED + 5 估补
- ✅ docs/ 实际 24 子目录 (R119-3a 重组后)
- ✅ reports/ 25+ 报告 (R 周期 + V 系列 + P 系列 + achievement 等)
- ✅ .github/workflows/ CI workflow (rust-ci + release-1.0.0 + dependabot + benchmark + eval-live)
- ✅ research/ 147 文件 / 2.2 MB (R14 阶段 1 调研归档)

## 不漂移

- 0 触碰 24 LOCKED crate
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
