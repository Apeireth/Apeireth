# R5-AS-02 · 蓝图完整性（R8–R38）

## 1 总览/口径
实扫 `apeireth/*.py`：1192 模块、116365 LOC；31轮=368 query/1690 merged。取哲学、顶设、深读形成45项概念表；文件名按规范化别名匹配，**24已占位/21未占位，53.3%**。这是“蓝图节点率”，不等于空壳率。JSON仅有 query/bocha/anysearch/merged，**无** `gap_identified/borrow_from`，故 gap 由 query+merged 归纳。

层级规则：D=≥80有效LOC+≥3定义+测试引用；C=≥30LOC+≥2定义；B=≥8LOC+定义；A=纯名；E=无匹配模块。全库 D266/C49/B867/A10；概念命中项本轮均D。例 D:`v1042_causal_reasoning`,`memory_3tier`,`v1083_asi_decision_router`；C:`identity`,`phi_proxy`；B:`v101_ppo_clip`,`v109_pipeline`；A:`asi_fun_score`,`deep_research`。

## 2 调研覆盖矩阵
|簇|命中轮|
|---|---|
|Prigogine/熵|15,16,18,25,27,28,32 (7)|
|Friston|11,15,17,25,28,31 (6)|
|Schrödinger|16,32 (2)|
|Latour/ANT|21,24,27,35 (4)|
|Rosen|19,28,36,37 (4)|
|Kauffman|14,15,25,26,36 (5)|
|AlphaEvolve|11,13–17,19–37除18 (24)|
|1M-context|9,10,12,13,15,37 (6)|

## 3 主人哲学概念表（45项）
仓内交付/HARNESS/16 daily/cron中“主 X:XX”508行（487 unique）。`D`：ASI、身份、中心AI、Harness、VCP/MCP、世界模型、因果、RL、自演化、自创生、自催化、三层记忆、涌现、意识、主动推理、Prigogine、Kauffman、审计、决策路由、部署、外部LLM、诚实边界、portable-seed、deliberation（24）。`E`：Phenomenal/同名哲学守门、自繁殖、梦子工程、形式验证、自改安全、Compiler-IR、机制设计、熵守门、时间现象学、自由意志、计算最优律、灵魂日志、死亡、生命意义、ANT、Rosen、Schrödinger、AlphaEvolve专项、1M-context专项、空间主权、agency（21）。

## 4 顶设/深读覆盖
Rust事实为6 crates：core/cli/gateway/ports/py/adapters，覆盖记忆、身份、WAL、LLM/检索端口；缺安全自改/形式法。Phase标记与V1071–V1083链覆盖VCP→身份→测量→部署/LLM→审计→诚实边界→路由。24个深读项目覆盖协议/UI、L1–L4记忆、SDK/Agent、Tokio/SQLx/Tantivy/Arrow、Candle/runtime；借鉴密集处仍未必有命名节点。

## 5 关键空白与R6+
主人时间戳直接关联且无模块名：**Phenomenal(17:58)、空间主权(22:33)、梦(23:28)、灵魂、时间现象学、自繁殖**。优先：P0 `self_reproduction`、`self_mod_safety`、`formal_verify`；P1 `dream_subsystem`、`memory_replay/hot_cold`、`compiler_ir`、`mechanism_design`；P2 `phenomenal_guard`、`entropy_gate`、`time_phenomenology`、`space_sovereignty`。先补契约壳+验收语义，禁止把 reproducibility 再误算 reproduction。

## 6 边界
仅盘点，未填壳/未跑V1074或V1082/未commit。沙箱外 `workspace/MEMORY.md` 与每日日志不可直接读取；其仓内镜像（阶段交付+project daily）已纳入，R6开工前须由Leader复核外层原文差量。
