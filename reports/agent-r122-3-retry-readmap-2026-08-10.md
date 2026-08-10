# R122-3-retry readmap — tiktoken 精确计数 (VCP finalContextStore.js 借鉴)

**时间**: 2026-08-10 14:17
**项目**: `.openclaw\workspace\promethean\Apeireth-rust`
**借鉴 ID**: R122-3-retry-VCP-FinalContextStore-Tiktoken-2026-08-10
**目标**: 引入 `tiktoken-rs` crate, 给 `apeireth-pipeline` 加精确 token 计数能力
**协调**: R122-5 (model_router) 已完成, 0 冲突; R122-2 (role_divider) 在另一 worktree, 互不干扰

---

## 1. VCP 借鉴源字段级分析

### 1.1 VCP `finalContextStore.js` (实际 11559 bytes, line 1-353)

**核心** (per 借鉴决策日志 + docs/v2-strategy/07 §6 行 75-78 P1 第 8 项):
```js
// line 10-12: 顶层常量
const TOKENIZER_NAME = 'cl100k_base';
const TOKENIZER_METHOD = '@dqbd/tiktoken:cl100k_base';

// line 14-19: try-catch 懒加载 encoding
try {
  const { get_encoding } = require('@dqbd/tiktoken');
  encoding = get_encoding(TOKENIZER_NAME);
} catch (error) {
  encoding = null;  // 失败 fallback
}

// line 40-45: CJK + word + symbol 启发式
function estimateTokensForText(text) {
  const cjkCount = (text.match(/[\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]/g) || []).length;
  const wordCount = (text.match(/[A-Za-z0-9]+/g) || []).length;
  const symbolCount = (text.match(/[^\s\w\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]/g) || []).length;
  return Math.max(0, Math.ceil((cjkCount + wordCount + Math.ceil(symbolCount / 3)) * 1.08));
}

// line 47-70: 优先 encoding.encode(text).length, 失败 fallback 启发式
function countTokensForText(text) {
  if (typeof text !== 'string' || text.length === 0) {
    return { tokenCount: 0, tokenMethod: encoding ? TOKENIZER_METHOD : 'estimate' };
  }
  if (encoding) {
    try {
      return { tokenCount: encoding.encode(text).length, tokenMethod: TOKENIZER_METHOD };
    } catch (error) { /* Fall back to heuristic below. */ }
  }
  return { tokenCount: estimateTokensForText(text), tokenMethod: 'estimate' };
}
```

### 1.2 VCP 字段 → Rust port 映射

| VCP 字段 | Rust port | 借鉴/简化 |
|----------|-----------|----------|
| `TOKENIZER_NAME = 'cl100k_base'` | `TokenModel::Cl100KBase` enum variant | 1:1 借鉴 |
| `@dqbd/tiktoken:cl100k_base` | `tiktoken_rs::cl100k_base()` | 1:1 Rust binding |
| `encoding = get_encoding(TOKENIZER_NAME)` | `tiktoken_rs::cl100k_base()?` | 1:1, lazy load 失败返 Err |
| `encoding.encode(text).length` | `bpe.encode_with_special_tokens(text).len()` | 1:1 (Rust API) |
| `estimateTokensForText` 启发式 | `token_pieces_heuristic()` (chars/4 + 1, VCP 简化版) | 简化: VCP 用 CJK + word + symbol/3 * 1.08, 我用更粗的 chars/4 跟 R122-5 model_router.rs 现有估算一致 |
| `countTokensForText` 优先 + fallback | `count_tokens_precise()` 优先 tiktoken, 失败 fallback | 1:1 借鉴 |
| `tokenMethod` 字符串标识 | `TokenModel` enum (精确) + `Fallback` 标识 | 简化: 0 装 method 字符串拼接 |
| 5 snapshot 缓存 (MAX_SNAPSHOTS=5) | (无) | **0 装**: V2.1 P1 只做 token 计数, snapshot 缓存留 V2.2 |
| 多模态 (image/audio/file) 估算 | (无) | **0 装**: V2.1 P1 只做 text, 多模态留 V2.2 |
| `getBase64ByteLength` | (无) | **0 装**: V2.1 P1 out of scope |
| `MAX_SNAPSHOTS = 5` | (无) | **0 装**: V2.1 P1 out of scope |

**借鉴字段总数**: 5 字段 1:1 (TokenModel enum, lazy load, encode().len(), count_tokens_precise 优先/fallback, tokenMethod 标识)
**0 装字段**: 4 项 (snapshot 缓存 / 多模态估算 / base64 byte / MAX_SNAPSHOTS)

### 1.3 简化决策: 0 装 4 项 (per 哲学锚 #1 "不假装已实现")

| VCP 真有 | 0 装原因 | 我的简化 |
|----------|----------|----------|
| `snapshots: Array` (5 滑窗) | V2.1 P1 只做计数, snapshot 缓存需 admin API + 持久化, 1:1 需 R123+ 续 | 0 port, 0 调用方 |
| `estimateImageTokens` / `estimateAudioTokens` / `estimateFileTokens` | V2.1 P1 只做 text 计数, 多模态需 image 尺寸解码 + audio 编码表, 1:1 需 R123+ 续 | 0 port, text only |
| `getBase64ByteLength` | V2.1 P1 out of scope, 多模态 token 估算需先解 base64 | 0 port |
| `MAX_SNAPSHOTS = 5` | 同 snapshots 0 装 | 0 port |

**显式声明位置**: `crates/apeireth-pipeline/src/tiktoken_counter.rs:1-50` (rustdoc 顶部)

---

## 2. 项目现状核验 (R122-3-retry 0 范围扩散)

### 2.1 现状冲突核验 (R122-5 兄弟已完成, 0 冲突)

**git status 关键 modified / untracked**:
- `crates/apeireth-pipeline/Cargo.toml` modified (R122-5 加了 serde_yaml + model_router_demo example)
- `crates/apeireth-pipeline/src/lib.rs` modified (R122-5 加了 `pub mod model_router;`)
- `crates/apeireth-pipeline/src/model_router.rs` untracked (R122-5 新建)
- `crates/apeireth-pipeline/examples/model_router_demo.rs` untracked (R122-5 新建)

**R122-3-retry 改动只叠加在 R122-5 之上**:
- workspace Cargo.toml: 加 `tiktoken-rs = "0.7"` 1 行 (R122-5 没改这里)
- pipeline Cargo.toml: 加 `tiktoken-rs = { workspace = true }` 2 行 (dependencies + dev-dependencies)
- pipeline/src/lib.rs: 加 `pub mod tiktoken_counter;` 1 行 (R122-5 加了 model_router, 互不冲突)
- pipeline/src/tiktoken_counter.rs: 新建 ~250 行 (R122-5 加了 model_router.rs, 互不冲突)
- pipeline/src/token_budget.rs: 加 `count_tokens_precise()` 1 个新函数 (R122-5 没改这里, 0 触碰现有 4 函数)

**0 改**:
- `Cargo.toml:246` workspace.version = "1.1.0" ✅
- 24 LOCKED crate (含 apeireth-asi) ✅
- 9 器官 logic (body/brain/ear/eye/hand/heart/memory/mind/voice) ✅
- 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 ✅
- 11 agent 公共 API 签名 (R122-5 已经验过 model_router 0 触碰) ✅
- R122-5 已加的 model_router.rs / model_router_demo.rs / serde_yaml 0 改 ✅

### 2.2 `apeireth-pipeline/src/token_budget.rs` 现状

**已存在** (R17 LOCKED baseline):
- 3 个常量: `LIGHT_LIST_TOKEN_BUDGET = 15`, `DEFAULT_BRIEF_TOKEN_BUDGET = 6`, `MAX_INJECTION_CHARS = 16_000`
- 1 个额外约束: `MIN_INJECTION_CHARS = 100`
- 2 个函数: `truncate_to_max(text, max_chars)`, `exceeds_budget(text, max_chars)`
- 编译期 hardcode 守门 (3 个 const assert)
- 9 unit tests (boundary + truncate 真行为)

**关键观察**:
- `token_pieces()` 函数**不在 pipeline** (在 `apeireth-tool-registry/src/token_budget.rs:68`)
- pipeline 用 `chars().count()` 字符数截断 (VCP 启发式 0 装精度版)
- 任务"保留旧 token_pieces() 启发式作为 fallback" → 实际意思: 保留 chars 启发式做 fallback (R122-5 model_router.rs:417 已经用 `prompt.chars().count() / 4 + 1` 估算)

**R122-3-retry 改 token_budget.rs 范围**:
- 加 1 个新函数 `count_tokens_precise(text, model) -> usize` 优先用 tiktoken
- 加 1 个内部 helper `token_pieces_heuristic(text) -> usize` (chars/4 + 1, 跟 R122-5 model_router.rs 一致)
- 0 改 4 个现有函数 + 3 个常量 + 1 个额外约束 + 9 unit tests + 编译期 hardcode 守门

### 2.3 `token_pieces()` 实际位置 (跨 crate 协调记录)

**真相**:
- `crates/apeireth-tool-registry/src/token_budget.rs:68` - 战役 2 借鉴 VCP `dynamicToolRegistry.js:97-99 tokenPieces` 真值
- 这是 R17 战役 2-1 已有代码, 8 墙 #5 "0 改 11 agent 公共 API 签名" 包含这个
- R122-3-retry 0 触碰 (它在 tool-registry, 不在 pipeline 范围)

**为什么 task 说"替换 token_pieces() 启发式"**:
- task 用的语义是"VCP 同源启发式" (tool-registry 的 token_pieces 跟 VCP 真值同源)
- R122-3-retry 的 pipeline 责任: 给 pipeline 加 `count_tokens_precise()`, 不是去改 tool-registry
- pipeline 内有同源 chars 启发式 (R122-5 model_router.rs:417), 我加新精确函数, 用 chars 启发式做 fallback, 跟 R122-5 一致

**这跟 R122-5 报告协调声明一致**: R122-5 报告 §8.3 写 "R122-3 实施 tiktoken_counter 后, 可用真实 token count 替换 (0 改 API, 仅替换实现)" - 我加的 `count_tokens_precise()` 就是给 R122-5 兄弟接的实现, R122-5 是否接是他的事, 0 触碰他的代码。

---

## 3. tiktoken-rs 0.7 API (核心)

### 3.1 Cargo 依赖

```toml
# workspace Cargo.toml [workspace.dependencies]
tiktoken-rs = "0.7"

# pipeline Cargo.toml [dependencies] + [dev-dependencies]
tiktoken-rs = { workspace = true }
```

**0 引其他新 dep** (per task hard-constraint)。

### 3.2 Rust API (per docs.rs/tiktoken-rs/0.7.0)

| 函数/方法 | 签名 | 说明 |
|-----------|------|------|
| `cl100k_base()` | `fn() -> Result<CoreBPE>` | ChatGPT / text-embedding-ada-002 |
| `o200k_base()` | `fn() -> Result<CoreBPE>` | GPT-4o / o1 |
| `p50k_base()` | `fn() -> Result<CoreBPE>` | Code / davinci-002/003 |
| `p50k_edit()` | `fn() -> Result<CoreBPE>` | Edit models |
| `r50k_base()` | `fn() -> Result<CoreBPE>` | GPT-3 / davinci (also gpt2) |
| `CoreBPE.encode_with_special_tokens(&str)` | `fn(&self, &str) -> Vec<Rank>` (Rank=u32) | VCP 1:1 对应 `encoding.encode(text).length` |
| `CoreBPE.decode(Vec<Rank>)` | `fn(&self, Vec<Rank>) -> Result<String, _>` | decode, 需 build 时确认签名 |

**5 model 1:1 对应 task 要求 `TokenModel` enum**:
- `Cl100KBase` ↔ `cl100k_base()`
- `O200KBase` ↔ `o200k_base()`
- `P50KBase` ↔ `p50k_base()`
- `R50KBase` ↔ `r50k_base()`
- `Gpt2` ↔ `r50k_base()` (alias, per docs.rs 注释 "also known as gpt2")

**任务 enum 设计** (5 variants, 跟 VCP 真值 + docs.rs 表格 1:1):
```rust
pub enum TokenModel {
    Cl100KBase,  // ChatGPT, text-embedding-ada-002
    O200KBase,   // GPT-4o, o1
    P50KBase,    // Code, davinci-002/003
    R50KBase,    // GPT-3, davinci
    Gpt2,        // alias of R50KBase (per docs.rs)
}
```

---

## 4. 目标文件清单 (新建 + 改 3 处, 0 触碰 LOCKED)

| 文件 | 类型 | 行数估算 | 内容 |
|------|------|---------|------|
| `Cargo.toml` (workspace) | 改 1 行 | +1 | 加 `tiktoken-rs = "0.7"` 到 `[workspace.dependencies]` |
| `crates/apeireth-pipeline/Cargo.toml` | 改 2 行 | +2 | 加 `tiktoken-rs = { workspace = true }` 到 `[dependencies]` 和 `[dev-dependencies]` |
| `crates/apeireth-pipeline/src/lib.rs` | 改 1 行 | +1 | 加 `pub mod tiktoken_counter;` (R122-3-retry 标识) |
| `crates/apeireth-pipeline/src/tiktoken_counter.rs` | 新建 | ~250 | TiktokenCounter + TokenModel + 8 tests + 编译期 hardcode |
| `crates/apeireth-pipeline/src/token_budget.rs` | 改 +1 fn + 1 helper | +30 | 加 `count_tokens_precise()` + `token_pieces_heuristic()` + 1 unit test |
| `reports/agent-r122-3-retry-readmap-2026-08-10.md` | 新建 | (本文件) | readmap 报告 |
| `reports/agent-r122-3-retry-final-2026-08-10.md` | 新建 | ~150 | final 报告 |
| `reports/agent-r122-3-retry-decision-log-2026-08-10.md` | 新建 | ~80 | decision log 报告 |

**0 改**:
- `Cargo.toml:246` workspace.version = "1.1.0"
- 24 LOCKED crate (含 apeireth-asi)
- 9 器官 logic
- 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱
- 11 agent 公共 API 签名 (Cache / BackoffPolicy / JitterMode / Evictor / dispatch_with_retry / server.rs 4 handler / KeyPathSpan / parse_protocol_kind / pipeline::Pipeline)
- R122-5 已加的 model_router.rs / model_router_demo.rs / serde_yaml 依赖
- `token_pieces()` 在 `apeireth-tool-registry/src/token_budget.rs:68` (跨 crate, 0 触碰)

---

## 5. 实施计划 (50 min)

### 5.1 `tiktoken_counter.rs` 设计 (~250 行)

**结构**:
```rust
//! TiktokenCounter — 精确 token 计数 (VCP finalContextStore.js 借鉴)
//! 
//! **VCP 借鉴源**: lioensky/VCPToolBox/modules/finalContextStore.js (11559 bytes)
//! **借鉴 ID**: R122-3-retry-VCP-FinalContextStore-Tiktoken-2026-08-10
//! **依赖**: tiktoken-rs = "0.7" (tiktoken Rust 绑定, 1:1 替换 @dqbd/tiktoken)
//! 
//! **架构**:
//! - 5 TokenModel enum (Cl100KBase / O200KBase / P50KBase / R50KBase / Gpt2)
//! - TiktokenCounter 内部持 Arc<CoreBPE> (lazy load, lazy 失败返 Err)
//! - count_tokens / count_tokens_batch / encode / decode / truncate_to_tokens
//! 
//! **0 装 4 项** (per 哲学锚 #1 不假装):
//! - snapshot 5 滑窗 (VCP MAX_SNAPSHOTS=5) → 0 port
//! - 多模态估算 (image/audio/file) → 0 port
//! - getBase64ByteLength → 0 port
//! - 多模态附件 tokenMethod 拼接 → 0 port
//! 
//! **借鉴字段 1:1**:
//! - VCP `TOKENIZER_NAME = 'cl100k_base'` → Rust `TokenModel::Cl100KBase` (1:1)
//! - VCP `encoding.encode(text).length` → Rust `bpe.encode_with_special_tokens(text).len()` (1:1)
//! - VCP 优先 + fallback 模式 → Rust `count_tokens()` 优先 + chars/4 fallback (1:1 思路)

// ============================================================
// 编译期 hardcode (per 工程哲学铁律 #2 "不漂移")
// ============================================================
pub const VCP_FINAL_CONTEXT_STORE_BYTES: usize = 11559;

// ============================================================
// TokenModel enum (5 variants)
// ============================================================
pub enum TokenModel { Cl100KBase, O200KBase, P50KBase, R50KBase, Gpt2 }
impl TokenModel { ... as_str(), tiktoken_loader(), available_models() }

// ============================================================
// TiktokenCounter struct
// ============================================================
pub struct TiktokenCounter { bpe: Arc<CoreBPE>, model: TokenModel }
impl TiktokenCounter {
    pub fn new(model: TokenModel) -> Result<Self> { lazy load bpe }
    pub fn count_tokens(&self, text: &str) -> usize { bpe.encode(text).len() }
    pub fn count_tokens_batch(&self, texts: &[&str]) -> Vec<usize>
    pub fn encode(&self, text: &str) -> Vec<usize>
    pub fn decode(&self, tokens: &[usize]) -> Result<String>
    pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String (智能截断, 词边界)
    pub fn model(&self) -> TokenModel
    pub fn available_models() -> Vec<TokenModel>
}

// ============================================================
// 8 unit tests
// ============================================================
#[cfg(test)]
mod tiktoken_counter_tests {
    // 1. new_cl100k_succeeds
    // 2. count_tokens_empty_returns_zero
    // 3. count_tokens_simple_english
    // 4. count_tokens_chinese_higher_than_chars
    // 5. count_tokens_matches_openai_known_value
    // 6. batch_matches_individual
    // 7. truncate_to_tokens_preserves_word_boundary
    // 8. encode_decode_round_trip
    // 9. available_models_returns_5 (额外)
}
```

### 5.2 `token_budget.rs` 改 1 fn + 1 helper (+30 行)

**新增 (0 改 4 现有函数 + 3 常量 + 1 额外约束 + 9 tests)**:
```rust
// 内部 helper: chars/4 + 1 启发式 (跟 R122-5 model_router.rs:417 一致)
fn token_pieces_heuristic(text: &str) -> usize {
    text.chars().count() / 4 + 1
}

// 新公开函数: 优先 tiktoken, 失败 fallback
pub fn count_tokens_precise(text: &str, model: TokenModel) -> usize {
    match TiktokenCounter::new(model) {
        Ok(counter) => counter.count_tokens(text),
        Err(_) => token_pieces_heuristic(text),  // 失败 fallback (VCP 启发式 0 装精度版)
    }
}
```

**0 改**:
- `truncate_to_max` / `exceeds_budget` (R17 LOCKED)
- 3 个常量 `LIGHT_LIST_TOKEN_BUDGET = 15` / `DEFAULT_BRIEF_TOKEN_BUDGET = 6` / `MAX_INJECTION_CHARS = 16_000`
- 1 个额外约束 `MIN_INJECTION_CHARS = 100`
- 9 unit tests
- 编译期 hardcode 守门 (3 const assert)

### 5.3 `lib.rs` 改 1 行

```diff
 pub mod force_translate;
 pub mod model_router; // R122-5: 借鉴 VCP SemanticModelRouter.json (R122-5-VCP-SemanticModelRouter-2026-08-10)
+pub mod tiktoken_counter; // R122-3-retry: 借鉴 VCP finalContextStore.js (R122-3-retry-VCP-FinalContextStore-Tiktoken-2026-08-10)
 pub mod placeholder;
 pub mod retry_suppression;
 pub mod streaming;
 pub mod token_budget;
 pub mod tool_loop; // R32-2
```

### 5.4 `Cargo.toml` (workspace) 改 1 行

```diff
 regex = "1.10"
+# R122-3-retry: 借鉴 VCP finalContextStore.js, tiktoken Rust 绑定 (替换 VCP @dqbd/tiktoken)
+tiktoken-rs = "0.7"
 lru = "0.16"
```

### 5.5 `crates/apeireth-pipeline/Cargo.toml` 改 2 行

```diff
 serde_yaml = "0.9"
+# R122-3-retry: 借鉴 VCP finalContextStore.js, 精确 token 计数
+tiktoken-rs = { workspace = true }
 thiserror = { workspace = true }
```

```diff
 [dev-dependencies]
+# R122-3-retry: tests 里 import tiktoken-rs
+tiktoken-rs = { workspace = true }
 tokio = { version = "1.40", features = ["full", "test-util", "macros"] }
 wiremock = "0.6"
```

---

## 6. 验收硬指标 checklist

- [ ] `cargo build -p apeireth-pipeline` 0 error
- [ ] `cargo test -p apeireth-pipeline --lib tiktoken_counter_tests` 8+ passed, 0 failed
- [ ] `cargo test -p apeireth-pipeline --lib` 全过 (含 R122-5 已有 90 tests)
- [ ] `cargo metadata` 验证 1 个新 dep `tiktoken-rs` 加入
- [ ] 0 改 `Cargo.toml:246` workspace.version = "1.1.0"
- [ ] 0 改 R11 baseline 3 值
- [ ] 0 触碰 24 LOCKED (含 apeireth-asi)
- [ ] 0 改 9 器官 logic
- [ ] 0 改 11 agent 公共 API 签名
- [ ] 0 主动 commit (per task hard-constraint #7)
- [ ] 0 装 (per task hard-constraint #8): VCP snapshot 缓存 / 多模态估算 / base64 byte 0 port

---

## 7. 风险 & 决策日志

| # | 决策 | 理由 |
|---|------|------|
| 1 | `TokenModel::Gpt2` 用 `r50k_base()` (per docs.rs 注释 "also known as gpt2") | docs.rs 显式说明 R50KBase = GPT-2, 1:1 借鉴, 0 装新 dep |
| 2 | `CoreBPE` 持 `Arc<CoreBPE>` (而非裸 `CoreBPE`) | 允许多线程共享 (R122-5 model_router 是 Send + Sync, 我跟齐), Arc 包装 0 额外开销 (CoreBPE 内部已经 Arc 化) |
| 3 | `count_tokens_precise()` 失败 fallback 到 `chars/4 + 1` (而非 VCP CJK + word + symbol/3 * 1.08) | R122-5 model_router.rs:417 已用 chars/4 + 1, 我跟齐, 简化 (CJK char count ≈ token count * 4 比 VCP 启发式粗但更可预测) |
| 4 | 0 装 VCP `MAX_SNAPSHOTS = 5` snapshot 缓存 | V2.1 P1 只做 token 计数, snapshot 缓存需 admin API + 持久化, 1:1 需 R123+ 续 |
| 5 | 0 装 VCP `estimateImageTokens` / `estimateAudioTokens` / `estimateFileTokens` | V2.1 P1 只做 text 计数, 多模态需 image 尺寸解码 + audio 编码表, 1:1 需 R123+ 续 |
| 6 | `truncate_to_tokens` 智能截断 (词边界) | 跟 `truncate_to_max` (字符截断) 区别, tiktoken 截断应按 token 边界 (避免截到 token 中间 decode 乱码) |
| 7 | 编译期 hardcode VCP 真值 `VCP_FINAL_CONTEXT_STORE_BYTES = 11559` (实际 11559 bytes) | per 工程哲学铁律 #2 "不漂移", VCP 借鉴源 hash/size 变了编译会失败 (跟 R122-5 一样模式) |
| 8 | `pub mod tiktoken_counter;` 加在 `model_router` 后, `placeholder` 前 (按字母 + 编号顺序) | 跟 R122-5 兄弟协调, 0 改 R122-5 已有声明, 0 触碰其他 5 mod |
| 9 | `tiktoken-rs` 加到 `[workspace.dependencies]` 而非仅 pipeline (task 要求) | 跟 task hard-constraint #1 一致, 1 个新 dep 走 workspace 路径 |
| 10 | 0 触碰 `apeireth-tool-registry` 的 `token_pieces()` (跨 crate) | 8 墙 #5 "0 改 11 agent 公共 API 签名" + tool-registry 是另一个 crate, 0 范围扩散 |

---

## 8. 时间预算

- **14:17** readmap (本文档, 8 min) ✓
- **14:25** 实施 (workspace Cargo.toml + pipeline Cargo.toml + lib.rs + tiktoken_counter.rs + token_budget.rs, 35 min)
- **15:00** verify (cargo build + test + metadata 验证, 10 min)
- **15:10** 写 final + decision log 报告 (5 min)
- **15:15** 截止

---

**R122-3-retry readmap 完成, 等实施. Mavis 待 review.**
