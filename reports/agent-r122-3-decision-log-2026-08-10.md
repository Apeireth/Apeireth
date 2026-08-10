# Agent R122-3 (retry) — 决策日志
**日期**: 2026-08-10
**作者**: Mavis 派 — R122-3-retry
**任务**: tiktoken 精确计数集成 (VCP finalContextStore.js 借鉴)

---

## D1. 默认 model 选 Cl100KBase
**时间**: 14:43
**决策**: `TokenModel::Cl100KBase` 作为 `Default` + 集成测试默认
**理由**: VCP `finalContextStore.js:11` 真值 `TOKENIZER_NAME = 'cl100k_base'`, 1:1 镜像
**风险**: GPT-4o 实际是 o200k_base, 但 VCP 用的就是 cl100k, 标"标 model 差异"即可
**应用**: `TokenModel::default() = Cl100KBase` 编译期保证

## D2. 启发式 fallback 保留
**时间**: 14:43
**决策**: `token_pieces()` (VCP `estimateTokensForText` 思路) 完整保留, `count_tokens_precise` 失败时回退
**理由**: VCP `finalContextStore.js:62-63` 明确 try/catch fallback to heuristic, 1:1 镜像
**风险**: 0 — 启发式本就存在, 改成"精确优先 + 启发式兜底"双轨
**应用**: `pub fn count_tokens_precise` 内部, `TiktokenCounter::new` 失败 → 公共 `count_tokens_fallback` API 兜底

## D3. tiktoken-rs 锁 0.7
**时间**: 14:43
**决策**: workspace dep 写 `tiktoken-rs = "0.7"`
**理由**: 用户明示, 0.7 是最新 stable (兼容 cl100k/o200k/p50k/r50k/gpt2 全 5 model)
**风险**: minor 漂移风险, 写 `"0.7"` 走 caret 等价 (≥0.7.0, <0.8.0), 0 引其他新 dep
**应用**: Cargo.toml 改 1 行

## D4. 0 引其他新 dep
**时间**: 14:43
**决策**: tiktoken-rs 的 transitive deps 让 cargo 自动解, 我们不写死
**理由**: 严守 8 墙 #1 0 改其他 dep, tiktoken-rs 的 transitive (rand + regex + ...) 都已大概率在 workspace 存在
**风险**: cargo deny bans 可能 warn (多版本), 不在本次任务范围
**应用**: 仅加 `tiktoken-rs = { workspace = true }` 1 行

## D5. lib.rs 加 mod 顺序
**时间**: 14:43
**决策**: `pub mod tiktoken_counter;` 加在 `pub mod token_budget;` 之后, 保持字母序
**理由**: 现有 mod 顺序: force_translate / placeholder / retry_suppression / streaming / **token_budget** / tool_loop (部分按字母序, 部分按依赖序)
**应用**: 在 token_budget 之后加 tiktoken_counter, 跟 token_budget 关联 (精确计数扩展 token 预算)

## D6. 不在 token_budget.rs 删旧 API
**时间**: 14:43
**决策**: `token_pieces()` 公共签名 0 改, 仅加新函数 `count_tokens_precise`
**理由**: 8 墙 #6 0 改 11 agent 公共 API 签名 + 8 墙 #1 0 改其他 dep 推论: 0 改公共 API
**应用**: token_budget.rs 仅在末尾加新 fn, 旧 const + 旧 fn 全保留

## D7. 集成测试在 tests/ 目录
**时间**: 14:43
**决策**: 集成 test 写 `crates/apeireth-pipeline/tests/tiktoken_integration.rs`, 0 写在 lib.rs
**理由**: 现有 `lib.rs` 集成测试都在 `mod lib_tests`, 但 1 个集成 vs 9 unit 拆开更清晰; tests/ 目录在 pipeline crate 已存在
**风险**: 0 — tests/ 子目录本来就在
**应用**: 新建 1 个 .rs 文件

## D8. truncate_to_tokens 边界
**时间**: 14:43
**决策**: `truncate_to_tokens(text, max_tokens)` 末尾加 `…(truncated, original>={max_tokens} tokens)`, 跟 `truncate_to_max` 字符级对称
**理由**: VCP finalContextStore.js 自身不截断, 但 token_budget.rs `truncate_to_max` 已有 `…(truncated, original>={max_chars} chars)`, 1:1 镜像字符 vs token
**风险**: 0 — 已有 `truncate_to_max` 的成功模式
**应用**: `TiktokenCounter::truncate_to_tokens`

---

## 总结
8 个决策点全部为"严守 8 墙"+"1:1 VCP 借鉴"+"减少风险"服务。0 装、0 范围扩散、0 触碰 LOCKED。
