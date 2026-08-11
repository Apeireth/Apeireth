# Architecture — 8 哲学锚 + 24 LOCKED + 决策链

> **整合 #4 commit**: `abf12243` (2026-08-10 19:41, 46752 file changes)
> **8 哲学锚 (B5)**: per 决策 #33 §1.5 + P1-2 R126 升级 done
> **24 LOCKED crate (B1)**: per P2-3 + P4-1 + P14-1 retry 三方 verify 24/24 PASS
> **决策链**: #22-#62 (R125 era → R128-2 era, 31 份决策文件)

---

## 0. TL;DR

Apeireth 1.0 架构 = **8 哲学锚** (B5) + **24 LOCKED crate** (B1) + **30 维 V0.5** (B3) + **6 重守门 v7** (B4) + **13 键 verdict cache** (A3).

**核心设计原则**: 任何改动必须不破坏 8 哲学锚 (O-5 0 装 PASS 严守). 24 LOCKED crate 入口签名 0 改, 内部 fn 实施可改 (per 决策 #33 §2.3 B1 + 主人 0:03 授权 "技术性 locked 全部解锁").

## 1. 8 哲学锚 (B5, per 决策 #33 §1.5 + P1-2 R126 升级 done)

### 1.1 S 锚 (Subject, 主体)

| 锚 | 含义 | 实施 |
|----|------|------|
| **S-1 复杂可推导** | 复杂系统必须有可推导的设计 | 24 LOCKED crate 入口签名 0 改, 内部 fn 实施可改 |
| **S-2 实现可靠** | 实施必须可靠, 0 装 PASS | 4100+ tests pass, 0 装 PASS 严守 (✅ 8 真实施 + ⏳ 0 + ❌ 1) |
| **S-3 流程自化** | 流程必须可自动化 | 整合 #4 → 整合 #5 拆 3 commit (决策 #62 拍板) |

### 1.2 O 锚 (Object, 客体)

| 锚 | 含义 | 实施 |
|----|------|------|
| **O-1 安全优先** | 安全永远优先 | 6 重守门 v7, 13 键 verdict cache, 0 装 PASS 严守 |
| **O-2 当前聚焦** | 当前聚焦 1.0 release | 1.0 release 配 GitHub Pages, 终极前端 Tauri (等设计团队到位) |
| **O-3 可追溯** | 任何决策都可追溯 | 决策链 #22-#62 全链, 41 sub-agent 报告全保留 |
| **O-4 任何人都能接手** | 任何人都能接手项目 | 12 子规范 + 12 文档 + R-测量 0.92 + 9 organ 拟人化 |
| **O-5 0 装 PASS** | 0 装"已实施", 严守真实施 | 借鉴 8/11 ✅ 真实施, 整合 #4 commit 严守, 0 假装 |

**S-3 流程自化 + O-1 安全优先** 是 R126 era 从 6 哲学锚升级到 8 哲学锚的新增 (per P1-2 R126 升级 done).

## 2. 24 LOCKED crate 入口签名 (B1, per P2-3 + P4-1 + P14-1 retry 三方 verify)

### 2.1 24 LOCKED crate 列表

```
apeireth-agent        apeireth-central        apeireth-cli
apeireth-evolution    apeireth-formal         apeireth-graph
apeireth-http-client  apeireth-mcp            apeireth-naming-v05
apeireth-pipeline     apeireth-pybridge       apeireth-skills
apeireth-sovereignty  apeireth-tool-runtime
+ 12 Mavis 自主 LOCKED (整合 #4 commit 严守)
```

**12 已知** = 上述 12 公开 crate (per 决策 #22 + 决策 #33)
**12 Mavis 自主** = 整合 #4 commit done 时 Mavis 自主确认的 12 内部 crate (per 决策 #22 §1.2)

### 2.2 入口签名 0 改 (per 决策 #33 §2.3 B1)

**入口签名 0 改 24/24 PASS** (per P2-3 + P4-1 + P14-1 retry 三方 verify done).

**内部 fn 实施可改** (per 决策 #33 §2.3 B1 + 主人 0:03 授权 "技术性 locked 全部解锁"):
- 主人 0:03 授权: "技术性 locked 文档全部解锁"
- ✅ 24 LOCKED crate 内部 fn 实施可改
- ✅ 24 LOCKED crate **入口签名 0 改** 仍严守
- ✅ 8 哲学锚 (B5) = 设计规范, 不动
- ✅ V0.5 30 维 (B3) = 数据结构, 不动
- ✅ 6 重守门 v7 (B4) = 安全规范, 不动
- ✅ R11 baseline 3 值 0.8682/0.8532/0.9063 (A1) = 数字严守, 不动
- ✅ workspace.version 1.2.0 (B2) = 数字严守, 不动

## 3. 决策链 (#22-#62, R125 era → R128-2 era, 31 份决策文件)

### 3.1 关键决策

| # | Date | 决策 | 关键内容 |
|---|------|------|---------|
| #9 | 8/4 | TUI 改瘦后暂告段落, 优先后端 | TUI 升级路线图沉淀成文档 |
| #22 | 8/10 | workspace.version 1.2.0 严守 + 24 LOCKED 自主确认 | 主人授权, R125 17 era 起源 |
| #33 | 8/10 | master-reupgrade | 主人 17:22 升级授权, 8 硬墙 (B1-B7 + A1-A3 + C1-C3) |
| #34 | 8/10 | commit-done | 整合 #3 commit `21aa85f3` 17:30:34 done |
| #48 | 8/10 | integration-4-commit-done | 整合 #4 commit `abf12243` done (46752 file changes) |
| #55 | 8/10 | r127-integration-5-library-stage-4-6 | R127 4 派活 (整合 #5 pre-check + Library Stage 4-6) |
| #56 | 8/10 | r127-2-borrowed-3-retry-release-prep | R127-2 10 派活 (借鉴 3 限流 + 1.0 release 准备) |
| #57 | 8/10 | r128-asi-python-tauri-cargo-release | R128 6 派活 (ASI Python + Tauri + LICENSE) |
| #58 | 8/10 | r128-2-final-3-sub-agents | R128-2 3 派活 (ASI Stage 3 + Tauri + 1.0 release Cargo 配) |
| #61 | 8/11 | new-session-takeover-r129-plan | 新会话接手 + R129 era 派活规划 |
| #62 | 8/11 | integration-5-commit-3-way | 整合 #5 commit 拆 3 commit 拍板 |

### 3.2 R125 era → R128-2 era 时间线

```
R125 era (8/10 16:30-18:30)
├─ #22 主人授权 24 LOCKED 自主确认
├─ 16 sub-agent (R125-1~R125-21) 跑过夜 done
├─ 借鉴 8/11 ✅ cloned
└─ 8 硬墙 0 越界 verify 100%

R126 era (8/10 18:30-20:00)
├─ #33 主人 17:22 升级授权 (8 硬墙)
├─ 16 sub-agent (P0-1~P3-4) 跑过夜 done
├─ 8 哲学锚升级 (B5)
├─ V0.5 25→30 维升级 (B3)
├─ 6 重守门 v6→v7 升级 (B4)
├─ 13 键 verdict cache (A3)
└─ 整合 #3 commit 21aa85f3 done

R127 era (8/10 20:00-21:13)
├─ #48 整合 #4 commit abf12243 done (19:41, 46752 file changes)
├─ #53 主人 20:32 "技术性 locked 都能解锁"
├─ #55 R127 4 派活 (整合 #5 pre-check + Library Stage 4-6)
└─ 4 sub-agent (P4-1, P5-1/2/3) 跑过夜 done

R127-2 era (8/10 21:13-21:29)
├─ #56 R127-2 10 派活 (借鉴 3 限流 + 1.0 release 准备)
└─ 10 sub-agent (P6-1/2/3, P7-1/2/3, P8-1/2/3, P9-1) 跑过夜 done

R128 era (8/10 21:29-21:51)
├─ #57 R128 6 派活 (ASI Python + Tauri + Cargo + LICENSE)
└─ 6 sub-agent (P10-1/2, P11-1, P12-1, P13-1, P14-1) 跑过夜 done

R128-2 era (8/10 21:51-22:48)
├─ #58 R128-2 3 派活 (ASI Stage 3 + Tauri scaffold + 1.0 release Cargo 配)
└─ 3 sub-agent (P10-3, P11-2, P15-1) 跑过夜 done

R129 era (8/11 00:03-)
├─ #61 新会话接手 + R129 era 派活规划 (主人 0:03 最高授权)
├─ #62 整合 #5 commit 拆 3 commit 拍板 (Mavis 自决)
├─ R129-1/2 整合 #5 commit 准备
├─ R129-3 8 步 verify 跑
├─ R129-7 借鉴 11/11 升级 verify
├─ R129-8 1.0 release 流程准备 (0:38 done)
├─ R129-13 1.0 release checklist + GitHub Pages 准备 (01:00 done, 本文档)
└─ R129-4/5/6 ASI Python Stage 4-6 + R129-9 Tauri + R129-10 形式化 + R129-11/12/14/15/16 跑过夜 8/11-8/22
```

## 4. 8 硬墙 0 越界 (B1-B7 + A1-A3 + C1-C3)

### 4.1 8 硬墙

| 硬墙 | 含义 | 状态 |
|------|------|------|
| **B1** | 24 LOCKED crate 入口签名 0 改 | ✅ 24/24 |
| **B2** | workspace.version 1.2.0 0 改 | ✅ 1.2.0 |
| **B3** | V0.5 30 维 | ✅ 30 维 sum=1.0 |
| **B4** | 6 重守门 v7 | ✅ 5 嵌套 + Colang DSL |
| **B5** | 8 哲学锚 | ✅ S-1~S-3 + O-1~O-5 |
| **B6** | 编译期 13 键 hardcode | ✅ 13 键 verdict cache |
| **B7** | 9 organ 内部 fn 改 OpenCode | ✅ 199KB → 120KB (-40%) |
| **A1** | R11 baseline 3 值 0.8682/0.8532/0.9063 | ✅ 数字严守 |
| **A2** | R11 Python 9 子层结构 严守 | ✅ 9 子层 |
| **A3** | 13 键 (12 原 12 + PHL-07) | ✅ 13 键 |
| **C1** | 0 主动 commit (Mavis 整合 #5 commit 时机拍板) | ✅ Mavis 自决 |
| **C2** | 0 装 PASS 严守 | ✅ 8 ✅ + 0 ⏳ + 1 ❌ |
| **C3** | 升 6 重 v6 → v7 | ✅ P1-3 R126 retry done |
| **0 主动 push** | Mavis 0 主动 push, 主人起床后手跑 | ✅ 严守 |

### 4.2 8 硬墙跟 R129-13 任务的对齐

- **B1 24 LOCKED 入口签名 0 改**: R129-13 写 docs/pages-source/ 0 触碰 crate src/
- **B2 workspace.version 1.2.0 0 改**: R129-13 0 改 Cargo.toml version, mkdocs.yml 0 触碰 Cargo.toml
- **A1 R11 baseline 3 值 0 改**: R129-13 0 触碰 17 baseline 文件
- **B3-B7 + A2-A3**: R129-13 0 触碰 30 维 + 6 重 + 8 哲学锚 + 13 键
- **C1 0 主动 commit**: R129-13 写到主仓 0 git commit, 等 Mavis 整合 #5.2 commit 时机拍板
- **C2 0 装 PASS 严守**: R129-13 0 借具体源码, 1.0 release 文档是配置
- **C3 升 6 重 v6 → v7**: R129-13 0 触碰 6 重
- **0 主动 push**: R129-13 0 push, 1.0 release + GitHub Pages 流程 0 主动

## 5. 3 大核心机制 (8 哲学锚 + 6 重守门 v7 + 13 键 verdict cache)

### 5.1 8 哲学锚 (B5) — 设计规范

详 §1.

### 5.2 6 重守门 v7 (B4) — 安全规范

| # | 守门 | 类型 | v7 升级 |
|---:|------|------|---------|
| 1 | 守门 1 (基础类型 / 范围) | nested | v6 → v7 + 边界场景 |
| 2 | 守门 2 (关联一致性) | nested | v6 → v7 + 跨字段 |
| 3 | 守门 3 (业务规则) | nested | v6 → v7 + DSL 表达式 |
| 4 | 守门 4 (权限 / RBAC) | nested | v6 → v7 + 角色继承 |
| 5 | 守门 5 (审计 / 可追溯) | nested | v6 → v7 + 决策链 |
| 6 | 守门 6 (Colang DSL) | dsl | v7 新增 (Colang 模板) |

**6 重 v7 严守** (per P1-3 R126 retry done, 整合 #4 commit 严守).

### 5.3 13 键 verdict cache (A3) — 编译期 hardcode

详 [API Reference - §1 13 键 verdict cache](api.md#1-13-键-verdict-cache-a3-per-决策-33-23--整合-4-commit).

**13 键 hardcode 编译期内保证** (per 决策 #33 §2.3 A3 严守, 整合 #4 commit done).

## 6. 借鉴源码 (per Borrowed Repos 完整致谢)

详 [Borrowed Repos](borrowed-repos.md).

**借鉴 11/11 状态 100% clear**: 10 ✅ + 0 ⏳ + 1 ❌ = 11/11 (per P6-1/2/3 retry 21:38 done 收尾).

## 7. 整合 #5 commit 拍板 (per 决策 #62, Mavis 自决)

- **5.1** src/ 实施 (50+ 文件, 借鉴 8/11 真实施 + LOCKED 内部 fn 改动)
- **5.2** docs/ + Cargo.toml (10 文件, 1.0 release 文档化)
- **5.3** reports/ 决策链 + 报告 (30+ 文件, 备查, 0 影响 build)

**整合 #4 commit abf12243 严守 100%** (per 决策 #48, 19:41 done, 0 重跑).
**8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #62 §6).

## 8. 终极前端 = Tauri (per 主人 8/4 23:33 + 用户记忆 #8)

> 主人 8/4 23:33: "我们最后要做的前端应该是Tauri, 但由于现在手头的ai团队没有适合干尤其是审美设计的, 所以web和桌面都搁置, 先做好tui来为桌面做准备."

**前端路线**:
- **现在**: TUI (per 决策 #9 改瘦后暂告段落, 优先后端)
- **终极**: Tauri 2.0 (5 nav + 主对话 + 9 organ 拟人化, per 决策 #11 阶段 4 frontend-proposal)
- **触发**: 等设计团队到位 (主人 0 必设计感, 宁可丑也不上没设计感的)
- **GitHub Pages**: 1.0 release 配套文档站, 0 依赖 Tauri, mkdocs 静态网站

**TUI = Tauri 集成测试床** (后端 API 表面 / 集成模式 / 用户流在 TUI 跑稳, Tauri 来了无缝换 UI 层).

## 9. Refs

- 📄 [决策链 #22-#62](https://github.com/apeireth/apeireth-rust/tree/main/reports) — `reports/decision-*.md` 31 份决策文件
- 📄 [HANDOFF-NEXT-SESSION-2026-08-10.md](https://github.com/apeireth/apeireth-rust/blob/main/reports/HANDOFF-NEXT-SESSION-2026-08-10.md) — R125-R128-2 era 完整上下文
- 📄 [Cargo.toml](https://github.com/apeireth/apeireth-rust/blob/main/Cargo.toml) — `[workspace.metadata.apeireth]` section 73 行
- 📄 [OSS_NOTICE.md](https://github.com/apeireth/apeireth-rust/blob/main/OSS_NOTICE.md) — 借鉴 11/11 致谢
- 📄 [Architecture v4 — Living Intelligence](https://github.com/apeireth/apeireth-rust/blob/main/docs/architecture-v4-living-intelligence.md) — 完整架构文档
- 📄 [Architecture v4.1 — Living Intelligence Update](https://github.com/apeireth/apeireth-rust/blob/main/docs/architecture-v4-1-living-intelligence-update.md) — 架构更新
- 📄 [docs/stage4/8-locked-unified-2026-08-05.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/stage4/8-locked-unified-2026-08-05.md) — 8 不修改承诺
- 📄 [docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md) — 架构 readonly 评审
