# R33-4: CouncilMember — AutoGen 借鉴

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★ (AutoGen 4 维借鉴, 多 LLM 协商成员结构化, 0 LLM 真接)

---

## 1. 目标

apeireth-council 已有 R19 Persona (name/character/voice/stance_bias) — 走"怎么做" (拟人化风格).

R33-4 借鉴 **AutoGen** 加 `CouncilMember` (role/goal/backstory/provider) — 走"做什么" (组织 + 目标 + 背景). 两者正交可组合:
- Persona: 拟人化风格 (怎么表达)
- CouncilMember: 组织定位 (做什么 + 为啥)

---

## 2. AutoGen 真代码借鉴

| CouncilMember 字段 | AutoGen 对应 | 真代码路径 |
|--------------------|-------------|-----------|
| `role` | `ConversableAgent.system_message` | `autogen/agentchat/conversable_agent.py:__init__` |
| `goal` | `GroupChatAdmin.description` | `autogen/agentchat/groupchat.py:__init__` |
| `backstory` | `human_input_mode` + `llm_config` | `autogen/agentchat/conversable_agent.py:__init__` |
| `provider` | `llm_config.config_list[0].model` | `autogen/oai/client.py:OpenAIClient` |

借鉴是**字段级抽象映射**, 不是抄字面 (AutoGen 是 Python, 我们 Rust).

---

## 3. 设计

### 3.1 `CouncilMember` struct

```rust
pub struct CouncilMember {
    pub role: String,        // 角色定位
    pub goal: String,        // 目标
    pub backstory: String,   // 背景故事
    pub provider: String,    // LLM provider
}
```

### 3.2 5 provider 编译期 hardcode (R35+R36 真合并)

```rust
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "claude_code", "codex", "copilot", "gemini_cli", "opencode",
];
```

跟 R36 5 老 provider 真删一致, 走 `apeireth-provider::xxx::Provider` 1:1.

### 3.3 `to_system_prompt` (AutoGen system_message 借鉴)

```rust
pub fn to_system_prompt(&self) -> String {
    format!("# 角色 (Role)\n{}\n# 目标 (Goal)\n{}\n# 背景 (Backstory)\n{}\n# LLM Provider\n{}",
            self.role, self.goal, self.backstory, self.provider)
}
```

---

## 4. 改动

### 4.1 新增 `crates/apeireth-council/src/council_member.rs` (197 LOC)

- 公开 API: `CouncilMember` + `to_system_prompt` + `SUPPORTED_PROVIDERS` + `is_valid_provider`
- 8 unit test (council_member_tests mod, 涵盖 5 场景: 基础构造 / 4 段 prompt / 5 provider / invalid reject / serde / 5 角色 fixture)

### 4.2 `crates/apeireth-council/src/lib.rs`

- 加 `pub mod council_member;` (跟 advisor / deliberation / persona 1:1)

---

## 5. 测试

### 5.1 8 个新 unit test 全过 (apeireth-council)

```
test council_member::council_member_tests::council_member_new_basic ... ok
test council_member::council_member_tests::to_system_prompt_contains_4_sections ... ok
test council_member::council_member_tests::supported_providers_has_5 ... ok
test council_member::council_member_tests::is_valid_provider_5_supported ... ok
test council_member::council_member_tests::is_valid_provider_rejects_unknown ... ok
test council_member::council_member_tests::council_member_serde_round_trip ... ok
test council_member::council_member_tests::council_member_partial_eq_clone ... ok
test council_member::council_member_tests::standard_council_5_member_fixture ... ok

test result: ok. 8 passed; 0 failed
```

### 5.2 回归 (全 workspace)

- 全 workspace 4083 lib test pass (R36 4056 + R32-3 6 + R33-3 13 + R33-4 8 = 4083)
- 0 fail, 0 退化

---

## 6. 不漂移 (主哲学锚 #1)

- 0 改 `persona.rs` (R19 LOCKED 0 触碰, 拟人化风格不动)
- 0 改 advisor / deliberation / hold / lifecycle / mock_llm / sovereignty / synthesis (0 业务漂移)
- 0 引入 I/O / 网络 (CouncilMember 0 业务状态, 0 真调 LLM)
- 0 改 R35+R36 5 provider 真合并 (5 provider 名单一致)

---

## 7. 后续路线

- ✅ R33-4 完成
- ⏭ R33-4-1 (1d): 把 CouncilMember 注入 deliberation.rs, 走 multi-provider 协商 (每人 1 个 provider, 跨 provider 取最大共识)
- ⏭ R33-4-2 (1d): 把 CouncilMember + Persona 组合 (`CouncilMember` 走"做什么", `Persona` 走"怎么做")
- ⏭ R33-5 (LangGraph conditional 实战) — 跟 R32-2 后续一起

---

**Total LOC**: 1 new file (197) + 1 modify (lib.rs 加 1 行 mod) + 8 new test.
**build/test**: 全 workspace pass, 0 退化, 0 breaking.
