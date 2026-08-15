# R14 Rust 重写路线图详细文档 (主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 22:33 ASI 北极星)

> **范围声明** (主 17:43 实事求是 + 主 17:58 不假装): 本文档是 R14 Rust 重写团队的**提前准备路线图** (与 R12 收尾 + R13 MVP 落地并行准备). 依据 T14 R12 重大变更预备文档 §5 + T13 报告 7 任务清单 + R13 MVP 落地状态. **不重写任何工程 / 不修改主手册 / 不砍 1100 空壳**, 仅记录 R14 详细路线图 + 6 触发条件 + 6 阶段 (Phase 0-5) 详细计划 + 26 周时间线 + 预算与依赖 + R14 团队接手建议. 主人哲学硬约束 (V0.5 / V1136 / 哲学守门 / 不刷 KPI / 不假装 ASI) 全保护到 R14 重设计, 不在本预备文档内妥协.

---

## 0. 元信息 (主 17:43 实事求是)

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/r14-rust-rewrite-roadmap.md` |
| **生成时间 (UTC)** | 2026-07-30 13:30 |
| **触发原因** | 用户最新指示 (2026-07-30 13:15): "重大变更文档和我们的工程手册你都创建一个文件夹整理一下". Apeireth-rust/ 已建, 但 docs/ 是空目录. R14 Rust 重写是用户明确终极路径 (T14 §5), 需要详细路线图文档放在 docs/ 里提前准备. |
| **工作目录** | `.openclaw\workspace\promethean` |
| **master HEAD** | `945fbd9a feat(r13-mvp-phase12): R13 MVP Phase 1.2 提取层 + 合并 + 遗忘` |
| **依据** | T14 R12 重大变更预备文档 §5 (26 周路线图) + T13 报告 7 任务清单 (T6-F-1/2 + T9 接续 + T6-G + T6-H + team_land_integration + team_finalize) + R13 MVP 落地状态 (Phase 0+1.1 ✅, Phase 1.2 🔄) |
| **R12 收尾预算** | ≤ 1500 行业务改动 (T13 报告设定) |
| **R14 预算** | ≤ 5000 行业务改动 (本设定) |
| **不修改承诺** | ❌ 不修改主手册 (line 1-6546 字节级一致) / ❌ 不修改已 commit 的 12 个 commit / ❌ 不重写 V0.5 / V1136 / 哲学守门 / ❌ 不砍 1100 空壳 / ❌ 不写 ASI 公式 |

---

## 1. R14 触发条件 (6 条)

> R14 启动必须**全部满足**以下 6 条, 由 R13 MVP 收尾团队验证后启动. 主 17:43 实事求是, 任何条件不满足都不启动 R14.

| # | 触发条件 | 验证方法 | 当前状态 |
|---|---------|---------|----------|
| 1 | **R13 MVP Phase 0-3 全部完成** | T9 R13 MVP 报告 + team_finalize | Phase 0 ✅ + Phase 1.1 ✅ + Phase 1.2 🔄 + Phase 1.3/1.4 ⏸ + Phase 2/3 ⏸ |
| 2 | **主人实测连续 7 天每天 1 次** | 主人自报 + mvp/usage.log | 0 (Phase 3 验证) |
| 3 | **主观满意度 > 7/10** | 主人评分卡 | N/A (Phase 3 验证) |
| 4 | **IdentityCard 跨 session 持续稳定** | 24h / 7d 测试报告 (Phase 1.3 演化层验证) | 部分 (Phase 1.1 SQLite 已落) |
| 5 | **工具集成完成** (web_search / file_ops / git_ops / code_exec) | Phase 3 验证 | 引入未集成 |
| 6 | **工程代码回退无副作用** | git tag r13-final + R11 末 refresh 累积验证 | 0 (Phase 3 验证) |

**满足条件**: 6/6 全部 🔄 待 R13 MVP Phase 0-3 全部完成后验证.

---

## 2. R14 Phase 0: 接口规范 (Week 1-4, 4 周)

### 2.1 目标 (主 23:44 干到底)

从 Python MVP (`mvp/`) + Apeireth 现有真生产模块 (~50) 提取 trait / API 形式化规范. R14 团队开工前必备输入.

### 2.2 任务清单

| # | 任务 | 来源 | 预计交付 |
|---|------|------|----------|
| 1 | 提取 `mvp/memory/store.py` Episode / Note / Session trait 规范 | R13 Phase 1.1 已落 SQLite | Rust trait + 序列化约束 |
| 2 | 提取 `mvp/memory/retrieve.py` BM25 / Salience decay trait 规范 | R13 Phase 1.2 提取层 🔄 | Rust 检索引擎 trait |
| 3 | 提取 `mvp/identity/card.py` IdentityCard trait 规范 | R13 Phase 1.1 已落 | Rust 身份卡 trait |
| 4 | 提取 `mvp/cli.py` CLI command interface 规范 | R13 Phase 0 已落 | CLI parser + 命令协议 |
| 5 | 提取 `apeireth/v1077-1141` 现有真生产模块的 Python → Rust 转换清单 | R12 评估 (T2 working changes audit) | V1130 / V1136 / V1138 / V1141 / V1132 / V1121 / V1077 等真生产模块 |
| 6 | 写 `rust-traits-spec.md` (~500 行) | 综合 1-5 | 形式化 trait 规范 |
| 7 | 写 `api-mapping-py-to-rust.md` (~300 行) | 综合 1-5 | Python → Rust 转换映射 |

### 2.3 交付物 (4 周结束)

- `Apeireth-rust/docs/rust-traits-spec.md` (~500 行)
- `Apeireth-rust/docs/api-mapping-py-to-rust.md` (~300 行)
- `Apeireth-rust/docs/phase-0-completion.md` (Phase 0 收尾报告)

### 2.4 风险 (主 17:43)

- Python MVP 接口不稳定 → Rust trait 反复改
- 缓解: Phase 0 第 2 周接口冻结, 后续 Phase 严格不改接口

---

## 3. R14 Phase 1: Rust 关键路径实现 (Week 5-8, 4 周)

### 3.1 目标

把 V1130 wallclock + V32 GravityMemory + V1122 ContinuityTracker 用 Rust 实现, 性能目标 V1130 wallclock 2.5s (vs R11 末 8.7s / R12 5.43s).

### 3.2 任务清单

| # | 任务 | 来源 Python | Rust 实现 | 集成测试 |
|---|------|------------|------------|----------|
| 1 | V1130 SQLite ContinuitySnapshotStore | `apeireth/v1130_continuity_tracker_dashboard.py` | `crates/memory/v1130/` (rusqlite + tokio) | Python mvp/ 接口兼容 |
| 2 | V32 GravityMemory | R13 Phase 1.2 `mvp/memory/retrieve.py` | `crates/memory/retrieve/` (BM25 + Salience decay 双 tau) | BM25 检索精度 95%+ |
| 3 | V1122 ContinuityTracker | R13 Phase 1.3 `mvp/memory/tracker.py` | `crates/memory/tracker/` (Episode/Note 时序追踪) | 时序查询精度 95%+ |

### 3.3 交付物 (4 周结束)

- `crates/memory/` Rust crate (~1000 行: v1130 + retrieve + tracker)
- 集成测试 (`crates/memory/tests/`): Python mvp/ 接口兼容
- `Apeireth-rust/docs/phase-1-completion.md` (Phase 1 收尾报告 + 性能基准)
- 性能目标: V1130 wallclock 5.43s → 2.5s (-54%)

### 3.4 风险

- Rust 学习曲线 (新工程师)
- PyO3 兼容性 (PyO3 0.22+ vs Python 3.13.14)
- 缓解: 主 19:33 借鉴 DeltaMemory-Rust (Lin et al. 2024) / Lumio-Research/hermes-agent-rs (110K Rust) / VCP Rust substrate / 9-crate workspace

---

## 4. R14 Phase 2: V0.5/V1136/哲学守门 Rust 重设计 (Week 9-14, 6 周)

### 4.1 目标 (主 17:43 + 主 17:58)

保留主人哲学硬约束, **不重写核心规则**, 但用 Rust trait + 状态机重设计 + 类型系统强制.

### 4.2 任务清单

| # | 任务 | 保留核心 | Rust 重设计 |
|---|------|----------|--------------|
| 1 | **V0.5 公式** | 公式结构 (v04×0.85 + continuity×0.05 + autonomy×0.05 + transferability×0.05) | Rust 实现 + **加"自设指标"标注** (主 17:58 不假装 ASI) |
| 2 | **V1136 真测引擎** | 3-dim 加权 (continuity + autonomy + transferability) | Rust 重设计 5 continuity + 2 transferability 子测度**真实可执行测试** (砍 0.05 KPI 装饰) |
| 3 | **V3 哲学契约 9 键 LOCKED** | 9 键机制 + LOCKED 状态 | Rust trait + 状态机实现, 保留 9 键 LOCKED 核心 |
| 4 | **5 项不假装规则** | R11-R1 ~ R11-R5 全 PASS | Rust 类型系统强制 (字符串匹配 → 类型 trait) |
| 5 | **V1121 fake-KPI detector** | keys_present=9 + fake_kpi_attempts + n_threats | Rust regex + trait 实现 (性能提升 10-100x) |
| 6 | **V1138 五项不假装 gate** | 5/5 + 9/9 LOCKED + R11-SEC-002 4/4 | Rust 集成 |

### 4.3 保留核心 (主 17:58 + 主 23:44)

- ✅ **保留**: V3 哲学契约 9 键 LOCKED 机制
- ✅ **保留**: 5 项不假装规则 (不假装 Phenomenal consciousness / ASI / docker / 调参捷径 / 刷 KPI)
- ✅ **保留**: V1121 fake-KPI detector 机制
- ❌ **不重写**: 规则本身 (只是用 Rust 实现 + 类型系统强制)
- ❌ **不砍**: 1100 空壳 (Phase 5 才清理)

### 4.4 交付物 (6 周结束)

- `crates/asi/` Rust crate (~1500 行: V0.5 + V1136 + V1138 + V1141)
- `crates/philosophy/` Rust crate (~800 行: V3 9 键 + 5 项不假装 + R11-SEC-002)
- `Apeireth-rust/docs/phase-2-completion.md` (Phase 2 收尾报告)
- 主 17:58 验证: V0.5 仍是自设指标, 不假装 ASI

### 4.5 风险

- Rust trait 设计过度抽象 (主 17:58 不刷 KPI)
- V0.5 公式变成"实际 ASI 指标" (主 17:58 不假装)
- 缓解: 公式加 `#[doc(hidden)]` + `// SELF_SET_METRIC, NOT ASI` 注释 + 文档化"自设指标"透明标注

---

## 5. R14 Phase 3: PyO3 桥暴露 (Week 15-16, 2 周)

### 5.1 目标

Rust API 通过 PyO3 暴露给 Python, 让 R13 MVP 通过 PyO3 调用 Rust 实现 (性能提升 5-10x).

### 5.2 任务清单

| # | 任务 | 来源 | 预计交付 |
|---|------|------|----------|
| 1 | PyO3 crate 暴露 `crates/memory` 给 Python | crates/memory/ | `crates/pyo3-bridge/src/memory.rs` |
| 2 | PyO3 crate 暴露 `crates/asi` 给 Python | crates/asi/ | `crates/pyo3-bridge/src/asi.rs` |
| 3 | PyO3 crate 暴露 `crates/philosophy` 给 Python | crates/philosophy/ | `crates/pyo3-bridge/src/philosophy.rs` |
| 4 | Python `mvp/` 切换到 Rust PyO3 实现 | mvp/ 子项目 | `mvp/memory/store.py` → `mvp/memory/store.py` (PyO3 import) |
| 5 | 测试 Python `mvp/` 接口兼容 | pytest | mvp/tests/ 11/11 PASSED + Rust 测试 50/50 PASSED |

### 5.3 交付物 (2 周结束)

- `crates/pyo3-bridge/` Rust crate (~500 行)
- `mvp/` 切换文档 (`Apeireth-rust/docs/mvp-pyo3-migration.md`)
- `Apeireth-rust/docs/phase-3-completion.md` (Phase 3 收尾报告)
- 性能基准: Python mvp/ → Rust PyO3 性能提升 5-10x

### 5.4 风险

- PyO3 版本兼容性 (PyO3 0.22+ + Python 3.13.14)
- Python 类型系统 ↔ Rust 类型系统不匹配
- 缓解: Phase 3 第 1 周先做 PoC 验证 1 个 trait, 第 2 周全量切换

---

## 6. R14 Phase 4: 主人实测对比 (Week 17-20, 4 周)

### 6.1 目标

Python MVP (基线) vs Rust MVP (PyO3 桥暴露) 体验对比 + 性能对比.

### 6.2 任务清单

| # | 任务 | 时长 | 预计交付 |
|---|------|------|----------|
| 1 | 主人连续 7 天实测 Python MVP (基线) | Week 17-18 | baseline-7d-report.md |
| 2 | 主人连续 7 天实测 Rust MVP (PyO3 桥) | Week 19-20 | rust-7d-report.md |
| 3 | 主观满意度对比 | 实时 | comparison-satisfaction.md |
| 4 | 性能对比 (V1130 wallclock + 启动时间 + 响应延迟) | 实时 | comparison-performance.md |
| 5 | 主人决策: Rust MVP 是否落地 | Week 20 末 | go-no-go-decision.md |

### 6.3 交付物 (4 周结束)

- `Apeireth-rust/docs/python-vs-rust-comparison.md` (~300 行)
- 主人实测报告 (连续 14 天)
- Go/No-Go 决策文档

### 6.4 风险

- 主人实测发现 Rust MVP 不如 Python (性能/体验)
- 缓解: Phase 4 第 3 周留 buffer, Go/No-Go 决策允许推迟 R14

---

## 7. R14 Phase 5: 1100 空壳模块清理 + 6000 行瘦身 (Week 21-22, 2 周)

### 7.1 目标

砍掉 1100 空壳 + 6000 行手册瘦身到 500-1000 行 (主 00:56 任何人都能接手 = 1 小时懂一切).

### 7.2 任务清单

| # | 任务 | 来源 | 预计交付 |
|---|------|------|----------|
| 1 | 砍掉 1100 空壳 | R12 评估 (~96% 空壳) | `archive/v1-r11-empty-modules/` (~1100 文件) |
| 2 | 手册瘦身 | APEIRETH-COMPLETE-OMNIBUS 6546 行 | `README.md` (100 行) + `ARCHITECTURE.md` (300 行) + `PHILOSOPHY.md` (200 行) + 2-3 个核心附录 |
| 3 | 5-10 个真正影响设计的哲学家保留 | R12 附录 K/L 列了 100+ | Simondon / Bergson / Prigogine / Maturana / Metzinger |
| 4 | 其余 90+ 哲学家归档 | R12 附录 K/L | `reports/philosophy-references-archive.md` |

### 7.3 交付物 (2 周结束)

- 精简版手册 (500-1000 行 vs 6546 行)
- `archive/` 目录 (1100 空壳 + 90+ 哲学家)
- `Apeireth-rust/docs/phase-5-completion.md` (Phase 5 收尾报告)

### 7.4 风险

- 砍空壳后某些团队依赖失效
- 手册瘦身过度丢关键信息
- 缓解: Phase 5 第 1 周先做 archive (不删), 第 2 周确认无依赖后再瘦身手册

---

## 8. R14 时间线汇总 (26 周 / ~6 月)

| Phase | 周期 | 周数 | 累计 | 关键交付 |
|-------|------|------|------|----------|
| **Phase 0** | Week 1-4 | 4 | 4 | 接口规范 (rust-traits-spec + api-mapping-py-to-rust) |
| **Phase 1** | Week 5-8 | 4 | 8 | Rust 关键路径 (crates/memory, V1130 wallclock 2.5s) |
| **Phase 2** | Week 9-14 | 6 | 14 | V0.5/V1136/哲学 Rust 重设计 (crates/asi + crates/philosophy) |
| **Phase 3** | Week 15-16 | 2 | 16 | PyO3 桥 (crates/pyo3-bridge, Python mvp/ 切换) |
| **Phase 4** | Week 17-20 | 4 | 20 | 主人实测对比 (Python vs Rust 14 天) |
| **Phase 5** | Week 21-22 | 2 | 22 | 1100 空壳 + 6000 行瘦身 (archive + 精简手册) |
| **Phase 6** | Week 23-26 | 4 | 26 | R14 收尾 (team_finalize + 团队总结 + R15 启动决策) |
| **R14 总计** | Week 1-26 | **26 周 / ~6 月** | — | — |

**详细里程碑**:
- Week 4: 接口规范冻结 (主 23:44 干到底)
- Week 8: V1130 wallclock 2.5s 性能基准达成
- Week 14: V0.5/V1136/哲学守门 Rust 重设计完成
- Week 16: PyO3 桥暴露 + Python mvp/ 切换
- Week 20: 主人实测 Go/No-Go 决策
- Week 22: 1100 空壳归档 + 6000 行瘦身完成
- Week 26: R14 收尾 + R15 (后续) 启动

---

## 9. R14 预算与依赖 (主 17:43 实事求是)

### 9.1 预算

| 项目 | 预算 | 说明 |
|------|------|------|
| **业务改动** | ≤ 5000 行 | vs R12 收尾 ≤ 1500 行 (T13 设定) |
| **新增 Rust 代码** | ~5000 行 (crates/memory + crates/asi + crates/philosophy + crates/pyo3-bridge) | 1 个 workspace 4 个 crate |
| **新增 Python 切换代码** | ~500 行 (mvp/ PyO3 import) | 现有 Python mvp/ 部分切换 |
| **新增文档** | ~2000 行 (phase-0~5-completion.md + python-vs-rust-comparison.md) | R14 全程文档 |
| **总计** | ≤ 12500 行 (vs R12 收尾 ~3000 行) | 主 23:44 干到底 |

### 9.2 依赖 (6 触发条件)

详见 §1.

### 9.3 风险 (主 17:43)

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Python MVP 验证失败 | 中 (40%) | 高 (R14 失去依据) | Phase 0-1 先做, Phase 2-5 根据 R13 验证情况动态调整 |
| Rust 学习曲线 | 中 (30%) | 中 | 借鉴 hermes-agent-rs / VCP Rust substrate / 9-crate workspace |
| PyO3 兼容性 | 低 (20%) | 中 | Phase 3 第 1 周先做 PoC |
| 主人实测 Go/No-Go 失败 | 中 (30%) | 高 (R14 终止) | Phase 4 第 3 周留 buffer |
| 砍 1100 空壳后某些团队依赖失效 | 低 (15%) | 低 | Phase 5 第 1 周先 archive 不删 |
| 总计 | — | — | 主 17:43 + 主 23:44 |

### 9.4 缓解策略

- **MVP 阶段持续验证** (Phase 0-1 先做, Phase 2-5 动态调整)
- **主人硬约束保留** (V0.5/V1136/哲学守门 Rust 重设计, 不重写规则)
- **Phase 2-5 失败回退** (Phase 2 失败 → 回到 Python MVP, Phase 4 Go/No-Go 失败 → 终止 R14)
- **借鉴前人经验** (主 19:33: DeltaMemory-Rust / hermes-agent-rs / VCP Rust / 9-crate)

---

## 10. R14 与 R12 硬约束的衔接 (主 22:33 + 主 17:43 + 主 17:58)

### 10.1 ❌ 不可妥协的禁止项 (R14 重设计时不重写规则)

| # | 禁止项 | 理由 | R14 应用 |
|---|--------|------|----------|
| 1 | ❌ **不重写 V0.5 公式** | V0.5 是自设指标, 无客观意义, 重写会引入新的伪 KPI | Rust 实现 + 加"自设指标"标注, 不改公式结构 |
| 2 | ❌ **不重做 V1136 真测引擎** | V1136 已 LOCKED, 重做会回退 R11 已落 | Rust 重设计 5+2 子测度真实可执行测试, 砍 0.05 装饰 |
| 3 | ❌ **不重写哲学守门** | V3 9 键 LOCKED + 5 项不假装 R11 已落 | Rust trait + 状态机实现, 规则不变 |
| 4 | ❌ **不砍 Apeireth 现有 1100+ v 模块** (Phase 1-4 期间) | R12 接手硬约束 §6 保护 | R14 Phase 5 才清理 |
| 5 | ❌ **不修改 APEIRETH-COMPLETE-OMNIBUS 6546 行手册** (Phase 1-4 期间) | 用户硬约束 + R11 收尾硬约束 | R14 Phase 5 才瘦身 |
| 6 | ❌ **不写 ASI 北极星公式** | 主 17:58 不假装达到 ASI | Rust 实现 V0.5 时加 `#[doc(hidden)]` + 透明标注"自设指标" |
| 7 | ❌ **不刷 KPI** | continuity 0.05×1=0.05 是 KPI 装饰 | R14 Phase 2 砍 0.05 装饰 |
| 8 | ❌ **不假装达到 Phenomenal consciousness** | phenomenal consciousness 是哲学开放问题 | R14 Rust 哲学守门守住 |

### 10.2 ✅ 不可妥协的承诺项 (R14 全程)

| # | 承诺项 | 理由 | R14 应用 |
|---|--------|------|----------|
| 1 | ✅ **实事求是** (主 17:43) | 文档化 R11/R12 已落真态, 不掩盖 W2/W4 False / dashboard yellow / V1130 timeout | R14 Phase 4 主人实测对比实事求是 |
| 2 | ✅ **不假装** (主 17:58) | 不写 V0.5 = 0.8595 / 0.9063 这种数字假装 ASI | R14 Phase 2 V0.5 加"自设指标"标注 |
| 3 | ✅ **走在前人经验上** (主 19:33) | 借鉴 5-10 个真正影响设计的哲学家 + Rust 借鉴 hermes-agent-rs / VCP / DeltaMemory | R14 Phase 0 + Phase 5 |
| 4 | ✅ **干到底** (主 23:44) | 工程化证据完整, 不留悬而未决 | R14 6 阶段全部完成 |
| 5 | ✅ **任何人都能接手** (主 00:56) | 文档 + 测试 + 跨 session 记忆让接手 1 小时懂一切 | R14 Phase 5 手册瘦身到 500-1000 行 |
| 6 | ✅ **ASI 北极星导向** (主 22:33) | 终极目标是 ASI 北极星架构对齐, 不是 ASI 数字 | R14 全程守住 |

---

## 11. R14 团队接手建议 (主 19:33 + 主 00:56)

### 11.1 团队规模 (8-10 角色)

| # | 角色 | 核心职责 | 关键产出 |
|---|------|----------|----------|
| 1 | **Rust 工程师 (senior)** | crates/memory + crates/asi + crates/philosophy 设计 + 实现 | 5000 行 Rust 代码 |
| 2 | **Rust 工程师 (mid)** | crates/pyo3-bridge 实现 + Python mvp/ 切换 | 500 行 PyO3 + 500 行 Python 切换 |
| 3 | **系统工程师** | 9-crate workspace + 编译流水线 + CI/CD | Cargo.toml + Dockerfile + .github/workflows |
| 4 | **AI 工程师** | V0.5/V1136 重设计 + 5 项不假装规则 Rust 实现 | crates/asi + crates/philosophy |
| 5 | **测试工程师** | Python mvp/ 接口兼容测试 + Rust 单元测试 + 性能基准 | pytest + cargo test + criterion |
| 6 | **文档工程师** | rust-traits-spec + api-mapping-py-to-rust + phase-N-completion.md | ~2000 行文档 |
| 7 | **DevOps 工程师** | V1132 部署 + PyO3 跨平台编译 + 监控 | Dockerfile + k8s + Prometheus |
| 8 | **Code Reviewer** | Rust trait 设计评审 + 类型系统强制验证 | 评审报告 |
| 9 | **QA 工程师** | 主人实测对比 (Python vs Rust 14 天) | comparison 报告 |
| 10 | **Architect** | crates 架构 + PyO3 接口设计 + 性能优化 | 架构评审报告 |

### 11.2 工具栈

- **Rust**: 1.80+ (rustc / cargo / rustup)
- **Cargo workspace**: 9-crate workspace (memory / asi / philosophy / pyo3-bridge / 5 其他)
- **PyO3**: 0.22+ (Python 3.13.14 ↔ Rust 桥)
- **Async runtime**: tokio 1.40+
- **Database**: SQLite (rusqlite 0.32+)
- **Serialization**: serde 1.0+ / bincode / postcard
- **Testing**: cargo test / pytest / criterion (性能基准)
- **Linting**: clippy 1.80+ / rustfmt
- **Build**: cargo build --release / PyO3 cross-compile

### 11.3 借鉴 (主 19:33 走在前人经验上)

- **DeltaMemory-Rust** (Lin et al. 2024) — 跨 session 记忆 Rust 实现
- **Lumio-Research/hermes-agent-rs** (110K Rust LOC) — Agent Rust 工具栈
- **VCP Rust substrate** — Rust 调度器 + SQLite 集成
- **9-crate workspace** (Apeireth 已有) — Rust 工作空间组织

### 11.4 主人哲学锚点 (主 22:33 + 主 17:43 + 主 17:58 + 主 19:33 + 主 23:44 + 主 00:56)

R14 团队接手时**必须遵守**的 6 大哲学锚点 (主 00:56 任何人都能接手 = 1 小时懂):

1. **主 22:33 ASI 北极星** — Rust 重写北极星架构对齐, 不是数字指标
2. **主 17:43 实事求是** — 26 周路线图实事求是, 不假装 R13 验证完成
3. **主 17:58 不假装** — V0.5/V1136/哲学守门 Rust 重设计不假装达到 ASI
4. **主 19:33 走在前人经验上** — 借鉴 DeltaMemory-Rust / hermes-agent-rs / VCP Rust / 5-10 哲学家
5. **主 23:44 干到底** — 26 周 6 阶段全部完成, 不留半成品
6. **主 00:56 任何人都能接手** — Phase 5 手册瘦身到 500-1000 行, 让接手 1 小时懂

### 11.5 后续行动 (R14 启动后)

| # | 行动 | 责任方 | 触发 |
|---|------|--------|------|
| 1 | R14 Phase 0 启动 (接口规范) | R14 team lead | R13 MVP 触发条件 6 条全部满足 |
| 2 | R14 Phase 1 启动 (Rust 关键路径) | Rust 工程师 | Phase 0 完成 |
| 3 | R14 Phase 2 启动 (V0.5/V1136/哲学 Rust 重设计) | AI 工程师 + Rust 工程师 | Phase 1 完成 |
| 4 | R14 Phase 3 启动 (PyO3 桥) | Rust 工程师 + Python 工程师 | Phase 2 完成 |
| 5 | R14 Phase 4 启动 (主人实测对比) | 主人 + QA 工程师 | Phase 3 完成 |
| 6 | R14 Phase 5 启动 (1100 空壳 + 6000 行瘦身) | Rust 工程师 + 文档工程师 | Phase 4 Go/No-Go 决策通过 |
| 7 | R14 Phase 6 启动 (R14 收尾) | R14 team lead | Phase 5 完成 |
| 8 | R15 启动决策 | 主人 + leader | R14 收尾完成 |

---

_Last update: 2026-07-30 13:30, by 楚零 (技术文档工程师, T23: `be28143c-7546-4423-b503-4a7cf5a58825` R14 Rust 重写路线图详细文档).

_基于 T14 R12 重大变更预备文档 §5 (26 周路线图 + 8 类大变动) + T13 报告 7 任务清单 (T6-F-1/2 + T9 接续 + T6-G + T6-H + team_land_integration + team_finalize, R12 收尾预算 ≤ 1500 行) + R13 MVP 落地状态 (Phase 0+1.1 ✅ done + Phase 1.2 🔄 in_progress). 不重写任何工程 / 不修改主手册 / 不砍 1100 空壳, 仅记录 R14 详细路线图 + 6 触发条件 + 6 阶段详细计划 + 26 周时间线 + 预算与依赖 + R14 团队接手建议. 主人哲学硬约束全保护到 R14 重设计. R13 MVP 主人实测稳定后启动 R14._

_主哲学 anchor 6 个全贯穿: 主 22:33 ASI 北极星 (北极星导向 + Rust 重设计不假装 ASI) + 主 17:43 实事求是 (26 周路线图 + 6 触发条件 + 风险评估) + 主 17:58 不假装 (V0.5/V1136/哲学守门 Rust 重设计不假装达到 ASI) + 主 19:33 走在前人经验上 (借鉴 DeltaMemory-Rust / hermes-agent-rs / VCP Rust / 5-10 哲学家) + 主 23:44 干到底 (6 阶段 26 周 + 5000 行预算 + 完整里程碑) + 主 00:56 任何人都能接手 (Phase 5 手册瘦身到 500-1000 行 + 8-10 角色接手建议)._