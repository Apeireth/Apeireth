# R248 -- Plutchik intensity adjustment + filtered history views

## Problem
ExtendedEmotionEngine 缺少对 intensity 的运行时调控和带过滤的历史视图。
上层需要:
- 直接 override current_intensity (e.g. 外部情绪输入 / 主人手动校正)
- 在 ordinal 空间里 bump (clamp 到 [Mild, Extreme] 4 档)
- 取最近 N 条历史 (UI 滚动视图)
- 按最低 intensity 过滤历史 (e.g. \"只看 Strong 以上情绪\" 审计)

## Solution

### plutchik.rs
`PlutchikIntensity::ordered_levels()` -- 已有, 复用作为 ordinal 索引源
`PlutchikEmotion::intensity()` -- 已有, 用于 history_min_intensity 内部比较

### plutchik_engine.rs -- 4 个新方法 (在 history() 后)

```rust
pub fn set_intensity(&mut self, intensity: PlutchikIntensity)
pub fn bump_intensity(&mut self, delta: i32) -> PlutchikIntensity
pub fn history_recent(&self, limit: usize) -> Vec<PlutchikEmotion>
pub fn history_min_intensity(&self, min: PlutchikIntensity) -> Vec<PlutchikEmotion>
```

设计要点:
- `set_intensity` 是直接赋值, 不触发 apply() 内部 resonance 升级
- `bump_intensity` 用 `ordered_levels()` 的 position 做索引, i32 delta 防止 underflow,
  clamp 到 [0, len-1] 永远落在 valid level 内
- `history_recent` 倒序 (most recent first), limit=0 返回空 Vec
- `history_min_intensity` 按 entry.intensity() 在 ordinal 数组里的位置比较,
  chronological 顺序 (与 history() 一致)

## Tests (4 new tests pass)
- r248_01: set_intensity 多次设值, current_intensity 跟最新
- r248_02: bump_intensity +1/-1/边界 +10/-10 全部 clamp 到 [Mild, Extreme]
- r248_03: history_recent(10) 全 3 项, history_recent(2) 只后 2 项, history_recent(0) 空
- r248_04: history_min_intensity 在 4 档下全部验证 (含边界 Strong/Extreme 过滤掉所有 Moderate)

注: PlutchikEvent::emotion() 编码 intensity=Moderate (常量), 所以 apply 推入的 history
entry 永远是 Moderate. r248_04 测试反映这个事实, 而不试图伪造不同 intensity 的 history
(那种路径需要新增 PlutchikEmotion 直接入列的 API, 留给后续 R).

## Files
- `crates/apeireth-consciousness/src/plutchik_engine.rs` (+4 methods, +4 tests)

cumulative: ~6345 tests pass.
