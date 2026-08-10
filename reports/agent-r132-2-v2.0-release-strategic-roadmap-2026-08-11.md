# R132-2 V2.0 release 战略路线图 (V1.1 release 之后的下一个 major release, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级)

**Date**: 2026-08-11 02:00+ (R132-2 sub-agent, Mavis 派, R132 era 战略规划阶段)
**Author**: R132-2 sub-agent (mvs_367e66fa session, **0 改 src/**, **0 改 Cargo.toml**, 0 主动 commit, 0 主动 push, 0 借具体源码, 0 装 PASS 严守)
**触发**: 决策 #71 §4 (R130 era 调研 → 差距 → 计划 → 继续干 永久循环 4 步, R132 era 进入"计划"阶段) + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #75 整合 #5 commit 时机 NOT ready 等 R130-1 修 bug + cron `watch-r129-era-auto-replenish-16` Section 2 派 R132-2 (60 min 时间盒, 8/11 02:00+ 派)
**任务**: **V2.0 release 战略路线图** (V1.1 release 之后的下一个 major release, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级), 严格 0 改 src 路线图阶段, 只写报告
**关联**:
- decision-9 (TUI 升级节奏: 改瘦后暂告段落, 优先后端) + decision-10 (主人离场 Mavis 自主决策) + decision-22 (24 LOCKED 自主确认) + decision-33 (8 硬墙 + 0 装 PASS) + decision-48 (整合 #4 commit abf12243) + decision-55 (R127 4 派活) + decision-56 (R127-2 10 派活) + decision-57 (R128 6 派活) + decision-58 (R128-2 3 派活) + decision-61 (新会话接手 + R129 era 派活规划) + decision-62 (整合 #5 commit 拆 3 commit 拍板) + decision-71 (R130 era 自动接续 4 步: 调研 + 差距 + 计划 + 继续干 永久循环) + decision-72 (R130 era 调研 6 sub-agent 派活) + decision-73 (主人 8/11 01:14 拍板 3 件套) + decision-74 (8 硬墙 B1 改写, V2.0 release 8 硬墙可重评) + decision-75 (整合 #5 commit 时机 NOT ready 等 R130-1 修 bug)
- R129-12 (R129 era 战略路线图) + R129-17 (R130 era 路线图详细) + R129-26 (R129 era 健康度 verify, 暴露 24+5+1 errors) + R129-27 (R129 era 1.0 release 流程实战终态) + R129-29 (R130 era 路线图 final, 含 V1.1 §4 + V1.2 §5 详细) + R130-5 (V1.1 minor release 路线图) + R130-6 (借鉴源 12 源调研, OpenCog AGPL-3.0 fork 决策) + R131-1 (V1.1 release 路线图 final, 派中估 done) + R131-2 (借鉴 12 源差距分析, 派中估 done) + R131-3 (V1.1 release 实施路线图, 派中估 done)
- 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备" + 8/4 23:55 "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui" + 8/6 01:14 "后面有需要决定的都按你想法倾向来, 最终收尾的时候把你的想法决策也都记录下来就行" + 8/10 16:31 拍板 "全部采纳, 全都能动, 需要具体确认的你自己确认就行, 你有最高权限" + 8/11 01:14 拍板 3 件套 "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板" + "不要怕复杂度"
- 用户记忆 #3 (用户看结果不看哲学) + #4 (AI 不会衰老病死, 只成长) + #5 (信息密度高 = 拟人化 + 拟物化) + #6 (派 sub-agent 干, 但要驾驭团队不重复造轮子) + #7 (推技术决策要守规范, 但要诚实, 砍掉装饰/无业务价值) + #8 (TUI → Tauri 终极路线) + #9 (TUI 升级节奏) + #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
- `docs/conventions/15-no-fear-complexity.md` (决策 #73 §3 主人 8/11 01:14 总哲学扩展)
- `docs/conventions/10-locked.md` (R119-3a-1 8 项形式撤销, 原意保留) + `docs/conventions/09-anchor.md` (8 哲学锚 S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)
- R119-2 思想层保留 (V2.0 终极路线图: 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作) + ROADMAP.md §4
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守)
**整合 #5 commit**: per decision-62 拆 3 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板, 8 项 verify 100% 后拍板, **当前 6/8 verify PARTIAL/FAIL (per R129-26 暴露 24+5+1 errors), 等 R130-1 修 30+1 src bug 后 8/8 verify 100% 才拍板**
**V1.0 release tag**: 估 8/11 (整合 #5 commit 拍板后, 主人起床后手跑 scripts/release/ 7 步 runbook, per R130-5)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`, per R130-5 + R131-3)
**V2.0 release tag**: 估 2027+ 远期 (per 决策 #71 §4 永久循环 + ROADMAP.md §4 + R119-2 思想层保留)
**状态**: ✅ done (R132-2 V2.0 release 战略路线图, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 不重写 R130-5 + R129-29 §4-§5, 拓维)

---

## 0. 一句话 (TL;DR)

**V2.0 release = V1.1 release (估 2026-11-30) 之后的下一个 major release (per semver `2.0.0` major bump, 估 2027+ 远期, 决策 #71 §4 永久循环 4 步 + ROADMAP.md §4 + R119-2 思想层保留), 8 大方向 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2.2 更好的架构 + R130-6 借鉴源 12 源调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md 复杂不恐惧哲学)**: ① **8 硬墙 可重评** (B1 24 LOCKED 入口签名 推翻 + 重建 [24 → 0/12/24/36/...] + B2 workspace.version 1.2.1 → 2.0.0 major bump + A1 R11 baseline 3 值 可重评 [新 baseline, 跟 R12 测度对齐] + A3 12 键 + PHL-07 可重评 [12 → 13 → 14/0/...] + B3 V0.5 30 维 可重评 [30 → 0/40/...] + B4 6 重守门 v7 可重评 [6 → 0/10/...] + B5 8 哲学锚 可重评 [**核心变化**, 8 → 0/12/...] + C1 0 主动 commit 可重评 [Mavis 自动 commit + push] + C2 0 装 PASS 可重评 [允许装特定包]) ② **8 哲学锚 可重建** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 → 0 锚 [无哲学] / 12 锚 [扩展] / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统]) ③ **Cargo workspace 可重构** (当前 30+ crate → 24 LOCKED 入口重构 [12 module + 24 micro-crate, 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex 实施 AGPL-3.0 fork, per 决策 #73 §2.2 + R130-6 调研]) ④ **三洋葱架构升级** (当前 原则 + 权限 + DSL → 四洋葱 [+ 智能涌现] / 五洋葱 [+ 自我演化] / 全新架构) ⑤ **9 organ 代码升级** (当前 body / brain / ear / eye / hand / heart / memory / mind / voice → 12 organ [+ 涌现 / 自演化 / 群体] / 全新架构) ⑥ **ASI Stage 10 终极自治** (当前 ASI Stage 9 长程 AI 成长 → ASI Stage 10 终极自治 [V2.0 release 核心, 借脑 OpenCog / CogPrime + ASI Stage 1-9 整合 + 长程 AI 成长平台]) ⑦ **Tauri 3.0+ 升级** (当前 Tauri 2.0 + 5 nav + 9 organ 拟人化 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试) ⑧ **永久循环** (V2.0 release → V2.1 minor → V3.0 major → ... [永久演化, per 决策 #71 §4 + 不要怕复杂度哲学]). **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2, V2.0 release 0 装严守可重评, 但默认仍 0 装, 仅特定场景允许, e.g. OpenCog AGPL-3.0 fork fork-then-borrow 模式) + **8 硬墙 0 越界 100%** (V2.0 release 8 硬墙可重评, 但 V1.0 release 严守 100% + V1.1 release B1 可改, 0 主动 IM 主人严守) + **不要怕复杂度哲学严守** (per 15-no-fear-complexity.md, 复杂度是实力的体现, 维护交给未来高水平团队) + **Cargo.toml 2.0.0 major bump 严守 semver** (per 决策 #22 §2.2, major bump 表示 breaking change, 24 LOCKED 入口签名可改, 0 装 PASS 严守可松绑). V2.0 release 跟 V1.0 / V1.1 release 边界清晰: V1.0 release = 0 改 src 严守 (R11 baseline) + 8 哲学锚 + Cargo.toml 1.2.0, V1.1 release = 24 LOCKED 入口签名可改 (前提: 更好的架构) + PHL-07 实施 + Cargo.toml 1.2.1, V2.0 release = 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + Cargo.toml 2.0.0. R132 era 派活规划 (V2.0 release 战略路线图, per 决策 #71 §4): R132-1 V1.1 release 路线图 final (派中估 done) + **R132-2 V2.0 release 战略路线图 (本任务)** + R132-3+ 派活等 R131 era 部分 done 后 (per 16 跑中上限严守 + 不要怕复杂度).

---

## 1. V2.0 release 战略总览 (V1.1 release 之后的下一个 major release era)

### 1.1 V2.0 release 定位 (per 决策 #71 §4 + 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 主人 8/11 01:14 拍板 3 件套 + R119-2 思想层保留 + ROADMAP.md §4)

**V2.0 release = V1.1 release (估 2026-11-30) 之后的下一个 major release (per semver 严守 2.0.0 major bump, 估 2027+ 远期)**:
- **起点**: V1.1 release tag v1.1.0 打上 (per R130-5 7 步 runbook 续, 估 2026-11-30, 主人起床后手跑)
- **终点**: V2.0 release tag v2.0.0 打上 (per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + ROADMAP.md §4 + R119-2 思想层保留, 估 2027+ 远期, 1-3 月时间窗)
- **核心任务**: 8 大方向 (per 决策 #74 §2.3 + 决策 #73 §2.2 + 决策 #73 §3 + R130-6 调研 + 15-no-fear-complexity.md):
  1. **8 硬墙可重评** (per 决策 #74 §2.3, 推翻 + 重建 8 硬墙, 8 哲学锚核心变化)
  2. **8 哲学锚可重建** (per 决策 #74 §2.3 + 决策 #73 §3, 0 锚 / 12 锚 / 全新架构)
  3. **Cargo workspace 可重构** (per 决策 #73 §2.2 + R130-6 调研 OpenCog AGPL-3.0 fork 决策, 24 LOCKED 入口 → 12 module + 24 micro-crate)
  4. **三洋葱架构升级** (per 决策 #73 §2.2, 原则 + 权限 + DSL → + 智能涌现 / + 自我演化)
  5. **9 organ 代码升级** (per 决策 #73 §2.2, 9 organ → 12 organ / 全新架构)
  6. **ASI Stage 10 终极自治** (per 决策 #73 §2.2 + R130-2 调研 Stage 9 路线, ASI Stage 10 终极自治 + 长程 AI 成长 + 平台化)
  7. **Tauri 3.0+ 升级** (per 决策 #73 §2.2, Tauri 2.0 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试)
  8. **永久循环** (per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学, V2.0 release → V2.1 minor → V3.0 major → ...)

**V2.0 release = major release (semver 严守 2.0.0, per 决策 #22 §2.2)**:
- **major bump**: V1.1.x → V2.0.0 表示 **breaking change** (24 LOCKED 入口签名可改, 8 硬墙可重评, 8 哲学锚可重建, Cargo workspace 可重构)
- **8 哲学锚 推翻 + 重建** (per 决策 #74 §2.3, V2.0 release 核心变化)
- **Cargo.toml 2.0.0** (per 决策 #22 §2.2 semver 严守, workspace.version 1.2.1 → 2.0.0 major bump)
- **OpenCog AGPL-3.0 fork 实施** (per 决策 #73 §2.2 + R130-6 调研, V2.0 release 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex, AGPL-3.0 license 兼容)
- **Tauri 3.0** (per 决策 #73 §2.2, 如果 2027+ 出, V2.0 release 升级)
- **ASI Stage 10 终极自治** (per 决策 #73 §2.2, V2.0 release 核心, 长程 AI 成长 + 平台化 + 真用户 + 多 AI 平台)

**V2.0 release 跟 V1.0 / V1.1 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #22 §2.2 semver)**:
- **V1.0 release** (估 8/11): 0 改 src 严守 (R11 baseline) + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml 1.2.0 → 1.0.0 + PHL-07 spec-only
- **V1.1 release** (估 2026-11-30): 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml 1.0.0 → 1.1.0
- **V2.0 release** (估 2027+ 远期): 8 硬墙 可重评 + 8 哲学锚 可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + Cargo.toml 1.1.0 → 2.0.0

### 1.2 V2.0 release 时间线 (per 决策 #71 §4 永久循环 + 决策 #22 §2.2 semver + 决策 #74 §2.3 + R130-5 V1.1 + R129-29 §4-§5 + 主人 8/11 01:14 拍板 3 件套)

```
[8/11 01:00+ 整合 #5 commit 拍板]   Mavis 自决 (5.1 → 5.2 → 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup, 等 R130-1 修 30+1 src bug)
[8/11 06:00-08:00 主人起床 1.0 release 实战]   主人手跑 R130-5 [R129-35 final-final 7 步 runbook] 7 步流程
[8/11 08:00+ 1.0 release done]    master HEAD = abf12243 + 3 commit (5.1/5.2/5.3), v1.0.0 tag, GitHub release, GitHub Pages 部署
[8/11 08:00+ R130 era 跑过夜]      R130-1~7 7 sub-agent 跑过夜 (后端 verify 修 bug [关键] + ASI 整合 + Tauri 深化 + 形式化 + V1.1 路线图 + TUI 升级 + 总览)
[8/11 08:00+ R131 era 差距]       per 决策 #71 §3 调研 → 差距, 派 R131 era 3 sub-agent (R131-1 + R131-2 + R131-3)
[8/12 R131 era 差距 done]         3 sub-agent 全 done, 决策链 #76+#77+#78 写
[8/12+ R132 era 计划]             per 决策 #71 §4 调研 + 差距 → 计划, 派 R132 era 2 sub-agent (R132-1 V1.1 release 路线图 final + **R132-2 V2.0 release 战略路线图 (本任务)**)
[9-10 月 R131 era 实施]            实施 R131 era 路线图 (TUI 升级 + Tauri Stage 4 + ASI Stage 7 + 形式化 Stage 5.4 + 后端 Stage 4-6 续, per R131-3 实施路线图)
[11 月 R131 era 总览 + 整合 #6 commit 拍板]   整合 #6 commit 拍板 (Mavis 自决, 5.1/5.2/5.3 顺序)
[11/30 06:00-08:00 主人起床 V1.1 release 实战]   主人手跑 V1.1 release 7 步 runbook (8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署)
[12 月 V1.1 release 后]           V1.2 路线图 (per R129-29 §5, 估 2027-02-28, 6 维度: TUI 阶段 3 + Tauri Stage 5 + ASI Stage 8 + 形式化 Stage 5.5 + 后端 Stage 7-8 续 + V1.2 release 实战)
[2027+ V2.0 release 战略规划]       per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + ROADMAP.md §4 + R119-2 思想层保留
[2027+ V2.0 release 调研 + 差距]   V2.0 release 调研 (8 硬墙可重评方案 + 8 哲学锚可重建方案 + Cargo workspace 可重构方案 + 三洋葱架构升级方案 + 9 organ 升级方案 + ASI Stage 10 终极自治方案 + Tauri 3.0+ 升级方案) + 差距分析
[2027+ V2.0 release 计划]         V2.0 release 计划 (per R132-2 战略路线图, 8 大方向详细 spec + 派活规划 + 时间盒 + 资源)
[2027+ V2.0 release 实施]         V2.0 release 实施 (per V2.0 release 计划, 派 30+ sub-agent, 6 方向 × 5-10 sub-agent)
[2027+ V2.0 release 实战]         V2.0 release tag v2.0.0 打上 (per 7 步 runbook, 主人起床后手跑, GitHub remote 已配)
[2027+ V2.0 release 后]            V2.1 minor 路线图 → V3.0 major → ... (永久循环, per 决策 #71 §4)
```

**时间窗口总结 (per 决策 #22 §2.2 + 决策 #71 §4 + R130-5 + R129-29 §4-§5)**:
- **V1.0 release** (估 8/11): 整合 #5 commit 拍板后, 主人起床后手跑
- **V1.1 release** (估 2026-11-30): V1.0 release 后 ~3.5 个月 (per R130-5 + R129-29 §4.1)
- **V1.2 release** (估 2027-02-28): V1.1 release 后 ~3 个月 (per R129-29 §5.1)
- **V2.0 release** (估 2027+ 远期, 1-3 月时间窗): V1.1 release 后 ~3-12 个月 (per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release + ROADMAP.md §4)
- **V2.1 release** (估 2027+ 远期, 1-3 月后): V2.0 release 后 ~1-3 个月 (per 永久循环)
- **V3.0 release** (估 2028+ 远期): V2.0 release 后 ~3-12 个月 (per 永久循环)

### 1.3 V2.0 release 跟 R125-R131 era + V1.0/V1.1 release + 永久循环的接力

| Era | 时间 | 状态 | 核心任务 | 决策链 |
|-----|------|------|---------|--------|
| **R125 era** | 8/10 14:00-17:22 | ✅ done (16 sub-agent) | 借鉴 8/11 ✅ cloned + 41 任务起步 | #30-#41 |
| **R126 era** | 8/10 17:22-21:00 | ✅ done (16 sub-agent) | 后端升级 + 8 哲学锚 + 30 维 + 6 重 v7 + Library v1.0 礼物 | #33 + #51-#54 |
| **R127 era** | 8/10 21:00-22:00 | ✅ done (4 sub-agent) | Library Stage 4-6 + 整合 #5 pre-check | #55 |
| **R127-2 era** | 8/10 22:00-22:30 | ✅ done (10 sub-agent) | 借鉴 3 限流重试 + 1.0 release 文档 + 形式化证明 | #56 |
| **R128 era** | 8/10 22:30-23:00 | ✅ done (6 sub-agent) | ASI Python Stage 1-2 + Tauri prototype + Cargo 实战 + LICENSE + 整合 #5 pre-stage | #57 |
| **R128-2 era** | 8/10 23:00-22:50 | ✅ done (3 sub-agent) | ASI Python Stage 3 + Tauri scaffold 深化 + Cargo 配 | #58 |
| **整合 #4 commit** | 8/10 19:41 | ✅ done | master HEAD = abf12243 严守 100% | #48 |
| **R129 era** | 8/11 00:08-01:00+ | ✅ 35 done | 整合 #5 commit 准备 + ASI Stage 4-6 续 + 1.0 release 流程 + 形式化扩展 + TUI/Tauri 路线图 + R130 路线图 + 健康度 verify | #61-#68 |
| **R130 era** | 8/11 整合 #5 commit 拍板后 → 主人起床 | 🟡 6 派 (R130-1 后端 修 bug 关键 + R130-2 ASI + R130-3 Tauri + R130-4 形式化 + R130-6 TUI + R130-7 总览) + R130-5 1.0 release 实战待主人起床 | #70-#78 |
| **1.0 release 实战** | 主人起床后 06:00-08:00 | 📋 主人手跑 R130-5 7 步 runbook | 8 步 verify + GitHub remote + git push + 1.0 release tag + GitHub Pages | #77 |
| **1.0 release 后** | 8/11 08:00+ | 📋 远期 | V1.1 + V1.2 + V2.0 路线图 (per 决策 #71 §4) | #79+ |
| **R131 era (V1.1 差距)** | 8/11+ | 🟡 3 派 (R131-1 V1.1 路线图 final + R131-2 借鉴 12 源差距 + R131-3 V1.1 实施路线图) | 调研 + 差距 (per 决策 #71 §3) | #76-#78 |
| **R132 era (V1.1 计划 + V2.0 战略)** | 8/11+ 02:00+ | 🟡 2 派 (R132-1 V1.1 路线图 final [派中估 done] + **R132-2 V2.0 release 战略路线图 [本任务]**) | 计划 (per 决策 #71 §4) | #79+ |
| **R133 era (V1.1 实施)** | 9-10 月 2026 | 📋 计划中 | V1.1 release 实施 (per R131-3 实施路线图) | #80+ |
| **R134 era (V1.1 整合)** | 11 月 2026 | 📋 计划中 | 整合 #6 commit 拍板 (Mavis 自决) | #81+ |
| **V1.1 release 实战** | 11/30 06:00-08:00 | 📋 主人手跑 V1.1 release 7 步 runbook | 8 步 verify + git push + v1.1.0 tag + GitHub Pages | (per R131-3) |
| **R135 era (V1.2 计划)** | 12 月 2026 | 📋 计划中 | V1.2 路线图 (per R129-29 §5) | #82+ |
| **V1.2 release** | 估 2027-02-28 | 📋 远期 | v1.2.0 tag 打上 (per R129-29 §5) | (per R129-29 §5) |
| **V2.0 release 战略** | 2027+ | 📋 远期 (per **本报告**, 8 大方向) | 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + 永久循环 | (per 决策 #74 §2.3 + 决策 #73 §2.2 + 15-no-fear-complexity.md) |
| **V2.0 release 调研** | 2027+ 估 1-3 月 | 📋 远期 | V2.0 release 调研 (8 大方向详细) | (per 决策 #71 §3) |
| **V2.0 release 差距** | 2027+ 估 1-3 月 | 📋 远期 | V2.0 release 差距分析 | (per 决策 #71 §3) |
| **V2.0 release 计划** | 2027+ 估 1-3 月 | 📋 远期 | V2.0 release 计划 (per R132-2 战略路线图, 8 大方向详细 spec) | (per 决策 #71 §4) |
| **V2.0 release 实施** | 2027+ 估 3-12 月 | 📋 远期 | V2.0 release 实施 (per V2.0 release 计划, 30+ sub-agent) | (per 决策 #71 §4) |
| **V2.0 release 实战** | 2027+ 估 1-3 月 | 📋 远期 | V2.0 release tag v2.0.0 打上 (per 7 步 runbook) | (per 决策 #71 §4) |
| **V2.1 minor release** | 2027+ 估 1-3 月后 | 📋 远期 | V2.1 minor 路线图 (per V2.0 release 后) | (永久循环) |
| **V3.0 major release** | 2028+ 估 3-12 月后 | 📋 远期 | V3.0 major 路线图 (per V2.0 release 后) | (永久循环) |

### 1.4 V2.0 release 8 大方向 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 决策 #73 §2.2 + 15-no-fear-complexity.md + R130-6 调研)

**V2.0 release 8 大方向 (per R132-2 调研 + 用户任务描述 + 决策 #74 §2.3)**:

| # | 方向 | 子任务核心 | 调研依据 | 状态 |
|---|------|----------|---------|------|
| **1** | **8 硬墙可重评** | B1 24 LOCKED 入口签名 推翻 + 重建 (24 → 0/12/24/36/...) + B2 workspace.version 1.2.1 → 2.0.0 + A1 R11 baseline 3 值 可重评 + A3 12 键 + PHL-07 可重评 (12 → 13 → 14/0/...) + B3 V0.5 30 维 可重评 (30 → 0/40/...) + B4 6 重守门 v7 可重评 (6 → 0/10/...) + B5 8 哲学锚 可重评 (**核心变化**) + C1 0 主动 commit 可重评 (Mavis 自动 commit + push) + C2 0 装 PASS 可重评 (允许装特定包) + 0 push 可重评 (Mavis 自动 push) | 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md | 📋 V2.0 release 必重评 |
| **2** | **8 哲学锚可重建** | S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 → 0 锚 (无哲学) / 12 锚 (扩展) / 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统) | 决策 #74 §2.3 + 决策 #73 §3 + 15-no-fear-complexity.md | 📋 V2.0 release 必重建 |
| **3** | **Cargo workspace 可重构** | 当前 30+ crate → V2.0 release 重构 (e.g. ASI Stage 10 整合 24 LOCKED → 12 module + 24 micro-crate) + 借脑 OpenCog AGPL-3.0 fork 实施 (per 决策 #73 §2.2 + R130-6 调研) + 借脑 CogPrime 实施 (AGPL-3.0) | 决策 #73 §2.2 + 决策 #74 §2.3 + R130-6 调研 | 📋 V2.0 release 必重构 |
| **4** | **三洋葱架构升级** | 当前三洋葱: 原则 + 权限 + DSL → V2.0 release 升级: 四洋葱 (原则 + 权限 + DSL + 智能涌现) / 五洋葱 (+ 自我演化) / 全新架构 | 决策 #73 §2.2 + 15-no-fear-complexity.md | 📋 V2.0 release 必升级 |
| **5** | **9 organ 代码升级** | 当前 9 organ: body / brain / ear / eye / hand / heart / memory / mind / voice → V2.0 release 升级: 12 organ (+ 涌现 / 自演化 / 群体) / 全新架构 | 决策 #73 §2.2 + 用户记忆 #4-#5 | 📋 V2.0 release 必升级 |
| **6** | **ASI Stage 10 终极自治** | ASI Stage 9 长程 AI 成长 → ASI Stage 10 终极自治 (V2.0 release 核心) + 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex + 借脑 ASI Stage 1-9 整合 + 长程 AI 成长平台 (V2.0 release 核心) | 决策 #73 §2.2 + R130-2 调研 Stage 9 路线 + R130-6 调研 | 📋 V2.0 release 必实施 |
| **7** | **Tauri 3.0+ 升级** | 当前 Tauri 2.0 + 5 nav + 9 organ 拟人化 → V2.0 release 升级: Tauri 3.0 (如果出) + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试 | 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 | 📋 V2.0 release 必升级 |
| **8** | **永久循环** | V2.0 release → V2.1 minor → V3.0 major → ... (永久演化, per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学 + 自然会有高水平团队接手维护) | 决策 #71 §4 + 15-no-fear-complexity.md | 📋 V2.0 release 必循环 |

**R132 era 派活规划 (V2.0 release 战略路线图, per 决策 #71 §4)**:
- **R132-1**: V1.1 release 路线图 final (派中估 done, per 决策 #71 §4 R132 era 计划阶段)
- **R132-2**: **V2.0 release 战略路线图 (本任务, 0 改 src, 0 改 Cargo.toml, 60 min 时间盒)**
- **R132-3+**: V2.0 release 调研 派活等 R131 era 部分 done (per 16 跑中上限严守 + 不要怕复杂度)

**总时间盒 (R132 era 计划阶段)**: R132 era 计划阶段 5-10 sub-agent × 平均 30-60 min = 150-600 min (估跑 1-2 天, 2 批派满 16 上限, per 决策 #71 §4).

---

## 2. V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3 + 决策 #33 §2.3 + 主人 8/11 01:14 拍板 3 件套)

### 2.1 8 硬墙分类 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

**8 硬墙 (per 决策 #33 §2.3)**:
- **B1**: 24 LOCKED 入口签名
- **B2**: workspace.version 1.2.0
- **A1**: R11 baseline 3 值 (0.8682 / 0.8532 / 0.9063)
- **A3**: 12 键 + PHL-07
- **B3**: V0.5 30 维
- **B4**: 6 重守门 v7
- **B5**: 8 哲学锚
- **C1**: 0 主动 commit (主人起床前)
- **C2**: 0 装 PASS 严守
- **0 push**: 0 主动 push (主人起床前)

**8 硬墙分类 (per 决策 #74 §3)**:
- **工程类 + 技术类** (松绑, B1 改写):
  - B1 24 LOCKED 入口签名: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- **哲学 + 思想类** (严守, 不松绑):
  - A1 R11 baseline 3 值
  - A3 12 键 + PHL-07
  - B3 V0.5 30 维
  - B4 6 重守门 v7
  - B5 8 哲学锚
- **状态 + 流程类** (严守, 不松绑):
  - B2 workspace.version 1.2.0
  - C1 0 主动 commit
  - C2 0 装 PASS 严守
  - 0 push

### 2.2 V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3)

**V2.0 release 8 硬墙 可重评 (per 决策 #74 §2.3 详细)**:

| # | 8 硬墙 | V1.0 release 严守 (per 决策 #33 §2.3) | V1.1 release 改写 (per 决策 #74 §1) | **V2.0 release 可重评 (per 决策 #74 §2.3)** |
|---|--------|---------------------------|------------------------|------------------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构) | 🟢🟢 **V2.0 release 可重评 (推翻 + 重建, 24 → 0/12/24/36/...)** |
| **B2** | **workspace.version** | 🔒 1.2.0 严守 (V1.0 release) | 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 | 🟢🟢 **V2.0 release 1.2.1 → 2.0.0 major bump (semver 严守, breaking change)** |
| **A1** | **R11 baseline 3 值** | 🔒 0 改 (R11 baseline) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (新 baseline, 跟 R12 测度对齐)** |
| **A3** | **12 键 + PHL-07** | 🔒 12 键 + PHL-07 严守 (PHL-07 spec-only) | 🔒 PHL-07 V1.0 spec-only + V1.1 实施 + 12 键其他可改 | 🟢🟢 **V2.0 release 可重评 (12 → 13 → 14/0/...)** |
| **B3** | **V0.5 30 维** | 🔒 30 维严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (30 → 0/40/...)** |
| **B4** | **6 重守门 v7** | 🔒 6 重严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (6 → 0/10/...)** |
| **B5** | **8 哲学锚** | 🔒 8 锚严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (**核心变化**, 8 → 0/12/...)** |
| **C1** | **0 主动 commit** | 🔒 0 主动 commit 严守 (主人起床前) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (Mavis 自动 commit + push, 主人起床后 0 主动)** |
| **C2** | **0 装 PASS 严守** | 🔒 0 装严守 (技术哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (允许装特定包, e.g. OpenCog AGPL-3.0 fork 实施)** |
| **0 push** | **0 主动 push** | 🔒 0 主动 push 严守 (主人起床前) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (Mavis 自动 push, 主人起床后 0 主动)** |

**核心变化 (per 决策 #74 §2.3)**:
- **B5 8 哲学锚 可重评 (核心变化)**: V2.0 release 8 哲学锚可重评, 0 锚 (无哲学) / 12 锚 (扩展) / 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)
- **B1 24 LOCKED 入口签名 可重评 (推翻 + 重建)**: V2.0 release 24 LOCKED 入口签名可推翻 + 重建, 24 → 0/12/24/36/...
- **B2 workspace.version 1.2.1 → 2.0.0 major bump (semver 严守)**: V2.0 release major bump 表示 breaking change
- **C1 + 0 push 可重评 (Mavis 自动 commit + push)**: V2.0 release Mavis 自动 commit + push, 主人起床后 0 主动
- **C2 0 装 PASS 可重评 (允许装特定包)**: V2.0 release 允许装特定包, e.g. OpenCog AGPL-3.0 fork 实施 (per 决策 #73 §2.2 + R130-6 调研)

### 2.3 V2.0 release 8 硬墙重评边界 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)

**V2.0 release 8 硬墙重评边界**:

| 8 硬墙 | V2.0 release 重评边界 | 决策依据 |
|--------|--------------------|---------|
| **B1** | 24 LOCKED 入口签名 可推翻 + 重建 (24 → 0/12/24/36/...) | 决策 #74 §2.3 + 决策 #73 §1 "工程类 + 技术类 locked 全早解锁" + 决策 #73 §2.2 "Mavis 自决架构拍板" |
| **B2** | workspace.version 1.2.1 → 2.0.0 major bump (semver 严守, breaking change) | 决策 #22 §2.2 semver 严守 + 决策 #74 §2.3 |
| **A1** | R11 baseline 3 值 可重评 (新 baseline, 跟 R12 测度对齐) | 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + R125 B3 + R127 25 维公式 |
| **A3** | 12 键 + PHL-07 可重评 (12 → 13 → 14/0/...) | 决策 #74 §2.3 + 决策 #33 §2.3 A3 升级 + R125-12 P0-3 |
| **B3** | V0.5 30 维 可重评 (30 → 0/40/...) | 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + 15-no-fear-complexity.md |
| **B4** | 6 重守门 v7 可重评 (6 → 0/10/...) | 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + 15-no-fear-complexity.md |
| **B5** | **8 哲学锚 可重评 (核心变化)** (8 → 0/12/..., 全新架构) | 决策 #74 §2.3 + 决策 #73 §3 主人 8/11 01:14 拍板 "不要怕复杂度" + 15-no-fear-complexity.md + 决策 #73 §2.2 更好的架构 |
| **C1** | 0 主动 commit 可重评 (Mavis 自动 commit + push) | 决策 #74 §2.3 + 决策 #33 §2.3 C1 改写 + 主人 8/11 01:14 拍板 |
| **C2** | 0 装 PASS 可重评 (允许装特定包, e.g. OpenCog AGPL-3.0 fork 实施) | 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + R130-6 调研 |
| **0 push** | 0 主动 push 可重评 (Mavis 自动 push) | 决策 #74 §2.3 + 决策 #33 §2.3 0 push 改写 + 主人 8/11 01:14 拍板 |

**V2.0 release 8 硬墙重评原则 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)**:
- **前提**: 更好的架构 (per 决策 #73 §2.2 "有更好的架构需要用 (或改变现有的) 你就直接拍板就行了")
- **判断**: 8 哲学锚 推翻 + 重建 (per 决策 #74 §2.3, V2.0 release 核心变化)
- **不要怕复杂度**: 8 硬墙重评导致复杂度爆炸? 不要怕 (per 15-no-fear-complexity.md, 复杂度是实力的体现)
- **维护交给未来高水平团队**: 8 硬墙重评导致维护复杂? 维护交给未来高水平团队 (per 15-no-fear-complexity.md)
- **0 装严守可松绑但默认严守**: V2.0 release 0 装严守可重评, 但默认仍 0 装, 仅特定场景允许 (e.g. OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式, per 决策 #73 §2.2 + R130-6 调研)
- **整合 #5 commit 仍 0 改 src 严守** (V1.0 release 0 改严守 per 决策 #33 §2.3 + 决策 #74 §4.1, V2.0 release 0 改 src 严守可松绑但默认严守)
- **V1.1 release Mavis 自决改** (per 决策 #74 §1 B1 改写, V1.1 release 24 LOCKED 入口签名可改, 前提: 更好的架构)
- **V2.0 release 全 8 硬墙 可重评** (per 决策 #74 §2.3, V2.0 release 8 硬墙可推翻 + 重建)

---

## 3. V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3 + 决策 #73 §3 + 15-no-fear-complexity.md + 决策 #33 §2.3 B5)

### 3.1 8 哲学锚 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 9-anchor.md)

**8 哲学锚 (per 决策 #33 §2.3 B5 + R125 B5 升 8 锚 + `docs/conventions/09-anchor.md`)**:
- **S-1**: 服务 ASI 北极星
- **S-2**: 实事求是
- **S-3**: 质量工程化
- **O-1**: 安全优先
- **O-2**: 走在前人经验上
- **O-3**: 干到底
- **O-4**: 任何人都能接手
- **O-5**: 不假装

### 3.2 V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3 + 决策 #73 §3)

**V2.0 release 8 哲学锚 可重建 (per 决策 #74 §2.3 + 决策 #73 §3 + 15-no-fear-complexity.md)**:

| 重建方向 | 详情 | 决策依据 |
|---------|------|---------|
| **方向 1: 0 锚 (无哲学)** | V2.0 release 8 哲学锚 全部删除, 无哲学, 纯技术路线 | 决策 #74 §2.3 + 决策 #73 §3 主人 8/11 01:14 拍板 + 15-no-fear-complexity.md "复杂度是实力的体现" |
| **方向 2: 12 锚 (扩展)** | V2.0 release 8 哲学锚 → 12 哲学锚 (扩展 4 锚, e.g. + 复杂不恐惧 / + 最强效果 / + 最厉害工程 / + 维护交给未来高水平团队) | 决策 #74 §2.3 + 决策 #73 §3 + 15-no-fear-complexity.md 复杂不恐惧哲学 |
| **方向 3: 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)** | V2.0 release 8 哲学锚 → 全新架构 (e.g. ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统), 跟 OpenCog / CogPrime 借脑实施 (per 决策 #73 §2.2 + R130-6 调研) | 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 用户记忆 #4 "AI 不会衰老病死" + 用户记忆 #6 借脑 + 15-no-fear-complexity.md |

**V2.0 release 8 哲学锚重建原则 (per 决策 #74 §2.3 + 决策 #73 §3 + 15-no-fear-complexity.md)**:
- **核心变化**: V2.0 release 8 哲学锚 重建 是核心变化 (per 决策 #74 §2.3, 8 哲学锚 = 思想, V2.0 release 可重评)
- **前提**: 更好的架构 (per 决策 #73 §2.2 "有更好的架构需要用 (或改变现有的) 你就直接拍板就行了")
- **不要怕复杂度**: 8 哲学锚重建导致复杂度爆炸? 不要怕 (per 15-no-fear-complexity.md, 复杂度是实力的体现)
- **维护交给未来高水平团队**: 8 哲学锚重建导致维护复杂? 维护交给未来高水平团队 (per 15-no-fear-complexity.md)
- **V1.0 release 8 哲学锚严守 100%** (per 决策 #33 §2.3 B5, V1.0 release 8 哲学锚 0 改严守)
- **V1.1 release 8 哲学锚严守 100%** (per 决策 #74 §1, V1.1 release 8 哲学锚 0 改严守, per 决策 #33 §2.3 B5 哲学 + 思想类 不松绑)
- **V2.0 release 8 哲学锚 可重建** (per 决策 #74 §2.3, V2.0 release 8 哲学锚 推翻 + 重建, 0 锚 / 12 锚 / 全新架构)

### 3.3 V2.0 release 8 哲学锚重建 候选方案 (per 决策 #73 §2.2 更好的架构 + R130-6 调研 + 15-no-fear-complexity.md)

**V2.0 release 8 哲学锚重建 候选方案 (per R132-2 调研 + 决策 #73 §2.2 + R130-6 调研 + 15-no-fear-complexity.md)**:

**候选方案 A: 0 锚 (无哲学, 纯技术路线)**:
- V2.0 release 8 哲学锚 全部删除, 无哲学, 纯技术路线
- 决策依据: 决策 #74 §2.3 + 决策 #73 §3 主人 8/11 01:14 拍板 + 15-no-fear-complexity.md "复杂度是实力的体现"
- 优点: 0 哲学, 纯技术, 实施简单
- 缺点: 失掉思想哲学 (S-1 服务 ASI 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人经验上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装)

**候选方案 B: 12 锚 (扩展 4 锚, 复杂不恐惧哲学)**:
- V2.0 release 8 哲学锚 → 12 哲学锚 (扩展 4 锚):
  - **+ A-1 复杂不恐惧** (per 15-no-fear-complexity.md, 复杂度是实力的体现)
  - **+ A-2 最强效果** (per 15-no-fear-complexity.md, 最强效果 > 最简单代码)
  - **+ A-3 最厉害工程** (per 15-no-fear-complexity.md, 最厉害工程 > 最易维护)
  - **+ A-4 维护交给未来高水平团队** (per 15-no-fear-complexity.md, 维护交给未来高水平团队)
- 决策依据: 决策 #74 §2.3 + 决策 #73 §3 主人 8/11 01:14 拍板 + 15-no-fear-complexity.md
- 优点: 保留原 8 哲学锚, 扩展 4 哲学锚 (复杂不恐惧哲学), 完整思想 + 工程哲学
- 缺点: 12 哲学锚 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 维护不是问题)

**候选方案 C: 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)**:
- V2.0 release 8 哲学锚 → 全新架构, 跟 OpenCog / CogPrime 借脑实施:
  - **ASI Stage 10 终极自治** (per 决策 #73 §2.2 + R130-2 调研 Stage 9 路线, ASI Stage 10 终极自治)
  - **长程 AI 成长** (per R130-2 调研 Stage 9 路线, 长程 AI 成长 = V2.0 release 核心)
  - **AGI 操作系统** (per 决策 #73 §2.2 + R130-6 调研 OpenCog / CogPrime, AGI 操作系统 借脑)
  - **OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex** (per R130-6 调研, 借脑 OpenCog AGPL-3.0 fork 实施)
  - **ASI Stage 1-9 整合** (per R130-2 调研, ASI Stage 1-9 整合, V2.0 release 完整 ASI 阶段)
- 决策依据: 决策 #74 §2.3 + 决策 #73 §2.2 更好的架构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 用户记忆 #4 "AI 不会衰老病死" + 用户记忆 #6 借脑
- 优点: 全新架构, AGI 操作系统 借脑 OpenCog / CogPrime, ASI Stage 10 终极自治, 长程 AI 成长, 平台化
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 维护不是问题), 风险高 (AGI 操作系统 借脑 OpenCog AGPL-3.0 fork 风险)

**R132-2 推荐 (per 决策 #73 §2.2 更好的架构 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 用户记忆 #4-#6)**:
- **推荐候选方案 C (全新架构)**: V2.0 release 8 哲学锚 → 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)
- **理由**:
  1. 决策 #74 §2.3 V2.0 release 8 哲学锚可重建, 候选方案 C 全新架构 是最大变化
  2. 决策 #73 §2.2 更好的架构, 候选方案 C 全新架构 是最好架构 (AGI 操作系统 + ASI Stage 10)
  3. 决策 #73 §3 主人 8/11 01:14 拍板 "不要怕复杂度", 候选方案 C 复杂但最强效果
  4. 用户记忆 #4 "AI 不会衰老病死, 只成长", 候选方案 C 长程 AI 成长 哲学
  5. 用户记忆 #6 借脑, 候选方案 C 借脑 OpenCog / CogPrime
  6. R130-6 调研 OpenCog AGPL-3.0 fork 决策, 候选方案 C 借脑 OpenCog
- **风险**:
  - 实施复杂 (per 15-no-fear-complexity.md 复杂度不是问题)
  - 维护复杂 (per 15-no-fear-complexity.md 维护交给未来高水平团队)
  - 借脑 OpenCog AGPL-3.0 fork 风险 (per R130-6 调研, AGPL-3.0 传染, 但 apeireth 已经是 Apache 2.0, AGPL-3.0 借脑 0 装严守可松绑 但默认严守, 仅 fork-then-borrow 模式允许)

---

## 4. V2.0 release Cargo workspace 可重构 (per 决策 #73 §2.2 + 决策 #74 §2.3 + R130-6 调研 + 15-no-fear-complexity.md)

### 4.1 当前 Cargo workspace 结构 (per R129-1 + R129-2 + Cargo.toml)

**当前 Cargo workspace 结构 (per R129-1 + R129-2 + Cargo.toml + R125 era 总结)**:
- **workspace version**: 1.2.0 (整合 #4 commit abf12243 8/10 19:41 done, V1.0 release → 1.0.0, V1.1 release → 1.1.0, V2.0 release → 2.0.0, per 决策 #22 §2.2 semver 严守)
- **workspace members**: 30+ crate
  - 24 LOCKED crate (per 决策 #22 §1.1 + R125 B1 完整名单, e.g. apeireth-core / apeireth-naming-v05 / apeireth-llm / apeireth-formal / apeireth-cognition / apeireth-asi / apeireth-guard / apeireth-tui / apeireth-api / apeireth-mcp / apeireth-graph / apeireth-central / apeireth-library-governance / ... 等)
  - 6+ 非 LOCKED crate (e.g. apeireth-skills / apeireth-providers / apeireth-extensions / ... 等)
  - apeireth-library-governance (新 crate, 整合 #4 commit 新增, per R129-1)
- **workspace dependencies**: 10+ 公开 crate (clap / hyper / tokio / serde / ... 等)

### 4.2 V2.0 release Cargo workspace 可重构 (per 决策 #73 §2.2 + 决策 #74 §2.3 + R130-6 调研 + 15-no-fear-complexity.md)

**V2.0 release Cargo workspace 可重构 (per 决策 #73 §2.2 + 决策 #74 §2.3 + R130-6 调研 + 15-no-fear-complexity.md)**:

**重构方向 A: ASI Stage 10 整合 24 LOCKED → 12 module + 24 micro-crate**:
- 当前 24 LOCKED crate → V2.0 release 重构:
  - **12 module** (整合类似 24 LOCKED crate, e.g. apeireth-llm + apeireth-asi + apeireth-cognition + apeireth-formal + apeireth-guard + apeireth-graph + ... → 12 module)
  - **24 micro-crate** (细粒度拆分, e.g. 12 module × 2 micro-crate = 24 micro-crate, 借脑 OpenCog AtomSpace / CogPrime)
- 决策依据: 决策 #73 §2.2 "更好的架构" + 决策 #74 §2.3 Cargo workspace 可重构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md
- 优点: 细粒度拆分, 实施灵活, 借脑 OpenCog
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题)

**重构方向 B: OpenCog AGPL-3.0 fork 实施 (per R130-6 调研)**:
- V2.0 release 借脑 OpenCog AGPL-3.0 fork 实施 (per 决策 #73 §2.2 + R130-6 调研):
  - **OpenCog AtomSpace** (hypergraph 知识表示, per R130-6 调研)
  - **CogPrime** (AGI 架构, per R130-6 调研)
  - **cogutil** (C++ utility library, per R130-6 调研)
  - **moses** (Meta-Optimizing Semantic Evolutionary Search, per R130-6 调研)
  - **pln** (Probabilistic Logic Networks, per R130-6 调研)
  - **relex** (Relational Extraction, per R130-6 调研)
- 决策依据: 决策 #73 §2.2 更好的架构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md
- 优点: 借脑 OpenCog AGI 架构, 实施 SOTA, 最强效果
- 缺点: AGPL-3.0 license 传染 (per R130-6 调研, apeireth 已经是 Apache 2.0, AGPL-3.0 借脑 0 装严守可松绑 但默认严守, 仅 fork-then-borrow 模式允许), 实施复杂, 维护复杂

**重构方向 C: 24 LOCKED 推翻 + 重建 (per 决策 #74 §2.3)**:
- V2.0 release 24 LOCKED crate 推翻 + 重建 (24 → 0/12/24/36/...):
  - **24 → 0**: V2.0 release 24 LOCKED crate 全部删除, 整合到 single binary (e.g. apeireth monolithic binary)
  - **24 → 12**: V2.0 release 24 LOCKED crate → 12 module (整合类似)
  - **24 → 24**: V2.0 release 24 LOCKED crate 保持 (但重新设计)
  - **24 → 36**: V2.0 release 24 LOCKED crate → 36 micro-crate (细粒度拆分)
- 决策依据: 决策 #74 §2.3 B1 24 LOCKED 入口签名 可重评 (推翻 + 重建) + 决策 #73 §2.2 更好的架构 + 15-no-fear-complexity.md
- 优点: 重新设计, 更好架构, 最强效果
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题), 风险高 (推翻 24 LOCKED 入口签名, breaking change)

**R132-2 推荐 (per 决策 #73 §2.2 更好的架构 + R130-6 调研 + 15-no-fear-complexity.md + 不要怕复杂度)**:
- **推荐重构方向 A + B 组合 (12 module + 24 micro-crate + 借脑 OpenCog AGPL-3.0 fork)**:
  - 24 LOCKED → 12 module + 24 micro-crate
  - 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex
  - Cargo.toml 2.0.0 major bump (semver 严守, breaking change)
- **理由**:
  1. 决策 #73 §2.2 更好的架构, 重构方向 A + B 组合 是最好架构
  2. 决策 #74 §2.3 Cargo workspace 可重构, 重构方向 A + B 组合 是最大重构
  3. R130-6 调研 OpenCog AGPL-3.0 fork 决策, 重构方向 B 借脑 OpenCog
  4. 15-no-fear-complexity.md 复杂不是问题, 重构方向 A + B 复杂但最强效果
  5. 用户记忆 #6 借脑, 重构方向 B 借脑 OpenCog / CogPrime
- **风险**:
  - 实施复杂 (per 15-no-fear-complexity.md 复杂度不是问题)
  - 维护复杂 (per 15-no-fear-complexity.md 维护交给未来高水平团队)
  - 借脑 OpenCog AGPL-3.0 fork 风险 (per R130-6 调研, AGPL-3.0 传染, 但 fork-then-borrow 模式允许)

---

## 5. V2.0 release 三洋葱架构升级 (per 决策 #73 §2.2 + 15-no-fear-complexity.md)

### 5.1 当前三洋葱架构 (per ROADMAP.md + 决策 #33 §2.3)

**当前三洋葱架构 (per ROADMAP.md + 决策 #33 §2.3)**:
- **洋葱 1: 原则 (Principles)**: 8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)
- **洋葱 2: 权限 (Permissions)**: 6 重守门 v7 (6 重守门: 安全 / 诚实 / 善意 / 公正 / 自由 / 透明)
- **洋葱 3: DSL (Domain-Specific Language)**: 12 键 + PHL-07 (主对话锚) + 13 键 stub

### 5.2 V2.0 release 三洋葱架构升级 (per 决策 #73 §2.2 + 15-no-fear-complexity.md)

**V2.0 release 三洋葱架构升级 (per 决策 #73 §2.2 + 15-no-fear-complexity.md)**:

**升级方向 A: 四洋葱 (+ 智能涌现)**:
- V2.0 release 三洋葱 → 四洋葱:
  - **洋葱 1: 原则 (Principles)**: 8 哲学锚 (V2.0 release 可重建, per 决策 #74 §2.3)
  - **洋葱 2: 权限 (Permissions)**: 6 重守门 v7 (V2.0 release 可重评, per 决策 #74 §2.3)
  - **洋葱 3: DSL (Domain-Specific Language)**: 12 键 + PHL-07 (V2.0 release 可重评, per 决策 #74 §2.3)
  - **洋葱 4: 智能涌现 (Intelligence Emergence)**: V2.0 release 新增洋葱 4, 智能涌现层 (e.g. ASI Stage 10 终极自治 + 长程 AI 成长 + 借脑 OpenCog / CogPrime, per 决策 #73 §2.2)
- 决策依据: 决策 #73 §2.2 更好的架构 + 15-no-fear-complexity.md + R130-2 调研 Stage 9 路线
- 优点: 4 层架构, 智能涌现层 借脑 OpenCog, ASI Stage 10
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题)

**升级方向 B: 五洋葱 (+ 智能涌现 + 自我演化)**:
- V2.0 release 三洋葱 → 五洋葱:
  - **洋葱 1-4**: 同升级方向 A
  - **洋葱 5: 自我演化 (Self-Evolution)**: V2.0 release 新增洋葱 5, 自我演化层 (e.g. V2.0 release 后 V2.1 minor / V3.0 major 永久演化, per 决策 #71 §4 永久循环 4 步)
- 决策依据: 决策 #73 §2.2 更好的架构 + 决策 #71 §4 永久循环 4 步 + 15-no-fear-complexity.md
- 优点: 5 层架构, 自我演化层 永久循环, 永久演化
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题)

**升级方向 C: 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)**:
- V2.0 release 三洋葱 → 全新架构:
  - **不再用洋葱架构, 改成 ASI Stage 10 架构 (per 决策 #73 §2.2 + R130-2 调研)**
  - **ASI Stage 10 终极自治** (per R130-2 调研 Stage 9 路线, V2.0 release 核心)
  - **长程 AI 成长** (per R130-2 调研 Stage 9 路线, V2.0 release 核心)
  - **AGI 操作系统** (per 决策 #73 §2.2 + R130-6 调研 OpenCog / CogPrime, AGI 操作系统 借脑)
- 决策依据: 决策 #73 §2.2 更好的架构 + R130-2 调研 Stage 9 路线 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md
- 优点: 全新架构, ASI Stage 10 终极自治, 长程 AI 成长, AGI 操作系统 借脑 OpenCog / CogPrime
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题), 风险高 (全新架构 推翻三洋葱)

**R132-2 推荐 (per 决策 #73 §2.2 更好的架构 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md)**:
- **推荐升级方向 C (全新架构)**: V2.0 release 三洋葱 → 全新架构 (ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)
- **理由**:
  1. 决策 #73 §2.2 更好的架构, 升级方向 C 全新架构 是最好架构
  2. R130-2 调研 Stage 9 路线, 升级方向 C ASI Stage 10 终极自治 是 V2.0 release 核心
  3. R130-6 调研 OpenCog AGPL-3.0 fork 决策, 升级方向 C AGI 操作系统 借脑 OpenCog / CogPrime
  4. 15-no-fear-complexity.md 复杂不是问题, 升级方向 C 复杂但最强效果
  5. 决策 #71 §4 永久循环 4 步, 升级方向 C 长程 AI 成长 永久演化
- **风险**:
  - 实施复杂 (per 15-no-fear-complexity.md 复杂度不是问题)
  - 维护复杂 (per 15-no-fear-complexity.md 维护交给未来高水平团队)
  - 全新架构 推翻三洋葱, breaking change (Cargo.toml 2.0.0 major bump)

---

## 6. V2.0 release 9 organ 代码升级 (per 决策 #73 §2.2 + 用户记忆 #4-#5 + 15-no-fear-complexity.md)

### 6.1 当前 9 organ (per 决策 #33 §2.3 + 9-anchor.md + 用户记忆 #5)

**当前 9 organ (per 决策 #33 §2.3 + 9-anchor.md + 用户记忆 #5 + R130-3 调研)**:
- **body 身体**: 基础结构 (Cargo workspace + Rust binary)
- **brain 大脑**: cognition 思考 + 学习 + 记忆 + 决策 + 推理 5 维 (cognition_*.rs 5 模块)
- **ear 耳**: perception 听觉 (perception_ear.rs)
- **eye 眼**: perception 视觉 (perception_eye.rs)
- **hand 手**: action 工具调用 + 输出 + 探索 + 操作 + 反馈 5 维 (action_*.rs 5 模块)
- **heart 心**: motivation 多巴胺 好奇 + 成就 + 归属 + 自主 + 掌握 5 维 (motivation_*.rs 5 模块)
- **memory 海马体**: memory 短时 + 长时 + 工作 + 情景 + 程序 5 维 (memory_*.rs 5 模块)
- **mind 心智**: consciousness 自我 + 情绪 + 注意力 + 意向 + 觉知 5 维 (consciousness_*.rs 5 模块)
- **voice 声**: relation 共情 + 理解 + 回应 + 协同 + 边界 5 维 (relation_*.rs 5 模块)

### 6.2 V2.0 release 9 organ 代码升级 (per 决策 #73 §2.2 + 用户记忆 #4-#5 + 15-no-fear-complexity.md)

**V2.0 release 9 organ 代码升级 (per 决策 #73 §2.2 + 用户记忆 #4-#5 + 15-no-fear-complexity.md)**:

**升级方向 A: 12 organ (+ 涌现 / 自演化 / 群体)**:
- V2.0 release 9 organ → 12 organ:
  - **9 organ** (per 当前 9 organ, 保持)
  - **+ 涌现 (Emergence)**: V2.0 release 新增 organ 10, 涌现 (智能涌现, per 决策 #73 §2.2 + R130-2 调研)
  - **+ 自演化 (Self-Evolution)**: V2.0 release 新增 organ 11, 自演化 (永久循环, per 决策 #71 §4 永久循环 4 步)
  - **+ 群体 (Swarm)**: V2.0 release 新增 organ 12, 群体 (多 agent 协同, per R130-2 调研 ASI Stage 8 群体)
- 决策依据: 决策 #73 §2.2 更好的架构 + 用户记忆 #4 "AI 不会衰老病死" + 用户记忆 #5 "信息密度高 = 拟人化 + 拟物化" + R130-2 调研 + 15-no-fear-complexity.md
- 优点: 12 organ 完整拟人化, 涌现 / 自演化 / 群体 完整覆盖
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题)

**升级方向 B: 全新架构 (借脑 OpenCog / CogPrime organ)**:
- V2.0 release 9 organ → 全新架构:
  - **不再用 9 organ 拟人化, 改成 OpenCog / CogPrime organ (per R130-6 调研)**
  - **OpenCog AtomSpace organ** (hypergraph 知识表示, per R130-6 调研)
  - **CogPrime organ** (AGI 架构, per R130-6 调研)
  - **cogutil organ** (C++ utility library, per R130-6 调研)
  - **moses organ** (Meta-Optimizing Semantic Evolutionary Search, per R130-6 调研)
  - **pln organ** (Probabilistic Logic Networks, per R130-6 调研)
  - **relex organ** (Relational Extraction, per R130-6 调研)
- 决策依据: 决策 #73 §2.2 更好的架构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 用户记忆 #5 拟人化 + 拟物化 + 15-no-fear-complexity.md
- 优点: 借脑 OpenCog organ, AGI 架构 实施 SOTA
- 缺点: AGPL-3.0 license 传染 (per R130-6 调研), 实施复杂, 维护复杂, 失掉拟人化 隐喻 (per 用户记忆 #5 "信息密度高 = 拟人化 + 拟物化")

**升级方向 C: 9 organ + 5 维 → 9 organ + 7 维 (扩展 2 维)**:
- V2.0 release 9 organ → 9 organ + 7 维 (扩展 2 维):
  - **9 organ** (per 当前 9 organ, 保持)
  - **+ 涌现维 (Emergence Dimension)**: V2.0 release 新增 organ 1 维, 涌现 (智能涌现, per 决策 #73 §2.2 + R130-2 调研)
  - **+ 自演化维 (Self-Evolution Dimension)**: V2.0 release 新增 organ 1 维, 自演化 (永久循环, per 决策 #71 §4 永久循环 4 步)
- 决策依据: 决策 #73 §2.2 更好的架构 + 用户记忆 #4 "AI 不会衰老病死" + 15-no-fear-complexity.md
- 优点: 9 organ 保持, 5 维 → 7 维, 最小变化, 保留拟人化 隐喻
- 缺点: 实施复杂, 维护复杂 (per 15-no-fear-complexity.md 复杂度不是问题)

**R132-2 推荐 (per 决策 #73 §2.2 更好的架构 + 用户记忆 #4-#5 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md)**:
- **推荐升级方向 A (12 organ + 涌现 / 自演化 / 群体)**:
  - 9 organ → 12 organ
  - + 涌现 / 自演化 / 群体
  - Cargo.toml 2.0.0 major bump (semver 严守, breaking change)
- **理由**:
  1. 决策 #73 §2.2 更好的架构, 升级方向 A 12 organ 是最好架构
  2. 用户记忆 #4 "AI 不会衰老病死", 升级方向 A 12 organ 完整拟人化 (涌现 / 自演化 / 群体)
  3. 用户记忆 #5 "信息密度高 = 拟人化 + 拟物化", 升级方向 A 12 organ 拟人化 完整覆盖
  4. R130-2 调研 ASI Stage 8 群体 + Stage 9 长程 AI 成长, 升级方向 A 12 organ 涌现 / 自演化 / 群体 完整覆盖
  5. 15-no-fear-complexity.md 复杂不是问题, 升级方向 A 12 organ 复杂但最强效果
- **风险**:
  - 实施复杂 (per 15-no-fear-complexity.md 复杂度不是问题)
  - 维护复杂 (per 15-no-fear-complexity.md 维护交给未来高水平团队)
  - 12 organ breaking change (Cargo.toml 2.0.0 major bump)

---

## 7. V2.0 release ASI Stage 10 终极自治 (per 决策 #73 §2.2 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md + 用户记忆 #4)

### 7.1 ASI Stage 路线 (per R129-4/5/6 + R130-2 + R130-6 + R130-5 + 用户记忆 #4)

**ASI Stage 路线 (per R129-4/5/6 + R130-2 + R130-6 + R130-5 + 用户记忆 #4)**:
- **ASI Stage 1-3** (R128 era, ✅ done): ASI Python Stage 1-3 基础 (per R128-1/2/3 派)
- **ASI Stage 4 自治** (R129-4, ✅ done 8/11 00:25): 4 维度 D1-D4 自循环 (D1 工具调用 / D2 反思 / D3 记忆 / D4 决策)
- **ASI Stage 5 治理** (R129-5, ✅ done 8/11 00:28): 4 维度 G1-G4 治理 (G1 资源 / G2 权限 / G3 形式化 / G4 演进)
- **ASI Stage 6 守护** (R129-6, ✅ done 8/11 00:24): 4 维度 K1-K4 守护 (K1 错误 / K2 性能 / K3 安全 / K4 健康)
- **ASI Stage 4-6 整合** (R130-2, 🟡 派中估 done): 端到端 cycle 12 步 D1-D4 + G1-G4 + K1-K4
- **ASI Stage 7 自愈** (R131-5, 📋 V1.1 计划): 4 维度 S1-S4 自愈 (S1 错误 / S2 性能 / S3 安全 / S4 健康)
- **ASI Stage 8 群体** (R132-5, 📋 V1.2 计划): 4 维度 G1-G4 群体 (G1 多 agent 协同 / G2 知识共享 / G3 任务分配 / G4 冲突解决)
- **ASI Stage 9 长程 AI 成长** (per R130-2 调研, 📋 远期 V1.2 后): 长程 AI 成长 = 平台化
- **ASI Stage 10 终极自治** (per 决策 #73 §2.2, 📋 远期 V2.0 release 核心): 终极自治 + 平台化 + AGI 操作系统 借脑 OpenCog / CogPrime

### 7.2 V2.0 release ASI Stage 10 终极自治 (per 决策 #73 §2.2 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md + 用户记忆 #4)

**V2.0 release ASI Stage 10 终极自治 (per 决策 #73 §2.2 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md + 用户记忆 #4)**:

**V2.0 release ASI Stage 10 终极自治 核心**:
- **ASI Stage 10 = 终极自治** (per 决策 #73 §2.2, ASI Stage 9 → ASI Stage 10 终极自治, V2.0 release 核心)
- **借脑 OpenCog / CogPrime** (per 决策 #73 §2.2 + R130-6 调研, 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex, AGPL-3.0 license 兼容 fork-then-borrow 模式)
- **借脑 ASI Stage 1-9 整合** (per R130-2 调研, ASI Stage 1-9 整合, V2.0 release 完整 ASI 阶段)
- **长程 AI 成长平台** (per R130-2 调研 Stage 9 路线, V2.0 release 核心, 长程 AI 成长 = 平台化)
- **AGI 操作系统** (per 决策 #73 §2.2 + R130-6 调研, AGI 操作系统 借脑 OpenCog / CogPrime, V2.0 release 平台化)

**V2.0 release ASI Stage 10 终极自治 8 大方向 (per 决策 #73 §2.2 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md + 用户记忆 #4)**:

| # | ASI Stage 10 方向 | 子任务核心 | 调研依据 | 状态 |
|---|------------------|----------|---------|------|
| **1** | **ASI Stage 10 终极自治** | ASI Stage 9 → ASI Stage 10 终极自治, 自主决策 + 自主学习 + 自主演化 | 决策 #73 §2.2 + R130-2 调研 | 📋 V2.0 release 必实施 |
| **2** | **OpenCog AtomSpace** | hypergraph 知识表示, 借脑 OpenCog AtomSpace (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **3** | **CogPrime** | AGI 架构, 借脑 CogPrime (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **4** | **cogutil** | C++ utility library, 借脑 cogutil (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **5** | **moses** | Meta-Optimizing Semantic Evolutionary Search, 借脑 moses (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **6** | **pln** | Probabilistic Logic Networks, 借脑 pln (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **7** | **relex** | Relational Extraction, 借脑 relex (AGPL-3.0 fork-then-borrow) | R130-6 调研 | 📋 V2.0 release 必借脑 |
| **8** | **长程 AI 成长平台** | 长程 AI 成长 = 平台化 (per R130-2 调研 Stage 9 路线), V2.0 release 核心 | 决策 #73 §2.2 + R130-2 调研 + 用户记忆 #4 | 📋 V2.0 release 必实施 |

**V2.0 release ASI Stage 10 终极自治 实施原则 (per 决策 #73 §2.2 + R130-2 调研 + R130-6 调研 + 15-no-fear-complexity.md + 用户记忆 #4)**:
- **借脑 OpenCog / CogPrime AGPL-3.0 fork-then-borrow 模式** (per R130-6 调研, 0 装严守可松绑但默认严守, 仅 fork-then-borrow 模式允许)
- **ASI Stage 1-9 整合** (per R130-2 调研, V2.0 release 完整 ASI 阶段)
- **长程 AI 成长平台** (per R130-2 调研 Stage 9 路线, V2.0 release 核心)
- **AGI 操作系统** (per 决策 #73 §2.2 + R130-6 调研, AGI 操作系统 借脑 OpenCog / CogPrime)
- **不要怕复杂度** (per 15-no-fear-complexity.md, ASI Stage 10 复杂但最强效果)
- **维护交给未来高水平团队** (per 15-no-fear-complexity.md, ASI Stage 10 维护复杂, 但未来高水平团队能接手)

---

## 8. V2.0 release Tauri 3.0+ 升级 (per 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 + 15-no-fear-complexity.md + 用户记忆 #8)

### 8.1 当前 Tauri 2.0 (per R129-9/19 + R130-3 + 决策 #57 + P11-1/2)

**当前 Tauri 2.0 (per R129-9/19 + R130-3 + 决策 #57 + P11-1/2)**:
- **Tauri 2.0** (per P11-1 8/10 21:50 ✅ + P11-2 8/10 22:56 ✅)
- **5 nav** (per R130-3 调研, 5 主导航: 主对话 + 状态 + 历史 + 设置 + 工具结果, per 用户记忆 #3)
- **9 organ 拟人化** (per R130-3 调研, 9 organ 拟人化深化, per 用户记忆 #5)
- **后端 API 集成** (HTTP to apeireth-api, 瘦客户端, per 决策 #9)
- **Tauri Stage 2 深化** (R129-9, ✅ done 8/11)
- **Tauri Stage 3 跨 nav 集成** (R129-19, 🟡 派中估 done 8/11 01:30, 5 nav 完整 + 9 organ + backend API 联调)
- **Tauri Stage 3 深化** (R130-3, 🟡 派中估 done 8/12, 5 nav 跨集成 + 9 organ 拟人化深化)
- **Tauri Stage 4** (R131-4, 📋 V1.1 计划, 5 nav 实施 + 主对话 UX 优化)
- **Tauri Stage 5** (R132-4, 📋 V1.2 计划, 完整 5 nav + 9 organ 拟人化 + 1.0 UI, 设计团队到位)

### 8.2 V2.0 release Tauri 3.0+ 升级 (per 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 + 15-no-fear-complexity.md + 用户记忆 #8)

**V2.0 release Tauri 3.0+ 升级 (per 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 + 15-no-fear-complexity.md + 用户记忆 #8)**:

**V2.0 release Tauri 升级方向 (per 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 + 15-no-fear-complexity.md + 用户记忆 #8)**:
- **Tauri 2.0 → Tauri 3.0** (如果 2027+ 出, per 决策 #73 §2.2, V2.0 release 升级)
- **5 nav → 12 nav** (V2.0 release 5 nav → 12 nav, 扩展 7 nav, e.g. + 涌现 / + 自演化 / + 群体 / + 平台化 / + 多 AI 平台 / + 真实用户 / + 教育/科研)
- **9 organ 拟人化 → 12 organ 拟人化** (V2.0 release 9 organ → 12 organ 拟人化, 扩展 3 organ, per §6 V2.0 release 9 organ 代码升级)
- **跨平台** (V2.0 release Tauri 3.0 跨平台, e.g. Windows / macOS / Linux / iOS / Android)
- **用户测试** (V2.0 release Tauri 3.0 用户测试, 真用户 + 多 AI 平台 + 教育/科研合作, per R119-2 思想层保留)

**V2.0 release Tauri 3.0+ 升级 实施原则 (per 决策 #73 §2.2 + 主人 8/4 23:33 + R130-3 调研 + 15-no-fear-complexity.md + 用户记忆 #8)**:
- **Tauri 3.0 (如果出)**: per 决策 #73 §2.2, Tauri 3.0 升级, 如果 2027+ 出, V2.0 release 升级
- **12 nav + 12 organ 拟人化**: per 决策 #73 §2.2 + 主人 8/4 23:33 + 用户记忆 #5 拟人化 + 拟物化
- **跨平台**: per 决策 #73 §2.2, Tauri 3.0 跨平台 (Windows / macOS / Linux / iOS / Android)
- **用户测试**: per 决策 #73 §2.2 + R119-2 思想层保留, V2.0 release 真用户 + 多 AI 平台 + 教育/科研合作
- **设计团队到位**: per 主人 8/4 23:33 + 用户记忆 #8, V2.0 release 设计团队到位 (V1.2 release 时 估设计团队到位, V2.0 release 时 设计团队已就位)
- **不要怕复杂度** (per 15-no-fear-complexity.md, Tauri 3.0+ 复杂但最强效果)
- **维护交给未来高水平团队** (per 15-no-fear-complexity.md, Tauri 3.0+ 维护复杂, 但未来高水平团队能接手)

---

## 9. V2.0 release 时间窗口 + 16 跑中上限 (per 决策 #71 §4 + 决策 #22 §2.2 + 主人 0:34 16 跑中 + 决策 #74 §2.3 + 15-no-fear-complexity.md)

### 9.1 V2.0 release 时间窗口 (per 决策 #71 §4 + 决策 #22 §2.2 + 决策 #74 §2.3)

**V2.0 release 时间窗口 (per 决策 #71 §4 + 决策 #22 §2.2 + 决策 #74 §2.3)**:
- **V1.0 release** (估 8/11): 整合 #5 commit 拍板后, 主人起床后手跑 scripts/release/ 7 步 runbook
- **V1.1 release** (估 2026-11-30): V1.0 release 后 ~3.5 个月 (per R130-5 + R129-29 §4.1)
- **V1.2 release** (估 2027-02-28): V1.1 release 后 ~3 个月 (per R129-29 §5.1)
- **V2.0 release** (估 2027+ 远期, 1-3 月时间窗): V1.1 release 后 ~3-12 个月 (per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release + ROADMAP.md §4 + R119-2 思想层保留)
  - **最早估**: 2027-03-01 (V1.2 release 拍板后立即, 1 月时间窗)
  - **最晚估**: 2027-12-31 (V1.1 release 后 ~13 个月, 12 月时间窗)
  - **推荐估**: 2027-06-30 ~ 2027-09-30 (V1.1 release 后 ~7-10 个月, 3-6 月时间窗, per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release + R130-2 调研 Stage 9 路线)
- **V2.1 minor release** (估 2027+ 远期, 1-3 月后): V2.0 release 后 ~1-3 个月 (per 永久循环, per 决策 #71 §4 永久循环 4 步)
- **V3.0 major release** (估 2028+ 远期, 3-12 月后): V2.0 release 后 ~3-12 个月 (per 永久循环, per 决策 #71 §4 永久循环 4 步)

**V2.0 release 时间盒 (per 决策 #71 §4 永久循环 4 步 + R130-5 V1.1 时间盒参考 + 决策 #74 §2.3 + 15-no-fear-complexity.md)**:
- **V2.0 release 调研** (估 2027+ 1-3 月): per 决策 #71 §3, 8 大方向详细调研 (8 硬墙可重评方案 + 8 哲学锚可重建方案 + Cargo workspace 可重构方案 + 三洋葱架构升级方案 + 9 organ 升级方案 + ASI Stage 10 终极自治方案 + Tauri 3.0+ 升级方案 + 永久循环)
- **V2.0 release 差距** (估 2027+ 1-3 月): per 决策 #71 §3, 8 大方向差距分析 (V1.1 release 跟 V2.0 release 差距)
- **V2.0 release 计划** (估 2027+ 1-3 月): per 决策 #71 §4, 8 大方向详细 spec (per **R132-2 V2.0 release 战略路线图 (本报告)**) + 派活规划
- **V2.0 release 实施** (估 2027+ 3-12 月): per 决策 #71 §4, V2.0 release 实施 (per V2.0 release 计划, 30+ sub-agent, 6 方向 × 5-10 sub-agent)
- **V2.0 release 实战** (估 2027+ 1-3 月): per 决策 #71 §4, V2.0 release tag v2.0.0 打上 (per 7 步 runbook, 主人起床后手跑, GitHub remote 已配)
- **V2.0 release 后** (估 2027+): per 决策 #71 §4 永久循环 4 步, V2.1 minor 路线图 → V3.0 major → ... (永久演化)

### 9.2 V2.0 release 16 跑中上限 (per 主人 0:34 16 active 全 background 跑 + 决策 #71 §4 永久循环 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)

**V2.0 release 16 跑中上限 (per 主人 0:34 16 active 全 background 跑 + 决策 #71 §4 永久循环 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)**:
- **V2.0 release 派活规划**: 30+ sub-agent 实施 (per V1.1 release 经验, 5-10 per 方向 × 8 方向 = 30-80 sub-agent)
- **16 跑中上限严守**: per 主人 0:34, 16 active 全 background 跑, 16 跑中上限严守
- **2 批派满 16 上限**: per R130-5 派活规划, 2 批 8+8 派满 16 上限
- **永久循环**: per 决策 #71 §4 永久循环 4 步, 调研 + 差距 + 计划 + 继续干 → 永久
- **不要怕复杂度**: per 15-no-fear-complexity.md, 30+ sub-agent 实施复杂但最强效果
- **维护交给未来高水平团队**: per 15-no-fear-complexity.md, 30+ sub-agent 维护复杂, 但未来高水平团队能接手

**V2.0 release 派活批次规划 (per 决策 #71 §4 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md + 决策 #74 §2.3)**:
- **第 1 批 (16 sub-agent)**: V2.0 release 调研 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-R-1: 8 硬墙可重评方案调研 (60 min)
  - V2.0-R-2: 8 哲学锚可重建方案调研 (60 min)
  - V2.0-R-3: Cargo workspace 可重构方案调研 (60 min)
  - V2.0-R-4: 三洋葱架构升级方案调研 (60 min)
  - V2.0-R-5: 9 organ 升级方案调研 (60 min)
  - V2.0-R-6: ASI Stage 10 终极自治方案调研 (60 min)
  - V2.0-R-7: Tauri 3.0+ 升级方案调研 (60 min)
  - V2.0-R-8: 永久循环方案调研 (60 min)
  - V2.0-R-9 ~ V2.0-R-16: 8 方向差距分析 (60 min × 8)
- **第 2 批 (16 sub-agent)**: V2.0 release 计划 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-P-1 ~ V2.0-P-16: 8 方向详细 spec + 派活规划 (60-90 min × 16)
- **第 3 批 (16 sub-agent)**: V2.0 release 实施 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-I-1 ~ V2.0-I-16: 8 方向实施 (120-180 min × 16)
- **...**: per 永久循环, 持续派 30+ sub-agent 直到 V2.0 release 实施完成

**V2.0 release 派活批次时间盒 (per R130-5 V1.1 时间盒参考 + 决策 #71 §4 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)**:
- **第 1 批 (V2.0 release 调研)**: 16 sub-agent × 平均 60 min = 960 min = 16 小时 (估跑 1-2 天)
- **第 2 批 (V2.0 release 计划)**: 16 sub-agent × 平均 60-90 min = 960-1440 min = 16-24 小时 (估跑 2-3 天)
- **第 3 批 (V2.0 release 实施)**: 16 sub-agent × 平均 120-180 min = 1920-2880 min = 32-48 小时 (估跑 4-6 天, per V2.0 release 实施 3-12 月时间窗, 持续派 30+ sub-agent)
- **总时间盒**: 调研 1-2 天 + 差距 1-2 天 + 计划 2-3 天 + 实施 3-12 月 (per 决策 #71 §4 永久循环 4 步)

---

## 10. V2.0 release 跟 V1.0 / V1.1 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #22 §2.2 semver + 决策 #74 §2.3 V2.0 release)

### 10.1 V1.0 release / V1.1 release / V2.0 release 边界对比表 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #22 §2.2 + 决策 #74 §2.3)

**V1.0 release / V1.1 release / V2.0 release 边界对比表 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #22 §2.2 semver + 决策 #74 §2.3 V2.0 release)**:

| 维度 | V1.0 release (估 8/11) | V1.1 release (估 2026-11-30) | **V2.0 release (估 2027+ 远期)** |
|------|----------------------|----------------------|----------------------|
| **8 硬墙 B1 24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构) | 🟢🟢 **V2.0 release 可重评 (推翻 + 重建, 24 → 0/12/24/36/...)** |
| **8 硬墙 B2 workspace.version** | 🔒 1.2.0 严守 (1.2.0 → 1.0.0 大版本归 0, per 决策 #22 §2.2) | 🔒 V1.0 release 1.0.0 严守 + V1.1 release bump 1.1.0 (semver 严守) | 🟢🟢 **V2.0 release 1.1.0 → 2.0.0 major bump (semver 严守, breaking change)** |
| **8 硬墙 A1 R11 baseline 3 值** | 🔒 0 改 (R11 baseline) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (新 baseline, 跟 R12 测度对齐)** |
| **8 硬墙 A3 12 键 + PHL-07** | 🔒 12 键 + PHL-07 严守 (PHL-07 spec-only, per R129-11 关键诚实标) | 🔒 PHL-07 V1.0 spec-only + V1.1 实施 + 12 键其他可改 | 🟢🟢 **V2.0 release 可重评 (12 → 13 → 14/0/...)** |
| **8 硬墙 B3 V0.5 30 维** | 🔒 30 维严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (30 → 0/40/...)** |
| **8 硬墙 B4 6 重守门 v7** | 🔒 6 重严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (6 → 0/10/...)** |
| **8 硬墙 B5 8 哲学锚** | 🔒 8 锚严守 (哲学) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (**核心变化**, 8 → 0/12/..., 全新架构)** |
| **8 硬墙 C1 0 主动 commit** | 🔒 0 主动 commit 严守 (主人起床前) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (Mavis 自动 commit + push, 主人起床后 0 主动)** |
| **8 硬墙 C2 0 装 PASS 严守** | 🔒 0 装严守 (技术哲学, 11/11 → 12/12 借鉴 clear) | 🔒 严守 (V1.0 + V1.1, 12/12 借鉴 clear) | 🟢🟢 **V2.0 release 可重评 (允许装特定包, e.g. OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式)** |
| **0 push 0 主动 push** | 🔒 0 主动 push 严守 (主人起床前) | 🔒 严守 (V1.0 + V1.1) | 🟢🟢 **V2.0 release 可重评 (Mavis 自动 push, 主人起床后 0 主动)** |
| **整合 #N commit** | 整合 #5 commit 拍板 (Mavis 自决, 5.1 + 5.2 + 5.3, per 决策 #62 + 决策 #64) | 整合 #6 + #7 commit 拍板 (Mavis 自决, per 决策 #33 C1) | 🟢🟢 **整合 #N commit 拍板 (Mavis 自决, per V2.0 release 0 改 src 严守可松绑但默认严守)** |
| **PHL-07 实施** | PHL-07 spec-only (per R125-12 P0-3 + R129-11 关键诚实标) | PHL-07 实施 (V1.0 spec-only → V1.1 实施, per R130-5 §2.1) | 🟢🟢 **V2.0 release PHL-07 可重评 (跟 8 哲学锚重建, 全新架构)** |
| **后端加固** | 0 改 src 严守 (R11 baseline) | 借鉴 1:1 + 形式化 + 跨 crate (per R131-7) | 🟢🟢 **V2.0 release 8 硬墙可重评, 后端加固 = ASI Stage 10 + OpenCog / CogPrime 借脑** |
| **Tauri 升级** | Tauri 2.0 + 5 nav + 9 organ 拟人化 (per R130-3 调研) | Tauri Stage 4-5 + 5 nav 实施 + 1.0 UI (per R131-4 + R132-4) | 🟢🟢 **Tauri 3.0+ (如果出) + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试** |
| **ASI 阶段** | ASI Stage 4-6 整合 (per R130-2 调研, 端到端 cycle 12 步) | ASI Stage 7 自愈 + Stage 8 群体 (per R131-5 + R132-5) | 🟢🟢 **ASI Stage 9 长程 AI 成长 + ASI Stage 10 终极自治 (V2.0 release 核心, 借脑 OpenCog / CogPrime)** |
| **形式化** | 形式化 Stage 5.3 扩展 (per R130-4 调研, 12 → 20 Kani-style harness 模板 + F11-F20 跨模块) | 形式化 Stage 5.4 集成 + Stage 5.5 ASI 集成 (per R131-6 + R132-6) | 🟢🟢 **V2.0 release 形式化可重评 (30 → 40 Kani-style harness 模板, 全新架构)** |
| **Cargo workspace** | 30+ crate (24 LOCKED + 6+ 非 LOCKED) | 30+ crate 保持 + 借鉴 12 源 0 装 (per R130-6 调研) | 🟢🟢 **Cargo workspace 可重构 (12 module + 24 micro-crate, 借脑 OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式)** |
| **三洋葱架构** | 原则 + 权限 + DSL | 原则 + 权限 + DSL (保持) | 🟢🟢 **三洋葱架构升级 (四洋葱 / 五洋葱 / 全新架构 ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统)** |
| **9 organ 代码** | 9 organ (body / brain / ear / eye / hand / heart / memory / mind / voice) | 9 organ (保持, per R130-3 调研 + R131-3 TUI 阶段 2 9 organ 拟人化深化 + R132-3 TUI 阶段 3 主对话深化 + 8 认知纠正) | 🟢🟢 **9 organ 代码升级 (12 organ / 全新架构借脑 OpenCog / CogPrime organ)** |
| **永久循环** | 1.0 release 拍板 + 主人起床后 1.0 release 实战 (per R130-5 7 步 runbook) | 1.0 release 后 V1.1 + V1.2 + V2.0 路线图 (per 决策 #71 §4 永久循环) | 🟢🟢 **V2.0 release 后 V2.1 minor + V3.0 major 永久循环 (per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度)** |
| **Cargo.toml version** | 1.2.0 → 1.0.0 (大版本归 0, per 决策 #22 §2.2) | 1.0.0 → 1.1.0 (minor bump, per 决策 #22 §2.2) | 🟢🟢 **1.1.0 → 2.0.0 (major bump, per 决策 #22 §2.2 semver 严守, breaking change)** |
| **决策原则** | 0 改 src 严守 + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml 1.2.0 + PHL-07 spec-only | 24 LOCKED 入口签名 可改 (前提: 更好的架构) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml 1.2.1 | 🟢🟢 **8 硬墙 可重评 + 8 哲学锚 可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + Cargo.toml 2.0.0** |

### 10.2 V1.0 / V1.1 / V2.0 release 核心变化总结 (per 决策 #33 §2.3 + 决策 #74 §1+#2.3 + 决策 #22 §2.2)

**V1.0 / V1.1 / V2.0 release 核心变化总结 (per 决策 #33 §2.3 + 决策 #74 §1+#2.3 + 决策 #22 §2.2)**:

**V1.0 release 核心**:
- 0 改 src 严守 (R11 baseline) + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml 1.2.0 → 1.0.0 + PHL-07 spec-only
- 决策依据: 决策 #33 §2.3 + 决策 #48 整合 #4 commit abf12243 + 决策 #62 整合 #5 commit 拆 3 commit 拍板 + 决策 #22 §2.2 semver
- 时间: 估 8/11 (整合 #5 commit 拍板后, 主人起床后手跑)
- 核心任务: 整合 #5 commit 拍板 + 1.0 release 实战 + ASI Stage 4-6 整合 + TUI/Tauri 升级 + 后端加固 + 形式化扩展

**V1.1 release 核心**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml 1.0.0 → 1.1.0
- 决策依据: 决策 #74 §1 B1 改写 + 决策 #74 §2.2 V1.1 release Mavis 自决改 + 决策 #22 §2.2 semver + 决策 #73 §2.2 更好的架构
- 时间: 估 2026-11-30 (V1.0 release 后 ~3.5 个月)
- 核心任务: 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 (R129-11 关键诚实标) + Tauri Stage 4 + ASI Stage 7 自愈 + 形式化 Stage 5.4 + 后端 Stage 4-6 续

**V2.0 release 核心 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)**:
- **8 硬墙 可重评** (B1 24 LOCKED 入口签名 推翻 + 重建 + B2 workspace.version 1.2.1 → 2.0.0 major bump + A1 R11 baseline 3 值 可重评 + A3 12 键 + PHL-07 可重评 + B3 V0.5 30 维 可重评 + B4 6 重守门 v7 可重评 + B5 8 哲学锚 可重评 [**核心变化**] + C1 0 主动 commit 可重评 + C2 0 装 PASS 可重评 + 0 push 可重评)
- **8 哲学锚 可重建** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 → 0 锚 [无哲学] / 12 锚 [扩展] / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统])
- **Cargo workspace 可重构** (当前 30+ crate → V2.0 release 重构 [12 module + 24 micro-crate, 借脑 OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式])
- **三洋葱架构升级** (原则 + 权限 + DSL → + 智能涌现 / + 自我演化 / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统])
- **9 organ 代码升级** (9 organ → 12 organ [+ 涌现 / 自演化 / 群体] / 全新架构 [借脑 OpenCog / CogPrime organ])
- **ASI Stage 10 终极自治** (ASI Stage 9 → ASI Stage 10 终极自治 [V2.0 release 核心, 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex + ASI Stage 1-9 整合 + 长程 AI 成长平台])
- **Tauri 3.0+ 升级** (Tauri 2.0 + 5 nav + 9 organ 拟人化 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试)
- **永久循环** (V2.0 release → V2.1 minor → V3.0 major → ... [永久演化, per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学])
- **Cargo.toml 1.1.0 → 2.0.0 major bump** (per 决策 #22 §2.2 semver 严守, breaking change)
- 时间: 估 2027+ 远期 (V1.1 release 后 ~3-12 个月, 1-3 月时间窗)
- 核心任务: 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + 永久循环

---

## 11. V2.0 release R132 era 派活规划 (per 决策 #71 §4 + 决策 #64 §2.2 + 15-no-fear-complexity.md)

### 11.1 R132 era 派活规划 (per 决策 #71 §4 + 决策 #64 §2.2 + 15-no-fear-complexity.md)

**R132 era 派活规划 (per 决策 #71 §4 R132 era 计划阶段 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)**:

**R132 era = 计划阶段 (per 决策 #71 §4 永久循环 4 步 = 调研 + 差距 + 计划 + 继续干)**:
- **起点**: R131 era 调研 + 差距 done (R131-1 + R131-2 + R131-3 全 done, per 决策 #73 §3.2 派活, 估 8/12 done)
- **终点**: R133 era 实施阶段开始 (per 决策 #71 §4, R133 era = 计划 → 实施)
- **核心任务**: R132 era 计划阶段 (per 决策 #71 §4)
  - **R132-1**: V1.1 release 路线图 final (派中估 done, per 决策 #71 §4 R132 era 计划阶段)
  - **R132-2**: **V2.0 release 战略路线图 (本任务, 0 改 src, 0 改 Cargo.toml, 60 min 时间盒)**
  - **R132-3+**: R132 era 计划阶段 sub-agent 派活等 R131 era 部分 done (per 16 跑中上限严守 + 不要怕复杂度)

**R132 era 派活规划表 (per 决策 #71 §4 + 决策 #64 §2.2 + 15-no-fear-complexity.md + 决策 #74 §2.3)**:

| Sub-agent | 任务 | 借鉴 | 时间盒 | 状态 |
|-----------|------|------|:-----:|:----:|
| **R132-1** | V1.1 release 路线图 final (R131-1 final 整合) | 0 借 (文档) | 30 min | 🟡 派中估 done |
| **R132-2** | **V2.0 release 战略路线图 (本任务)** | 0 借 (文档) | 60 min | 🟡 done (本报告) |
| **R132-3+** | R132 era 计划阶段 sub-agent 派活 (V1.1 release 实施路线图 + V2.0 release 调研 + ... 等) | 0 借 (文档) | 30-60 min | 📋 估 done (R131 era 部分 done 后派) |

**R132 era 派活批次规划 (per 决策 #71 §4 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)**:
- **第 1 批 (估 8/11 02:00+ 派)**: R132-1 + R132-2 派活 (per 决策 #71 §4 R132 era 计划阶段 + cron `watch-r129-era-auto-replenish-16` Section 2 派 R132-2)
- **第 2 批 (估 8/12 派)**: R132-3+ 派活等 R131 era 部分 done (per 16 跑中上限严守 + 不要怕复杂度)
- **R132 era 跑中上限**: 16 active 全 background 跑 (per 主人 0:34, 16 跑中上限严守)
- **R132 era 时间盒**: 5-10 sub-agent × 平均 30-60 min = 150-600 min (估跑 1-2 天, 2 批派满 16 上限)

### 11.2 V2.0 release 派活规划 (per 决策 #71 §4 + 决策 #74 §2.3 + 15-no-fear-complexity.md)

**V2.0 release 派活规划 (per 决策 #71 §4 + 决策 #74 §2.3 + 15-no-fear-complexity.md)**:

**V2.0 release 派活阶段 (per 决策 #71 §4 永久循环 4 步 = 调研 + 差距 + 计划 + 继续干)**:
- **V2.0 release 调研 (R135+ era 估 2027+ 1-3 月)**: 8 方向 × 2 sub-agent = 16 sub-agent (第 1 批派满 16 上限)
- **V2.0 release 差距 (R135+ era 估 2027+ 1-3 月)**: 8 方向 × 2 sub-agent = 16 sub-agent (第 2 批派满 16 上限)
- **V2.0 release 计划 (R136+ era 估 2027+ 1-3 月)**: 8 方向 × 2 sub-agent = 16 sub-agent (第 3 批派满 16 上限)
- **V2.0 release 实施 (R137+ era 估 2027+ 3-12 月)**: 8 方向 × 2-4 sub-agent = 16-32 sub-agent (第 4+ 批派满 16 上限, 持续派 30+ sub-agent)
- **V2.0 release 实战 (估 2027+ 1-3 月)**: 主人起床后手跑 V2.0 release 7 步 runbook (per R130-5 + R131-8 + R132-8 续)
- **V2.0 release 后**: V2.1 minor + V3.0 major 永久循环 (per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度)

**V2.0 release 派活批次规划 (per 决策 #71 §4 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md + 决策 #74 §2.3)**:
- **第 1 批 (16 sub-agent)**: V2.0 release 调研 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-R-1: 8 硬墙可重评方案调研 (60 min)
  - V2.0-R-2: 8 哲学锚可重建方案调研 (60 min)
  - V2.0-R-3: Cargo workspace 可重构方案调研 (60 min)
  - V2.0-R-4: 三洋葱架构升级方案调研 (60 min)
  - V2.0-R-5: 9 organ 升级方案调研 (60 min)
  - V2.0-R-6: ASI Stage 10 终极自治方案调研 (60 min)
  - V2.0-R-7: Tauri 3.0+ 升级方案调研 (60 min)
  - V2.0-R-8: 永久循环方案调研 (60 min)
  - V2.0-R-9 ~ V2.0-R-16: 8 方向差距分析 (60 min × 8)
- **第 2 批 (16 sub-agent)**: V2.0 release 计划 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-P-1 ~ V2.0-P-16: 8 方向详细 spec + 派活规划 (60-90 min × 16)
- **第 3 批 (16 sub-agent)**: V2.0 release 实施 8 方向 × 2 sub-agent = 16 sub-agent
  - V2.0-I-1 ~ V2.0-I-16: 8 方向实施 (120-180 min × 16)
- **...**: per 永久循环, 持续派 30+ sub-agent 直到 V2.0 release 实施完成

**V2.0 release 派活批次时间盒 (per R130-5 V1.1 时间盒参考 + 决策 #71 §4 + 决策 #64 §2.2 cron auto-pickup + 15-no-fear-complexity.md)**:
- **第 1 批 (V2.0 release 调研)**: 16 sub-agent × 平均 60 min = 960 min = 16 小时 (估跑 1-2 天)
- **第 2 批 (V2.0 release 计划)**: 16 sub-agent × 平均 60-90 min = 960-1440 min = 16-24 小时 (估跑 2-3 天)
- **第 3 批 (V2.0 release 实施)**: 16 sub-agent × 平均 120-180 min = 1920-2880 min = 32-48 小时 (估跑 4-6 天, per V2.0 release 实施 3-12 月时间窗, 持续派 30+ sub-agent)
- **总时间盒**: 调研 1-2 天 + 差距 1-2 天 + 计划 2-3 天 + 实施 3-12 月 (per 决策 #71 §4 永久循环 4 步)

---

## 12. V2.0 release 风险 + 决策原则 (per 决策 #74 §7 + 决策 #73 §8 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

### 12.1 V2.0 release 风险 (per 决策 #74 §7 + 决策 #73 §8 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

**V2.0 release 风险 (per 决策 #74 §7 + 决策 #73 §8 + 15-no-fear-complexity.md + 用户记忆 #7-#10)**:

| # | 风险 | 缓解 |
|---|------|------|
| **R1** | 主人 8/11 01:14 决策 3 件套理解有误 (V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构) — **缓解**: 决策 #73 §2.1-§4.1 详细解读, 决策 #74 §1 8 硬墙改写表 + §2.3 V2.0 release 详细, R132-2 V2.0 release 战略路线图 (本报告) |
| **R2** | V2.0 release 8 硬墙重评 + 8 哲学锚重建 + Cargo workspace 重构 引入新 bug — **缓解**: 0 装严守可松绑但默认严守, 仅特定场景允许 (e.g. OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式), V2.0 release 0 改 src 严守可松绑但默认严守, Mavis 自决架构 + 主人 8/11 01:14 拍板 |
| **R3** | V2.0 release 推翻 + 重建 24 LOCKED 入口签名 打破向后兼容 — **缓解**: V2.0 release 是 major release (semver 严守 2.0.0, breaking change), per 决策 #22 §2.2, V2.0 release 推翻 + 重建 24 LOCKED 入口签名 是 major bump, 跟 semver 一致 |
| **R4** | V2.0 release 8 哲学锚 推翻 + 重建 失掉原 8 哲学锚 (S-1 服务 ASI 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人经验上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装) — **缓解**: 候选方案 B (12 锚扩展) 保留原 8 哲学锚 + 扩展 4 哲学锚 (复杂不恐惧 / 最强效果 / 最厉害工程 / 维护交给未来高水平团队), 候选方案 C (全新架构) 失掉原 8 哲学锚但借脑 OpenCog / CogPrime 哲学 (per R130-6 调研), 主人 8/11 01:14 拍板 3 件套 哲学不漂移 |
| **R5** | V2.0 release OpenCog AGPL-3.0 fork 实施 引入 AGPL-3.0 传染风险 — **缓解**: per R130-6 调研, apeireth 已经是 Apache 2.0, AGPL-3.0 借脑 0 装严守可松绑但默认严守, 仅 fork-then-borrow 模式允许 (e.g. OpenCog AGPL-3.0 fork → 修改 → 借鉴模式), 0 装严守 100% (per 决策 #33 §2.3 C2) |
| **R6** | V2.0 release Cargo workspace 重构 引入新 bug (24 LOCKED → 12 module + 24 micro-crate) — **缓解**: 0 改 src 严守可松绑但默认严守, 24 LOCKED 入口签名 0 改严守 (V2.0 release 24 → 12/24/36/... 重构是 entry point 重构, 不是 src 内部 fn 改动, per 决策 #74 §2.3) |
| **R7** | V2.0 release 三洋葱架构升级 失掉三洋葱架构 — **缓解**: 候选方案 A (四洋葱) 保留原三洋葱 + 加智能涌现层, 候选方案 B (五洋葱) 保留四洋葱 + 加自我演化层, 候选方案 C (全新架构) 失掉三洋葱但借脑 ASI Stage 10 架构, 主人 8/11 01:14 拍板 3 件套 架构不漂移 |
| **R8** | V2.0 release 9 organ 代码升级 失掉 9 organ 拟人化 — **缓解**: 候选方案 A (12 organ) 保留原 9 organ + 加涌现 / 自演化 / 群体, 候选方案 B (全新架构) 失掉 9 organ 拟人化但借脑 OpenCog / CogPrime organ, 用户记忆 #5 拟人化 + 拟物化 哲学不漂移 |
| **R9** | V2.0 release ASI Stage 10 终极自治 借脑 OpenCog / CogPrime 实施风险 — **缓解**: per R130-6 调研, 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex, AGPL-3.0 fork-then-borrow 模式, ASI Stage 1-9 整合, 长程 AI 成长平台, 15-no-fear-complexity.md 复杂不是问题 |
| **R10** | V2.0 release Tauri 3.0+ 升级 等 Tauri 3.0 出 — **缓解**: per 决策 #73 §2.2, Tauri 3.0 (如果 2027+ 出, V2.0 release 升级, 如果 2027+ 不出, V2.0 release 仍 Tauri 2.0+ 升级 (12 nav + 12 organ 拟人化 + 跨平台 + 用户测试)) |
| **R11** | V2.0 release 永久循环 V2.1 minor + V3.0 major ... 持续演化 — **缓解**: per 决策 #71 §4 永久循环 4 步 + 15-no-fear-complexity.md 维护交给未来高水平团队, V2.0 release 后 V2.1 minor + V3.0 major 永久循环, 自然会有高水平团队接手维护 |
| **R12** | V2.0 release 复杂度爆炸 — **缓解**: per 15-no-fear-complexity.md, 复杂度不是问题, 复杂度是实力的体现, 最强效果 > 最简单代码, 最厉害工程 > 最易维护, 维护交给未来高水平团队 |
| **R13** | V2.0 release 维护复杂 — **缓解**: per 15-no-fear-complexity.md, 维护不是问题, 维护交给未来高水平团队, 自然会有高水平团队接手维护 |
| **R14** | V2.0 release 30+ sub-agent 资源竞争 — **缓解**: per 决策 #64 §2.2 cron auto-pickup + 主人 0:34 16 跑中上限, 16 跑中上限严守, 2 批 8+8 派满 16 上限, 错开时间盒 (per R130-5 派活规划) |
| **R15** | V2.0 release 0 装 PASS 严守 松绑导致 0 装严守 violation — **缓解**: V2.0 release 0 装严守可松绑但默认严守, 仅特定场景允许 (e.g. OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式), 0 装严守 100% (per 决策 #33 §2.3 C2) |
| **R16** | V2.0 release Cargo.toml 2.0.0 major bump 0 严守 — **缓解**: per 决策 #22 §2.2 semver 严守, V2.0 release 1.1.0 → 2.0.0 major bump (breaking change), Cargo.toml 2.0.0 严守 semver |

### 12.2 V2.0 release 决策原则 (per 决策 #74 §7.2 + 决策 #73 §8.2 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

**V2.0 release 决策原则 (per 决策 #74 §7.2 + 决策 #73 §8.2 + 15-no-fear-complexity.md + 用户记忆 #7-#10)**:

**核心原则**:
- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权 + 决策 #73 §1)
- **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑)
- **中断接手** (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- **编译产物清理决策矩阵** (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- **计划内任务完成自动接续 4 步 + 永久循环** (per 主人 0:57 + 决策 #71 §4: 调研 + 差距 + 计划 + 继续干 → 永久)
- **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构)
- **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增, V2.0 release 架构升级方案是永久工作项)
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, 15-no-fear-complexity.md, V2.0 release 复杂不是问题, 维护交给未来高水平团队)

**8 硬墙严守 + B1 改写 + V2.0 release 可重评**:
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + **V2.0 release 可重评 (推翻 + 重建, 24 → 0/12/24/36/...)**
- **B2 workspace.version**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.1.0 + **V2.0 release 1.1.0 → 2.0.0 major bump**
- **A1 R11 baseline 3 值**: V1.0 release 0 改严守 + V1.1 release 严守 + **V2.0 release 可重评 (新 baseline)**
- **A3 12 键 + PHL-07**: V1.0 release 12 键 + PHL-07 严守 + V1.1 release PHL-07 实施 + 12 键其他可改 + **V2.0 release 可重评 (12 → 13 → 14/0/...)**
- **B3 V0.5 30 维**: V1.0 release 30 维严守 + V1.1 release 严守 + **V2.0 release 可重评 (30 → 0/40/...)**
- **B4 6 重守门 v7**: V1.0 release 6 重严守 + V1.1 release 严守 + **V2.0 release 可重评 (6 → 0/10/...)**
- **B5 8 哲学锚**: V1.0 release 8 锚严守 + V1.1 release 严守 + **V2.0 release 可重评 (核心变化, 8 → 0/12/..., 全新架构)**
- **C1 0 主动 commit**: V1.0 release 0 主动 commit 严守 (主人起床前) + V1.1 release 严守 + **V2.0 release 可重评 (Mavis 自动 commit + push)**
- **C2 0 装 PASS 严守**: V1.0 release 0 装严守 + V1.1 release 严守 + **V2.0 release 可重评 (允许装特定包, e.g. OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式)**
- **0 push**: V1.0 release 0 主动 push 严守 (主人起床前) + V1.1 release 严守 + **V2.0 release 可重评 (Mavis 自动 push)**

**流程严守**:
- **整合 #N commit 由 Mavis 自决拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5, 整合 #5/6/7/8/9/... commit 拍板)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6, V1.0/V1.1 release 严守, V2.0 release 可重评)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (V2.0 release 可重评)**:
- **8 哲学锚 (思想)**: 服务 ASI 北极星 + 实事求是 + 质量工程化 + 安全优先 + 走在前人经验上 + 干到底 + 任何人都能接手 + 不假装
- **不要怕复杂度 (工程)**: 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队
- **V2.0 release 8 哲学锚可重建** (per 决策 #74 §2.3, 0 锚 [无哲学] / 12 锚 [扩展] / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统])

**8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 (V2.0 release 可重评)**:
- **8 硬墙严守 (底线)**: V0.5 30 维 / 6 重守门 v7 / 8 哲学锚 / R11 baseline / 12 键 + PHL-07 / 0 装 / 0 commit (主人起床前) / 0 push (主人起床前) / 24 LOCKED 入口签名 (V1.0 release)
- **不要怕复杂度上限 (上限)**: 24 LOCKED 入口签名 (V1.1 release Mavis 自决改) + 借鉴源 12 源 (OpenCog AGPL-3.0 fork 决策) + ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + Cargo workspace 重构 + **V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + 永久循环**

**8 哲学锚 + 不要怕复杂度哲学 严守 (V1.0/V1.1/V2.0 release 严守)**:
- **V1.0 release 0 改 src 严守** (R11 baseline) + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml 1.2.0 + PHL-07 spec-only
- **V1.1 release 24 LOCKED 入口签名 Mavis 自决改** (前提: 更好的架构) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml 1.2.1
- **V2.0 release 8 硬墙 可重评 + 8 哲学锚 可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + Cargo.toml 2.0.0** (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)

---

## 13. 总结 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

### 13.1 V2.0 release 一句话 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

**V2.0 release = V1.1 release (估 2026-11-30) 之后的下一个 major release (per semver 严守 2.0.0 major bump, 估 2027+ 远期, 决策 #71 §4 永久循环 4 步 + ROADMAP.md §4 + R119-2 思想层保留), 8 大方向 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2.2 更好的架构 + R130-6 借鉴源 12 源调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md 复杂不恐惧哲学)**: ① **8 硬墙 可重评** (B1 24 LOCKED 入口签名 推翻 + 重建 [24 → 0/12/24/36/...] + B2 workspace.version 1.2.1 → 2.0.0 major bump + A1 R11 baseline 3 值 可重评 [新 baseline, 跟 R12 测度对齐] + A3 12 键 + PHL-07 可重评 [12 → 13 → 14/0/...] + B3 V0.5 30 维 可重评 [30 → 0/40/...] + B4 6 重守门 v7 可重评 [6 → 0/10/...] + B5 8 哲学锚 可重评 [**核心变化**, 8 → 0/12/...] + C1 0 主动 commit 可重评 [Mavis 自动 commit + push] + C2 0 装 PASS 可重评 [允许装特定包]) ② **8 哲学锚 可重建** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 → 0 锚 [无哲学] / 12 锚 [扩展] / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统]) ③ **Cargo workspace 可重构** (当前 30+ crate → 24 LOCKED 入口重构 [12 module + 24 micro-crate, 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex 实施 AGPL-3.0 fork, per 决策 #73 §2.2 + R130-6 调研]) ④ **三洋葱架构升级** (当前 原则 + 权限 + DSL → 四洋葱 [+ 智能涌现] / 五洋葱 [+ 自我演化] / 全新架构) ⑤ **9 organ 代码升级** (当前 body / brain / ear / eye / hand / heart / memory / mind / voice → 12 organ [+ 涌现 / 自演化 / 群体] / 全新架构) ⑥ **ASI Stage 10 终极自治** (当前 ASI Stage 9 长程 AI 成长 → ASI Stage 10 终极自治 [V2.0 release 核心, 借脑 OpenCog / CogPrime + ASI Stage 1-9 整合 + 长程 AI 成长平台]) ⑦ **Tauri 3.0+ 升级** (当前 Tauri 2.0 + 5 nav + 9 organ 拟人化 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试) ⑧ **永久循环** (V2.0 release → V2.1 minor → V3.0 major → ... [永久演化, per 决策 #71 §4 + 不要怕复杂度哲学]).

### 13.2 V2.0 release 跟 V1.0 / V1.1 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1+#2.3 + 决策 #22 §2.2)

**V2.0 release 跟 V1.0 / V1.1 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1+#2.3 + 决策 #22 §2.2)**:
- **V1.0 release** (估 8/11): 0 改 src 严守 (R11 baseline) + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml 1.2.0 → 1.0.0 + PHL-07 spec-only
- **V1.1 release** (估 2026-11-30): 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml 1.0.0 → 1.1.0
- **V2.0 release** (估 2027+ 远期): **8 硬墙 可重评 + 8 哲学锚 可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + Cargo.toml 1.1.0 → 2.0.0** (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)

### 13.3 V2.0 release 时间窗口 + 16 跑中上限 (per 决策 #71 §4 + 主人 0:34 + 决策 #64 §2.2 + 15-no-fear-complexity.md)

**V2.0 release 时间窗口 + 16 跑中上限 (per 决策 #71 §4 + 主人 0:34 + 决策 #64 §2.2 + 15-no-fear-complexity.md)**:
- **V2.0 release 时间窗口**: V1.1 release 拍板 → 1-3 月后 V2.0 release 拍板 (major 版本, 长期规划, per 决策 #71 §4 永久循环 4 步 + 决策 #74 §2.3 V2.0 release + ROADMAP.md §4 + R119-2 思想层保留)
- **16 跑中上限**: 30+ sub-agent 实施 (per V1.1 release 经验, 5-10 per 方向 × 8 方向 = 30-80 sub-agent, 16 跑中上限严守, 2 批 8+8 派满 16 上限)
- **永久循环**: V2.0 release → V2.1 minor → V3.0 major → ... (永久演化, per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学)

### 13.4 V2.0 release 风险 + 决策原则 (per 决策 #74 §7 + 决策 #73 §8 + 15-no-fear-complexity.md + 用户记忆 #7-#10)

**V2.0 release 风险 + 决策原则 (per 决策 #74 §7 + 决策 #73 §8 + 15-no-fear-complexity.md + 用户记忆 #7-#10)**:
- **风险**: 16 项风险 (R1-R16), 缓解: 决策 #73 §2.1-§4.1 详细解读, 决策 #74 §1 8 硬墙改写表 + §2.3 V2.0 release 详细, R132-2 V2.0 release 战略路线图 (本报告)
- **决策原则**: 8 硬墙严守 + B1 改写 + V2.0 release 可重评 + 8 哲学锚严守 + 不要怕复杂度哲学 严守 + 整合 #N commit 由 Mavis 自决拍板 + 0 主动 push 严守 (V1.0/V1.1 release) + 0 主动 IM 主人 (仅 done notification) + 0 主动删 + 决策日志写 + 8 哲学锚 + 不要怕复杂度哲学 = 9 件套 总哲学 + 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界

### 13.5 V2.0 release R132 era 派活规划 (per 决策 #71 §4 + 决策 #64 §2.2 + 15-no-fear-complexity.md)

**V2.0 release R132 era 派活规划 (per 决策 #71 §4 + 决策 #64 §2.2 + 15-no-fear-complexity.md)**:
- **R132 era 派活规划** (V2.0 release 战略路线图, per 决策 #71 §4 R132 era 计划阶段):
  - **R132-1**: V1.1 release 路线图 final (派中估 done)
  - **R132-2**: **V2.0 release 战略路线图 (本任务, 0 改 src, 0 改 Cargo.toml, 60 min 时间盒)** ✅ done
  - **R132-3+**: R132 era 计划阶段 sub-agent 派活等 R131 era 部分 done (per 16 跑中上限严守 + 不要怕复杂度)
- **V2.0 release 派活规划** (估 2027+ 远期, per 决策 #71 §4 + 决策 #74 §2.3 + 15-no-fear-complexity.md):
  - **V2.0 release 调研** (R135+ era 估 2027+ 1-3 月): 8 方向 × 2 sub-agent = 16 sub-agent
  - **V2.0 release 差距** (R135+ era 估 2027+ 1-3 月): 8 方向 × 2 sub-agent = 16 sub-agent
  - **V2.0 release 计划** (R136+ era 估 2027+ 1-3 月): 8 方向 × 2 sub-agent = 16 sub-agent
  - **V2.0 release 实施** (R137+ era 估 2027+ 3-12 月): 8 方向 × 2-4 sub-agent = 16-32 sub-agent (持续派 30+ sub-agent)
  - **V2.0 release 实战** (估 2027+ 1-3 月): 主人起床后手跑 V2.0 release 7 步 runbook

### 13.6 V2.0 release 决策链更新 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)

**V2.0 release 决策链更新 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 15-no-fear-complexity.md)**:
- **决策 #79+** (R132 era 计划阶段, 估 8/11+): R132 era 计划阶段 sub-agent 派活 (R132-1 V1.1 release 路线图 final + R132-2 V2.0 release 战略路线图 + R132-3+ ...)
- **决策 #80+** (R133 era 实施阶段, 估 2026 9-10 月): V1.1 release 实施阶段 sub-agent 派活 (per R131-3 实施路线图 + R132-1 final 整合)
- **决策 #81+** (R134 era 整合阶段, 估 2026 11 月): 整合 #6 commit 拍板 (Mavis 自决, per 决策 #33 C1)
- **决策 #82+** (R135 era V1.2 计划阶段, 估 2026 12 月): V1.2 路线图 (per R129-29 §5)
- **决策 #83+** (R136 era V1.2 实施阶段, 估 2027 1-2 月): V1.2 release 实施 (per R129-29 §5)
- **决策 #84+** (R137+ era V2.0 release 调研 + 差距 + 计划 + 实施, 估 2027+ 远期): V2.0 release 调研 + 差距 + 计划 + 实施 (per **R132-2 V2.0 release 战略路线图 (本报告)**)
- **决策 #85+** (R138+ era V2.0 release 实战, 估 2027+ 远期): V2.0 release 实战 (整合 #N commit 拍板 + 7 步流程 + 8 步 verify + git push + v2.0.0 tag + GitHub Pages 重新部署)
- **决策 #86+** (R139+ era V2.0 release 后, 估 2027+ 远期): V2.1 minor + V3.0 major 永久循环 (per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度)

### 13.7 一句话 (再次强调)

**V2.0 release = V1.1 release (估 2026-11-30) 之后的下一个 major release (per semver 严守 2.0.0 major bump, 估 2027+ 远期, 决策 #71 §4 永久循环 4 步 + ROADMAP.md §4 + R119-2 思想层保留), 8 大方向 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2.2 更好的架构 + R130-6 调研 OpenCog AGPL-3.0 fork 决策 + 15-no-fear-complexity.md 复杂不恐惧哲学): 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + 永久循环. 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, V2.0 release 0 装严守可重评但默认严守, 仅 OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式允许) + 8 硬墙 0 越界 100% (per 决策 #33 §2.3, V2.0 release 8 硬墙可重评但 V1.0 release 严守 100% + V1.1 release B1 可改) + 不要怕复杂度哲学严守 (per 15-no-fear-complexity.md, 复杂度是实力的体现, 维护交给未来高水平团队) + Cargo.toml 2.0.0 major bump 严守 semver (per 决策 #22 §2.2, breaking change). R132 era 派活规划 (V2.0 release 战略路线图, per 决策 #71 §4): R132-1 V1.1 release 路线图 final (派中估 done) + R132-2 V2.0 release 战略路线图 (本任务, 0 改 src, 0 改 Cargo.toml, 60 min 时间盒) + R132-3+ 派活等 R131 era 部分 done.**

---

## 14. refs

- `reports/decision-73-locked-unlocked-architecture-audit-philosophy-extension-2026-08-11.md` (主人 8/11 01:14 拍板 3 件套)
- `reports/decision-74-8-hard-walls-b1-rewrite-v1-0-0-改-v1-1-自决-2026-08-11.md` (8 硬墙 B1 改写, V2.0 release 8 硬墙可重评)
- `docs/conventions/15-no-fear-complexity.md` (R130 era 主人 8/11 01:14 拍板, 复杂不恐惧哲学)
- `docs/conventions/10-locked.md` (R119-3a-1 8 项形式撤销, 原意保留 + 决策 #74 §1 B1 改写)
- `docs/conventions/09-anchor.md` (8 哲学锚 S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)
- `reports/agent-r130-5-v1.1-minor-release-roadmap-2026-08-11.md` (R130-5 V1.1 minor release 路线图)
- `reports/agent-r129-12-r129-roadmap-2026-08-11.md` (R129 era 战略路线图, 61.1 KB)
- `reports/agent-r129-29-r130-roadmap-final-2026-08-11.md` (R130 era 路线图 final, 含 V1.1 §4 + V1.2 §5 详细)
- `reports/agent-r130-6-borrowed-12-sources-research-2026-08-11.md` (R130-6 借鉴源 12 源调研, OpenCog AGPL-3.0 fork 决策)
- `reports/agent-r130-2-asi-stage-8-integration-deepening-2026-08-11.md` (R130-2 ASI Stage 8 集成深化, Stage 9 路线)
- `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md` (R130-3 Tauri Stage 5 集成深化)
- `reports/agent-r130-4-formal-proof-stage-5.5-integration-deepening-2026-08-11.md` (R130-4 形式化 Stage 5.5 集成深化)
- `reports/agent-r130-1-integration-5-cargo-verify-2026-08-11.md` (R130-1 整合 #5 commit cargo 二次 verify)
- decision-9 (TUI 升级节奏) + decision-10 (主人离场 Mavis 自主决策) + decision-22 (24 LOCKED 自主确认 + semver) + decision-33 (8 硬墙 + 0 装 PASS) + decision-48 (整合 #4 commit abf12243) + decision-55 (R127 4 派活) + decision-56 (R127-2 10 派活) + decision-57 (R128 6 派活) + decision-58 (R128-2 3 派活) + decision-61 (R129 era 派活规划) + decision-62 (整合 #5 commit 拆 3 commit 拍板) + decision-71 (R130 era 自动接续 4 步: 调研 + 差距 + 计划 + 继续干 永久循环) + decision-72 (R130 era 调研 6 sub-agent 派活) + decision-73 (主人 8/11 01:14 拍板 3 件套) + decision-74 (8 硬墙 B1 改写, V2.0 release 8 硬墙可重评) + decision-75 (整合 #5 commit 时机 NOT ready 等 R130-1 修 bug)
- 用户记忆 #3 (用户看结果不看哲学) + #4 (AI 不会衰老病死, 只成长) + #5 (信息密度高 = 拟人化 + 拟物化) + #6 (派 sub-agent 干, 但要驾驭团队不重复造轮子) + #7 (推技术决策要守规范, 但要诚实, 砍掉装饰/无业务价值) + #8 (TUI → Tauri 终极路线) + #9 (TUI 升级节奏) + #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
- R119-2 思想层保留 (V2.0 终极路线图: 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作) + ROADMAP.md §4

---

## 15. 一句话 (TL;DR, 再次强调)

**V2.0 release = V1.1 release (估 2026-11-30) 之后的下一个 major release (per semver 严守 2.0.0 major bump, 估 2027+ 远期), 8 大方向 (per 决策 #74 §2.3 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 + 15-no-fear-complexity.md): 8 硬墙可重评 (B1 24 LOCKED 入口签名 推翻 + 重建 [24 → 0/12/24/36/...] + B2 workspace.version 1.2.1 → 2.0.0 major bump + A1 R11 baseline 3 值 可重评 [新 baseline] + A3 12 键 + PHL-07 可重评 [12 → 13 → 14/0/...] + B3 V0.5 30 维 可重评 [30 → 0/40/...] + B4 6 重守门 v7 可重评 [6 → 0/10/...] + B5 8 哲学锚 可重评 [**核心变化**, 8 → 0/12/..., 全新架构] + C1 0 主动 commit 可重评 [Mavis 自动 commit + push] + C2 0 装 PASS 可重评 [允许装特定包] + 0 push 可重评 [Mavis 自动 push]) + 8 哲学锚可重建 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 → 0 锚 [无哲学] / 12 锚 [扩展] / 全新架构 [ASI Stage 10 终极自治 + 长程 AI 成长 + AGI 操作系统]) + Cargo workspace 可重构 (30+ crate → 12 module + 24 micro-crate, 借脑 OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式, per R130-6 调研) + 三洋葱架构升级 (原则 + 权限 + DSL → 四洋葱 [+ 智能涌现] / 五洋葱 [+ 自我演化] / 全新架构) + 9 organ 代码升级 (body / brain / ear / eye / hand / heart / memory / mind / voice → 12 organ [+ 涌现 / 自演化 / 群体] / 全新架构) + ASI Stage 10 终极自治 (ASI Stage 9 长程 AI 成长 → ASI Stage 10 终极自治 [V2.0 release 核心, 借脑 OpenCog / CogPrime + ASI Stage 1-9 整合 + 长程 AI 成长平台]) + Tauri 3.0+ 升级 (Tauri 2.0 + 5 nav + 9 organ 拟人化 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试) + 永久循环 (V2.0 release → V2.1 minor → V3.0 major → ..., per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学 + 自然会有高水平团队接手维护). 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, V2.0 release 0 装严守可重评但默认严守, 仅 OpenCog AGPL-3.0 fork 实施 fork-then-borrow 模式允许) + 8 硬墙 0 越界 100% (per 决策 #33 §2.3, V2.0 release 8 硬墙可重评但 V1.0 release 严守 100% + V1.1 release B1 可改) + 不要怕复杂度哲学严守 (per 15-no-fear-complexity.md) + Cargo.toml 2.0.0 major bump 严守 semver (per 决策 #22 §2.2). V2.0 release 跟 V1.0 / V1.1 release 边界清晰: V1.0 release = 0 改 src 严守 (R11 baseline) + 8 哲学锚 + Cargo.toml 1.2.0, V1.1 release = 24 LOCKED 入口签名可改 (前提: 更好的架构) + PHL-07 实施 + Cargo.toml 1.2.1, V2.0 release = 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + Cargo.toml 2.0.0. R132 era 派活规划 (V2.0 release 战略路线图, per 决策 #71 §4): R132-1 V1.1 release 路线图 final (派中估 done) + R132-2 V2.0 release 战略路线图 (本任务, 0 改 src, 0 改 Cargo.toml, 60 min 时间盒) ✅ done + R132-3+ 派活等 R131 era 部分 done.**
