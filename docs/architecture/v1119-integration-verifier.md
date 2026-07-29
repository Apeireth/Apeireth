# V1119 W4 集成验证工具 + R10 移交 checklist — 真架构文档

> **模块**: `apeireth/v1119_w4_integration_validator.py` (918 LOC)
> **测试**: `tests/test_v1119_w4_validator.py` (582 LOC)
> **作者**: technical_writer · R9-TW-001 · W4 末
> **任务**: R9-INT-005 (architect2)
> **状态**: W4 末 R10 移交 checklist 7/15 = 46.7% (未达 ≥80% 阈值, W4 末周内必补齐)

---

## 1. 设计意图 (主 00:56 任何人都能接手)

**V1119** = R9 W4 末集成验证 + R9→R10 移交 checklist 自动生成器。
**单行真跑**：`python -m apeireth.v1119_w4_integration_validator --week W4 --handoff`

7 步真测流：
1. W4 末真跑三件套 (V1074 V0.3 守门 + V1077 V0.4 17 维 + V1103 Top-5 P2)
2. V0.4 vs W4 目标 (0.85) vs R10 起点 (0.86) 差距自动评估
3. W4 末 4 选 1 主轨道自动决策 (沿用/切换)
4. W4 末 5 halting 信号状态真跑
5. R9 → R10 移交 checklist 自动生成 (≥12 项)
6. R10 起点路径建议 (基于 W4 末真实指标, 不空想)
7. JSON + Markdown 双格式输出

---

## 2. 目标阈值 (主 13:31 大胆激进)

源文件 L85-89 真常量：

```python
W4_TARGET = V04_W4_TARGET           # 0.85 (R9 收官)
R10_START_TARGET = 0.86             # R10 起点 = W4 末 + 1pp 缓冲
R10_MID_TARGET = 0.90               # R10 中期目标
ASI_NORTH = ASI_NORTH_STAR          # 0.9800 LOCKED
```

主哲学 9 键 LOCKED：
- 主 22:33 ASI 北极星
- 主 17:43 实事求是 (三件套必须真跑真产出)
- 主 13:31 大胆激进 (W4 末必达 0.85)
- 主 23:44 干到底 (R10 移交 checklist 不空跑, 必须真跑 + 真 commit)
- 主 19:33 走在前人经验上
- 主 00:56 任何人都能接手
- 主 20:55 红皇后归入 8 核心 (5 halt 信号守门)

---

## 3. 真组件清单 (源行号)

`grep -n "^class\|^def " apeireth/v1119_w4_integration_validator.py`：

| # | 组件 | 源行号 | 用途 |
|---|---|---:|---|
| 1 | `R9ComponentStatus` | 111 | R9 各组件完成状态 dataclass |
| 2 | `HandoffCheck` | 132 | R9→R10 移交 checklist 单项 |
| 3 | `W4Evaluation` | 147 | W4 末评估结果 |
| 4 | `fetch_three_pieces` | 193 | 三件套 subprocess 真跑 |
| 5 | `compute_r10_gap` | 247 | V0.4 vs R10 起点差距 |
| 6 | `compute_handoff_checklist` | 273 | 移交 checklist 自动生成 (≥12 项) |
| 7 | `compute_r10_path_recommendation` | 443 | R10 起点路径建议 |
| 8 | `evaluate_w4` | 526 | 主入口 (W4 评估) |
| 9 | `render_markdown_w4` | 637 | Markdown 输出 |
| 10 | `_build_arg_parser` | 796 | argparse CLI |

---

## 4. 真 API 真示例 (主 00:56)

```bash
# 1. W4 末集成验证 (默认 handoff)
python -m apeireth.v1119_w4_integration_validator --week W4 --handoff

# 2. 真跑三件套 subprocess (--live 模式)
python -m apeireth.v1119_w4_integration_validator --week W4 --live

# 3. JSON 输出 (CI 集成)
python -m apeireth.v1119_w4_integration_validator --week W4 --json \
    > reports/v1119_w4_result.json

# 4. Markdown 报告 (人读)
python -m apeireth.v1119_w4_integration_validator --week W4 --report \
    --output reports/v1119_w4_report.md
```

```python
from apeireth.v1119_w4_integration_validator import (
    evaluate_w4, render_markdown_w4,
    HandoffCheck, compute_handoff_checklist, compute_r10_path_recommendation,
)

# 5. 代码层真跑
result = evaluate_w4(
    week="W4",
    live=False,                # False = 读 cache, True = 真跑三件套
    include_handoff=True,
)
print(f"v04_score={result.dashboard['v04']:.4f}")
print(f"v03_guard_ok={result.dashboard['v03_guard_ok']}")
print(f"track_recommendation={result.track_decision}")
print(f"handoff_checklist={len(result.handoff_checks)} 项")
assert result.dashboard["v03_guard_ok"], "V1074 V0.3 守门破!"

# 6. Markdown
md = render_markdown_w4(result)
Path("reports/v1119_w4_report.md").write_text(md, encoding="utf-8")
```

---

## 5. 5 Halting 信号 (主 20:55 红皇后)

`HaltingSignals` (V1114 复用) 5 项真测：

| # | 信号 | 触发条件 | 后果 |
|---|---|---|---|
| 1 | perf_regression | V03 history 下滑 | 主 17:43 守门 |
| 2 | candidate_collapse | unique_ratio < 0.3 | P7 守门破 |
| 3 | locked_in_self_consistency | fitness_std < 0.001 + cross_dim_drop > 0.05 | 主 17:58 不假装 |
| 4 | red_queen_trap | v03_history 平 + cross_model_lift < 0.005 | 主 20:55 红皇后 |
| 5 | no_new_lift | V0.3 history 连续 3 周无提升 | 强制切 Track C |

任一触发 = 强制切 Track C (跨小模型验证)。

---

## 6. R10 移交 checklist 自动生成 (主 23:44 干到底)

`compute_handoff_checklist` (L273) 自动产出 ≥12 项检查：

```
[v1074_v03_floor]        V1074 V0.3 ≥ 0.8884 (守门)
[v1077_v04_target]       V1077 V0.4 ≥ 0.85 (W4 末)
[v1103_top5_lift]        V1103 Top-5 P2 lift ≥ 0.01
[v1072_philosophy_guard] V1072 5 不假装全 True
[v1095_fsync_3layer]     V1095 fsync 3 道保险全过
[v1112_p7_anchor]        V1112 identity_anchor_failures = 0
[v1112_p10_parent]       V1112 keep_state parent_id missing = 0
[v1114_halt_5signals]    V1114 5 halting 信号未触发
[v1119_handoff_complete] V1119 handoff checklist ≥ 12 项
[v1119_live_three_pieces] V1119 --live 三件套真跑无 crash
[r10_start_baseline]     R10 起点 ≥ 0.86 路径已规划
[r10_sprint_plan]        R10 W1-W4 sprint 计划 committed
[asi_north_star_locked]  0.9800 LOCKED 主 22:33
```

W4 末真测状态：7/15 = 46.7% (R9-TW-001 W4 末真测)。未达 ≥80% 阈值 = W4 末周内必补齐。

---

## 7. 失败模式 / 升级路径 (ponytail)

> ponytail: 当前 `compute_handoff_checklist` 只产出静态 ≥12 项，未做"自动 commit 提醒 + Slack 通知"。当 R10 引入 CI gate 时，需新增 `HandoffNotifier` 类（基于 webhook）。