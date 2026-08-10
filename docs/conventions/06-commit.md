# 06 Commit Message 规范

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §6 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/06-commit.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`<scope>: <subject>` (≤72 字符 subject)

## 11 种 scope

| scope | 含义 | 例子 (核验) |
|---|---|---|
| `R14` | R14 Rust 重写周期 | `R14: apeireth-cli session 启动` |
| `crate:<name>` | 特定 crate | `crate:apeireth-memory SQLite 存储` |
| `ci` | CI 配置 | `ci: GitHub Actions nightly + coverage` |
| `docs` | 通用文档 | `docs: 施工手册 README 引用` |
| `Fix-N` | 修正链 | `Fix-10: Apeireth 版本号系统` |
| `Manual-Rev-X` | 手册修订 | `Manual-Rev-G: 加版本号系统` |
| `Design-X.Y` | 设计层 | `Design-2.1: D2 增补` |
| `perf` | 性能 | `perf: V1130 wallclock → 2.5s` |
| `sec` | 安全 | `sec: Self-Disable 5 大机制` |
| `R17-<topic>` (R17+) | R17 短格式 | `R17-conventions: APEIRETH-CONVENTIONS 升 v12` |
| `round<N>-<NN> (<author>)` (R15+) | R15+ 长格式 | `round16-12 (chuling): 后端系统性验收 - 73/73 PASS` |

## R119 当前实际(核验)

| R | commit (核验) | 格式 |
|---|---|---|
| R119-1 | `79571cb4 chore(hygiene): R119-1 清 90 .tmp-* + _v1306_backup + _workspace/ 给施工人` | chore + R 周期短格式 |
| R119-2 | `00df7125 docs(top): R119-2 顶层 README/CHANGELOG/ROADMAP 瘦身 137KB → 9KB` | docs + R 周期短格式 |
| R119-3a-1 | (本批) | docs + R 周期短格式 |
| R78-R113 | `dd9b38f6 feat+docs+live: R78-R113 1.2 patch LIVE 续 11 R + 1 LIVE 一气呵成` | feat+docs+live + R batch + 1 commit 总 |
| R114-R118 | `5c546a84 feat: finish R114-R118 dynamic operations layer` | feat + R batch (codex 风格, 简) |

## 主人 R119 拍板节奏

> 主人 8/9 拍板"1 commit 也行",3 batch 1 commit 总,主人 R78-R113 实际节奏。

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
