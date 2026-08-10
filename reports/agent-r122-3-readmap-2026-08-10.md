# Agent R122-3 (retry) — Readmap 报告
**日期**: 2026-08-10
**作者**: Mavis 派 — R122-3-retry (tiktoken 精确计数)
**任务**: VCP `finalContextStore.js` 借鉴 — 引入 `tiktoken-rs` crate 替换启发式 token 计数
**阶段**: 读图 (0-8 min, 截至 14:26)
**借鉴 ID**: `R122-3-VCP-TiktokenCounter-2026-08-10`

---

## 1. VCP 真代码 (借鉴字段级)

### 1.1 来源文件
`research/source/vcptoolbox/modules/finalContextStore.js` (11,559 字节, 8/6 抓取)

### 1.2 字段级真值 (per 07 §1 O-2, 必须引用真代码)
| 字段 | VCP 真代码 | 行号 | 我们要干什么 |
|---|---|---|---|
| `TOKENIZER_NAME` | `'cl100k_base'` | line 11 | 借鉴: 默认模型 = Cl100KBase (gpt-3.5/4 系列) |
| `TOKENIZER_METHOD` | `'@dqbd/tiktoken:cl100k_base'` | line 12 | 借鉴: TokenModel 枚举 + method 标识 |
| `get_encoding(TOKENIZER_NAME)` | JS 加载 | line 16 | 借鉴: Rust `tiktoken_rs::cl100k_base()` |
| `encoding.encode(text).length` | JS 调用 | line 58 | 借鉴: `CoreBPE.encode(text).len()` → count_tokens |
| `countTokensForText` 优雅降级 | try/catch | line 55-70 | 借鉴: tiktoken 失败 → 启发式 fallback (双轨制) |
| `estimateTokensForText` | CJK/word/symbol | line 40-45 | **保留** (token_budget.rs 现有 `token_pieces()` 启发式) |

### 1.3 VCP 业务定位
- **入口**: `chatCompletionHandler` 完成请求体合成后,调 `setLastFinalContext(body, metadata)`
- **用途**: 5 组缓存滑窗,前端管理面板切换查看
- **token 计数的角色**: summary 字段 `totalTextTokenCount` (line 269) 用于诊断和限额

---

## 2. 目标文件 (实施清单)

### 2.1 改 2 个 Cargo.toml
- `Cargo.toml` (workspace) — 加 `tiktoken-rs = "0.7"` 到 `[workspace.dependencies]`
- `crates/apeireth-pipeline/Cargo.toml` — 加 `tiktoken-rs = { workspace = true }` 到 `[dependencies]` + `[dev-dependencies]`

### 2.2 新建 1 个 rs
- `crates/apeireth-pipeline/src/tiktoken_counter.rs` (~200-300 行)
  - `pub struct TiktokenCounter { bpe: Arc<CoreBPE>, model: TokenModel }`
  - `pub enum TokenModel { Cl100KBase, O200KBase, P50KBase, R50KBase, Gpt2 }`
  - `pub fn new(model: TokenModel) -> Result<Self>`
  - `pub fn count_tokens(&self, text: &str) -> usize`
  - `pub fn count_tokens_batch(&self, texts: &[&str]) -> Vec<usize>`
  - `pub fn encode(&self, text: &str) -> Vec<usize>`
  - `pub fn decode(&self, tokens: &[usize]) -> Result<String>`
  - `pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String`
  - `pub fn available_models() -> Vec<TokenModel>`

### 2.3 改 2 个 rs
- `crates/apeireth-pipeline/src/token_budget.rs` — 加 `count_tokens_precise(text, model) -> usize`, 优先 tiktoken, 失败回退启发式
- `crates/apeireth-pipeline/src/lib.rs` — 加 `pub mod tiktoken_counter;` + re-export

### 2.4 加 9 unit test + 1 集成 test
- 8+ 单元 (8 个写在 tiktoken_counter.rs + 1 集成写在 tests/)

---

## 3. 8 墙自检 (实施前)

| 墙 | 状态 | 备注 |
|---|---|---|
| 1. 0 改 workspace.version (1.1.0) | ✅ | 仅加 `[workspace.dependencies]` 1 行 |
| 2. 0 改 R11 baseline 3 值 | ✅ | 不碰 pipeline lib.rs 主 chat 模式 |
| 3. 0 触碰 24 LOCKED crate mtime | ✅ | tiktoken_counter.rs 是新建,不动其他 crate |
| 4. 0 触碰 9 器官 logic | ✅ | 不在器官 crate 加依赖 |
| 5. 0 触碰 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 | ✅ | 全部不碰 |
| 6. 0 改 11 agent 公共 API 签名 | ✅ | 只在 pipeline 加 pub mod, 不改任何 agent 公共接口 |
| 7. 0 主动 commit | ✅ | 改完不 commit, 等主 review |
| 8. 0 装 (O-5) | ✅ | 标 "approximate BPE", 标 model 差异, 不说"100% OpenAI" |

---

## 4. 0 冲突核验 (与 R122-5 / R122-2)

- **R122-5 (语义模型路由)**: 已 succeeded,改 `pipeline/Cargo.toml` 加 `serde_yaml` dep + example。**0 冲突**: 我加 `tiktoken-rs` (不同 dep, 不同 [[example]] name)
- **R122-2 (角色划分) retry**: 会改 `pipeline/src/lib.rs` 加 `pub mod role_divider;`。**0 冲突**: 我加 `pub mod tiktoken_counter;` 不同行, Mavis 兜底 (14:18 后 5min 内 R122-2 还没碰 lib.rs, 我先加)

---

## 5. 风险预判

1. **tiktoken-rs API 形状**: 需确认 `tiktoken-rs 0.7` 的 `cl100k_base()` 返回 `CoreBPE`, 0.6/0.7 可能有 minor 差异
2. **CoreBPE 是否 Send + Sync**: `Arc<CoreBPE>` 字段需要 CoreBPE 是 Send + Sync (标准)
3. **Cargo 注册表可达性**: cargo build 需联网拉 crate, 兜底 cargo vendor 或已有 cache

---

## 6. 时间预算

- 14:18 启动 → 14:26 读图完成 (8 min) ✅
- 14:26-14:56 实施 (30 min) — Cargo.toml 改 + 新建 tiktoken_counter.rs + 改 token_budget.rs + 改 lib.rs
- 14:56-15:15 verify + report (19 min) — cargo build + cargo test + 4 报告
