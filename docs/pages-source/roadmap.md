# Roadmap — 1.0 → 2.0

> **完整 ROADMAP**: 见根目录 [`ROADMAP.md`](https://github.com/apeireth/apeireth-rust/blob/main/ROADMAP.md) (P7-2 21:22 写, 28.7KB / 235 行)
> **历史路线图**: 见 [`docs/roadmap/`](https://github.com/apeireth/apeireth-rust/tree/main/docs/roadmap)
> **整合 #4 commit**: `abf12243` (2026-08-10 19:41, 46752 file changes, master HEAD 严守)

---

## 0. TL;DR

| 版本 | 状态 | 周期 | 关键里程碑 |
|------|------|------|-----------|
| **v1.0** | ✅ 已发布 (R125-R127) | 8/10 | 24 LOCKED + 8 哲学锚 + 30 维 V0.5 + 6 重 v7 + 13 键 + 借鉴 8/11 + Library v1.0 |
| **v1.1** | 🟡 准备 (8/11-9/14) | 8/11-9/14 | 借鉴 11/11 (LiteLLM/opencode/Guardrails 3 收尾) + Library Stage 4-6 + Cargo 验证 + 整合 #5 commit + 1.0 release + GitHub Pages |
| **v1.5** | ⏳ 计划 (9-12 月) | 9-12 月 | ASI Python 整合 (R11 baseline 升级) + Tauri 终极前端 prototype + 5 nav crate + StateGraph 4 协议 handler trait 强化 |
| **v2.0** | ⏳ 计划 (2027+) | 2027+ | R128+ 升级 + 正式 1.0 release 路线 + GitHub remote + 终极路线图 |

## 1. v1.0 (R125-R127, ✅ 已发布 8/10)

**v1.0 已发布** (per 整合 #4 commit `abf12243` 19:41 done, 46752 file changes):

### 关键里程碑

- ✅ **整合 #4 commit `abf12243` done** (8/10 19:41, 46752 file changes, 0 M+?? 异常)
- ✅ **24 LOCKED crate 入口签名 0 改** (B1, 12 已知 + 12 Mavis 自主, 整合 #4 commit 严守)
- ✅ **8 哲学锚升级** (B5, 6→8: 增 S-3 流程自化 + O-1 安全优先)
- ✅ **V0.5 25→30 维升级** (B3, P1-4 R126 verify retry done, 60 tests 30 维 sum=1.0)
- ✅ **6 重守门 v6 → v7 升级** (B4, P1-3 R126 retry done, 5 嵌套 + Colang DSL)
- ✅ **13 键 verdict cache** (A3, 12 原 12 + PHL-07 = 13 键, 整合 #4 commit done)
- ✅ **Library v1.0 礼物** (30 经典书 + 100+ 论文 + 50+ 视频 + 10+ 课程 + 10+ hub = 200+ 资源, 9 organ 1:1)
- ✅ **借鉴源码 8/11 ✅ cloned 真实施** (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234, 3 限流重试中, 1 跳过)

### 8 硬墙 0 越界 100%

B1 24 LOCKED 入口签名 0 改 / B2 1.2.0 0 改 / A1 3 值 0 改 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / C3 升 v7 / 0 主动 push.

## 2. v1.1 (8/11-9/14, 🟡 准备中)

**v1.1 准备** (per R129 era 派活规划, decision-61 §3):

### 关键里程碑

- 🟡 **整合 #5 commit 拍板** (per 决策 #62, Mavis 自决, 拆 3 commit 5.1 + 5.2 + 5.3, 等 R129-3 8 步 verify done)
- 🟡 **1.0 release 配套** (per R129-8 scripts/release/ 10 文件 done 0:38 + R129-13 GitHub Pages 准备 done 01:00)
- 🟡 **借鉴 3/11 收尾** (LiteLLM P6-1 21:38 done + opencode P6-2 done + Guardrails P6-3 done → 11/11 全状态 clear, ❌ 1 跳过)
- 🟡 **ASI Python Stage 4-6 整合** (per R129-4/5/6 跑过夜 8/11-8/22)
- 🟡 **Tauri 终极前端 Stage 2 深化** (per R129-9 跑过夜 8/11-8/22)
- 🟡 **形式化证明扩展 Stage 5.2** (per R129-10 续 P8-2 跑过夜 8/11-8/22)
- 🟡 **R129 era 决策链更新** (per R129-16)
- 🟡 **TUI 升级路线图沉淀** (per R129-15 + 决策 #9 改瘦后暂告段落)

### 决策链更新

- **决策 #61**: 新会话接手 + R129 era 派活规划 (主人 0:03 最高授权)
- **决策 #62**: 整合 #5 commit 拆 3 commit 拍板 (Mavis 自决)
- **决策 #63-#67**: R129 era 后续 (待写)

## 3. v1.5 (9-12 月, ⏳ 计划)

**v1.5 计划** (per ROADMAP.md P7-2 阶段 v1.5):

### 关键里程碑

- ⏳ **ASI Python 整合深化** (R11 baseline 升级, 跨语言桥强化)
- ⏳ **Tauri 终极前端 prototype** (P11-1/2 + R129-9, 5 nav + 主对话 + 9 organ 拟人化)
- ⏳ **5 nav crate 新增** (per 决策 #11 阶段 4 frontend-proposal)
- ⏳ **StateGraph 4 协议 handler trait 强化** (langgraph 829 深化)
- ⏳ **6 重守门 v7 → v8 升级** (B4 持续)
- ⏳ **30 维 V0.5 → 40 维 V1.0 升级** (B3 持续)

### 5 nav 设计 (per 决策 #11 阶段 4 frontend-proposal)

1. **Home** — 状态主页 (9 organ 拟人化)
2. **Chat** — 主对话 (single AI conversation)
3. **History** — 历史记录
4. **Tools** — 工具集
5. **Settings** — 设置


## 3.5 R128 大规模合并 (2026-08-12, ✅ 落实待 commit)

**R128 实际执行 (per [conventions/16-crate-merge-policy.md](../conventions/16-crate-merge-policy.md))**:

- ✅ **13 frozen crate** 移至 crates/_frozen/ (credentials / cache / tracing / metrics / oauth / update / sandbox / tree-sitter / i18n[recovery] / image-prompt / plugin / observability / task)
- ✅ **5 merge 源 crate** 移至 crates/_archived/ (rollback → upgrade / keyring + machine-id → host / repo-scan + repo-analyzer → repo-tools)
- ✅ **peireth-host 新 crate** = peireth-keyring + peireth-machine-id (5 子模块)
- ✅ **peireth-repo-tools 新 crate** = peireth-repo-scan + peireth-repo-analyzer
- ✅ **peireth-upgrade::rollback** 子模块合并
- ✅ **peireth-integration-r20-stage4** superseded by peireth-integration-e2e → crates/_archived/
- ✅ **调用方迁移**: TUI / API / SDK-{sandbox,lark,livekit,voice} 全部切到新路径
- ✅ **cargo check --workspace 0 errors** (296 historical warnings, 0 effect)
- ✅ **24 LOCKED 入口签名冻结降级** (per 主人 8/11 22:31 拍板),仅保 3 项不可变脊柱

**workspace 收敛**: 74 active + 18 archived/frozen (从原 ~94 收敛)

**核验**: cargo check --workspace exit 0,23.23s 完成
## 4. v2.0 (2027+, ⏳ 计划)

**v2.0 计划** (per ROADMAP.md P7-2 阶段 v2.0):

### 关键里程碑

- ⏳ **R128+ 升级** (R128 era 后端加固)
- ⏳ **正式 1.0 release 路线** (per 主人 8/4 23:33 + 决策 #9)
- ⏳ **GitHub remote 公开** (1.0 release 配 GitHub, per R129-8 setup-github-remote)
- ⏳ **终极路线图** (per R129-12 路线图)
- ⏳ **Tauri 终极前端 v1** (等设计团队到位, 主人 0 必设计感)
- ⏳ **TUI = Tauri 集成测试床** (后端 API 表面 / 集成模式 / 用户流在 TUI 跑稳, Tauri 来了无缝换 UI 层)

## 5. 决策链 (#22-#62, per ROADMAP.md 引用)

| 决策 | Date | 关键内容 |
|------|------|---------|
| #9 | 8/4 | TUI 改瘦后暂告段落, 优先后端 |
| #22 | 8/10 | workspace.version 1.2.0 严守 + 24 LOCKED 自主确认 |
| #33 | 8/10 | master-reupgrade (8 硬墙) |
| #48 | 8/10 | 整合 #4 commit abf12243 done |
| #55 | 8/10 | R127 4 派活 (整合 #5 pre-check + Library Stage 4-6 + 1.0 release 准备) |
| #57 | 8/10 | R128 6 派活 (ASI Python + Tauri + Cargo + LICENSE) |
| #58 | 8/10 | R128-2 3 派活 (ASI Stage 3 + Tauri scaffold + 1.0 release Cargo 配) |
| #61 | 8/11 | 新会话接手 + R129 era 派活规划 |
| #62 | 8/11 | 整合 #5 commit 拆 3 commit 拍板 |

完整决策链见 [`reports/decision-*.md`](https://github.com/apeireth/apeireth-rust/tree/main/reports) (R125 era → R128-2 era, 31 份决策文件).

## 6. Refs

- 📄 [ROADMAP.md](https://github.com/apeireth/apeireth-rust/blob/main/ROADMAP.md) — 完整路线图 (P7-2 21:22 写, 28.7KB / 235 行)
- 📄 [docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md)
- 📄 [docs/roadmap/v1.0-released-r125-r127-2026-08-10.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/roadmap/v1.0-released-r125-r127-2026-08-10.md)
- 📄 [docs/roadmap/r20-product-finalize-2026-08-05.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/roadmap/r20-product-finalize-2026-08-05.md)
- 📄 [docs/v2-strategy/00-VISION.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/v2-strategy/00-VISION.md) — v2 战略
- 📄 [docs/v2-strategy/05-EXECUTION-NOW.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/v2-strategy/05-EXECUTION-NOW.md) — v2 执行
