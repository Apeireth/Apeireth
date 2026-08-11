# Apeireth 1.0 — AGI 操作系统 (Rust 重写)

> **Version**: 1.0.0 · **Edition**: 2021 · **License**: [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)
> **Tag**: `v1.0.0` (planned, per [ROADMAP](../ROADMAP.md) `v1.0.0 @ 2026-08-15`)
> **整合 #4 commit**: `abf12243` (2026-08-10 19:41, 46752 file changes)
> **整合 #5 commit**: 拆 3 commit (per 决策 #62, Mavis 自决, 等 R129-3 8 步 verify done)

---

## 这是什么

**Apeireth** = VCP 全栈 Rust 重写 + 单一定形式可全自关 (Self-Disable) + 双层中架构 + 编译期内保证 (13 键 hardcode) + R-测量 0.92.

5 战略 (终端 Coding Agent / LLM 工具 / Multi-Agent / 长程记忆 / 跨工具 协议) 同时打, **终极前端 Tauri**, **TUI 是"集成测试床"** (主人 8/4 23:33).

| 维度 | 1.0.0 状态 |
|------|----------:|
| **24 LOCKED crate** 入口签名 0 改 (B1) | ✅ 24/24 |
| **8 哲学锚** (B5, 6→8) | ✅ |
| **V0.5 30 维** (B3, 25→30) | ✅ |
| **6 重守门 v7** (B4, v6→v7) | ✅ |
| **13 键 verdict cache** (A3, 12+PHL-07) | ✅ |
| **借鉴 8/11** 真实施 (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234 + LiteLLM) | ✅ |
| **4100+ tests** pass (R125-16 + P12-1 verify) | ✅ |
| **8 硬墙 0 越界** (B1-B7 + A1-A3 + C1-C3) | ✅ |
| **0 装 PASS 严守** (8 真实施 + 0 限流 + 1 跳过) | ✅ |
| **整合 #4 commit** `abf12243` 严守 (0 重跑) | ✅ |
| **Cargo.toml version** 1.2.0 严守 (B2, tag 1.0.0 是 semver 大版本归 0) | ✅ |

## 5 quick links

- :material-rocket-launch: [**Getting Started**](getting-started.md) — 5 分钟跑通 `cargo install` + `cargo run`
- :material-api: [**API Reference**](api.md) — 13 键 verdict cache + 30 维 V0.5 + 6 重守门 v7 + 24 LOCKED
- :material-roadmap: [**Roadmap**](roadmap.md) — 1.0 → 2.0 路线图 (1.1 / 1.5 / 2.0 三个里程碑)
- :material-sitemap: [**Architecture**](architecture.md) — 8 哲学锚 + 24 LOCKED + 决策链 #22-#62
- :material-history: [**Changelog**](changelog.md) — v1.0.0 完整变更日志 (Keep a Changelog 1.1.0)
- :material-source-repository: [**Borrowed Repos**](borrowed-repos.md) — 借鉴 11/11 致谢 (8 真实施 + 3 限流 → 11/11 + 1 跳过)

## LICENSE 引用链 (per Apache 2.0 §4(d) NOTICE 条款)

```
LICENSE (Apache 2.0 verbatim, 175 行, P13-1 写)
  ↓ 引用 (§4(d))
NOTICE (项目 attribution, 66 行, R20 阶段 6 写)
  ↓ 引用
OSS_NOTICE.md (借鉴源码 11/11 致谢 + 决策链, 346 行, P13-1 写)
  ↓ 引用
THIRD-PARTY-NOTICES.md (cargo-about 0.8.4 生成, 1709 lines / 12 SPDX / 0 cargo-deny violation, 106KB)
```

- 📄 [LICENSE](../../LICENSE) — Apache License 2.0 verbatim
- 📄 [NOTICE](../../NOTICE) — 项目 attribution
- 📄 [OSS_NOTICE.md](../../OSS_NOTICE.md) — 借鉴源码 11/11 致谢
- 📄 [THIRD-PARTY-NOTICES.md](../../THIRD-PARTY-NOTICES.md) — 561 crates attribution
- 📄 [Cargo.toml](../../Cargo.toml) — `[workspace.package] license = "Apache-2.0"` 单一来源

## 借鉴 8/11 ✅ 真实施 (per 决策 #55 §3 + 决策 #57 §3)

| 借鉴源码 | 版本 / commit | sub-agent | 实施 |
|----------|--------------:|-----------|------|
| [clap-rs/clap](https://github.com/clap-rs/clap) | 4.6.6 (725 文件) | R125-2 ✅ done | derive 模式 |
| [hyperium/hyper](https://github.com/hyperium/hyper) | 0.1.20 (80 文件) | R125-3 ✅ done | 池复用 |
| [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) | 76d64c8 (175 文件) | R125-4 ✅ done | MCP 协议对齐 |
| [PyO3/PyO3](https://github.com/PyO3/PyO3) | 0.29.2 (928 文件) | R125-9 ✅ done | pybridge |
| [model-checking/kani](https://github.com/model-checking/kani) | 0.67.0 (4502 文件) | R125-10 ✅ done | 形式化模型 |
| [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) | d56666f (829 文件) | R125-13 ✅ done | StateGraph |
| [obra/superpowers](https://github.com/obra/superpowers) | 6.2.0 (234 文件) | R125-14 ✅ done | 9 skill files |
| [BerriAI/litellm](https://github.com/BerriAI/litellm) | 公开设计 1:1 翻译 | P6-1 retry 21:38 ✅ done | Provider Registry |

**借鉴 3/11 ⏳ 限流 → 全 done (per 决策 #36 + #47)**:
- LiteLLM (P6-1 retry 21:38 done) + opencode (P6-2 done) + Guardrails (P6-3 done) → 实际 10/11 ✅ + 0/11 ⏳ + 1/11 ❌

**借鉴 1/11 ❌ 跳过 (per 决策 #36 + 主人 8/6 授权)**:
- OpenCog AGPL-3.0 (协议不兼容, 0 集成)

## 8 哲学锚 (B5, per 决策 #33 §1.5 + P1-2 R126 升级)

- **S-1 复杂可推导**: 24 LOCKED crate 入口签名 0 改, 内部 fn 实施可改
- **S-2 实现可靠**: 4100+ tests pass, 0 装 PASS 严守
- **S-3 流程自化**: 整合 #4 → 整合 #5 拆 3 commit (决策 #62 拍板)
- **O-1 安全优先**: 6 重守门 v7, 13 键 verdict cache
- **O-2 当前聚焦**: 1.0 release 配 GitHub Pages, 终极前端 Tauri (等设计团队到位)
- **O-3 可追溯**: 决策链 #22-#62 全链, 41 sub-agent 报告全保留
- **O-4 任何人都能接手**: 12 子规范 + 12 文档 + R-测量 0.92
- **O-5 0 装 PASS**: ✅ 8 真实施 + 0 限流 + 1 跳过, 0 假装 "已实施"

## 终极前端 = Tauri (per 主人 8/4 23:33 + 用户记忆 #8)

> 主人 8/4 23:33: "我们最后要做的前端应该是Tauri, 但由于现在手头的ai团队没有适合干尤其是审美设计的, 所以web和桌面都搁置, 先做好tui来为桌面做准备."

**TUI = Tauri 集成测试床** (后端 API 表面 / 集成模式 / 用户流在 TUI 跑稳, Tauri 来了无缝换 UI 层).
**TUI 跟后端走 HTTP** (瘦客户端), 不直接调 lib.

**前端路线**:
- **现在**: TUI (per 决策 #9 改瘦后暂告段落, 优先后端)
- **终极**: Tauri 2.0 (5 nav + 主对话 + 9 organ 拟人化, per 决策 #11 阶段 4 frontend-proposal)
- **触发**: 等设计团队到位 (主人 0 必设计感, 宁可丑也不上没设计感的)
- **GitHub Pages**: 1.0 release 配套文档站, 0 依赖 Tauri, mkdocs 静态网站

## 整合 #5 commit 拍板 (per 决策 #62, Mavis 自决)

- **5.1** `整合 #5 commit: R125-R128-2 era 41 任务 src/ 实施 (50+ 文件)` — 31 M + 50+ untracked src/ + tests/ + examples/
- **5.2** `整合 #5 commit: 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)` — 6 文档 + Cargo.toml license 字段 + workspace.metadata.apeireth
- **5.3** `整合 #5 commit: 决策链 #30-#60 + 41 sub-agent 报告 + HANDOFF (reports/)` — 30+ reports/ 文件, 备查用, 0 影响 build

**整合 #4 commit abf12243 严守 100%** (0 重跑, 0 重 commit, master HEAD 严守).
**8 硬墙 0 越界 100%** (B1 24 LOCKED 入口签名 0 改 / B2 1.2.0 0 改 / A1 3 值 0 改 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / C3 升 v7 / 0 主动 push).
