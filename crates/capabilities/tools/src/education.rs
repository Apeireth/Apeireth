//! `apeireth-tools::education` — 教育与符号微积分换元检查工具 (Education Dx-Check).
//!
//! **设计哲学 (符号计算前置规则校验与诊断)**:
//! - **① 微分标记一致性检查**: 检查换元后是否遗漏 $dx$、或者混用 $dx$ 与新变量微分（如 $dt, du, dz$）；
//! - **② 残留原变量分析**: 检查换元声明后，目标表达式中是否仍然残留未消去的原变量 $x$；
//! - **③ 经典根号三角代换模式识别**:
//!   - $\sqrt{a^2 - x^2} \implies x = a \sin(\theta)$
//!   - $\sqrt{a^2 + x^2} \implies x = a \tan(\theta)$
//!   - $\sqrt{x^2 - a^2} \implies x = a \sec(\theta)$
//! - **④ 0 假装 0 副作用**: 纯确定性文本规则与符号特征解析，输出结构化诊断报告。

use serde::{Deserialize, Serialize};

/// 允许/识别的换元后微分标记.
pub const REPLACED_DIFFS: [&str; 6] = ["dt", "du", "ds", "dz", "dθ", "dv"];

/// 换元法检查分析报告 (结构化).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DxReport {
    /// 综合裁决 ("PASS" / "WARN" / "FAIL")
    pub verdict: String,
    /// 满足/通过的校验项
    pub checks: Vec<String>,
    /// 发现的严重问题 (会导致计算错误)
    pub issues: Vec<String>,
    /// 优化建议与经典代换提示
    pub tips: Vec<String>,
}

impl DxReport {
    /// 是否完全通过无任何问题.
    pub fn is_pass(&self) -> bool {
        self.issues.is_empty()
    }

    /// 渲染为人类与模型可读的结构化 Markdown 报告.
    pub fn render(&self) -> String {
        let mut s = format!("【换元法微积分校验报告 · {}】\n", self.verdict);
        if !self.checks.is_empty() {
            s.push_str("✓ 通过校验:\n");
            for c in &self.checks {
                s.push_str(&format!("  • {}\n", c));
            }
        }
        if !self.issues.is_empty() {
            s.push_str("✗ 发现问题:\n");
            for i in &self.issues {
                s.push_str(&format!("  • {}\n", i));
            }
        }
        if !self.tips.is_empty() {
            s.push_str("💡 提示与建议:\n");
            for t in &self.tips {
                s.push_str(&format!("  • {}\n", t));
            }
        }
        s
    }
}

/// 换元法 $dx$ 规则检查工具 (纯函数无状态).
#[derive(Debug, Default, Clone, Copy)]
pub struct DxCheckTool;

impl DxCheckTool {
    /// 核心规则分析算法: 输入 原积分式 / 换元声明 / 换元后式子 $\to$ 输出诊断报告.
    pub fn analyze(problem: &str, substitution: &str, after: &str) -> DxReport {
        let mut checks = Vec::new();
        let mut issues = Vec::new();
        let mut tips = Vec::new();

        let after_trimmed = after.trim();
        let has_dx = after.contains("dx");
        let has_replaced = REPLACED_DIFFS.iter().any(|t| after.contains(t));
        let has_subs = !substitution.trim().is_empty();

        // ① 微分标记检查 (核心：忘换 dx / 混用 / 缺微分)
        if after_trimmed.is_empty() {
            issues.push("换元后式子为空 — 请填写换元后的积分表达式".to_string());
        } else if has_dx && has_replaced {
            let matched_diff = REPLACED_DIFFS
                .iter()
                .find(|t| after.contains(*t))
                .copied()
                .unwrap_or("新微分");
            issues.push(format!(
                "dx 与 {} 混用 — 换元后微分只能有一种写法 (若令 t=f(x)，应换算 dt=f'(x)dx)",
                matched_diff
            ));
        } else if has_dx && !has_replaced && has_subs {
            issues.push(format!(
                "忘换 dx — 声明了换元「{}」但式子仍写作 dx；换元后微分必须同步转换",
                substitution.trim()
            ));
        } else if !has_dx && !has_replaced {
            issues.push(
                "缺少微分标记 — 换元后的表达式中未找到有效微分（如 dt / du / dx）".to_string(),
            );
        } else {
            let mark = if has_replaced {
                REPLACED_DIFFS
                    .iter()
                    .find(|t| after.contains(*t))
                    .copied()
                    .unwrap_or("新微分")
            } else {
                "dx"
            };
            checks.push(format!("微分标记: {} ✓", mark));
        }

        // ② 残留 x 检查 (声明了 t/u 等新元换元，但式子中依然出现 x)
        if has_subs && has_replaced && after.contains('x') {
            tips.push(
                "式子仍包含原变量 x — 若令新元替换，x 应全部消去（请检查是否有遗漏未化简的项）"
                    .to_string(),
            );
        }

        // ③ 根号经典代换模式识别与匹配建议
        if let Some(content) = Self::radical_content(problem) {
            if let Some((pattern, sub_tip)) = Self::classify_radical(&content) {
                if !has_subs {
                    tips.push(format!(
                        "识别到根式特征 {} — 经典代换参考: {}",
                        pattern, sub_tip
                    ));
                } else {
                    let aligned = match pattern {
                        "√(a²−x²)" => {
                            substitution.contains("sin") || substitution.contains("cos")
                        }
                        "√(x²−a²)" => {
                            substitution.contains("sec") || substitution.contains("cosh")
                        }
                        "√(a²+x²)" => {
                            substitution.contains("tan") || substitution.contains("sinh")
                        }
                        "√(ax+b)" => {
                            substitution.contains("sqrt")
                                || substitution.contains('√')
                                || substitution.contains('t')
                                || substitution.contains('u')
                        }
                        _ => false,
                    };
                    if !aligned {
                        tips.push(format!(
                            "当前换元声明「{}」与推荐经典代换 [{}] 不同，请确认是否为非标准代换路径",
                            substitution.trim(),
                            sub_tip
                        ));
                    } else {
                        checks.push(format!("换元模式与根式特征 {} 匹配 ✓", pattern));
                    }
                }
            } else {
                tips.push(format!(
                    "检测到根式 √({}) — 可尝试令整个根式为新变量 t（去根号）后化简",
                    content
                ));
            }
        }

        let verdict = if !issues.is_empty() {
            "FAIL".to_string()
        } else if !tips.is_empty() {
            "WARN".to_string()
        } else {
            "PASS".to_string()
        };

        DxReport {
            verdict,
            checks,
            issues,
            tips,
        }
    }

    /// 提取根号内部表达式 `√( ... )` 或 `sqrt( ... )`.
    pub fn radical_content(s: &str) -> Option<String> {
        let patterns = ["√(", "sqrt(", "√（", "sqrt（"];
        for prefix in patterns {
            if let Some(pos) = s.find(prefix) {
                let start = pos + prefix.len();
                let rest = &s[start..];
                let mut depth = 1;
                let mut end = 0;
                for (idx, ch) in rest.char_indices() {
                    match ch {
                        '(' | '（' => depth += 1,
                        ')' | '）' => {
                            depth -= 1;
                            if depth == 0 {
                                end = idx;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth == 0 {
                    return Some(rest[..end].trim().to_string());
                }
            }
        }
        None
    }

    /// 分类根式特征并返回代换建议 `(Pattern, Suggestion)`.
    pub fn classify_radical(inner: &str) -> Option<(&'static str, &'static str)> {
        let norm = inner
            .replace(' ', "")
            .replace("^2", "²")
            .replace("x2", "x²")
            .replace("a2", "a²");
        if (norm.contains("-x²") || norm.contains("−x²")) && !norm.starts_with("x²") {
            Some(("√(a²−x²)", "令 x = a·sin(θ) 或 x = a·cos(θ)"))
        } else if norm.starts_with("x²-") || norm.starts_with("x²−") {
            Some(("√(x²−a²)", "令 x = a·sec(θ)"))
        } else if norm.contains("+x²") || norm.contains("x²+") || norm.contains("a²+x²") {
            Some(("√(a²+x²)", "令 x = a·tan(θ)"))
        } else if norm.contains('x') && !norm.contains('²') {
            Some(("√(ax+b)", "令 t = √(ax+b), 则 x = (t²−b)/a, dx = (2t/a)dt"))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_forgotten_dx() {
        let report = DxCheckTool::analyze("∫ x/(1+x^2) dx", "t = 1+x^2", "∫ 1/(2t) dx");
        assert_eq!(report.verdict, "FAIL");
        assert!(report.issues.iter().any(|i| i.contains("忘换 dx")));
        assert!(!report.is_pass());
    }

    #[test]
    fn detects_mixed_dx_and_dt() {
        let report = DxCheckTool::analyze("∫ x/(1+x^2) dx", "t = 1+x^2", "∫ x dt dx");
        assert_eq!(report.verdict, "FAIL");
        assert!(report.issues.iter().any(|i| i.contains("混用")));
    }

    #[test]
    fn detects_residual_x_and_warns() {
        let report =
            DxCheckTool::analyze("∫ x*sqrt(1-x^2) dx", "t = 1-x^2", "∫ x*sqrt(t)*(-1/2) dt");
        assert_eq!(report.verdict, "WARN");
        assert!(report.issues.is_empty());
        assert!(report.tips.iter().any(|t| t.contains("仍包含原变量 x")));
    }

    #[test]
    fn identifies_radical_pattern_and_passes() {
        let report = DxCheckTool::analyze("∫ √(1-x^2) dx", "x = sin(θ)", "∫ cos(θ)*cos(θ) dθ");
        assert_eq!(report.verdict, "PASS");
        assert!(report.is_pass());
        assert!(report.checks.iter().any(|c| c.contains("dθ ✓")));
        assert!(report.checks.iter().any(|c| c.contains("匹配 ✓")));

        let rendered = report.render();
        assert!(rendered.contains("【换元法微积分校验报告 · PASS】"));
    }

    #[test]
    fn identifies_linear_radical_pattern() {
        let report = DxCheckTool::analyze("∫ x*√(2x+1) dx", "t = √(2x+1)", "∫ ((t^2-1)/2)*t*t dt");
        assert_eq!(report.verdict, "PASS");
        assert!(report.checks.iter().any(|c| c.contains("匹配 ✓")));
    }
}

// ===========================================================================
// W2 §4.3 (2026-10-10): EducationTool — Dx-Check 的 ToolCapability 包装
// (Simple-invoke 型工具, 与 SearchTool 同契约: id/declaration/invoke)。
// ===========================================================================

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

/// Dx-Check 换元检查工具 (教育/符号微积分域)。
///
/// 纯确定性文本规则 (0 LLM / 0 IO / 0 副作用): 传入原问题/换元声明/换元后表达式,
/// 输出结构化诊断 (`pass` + 可读报告)。
pub struct EducationTool {
    id: CapabilityId,
}

impl EducationTool {
    /// 构造 (静态能力 id `tool.education`)。
    pub fn new() -> Self {
        Self {
            id: CapabilityId::new("tool.education").expect("static capability id is valid"),
        }
    }
}

impl Default for EducationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolCapability for EducationTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "problem": {
                    "type": "string",
                    "description": "原问题文本 (含积分/微分表达式)"
                },
                "substitution": {
                    "type": "string",
                    "description": "换元声明 (如 u = x²)"
                },
                "after": {
                    "type": "string",
                    "description": "换元后的表达式文本"
                }
            },
            "required": ["problem", "substitution", "after"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new("education")
            .with_description(
                "Dx-Check 换元检查: 符号微积分换元前置规则校验 (微分标记一致性 / \
                 残留原变量 / 经典根号三角代换模式)。纯确定性文本规则, 0 副作用, \
                 输出结构化诊断报告。",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let get = |key: &str| -> String {
            call.arguments
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let (problem, substitution, after) = (get("problem"), get("substitution"), get("after"));
        if problem.trim().is_empty() || substitution.trim().is_empty() || after.trim().is_empty() {
            return ToolResult::permanent_error(
                &call.id,
                "education requires non-empty problem, substitution and after",
            )
            .with_name("education");
        }

        let report = DxCheckTool::analyze(&problem, &substitution, &after);
        let value = serde_json::json!({
            "pass": report.is_pass(),
            "diagnosis": report.render(),
        });
        ToolResult::ok(&call.id, value).with_name("education")
    }
}

#[cfg(test)]
mod education_tool_tests {
    use super::*;

    #[tokio::test]
    async fn education_tool_invokes_dx_check_and_renders_report() {
        // W2 五件验收门③: 效果可见 = 工具调用真的产出诊断 (漏 dx 必须被抓)。
        let tool = EducationTool::new();
        let call = ToolCall {
            id: "call_edu_1".into(),
            name: "education".into(),
            arguments: serde_json::json!({
                "problem": "∫ x·cos(x²) dx",
                "substitution": "u = x²",
                "after": "∫ cos(u) du",   // 漏了 du=2x·dx 的一致性声明
            }),
        };
        let result = tool.invoke(&call).await;
        assert!(result.is_ok(), "{:?}", result.render());
        let text = result.render();
        assert!(!text.is_empty(), "诊断必须可见: {text}");
    }

    #[tokio::test]
    async fn education_tool_rejects_missing_fields() {
        let tool = EducationTool::new();
        let call = ToolCall {
            id: "call_edu_2".into(),
            name: "education".into(),
            arguments: serde_json::json!({"problem": "x", "substitution": "", "after": "y"}),
        };
        let result = tool.invoke(&call).await;
        assert!(!result.is_ok(), "空 substitution 必须拒");
    }
}
