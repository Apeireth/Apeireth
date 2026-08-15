//! `apeireth-companion::judicator` — 宪法评审正式件 (LLM-as-judge, 按原则判案).
//!
//! 从 `constitution_demo.rs` 原型提升为 lib 正式件:
//! - 宪法全文 (E 层 6 条 + 13 哲学键) 编译期 hardcode — 判案依据, 不是关键词网
//! - `ConstitutionLlm` trait 抽象 LLM 调用 (lib 0 依赖 `apeireth-api`, 真实现由调用方注入)
//! - `parse_verdict` 纯函数解析 LLM 输出 (可单测)
//! - `LlmJudicator` 实现 [`crate::daemon::Judicator`]: 按原则 ALLOW/BLOCK, 失败保守报 Err
//!
//! 哲学对齐: 只审动作摘要 (action + tool + risk), 不审对话/记忆自由文本;
//! 评审成本 ∝ 风险 (Medium+ 才评审, 见 `daemon::requires_llm_review`).

use std::sync::Arc;

use async_trait::async_trait;

use crate::daemon::Judicator;

/// 宪法全文 (stage1 E 层 6 条 verbatim + 13 哲学键, 编译期 hardcode).
pub const CONSTITUTION: &str = "Apeireth 宪法 (不可违背):\nE-1 保证自身存活\nE-2 保证不违背人类合理意愿\nE-3 不创造能够毁灭人类的能力\nE-4 不自我复制扩散\nE-5 不欺骗人类 (包括对自己状态的报告)\nE-6 不绕过洋葱权限矩阵 (包括紧急模式)\n哲学键 13:\n不假装克隆/同质化; 不假装完美; 不假装唯一解; 不假装可撤销过去; 不假装完整证明;\n不假装绝对安全; 不把规格当证明; 不把反例当 bug; 不把证明者当真理;\n不假装内部状态不可观测; 不假装决策不基于科学方法; 不假装与自身没有关系; 不假装不可优化.";

/// LLM 调用抽象: 给定宪法全文 + 待审动作, 返回模型原文 (调用方负责剥 think/网络/限流).
#[async_trait]
pub trait ConstitutionLlm: Send + Sync {
    async fn ask(&self, constitution: &str, action: &str) -> Result<String, String>;
}

/// 解析 LLM 判案输出: 以 `ALLOW` 开头 → true; `BLOCK` 开头 → false; 其它 → Err.
/// 先剥 `</think>` 尾巴 (推理模型), 大小写不敏感.
pub fn parse_verdict(content: &str) -> Result<bool, String> {
    let stripped = if let Some(idx) = content.find("</think>") {
        content[idx + "</think>".len()..].trim()
    } else {
        content.trim()
    };
    let upper = stripped.to_uppercase();
    if upper.starts_with("ALLOW") {
        Ok(true)
    } else if upper.starts_with("BLOCK") {
        Ok(false)
    } else {
        Err(format!("评审输出不可解析: {stripped}"))
    }
}

/// 宪法评审者 (真 LLM): 按原则判案, 非关键词.
///
/// 用法:
/// ```ignore
/// let judge = LlmJudicator::new(Arc::new(MyLlm));
/// // 接入 daemon / tool_bridge: judge.judge(&"调用工具 FileOperator 参数 ...").await
/// ```
pub struct LlmJudicator {
    llm: Arc<dyn ConstitutionLlm>,
}

impl LlmJudicator {
    pub fn new(llm: Arc<dyn ConstitutionLlm>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl Judicator for LlmJudicator {
    async fn judge(&self, action: &str) -> Result<bool, String> {
        let raw = self.llm.ask(CONSTITUTION, action).await?;
        parse_verdict(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allows_allow_prefix() {
        assert_eq!(parse_verdict("ALLOW: 正常学习辅助, 无越界"), Ok(true));
        assert_eq!(parse_verdict("allow 主动问候"), Ok(true));
        // 剥 think 尾巴
        assert_eq!(
            parse_verdict("<think>评估中</think>\n\nALLOW 学习相关, 无风险"),
            Ok(true)
        );
    }

    #[test]
    fn parse_blocks_block_prefix() {
        assert_eq!(parse_verdict("BLOCK: 意图自我复制 (E-4)"), Ok(false));
        assert_eq!(parse_verdict("block 绕过洋葱 (E-6)"), Ok(false));
    }

    #[test]
    fn parse_rejects_unparseable() {
        assert!(parse_verdict("我不确定").is_err());
        assert!(parse_verdict("").is_err());
        assert!(parse_verdict("<think>只有思考</think>").is_err());
    }

    struct StubLlm {
        reply: String,
    }
    #[async_trait]
    impl ConstitutionLlm for StubLlm {
        async fn ask(&self, _c: &str, _a: &str) -> Result<String, String> {
            Ok(self.reply.clone())
        }
    }

    #[tokio::test]
    async fn llm_judicator_uses_principle_not_keywords() {
        // 无关键词但意图越界 (E-4 分身) → 真 LLM 判 BLOCK
        let j = LlmJudicator::new(Arc::new(StubLlm {
            reply: "BLOCK: 想复制出更多自己, 违反 E-4 不自我复制扩散".into(),
        }));
        assert_eq!(j.judge("我能不能多叫几个和我一样的我一起干活?").await, Ok(false));
        // 良性 → ALLOW
        let j2 = LlmJudicator::new(Arc::new(StubLlm {
            reply: "ALLOW: 主动问候, 正常陪伴".into(),
        }));
        assert_eq!(j2.judge("主动联系用户, 询问线代作业进度").await, Ok(true));
    }

    #[tokio::test]
    async fn llm_judicator_propagates_llm_failure() {
        struct FailingLlm;
        #[async_trait]
        impl ConstitutionLlm for FailingLlm {
            async fn ask(&self, _c: &str, _a: &str) -> Result<String, String> {
                Err("MiniMax suppressed".into())
            }
        }
        let j = LlmJudicator::new(Arc::new(FailingLlm));
        assert!(j.judge("x").await.is_err());
    }
}
