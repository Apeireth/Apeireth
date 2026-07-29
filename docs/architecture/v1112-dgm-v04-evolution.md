# V1112 DGM Archive v0.4 — 真演化 + 候选隔离 — 真架构文档

> **模块**: `apeireth/v1112_dgm_v04.py` (879 LOC)
> **测试**: `tests/test_v1112_dgm_v04.py` (580 LOC)
> **作者**: technical_writer · R9-TW-001 · W4 末
> **守门**: 主 19:33 走在前人经验上 (Sakana AI arXiv:2505.22954) + 主 13:31 大胆激进 + 主 20:55 红皇后
> **关键 commit**: `da1a2483` (R9-AO-001 真演化 50 轮)

---

## 1. 设计意图 (主 20:55 红皇后 never stop evolving)

**DGM (Darwin Gödel Machine)** = 真演化闭环：
```
archive (历史候选池) → candidate (新生成) → evaluate (HQB 4 维) → retain/discard
```

V0.4 vs V0.3 (v1093_dgm_archive.py 305 LOC) 真增量：

| # | 增量 | v0.3 | v0.4 |
|---|---|---|---|
| P5 | 真演化闭环 | 仅 metric 收集 | archive → candidate → evaluate → retain/discard |
| P6 | 3 方法对照 | 单 parent-child | parent-child + sexual + asexual |
| P7 | Identity 锚定 | 无 | candidate 必须 identity_id 锚定 |
| P8 | V1072 桥接 | 仅 JSON state | state + identity 元数据往返 |
| P9 | 50 轮真演化 | 30 轮 | 50 轮 (+20, 记录每轮 lift) |
| P10 | keep_state 父本引用 | 无 | child 必须引用真实 parent_id |

---

## 2. 真借鉴 (主 19:33 走在前人经验上)

| 来源 | 借鉴点 | 用途 |
|---|---|---|
| **Sakana AI Darwin Gödel Machine** (arXiv:2505.22954, 2025) | archive + UCB1 bandit | 候选池 + 选择策略 |
| **v1095 IdentityStore** | 中央 AI 永恒身份 + 多 persona 槽位 | Identity 锚定源 |
| **v1072 ASI Central AI Eternal Identity** | identity_id 锚定 + schema 桥接 | V1072 桥接 |
| **v1093 DGM Archive v0.3** | 5 选择方法 + keep_better + open-ended 30% | 演化基线 |
| **遗传算法** | parent-child / sexual / asexual 3 方法 | P6 借鉴 |

---

## 3. 真组件清单 (源行号)

`grep -n "^class\|^def " apeireth/v1112_dgm_v04.py`：

| # | 组件 | 源行号 | 用途 |
|---|---|---:|---|
| 1 | `ucb1` | 86 | UCB1 bandit 选择 (主借鉴 Sakana) |
| 2 | `_json_hash` | 93 | 候选 hash (state diff) |
| 3 | `IdentityAnchor` | 132 | identity_id 锚定 (P7) |
| 4 | `build_default_anchor` | 190 | 默认锚定构造 |
| 5 | `try_attach_identity_store` | 199 | V1095 串联 |
| 6 | `reproduce_parent_child` | 237 | 单亲变异 |
| 7 | `reproduce_sexual` | 270 | 双亲交叉重组 (50% 字段 swap) |
| 8 | `reproduce_asexual` | 299 | 随机漂变 (30% 字段重置) |
| 9 | `reproduce` | 328 | 3 方法统一入口 |
| 10 | `_hqb_for` | 360 | HQB 4 维计算 |
| 11 | `_evaluate_candidate` | 376 | 候选评估 |
| 12 | `_should_retain` | 384 | retain 阈值 ≥ baseline + 0.015 |
| 13 | `V04EvolutionRun` | 418 | 演化 run dataclass |
| 14 | `run_experiment` | 460 | 50 轮真演化主入口 |

辅助：`report()` (L722) 输出 Markdown，`main()` (L794) CLI。

---

## 4. 真演化 50 轮真示例 (主 00:56)

```bash
python -m apeireth.v1112_dgm_v04 \
    --iterations 50 \
    --anchor v1095 \
    --method sexual \
    --output reports/v1112_w4_50r.json
```

```python
# 代码层真跑
from apeireth.v1112_dgm_v04 import (
    V04EvolutionRun, run_experiment, report, IdentityAnchor,
)

# 1. 锚定 identity_id (P7 守门)
anchor = IdentityAnchor(identity_id="chu-ling", v1095_db="data/v1095.db")

# 2. 真演化 50 轮 (P9)
result = run_experiment(
    iterations=50,
    anchor=anchor,
    method="sexual",  # parent-child | sexual | asexual
    retain_threshold=0.015,
)
print(f"retained={result['n_retained']}/{result['n_total']}")
print(f"lift_avg={result['lift_avg']:.4f}")
print(f"identity_anchor_failures={result['identity_anchor_failures']}")
assert result["identity_anchor_failures"] == 0, "P7 锚定守门破!"

# 3. Markdown 报告
md = report(result["archive"])
Path("reports/v1112_w4_50r.md").write_text(md)
```

---

## 5. 候选隔离 (主 17:58 不假装)

P7 锚定守门：
- candidate 必须 `identity_id` 锚定才能入 archive
- 锚定失败 = 强制 reject (V3 守门不假装)
- `try_attach_identity_store(store)` (L199) 真尝试 V1095 串联
- V1095 不可用时 = `Optional[IdentityAnchor]` = None，候选全部 reject

候选隔离 3 阶段：
1. **生成期** — `reproduce_*` 任意方法，child 必带 `parent_id` (P10 守门)
2. **评估期** — `_evaluate_candidate` + `_hqb_for` 4 维
3. **保留期** — `_should_retain` 阈值 ≥ `baseline + 0.015` (vs v0.3 是 `baseline + 0.0`)

---

## 6. R9 W4 末真测状态

| 指标 | 真测 | 阈值 | 状态 |
|---|---:|---:|---|
| 演化轮数 | 50 | ≥ 50 | ✅ |
| retained 数 | TBD | ≥ 30 (60%) | 主 17:43 |
| lift_avg | TBD | ≥ 0.02 | 主 13:31 |
| identity_anchor_failures | 0 | = 0 | ✅ P7 守门 |
| parent_id missing | 0 | = 0 | ✅ P10 守门 |
| n_asi_pretend_total | 0 | = 0 | ✅ V3 守门 |

> ponytail: 当前 `retain_threshold=0.015` 是硬编码。当 R10 引入跨小模型验证时，需新增 `AdaptiveThreshold` 类（基于 archive_scores 自适应）。