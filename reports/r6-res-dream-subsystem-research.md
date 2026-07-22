# R6-RES-06｜DreamSubsystem 预研

## 定义/边界
主23:28命名“梦”；主12:14永恒身份/Phase46三层记忆；主23:44干到底。DreamSubsystem是静默期主动选择、压缩、巩固、遗忘、回放的后台编排，不是睡眠（资源暂停），更不是意识。

- `dream`周期/压力触发整理；`memory_replay`由事件触发按ID/窗口取回，Dream只调用端口。
- forgetting是可选动作，策略/墓碑归记忆层；`entropy_gate`只给阈值，不能删LTM。
- 新关联是可审计候选，非Phenomenal或“理解”。
- V1052已有Episode/Note、三层Store、TierPolicy、WAL/replay、Reconsolidator、ForgettingCurve；R7只做phase/事务/信号，不复制。

## AgentMemory现状与证据诚实性
交接称：`DreamScheduler.tick()`调用Engine；`DreamPhaseSelector`按force/first/rem/high_tension/high_density/high_memory/healthy_skip/rem_priority/no_store选择light/REM。**本地AgentMemory全文无这些符号**；且称“10 phase”却列9标签。仅作上游草案；R7须先取得commit/path并冻结枚举，否则做最小selector。

## 真借鉴
1. Apeireth V1052：三层迁移、WAL、衰减、再固化。
2. MemoryOS `manager.rs:781-899`：容量触发STM→摘要/embedding→MTM，保留5条。
3. Letta `letta_agent.py:95-153`：buffer阈值、partial-evict、EphemeralSummaryAgent。
4. Mem0 `prompts.py:176-185`：ADD/UPDATE/DELETE/NONE动作日志。
5. claude-mem `index.ts:747-861`：`after_compaction`去重、`agent_end`摘要。
6. Tonbo `common.rs:49-64`：periodic tick、quiescence轮询、并发指标。
7. R37 q5 hippocampal replay/sharp-wave ripples；仅启发优先级。密度：dream R27/34/37；consolidation R9/37；REM 0轮。

## R7-BE-01 状态机/方法
`IDLE→SELECT→LIGHT|REM→CONSOLIDATE→FORGET?→REPLAY?→EMIT→IDLE`；任一步失败→`ROLLBACK`（WAL）→`EMIT_FAILED`，单实例租约防重入。

|方法|契约草案|
|---|---|
|`dream(ctx)`|建run_id/快照，编排一次事务，返回可审计报告|
|`should_dream(metrics,force)`|selector纯函数；返回phase+reason或skip|
|`consolidate(ids)`|委托V1052完成STM→MTM→LTM，幂等|
|`forget(candidates)`|仅Note；墓碑+撤销窗；LTM/主人记忆禁止自动删|
|`replay(refs)`|调用MemoryReplay端口，去重、限额，不自行查询|
|`emit_signal(event)`|发phase/result/证据给哲学守门与观测层|

## 守门/验收/下一步
V3：`dream_is_not_consciousness`；V1074：不直接改ASI分，最多提供LTM质量证据；V1081：phase/摘要/关联均heuristic，不是真理解。还需：确定性selector表、同run幂等、WAL崩溃恢复、LTM保护、并发tick去重、信号含输入hash/动作/反例。

R7由backend主跑、architect2审接口、QA做崩溃/重复测试；先补AgentMemory来源证据，再实现selector→light，最后REM/replay/forget。不得把biology metaphor升级为事实。
