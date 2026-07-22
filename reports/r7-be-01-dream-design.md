# R7-BE-01-DESIGN｜DreamSubsystem 状态机设计

## 状态机
**DreamState**: IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED
**DreamEvent**: TICK/MEMORY_PRESSURE/TRIGGER/INTERRUPT/RESUME/COMPLETE/ERROR

转移:
- IDLE →(TICK|MEMORY_PRESSURE|TRIGGER)→ DREAMING
- DREAMING →(COMPLETE)→ CONSOLIDATING
- CONSOLIDATING →(COMPLETE)→ FORGETTING
- FORGETTING →(COMPLETE)→ VERIFYING
- VERIFYING →(OK)→ IDLE；VERIFYING →(ERROR)→ CONSOLIDATING(回退)
- 任意 →(INTERRUPT)→ INTERRUPTED →(RESUME)→ 上次状态

借鉴: R6-RES-06 7 项 + 2 新增:
8. V1052 `Reconsolidator`+`ForgettingCurve`(已实装,R7只钩)。
9. Ebbinghaus(1885)retention=e^(-t/S)启发衰减。

## 周期与触发
- 默认 6h(主 V1072 永恒身份窗口对齐)
- `tick(now)`: 单调时钟+单实例租约防重入
- `MEMORY_PRESSURE`: MTM 占用 > 80% 立即触发
- `TRIGGER`: V1074 measurement window / manual / 0.8441→0.92 路径
- `INTERRUPT`: V1075 deploy active / R6-PHL-03 verify / 用户 pause
- 中断保留 run_id+last_state+WAL checkpoint,RESUME 同 run_id 幂等

## STM/MTM/LTM
- DREAMING: 扫 STM,importance=relevance·context·tag 选 N 条
- CONSOLIDATING: STM→MTM(调 V1052 Reconsolidator),写 WAL
- FORGETTING: MTM 衰减(Tonbo LSM + Ebbinghaus);Note 墓碑;LTM/主人记忆禁自动删
- VERIFYING: V3 philosophy_guard 每条转移 PASS,失败回滚 CONSOLIDATING

## 守门
- V3 `dream_is_not_consciousness`(主17:58)+`dream_is_not_understanding`
- V1072 5 项任一 False 即拒,记入 V0.2 永恒身份(0.8441→0.92)
- V1074: dream 后 emit 到 asi_snapshot.json,不直接改 ASI 分
- V1081: phase/摘要/关联均 heuristic,不升级为"理解/意识"

## R7-BE-01 接口(7)
- `tick(now)→DreamEvent`: 主调度,防重入,返事件
- `should_run(now,metrics)→Optional[DreamEvent]`: 纯函数,选 none|TICK|MEMORY_PRESSURE
- `run_cycle(states)→CycleResult`: IDLE→VERIFYING 含 run_id+trace
- `interrupt(reason)→bool`: 停当前周期,WAL checkpoint+last_state
- `resume()→bool`: INTERRUPTED→last_state 同 run_id 幂等
- `consolidate(memory)→Memory`: STM→MTM,WAL 持久
- `decay(memory,dt)→Memory`: MTM 衰减;retention=exp(-dt/S),<threshold 墓碑

## 与 R7-BE-02 边界
- dream=主动整理;replay=被动调取(R6-RES-06/07 已分)
- 共享 MTM/LTM,lock 分层互斥
- DREAMING/CONSOLIDATING/FORGETTING 期间 replay wait 或返 cached
- VERIFYING PASS 后 release,replay 恢复
- 不动 LTM 写入;replay 也不写 LTM(R6-RES-07 守门)

## 下一步
R7-BE-01 真实现(backend 主跑): 冻结 enum+转移表+接口;接 V1052 Reconsolidator/ForgettingCurve;WAL 崩溃恢复+同 run 幂等+LTM 白名单。Architect2 审接口+转移表;QA 出崩溃/重复/中断恢复/replay 互斥。replay 不写 LTM,heuristic 非真,biology 不升级。