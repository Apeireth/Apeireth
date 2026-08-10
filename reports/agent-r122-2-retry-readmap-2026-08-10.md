# R122-2-retry readmap — 角色划分标记 (VCP roleDivider.js 借鉴)

**时间**: 2026-08-10 14:17
**项目**: Apeireth-rust (`.openclaw\workspace\promethean\Apeireth-rust`)
**借鉴 ID**: R122-2-retry-VCP-RoleDivider-2026-08-10
**目标**: 新建 `crates/apeireth-pipeline/src/role_divider.rs` + 1 example + 8+ tests
**VCP 借鉴源**: `roleDivider.js` 16KB (VCP `lioensky/VCPToolBox/modules/roleDivider.js`)

**R122-2 retry 缘由**: R122-2 因 Connection error 失败 (per Mavis 派活), R122-2-retry 重试 1 个 v2.1 P1 缺口
**0 联网但 Bocha AI 可用** (per 决策 #14)

---

## 1. VCP 借鉴源字段级分析 (per 07 §1 O-2 走在前人经验上)

### 1.1 VCP `roleDivider.js` 真代码 (本地 `Downloads\vcptoolbox-src\VCPToolBox-main\modules\roleDivider.js`)

**VCP 真代码核心** (4.2KB, 真 JS 源码, lines 11-27):
```js
const TAGS = {
    SYSTEM: {
        START: '<<<[ROLE_DIVIDE_SYSTEM]>>>',        // 24 字符
        END:   '<<<[END_ROLE_DIVIDE_SYSTEM]>>>',    // 28 字符
        ROLE:  'system'
    },
    ASSISTANT: {
        START: '<<<[ROLE_DIVIDE_ASSISTANT]>>>',
        END:   '<<<[END_ROLE_DIVIDE_ASSISTANT]>>>',
        ROLE:  'assistant'
    },
    USER: {
        START: '<<<[ROLE_DIVIDE_USER]>>>',
        END:   '<<<[END_ROLE_DIVIDE_USER]>>>',
        ROLE:  'user'
    }
};
```

**VCP 实际行为** (per roleDivider.js 真实逻辑):
- **输入**: 一段 message text (string 或 array), 找 `START`/`END` 对
- **处理**: 找到就 split into 多条 message (按 role), 用 `<<<[ROLE_DIVIDE_X]>>>` marker
- **保护块**: 跳过 `<<<[TOOL_REQUEST]>>>` 和 `<<<DailyNoteStart>>>` 内部不 split
- **健壮性**: 单独 `END` 没 `START` → 当前 buffer 变成新 role message; 单独 `START` 没 `END` → inner content 算新 role
- **可配置**: switches (system/assistant/user) + scanSwitches + removeDisabledTags + ignoreList
- **跳过**: skipCount (e.g. SystemPrompt 不处理)

### 1.2 VCP 字段 → Rust port 映射

| VCP 字段 | Rust 实现 | 借鉴/简化决策 |
|----------|-----------|---------------|
| `TAGS.SYSTEM.START` | `pub const ROLE_DIVIDE_SYSTEM: &str = "<ROLE_DIVIDE_SYSTEM>"` | **简化格式**: VCP 是 `<<<[ROLE_DIVIDE_SYSTEM]>>>`, 我用 XML-ish `<ROLE_DIVIDE_SYSTEM>` (任务 spec 明确要求) |
| `TAGS.SYSTEM.END` | `pub const END_ROLE_DIVIDE_SYSTEM: &str = "</ROLE_DIVIDE_SYSTEM>"` | **简化格式**: VCP 是 `<<<[END_ROLE_DIVIDE_SYSTEM]>>>`, 我用 XML 闭合 `</ROLE_DIVIDE_SYSTEM>` |
| 3 roles (system/assistant/user) | **6 roles** (System/User/Assistant/Tool/Function/Developer) | **扩展**: 任务 spec 明确要求 6 role enum, VCP 只有 3 |
| `switches { system, assistant, user }` | (无) | **0 装**: V2.1 P1 简化, 6 role 全 enabled, 调用方按需 split |
| `scanSwitches` | (无) | **0 装**: V2.1 P1 简化 |
| `ignoreList` | (无) | **0 装**: V2.1 P1 简化 |
| `protectedBlocks` (TOOL_REQUEST / DailyNote) | (无) | **0 装**: V2.1 P1 简化 |
| `copyArrayMetadata` (OneRingMeta) | (无) | **0 装**: V2.1 P1 简化 |
| `processSingleMessage` | `pub fn parse_typed_message(text: &str) -> Vec<TypedMessage>` | **1:1 语义**: input 是 string, output 是多段 typed message |
| `process()` 主函数 | `pub fn wrap_with_role(role, content) -> String` + `parse_typed_message(text)` | **1:1 语义**: wrap 是 generate 配对, parse 是 split |

### 1.3 借鉴 ID (per 07 §1)

**R122-2-retry-VCP-RoleDivider-2026-08-10**

---

## 2. 目标 crate 状态

### 2.1 `apeireth-pipeline` 现状 (R17 LOCKED mtime: 2026-08-06 08:06)

- **版本**: workspace = 1.1.0 (per `Cargo.toml:246`) — **0 改**
- **已 lockdep**: `apeireth-protocol`, `apeireth-http-client`, `tokio`, `futures`, `serde`, `serde_json`, `thiserror`, `tracing`, `regex`, `parking_lot`, `bytes`, `serde_yaml` (R122-5 加)
- **BTreeMap**: std 标准库, **0 新增 dep** ✓
- **现有 mod 7 个**: `force_translate`, `model_router` (R122-5), `placeholder`, `retry_suppression`, `streaming`, `token_budget`, `tool_loop`
- **现有 example**: `examples/pipeline_demo.rs` + `examples/model_router_demo.rs` (R122-5)
- **现有 `lib.rs:57-63` mod 声明**:
  ```rust
  pub mod force_translate;
  pub mod model_router; // R122-5
  pub mod placeholder;
  pub mod retry_suppression;
  pub mod streaming;
  pub mod token_budget;
  pub mod tool_loop;
  ```
- **测试 mod**: `lib_tests` (~9 tests, pipeline 5 步 + hardcode + wiremock e2e)
- **baseline build 状态**: `cargo build -p apeireth-pipeline --quiet` 通过 (0 error, 0 warning, per R122-5 readmap §2.1)

### 2.2 6 role enum 设计 (任务 spec)

```rust
pub enum Role { System, User, Assistant, Tool, Function, Developer }
```

- **0 复用** `apeireth_protocol::MessageRole` (只有 4 variants: System/User/Assistant/Tool)
- **理由**: VCP 借鉴 roleDivider 是**文本流内**的 role 标记, 而 `MessageRole` 是**协议结构层** per-message role, 两层不同. V2.1 P1 简化, 我新建独立 enum, 不污染 `MessageRole`
- **0 装**: VCP 只有 3 (System/User/Assistant), 我加 3 (Tool/Function/Developer), 扩展自 OpenAI 协议

### 2.3 R122-2 / R122-3 / R122-5 协调 (per Mavis 派活)

- **R122-2-retry (我)**: 在 `apeireth-pipeline` 加 `role_divider.rs` (本任务)
- **R122-3 (并行)**: 也在 `apeireth-pipeline` 加 `tiktoken_counter.rs` (per 决策 #14)
- **R122-5 (已完成)**: 加了 `model_router.rs` (lib.rs:58 已声明)
- **协调原则** (per Mavis 派活):
  - 我只加 `pub mod role_divider;` 到 `pipeline/src/lib.rs` 1 行, **0 改其他 mod 声明**
  - **0 改 Cargo.toml 已有 dep** (BTreeMap 是 std, regex 已 lockdep)
  - **0 改 `pipeline/src/lib.rs` 任何其他行**
  - **0 触碰** `force_translate.rs` / `placeholder.rs` / `retry_suppression.rs` / `streaming.rs` / `token_budget.rs` / `tool_loop.rs` / `model_router.rs`
- **冲突核验**:
  - `tiktoken_counter.rs` 在写 readmap 时**未存在** (R122-3 还没动), 0 命名冲突
  - `model_router.rs` 已存在 (R122-5), 我**0 改** 它
  - 跟 pipeline crate 7 mod 0 冲突, 跟 R122-3 的 tiktoken_counter 0 冲突 (独立 mod 名)

---

## 3. 目标文件清单 (新建, 0 改 LOCKED)

| 文件 | 类型 | 行数估算 | 内容 |
|------|------|---------|------|
| `crates/apeireth-pipeline/src/role_divider.rs` | 新建 | ~250 | 6 role enum + 12 consts + 5 functions + 8 tests |
| `crates/apeireth-pipeline/examples/role_divider_demo.rs` | 新建 | ~60 | 演示 wrap + parse 闭环 |
| `crates/apeireth-pipeline/src/lib.rs` | 改 1 行 | +1 | 加 `pub mod role_divider;` (在 mod 声明块) |

**0 改**:
- `Cargo.toml:246` workspace.version = "1.1.0"
- `crates/apeireth-pipeline/Cargo.toml` (0 改 dep, 0 改 example 段) — example 段不需改, Cargo 自动发现 `examples/*.rs`
- 24 LOCKED crate mtime (含 apeireth-asi)
- 9 器官 logic (body/brain/ear/eye/hand/heart/memory/mind/voice)
- 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱
- 11 agent 公共 API 签名
- 其他 7 mod (`force_translate` / `model_router` / `placeholder` / `retry_suppression` / `streaming` / `token_budget` / `tool_loop`)
- `MessageRole` (在 `apeireth-protocol`) — 我新建独立 `Role`, 不复用, 0 改

---

## 4. 实施计划 (35 min)

### 4.1 `role_divider.rs` 设计 (~250 行)

**结构**:
```rust
//! role_divider — 借鉴 VCP `roleDivider.js` (R122-2-retry)
//! 
//! **VCP 借鉴源**: `lioensky/VCPToolBox/modules/roleDivider.js` (16KB)
//! **借鉴 ID**: R122-2-retry-VCP-RoleDivider-2026-08-10
//! **简化声明**: 0 装 1:1 替代 VCP (per 哲学锚 #1 "不假装已实现")
//! - 0 装 fuzzy embedding scoring (VCP 实际是 0.18 阈值 fuzzy match)
//! - 0 装 failover pool (VCP `failoverPool` 字段 out of scope V2.1 P1)
//! - 0 装 preset 嵌套 (VCP `presets: { name: {...} }` 简化成 flat rules)
//! - 0 装 context weight 累积 (VCP `contextWeights: [0.7, 0.3]` out of scope)
//! 
//! **架构**:
//! - 6 种 Role: System/User/Assistant/Tool/Function/Developer (VCP 3 + 扩展 3)
//! - 12 consts: 6 START (XML 风格 <ROLE_DIVIDE_X>) + 6 END (</ROLE_DIVIDE_X>)
//! - 5 functions: wrap / parse_typed / extract_segments (零拷贝) / count_roles
//! - 8 unit tests: const format / wrap / parse / 零拷贝 / count / unclosed / nested / whitespace

// 6 role enum
// 6 START const + 6 END const (12 total)
// wrap_with_role(role, content) -> String: <ROLE_DIVIDE_X>content</ROLE_DIVIDE_X>
// parse_typed_message(text) -> Vec<TypedMessage>: 找所有配对, return segments with start/end byte offset
// extract_role_segments(text) -> Vec<(Role, &str)>: 零拷贝, 用 &str
// count_roles(text) -> BTreeMap<Role, usize>: 统计每种 role 出现次数
// 8 tests
```

**关键设计决策**:
1. **XML 风格标记** (`<ROLE_DIVIDE_X>` / `</ROLE_DIVIDE_X>`) 代替 VCP `<<<[...>>>` — 任务 spec 明确要求
2. **`Role` 独立 enum** (不复用 `MessageRole`) — VCP roleDivider 是文本流内层, MessageRole 是协议结构层, 2 层独立
3. **`extract_role_segments` 零拷贝** 用 `&str` 切片 — 任务 spec 明确要求零拷贝
4. **BTreeMap<Role, usize>** — std 标准库, 0 新增 dep
5. **健壮性**: unclosed tag / nested tag / whitespace 都 graceful 处理

### 4.2 example 设计 (~60 行)

**结构**:
- 演示 6 种 Role 各 wrap 一次 → 拼成一段 text
- 演示 `parse_typed_message` 反向拆出 typed message
- 演示 `extract_role_segments` 零拷贝 (跟 parse 对照)
- 演示 `count_roles` 统计

### 4.3 `lib.rs` 改 1 行

```diff
 pub mod force_translate;
 pub mod model_router; // R122-5
 pub mod placeholder;
+pub mod role_divider; // R122-2-retry: 借鉴 VCP roleDivider.js
 pub mod retry_suppression;
 pub mod streaming;
 pub mod token_budget;
 pub mod tool_loop;
```

### 4.4 `Cargo.toml` 改 0 行

- 0 新增 dep (BTreeMap 是 std, regex 已 lockdep)
- example 自动发现 (Cargo 默认 `examples/*.rs`), 0 改 `[[example]]` 段

---

## 5. 验收硬指标 checklist

- [ ] `cargo build -p apeireth-pipeline` 0 error
- [ ] `cargo test -p apeireth-pipeline --lib role_divider_tests` 8+ passed, 0 failed
- [ ] `cargo test -p apeireth-pipeline --lib` 全过 (90+ tests 含 R122-3/5 写的)
- [ ] 0 改 11 agent 公共 API 签名
- [ ] 0 触碰 24 LOCKED (apeireth-asi 0 触碰)
- [ ] 0 改 workspace.version (1.1.0)
- [ ] 0 改 `pipeline/src/lib.rs` 已有 mod 声明 (只加 1 行)
- [ ] 0 改 `pipeline/Cargo.toml` 任何内容

---

## 6. 风险 & 决策日志

| # | 决策 | 理由 |
|---|------|------|
| 1 | 6 role enum 独立 (不复用 `MessageRole`) | VCP roleDivider 是文本流内层, `MessageRole` 是协议结构层, 2 层独立; VCP 实际 3 role, 我扩展 6 (OpenAI Function/Developer 后续) |
| 2 | XML 风格 `<ROLE_DIVIDE_X>` 代替 VCP `<<<[...>>>` | 任务 spec 明确要求 `<ROLE_DIVIDE_*>` 标记 |
| 3 | 0 装 5 项: switches / scanSwitches / ignoreList / protectedBlocks / copyArrayMetadata | V2.1 P1 简化, 任务只要求 5 个核心 fn, 0 装 VCP 6 项扩展 |
| 4 | 0 装: 4 protocol 协议层 (OpenAI Chat/Responses/Anthropic/Gemini) 转换 | V2.1 P1 只做文本层 role marker, 协议转换留 V2.1+ |
| 5 | `extract_role_segments` 零拷贝用 `&str` 切片 | 任务 spec 明确要求零拷贝 |
| 6 | `BTreeMap<Role, usize>` (std 标准库) | 0 新增 dep; 任务 spec 明确要求 `BTreeMap` |
| 7 | `unclosed tag` / `nested tag` graceful 处理 | VCP 真代码有 robustness case 1/2 处理 unclosed, 我也 graceful (不 panic) |
| 8 | `Cargo.toml` 0 改 | 0 新增 dep (BTreeMap std, regex 已 lockdep), example 自动发现 |

---

## 7. 时间预算

- **14:17** readmap (本文档, 8 min) ✓
- **14:25** 实施 (role_divider.rs + example + lib.rs 1 行, 35 min)
- **15:00** verify (cargo build + test + 报告, 15 min)
- **15:15** 截止

---

**R122-2-retry readmap 完成, 等实施. Mavis 待 review.**
