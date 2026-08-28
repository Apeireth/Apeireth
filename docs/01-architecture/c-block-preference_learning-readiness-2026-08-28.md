# C 块 preference_learning 真实施 readiness (2026-08-28)

**作者**: Sub-Agent (主代理 Mavis 派)
**用途**: 给主代理 C 块 preference_learning 真实施决策参考

## 1. v1 真实现 (TopicPredictor + PreloadChannel)

源: `legacy/donor/apeireth-companion/src/proactive_memory.rs` (919 行, 0 LLM, 启发式).

**TopicPredictor** (L225-258, 纯函数 `predict_topic(cue)`):
- 输入 `TopicCue { recent_user_messages, recent_assistant_messages, now, user_mood }`
- 输出 `TopicPrediction { hints: Vec<TopicHint { topic: &'static str, confidence: f32 }> }`
- 三路信号聚合 (BTreeMap merge + 同 topic 取 max 不 sum + sort by confidence desc + topic name asc, **严格确定性**):
  1. **关键词路**: `TOPIC_KEYWORDS` 30+ 条 (L99-135) — "考试/备考/线代/高数/作业/课题" → exam_prep/study; "项目/部署/bug/代码/commit" → project; "累/烦/难过/孤独/陪我/抱抱" → companion; "计划/安排/约定/明天" → plan; "股票/基金/仓位/行情" → invest; "日记/反思/回顾" → reflection. 命中: `(n × 0.35).min(0.6)` (L154-166).
  2. **时间锚路**: `TIME_ANCHORS` 3 条 (L138-143) — (6,9,morning_briefing) / (21,24,evening_recap) / (0,6,late_night_checkin); 周末 morning_briefing 加权 0.35, 否则 0.25.
  3. **情绪锚路**: `MOOD_ANCHORS` 5 条 (L146-152) — low/sad/tired → companion (0.4); high/excited → study (0.4).
- 0 LLM, 0 NLP 库, 0 chrono::Utc::now() 隐式 (now 显式传入).

**PreloadChannel trait + 4 impl** (L273-441):
- 接口 `fetch(topics, candidates: &[MemoryCandidate], top_k) -> Vec<MemoryCandidate>`. `MemoryCandidate { content, timestamp: i64, importance: u8 }` (L266-270).
- **KeywordChannel** (L285-338): 反查 TOPIC_KEYWORDS 拿话题→关键词列表, substring 命中, 排序 (hit 数 desc → importance desc), 截 top_k. 无映射时 fallback 用话题键作为关键词.
- **TimeChannel** (L341-362): `within_secs: i64` (兜底, 实际按 timestamp desc 排序截 top_k; since 无 anchor 时间, within_secs 仅记录).
- **ImportanceChannel** (L365-391): `threshold: u8` 过滤 (≥threshold), 排序 (importance desc → timestamp desc).
- **CompositeChannel** (L394-430): 多道并行拉 → HashSet 按 content 去重 → 截 top_k. `default_composite_channel()` (L433-441): keyword + time(3600s) + importance(≥8).

**入口与渲染**:
- `render_proactive_content(entries, max_chars)` (L471-493): 编号列表 + 反幻觉尾注 ("仅当用户提到上述话题时引用; 不主动说「我记得」"), 严格截断.
- `build_proactive_block(cue, candidates, channel, max_chars)` (L503-520): 主入口 → `ProactiveBlock { block: ContextBlock::new("proactive", ...) + topics }`.
- `recommend_proactive_cap(total_budget)` (L523-531): 总预算 1/4, 钳位 [400, 2000]; <400 → 0.

**哲学纪律**: 主动预载 (W4) vs 被动检索 (N7 morphology) 在 ContextAssembler 6000 chars 总预算相遇; 主动块 `core=false` 默认 cap 1500; **0 改 ContextAssembler 既有 API** (L24, L856-901 测试严守向后兼容).

## 2. v2 真实施 mapping (1:1 翻译表)

| v1 类型/方法 | v2 对应 | 翻译纪律 |
|---|---|---|
| `proactive_memory.rs` (整模块) | **新 crate `crates/engine/preference_learning/`** | 独立 crate, workspace member, 0 改现有 |
| `TopicCue` | `PreferenceLearningInput` (或 `OrganInput` 复用) | 字段 1:1; `now: NaiveDateTime` → `at_ms: i64` 显式 (per F6 同模式) |
| `TopicHint { topic: &'static str, confidence: f32 }` | `Topic { key: String, confidence: f32 }` | serde rename, 0 静态字符串; 1:1 字段 |
| `TopicPrediction { hints }` | `Vec<Topic>` | 0 装 wrapper struct |
| `TOPIC_KEYWORDS` / `TIME_ANCHORS` / `MOOD_ANCHORS` const | 同 3 表 (Vec<(String,String)>) | 1:1 翻译, 内置默认 + 允许 override |
| `predict_topic(&TopicCue) -> TopicPrediction` | `TopicPredictor::predict_topics(&PreferenceLearningInput)` | 算法骨架 1:1 (BTreeMap merge + sort_by + max-not-sum), 0 LLM |
| `TopicPrediction::top_topics(k) / primary()` | `Vec<Topic>::top_topics(k) / primary()` | 1:1 翻译 |
| `MemoryCandidate { content, timestamp, importance }` | **复用 R11 `Episode`** 主路径核心类型 | 1:1 字段映射 (content→content/text, timestamp→created_at, importance→importance) |
| `PreloadChannel` trait | `PreloadChannel` trait (同模式) | `fetch(topics: &[String], episodes: &[Episode], top_k) -> Vec<Episode>` |
| `KeywordChannel` | `KeywordChannel` | substring 命中 1:1; 反查 keywords_for_topic |
| `TimeChannel { within_secs }` | `TimeChannel { within_secs }` | 1:1; 字段保留 (供 future anchor 时间窗) |
| `ImportanceChannel { threshold }` | `ImportanceChannel { threshold }` | 1:1; 默认 threshold=8 (主人惯例) |
| `CompositeChannel { channels }` + `default_composite_channel()` | 同 (含默认 keyword + time(3600) + importance(≥8)) | 1:1; 按 `Episode::id` 去重 (R11 schema) 而非 content |
| `render_proactive_content` | `render_preference_evidence` | 编号 + 反幻觉尾注 ("不主动说「我记得」") 1:1 |
| `build_proactive_block` | `PreferenceLearningOrgan::process` (OrganTrait) | 1:1; 注入 OrganInput/OrganOutput |
| `recommend_proactive_cap` | `recommend_preference_cap` | 1/4 + [400,2000] 钳位 1:1 |
| **写入路径**: v1 0 写入 (只返 candidates) | v2 **organ 返 `topics + preloaded`, 不直写 PreferenceStore** | 0 装 PASS (per ledger L30 "no implicit preference mutation"); 写入交 cognitive module AfterTurn hook |

## 3. R15 spec 跟 v1 一致性 verify

R15 spec §1.2 1:1 翻译表 (L73-80) 跟 v1 真实现核验:

| R15 spec 翻译条目 | v1 真实现 | 一致性 |
|---|---|---|
| `predict_topic` → `predict_topics` | L225 `pub fn predict_topic` | ✅ 函数名 1:1 翻译 (复数因返 Vec) |
| `TopicHint { topic, confidence }` → `Topic { key, confidence }` | L57-63 | ✅ 字段 rename 1:1 |
| `PreloadChannel` trait + 4 impl 1:1 | L273 / 285 / 341 / 365 / 394 | ✅ 4 impl 全列 (含 CompositeChannel) |
| `MemoryCandidate` → `Episode` | L266-270 vs R11 Episode | ✅ 字段映射合理, 需 R20 核 R11 Episode 字段名 (content/text + created_at + importance) |
| `chrono::Utc::now()` → `at_ms: i64` 显式 | v1 L191 `time_topic(now)` 已接 `NaiveDateTime` 不调 Utc; spec 措辞略不准但意图对 | ⚠ 微差: v1 是 `NaiveDateTime` 已显式; spec 标 "v1 Utc::now 隐式" 不准, **R20 应核** — 不影响翻译方向, 仅描述修正 |
| 0 LLM (`llm_factory()` 返 None) | v1 L12-14 文档明示 0 LLM | ✅ |

**结论**: R15 spec §1.2 翻译表 6 行全 1:1 准确, 1 处措辞微差 (Utc 显式性), 不影响实施.

## 4. v2 真实施需要新建文件 + 依赖

**新建 crate**: `crates/engine/preference_learning/` (workspace member)

**文件清单**:
- `Cargo.toml` — apeireth-plugin, apeireth-core, serde, serde_json, tokio, async-trait, chrono (workspace 已有, **0 新外部 dep**)
- `src/lib.rs` — pub use 7 项
- `src/topic_predictor.rs` — TopicPredictor + 3 const 表 1:1 翻译
- `src/preload_channel.rs` — trait + 4 impl
- `src/preference_learning_organ.rs` — OrganTrait::process (per `crates/foundation/plugin/src/organ.rs`)
- `src/render.rs` — render_preference_evidence
- `tests/` — 3 单元 (topic_predictor) + 4 单元 (preload_channel) + 1 集成 (organ) = 估 7-10 tests

**依赖** (Cargo.toml):
```toml
apeireth-plugin = { path = "../../foundation/plugin" }   # OrganTrait / PreferenceStore / UserPreference
apeireth-core = { path = "../../foundation/core" }        # SessionId / Episode (R11)
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }                            # 仅 NaiveDateTime 类型, 不调 Utc
```

**0 新外部 dep** → `Cargo.lock` 0 行 diff.

**跟现有 crate 耦合**:
- **写** `apeireth-plugin/src/organ.rs`: 加 `OrganKind::PreferenceLearning` variant + `OrganOutput::PreferenceLearning { topics, preloaded }` variant (R10 spec 决策点, **R20 待 R10 决后实施**; 加 variant 不改现有 9 个, LOCKED 严守).
- **写** `crates/runtime/.../ProductionCognitiveModules` (per ledger L9-18): 加新 slot 注册; 注册顺序建议 `AfterTurn: self_assessment → memory_writeback → preference_learning` (per spec §3.2 决策 1).
- **不写** PreferenceStore SQLite impl (RC-3 已就位 per `crates/engine/memory/src/preference_store_sqlite.rs`); 只在 cognitive module 集成时调 `PreferenceStore::record(&UserPreference { ... })`.
- **0 改** `cognitive-module-wiring.md` (12 slot ledger 改 L30 `DEFERRED` → `WIRED` 是 doc-only 1 行, 但 spec §7.2 标 "0 改 ledger" — **矛盾, R20 应核**: 实际激活时 ledger L30 状态标必须改, 这是文档 sync 不是代码触碰).

## 5. 跟 12 slot ledger 集成 (registration order)

当前 ledger (L22-43):
```
TurnStart:          memory_recall -> preference_recall
AfterModelResponse: judge -> council
AfterTurn:          self_assessment -> memory_writeback
```

`preference_learning` 是**写入侧** (vs `preference_recall` 读取侧, WIRED @ TurnStart). R15 spec §3.2 建议挂 `AfterTurn`, 与 `memory_writeback` 并列:

```
AfterTurn: self_assessment -> memory_writeback -> preference_learning
```

**理由**:
1. 写入路径应在 turn 成功结束 (per `memory_writeback` 模式 "successful final turn only").
2. TopicPredictor 需要 `recent_user_messages` + `recent_assistant_messages`, AfterTurn 时已收齐.
3. organ 返 topics + preloaded, cognitive module 拿到后调 `PreferenceStore::record(&UserPreference { stance, evidence_refs, confidence })` — **不**让 organ 自己写.

**0 装 PASS**: organ 写 prefs 是 "implicit preference mutation" (ledger L30 红线), 必须由 cognitive module 调, organ 0 写入. 这是 R15 §3.2 决策 3 独立判断, 我同意.

**跟 preference_recall 关系**: learning 写, recall 读. 两 slot 共享同一 `Arc<dyn PreferenceStore>` (per RC-3 已就位). 时序: TurnStart recall (已有) → ... → AfterTurn learning (新). 0 竞争 (recall 读 snapshot, learning 写新 row, INSERT OR REPLACE by id).

## 6. 0 触碰 LOCKED 验证

| LOCKED 项 | R20 真实施触碰? |
|---|---|
| 9 哲学锚本体 (`eight_anchors.rs:58-79`) | ❌ 0 改 (新 crate 独立) |
| 13 键 (`philosophy.rs:142`) | ❌ 0 改 (preference ≠ verdict cache, per `preference.rs:12-13`) |
| 3 项不可变脊柱 (`onion.rs:249`) | ❌ 0 改 |
| `workspace.version` (Cargo.toml:43) | ❌ 0 改 (新 crate 用 `version.workspace = true`) |
| R11 baseline (0.8682/0.8532/0.9063) | ❌ 0 改 |
| 9 organ trait (`organ.rs:70-89`) | ❌ 0 改现有; **加新 variant `OrganKind::PreferenceLearning` + `OrganOutput::PreferenceLearning`** (R10 spec 决策, 加 variant 不破 9 现有) |
| `preference.rs` trait | ❌ 0 改 (F6 真实现已就位) |
| `preference_store_sqlite.rs` impl | ❌ 0 改 (RC-3 已就位) |
| 12 slot ledger doc | ⚠ 需 1 行改 L30 `DEFERRED → WIRED` (doc-only, R15 spec §7.2 标 "0 改" 跟这冲突, R20 应核) |
| `Cargo.lock` | ❌ 0 行 (0 新外部 dep) |

**净触碰**: 1 新 crate (含 6 文件) + 加 2 variant 到 `organ.rs` + 1 行 ledger doc sync. **0 触碰** 9 哲学锚 / 13 键 / 3 脊柱 / workspace.version / R11 baseline / preference trait+impl / Cargo.lock.

## 7. 估时真账

R15 spec 估 2 周 (10 工作日, 1 人). 真实施拆账:

| 步骤 | 内容 | 估时 |
|---|---|---|
| 1 | 新建 crate + Cargo.toml + workspace member + lib.rs | 1 天 |
| 2 | TopicPredictor 1:1 翻译 (含 30+ 关键词表 + 3 时间锚 + 5 情绪锚) | 2 天 |
| 3 | PreloadChannel trait + 4 impl + CompositeChannel + 默认 | 2 天 |
| 4 | PreferenceLearningOrgan::process (OrganTrait) + R10 variant 加 (若 R10 已决) | 1 天 |
| 5 | render_preference_evidence + recommend_cap | 0.5 天 |
| 6 | 集成 cognitive module (注册新 slot, AfterTurn hook 调 PreferenceStore::record) | 1 天 |
| 7 | 7-10 单元测试 + 1 集成测试 | 1.5 天 |
| 8 | 0 触碰 LOCKED 核验 (git diff / cargo test / cargo clippy) + commit msg | 1 天 |
| **合计** | | **10 工作日 (2 周)** ✅ 跟 spec 一致 |

**派谁**:
- **推荐派 R20** (sub-agent) — R15 spec 路径明确, 1:1 翻译 0 模糊, sub-agent 真做风险可控; 主代理核验 5 项 LOCKED.
- **不推荐主代理亲做** — 主代理应保留给 frontend 对接 + OrganOrchestrator 真实施 (R12 跑中, 1-3 周) + R10 OrganKind 新 variant 决策; preference_learning 是低风险子任务, 适合 sub-agent.
- **前置**: R12 OrganOrchestrator 真实施 优先 (R20 可与 R12 部分并行, 新 crate 0 改 cognitive.rs); R10 OrganKind 决策可在 R20 实施第 4 步前到位即可.

## 8. 风险 + 阻塞

**R1 (技术, 中)**: R11 `Episode` 字段名未在 spec 标 (R15 spec §1.2 只说 "用 Episode"). R20 实施前**必读** R11 Episode 定义 (估 `crates/runtime/...` 或 `crates/foundation/core/src/episode.rs`) 核字段: `content` vs `text`, `timestamp` vs `created_at`, `importance` 字段是否在. 若字段名错, fetch 逻辑需 adapter. **缓解**: R20 第一天先读 R11 Episode 定义, 再开始 PreloadChannel impl.

**R2 (估时, 低)**: spec 估 2 周是乐观 (10 工作日, 含测试 + 核验). 实际可能因 R11 Episode 适配 + 集成 cognitive module + 跨 crate 测试遇问题 → 2.5-3 周. 留 20% buffer.

**R3 (接力, 中)**: R10 OrganKind 新 variant 决策未出 (R15 spec §3.2 决策 1 留 R10). 若 R10 延期, R20 第 4 步阻塞. **缓解**: R20 前 3 步可与 R10 并行 (TopicPredictor + PreloadChannel + render 不依赖 OrganKind); 第 4 步等 R10 决后 1 天接上.

**R4 (主代理手动 vs sub-agent, 低)**: sub-agent 真实施偏好类功能风险低 (1:1 翻译, 0 LLM, 0 新外部 dep), 主代理监督即可; 但 commit message 必含 "1:1 翻译 v1, 0 LLM, 0 触碰 LOCKED, 0 装诱导 prevention" 4 项标 — R20 模板化 commit msg 防漏.

**R5 (ledger doc sync, 低)**: R15 spec §7.2 标 "0 改 ledger", 但真实施时 L30 状态 DEFERRED→WIRED 是必改. 这是 spec 描述不严, **R20 实施前主代理或 R20 自己在 spec §7.2 加 1 行标 "L30 状态标必改 (1 行 doc sync)"**.

**阻塞**: 无硬阻塞. 软依赖: R10 OrganKind 决策 (1 周内应出) + R12 OrganOrchestrator (并行).

**主代理决策建议**:
1. **派 R20 真实施**, 不主代理亲做 (保留精力给 R12 + frontend).
2. **R20 任务 brief 必含**: 5 项 LOCKED + ledger L30 doc sync 例外 + R11 Episode 字段预读 + commit msg 4 项标.
3. **不预先派 R21-R24** (R15 spec §5 估 6-10 周总, 但 R16-R19 spec 还未写, 派单顺序 R21 (critic) → R22 (reflection) → R23 (planner, LLM 重建非 1:1) → R24 (orchestrator, 区分 R12)).
4. **R15 spec 措辞微差** (Utc 显式性) R20 实施时按 v1 真实现为准, 不影响翻译.