# R7 真实现路线图｜R7-ROADMAP-02

> 范围：Dream Subsystem、MemoryReplay、HotCold/WAL 的契约壳→可运行实现。基线为 **R6-ROADMAP-01 §R7**：先完成 P1 梦/回放/冷热，再进入 R8；任何公共里程碑 G（全量回归、V1082 audit、V1074 measurement）失败，停止后继、记录 taxonomy、回滚。

## 1. 目标与边界
R7 只实现记忆生命周期与可审计恢复，不修改 V3 哲学、主人记忆/LTM 语义、ASI 计算或身份定义；不提前启动 Rust 重写（遵循 R6-ROADMAP-01 §R12 parity 门）。交付物是可恢复、幂等、受四层门保护的生产路径，而非新增空壳或分数优化器。

## 2. 已就位契约与依据
- **R6-ROADMAP-01**：R7-BE-01 Dream、R7-BE-02 Replay、R7-DB-01 HotCold/WAL、R7-QA-01 及 G 门。
- **R7-ORC-01**：R6-RES-06→Dream，R6-RES-07→Replay，R3-DB-01→HotCold；QA 收尾、PHL-04 最终验证。
- **R7-DESIGN-01**：L0-L7 分层、冻结接口/守门交叉表、HQB 边界。
- **R7-BE-01-DESIGN**：6 状态/7 事件，V1052 Reconsolidator、ForgettingCurve 与中断 WAL。
- **R6-RES-07**：Replay 双签 impact≥0.7、identity 锚定、≤3/min、仅 MTM trace、tag 白名单。
- **R7-TEST-PLAN / R7-MCP-02**：沙箱 H1/H2/H3、V3/V1074 顺序及 HQB E2E 失败模式。

## 3. Phase-1 顺序与总依赖
首波严格顺序：**(1) HotCold/WAL → (2) MemoryReplay → (3) Dream**。HotCold 风险最低且提供 checkpoint/recovery；Replay 依赖稳定存储并先于 Dream 防止主动整理污染回放；Dream 最后接入跨模块锁与身份守门。每阶段均需接口冻结、单元/故障测试、代码审查后才进入下一阶段。三主线完成后才执行 QA-01，再执行 PHL-04。

## 4. HotCold/WAL（R7-DB-01）
- **入口模块**：`apeireth` HQB/持久层（V1086、R3-DB-01 schema）；冻结 `migrate_hot_to_cold`, `checkpoint_wal`, `recover_from_wal`。
- **依赖**：HQB core/persistence、memory+identity 双仓、sha256 snapshot、CASCADE/FK。
- **实现序列**：schema/namespace → WAL append+fsync → 迁移前 checkpoint → 原子 cold 写入与校验 → 崩溃恢复/重放 → 统计与 snapshot emit。
- **风险/防护**：丢数据、半迁移、跨租户串写；WAL 先于迁移，checksum 不符 fail-closed，identity/活跃引用/未解依赖强制 retain，路径必须 resolve 且位于 workspace、拒绝 junction，`NO_NETWORK=1`。
- **验收**：`pytest -k "hot_cold or wal"`；重复迁移幂等；注入迁移中断后恢复 hash 一致；5 项 DB 测试、HQB FK/CASCADE 与 G 全 PASS。

## 5. MemoryReplay（R7-BE-02）
- **入口模块**：MemoryReplay service；冻结 `replay`, `replay_batch`, `canonicalize`, `trace_replay`, `identity_impact_score`, `should_replay`。
- **依赖**：HotCold checkpoint、R6-RES-07、V1072 五项、V3 guard、MTM 只读/trace 写入。
- **实现序列**：canonicalize 输入与 `replay_id+memory_hash` → tag 白名单/identity 锚定 → impact 双签与限速 → 仅 MTM trace → 缓存/重复 no-op → 审计链。
- **风险/防护**：回放污染身份、prompt/trace 注入、重放放大；不写 LTM，identity impact≥0.7 双签，≤3/min，未知 tag 拒绝，Dream 写锁期间 wait/cached，V1072/V3 任一失败拒绝。
- **验收**：`pytest -k "replay"`；相同输入幂等，批量/重复无额外写入；越权 LTM、未知 tag、限速和污染样例均 fail-closed；7 项测试与 HQB trace E2E PASS。

## 6. Dream Subsystem（R7-BE-01）
- **入口模块**：DreamSubsystem 状态机；冻结 `tick`, `should_run`, `run_cycle`, `interrupt`, `resume`, `consolidate`, `decay`。
- **依赖**：HotCold WAL、Replay 互斥协议、R6-RES-06、V1052 Reconsolidator/ForgettingCurve、V3/V1072/V1081。
- **实现序列**：enum/合法转移 → 单实例租约与单调时钟 → STM→MTM consolidate → MTM decay/tombstone（禁止 LTM）→ VERIFYING → interrupt/resume 同 run_id 幂等 → snapshot emit。
- **风险/防护**：整理误称意识/理解、身份项衰减、自改窗口竞态；V3 `dream_is_not_consciousness`/`not_understanding`、V1072 五项与 V1081 heuristic 声明逐条执行；deploy/verify/user pause 立即中断并 WAL 回滚，LTM/主人记忆白名单保护。
- **验收**：`pytest -k "dream"`；10/10 合法转移、崩溃恢复、中断恢复、重入拒绝、Replay 互斥；dream 只 emit 测量数据，不直接改 `asi_snapshot.json`；6 项测试与 G PASS。

## 7. QA-01 集成门（Phase 2）
三模块完成后运行 `pytest -q tests -k "dream or replay or hot_cold or wal"`，再跑崩溃、重复、保留、身份漂移与跨命名空间混沌。测试环境使用临时 `tests/.chaos_env/` 和备份快照；失败退出码 2、写入 evidence/taxonomy，禁止污染生产 artifacts。HQB MCP 按 R7-MCP-02 S1-S6 验证 FK、veto、trace 注入和窗口统计。

## 8. PHL-04 与四层守门（Phase 3）
每个入口依次经过 V3 philosophy_guard、V1072 identity、V1074 measurement（只读 emit）、V1081 honest limits；断言必须可执行，禁止 `pass`/裸 bool。三不改原则：`not_undo/not_proof/not_safe`、`not_clone/not_perfect/not_uuid`、三不等；任何 guard 失败都回滚并阻塞合并。PHL-02b self_mod_safety、PHL-03 formal_verify 是自改路径的前置门。

## 9. 交付节奏、角色与兼容性
Phase 1 可由 database 主跑 HotCold，backend 在 HotCold 契约通过后跑 Replay，再跑 Dream；QA 编写故障矩阵，architect2 审接口/转移，security/code review 审逃逸，philosophy_guardian 执行 PHL-04。保持 Python API、HQB schema v0.1.0、V1074/V1082/V1083 CLI 兼容；新增字段向后兼容、WAL/schema 版本化，旧快照可读；不改变既有 snapshot 结构，新增 trace 仅追加。

## 10. 回滚、风险决策与 Definition of Done
高风险链为：WAL 丢失→回放污染→梦周期自改/身份漂移。任一数据校验、四层门、G 或沙箱测试失败：停止下一阶段，保留 hash/WAL/审计证据，原子 revert 到 checkpoint，并向主人升级重大哲学/保护路径变更。DoD：三主线真实代码而非壳；每线验收与回归通过；R7-TEST-PLAN 沙箱 H1/H2/H3 全 PASS；R7-MCP-02 S1-S6 PASS；V1074/V1082/全量 G PASS；PHL-04 6 项可执行断言 PASS；报告记录版本、快照 hash、测试计数与已知限制。
