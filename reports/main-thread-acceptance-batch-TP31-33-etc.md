# 主线程验收报告 — 团队批次（TP31/32/33/22/27/29 + W4/W6 等，integration 树实测）

> **验收人**: 主线程（主人指示: "他们干完你验收，等他们干完你再看缺什么"）
> **验收方式**: 以 integration 树（HEAD 3d893e4a 及后续）实测为准，不依赖团队自报
> **状态**: 初稿（团队可能仍在推进，最终树确认后更新）

---

## 一、批次核对（声称 vs 树实现）

| 批次 | 团队声称 | 树实测 | 结论 |
|---|---|---|---|
| TP31 W1 文本模拟器 | 完成 | world_model.rs（CounterfactualChain/TimelineLlm/Brier 拒绝阈值/CalibratedResolver）| ✅ 11 测试绿 |
| TP32 W2+W3 因果推演 | 完成 | causal_world_model.rs 1020 行（CausalNode/因果图 MCTS/记忆时间线挖边）| ✅ 11 测试绿 |
| TP33 发布门槛 | 完成（docker 待实测）| compose 已外部化（3 处"明文"均为注释说明 + .env.example 在）；Dockerfile 补 2 成员 COPY；无 docker 诚实标"待实测" | ✅ 符合纪律 |
| TP22 Observer/W5 | 完成 | observer_capture.rs（ExperienceCandidate/Outcome + PostExecuteHook 挂 tool_bridge）| ✅ |
| TP27 stock | 完成 | apeireth-stock（refresh 模块 + V6 migration，61 测试）| ✅ |
| TP29 YAML 声明 | 完成 | yaml_spec.rs 1039 行 + tool_bridge +159 | ✅ |
| W4 主动推销 | 完成 | proactive_memory.rs 883 行 | ✅ |
| W6 Brier 自我诊断 | 完成（返工 2 轮）| 实现位置待最终树核对 | 🟡 待核对 |

## 二、预验收发现并修复（已落团队分支 386d174，收工后同步 integration）

| # | 问题 | 根因 | 修法 | 验证 |
|---|---|---|---|---|
| 1 | tool-shell persist 2 测试挂 | open_in_memory 建表缺 status 列（b77c0791 N17 Mutex 改造引入）| 补列 | 39/39 ✓ |
| 2 | arbitration t08 偶发失败 | "跨日志模式一致"在时间戳序语义下不可满足 | 改验同日志重复查询确定性 | 13/13 ✓ |
| 3 | wiki doc-test 编译挂 | frontmatter 示例代码块未标语言 | 标 text | 24/24 + doc ✓ |

## 三、全量验证基线

- 团队分支全量: 366 组 ok + 上述 3 处修复后全绿（待收工后 integration 最终树重跑确认）
- 编译: integration 树 --all-targets 干净（22:31 树）

## 四、剩余缺口（integration 树实测，收工后最终确认）

| 项 | 实测 | 状态 |
|---|---|---|
| E4 好奇驱动引擎 | 全树零匹配 | ⬜ 真缺（五原型唯一未做） |
| F4 假设检验闭环 | 全树零匹配 | ⬜ 真缺（主人钦定"闭环想法进步"） |
| F1 情感记忆（记忆维度）| mood 仅门控输入（emergence/proactive_memory），memory 无 mood 字段 | ⬜ 部分 |
| F6 价值内化 | 全树零匹配 | ⬜ 真缺 |
| S4 出站网络策略 | gateway 无出站白名单/拒绝 | ⬜ 真缺 |
| A4 事件流统一打通 | 零件齐（bus event_log/agent AgentEvent/workflow EventHistory），缺统一 + PerceptionGate | ⬜ 缺打通 |

## 五、验收结论（初稿）

- **本批交付质量**: 核心实现真实（树实测），0 装 PASS 纪律保持（TP33 docker 诚实标注）
- **需修复后合入**: 3 处（见 §二，已修待同步）
- **发布前置**: 世界模型前两层（TP31/32）已合入且测试绿 → 发布前置最大项完成
- **剩余缺口**: 见 §四，建议下批排 E4 → F4 → F6（她本身），A4/S4 并行，F1 挂记忆 v2
