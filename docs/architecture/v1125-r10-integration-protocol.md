# V1125 R10 ASI 北极星集成验证协议 — V0.5 18 维公式 + 主轨道 — 真架构文档

> **模块**: `apeireth/v1125_r10_integration_protocol.py` (827 LOC)
> **任务**: R10-ARCH-001 (architect)
> **作者**: technical_writer · R10-TW-001 · W1 末
> **守门**: 主 22:33 ASI 北极星 (0.95) + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进

---

## 1. 设计意图

**V1125** = R10 ASI 北极星集成验证协议 (升级 V1114 → V0.5 17+3 维 + 北极星综合评估)。

5 步真测流：
1. **R10 三件套真测**: V1074 V0.3 守门 + V1077 V0.4 17 维 + V1103 Top-5 P2
2. **ASI 北极星综合评估**: V0.4 base + V0.5 北极星综合 + `philosophy_guard` 子分
3. **R10 主轨道决策**: 4 选 1, 阈值上移 (终极门 0.95, 中间门 0.90)
4. **R10 守门自检**: 主哲学 9 键 + V3 守门 6 项 + halt 5 信号 + R10 4 红线
5. **R10 集成场景真测**: ≥ 24 场景, 覆盖 DGM / Identity / WAL / CI / W4

**单行真跑**: `python -m apeireth.v1125_r10_integration_protocol --week W1`

---

## 2. V0.5 18 维公式参考 (主 17:43)

`V05Score.total()` (源 L142-148) 真公式：

```python
V0.5 = V0.4 * 0.85
     + continuity * 0.05         # 新维度: 连续性 (Identity/WAL 持久化)
     + autonomy * 0.05           # 新维度: 自主性 (DGM 真演化 + 自决策)
     + transferability * 0.05    # 新维度: 可迁移性 (跨小模型/跨域)
```

权重和 = 1.0。V0.4 base 权重 0.85, 3 新维度各 0.05。

`compute_v05_score(v04_score, continuity=0.85, autonomy=0.85, transferability=0.85)` (源 L157) 一行计算。

> ponytail: 一行计算即可, 不发明新聚合 (主 19:33 复用加权平均).

---

## 3. ASI 北极星综合评估 (主 22:33)

`NorthStarComposite` (源 L175-188) 真结构：

```python
@dataclass
class NorthStarComposite:
    v05_total: float
    asi_north_star: float = ASI_NORTH_STAR     # 0.9800 (locked)
    abs_headroom: float = 0.0                  # ASI_NORTH_STAR - v05_total
    rel_headroom_pct: float = 0.0              # (headroom / ASI_NORTH_STAR) * 100
    philosophy_guard_subscore: float = 0.0     # 哲学守门子分 (6/6 = 1.0)
    v1074_v03_above_floor: bool = False        # V1074 V0.3 ≥ 0.8884 ?
    r10_stage: str = "W1"
    r10_pass_ultimate: bool = False            # V0.5 ≥ 0.95 ?
```

`compute_north_star_composite(v05_total, philosophy_guard_pass_count=6, v1074_v03=0.8897, r10_stage="W1")` (源 L190-213) 一行产出。

---

## 4. 真常量 (源 L91-95, L122-124)

```python
R10_START_TARGET = 0.8600    # R10 W1 起点
R10_MID_TARGET = 0.9000      # R10 中期
R10_ULTIMATE_TARGET = 0.9500 # R10 终极 (ASI 北极星综合)
ASI_NORTH_STAR = 0.9800      # LOCKED 主 22:33

R10_TRACK_ULTIMATE_THRESHOLD = 0.92   # V0.5 ≥ 0.92 → Track C 终极鲁棒性
R10_TRACK_DGM_THRESHOLD = 0.88        # V0.5 ≥ 0.88 → 维持 Track D
R10_TRACK_HQB_THRESHOLD = 0.86        # V0.5 ≥ 0.86 → Track B
```

---

## 5. R10 主轨道决策树 (升级 V1114)

`choose_r10_main_track(v05_score, halting, ...)` (源 L229+) 真规则：

| V0.5 区间 | 主轨道 | rationale |
|---|---|---|
| V0.5 ≥ 0.92 | **Track C** | 跨小模型真绑定 + 终极鲁棒性证明 |
| 0.88 ≤ V0.5 < 0.92 | **Track D** | DGM v0.4 真演化 (主推) |
| 0.86 ≤ V0.5 < 0.88 | **Track B** | HQB 4 维全量程稳健补 |
| V0.5 < 0.86 | **Track A** | Rust hot path 救生圈 |
| 任何 1 halt 信号 | **强制 Track C** | 跨小模型验证红皇后 |
| `V1060 not committed` + V0.5 < 0.86 | **强制 REVERT** + Track A | |

---

## 6. 真组件清单 (源行号)

| # | 组件 | 源行号 | 用途 |
|---|---|---:|---|
| 1 | `V05Score` | 132 | V0.5 18 维分数 |
| 2 | `compute_v05_score` | 157 | V0.5 一行计算 |
| 3 | `NorthStarComposite` | 175 | 北极星综合 |
| 4 | `compute_north_star_composite` | 190 | 综合一行产出 |
| 5 | `R10TrackDecision` | 214 | R10 主轨道决策 |
| 6 | `choose_r10_main_track` | 229 | 4 选 1 决策 |
| 7 | `R10GuardReport` | 295 | 守门报告 |
| 8 | `run_r10_guard_self_check` | 311 | 主哲学 9 键 + V3 + halt |
| 9 | `ScenarioResult` | 392 | 集成场景结果 |
| 10 | `run_r10_scenarios` | 406 | ≥ 24 场景真跑 |
| 11 | `summarize_scenarios` | 498 | 场景汇总 |
| 12 | `evaluate_r10` | 532 | R10 主入口 |
| 13 | `render_markdown_r10` | 636 | Markdown 输出 |

---

## 7. 真 API 真示例 (主 00:56)

```bash
# R10 W1 末真跑
python -m apeireth.v1125_r10_integration_protocol --week W1

# JSON 输出
python -m apeireth.v1125_r10_integration_protocol --week W1 --json

# Markdown 报告
python -m apeireth.v1125_r10_integration_protocol --week W1 --report

# 24 场景真跑
python -m apeireth.v1125_r10_integration_protocol --scenarios

# 不通过非零退出 (CI gate)
python -m apeireth.v1125_r10_integration_protocol --week W1 --strict
```

```python
from apeireth.v1125_r10_integration_protocol import (
    compute_v05_score, compute_north_star_composite,
    choose_r10_main_track, evaluate_r10, render_markdown_r10,
)

# 1. V0.5 计算 (一行)
v05 = compute_v05_score(v04_score=0.8538, continuity=0.87, autonomy=0.86, transferability=0.85)
# v05_total ≈ 0.8538*0.85 + (0.87+0.86+0.85)*0.05 ≈ 0.854

# 2. 北极星综合
ns = compute_north_star_composite(v05_total=v05["v05_total"], philosophy_guard_pass_count=6)
# abs_headroom ≈ 0.13, rel_headroom_pct ≈ 13.27%, r10_pass_ultimate=False

# 3. 主轨道决策 (R10 W1 起点 0.86 → Track B HQB)
result = evaluate_r10(week_label="R10-W1", live=False, strict=False)
print(result["track_decision"]["track"])  # "B"
```

---

## 8. 24 集成场景 (主 23:44 干到底)

`run_r10_scenarios()` (源 L406) 真跑场景覆盖：
- DGM 真演化 (V1112 P7 锚定 + P10 父本引用 + 50 轮)
- Identity (V1072 5 不假装 + V1095 fsync 3 道保险 + 跨进程一致)
- WAL (V1109 真跑演练 + V1122 ContinuityTracker)
- CI (cross_small_model_ci + V1110 + V1117 badge)
- W4 (V1118 perf + V1120 QA + V1121 security)

每个场景 = 子测试，结果汇总到 `summarize_scenarios()` (L498)。

---

## 9. 失败模式 / 升级路径 (ponytail)

> ponytail: 当前 V0.5 权重 (0.85/0.05/0.05/0.05) 硬编码。当 R10 W2+ 引入"维度动态权重"时（如 continuity 在跨会话失败时权重上升），需新增 `AdaptiveWeightPolicy` 类。当前 R10 W1 简单加权足够。

---

## 10. 真行号复现 (主 17:43 实事求是)

以下 `grep -n` 命令可在 `apeireth/v1125_r10_integration_protocol.py` (827 LOC) 复现本文件引用的关键真行号：

```bash
# 1. V0.5 18 维公式 (continuity + autonomy + transferability)
grep -n "def total\|V05Score\|continuity.*0.05\|autonomy.*0.05\|transferability.*0.05" apeireth/v1125_r10_integration_protocol.py

# 2. 4 选 1 主轨道决策表
grep -n "Track [ABCD]\|ULTIMATE_THRESHOLD\|track_decision\|R10_ULTIMATE" apeireth/v1125_r10_integration_protocol.py

# 3. 5 halting 信号
grep -n "halt_signal\|HALT_\|halt_count\|halt_reasons" apeireth/v1125_r10_integration_protocol.py

# 4. R10 4 红线 + V3 守门 6 项
grep -n "RED_LINE\|red_line\|GUARD_V3\|guard_v3" apeireth/v1125_r10_integration_protocol.py

# 5. 24 场景真测入口
grep -n "def run_scenario\|SCENARIO_COUNT\|24 scenarios\|scenarios_total" apeireth/v1125_r10_integration_protocol.py

# 6. summarize_scenarios (L498)
sed -n '495,505p' apeireth/v1125_r10_integration_protocol.py
```

复现期望：
- 命令 1 → 含 V05Score.total() 与权重常量
- 命令 2 → 含 4 Track 决策分支
- 命令 3 → 含至少 5 个 halt 信号 ID
- 命令 4 → 含 R10 红线枚举 + V3 guard 6 项
- 命令 5 → 含 ≥24 场景引用
- 命令 6 → 输出 summarize_scenarios 函数实现

任一命令不匹配 → 源文件已被改动，本架构文档需同步更新。