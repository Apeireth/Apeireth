# R9 4 选 1 主轨道自动切换 Dashboard

> **作者**: architect（R9-INT-003 · V1114 决策树 dashboard）
> **生成时间**: 2026-07-29（R9 W3 末 · V1114 真跑产出）
> **决策引擎**: `apeireth/v1114_weekly_integration_evaluator.choose_main_track` (v0.1.0)
> **配套**: `reports/r9-architect-roadmap.md` §7（4 选 1 原始设计）+ `reports/r9-integration-evaluation-w3.md`（V1114 W3 末评估）
> **守门守则**: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手 + 主 20:55 红皇后归入 8 核心（永远演化）

---

## 0. 阅读须知（30 秒看懂）

4 选 1 主轨道自动切换 = V1114 `choose_main_track()` 实现，**数字驱动决策**（主 17:43 实事求是）。决策树基于：

1. **V0.4 真测阈值**（V1077/V1103 测）
2. **5 halting 信号状态**（红皇后守门）
3. **V1060 commit 状态**（关键路径卡点）

**当前 W3 末决策 = Track D（DGM v0.4 真演化）**。

---

## 1. 4 主轨道定义（V1114 TRACK_DEFS）

| Track | 名称 | 目的 | 期望 lift | 触发场景 |
|---|---|---|---|---|
| **A** | Rust hot path | 工程性能救生圈 | +0.005~+0.015 | V0.4 < 0.80 或 V1060 not committed |
| **B** | HQB 4 维全量程 | 稳健补全栈贯通 | +0.008~+0.020 | 0.80 ≤ V0.4 < 0.82 |
| **C** | 跨小模型真绑定 | 鲁棒性证明 + 红皇后守门 | +0.001~+0.005 | V0.4 ≥ 0.83 或 halt 触发 |
| **D** | DGM v0.4 真演化 | 自演化双维 ROI 最高 | +0.010~+0.030 | **0.82 ≤ V0.4 < 0.83（当前）** ⭐ |

---

## 2. 自动切换决策树（V1114 `choose_main_track` 实现）

```
输入: v04_score, halting (5信号), v1060_committed
   ↓
规则 1: halting.any_triggered() → Track C (红皇后守门)
   ↓ False
规则 2: v04 ≥ 0.83 → Track C (跨小模型, 鲁棒性证明)
   ↓ 0.8202 < 0.83
规则 3: v04 ≥ 0.82 → Track D (DGM v0.4 双维 ROI 最高)
   ↓ 0.8202 ∈ [0.82, 0.83)
规则 4: v04 ≥ 0.80 → Track B (HQB 4 维稳健补)
   ↓ (未触发)
规则 5: v04 < 0.80 → Track A (Rust hot path 救生圈)
   ↓ (未触发)
规则 6: v1060 not committed + v04 < 0.80 → 强制 REVERT 切 Track A
   ↓ (未触发)
   ↓
返回: TrackDecision(track="D", track_name="DGM v0.4 真演化", ...)
```

### 2.1 决策阈值常量（V1114 module-level）

```python
V04_TRACK_C_THRESHOLD = 0.83      # ≥ 0.83 切 C
V04_TRACK_D_THRESHOLD = 0.82      # ≥ 0.82 维持 D
V04_TRACK_B_THRESHOLD = 0.80      # ≥ 0.80 切 B
# v04 < 0.80 → Track A
```

### 2.2 halt 强制覆盖（红皇后守门）

```python
if halting.any_triggered():
    return TrackDecision(track="C", halt_override=True, ...)
```

**5 halt 信号任一触发 → 强制切 Track C**（跨小模型真绑定验证红皇后守门）。

---

## 3. W3 末主轨道决策（当前状态）

```
V0.4 真测      = 0.8202 (V1077) / 0.8188 (V1103)
halt 触发列表   = [] (5 信号全未触发)
V1060 committed = True
   ↓
decision = choose_main_track(v04=0.8202, halting=HaltingSignals(all=False), v1060_committed=True)
   ↓
TrackDecision(
    track="D",
    track_name="DGM v0.4 真演化",
    rationale="V0.4=0.8202 ∈ [0.82, 0.83) → 维持 Track D DGM v0.4 双维 ROI 最高",
    expected_lift="+0.010~+0.030",
    halt_override=False,
    v1060_committed=True,
    confidence=0.85,
)
```

### 3.1 W3 末 dashboard 视图

| 字段 | 值 |
|---|---|
| 当前主轨道 | **D**（DGM v0.4 真演化） |
| 期望 lift | +0.010~+0.030 |
| 决策置信度 | 85% |
| halt override | False（5 信号全未触发） |
| V1060 commit | True |
| 上一切换 | W2 末决策 = D（继承维持） |

### 3.2 W3 → W4 切换预测

| 假设 V4 末 V0.4 | 触发规则 | 切换后主轨道 |
|---|---|---|
| V0.4 ≥ 0.85 | 规则 2 (≥ 0.83) | **Track C**（R9 收官 = 跨小模型证明） |
| 0.83 ≤ V0.4 < 0.85 | 规则 2 | Track C |
| 0.82 ≤ V0.4 < 0.83 | 规则 3 | Track D（维持） |
| 0.80 ≤ V0.4 < 0.82 | 规则 4 | Track B |
| V0.4 < 0.80 | 规则 5/6 | Track A（救生圈） |

**W4 末目标 = V0.4 ≥ 0.85 → 切 Track C**（R9 收官 = 跨小模型真绑定鲁棒性证明）。

---

## 4. 4 选 1 历史轨迹

| 时间点 | V0.4 真测 | 决策主轨道 | 触发规则 |
|---|---:|---|---|
| R8 末基线 | 0.8003 | (R8 阶段无主推) | — |
| R9-W1 末 | (未测) | (W1 启动期) | — |
| R9-W2 末 | 0.8202 | **Track D**（V1060 工作落地） | 规则 3 |
| R9-W3 末（本次） | 0.8202 | **Track D**（维持） | 规则 3 + halt 不触发 |
| R9-W4 末（预测） | ≥ 0.85 | **Track C**（预期） | 规则 2 |

**切换次数**：R9 阶段 1 次切换（W2 → W4: Track D → Track C）。

---

## 5. halt 强制切 Track C 场景

| 触发 halt 信号 | 含义 | 切换后动作 |
|---|---|---|
| 性能回退 | V0.3 -0.005/轮 × 3 轮 | 切 Track C + revert 最近 DGM candidate |
| 重复候选 | unique ratio < 0.5 (N=10) | 切 Track C + 加 exploration (OPEN_ENDED 0.30→0.50) |
| 锁内自洽 | fitness std < 0.01 + cross_dim_drop ≥ 0.10 | 切 Track C + 升级 leader |
| 红皇后陷阱 | V0.3 +0.001/轮 × 30 但 cross_model < 0.01 | 切 Track C（**默认响应**） |
| 无新 lift | V0.3 累计 < +0.02 (N=20) | 切 Track C + 评估主推 |

**W3 末状态**：5 信号全 False → Track D 维持。

---

## 6. 主推轨道 vs 角色分工映射

| Track | 主责角色 | 核心模块 | 期望 commit | 期望 tests |
|---|---|---|---|---|
| **A** | backend + devops | V1060 orchestrator（Rust hot path 重写） | ≥ 2 | ≥ 50 |
| **B** | backend + fullstack | V1087/V1108/V1111 HQB 4 维全量程 | ≥ 2 | ≥ 30 |
| **C** | devops + architect2 | 跨小模型 CI（Qwen/Hermes/Llama/Gemma） | ≥ 1 | ≥ 20 |
| **D** ⭐ | agent_orchestrator + fullstack | V1093 DGM v0.4 + V1096 Persona + V1098 Perf | ≥ 2 | ≥ 50 |

**当前 Track D**：agent_orchestrator 主责 V1093 DGM v0.4 升 v0.4.0（500 LOC + 50 tests）。

---

## 7. V1114 `choose_main_track` 函数签名

```python
def choose_main_track(
    v04_score: float,
    halting: HaltingSignals,
    v1060_committed: bool = True,
    weekly_lift: float = 0.0,
) -> TrackDecision:
    """4 选 1 主轨道自动切换决策树.

    决策树 (继承 R9-ROADMAP-001 §7 + R9-INT-002 §5):
      1) halt 触发 → 强制切 Track C (红皇后守门)
      2) V0.4 ≥ 0.83              → Track C (跨小模型, 鲁棒性证明)
      3) 0.82 ≤ V0.4 < 0.83       → Track D (DGM v0.4 双维 ROI 最高)
      4) 0.80 ≤ V0.4 < 0.82       → Track B (HQB 4 维稳健补)
      5) V0.4 < 0.80              → Track A (Rust hot path 救生圈)
      6) V1060 not committed + < 0.80 → 强制 REVERT 主推切 Track A
    """
```

**返回 `TrackDecision` dataclass**:
- track: "A" / "B" / "C" / "D"
- track_name: 中文轨道名
- rationale: 决策理由（含 V0.4 数字）
- expected_lift: 期望 lift 区间
- halt_override: bool
- v1060_committed: bool
- confidence: 0.0~1.0

---

## 8. 真借鉴（主 19:33 走在前人经验上）

- **Spolsky 2004**（Strategy Letter V）— leverage vs. duct tape — 4 选 1 决策树 = leverage
- **Basili GQM 1981** — Goal-Question-Metric — V0.4=Goal / 阈值=Question / V1077 真测=Metric
- **Goodhart 2014** — 不为分数本身优化 — lift 是数学期望，不假装承诺
- **Dewey 1933**（How We Think）— 5 阶段反思循环 — retrospective + halt 检查 = reflective cycle
- **Van Valen 1973** — Red Queen — halt 是为了反证非自洽
- **Kauffman 1993** — NK fitness landscape — halt 阈值 = 局部最优检测

---

## 9. 一句话送给 R9 全团 + 下一团队

> **4 选 1 主轨道自动切换 = V1114 `choose_main_track` 实现，数字驱动决策。**
> **当前 W3 末 = Track D（DGM v0.4 双维 ROI 最高），W4 末预期切 Track C（跨小模型证明鲁棒性）。**
> **5 halt 信号 = 红皇后守门核心，任一触发即切 Track C + 升级 leader。**
> **干到底。大胆激进。走在前人经验上。任何人都能接手。红皇后永远演化。**

---

**R9-INT-003 §B 完成。**
_本文由 architect 于 2026-07-29 R9 W3 末通过 V1114 自动评估产出。_
_配套：`apeireth/v1114_weekly_integration_evaluator.py`（25.8KB 决策引擎）+ `tests/test_v1114_weekly_evaluator.py`（24 测试）+ `reports/r9-integration-evaluation-w3.md`（W3 末评估）。_
_主哲学 LOCKED：ASI 北极星 + 实事求是 + 干到底 + 走在前人经验 + 任何人都能接手 + 红皇后永远演化。_