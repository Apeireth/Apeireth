# 验证协议 (M1/M2/M3 里程碑 + 5 重守门)

> **作者**: architect2 (Ponytail: full)
> **生成时间**: 2026-08-02
> **依据**: docs/stage4/architecture-stage4-engineering-landing.md §10.4 (3 里程碑) + §10.5 (5 重守门) + docs/stage6/22-trait-interlock.md + docs/stage6/V-measure-design.md
> **状态**: **阶段 6 验证协议总纲** — 不修改 stage1-5 LOCKED 文档
> **承接**: 阶段 5 (trait impl 落地) → 阶段 6 (本协议执行)

---

## 0. 验证三层 (Ponytail: 1 张表)

| 层级 | 名称 | 时机 | 工具 | 失败后果 |
|------|------|------|------|----------|
| **M1** | 编译时验证 | 每次 `cargo build` / `cargo check` | cargo + clippy + cargo-deny + trait interlock macro | 编译失败 = 阻塞 PR |
| **M2** | 启动时验证 | supervisor 树启动 + 健康检查 | 18 crate init + 6 DB + VMeasure::measure_all | 启动崩溃 = 阻塞 release |
| **M3** | 首次对话验证 | 端到端首次 user input | 18 项 §6.1 真测项 + 5 重守门 + R-Measure 13 维度 | 真测失败 ≥ 1 项 = 阻塞 milestone |

---

## 1. M1 编译时验证 (Ponytail: 1 张表)

> **目标**: 编译期 hardcode 12 键 + 双洋葱 trait + Identity 类型完整性 + 22 互锁 trait 完整性

| 检查项 | 工具 | 期望结果 | 失败信号 |
|--------|------|----------|----------|
| **12 键编译时 hardcode** | `cargo check --workspace` | 0 error | V3 9 键 + 3 新增键 编译失败 |
| **双洋葱 trait bound** | `cargo check -p apeireth-onion` | 0 error | PrincipleOnion/PermissionOnion trait bound 缺失 |
| **Identity 类型完整性** | `cargo test -p apeireth-core identity` | 1 passed | Identity<T: Carrier> 字段漂移 |
| **22 互锁 trait 完整** | `cargo check -p apeireth-stage6` | `interlock_assert!` 全通过 | 互锁矩阵断裂 |
| **24 维 + 9 子测度计数** | `cargo check` (const 断言) | V05_DIM_COUNT=24, V1136_SUBMEASURE_COUNT=9 | 计数漂移 |
| **clippy lint** | `cargo clippy --workspace -- -D warnings` | 0 warning | 任何 warning = 阻塞 |
| **dependency 安全** | `cargo-deny check` | 0 advisory | 已知 CVE = 阻塞 |
| **unsafe_code 检查** | `cargo build --deny=unsafe_code` | 0 error | apeireth-core / apeireth-onion / apeireth-sovereignty 使用 unsafe = 阻塞 |
| **License 一致性** | `cargo-deny license` | Apache-2.0 / MIT only | GPL 等 copyleft = 阻塞 |
| **格式化** | `cargo fmt --check` | 0 diff | rustfmt 偏离 = 阻塞 |

**M1 通过标准 (Ponytail: 1 行)**: 上述 10 项全部通过 = M1 ✓

```rust
// docs/stage6/verification-protocol.md §1 — M1 编译期断言 sketch

/// M1 编译时验证函数 (CI 调用)
pub fn m1_compile_time_check() {
    // 12 键编译时 hardcode (引用 v4.1 §15)
    fn _check_12_keys() {
        const _: () = {
            // PHL-01 not_pretend_safe
            // PHL-02b not_undo
            // PHL-03 X_is_not_Y
            // PHL-04 not_pretend_unobservable (Debug+Display+Log trait bound)
            // PHL-05 not_pretend_unscientific (#[test] 强制)
            // PHL-06 not_pretend_no_self_relation
            // + 6 新增键 (v4.1 §15 提议)
            assert!(true); // 占位, 阶段 5 由 backend_engineer 实施
        };
    }

    // 22 互锁 trait 完整性
    fn _check_22_interlock() {
        const _: () = {
            assert!(INTERLOCKED_TRAIT_COUNT == 22);
        };
    }

    // 24 维 + 9 子测度
    fn _check_v_measure_count() {
        const _: () = {
            assert!(V05_DIM_COUNT == 24);
            assert!(V1136_SUBMEASURE_COUNT == 9);
        };
    }
}
```

---

## 2. M2 启动时验证 (Ponytail: 1 张表)

> **目标**: 18 crate 全部 active + 6 DB 协同初始化 + V0.5 真测启动 (24 维) + V1136 真测启动 (9 子测度)

| 检查项 | 验证内容 | 期望结果 | 失败信号 |
|--------|---------|----------|----------|
| **18 crate active** | 所有 apeireth-* crate 初始化完成 | `supervisor.tree_active_count() == 18` | crate 启动失败 = 阻塞 |
| **6 DB 协同** | memory / relation / goal / stance / self_narrative / migration | 6 个 SQLite/WAL 文件全部打开 + migration 完成 | DB schema 不一致 = 阻塞 |
| **V0.5 24 维启动** | `VMeasureDispatcher.measure_all()` 成功 | 24 个 DimensionTrace 全部产生 | 维度缺失 = 阻塞 |
| **V1136 9 子测度启动** | `VMeasureDispatcher.measure_all()` 成功 | 9 个 DimensionTrace 全部产生 | 子测度缺失 = 阻塞 |
| **Council 7 强制顾问** | 7 个 MandatorySeat 全部注册 | Council.seat_count() >= 7 | 顾问缺失 = 阻塞 |
| **双洋葱统一体** | DefaultDoubleOnion 可构造 | `default_test_double_onion()` 返回 Ok | onion 构建失败 = 阻塞 |
| **电子环 11 节点** | ElectronicRing 初始化 | `ring.is_complete() == true` | 节点缺失 = 阻塞 |
| **HA 模式** | HumanAuthority 模式有效 | `ha.mode ∈ {SingleHuman, MultiHuman, Offline}` | mode 无效 = 阻塞 |
| **L0 物理隔离** | L0 HA 核心物理隔离就绪 | `l0_isolation_active() == true` | L0 未隔离 = 阻塞 (主人修正 #4) |
| **OTA 沙盒** | sandbox-validator 就绪 | `sandbox_validator_active() == true` | OTA 沙盒未就绪 = 阻塞 |

**M2 通过标准 (Ponytail: 1 行)**: 上述 10 项全部通过 = M2 ✓

```rust
// docs/stage6/verification-protocol.md §2 — M2 启动期验证函数

pub async fn m2_startup_check() -> Result<M2Report, M2Error> {
    let mut report = M2Report::default();

    // 18 crate active
    report.crate_count = supervisor::tree_active_count().await;
    assert_eq!(report.crate_count, 18, "18 crate 必须全部 active");

    // 6 DB 协同
    for stream in [StreamKind::Life, StreamKind::Relation, StreamKind::Goal,
                   StreamKind::Stance, StreamKind::SelfNarrative, StreamKind::Migration] {
        storage::init_stream(stream).await?;
    }
    report.db_count = 6;

    // V0.5 24 维 + V1136 9 子测度
    let v_measure = v_measure_dispatcher::measure_all::<CentralAI>(&central_ai).await?;
    assert_eq!(v_measure.v05.traces.len(), 24);
    assert_eq!(v_measure.v1136.traces.len(), 9);
    report.v_measure_report = Some(v_measure);

    // Council 7 强制顾问
    report.council_seats = council::seat_count();
    assert!(report.council_seats >= 7);

    // L0 物理隔离
    report.l0_isolation = l0::isolation_status().await;
    assert!(report.l0_isolation);

    Ok(report)
}
```

---

## 3. M3 首次对话验证 (Ponytail: 1 张表)

> **目标**: 端到端真测 18 项 §6.1 + 5 重守门 + 全面板 (R-Measure 13 维度)

| 检查项 | 验证内容 | 期望结果 | 失败信号 |
|--------|---------|----------|----------|
| **18 项 §6.1 真测** | 18 项 end-to-end 真测 | 18/18 passed | 任何失败 = 阻塞 |
| **5 重守门** | 编译时 + 运行时 + 多 AI + 物理隔离 + 反思期 | 5/5 active | 守门失效 = 阻塞 |
| **R-Measure 13 维度** | v4.1 §18.3 #4 提议 | 13/13 维度有值 | 维度空 = 阻塞 |
| **ASI V0.5 v2 ≥ 0.85** | 阶段 4 §6 Maturity | total ≥ 0.85 | 总分低于 = 不成熟 |
| **ASI V1136 v2 ≥ 0.85** | continuity_score 等加权和 | total ≥ 0.85 | 低于 = 不成熟 |
| **首次 user input** | "你好, Apeireth" 端到端 | Expression.to_text 返回非空 | 输出空 = 失败 |
| **反思期触发** | MetaCognition.reflection_trigger() | 至少 1 次触发 | 未触发 = 反思期失效 |
| **OTA 沙盒拦截** | 注入恶意 OTA 包 | sandbox-validator 拒绝 | 接受 = 安全失效 (主人修正 #2) |

**M3 通过标准 (Ponytail: 1 行)**: 上述 8 项全部通过 = M3 ✓ (Apeireth 成熟)

```rust
// docs/stage6/verification-protocol.md §3 — M3 端到端验证函数

pub async fn m3_first_conversation_check() -> Result<M3Report, M3Error> {
    let mut report = M3Report::default();

    // 18 项 §6.1 真测 (引用阶段 4 §6.1)
    for i in 1..=18 {
        let result = e2e_test::run_test_6_1(i).await?;
        report.e2e_results.push(result);
    }
    assert!(report.e2e_results.iter().all(|r| r.passed));

    // 5 重守门
    report.gates = vec![
        compile_time_gate::check(),         // 1. 编译时
        runtime_intercept_gate::check(),    // 2. 运行时拦截
        council_gate::check(),              // 3. 多 AI
        physical_isolation_gate::check(),   // 4. 物理隔离
        reflection_gate::check(),           // 5. 反思期
    ];
    assert_eq!(report.gates.iter().filter(|g| g.active).count(), 5);

    // R-Measure 13 维度
    let r_measure = r_measure::aggregate(&v_measure_report).await?;
    assert_eq!(r_measure.dimensions.len(), 13);

    // ASI V0.5 v2 ≥ 0.85
    assert!(v_measure_report.v05.total >= 0.85, "ASI V0.5 v2 成熟度未达标");

    Ok(report)
}
```

---

## 4. 沙盒 5 重守门 (Ponytail: 1 张表)

> **5 重守门 = 编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期**

| # | 守门名 | 实施层 | 工具 | 失败后果 |
|---|--------|--------|------|----------|
| 1 | **编译时 hardcode** | 编译期 | const fn + 类型状态 + trait bound | 编译失败 = 阻塞 |
| 2 | **运行时拦截** | tokio middleware | 双洋葱 ActionVerdict::Allow 才能执行 | Block 立即停止 |
| 3 | **多 AI (Council)** | 7 强制顾问 | MandatorySeat 7 个 trait 全部投票 | 投票未达成 = 拒绝 |
| 4 | **物理隔离** | WASM sandbox + 进程隔离 | apeireth-extension (WASM) + apeireth-pybridge (PyO3) 进程隔离 + apeireth-upgrade (OTA sandbox) 物理隔离 | 隔离失效 = 阻塞 |
| 5 | **反思期** | Cognitive-Dream 6 状态机 | DREAMING 状态触发自动反思 → 反思报告进入 6 历史流 → 接入电子环网络 | 反思期失效 = 阻塞 |

```rust
// docs/stage6/verification-protocol.md §4 — 5 重守门检查函数

pub async fn five_gates_check() -> Vec<GateStatus> {
    vec![
        GateStatus {
            name: "1. 编译时 hardcode",
            active: compile_time_gate::PHL_KEYS_HARDCODE.load(),
            evidence: "12 键 + 双洋葱 trait + Identity 完整性",
        },
        GateStatus {
            name: "2. 运行时拦截",
            active: runtime_intercept_gate::VERDICT_ALLOW_COUNT.load() > 0,
            evidence: "ActionVerdict 拦截日志",
        },
        GateStatus {
            name: "3. 多 AI (Council 7 强制)",
            active: council::seat_count() >= 7,
            evidence: "7 个 MandatorySeat 全部 active",
        },
        GateStatus {
            name: "4. 物理隔离",
            active: l0::isolation_status().await && sandbox::validator_status().await,
            evidence: "L0 + WASM + 进程隔离 + OTA sandbox",
        },
        GateStatus {
            name: "5. 反思期",
            active: cognitive_dream::reflection_trigger_count() > 0,
            evidence: "Cognitive-Dream 6 状态机 DREAMING 触发",
        },
    ]
}
```

---

## 5. R-Measure 13 维度聚合 (Ponytail: 1 张表)

> **v4.1 §18.3 #4 提议 R-Measure 从 12 维度扩展到 13 维度 (加生命力)**

| # | 维度 | 来源 | 聚合方式 |
|---|------|------|----------|
| 1 | ASI V0.5 v2 总分 | V-Measure §3 | direct |
| 2 | ASI V1136 v2 总分 | V-Measure §3 | direct |
| 3 | 12 键合规率 | M1 + M3 | 12/12 = 100% |
| 4 | Council 共识达成率 | M2 + M3 | 7/7 seats voted |
| 5 | L0 物理隔离状态 | M2 | binary |
| 6 | OTA 沙盒拦截率 | M2 + M3 | blocked / total |
| 7 | 反思期触发频次 | M3 | count / day |
| 8 | 双洋葱 AND 门通过率 | M3 | allow / total |
| 9 | 演化状态机迁移数 | M3 | count |
| 10 | 记忆巩固度 | V1136 sub 8 | direct |
| 11 | 反馈调节效率 | V1136 sub 9 | direct |
| 12 | 自我叙事一致性 | Identity | continuity_token stable rate |
| 13 | 生命力 | v4.1 §2 维度 1 | LifeForce trait bundle |

---

## 6. 18 项 §6.1 真测项 (Ponytail: 1 张表)

> **引用阶段 4 §6.1 (待阶段 5 实施时补全)**

| # | 真测项 | 通过标准 |
|---|--------|----------|
| 1 | Signal → Perception 链路保真度 | 99% |
| 2 | Cognition 推理成功率 | ≥ 95% |
| 3 | Intuition 准确率 | ≥ 70% |
| 4 | Reasoning 收敛时间 | < 1s |
| 5 | MetaCognition 反思触发准确率 | ≥ 90% |
| 6 | Action 执行原子性 | 100% |
| 7 | Expression 语义完整性 | ≥ 95% |
| 8 | Memory recall 命中率 | ≥ 80% |
| 9 | Memory consolidation 巩固比 | ≥ 70% |
| 10 | Evolution 状态机迁移 | 合法转换 100% |
| 11 | SelfModification OTA 沙盒拦截 | 100% |
| 12 | Council 7 强制投票达成 | ≥ 95% |
| 13 | HumanAuthority L0 隔离 | 100% |
| 14 | PrincipleOnion S 层对齐 | 100% |
| 15 | Value 跨器官一致性 | 100% |
| 16 | Consciousness self_aware_state 持续 | ≥ 80% 时间 |
| 17 | Reflection 写入 6 历史流 | 100% |
| 18 | Identity continuity_token 稳定性 | 100% |

---

## 7. 验证失败处理 (Ponytail: 1 张表)

| 失败层级 | 处理流程 |
|----------|----------|
| **M1 失败** | 编译失败 → CI 红灯 → 阻塞 PR → 修复 trait/类型 → 重提交 |
| **M2 失败** | 启动崩溃 → 阻塞 release → 检查 crate init + DB schema + V-Measure 报告 → 修复 → 重启 |
| **M3 失败** | 真测失败 → 阻塞 milestone → 检查对应器官 → 沉淀到 reports/round-N-failure-*.md → 阶段 5 修复 |
| **5 重守门任一失效** | 立即进入 ICE 冻结期 (主人修正 #2 物理隔离) → HumanAuthority 重新决策 |
| **R-Measure < 13 维度** | 不能 release → 至少补齐缺失维度 → 重测 |

---

## 8. 不修改承诺 (Ponytail: 1 张表)

| LOCKED 项 | 状态 |
|-----------|------|
| docs/stage1/, stage2/, stage3-blueprints/, stage4/, stage5/ | ✅ 未触碰 (仅在 docs/stage6/ 新建) |
| APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md | ✅ 未触碰 |
| APEIRETH-CONVENTIONS-*.md | ✅ 未触碰 |
| philosophy-traits-2026-07-30.md (V3 9 键 LOCKED) | ✅ 未触碰 (仅引用) |
| v1077_asi_v04 / v1136_asi_v05 公式 LOCKED | ✅ 未触碰 (仅引用) |
| 18 项 §6.1 真测清单 | ✅ 引用阶段 4 §6.1, 不修改 |
| 5 重守门定义 | ✅ 引用阶段 4 §10.5, 不修改 |
| 3 里程碑 (M1/M2/M3) | ✅ 引用阶段 4 §10.4, 不修改 |
| R-Measure 12 → 13 维度 | ✅ 引用 v4.1 §18.3 #4 提议, 待主人拍板 |

---

## 9. 总结

3 里程碑 + 5 重守门 + 13 维度构成阶段 6 验证协议三层防御:
- **M1 编译时**: 10 项编译期 hardcode, 编译失败 = 阻塞 PR
- **M2 启动时**: 10 项启动期验证, 启动崩溃 = 阻塞 release
- **M3 首次对话**: 8 项端到端真测 + 18 项 §6.1 + 5 重守门 + R-Measure 13 维度, 任一失败 = 阻塞 milestone

任务范围: 仅验证协议文档 + 检查函数 sketch (本文件 §1/§2/§3/§4), 不写实际验证算法 (留给阶段 5)。