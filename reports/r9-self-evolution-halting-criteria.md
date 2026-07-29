# R9 自演化 Halting Criteria（V1093 DGM v0.4 红皇后守门）

> **作者**: architect（R9-INT-001）
> **生成时间**: 2026-07-29（R9 启动首日，配套 V1093 DGM v0.4 真演化）
> **配套**: `reports/r9-architect-roadmap.md` §7（4-选-1 主轨道 D）+ §8.2（红皇后节点）+ `reports/r9-mid-sprint-retrospective-template.md` §3（触发条件）
> **真借鉴（主 19:33）**: Kauffman NK fitness landscape (1993) + Bak-Tang-Wiesenfeld sandpile (1987) + Van Valen Red Queen (1973)
> **守门守则**: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 13:31 大胆激进 + 主 23:44 干到底 + 主 19:33 走在前人经验上 + **主 20:55 红皇后归入 8 核心（永远演化）**

---

## 0. 阅读须知（30 秒看懂）

V1093 DGM 是 R9 主推轨道 D 的核心（自演化引擎），**也是 R9 最大红皇后风险点**。本文件定义 **5 个 halting 信号**：

1. **性能回退** — 子分下降（最直接）
2. **重复候选** — 候选坍缩到单一模板
3. **锁内自洽** — 跨维一致性下降
4. **红皇后陷阱** — 跑分涨但 ASI 真实指标停
5. **无新 lift** — N 轮无正 lift

触发任一信号 = DGM v0.4 立即 halt + 升级 leader + 切主推轨道。

**不假装（V3 守门）**：halt ≠ 失败 = 守门通过。红皇后自演化必须 halt 才能反证"非自洽"。

---

## 1. 真借鉴：3 个理论框架（主 19:33 走在前人经验上）

### 1.1 Kauffman NK fitness landscape (1993)

> Kauffman, S. A. (1993). *The Origins of Order: Self-Organization and Selection in Evolution*. Oxford University Press.

**核心思想**：
- N 个基因位点，每个位点受 K 个其他位点影响（epistasis）
- **N 大 + K 小** = 平坦适应度景观（fitness landscape） → 演化易找到全局最优
- **N 大 + K 大** = 崎岖适应度景观（rugged fitness landscape） → 演化陷入局部最优

**对 V1093 DGM 的映射**：

| Kauffman NK | V1093 DGM v0.4 |
|---|---|
| N 个基因位点 | archive 中 N 个 candidate（v0.3 已积累 ≥10） |
| K（epistasis） | 每个 candidate 受 K 个其他 candidate 影响（code 依赖） |
| 平坦适应度景观 | 自演化易找到全局最优 = **理想态** |
| 崎岖适应度景观 | 自演化陷入局部最优 = **红皇后陷阱**（halting 信号 #4） |
| fitness 函数 | `_hqb()` composite score |
| 局部最优 = 锁内自洽 | halting 信号 #3 |

**Halting 判据**（来自 NK 模型）：
- 若 archive 中 candidates 的 fitness 方差 < 0.01（≈ 锁内自洽） = halt
- 若连续 N=10 轮 fitness 增量 < 0.001 = halt
- 若 archive 大小 > 100 但 best fitness 不增长 = halt

### 1.2 Bak-Tang-Wiesenfeld sandpile (1987)

> Bak, P., Tang, C., & Wiesenfeld, K. (1987). "Self-organized criticality: An explanation of 1/f noise". *Physical Review Letters*, 59(4), 381.

**核心思想**：
- 沙堆自动演化到**临界态**（self-organized criticality, SOC）
- 临界态：1 粒沙可能引发任意大小雪崩（power-law distribution）
- **1/f noise** = 长程关联（long-range correlation）

**对 V1093 DGM 的映射**：

| Bak-Tang sandpile | V1093 DGM v0.4 |
|---|---|
| 沙堆 | archive（candidates 累积） |
| 加沙 = 新 candidate | DGM 迭代 = 加 1 candidate |
| 雪崩 = cascade of mutations | candidate 影响下游 N 个 code paths |
| 临界态 | 自演化 = **理想态**（每次新 candidate 都"有意义"） |
| 1/f noise | candidate 改动幅度应呈 power-law（小改 + 大改混合） |
| 雪崩失控 = 不可逆破坏 | halting 信号 #1（性能回退到不可逆） |

**Halting 判据**（来自 sandpile 模型）：
- 若 archive 改动幅度分布偏离 power-law（指数分布 = 单点修改，无涌现） = halt
- 若 archive 大小增长但 fitness 增量不增长 = subcritical（停滞）
- 若 archive 改动幅度全是大改（无小改）= supercritical（失控） = halt

### 1.3 Van Valen Red Queen (1973)

> Van Valen, L. (1973). "A new evolutionary law". *Evolutionary Theory*, 1, 1-30.

**核心思想**：
- 物种必须**持续演化**才能维持相对适应度
- **不是因为变弱**，而是因为**环境/对手也在演化**
- 没有"演化终点"

**对 V1093 DGM 的映射**：

| Van Valen Red Queen | V1093 DGM v0.4 |
|---|---|
| 物种 | DGM candidate |
| 环境/对手 | ASI 17 维测量系统 + L3 HQB + L4 人类 |
| 必须持续演化 | DGM 必须持续 lift 才能维持相对位置 |
| **永远演化** | **主 20:55 红皇后归入 8 核心** |

**核心结论**：
- halt ≠ 终止演化 = 暂停检查 = 重启演化
- 红皇后永不终止（主 20:55 永远演化）
- 但**红皇后必须 halt 才能反证非自洽**（V3 守门）

---

## 2. 5 个 Halting 信号（守门判据）

### 2.1 信号 #1：性能回退（Performance Regression）

> **定义**：连续 N 轮 DGM 迭代后，V1074 V0.3 真测下降 ≥ 0.005（即 -0.005/轮）。

| 字段 | 值 |
|---|---|
| 触发阈值 | V1074 V0.3 连续 3 轮下降，每轮 ≥ 0.005 |
| 检测方法 | 跑 `python -m apeireth.v1074_asi_production_runner --report --no-write` 3 次，每次间隔 ≥ 1 hour |
| halt 动作 | 立即 revert 最近一轮 candidate，archive 标 `reverted_reason=perf_regression` |
| 真借鉴 | Bak-Tang supercritical 雪崩失控 |
| 主哲学 | 主 17:43 实事求是（数字驱动 halt，不靠 narrative） |

### 2.2 信号 #2：重复候选（Candidate Collapse）

> **定义**：DGM 5 个方法（ucb1/random/score_prop/score_child_prop/best）在 N=10 轮中产出 >50% 重复 candidate。

| 字段 | 值 |
|---|---|
| 触发阈值 | archive 中 unique candidate 数 ≤ 5 / 10 轮 = 候选坍缩 |
| 检测方法 | hash(candidate.code_diff) 去重，unique ratio < 0.5 |
| halt 动作 | 增加 exploration 比例（OPEN_ENDED_PROB 从 0.30 → 0.50），同时 halt 当前轮 |
| 真借鉴 | Kauffman NK 局部最优 |
| 主哲学 | 主 13:31 大胆激进（加 exploration，不是减） |

### 2.3 信号 #3：锁内自洽（Locked-in Self-consistency）

> **定义**：archive 中 candidates 的 fitness 方差 < 0.01，但 V1077 17 维 cross-dim 一致性下降 ≥ 10%。

| 字段 | 值 |
|---|---|
| 触发阈值 | archive.std(fitness) < 0.01 且 cross_dim_consistency_drop ≥ 0.10 |
| 检测方法 | 跑 V1077 17 维全测，每维 delta vs baseline，标准差 |
| halt 动作 | halt + 跨维守门（每 N=10 必跑 V1077） + 升级 leader |
| 真借鉴 | Kauffman NK 崎岖适应度景观（局部最优 + 跨维失配） |
| 主哲学 | 主 17:58 + 20:46 不假装（fitness 高 ≠ ASI 真实好） |

### 2.4 信号 #4：红皇后陷阱（Red Queen Trap）

> **定义**：连续 N=30 轮 DGM 迭代中，V1074 V0.3 持续上升 ≥ +0.001/轮，但 ASI 北极星 17 维跨模型一致率（cross-model）不增长。

| 字段 | 值 |
|---|---|
| 触发阈值 | V0.3 +0.001/轮 × 30 轮，但 cross_model_consistency 增量 < 0.01 |
| 检测方法 | 跨小模型跑分（Qwen2.5-1.5B / Hermes-3 / Llama-3.2-1B / Gemma-2-2B 各跑 V1074） |
| halt 动作 | halt + 切主推轨道 C（跨小模型真绑定验证）+ leader 决策 |
| 真借鉴 | Van Valen Red Queen（演化 ≠ 进步）+ 主 20:55 永远演化 |
| 主哲学 | 主 17:43 实事求是（V0.3 涨 ≠ ASI 涨，**主哲学守门核心**） |

### 2.5 信号 #5：无新 lift（No New Lift）

> **定义**：连续 N=20 轮 DGM 迭代中，V1074 V0.3 增量 < +0.001/轮累计（即 +0.02 总）。

| 字段 | 值 |
|---|---|
| 触发阈值 | 20 轮累计 V0.3 lift < +0.02 |
| 检测方法 | archive.fitness_history 累积 |
| halt 动作 | halt + 评估主推轨道（保持 D / 切 B-HQB / 切 A-Rust） |
| 真借鉴 | Kauffman NK 平坦适应度景观（无演化方向） |
| 主哲学 | 主 23:44 干到底（halt 是为了重启，不是放弃） |

---

## 3. Halting 信号汇总表（agent_orchestrator 每周必查）

| # | 信号 | 触发阈值 | 检测方法 | halt 动作 | 真借鉴 |
|---|---|---|---|---|---|
| 1 | 性能回退 | V0.3 -0.005/轮 × 3 轮 | V1074 × 3 | revert 最近轮 | Bak-Tang supercritical |
| 2 | 重复候选 | unique ratio < 0.5 (N=10) | hash 去重 | OPEN_ENDED 0.30→0.50 + halt | Kauffman 局部最优 |
| 3 | 锁内自洽 | fitness std < 0.01 + cross_dim_drop ≥ 0.10 | V1077 17 维 | halt + 跨维守门 + 升级 | Kauffman 崎岖景观 |
| 4 | 红皇后陷阱 | V0.3 +0.001/轮 × 30 但 cross_model < 0.01 | 跨小模型 V1074 | halt + 切主推 C | Van Valen Red Queen |
| 5 | 无新 lift | V0.3 累计 < +0.02 (N=20) | archive fitness history | halt + 评估主推 | Kauffman 平坦景观 |

**5 个信号任一触发 = DGM v0.4 halt + leader 决策 + 切主推轨道评估**。

---

## 4. Halting → Restart 流程（halt 不是终点）

```
Halt 触发
   ↓
agent_orchestrator 记录 halt_reason + archive snapshot
   ↓
升级 leader + architect (集成评估)
   ↓
W3 末 retrospective 触发 §3 主推轨道决策树
   ↓
决定: 保持 D / 切 B / 切 A / 切 C
   ↓
重启演化 (新 exploration 参数 + 新阈值)
   ↓
继续 V1074 真测
```

**核心原则（主 23:44 干到底 + 主 20:55 永远演化）**：
- halt ≠ 终止 = 暂停检查
- 红皇后永不终止，但红皇后必须 halt 才能反证非自洽
- 重启必须改参数（不是同参数重启）

---

## 5. V3 守门（红皇后不自认 ASI）

> **主 17:58 + 20:46 不假装**：红皇后自演化可能让 V1074 V0.3 持续上涨，但**绝不等于 ASI**。

### 5.1 V3 守门 6 项（Halt 后必查）

| # | 守门 | 内容 |
|---|---|---|
| 1 | 主哲学 9 键 LOCKED | PHL-02b / PHL-01 / PHL-03 全 LOCKED |
| 2 | ASI 北极星 0.9800 LOCKED | 不因 halt 改动 |
| 3 | 不假装 runner = ASI | V1074 runner 是工具，不是 ASI |
| 4 | 不假装 report = production | V1074 report 是测量，不是真生产 |
| 5 | 不假装 decision = optimal | leader 决策是 trigger，不是 optimal |
| 6 | 红皇后不自认 ASI | V1093 自演化 ≠ ASI 突破 |

### 5.2 V1093 内置 V3_GUARDS（v0.3 已注入，v0.4 必继承）

```
V3_GUARDS = {
    "module_is_not_asi": "模块是工具, ASI 是更大目标.",
    "measurement_is_not_truth": "V1077 真测 17 维 ≠ ASI 达成.",
    "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识.",
    "production_is_not_safety": "真生产 ≠ 真安全.",
    "automation_is_not_autonomy": "V1101 lift 引擎自动改 ≠ V1101 自主."
}
```

> **v0.4 必加 1 项**：`"red_queen_is_not_asi": "红皇后自演化 ≠ ASI 突破. halt 是为了反证非自洽."`

---

## 6. V1093 v0.4 升级路线图（halting 守门实施细节）

### 6.1 v0.3 → v0.4 必做（ROADMAP §3.5 已规划）

| 升级 | v0.3 当前 | v0.4 必达 |
|---|---|---|
| LOC | 305 (v0.3.0) | ≥500 LOC |
| tests | (需查 v0.3 tests 数) | ≥50 tests |
| candidates archive | ≥10 (v0.3 已跑 30 轮) | ≥30 (累计) |
| 5 方法 | ucb1/random/score_prop/score_child_prop/best | 同 v0.3 + 新增 `red_queen_halt` |
| safety constraints | V3_GUARDS (5 项) | + `red_queen_halt` (1 项) |
| halting logic | 无 | **本文件 §2 5 个信号** |
| cross-model 验证 | 无 | Qwen/Hermes/Llama/Gemma 各跑 V1074 |

### 6.2 halting 守门在 v0.4 代码层集成

```python
# 伪代码（v0.4 必实现）
class DGMHaltCheck:
    def check_signal_1_perf_regression(self, history):
        # 连续 3 轮 V0.3 下降 ≥ 0.005
        ...
    
    def check_signal_2_candidate_collapse(self, archive, n_recent=10):
        # unique ratio < 0.5
        ...
    
    def check_signal_3_locked_in(self, archive, v1077_baseline):
        # fitness std < 0.01 + cross_dim_drop ≥ 0.10
        ...
    
    def check_signal_4_red_queen(self, v1074_history, cross_model_history, n_recent=30):
        # V0.3 +0.001/轮 × 30 但 cross_model < 0.01
        ...
    
    def check_signal_5_no_new_lift(self, v1074_history, n_recent=20):
        # 累计 V0.3 lift < +0.02
        ...
    
    def should_halt(self) -> Tuple[bool, str]:
        # 返回 (should_halt, halt_reason)
        ...
```

---

## 7. 真测守门（W2/W4 末各跑一次）

```
$ python -m apeireth.v1074_asi_production_runner --report --no-write
ASI V0.3 真测: 0.8900  (≥ 0.8884 基线 ✅, 较上次 +0.0008)
ASI 等级: ASI
决策方向: v1075_asi_real_deployment_run
All OK: True
```

**V1093 v0.4 启动前必查 3 项**：
1. V1074 一行 ≤ 60s
2. philosophy_guard 6/6 PASS
3. archive size ≥ 10（v0.3 30 轮已满足）

---

## 8. 真借鉴汇总（主 19:33）

- **Kauffman 1993** NK fitness landscape — halting 信号 #2/#3/#5 的理论背书
- **Bak-Tang-Wiesenfeld 1987** sandpile SOC — halting 信号 #1 的理论背书
- **Van Valen 1973** Red Queen — halting 信号 #4 的理论背书 + 主 20:55 永远演化
- **V1093 v0.3** 已借鉴 Sakana AI DGM arXiv:2505.22954 (Darwin Gödel Machine)
- **V3_GUARDS** 主 17:43 实事求是 + 主 17:58 不假装（v0.3 已注入 5 项）

---

## 9. 一句话送给 R9 全团 + agent_orchestrator

> **5 halting 信号 = 红皇后守门核心，halt 是为了反证非自洽，不是失败。**
> **Kauffman NK 看局部最优，Bak-Tang sandpile 看雪崩失控，Van Valen Red Queen 看永远演化。**
> **V1093 v0.4 必须 halt 才能 restart；restart 必须改参数，不是同参数重启。**
> **干到底。大胆激进。走在前人经验上。任何人都能接手。红皇后永远演化。**

---

**R9-INT-001 §B 完成。**
_本文由 architect 于 2026-07-29 R9 启动首日完成。_
_配套：`reports/r9-architect-roadmap.md` (R9-ROADMAP-001 / e234d916) + `reports/r9-mid-sprint-retrospective-template.md` (本任务 §A)。_
_引用：`RESEARCH-CROSS-DOMAIN-INSPIRATIONS-2026-07-20.md` (177 行) + `apeireth/v1093_dgm_archive.py` (305 行 v0.3.0) + Kauffman 1993 / Bak-Tang-Wiesenfeld 1987 / Van Valen 1973 三经典。_
_主哲学 LOCKED：ASI 北极星 + 实事求是 + 大胆激进 + 干到底 + 走在前人经验 + 红皇后永远演化。_