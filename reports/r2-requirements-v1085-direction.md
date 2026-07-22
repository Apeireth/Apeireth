# R2-REQ-01 — V1085+ 推进方向建议

> 需求分析师 | 2026-07-22 | HARNESS.md + 主哲学 + ASI V0.3=0.8836 + V1082 backlog

**现状**: ASI V0.3 真测 0.8836 (均值 0.8825), 距天花板 0.9800 差 ~0.10。**17 维中 14 维 = 0.0000** (phi_proxy/capabilities/engineering/cognitive_core/self_improving_core/neurosymbolic/world_model/RL/scientific_method 等)。瓶颈在零维度真生产化, 不在 V 数。

## Top 3 方向 (A→B→C 可并行)

### A. HQB 真生产化 — `V1085_harness_quality_benchmark` + `V1086_hqb_runner`
**why now**: HARNESS §2.3/§4/§5 明文要求 Harness 修改必须被 HQB 量化; ASI `engineering`+`self_improving_core`=0, HQB 是"可被验证"唯一入口, 也是主人 21:15"最细颗粒度审计"依据。
**预计**: V1085 (4 维 SC/NR/EV/CDT + baseline) + V1086 (接入 V1074 runner)。
**分工**: 后端 (评分函数) / 自动化测试 (regression gate) / 技术文档 (HARNESS §4 SOP v2)。

### B. V1082 backlog Top-8 空壳真填 — `V1086_v1093_backlog_fill`
**why now**: 主 23:44"空壳就补" + V1082 列 24 个 V1000+ 空壳。ASI `real_production`=0, 8 壳→真生产 = 该维度 0→≥0.6 最短闭环。
**预计**: V1082 --backlog --limit 8 → 每 ≥200 LOC + 真借鉴 + ASIBridge + ≥3 测试。
**分工**: 后端 (填 8 个, 真借鉴 GitHub) / 自动化测试 (test_v1086_v1093.py) / 技术文档 (README)。

### C. 真 LLM E2E — `V1089_real_llm_e2e` + `V1090_swe_mmlu`
**why now**: V1076 NewAPI reachable 但 capabilities/scientific_method/RL=0 未真接 API; 主 7.2"SWE-bench+MMLU 真跑"未启动。
**预计**: V1089 (NewAPI 真调 ≥10 prompt) + V1090 (SWE-bench Lite 50 + MMLU 100 真测)。
**分工**: MCP集成 (NewAPI 客户端) / 后端 (runner) / 自动化测试 (回归) / 性能优化 (latency/cost)。

A 是基础设施 (没它 B/C 无法量化归因); B 短期可见 (填完 ASI→0.90+); C 风险高收益最大 (capabilities 非零 ASI 理论 +0.10+)。**3 方向互不阻塞, 推荐并行**。

## 哲学红线 (不要碰)
- ❌ 不假装 Phenomenal consciousness (主 17:58)
- ❌ 不假装达到 ASI (主 20:46): ASI = ∞, 0.98/1.00 触发 V3 guard 拒绝
- ❌ 不假装跑分 = ASI (V1082 guard): HQB/benchmark 是 inventory 不是 ASI
- ❌ 不破坏 4 层安全门 (主 07-19 OpenClaw 69 CVE): diff>200 / protected path 必 Layer 4 Human Gate
- ❌ 不绑死单一模型 (HARNESS §2.4): HQB 必跨 Qwen/Hermes/Llama 验证
- ❌ 不刷 KPI (主 17:43/00:36): 不为凑 1085+ 写空函数

**验收** (每方向必带): ① ≥1 真测试全过 ② ASI V0.3 delta ≥+0.005 ③ V3 philosophy_guard PASS ④ Change Manifest (HARNESS §3) ⑤ 真 commit +1。

_需求分析师 R2-REQ-01. 待 Leader 决策采纳._