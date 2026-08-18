//! `apeireth-companion::education` — 教育升级套件内容 (插件: education-dx-check).
//!
//! 哲学 (docs/release-plan.md): 升级套件 = 「专业团队能力」; 插件 = 生态最小贡献单元.
//! 本模块 = 教育套件的**真内容**: 换元法 (不定积分) dx 检查器 + 插件装配.
//!
//! 0 假装 (诚实标注):
//! - v1 是**字符串级规则表**, 不是真实符号计算 (无 CAS 引擎, 不宣称能解积分)
//! - 覆盖四个检查: 忘换 dx / dx 与 dt 混用 / 缺微分 / 残留 x; 外加常见根号模式提示 (三角换元表)
//! - 模式匹配用简单字符串扫描 (√( 括号配对), 复杂公式请让 AI 自己拆解后调用
//!
//! 装配: `EducationDxPlugin` 的 on_load 注册 `dx_check` 工具 + 授权日常包;
//! 教育套件 (suites.rs) 声明 plugins: `["education-dx-check"]` — 插件先装, 套件才能装配.

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use serde_json::{json, Value};

use crate::plugin::Plugin;
use crate::tool_bridge::ToolBridge;

// ============================================================
// 换元法 dx 检查器 (规则层)
// ============================================================

/// 已替换微分的标记 (换元后应出现之一).
const REPLACED_DIFFS: [&str; 6] = ["dt", "du", "ds", "dz", "dθ", "dv"];

/// 分析报告 (诚实结构: 通过项/问题/提示 分列).
pub struct DxReport {
    pub verdict: &'static str,
    pub checks: Vec<String>,
    pub issues: Vec<String>,
    pub tips: Vec<String>,
}

/// 换元法 dx 检查工具 — 规则层 (无副作用, 纯文本分析).
pub struct DxCheckTool;

impl DxCheckTool {
    /// 核心规则表: 输入 原题 / 声明的换元 / 换元后式子 → 报告.
    pub fn analyze(problem: &str, substitution: &str, after: &str) -> DxReport {
        let mut checks = Vec::new();
        let mut issues = Vec::new();
        let mut tips = Vec::new();

        let has_dx = after.contains("dx");
        let has_replaced = REPLACED_DIFFS.iter().any(|t| after.contains(t));
        let has_subs = !substitution.trim().is_empty();

        // ① 微分检查 (核心: 忘换 dx)
        if after.trim().is_empty() {
            issues.push("换元后式子为空 — 请把换元后的积分式写进 after".to_string());
        } else if has_dx && has_replaced {
            issues.push(format!(
                "dx 与 {} 混用 — 换元后微分只能有一种写法 (若令 t=f(x), 应写 dt=f'(x)dx)",
                REPLACED_DIFFS
                    .iter()
                    .find(|t| after.contains(*t))
                    .unwrap_or(&"新微分")
            ));
        } else if has_dx && !has_replaced && has_subs {
            issues.push(format!(
                "忘换 dx — 你声明了换元「{substitution}」但式子仍写 dx; 换元后微分必须跟着换 (令 t=f(x) → dt=f'(x)dx)"
            ));
        } else if !has_dx && !has_replaced {
            issues.push(
                "缺微分标记 — 换元后的式子应有微分 (如 dt / du / dx), 检查是否写全".to_string(),
            );
        } else {
            let mark = if has_replaced {
                REPLACED_DIFFS
                    .iter()
                    .find(|t| after.contains(*t))
                    .unwrap_or(&"新微分")
            } else {
                "dx"
            };
            checks.push(format!("微分标记: {mark} ✓"));
        }

        // ② 残留 x 检查 (声明了 t/u 换元但式子还有 x)
        if has_subs && has_replaced && after.contains('x') {
            tips.push(
                "式子仍含 x — 若换元令 t=..., x 应全部换成 t 的表达式 (检查未替换干净的项)"
                    .to_string(),
            );
        }

        // ③ 根号模式提示 (三角换元表; 只提示不判决 — 换元选择是主人的自由)
        if let Some(content) = Self::radical_content(problem) {
            if let Some((pattern, sub_tip)) = Self::classify_radical(&content) {
                tips.push(format!("检测到 {pattern} — 经典换元: {sub_tip}"));
                // 若已声明换元且与推荐明显不符 → 温和提示核对
                if has_subs {
                    let aligned = match pattern {
                        "√(a²−x²)" => substitution.contains("sin"),
                        "√(x²−a²)" => substitution.contains("sec"),
                        "√(a²+x²)" => substitution.contains("tan"),
                        _ => false,
                    };
                    if !aligned {
                        tips.push(format!("你的换元「{substitution}」与 {pattern} 的经典换元不同 — 可能可行 (凑微分), 请自行核对 dx 是否成立"));
                    }
                }
            } else {
                tips.push(format!(
                    "检测到根式 √({content}) — 试试 令整个根式为 t (去根号), 再算 dx"
                ));
            }
        }

        let verdict = if !issues.is_empty() {
            "fix"
        } else if !tips.is_empty() {
            "warn"
        } else {
            "ok"
        };
        DxReport {
            verdict,
            checks,
            issues,
            tips,
        }
    }

    /// 提取 √( ... ) 的内容 (括号配对, 未闭合返回 None).
    fn radical_content(s: &str) -> Option<String> {
        let idx = s.find("√(")?;
        // '√' 是 3 字节 UTF-8, '(' 1 字节 → 跳过 4 字节 (不能 idx+2, 会落在字符中间)
        let rest = &s[idx + '√'.len_utf8() + 1..];
        let mut depth = 1usize;
        for (i, ch) in rest.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(rest[..i].to_string());
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// 根式模式分类 → (模式名, 经典换元建议).
    fn classify_radical(content: &str) -> Option<(&'static str, &'static str)> {
        let c = content.replace('²', "^2");
        if !c.contains("^2") {
            // 线性根式 √(ax+b) 或 √(1-x) 等
            if c.contains('x') && (c.contains('+') || c.contains('-')) {
                return Some((
                    "线性根式 √(ax+b)",
                    "令 t = √(ax+b), 则 x=(t²−b)/a, dx=(2t/a)dt",
                ));
            }
            return None;
        }
        if c.contains("-x^2") || c.ends_with("-x^2") {
            Some(("√(a²−x²)", "x = a·sinθ, dx = a·cosθ dθ"))
        } else if c.contains("x^2-") || c.starts_with("x^2-") {
            Some(("√(x²−a²)", "x = a·secθ, dx = a·secθ·tanθ dθ"))
        } else if c.contains("+x^2") || c.starts_with("x^2+") {
            Some(("√(a²+x²)", "x = a·tanθ, dx = a·sec²θ dθ"))
        } else {
            None
        }
    }

    fn to_json(r: &DxReport) -> Value {
        json!({
            "verdict": r.verdict,
            "checks": r.checks,
            "issues": r.issues,
            "tips": r.tips,
        })
    }
}

#[async_trait::async_trait]
impl Tool for DxCheckTool {
    fn name(&self) -> &str {
        "dx_check"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let problem = args
            .get("problem")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let substitution = args
            .get("substitution")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let after = args
            .get("after")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if problem.trim().is_empty() && after.trim().is_empty() {
            return Err("需要 problem (原题) 和/或 after (换元后的式子)".to_string());
        }
        Ok(Self::to_json(&Self::analyze(
            &problem,
            &substitution,
            &after,
        )))
    }
}

// ============================================================
// 插件装配: education-dx-check
// ============================================================

/// 教育套件插件: 注册 dx_check 工具 + 授权日常调用.
/// 装配路径: PluginRegistry.install → on_load → ToolBridge.registry.register + packs.grant.
pub struct EducationDxPlugin;

impl Plugin for EducationDxPlugin {
    fn id(&self) -> &str {
        "education-dx-check"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "换元法 dx 检查: 忘换 dx / 混用 / 缺微分 / 残留 x / 根号模式提示 (规则层)"
    }
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge
            .registry
            .register("dx_check".to_string(), Arc::new(DxCheckTool));
        bridge.packs.grant(crate::packs::PermissionPack::permanent(
            "教育插件授权",
            vec!["dx_check".to_string()],
        ));
        Ok(())
    }
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String> {
        // 真清理: 注销工具 + 撤销授权 (幂等; 卸载后 dx_check 不可再调)
        bridge.registry.unregister("dx_check");
        bridge.packs.revoke_by_name("教育插件授权");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catches_forgotten_dx() {
        // 经典错法: 令 t=x², 但式子还写 dx (验收场景: 主人「换元后忘记换 dx」)
        let r = DxCheckTool::analyze("∫ x·e^(x²) dx", "令 t = x²", "∫ e^t dx");
        assert_eq!(r.verdict, "fix");
        assert!(
            r.issues.iter().any(|i| i.contains("忘换 dx")),
            "{:?}",
            r.issues
        );
        assert!(
            r.issues.iter().any(|i| i.contains("dt=f'(x)dx")),
            "错误信息应教正确写法"
        );
    }

    #[test]
    fn ok_when_dx_replaced() {
        let r = DxCheckTool::analyze("∫ x·e^(x²) dx", "t = x²", "∫ e^t · (1/2) dt");
        assert_eq!(r.verdict, "ok", "{:?} {:?}", r.issues, r.tips);
        assert!(r.checks.iter().any(|c| c.contains("dt")));
    }

    #[test]
    fn mixed_dx_dt_detected() {
        let r = DxCheckTool::analyze("∫ x·√(1-x²) dx", "t = 1-x²", "∫ x·√(t) dx dt");
        assert_eq!(r.verdict, "fix");
        assert!(r.issues.iter().any(|i| i.contains("混用")));
    }

    #[test]
    fn missing_differential_detected() {
        let r = DxCheckTool::analyze("∫ x·e^(x²) dx", "t = x²", "∫ e^t · (1/2)");
        assert_eq!(r.verdict, "fix");
        assert!(r.issues.iter().any(|i| i.contains("缺微分")));
    }

    #[test]
    fn residual_x_hinted() {
        let r = DxCheckTool::analyze("∫ x·e^(x²) dx", "t = x²", "∫ x·e^t dt");
        assert_eq!(r.verdict, "warn", "残留 x 是提示不是硬错");
        assert!(r.tips.iter().any(|t| t.contains("仍含 x")));
    }

    #[test]
    fn radical_patterns_table() {
        // .1 是建议 (含换元写法); .0 是模式名
        assert!(
            DxCheckTool::classify_radical("a²-x²")
                .unwrap()
                .1
                .contains("sin"),
            "√(a²−x²) → sin"
        );
        assert!(
            DxCheckTool::classify_radical("x²-a²")
                .unwrap()
                .1
                .contains("sec"),
            "√(x²−a²) → sec"
        );
        assert!(
            DxCheckTool::classify_radical("a²+x²")
                .unwrap()
                .1
                .contains("tan"),
            "√(a²+x²) → tan"
        );
        assert!(
            DxCheckTool::classify_radical("1-x")
                .unwrap()
                .0
                .contains("线性"),
            "√(1-x) → 线性"
        );
        assert!(DxCheckTool::classify_radical("1-x²")
            .unwrap()
            .1
            .contains("sin"));
        // 完整问题文本 → 提示带建议换元
        let r = DxCheckTool::analyze("∫ x/√(1-x²) dx", "", "∫ x/√(1-x²) dx");
        assert!(r.tips.iter().any(|t| t.contains("sin")), "{:?}", r.tips);
    }

    #[test]
    fn no_substitution_no_issue_for_dx() {
        // 还没换元时写 dx 是正常的 → ok (无根式无换元, 无可提示)
        let r = DxCheckTool::analyze("∫ x·e^(x²) dx", "", "∫ x·e^(x²) dx");
        assert_eq!(r.verdict, "ok", "无换元声明无根式 → 干净通过");
        assert!(r.issues.is_empty());
    }

    #[test]
    fn unclosed_radical_is_none() {
        assert!(DxCheckTool::radical_content("∫ x·√(1-x² dx").is_none());
        assert!(DxCheckTool::radical_content("∫ x·√(1-(x+1)²) dx").is_some());
    }

    #[tokio::test]
    async fn plugin_registers_tool_and_pack() {
        use apeireth_memory::SqliteMemoryStore;
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = crate::plugin::PluginRegistry::new();
        reg.install(&bridge, Arc::new(EducationDxPlugin)).unwrap();
        assert!(reg.is_installed("education-dx-check"));
        assert!(bridge.registry.list().iter().any(|n| n == "dx_check"));
        // 授权包覆盖 → 免现场审批直接执行
        assert!(bridge
            .packs
            .check_and_consume("dx_check", chrono::Utc::now().timestamp_millis()));
        // 全链路: 桥执行 dx_check (忘换 dx 场景)
        let call = apeireth_tool_runtime::parser::ParsedToolCall {
            tool_name: "dx_check".into(),
            args: json!({
                "problem": "∫ x·e^(x²) dx",
                "substitution": "令 t = x²",
                "after": "∫ e^t dx"
            }),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "dx_check 应可执行: {:?}", r.error);
        assert_eq!(r.output["verdict"], json!("fix"));
        // 卸载 → 真清理: 工具注销 + 授权撤销 (幂等)
        reg.uninstall(&bridge, "education-dx-check").unwrap();
        assert!(
            !bridge.registry.list().iter().any(|n| n == "dx_check"),
            "卸载后工具应注销"
        );
        assert!(
            !bridge
                .packs
                .check_and_consume("dx_check", chrono::Utc::now().timestamp_millis()),
            "卸载后授权应撤销"
        );
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success, "卸载后 dx_check 不可再调");
    }
}
