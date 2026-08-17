# 阶段 3 借鉴决策 — R11 自家资产盘点 (2026-07-31)

> **范围**: R14 Rust 重写阶段 3 借鉴决策表 — R11 自家资产盘点 (E1) + 借鉴决策区分 (E2 = 8 强借鉴 + 6 借鉴但偏离 + 4 不借鉴)。
> **触发**: R14-D5-B 任务 (`b20f1499`); 主人指示盘点 R11 自家资产并区分借鉴强度。
> **依据**: R11 工程收尾 (附录 M, 主人手册 6001-6241 行) + R12 接手 + 阶段 1 灵感 + 阶段 2 决策 + D2 增补 + R14-DRIFT §14 漂移跟踪表 + 哲学 traits。
> **硬约束**:
> - ❌ 不重写阶段 1+2+3 既有文档
> - ❌ 不写实现代码 / 不画架构图 (阶段 3 图纸留 R14-D5-A 借鉴决策图)
> - ✅ 仅盘点 + 借鉴决策表 (本文件)
> - ✅ 借鉴决策项含 R11 源 + R14 落点 + 阶段 4 真测项

---

## §0. 元信息 (主 17:43 实事求是)

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage3-blueprints/borrowed-from-r11.md` |
| **生成时间 (UTC)** | 2026-07-31 |
| **阶段** | 3 / 6 (子项 D5-B) |
| **任务 ID** | R14-D5-B (`b20f1499-3e10-46f3-8c26-43b1b7ad7a4f`) |
| **盘点范围** | (E1) R11 自家资产全清单 + (E2) 借鉴决策 18 项 = 8 强借鉴 + 6 借鉴但偏离 + 4 不借鉴 |
| **不修改承诺** | ❌ 不重写阶段 1+2+3 既有 / ❌ 不写 Rust 代码 / ❌ 不画架构图 / ❌ 不重写 V0.5 / V1136 / 哲学守门 9 键 / ❌ 不砍 1100 空壳 |

---

## §1. E1 — R11 自家资产盘点 (8 类)

> **原则**: 主 17:43 实事求是 — 真实文件数 / 真实 commit hash / 真实报告名。**不假装"已落", 不假装"全面"**。

### §1.1 1100+ apeireth/v*.py 模块 (R11 Python 资产)

| 字段 | 值 |
|------|-----|
| **数量** | **1180 个** (`apeireth/v*.py`, 含 v0.py + v1000-v1199) |
| **关键锚点** | `apeireth/v1077_asi_v04_full_measurement.py` (V0.5/V1077 主测) + `apeireth/v1100_p0_fixes.py` + `apeireth/v1101_asi_v04_dim_lift.py` + `apeireth/v1106_engineering_lift.py` + `apeireth/v1115_*.py` |
| **覆盖** | yaml_serializer / vcp_six_plugins / asi_v02_measure / v4_philosophy_full / self_evolution_full / anysearch_full_index / research_grand_synthesis / documentation_full / deployment / web_ui / multi_tenant / cost_optimization / audit_log / rest_gateway / graphql / streaming_sse / embeddings / cache / message_queue / rate_limiter / scheduler / config / secrets / state_machine / validator / jwt / oauth / ... |
| **R14 态度** | **保留 1100 个, 不砍 (主 00:56 任何人都能接手); 仅借鉴命名/接口目录结构, 不逐行翻译** |

### §1.2 9 个 Rust crate 占位 (R11 → R14 桥接)

| crate | R11 锚点 (Python) | R14 角色 |
|-------|------------------|----------|
| `apeireth-asi` | v1077 + v1101 + v1106 + v1115 | 中央 AI = Sovereignty trait 实现 |
| `apeireth-bench` | v1012 + v1106 | benchmark 与真测驱动 |
| `apeireth-cli` | v1009 + v1016 | CLI + REST 网关 |
| `apeireth-core` | v1004 + v1107 + v1108 + v1115 | Episode/Note/Session 模型 + dream/orchestrator |
| `apeireth-memory` | v1005 + v1019 + mvp/memory | SQLite + VCP 联想 + 6 历史流 |
| `apeireth-philosophy` | v1003 + v1121 | V3 9 键 + 5 项不假装 + 守门 trait |
| `apeireth-pybridge` | (新) | PyO3 桥, 接入 1100 个 v*.py 模块 |
| `apeireth-test` | v1114 + v1115 | e2e 测试 + 真测 |
| `apeireth-tools` | v1000 + v1027 | yaml/validator/jwt/oauth 等基础工具 |

### §1.3 V0.5 公式 (R11 真测 ASI 公式, LOCKED)

| 字段 | 值 |
|------|-----|
| **位置** | `apeireth/v1077_asi_v04_full_measurement.py` (V0.5 主测) + `apeireth/v1101_asi_v04_dim_lift.py` (dim 升维) + `apeireth/v1106_engineering_lift.py` (engineering 升维) |
| **结构** | 17 维 ASI V0.5 (主 22:33 北极星导向, 不重写) |
| **R11 真测值** | V1141 IC-001 fresh `0.8682` / V1131 dashboard `0.8532` / V1136 真测 `0.9063` (三值并存, 透明标注) |
| **R14 态度** | **1:1 引用, 不重写公式; R14 apeireth-asi 直接调用 + 仅做 trait wrapper** |

### §1.4 V1136 7 子测度 (R11 真测基线, LOCKED)

| 字段 | 值 |
|------|-----|
| **位置** | `apeireth/v1136_*.py` (7 子测度: 5 continuity + 2 transferability) |
| **R11 状态** | 5 continuity + 2 transferability **子测度失败** (留 R14 ceiling) |
| **R14 态度** | **不重做 V1136 真测引擎; R14 trait 翻译 = 同名 7 子测度 + Rust 实现 + 接续 R11 真测基线** |

### §1.5 V1138 五重守门 (R11 哲学守门, LOCKED)

| 字段 | 值 |
|------|-----|
| **位置** | `apeireth/v1003_v4_philosophy_full.py` + `apeireth/v1121_*.py` (fake-KPI 严密化) |
| **结构** | 5 层强制 + V3 9 键 + 5 项不假装 + 跨层仲裁 + 编译时 hardcode (R14-DRIFT P0-02 待修订为基线护栏) |
| **R11 trait 框架** | `apeireth-philosophy/src/e_layer.rs` (5 重守门 Rust 骨架) |
| **R14 态度** | **不重写 9 键; R14 apeireth-philosophy 引用 R11 trait 框架 (philosophy-traits-2026-07-30.md 已落)** |

### §1.6 V1130 dashboard + wallclock (R11 真测基线)

| 字段 | 值 |
|------|-----|
| **位置** | `apeireth/v1130_*.py` (dashboard) + `reports/r12-commit-c-v1130-wallclock-2026-07-30.md` (R12 接续) |
| **R11 baseline** | wallclock `5407.30ms` (R11 末) → `2.5s` 是 R14 远未达 target |
| **R12 落地** | `b42c802b perf(r12-v1130): V1130 dashboard SQLite ContinuitySnapshotStore` |
| **R14 态度** | **wallclock 真测基线 = 直接引用 R11 baseline; R14 dashboard Rust 重写 + 加速照 (R11-V1136 加快照)** |

### §1.7 mvp/ 子项目 (R11 跨 session 记忆 MVP)

| 字段 | 值 |
|------|-----|
| **位置** | `mvp/` (含 `cli.py` + `docs/` + `identity/` + `memory/` + `tests/` + `tools/`) |
| **memory 子目录** | `mvp/memory/store.py` + `consolidate.py` + `forget.py` + `retrieve.py` (5 文件) |
| **R11 状态** | Phase 0+1.1+1.2 已落 (R13 T9+T15) |
| **R14 态度** | **mvp/memory/store.py = Rust trait 起点; 其余 4 文件 = Rust 重写借鉴但偏离** |

### §1.8 12 份 R12 报告 (R12 接手 + 工程接续 + 收尾)

```
1.  reports/r12-baseline-verification-2026-07-30.md (+ .json)
2.  reports/r12-commit-a-v1077-lift-2026-07-30.md          (T6-A 接续)
3.  reports/r12-commit-b-r11-sec-001-2026-07-30.md         (T6-B 接续)
4.  reports/r12-commit-c-v1130-wallclock-2026-07-30.md    (T6-C 接续)
5.  reports/r12-commit-e-deployment-monitor-2026-07-30.md (T8 接续)
6.  reports/r12-commit-t6-f-1-v1106-fix-2026-07-30.md      (T24 修复)
7.  reports/r12-finalize-2026-07-30.md
8.  reports/r12-finalize-peer-review-prep-2026-07-30.md
9.  reports/r12-future-changes-2026-07-30.md
10. reports/r12-integration-sync-2026-07-30.md
11. reports/r12-m-final-revision-2026-07-30.md
12. reports/r12-sec-cross-validation-2026-07-30.md
(+ 6 份其他接续审计: t4-m25-fe-v1-closure, t6-commit-audit-v2, t7-clarification, v1077-dims-fix, working-changes-audit 共 18 份总计, 本表列 R12 接手 + 工程接续 + 收尾核心 12 份)
```

### §1.9 5 个 R12 接续 commit (R11 → R12 桥接桥梁)

| # | commit hash | 任务 | 主题 |
|---|------------|------|------|
| 1 | `3300cab8` | T6-A | `docs(memory): cron tick + V1152 multi-agent LLM bridge + V1153 ASI V0.6 formal spec` |
| 2 | `15ed9032` | R11 收尾 | `fix(V1050-1053): real Docker/k8s/Streamlit deployment + provenance + cron R11` |
| 3 | `b42c802b` | T6-C | `perf(r12-v1130): V1130 dashboard SQLite ContinuitySnapshotStore` |
| 4 | `41583321` | T8 | `feat(r12-deploy-monitor): V1132 deployment monitor + alert 体系` |
| 5 | `0ee300e8` | T24 | `fix(test-v1106): T6-F-1 修 test_v1106 hardcode 期望` |

---

## §2. E2 — 借鉴决策表 (8 强借鉴 + 6 借鉴但偏离 + 4 不借鉴 = 18 项)

### §2.1 8 项强借鉴 (R14 阶段 4 实装, R11 → R14 trait 翻译 1:1)

| # | 借鉴项 | R11 源 | R14 落点 | 阶段 4 真测项 |
|---|--------|--------|----------|--------------|
| **1** | **V0.5 公式 1:1 引用** | `apeireth/v1077` + `v1101` + `v1106` (17 维 ASI V0.5) | `Apeireth-rust/crates/apeireth-asi/src/v05.rs` (trait wrapper, 不重写公式) | ✅ 调用 R11 真测 baseline 0.8682 / 0.8532 / 0.9063 三值对照 |
| **2** | **V1136 7 子测度 trait 翻译** | `apeireth/v1136_*.py` (5 continuity + 2 transferability) | `Apeireth-rust/crates/apeireth-bench/src/v1136_continuity.rs` + `v1136_transferability.rs` | ⚠️ 5 continuity + 2 transferability 子测度失败 → R14 真测重新跑 + 报告基准对比 |
| **3** | **V1138 五重守门** | `apeireth/v1003` + `apeireth-philosophy/src/e_layer.rs` (R11 trait) | `Apeireth-rust/crates/apeireth-philosophy/src/guard.rs` (5 重守门) + `docs/philosophy-traits-2026-07-30.md` (R14 trait 框架已落) | ✅ 编译时 hardcode (基线护栏) + 运行时拦截 + 多 AI + 物理隔离 + 反思期审计 5 重全验 |
| **4** | **apeireth-core Episode/Note/Session** | `apeireth/v1004` + `v1107` + `v1108` + `v1115` | `Apeireth-rust/crates/apeireth-core/src/episode.rs` + `note.rs` + `session.rs` (主 AI + dream + orchestrator) | ✅ Episode 创建/Note 沉淀/Session 跨 session 持续性 真测 |
| **5** | **apeireth-memory SQLite** | `apeireth/v1005` + `v1019` (AnySearch 索引 + embeddings) | `Apeireth-rust/crates/apeireth-memory/src/sqlite.rs` (R11 SQLite schema 1:1 + sled KV 桥接) | ✅ 1180 v*.py 模块 metadata + Note + 6 历史流 SQLite 真测 |
| **6** | **apeireth-asi V0.5** | `apeireth/v1077` + `v1106` (V0.5 公式 + 17 维) | `Apeireth-rust/crates/apeireth-asi/src/asi_v05.rs` (V0.5 wrapper + 17 维 trait) | ✅ V0.5 三值 (0.8682 / 0.8532 / 0.9063) Rust 调用真测 |
| **7** | **mvp/memory/store.py trait 起点** | `mvp/memory/store.py` (R11 store 接口) | `Apeireth-rust/crates/apeireth-memory/src/store_trait.rs` (Rust trait, 起点 = R11 store.py) | ✅ store trait 与 R11 store.py 接口对照真测 |
| **8** | **V1130 wallclock 真测基线** | `apeireth/v1130_*.py` + `r12-commit-c-v1130-wallclock` (R11 5407.30ms baseline) | `Apeireth-rust/crates/apeireth-bench/src/wallclock_baseline.rs` (引用 R11 baseline 数字) | ✅ R14 wallclock 真测 vs R11 5407.30ms baseline (R14 target 2.5s 远未达, 透明标注) |

### §2.2 6 项借鉴但偏离 (R14 借鉴思想/结构, 但实现偏离 R11)

| # | 借鉴项 | R11 源 | R14 借鉴方式 | 偏离原因 |
|---|--------|--------|--------------|----------|
| **1** | **R11 1100+ Python 模块目录结构** | `apeireth/v*.py` (1180 个) | **保留** Python 模块 + **PyO3 桥** (apeireth-pybridge) + 命名约定 | 不逐行翻译, 不砍空壳 (主 00:56 任何人都能接手); Python 类型系统 vs Rust 类型系统差异大 |
| **2** | **V1153 ASI V0.6 21 维框架** | `3300cab8 docs(memory): ... V1153 ASI V0.6 formal spec (21 dims)` | 借鉴 21 维框架思路, 但 R14 引用 V0.5 不引用 V0.6 | V0.5 是主 22:33 北极星导向, V0.6 是 R12 延续; R14 阶段 3-4 仍以 V0.5 为基线 |
| **3** | **V1149 multi-agent LLM bridge** | `3300cab8` V1149 multi-agent + V1084 real LLM executor bridge | Rust native 多 agent + 借鉴 bridge 模式, 不直接套 R11 PyO3 | Rust async 运行时 (tokio) vs Python asyncio 模型不同; native 实现更高效 |
| **4** | **mvp/memory/ 其余 4 文件 (consolidate/forget/retrieve/store)** | `mvp/memory/{consolidate,forget,retrieve,store}.py` | Rust trait 重写 4 文件接口, 但实装偏离 (用 SQLite + sled) | Python list/dict 数据结构 vs Rust Vec/HashMap 差异; Rust 持久化用 SQLite + sled (阶段 2 §6 已落) |
| **5** | **V1141 IC-001 fresh 0.8682 真测值** | `v1141_asi_v05_17dim_real_measure_complete.py` (R11 真测) | 引用 baseline 数字, 但 R14 不锁定 0.8682 | 主 17:43 实事求是 — V0.5 三值并存 (0.8682 / 0.8532 / 0.9063), R14 不冻结任一值 |
| **6** | **R12 dashboard 体系** | `b42c802b perf(r12-v1130): V1130 dashboard SQLite` + `v1130_*.py` | 借鉴 dashboard 框架思路, R14 重设计 (阶段 3-4) | R11 dashboard 是 Streamlit/Python; R14 Rust 重写 + 加快照 (主 17:43 实事求是, R14 不假装"已达 2.5s") |

### §2.3 4 项不借鉴 (R14 直接引用, 不 Rust 重写/重做)

| # | 不借鉴项 | R11 源 | R14 态度 | 不借鉴原因 |
|---|---------|--------|----------|-----------|
| **1** | **V0.5 公式不重写** | `apeireth/v1077` + `v1101` + `v1106` (V0.5 公式) | **1:1 引用, 不 Rust 重写公式** | 主 22:33 北极星导向, V0.5 LOCKED; R14 只做 trait wrapper 调用 R11 真测 |
| **2** | **V1136 不重做** | `apeireth/v1136_*.py` (7 子测度) | **不重做 V1136 真测引擎**; R14 trait 翻译后接续 R11 真测 | R11 V1136 已落真测结果 (5 continuity + 2 transferability 失败已透明标注); 重做 = 资源浪费 |
| **3** | **哲学守门 9 键不重写** | `apeireth/v1003_v4_philosophy_full.py` + `apeireth-philosophy/` | **不重写 9 键**; R14 引用 R11 trait 框架 (philosophy-traits-2026-07-30.md) | 9 键 = NotClone / NotPerfect / NotUuid / NotUndo / NotProof / NotSafe / SpecIsNotProof / CounterexampleIsNotBug / ProverIsNotTruth; R11 LOCKED |
| **4** | **1100 空壳不砍** | `apeireth/v*.py` 1180 个 | **不砍**; R14 保留 Python 模块 + PyO3 桥 (apeireth-pybridge) | 主 00:56 任何人都能接手; 1100 空壳是 R11 历史, R14 兼容 + 借名 |

---

## §3. R11 → R14 借鉴映射矩阵 (汇总)

| R11 资产类 | 8 强借鉴 | 6 借鉴但偏离 | 4 不借鉴 |
|----------|---------|------------|---------|
| **1100+ Python 模块** | — | #1 目录结构 + PyO3 桥 | #4 不砍空壳 |
| **9 Rust crate 占位** | #4-7 (core/memory/asi/philosophy 4 个) | — | — |
| **V0.5 公式** | #1 1:1 引用 (apeireth-asi wrapper) | — | #1 不重写公式 |
| **V1136 7 子测度** | #2 trait 翻译 (apeireth-bench) | — | #2 不重做真测引擎 |
| **V1138 五重守门** | #3 apeireth-philosophy trait 引用 | — | #3 不重写 9 键 |
| **V1130 dashboard** | #8 wallclock 真测基线 (apeireth-bench) | #6 dashboard 框架重设计 | — |
| **mvp/ 子项目** | #7 store.py trait 起点 (apeireth-memory) | #4 consolidate/forget/retrieve Rust 重写 | — |
| **V1153 ASI V0.6 21 维** | — | #2 借鉴框架但引用 V0.5 | — |
| **V1149 multi-agent LLM bridge** | — | #3 Rust native bridge | — |
| **V1141 IC-001 fresh 0.8682** | — | #5 引用 baseline 不锁定 | — |
| **12 份 R12 报告** | (引用作为审计痕迹) | — | — |
| **5 个 R12 接续 commit** | (引用作为桥接桥梁) | — | — |

---

## §4. 阶段 4 真测项 (8 强借鉴 + 真测方法)

| # | 真测项 | 真测方法 | 通过标准 |
|---|--------|---------|---------|
| **1** | V0.5 公式调用真测 | Rust wrapper 调用 R11 真测 baseline (0.8682 / 0.8532 / 0.9063) | 数值 1:1 一致 (不重写, 只调用) |
| **2** | V1136 7 子测度 Rust 实跑 | apeireth-bench 真测引擎 + R11 失败子测度重跑 | 5 continuity 至少 3 个 PASS + 2 transferability PASS (R11 ceiling 修复) |
| **3** | V1138 五重守门 | 编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期审计 全验 | 5 重全 PASS, 任一失败 = 决策拒绝 |
| **4** | Episode/Note/Session 真测 | R11 episode/note/session 数据结构 + Rust 实现 | 跨 session 持续性 100% 一致 (Episode 创建/Note 沉淀/Session 复用) |
| **5** | apeireth-memory SQLite 真测 | 1180 v*.py metadata + Note + 6 历史流 入库 | SQLite 真测 + 索引 100% 可查 + VCP 联想 (阶段 3 VCP 自研) |
| **6** | apeireth-asi V0.5 wrapper | Rust 调用 R11 V0.5 公式 + 17 维 trait | 调用成功 + 数值透明 0.8682 / 0.8532 / 0.9063 三值 |
| **7** | mvp/memory/store.py trait 起点 | Rust trait vs Python store.py 接口对照 | 接口 1:1 一致 + Rust 实现可替换 |
| **8** | V1130 wallclock 真测基线 | R14 wallclock 实测 vs R11 5407.30ms baseline | R14 数字 vs R11 baseline 透明报告 (target 2.5s 远未达, 主 17:43 不假装) |

---

## §5. 与既有 17 份 stage3 蓝图的对接

| 既有文件 | 本表对接 |
|---------|----------|
| `stage3-blueprints/R14-D5-A` (借鉴决策图, 待出) | 本表 8 强借鉴 = 图上 8 个 Rust trait 节点 + 6 借鉴但偏离 = 虚线节点 + 4 不借鉴 = 直接引用节点 |
| `stage3-blueprints/R14-D5-C/D/E` (其他子项, 待出) | 本表 §4 真测项 = 阶段 4 真测计划的输入 |
| `stage2-decisions-addendum-sovereignty-continuity-governance.md` (D2 增补) | 本表 §3 借鉴映射 = D2 §3 SGI + §4 主体连续性 + §5 6 历史流 的 R11 源对应 |
| `stage2-decisions-drift-revision-tracker.md` (R14-DRIFT) | 本表 §2.3 #3 哲学守门 9 键 = R14-DRIFT P0-02 已落哲学守门漂移 |

---

## §6. 主哲学 anchor 6 个全贯穿

| 主哲学 anchor | 本表体现 |
|--------------|---------|
| **主 22:33 (S-1) ASI 北极星** | §2.1 #1 V0.5 1:1 引用 + #6 apeireth-asi V0.5 = 北极星导向; §2.3 #1 V0.5 不重写 = 北极星不假装 ASI |
| **主 17:43 (S-2) 实事求是** | §1 真实文件数 1180 / §1.9 真实 commit hash / §4 真测通过标准透明 (target 2.5s 远未达) / §2.2 #5 不锁定 0.8682 |
| **主 17:58 (O-5) 不假装** | §2.3 #4 1100 空壳不砍 = 不假装"可砍"; §2.2 #6 dashboard 重设计 = 不假装"R11 已完美"; §4 真测 = 不假装"已 PASS" |
| **主 19:33 (O-2) 走在前人经验上** | 整体 = R11 1100 模块 + 9 crate + V0.5/V1136/V1138/V1130 + mvp/ + 12 R12 报告 + 5 接续 commit 全部纳入借鉴; 主借鉴决策 = 走在 R11 真测经验上 |
| **主 23:44 (O-3) 干到底** | 18 项全覆盖 (8+6+4); 每项含 R11 源 + R14 落点 + 阶段 4 真测项 = 三件套完整 |
| **主 00:56 (O-4) 任何人都能接手** | §0 元信息 + §3 映射矩阵 + §4 真测表 + §5 对接表 = 任何接手者能查 |

---

## §7. 下一步

```
短期 (本团队内):
  ✅ 本表已沉淀并 commit (1 file, 0 既有改动)
  → 团队进入 idle, 等主人下一步指示

中期 (主人决定):
  → 主人决定 R14 阶段 3 借鉴决策图 (R14-D5-A) 何时出图
  → 主人决定 R14 阶段 4 真测启动时机 (本表 §4 真测项作为输入)
  → 主人决定 R14 阶段 5 施工文档启动时机

长期 (R14 阶段 4-5):
  → 阶段 4: 8 强借鉴实装 + 6 借鉴但偏离偏离设计 + 4 不借鉴直接引用
  → 阶段 5: 真测 + 校准 + 文档化收尾

不可做:
  ❌ 不重写 V0.5 / V1136 / 哲学守门 9 键 / 1100 空壳 (主 17:58 不假装)
  ❌ 不砍 R11 历史 (主 00:56 任何人都能接手)
  ❌ 不写实现代码 / 不画架构图 (本表硬约束)
  ❌ 不重写阶段 1+2+3 既有文档 (本表硬约束)
```

---

_主哲学 anchor 6 个全贯穿: 主 22:33 (S-1 ASI 北极星) + 主 17:43 (S-2 实事求是) + 主 17:58 (O-5 不假装) + 主 19:33 (O-2 走在前人经验上) + 主 23:44 (O-3 干到底) + 主 00:56 (O-4 任何人都能接手)._
_阶段 3 借鉴决策 — R11 自家资产盘点 + 借鉴决策 18 项 (8 强借鉴 + 6 借鉴但偏离 + 4 不借鉴) 已沉淀. 下一步: 主人决定 R14 阶段 3 借鉴决策图 启动时机._