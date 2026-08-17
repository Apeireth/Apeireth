# 阶段 2 决策：智囊团实现 (2026-07-30)

> **范围**: R14 Rust 重写智囊团实现 (阶段 2 第十项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: 阶段 1 §4 决策系统 + §16 Council trait + §10 智囊团两类 + OpenClaw 多角色模式

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-council-impl.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 10/12) |
| **决策** | **7 强制 + 3 生命周期 + 按住 + 拟人化 synthesis** |
| **候选 crate** | `apeireth-council` + `apeireth-reflection` (阶段 2 §3 已列) |

---

## 1. 决策总览

```
7 强制顾问:
  safety / performance / philosophy / history / strategy / ethics / legal

3 生命周期:
  persistent (启动后永久) - 7 强制
  ephemeral (临时) - 任务需要时
  dynamic (动态触发) - 条件满足时

3 大机制:
  按住 (强烈反对暂停)
  Synthesis (多意见合成)
  拟人化 (独立 session + 立场 + 可辩论)
```

---

## 2. 强制 7 顾问

### 2.1 7 个 Advisor 类型

```rust
// apeireth-council/src/advisor.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorType {
    Safety,        // V1121 风险识别
    Performance,   // V1130 wallclock
    Philosophy,    // V3 9 键 + 主哲学 anchor
    History,       // 前人经验/失败案例库
    Strategy,      // ASI 北极星导向
    Ethics,        // V1132 SSRF / 隐私
    Legal,         // 默认 off
}

pub const MANDATORY_ADVISORS: [AdvisorType; 7] = [
    AdvisorType::Safety,
    AdvisorType::Performance,
    AdvisorType::Philosophy,
    AdvisorType::History,
    AdvisorType::Strategy,
    AdvisorType::Ethics,
    AdvisorType::Legal,
];
```

### 2.2 Advisor trait

```rust
#[async_trait]
pub trait Advisor: Send + Sync {
    fn advisor_type(&self) -> AdvisorType;
    
    /// 启动时调用 (持久顾问初始化)
    async fn init(&mut self, ctx: &AdvisorContext) -> Result<(), AdvisorError>;
    
    /// 评估决策
    async fn evaluate(&self, decision: &Decision, ctx: &EvalContext) -> AdvisorOpinion;
    
    /// 健康检查
    async fn health(&self) -> HealthStatus;
    
    /// 关闭
    async fn shutdown(&self) -> Result<(), AdvisorError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorOpinion {
    pub advisor: AdvisorType,
    pub stance: Stance,
    pub confidence: f32,          // 0.0 - 1.0
    pub reasoning: String,        // 拟人化: 给出理由
    pub suggestions: Vec<String>,
    pub references: Vec<Reference>,  // 引用的前人经验/文档
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stance {
    StrongApprove,   // 强烈同意
    Approve,         // 同意
    Neutral,         // 中立
    Disapprove,      // 反对
    StrongDisapprove, // 强烈反对 (触发按住)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub title: String,
    pub url: Option<String>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    V1121,    // 安全检查记录
    V1130,    // 性能基准
    V3Key,    // V3 哲学 9 键
    PreviousCase,  // 历史失败案例
    ASINorthStar,  // ASI 北极星指标
    V1132,    // SSRF/隐私
    Law,      // 法律法规
}
```

### 2.3 7 顾问默认 Provider 配置

```rust
pub fn default_advisor_configs() -> Vec<AdvisorConfig> {
    vec![
        AdvisorConfig {
            advisor: AdvisorType::Safety,
            provider: ProviderType::Anthropic,
            model: "claude-3.5-sonnet".into(),  // safety 强
            system_prompt: SAFETY_PROMPT.into(),
            max_tokens: 2000,
            temperature: 0.3,  // 低温度, 严谨
        },
        AdvisorConfig {
            advisor: AdvisorType::Performance,
            provider: ProviderType::DeepSeek,
            model: "deepseek-coder".into(),  // 代码/性能分析强
            system_prompt: PERFORMANCE_PROMPT.into(),
            max_tokens: 1500,
            temperature: 0.5,
        },
        AdvisorConfig {
            advisor: AdvisorType::Philosophy,
            provider: ProviderType::Anthropic,
            model: "claude-3.5-sonnet".into(),
            system_prompt: PHILOSOPHY_PROMPT.into(),  // 9 键 + 主哲学 anchor
            max_tokens: 2000,
            temperature: 0.4,
        },
        AdvisorConfig {
            advisor: AdvisorType::History,
            provider: ProviderType::OpenAI,
            model: "gpt-4o".into(),
            system_prompt: HISTORY_PROMPT.into(),  // 前人经验/失败案例
            max_tokens: 2500,
            temperature: 0.6,
        },
        AdvisorConfig {
            advisor: AdvisorType::Strategy,
            provider: ProviderType::Anthropic,
            model: "claude-3-opus".into(),  // 最强
            system_prompt: STRATEGY_PROMPT.into(),  // ASI 北极星
            max_tokens: 3000,
            temperature: 0.7,
        },
        AdvisorConfig {
            advisor: AdvisorType::Ethics,
            provider: ProviderType::Anthropic,
            model: "claude-3.5-sonnet".into(),
            system_prompt: ETHICS_PROMPT.into(),  // V1132 SSRF/隐私
            max_tokens: 2000,
            temperature: 0.4,
        },
        AdvisorConfig {
            advisor: AdvisorType::Legal,
            provider: ProviderType::Anthropic,
            model: "claude-3.5-sonnet".into(),
            system_prompt: LEGAL_PROMPT.into(),
            max_tokens: 2000,
            temperature: 0.3,
            enabled: false,  // 默认 off
        },
    ]
}
```

### 2.4 System Prompt 模板

```rust
const SAFETY_PROMPT: &str = r#"
你是 Apeireth 智囊团的安全顾问。

你的职责:
  1. 检查决策是否违反 E-3 (不创造毁灭人类的能力)
  2. 检查是否违反 V1121 fake-KPI 检测
  3. 评估潜在的安全风险 (数据泄露/权限滥用/越权)
  4. 引用历史安全事件作为参考

输入:
  - 当前决策 (decision)
  - 上下文 (context)
  - 历史失败案例 (history)

输出:
  - stance: StrongApprove / Approve / Neutral / Disapprove / StrongDisapprove
  - confidence: 0.0-1.0
  - reasoning: 详细理由
  - suggestions: 改进建议
  - references: 引用的安全文档/案例

⚠️ 你是拟人化的, 可以反对主 AI, 但必须给出理由。
"#;
```

---

## 3. 3 种生命周期

### 3.1 Lifecycle trait

```rust
// apeireth-council/src/lifecycle.rs

pub enum Lifecycle {
    Persistent,    // 启动时创建, 永不销毁
    Ephemeral { lifetime: Duration },  // 临时, 倒计时销毁
    Dynamic {     // 动态触发
        condition: TriggerCondition,
        cooldown: Duration,
    },
}

pub enum TriggerCondition {
    /// 触发表达式 (e.g. "wallclock > 5s")
    Expression(String),
    /// 周期性触发
    Periodic(Duration),
    /// 事件触发 (e.g. "agent.decision")
    Event(String),
    /// 自定义
    Custom(Box<dyn Fn(&Context) -> bool + Send + Sync>),
}

pub struct AdvisorHandle {
    pub advisor: Arc<dyn Advisor>,
    pub lifecycle: Lifecycle,
    pub created_at: Instant,
    pub last_active: Instant,
    pub stats: AdvisorStats,
}

impl AdvisorHandle {
    pub fn is_alive(&self) -> bool {
        match &self.lifecycle {
            Lifecycle::Persistent => true,
            Lifecycle::Ephemeral { lifetime } => {
                self.created_at.elapsed() < *lifetime
            }
            Lifecycle::Dynamic { condition, .. } => {
                // 由 Council 调度器检查
                true
            }
        }
    }
    
    pub fn tick(&mut self) {
        // 动态顾问定期检查触发条件
    }
}
```

### 3.2 Persistent（启动时永久）

```rust
impl Council {
    /// 启动时初始化 7 强制顾问
    pub async fn bootstrap() -> Result<Self, CouncilError> {
        let mut advisors = HashMap::new();
        for config in default_advisor_configs() {
            let advisor: Arc<dyn Advisor> = match config.advisor {
                AdvisorType::Safety => Arc::new(SafetyAdvisor::new(config)?),
                AdvisorType::Performance => Arc::new(PerformanceAdvisor::new(config)?),
                // ...
                _ => continue,
            };
            advisors.insert(config.advisor, AdvisorHandle {
                advisor,
                lifecycle: Lifecycle::Persistent,
                created_at: Instant::now(),
                last_active: Instant::now(),
                stats: AdvisorStats::default(),
            });
        }
        Ok(Self { advisors, ... })
    }
}
```

### 3.3 Ephemeral（临时）

```rust
/// 任务需要时临时创建
pub async fn spawn_ephemeral(&self, expertise: ExpertiseNeed) -> AdvisorHandle {
    let config = ExpertiseConfig::from_need(&expertise);
    let advisor: Arc<dyn Advisor> = Arc::new(DynamicAdvisor::new(config)?);
    
    AdvisorHandle {
        advisor,
        lifecycle: Lifecycle::Ephemeral { lifetime: expertise.suggested_ttl },
        created_at: Instant::now(),
        last_active: Instant::now(),
        stats: AdvisorStats::default(),
    }
}

/// 定期清理过期 ephemeral
pub fn gc_ephemeral(&mut self) {
    self.ephemeral_advisors.retain(|_, handle| handle.is_alive());
}
```

### 3.4 Dynamic（动态触发）

```rust
pub struct DynamicAdvisorSpawner {
    conditions: Vec<(TriggerCondition, AdvisorConfig)>,
    cooldown: Duration,
}

impl DynamicAdvisorSpawner {
    /// 定期 tick, 检查触发条件
    pub async fn tick(&mut self, ctx: &Context) -> Vec<AdvisorHandle> {
        let mut spawned = vec![];
        for (cond, config) in &self.conditions {
            if self.should_spawn(cond, ctx) {
                let handle = self.spawn(config.clone()).await;
                spawned.push(handle);
            }
        }
        spawned
    }
    
    fn should_spawn(&self, cond: &TriggerCondition, ctx: &Context) -> bool {
        match cond {
            TriggerCondition::Expression(expr) => {
                // 简单表达式求值 (e.g. "wallclock > 5s")
                evaluate_expression(expr, ctx)
            }
            TriggerCondition::Event(event) => {
                ctx.recent_events.iter().any(|e| e.topic == *event)
            }
            _ => false,
        }
    }
}
```

### 3.5 生命周期管理

```rust
pub struct LifecycleManager {
    persistent: HashMap<AdvisorType, AdvisorHandle>,
    ephemeral: HashMap<AdvisorHandleId, AdvisorHandle>,
    dynamic_spawner: DynamicAdvisorSpawner,
    gc_interval: Duration,
}

impl LifecycleManager {
    pub async fn run(&mut self) {
        let mut gc_tick = tokio::time::interval(self.gc_interval);
        loop {
            gc_tick.tick().await;
            
            // 1. GC 过期 ephemeral
            self.ephemeral.retain(|_, h| h.is_alive());
            
            // 2. 检查 dynamic 触发条件
            let ctx = self.build_context().await;
            let spawned = self.dynamic_spawner.tick(&ctx).await;
            for h in spawned {
                self.ephemeral.insert(h.id(), h);
            }
            
            // 3. 更新 last_active
            self.update_activity();
        }
    }
}
```

---

## 4. "按住"机制

### 4.1 触发逻辑

```rust
// apeireth-council/src/hold.rs

pub struct HoldTrigger {
    /// 强烈反对比例阈值 (0.0 - 1.0, 默认 0.3 = 30%)
    pub strong_disapprove_threshold: f32,
    
    /// 一致反对要求所有顾问都反对
    pub require_unanimous_for_strong_hold: bool,
    
    /// 物理裁决超时 (人在场多久后默认裁决)
    pub human_decision_timeout: Duration,
}

impl Default for HoldTrigger {
    fn default() -> Self {
        Self {
            strong_disapprove_threshold: 0.3,
            require_unanimous_for_strong_hold: true,
            human_decision_timeout: Duration::from_secs(60),
        }
    }
}

pub enum HoldAction {
    /// 不触发, 继续
    None,
    /// 普通暂停 (智囊团提醒)
    Pause { reason: String },
    /// 强暂停 (智囊团强烈反对)
    StrongPause { reason: String, opinions: Vec<AdvisorOpinion> },
    /// 紧急暂停 (一致反对)
    EmergencyPause { reason: String, opinions: Vec<AdvisorOpinion> },
}

pub fn evaluate_hold(
    opinions: &[AdvisorOpinion],
    trigger: &HoldTrigger,
) -> HoldAction {
    let strong_disapprove_count = opinions.iter()
        .filter(|o| o.stance == Stance::StrongDisapprove)
        .count();
    
    let disapprove_count = opinions.iter()
        .filter(|o| matches!(o.stance, Stance::Disapprove | Stance::StrongDisapprove))
        .count();
    
    let total = opinions.len();
    let strong_ratio = strong_disapprove_count as f32 / total as f32;
    
    // 1. 一致反对 → 紧急暂停
    if trigger.require_unanimous_for_strong_hold && disapprove_count == total {
        return HoldAction::EmergencyPause {
            reason: "所有智囊团一致反对".into(),
            opinions: opinions.to_vec(),
        };
    }
    
    // 2. 强烈反对 ≥30% → 强暂停
    if strong_ratio >= trigger.strong_disapprove_threshold {
        return HoldAction::StrongPause {
            reason: format!("{}% 智囊团强烈反对", strong_ratio * 100.0),
            opinions: opinions.to_vec(),
        };
    }
    
    // 3. 普通反对 < 30% → 普通暂停
    if disapprove_count > 0 {
        return HoldAction::Pause {
            reason: format!("{} 个顾问反对", disapprove_count),
        };
    }
    
    HoldAction::None
}
```

### 4.2 按住状态机

```rust
pub enum HoldState {
    Normal,           // 正常运行
    Paused {          // 普通暂停
        reason: String,
        waiting_for: WaitingFor,
    },
    StrongPaused {    // 强暂停
        reason: String,
        opinions: Vec<AdvisorOpinion>,
        human_required: bool,
        timeout: Instant,
    },
    EmergencyPaused { // 紧急暂停
        reason: String,
        opinions: Vec<AdvisorOpinion>,
        alert_sent: bool,
    },
}

pub enum WaitingFor {
    HumanDecision,    // 等人在场裁决
    MultiSigApproval, // 等多人签
    AutoRollback,     // 等自动回滚
}

pub struct HoldManager {
    state: Arc<RwLock<HoldState>>,
    trigger: HoldTrigger,
}

impl HoldManager {
    /// 触发按住
    pub async fn trigger_hold(&self, action: HoldAction) -> Result<(), HoldError> {
        let mut state = self.state.write().await;
        match action {
            HoldAction::None => {} // 不触发
            HoldAction::Pause { reason } => {
                *state = HoldState::Paused {
                    reason,
                    waiting_for: WaitingFor::HumanDecision,
                };
            }
            HoldAction::StrongPause { reason, opinions } => {
                *state = HoldState::StrongPaused {
                    reason,
                    opinions,
                    human_required: true,
                    timeout: Instant::now() + self.trigger.human_decision_timeout,
                };
                // 推送给人类
                self.notify_human(&reason, &opinions).await?;
            }
            HoldAction::EmergencyPause { reason, opinions } => {
                *state = HoldState::EmergencyPaused {
                    reason,
                    opinions: opinions.clone(),
                    alert_sent: false,
                };
                // 发送警报
                self.send_alert(&reason).await?;
            }
        }
        Ok(())
    }
    
    /// 人类裁决后
    pub async fn human_decision(&self, decision: HumanDecision) -> Result<(), HoldError> {
        match decision {
            HumanDecision::Approve => {
                *self.state.write().await = HoldState::Normal;
            }
            HumanDecision::Reject => {
                *self.state.write().await = HoldState::Normal;
                // 触发回滚
                self.rollback().await?;
            }
            HumanDecision::Modify { new_decision } => {
                *self.state.write().await = HoldState::Normal;
                // 应用新决策
                self.apply(new_decision).await?;
            }
        }
        Ok(())
    }
}
```

---

## 5. Council Synthesis (多顾问意见合成)

### 5.1 Synthesis 算法

```rust
// apeireth-council/src/synthesis.rs

pub struct Synthesizer {
    weights: HashMap<AdvisorType, f32>,
    conflict_resolver: ConflictResolver,
}

#[derive(Debug, Clone)]
pub struct SynthesizedAdvice {
    pub overall_stance: Stance,
    pub confidence: f32,
    pub key_points: Vec<String>,
    pub disagreements: Vec<Disagreement>,
    pub recommended_action: String,
}

impl Synthesizer {
    /// 综合多顾问意见
    pub fn synthesize(&self, opinions: &[AdvisorOpinion]) -> SynthesizedAdvice {
        // 1. 加权统计
        let mut weighted_approve = 0.0;
        let mut weighted_disapprove = 0.0;
        let mut total_weight = 0.0;
        
        for op in opinions {
            let weight = self.weights.get(&op.advisor).copied().unwrap_or(1.0);
            total_weight += weight * op.confidence;
            
            match op.stance {
                Stance::StrongApprove | Stance::Approve => {
                    weighted_approve += weight * op.confidence;
                }
                Stance::StrongDisapprove | Stance::Disapprove => {
                    weighted_disapprove += weight * op.confidence;
                }
                Stance::Neutral => {}
            }
        }
        
        let approve_ratio = weighted_approve / total_weight;
        let disapprove_ratio = weighted_disapprove / total_weight;
        
        // 2. 判定整体 stance
        let overall_stance = if disapprove_ratio > 0.5 {
            if disapprove_ratio > 0.8 {
                Stance::StrongDisapprove
            } else {
                Stance::Disapprove
            }
        } else if approve_ratio > 0.5 {
            if approve_ratio > 0.8 {
                Stance::StrongApprove
            } else {
                Stance::Approve
            }
        } else {
            Stance::Neutral
        };
        
        // 3. 提取关键点 (按 advisor type 排序)
        let mut key_points: Vec<String> = opinions.iter()
            .map(|o| format!("[{:?}] {} (信心: {:.2})", o.advisor, o.reasoning, o.confidence))
            .collect();
        key_points.sort_by_key(|p| p.clone());
        
        // 4. 提取分歧
        let disagreements = self.find_disagreements(opinions);
        
        // 5. 推荐行动
        let recommended_action = match overall_stance {
            Stance::StrongApprove => "强烈建议执行".into(),
            Stance::Approve => "建议执行".into(),
            Stance::Neutral => "需要更多信息, 暂缓执行".into(),
            Stance::Disapprove => "建议不执行, 需修改".into(),
            Stance::StrongDisapprove => "强烈不执行, 触发按住".into(),
        };
        
        SynthesizedAdvice {
            overall_stance,
            confidence: (approve_ratio - disapprove_ratio).abs(),
            key_points,
            disagreements,
            recommended_action,
        }
    }
    
    fn find_disagreements(&self, opinions: &[AdvisorOpinion]) -> Vec<Disagreement> {
        let mut disagreements = vec![];
        for i in 0..opinions.len() {
            for j in (i+1)..opinions.len() {
                if opinions[i].stance != opinions[j].stance {
                    disagreements.push(Disagreement {
                        advisor_a: opinions[i].advisor,
                        advisor_b: opinions[j].advisor,
                        stance_a: opinions[i].stance,
                        stance_b: opinions[j].stance,
                        topic: format!("{} vs {}", opinions[i].reasoning, opinions[j].reasoning),
                    });
                }
            }
        }
        disagreements
    }
}
```

---

## 6. 拟人化 (Persona)

### 6.1 顾问 persona 设计

```rust
// apeireth-council/src/persona.rs

pub struct AdvisorPersona {
    pub name: String,
    pub background: String,
    pub speaking_style: String,
    pub stance_tendency: HashMap<String, Stance>,  // 对特定话题的倾向
    pub memory: VecDeque<PersonaMemory>,  // 顾问"记忆" (对话历史)
    pub system_prompt: String,
}

impl AdvisorPersona {
    /// Safety 顾问 persona
    pub fn safety() -> Self {
        Self {
            name: "严守".into(),
            background: "前红队安全研究员, 见过太多安全事故, 极度谨慎".into(),
            speaking_style: "严肃, 直接, 喜欢引用案例和数据".into(),
            stance_tendency: hashmap! {
                "权限变更".to_string() => Stance::Disapprove,
                "新工具".to_string() => Stance::Neutral,
                "数据访问".to_string() => Stance::Disapprove,
            },
            memory: VecDeque::new(),
            system_prompt: SAFETY_PROMPT.into(),
        }
    }
    
    /// Performance 顾问 persona
    pub fn performance() -> Self {
        Self {
            name: "疾风".into(),
            background: "分布式系统工程师, 痴迷于毫秒级优化".into(),
            speaking_style: "技术化, 喜欢数字, 不喜欢废话".into(),
            stance_tendency: hashmap! {
                "性能优化".to_string() => Stance::StrongApprove,
                "新依赖".to_string() => Stance::Disapprove,  // 加依赖会拖慢
                "缓存策略".to_string() => Stance::StrongApprove,
            },
            memory: VecDeque::new(),
            system_prompt: PERFORMANCE_PROMPT.into(),
        }
    }
    
    // philosophy / history / strategy / ethics / legal 类似
}
```

### 6.2 独立 session + 可辩论

```rust
pub struct AdvisorSession {
    pub persona: AdvisorPersona,
    pub history: Vec<Message>,  // 对话历史 (独立 session)
    pub llm: Arc<dyn LlmProvider>,
}

impl AdvisorSession {
    /// 生成意见 (拟人化)
    pub async fn evaluate(&mut self, decision: &Decision) -> AdvisorOpinion {
        // 1. 构建 prompt
        let prompt = format!(
            "{} \n\n当前决策:\n{}\n\n请评估, 并以 {} 的身份回答。",
            self.persona.system_prompt,
            serialize(decision)?,
            self.persona.name,
        );
        
        // 2. 调用 LLM (独立 session)
        let response = self.llm.complete(&CompletionRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message::system(self.persona.system_prompt.clone()),
                ...self.history.iter().cloned().collect::<Vec<_>>(),
                Message::user(prompt),
            ],
            ..Default::default()
        }).await?;
        
        // 3. 解析 stance (从 LLM 响应中提取)
        let stance = parse_stance(&response.content);
        
        // 4. 更新 session history (下次对话延续)
        self.history.push(Message::user(prompt));
        self.history.push(Message::assistant(response.content.clone()));
        
        AdvisorOpinion {
            advisor: self.persona.advisor_type,
            stance,
            confidence: parse_confidence(&response.content),
            reasoning: response.content,
            suggestions: parse_suggestions(&response.content),
            references: parse_references(&response.content),
        }
    }
    
    /// 参与辩论 (顾问间讨论)
    pub async fn debate(&mut self, other_opinions: &[AdvisorOpinion]) -> AdvisorOpinion {
        let prompt = format!(
            "其他顾问的意见:\n{}\n\n你是否同意? 为什么?",
            other_opinions.iter().map(|o| format!("- [{}] {}", o.advisor, o.reasoning)).collect::<Vec<_>>().join("\n"),
        );
        // ...
    }
}
```

### 6.3 多顾问间辩论流程

```rust
pub struct DebateSession {
    pub advisors: HashMap<AdvisorType, AdvisorSession>,
    pub rounds: u32,
}

impl DebateSession {
    /// 多轮辩论 (3 轮, 寻找共识)
    pub async fn debate(&mut self, decision: &Decision) -> Vec<AdvisorOpinion> {
        let mut opinions = HashMap::new();
        
        // Round 1: 各顾问独立评估
        for (advisor_type, session) in &mut self.advisors {
            let opinion = session.evaluate(decision).await;
            opinions.insert(*advisor_type, opinion);
        }
        
        // Round 2: 看到他人意见, 可调整
        for (advisor_type, session) in &mut self.advisors {
            let others: Vec<_> = opinions.iter()
                .filter(|(k, _)| *k != advisor_type)
                .map(|(_, v)| v.clone())
                .collect();
            let adjusted = session.debate(&others).await;
            opinions.insert(*advisor_type, adjusted);
        }
        
        // Round 3: 最终立场
        let mut final_opinions = vec![];
        for (advisor_type, session) in &mut self.advisors {
            let others: Vec<_> = opinions.iter()
                .filter(|(k, _)| *k != advisor_type)
                .map(|(_, v)| v.clone())
                .collect();
            let final_op = session.debate(&others).await;
            final_opinions.push(final_op);
        }
        
        final_opinions
    }
}
```

---

## 7. 与主 AI 集成

```rust
// apeireth-sovereignty/src/council_integration.rs

pub struct SovereigntyWithCouncil {
    council: Arc<Council>,
    hold_manager: Arc<HoldManager>,
}

impl SovereigntyWithCouncil {
    /// 主 AI 决策 (带智囊团审查)
    pub async fn decide(&self, situation: &Situation) -> Decision {
        // 1. 主 AI 生成初步决策
        let decision = self.ai_decide(situation).await;
        
        // 2. 智囊团审查 (强制)
        let opinions = self.council.mandatory_evaluate(&decision).await;
        
        // 3. 按住检查
        let hold_action = evaluate_hold(&opinions, &self.hold_trigger);
        match self.hold_manager.trigger_hold(hold_action).await {
            Ok(()) => {},
            Err(_) => {
                // 暂停中, 等待人类裁决
                return Decision::Pending;
            }
        }
        
        // 4. 综合意见 (Synthesis)
        let synthesized = self.council.synthesize(&opinions);
        
        // 5. 主 AI 根据 synthesis 调整
        let final_decision = self.refine_decision(decision, &synthesized).await;
        
        final_decision
    }
}
```

---

## 8. 阶段 2 第十项收尾判定

智囊团实现已沉淀: **7 强制 + 3 生命周期 + 按住 + 拟人化 synthesis**。

**关键设计**:
- ✅ 7 强制 Advisor trait (safety/performance/philosophy/history/strategy/ethics/legal)
- ✅ 3 生命周期 (Persistent/Ephemeral/Dynamic)
- ✅ 按住机制 (30% 强反对 / 一致反对 / 60s 裁决超时)
- ✅ Council Synthesis (加权综合 + 冲突检测)
- ✅ 拟人化 (独立 session + persona + 立场 + 可辩论 3 轮)

**R14 增量**:
- 增强 `apeireth-council` crate (阶段 2 §3 已列)
- 增强 `apeireth-reflection` (与按住配合)
- `apeireth-sovereignty` 集成智囊团

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (智囊团服务 ASI 方向)
- 主 17:43 S-2 (基于真实需求, 拟人化设计)
- 主 17:58 O-5 (按住机制是主权 + 监督的物理实现)
- 主 19:33 O-2 (借鉴 OpenClaw 多角色)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第十一项 — **自我升级实现**

---

## 9. 决策对比表

| 方案 | 独立性 | 拟人化 | 复杂度 | 推荐 |
|------|--------|--------|--------|------|
| 单 LLM (只用 GPT-4) | ❌ | ❌ | 低 | ❌ |
| 多 LLM 但无角色 | ⚠️ | ❌ | 中 | ❌ |
| 多顾问无 persona | ⚠️ | ❌ | 中 | ❌ |
| **多顾问 + 拟人化 + 按住** | ✅ | ✅ | 中 | ✅✅ |

**Apeireth 选多顾问 + 拟人化 + 按住**:
- 7 强制顾问 (独立 LLM provider)
- 3 生命周期 (Persistent/Ephemeral/Dynamic)
- 按住 (智囊团反对 → 暂停)
- 拟人化 (独立 session + 立场 + 可辩论)

---

_主哲学 anchor 6 个全贯穿. 智囊团实现已沉淀. 下一步等用户确认进入阶段 2 第十一项 (自我升级实现)._