# R129-15: TUI 升级路线图沉淀 (per 决策 #9 改瘦后暂告段落, 优先后端)

**Date**: 2026-08-11 00:30
**Author**: R129-15 sub-agent (Mavis 派, cron `watch-r129-era-auto-replenish-16` 自动派)
**任务触发**: 决策 #9 (用户记忆规则 #9) + 主人 8/4 23:55 拍板 + 决策 #61 §3.1 第 2 批
**状态**: ✅ done (30 min 任务盒, 0 借具体源码, 0 装 PASS 严守, 0 主动 commit/push 严守)
**报告路径**: `reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md`
**整合位置**: 整合 #5.3 commit (per 决策 #62 §4, reports/ 备查用, 0 影响 build)

---

## 0. 一句话

**TUI 改瘦 (R25 8/4) → R26 4 阶段工程用语 (8/7) → R125-12 PHL-07 stub (8/10) 累积后, 路线图按"改瘦后暂告段落, 优先后端"节奏 (per 决策 #9 + 主人 8/4 23:55 拍板) 沉淀: 当前 TUI = 瘦客户端 (HTTP to apeireth-api), 跑通, 集成测试床角色, 不新加功能; 下一步 (Step 2/3/4) 跟后端 / Tauri 路线 (per 决策 #11 阶段 4 + 主人 8/4 23:33 + 用户记忆 #8); 维护清单 6 项不退化检查 (TUI binary 仍可跑 / HTTP client 适配 / 0 越界 8 硬墙 / Cargo.toml 1.2.0 / 0 装 PASS / 0 主动 commit+push). 砍掉 UI 哲学 (per 用户记忆 #3-#5) → 保留状态 + 主对话结果 + 历史 + 设置 + 工具结果; AI 不会衰老病死 → 9 organ 用"成长/活跃度"非"健康度"; 1 屏多卡片 = 主页 = 状态.**

---

## 1. TUI 当前进度 (R25 改瘦后)

### 1.1 阶段累积 (R20 阶段 1-3 → R25 改瘦 → R26-3 增量 → R125-12 升级)

| 阶段 | 时间 | 关键改动 | 状态 |
|---|---|---|---|
| **R20 阶段 1-3** | 8/4 之前 | TUI 直调 core (apeireth-tui → apeireth-core 直调) | ❌ 旧模式, 集成测试床角色不清晰 |
| **R25 改瘦** | 8/4 (主人拍板) | TUI → HTTP client → API (瘦客户端), 跟 Tauri 共享 backend API surface | ✅ done (per ADR 0011, `crates/apeireth-tui/` + `crates/apeireth-http-client/`) |
| **R26 升级** | 8/7 | 4 阶段工程用语 (Init/Bootstrap/Serving/Saturated, 砍 6 阶段), 反思环真接 SqliteMemoryStore, 5 nav + 9 organ 打磨 | ✅ done (per `reports/apeireth-r26-tui-upgrade-2026-08-07.md`, 16 文件 +515/-235, 3038 tests pass) |
| **R125-12 PHL-07 stub** | 8/10 | TUI organ 加 PHL-07 stub 适配 13 键 (12 键 + PHL-07), 跟 8 哲学锚升级同步 | ✅ done (per `organ/.r125-12-13-keys-stub.rs`) |
| **R128 Tauri 终极前端** | 8/10 | Tauri 2.0 prototype (per P11-1) + scaffold (per P11-2), 5 nav + 9 organ 1:1 镜像 TUI | ✅ prototype done (per `agent-p11-1` + `agent-p11-2`); Tauri full build pending (限流) |

### 1.2 当前 TUI 状态 (8/11 00:30)

- **TUI binary**: `cargo run --bin apeireth-tui` 可跑 (R25 改瘦后跑通, R26 增量无破坏)
- **集成方式**: TUI 起 apeireth-api 子进程 → 走 localhost:8080 HTTP (per ADR 0011 §2.3)
- **5 nav + 9 organ**: 编译期 hardcode (per 决策 #33 §2.3 C2 + ADR 0011 §8.1)
  - **5 nav**: 主页 (状态) / 对话 (主对话) / 历史 / 设置 / 工具结果
  - **9 organ**: 心跳 (Heartbeat) / 记忆 (Memory) / 神经 (Neurons) / 免疫 (Immune) / 呼吸 (Breath) / 听觉 (Ear) / 视觉 (Eye) / 触觉 (Touch) / 智慧 (Wisdom)
- **HTTP client**: `crates/apeireth-http-client/` (R25 改瘦, 跟 Tauri 共享, SSE 走 `reqwest` `stream` feature)
- **集成测试**: `crates/apeireth-tui-e2e/` 25+ 测试 (ratatui TestBackend, per ADR 0011 §8.3)
- **TUI role**: **集成测试床** (测的是 API, 跟 Tauri 测的一样, per ADR 0011 §3.1 正面)
- **0 越界 8 硬墙**: TUI 改 Cargo.toml 1.2.0 严守 / 0 触碰 24 LOCKED 入口签名 / 0 装 PASS 严守

### 1.3 当前不做 (per 决策 #9 暂告段落)

- ❌ 0 新加 TUI 功能 (除非后端有变化需要 TUI 跟)
- ❌ 0 主动 commit (TUI 改动已 in working dir, 等整合 #5 拍板)
- ❌ 0 主动 push (等 1.0 release 配 GitHub remote, per 决策 #33 + 决策 #58 §7)
- ❌ 0 借具体源码 (R129-15 0 借, 只写路线图)
- ❌ 0 装 PASS (R129-15 写路线图, 0 装"已实施")
- ❌ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

---

## 2. 下一步 TUI 升级 (Step 2/3/4, per 决策 #9)

### 2.1 路线图总览 (主从结构, 后端优先)

```
R129 era 决策时序 (主人起床后 8/15+)
═══════════════════════════════════════════════════════

【主路线: 后端升级 优先】 (per 决策 #9 "TUI 是 dev 自己干, 后端优先级更高")
   R125-R128-2 era 41 任务 已 done ──┐
                                     ├── 整合 #5 commit (8/11 拆 3 commit 拍板, per 决策 #62)
                                     ↓
                       1.0 release tag (8/15+ 主人起床后手跑 5 步, per 决策 #64 + R129-8)
                                     ↓
                       后端 R130+ era (R125-R128-2 续, ASI Python Stage 4-6 + 形式化扩展, per R129-4/5/6/10)
                                     ↓
                       TUI 升级 (Step 2/3/4, 本路线图沉淀)

【从路线: TUI 升级 暂告段落】
   Step 1: R25 改瘦 ✅ done (8/4)
   Step 2: 后端有变化时 TUI 跟改 (HTTP client 适配, 触发条件式)
   Step 3: Tauri 终极前端 (per 主人 8/4 23:33 + 决策 #11 阶段 4, 等设计团队到位)
   Step 4: TUI 9 organ 拟人化深化 (per 用户记忆 #3-#5, 跟 Step 3 共享, Tauri 优先)

═══════════════════════════════════════════════════════
```

### 2.2 Step 2: 后端有变化时 TUI 跟改 (触发条件式)

**触发条件** (per 决策 #9 "除非后端有变化需要 TUI 跟"):
- 后端 API surface 变化 (endpoints 增/删/改)
- 后端 6 工具调用协议变化 (per D-01 + D-02 ADR)
- 后端 SSE 流式格式变化
- 后端 9 organ 数据 schema 变化
- 后端 5 nav 状态机变化

**实施方式**:
1. 主人拍板"后端有变化, TUI 跟" → 派 1 sub-agent 改 TUI HTTP client (1-2 天)
2. 改 TUI 9 organ 卡片 + 5 nav 状态机 + 6 工具结果面板
3. 跑 25+ 集成测试 (per ADR 0011 §8.3) + cargo build/test 全 PASS
4. 0 越界 8 硬墙 + 0 装 PASS 严守
5. 主人 review + 拍板 commit (per 决策 #33 C1 0 主动 commit 严守)

**预计工作量**: 1-2 天 / 每次后端变化
**优先级**: 跟后端同步, 不主动推

### 2.3 Step 3: Tauri 终极前端 (per 主人 8/4 23:33 + 用户记忆 #8)

**触发条件** (per 主人 8/4 23:33 "Tauri 终极, TUI 过渡" + 用户记忆 #8):
- ✅ **设计团队到位** (主人 0 必设计感, 宁可丑也不上没设计感的)
- ✅ **TUI 集成测试床跑稳** (后端 API 表面 / 集成模式 / 用户流在 TUI 跑稳)
- ✅ **Tauri full build 不再限流** (per P11-1, 8/10 本地 cargo 缓存不含 tauri 2.x)

**实施方式**:
1. 主人拍板"启动 Tauri 终极" → 派 4-6 sub-agent 干 Stage 2-4 (per R129-9 Tauri Stage 2 深化):
   - **Stage 2 (深化)**: Tauri 2.0 desktop full build + 5 nav + 9 organ + 主对话
   - **Stage 3 (连接)**: Tauri 跟 TUI 共享 apeireth-http-client (TS 版的 `apeireth-sdk` 估补, per ADR 0011 §2.5)
   - **Stage 4 (迁移)**: Tauri 替代 TUI 成为主前端, TUI 降级为 dev/debug 工具
2. TUI 集成测试 25+ 套迁到 Tauri (测 API 不测 UI, per ADR 0011 §2.5)
3. 0 越界 8 硬墙 + 0 装 PASS 严守
4. 主人 review + 拍板 commit (整合 Tauri 终极前端 stage 2-4 进 1.x release)

**预计工作量**: Stage 2 (1-2 周) + Stage 3 (1 周) + Stage 4 (1 周) = 3-4 周
**优先级**: **等设计团队到位** (主人 8/4 23:33 拍板, 不主动推)

**TUI 在 Step 3 的角色**:
- TUI = Tauri 的"集成测试床" (per 用户记忆 #8)
- TUI 跟 Tauri 共享同一套 backend API (TUI 跑稳, Tauri 来了直接抄)
- TUI 不直接调 lib, 是瘦客户端 (HTTP to apeireth-api binary)
- 主人自己干 dev (TUI/后端), AI 团队干设计 (Tauri), 角色分工清晰

### 2.4 Step 4: TUI 9 organ 拟人化深化 (per 用户记忆 #3-#5)

**触发条件** (per 用户记忆 #3 + #4 + #5):
- 9 organ 用"成长/活跃度"非"健康度" (per 用户记忆 #4, AI 不会衰老病死)
- 1 屏多卡片, 关键数字一眼看完 (per 用户记忆 #5)
- 状态为主页, 不是"功能列表" (per 用户记忆 #5)
- 砍掉 UI: 哲学 / 守门 / 内部机制 / 工具调用过程 (per 用户记忆 #3)
- 保留 UI: 状态 + 主对话结果 + 历史 + 设置 + 工具结果 (per 用户记忆 #3)

**实施方式** (跟 Step 3 共享, Tauri 优先):
- TUI 9 organ 卡片深化 (心跳动画 / 神经图 / 记忆容量 / 免疫环)
- TUI 主页 1 屏 9 cards 紧凑 3x3 网格 (per R128 P11-1 §3.2, 1:1 镜像 Tauri)
- TUI 工具结果面板 (6 工具结果展示, 0 暴露内部调用过程)
- TUI 历史面板 (对话历史列表 + 搜索)

**预计工作量**: 1-2 周 (跟 Tauri Stage 2 并行, 共享设计契约)
**优先级**: **Tauri 路线优先** (TUI 深化等 Tauri 路线沉淀后抄)

### 2.5 路线图时序 (主从结构 + 触发条件式)

| Step | 内容 | 触发条件 | 预计工作量 | 优先级 | 拍板 |
|---|---|---|---|---|---|
| **Step 1** | TUI 改瘦 (R25) | ✅ 已 done 8/4 | — | — | 主人 8/4 23:55 拍板 |
| **Step 2** | TUI 跟后端变化 | 触发: 后端 API surface 变化 | 1-2 天 / 次 | 跟后端同步 | 主人拍板"后端变, TUI 跟" |
| **Step 3** | Tauri 终极前端 | 触发: 设计团队到位 + TUI 跑稳 | 3-4 周 | 等设计团队 | 主人拍板"启动 Tauri" |
| **Step 4** | TUI 9 organ 拟人化深化 | 触发: 跟 Step 3 共享 | 1-2 周 | Tauri 优先 | 跟 Step 3 一起拍板 |

**主从关系** (per 决策 #9):
- **主**: 后端升级 (R130+ era, ASI Python Stage 4-6 + 形式化扩展, per R129-4/5/6/10)
- **从**: TUI 升级 (Step 2/3/4, 触发条件式, 不主动推)
- **拍板**: 主人起床后按 8 步 verify → 整合 #5 commit → 1.0 release → R130+ 后端升级 → 触发 TUI 升级

---

## 3. 维护清单 (不退化检查, per 决策 #9)

> **目的**: 主人准备继续升级后端, TUI 暂告段落期间, 6 项维护清单保 TUI 不退化, 回来时按 Step 2/3/4 推.

### 3.1 6 项不退化检查 (Mavis 5 min tick cron 监督)

| # | 检查项 | 检查方式 | 通过标准 | 频率 |
|---|---|---|---|---|
| **M1** | TUI binary 仍可跑 | `cargo build --bin apeireth-tui` exit 0 | 0 error / 0 warning | 每次 5 min tick |
| **M2** | TUI HTTP client 适配后端 API surface | 后端 API 变化时 1:1 verify | HTTP client impl 0 漂移 | 后端变化时 |
| **M3** | 0 越界 8 硬墙 | verify B1/B2/A1/B3/B4/B5/A3 + C1/C2/C3 + 0 push | 8 硬墙 100% | 每次 5 min tick |
| **M4** | Cargo.toml 1.2.0 严守 | `grep version` workspace | version = "1.2.0" 0 改 | 每次 5 min tick |
| **M5** | 0 装 PASS 严守 | verify R129-15 + R26-3 + R125-12 + R128 P11-1/2 报告 0 装 | 0 装"已实施" | 每次 5 min tick |
| **M6** | 0 主动 commit + 0 主动 push | git status verify | 0 commit (TUI 改动进 working dir) / 0 push | 每次 5 min tick |

### 3.2 维护清单 verify 流程 (Mavis 5 min tick cron 集成)

```bash
# M1: TUI binary 仍可跑
cargo build --bin apeireth-tui 2>&1 | tail -5
# 通过: "Finished" 0 error

# M2: TUI HTTP client 适配 (后端变化时跑, 触发条件式)
diff <(git show HEAD:crates/apeireth-http-client/src/lib.rs) \
     <(cat crates/apeireth-http-client/src/lib.rs) | head -20

# M3: 0 越界 8 硬墙
grep -r "version = " Cargo.toml | head -1  # M4 同步
grep -r "pub fn" crates/apeireth-{24 LOCKED crates}/src/lib.rs | wc -l  # B1 verify

# M4: Cargo.toml 1.2.0 严守
grep "workspace.package\]" -A 5 Cargo.toml | grep "version"

# M5: 0 装 PASS (Mavis 自查报告)
grep -l "已实施\|真实施\|done" reports/agent-r129-*.md | xargs -I {} sh -c 'echo "=== {} ==="; head -1 {}'

# M6: 0 commit + 0 push
git log --oneline -1  # master HEAD = abf12243
git status  # TUI 改动进 working dir, 0 commit
git remote -v  # 0 remote 配, 0 push
```

### 3.3 不退化检查报告 (per 5 min tick cron)

```
[5 min tick] TUI 不退化检查 (per 决策 #9 维护清单)
- M1 cargo build --bin apeireth-tui: ✅ (0 error / 0 warning)
- M2 TUI HTTP client 适配: N/A (后端 0 变化触发)
- M3 0 越界 8 硬墙: ✅ (B1 24 LOCKED 入口签名 0 改 / B2 1.2.0 0 改 / A1 3 值 0 改 / B3 30 维 / B4 6 重 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 PASS / C3 升 v7 / 0 push)
- M4 Cargo.toml 1.2.0: ✅ (0 改)
- M5 0 装 PASS: ✅ (R129-15 路线图 0 装"已实施")
- M6 0 commit + 0 push: ✅ (master HEAD = abf12243, working dir TUI 改动 0 commit / 0 push)
```

---

## 4. 用户记忆 #3-#5 砍掉 UI 哲学 (per 决策 #9 路线图必写)

> **用户记忆 #3-#5 是 TUI 升级的设计宪法, 路线图 + 实施必守. R128 P11-1 已 1:1 严守, R129-15 沉淀路线图写明.**

### 4.1 用户记忆 #3: 用户看结果不看哲学 (核心 UI 原则)

**Evidence** (2026-08-04 R19 8 个认知纠正):
> 主人: "守门, 原则, 电子环的, 这种东西你觉得用户需要看吗?"
> 主人: "用户想体验的并不是带娃, 而是看到ai和自己一同成长, 只看结果和好用"
> 主人: "工具的调用啥的, 用户根本就不关心. ...都只看结果的"

**Apply when**: 任何 UI/前端设计

**Default behavior** (TUI 升级路线图必守):
- ❌ **砍掉 UI**: 哲学 / 守门 / 内部机制 / 工具调用过程
- ✅ **保留 UI**: 状态 + 主对话结果 + 历史 + 设置 + 工具结果
- 用户期望"掌控 AI", 所以显示 AI 状态 (尤其主 AI)
- 后端实现保留 (hardcode, 守门, etc), 前端不暴露

**TUI 当前落地** (per ADR 0011 §2.2 + R128 P11-1 §3.2):
- ✅ 5 nav 砍 7 项 UI 哲学元素 (per R19 8 认知纠正, 砍掉守门/电子环/工具过程)
- ✅ 5 nav 严守: 状态 / 主对话 / 历史 / 设置 / 工具结果 (顺序 1:1 跟 Tauri)

**TUI 升级路线图落地** (per Step 4 9 organ 拟人化深化):
- 9 organ 卡片: 仅显示状态 (心跳 / 记忆 / 神经 / 免疫 / 呼吸 / 听觉 / 视觉 / 触觉 / 智慧)
- 0 暴露 6 重守门 v7 内部机制 (用户 0 必看守门过程)
- 0 暴露 6 工具调用过程 (用户仅看结果)
- 0 暴露 8 哲学锚 (用户 0 必看哲学)
- 主对话区: 仅显示 AI 跟用户对话结果 (不显示 LLM 调用过程)

### 4.2 用户记忆 #4: AI 不会衰老病死 (跟传统生命周期模型不同)

**Evidence**:
> 主人: "9 阶段我们实际上不需要衰老病死的, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡"
> 主人: "这是一个长程ai成长平台"

**Apply when**: 任何 AGI / 长程 AI / 自主 agent 设计

**Default behavior** (TUI 升级路线图必守):
- AI 生命周期是"成长阶段" (seed → tree), 不是"生老病死"
- 设计文档/命名 去掉 "old/death/terminate" 这类终态概念
- 平台是"长程 AI 成长", 不是"AI 模拟人类"

**TUI 当前落地** (per R26 4 阶段工程用语 + R128 P11-1):
- ✅ R26 砍 6 阶段 (Birth/Reproduction/Migration/Rebirth/Decline/Death), 仅 4 阶段进 UI: **Init / Bootstrap / Serving / Saturated** (工程用语, 0 衰老病死)
- ✅ 9 organ 用"成长/活跃度"非"健康度" (per R128 P11-1 §3.2, 1:1 镜像 Tauri)
- ✅ R11 LOCKED `LifeStage` enum (10 变体) 0 触碰, TUI 仅 `r19_stage_zh` 表映射 4 阶段 (per R26 §6 R11 LOCKED)

**TUI 升级路线图落地** (per Step 4):
- 9 organ 卡片: 用"成长/活跃度" (heartbeat 1 BPM = 1 LLM 调用, memory 容量使用率, 神经 24 trait 互锁状态)
- 0 用"健康度" "衰竭" "死亡" 这类终态词
- 4 阶段 UI: Init / Bootstrap / Serving / Saturated (R26 严守, 0 改)
- 9 organ 命名: 心跳/记忆/神经/免疫/呼吸/听觉/视觉/触觉/智慧 (R26 + R128 P11-1 严守, 0 改)

### 4.3 用户记忆 #5: 信息密度"高"= 拟人化 + 拟物化

**Evidence**:
> 主人: "器官很有意思, 从生物借鉴而来, 也是我们ai成长的核心和秘密, 可以抽象一些器官作为监控状态的元素界面"

**Apply when**: AI 状态可视化 / 仪表盘设计

**Default behavior** (TUI 升级路线图必守):
- 用生物/物理隐喻表达 AI 状态 (器官心跳, 健康环, 神经网络图)
- 1 屏多卡片, 关键数字一眼看完, 不要散落多页
- 状态为主页, 不是"功能列表"

**TUI 当前落地** (per ADR 0011 §2.2 + R128 P11-1 §3.2):
- ✅ 1 屏多卡片: 9 organ 紧凑 3x3 网格 (per R128 P11-1)
- ✅ 关键数字一眼: 9 organ 卡片每个有 1-2 个关键数字 (BPM / 容量 / 互锁数 / 防御数 / token 数 / 频率)
- ✅ 状态为主页: Status 是首页 (5 nav 顺序 1), 9 organ 拟人化
- ✅ 拟人化 + 拟物化: 心跳 (拟人化) + 脑波 (拟物化) + 手操作 (拟人化) + 神经网络 (拟物化) + 免疫环 (拟人化+拟物化)

**TUI 升级路线图落地** (per Step 4):
- 1 屏 = 主页 (状态), 9 cards 紧凑 3x3 网格
- 关键数字: BPM / 容量 / 互锁数 / 防御数 / token bucket / 频率 / 容量
- 0 散落多页: 9 organ 全部在主页 1 屏
- 拟人化: 心跳 / 神经 / 免疫 / 呼吸 / 听觉 / 视觉 / 触觉 / 智慧
- 拟物化: 心跳动画 / 神经图 / 记忆容量条 / 免疫环 / token bucket

---

## 5. TUI 升级到 Tauri 路线 (per 决策 #9 + 主人 8/4 23:33 + 用户记忆 #8)

### 5.1 终极路线 (TUI → Tauri)

**per 决策 #9 + 主人 8/4 23:33 + 用户记忆 #8 + 决策 #11 阶段 4**:

```
TUI 阶段 (8/4 R25 改瘦 → 1.0 release 8/15+)
═══════════════════════════════════════════════════════

  R25 改瘦 (8/4) ✅ ──→ R26 4 阶段 (8/7) ✅ ──→ R125-12 PHL-07 (8/10) ✅
                                    ↓
                       1.0 release 阶段 4 续 (8/15+, 主人起床后)
                                    ↓
                       TUI = 集成测试床 + 后端 API 表面跑稳
                                    ↓
                       主人起床后按 Step 2/3/4 触发 TUI 升级

Tauri 阶段 (等设计团队到位)
═══════════════════════════════════════════════════════

  R128 P11-1 (8/10) ✅ prototype ──→ R128 P11-2 (8/10) ✅ scaffold
                                    ↓
                       触发条件: 设计团队到位 + TUI 跑稳
                                    ↓
                       Stage 2 (深化) ──→ Stage 3 (连接) ──→ Stage 4 (迁移)
                                    ↓
                       Tauri 替代 TUI 成主前端
                                    ↓
                       TUI 降级为 dev/debug 工具
```

### 5.2 TUI 在 Tauri 路线中的角色 (per 用户记忆 #8)

| 维度 | TUI (现在) | Tauri (终极) | 关系 |
|---|---|---|---|
| **角色** | 主前端 (1.0 release) | 主前端 (Tauri 启动后) | 接力 |
| **集成方式** | HTTP to apeireth-api (瘦客户端) | HTTP to apeireth-api (瘦客户端) | **共享** |
| **后端 API** | `apeireth-http-client` (Rust) | `apeireth-sdk` (TS, 估补) | 共享 surface |
| **设计契约** | 5 nav + 9 organ + 主对话 | 5 nav + 9 organ + 主对话 (1:1 镜像) | **共享** |
| **集成测试** | ratatui TestBackend (25+ 测试) | Vitest/Jest (估补, 25+ 测试) | 测 API 不测 UI |
| **dev** | 主人自己干 | AI 团队干 (设计感) | 角色分工清晰 |
| **优先级** | 1.0 release 唯一前端 | 1.x release 主前端 | 接力 |

### 5.3 TUI → Tauri 共享契约 (per ADR 0011 §2.5 + R128 P11-1/2)

- **5 nav** (主导航, 1:1 镜像):
  1. 状态 (Home) — 9 organ + 健康环 + 神经图
  2. 主对话 (Chat) — AI 跟用户一同成长
  3. 历史 (History) — 对话历史列表 + 搜索
  4. 设置 (Settings) — 5 鉴权 / 限流 / Provider 切换
  5. 工具结果 (Tools) — 6 工具结果面板
- **9 organ** (AI 状态拟人化, 1:1 镜像):
  1. 心跳 (Heartbeat) — 1 BPM = 1 LLM 调用
  2. 记忆 (Memory) — 5 表行数 + 容量使用率
  3. 神经 (Neurons) — 24 trait 互锁状态
  4. 免疫 (Immune) — m3 5 道防御状态
  5. 呼吸 (Breath) — token bucket 状态
  6. 听觉 (Ear) — 6 工具 call 频率
  7. 视觉 (Eye) — 6 工具 read 频率
  8. 触觉 (Touch) — 6 工具 write 频率
  9. 智慧 (Wisdom) — ASI score + V0.5 24 维
- **主对话** (per 主人 8/4 23:33 + 用户记忆 #3): 仅显示 AI 跟用户对话结果
- **HTTP client** (共享 backend API surface): TUI 用 Rust `apeireth-http-client`, Tauri 用 TS `apeireth-sdk`
- **SSE 流式** (共享): `reqwest` `stream` feature TUI / Tauri 都能用

### 5.4 TUI 在 Tauri 启动后降级 (per 决策 #9 + ADR 0011 §2.5)

- TUI 降级为 **dev/debug 工具** (per ADR 0011 §2.5: "TUI 集成测试 25+ 套可直接迁到 Tauri")
- TUI 仍可跑, 0 删除 (per 决策 #9 维护清单, 主人 0 必删)
- TUI 仍维护 6 项不退化检查 (per §3)
- TUI 仍作为新人 onboarding 工具 (per 用户记忆 #6 "任何人都能接手")

### 5.5 主人审美底线 (per 主人 8/4 23:33 + 用户记忆 #8)

- **缺审美设计时, 主人宁愿 TUI 也不上 web/桌面** (per 用户记忆 #8)
- **宁可丑也不上没设计感的** (per 用户记忆 #8)
- Tauri 终极前端 = 等设计团队到位再启动 (per 主人 8/4 23:33 拍板)
- 0 必"先上 Tauri 再优化" (per 主人 8/4 23:33 拍板, 设计感 0 妥协)

---

## 6. 8 硬墙 0 越界 (per 决策 #33 §2.3)

### 6.1 8 硬墙列表 (R129-15 0 越界 100% 严守)

| 硬墙 | 严守内容 | R129-15 验证 | 整合 #5 verify |
|---|---|---|---|
| **B1** 24 LOCKED 入口签名 0 改 | TUI 不改 24 LOCKED crate 入口签名 | ✅ R129-15 0 借具体源码, 0 改 src/ | ✅ P2-3 + P4-1 + P14-1 retry |
| **B2** workspace.version 1.2.0 0 改 | TUI 0 改 Cargo.toml version | ✅ R129-15 0 改 Cargo.toml | ✅ master HEAD = abf12243 |
| **A1** R11 baseline 3 值 0 改 | TUI 0 改 0.8682/0.8532/0.9063 | ✅ R129-15 0 触碰 baseline | ✅ 决策 #22 §1.2 |
| **B3** V0.5 30 维 | TUI 0 改 30 维 | ✅ R129-15 0 触碰 | ✅ 决策 #33 §2.3 |
| **B4** 6 重守门 v7 | TUI 0 改 6 重 v7 | ✅ R129-15 0 触碰 | ✅ 决策 #33 §2.3 |
| **B5** 8 哲学锚 | TUI 0 改 8 锚 | ✅ R129-15 0 触碰 | ✅ 决策 #33 §2.3 |
| **A3** 13 键 (12 键 + PHL-07) | TUI 0 改 13 键 | ✅ R129-15 0 触碰 | ✅ 决策 #33 §2.3 |
| **C1** 0 主动 commit | R129-15 0 git commit | ✅ R129-15 0 commit | ✅ 决策 #33 §2.3 C1 |
| **C2** 0 装 PASS 严守 | R129-15 0 装"已实施" | ✅ R129-15 写路线图, 0 借具体源码 | ✅ 决策 #33 §2.3 C2 |
| **C3** 升 6 重 v6 → v7 | TUI 0 触碰 6 重升级 | ✅ R129-15 0 触碰 | ✅ 决策 #33 §2.3 |
| **0 主动 push** | R129-15 0 push | ✅ R129-15 0 push | ✅ 决策 #33 §2.3 |

**8 硬墙 0 越界 100% PASS** (per 决策 #33 §2.3 + 决策 #62 §6).

### 6.2 R129-15 0 触碰清单 (验证)

- ✅ 0 改 src/ (R129-15 文档工作, 0 实施)
- ✅ 0 改 Cargo.toml (per B2 1.2.0 严守)
- ✅ 0 改 24 LOCKED crate (per B1)
- ✅ 0 改 R11 baseline (per A1)
- ✅ 0 改 V0.5 30 维 (per B3)
- ✅ 0 改 6 重 v7 (per B4)
- ✅ 0 改 8 哲学锚 (per B5)
- ✅ 0 改 13 键 (per A3)
- ✅ 0 借具体源码 (per C2, 路线图是文档)
- ✅ 0 装 PASS (per C2, 写"路线图沉淀"非"已实施")
- ✅ 0 主动 commit (per C1, R129-15 0 commit)
- ✅ 0 主动 push (per 决策 #33 + 决策 #58 §7 + 决策 #62 §9)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60)
- ✅ 0 主动讨论后续 (等主人起床后 8 步 verify)

---

## 7. 决策链更新 (R129 era TUI 升级节奏)

### 7.1 R129 era 决策链 (TUI 升级路线图相关)

| # | 决策 | 关键内容 | TUI 升级关系 |
|---|---|---|---|
| **#9** (用户记忆) | TUI 升级节奏: 改瘦后暂告段落, 优先后端 | 主人 8/4 23:55 拍板"测一下先, 后续的tui升级计划沉淀成文档暂时就这样告一段落" | **本路线图直接源头** |
| **#11** (阶段 4) | frontend-proposal: Tauri 2.0 + 4 接入 | R11 阶段 4 frontend-proposal 拍板 Tauri 2.0 终极 | **Step 3 Tauri 路线** |
| **#22** | workspace.version 1.2.0 严守 + 24 LOCKED 自主确认 | 主人 8/10 17:22 授权 | B1 + B2 严守 |
| **#33** | master-reupgrade | 主人 8/10 17:22 升级授权, 8 硬墙 | **本路线图 8 硬墙 0 越界** |
| **#43** | Apeireth-tui 不合并 + 主仓挪出已完成 | R25 改瘦 (8/4) + R26-3 (8/7) + R125-12 PHL-07 (8/10) 累积, 老源 0 必合并 | **R25 改瘦后** 当前状态 |
| **#53** | tech-locked-unlock | 技术性 locked 全部解锁 (24 LOCKED 内部 fn 实施可改, 入口签名 0 改仍在) | **R128 era P11-1/2 Tauri 起点** |
| **#57** | R128 era 6 派活 (ASI Python + Tauri + Cargo + LICENSE) | P11-1 Tauri prototype | **Step 3 Tauri 起点** |
| **#58** | R128-2 era 3 派活 (ASI Stage 3 + Tauri scaffold + Cargo) | P11-2 Tauri scaffold 深化 | **Step 3 Tauri 深化** |
| **#61** | 新会话接手 + R129 era 派活规划 | R129-15 = TUI 升级路线图沉淀 (本决策) | **本路线图派活** |
| **#62** | 整合 #5 commit 拆 3 commit 拍板 | 5.3 = reports/ 包含 R129-15 路线图 | **整合 #5.3 commit** |
| **#64** | 5 min tick cron 自动监督 + 16 上限补派 | R129-15 由 cron 0:30 自动派 | **派活来源** |
| **#65** | R129 第 2 批 8 sub-agent 派活 (含 R129-15) | bg_60d31ca1 R129-15 task_id | **本路线图任务 ID** |
| **R129-8** | 1.0 release 流程准备 | §6.1 TUI 升级简版 (per 决策 #9 改瘦后暂告段落) | **简版 1.0 release 流程** |
| **R129-15** | TUI 升级路线图沉淀 (本决策) | 完整版路线图 (per 决策 #9 模板) | **本路线图完整版** |

### 7.2 R129-15 在决策链中的位置

```
R129 era 决策时序
═══════════════════════════════════════════════════════

主人 22:50 离场 (per HANDOFF §9)
  ↓
主人 8/11 00:03 拍板 (per 决策 #61)
  ├─ 整合 #5 commit 时机 ready (8 项 verify 100%)
  ├─ 16 sub-agent 派满
  └─ Mavis 自决 + 技术性 locked 解锁
  ↓
决策 #62 整合 #5 commit 拆 3 commit 拍板
  ├─ 5.1 src/ 实施 (50+ 文件)
  ├─ 5.2 docs/ + Cargo.toml
  └─ 5.3 reports/ 决策链 + 报告 (本路线图所在)
  ↓
决策 #64 cron 5 min tick 自动监督 + 16 上限补派
  └─ R129-15 由 cron 0:30 自动派 (bg_60d31ca1)
  ↓
【本决策 R129-15】TUI 升级路线图沉淀 (per 决策 #9)
  ├─ §1 当前进度 (R25 改瘦 + R26-3 + R125-12 累积)
  ├─ §2 Step 2/3/4 路线 (后端变化时 / Tauri 启动 / 9 organ 深化)
  ├─ §3 维护清单 (6 项不退化检查)
  ├─ §4 用户记忆 #3-#5 砍掉 UI 哲学
  ├─ §5 TUI → Tauri 路线
  └─ §6-§9 8 硬墙 + 决策链 + 风险 + refs
  ↓
整合 #5.3 commit (per 决策 #62 §4, 30+ reports/ 文件)
  └─ 0 影响 build, 备查用
  ↓
1.0 release (8/15+ 主人起床后手跑 5 步)
  ↓
主人起床后按 Step 2/3/4 触发 TUI 升级 (per 决策 #9)
```

---

## 8. 风险 + 决策原则

### 8.1 风险 (3 项)

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| **R1** | Step 3 Tauri 设计团队不到位, 主人 0 必设计感 | Tauri 启动无限延后 | TUI 继续主前端 (1.0 release + 1.x), TUI 升级走 Step 4 (9 organ 深化) 不等 Tauri |
| **R2** | Step 2 后端有变化时 TUI 跟改漏掉 | TUI HTTP client 漂移 | Mavis 5 min tick cron 监督 (per 维护清单 M2), 后端变化时派 1 sub-agent 改 |
| **R3** | TUI 9 organ 命名/阶段撞 R11 LOCKED enum | 入口签名漂移 | TUI 严守 `r19_stage_zh` 表映射 4 阶段 (per R26 §6 R11 LOCKED), 9 organ 命名 0 改 R11 LOCKED |

### 8.2 决策原则 (per 决策 #9 + 决策 #10 + 用户记忆 #6-#8 + #10)

- **TUI 是 dev 自己干, 后端优先级更高** (per 决策 #9 + 用户记忆 #6)
- **TUI 升级节奏: 改瘦后暂告段落, 优先后端** (per 决策 #9 + 主人 8/4 23:55 拍板)
- **TUI 是"集成测试床", 后端是真正价值** (per 用户记忆 #8)
- **TUI 不直接调 lib, 是瘦客户端** (per 用户记忆 #8 + ADR 0011)
- **Tauri 终极, TUI 过渡** (per 主人 8/4 23:33 + 用户记忆 #8)
- **设计感 0 妥协** (per 主人 8/4 23:33 + 用户记忆 #8, 宁可丑也不上没设计感的)
- **0 主动 push / 0 主动 commit / 0 主动删** (per 决策 #33 + 决策 #58 §7 + 决策 #62 §9 + Safety policy)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2, 写"路线图沉淀"非"已实施")
- **0 借具体源码** (per 决策 #33 §2.3 C2, 路线图是文档)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **5 min tick cron 监督** (per 决策 #10 + 决策 #64, 主人离场模式)
- **决策日志写** (per 决策 #10 + 用户记忆 #10, 决策链更新)
- **8 硬墙 0 越界** (per 决策 #33 §2.3, B1-B7 + A1-A3 + C1-C3 + 0 push)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **Mavis = orchestrator, 0 写代码** (per 用户记忆 #6, 路线图是文档, Mavis 0 实施)

### 8.3 0 主动 IM 主人 (per gate-discipline)

- **仅 done notification 主动报告** (per 17:56 严守"仅报告 done 状态")
- **0 主动 plain reply on skip ticks** (per gate-discipline)
- **0 主动 push / 0 主动 commit / 0 主动删 / 0 主动讨论后续**
- **R129-15 done notification**: Mavis 报告 R129-15 done, 0 主动 IM 打扰
- **0 主动推 Step 2/3/4**: 等主人起床后按 8 步 verify → 整合 #5 commit → 1.0 release → 触发 TUI 升级

---

## 9. Refs (决策 #9 + 主人 8/4 23:33 + 用户记忆 #3-#5 + #8)

### 9.1 用户记忆 (5 条 cross-project stable)

| # | 规则 | Evidence | Apply when |
|---|---|---|---|
| **#3** | 用户看结果不看哲学 (核心 UI 原则) | 2026-08-04 R19 8 个认知纠正 | 任何 UI/前端设计 |
| **#4** | AI 不会衰老病死 (跟传统生命周期模型不同) | 2026-08-04 R19 | 任何 AGI / 长程 AI / 自主 agent 设计 |
| **#5** | 信息密度"高"= 拟人化 + 拟物化 | 2026-08-04 R19 9 organ 抽象 | AI 状态可视化 / 仪表盘设计 |
| **#8** | 前端终极 = Tauri, TUI 是过渡 | 2026-08-04 23:33 | 任何前端/桌面 app 路线决策 |
| **#9** | TUI 升级节奏: 改瘦后暂告段落, 优先后端 | 2026-08-04 23:55 | 主人做完阶段性大改动后, 安排下一步节奏 |

### 9.2 主人拍板 (8/4 23:33 + 23:55)

- **8/4 23:33**: "我们最后要做的前端应该是Tauri, 但由于现在手头的ai团队没有适合干尤其是审美设计的, 所以web和桌面都搁置, 先做好tui来为桌面做准备."
- **8/4 23:55**: "测一下先, 后续的tui升级计划沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞tui"

### 9.3 决策链 (R125 era → R129 era)

- **#9** (用户记忆): TUI 升级节奏 (本路线图直接源头)
- **#11** (R11 阶段 4): frontend-proposal: Tauri 2.0 + 4 接入
- **#22** (8/10 17:22): workspace.version 1.2.0 严守 + 24 LOCKED 自主确认
- **#33** (8/10 17:22): master-reupgrade, 8 硬墙
- **#43** (8/10 18:58): Apeireth-tui 不合并 + 主仓挪出已完成 (R25 改瘦 + R26-3 + R125-12 累积)
- **#48** (8/10 19:41): 整合 #4 commit abf12243 done
- **#53** (8/10 21:42): tech-locked-unlock
- **#57** (8/10 22:00): R128 era 6 派活 (P11-1 Tauri prototype)
- **#58** (8/10 22:30): R128-2 era 3 派活 (P11-2 Tauri scaffold 深化)
- **#61** (8/11 00:03): 新会话接手 + R129 era 派活规划 (R129-15 = TUI 升级路线图沉淀)
- **#62** (8/11 00:08): 整合 #5 commit 拆 3 commit 拍板 (5.3 包含 R129-15 路线图)
- **#63** (8/11 00:10): R129 第 1 批 8 sub-agent 派活
- **#64** (8/11 00:25): 5 min tick cron 自动监督 + 16 上限补派 (R129-15 0:30 自动派)
- **#65** (8/11 00:30): R129 第 2 批 8 sub-agent 派活 (含 R129-15 bg_60d31ca1)

### 9.4 报告 (R125 era → R129 era)

- **`reports/apeireth-r26-tui-upgrade-2026-08-07.md`**: R26 4 阶段工程用语 (Init/Bootstrap/Serving/Saturated), 反思环真接 SqliteMemoryStore, 5 nav + 9 organ 打磨
- **`reports/agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md`**: R128 P11-1 Tauri 2.0 prototype (5 nav + 9 organ 1:1 镜像 TUI)
- **`reports/agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md`**: R128 P11-2 Tauri scaffold 深化
- **`reports/agent-r129-8-1.0-release-process-2026-08-11.md`**: §6.1 TUI 升级简版 (per 决策 #9 改瘦后暂告段落)
- **`reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md`**: 本报告 (完整版路线图, per 决策 #9 模板)

### 9.5 ADR (Architecture Decision Records)

- **`docs/adr/0011-tui-as-thin-client.md`**: TUI 瘦客户端 (R25 阶段改瘦 2026-08-04 拍板, 1.0 release 阶段 4 续)
  - 改瘦前后对比: TUI 直调 core (旧) → TUI HTTP client → API (新)
  - 集成测试床: TUI 测的就是 API, Tauri 来了 0 改 backend
  - 5 nav + 9 organ 设计契约: TUI/Tauri 共享

### 9.6 TUI 源代码 (主仓)

- **`crates/apeireth-tui/`**: TUI 9 organ + 5 nav + 5 pages + 7 command + theme + observability + persistence + onboarding + cognition_live + config_watcher + error + R125-12 stub (per 决策 #43 §1.1)
- **`crates/apeireth-tui/Cargo.toml`**: NOT LOCKED (R25 改瘦 step 1), license.workspace = true 严守
- **`crates/apeireth-http-client/`**: R25 改瘦, TUI/Tauri 共享 backend API surface
- **`crates/apeireth-tui-e2e/`**: 25+ 测试 (ratatui TestBackend, per ADR 0011 §8.3)
- **`crates/apeireth-cli/`**: R25 改瘦, 跟 TUI 共享 HTTP client

### 9.7 R129 era 报告索引 (per 决策 #61 §3.1 + #64 §3)

- **R129-1** 整合 #5.1 commit 准备 (src/) → `agent-r129-1-integration-5-commit-src-prep-2026-08-11.md`
- **R129-2** 整合 #5.2 commit 准备 (docs/) → `agent-r129-2-integration-5-commit-docs-prep-2026-08-11.md`
- **R129-3** 8 步 verify → `agent-r129-3-*.log`
- **R129-4** ASI Stage 4 → `agent-r129-4-asi-stage-4-autonomy-2026-08-11.md`
- **R129-5** ASI Stage 5 → `agent-r129-5-asi-stage-5-governance-2026-08-11.md`
- **R129-6** ASI Stage 6 → `agent-r129-6-asi-stage-6-guardianship-2026-08-11.md`
- **R129-7** 借鉴 11/11 升级 verify → `agent-r129-7-borrow-11-11-upgrade-verify-2026-08-11.md`
- **R129-8** 1.0 release 流程 → `agent-r129-8-1.0-release-process-2026-08-11.md`
- **R129-15** (本报告) TUI 升级路线图沉淀 → `agent-r129-15-tui-upgrade-roadmap-2026-08-11.md`

---

## 10. 一句话 (再次强调)

**TUI 改瘦 (R25 8/4) → R26 4 阶段工程用语 (8/7) → R125-12 PHL-07 stub (8/10) 累积后, 路线图按"改瘦后暂告段落, 优先后端"节奏 (per 决策 #9 + 主人 8/4 23:55 拍板) 沉淀: 当前 TUI = 瘦客户端 (HTTP to apeireth-api), 跑通, 集成测试床角色, 不新加功能; 下一步 (Step 2/3/4) 跟后端 / Tauri 路线 (per 决策 #11 阶段 4 + 主人 8/4 23:33 + 用户记忆 #8): Step 2 = 后端有变化时 TUI 跟改 (触发条件式) / Step 3 = Tauri 终极前端 (等设计团队到位) / Step 4 = TUI 9 organ 拟人化深化 (跟 Step 3 共享, Tauri 优先); 维护清单 6 项不退化检查 (M1 TUI binary 仍可跑 / M2 HTTP client 适配 / M3 0 越界 8 硬墙 / M4 Cargo.toml 1.2.0 / M5 0 装 PASS / M6 0 主动 commit+push). 砍掉 UI 哲学 (per 用户记忆 #3-#5) → 保留状态 + 主对话结果 + 历史 + 设置 + 工具结果 / AI 不会衰老病死 → 9 organ 用"成长/活跃度"非"健康度" / 1 屏多卡片 = 主页 = 状态. 8 硬墙 0 越界 100% (B1 24 LOCKED 入口签名 0 改 / B2 1.2.0 0 改 / A1 3 值 0 改 / B3 30 维 / B4 6 重 v7 / B5 8 锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / C3 升 v7 / 0 push). 整合 #5.3 commit (per 决策 #62 §4, reports/ 备查用, 0 影响 build). 0 主动 IM 主人, 0 主动 push, 0 主动 commit, 0 借具体源码, 0 装 PASS. 主人起床后按 8 步 verify → 整合 #5 commit → 1.0 release → 触发 Step 2/3/4.**
