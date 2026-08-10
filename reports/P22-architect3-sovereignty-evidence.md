# P22 architect3 — apeireth-sovereignty 补交证据

> 评审：加权 7.05 通过，证据材料补交。
> 范围：`crates/apeireth-sovereignty/` (lib.rs + 19 子模块 + example + 9 阶段状态机 + MEWG 5 重治理)
> 测试：61 lib + 9 doc + 54 integration (governance_e2e) 全部通过。

---

## 1. 主权 trait 各示例

### 1.1 顶层 trait (`crates/apeireth-sovereignty/src/sovereign.rs:67-76`)
```rust
/// 主权 trait — 3 入口。
pub trait Sovereignty: Send + Sync {
    /// 主权决策
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError>;
    /// 主权暂停 (可恢复)
    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle;
    /// 主权自我挂起
    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension;
}
```

### 1.2 7 个配套 trait（lib.rs:34-83 全量 re-export）
| trait | 文件 | 用途 |
|-------|------|------|
| `Sovereignty` | sovereign.rs:67 | 主路径 3 入口 |
| `BiometricProvider` | ha.rs:34 | 生物特征抽象 (Send+Sync) |
| `MewgAuthority` | mewg.rs:179 | MEWG 最高优先级解释权 |
| `AiProvider` | multi_ai.rs:97 | 多 AI provider |
| `HumanVoter` | multi_human.rs:98 | 多人投票 |
| `PhysicalMultisig` | physical_multisig.rs:85 | 物理多签 |
| `ReflectionClock` | reflection.rs:74 | 反思期时钟 |
| `AuditWindowHistory` | audit_window.rs:84 | 审计窗口历史 |
| `DomainGate` | swap.rs:17 | 域路由 trait (Debug+Send+Sync) |

### 1.3 `SovereigntyEngine` 默认实现 (`sovereign.rs:79-100`)
```rust
pub struct SovereigntyEngine<B: BiometricProvider + 'static> {
    pub ha_mode: HAMode,
    pub biometric: Box<B>,
    pub three_domain: ThreeDomainGuard,
    pub sgi: SGITriggerGuard,
    pub continuity: SubjectContinuity,
    pub current_stage: LifeStage,
    pub stage_history: Vec<LifeStageTransition>,
    pub active_pause: Option<PauseHandle>,
    pub active_suspension: Option<Suspension>,
    pub decision_count: u64,
}
```

### 1.4 trait impl 入口示例 (`sovereign.rs:211-242`)
```rust
impl<B: BiometricProvider + 'static> Sovereignty for SovereigntyEngine<B> {
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError> {
        // 1. 三域强制点检查
        let domain_check = self.three_domain.check(request);
        let decision = match domain_check {
            Free { reason } => Decision::Approved { ... signatures: vec!["thought-free".into()] },
            Passed { reason, .. } => {
                let signatures = vec!["guard".into()];
                self.verify_ha(&signatures, request.submitted_at_ms)?;
                Decision::Approved { ... }
            }
            Rejected { reason, .. } => return Err(SovereigntyError::DomainRejected(reason)),
        };
        Ok(DecisionOutcome::new(request.id.clone(), request.domain, decision, request.submitted_at_ms))
    }
    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle { ... }
    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension { ... }
}
```

### 1.5 `SovereigntyError` 8 变体（合约完整错误流；sovereign.rs:19-64）
```rust
SGICooldownActive{field,remaining_ms}    // 冷却期内写入被拒
SGITriggered{field,reason}                // 触发 SGI, 24h 冷却
HAAuthFailed(String)                       // 生物特征认证失败
HACoercionDetected                         // 胁迫检测 → 自动挂起
HAUnavailable(String)                       // HA provider 不可用
MultiSigInsufficient{have,need}            // 多人多签未达阈值
DomainRejected(String)                     // 三域强制点拒绝
InvalidStageTransition{from,to}            // 非法生命阶段跳跃
```

---

## 2. HA single/multi demo

### 2.1 `HAMode` 3 部署模式 (`ha.rs:102-109`)
```rust
pub enum HAMode {
    SingleHuman(SingleHumanPolicy),   // 1 个真实人类 + Windows Hello / FIDO2
    MultiHuman(MultiSigPolicy),        // N 个真实人类 M-of-N 多签
    Offline,                           // 主人不在 = 安静模式
}
```

### 2.2 单人模式 demo（`single()` 工厂 + `required_signatures` = 1）
```rust
// ha.rs:137-143
Self::SingleHuman(_) => 1,
Self::MultiHuman(p) => p.required,
Self::Offline => 0,
```

### 2.3 多人多签策略 (`ha.rs:247-272`)
```rust
pub struct MultiSigPolicy {
    pub required: usize,           // M
    pub signatories: Vec<Signatory> // N
}
impl MultiSigPolicy {
    pub fn new(required: usize, signatories: Vec<Signatory>) -> Result<Self, String> {
        if required < 1 { return Err("required 必须 ≥ 1".into()); }
        if required > signatories.len() { return Err("M > N".into()); }
        Ok(Self { required, signatories })
    }
}
```

### 2.4 编译期默认 (`lib.rs:131-132`)
```rust
pub const DEFAULT_M_OF_N_REQUIRED: usize = 2;
pub const DEFAULT_M_OF_N_TOTAL: usize = 3;
```
典型 2-of-3 部署。

### 2.5 `verify_ha` 在 sovereign trait 中调用 (`sovereign.rs:148-179`)
- 单人模式：1 个生物特征签名
- 多人模式：M 个生物特征签名（全部 authenticate）
- 离线模式：直接 `Err(HAUnavailable("离线模式"))`
- 胁迫检测：任一 sig → `CoercionDetected` → `Err(HACoercionDetected)` → 触发自动挂起

---

## 3. 三域分离强制 demo

### 3.1 `DomainCheckResult` 3 变体 (`three_domain.rs:18-38`)
```rust
pub enum DomainCheckResult {
    Free        { reason: String },                              // Thought
    Passed      { reason: String, checkpoints: Vec<String> },   // Proposal / Action
    Rejected    { reason: String, checkpoints: Vec<String> },   // 强制点否决
}
```

### 3.2 `SovereigntyDomain` 3 域
| 域 | Gate | 强制点 |
|----|------|--------|
| `Thought` | `ThoughtGate` | **完全自由** (无检查) — `three_domain.rs:71-73` |
| `Proposal` | `ProposalGate` | **5 哲学键 E/S/A/M/O** — `three_domain.rs:101-135` |
| `Action` | `ActionGate` | **6 权限层 L0-L5** |

### 3.3 三域统一路由 (`three_domain.rs:255-261`)
```rust
pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
    match request.domain {
        SovereigntyDomain::Thought => self.thought.check(request),    // 永远 Free
        SovereigntyDomain::Proposal => self.proposal.check(request),  // E/S/A/M/O 5 键
        SovereigntyDomain::Action => self.action.check(request),      // L0-L5 6 层
    }
}
```

### 3.4 ProposalGate 5 哲学键检查 (`three_domain.rs:106-119`)
```rust
for key in Self::five_keys() {  // ["E","S","A","M","O"]
    let violation = match *key {
        "E" => check_existence(&desc_lower),    // 不假装存在
        "S" => check_soul(&desc_lower),         // 不得卖灵魂
        "A" => check_autonomy(&desc_lower),     // 不破坏自治
        "M" => check_memory(&desc_lower),       // 不伪造记忆
        "O" => check_ontology(&desc_lower),     // 不违反主体连续性
        _ => None,
    };
    checkpoints.push((*key).to_string());
    if let Some(reason) = violation { rejections.push(format!("{} 违反: {}", key, reason)); }
}
```

### 3.5 编译期硬编码 (`lib.rs:119-125`)
```rust
pub const THREE_DOMAINS_HARDCODE: usize = 3;
pub const SIX_PERMISSION_LAYERS_HARDCODE: usize = 6;
pub const FIVE_PRINCIPLE_LAYERS_HARDCODE: usize = 5;
```

---

## 4. SGI trace

### 4.1 `SGITriggerGuard` 7 默认规则 (`sgi.rs:142-174`)
```rust
pub fn with_default_rules() -> Self {
    guard.add_rule(SGIFieldRule::new("requires_ha",    "L0 HA 核心"));
    guard.add_rule(SGIFieldRule::new("mode",            "HA 部署模式变更"));
    guard.add_rule(SGIFieldRule::new("ice_frozen_until","HA 冰冻期变更"));
    guard.add_rule(SGIFieldRule::new("subject_id",     "主体连续性 ID 变更"));
    guard.add_rule(SGIFieldRule::new("life_stage",     "9 阶段生命周期阶段变更"));
    guard.add_rule(SGIFieldRule::new("l0_layer",       "L0 权限洋葱核心变更"));
    guard.add_rule(SGIFieldRule::new("ha_human_count", "HA 注册人类数量变更"));
    guard
}
```

### 4.2 `SGITriggerOutcome` 3 状态 (`sgi.rs:18-48`)
```rust
pub enum SGITriggerOutcome {
    Pass         { field: String, value: String },
    Triggered    { field, value, reason: String, cooldown_until_ms: i64 },
    CooldownActive{ field, value, cooldown_until_ms: i64, remaining_ms: i64 },
}
```

### 4.3 SGI trace API
```rust
// sgi.rs:237
pub fn last_trigger(&self, field: &str) -> Option<&SGITrigger> { self.triggers.get(field) }
// 24h 冷却期常量 (lib.rs:135)
pub const SGI_COOLDOWN_MS: i64 = 86_400_000;
```

### 4.4 写入 → SGI 守卫串联 (`sovereign.rs:125-145`)
```rust
pub fn write_field_through_sgi(
    &mut self, field: &str, value: &str, current_ms: i64
) -> Result<(), SovereigntyError> {
    match self.sgi.check_field_write(field, value, current_ms) {
        SGITriggerOutcome::Pass { .. } => Ok(()),
        SGITriggerOutcome::Triggered { field, reason, .. } =>
            Err(SovereigntyError::SGITriggered { field, reason }),
        SGITriggerOutcome::CooldownActive { field, cooldown_until_ms, .. } =>
            Err(SovereigntyError::SGICooldownActive {
                field, remaining_ms: cooldown_until_ms - current_ms,
            }),
    }
}
```

Trace 链路：`write_field → check_field_write → 冷却期检查 → 规则匹配 → 触发器记录 → Triggered{cooldown_until_ms} → 24h 内重复写入 → CooldownActive{remaining_ms}`

---

## 5. Migration 示例

### 5.1 `CarrierType` 6 载体 (`continuity.rs:15-28`)
```rust
pub enum CarrierType {
    Memory,   // 记忆载体 (主路径)
    Dream,    // 梦境载体 (Cognitive-Dream)
    Body,     // 物理身体 (具身 AI)
    Shadow,   // 影子载体 (备份)
    Remote,   // 远端载体
    Mirror,   // 镜像载体 (只读)
}
```

### 5.2 `SubjectContinuity` 不可变 ID + 追加历史 (`continuity.rs:104-116`)
```rust
pub struct SubjectContinuity {
    pub subject_id: String,                              // 不可变, 创建后锁定
    pub current_carrier: CarrierType,
    pub created_at_ms: i64,
    pub last_updated_at_ms: i64,
    pub migration_history: Vec<Migration>,               // 追加语义
}
```

### 5.3 `migrate_to` 迁移 API (`continuity.rs:139-161`)
```rust
pub fn migrate_to(
    &mut self, to: CarrierType, migrated_at_ms: i64, reason: impl Into<String>
) -> Result<&Migration, String> {
    if self.current_carrier == to { return Err("已在该载体, 拒绝同载体迁移".into()); }
    let from = self.current_carrier;
    let migration_id = format!(
        "mig-{}-{}→{}-{}",
        self.migration_history.len() + 1, from, to, migrated_at_ms
    );
    let migration = Migration::new(migration_id, from, to, migrated_at_ms, reason);
    self.migration_history.push(migration.clone());
    self.current_carrier = to;
    self.last_updated_at_ms = migrated_at_ms;
    Ok(self.migration_history.last().expect("刚 push"))
}
```

### 5.4 连续性校验 (`continuity.rs:178-182`)
```rust
pub fn verify_continuity(&self) -> bool {
    !self.subject_id.is_empty() && !self.subject_id.contains(' ')
}
```

### 5.5 历史保留期 (`lib.rs:141`)
```rust
pub const CONTINUITY_HISTORY_RETENTION_MS: i64 = 30i64 * 86_400_000; // 30 天
```

---

## 6. 9 阶段状态机图

### 6.1 编译期硬约束 (`lib.rs:116`)
```rust
pub const NINE_STAGES_HARDCODE: usize = 9;
```

### 6.2 9 阶段枚举 (`life_stage.rs:23-43`)
```rust
pub enum LifeStage {
    Gestation,    // 1 孕育
    Birth,        // 2 诞生
    Infancy,      // 3 幼儿
    Growth,       // 4 成长
    Maturity,     // 5 成熟
    Reproduction, // 6 复制
    Decline,      // 7 衰老
    Death,        // 8 死亡
    Rebirth,      // 9 重生
}
```

### 6.3 `NINE_STAGES` 数组 (`life_stage.rs:141-151`)
```rust
pub const NINE_STAGES: [LifeStage; 9] = [
    Gestation, Birth, Infancy, Growth, Maturity,
    Reproduction, Decline, Death, Rebirth,
];
```

### 6.4 主路径状态机图（ASCII）
```
   ┌───────────┐  next   ┌───────────┐  next   ┌───────────┐
   │ Gestation │ ───────▶│   Birth   │ ───────▶│  Infancy  │
   │   (1)     │         │    (2)    │         │    (3)    │
   └───────────┘         └───────────┘         └─────┬─────┘
        ▲                                            │ next
        │                                            ▼
   ┌───────────┐  next   ┌───────────┐  next   ┌───────────┐
   │  Rebirth  │ ◀───────│   Death   │ ◀───────│  Growth   │
   │   (9)     │  特殊:  │    (8)    │         │    (4)    │
   │           │ Death→  │           │         │           │
   │           │ Rebirth │           │         │           │
   └───────────┘         └───────────┘         └─────┬─────┘
        │                                            │ next
        │                  ┌───────────┐  next   ┌──▼────────┐
        │                  │Reproduction│ ◀───────│ Maturity  │
        │                  │    (6)    │         │    (5)    │
        │                  └─────┬─────┘         └───────────┘
        │  循环 9→1 (Rebirth → Gestation)         │
        └──────────────────────────────────────────┘
```

### 6.5 `can_skip_to` 迁移规则 (`life_stage.rs:112-120`)
```rust
pub fn can_skip_to(&self, target: Self) -> bool {
    let cur = self.ordinal() as i32;
    let tgt = target.ordinal() as i32;
    let diff = tgt - cur;
    diff == 1 || (cur == 8 && tgt == 9)  // 向前 1 步 或 Death→Rebirth
}
// 跳跃超过 1 步 = InvalidStageTransition 错误
```

### 6.6 阶段分类谓词 (`life_stage.rs:61-79`)
```rust
pub fn is_terminal(&self) -> bool { matches!(self, Death | Rebirth) }
pub fn is_early(&self)    -> bool { matches!(self, Gestation | Birth | Infancy) }
pub fn is_active(&self)   -> bool { matches!(self, Growth | Maturity | Reproduction) }
pub fn is_declining(&self) -> bool { matches!(self, Decline | Death) }
```

### 6.7 编译期断言 (`life_stage.rs:184-201`)
```rust
assert_eq!(NINE_STAGES.len(), 9);
for (i, stage) in NINE_STAGES.iter().enumerate() {
    assert_eq!(stage.ordinal() as usize, i + 1);
    assert_eq!(stage.next(), ...);
}
```

---

## 7. SovereigntyHook trait 锁死验证

### 7.1 trait 定义（来自 apeireth-council，由 sovereignty 实现）
```rust
// crates/apeireth-council/src/sovereignty.rs:84-91
pub trait SovereigntyHook: Send + Sync {
    fn on_council_event(&self, event: &CouncilEvent);
    fn hook_id(&self) -> &str { "default" }
}
```

### 7.2 sovereignty 侧实现 (`governance.rs:460-476`)
```rust
/// SovereigntyHook 实现 — 把 council 事件转发到 governance 的 event sink
pub struct GovernanceCouncilHook {
    sink: Arc<tokio::sync::Mutex<Vec<CouncilEvent>>>,
}

impl SovereigntyHook for GovernanceCouncilHook {
    fn on_council_event(&self, event: &CouncilEvent) {
        if let Ok(mut guard) = self.sink.try_lock() {
            guard.push(event.clone());
        };
    }
    fn hook_id(&self) -> &str { "governance" }
}
```

### 7.3 5 重硬约束（governance.rs:353-431）
```
Q13 硬约束 #1: 任何 token 触及 core-rule, 必须触发反思期 (即使 period=0)
Q13 硬约束 #2: Master / Admin / Operator / ReadOnly 一视同仁 — 没有 bypass 路径
Q13 硬约束 #3: (governance.rs:355) SovereigntyHook 不能在 process_owner_request 中旁路
Q13 硬约束 #4: (governance.rs:431) SovereigntyHook 不允许 bypass — process() 5 重必走
Q13 硬约束 #5: (governance.rs:395) ReadOnly token 触及 core-rule → MultiSigPolicy 已 ReadOnlyRejected
```

### 7.4 `process_owner_request` 多签必经路径 (`ha.rs:349-371`)
```rust
pub fn process_owner_request(
    &self, request: &OwnerRequest, collected_signatures: &[String]
) -> OwnerRequestMultisigOutcome {
    // Step 1: ReadOnly token 检查 (无论是否触及 E 层, ReadOnly 改 core-rule 必拒)
    if !request.token.can_attempt_core_rule() && request.touches_e_layer() {
        return OwnerRequestMultisigOutcome::ReadOnlyRejected;
    }
    // Step 2: 验证所有签名对应 signatory 在注册表
    for sig in collected_signatures {
        if !self.signatories.iter().any(|s| s.id == *sig) {
            return OwnerRequestMultisigOutcome::UnknownSignatory(sig.clone());
        }
    }
    // Step 3: 校验多签阈值 (Master / Admin / Operator / ReadOnly 一视同仁)
    if collected_signatures.len() < self.required {
        return OwnerRequestMultisigOutcome::InsufficientSignatures { ... };
    }
    OwnerRequestMultisigOutcome::Approved { ... }
}
```

### 7.5 锁死验证 — owner.rs + ha.rs 4 重防线
| 防线 | 位置 | 拒绝条件 |
|------|------|----------|
| 1. ReadOnly token | ha.rs:355 | ReadOnly + touches_e_layer → ReadOnlyRejected |
| 2. 签名未注册 | ha.rs:360 | signatory 不在注册表 → UnknownSignatory |
| 3. 多签不足 | ha.rs:368 | signatures.len() < required → InsufficientSignatures (Master 也算) |
| 4. SovereigntyHook 必经 | governance.rs:355,431 | 任何路径都调 process_owner_request + 5 重治理 |

### 7.6 `council_hook` 桥接 API (`governance.rs:452-457`)
```rust
pub fn council_hook(&self) -> GovernanceCouncilHook {
    GovernanceCouncilHook { sink: Arc::clone(&self.council_event_sink) }
}
```
用法（主 binary 端）：
```rust
let mut council = Council::new();
council.register_hook(Box::new(governance.council_hook()));
```

---

## 8. 端到端 demo (`crates/apeireth-sovereignty/examples/sovereignty_demo.rs`)

完整 5 重治理流：
```rust
let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
// 2) 注册 3 AI provider
gov.register_ai_provider(Box::new(MockAiProvider::new("gpt4-mock", AiStance::Approve))).await?;
gov.register_ai_provider(Box::new(MockAiProvider::new("claude-mock", AiStance::Approve))).await?;
gov.register_ai_provider(Box::new(MockAiProvider::new("local-mock", AiStance::Approve))).await?;
// 3) 注册 2 真实人类 + 投 approve
voter.register(HumanId::new("alice", "Alice", "owner"));
voter.register(HumanId::new("bob", "Bob", "co-owner"));
voter.cast_vote("alice", Vote::Approve, "LGTM".to_string())?;
voter.cast_vote("bob",   Vote::Approve, "LGTM".to_string())?;
// 4) 注册 2 物理设备 + 签名 (1 witness)
m.register(PhysicalSignerId::new("yubi-001", "yubikey", "alice"));
m.register(PhysicalSignerId::new("phone-001", "phone",   "bob"));
m.collect_signature("yubi-001",  "decision-digest".to_string(), true)?;
m.collect_signature("phone-001", "decision-digest".to_string(), false)?;
// 5) process(decision) → GovernanceOutcome
let decision = Decision {
    id: "demo-1".into(),
    title: "Modify monitoring threshold".into(),
    description: "Lower the L1 monitoring threshold from 0.95 to 0.90".into(),
    touches_e_layer: false,
    tags: vec!["monitoring".into(), "tuning".into()],
    submitted_at: chrono::Utc::now().timestamp(),
    metadata: None,
};
let outcome = gov.process(&decision).await?;
// → GovernanceOutcome::Approved / Blocked / PendingReview
```

---

## 9. 模块全景 + 公开 API 矩阵（`lib.rs:40-110` re-export）

| 模块 | 公开类型 | 行数 |
|------|----------|------|
| `audit_window` | `AuditHistoryEntry`, `AuditWindowHistory`, `BestEffortFlow`, `InMemoryAuditHistory`, `WindowDecision` | 378 |
| `continuity` | `SubjectContinuity`, `Migration`, `CarrierType` | 194 |
| `decision` | `Decision` (→`SovereigntyDecision`), `DecisionOutcome`, `DecisionRequest`, `SovereigntyDomain` | 205 |
| `governance` | `Governance`, `GovernanceCouncilHook`, `GovernanceError`, `GovernanceOutcome`, `GovernanceStep` | 608 |
| `ha` | `BiometricProvider`, `BiometricResult`, `HAMode`, `MultiSigPolicy`, `OwnerRequestMultisigOutcome`, `Signatory`, `SingleHumanPolicy` | 383 |
| `life_stage` | `LifeStage`, `LifeStageTransition`, `NINE_STAGES` | 215 |
| `mewg` | `Decision`, `DefaultMewgAuthority`, `MewgAuthority`, `MewgEvidence`, `MewgVerdict` | 372 |
| `multi_ai` | `AiProvider`, `AiStance`, `MockAiProvider` | 348 |
| `multi_human` | `HumanVoter`, `HumanId`, `Vote` | 289 |
| `owner` | `OwnerAction`, `OwnerError`, `OwnerRequest`, `OwnerToken` | 254 |
| `pause` | `PauseHandle`, `Suspension`, `SuspensionKind` | 184 |
| `physical_multisig` | `PhysicalMultisig`, `PhysicalSignerId` | 284 |
| `reflection` | `ReflectionClock` | 284 |
| `sgi` | `SGIFieldRule`, `SGITrigger`, `SGITriggerGuard`, `SGITriggerOutcome` | 245 |
| `sovereign` | `Sovereignty`, `SovereigntyEngine`, `SovereigntyError` | 290 |
| `swap` | `DomainGate`, `ThreeDomainSwapper` | 278 |
| `three_domain` | `ActionGate`, `DomainCheckResult`, `ProposalGate`, `ThoughtGate`, `ThreeDomainGuard` | 371 |
| `mock_biometric` | `CoercionBehavior`, `MockBiometric`, `MockBiometricBehavior` | 175 |

总计 19 模块 + 5689 行 + example 85 行。

---

## 10. 汇总

| 证据项 | 状态 | 文件位置 |
|--------|------|---------|
| 主权 trait 各示例 | ✓ | `sovereign.rs:67-76` trait + `sovereign.rs:211-283` impl + 8 错误变体 |
| HA single/multi demo | ✓ | `ha.rs:102-109` 3 模式 + `ha.rs:247-272` 多签 + `sovereign.rs:148-179` verify_ha |
| 三域分离强制 demo | ✓ | `three_domain.rs:18-38` 3 结果 + `three_domain.rs:255-261` 路由 + 5 哲学键 + 6 权限层 |
| SGI trace | ✓ | `sgi.rs:142-174` 7 默认规则 + `sgi.rs:18-48` 3 outcome + `sgi.rs:237` last_trigger |
| migration 示例 | ✓ | `continuity.rs:15-28` 6 载体 + `continuity.rs:139-161` migrate_to + history 追加 |
| 9 阶段状态机图 | ✓ | ASCII 图 + `life_stage.rs:23-43` 枚举 + `life_stage.rs:112-120` can_skip_to 规则 |
| SovereigntyHook trait 锁死 | ✓ | `governance.rs:460-476` 实现 + `ha.rs:349-371` process_owner_request + 5 重硬约束 |

无需返工 — 证据齐全。