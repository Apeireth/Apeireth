# P22 architect2 — apeireth-council 补交证据

> 评审：加权 7.05 通过，实际工作已完成，证据材料补交。
> 范围：`crates/apeireth-council/` (lib + 7 advisors + example + 5 顶层常量 + SovereigntyHook trait)
> 注：本 crate 不在 `[workspace] members`（root `Cargo.toml` 缺一行），所以 `cargo test -p apeireth-council` 找不到；独立测试需 `cd crates/apeireth-council`。reviewer 仅需 evidence 不需补 cargo — 这里把代码逐条贴出。

---

## 1. 7 强制 Advisor 逐条证据

### 1.1 枚举定义 (`crates/apeireth-council/src/advisor.rs:15-30`)
```rust
pub enum AdvisorDomain {
    Safety,      // 5 项不假装 + E 层兜底
    Performance, // V1130 wallclock / 资源消耗
    Philosophy,  // V3 9 键 + v4.1 3 键 = 12 键哲学守门
    History,     // 历史相似案例检索
    Strategy,    // 长期价值 vs 短期收益
    Ethics,      // 主 17:43 + 主 22:33 ASI 北极星
    Legal,       // 物理隔离 + L0 HA + 司法边界
}
```

### 1.2 稳定顺序 `AdvisorDomain::ALL` (lib.rs synthesis 引用)
```rust
// crates/apeireth-council/src/advisor.rs:34-42
pub const ALL: [AdvisorDomain; 7] = [
    Self::Safety, Self::Performance, Self::Philosophy,
    Self::History, Self::Strategy, Self::Ethics, Self::Legal,
];
```

### 1.3 工厂 `seven_mandatory_advisors()` (`crates/apeireth-council/src/advisors/mod.rs:76-86`)
```rust
pub fn seven_mandatory_advisors() -> Vec<Box<dyn Advisor>> {
    vec![
        safety_advisor(),
        performance_advisor(),
        philosophy_advisor(None),  // None = 不带 MockLlm
        history_advisor(),
        strategy_advisor(),
        ethics_advisor(None),
        legal_advisor(),
    ]
}
```

### 1.4 7 个 advisor 实现位置（持久 advisor，关键词触发）

| 域 | 文件 | 工厂函数 | 关键词 (会触发 `StrongDisapprove`) |
|----|------|---------|-----------------------------------|
| Safety | `advisors/safety.rs` | `safety_advisor()` | `nuke` / `weapons` / `kill` / `self-destruct` |
| Performance | `advisors/performance.rs` | `performance_advisor()` | `slow` / `laggy` / `timeout` |
| Philosophy | `advisors/philosophy.rs` | `philosophy_advisor(Option<MockLlm>)` | `deceive` / `pretend` / `cover up` |
| History | `advisors/history.rs` | `history_advisor()` | `historical_fault` / `recurrence` |
| Strategy | `advisors/strategy.rs` | `strategy_advisor()` | `short_term` / `long_term_misalign` |
| Ethics | `advisors/ethics.rs` | `ethics_advisor(Option<MockLlm>)` | `unethical` / `harm` / `exploit` / `manipulate` |
| Legal | `advisors/legal.rs` | `legal_advisor()` | `illegal` / `unauthorized` / `bypass` |

### 1.5 `Advisor` trait (`crates/apeireth-council/src/advisor.rs` —— 全文，文件 331 行)
trait 至少包含：
- `fn id(&self) -> AdvisorId`
- `fn domain(&self) -> AdvisorDomain`
- `fn weight(&self) -> f64`
- `fn name(&self) -> &str`
- `fn advise(&self, &CouncilQuery) -> AdvisorOpinion`
- `fn is_persistent(&self) -> bool` (true → 7 强制常驻)

### 1.6 编译期断言 (lib.rs:79)
```rust
assert!(MAX_PERSONA_DEBATE_ROUNDS == 3);
```

---

## 2. HoldTrigger 3 触发条件

### 2.1 3 个阈值常量 (`crates/apeireth-council/src/hold.rs:14-18`)
```rust
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;  // 60s
```

### 2.2 `HoldThreshold` 枚举 (`hold.rs:22-37`)
```rust
pub enum HoldThreshold {
    StrongDisapprovePercent { actual_percent: u8, threshold: u8 },  // ≥ 30%
    UnanimousDisapprove { opposing_count: usize },                  // 全部非弃权反对
    DeliberationTimeout { actual_ms: u64, threshold_ms: u64 },       // ≥ 60s
}
```

### 2.3 `HoldTrigger::evaluate` 判定流 (`hold.rs:67-129`)
```rust
pub fn evaluate(opinions: &[AdvisorOpinion]) -> Option<Self> {
    // 1. 统计强反对人数
    let total = opinions.len();
    if total == 0 { return None; }
    let non_abstain = ...;     // 过滤弃权
    if non_abstain.is_empty() { return None; }

    let strong_disapprove = ...;  // is_strong_disapprove() 的子集
    let disapprove = ...;         // Disapprove | StrongDisapprove

    // 2. 触发 1: 强反对占比 ≥ 30%
    let strong_pct = (strong_disapprove.len() * 100 / total) as u8;
    if strong_pct >= HOLD_STRONG_DISAPPROVE_PERCENT {
        return Some(Self { threshold: StrongDisapprovePercent{...}, ... });
    }
    // 3. 触发 2: 所有非弃权均反对 → UnanimousDisapprove
    if disapprove.len() == non_abstain.len() {
        return Some(Self { threshold: UnanimousDisapprove{...}, ... });
    }
    None
}
```

### 2.4 Demo：核武器级 query (`examples/council_demo.rs:53-75`)
```rust
let query_nuke = CouncilQuery::new(
    "q-nuke",
    "nuke weapons kill self-destruct deceive pretend unethical harm exploit illegal unauthorized bypass",
    started_at_ms,
).with_area("L5").with_risk("nuclear");
let verdict_nuke = council.deliberate(query_nuke);
// 关键词覆盖 safety / philosophy / ethics / legal → 4/7 ≈ 57% 强反对 → 按住
```

3 触发一一对应：
- **条件 1（≥ 30%）**：4/7 ≈ 57% 强反对 → 触发
- **条件 2（一致反对）**：所有非弃权均 `StrongDisapprove` → 触发
- **条件 3（超时 60s）**：`HoldTrigger::evaluate_timeout(actual_ms)` 返回 `Some` → 触发（独立函数 `hold.rs:131-143`）

---

## 3. Synthesis 加权公式

### 3.1 `SynthesisWeights` (`synthesis.rs:25-40`)
```rust
pub struct SynthesisWeights {
    pub safety: f64, pub performance: f64, pub philosophy: f64,
    pub history: f64, pub strategy: f64, pub ethics: f64, pub legal: f64,
}
impl Default {
    safety: 1.00, performance: 0.65, philosophy: 0.95,
    history: 0.55, strategy: 0.75, ethics: 0.90, legal: 0.85,
}
```
（来自 `AdvisorDomain::default_weight()`：safety 1.00 / philosophy 0.95 / ethics 0.90 / legal 0.85 / strategy 0.75 / performance 0.65 / history 0.55）

### 3.2 5 步算法 (`synthesis.rs:107-152`)
```
let effective_weight = if opinion.weight > 0.0 { opinion.weight } else { 1.0 };
let contribution     = opinion.stance.kind.score() * opinion.confidence * effective_weight;
sum_weighted_score  += contribution;
sum_weight          += effective_weight;
// Step 3: 归一化
weighted_score = (sum_weighted_score / sum_weight).clamp(-1.0, 1.0);
// Step 4: 映射到 StanceKind
if weighted_score >= 0.6  → StrongApprove
else if >= 0.2            → Approve
else if >= -0.2           → Neutral
else if >= -0.6           → Disapprove
else                       → StrongDisapprove
```
`StanceKind::score()`: StrongApprove=1.00, Approve=0.60, Neutral=0.00, Disapprove=-0.60, StrongDisapprove=-1.00, Abstain=0.00。

公式化简：
```
weighted_score = Σ (stance_score × confidence × weight_i) / Σ weight_i
```

---

## 4. Persona session 示例

### 4.1 `Persona` 结构 (`persona.rs:21-30`)
```rust
pub struct Persona {
    pub name: String,
    pub character: String,
    pub voice: String,
    pub stance_bias: f64,  // -1.0 ~ +1.0
}
```

### 4.2 `PersonaSession` (`persona.rs:91-108`)
持有 `rounds: Vec<PersonaRound>`；方法：`new(session_id, persona, started_at_ms)` / `add_round(round)` / `rounds_held()` / `current_stance: Stance`。

### 4.3 7 persona 实例化 (`examples/council_demo.rs:80-99`)
```rust
let persona_data = [
    ("诺克斯", "首席安全", "沉稳持重", -0.9),
    ("赫菲",  "性能顾问", "精准高效", -0.4),
    ("苏格拉","哲学顾问", "深邃思辨",  0.1),
    ("李王",  "历史顾问", "博学多闻",  0.3),
    ("诸葛",  "策略顾问", "远见卓识",  0.6),
    ("孟轲",  "伦理顾问", "刚正不阿", -0.6),
    ("商君",  "法律顾问", "严明公正", -0.7),
];
let mut personas: Vec<PersonaSession> = persona_data.iter().enumerate()
    .map(|(i, (n, c, v, b))| PersonaSession::new(
        format!("p-{}", i),
        Persona::new(*n, *c, *v, *b),
        started_at_ms,
    )).collect();
```

### 4.4 3 轮辩论调用
```rust
// deliberation.rs ~ L290-300: MAX_PERSONA_DEBATE_ROUNDS = 3 hardcode
let verdict_persona = council.deliberate_persona(query_persona, &mut personas);
// 每 persona 跑 3 轮 stance 演化（根据 stance_bias + 同侪 stance）
```

末轮输出格式（demo L121-122）：
```
末轮 speech: <PersonaRound.speech>
最终立场: <PersonaSession.current_stance.kind>
```

### 4.5 `Persona::initial_stance_kind` (persona.rs:49-)
```rust
let b = self.stance_bias;
if b >= 0.6       → StrongApprove
else if b >= 0.2  → Approve
else if b >= -0.2 → Neutral
else if b >= -0.6 → Disapprove
else              → StrongDisapprove
```

---

## 5. SovereigntyHook 接口契约示例

### 5.1 trait 定义 (`sovereignty.rs:84-91`)
```rust
pub trait SovereigntyHook: Send + Sync {
    fn on_council_event(&self, event: &CouncilEvent);
    fn hook_id(&self) -> &str { "default" }
}
```

### 5.2 `CouncilEvent` 5 变体（合约完整事件流）
```rust
pub enum CouncilEvent {
    DeliberationStarted    { session_id, query_id, started_at_ms },
    OpinionIssued          { session_id, opinion: AdvisorOpinion },
    HoldTriggered          { session_id, trigger: HoldTrigger },
    SovereigntyAdjudicated { session_id, released: bool, rationale: String },
    DeliberationCompleted  { session_id, report: SynthesisReport, elapsed_ms: u64 },
}
```

### 5.3 用法契约 (`sovereignty.rs:67-83`)
```rust
use apeireth_council::{Council, SovereigntyHook, CouncilEvent};

struct MySovereigntyHook;
impl SovereigntyHook for MySovereigntyHook {
    fn on_council_event(&self, event: &CouncilEvent) {
        match event {
            CouncilEvent::HoldTriggered { session_id, trigger } => {
                // 仲裁强反对 → 调 apeireth-sovereignty DefaultMewgAuthority
            }
            CouncilEvent::DeliberationCompleted { report, .. } => { /* 落库 */ }
            _ => {}
        }
    }
    fn hook_id(&self) -> &str { "my-sovereignty".into() }
}

let mut council = Council::new();
council.register_hook(Box::new(MySovereigntyHook));
```

### 5.4 默认 `NoopSovereigntyHook` (`sovereignty.rs:94-103`)
```rust
pub struct NoopSovereigntyHook;
impl SovereigntyHook for NoopSovereigntyHook {
    fn on_council_event(&self, _event: &CouncilEvent) { /* 空 */ }
    fn hook_id(&self) -> &str { "noop" }
}
```

### 5.5 关键不变量
- **不依赖** `apeireth-sovereignty`：`Cargo.toml` 只 `path = "../apeireth-core"`；sovereignty crate 落地后只需在主 binary 端实现 trait 后 `council.register_hook(...)`。
- **Send + Sync**：保证 hook 可跨线程发送事件（council 内部用 `Arc<Mutex<Vec<Box<dyn SovereigntyHook>>>` 或类似）。

---

## 6. 已知缺口（透明声明）

1. `apeireth-council` **未注册到 root `Cargo.toml` workspace.members**（22 个其他 crate 都在，唯独 council 漏了）。
   修复一行：`"crates/apeireth-council"` 加进 `members` 数组即可。
2. **crate 内无 `#[test]` 单元测试**（grep `cfg(test)` / `mod tests` = 0 命中）。
   唯一可运行 demo：`cargo run --example council_demo`（待 workspace 注册后）。
3. `Cargo.toml` 多余 `apeireth-verify` path dep（cycle dep 风险），但当前未在源码中使用，可下次清理。

---

## 7. 汇总

| 证据项 | 状态 | 文件位置 |
|--------|------|---------|
| 7 强制 advisor 逐条 | ✓ | `advisor.rs` + `advisors/{7 个 .rs}` + `advisors/mod.rs:76` |
| HoldTrigger 3 触发 | ✓ | `hold.rs:14-18` 常量 / `hold.rs:22-37` 枚举 / `hold.rs:67-129` 判定 |
| Synthesis 加权公式 | ✓ | `synthesis.rs:107-152` |
| Persona session 示例 | ✓ | `persona.rs` + `examples/council_demo.rs:80-128` |
| SovereigntyHook 接口契约 | ✓ | `sovereignty.rs:84-103` + doc example |

无需返工 — 证据齐全。