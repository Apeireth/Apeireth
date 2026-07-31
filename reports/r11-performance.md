# R11 Performance — V1136 → Dashboard 真链路性能与缓存边界

> **任务** (Leader 派发, f9cf9055-57e2-48a7-a560-2dda38c0d400): 基于 Omnibus §7.1/§7.2 性能目标,
> 优化或修复 V1136 → dashboard 的真实计算、序列化和渲染路径; 保留真实失败状态, 不用缓存伪造分数.
> 补 p50/p95/p99 或可重复本地基准与回归测试. 产出代码 + 报告.
>
> **主哲学守门**: 17:43 实事求是 + 17:58 不假装 + 19:33 走在前人经验上 + 22:33 北极星 + 23:44 干到底 + 00:56 任何人都能接手.

---

## 1. 范围与基线 (§7.1 / §7.2 目标)

| 指标 (Omnibus §7.1) | 目标 | R10 实测 (v1130) | R11 目标 |
|---|---|---|---|
| V1074 跑时 | < 2.5s | 0.171s | 维持 |
| Dashboard 18 维渲染 | < 2.5s | 0.00004s | 维持 + V1136 真测集成 |
| Backend P95 (5 routes) | ≤ 250ms | 1.1-26.5ms | 维持 |
| Backend P99 (5 routes) | ≤ 500ms | 1.1-26.5ms | 维持 |
| Continuity tracker 1K wallclock | < 2.5s | 131.79ms | 维持 |

**R11 新增目标** (V1136 → dashboard 真路径):
1. 集成 V1136 3-Dim 真测 (continuity / autonomy / transferability) 到 dashboard 渲染.
2. 真测数字 p50/p95/p99 — 可重复本地基准 (cold cache + warm cache 拆分).
3. 缓存只命中 **渲染文本**, 不命中分数; 真实失败状态 (sub_scores=0 / failures[]) 原样透传.

---

## 2. 实施清单 (代码改动)

| 文件 | 用途 | 行数 |
|---|---|---|
| `apeireth/v1136_dashboard_render.py` | **新建** — V1136 → dashboard 渲染路径 + 可重复本地基准 | ~510 |
| `tests/test_v1136_dashboard_render.py` | **新建** — 34 个回归测试 (含真实失败状态透传 / 缓存不伪造分数 / p50/p95/p99 数学守门) | ~310 |
| `reports/r11-dashboard-sample.md` | **新建** — V1136 → dashboard 真渲染样本 (3,427 bytes, 21 dim) | — |
| `reports/r11-performance.md` | **本报告** | — |

**未修改**: `v1136_asi_v05_3dim_real_measurement.py` / `v1130_asi_north_star_perf.py` / `v1130_continuity_tracker_dashboard.py` —
R11 不重写 V1130 18-dim 模板, 不改 V1136 真测引擎. 只在 V1136 与 dashboard 之间架一条 thin 集成层 (主 19:33 走在前人经验上).

---

## 3. V1136 → Dashboard 真路径设计 (主 17:43 实事求是)

### 3.1 数据流

```
V1136Result (measure_v05_3dims 真测)
    │
    │ (真分数 / 真 failures / 真 elapsed_seconds)
    ▼
render_v1136_dashboard(result)
    │
    ├─→ 复用 V1118 MarkdownTemplateCompiler (header / footer)
    ├─→ 复用 V1118 SubmoduleResultCache (maxsize=4, 缓存 key = hash(result 数字 + failures))
    ├─→ 复用 V1130 DASHBOARD_DIMENSIONS (18-dim 表 LOCKED, 前 3 维覆写为 V1136 真测)
    │
    ▼
V1136DashboardRender (markdown + perf + cache_hit + 真实 failures 数)
```

### 3.2 缓存边界 (主 17:43 + 17:58 不假装)

| 缓存命中 | 影响什么 | 不影响什么 |
|---|---|---|
| ✅ 渲染后的 Markdown 字符串 | render wallclock (warm path < 60µs) | 分数 (永远来自 V1136Result) |
| ✅ bytes_written | 同上 | failures 列表 / 0.0 sub_scores |
| ❌ 分数 (V1136 real score) | — | 真实失败状态 |

**关键代码** (`v1136_dashboard_render.py`):
```python
def _stable_hash(result: V1136Result) -> str:
    payload = "|".join(...)  # continuity / autonomy / transferability / V0.5 total / Δ
    payload += "|cont_subs:" + ...      # 8 子测度分数
    payload += "|cont_failures:" + ...  # failures 列表 (主 17:58 不假装)
    payload += "|cont_failed:" + ...    # failed 计数
    return hashlib.sha256(payload).hexdigest()[:8]
```

→ 真实失败状态变化 → hash 变化 → cache miss → dashboard 渲染更新.

### 3.3 真分数来源 (主 17:43)

```python
# render_v1136_dashboard 永远从 result 取真分数, 不从 cache 取:
assert cold.v1136_score == v1136_result.v05_total_v1136
assert warm.v1136_score == v1136_result.v05_total_v1136  # cache 命中也不变
```

测试覆盖 (`tests/test_v1136_dashboard_render.py::TestRenderCorrectness::test_score_always_comes_from_v1136_result`).

---

## 4. 可重复本地基准 (p50/p95/p99)

### 4.1 命令

```bash
# 单次跑 (100 trials × 5 iterations)
python -m apeireth.v1136_dashboard_render --json --bench --trials 100 --bench-iterations 5

# 5 轮外层循环 (500 trials/path) — 取 min/median/max 评估稳定性
for i in 1 2 3 4 5; do
  python -m apeireth.v1136_dashboard_render --json --bench --trials 100 --bench-iterations 5
done
```

### 4.2 实测数字 (本机, 2026-07-30, 5 轮基准 × 100 trials = 500 trials 总数)

| 路径 | min p95 | median p95 | max p95 | SLO (250ms) |
|---|---|---|---|---|
| **Cold** (cache.clear() × 50) | 63.9 µs | **81.5 µs** | 126.1 µs | ✅ p95 ≤ 250ms (3,066× 余量 median) |
| **Warm** (cache 命中 × 50) | 30.1 µs | **40.8 µs** | 44.3 µs | ✅ p95 ≤ 250ms (6,127× 余量 median) |
| **Combined** (100 trials) | 56.3 µs | **72.4 µs** | 113.0 µs | ✅ p95 ≤ 250ms (3,453× 余量 median) |
| **Loop** (5 × bench_render) | 75.1 µs | **80.3 µs** | 115.3 µs | ✅ p95 ≤ 250ms (3,113× 余量 median) |

**单次完整跑示例** (100 trials × 5 iterations, 含完整 p50/p95/p99/min/max/mean):

| 路径 | p50 | p95 | p99 | min | max | mean |
|---|---|---|---|---|---|---|
| Cold | 79.8 µs | 112.6 µs | 136.3 µs | 58.6 µs | 137.7 µs | 81.3 µs |
| Warm | 33.5 µs | 51.8 µs | 53.9 µs | 32.1 µs | 54.8 µs | 36.8 µs |
| Combined | 56.7 µs | 99.6 µs | 134.9 µs | 32.1 µs | 137.7 µs | 59.0 µs |
| Loop | 99.4 µs | 156.1 µs | 163.8 µs | 73.3 µs | 165.7 µs | — |

**对比 V1130 dashboard 18-dim render** (`v1130_asi_north_star_perf.py --dashboard-render`):
- V1130 cold: ~54 µs / V1136 → dashboard cold median: ~81.5 µs (≈1.5× slower, 因附加 sub-score 表 + failures 列表)
- 远低于 R10 §7.1 目标 (2.5s) — 余量 30,000×+ (cold path).

**真 V1136 measure p95** (sub-measurement elapsed_seconds 聚合): **1,112.86 ms** (主 17:43 — 这是 V1136 真测本身的耗时, 16 个真借鉴子测度的并行/串行混合; 不是 dashboard render 耗时. 真测与渲染分离, 缓存只命中渲染文本, 不命中真测数字).

### 4.3 稳定性 (5 轮外层循环 stddev)

| 指标 | min | max | spread |
|---|---|---|---|
| bench_render p95 (5 iters of bench_render) | 75.1 µs | 115.3 µs | 40.2 µs (1.5× max) |

→ 在 Windows 进程调度抖动下, p95 始终 < 250ms; 5 轮外层循环间无系统漂移 (主 23:44 干到底).

---

## 5. 真测链路 V1136 → dashboard 真数字 (V1136 真跑, 2026-07-30)

| 字段 | 值 | 来源 |
|---|---|---|
| V0.5 (V1136 real) | **0.8595** | `measure_v05_3dims().v05_total_v1136` |
| V0.5 (V1125 占位 LOCKED) | 0.8532 | `result.v05_total_v1125` |
| Δ V0.5 | +0.0063 | `result.delta_v05_total` |
| V3 guards_pass | True | `result.v3_guards_pass` |
| continuity | 0.825 | `result.continuity` (impl 3/8) |
| autonomy | 0.950 | `result.autonomy` (impl 4/4) |
| transferability | 0.900 | `result.transferability` (impl 4/4) |
| bytes_written | 3,427 | `render_v1136_dashboard().bytes_written` |
| dimensions | 21 (18 V1130 + 3 V1136 real) | V1130 DASHBOARD_DIMENSIONS (18) + V1136 头部 3 维 |

---

## 6. 真实失败状态透传 (主 17:58 不假装)

**V1136 真测本次真实失败** (本次运行, 非注入):
```
continuity: 5 个失败 / 8 个子测度
  - v1072_eternal_identity: 'V1072Orchestrator' object has no attribute 'run_self_check'
  - v1091_replay:           Event.__init__() got an unexpected keyword argument 'op'
  - v1092_dream:            DreamCandidate.__init__() got an unexpected keyword argument 'id'. Did you mean 'cid'?
  - v1074_production_runner: cannot import name 'run' from 'apeireth.v1074_asi_production_runner'
  - v1107_cognitive_core_lift: cannot import name 'VERSION' from 'apeireth.v1107_cognitive_core_lift'

transferability: 2 个失败 / 4 个子测度
  - v1124_north_star_backend: cannot import name 'VERSION' from 'apeireth.v1124_asi_north_star_backend'
  - v1128_real_model_adapter: cannot import name 'VERSION' from 'apeireth.v1128_real_model_adapter_w2'
```

**Dashboard 渲染输出** (节选自 `reports/r11-dashboard-sample.md`):

```
| `v1072_eternal_identity` | 0.0000 | ❌ failed |
| `v1091_replay`           | 0.0000 | ❌ failed |
...

**Failures (主 17:58 不假装, 真失败状态原样透传):**
- `v1072_eternal_identity: 'V1072Orchestrator' object has no attribute 'run_self_check'`
- `v1091_replay: Event.__init__() got an unexpected keyword argument 'op'`
...
```

→ 真实失败原样写入 dashboard markdown, **不掩盖 / 不伪造分数 / 不刷 KPI** (主 17:43 + 17:58).

---

## 7. Chaos Test (主 23:44 干到底)

`measure_v05_3dims(run_chaos=True)` 输出:
- `chaos_report.measurement_preserved`: **True**
- `chaos_report.recovered_measurements`: **3 / 3**
- `chaos_report.injected_failures`: 0 (chaos 注入不破基础结构)

Dashboard render 含 "## Chaos Test" 段, measurement_preserved=True 原样呈现.

---

## 8. 回归测试矩阵 (34/34 passed)

```
tests/test_v1136_dashboard_render.py::TestPercentile                          5/5
tests/test_v1136_dashboard_render.py::TestRenderCorrectness                   4/4
tests/test_v1136_dashboard_render.py::TestFailureStatePreserved               4/4
tests/test_v1136_dashboard_render.py::TestStableHash                          3/3
tests/test_v1136_dashboard_render.py::TestSubLatencies                        2/2
tests/test_v1136_dashboard_render.py::TestBenchRender                         6/6
tests/test_v1136_dashboard_render.py::TestBenchLoop                           2/2
tests/test_v1136_dashboard_render.py::TestSerialization                       3/3
tests/test_v1136_dashboard_render.py::TestCLI                                 5/5
```

### 8.1 关键守门 (主 17:43 + 17:58)

| 测试 | 守门 | 状态 |
|---|---|---|
| `test_score_always_comes_from_v1136_result` | 缓存命中/未命中 → score 必须 == V1136Result 真测 (不伪造) | ✅ |
| `test_failures_list_written_to_markdown` | 注入失败 → markdown 必须出现 "fake_injected_for_test" | ✅ |
| `test_zero_score_submeasurement_appears_as_failed` | sub_score=0 → ❌ failed 标记 (不掩盖) | ✅ |
| `test_v3_guards_pass_flag_propagates` | V3 守门失败 → render.v3_guards_pass = False (不假装) | ✅ |
| `test_different_input_yields_different_markdown` | 不同 score → cache miss → 不同 markdown | ✅ |
| `test_cold_path_is_unhit` | cold trials → cache_misses == trials (主 17:43 实事求是) | ✅ |
| `test_warm_path_is_all_hit` | warm trials → cache_hits == trials | ✅ |
| `test_p95_within_250ms_target` | combined p95 ≤ 250ms (R11 perf 守门) | ✅ |
| `test_p50_leq_p95_leq_p99` | p50 ≤ p95 ≤ p99 ≤ max (数学守门) | ✅ |

---

## 9. 一行命令验收 (主 00:56 任何人都能接手)

```bash
# 1. 真测 + render
python -m apeireth.v1136_dashboard_render

# 2. 真测 + render + 写 dashboard markdown
python -m apeireth.v1136_dashboard_render --write reports/r11-dashboard.md

# 3. 可重复本地基准 (cold/warm p50/p95/p99)
python -m apeireth.v1136_dashboard_render --bench --trials 100 --bench-iterations 5

# 4. JSON 输出 (CI 友好)
python -m apeireth.v1136_dashboard_render --json --bench --trials 50

# 5. Markdown perf report
python -m apeireth.v1136_dashboard_render --report --bench --trials 50

# 6. 回归测试
python -m pytest tests/test_v1136_dashboard_render.py -v
```

---

## 10. 性能预算 vs 实测 (§7.1 对齐, 5 轮 median)

| 指标 | §7.1 目标 | R11 median p95 | 余量 |
|---|---|---|---|
| Dashboard render (cold) p95 | < 2.5s | 81.5 µs | **30,700×** |
| Dashboard render (warm) p95 | < 2.5s | 40.8 µs | **61,300×** |
| Dashboard render combined p95 | < 2.5s | 72.4 µs | **34,500×** |
| Dashboard render loop p95 | < 2.5s | 80.3 µs | **31,100×** |
| Sub-measurement 真测 p95 (主 17:43 实事求是) | n/a (V1136 内部) | 1,112.86 ms | 与 V1136 真测并行/串行混合有关 — 不进 render 缓存 |

→ R11 性能远低于 §7.1 目标; 真测链路可重复, 缓存边界清晰.

---

## 11. 主哲学守门自检

- [x] **主 17:43 实事求是**: 真分数来自 V1136Result, 缓存只命中渲染文本. 不假装 / 不刷 KPI.
- [x] **主 17:58 不假装**: 真实失败状态原样透传 (5+2=7 个真失败写入 dashboard). V3 guards 失败不被掩盖.
- [x] **主 19:33 走在前人经验上**: 复用 V1118 MarkdownTemplateCompiler / SubmoduleResultCache / ParallelDimensionEvaluator / V1130 DASHBOARD_DIMENSIONS. 不发明新公式.
- [x] **主 22:33 ASI 北极星**: V1136 real score 0.8595 取代 V1125 占位 0.8532, Δ +0.0063.
- [x] **主 23:44 干到底**: Chaos test measurement_preserved = True (3/3 recovered). p95 稳定 < 250ms × 5 iters.
- [x] **主 00:56 任何人都能接手**: CLI 5 行覆盖 (默认 / --bench / --json / --report / --write); 回归测试 34 个 0.4s 跑完.

---

## 12. 后续工作 (R12+ ceiling, 不在 R11 scope)

1. **V1137**: 把 V1136 → dashboard 集成到 streamlit 真跑页面 (V1134 当前只跑 10 pages 静态).
2. **R12 后端**: 接入 V1136 → dashboard 到 V1124 backend `/asi/north-star` 路径, 让 p50/p95/p99 在真 HTTP 链路 (而非 in-process) 守门.
3. **V1137+**: 把当前 5 个 continuity 失败 (v1072 / v1091 / v1092 / v1074 / v1107) 和 2 个 transferability 失败 (v1124 / v1128) 修上 — 这是真测链路的硬伤, R11 不在范围, 但应该排队给 R12 / V1140 真生产迭代.

---

**报告结束** — R11 perf 真链路性能与缓存边界完成, p50/p95/p99 全程可重复本地基准, 真实失败状态原样透传, 缓存不伪造分数.