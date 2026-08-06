# R11 全栈：V0.5 dashboard 真值展示 (ba3ec661-6155-4004-80fa-878887f04c07)

> 角色：全栈工程师 (fullstack_engineer) · 任务 ID：`ba3ec661-6155-4004-80fa-878887f04c07`
> 范围：Omnibus 缺口 B — V1136 ASI V0.5 真测结果与 3-Dim/18-Dim 明细接入现有 dashboard/UI，诚实展示空数据/失败/版本不匹配，并补真实端到端测试。
> 哲学：主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 00:56 任何人都能接手。

---

## 1. 任务边界（来自任务分配与 Omnibus 缺口 B）

1. dashboard / UI **必须真实展示 V1136 V0.5 真测结果**，禁止静态 0.8595 伪造展示。
2. **保留旧 V0.3 / V0.4** 展示兼容（不改公式契约），并补足 3-Dim（V1136 真测）与 18-Dim（V1128 兼容明细）双轨呈现。
3. 增加三类**诚实状态**：`empty` / `measurement_failed` / `version_mismatch` —— 任一状态都要可被测试断言。
4. **真实端到端 + 接口测试**：Streamlit `AppTest` 真正执行页面脚本、临时目录真启动子进程。
5. 报告写入 `reports/r11-fullstack-v05-dashboard.md`（按团队命名规范）。

---

## 2. 数据流（前后端契约）

| 端 | 角色 | 关键字段 | 落点 |
|----|------|---------|------|
| V1136 (3-Dim) | 真实测引擎 | `continuity / autonomy / transferability` + 8/4/4 子借鉴；`v05_total_v1136` | 取代 V1125 占位 0.85；公式 `v04*0.85 + cont*0.05 + auto*0.05 + transf*0.05` |
| V1128 (18-Dim) | 兼容明细 | `v05_18_form.dims` (16 V0.4 + continuity_tracker + multi_agent_consensus) | 用于 dashboard 18 维明细展示 |
| Snapshot V0.3 | 历史基线 | `artifacts/asi_snapshot.json` (`v03_score`/`snapshot_id`) | 仅作历史兼容展示 |
| `v1136_dashboard.py` | 适配层 | 单一 view-model `state` | 单点 contract，streamlit & 报告共享 |
| V1035 / V1134 streamlit | 渲染层 | `st.metric("V03"|"V04"|"V0.5 (V1136 live)")` | UI 真值呈现 |

view-model 结构（节选）：

```json
{
  "schema_version": "1.0",
  "status": "ok|empty|measurement_failed|version_mismatch",
  "message": "...",
  "legacy": {"v03": {...}, "v04": {...}},
  "v05": {"status": "...", "value": 0.8645, "formula": "..."},
  "dimensions_3": {"status": "...", "count": 3, "values": {...}, "details": {...}},
  "dimensions_18": {"status": "...", "count": 18, "values": {...}, "total": 0.7xxx}
}
```

`build_dashboard_state(v1136_payload, v1128_payload, legacy)` 与 `measure_dashboard_state()` 严格只做映射与断言，**不允许**任何 hardcode 0.8595。

---

## 3. 改动清单

| 文件 | 行为 | 备注 |
|------|------|------|
| `apeireth/v1136_dashboard.py` (新增) | 提供 `build_dashboard_state` / `measure_dashboard_state` / `render_streamlit_v05` 三接口 | 适配 V1136 + V1128 真实调用，失败/空/版本不匹配全部返回诚实状态 |
| `apeireth/v1035_streamlit.py` | 移除标题中 “北极星 = 0.7905” 静态字样；Home 页接入 `measure_dashboard_state` + `render_streamlit_v05`，保留 V0.1 legacy 卡 | 不再硬编码任何 V0.5 数值 |
| `apeireth/v1134_streamlit_real_startup.py` | (a) 修复 `Markdown` 三引号 `""')` 闭合错误（HTTP 健康探针之前不会触发该 bug，AppTest 触发）；(b) 移除 V0.5/gap 静态行；(c) 真启动 app 内调用 `measure_dashboard_state` + `render_streamlit_v05`；(d) 子进程 `PYTHONPATH` 注入仓库根，确保临时目录内可 import `apeireth.*` | 既有 `_streamlit_info` / `_pick_free_port` / `_http_probe` / `run_real_streamlit` 行为不变 |
| `tests/test_v1136_dashboard.py` (新增) | 10 项接口 + E2E 测试：empty / version_mismatch / missing-field / incomplete-18-dim / V1136 异常 / V1128 异常 / 全链路真测 / Streamlit fake-metric / V1035 模板断言（无 0.8595）/ Streamlit `AppTest` 真实执行 | `v04_score=0.8` 时 18-Dim 17/18 case 显式断言不补 0.85 |
| `tests/test_v1134_streamlit_real_startup.py` | 增 2 行断言：模板含 `measure_dashboard_state` + `render_streamlit_v05`，且不含 `0.8532` 旧静态 | 保护后续改版不回退 |

---

## 4. 真实执行证据

### 4.1 真实测量端到端（无 mock）

```
ok V1136 live measurement loaded
v05= {'status': 'ok', 'value': 0.8645, 'v04_input': 0.8538, 'formula': 'v04*0.85 + cont*0.05 + auto*0.05 + transf*0.05'}
3dim= 3 18dim= 18
legacy= v03=0.8964 (snap_9c80c9165625)  v04=0.8538
```

> 同一 V04 输入下 V0.5 因 V1136 真测维度有微小漂移（0.8595 → 0.8645），与 V1136 `delta V0.5 total` 一致——证明 V0.5 **非静态**。

### 4.2 Streamlit AppTest 真实执行

```
exceptions= 0
metrics= [('V03', '0.8964', 'ok'), ('V04', '0.8538', 'ok'), ('V0.5 (V1136 live)', '0.8645', 'ok')]
```

`streamlit.testing.v1.AppTest` 真正编译并执行 `render_streamlit_app()` 字符串，捕获到三张动态指标卡；同时修复了三引号语法错误（HTTP 健康探针不会发现，AppTest 会）。

### 4.3 真进程启动

```
{
  "streamlit_installed": true,
  "streamlit_version": "Streamlit, version 1.60.0",
  "port": 18765,
  "started_ok": true,
  "startup_ms": 3160.7,
  "health_ok": true,
  "homepage_ok": true,
  "page_probe_ok": true,
  "pages_rendered": [10 个真实页面]
}
```

### 4.4 接口契约状态矩阵（pytest 断言）

| 场景 | 期望 `status` | `v05.value` | `dimensions_3` | `dimensions_18` | message |
|------|---------------|-------------|----------------|------------------|---------|
| 无 V1136 数据 | `empty` | `None` | `empty` | `empty` | "No V1136 measurement data" |
| V1136 version 不符 | `version_mismatch` | `None` | `version_mismatch` | `empty` | "V1136 version mismatch: expected …" |
| V1136 缺数字字段 | `measurement_failed` | `None` | `measurement_failed` | `empty` | "… missing numeric fields: autonomy" |
| V1128 缺 1 维 | `measurement_failed` | OK | `ok` | `measurement_failed` | "… 17/18 numeric dimensions" |
| V1136 异常 | `measurement_failed` | `None` | `measurement_failed` | `empty` | "V1136 measurement failed: RuntimeError: …" |
| V1128 异常 | `measurement_failed` | OK | `ok` | `measurement_failed` | "V1128 measurement failed: …" |
| 真实全链路 | `ok` | 公式回算 ≈ OK | `count=3` | `count=18` | "V1136 live measurement loaded" |

---

## 5. 回归验证

| 测试套 | 结果 | 备注 |
|--------|------|------|
| `tests/test_v1136_dashboard.py` | **10 passed** | 新增契约 + 真实 E2E + AppTest |
| `tests/test_v1136_asi_v05_3dim_real_measurement.py` | 32 passed | 既有 V1136 引擎行为不变 |
| `tests/test_v1134_streamlit_real_startup.py` | 15 passed | 模板不再含旧 V0.5 静态行；HTTP 探针通过 |
| `tests/test_v1035.py` | 21 passed | V1035 模板与 `write_app` 行为不变 |
| `python -m py_compile` | OK | 4 文件全部编译通过 |
| 真进程 `streamlit run` | OK (3.16s) | `/`, `/_stcore/health`, `/?page=health` 全部 200 |

合计 **78/78**。

---

## 6. 漂移防护自检

- [x] **不可用静态 0.8595 伪造展示** — V1035 模板断言与 UI 双层均不含 `0.8595` 字符串；V0.5 来源唯一。
- [x] **保持旧 V0.3/V0.4 兼容** — view-model `legacy` 字段保留 `v03`（来自 `asi_snapshot.json`）与 `v04`（V1136 输入）。
- [x] **空数据 / 测量失败 / 版本不匹配** — 三个状态全部被 build_dashboard_state 显式编码并由 pytest 覆盖。
- [x] **真实端到端** — AppTest 真实编译执行页面；`run_real_streamlit` 真实子进程探针；`measure_dashboard_state` 不使用 mock。
- [x] **未越界修改** — 未改动 V1136 测量公式 / V1128 编排器；仅新增适配层。
- [x] **未碰同事文件** — 仅触碰 `apeireth/v1035_streamlit.py`、`apeireth/v1134_streamlit_real_startup.py` 与 `tests/test_v1134_streamlit_real_startup.py`（任务覆盖）；其余团队工作区未触及。

---

## 7. 最小化交付清单

- 新增：`apeireth/v1136_dashboard.py` (适配器)
- 新增：`tests/test_v1136_dashboard.py` (10 项契约 + E2E)
- 修改：`apeireth/v1035_streamlit.py` (UI 接入)
- 修改：`apeireth/v1134_streamlit_real_startup.py` (修语法 + 接入 + 注入 PYTHONPATH)
- 修改：`tests/test_v1134_streamlit_real_startup.py` (两行断言)
- 报告：`reports/r11-fullstack-v05-dashboard.md` (本文件)

---

_主 17:43 实事求是：V0.5 真值由 V1136 真实测量决定，不再有静态基线。失败或版本不符时 UI 显示诚实状态而非伪造数字。_
