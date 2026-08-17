# 阶段 2 决策：自我升级实现 (2026-07-30)

> **范围**: R14 Rust 重写自我升级实现 (阶段 2 第十一项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: 阶段 1 §6 自我升级 (沙盒 + 洋葱测试矩阵) + B+E 架构 + 智囊团按住机制 + Erlang/OTP 双实例

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-upgrade-impl.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 11/12) |
| **决策** | **OTA 7 阶段 + 沙盒 + 双实例 + 自动回滚 + Layer 5 物理多签** |
| **候选 crate** | `apeireth-upgrade` (阶段 2 §3 已列) |

---

## 1. 决策总览

```
OTA 升级 7 阶段:
  Intent → Council → MultiSig → Sandbox → Switchover → Monitor → Done
  失败 → Rollback

5 大机制:
  1. UpgradeIntent (主 AI 生成升级意图, manifest)
  2. Council 审核 (智囊团强制, E-3 守门)
  3. 物理多签 (Layer 5 必走, 单人用密钥替代)
  4. Sandbox 验证 (完整集成测试 + 洋葱测试矩阵)
  5. 双实例 + 流量切换 (Erlang/OTP 标准做法)
```

---

## 2. OTA 升级 Pipeline

```rust
// apeireth-upgrade/src/pipeline.rs

pub enum UpgradeStage {
    Intent,        // 主 AI 生成升级意图
    Council,       // 智囊团审核
    MultiSig,      // 物理多签
    Sandbox,       // 沙盒验证
    Switchover,    // 流量切换
    Monitor,       // 监控 30 分钟
    Done,          // 完成
    Rollback,      // 回滚
}

pub struct UpgradePipeline {
    stage: Arc<RwLock<UpgradeStage>>,
    intent: Option<UpgradeIntent>,
    history: Vec<UpgradeRecord>,
}

impl UpgradePipeline {
    /// 主入口: 主 AI 提议升级
    pub async fn propose(&self, intent: UpgradeIntent) -> Result<(), UpgradeError> {
        self.set_stage(UpgradeStage::Intent);
        
        // 1. 智囊团审核 (强制)
        self.set_stage(UpgradeStage::Council);
        let opinions = self.council.evaluate(&intent).await;
        let hold = evaluate_hold(&opinions, &self.hold_trigger);
        if hold.is_strong() {
            return Err(UpgradeError::CouncilRejected(opinions));
        }
        
        // 2. 物理多签 (Layer 4+)
        self.set_stage(UpgradeStage::MultiSig);
        let sigs = self.collect_multisig(&intent).await?;
        if sigs < intent.required_sigs() {
            return Err(UpgradeError::MultisigInsufficient(sigs, intent.required_sigs()));
        }
        
        // 3. 沙盒验证 (洋葱测试矩阵)
        self.set_stage(UpgradeStage::Sandbox);
        self.run_sandbox_tests(&intent).await?;
        
        // 4. 流量切换 (双实例)
        self.set_stage(UpgradeStage::Switchover);
        self.switch_traffic(&intent).await?;
        
        // 5. 监控 30 分钟
        self.set_stage(UpgradeStage::Monitor);
        self.monitor_post_upgrade(&intent).await?;
        
        // 6. 完成
        self.set_stage(UpgradeStage::Done);
        self.archive_intent(&intent).await?;
        
        Ok(())
    }
}
```

---

## 3. UpgradeIntent (升级意图)

```rust
pub struct UpgradeIntent {
    pub id: Uuid,
    pub version_from: semver::Version,
    pub version_to: semver::Version,
    pub upgrade_type: UpgradeType,
    pub target: UpgradeTarget,
    pub manifest: UpgradeManifest,
    pub required_sigs: u32,
    pub created_at: i64,
    pub created_by: PrincipalId,  // 通常是主 AI
    pub reason: String,
}

pub enum UpgradeType {
    Patch,         // 修订版本 (主 AI 自主)
    Minor,         // 次版本 (智囊团审核)
    Major,         // 主版本 (物理多签)
    Emergency,     // 紧急 (低阈值)
}

pub enum UpgradeTarget {
    Crate(String),                  // 升级单个 crate
    Crates(Vec<String>),            // 升级多个 crate
    Binary,                         // 重编译整个 apeireth 二进制 (Layer 5)
    Plugin(String),                 // 升级 plugin
    Config,                         // 配置变更
}

pub struct UpgradeManifest {
    pub changes: Vec<Change>,
    pub rollback_plan: RollbackPlan,
    pub test_plan: TestPlan,
    pub risk_assessment: RiskAssessment,
}

pub struct Change {
    pub file: String,
    pub description: String,
    pub lines_added: u32,
    pub lines_removed: u32,
}

pub struct RollbackPlan {
    pub strategy: RollbackStrategy,
    pub estimated_time_seconds: u32,
    pub requires_data_migration: bool,
}

pub enum RollbackStrategy {
    ImmediateCutover,    // 立即切回旧版本 (Erlang/OTP)
    GradualRampdown,     // 渐进降流量
    ManualIntervention,  // 需人工介入
}

pub struct TestPlan {
    pub unit_tests: bool,
    pub integration_tests: bool,
    pub regression_tests: bool,
    pub fuzz_tests: Option<Duration>,
    pub formal_verify: bool,
    pub perf_benchmark: bool,
}

pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub mitigation: String,
    pub estimated_downtime: Duration,
}

pub enum RiskLevel {
    Low,      // PATCH, 几乎无风险
    Medium,   // MINOR, 有接口变更
    High,     // MAJOR, 破坏性
    Critical, // LAYER 5 二进制
}
```

---

## 4. Council 审核

```rust
// apeireth-upgrade/src/council_review.rs

pub struct CouncilReview {
    council: Arc<Council>,
    hold_trigger: HoldTrigger,
}

impl CouncilReview {
    pub async fn review(&self, intent: &UpgradeIntent) -> ReviewResult {
        // 1. 7 强制顾问评估
        let opinions = self.council.mandatory_evaluate(intent).await;
        
        // 2. 按住检查
        let hold = evaluate_hold(&opinions, &self.hold_trigger);
        
        match hold {
            HoldAction::None => {
                // 全部通过
                ReviewResult::Approved(opinions)
            }
            HoldAction::Pause { reason } => {
                ReviewResult::Paused { reason, opinions }
            }
            HoldAction::StrongPause { reason, opinions } => {
                // 强暂停, 等人类裁决
                ReviewResult::StrongPaused { reason, opinions }
            }
            HoldAction::EmergencyPause { reason, opinions } => {
                // 紧急暂停, 警报
                ReviewResult::Rejected { reason, opinions }
            }
        }
    }
}
```

**关键检查**:
- E-3 守门（不创造毁灭能力）
- V1121 fake-KPI 检测（不刷 KPI）
- V3 哲学 9 键（不假装）
- ASI 北极星导向（升级是否朝 ASI 方向）

---

## 5. 物理多签

```rust
// apeireth-upgrade/src/multisig.rs

pub struct MultiSigCollector {
    required_sigs: u32,
    collected: Arc<Mutex<Vec<Signature>>>,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub signer: PrincipalId,
    pub key_id: String,
    pub intent_hash: Hash,
    pub signed_at: i64,
    pub method: SignatureMethod,
}

pub enum SignatureMethod {
    YubiKey,
    PhoneTOTP,
    PasswordManager,
    Biometric,
    HardwareToken,
}

impl MultiSigCollector {
    pub async fn collect(&self, intent: &UpgradeIntent) -> Result<u32, MultiSigError> {
        let required = match intent.upgrade_type {
            UpgradeType::Patch => 1,    // 主 AI 1 票
            UpgradeType::Minor => 2,    // 主 AI + 智囊团 1 票
            UpgradeType::Major => 3,    // 主 AI + 智囊团 + 人
            UpgradeType::Emergency => 2, // 紧急: 主 AI + 智囊团 + 物理多签 1
            _ => 0,
        };
        
        // 等待签收
        let mut received = 0;
        let deadline = Instant::now() + self.timeout;
        while received < required && Instant::now() < deadline {
            // 等待签名
            tokio::time::sleep(Duration::from_secs(1)).await;
            received = self.collected.lock().await.len() as u32;
        }
        
        if received < required {
            return Err(MultiSigError::Timeout(required, received));
        }
        Ok(received)
    }
}
```

**签名要求矩阵** (单人 vs 多人场景, 阶段 1 §5.2/5.3 对齐):

| 升级类型 | 单人场景 | 多人场景 | 阶段 1 §5.2 对应 |
|---------|---------|---------|---------------|
| **PATCH** | AI × 1 | AI × 1 | Layer 1: AI × 1 |
| **MINOR** | AI × 1 + 密钥 × 1 | AI × 1 + 人 × 1 | Layer 2: 单人=AI+密钥, 多人=AI+人 |
| **MAJOR** | AI × 2 + 密钥 × 1 | AI × 2 + 人 × 1 | Layer 4: 单人=AI×2+密钥, 多人=AI×2+人 |
| **Emergency** | AI × 2 + 密钥 × 1 | AI × 2 + 人 × 1 | 介于 MINOR/MAJOR |
| **Layer 5 二进制** | AI × 3 + 密钥 × 3 | AI × 3 + 人 × 2 | Layer 5: 单人=AI×3+密钥×3, 多人=AI×3+人×2 |

**关键原则** (阶段 1 §5.4 密钥的语义):

```
密钥不是简单密码。
权限密钥 = "主人预先授权的意图":
  - "允许 AI 修改 memory store 任何内容" (高风险密钥)
  - "允许 AI 升级自己到任意版本" (核武器密钥)

密钥机制让单人在咖啡馆也能安全授权高风险操作, 不需要每次找人签。
```

**单人场景的工作流**:
1. 主人预先签发密钥 (YubiKey / 手机 TOTP / 密码管理器)
2. 主 AI 需要升级时, 加载密钥
3. 密钥 + 物理多签请求 → 升级执行
4. 密钥可销毁 (一次性) 或可重用 (限时)

**具体实现** (单人 vs 多人自适应):

```rust
// apeireth-keys/src/multisig.rs (新增)

pub struct SignatureRequirement {
    pub min_ai: u32,
    pub min_human: u32,
    pub min_key: u32,
    pub min_council: u32,
}

pub enum DeploymentMode {
    Solo,    // 单人 (密钥代替多人)
    Multi,   // 多人 (实际签)
}

impl SignatureRequirement {
    pub fn for_upgrade(upgrade_type: &UpgradeType, mode: DeploymentMode) -> Self {
        match upgrade_type {
            UpgradeType::Patch => SignatureRequirement {
                min_ai: 1, min_human: 0, min_key: 0, min_council: 0,
            },
            UpgradeType::Minor => match mode {
                DeploymentMode::Solo => SignatureRequirement {
                    min_ai: 1, min_human: 0, min_key: 1, min_council: 1,
                },
                DeploymentMode::Multi => SignatureRequirement {
                    min_ai: 1, min_human: 1, min_key: 0, min_council: 1,
                },
            },
            UpgradeType::Major => match mode {
                DeploymentMode::Solo => SignatureRequirement {
                    min_ai: 2, min_human: 0, min_key: 1, min_council: 1,
                },
                DeploymentMode::Multi => SignatureRequirement {
                    min_ai: 2, min_human: 1, min_key: 0, min_council: 1,
                },
            },
            UpgradeType::Emergency => match mode {
                DeploymentMode::Solo => SignatureRequirement {
                    min_ai: 2, min_human: 0, min_key: 1, min_council: 1,
                },
                DeploymentMode::Multi => SignatureRequirement {
                    min_ai: 2, min_human: 1, min_key: 0, min_council: 1,
                },
            },
            UpgradeType::Layer5Binary => match mode {
                DeploymentMode::Solo => SignatureRequirement {
                    min_ai: 3, min_human: 0, min_key: 3, min_council: 1,
                },
                DeploymentMode::Multi => SignatureRequirement {
                    min_ai: 3, min_human: 2, min_key: 0, min_council: 1,
                },
            },
        }
    }
    
    pub fn is_met(&self, sigs: &CollectedSignatures) -> bool {
        if sigs.ai_count < self.min_ai { return false; }
        if sigs.council_count < self.min_council { return false; }
        if sigs.human_count < self.min_human { return false; }
        if sigs.key_count < self.min_key { return false; }
        true
    }
}
```

**密钥生成与使用**:

```rust
// apeireth-keys/src/key.rs (新增)

pub struct PermissionKey {
    pub id: KeyId,
    pub scope: KeyScope,
    pub expires_at: i64,
    pub created_by: PrincipalId,
    pub allowed_layers: Vec<PermissionLayer>,
    pub intent_hash: Hash,
}

pub enum KeyScope {
    OneTime,                            // 一次性 (升级后销毁)
    TimeBound { until: i64 },            // 限时 (如 7 天)
    OperationBound { op: Operation },    // 操作限定 (如只能升 v0.14 → v0.15)
}

impl PermissionKey {
    /// 主人签发密钥 (YubiKey / 手机 TOTP)
    pub fn issue(scope: KeyScope, signer: &dyn Signer) -> Result<Self, KeyError> { ... }
    
    /// 主 AI 使用密钥
    pub fn use_for(&self, op: &Operation) -> Result<(), KeyError> {
        if !self.allows(op) { return Err(KeyError::NotAllowed); }
        if self.is_expired() { return Err(KeyError::Expired); }
        Ok(())
    }
}
```

---

## 6. Sandbox 验证

### 6.1 Sandbox 配置

```rust
// apeireth-upgrade/src/sandbox.rs

pub struct Sandbox {
    workdir: PathBuf,
    resources: ResourceLimits,
    network: NetworkPolicy,
    filesystem: FilesystemPolicy,
    timeout: Duration,
}

pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub max_open_files: u64,
    pub max_threads: u32,
}

pub struct NetworkPolicy {
    pub allowed_hosts: Vec<String>,  // e.g. ["crates.io", "github.com"]
    pub denied_ports: Vec<u16>,
    pub rate_limit_per_sec: u32,
}

pub struct FilesystemPolicy {
    pub readonly_paths: Vec<PathBuf>,  // 不可写
    pub writable_paths: Vec<PathBuf>,  // 可写
}
```

### 6.2 Sandbox 测试运行

```rust
impl Sandbox {
    /// 跑完整升级验证
    pub async fn run_upgrade_validation(&self, intent: &UpgradeIntent) -> Result<ValidationReport, SandboxError> {
        // 1. 准备沙盒
        self.setup().await?;
        
        // 2. 编译新版本 (在沙盒内, 不影响主基地)
        let new_binary = self.compile_in_sandbox(intent).await?;
        
        // 3. 启动新实例 (隔离进程)
        let new_instance = self.spawn_isolated(&new_binary).await?;
        
        // 4. 跑洋葱测试矩阵 (灵感 §6.2)
        let report = self.run_onion_tests(&new_instance, intent).await?;
        
        // 5. 对比新旧行为
        if report.has_regression() {
            return Err(SandboxError::Regression(report));
        }
        
        Ok(report)
    }
    
    async fn run_onion_tests(
        &self,
        instance: &TestInstance,
        intent: &UpgradeIntent,
    ) -> OnionTestReport {
        let mut report = OnionTestReport::default();
        
        let plan = &intent.manifest.test_plan;
        
        // Layer 0: 单元测试
        if plan.unit_tests {
            report.layer_0 = instance.run_unit_tests().await;
        }
        
        // Layer 1: 集成测试
        if plan.integration_tests {
            report.layer_1 = instance.run_integration_tests().await;
        }
        
        // Layer 2: 回归对比
        if plan.regression_tests {
            report.layer_2 = instance.run_regression_tests().await;
        }
        
        // Layer 3: 模糊测试
        if let Some(duration) = plan.fuzz_tests {
            report.layer_3 = instance.run_fuzz_tests(duration).await;
        }
        
        // Layer 4: 形式化验证
        if plan.formal_verify {
            report.layer_4 = instance.run_formal_verify().await;
        }
        
        // Layer 5: 性能基准
        if plan.perf_benchmark {
            report.layer_5 = instance.run_perf_benchmark().await;
        }
        
        report
    }
}
```

### 6.3 洋葱测试矩阵 (回顾)

| Layer | 单元 | 集成 | 回归 | 模糊 | 形式化 | 性能 |
|-------|------|------|------|------|--------|------|
| 0 | ✓ | — | — | — | — | — |
| 1 | ✓ | ✓ | ✓ | — | — | — |
| 2 | ✓ | ✓ | ✓ | 1min | — | ✓ |
| 3 | ✓ | ✓ | ✓ | 10min | 部分 | ✓ |
| 4 | ✓ | ✓ | ✓ | 1h | 全 | ✓ |
| 5 | ✓ | ✓ | ✓ | 8h | 全 | ✓ |

---

## 7. 双实例 + 流量切换 (Erlang/OTP)

```rust
// apeireth-upgrade/src/traffic.rs

pub struct TrafficShifter {
    instances: Arc<RwLock<HashMap<String, InstanceState>>>,
    load_balancer: Arc<LoadBalancer>,
}

pub struct InstanceState {
    pub id: String,
    pub binary: PathBuf,
    pub version: semver::Version,
    pub traffic_percent: u32,
    pub health: HealthStatus,
    pub started_at: Instant,
}

impl TrafficShifter {
    /// 双实例灰度切换
    pub async fn gradual_cutover(&self, new_instance: InstanceState) -> Result<(), SwitchError> {
        // 1. 启动新实例
        self.start_instance(&new_instance).await?;
        let new_id = new_instance.id.clone();
        
        // 2. 健康检查
        self.wait_for_healthy(&new_id, Duration::from_secs(60)).await?;
        
        // 3. 渐进切流量: 10% → 50% → 100%
        let stages = vec![10, 30, 50, 80, 100];
        for percent in stages {
            self.set_traffic(&new_id, percent).await?;
            tracing::info!(new = percent, "切换流量");
            
            // 每阶段监控 5 分钟
            tokio::time::sleep(Duration::from_secs(300)).await;
            
            // 检查健康度
            if !self.is_healthy(&new_id).await? {
                // 不健康, 回滚
                self.rollback(&new_id).await?;
                return Err(SwitchError::Unhealthy);
            }
        }
        
        // 4. 关闭旧实例
        self.shutdown_old_instances().await?;
        
        Ok(())
    }
    
    /// 立即回滚
    pub async fn rollback(&self, new_id: &str) -> Result<(), SwitchError> {
        // 切回 100% 给旧实例
        let old_instances = self.find_old_instances().await;
        for old in old_instances {
            self.set_traffic(&old.id, 100).await?;
        }
        
        // 关闭新实例
        self.shutdown_instance(new_id).await?;
        
        tracing::warn!(rolled_back = new_id, "升级回滚");
        Ok(())
    }
}
```

### 7.1 灰度切换时序

```
T+0:    v1 (100%) + v2 (0%, 准备)
T+5m:   v1 (90%) + v2 (10%)   ← 健康检查
T+10m:  v1 (70%) + v2 (30%)
T+15m:  v1 (50%) + v2 (50%)
T+20m:  v1 (20%) + v2 (80%)
T+25m:  v1 (0%) + v2 (100%)   ← 关闭 v1
T+30m:  v2 (100%, 监控)

如果任何阶段不健康 → 立即切回 v1 (100%)
```

---

## 8. 监控 + 自动回滚

```rust
// apeireth-upgrade/src/monitor.rs

pub struct PostUpgradeMonitor {
    duration: Duration,           // 30 分钟
    rollback_conditions: Vec<RollbackCondition>,
}

pub enum RollbackCondition {
    ErrorRate { threshold: f32 },       // 错误率 > 5%
    LatencyP99 { max_ms: u32 },         // p99 延迟 > 500ms
    MemoryLeak { growth_mb_per_min: u32 },  // 内存泄漏 > 10MB/min
    E3Violation,                        // E-3 守门触发
    CouncilEmergencyPause,              // 智囊团紧急暂停
    WatchdogTimeout { seconds: u32 },   // 看门狗超时
}

impl PostUpgradeMonitor {
    pub async fn monitor(&self, new_id: &str) -> Result<(), MonitorError> {
        let deadline = Instant::now() + self.duration;
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        while Instant::now() < deadline {
            interval.tick().await;
            
            let metrics = self.collect_metrics(new_id).await?;
            
            for cond in &self.rollback_conditions {
                if self.check_condition(cond, &metrics) {
                    tracing::error!(condition = ?cond, "触发自动回滚");
                    self.trigger_rollback(new_id, cond).await?;
                    return Err(MonitorError::RollbackTriggered(cond.clone()));
                }
            }
        }
        
        Ok(())
    }
}
```

---

## 9. 回滚机制

```rust
// apeireth-upgrade/src/rollback.rs

pub struct RollbackManager {
    intent_history: Arc<RwLock<VecDeque<UpgradeIntent>>>,
    rollback_strategies: HashMap<UpgradeType, RollbackStrategy>,
}

impl RollbackManager {
    /// 自动回滚
    pub async fn auto_rollback(&self, reason: RollbackReason) -> Result<(), RollbackError> {
        // 1. 取最近一个成功 intent
        let last_intent = self.intent_history.read().await.back().cloned()
            .ok_or(RollbackError::NoHistory)?;
        
        // 2. 根据 upgrade_type 选策略
        let strategy = self.rollback_strategies.get(&last_intent.upgrade_type)
            .cloned()
            .unwrap_or(RollbackStrategy::ImmediateCutover);
        
        // 3. 执行回滚
        match strategy {
            RollbackStrategy::ImmediateCutover => {
                // 立即切回旧版本
                self.traffic_shifter.rollback(&last_intent.id.to_string()).await?;
            }
            RollbackStrategy::GradualRampdown => {
                // 渐进切回
                self.traffic_shifter.gradual_rollback(&last_intent.id.to_string()).await?;
            }
            RollbackStrategy::ManualIntervention => {
                // 需人工
                self.notify_human(&last_intent).await?;
            }
        }
        
        // 4. 存档
        self.archive_rollback(&last_intent, reason).await?;
        
        Ok(())
    }
}
```

---

## 10. Layer 5 二进制重编译 (特殊路径)

```rust
pub async fn layer_5_binary_recompile(intent: UpgradeIntent) -> Result<(), UpgradeError> {
    // 1. 物理多签要求 (最高)
    // AI × 3 + 人 × 2 + 密钥 × 3
    // 单人场景: AI × 3 + 密钥 × 3
    let required_sigs = 3 + 3;  // AI + 密钥
    intent.required_sigs = required_sigs;
    
    // 2. 24h 完整集成测试 (不是 30min 监控)
    intent.manifest.test_plan = TestPlan {
        unit_tests: true,
        integration_tests: true,
        regression_tests: true,
        fuzz_tests: Some(Duration::from_secs(8 * 3600)),  // 8 小时
        formal_verify: true,    // 全形式化
        perf_benchmark: true,
    };
    
    // 3. 双实例切流量 (Erlang/OTP)
    // 4. 监控 24h (不是 30min)
    // 5. 失败 → 立即回滚 + 智囊团追溯
    
    // 详见 pipeline
    pipeline.propose(intent).await
}
```

**Layer 5 升级特殊**:
- ✅ 需要 24h 完整集成测试
- ✅ 需要 24h 监控（不是 30min）
- ✅ 失败 → 立即回滚 + 智囊团追溯
- ✅ 物理多签（AI × 3 + 人 × 2 + 密钥 × 3）
- ✅ 全形式化验证
- ✅ 不允许紧急升级（即使紧急也要走完整流程）

---

## 11. 阶段 2 第十一项收尾判定

自我升级实现已沉淀: **OTA 7 阶段 + 沙盒 + 双实例 + 自动回滚 + Layer 5 物理多签**。

**关键设计**:
- ✅ UpgradeIntent manifest (升级意图)
- ✅ Council 审核 (E-3 守门)
- ✅ 物理多签 (Layer 4-5)
- ✅ Sandbox 验证 (洋葱测试矩阵)
- ✅ 双实例 + 流量切换 (Erlang/OTP)
- ✅ 监控 30min + 自动回滚
- ✅ Layer 5 二进制重编译特殊路径 (24h 测试 + 24h 监控)

**R14 增量**:
- 新增 `apeireth-upgrade` crate (阶段 2 §3 已列)
- 与 `apeireth-supervisor` (升级子进程) + `apeireth-council` (智囊团审核) 协同

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (自我升级服务 ASI 方向)
- 主 17:43 S-2 (基于 Erlang/OTP 现有模式, 不重造)
- 主 17:58 O-5 (物理多签是 E-3 物理实现)
- 主 19:33 O-2 (Erlang/OTP 双实例借鉴)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第十二项 — **哲学守门实现** (阶段 2 最后一项)

---

## 12. 决策对比表

| 方案 | 安全 | 自动化 | 复杂度 | 推荐 |
|------|------|--------|--------|------|
| 手动升级 | ✅ 高 | ❌ 无 | 低 | ❌ 违反"自我升级" |
| 脚本升级 | ⚠️ 中 | ⚠️ 中 | 中 | ⚠️ 无守门 |
| 单实例蓝绿 | ✅ 高 | ✅ 中 | 中 | ⚠️ |
| **OTA + 智囊团 + 多签 + 双实例** | ✅✅ | ✅✅ | 中 | ✅✅ |
| **Layer 5 特殊路径** | ✅✅✅ | ✅✅ | 高 | ✅✅✅ |

**Apeireth 选 OTA + 智囊团 + 多签 + 双实例**:
- 默认: 智囊团 + 物理多签 + 双实例 (Layer 0-4)
- Layer 5: 24h 测试 + 24h 监控 + 物理多签最高

---

_主哲学 anchor 6 个全贯穿. 自我升级实现已沉淀. 下一步等用户确认进入阶段 2 第十二项 (哲学守门实现) — 阶段 2 最后一项._