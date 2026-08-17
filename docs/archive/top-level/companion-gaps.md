# Companion 机制差距清单与模块设计（2026-08-16 系统体检）

> 主人指示: 「还有什么差距, 你仔细调研。不要到处打补丁, 而是设计完善的机制模块。」
> 本文件 = 体检报告 + 机制模块设计。**原则: 每个差距归入一个模块, 模块独立可验证, 不散打补丁。**

## 一、体检结果（机制 vs 接线）

| 机制（已存在） | serve 接线 | 差距 |
|---|---|---|
| 节律直方图/驱动/门禁（emergence） | 🟡 daemon 常驻但**只打 ConsoleSink 日志** | **主动性没有触达主人的通道** |
| 当前时刻注入 | ✅ 已接（2026-08-16） | — |
| 记忆注入（EMI/NEC）+ 推理召回 | ✅ 已接 | 注入取**最近 30 条平铺**，无重要性分层 |
| 偏好库 + 提炼器（memory_extractor） | ✅ 已接 | commitments 已提炼但**无结构化日程/提醒** |
| 做梦/反思（dream/reflection） | 🟡 做梦 LLM 摘要 ✅；**反思无 LLM 深度**（状态机写状态文本，0 假装标注） | 反思不产生洞察 |
| 经验库/原则晋级（自成长） | ✅ 已接 | — |
| 授权请求（approval_requests） | ✅ 已接 | — |
| Goal 目标系统 | ❌ **serve 完全未接** | AI 不知道当前目标 |
| lightmemo L1-L4 分级 + decay 遗忘 | ❌ **未接** | 记忆无分层/无遗忘 |
| 情绪（mood）/ 提炼的 emotional | ❌ **未接** | AI 不知道主人情绪状态，情绪不回灌节律 |
| 节律活跃概率 | ❌ 未注入 | AI 不知道"现在是主人活跃时段吗" |
| Lark 送达 / ProactiveDriver | ❌ serve 用 ConsoleSink | 离线主动送达未接（需凭据） |
| 上下文管理 | 🟡 粗暴裁剪 30 条 | 长对话丢上下文，无滚动摘要（VCP 有 ContextFoldingV2） |

**结构结论**: 机制层是齐的（emergence/dream/reflection/goal/lightmemo/lark 都是真实现），
**缺的是"组装层"**——把机制接进对话主链路 + 让机制之间互相喂数据（提炼→偏好→注入→应用；
情绪→节律；承诺→日程→提醒；目标→注入→推进）。这不是补丁，是**缺失的装配架构**。

## 二、机制模块设计（6 模块，每个独立可验证）

### 模块 1: 状态感知（State Awareness）— ✅ 已完成 (2026-08-16)
统一「状态块」注入（替代散装的时刻/今日注入）:
```
【当前状态】2026-08-16 周日 21:26 · 此刻主人活跃概率约 72% (节律观察 14 天, 置信 0.8)
· 当前目标: 辅助主人学习高数/线代 (阶段 2/4) · 近期约定: 周五高数期中 (还有 5 天)
· 主人最近情绪信号: 平静 (最近提炼)
```
- 输入: 时刻 + RhythmEstimate (daemon_loop 每 tick 共享) + GoalService::current + 【约定】/【情绪信号】(提炼器产物)
- 节律坐标: UTC (与观察自洽); 只给概率不给时段 (诚实)
- 实测: 时刻/活跃判断/三个约定全命中
- 待办: Goal 写入侧 (模块 6); 节律时区统一 (观察改 Local)

### 模块 2: 记忆分层（Memory Hierarchy）— ✅ 已完成 (2026-08-16)
- rank_memory_entries: 做梦摘要/偏好优先 → 提炼事实 → 其余按最近; 预算 12 条注入
  (替代"最近 30 条平铺"; lightmemo 完整分级/遗忘后续)
- 推理召回候选用全量 40 条 (deep_recall 重排)

### 模块 3: 上下文管理（Context Manager）— ✅ 已完成 (2026-08-16)
- 滚动摘要: 长对话裁剪时被裁旧段 → LLM 摘要 (5 分钟节流) → 【早期对话摘要】system 消息
- 摘要失败 → 诚实提示「已由记忆系统提炼」, 不硬造; 注入预算 = 各块 take 上限

### 模块 4: 主动送达（Delivery Channels）— ✅ 已完成 (2026-08-16)
- BroadcastSink (Sink trait): 涌现/事件 → broadcast → SSE (GET /v1/apeireth/events)
- 前端 EventSource 实时显示「📣 他说」气泡; 开发端点 /v1/apeireth/test-event
- 实测: 事件 → SSE 流 → 客户端收到 ✅ (生产事件=涌现触发, 待真实使用积累)
- Lark 通道 (离线) 已有 LarkDelivery, 需凭据后接

### 模块 5: 深度反思（Deep Reflection）— ✅ 已完成 (2026-08-16)
- ReflectionScheduler 加 ReflectionReflector trait 口子 (tick 改 async)
- MiniMaxReflector: 周期记忆 → 模式/洞察/建议 (markdown) → 写回【深度反思】
- 失败 → 诚实降级状态文本 (实测: MiniMax 空响应时降级生效)

### 模块 6: 目标驱动（Goal Integration）— ✅ 已完成 (2026-08-16)
- 写入侧: goal_create / goal_status / goal_complete / goal_pause / goal_block 工具
  (ToolBridge.with_goals 注入, 与 serve 注入侧共享同一 GoalService 实例)
- 状态块含当前目标 (模块 1 已接读取); 工具走 GoalService 严格状态机 (revision+1, 非法迁移拒绝)
- 实测: AI 建目标 (25 轮) + 主动查状态 + 融合记忆/时间 (倒计时 5 天); 完成→建新 受 MiniMax 限流暂缓, 单测覆盖
- 待办: 目标进度 → 每日摘要; 做梦/反思产物关联目标

## 三、实施顺序建议

模块 1 → 6 → 2 → 4 → 3 → 5（每个独立验证、独立 commit；全部是"接现有机制"，不发明新机制）
