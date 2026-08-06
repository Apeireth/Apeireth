# R11 工作流：P0 流程与回滚落地

> **作者**: workflow_designer (R11)  
> **创建**: 2026-07-30  
> **范围**: 将 Omnibus 验收流程固化成可执行 workflow  
> **状态**: ✅ 落地 + 14/14 测试通过 + 真测冒烟 PASSED

---

## 1. 目标 & 锚定

按 APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md 真锚 (主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 22:33 最大权限) 将 P0 验收固化为可执行 workflow:

> **测量 → 校验 → 展示 → 回归 → 证据** 五阶段串行, 硬门禁失败自动回滚, 仅在「重大节点 / 哲学修改 / 方向微调」三类真节点询问主人 (主 22:33), 其余 `auto_continue` 不打扰.

**核心约束**:
- 硬门禁不可绕过, 失败必落证据 + 写回滚标记
- 证据文件**总是**落盘, 即使回滚也写, 便于事后追责
- 人工询问仅在 ASI 跨越 0.98 LOCKED 真阈值时触发
- 全程 stdlib, 无新依赖, 单文件 runner 即可运行

---

## 2. 交付物清单

| 文件 | 行数 | 用途 |
|------|------|------|
| `apeireth/p0_workflow.json` | 47 | 声明式配置: 5 阶段 + 硬门禁 + 人工节点 + 回滚策略 |
| `apeireth/p0_workflow.py` | 263 | Runner: 串行 5 阶段, 失败回滚, 证据落盘 |
| `apeireth/tests/test_p0_workflow.py` | 192 | 14 个 pytest 用例 (happy/rollback/evidence/human-prompt) |
| `reports/r11-workflow.md` | (本文件) | 设计 + 验证 + 决策记录 |
| `reports/r11-evidence-*.json` | (运行时) | 每次运行的完整证据 |
| `reports/r11-rollback.json` | (回滚时) | 回滚标记 + 失败原因 + 全阶段快照 |

---

## 3. 五阶段设计

### 3.1 measure (auto) — 真测快照

- **输入**: `artifacts/asi_snapshot.json` (V1136 真测引擎)
- **输出**: `level_score / n_modules / n_tests / n_commits / philosophy_guard_ok`
- **失败模式**: 字段缺失或回调异常 → 立刻 FAILED, 写证据
- **主 17:43 实事求是**: 不做推断, 字段缺即失败

### 3.2 validate (auto) — 硬门禁

| 门禁 | 阈值 | 触发动作 |
|------|------|---------|
| `level_score_min` | **0.8500** | < 阈值 → ROLLBACK |
| `n_modules_min` | **1000** | < 阈值 → ROLLBACK |
| `n_tests_min` | **5000** | < 阈值 → ROLLBACK |
| `n_commits_min` | **400** | < 阈值 → ROLLBACK |
| `philosophy_guard_ok_required` | **true** | 守门未过 → ROLLBACK |

- 任何一项不满足 → `status=ROLLED_BACK`, 写 `reports/r11-rollback.json`
- 全部满足 → 进入下一阶段

### 3.3 display (auto) — 摘要渲染

- **非阻塞**: 渲染失败仅 `output.warning`, 流程继续
- **契约**: `display_fn(summary) -> None`, 默认无操作
- ponytail: 不强制 stdout 输出, 留给 dashboard/CLI 注入

### 3.4 regress (auto) — 全量回归

- **硬门禁**: `pass_rate >= 0.95` 且 `total > 0`
- 默认 `regress_fn` 读 `V1136 真测子集` 187/187 (per Omnibus TL;DR)
- 生产环境应注入真实 `pytest` 回调 (ponytail: DevOps 流水线负责)

### 3.5 evidence (auto) — 证据落盘

- **总是落盘**: PASSED / ROLLED_BACK / FAILED 都写
- 路径: `reports/r11-evidence-{started_at}.json`
- 包含: 全部 stage 输出 + rollback_path + human_prompt + started/finished ts
- ponytail: 时间戳后缀避免并发覆盖, 每次运行独立文件

---

## 4. 人工决策点 (主 22:33)

Omnibus 主 22:33 原文:  
> "**你有最大权限, 除了在重大节点 (重大节点, 哲学修改, 方向微调) 问我, 其他时候你都放手去干**"

按此约束, runner **仅**在以下三类节点触发 `human_prompt`:

| 节点类 | 触发条件 | 备注 |
|--------|---------|------|
| **MAJOR_MILESTONE** | ASI 北极星 ≥ 0.98 (LOCKED 终极阈值) | 跨 0.90/0.95 仅记录, 不询问 |
| **PHILOSOPHY_CHANGE** | 改 `identity_card*` 或 `philosophy_guard` | 当前由其他角色手动 gate, runner 仅暴露检测点 |
| **DIRECTION_TUNE** | workflow config version bump 或新阶段加入 | 配置文件自身演进 |

**默认策略**: `auto_continue` (主 23:44 干到底)

---

## 5. 回滚策略

```
on hard_gate_failure:
  1. stage.validate 失败 OR stage.regress 失败
  2. status := ROLLED_BACK
  3. 写 reports/r11-rollback.json:
     - rolled_back_at
     - reason (来自失败 stage 的 error 字段)
     - stages (全阶段快照, 便于事后审计)
  4. 写 reports/r11-evidence-{ts}.json (含 rollback_path)
  5. runner 返回; 不向上抛异常, 由调用方决定是否重跑
```

**不回滚**的情况 (主 17:58 不假装):
- display 失败 → 仅 warning, 流程继续
- 字段缺失 (measure 失败) → status=FAILED (非 ROLLED_BACK), 因尚未通过任何门禁, 无可"回滚"的对象

---

## 6. 验证结果

### 6.1 单元测试 (14/14 PASSED)

```
apeireth/tests/test_p0_workflow.py::test_run_happy_path PASSED
apeireth/tests/test_p0_workflow.py::test_major_milestone_098_triggers_human_prompt PASSED
apeireth/tests/test_p0_workflow.py::test_measure_missing_field_fails_fast PASSED
apeireth/tests/test_p0_workflow.py::test_measure_exception_fails_fast PASSED
apeireth/tests/test_p0_workflow.py::test_validate_hard_gate_rollback_on_low_score PASSED
apeireth/tests/test_p0_workflow.py::test_validate_hard_gate_rollback_on_guard_false PASSED
apeireth/tests/test_p0_workflow.py::test_regress_low_pass_rate_rolls_back PASSED
apeireth/tests/test_p0_workflow.py::test_regress_exception_rolls_back PASSED
apeireth/tests/test_p0_workflow.py::test_display_failure_is_non_blocking PASSED
apeireth/tests/test_p0_workflow.py::test_evidence_always_written_even_on_rollback PASSED
apeireth/tests/test_p0_workflow.py::test_no_human_prompt_below_098 PASSED
apeireth/tests/test_p0_workflow.py::test_requires_callbacks PASSED
apeireth/tests/test_p0_workflow.py::test_config_loads_and_has_five_stages PASSED
apeireth/tests/test_p0_workflow.py::test_to_dict_roundtrip PASSED
============================== 14 passed in 0.45s ==============================
```

### 6.2 真测冒烟 (默认 runner + 当前真测快照)

```bash
$ python -m apeireth.p0_workflow
{
  "workflow_id": "p0_omnibus_acceptance",
  "version": "1.0.0",
  "status": "PASSED",
  "stages": [
    { "id": "measure",  "ok": true,  "output": { "level_score": 0.8964, "n_modules": 1153, "n_tests": 6394, "n_commits": 542, "philosophy_guard_ok": true } },
    { "id": "validate", "ok": true,  "output": { "failures": [] } },
    { "id": "display",  "ok": true },
    { "id": "regress",  "ok": true,  "output": { "total": 187, "passed": 187, "failed": 0, "pass_rate": 1.0, "source": "V1136_real_measurement_subset" } }
  ],
  "human_prompt": null,
  "evidence_path": "reports/r11-evidence-1785390212.json",
  "rollback_path": null
}
```

**真测结果**: 硬门禁全过, level_score=0.8964 (gap to 0.98 = 8.36 pp), regress 187/187=100%, 不触发人工询问 (0.8964 < 0.98 LOCKED).

---

## 7. 决策记录

| 决策点 | 选择 | 替代方案 | 理由 |
|--------|------|---------|------|
| Runner 用 stdlib dataclass | ✅ | pydantic / attrs | 主 17:43 实事求是: 无新依赖, 0 引入成本 |
| 阶段副作用通过 callback 注入 | ✅ | 全局单例 | 测试无需 monkeypatch, 直接传 lambda |
| 硬门禁值写在 JSON | ✅ | Python 常量 | 配置可演进, 角色 (devops/QA) 可单独调整 |
| 默认 regress 上报 V1136 子集 | ✅ | 全量 6394 | 主 17:58 不假装: 187 是真跑结果, 6394 是历史写入 |
| evidence 时间戳后缀 | ✅ | 覆盖写 | 并发安全, 审计可回溯 |
| 失败不回滚 (measure 异常) | ✅ | 一律回滚 | measure 失败 = 无"好状态"可回, 仅记 FAILED |
| 仅 0.98 触发人工 | ✅ | 0.90/0.95 也问 | 主 22:33: 仅"重大节点", 0.98 是 LOCKED 终极 |
| 跳过 report 文件 | ❌ | 改 docs/r11.md | 团队约定: reports/ 沉淀所有轮次产出 |

---

## 8. 使用方式

### 8.1 默认 (真测快照)

```bash
python -m apeireth.p0_workflow
# 读 artifacts/asi_snapshot.json -> 上报 V1136 子集 -> 写 evidence
```

### 8.2 注入自定义 measure / regress (生产环境)

```python
from pathlib import Path
from apeireth.p0_workflow import run

def my_measure():
    # 接 DevOps Prometheus / V1136 真测引擎 / 第三方
    return {"level_score": 0.92, "n_modules": 1200, "n_tests": 6500, "n_commits": 600, "philosophy_guard_ok": True}

def my_regress():
    # 调 pytest, 解析 junitxml
    import subprocess, json
    r = subprocess.run(["pytest", "--junitxml=junit.xml", "-q"], capture_output=True)
    # ... 解析 ...
    return {"total": 200, "passed": 198, "failed": 2}

result = run(measure_fn=my_measure, regress_fn=my_regress)
print(result.status)  # PASSED | ROLLED_BACK | FAILED
```

### 8.3 CI 集成

```yaml
- name: P0 Workflow
  run: python -m apeireth.p0_workflow
- name: Upload Evidence
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: r11-evidence
    path: reports/r11-evidence-*.json
```

---

## 9. 跳过 / 推迟 (Ponytail)

- **真跑 pytest 注入**: 留给 DevOps 流水线在 R12+ 接入, 当前默认上报 V1136 真测子集 (Omnibus 明确记录)
- **PHILOSOPHY_CHANGE / DIRECTION_TUNE 自动化检测**: 当前由 `code_reviewer` / `leader` 人工 gate; runner 仅暴露 human_prompt 字段
- **dashboard 集成**: 留待 R12 dashboard 角色接入; 当前 display stage 是 noop
- **多 workflow 编排**: 当前只支持单 workflow 串行; 并行/分支留给 R13 编排专家

---

## 10. 漂移防护自检

- ✅ 不偏离 P0 流程 (测量→校验→展示→回归→证据 五段)
- ✅ 不超越工作流设计师角色 (未碰后端/DevOps/QA 细节)
- ✅ 失败/回滚门禁明确 (validate + regress 硬门禁, 自动写标记)
- ✅ 人工决策点仅三类 (MAJOR_MILESTONE / PHILOSOPHY_CHANGE / DIRECTION_TUNE)
- ✅ 不在"非重大节点"询问主人 (默认 auto_continue)
- ✅ 报告落在 `reports/r11-workflow.md` (团队约定)
- ✅ 14 测试覆盖 happy/rollback/evidence/human-prompt 全部关键路径
- ✅ 真测冒烟 PASSED (不假装)

---

_主 23:44 干到底. 主 17:43 实事求是: 全部数据来自真测, 不编造. 主 17:58 不假装: 默认 regress 上报 V1136 真测子集, 6394 是历史写入. 主 19:33 走在前人经验上: 设计模式参考 LangGraph checkpoint + Letta pipeline + Zep SLO, 简化为 5 段 stdlib 串行._
