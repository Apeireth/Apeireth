# R228 — apeireth-bus L0Bus subscribe_pattern 集成

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R228
> **日期**: 2026-08-13
> **状态**: 1 commit, 8 测试 +8, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱 + 一体化优美" + "继续全做完"

## 1. 设计

R227 落地了 `TopicPattern` + `PatternRegistry` utility module, 但未接入 L0Bus.
R228 把 pattern subscribe 集成进 L0Bus, 让 subscribe_pattern 返回真实 stream,
并让 publish 自动 fan-out 到匹配 pattern.

### 1.1 新字段

```rust
pattern_topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<BusMessage<T>>>>>
```

跟现有 `topics` 平行, 但 key 是 pattern 字符串而非 exact topic.

### 1.2 新 API

```rust
// R228
pub async fn subscribe_pattern(&self, pattern: &str) 
    -> BusResult<BoxStream<'static, BusResult<BusMessage<T>>>>;
pub async fn unsubscribe_pattern(&self, pattern: &str) -> bool;
pub async fn pattern_count(&self) -> usize;
```

**语义**:
- 同 pattern 多次 subscribe 时覆盖 (last-wins): 新 Sender 创建, 之前的流停止接收 (因为 Sender 被替换).
- unsubscribe_pattern 移除 entry, 流的 Sender 被 drop, Receiver 收到 Closed, stream.next() 立即返 None.
- pattern_count 反映当前注册的 pattern 数.

### 1.3 publish 路径改造

```rust
// 主路径 (existing): exact topic fan-out
let _ = match self.policy { ... };  // 不可能失败, 每个 arm 返 Ok(())

// R228: pattern fan-out (best-effort)
let pattern_txs: Vec<broadcast::Sender<...>> = {
    let map = self.pattern_topics.read().await;
    map.iter()
        .filter(|(p, _)| TopicPattern::parse(p).matches(topic))
        .map(|(_, tx)| tx.clone())
        .collect()
};
for ptx in pattern_txs {
    if let Err(e) = ptx.send(msg_for_patterns.clone()) {
        eprintln!("[apeireth-bus] pattern send failed: {e}");
    }
}
Ok(())
```

**关键设计**:
- `msg_for_patterns` clone 在 match 前 (msg 会被 tx.send move 走)
- pattern send 失败 best-effort, 仅 eprintln (不阻断主 publish)
- pattern fan-out 在 BackpressurePolicy match 之后, 跟 exact send 独立计数

### 1.4 unsubscribe 后 stream 行为

当 Sender 被 drop, Receiver 收到 `broadcast::error::RecvError::Closed`.
stream.next() 在 Closed 错误时返 None (流结束), 不阻塞.
这意味着 unsubscribe 是干净的: stream 立即可用 `while let Some(...) = stream.next().await` 循环退出.

## 2. 测试 (8 cases)

| 测试 | 验证 |
|---|---|
| r228_01_subscribe_pattern_receives_matching | `agent.*` 接收 `agent.bob` |
| r228_02_subscribe_pattern_no_match_doesnt_receive | `agent.*` 不接收 `system.cpu` |
| r228_03_multi_wildcard_matches_multi_segments | `agent.#` 接收 `agent.team.lead` |
| r228_04_pure_multi_wildcard_receives_all | `#` 接收 `foo` + `foo.bar` |
| r228_05_publish_fans_out_to_multiple_patterns | `agent.bob` 同时被 `agent.*` + `*.bob` 接收 |
| r228_06_pattern_count_tracks | register/unregister 跟踪 |
| r228_07_unsubscribe_pattern_stops_delivery | unsubscribe 后 stream 立即 close (None, 不 timeout) |
| r228_08_exact_and_pattern_both_receive | exact + pattern 同一 topic 都收到 |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 54 → 62 (+8)

## 4. 战区意义

apeireth-bus 从"exact topic pub/sub"升级到"exact + wildcard pattern pub/sub"
(Kafka 风格). 这是 pub/sub 系统的关键基础设施补全, 适用场景:

- **多 AI agent 编排监控**: `agent.*` 一次性订阅所有 agent
- **监控层级**: `system.cpu.*` / `system.mem.*` 分别订阅
- **错误聚合**: `error.#` 捕获所有错误子路径
- **事件溯源**: `event.user.#` 捕获用户事件链

## 5. 下一步候选

- **R229** bus event_log / replay (记录所有 publish + replay API)
- **R230** consciousness temporal emotion decay per-event
- **R231** council streaming deliberation (yield each AdvisorOpinion)
- **R232+** tool-codesearch ast-grep in-process (no CLI dep)