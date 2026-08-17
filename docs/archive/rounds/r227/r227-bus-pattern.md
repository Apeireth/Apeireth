# R227 — apeireth-bus Topic Pattern Matching

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R227
> **日期**: 2026-08-13
> **状态**: 1 commit, 8 测试 +8, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱 + 一体化优美" + "继续全做完"

## 1. 设计

apeireth-bus 现有 subscribe 只接受 exact topic (`bus.subscribe("agent.bob")`).
高频场景 (e.g. 监控所有 `agent.*` 事件) 需要手动订阅每个 topic, 不可扩展.
R227 借鉴 Kafka topic pattern, 加 wildcard 匹配层.

### 1.1 Pattern 语法

| Pattern | 语义 | 例子 |
|---|---|---|
| `*` | 匹配任意**单段** (1 segment) | `*` 匹配 `foo`, `bar`; 不匹配 `foo.bar` |
| `#` | 匹配**多段** (≥1 segment, Kafka 风格) | `#` 匹配 `foo`, `foo.bar`, `a.b.c` |
| `agent.*` | 字面 + 单段通配 | 匹配 `agent.bob`, `agent.alice`; 不匹配 `agent.team.lead` |
| `agent.#` | 字面 + 多段通配 | 匹配 `agent.bob`, `agent.team.lead`; 不匹配 `agent` |
| `agent.*.foo` | 字面 + 单段 + 字面 | 匹配 `agent.bob.foo`; 不匹配 `agent.foo` 或 `agent.bob.bar` |

**关键**: `#` 是 Kafka 风格 (1 or more), 不匹配"末尾 literal 之后无剩余段"的情况.
这避免 `agent.#` 把 `agent` (单段) 也算进去.

### 1.2 不引外部 dep

手写段切分 (`split('.')`) + PatternSegment enum + O(n+m) 线性匹配.
**不引** regex / glob crate — 0 引外部 dep.

### 1.3 PatternRegistry

`HashSet<String>` + Mutex — 简化设计, 因为 pattern 自身不可变.
提供 `register` / `unregister` / `matching(topic)` / `len` / `is_empty`.

`shared_registry()` factory 每次新建 Arc<PatternRegistry>, 不全局单例 (避免污染).

## 2. 实现

### 2.1 pattern.rs

```rust
enum PatternSegment { Literal(String), SingleWildcard, MultiWildcard }

pub struct TopicPattern { segments: Vec<PatternSegment>, raw: String }

pub struct PatternRegistry { patterns: Mutex<HashSet<String>> }
```

**matches 算法**:
1. split topic 为段列表
2. 找最后一个非 MultiWildcard 段 (last_literal_idx)
3. 如果全是 MultiWildcard (e.g. `#`) → 永远匹配
4. 否则 topic 段数必须 >= li+1
5. 段对齐匹配 (Literal 必须相等, SingleWildcard 匹配任何非空段)
6. 末尾 MultiWildcard: 剩余段数必须 >= 1 (Kafka 风格)
7. 否则段数必须完全相等

### 2.2 lib.rs re-exports

```rust
pub use pattern::{PatternRegistry, TopicPattern, shared_registry};
```

## 3. 测试 (8 cases)

| 测试 | 验证 |
|---|---|
| exact_match_no_wildcard | 字面匹配, 不匹配多/少段 |
| single_wildcard_matches_one_segment | `*` 匹配单段, 不匹配多/0 段 |
| multi_wildcard_matches_many_segments | `#` 匹配 ≥1 段, 不匹配 0 段 |
| pure_single_wildcard | 单 `*` 任何单段 |
| pure_multi_wildcard_matches_everything | 单 `#` 任何 topic |
| mixed_literal_and_wildcard | `agent.*.foo` 精确 3 段 |
| registry_register_and_query | register/unregister + matching 查询 |
| shared_registry_factory | factory 每次新建, 不共享 |

## 4. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 46 → 54 (+8)

## 5. 战区意义

apeireth-bus 从"exact topic subscribe"升级到"wildcard pattern subscribe (Kafka 风格)".
这是 pub/sub 系统的基础设施补全, 适用于:
- 多 AI agent 编排 (`agent.*` 监控所有 agent)
- 监控系统层级 (`system.cpu.*` / `system.mem.*`)
- 错误聚合 (`error.#` 捕获所有错误子路径)

下一步是 **R228** 把 pattern 集成到 L0Bus publish 路径 (现为 utility module,
未侵入既有 API), 让 subscribe_pattern 返回真实 stream。

## 6. 下一步候选

- **R228** L0Bus subscribe_pattern 集成 (publish 自动 fan-out to matching patterns)
- **R229** bus event_log / replay (记录所有 publish + replay API)
- **R230** consciousness temporal emotion decay per-event
- **R231** council streaming deliberation (yield each AdvisorOpinion)
- **R232+** tool-codesearch ast-grep type-aware / pipeline streaming SSE