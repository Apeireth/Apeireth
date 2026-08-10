# 01 主代码版本 (semver)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-VERSIONING.md §1 拆出,核验后写。

```
[Document-Meta]
Document: docs/versioning/01-code.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`Apeireth-MAJOR.MINOR.PATCH` (标准 semver)

| 部分 | 含义 | 例子 |
|---|---|---|
| MAJOR | 重大架构变更 | 1.0, 2.0 (打破式) |
| MINOR | 新功能 + 新 crate + 修正链 | 0.14, 0.15 |
| PATCH | 修复 bug + 小修 | 0.14.0, 0.14.1 |

## 当前 (R119 核验)

| 维度 | 值 | 核验来源 |
|---|---|---|
| **Cargo.toml `[workspace.package] version`** | **`1.1.0`** | `Apeireth-rust/Cargo.toml` line 246 (master HEAD `5c546a84`) |
| **doc-level Apeireth-VERSIONING.md** | `Apeireth-1.1.2-R72` (R70-R72 1.2 patch LIVE, doc-level) | per APEIRETH-VERSIONING.md §R70-R72 |
| **R-Cycle 标识** | `R114-R118` (R119 重建 R114-R118 之上) | per codex `5c546a84` |
| **Doc-level 灵活** | per master 8/9 拍板 "locked 文档可灵活" | per APEIRETH-VERSIONING.md R54 续 |

## 实际 Cargo.toml 位置

```toml
# Apeireth-rust/Cargo.toml
[workspace.package]
version = "1.1.0"  # R119 严守, semver 严守
edition = "2021"
rust-version = "1.80"
```

## 历史版本演进(核验)

| R | workspace.version | 来源 |
|---|---|---|
| R11 | 0.14.0 | 历史 |
| R14 | 0.14.0 | 历史 |
| R17 | 0.14.0 | 历史 (1.0 release 收官前) |
| R20 阶段 6 | 1.0.0 | `702942fb` (R19 T10 known bug 修) |
| R38 1.1 RC | **1.1.0** | `a64fe197` (B9 workspace 1.0 → 1.1) |
| R70-R72 1.2 patch LIVE | **1.1.0** (doc-level 1.1.2-R72) | 严守 + 文档灵活 |
| R78-R113 1.2 patch LIVE 续 | **1.1.0** | 严守 |
| R114-R118 动态运营层 | **1.1.0** | codex 严守 |
| R119 文档重建 | **1.1.0** (本批) | 严守 |

## 严守原则 (R119)

- 🔒 workspace.version = 1.1.0 **不动**
- 🟢 doc-level 可以反映 patch 内容 (1.1.2-R72, 1.1.0-R114 等)
- 🟢 1.2 release 时统一 bump workspace.version

## 不漂移

- 0 触碰 24 LOCKED crate
- 0 改 R11 baseline 3 值
- 0 改 6 哲学锚
