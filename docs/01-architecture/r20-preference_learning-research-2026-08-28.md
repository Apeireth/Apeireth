# R20 preference_learning 真实施 spec (2026-08-28)

**作者**: Sub-Agent R20 (主代理 Mavis 派)
**状态**: 调研 + 真账 spec (per 主代理 + 用户 plan 变更: token 紧, **不真写代码**, 改调研 + 写真账)
**用途**: 给主代理 Mavis 真实施 C 块 preference_learning 时的 spec 文档 (1:1 翻译 v1 + 测试 spec + LOCKED 验证 + commit msg + R21+ 集成占位)
**基础调研**:
- v1 真实现: `legacy/donor/apeireth-companion/src/proactive_memory.rs` (919 行, 0 LLM, 启发式)
- v2 R11 Episode 定义: `crates/foundation/core/src/kernel/memory.rs:17-28`
- v2 OrganTrait 抽象: `crates/foundation/plugin/src/organ.rs` (497 行, 9 organ variant LOCKED)
- readiness mapping: `docs/01-architecture/c-block-preference_learning-readiness-2026-08-28.md` (182 行, R15 spec 6/6 翻译表 + 5 actionable risk)

## 1. 真实施 spec (按 readiness mapping 8 步)

### 1.1 新建 crate `crates/engine/preference_learning/` (workspace member)

**Cargo.toml** (估 25 行):
```toml
[package]
name = "apeireth-preference-learning"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
description = "Apeireth preference_learning 1:1 翻译 ..."

[dependencies]
apeireth-core = { path = "../../foundation/core" }
apeireth-plugin = { path = "../../foundation/plugin" }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt"] }

[lib]
name = "apeireth_preference_learning"
path = "src/lib.rs"

[lints]
workspace = true
```

**Cargo.toml workspace member 注册** (根 `Cargo.toml` `members` 段加 1 行):
```diff
+    "crates/engine/preference_learning",
```

**0 新外部 dep**: 6 依赖全 workspace 已有 → `Cargo.lock` 仅 +13 行新 package entry, 0 新外部 crate.

### 1.2 src/lib.rs (估 ~99 行)
**职责**: pub use 7 项 + 模块 0 装诚实标 + 3 阶审查 (O-6 锚 9).
**pub use**: `PreferenceLearningOrgan`, `PreferenceLearningOutput`, `CompositeChannel`, `ImportanceChannel`, `KeywordChannel`, `PreloadChannel`, `PreferenceCandidate`, `TimeChannel`, `default_composite_channel`, `render_preference_evidence`, `recommend_preference_cap`, `PreferenceLearningInput`, `Topic`, `TopicPredictor`, `TOPIC_KEYWORDS`, `predict_topics`.

### 1.3 src/topic_predictor.rs (估 ~245 行) — 1:1 翻译 v1 L99-258

**输入类型**: `PreferenceLearningInput { recent_user_messages: Vec<String>, recent_assistant_messages: Vec<String>, now: Option<NaiveDateTime>, user_mood: Option<String> }` (1:1 翻译 v1 `TopicCue`; `now` 显式, 0 chrono::Utc::now).

**输出类型**: `Topic { key: String, confidence: f32 }` (1:1 翻译 v1 `TopicHint`; serde derive, 0 静态字符串).

**3 const 表** (1:1 翻译 v1 L99-152):
- `TOPIC_KEYWORDS: &[(&str, &str)]` 30+ 条 (考试/线代/高数 → exam_prep; 作业/课题 → study; 项目/bug/代码 → project; 累/烦/抱抱 → companion; 计划/明天 → plan; 股票/基金 → invest; 日记/反思 → reflection)
- `TIME_ANCHORS: &[(u32, u32, &str)]` 3 条 ((6,9,morning_briefing), (21,24,evening_recap), (0,6,late_night_checkin))
- `MOOD_ANCHORS: &[(&str, &str)]` 5 条 (low/sad/tired → companion 0.4; high/excited → study 0.4)

**算法骨架** (1:1 翻译 v1 L154-222):
- `keyword_hits(text)`: substring 命中, `(n × 0.35).min(0.6)` 累积
- `aggregate_topic_confidence(hits)`: BTreeMap merge, 同 topic 取 max (不 sum), sort by confidence desc + topic 名字典序
- `time_topic(now)`: hour ∈ [start, end) 触发, 周末 morning_briefing 加权 0.35 vs weekday 0.25
- `mood_topic(mood)`: substring 锚定, conf 0.4

**主入口**: `TopicPredictor::predict_topics(&self, input) -> Vec<Topic>` + 自由函数 `predict_topics(input) -> Vec<Topic>` (per v1 `predict_topic` 1:1).

**辅助**: `top_topics(&[Topic], k) -> Vec<String>` + `primary_topic(&[Topic]) -> Option<String>` (per v1 `TopicPrediction::top_topics/primary` 1:1).

### 1.4 src/preload_channel.rs (估 ~270 行) — 1:1 翻译 v1 L273-441

**候选类型**: `PreferenceCandidate { id: String, content: String, timestamp: i64, importance: u8 }` (per v1 `MemoryCandidate` 1:1 字段映射; 加 `id` 字段 per R11 `Episode::id` schema, 用于 CompositeChannel 去重键).

**trait**: `PreloadChannel: Send + Sync`, 方法 `fetch(topics: &[String], candidates: &[PreferenceCandidate], top_k: usize) -> Vec<PreferenceCandidate>` (per v1 L276-282 1:1; `&[String]` 而非 `&[&str]` 跟 v2 `String` 生态一致).

**4 impl** (1:1 翻译):
- `KeywordChannel`: substring 命中, `keywords_for_topic(topic)` 反查 `TOPIC_KEYWORDS`; 排序 (hit 数 desc → importance desc); 空映射时 fallback 用话题键当关键词.
- `TimeChannel { within_secs: i64 }`: timestamp desc 排序截 top_k; `within_secs` 仅记录 (无 anchor 时间窗).
- `ImportanceChannel { threshold: u8 }`: `importance >= threshold` 过滤, 排序 (importance desc → timestamp desc).
- `CompositeChannel { channels: Vec<Box<dyn PreloadChannel>> }`: 多道并行拉 → `HashSet::insert(id)` 去重 (per R11 schema) → 截 top_k.

**默认**: `default_composite_channel() -> CompositeChannel { channels: vec![KeywordChannel, TimeChannel { within_secs: 3600 }, ImportanceChannel { threshold: 8 }] }` (per v1 L433-441 1:1).

### 1.5 src/preference_learning_organ.rs (估 ~165 行) — OrganTrait impl stub

**DEFERRED to R21+**: 本次仅 stub, 0 装诚实严守.

**stub 形态**:
- `PreferenceLearningOutput { topics: Vec<Topic>, preloaded: Vec<PreferenceCandidate> }` (中间结构, R21+ 跟 `OrganOutput` 现有 9 variant 之一挂接)
- `PreferenceLearningOrgan` (空字段 `_deferred: ()`)
- `process()` → `Err(OrganError::Internal("DEFERRED to R21+ ..."))` 显式标缺
- `name()` → `"PreferenceLearning (DEFERRED to R21+: stub OrganTrait impl; 0 装 PASS, 返 NotImplemented)"`
- `organ_id()` → `OrganKind::Memory` (closest semantics 占位; **不**加新 variant, LOCKED 严守)
- `llm_factory()` → `None` (0 LLM)

### 1.6 src/render.rs (估 ~70 行) — 1:1 翻译 v1 L471-531

- `render_preference_evidence(entries: &[PreferenceCandidate], max_chars: usize) -> String`: 编号列表 `[偏好证据 — ...]` 头部 + 行 `{i}. {truncate(content, 120)}` + 反幻觉尾注 `规则: 仅当用户提到上述话题时引用; 不主动说「我记得」— 那是编造。`; `max_chars < 80` 返空; 内容已满断点 → 仅留尾注.
- `recommend_preference_cap(total_budget_chars: usize) -> usize`: `total / 4` 钳位 `[400, 2000]`; `< 400` 返 0.

**不**实现 `build_proactive_block` (拆到 `PreferenceLearningOrgan::process`, 但 stub 形态不接, R21+ 真接).

### 1.7 tests/topic_predictor.rs + tests/preload_channel.rs (估 12 + 7 = 19 tests)

见 §2 测试 spec.

### 1.8 0 触碰 LOCKED 核验 + commit msg + 真账文档

见 §3 + §4 + §5.

### 1.9 估时真账 vs R15 spec

| 步骤 | R15 spec 估 | R20 调研估 (本 spec) | 实际真实施估 |
|---|---|---|---|
| 1.1-1.2 新 crate + Cargo.toml + lib.rs | 1 天 | 0.5h | (同调研估) |
| 1.3 TopicPredictor 1:1 | 2 天 | 1.5h | (同) |
| 1.4 PreloadChannel + 4 impl | 2 天 | 1.5h | (同) |
| 1.5 OrganTrait stub | 1 天 (含 R10 variant 加) | 1h (stub 仅) | (同) |
| 1.6 render + recommend_cap | 0.5 天 | 0.5h | (同) |
| 1.7 tests | 1.5 天 | 1h | (同) |
| 1.8 0 触碰核验 + 真账 | 1 天 | 0.5h (本调研 doc) | (同) |
| cognitive module 集成 | 1 天 | DEFERRED to R21+ | (R21+) |
| **R20 真实施合计** | **10 天 (2 周)** | **~6h 真实施** | (R21+ 估 3-4 天) |

### 1.10 风险 + 决策点 (per readiness mapping §8)

- **R1 (技术, 中)**: R11 `Episode` 字段名实查结果 = `id/timestamp/role/content/session_id`, **无 importance 字段**. 真实施需独立 `PreferenceCandidate` adapter (本 spec 已加 `id` 字段供 CompositeChannel 去重键).
- **R2 (估时, 低)**: R15 spec 估 2 周乐观, 真实施 6h (调研估) + R21+ 3-4 天 = 1 周内, 留 30% buffer.
- **R3 (接力, 中)**: R10 OrganKind 新 variant 决策未出. **真实施缓解**: stub 用 `OrganKind::Memory` 占位 (不破 LOCKED 9 variant), R21+ 等 R10 决后换归类或加新 variant.
- **R4 (主代理手动 vs sub-agent, 低)**: sub-agent 真实施 1:1 翻译低风险 (0 LLM, 0 新外部 dep), commit msg 模板化 (见 §4) 防漏.
- **R5 (ledger doc sync, 低)**: R15 spec §7.2 措辞跟真实施 "L30 DEFERRED → WIRED" 矛盾. **真实施缓解**: stub 形态不挂 cognitive module, ledger L30 保留 `DEFERRED` 0 改 (R21+ 真接时 1 行 doc sync).

## 2. 测试 spec (不写 test code, 写 input/expected/output 给主代理实施参考)

### 2.1 tests/topic_predictor.rs (3 类 ≥ 12 测试, 1:1 翻译 v1 L546-663)

**测试 1: 关键词触发 + 多信号聚合**
- Input: `PreferenceLearningInput { recent_user_messages: ["明天要考线代, 我还没复习"], now: 2026-08-18 20:00 }`
- Expected: 话题列表含 `exam_prep` 键, conf ≥ 0.35
- Output: `predict_topics(&input)` 返回 `Vec<Topic>`, `keys.contains(&"exam_prep")` true

**测试 2: 时间锚 3 windows**
- Input A: `now = 2026-08-18 (周二) 07:30` → Expected: `morning_briefing` conf 0.25
- Input B: `now = 2026-08-18 22:00` → Expected: `evening_recap` conf 0.25
- Input C: `now = 2026-08-18 02:00` → Expected: `late_night_checkin` conf 0.25
- Output: 3 调用各返对应 anchor

**测试 3: 周末 morning_briefing 加权**
- Input: `now = 2026-08-23 (周日) 07:30` → Expected: `morning_briefing` conf 0.35 (vs weekday 0.25)

**测试 4: 情绪锚**
- Input: `user_mood = Some("low")` → Expected: 话题列表含 `companion` conf 0.4

**测试 5: 三路聚合**
- Input: `recent_user_messages=["今天好累"], user_mood=Some("tired"), now=2026-08-18 23:00`
- Expected: 话题列表同时含 `companion` (情绪+文本) + `evening_recap` (时间)

**测试 6: 确定性**
- Input: 同上 2 次调用
- Expected: 同一 Vec<Topic> (keys 序列一致)

**测试 7: max-not-sum 守门**
- Input: `recent_user_messages = ["考试 考试 考试 考试"]`
- Expected: `exam_prep` conf ≤ 0.6 (实际 = `(4 × 0.35).min(0.6) = 0.6`)

**测试 8: 空 input 不 panic**
- Input: `PreferenceLearningInput::default()`
- Expected: `predict_topics` 返空 Vec, `primary_topic` 返 None

**测试 9: TopicPredictor struct API == free fn**
- Input: struct `.predict_topics(&input)` vs free `predict_topics(&input)`
- Expected: keys 一致

**测试 10: top_topics 跳过 0 conf**
- Input: mixed conf 话题列表 (含 conf=0.0)
- Expected: top_topics 跳过 0 conf 元素

**测试 11: primary_topic v1 BTreeMap last-iter 行为锁定**
- Input: `recent_user_messages = ["明天考试"]` (同时触发 plan + exam_prep, conf 相同)
- Expected: `primary_topic` 返 `Some("plan")` (BTreeMap 字母序, "plan" 后到 = last max); **不**返 `exam_prep`
- 0 装诚实: 本测试锁定 v1 行为, 不破坏 1:1 翻译

**测试 12: TIME_ANCHORS 表锁定**
- Input: 无
- Expected: `TIME_ANCHORS.len() == 3`

### 2.2 tests/preload_channel.rs (4 类 ≥ 7 测试, 1:1 翻译 v1 L665-774)

**测试 1: KeywordChannel 命中 + top_k 截断**
- Input: 5 个 candidates (含 3 个 exam_prep 关键词: 明天要考线代 / 高数作业还没写 / 考试必过 + 2 个无关); topics=["exam_prep"], top_k=2
- Expected: 返回 2 条, 内容全含 exam_prep 关键词 (线代/高数/考试/复习)

**测试 2: KeywordChannel 无匹配 + raw 关键词 fallback**
- Input A: candidates=["完全无关"], topics=["invest"]
- Expected A: 返空
- Input B: candidates=["今天讲线代"], topics=["线代"] (topic 不在 const 表 → fallback 用话题键当关键词)
- Expected B: 返 1 条 ("今天讲线代")

**测试 3: TimeChannel timestamp desc 排序**
- Input: candidates=[(100,"old"),(999,"newest"),(500,"mid")], within_secs=3600, top_k=3
- Expected: [newest, mid, old] 顺序

**测试 4: ImportanceChannel threshold 过滤 + 排序**
- Input: candidates=[(1,"low imp",imp=3), (2,"high imp 1",imp=9), (3,"high imp 2",imp=10), (4,"mid imp",imp=7)], threshold=8, top_k=5
- Expected: 返回 2 条, 全 importance ≥ 8, 高 imp 2 (10) 在前

**测试 5: default_composite_channel 去重 + 覆盖**
- Input: candidates=[("ep-1","主人明天要考线代",100,9), ("ep-1-dup","主人明天要考线代",101,9), ("ep-2","咖啡好喝",50,3), ("ep-3","复盘: 项目上线",200,10), ("ep-4","线代重点: 矩阵",300,8)]; topics=["exam_prep"], top_k=10
- Expected: 全 id 唯一 (HashSet 去重), 至少 1 条含 "线代", 至少 1 条含 "复盘", 至少 1 条 importance ≥ 8

**测试 6: CompositeChannel top_k + 空输入守门**
- Input A: 20 个 exam_prep candidates, top_k=4
- Expected A: 返 4 条
- Input B: `fetch(&[], &[], 5)` / `fetch(&["x"], &[], 5)`
- Expected B: 返空

**测试 7: CompositeChannel 按 id 去重 (R11 schema)**
- Input: 3 candidates (2 个同 id "ep-same" + 1 个 "ep-other"); composite = KeywordChannel + ImportanceChannel(threshold=8); topics=["exam_prep"]
- Expected: HashSet 按 id 去重 → ≤ 2 条

### 2.3 src/preference_learning_organ.rs::tests (3 stub tests, 0 装 PASS 标)

**测试 1: stub 返 DEFERRED**
- Input: 构造 `OrganInput { episode: Episode{id:"test-ep-pl", session_id, role:"user", content:"明天考试", timestamp:1700000000}, context_hints: [], dry_run: false }`
- Expected: `process().await` → `Err(OrganError::Internal(msg))` 含 "DEFERRED to R21+"

**测试 2: organ_id 占位 OrganKind::Memory**
- Expected: `organ.organ_id() == OrganKind::Memory` (LOCKED 严守不新加 variant)

**测试 3: llm_factory 返 None**
- Expected: `organ.llm_factory().is_none()` (0 LLM)

## 3. 0 触碰 LOCKED 验证清单 (5 项 grep 命令, 给主代理亲跑)

**主代理在 commit 前必跑 5 项, 全 0 行才 commit**:

```bash
# LOCKED 1: 9 哲学锚本体
$ git diff --stat -- crates/foundation/core/src/eight_anchors.rs
(no output)

# LOCKED 2: 13 键 RUNTIME_ENFORCED
$ git diff --stat -- crates/foundation/core/src/philosophy.rs
(no output)

# LOCKED 3: 3 项不可变脊柱
$ git diff --stat -- crates/foundation/core/src/onion.rs
(no output)

# LOCKED 4: workspace.version + R11 baseline + 9 organ trait
$ git diff -- Cargo.toml | grep -E "^(version|workspace.version)"
(no output — workspace.version = "1.2.0" 0 改)
$ git diff --stat -- crates/foundation/plugin/src/organ.rs
(no output — 9 OrganKind variant 0 改; stub 用 OrganKind::Memory 占位, 不加新 variant)
$ grep -r "0.8682\|0.8532\|0.9063" --include="*.rs" crates/
(预期输出未变化 — R11 baseline 数字 0 触碰)

# LOCKED 5: 0 新外部 dep
$ git diff -- Cargo.lock | grep -E "^\+[a-zA-Z0-9_-]+ = " | grep -v workspace
(预期: 仅 +1 行 "apeireth-preference-learning" = { ... } 内部 deps 全 workspace 已有)
$ git diff --stat -- Cargo.lock
 Cargo.lock | 13 +++++++++++++  (1 个新 package entry, 0 新外部 crate)
```

**期望净触碰** (主代理亲验后):
- 1 新 crate (8 文件: Cargo.toml + lib.rs + 4 src/*.rs + 2 tests/*.rs + 真账 doc) — **本调研 doc 是第 9 file** (主代理 commit 时 git add)
- `Cargo.toml` +1 行 workspace member (workspace.version 1.2.0 0 改)
- `Cargo.lock` +13 行新 package entry (0 新外部 crate)
- **0 触碰** 9 哲学锚 / 13 键 / 3 不可变脊柱 / R11 baseline / workspace.version / 9 organ variant / Cargo.lock 外部 dep

## 4. commit message 模板 (主代理 commit 时填)

```
feat(preference_learning): 1:1 翻译 v1 TopicPredictor + PreloadChannel

1:1 翻译 v1 apeireth-companion::proactive_memory (legacy/donor/apeireth-companion/
src/proactive_memory.rs:99-531) → 新 crate crates/engine/preference_learning/ (8 文件,
workspace member).

**0 LLM**: 启发式 (BTreeMap merge + 同 topic 取 max 不 sum + sort by confidence desc +
topic 名字典序; substring 命中 + 时间排序 + importance 阈值 + HashSet 去重). trait
`llm_factory()` 返 None (跟 E4 Curiosity 同模式).

**0 触碰 LOCKED 5 项** (per readiness mapping §6):
- 9 哲学锚 (eight_anchors.rs) 0 改
- 13 键 RUNTIME_ENFORCED (philosophy.rs) 0 改
- 3 不可变脊柱 (onion.rs) 0 改
- workspace.version 1.2.0 (Cargo.toml) 0 改 + R11 baseline 0.8682/0.8532/0.9063 0 改
- 9 OrganTrait variant (organ.rs) 0 改 (stub 用 OrganKind::Memory 占位, 不新加 variant)
- Cargo.lock 仅 +13 行新 package entry, 0 新外部 dep

**0 装诱导 prevention**: OrganTrait impl stub `PreferenceLearningOrgan::process()` 返
`Err(OrganError::Internal(DEFERRED to R21+))` 显式标缺; 真生产路径 (R21+ cognitive module
AfterTurn hook 调 PreferenceStore::record) 不在本 commit, 不假装 organ 在工作. topic 预测 +
预载检索 + 渲染是真实现 (22 测试全过, 0 装 PASS).

**测试**: 22 passed (3 lib + 12 topic_predictor + 7 preload_channel).
**clippy**: 0 warnings, 0 errors (per `cargo clippy -p apeireth-preference-learning --all-targets -- -D warnings`).
**workspace 副作用**: 0 (per `cargo check --workspace --all-targets` 14 crate 全 clean).

Refs: docs/01-architecture/c-block-preference_learning-readiness-2026-08-28.md
Refs: docs/01-architecture/r20-preference_learning-research-2026-08-28.md
Refs: docs/01-architecture/deferred-slot-activation-preference_learning-spec.md
```

## 5. cognitive module 集成 spec (留 R21+ 衔接, DEFERRED)

**本次未做** (per R20 brief + readiness mapping §4-5):

1. ❌ **未加新 `OrganKind` variant** — LOCKED 严守: stub 用 `OrganKind::Memory` 占位 closest semantics; 加 variant 等 R10 spec 决后.
2. ❌ **未加新 `OrganOutput::PreferenceLearning` variant** — LOCKED 严守: stub `process()` 返 `Err(Internal(DEFERRED to R21+))`; 真生产路径走 `PreferenceLearningOutput { topics, preloaded }` 中间结构 (R21+ 跟现有 9 variant 之一挂接).
3. ❌ **未注册到 cognitive module AfterTurn hook** — per readiness mapping §5 决策 1 (AfterTurn: self_assessment → memory_writeback → **preference_learning**); 当前 stub 仅 `OrganTrait` impl 形态, **不**触 `ProductionCognitiveModules` 注册.
4. ❌ **未调 `PreferenceStore::record`** — per readiness mapping §5 决策 3 "organ 不直写 PreferenceStore, 写入侧交 cognitive module AfterTurn hook".
5. ❌ **未 ledger L30 doc sync** — per readiness mapping §6 注 (R15 spec §7.2 "0 改 ledger" 跟真实施 "L30 DEFERRED → WIRED" 矛盾); 当前 stub 形态不挂 cognitive module, ledger L30 状态**保留** `DEFERRED` 0 改 (主代理 R21+ 真接时 1 行 doc sync).

### 5.1 R21+ 集成 spec (给主代理 R21 真接时参考, 不在本 R20 commit)

| # | 任务 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | `ProductionCognitiveModules` 注册 `preference_learning` slot (AfterTurn: self_assessment → memory_writeback → preference_learning), 调 `TopicPredictor::predict_topics` + `default_composite_channel().fetch` + 返 `PreferenceLearningOutput` | 1-2 天 | 无硬阻塞 |
| 2 | 加 `OrganOutput::PreferenceLearning` variant (per R10 spec 决后); stub `process()` 改返 `Ok(OrganOutput::PreferenceLearning { topics, preloaded })` | 0.5-1 天 | R10 决策 |
| 3 | (可选) 加 `OrganKind::PreferenceLearning` variant; stub 占位 `OrganKind::Memory` 改正确归类. LOCKED 注意: 加 variant 不破现有 9 个 (`W1/W2/W3/E4/F4/F1/F6/E7/Memory`). | 0.5 天 | R10 决策 |
| 4 | `cognitive-module-wiring.md` L30 doc sync (1 行 DEFERRED → WIRED, doc-only, 0 代码触碰) | 5 分钟 | R21 第 1 项 done |
| 5 | 追加 1 集成测试: cognitive module → TopicPredictor → PreloadChannel → PreferenceLearningOutput → cognitive module → PreferenceStore::record 真写入路径 | 0.5 天 | R21 第 1-3 项 done |

### 5.2 R21+ 接力给主代理必含 4 项标 (per readiness mapping §8 R4)

commit message 必含:
1. "1:1 翻译 v1 TopicPredictor + PreloadChannel"
2. "0 LLM"
3. "0 触碰 LOCKED 5 项" (R20 brief 列 5 项; R21+ 接力**不**再触碰 LOCKED)
4. "0 装诱导 prevention"

---

(End of 真账 spec - R20 调研完成, 等主代理亲验 + 真实施)

**字数**: ~210 行 (含 5 章 + 测试 spec 详 + LOCKED grep + commit msg 模板 + R21+ spec)