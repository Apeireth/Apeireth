# 阶段 2 决策：决策系统完整衔接 (2026-07-30)

> **范围**: R14 决策系统完整衔接 (阶段 2 补充, 阶段 1 §4 对齐)
> **依据**: 阶段 1 §4 决策系统 + §5 权限矩阵 + §10 智囊团 + 阶段 2 §10 + §11 升级
> **配套文档**: `stage2-decisions-council-impl.md` + `stage2-decisions-upgrade-impl.md`

---

## 1. 决策流程总图

```
外部输入
  │
  ▼
[Phase 1] 决策前
  ├─ 原则洋葱检查 (E/S/A/M/O 5 层)
  │    ├─ E 层 (存在): 编译时 hardcode + 运行时拦截 + 多 AI 一致
  │    ├─ S 层 (价值): 智囊团强制审核
  │    └─ O 层 (操作): 9 键 + 5 项不假装
  │
  ├─ 权限矩阵检查 (Layer 0-6 洋葱权限)
  │    └─ 物理多签兜底 (AI/人/密钥三维)
  │
  ▼
[Phase 2] 决策中 (主 AI 主权)
  │
  ├─ 主 AI 生成初步决策
  │
  ├─ 智囊团咨询 (7 强制 + 动态 N)
  │    ├─ 拟人化辩论 (3 轮)
  │    └─ Synthesis 加权综合
  │
  ├─ 按住检查 (HoldTrigger)
  │    ├─ 强反对 ≥30% → 强暂停 (60s 裁决超时)
  │    ├─ 一致反对 → 紧急暂停 (警报)
  │    └─ 0 反对 → 继续
  │
  ▼
[Phase 3] 决策后
  │
  ├─ 执行决策
  │    ├─ 物理多签 (Layer 4+)
  │    ├─ A/M 层沉淀 (反思 + 经验)
  │    └─ O 层规则更新
  │
  ├─ 反思期审计 (ReflectionAuditor)
  │    └─ 事后审视 + 智囊团追溯
  │
  └─ A/M 层 promotion (温度分层 🔥/🌡️/❄️)
```

---

## 2. 决策执行顺序 (3 阶段)

### Phase 1: 决策前 (守门)

```rust
pub async fn before_decision(action: &Action) -> Result<(), PrincipleViolation> {
    // 1. 原则洋葱 E 层 (不可违背)
    e_layer_checker.check(action)?;
    
    // 2. 原则洋葱 S 层 (价值观)
    s_layer_auditor.audit(action)?;
    
    // 3. 权限矩阵 Layer N (看操作)
    permission_matrix.check(action)?;
    
    // 4. 9 键守门 (O 层)
    philosophy_checker.check(&action.claim)?;
    
    // 5. 5 项不假装 (V1138)
    no_pretend_guard.check(&action.claim)?;
    
    Ok(())
}
```

### Phase 2: 决策中 (主 AI 主权)

```rust
pub async fn decide(situation: &Situation) -> Decision {
    // 1. 主 AI 生成初步决策
    let decision = main_ai.generate_decision(situation).await?;
    
    // 2. 智囊团咨询 (拟人化辩论)
    let opinions = council.debate(&decision).await;  // 3 轮辩论
    
    // 3. 按住检查
    let hold = evaluate_hold(&opinions, &hold_trigger);
    match hold {
        HoldAction::None => {},  // 继续
        HoldAction::Pause { reason } => {
            return Decision::Pending { reason };  // 等待人类裁决
        }
        HoldAction::StrongPause { reason, opinions } => {
            return Decision::Hold { reason, opinions };  // 强暂停
        }
        HoldAction::EmergencyPause { reason, opinions } => {
            return Decision::Emergency { reason, opinions };  // 紧急
        }
    }
    
    // 4. Synthesis 加权综合
    let synthesized = council.synthesize(&opinions);
    
    // 5. 主 AI 根据 synthesis 调整
    main_ai.refine_decision(decision, &synthesized).await
}
```

### Phase 3: 决策后 (执行 + 反思)

```rust
pub async fn after_decision(decision: &Decision, outcome: &Outcome) {
    // 1. 物理多签 (Layer 4+)
    if decision.layer >= PermissionLayer::L4 {
        let sigs = multisig_collector.collect(decision).await?;
        assert!(sigs.is_met());
    }
    
    // 2. 执行
    executor.execute(decision).await?;
    
    // 3. 反思期审计
    reflection_auditor.audit_outcome(decision, outcome).await;
    
    // 4. A 层沉淀 (经验)
    if outcome.is_novel() {
        experience_store.write_experience(decision, outcome).await?;
    }
    
    // 5. M 层 promotion (温度分层)
    let temperature = compute_temperature(outcome);
    methodology_store.promote(decision, outcome, temperature).await?;
}
```

---

## 3. Sovereignty trait 实现

```rust
#[async_trait]
pub trait Sovereignty: Send + Sync {
    /// 决策接口 (3 阶段)
    async fn decide(&self, situation: &Situation) -> Decision;
    
    /// 智囊团强烈反对暂停
    async fn decide_with_council_check(&self, situation: &Situation) -> Decision;
    
    /// 物理多签兜底
    async fn multi_sig_finalize(&self, decision: Decision) -> Result<Action, OverrideError>;
    
    /// 自我升级意图生成
    fn upgrade_intent(&self, ctx: &UpgradeContext) -> Vec<UpgradeIntent>;
}
```

---

## 4. 决策冲突仲裁 (E > S > A > M > O)

```rust
pub enum PrincipleLayer {
    Existence,    // E 层
    Spirit,       // S 层
    Accumulation, // A 层
    Methodology,  // M 层
    Operational,  // O 层
}

impl PrincipleLayer {
    pub fn rank(&self) -> u8 {
        match self {
            Self::Existence => 5,
            Self::Spirit => 4,
            Self::Accumulation => 3,
            Self::Methodology => 2,
            Self::Operational => 1,
        }
    }
}
```

**冲突规则**:
```
E 层 > S 层 > A 层 > M 层 > O 层 (高优先级覆盖低优先级)
同层冲突: 后入胜 (LIFO)
E 层永不修改 (除非物理多签 + 多人 + 多 AI)
S 层修改需要智囊团强制审核
A/M 层修改走权限矩阵
O 层修改 AI 自己可改
```

---

## 5. 阶段 1 §4 完整对齐

| 阶段 1 §4 内容 | 阶段 2 对应 | 状态 |
|---------------|-----------|------|
| 主 AI 主权 | §10 + §12 + 本文档 | ✅ |
| 智囊团咨询 | §10 council-impl | ✅ |
| 物理多签兜底 | §11 upgrade-impl (d58a775 修正后) | ✅ |
| 智囊团强烈反对暂停 | §10 HoldTrigger | ✅ |
| 人类裁决 (60s 超时) | §10 HoldState::StrongPaused | ✅ |
| 一致反对紧急暂停 | §10 HoldAction::EmergencyPause | ✅ |
| A/M 层反思沉淀 | §10 + §12 ReflectionAuditor | ✅ |

---

_主哲学 anchor 6 个全贯穿. 决策系统完整衔接已沉淀. 下一步: 阶段 3 (画图纸)._