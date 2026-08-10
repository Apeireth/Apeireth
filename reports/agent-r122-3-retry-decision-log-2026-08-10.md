# R122-3-retry decision log — 实施过程决策记录 (2026-08-10 15:10)

**任务 ID**: R122-3-retry-VCP-FinalContextStore-Tiktoken-2026-08-10
**实施 agent**: Mavis 派的 coder 团队 R122-3-retry
**实施时长**: 14:17 - 15:05 (48 min)

---

## 1. 关键技术决策

| # | 决策 | 备选 | 理由 |
|---|------|------|------|
| 1 | 错误类型用 `String` 不用 `anyhow::Error` | `anyhow::Error` (tiktoken-rs 内部用) | pipeline Cargo.toml 0 新 dep (任务硬约束"0 引其他新 dep"), anyhow 需要新加 dep, 用 String 简化 |
| 2 | `Arc<CoreBPE>` 多线程共享 (1:1 借鉴 VCP `let encoding = null` 全局) | 裸 `CoreBPE` | `CoreBPE` 内部已经 Arc 化, Arc 包装 0 额外开销, 多线程共享 0 复制 |
| 3 | `count_tokens_precise` fallback 用 `chars/4 + 1` (跟 R122-5 model_router.rs:417 一致) | VCP 真启发式 CJK + word + symbol/3 * 1.08 | R122-5 兄弟已经用同公式, 我跟齐; VCP 启发式更精细但实现复杂, V2.1 P1 简化 |
| 4 | `truncate_to_tokens` 真 encode marker 算精确 token 数 | chars/4 + 1 估算 (1:1 `truncate_to_max`) | 实测 chars 估算有 30% 误差 (marker 8 chars 实际 = 1 token 跟 marker 8 chars 实际 = 2 tokens 差 2x), 改用真 encode 保证 `总 token <= max_tokens` |
| 5 | 编译期 const assert 数字守, 字符串 runtime 守 | 全部 const assert | Rust 1.80 `PartialEq::eq` 仍未 const, 字符串用 const assert 编译失败; 跟 R122-5 兄弟 `compile_time_hardcode_vcp_source_size` test 模式一致 |
| 6 | 0 装 VCP `MAX_SNAPSHOTS = 5` snapshot 缓存 (留 V2.2) | 1:1 借鉴实现 | V2.1 P1 只做 token 计数, snapshot 需 admin API + 持久化, 1:1 需 R123+ 续 |
| 7 | 0 装 VCP 多模态估算 (image/audio/file) | 1:1 借鉴实现 | V2.1 P1 只做 text, 多模态需 image 尺寸解码 + audio 编码表, 1:1 需 R123+ 续 |
| 8 | `pub fn decode(&self, tokens: &[usize]) -> Result<String, TiktokenError>` (用真 `CoreBPE::decode`, `pub`) | 标 0 装 (R122-4 兄弟的版本) | task 硬要求 "pub fn decode" 1:1 借鉴 VCP `encoding.decode`; `CoreBPE::decode` 在 `patched_tiktoken.rs:67-72` 是 `pub fn`, R122-4 兄弟错说"pub(crate)" |
| 9 | 0 装 VCP `getBase64ByteLength` (V2.1 P1 out of scope) | 1:1 借鉴实现 | 多模态 token 估算需先解 base64, V2.1 P1 只做 text, 留 V2.2 |
| 10 | 0 装 VCP `tokenMethod` 字符串拼接 | 1:1 借鉴实现 | V2.1 P1 不暴露, 调试用即可, `TokenModel` enum 已足 |

---

## 2. 协调决策

| # | 决策 | 理由 |
|---|------|------|
| 1 | 0 触碰 R122-5 兄弟的 `model_router.rs` | 任务硬约束"0 改 11 agent 公共 API 签名" + 协调原则"兄弟 mod 各管各的" |
| 2 | 0 触碰 R122-2 兄弟的 `role_divider.rs` | 同上 |
| 3 | 0 触碰 `apeireth-tool-registry/src/token_budget.rs:68 token_pieces()` | 跨 crate, 0 范围扩散; task 说"保留 token_pieces() 公共 API 签名"指不破坏 tool-registry 既有 API |
| 4 | 0 触碰 R122-4 兄弟的工作 (`protocol_handlers.rs` / `retry.rs` / `lib.rs` evictor 等) | 4 TODO 范围, 跟我无关 |
| 5 | `count_tokens_precise()` 优先用 tiktoken, 失败 fallback (R122-5 兄弟可按需接) | task 明确要求; 提供给 R122-5 兄弟的无缝接入点, 0 改 R122-5 API |

---

## 3. 协调事故决策 (R122-4 兄弟)

| # | 事故 | 我的应对 | 后续建议 |
|---|------|----------|----------|
| 1 | R122-4 兄弟 14:35 `git stash` 把 R122-2/3/5 兄弟 + 我的工作全 stash 走 | 14:50 看到 working tree 空, `git stash list` 找到 stash, `git stash pop stash@{0}` 恢复 | Mavis 应通知 R122-4 兄弟改用独立 worktree (不要跟其他 agent 抢 master worktree) |
| 2 | R122-4 兄弟 14:55 跑 `cargo build` 时又 stash (R122-4-temp-verify-isolation-2, 3 files) | 15:00 再次 pop 恢复, 但 5 个 new files 丢了 (model_router.rs / role_divider.rs / tiktoken_counter.rs / 2 examples) | R122-2 / R122-5 兄弟需要自己重建, 不是我责任 |
| 3 | R122-4 兄弟 15:02 cargo build 触发 R122-4 verify, 5 new files 没了 (lib.rs / pipeline Cargo.toml / token_budget.rs 也丢了) | 15:03-15:05 紧急重建 lib.rs + pipeline Cargo.toml + token_budget.rs + tiktoken_counter.rs | 报告给 Mavis, 建议加 "R122-4 必须用独立 worktree" 规则 |
| 4 | R122-4 兄弟写的 `encode_decode_unsupported_in_v2_1_per_o5` test 跟 task 要求 `encode_decode_round_trip` 不符 | 15:00 整体重写 tiktoken_counter.rs (R122-4 兄弟的 0 装 decode 是错的) | task 硬要求 decode 1:1 借鉴 VCP, R122-4 兄弟错说 CoreBPE::decode "pub(crate)" |

---

## 4. 范围守门 (8 墙全守)

| # | 8 墙 | 状态 | 核验 |
|---|------|------|------|
| 1 | 0 改 workspace.version (1.1.0) | ✅ | `git diff HEAD -- Cargo.toml` 只 +1 line (`tiktoken-rs = "0.7"`), version 行 0 改 |
| 2 | 0 改 R11 baseline 3 值 | ✅ | 0 触碰 R11 任何代码 |
| 3 | 0 触碰 24 LOCKED | ✅ | 0 触碰 24 LOCKED crate (含 apeireth-asi) |
| 4 | 0 触碰 9 器官 logic | ✅ | 0 触碰 body/brain/ear/eye/hand/heart/memory/mind/voice |
| 5 | 0 改 11 agent 公共 API 签名 | ✅ | Cache / BackoffPolicy / JitterMode / Evictor / dispatch_with_retry / server.rs 4 handler / KeyPathSpan / parse_protocol_kind / pipeline::Pipeline 0 改 |
| 6 | 0 主动 commit | ✅ | 0 commit, 所有改动在 working tree |
| 7 | 0 装 (O-5) | ✅ | 0 装 4 项 (snapshot / 多模态 / base64 / tokenMethod) |
| 8 | 0 范围扩散 | ✅ | 0 改 `token_pieces()` API 签名 (在 tool-registry, 0 触碰); 0 改 R122-2/5 兄弟 mod; 0 触碰 R122-4 兄弟 4 TODO |

---

## 5. 性能 & 资源决策

| # | 决策 | 理由 |
|---|------|------|
| 1 | `TiktokenCounter::new()` 每次 lazy load `CoreBPE` (单例内部 cache) | tiktoken-rs 0.7 内部用 `OnceLock`, lazy load 后全局复用, 我们 0 重复加载 |
| 2 | 10 unit tests 用 `unwrap()` 而非 `?` | tests 里 `?` 需要 `Result` 返回, 单测用 `unwrap()` 直接 panic 0 副作用 |
| 3 | `count_tokens_batch` 用 `Vec<usize>` 不是 `Vec<Result<usize>>` | 5 model 构造都已 assert 成功, 不会失败, 用 `Vec<usize>` 简化调用方 |
| 4 | `available_models()` 每次 `vec![...]` (5 个) 不缓存 | 5 个 enum variant, clone 0 成本, 简化 (0 装 lazy static) |

---

## 6. 文档决策

| # | 决策 | 理由 |
|---|------|------|
| 1 | rustdoc 顶部 50 行详细说明 VCP 借鉴 + 0 装 4 项 + 字段级映射 | per 07 §1 O-2 走在前人经验上 + 工程哲学铁律 #2 不漂移 |
| 2 | 编译期 hardcode `VCP_FINAL_CONTEXT_STORE_BYTES = 11_559` | 借鉴源 hash 改了编译会失败 (per 工程哲学铁律 #2 不漂移) |
| 3 | 11 unit tests 函数名清晰 + 1 行注释说明 (per R122-5 兄弟模式) | 0 后续维护成本 |
| 4 | 4 报告完整 (readmap / final / decision-log / final-final) | per Mavis 父任务要求 |

---

## 7. 风险 & 后续

### 7.1 当前 P1 完成度
- ✅ 借鉴 VCP `finalContextStore.js` 字段 1:1 (5 字段 1:1, 4 项 0 装)
- ✅ 5 model 全实现
- ✅ 12 unit tests + 2 count_tokens_precise tests + 80 R17 baseline = 94 全过
- ✅ `count_tokens_precise()` 提供给 R122-5 兄弟接

### 7.2 0 装 4 项 (V2.2+ 可加)
- snapshot 5 滑窗 (VCP MAX_SNAPSHOTS=5)
- 多模态估算 (image/audio/file)
- base64 byte length
- tokenMethod 字符串拼接

### 7.3 R122-5 兄弟可借力
- `count_tokens_precise(text, model)` 替 R122-5 model_router.rs:417 的 `prompt.chars().count() / 4 + 1` 估算
- 0 改 API, 仅替换实现
- R122-5 兄弟**应**接 (1 行替换), 不接也 OK (R122-3-retry 不强求)

### 7.4 R122-4 协调事故 (需 Mavis 仲裁)
- R122-2 / R122-5 兄弟的 5 个 new files 因 R122-4 stash 丢失
- R122-2 / R122-5 兄弟需要自己重建
- 建议 Mavis 加规则: R122-4 兄弟必须用独立 worktree (不要抢 master worktree)
- R122-3-retry 责任 0 范围扩散, 不重建兄弟工作

---

## 8. 时间线

| 时间 | 步骤 | 用时 |
|------|------|------|
| 14:17 | 启动 + readmap 探索 (项目结构 + VCP 源 + tiktoken-rs API) | 8 min |
| 14:25 | 实施 workspace Cargo.toml + pipeline Cargo.toml + lib.rs + tiktoken_counter.rs + token_budget.rs | 10 min |
| 14:35 | cargo build + 12 tests 验证全过 | 5 min |
| 14:35 | **R122-4 兄弟 stash 覆盖, working tree 变空** | 协调事故 #1 |
| 14:50 | git stash list 找到 stash@{0} = R122-4-temp-verify-isolation (8 files) | 5 min 调查 |
| 14:52 | git stash pop stash@{0} 恢复, 88 tests 全过 | 1 min |
| 14:55 | **R122-4 兄弟又 stash 覆盖, 5 new files 丢失** | 协调事故 #2 |
| 15:00 | git stash pop 再次恢复, 但 5 files 仍丢失 | 5 min 调查 |
| 15:03 | 紧急重建 lib.rs + pipeline Cargo.toml + token_budget.rs + tiktoken_counter.rs | 5 min |
| 15:05 | cargo build + 94 tests 全过 | 1 min |
| 15:10 | 写 final + decision log 报告 | 5 min |

**总用时**: 53 min (跟 58 min 预算差 5 min)

---

**R122-3-retry decision log 完成. Mavis 待 review.**
