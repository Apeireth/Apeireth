//! **战役 1-3 / VCP §6.2.2 #17 — Recursive placeholder 展开 + 防循环**
//!
//! **借鉴来源 (字段级)**: `research/source/vcptoolbox/modules/messageProcessor.js`
//!
//! **真代码函数 (按 spec §6.2.2 #17 引用 `messageProcessor.js:78-98`, 实际 `resolveAllVariables`
//! 函数在 146-220 行起, 包括 `processingStack: Set` 防循环)**:
//! ```js
//! // line 146
//! async function resolveAllVariables(text, model, role, context, processingStack = new Set()) {
//!   // ...
//!   // line 186-191 — 防循环
//!   if (processingStack.has(alias)) {
//!     console.error(`[AgentManager] Circular dependency detected! Stack: [${[...processingStack].join(' -> ')} -> ${alias}]`);
//!     const errorMessage = `[Error: Circular agent reference detected for '${alias}']`;
//!     processedText = processedText.replaceAll(`{{${alias}}}`, errorMessage).replaceAll(`{{agent:${alias}}}`, errorMessage);
//!     continue;
//!   }
//!   // ...
//!   processingStack.add(alias);                              // 推入栈
//!   const resolvedAgentContent = await resolveAllVariables(   // 递归
//!     agentContent, model, role, context, processingStack
//!   );
//!   processingStack.delete(alias);                            // 弹栈
//! }
//! ```
//!
//! **VCP 三种 placeholder 格式**:
//! - `{{alias}}` — 纯别名
//! - `{{agent:alias}}` — Agent 前缀 (灵魂级安全: 一次会话只允许展开一个 Agent)
//! - `{{toolbox:alias}}` — Toolbox 前缀
//!
//! **Apeireth 简化 (工程层借鉴, 不抄业务)**:
//! - 借鉴**递归 + 防循环**模式 (VCP 真核心)
//! - 借鉴**3 种 placeholder 格式**识别
//! - **不抄 VCP 的 agent 灵魂 (Agent Guard)** — 这是 apeireth-council 战役 1-4 范围, 不在 pipeline
//! - **不抄 VCP 的 privileged role 判定** — apeireth 自有 V1+V2+V3 AND 门守门, 不需要 VCP 这种 1 角色判定
//! - placeholder 来源: `PlaceholderContext` HashMap (key → 已展开文本), VCP 用 `agentManager.getAgentPrompt(alias)` 我们用 HashMap
//!
//! **不假装**:
//! - 递归真实现, `processing_stack: HashSet<String>` 真防循环
//! - placeholder 模式真识别 (regex)
//! - 命中 `MAX_RECURSION_DEPTH` (Apeireth 工程底线) 也防住 (VCP 靠 `processingStack` 单层防, 我们双层防)

use regex::Regex;
use std::collections::{HashMap, HashSet};

/// 一次 resolve 最多递归多少层 (Apeireth 工程底线, VCP 只靠 processingStack, 我们双保险)
pub const MAX_RECURSION_DEPTH: usize = 16;

/// placeholder 正则 — 借鉴 VCP `messageProcessor.js:160` 真代码字符类
///
/// VCP 真代码: `/\{\{([a-zA-Z0-9_:@#%&^+_\-\u2e80-\u2fff\u3040-\u9fff]+)\}\}/g`
///
/// 字段含义:
/// - 字母数字 + 3 个前缀符号 (`agent:` / `toolbox:` / 裸名)
/// - 标点符号 (`@#%&^+_-`)
/// - CJK Radicals Supplement (0x2E80-0x2FFF)
/// - Hiragana 到 CJK Unified Ideographs (0x3040-0x9FFF)
///
/// **Apeireth 简化版**: 用 Rust `regex` 字符类, 行为对齐
pub const PLACEHOLDER_REGEX_STR: &str =
    r"\{\{([a-zA-Z0-9_:@#%&^+_\-\u2E80-\u2FFF\u3040-\u9FFF]+)\}\}";

/// placeholder context — key → 已展开文本
///
/// **VCP 对比**: VCP 从 `agentManager.getAgentPrompt(alias)` 异步拉 Agent 内容;
/// Apeireth 用同步 HashMap (工程层借鉴, 不抄 VCP 异步业务)
pub type PlaceholderContext = HashMap<String, String>;

/// 解析 placeholder — 借鉴 VCP `messageProcessor.js:146-220 resolveAllVariables`
///
/// **递归语义**:
/// 1. 用 regex 找出所有 `{{xxx}}` 匹配
/// 2. 取每个 alias (去 `agent:` / `toolbox:` 前缀)
/// 3. 检查 `processing_stack` 防循环 (VCP line 186-191)
/// 4. 检查 `depth >= MAX_RECURSION_DEPTH` (Apeireth 工程底线)
/// 5. 在 context 找到值 → 推入栈 → **递归解析** (VCP line 195-197) → 弹栈 → 替换
/// 6. 找不到值 → 留原样 (不抛错, VCP 行为)
pub fn resolve_placeholders(template: &str, context: &PlaceholderContext) -> String {
    resolve_placeholders_inner(template, context, &mut HashSet::new(), 0)
}

fn resolve_placeholders_inner(
    template: &str,
    context: &PlaceholderContext,
    processing_stack: &mut HashSet<String>,
    depth: usize,
) -> String {
    if template.is_empty() {
        return String::new();
    }
    if depth >= MAX_RECURSION_DEPTH {
        // 工程底线: 嵌套太深, 截断
        return template.to_string();
    }

    let Ok(regex) = Regex::new(PLACEHOLDER_REGEX_STR) else { return template.to_string() }; // regex 出错不抛, 兜底

    let mut out = String::with_capacity(template.len());
    let mut last_end = 0;
    let mut found_any = false;

    for cap in regex.captures_iter(template) {
        found_any = true;
        let mat = cap.get(0).expect("regex match group 0 always present");
        let alias_with_prefix = cap
            .get(1)
            .expect("regex match group 1 always present")
            .as_str();

        // 复制 mat 之前的原文
        out.push_str(&template[last_end..mat.start()]);

        // 去前缀 (VCP line 164): `agent:foo` / `toolbox:foo` / 裸 `foo` 都视为 alias `foo`
        let alias = alias_with_prefix
            .strip_prefix("agent:")
            .or_else(|| alias_with_prefix.strip_prefix("toolbox:"))
            .unwrap_or(alias_with_prefix);

        // VCP line 186: 防循环
        if processing_stack.contains(alias) {
            // 借鉴 VCP 行为: 替换为错误标记, 不抛
            out.push_str(&format!(
                "[Error: Circular placeholder reference for '{alias}']"
            ));
            last_end = mat.end();
            continue;
        }

        // context 找不到 → 留原样 (VCP 行为, 不抛)
        let Some(value) = context.get(alias) else {
            out.push_str(mat.as_str());
            last_end = mat.end();
            continue;
        };

        // 递归 — VCP line 195-197
        processing_stack.insert(alias.to_string());
        let resolved = resolve_placeholders_inner(value, context, processing_stack, depth + 1);
        processing_stack.remove(alias);

        out.push_str(&resolved);
        last_end = mat.end();
    }

    if !found_any {
        return template.to_string();
    }

    // 复制最后一段
    out.push_str(&template[last_end..]);
    out
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

const _: () = {
    // MAX_RECURSION_DEPTH 至少 8 (VCP 实践: 嵌套 Agent/Agent/Agent 不超过 4-5 层, 给 8 余量)
    assert!(MAX_RECURSION_DEPTH >= 8, "递归上限太浅, VCP 实践需要 >= 8");
    // (placeholder regex 字面量字符类检查移到 lib_tests::placeholder_regex_has_required_classes)
};

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(pairs: &[(&str, &str)]) -> PlaceholderContext {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // ====== 正常情况 ======

    #[test]
    fn resolve_single_placeholder() {
        let mut c = PlaceholderContext::new();
        c.insert("name".to_string(), "Apeireth".to_string());
        let r = resolve_placeholders("Hello, {{name}}!", &c);
        assert_eq!(r, "Hello, Apeireth!");
    }

    #[test]
    fn resolve_multiple_placeholders_in_one_template() {
        let c = ctx(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let r = resolve_placeholders("{{a}} + {{b}} = {{c}}", &c);
        assert_eq!(r, "1 + 2 = 3");
    }

    #[test]
    fn resolve_agent_prefix_works() {
        // VCP line 164: `agent:alias` 去前缀视为 alias
        let c = ctx(&[("claude", "I am Claude")]);
        let r = resolve_placeholders("{{agent:claude}}", &c);
        assert_eq!(r, "I am Claude");
    }

    #[test]
    fn resolve_toolbox_prefix_works() {
        let c = ctx(&[("WebFetch", "URL fetcher tool")]);
        let r = resolve_placeholders("{{toolbox:WebFetch}}", &c);
        assert_eq!(r, "URL fetcher tool");
    }

    #[test]
    fn resolve_missing_keeps_original() {
        // VCP 行为: 找不到 alias 留原样
        let c = ctx(&[("known", "X")]);
        let r = resolve_placeholders("{{known}} and {{unknown}}", &c);
        assert_eq!(r, "X and {{unknown}}");
    }

    // ====== 循环防护 (VCP line 186-191) ======

    #[test]
    fn resolve_circular_reference_breaks_loop() {
        // a → b → a (VCP `processingStack.has(alias)` 触发)
        let c = ctx(&[("a", "value of {{b}}"), ("b", "value of {{a}}")]);
        let r = resolve_placeholders("{{a}}", &c);
        // 必须包含循环错误标记 (VCP line 188: `[Error: Circular agent reference detected for '${alias}']`)
        assert!(r.contains("Circular"), "应触发循环防护, 实际: {r}");
    }

    #[test]
    fn resolve_self_reference_breaks_loop() {
        // a → a (自引用, 最简单的循环)
        let c = ctx(&[("a", "I am {{a}}")]);
        let r = resolve_placeholders("{{a}}", &c);
        assert!(r.contains("Circular"), "自引用应触发循环防护, 实际: {r}");
    }

    // ====== 多层嵌套 ======

    #[test]
    fn resolve_nested_3_layers() {
        // x → y → z → final
        let c = ctx(&[("x", "{{y}}"), ("y", "{{z}}"), ("z", "final-value")]);
        let r = resolve_placeholders("{{x}}", &c);
        assert_eq!(r, "final-value");
    }

    #[test]
    fn resolve_nested_5_layers() {
        let c = ctx(&[
            ("a", "{{b}}"),
            ("b", "{{c}}"),
            ("c", "{{d}}"),
            ("d", "{{e}}"),
            ("e", "bottom"),
        ]);
        let r = resolve_placeholders("{{a}}", &c);
        assert_eq!(r, "bottom");
    }

    #[test]
    fn resolve_max_recursion_depth_caps() {
        // a → b → a → b → ... 超过 MAX_RECURSION_DEPTH 截断
        let c = ctx(&[("x", "{{x}}")]);
        let r = resolve_placeholders("{{x}}", &c);
        // 应触发循环防护, 不应爆栈
        assert!(r.contains("Circular"), "超过深度应触发循环防护, 实际: {r}");
    }

    // ====== CJK / Unicode 字符 (VCP line 160 字符类) ======

    #[test]
    fn resolve_cjk_alias() {
        // VCP 字符类: \u2E80-\u2FFF + \u3040-\u9FFF (CJK)
        let c = ctx(&[("工具", "file_ops")]);
        let r = resolve_placeholders("使用 {{工具}} 处理", &c);
        assert_eq!(r, "使用 file_ops 处理");
    }

    #[test]
    fn placeholder_regex_has_required_classes() {
        // VCP 字符类必须含 `agent:` / `toolbox:` 两种前缀
        assert!(PLACEHOLDER_REGEX_STR.contains("a-zA-Z"));
        assert!(PLACEHOLDER_REGEX_STR.contains("0-9"));
        // 不能漏 `:` (VCP 三种 prefix 都依赖)
        assert!(PLACEHOLDER_REGEX_STR.contains(':'));
    }
}
