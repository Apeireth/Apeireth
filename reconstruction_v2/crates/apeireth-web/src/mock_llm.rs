//! 本地 mock LLM 模块 (替代 apeireth_council, 因为 v2 apeireth-council crate 当前 broken 不能编译).
//!
//! 提供跟 `apeireth_council::mock_llm` 同款公开 API surface:
//! - `MockLlmProvider` trait
//! - `MockLlmResponse` struct
//! - `ScriptedMockLlm` impl (按 prompt 关键词匹配)
//! - `CouncilMember` struct (role / goal / backstory / provider + to_system_prompt)
//!
//! **替代原因**: v2 workspace 里 `apeireth-council` crate 自身有 15+ E0432 错误,
//! 内部 stale 引用 `crate::sovereign` / `crate::lifecycle::AdvisorLifecycle` /
//! `deliberation::Council` / `persona::DebateRound` 等都不存在, 整个 crate 编译不过.
//! 我们的 main.rs (`bin`) 只用 mock_llm 部分 + CouncilMember, 直接 inline 简化版.
//! 真 LLM 接入后续接 `apeireth_asi::llm_judge::LlmProvider` (R33-4+, 后续接入).

use std::sync::Mutex;

/// Mock LLM 响应.
#[derive(Debug, Clone, PartialEq)]
pub struct MockLlmResponse {
    /// 响应文本
    pub text: String,
    /// 是否触发按住 (True = 强反对)
    pub triggers_hold: bool,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
}

impl MockLlmResponse {
    /// 便利构造 — 不触发按住.
    pub fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), triggers_hold: false, confidence: 0.8 }
    }

    /// 便利构造 — 强反对 (按住触发).
    pub fn reject(text: impl Into<String>) -> Self {
        Self { text: text.into(), triggers_hold: true, confidence: 0.95 }
    }

    /// 自定义置信度.
    pub fn with_confidence(mut self, c: f64) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }
}

/// Mock / scripted LLM trait.
pub trait MockLlmProvider: Send + Sync {
    /// 生成响应
    fn generate(&self, prompt: &str, system: &str) -> MockLlmResponse;
}

/// 脚本化 Mock LLM — 按 prompt 关键词匹配响应.
/// 按插入顺序匹配第一个命中的关键词, 无命中返回 default.
pub struct ScriptedMockLlm {
    scripts: Vec<(String, MockLlmResponse)>,
    default: MockLlmResponse,
    call_count: Mutex<u64>,
}

impl ScriptedMockLlm {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            default: MockLlmResponse::ok("默认响应 — 无关键词命中, 默认赞成"),
            call_count: Mutex::new(0),
        }
    }

    pub fn with_script(mut self, keyword: impl Into<String>, response: MockLlmResponse) -> Self {
        self.scripts.push((keyword.into(), response));
        self
    }

    pub fn with_default(mut self, response: MockLlmResponse) -> Self {
        self.default = response;
        self
    }

    pub fn call_count(&self) -> u64 {
        *self.call_count.lock().expect("mock_llm mutex poisoned")
    }
}

impl Default for ScriptedMockLlm {
    fn default() -> Self { Self::new() }
}

impl MockLlmProvider for ScriptedMockLlm {
    fn generate(&self, prompt: &str, _system: &str) -> MockLlmResponse {
        *self.call_count.lock().expect("mock_llm mutex poisoned") += 1;
        for (kw, resp) in &self.scripts {
            if prompt.contains(kw.as_str()) {
                return resp.clone();
            }
        }
        self.default.clone()
    }
}

/// CouncilMember — 跟 apeireth_council 同款 4 字段.
#[derive(Debug, Clone, PartialEq)]
pub struct CouncilMember {
    pub role: String,
    pub goal: String,
    pub backstory: String,
    pub provider: String,
}

impl CouncilMember {
    pub fn new(
        role: impl Into<String>,
        goal: impl Into<String>,
        backstory: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            goal: goal.into(),
            backstory: backstory.into(),
            provider: provider.into(),
        }
    }

    pub fn to_system_prompt(&self) -> String {
        format!(
            "# 角色 (Role)\n{}\n\n# 目标 (Goal)\n{}\n\n# 背景 (Backstory)\n{}\n\n# LLM Provider\n{}",
            self.role, self.goal, self.backstory, self.provider
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_mock_llm_matches_keyword() {
        let llm = ScriptedMockLlm::new()
            .with_script("safety", MockLlmResponse::ok("safe response"))
            .with_script("risk", MockLlmResponse::reject("risky!"));

        let r1 = llm.generate("this is a safety question", "");
        assert_eq!(r1.text, "safe response");

        let r2 = llm.generate("high risk scenario", "");
        assert_eq!(r2.text, "risky!");
        assert!(r2.triggers_hold);

        let r3 = llm.generate("neutral question", "");
        assert!(r3.text.contains("默认"));
    }

    #[test]
    fn scripted_mock_llm_call_count() {
        let llm = ScriptedMockLlm::new();
        assert_eq!(llm.call_count(), 0);
        llm.generate("hello", "");
        llm.generate("world", "");
        assert_eq!(llm.call_count(), 2);
    }

    #[test]
    fn council_member_new_and_prompt() {
        let m = CouncilMember::new("safety_advisor", "find risks", "10 yr", "mock");
        assert_eq!(m.role, "safety_advisor");
        let p = m.to_system_prompt();
        assert!(p.contains("safety_advisor"));
        assert!(p.contains("find risks"));
        assert!(p.contains("10 yr"));
        assert!(p.contains("mock"));
        assert!(p.contains("角色"));
        assert!(p.contains("目标"));
        assert!(p.contains("背景"));
    }

    #[test]
    fn mock_response_with_confidence_clamps() {
        let r = MockLlmResponse::ok("ok").with_confidence(2.0);
        assert_eq!(r.confidence, 1.0);
        let r2 = MockLlmResponse::ok("ok").with_confidence(-1.0);
        assert_eq!(r2.confidence, 0.0);
    }
}
