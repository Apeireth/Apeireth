# Changelog — v1.0.0

> **完整 CHANGELOG**: 见根目录 [`CHANGELOG.md`](https://github.com/apeireth/apeireth-rust/blob/main/CHANGELOG.md) (P7-1 21:23 写 v1.0.0, 42.8KB / 435 行)
> **格式**: [Keep a Changelog 1.1.0](https://keepachangelog.com/zh-CN/1.1.0/) + [Semantic Versioning](https://semver.org/lang/zh-CN/)
> **整合 #4 commit**: `abf12243` (2026-08-10 19:41, 46752 file changes)
> **0 主动 commit 严守** (Mavis 整合 #5 commit 时机拍板, per 决策 #62 + decision-33 C1)

---

## [1.0.0] - 2026-08-10

> **R127-2 P7-1 准备 (2026-08-10)**: 重写项目 CHANGELOG, 遵循 Keep a Changelog 1.1.0 格式. **0 主动 commit 严守** (写到主仓, Mavis 整合 #5 commit 时机拍板).

### 🎉 Highlights

Apeireth **1.0.0** 是 R14 Rust 重写项目的**第一个稳定版本 0 release**, 包含 R125 era 真实施 (8/11 真实施) + R126 era 升级完成 (8 哲学锚/30 维/6 重 v7/13 键) + R127 era 整合 #5 pre-check + Library Stage 4-6, **8 硬墙 0 越界**, **0 装 PASS 严守**, 整合 #4 commit `abf12243` 严守 master.

### ✅ Added (R125-R128-2 era 41 任务全 done)

- ✅ **整合 #4 commit `abf12243` done** (8/10 19:41, 主人自执行 A 选项, 46752 file changes, 0 M+?? 异常)
- ✅ **24 LOCKED crate 入口签名 0 改** (B1, 12 已知 + 12 Mavis 自主, 入口签名 0 改, 内部 fn 实施可改)
- ✅ **8 哲学锚升级** (B5, 6→8: 增 S-3 流程自化 + O-1 安全优先, P1-2 R126 升级 done)
- ✅ **V0.5 25→30 维升级** (B3, R125-13 60 tests 30 维 sum=1.0 严守, R126 retry done)
- ✅ **6 重守门 v6 → v7 升级** (B4, R126 retry done, v7 = v6 6 重 + Colang DSL 第 6 重)
- ✅ **13 键 verdict cache** (A3, 12 原 12 + PHL-07 NotUnoptimizable, 整合 #4 commit done)
- ✅ **Library v1.0 礼物** (30 经典书 + 100+ 论文 + 50+ 视频 + 10+ 课程 + 10+ hub = 200+ 资源, 9 organ 1:1)
- ✅ **借鉴源码 8/11 ✅ cloned 真实施** (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234 + LiteLLM P6-1 retry 21:38)

### 🔄 Changed

- **workspace.version** 1.1.0 → 1.2.0 (per P15-1 22:48, B2 upgrade from 1.1.0, 1.0 release 时 tag = 1.0.0 是 semver 大版本归 0)
- **Cargo.toml** 配 LICENSE 字段 (`license.workspace = true` 单一来源, 90+ sub-crate 中 65+ 继承, 27 硬编码 = 已知 TODO)
- **Cargo.toml metadata.apeireth** section (73 行, 8 字段: borrow / hard_walls / locked_crates_count / philosophy_anchors / measurement_dimensions / guard_gates_version / verdict_cache_keys / integration_chain / license_files / commit_policy / decision_chain_range)
- **LICENSE 引用链** 完整化 (LICENSE 175 行 + NOTICE 66 行 + OSS_NOTICE.md 346 行 + THIRD-PARTY-NOTICES.md 1709 lines)

### 🐛 Fixed

- **24 LOCKED crate 内部 fn 实施可改** (per 决策 #33 §2.3 B1, 整合 #4 commit 严守, 0 必重跑)
- **.gitignore 升级版** (整合 #4 commit 包含, 0 必重跑)
- **0 M+?? 异常** (整合 #4 commit 严守, 0 必重跑)

### 🔒 Security (B4 6 重守门 v7)

- **守门 1-5** (嵌套, v6 → v7 升级 + 边界场景 + 跨字段 + DSL 表达式 + 角色继承 + 决策链)
- **守门 6** (Colang DSL, v7 新增, Colang 模板)
- **6 重 v7 严守** (per P1-3 R126 retry done, 整合 #4 commit 严守)

### 📦 Dependencies (Cargo.toml workspace.metadata.apeireth.borrow)

- **clap 4.6.6** (725 文件) — derive 模式
- **hyper 0.1.20** (80 文件) — 池复用
- **servers 76d64c8** (175 文件) — MCP 协议对齐
- **PyO3 0.29.2** (928 文件) — pybridge
- **kani 0.67.0** (4502 文件) — 形式化模型
- **langgraph d56666f** (829 文件) — StateGraph
- **superpowers 6.2.0** (234 文件) — 9 skill files
- **LiteLLM** (公开设计 1:1 翻译, P6-1 retry 21:38 done) — Provider Registry

### 📊 Metrics (R-测量 0.92)

| 指标 | 1.0.0 值 | 来源 |
|------|---------:|------|
| R-测量 | 0.92 | R125-16 + P12-1 verify |
| Tests | 4100+ | R125-16 + P12-1 verify |
| 24 LOCKED 入口签名 | 24/24 PASS | P2-3 + P4-1 + P14-1 retry 三方 verify |
| 8 哲学锚 | 8/8 | P1-2 R126 升级 done |
| V0.5 30 维 | 30/30 sum=1.0 | P1-4 R126 verify retry done |
| 6 重守门 v7 | 6/6 | P1-3 R126 retry done |
| 13 键 verdict cache | 13/13 | 整合 #4 commit done |
| Library v1.0 资源 | 200+ | P2-4 R126 done |
| 借鉴 8/11 真实施 | 8/8 | P6-1/2/3 retry done |
| 8 硬墙 0 越界 | 11/11 | 整合 #4 commit 严守 |

### 📝 Notes

- **整合 #4 commit 严守 100%** (per 决策 #48, 19:41 done, 0 必重跑, 0 必重 commit)
- **整合 #5 commit 时机** = 8 项 verify 100% 落实 (per 决策 #61 §1.4 + 决策 #62 §7), Mavis 自决拍板
- **整合 #5 commit 拆 3 commit** (per 决策 #62, Mavis 自决):
  - 5.1 src/ 实施 (50+ 文件)
  - 5.2 docs/ + Cargo.toml (10 文件)
  - 5.3 reports/ 决策链 + 报告 (30+ 文件)
- **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #58 §7 + 决策 #61 §6 + 决策 #62 §9, Mavis 0 主动 push, 主人起床后手跑)
- **1.0 release tag = v1.0.0** (per semver 大版本归 0, 决策 #22 §2.2)
- **GitHub Pages 1.0 release 配套** (per R129-13 done 01:00, 7 文档 + mkdocs.yml)

### 🔗 Refs

- 📄 [ROADMAP.md](roadmap.md) — 1.0 → 2.0 路线图
- 📄 [Architecture](architecture.md) — 8 哲学锚 + 24 LOCKED + 决策链
- 📄 [Borrowed Repos](borrowed-repos.md) — 借鉴 11/11 致谢
- 📄 [OSS_NOTICE.md](https://github.com/apeireth/apeireth-rust/blob/main/OSS_NOTICE.md) — 借鉴源码 11/11 完整致谢
- 📄 [RELEASE_NOTES.md](https://github.com/apeireth/apeireth-rust/blob/main/RELEASE_NOTES.md) — 1.0.0 release notes (P7-3 retry 21:27 写, 36.8KB / 419 行)

## 历史版本

| 版本 | Date | 关键 |
|------|------|------|
| 0.x | R114-R118 动态试运营 | 4921 passed / 88 suites / 0 failed, workspace.version 1.1.0 |
| 1.0.0 | 2026-08-10 (本版本) | R125-R127 era + 整合 #4 commit done + 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 |

完整历史见 [`docs/release/`](https://github.com/apeireth/apeireth-rust/tree/main/docs/release) 目录 (1.0.0 / 1.1.0 / 1.1.1 / 1.1.2 / 1.2-candidate / 1.2-r114-r118 等).
