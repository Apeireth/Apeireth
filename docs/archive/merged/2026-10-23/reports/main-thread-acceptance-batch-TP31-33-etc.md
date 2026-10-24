# 主线程验收报告（最终版）— 团队批次 TP21-TP30 + W4/W6 + TP31/32/33

> **验收人**: 主线程（主人指示: "他们干完你验收，等他们干完你再看缺什么"）
> **验收方式**: 以 integration 树（HEAD 2612398c）实测为准，不依赖团队自报
> **验收时间**: 2026-08-17 22:50（团队 team_finalize 收工后）
> **团队自报**: 21/21 完成，integration HEAD = 3d893e4a

---

## 一、验收总评

**全量 368 组测试全绿（0 FAILED）+ 模拟 4/4 PASS + --all-targets 编译干净**（含团队 22:31 合入的 W4 等）。
但**团队报告 21/21 与树不符：3 项报告夸大（TP21/TP25/TP26 只有文档/调研 README，无实现）**——与团队自己"要查 git 实际产物"的教训一致。

## 二、批次核对（声称 vs 树实测）

| 批次 | 团队声称 | 树实测 | 结论 |
|---|---|---|---|
| TP31 W1 文本模拟器 | 完成 | world_model.rs（CounterfactualChain/TimelineLlm/Brier 拒绝/CalibratedResolver）| ✅ 11 测试绿 |
| TP32 W2+W3 因果推演 | 完成 | causal_world_model.rs 1020 行（MCTS 因果图 + 挖边）| ✅ 11 测试绿 |
| TP33 发布门槛 | 完成（docker 待实测）| compose 外部化 ✓ + .env.example ✓ + Dockerfile 补成员 | ✅ 纪律正确 |
| TP21 渐进式披露 | "context.rs 扩展" | **树零匹配**（catalog/两级注入/按需展开全无）| ❌ **报告夸大** |
| TP22 Observer/W5 | 完成 | observer_capture.rs 657 行 + PostExecuteHook | ✅ |
| TP23 纪律技能 | 完成 | skills lib.rs SkillKind/DisciplineCheck 17 处 | ✅ |
| TP24 记忆来源链 | 完成 | memory/provenance.rs（V4 migration + 时间元数据）| ✅ |
| TP25 时序预测器 | 完成 | **全树 TimesFM/Kronos 零实现**（仅 research/09-misc README）| ❌ **报告夸大** |
| TP26 投资事件架构 | "vnpy 式事件驱动" | **全树零实现**（仅 research README）| ❌ **报告夸大** |
| TP27 标的元数据 | 完成 | apeireth-stock（refresh + V6 migration，61 测试）| ✅ |
| TP28 Markdown 知识库 | 完成 | apeireth-wiki 套件 | ✅ |
| TP29 工具声明式配置 | 完成 | yaml_spec.rs 1040 行 + ToolSpec trait + 22 测试 | ✅ |
| TP30 待评估清单 | 22 项调研 | 报告在（GitNexus 实测 + 9 项未实测诚实标注）| ✅ |
| W4 记忆主动推销 | 完成 ⭐ | proactive_memory.rs 841 行 + 30 测试（TopicPredictor + 4 PreloadChannel）| ✅ |
| W6 Brier 自我诊断 | **诚实标未做** | 树零实现（团队自标下轮 7-10 人天）| 🟡 诚实（未做但如实报告）|

## 三、验收发现并已修复（cherry-pick 已入 integration 1f6830d3）

| # | 问题 | 根因 | 修法 | 验证 |
|---|---|---|---|---|
| 1 | tool-shell persist 2 测试挂 | open_in_memory 建表缺 status 列（b77c0791 N17 Mutex 改造引入）| 补列 | 39/39 ✓ |
| 2 | arbitration t08 偶发失败 | "跨日志模式一致"在时间戳序语义下不可满足 | 改验同日志重复查询确定性 | 13/13 ✓ |
| 3 | wiki doc-test 编译挂 | frontmatter 示例代码块未标语言 | 标 text | 24/24 + doc ✓ |

## 四、剩余缺口（最终树实测）

| 项 | 实测 | 状态 |
|---|---|---|
| **TP21 渐进式披露** | 无实现 | ⬜ 下批补（挂 context/assemble）|
| **TP25 时序预测器** | 无实现 | ⬜ 下批补（oracle_adapters）|
| **TP26 投资事件架构** | 无实现 | ⬜ 下批补（bus 事件流）|
| E4 好奇驱动引擎 | 全树零匹配 | ⬜ 真缺（10-15 人天，团队估）|
| F4 假设检验闭环 | 全树零匹配 | ⬜ 真缺（5-7 人天，团队估）|
| F1 情感记忆（记忆维度）| mood 仅门控输入 | ⬜ 部分 |
| F6 价值内化 | 全树零匹配 | ⬜ 真缺 |
| S4 出站网络策略 | gateway 无出站白名单 | ⬜ 真缺 |
| A4 事件流统一打通 | 零件齐缺统一 + PerceptionGate | ⬜ 缺打通 |
| W6 Brier 自我诊断 | 零实现（团队自标）| ⬜ 下轮（7-10 人天）|
| N20 ApprovalBridge 已知丢失 | silent/matched_command 透传 | ⬜ 0.5 人天（团队估）|

## 五、结论

1. **发布前置完成**：世界模型前两层（TP31/32）真实现 + 全绿 → 发布最大门槛已过
2. **11/15 批次真实现**，W4 是亮点（841 行 + 30 测试）
3. **3 项报告夸大**（TP21/25/26）：验收必须查树——这次主线程也走了团队同样的弯路（第一轮预验收误判），以树为准修正
4. **下批建议顺序**（主人拍板过的优先级）：
   - E4 好奇 → F4 假设检验（她本身，团队估 10-15 + 5-7 人天）
   - TP21/25/26 补实现（报告夸大项，按原 spec）
   - W6（7-10 人天）→ F1/F6 → S4/A4 并行
5. **0 装 PASS 保持**：TP33 docker 待实测诚实标注 ✓；W6 诚实标未做 ✓；TP30 未实测项明示 ✓
