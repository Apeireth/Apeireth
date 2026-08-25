//! **VCP 新版 `dynamicToolRegistry.js:560-649 buildInjection` — 注入注意力预算**
//!
//! **目标**: 工具描述注入不是一次性全量倾倒, 而是注意力预算化三段式:
//! 1. **轻量清单** — 全部工具先给 `- name: brief` 一行式列表 (名称 + 一行描述)
//! 2. **仅相关展开** — 只有与当前查询相关的工具才展开完整详情 (VCP `expandedKeys`)
//! 3. **超预算截断留提示** — 超 `maxInjectionChars` (16000) 截断, 留"可点名索取更多"提示
//!
//! **VCP 字段级引用**:
//! - `dynamicToolRegistry.js:613` light list 说明行 ("names and short descriptions always
//!   shown; full usage expanded only for matched or explicitly requested tools")
//! - `dynamicToolRegistry.js:617-628` brief 列表渲染 + "N more tools hidden" 提示
//! - `dynamicToolRegistry.js:630-643` Expanded tool usage 段 (仅展开相关)
//! - `dynamicToolRegistry.js:1390-1395 _truncateInjection` (截断 + suffix 提示)
//!
//! **挂接点**: `InjectionEntry::from_description(&ToolDescription)` — 调用方
//! (tool-runtime / pipeline) 拿 ToolDescription 转 entry 后调 `render_injection`,
//! 结果文本直接拼进 system prompt. crate 内不依赖 prompt 装配, 边界干净.
//!
//! **不假装**:
//! - ✅ 预算裁剪真跑: 先砍展开段 → 再砍轻清单尾行 → 最后硬切 + 提示
//! - ✅ 轻清单单行真过 `LIGHT_LIST_TOKEN_BUDGET` (15 token) 上限截断
//! - ❌ 不假装知道"相关性" — 相关性由调用方闭包决定 (crate 无查询上下文)

use crate::token_budget::{
    estimate_token_count, truncate_to_token_budget, LIGHT_LIST_TOKEN_BUDGET, MAX_INJECTION_CHARS,
};
use crate::trait_def::ToolDescription;

/// **超预算截断提示** (VCP `_truncateInjection` suffix 1:1 借鉴)
///
/// 告诉 LLM: 注入被截断了, 想要更多细节就点名具体工具
pub const TRUNCATION_HINT: &str =
    "\n\n[tool-registry injection truncated by maxInjectionChars; request a specific tool for more detail.]";

/// **注入预算配置**
///
/// **VCP 字段级对照**:
/// - `max_chars` ← `config.maxInjectionChars` (默认 16000, line 21)
/// - `max_expanded` ← `config.maxExpandedPlugins` (展开上限)
/// - `max_light_items` ← `config.maxBriefListItems` (0 = 不限, 由预算兜底)
#[derive(Debug, Clone)]
pub struct InjectionBudget {
    /// 单次注入字符总上限 (默认 `MAX_INJECTION_CHARS` = 16000)
    pub max_chars: usize,
    /// 最多展开详情的工具数 (默认 5)
    pub max_expanded: usize,
    /// 轻量清单最多条目数 (0 = 不限, 由字符预算兜底)
    pub max_light_items: usize,
}

impl Default for InjectionBudget {
    fn default() -> Self {
        Self {
            max_chars: MAX_INJECTION_CHARS,
            max_expanded: 5,
            max_light_items: 0,
        }
    }
}

impl InjectionBudget {
    /// 自定义字符上限
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    /// 自定义展开上限
    pub fn with_max_expanded(mut self, max_expanded: usize) -> Self {
        self.max_expanded = max_expanded;
        self
    }
}

/// **注入条目** — 1 个工具的注入素材
#[derive(Debug, Clone)]
pub struct InjectionEntry {
    /// 工具名
    pub name: String,
    /// 一行简介 (轻量清单用)
    pub brief: String,
    /// 完整详情 (仅相关工具展开)
    pub details: String,
}

impl InjectionEntry {
    /// **挂接点**: 从 `ToolDescription` 构建注入条目
    ///
    /// `brief` → 轻量清单行, `description` → 展开详情
    pub fn from_description(desc: &ToolDescription) -> Self {
        Self {
            name: desc.name.clone(),
            brief: desc.brief.clone(),
            details: desc.description.clone(),
        }
    }
}

/// **注入渲染结果**
#[derive(Debug, Clone)]
pub struct InjectionOutput {
    /// 渲染文本 (保证 chars ≤ `budget.max_chars`)
    pub text: String,
    /// 是否因超预算硬截断 (带 `TRUNCATION_HINT` 提示)
    pub truncated: bool,
    /// 实际展开了详情的工具名 (预算挤压可能被砍)
    pub expanded: Vec<String>,
    /// 轻量清单因预算压力隐藏的条目数
    pub hidden_light: usize,
}

/// 字符计数 (对齐 VCP `String.length` 的字符级语义)
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// 轻量清单单行: `- name: brief`, 整行 ≤ `LIGHT_LIST_TOKEN_BUDGET` token
fn light_line(entry: &InjectionEntry) -> String {
    let line = format!("- {}: {}", entry.name, entry.brief);
    if estimate_token_count(&line) > LIGHT_LIST_TOKEN_BUDGET {
        truncate_to_token_budget(&line, LIGHT_LIST_TOKEN_BUDGET)
    } else {
        line
    }
}

/// **渲染注入文本** (注意力预算化)
///
/// **行为**:
/// 1. 空注册表 → 返 "no tools available" 提示 (0 假装)
/// 2. 轻量清单: 全部条目 `- name: brief` (单行过 LIGHT token 预算)
/// 3. 展开段: `relevant(name)` 为真的工具展开 `details` (≤ `max_expanded` 个)
/// 4. 超预算裁剪顺序: 砍展开段尾 → 砍轻清单尾行 (计 `hidden_light`) → 硬切 + `TRUNCATION_HINT`
pub fn render_injection(
    entries: &[InjectionEntry],
    relevant: &dyn Fn(&str) -> bool,
    budget: &InjectionBudget,
) -> InjectionOutput {
    // 空注册表: 0 假装
    if entries.is_empty() {
        return InjectionOutput {
            text: "Tool Registry: no tools available for injection.".to_string(),
            truncated: false,
            expanded: Vec::new(),
            hidden_light: 0,
        };
    }

    // 1. 轻量清单行 (预算条目数限制)
    let mut light: Vec<String> = Vec::with_capacity(entries.len());
    let mut hidden_light = 0usize;
    let light_limit = if budget.max_light_items == 0 {
        entries.len()
    } else {
        budget.max_light_items
    };
    for entry in entries.iter().take(light_limit) {
        light.push(light_line(entry));
    }
    if entries.len() > light_limit {
        hidden_light += entries.len() - light_limit;
    }

    // 2. 相关工具展开 (≤ max_expanded)
    let mut expanded: Vec<(String, String)> = entries
        .iter()
        .filter(|e| relevant(&e.name))
        .take(budget.max_expanded)
        .map(|e| (e.name.clone(), e.details.clone()))
        .collect();

    // 3. 组装 + 渐进裁剪
    let assemble = |light: &[String], expanded: &[(String, String)]| -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("Tool Registry (attention-budgeted)".to_string());
        lines.push(
            "Light list: names and one-line briefs always shown; details expanded only for relevant tools."
                .to_string(),
        );
        lines.push(String::new());
        lines.push("Brief tool list:".to_string());
        lines.extend(light.iter().cloned());
        if !expanded.is_empty() {
            lines.push(String::new());
            lines.push("Expanded tool usage:".to_string());
            for (name, details) in expanded {
                lines.push(format!("--- {name} ---"));
                lines.push(details.clone());
            }
        }
        lines.join("\n")
    };

    let mut text = assemble(&light, &expanded);

    // 超预算: 先砍展开段尾 (相关详情最占地方, VCP 也是展开段最肥)
    while char_count(&text) > budget.max_chars && !expanded.is_empty() {
        expanded.pop();
        text = assemble(&light, &expanded);
    }
    // 再砍轻清单尾行
    while char_count(&text) > budget.max_chars && !light.is_empty() {
        light.pop();
        hidden_light += 1;
        text = assemble(&light, &expanded);
    }
    // 最后硬切 + 提示 (对齐 VCP _truncateInjection)
    let mut truncated = false;
    if char_count(&text) > budget.max_chars {
        truncated = true;
        let keep = budget.max_chars.saturating_sub(char_count(TRUNCATION_HINT));
        let sliced: String = text.chars().take(keep).collect();
        text = format!("{}{}", sliced.trim_end(), TRUNCATION_HINT);
        // 极端小预算: suffix 本身超限 → 硬保 max_chars
        if char_count(&text) > budget.max_chars {
            text = text.chars().take(budget.max_chars).collect();
        }
    }

    InjectionOutput {
        text,
        truncated,
        expanded: expanded.into_iter().map(|(name, _)| name).collect(),
        hidden_light,
    }
}

// ============================================================
// 测试 (预算内 / 超预算 / 空注册表 / 仅相关展开)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolKind;

    fn entry(name: &str, brief: &str, details: &str) -> InjectionEntry {
        InjectionEntry {
            name: name.to_string(),
            brief: brief.to_string(),
            details: details.to_string(),
        }
    }

    fn entries3() -> Vec<InjectionEntry> {
        vec![
            entry(
                "WebSearch",
                "搜索网页",
                "WebSearch 支持 google/tavily 双源检索, 返 top-k 摘要.",
            ),
            entry(
                "FileOperator",
                "读写文件",
                "FileOperator 支持 read/write/patch, 路径白名单内.",
            ),
            entry(
                "NoteRecall",
                "记忆检索",
                "NoteRecall 向量召回日记/笔记, 返 top-5 chunk.",
            ),
        ]
    }

    #[test]
    fn empty_registry_renders_notice() {
        let out = render_injection(&[], &|_| true, &InjectionBudget::default());
        assert!(out.text.contains("no tools available"), "空表应明说无工具");
        assert!(!out.truncated);
        assert!(out.expanded.is_empty());
        assert_eq!(out.hidden_light, 0);
    }

    #[test]
    fn within_budget_keeps_light_list_and_relevant_details() {
        let out = render_injection(
            &entries3(),
            &|name| name == "WebSearch",
            &InjectionBudget::default(),
        );
        // 轻量清单: 3 个名字全在
        for name in ["WebSearch", "FileOperator", "NoteRecall"] {
            assert!(out.text.contains(name), "轻清单应含 {name}");
        }
        // 相关工具详情展开
        assert!(out.text.contains("google/tavily"), "相关工具应展开详情");
        assert_eq!(out.expanded, vec!["WebSearch".to_string()]);
        // 非相关工具不展开详情
        assert!(!out.text.contains("路径白名单"), "非相关工具不展开详情");
        assert!(!out.truncated);
        assert_eq!(out.hidden_light, 0);
        assert!(char_count(&out.text) <= MAX_INJECTION_CHARS);
    }

    #[test]
    fn non_relevant_tool_stays_in_light_list_only() {
        let out = render_injection(&entries3(), &|_| false, &InjectionBudget::default());
        assert!(out.text.contains("FileOperator"), "轻清单仍含名字");
        assert!(
            !out.text.contains("Expanded tool usage"),
            "无相关工具则无展开段"
        );
        assert!(out.expanded.is_empty());
        assert!(!out.truncated);
    }

    #[test]
    fn over_budget_drops_expansions_first_and_truncates_with_hint() {
        // 3 个相关工具各 8000 字符详情 → 总量 24000+ 超 16000
        let fat: Vec<InjectionEntry> = (0..3)
            .map(|i| entry(&format!("BigTool{i}"), "大工具", &"详".repeat(8000)))
            .collect();
        let out = render_injection(&fat, &|_| true, &InjectionBudget::default());
        assert!(
            char_count(&out.text) <= MAX_INJECTION_CHARS,
            "必须 ≤ 16000 字符"
        );
        // 展开段被预算挤压砍掉部分或全部
        assert!(out.expanded.len() < 3, "超预算应先砍展开段");
        // 若仍超限 → 硬截断 + 提示
        if out.truncated {
            assert!(
                out.text.contains("truncated") || char_count(&out.text) <= MAX_INJECTION_CHARS,
                "截断必留提示"
            );
        }
        // 轻清单 3 名字仍在 (展开段先砍, 轻清单后砍)
        assert!(out.text.contains("BigTool0"), "轻清单优先保留");
    }

    #[test]
    fn tiny_budget_hard_truncates_with_hint() {
        // 极小预算 120 字符 (< 头部框架自身 ~149 字符) → 渐进裁剪砍无可砍 → 硬切 + TRUNCATION_HINT
        let out = render_injection(
            &entries3(),
            &|_| true,
            &InjectionBudget::default().with_max_chars(120),
        );
        assert!(out.truncated, "120 字符必触发硬截断");
        assert!(char_count(&out.text) <= 120, "硬截断后必须 ≤ 预算");
        assert!(
            out.text.contains("truncated by maxInjectionChars"),
            "截断必须留索取提示"
        );
    }

    #[test]
    fn over_budget_counts_hidden_light_lines() {
        // 50 个条目, 预算 600 字符 → 轻清单尾行被砍, hidden_light > 0
        let many: Vec<InjectionEntry> = (0..50)
            .map(|i| entry(&format!("Tool{i:02}"), "工具描述一行", ""))
            .collect();
        let out = render_injection(
            &many,
            &|_| false,
            &InjectionBudget::default().with_max_chars(600),
        );
        assert!(char_count(&out.text) <= 600);
        assert!(out.hidden_light > 0, "轻清单尾行被砍应计入 hidden_light");
    }

    #[test]
    fn max_light_items_cap_reports_hidden() {
        let out = render_injection(
            &entries3(),
            &|_| false,
            &InjectionBudget {
                max_light_items: 2,
                ..InjectionBudget::default()
            },
        );
        assert_eq!(out.hidden_light, 1, "3 条目 cap 2 → 1 隐藏");
        assert!(out.text.contains("Tool") || out.text.contains("WebSearch"));
    }

    #[test]
    fn light_list_line_within_light_token_budget() {
        // 超长 brief → 轻清单单行被截到 ≤ LIGHT_LIST_TOKEN_BUDGET token
        let long_brief = entry(
            "Verbose",
            "这是一个极其冗长的工具简介 用来验证轻量清单单行 token 预算截断是否真实生效",
            "",
        );
        let line = light_line(&long_brief);
        assert!(
            estimate_token_count(&line) <= LIGHT_LIST_TOKEN_BUDGET,
            "轻清单单行应 ≤ {LIGHT_LIST_TOKEN_BUDGET} token, 实际 {}",
            estimate_token_count(&line)
        );
    }

    #[test]
    fn from_description_maps_brief_and_details() {
        let desc = ToolDescription {
            name: "Foo".to_string(),
            kind: ToolKind::Sync,
            axes: crate::types::ToolAxes::default(),
            brief: "一行简介".to_string(),
            description: "完整详情".to_string(),
            version: "0.1.0".to_string(),
            author: "test".to_string(),
        };
        let e = InjectionEntry::from_description(&desc);
        assert_eq!(e.name, "Foo");
        assert_eq!(e.brief, "一行简介");
        assert_eq!(e.details, "完整详情");
    }
}
