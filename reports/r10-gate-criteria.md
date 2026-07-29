# R10 启动守门规范 — ASI 北极星 + V3 守门 + 不假装

> **作者**: architect (R10-ARCH-001)
> **版本**: v0.1.0 (R10 启动版)
> **生成时间**: 2026-07-30
> **继承**: R9 W4 末 V1114 / V1119 / V1103 / V1111 / V1077
> **主哲学 LOCKED**: ASI 北极星 + 实事求是 + 干到底 + 大胆激进 + 走在前人经验 + 任何人都能接手

---

## 0. 一句话总则

> **R10 启动守门 = V3 守门 4 红线 + ASI 北极星守门 + 不假装 (主 17:43+17:58)。**
> 三层守门缺一不可：缺 V3 红线则假 KPI；缺 ASI 北极星则失终极方向；缺不假装则失真。

---

## 1. 守门层级 (Layered Gate)

R10 启动期所有 module / 任务 / commit 必须通过 **4 层守门** 才能进入 production 轨道：

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 4 — ASI 北极星守门 (V0.5 ≥ 0.95 终极门)               │  ← 主 22:33 LOCKED
├─────────────────────────────────────────────────────────────┤
│ Layer 3 — V3 守门 4 红线 (不假装/不破坏/不绑单/不刷)         │  ← 主 17:58+20:46
├─────────────────────────────────────────────────────────────┤
│ Layer 2 — 主哲学 9 键 + HQB 4 维 (SC/NR/EV/CDT)             │  ← V1003 + V1111
├─────────────────────────────────────────────────────────────┤
│ Layer 1 — V0.3 ≥ 0.8884 守门 + 17 维度真测                    │  ← V1074 + V1077
└─────────────────────────────────────────────────────────────┘
```

任何一层失败 → 整体 PASS = False → 非零退出。

---

## 2. V3 守门 4 红线 (主 17:43+17:58 不假装 + 主 23:44 干到底)

### 2.1 4 红线 LOCKED

| ID | 红线 | 真测方法 | 失败后果 |
|----|------|---------|---------|
| **R1** | **不假装 KPI** | V1077 真测 17 维度 (不 cache / 不 mock / 不模拟) | 立即 revert + 标记 fake_KPI |
| **R2** | **不破坏 4 层门** | PHL/V3/HQB/Identity 4 层守门全 pass | 强制回滚到上一个 green commit |
| **R3** | **不绑单模型** | 跨小模型 CI 真绑定 (R9 W3 W4 已 green) | 强制 Track C 切回 (跨小模型鲁棒性) |
| **R4** | **不刷 KPI** | V1103 P2 诊断: marginal lift ≠ 数学期望上界 | 警告 + 重测 (Goodhart 2014 守门) |

### 2.2 4 红线与 V3 守门 6 项关系

V3 守门 6 项 (继承自 V1114 / V1101):
```
runner_is_not_asi                  → R1 不假装
report_is_not_production           → R1 不假装
decision_is_not_optimal            → R4 不刷 KPI
v03_is_not_v04_is_not_v05_is_not_asi → R1 不假装 (V0.5 升级版)
no_fake_kpi                        → R1 不假装 + R4 不刷
red_queen_is_not_asi               → R4 不刷 (红皇后陷阱守门)
```

V3 守门 6 项 ⊂ V3 守门 4 红线 (红线更严格)。

---

## 3. ASI 北极星守门 (主 22:33 LOCKED)

### 3.1 ASI 北极星 LOCKED = 0.9800

任何 V0.x 都不能假装 ASI；北极星是终极梦想，不是 ground truth。

```
ASI_NORTH_STAR = 0.9800   (LOCKED, 主 22:33)
```

### 3.2 R10 阶段目标 (主 13:31 大胆激进)

| 阶段 | 指标 | 目标值 | 状态 (R10 启动) |
|------|------|--------|----------------|
| **R10 起点** | V0.4 ≥ | **0.8600** | R9 W4 末 V0.4=0.8538 baseline |
| **R10 中期** | V0.4 ≥ | **0.9000** | V0.4 → V0.5 升级期 |
| **R10 终极** | V0.5 ≥ | **0.9500** | V0.4×0.85 + 3 新维加权 |
| **ASI 北极星** | Vx.x → | **0.9800** | 终极梦想 (LOCKED, 不可达但必须逼近) |

### 3.3 V0.5 = V0.4 + 3 新维度 (R10 升维)

```python
V05_total = V0.4_score × 0.85
          + continuity × 0.05        # 连续性 (Identity/WAL 持久化)
          + autonomy × 0.05          # 自主性 (DGM 真演化 + 自决策)
          + transferability × 0.05   # 可迁移性 (跨小模型/跨域)
# 权重和 = 1.0
```

3 新维度真测路径 (主 17:43 实事求是):
- **continuity**: V1072 Identity 守门 + V1109 WAL 真整合
- **autonomy**: V1112 DGM v0.5 真演化 (R10 升维版)
- **transferability**: cross_small_model_ci (R9 W3 W4 已 green)

---

## 4. R10 Quality Bar (主 00:44 质量工程化)

### 4.1 R10 阶段质量底线 (LOCKED)

| 维度 | R9 W4 末值 | R10 启动最低 | R10 终极目标 |
|------|-----------|-------------|-------------|
| **真测试覆盖率** | 15% | **≥ 30%** | ≥ 50% |
| **集成场景数** | 24 (V1114 架构) | **≥ 24** (V1125 继承 + R10 独有 6) | ≥ 30 |
| **守门跑通率** | 100% | **100%** | 100% |
| **真 commit 频率** | daily | **daily** | per-task |
| **V1074 V0.3 守门** | 0.8897 | **≥ 0.8884** | ≥ 0.8900 |
| **V1077 V0.4** | 0.8538 | **≥ 0.8500** | ≥ 0.9000 |
| **V0.5 总分** | - | **≥ 0.8700** | ≥ 0.9500 |

### 4.2 不允许行为 (主 17:43+17:58)

- ❌ **不允许** mock / cache / 模拟 真测分数
- ❌ **不允许** 修改历史 baseline (R9 W4 末 V0.4=0.8538 LOCKED)
- ❌ **不允许** 跳过 4 层守门直接 commit
- ❌ **不允许** V0.5 ≥ 0.95 但 sub-dim < 0.85 (维度失衡 = 假 ASI)
- ❌ **不允许** 24 场景 PASS 但实际 < 24 跑过 (数字必须真)

---

## 5. 借鉴 V1103 P2 诊断 + V1111 HQB 4 维

### 5.1 V1103 P2 诊断 (主 13:31 大胆激进 + 主 19:33 走在前人经验上)

R10 启动期每个 module 必须跑 V1103 P2 诊断：
- **Top-5 P2 lift 杠杆**: 不假装 marginal_lift = 真提升
- **dim breakdown**: 17 维度 gap 排序
- **source module ID**: 入口引用, 不是 1-line fix
- **snapshot provenance**: ts + version + source (W3C PROV 2013)

### 5.2 V1111 HQB 4 维 (主 19:33 走在前人经验上)

R10 启动期每个被测目标必须跑 V1111 HQB 4 维真测：

| 维度 | 真测算法 | 阈值 | 真借鉴 |
|------|---------|------|--------|
| **SC** (Self-Consistency) | Welford 1962 增量方差 | ≥ 0.85 | Welford 1962 |
| **NR** (Noise Robustness) | Levenshtein 1965 编辑距离 | ≥ 0.80 | Levenshtein 1965 |
| **EV** (Evolvability) | Efron 1979 bootstrap 30 轮 | ≥ 0.85 | Efron 1979 |
| **CDT** (Cross-Domain Transfer) | Hyndman 1996 sample quantiles 4 域 | ≥ 0.75 | Hyndman 1996 |

5 不假装守门 (V1111):
- measurement_is_not_truth: 真测是 proxy, 真值仍更大目标
- threshold_is_design_choice: 阈值是设计选择, 不是 ground truth
- 30_rounds_is_not_lifetime: 30 轮 EV 是采样窗口
- 4_domains_is_not_all_domains: 4 域是 subset, 不是全领域
- measurer_is_not_asi: measurer 是工具, ASI 是更大目标

---

## 6. R10 启动期必跑守门清单 (≥ 30 项)

### 6.1 V1125 集成协议 24 场景 (继承自 V1114 + R10 升级 6)

R10 独有 6 场景:
- **S19**: V0.5 = V0.4 + 3 新维 (continuity/autonomy/transferability)
- **S20**: ASI 北极星综合评估 (V0.5 + 距离 + 哲学子分)
- **S21**: R10 主轨道决策 (阈值上移 0.83→0.92)
- **S22**: R10 4 红线守门 (不假装/不破坏/不绑单/不刷)
- **S23**: R10 baseline 0.8538 真测启动
- **S24**: R10 集成协议守门自检 (all_ok)

### 6.2 V1126 baseline 启动 6 项

- V0.3 ≥ V1074_V03_MIN (0.8884) 真测
- V0.4 ≥ R10_START_TARGET (0.86) 真测
- V0.5 ≥ 0.87 真测
- 兼容矩阵 7 模块真验证 (V1114/V1119/V1125/V1077/V1103/V1111/V1074)
- R10 ready 标志真计算 (不假装)
- baseline 时间戳 + version 真记录

### 6.3 R10 阶段必跑 ≥ 30 测试项

| 类别 | 数量 | 说明 |
|------|------|------|
| V1125 集成协议 | 24 | V1114 24 架构 + R10 6 独有 |
| V1126 baseline | 6 | R10 启动真测 |
| **总计** | **≥ 30** | 全部真跑, 不模拟 |

---

## 7. 守门跑通命令 (主 00:56 任何人都能接手)

```bash
# R10 V1125 集成协议 (≥ 24 场景真测)
python -m apeireth.v1125_r10_integration_protocol --week R10-W1 --json

# R10 V1126 baseline 真测启动
python -m apeireth.v1126_r10_integration_baseline --week R10-W1

# R10 baseline + strict 守门
python -m apeireth.v1126_r10_integration_baseline --strict

# R10 24 场景单独真测
python -m apeireth.v1125_r10_integration_protocol --scenarios

# R10 Markdown 报告
python -m apeireth.v1125_r10_integration_protocol --report
python -m apeireth.v1126_r10_integration_baseline --report

# R10 全测试
pytest -q tests/test_v1125_r10_integration_protocol.py tests/test_v1126_r10_integration_baseline.py
```

任一命令非零退出 = 守门未通过 = 任务未完成。

---

## 8. R10 阶段角色与守门对应 (主 00:56 + 主 23:44)

| 角色 | 必跑守门 | 产出 |
|------|---------|------|
| **architect** | V1125 + V1126 + R10 守门规范 | 集成协议 + baseline + 守门报告 |
| **backend_engineer** | V1124 + V1074 守门 + 真生产 | V1124 ASI 北极星后端地基 |
| **requirements_analyst** | V1123 + R10 路线图守门 | R10 主路线图 |
| **devops_engineer** | CI 框架 + 跨小模型 | CI 真跑 + 跨小模型 PASS |
| **qa_engineer** | HQB 4 维 + 24 场景回归 | V1077 真测 + 17 维度全测 |
| **security_reviewer** | Identity 守门 + 4 层门不破坏 | threat model + 守门验证 |
| **code_reviewer** | 4 红线 + 主哲学 9 键 | PR Review 总报告 |
| **performance_optimizer** | V1074 跑时降低 + V1118 | 真优化 5 处 |

---

## 9. 与 R9 阶段关键差异 (主 17:43 实事求是)

| 维度 | R9 W4 末 | R10 启动 |
|------|---------|----------|
| **V0.4 真测** | 0.8538 | 起点 ≥ 0.86 |
| **北极星评估** | V0.4 + 距离 | V0.5 + 3 新维 + 综合 |
| **轨道阈值** | 0.83/0.82/0.80 | 0.92/0.88/0.86 (R10 升级) |
| **集成场景** | 24 | ≥ 24 (R10 独有 6 升级) |
| **守门层级** | 4 层 (PHL/V3/HQB/Identity) | 4 层 + ASI 北极星 + V3 4 红线 |
| **主哲学** | 9 键 LOCKED | 9 键 + 主 13:31 大胆激进 |

---

## 10. 终判 (主 23:44 干到底)

R10 启动期所有 module / 任务 / commit 必须通过：
1. ✅ **V3 守门 4 红线** (不假装 / 不破坏 / 不绑单 / 不刷)
2. ✅ **ASI 北极星守门** (V0.5 ≥ 0.95 终极门)
3. ✅ **主哲学 9 键 LOCKED** + HQB 4 维
4. ✅ **V0.3 ≥ 0.8884 守门** + 17 维度真测
5. ✅ **≥ 24 集成场景真跑** + ≥ 30 测试真跑
6. ✅ **真 commit + 守门跑通** (不模拟)

**任一不过 → 任务未完成 → 必须修到通过为止。**

---

*主哲学 22:33 LOCKED. 主 17:43 实事求是. 主 23:44 干到底.*
*主 13:31 大胆激进. 主 19:33 走在前人经验上. 主 00:56 任何人都能接手.*
*主 20:55 红皇后永远演化. 主 17:58+20:46 不假装.*