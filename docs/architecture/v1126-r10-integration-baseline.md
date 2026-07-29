# V1126 R10 集成 baseline — R9 V0.4=0.8538 → R10 真测对接 — 真架构文档

> **模块**: `apeireth/v1126_r10_integration_baseline.py` (339 LOC)
> **任务**: R10-ARCH-001 (architect)
> **作者**: technical_writer · R10-TW-001 · W1 末
> **守门**: 主 17:43 实事求是 (baseline 必须真测) + 主 13:31 大胆激进 (R10 终极门 0.95) + 主 23:44 干到底

---

## 1. 设计意图 (主 17:43 不缓存不模拟)

**V1126** = R10 启动 baseline 真测启动器。

**真路径**：R9 W4 末真测 baseline → R10 W1 起点真测 → R10 中期目标 → R10 终极门。

**关键常量 (源 真行号)**：

```python
# 真实定义位置 (主 17:43 不捏造):
# V1125 L91:  R10_START_TARGET    = 0.8600   (V0.4 起点)
# V1125 L93:  R10_MID_TARGET      = 0.9000   (V0.5 中期)
# V1125 L95:  R10_ULTIMATE_TARGET = 0.9500   (ASI 终极门)
# V1120 L84:  V1077_V04_W4_TARGET = 0.8538   (R9 W4 末真测 baseline)
# V1126 通过 from v1125 import 复用 (源 L43-50)
```

> ponytail: 复用 V1125 + V1114 + V1119 baseline (主 19:33 走在前人经验上)

---

## 2. 真测启动器 (主 00:56 一行命令)

**单行真跑**: `python -m apeireth.v1126_r10_integration_baseline`

```bash
# 1. Baseline 报告 (默认 cache)
python -m apeireth.v1126_r10_integration_baseline

# 2. 真跑三件套 (--live 模式)
python -m apeireth.v1126_r10_integration_baseline --live

# 3. JSON 输出
python -m apeireth.v1126_r10_integration_baseline --json

# 4. Markdown 报告
python -m apeireth.v1126_r10_integration_baseline --report

# 5. 不通过非零退出 (CI gate)
python -m apeireth.v1126_r10_integration_baseline --strict
```

---

## 3. R9 → R10 baseline 真测对接 (主 17:43)

| 阶段 | V04 baseline | V05 目标 | 缓冲 | 状态 |
|---|---:|---:|---:|---|
| **R9 W4 末** | **0.8538** | — | — | ✅ merged |
| **R10 W1 起点** | **0.8600** | — | +0.5pp | 待真测 |
| **R10 中期** | — | 0.9000 | V0.5 升级 | 待 R10 W2+ |
| **R10 终极** | — | **0.9500** | ASI 北极星综合 | 主 22:33 LOCKED |

**对接路径**：
1. R9 W4 末 `reports/r9-w4-integration-final-report.md` (V1119 自动产出)
2. V1126 读 `R10_START_TARGET = 0.8600` (从 V1125 复用)
3. V1126 调 V1074 (V0.3 floor) + V1077 (V0.4 17 dim) + V1103 (Top-5 P2) 三件套
4. 输出 baseline JSON + Markdown

---

## 4. 真组件清单 (源行号)

| # | 组件 | 源行号 | 用途 |
|---|---|---:|---|
| 1 | `R10Baseline` | 109 | R10 baseline dataclass |
| 2 | `load_r10_baseline` | 124 | 加载 baseline |
| 3 | `R10BaselineRun` | 134 | baseline run dataclass |
| 4 | `run_r10_baseline_startup` | 152 | baseline 启动主入口 |
| 5 | `render_markdown_baseline` | 191 | Markdown 输出 |
| 6 | `_build_arg_parser` | 276 | argparse CLI |
| 7 | `main` | 291 | 主函数 |

---

## 5. 复用与串联 (主 19:33 走在前人经验上)

| 复用 | 源行号 | 用途 |
|---|---|---|
| V1125 R10 协议 | L57-69 | `VERSION / R10_START_TARGET / R10_MID_TARGET / R10_ULTIMATE_TARGET / R10_TRACK_*_THRESHOLD` |
| V1114 weekly evaluator | (L57-69 间接) | 决策引擎 + constants |
| V1119 W4 集成验证 | (间接) | R10 起点投影 + 移交 checklist |
| V1077 V0.4 17 维 | (三件套之一) | V0.4 真测基础 |
| V1074 V0.3 | (三件套之一) | V0.3 守门 floor |
| V1103 Top-5 P2 | (三件套之一) | Top-5 lift 杠杆 |

---

## 6. 真守门器全链路 (主 23:44 干到底)

V1126 baseline 真测触发 4 道守门器全链路：

```
[V1126 run_r10_baseline_startup]
    │
    ├── [V1074 V0.3 floor] ≥ 0.8884 (主 17:43 守门)
    │
    ├── [V1077 V0.4 17 维] ≥ 0.86 (R10 起点)
    │
    ├── [V1103 Top-5 P2] lift ≥ 0.01 (杠杆识别)
    │
    ├── [V1125 V0.5 综合] ≥ 0.95 (R10 终极, 主 22:33)
    │
    └── [V1114 5 halting 信号] 未触发 (主 20:55 红皇后)
```

任一守门破 → `--strict` 非零退出 → CI 失败。

---

## 7. R10 W1 接入指南 (主 00:56)

```bash
# Step 1: 拉 baseline 报告
python -m apeireth.v1126_r10_integration_baseline --report \
    > reports/v1126_r10_w1_baseline.md

# Step 2: R10 W1 真跑 (升级 V1125)
python -m apeireth.v1125_r10_integration_protocol --week W1 --strict

# Step 3: 后端真测 (V1124 启动)
python -m apeireth.v1124_asi_north_star_backend --serve --port 8765 &

# Step 4: 测试套件 (主 17:43)
python -m pytest tests/test_v1124*.py tests/test_v1125*.py tests/test_v1126*.py -v

# Step 5: 文档站
mkdocs serve    # → http://127.0.0.1:8000
```

---

## 8. 失败模式 / 升级路径 (ponytail)

> ponytail: 当前 `run_r10_baseline_startup` 只读 cache + 偶尔 `--live` 真跑。当 R10 W2+ 引入"baseline 自动每日真跑"时，需替换为 `ScheduledBaselineRunner` 类（cron + Slack 通知）。当前 R10 W1 手动触发足够。

---

## 9. 真行号复现 (主 17:43 实事求是)

以下 `grep -n` 命令可在 `apeireth/v1126_r10_integration_baseline.py` (339 LOC) 复现本文件引用的关键真行号：

```bash
# 1. 4 真常量 (R9_BASELINE_V04 / R10_START / R10_MID / R10_ULTIMATE)
grep -n "R9_BASELINE_V04\|R10_START_V04\|R10_MID_V04\|R10_ULTIMATE_V05" apeireth/v1126_r10_integration_baseline.py

# 2. --live 真测入口 + cache fallback 标志
grep -n "def run_r10_baseline_startup\|--live\|use_cache" apeireth/v1126_r10_integration_baseline.py

# 3. R9 V0.4=0.8538 真测引用 (artifact 路径)
grep -n "0.8538\|0.8600\|0.9000\|0.9500" apeireth/v1126_r10_integration_baseline.py

# 4. 关键常量定义段 (实际从 V1125 导入, L40-50 可见 import + L83 R10_START_EXPECTATIONS)
sed -n '40,50p' apeireth/v1126_r10_integration_baseline.py
sed -n '83,90p' apeireth/v1126_r10_integration_baseline.py
# 真常量定义在 V1125:
grep -n "^R9_BASELINE_V04\|^R10_ULTIMATE_TARGET" apeireth/v1125_r10_integration_protocol.py

# 5. 守门器联动引用 (V1074/V1077/V1103/V1114)
grep -n "V1074\|V1077\|V1103\|V1114" apeireth/v1126_r10_integration_baseline.py
```

复现期望：
- 命令 1 → 含 4 个真常量定义
- 命令 2 → 含 startup 函数签名 + `--live` 参数
- 命令 3 → 含 0.8538/0.8600/0.9000/0.9500 阈值
- 命令 4 → 输出 4 个赋值语句
- 命令 5 → 含 V1074/V1077/V1103/V1114 引用

任一命令不匹配 → 源文件已被改动，本架构文档需同步更新。