//! `apeireth-companion::critic` — CRITIC 反思带工具调用 (审计 P2#10, 2026-08-16).
//!
//! CRITIC (Wang et al.) 精神: LLM 自我反思不可靠 → 反思中的**可验证声明**
//! 交由外部工具核对, 结果回注。本模块是纯机制:
//!   1. `extract_claims` — 从反思文本提取可验证声明 (启发式: 含推断词/统计性陈述的句子)
//!   2. `ClaimVerifier` trait — 验证口 (宿主注入; ToolBridge 只读工具实现或轻量规则实现)
//!   3. `ReflectionCritic` — 组合器: 提取 → 逐条验证 → `CritiqueReport`
//!
//! 0 假装 (诚实): 声明提取是行级启发式 (非 LLM 语义判定); 无法验证的声明
//! 如实标 `Unverifiable`, 不猜测; 验证失败 (工具限流等) 记入报告不吞。

use std::fmt;

/// 可验证声明 (从反思文本提取).
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub text: String,
    /// 行号 (1-based, 供报告定位).
    pub line: usize,
}

/// 验证结论.
#[derive(Debug, Clone, PartialEq)]
pub enum Verification {
    /// 工具确认成立.
    Confirmed,
    /// 工具发现矛盾.
    Contradicted,
    /// 无法验证 (工具不可用/声明不可核).
    Unverifiable,
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verification::Confirmed => write!(f, "confirmed"),
            Verification::Contradicted => write!(f, "contradicted"),
            Verification::Unverifiable => write!(f, "unverifiable"),
        }
    }
}

/// 验证器: 对单条声明给出外部核对结论.
#[async_trait::async_trait]
pub trait ClaimVerifier: Send + Sync {
    /// 验证一条声明; Err = 验证过程失败 (工具不可用等, 报告方记为 Unverifiable).
    async fn verify(&self, claim: &str) -> Result<Verification, String>;
}

/// 逐条声明 + 验证结论.
#[derive(Debug, Clone)]
pub struct CritiqueReport {
    pub items: Vec<(Claim, Verification)>,
}

impl CritiqueReport {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn confirmed(&self) -> usize {
        self.items.iter().filter(|(_, v)| *v == Verification::Confirmed).count()
    }

    pub fn contradicted(&self) -> usize {
        self.items.iter().filter(|(_, v)| *v == Verification::Contradicted).count()
    }

    pub fn unverifiable(&self) -> usize {
        self.items.iter().filter(|(_, v)| *v == Verification::Unverifiable).count()
    }

    /// markdown 报告 (回注反思/写记忆用).
    pub fn to_markdown(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let mut s = String::from("【反思核查】(CRITIC) 反思声明的工具核对结果:\n");
        for (c, v) in &self.items {
            let mark = match v {
                Verification::Confirmed => "✓",
                Verification::Contradicted => "✗",
                Verification::Unverifiable => "?",
            };
            s.push_str(&format!("  {mark} [L{}] {} — {v}\n", c.line, c.text));
        }
        s
    }
}

/// 声明提取启发式: 含推断词/不确定表述的句子视为可验证声明.
/// 0 假装: 行级规则, 非 LLM 语义判定 (LLM 判定留 trait 口由宿主升级).
const HINT_WORDS: &[&str] = &[
    "推测", "可能", "似乎", "我观察", "观察", "模式", "倾向", "看起来", "大约", "约", "趋势",
];

/// 从反思文本提取可验证声明 (按行).
pub fn extract_claims(text: &str) -> Vec<Claim> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // 跳过明显的结构行 (markdown 标题/列表符号/代码块标记)
        if line.starts_with('#') || line.starts_with("```") || line.starts_with("---") {
            continue;
        }
        let clean = line.trim_start_matches(['-', '*', '•', ' ', '\t']).trim();
        if clean.is_empty() {
            continue;
        }
        // 声明 = 含推断词的句子
        if HINT_WORDS.iter().any(|w| clean.contains(w)) {
            out.push(Claim { text: clean.to_string(), line: i + 1 });
        }
    }
    out
}

/// CRITIC 组合器: 反思文本 → 提取声明 → 逐条验证 → 报告.
pub struct ReflectionCritic<V: ClaimVerifier> {
    verifier: V,
}

impl<V: ClaimVerifier> ReflectionCritic<V> {
    pub fn new(verifier: V) -> Self {
        Self { verifier }
    }

    /// 核查反思文本; 验证过程失败 (Err) 的声明记为 Unverifiable (不吞不猜).
    pub async fn critique(&self, reflection_text: &str) -> CritiqueReport {
        let claims = extract_claims(reflection_text);
        let mut items = Vec::with_capacity(claims.len());
        for c in claims {
            let v = match self.verifier.verify(&c.text).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[critic] 验证失败 (记 Unverifiable): {e}");
                    Verification::Unverifiable
                }
            };
            items.push((c, v));
        }
        CritiqueReport { items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_inferential_claims_only() {
        let text = "主人最近似乎更喜欢深色主题。\n这是纯事实句, 无推断。\n- 列表项推测下周会忙\n## 标题\n```\ncode\n```";
        let claims = extract_claims(text);
        let texts: Vec<&str> = claims.iter().map(|c| c.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains("深色主题")), "含'似乎'应提取: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("下周会忙")), "列表内推断也应提取");
        assert!(!texts.iter().any(|t| t.contains("纯事实句")), "无推断词不应提取");
        assert!(!texts.iter().any(|t| t.contains("标题")), "标题行应被跳过");
    }

    #[test]
    fn line_numbers_are_1_based() {
        let text = "第一行。\n主人可能明天出门。";
        let claims = extract_claims(text);
        assert_eq!(claims[0].line, 2);
    }

    #[test]
    fn empty_text_yields_empty() {
        assert!(extract_claims("").is_empty());
        assert!(extract_claims("\n\n  \n").is_empty());
    }

    struct StubVerifier {
        confirmed: Vec<String>,
        contradicted: Vec<String>,
    }

    #[async_trait::async_trait]
    impl ClaimVerifier for StubVerifier {
        async fn verify(&self, claim: &str) -> Result<Verification, String> {
            if self.confirmed.iter().any(|c| claim.contains(c)) {
                Ok(Verification::Confirmed)
            } else if self.contradicted.iter().any(|c| claim.contains(c)) {
                Ok(Verification::Contradicted)
            } else {
                Ok(Verification::Unverifiable)
            }
        }
    }

    #[tokio::test]
    async fn critique_reports_verdicts() {
        let text = "主人可能明天出门。\n主人似乎喜欢蓝色。";
        let critic = ReflectionCritic::new(StubVerifier {
            confirmed: vec!["蓝色".to_string()],
            contradicted: vec!["出门".to_string()],
        });
        let report = critic.critique(text).await;
        assert_eq!(report.items.len(), 2);
        assert_eq!(report.contradicted(), 1);
        assert_eq!(report.confirmed(), 1);
        assert_eq!(report.unverifiable(), 0);
        let md = report.to_markdown();
        assert!(md.contains("【反思核查】"));
        assert!(md.contains("✗"));
        assert!(md.contains("✓"));
    }

    #[tokio::test]
    async fn verifier_error_becomes_unverifiable() {
        struct FailingVerifier;
        #[async_trait::async_trait]
        impl ClaimVerifier for FailingVerifier {
            async fn verify(&self, _claim: &str) -> Result<Verification, String> {
                Err("工具限流".to_string())
            }
        }
        let critic = ReflectionCritic::new(FailingVerifier);
        let report = critic.critique("主人可能明天出门。").await;
        assert_eq!(report.unverifiable(), 1, "验证失败 → Unverifiable (不吞不猜)");
    }

    #[tokio::test]
    async fn no_claims_yields_empty_report() {
        let critic = ReflectionCritic::new(StubVerifier { confirmed: vec![], contradicted: vec![] });
        let report = critic.critique("纯事实, 无推断。").await;
        assert!(report.is_empty());
        assert!(report.to_markdown().is_empty());
    }
}
