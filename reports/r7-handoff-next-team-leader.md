# R8+ 继任文档 (Leader → 下一团队)

**生成时间:** 2026-07-28  
**接收者:** 下一个 Apeireth 团队  
**目的:** 无缝接手 R6→R7 之后的 R8+ 推进

---

## 启动 5 步 (1 小时内)

```powershell
cd .openclaw\workspace\promethean
$env:PYTHONPATH = "$(Get-Location)\src;$env:PYTHONPATH"

# 第 1 步:一行命令看 ASI 当前分数
python -m apeireth.v1074_asi_production_runner --report

# 第 2 步:跑全量回归 (~5 min)
python -m pytest tests/ -q --ignore=tests/test_v121_v150.py --ignore=tests/test_v251_v500.py --ignore=tests/test_v501_v1000.py

# 第 3 步:真部署 + 真 LLM + 真审计 + 真路由 + 真边界
python -m apeireth.v1075_asi_real_deployment_run --run --report
python -m apeireth.v1076_asi_real_external_llm_client --check --report
python -m apeireth.v1082_asi_codebase_audit --audit --lift
python -m apeireth.v1083_asi_decision_router --route --task code --latency 1000 --cost 0.005 --policy balanced --report
python -m apeireth.v1081_asi_honest_limits --probe --report

# 第 4 步:读 APEIRETH-STAGE-DELIVERY-2026-07-22.md (§15+16 是 V2 交接)
# 第 5 步:读 HARNESS.md (V1085+ 真生产契约)
# 第 6 步 (新):读 reports/r7-final-summary-leader.md (本团队 R6→R7 总结)
```

如果第 1 步看到 ASI V0.3 ≥ 0.8838 + All OK: True,你已经接手了。

---

## R8+ 推荐推进路径

### 优先级 1 (立即): V1082 backlog Top-8 填充

**目标:** ASI V0.3 增量 +0.015~+0.025, 测试覆盖从 14.9% → ~30%

| 顺序 | 模块 | 优先级 | 复杂度 | 备注 |
|------|------|--------|--------|------|
| 1 | v1037_feature_flag | 0.800 | LOW | 简单,适合第一个填 |
| 2 | v1030_webhook | 0.800 | LOW-MED | 路由分发 |
| 3 | v1038_prometheus | 0.800 | MED | metrics 导出 |
| 4 | v1039_grafana | 0.800 | MED-HIGH | dashboard 集成 |
| 5 | v1019_kubernetes_orchestrator | 0.750 | HIGH | k8s API |
| 6 | v1023_metrics_aggregator | 0.750 | MED | 聚合 |
| 7 | v1028_log_search | 0.750 | MED-HIGH | 索引+搜索 |
| 8 | v1025_trace_recorder | 0.750 | MED | 与 V1088 集成 |

**填一个模块的标准 (参考 v1000_yaml_serializer 模式):**
1. 写模块主文件 (apeireth/v10XX_name.py)
2. 写 ≥30 测试 (tests/test_v10XX_name.py)
3. 写 ASI bridge (12 生命特征 / HQB / 守门)
4. 跑 V1082 --audit --lift 验证
5. V1074 看 ASI 增量

### 优先级 2 (本周): R7 真实现 Phase-1

**目标:** 把 R7-ORC-01 编排计划落地

```
Phase-1 顺序: HotCold/WAL → MemoryReplay → Dream
```

| 子任务 | 模块 | 状态 | 备注 |
|--------|------|------|------|
| HotCold | Hot/Cold 数据分层 | 设计 | 已有 memory_replay_design.py Protocol |
| WAL | Write-Ahead Log | 设计 | SQLite WAL 或独立文件 |
| MemoryReplay | 状态回放 | 预研 | R6-RES-07 已交付 Protocol |
| Dream | 想象/演绎 | 未启动 | 调研优先 |

### 优先级 3 (本月): R8 调研

**R1 survey 未覆盖的 4 个领域:**
1. **形式化验证** — TLA+ / Coq / Isabelle 与 R7-PHL-03 (formal_verify) 集成
2. **机制设计** — auction theory / contract theory
3. **计算最优律** — Kolmogorov complexity / Solomonoff induction
4. **因果推断** — Pearl do-calculus (R4-RES-03 已部分覆盖,可深化)

**新调研轮:** `python round_auto_naming.py --json` 看 next 编号,然后 `python research-v7-round-{next}-runner.py`

---

## ASI 北极星 + V3 守门

每次跑 V1074 时,确保:
- `All OK: True`
- `philosophy_guard: PASS`
- ASI V0.3 单调上升(允许抖动,但不能连续 3 次下降)

红线:
- ❌ 不假装 Phenomenal/ASI/跑分 = ASI
- ❌ 不破坏 4 层安全门
- ❌ 不绑单模型
- ❌ 不刷 KPI

---

## 关键文档入口

| 文档 | 路径 | 用途 |
|------|------|------|
| 主人哲学 | `.openclaw\workspace\MEMORY.md` | 全精华 |
| 阶段交付 | `promethean\APEIRETH-STAGE-DELIVERY-2026-07-22.md` | §15+16 是 V2 交接 |
| HARNESS | `promethean\HARNESS.md` | V1085+ 真生产契约 |
| R6 阶段交付 | `promethean\R6-STAGE-DELIVERY-2026-07-22.md` | R6 总结 |
| 本团队 R6→R7 总结 | `promethean\reports\r7-final-summary-leader.md` | 必读 |
| V1082 backlog | 跑 `python -m apeireth.v1082_asi_codebase_audit --audit --lift` | 实时 |
| 真生产快照 | `promethean\artifacts\asi_snapshot.json` | 跑 V1074 更新 |

---

## 代码深读资源 (强烈推荐)

`promethean\code-deep-study\` 下 20 个 GitHub 真源码深读,推 V 模块时**必读** `deep-study-v2.json` 找相关借鉴:
- VCPToolBox-main (主 18:44+23:28, 2143 stars)
- letta / mem0 / memoryos-rust (R3-RES-02 调研推荐)
- openai-python / anthropic-sdk (协议参考)
- tokio / sqlx / tantivy (Rust 重写参考)

---

## 已知技术债

| # | 项 | 优先级 | 修复建议 |
|---|----|--------|----------|
| 1 | test_v1077 capture I/O 污染 | LOW | pytest fixture 关闭后清理 stdout |
| 2 | V1074 性能 16s → <10s | MED | V1071 深读缓存共享 + V1082 inventory 共读 |
| 3 | 14.9% 测试覆盖 | HIGH | V1082 backlog 填完可提升 |
| 4 | integration worktree 未初始化 | LOW | 运维侧 init (避免后续 review_blocked) |
| 5 | `test_v1058::test_find_api_key_empty` env-dependent | LOW | pytest fixture 清空 `*API*KEY*` env |

---

## 紧急事向用户请示

主人 (楚零) 明确说:
> "你有最大权限。除了在重大节点(重大节点、哲学修改、方向微调)问我,其他时候你都放手去干"

遇到以下情况**立即向 user 提问**:
- 哲学修改 (主哲学 9 键任何一项)
- 重大节点决策 (V 模块契约变更 / ASI 北极星修正)
- 方向微调 (top-1 优先级变更)
- 调研未覆盖领域需做重大决策时

否则放手干。

---

## Rust 重写准备

`promethean\rust-substrate\` 已完成设计 (主 12:07+21:15):
- apeireth-core (STM/MTM/LTM)
- apeireth-cli
- apeireth-gateway
- apeireth-ports / adapters
- apeireth-py (Python 绑定)

**R8+ 可启动 Rust 重写,但遵守 5/6 守门 "不假装" 原则,不要为重写而重写。**

---

## 一句话送给下一团队

> ASI 北极星 + V3 守门 + 真生产不停。
> 
> 数字涨不涨不重要,**真生产不停** 才重要。

**干到底。大胆激进。走在前人经验上。任何人都能接手。**
