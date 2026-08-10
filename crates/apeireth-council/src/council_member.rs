//! R33-4: CouncilMember — AutoGen 借鉴
//!
//! **AutoGen 真代码借鉴** (`autogen/agentchat/conversable_agent.py` + `groupchat.py`):
//! - `role` 借鉴 `ConversableAgent.system_message` (角色定位)
//! - `goal` 借鉴 `GroupChatAdmin.description` (目标描述)
//! - `backstory` 借鉴 `human_input_mode` + `llm_config` (行为背景)
//! - `provider` 借鉴 `llm_config.config_list[0].model` (用啥 LLM)
//!
//! **Apeireth 现有 Persona** (per R19 `persona.rs`):
//! - name / character / voice / stance_bias
//!
//! **R33-4 新增 CouncilMember** (跟 Persona 正交, 补充组织 / 目标 / 背景):
//! - role / goal / backstory / provider
//! - 跟 Persona 可组合 (CouncilMember 走"做什么", Persona 走"怎么做")
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 Persona (R19 LOCKED 0 触碰)
//! - 0 改 advisor / deliberation / hold / lifecycle / mock_llm / sovereignty / synthesis (0 业务漂移)
//! - 0 引入 I/O / 网络 (CouncilMember 0 业务状态, 0 真调 LLM)
use serde::{Deserialize, Serialize};

/// CouncilMember — 多 LLM 协商成员 (AutoGen 借鉴)
///
/// **4 字段对应 AutoGen 4 维度**:
/// - `role` 角色 (e.g. "architect" / "security_reviewer" / "product_manager")
/// - `goal` 目标 (e.g. "找到最稳的 Rust 锁版本方案")
/// - `backstory` 背景故事 (e.g. "10 年 Rust 老兵, 主导 3 个 1.0 release")
/// - `provider` LLM provider (e.g. "claude_code" / "codex" / "gemini_cli")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilMember {
    /// 角色定位 (e.g. "architect", "security_reviewer", "product_manager")
    pub role: String,
    /// 目标 (e.g. "找到最稳的 Rust 锁版本方案")
    pub goal: String,
    /// 背景故事 (e.g. "10 年 Rust 老兵, 主导 3 个 1.0 release")
    pub backstory: String,
    /// LLM provider (e.g. "claude_code" / "codex" / "gemini_cli" / "opencode" / "copilot")
    /// (per R35+R36 apeireth-provider 5 provider 真合并, 0 老 crate 引用)
    pub provider: String,
}

impl CouncilMember {
    /// 便利构造
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

    /// 拼成 system prompt (AutoGen-style system_message 借鉴)
    pub fn to_system_prompt(&self) -> String {
        format!(
            "# 角色 (Role)\n{}\n\n# 目标 (Goal)\n{}\n\n# 背景 (Backstory)\n{}\n\n# LLM Provider\n{}",
            self.role, self.goal, self.backstory, self.provider,
        )
    }
}

/// 5 provider 编译期 hardcode (per R35+R36 真合并, 5 provider 走 apeireth-provider 1:1)
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "claude_code",
    "codex",
    "copilot",
    "gemini_cli",
    "opencode",
];

/// 校验 provider 在 5 supported 内
pub fn is_valid_provider(p: &str) -> bool {
    SUPPORTED_PROVIDERS.contains(&p)
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod council_member_tests {
    use super::*;

    #[test]
    fn council_member_new_basic() {
        let m = CouncilMember::new(
            "architect",
            "设计稳的架构",
            "10 年 Rust",
            "claude_code",
        );
        assert_eq!(m.role, "architect");
        assert_eq!(m.goal, "设计稳的架构");
        assert_eq!(m.backstory, "10 年 Rust");
        assert_eq!(m.provider, "claude_code");
    }

    #[test]
    fn to_system_prompt_contains_4_sections() {
        let m = CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全审计", "codex");
        let p = m.to_system_prompt();
        assert!(p.contains("# 角色"));
        assert!(p.contains("security_reviewer"));
        assert!(p.contains("# 目标"));
        assert!(p.contains("找安全漏洞"));
        assert!(p.contains("# 背景"));
        assert!(p.contains("5 年安全审计"));
        assert!(p.contains("# LLM Provider"));
        assert!(p.contains("codex"));
    }

    #[test]
    fn supported_providers_has_5() {
        assert_eq!(SUPPORTED_PROVIDERS.len(), 5);
        assert!(SUPPORTED_PROVIDERS.contains(&"claude_code"));
        assert!(SUPPORTED_PROVIDERS.contains(&"codex"));
        assert!(SUPPORTED_PROVIDERS.contains(&"copilot"));
        assert!(SUPPORTED_PROVIDERS.contains(&"gemini_cli"));
        assert!(SUPPORTED_PROVIDERS.contains(&"opencode"));
    }

    #[test]
    fn is_valid_provider_5_supported() {
        for p in SUPPORTED_PROVIDERS {
            assert!(is_valid_provider(p), "{p} should be valid");
        }
    }

    #[test]
    fn is_valid_provider_rejects_unknown() {
        assert!(!is_valid_provider("unknown"));
        assert!(!is_valid_provider(""));
        assert!(!is_valid_provider("gpt-4"));  // 跟 R35+R36 5 provider 不重叠
    }

    #[test]
    fn council_member_serde_round_trip() {
        let m = CouncilMember::new("pm", "用户价值", "5 年产品", "gemini_cli");
        let json = serde_json::to_string(&m).unwrap();
        let back: CouncilMember = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn council_member_partial_eq_clone() {
        let m1 = CouncilMember::new("a", "g", "b", "codex");
        let m2 = m1.clone();
        assert_eq!(m1, m2);
    }

    /// 5 标准 CouncilMember fixture (借鉴 AutoGen group_chat 5 角色)
    #[test]
    fn standard_council_5_member_fixture() {
        let council = vec![
            CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
            CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
            CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
            CouncilMember::new("qa", "测覆盖", "3 年 QA", "opencode"),
            CouncilMember::new("devops", "稳上线", "5 年 DevOps", "copilot"),
        ];
        assert_eq!(council.len(), 5);
        for m in &council {
            assert!(is_valid_provider(&m.provider));
            assert!(!m.to_system_prompt().is_empty());
        }
    }
}
