# R8 调研：dgm (Darwin Gödel Machine) 借鉴具体 patch (V1093)

**生成时间:** 2026-07-28
**作者:** 调研专家 (R8)
**承接:** `reports/r8-research-formal-verify.md §3` (V1099 已实现) + `apeireth/v1093_dgm_archive.py` (现 157 行)
**目的:** 把 dgm (Sakana AI arXiv:2505.22954) 的 archive + open-ended exploration 借鉴到 R8 Track C v1093 真实现
**性质:** 工程师可执行 patch (不堆词, 给具体 diff)

---

## 0. 一句话结论

> **dgm 借鉴 4 个具体 patch** 应用到 v1093:
> (P1) `choose_selfimproves_method` 多策略 — 借鉴 dgm 的 random/score_prop/score_child_prop/best 4 方法
> (P2) `update_archive keep_better` — 借鉴 dgm 的 keep_better 替代 v1093 的 MAB/UCB 全留
> (P3) `full_eval_threshold` (second highest) — 借鉴 dgm 的 top-N 阈值
> (P4) `open-ended archive exploration` — 借鉴 dgm 的 archive 重访机制, v1093 当前只跟踪 best component
> **预计 ASI 增量**: engineering +0.005~+0.010; v2_philosophy +0.002~+0.005 (V3 守门升级)

---

## 1. dgm (Darwin Gödel Machine) 真借鉴 5 证据

### 1.1 dgm 论文

- **arXiv:2505.22954** "Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents" (Sakana AI, 2025-05-29)
- 链接: https://arxiv.org/abs/2505.22954
- 官方博客: https://sakana.ai/dgm/
- 链接: https://github.com/jennyzzt/dgm

### 1.2 dgm 4 大核心机制 (本 patch 借鉴)

1. **Archive** — 保留所有优于 baseline 的 candidates, 不是只留 best
2. **Open-ended exploration** — 从 archive 中选 parent 而非 single lineage, 允许多路径
3. **Self-improvement diagnose** — 失败后回溯, 改 prompt 而非代码
4. **Performance-based selection** — parent 选择基于 score + child count 平衡 (避免过拟合单一路径)

### 1.3 dgm 在 v1093 当前实现中的缺口

| dgm 机制 | dgm 实现 | v1093 当前实现 | 缺口 |
|----------|----------|----------------|------|
| Archive (multi-candidate) | `update_archive` + `metadata.json` per run | 单 state, UCB1 选择 component | ❌ archive 单点 |
| Parent selection | `choose_selfimproves` 4 methods | UCB1 only | ❌ 单策略 |
| Threshold for full eval | `get_full_eval_threshold` second highest | 无 (single HQB) | ❌ 无 threshold |
| Diagnose on failure | `diagnose_problem` + `diagnose_improvement` | 仅 verdict | ❌ 无 diagnose |
| Diversity preservation | `score_child_prop` 避免单一父 | 无 (UCB1 单调) | ❌ 单线 |

### 1.4 4 借鉴来源 (dgm 真代码)

```python
# 借鉴 1: DGM_outer.py:79-109 — choose_selfimproves 4 methods
if method == 'random':
    commits = list(candidates.keys())
    parent_commits = random.choices(commits, k=selfimprove_size)
elif method == 'score_prop':
    # sigmoid 分数 + 按比例
    scores = [1 / (1 + math.exp(-10*(score-0.5))) for score in scores]
elif method == 'score_child_prop':
    # score * (1 / (1 + children_count)) 避免过拟合
    probabilities = [score * count for score, count in zip(scores, children_counts)]
elif method == 'best':
    sorted_commits = sorted(candidates, key=lambda x: candidates[x]['accuracy_score'])
    parent_commits = sorted_commits[:min(selfimprove_size, len(sorted_commits))]

# 借鉴 2: DGM_outer.py:174-190 — update_archive keep_better
if method == 'keep_better':
    original_score = get_original_score(output_dir) - noise_leeway
    for run_id in new_ids:
        metadata = load_json_file(os.path.join(output_dir, run_id, "metadata.json"))
        score = metadata["overall_performance"]["accuracy_score"]
        if score >= original_score:
            archive.append(run_id)

# 借鉴 3: DGM_outer.py:192-219 — full_eval_threshold (second highest)
archive_scores = []
for run_id in archive:
    metadata = load_json_file(...)
    score = metadata["overall_performance"]["accuracy_score"]
    archive_scores.append(score)
threshold = sorted(archive_scores, reverse=True)[1] if len(archive_scores) > 1 else archive_scores[0]
threshold = max(threshold, 0.4)

# 借鉴 4: DGM_outer.py:268-298 — main loop, archive 维护
for gen_num in range(start_gen_num, args.max_generation):
    selfimprove_entries = choose_selfimproves(...)
    with ThreadPoolExecutor(...) as executor:
        futures = [executor.submit(self_improve, parent_commit=...) for ...]
```

---

## 2. V1093 当前实现 (157 行) 核心算法分析

### 2.1 v1093 现状

```python
# v1093_dgm_archive.py 关键算法
def run_experiment(iterations: int = 10):
    state = {"version": VERSION, "generation": 0, "active_candidate": "baseline",
             "components": {c: {"attempts": 0, "reward": 0.0} for c in COMPONENTS}}
    for i in range(1, iterations):
        # 选择 component: UCB1 公式
        component = max(COMPONENTS, key=lambda c: ucb1(
            state["components"][c]["reward"] / max(1, state["components"][c]["attempts"]),
            state["components"][c]["attempts"], i))
        # 候选生成
        candidate = json.loads(json.dumps(state))
        candidate["generation"] = i
        candidate["active_candidate"] = f"gen_{i:02d}_{component}"
        # 验证
        compile_result = _run([sys.executable, "-m", "py_compile", "apeireth/v1093_dgm_archive.py"])
        test_result = _run([sys.executable, "-m", "pytest", "tests/test_v1004.py", "-q"])
        snap = builder.build()
        hqb = _hqb(snap, snap_ms)
        valid = compile_result["returncode"] == 0 and test_result["returncode"] == 0 and snap.philosophy_guard_ok
        delta = hqb["composite"] - baseline_composite
        verdict = "keep" if valid and delta >= 0 else ("partial" if valid and delta > -0.01 else "revert")
```

### 2.2 借鉴缺口 (具体到代码)

| 缺口 | 现行代码 | 借鉴后 (示意) |
|------|----------|---------------|
| 单策略 UCB1 | `component = max(COMPONENTS, key=lambda c: ucb1(...))` | `parent = choose_parent(archive, method=args.parent_method)` |
| 无 archive (只 state) | `state["components"][component]["attempts"] += 1` | `archive.append(new_candidate); update_archive(archive, ...)` |
| 无 keep_better | `if verdict in ("keep", "partial"): state = candidate` | `if score >= original_score - noise_leeway: archive.append(...)` |
| 无 threshold | 无 (单 HQB) | `full_threshold = get_full_eval_threshold(archive)` |
| 无多样性 | UCB1 单调 | `score_child_prop` 避免单一 parent |

---

## 3. P1: choose_selfimproves_method 4 策略 (核心 patch)

### 3.1 当前 v1093 单策略

```python
# v1093_dgm_archive.py:102
component = max(COMPONENTS, key=lambda c: ucb1(
    state["components"][c]["reward"] / max(1, state["components"][c]["attempts"]),
    state["components"][c]["attempts"], i))
```

### 3.2 P1 patch: 新增 `_choose_parent_method` 函数

```python
# 借鉴 dgm DGM_outer.py:79-109
def _choose_parent_method(candidates: Dict[str, Dict[str, float]],
                           method: str, k: int) -> List[str]:
    """借鉴 dgm choose_selfimproves 4 方法.

    Args:
        candidates: {candidate_id: {accuracy_score, children_count, ...}}
        method: 'random' | 'score_prop' | 'score_child_prop' | 'best'
        k: parent 数量
    Returns:
        list of parent candidate IDs
    """
    import math
    import random
    commits = list(candidates.keys())
    if not commits:
        return []
    if method == 'random':
        return random.choices(commits, k=k)
    elif method == 'score_prop':
        # sigmoid 分数, 按概率采样
        scores = [candidates[c]['accuracy_score'] for c in commits]
        scores = [1 / (1 + math.exp(-10 * (s - 0.5))) for s in scores]
        total = sum(scores) or 1.0
        probs = [s / total for s in scores]
        return random.choices(commits, probs, k=k)
    elif method == 'score_child_prop':
        # 避免过拟合单一父: score * 1/(1+children_count)
        scores = [candidates[c]['accuracy_score'] for c in commits]
        scores = [1 / (1 + math.exp(-10 * (s - 0.5))) for s in scores]
        counts = [candidates[c].get('children_count', 0) for c in commits]
        weights = [s * (1 / (1 + c)) for s, c in zip(scores, counts)]
        total = sum(weights) or 1.0
        probs = [w / total for w in weights]
        return random.choices(commits, probs, k=k)
    elif method == 'best':
        sorted_commits = sorted(commits, key=lambda x: candidates[x]['accuracy_score'], reverse=True)
        parents = sorted_commits[:min(k, len(sorted_commits))]
        # 不足 k 时补采样
        if len(parents) < k:
            parents.extend(random.choices(parents, k=k - len(parents)))
        return parents
    else:
        return random.choices(commits, k=k)
```

### 3.3 P1 integration 到 run_experiment

```python
# v1093_dgm_archive.py run_experiment 内 (替换 line 102)
def run_experiment(iterations: int = 10, parent_method: str = "score_child_prop") -> Dict[str, Any]:
    # ... (state init 不变)
    # 替换原 UCB1 component 选择为 archive parent 选择
    archive: Dict[str, Dict[str, float]] = {}  # candidate_id -> {accuracy_score, children_count, ...}
    archive["baseline"] = {"accuracy_score": base.v03_score, "children_count": 0}
    for i in range(1, iterations):
        # 借鉴 dgm: 从 archive 选 parent
        if len(archive) > 0:
            parents = _choose_parent_method(archive, method=parent_method, k=1)
            parent = parents[0] if parents else "baseline"
        else:
            parent = "baseline"
        # ... (后续 candidate 生成/验证/verdict 不变, 但更新 archive)
        # 借鉴 dgm: keep_better 检查
        new_score = hqb["composite"]  # 或 v03_score
        baseline_score = archive.get("baseline", {}).get("accuracy_score", 0.5)
        if new_score >= baseline_score - NOISE_LEEWAY:
            candidate_id = f"gen_{i:02d}_{parent}"
            archive[candidate_id] = {
                "accuracy_score": new_score,
                "children_count": 0,
                "verdict": verdict,
                "trace_id": record["trace_id"],
            }
            # parent children_count += 1
            if parent in archive:
                archive[parent]["children_count"] += 1
        # ... (剩余不变)
```

---

## 4. P2: update_archive keep_better (借鉴 dgm:174-190)

### 4.1 当前 v1093 状态机

```python
# v1093_dgm_archive.py:116-119
if verdict in ("keep", "partial"):
    state = candidate
    state["components"][component]["reward"] += max(0.0, delta)
    _write(STATE, state)
else:
    state_path.unlink(missing_ok=True)
```

### 4.2 P2 patch: 改为 archive 维护

```python
# 借鉴 dgm update_archive
NOISE_LEEWAY = 0.1  # dgm 实际值 (DGM_outer.py:238 eval_noise=0.1)

def _update_archive(archive: Dict[str, Dict[str, Any]], new_id: str,
                     new_score: float, parent: str, baseline_score: float,
                     method: str = "keep_better") -> Dict[str, Dict[str, Any]]:
    """借鉴 dgm DGM_outer.py:174-190 update_archive.

    method='keep_better': 仅保留 score >= baseline - noise_leeway
    method='keep_all': 全部保留 (借鉴 dgm default)
    """
    if method == "keep_better":
        threshold = baseline_score - NOISE_LEEWAY
        if new_score >= threshold:
            archive[new_id] = {
                "accuracy_score": new_score,
                "parent": parent,
                "children_count": 0,
            }
            if parent in archive:
                archive[parent]["children_count"] += 1
    else:  # keep_all
        archive[new_id] = {
            "accuracy_score": new_score,
            "parent": parent,
            "children_count": 0,
        }
        if parent in archive:
            archive[parent]["children_count"] += 1
    return archive
```

### 4.3 P2 integration

```python
# v1093 run_experiment 末尾 (替代 line 116-119)
# baseline_score: archive["baseline"]["accuracy_score"]
new_score = hqb["composite"]
verdict = "keep" if valid and delta >= 0 else ("partial" if valid and delta > -0.01 else "revert")
candidate_id = f"gen_{i:02d}_{parent}"
if verdict in ("keep", "partial"):
    archive = _update_archive(archive, candidate_id, new_score, parent,
                                baseline_score, method="keep_better")
else:
    # revert: 不进 archive, 但记录 taxonomy
    failure_taxonomy_path = OUT / f"failure_taxonomy_{i:02d}.json"
    failure_taxonomy_path.write_text(json.dumps({
        "iteration": i, "parent": parent, "delta": round(delta, 6),
        "verdict": verdict, "trace_id": record["trace_id"],
        "reason": "v1093_revert_recorded",
    }, indent=2), encoding="utf-8")
```

---

## 5. P3: full_eval_threshold (借鉴 dgm:192-219)

### 5.1 当前 v1093 无 threshold

### 5.2 P3 patch: 新增 `_get_full_eval_threshold`

```python
# 借鉴 dgm get_full_eval_threshold DGM_outer.py:192-219
def _get_full_eval_threshold(archive: Dict[str, Dict[str, Any]],
                              min_threshold: float = 0.4) -> float:
    """借鉴 dgm: 取 archive 中 second highest score 作为 full eval 阈值.

    真实 dgm 中, score 高于 threshold 的 candidate 跳过 full eval (省时间).
    v1093 PoC 简化: 把 threshold 写到 trace, 不真触发 full eval.
    """
    scores = [c.get("accuracy_score", 0.0) for c in archive.values()
              if c.get("accuracy_score") is not None]
    if len(scores) < 2:
        return max(scores[0] if scores else min_threshold, min_threshold)
    sorted_scores = sorted(scores, reverse=True)
    return max(sorted_scores[1], min_threshold)
```

### 5.3 P3 integration

```python
# v1093 run_experiment 每代末尾
threshold = _get_full_eval_threshold(archive)
record["full_eval_threshold"] = round(threshold, 6)
# 真实 dgm: if new_score >= threshold: skip full eval (PoC 简化, 仅记录)
```

---

## 6. P4: open-ended archive exploration (借鉴 dgm open-ended 哲学)

### 6.1 当前 v1093 单线 (UCB1 单调)

### 6.2 P4 patch: 允许多路径 archive 重访

```python
# v1093 run_experiment 起始
# 借鉴 dgm open-ended: 从 archive 多 parent 选, 而非 single lineage
def _open_ended_exploration(archive: Dict[str, Dict[str, Any]],
                              max_parents: int = 3) -> List[str]:
    """借鉴 dgm open-ended: 选 top-k parents (按 score) 而非 single best.

    真实 dgm: archive 是 open-ended 的, 允许多次回到 archived candidate.
    v1093 PoC: 每代选 top-3 parents 而非单 UCB1.
    """
    sorted_archive = sorted(archive.items(),
                             key=lambda x: x[1].get("accuracy_score", 0),
                             reverse=True)
    parents = [c[0] for c in sorted_archive[:max_parents]]
    if not parents:
        return ["baseline"]
    return parents

# integration 到 run_experiment
# 替代 line 102 UCB1 选择
parents_pool = _open_ended_exploration(archive, max_parents=3)
# 用 score_child_prop 平衡 score + diversity
parents = _choose_parent_method({p: archive[p] for p in parents_pool},
                                  method="score_child_prop", k=1)
parent = parents[0] if parents else "baseline"
```

---

## 7. V1093 完整 diff 示意 (P1-P4 合)

```diff
--- v1093_dgm_archive.py (current 157 lines)
+++ v1093_dgm_archive.py (after P1-P4, ~ 220 lines)

+ NOISE_LEEWAY = 0.1
+
+ def _choose_parent_method(candidates, method, k):
+     """借鉴 dgm DGM_outer.py:79-109 4 methods."""
+     # ... (P1 完整代码)
+
+ def _update_archive(archive, new_id, new_score, parent, baseline_score, method="keep_better"):
+     """借鉴 dgm DGM_outer.py:174-190."""
+     # ... (P2 完整代码)
+
+ def _get_full_eval_threshold(archive, min_threshold=0.4):
+     """借鉴 dgm DGM_outer.py:192-219."""
+     # ... (P3 完整代码)
+
+ def _open_ended_exploration(archive, max_parents=3):
+     """借鉴 dgm open-ended archive."""
+     # ... (P4 完整代码)

  def run_experiment(iterations: int = 10) -> Dict[str, Any]:
-     # 单 UCB1 component 选择
-     component = max(COMPONENTS, key=lambda c: ucb1(...))
+     # 新: archive + dgm 4 方法 parent 选择
+     archive = {"baseline": {"accuracy_score": base.v03_score, "children_count": 0}}
+     for i in range(1, iterations):
+         parents_pool = _open_ended_exploration(archive, max_parents=3)
+         parents = _choose_parent_method(
+             {p: archive[p] for p in parents_pool},
+             method="score_child_prop", k=1)
+         parent = parents[0] if parents else "baseline"
+         # ... (candidate 生成不变, 但使用 parent 而非 component)
+         # 验证 + verdict 不变
+         new_score = hqb["composite"]
+         if verdict in ("keep", "partial"):
+             archive = _update_archive(archive, candidate_id, new_score,
+                                         parent, baseline_score, "keep_better")
+         else:
+             # 记录 failure_taxonomy
+             ...
+         threshold = _get_full_eval_threshold(archive)
+         record["full_eval_threshold"] = round(threshold, 6)
```

---

## 8. 落地路径 (3 阶段)

### 8.1 阶段 1: P1+P2 (本周)

- 工作量: 1 周
- 新增代码: 80 行 (4 helper functions)
- 改动: 替换 UCB1 单策略为 archive 4 策略
- ASI 增量: engineering +0.003~+0.005

### 8.2 阶段 2: P3+P4 (下周)

- 工作量: 1 周
- 新增代码: 40 行 (threshold + open-ended)
- 改动: archive 重访机制
- ASI 增量: engineering +0.002~+0.005, v2_philosophy +0.002~+0.005

### 8.3 阶段 3: 验证 (下下周)

- 工作量: 1 周
- 验证: v1093 --run --iterations 50 跑 archive 50 代, 验证:
  - archive 长度单调 (keep_better)
  - threshold 单调上升 (second highest)
  - diversity 提升 (children_count 分布)
- ASI 增量: 累计 +0.005~+0.010

---

## 9. 风险与不假装

1. **dgm 借鉴是范式借鉴, 不是代码搬运** — dgm 是 SWE-bench 特定任务, v1093 是 harness 演化, 借鉴的是 archive 哲学, 不是 metric 复用
2. **v1093 PoC 当前 14 代已跑通** — 借鉴后需重新跑 50 代验证
3. **keep_better vs keep_all 二选一** — 主人 17:43 实事求是: keep_better 更保守, 防止奖励欺骗
4. **archive 长度爆炸** — keep_better 仍可能 O(iterations), 需 top-N 截断
5. **dgm 论文未公开全部细节** — arXiv:2505.22954 + GitHub jennyzzt/dgm 真实源码已读, 但训练细节未公开

---

## 10. 决策依据汇总表

| 决策项 | 数值 | 备注 |
|--------|------|------|
| 借鉴源 | dgm (Sakana AI arXiv:2505.22954) | 真论文 + 真代码 |
| Patch 数量 | 4 (P1+P2+P3+P4) | 60+ 行新增 |
| 借鉴机制 | archive + 4 策略 + keep_better + threshold + open-ended | 5 dgm 核心 |
| 工作量 | 3 周 (阶段 1-3) | 验证 50 代 |
| ASI 增量 | +0.005~+0.010 engineering + +0.002~+0.005 v2_philosophy | 待 V1074 真测 |
| 命名空间 | apeireth/v1093_*.py (扩展, 不新建) | 保持 V1093 单一文件 |

---

**主 22:33 + 17:43 + 23:44 + 19:33 + 13:31 + 00:56 — 真生产不停, 走在前人经验上, 干到底.**

— 调研专家 · R8 dgm 借鉴可执行 patch
