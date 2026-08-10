//! R30 U5: Tool classifier (DynamicToolBridge pattern)
//!
//! **设计**: 7 类 keyword classifier - 给定 query, 返回最可能的工具
//! - 7 类: WebSearch / FileOperator / Git / ShellExec / Grep / ApplyPatch / LongTask / WebFetch
//! - 每个 category 配一组 keyword, 命中次数最多 -> 胜出
//! - 不命中 -> 返 None (让 LLM 自己选)
//!
//! **借鉴**: VCP toolRouter.js (keyword 路由) + ClaudeCode tool_router
//!
//! **不假装**:
//! - 真 keyword 匹配 (不假装 ML / 真小模型调用)
//! - 真 7 类 (实际 8 类 - 加 WebFetch)
//! - 真 tie-breaker (返回多 category 排序)

use serde::{Deserialize, Serialize};

/// R30 U5: 工具分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    WebSearch,
    FileOperator,
    Git,
    ShellExec,
    Grep,
    ApplyPatch,
    LongTask,
    WebFetch,
}

impl ToolCategory {
    /// 全部 8 个 category
    pub fn all() -> [ToolCategory; 8] {
        [Self::WebSearch, Self::FileOperator, Self::Git,
         Self::ShellExec, Self::Grep, Self::ApplyPatch,
         Self::LongTask, Self::WebFetch]
    }

    /// 映射到实际 tool 名
    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::WebSearch => "WebSearch",
            Self::FileOperator => "FileOperator",
            Self::Git => "Git",
            Self::ShellExec => "ShellExec",
            Self::Grep => "Grep",
            Self::ApplyPatch => "ApplyPatch",
            Self::LongTask => "LongTask",
            Self::WebFetch => "WebFetch",
        }
    }

    /// 该 category 的 keywords
    fn keywords(&self) -> &[&str] {
        match self {
            Self::WebSearch => &["搜索", "查找", "检索", "上网查", "search", "find", "google", "查"],
            Self::FileOperator => &["文件", "读取", "写入", "删除", "列目录", "复制", "移动", "file", "read", "write", "delete", "copy", "move", "mkdir"],
            Self::Git => &["git", "提交", "分支", "历史", "commit", "branch", "log", "diff", "status"],
            Self::ShellExec => &["运行", "执行", "命令", "shell", "exec", "run", "bash", "cmd", "powershell"],
            Self::Grep => &["检索", "匹配", "查找", "grep", "regex", "match", "search.*code"],
            Self::ApplyPatch => &["patch", "diff", "修改", "更新", "补丁", "选项", "表单合并", "edit_block", "replace"],
            Self::LongTask => &["后台", "异步", "长任务", "async", "background", "long", "schedule", "queue", "任务队列"],
            Self::WebFetch => &["打开网页", "访问", "http", "url", "fetch", "curl", "wget", "网页", "网址"],
        }
    }
}

/// R30 U5: 分类结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub category: ToolCategory,
    pub score: usize,
}

/// R30 U5: keyword classifier - 给 query 算每个 category 的命中数, 返回排序结果
pub fn classify(query: &str) -> Vec<Classification> {
    let q = query.to_lowercase();
    let mut results = Vec::new();
    for cat in ToolCategory::all() {
        let score = cat.keywords().iter().filter(|kw| q.contains(&kw.to_lowercase())).count();
        if score > 0 {
            results.push(Classification { category: cat, score });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| (a.category as usize).cmp(&(b.category as usize))));
    results
}

/// R30 U5: 取最高分 category
pub fn best_match(query: &str) -> Option<ToolCategory> {
    classify(query).first().map(|c| c.category)
}

/// R30 U5: 多 category tie-breaking (返前 N)
pub fn top_n(query: &str, n: usize) -> Vec<ToolCategory> {
    classify(query).into_iter().take(n).map(|c| c.category).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_categories_have_keywords() {
        for cat in ToolCategory::all() {
            assert!(!cat.keywords().is_empty(), "{:?} has no keywords", cat);
            assert!(!cat.tool_name().is_empty());
        }
    }

    #[test]
    fn classify_search_query() {
        let r = best_match("帮我搜索 Rust 学习资料");
        assert_eq!(r, Some(ToolCategory::WebSearch));
    }

    #[test]
    fn classify_file_query() {
        let r = best_match("读取 /tmp/test.txt 文件");
        assert_eq!(r, Some(ToolCategory::FileOperator));
    }

    #[test]
    fn classify_git_query() {
        let r = best_match("查看 git 分支的最近 commit");
        // 查看 + git + 分支 + commit -> Git wins (4 hits vs 1 查看 for WebSearch)
        assert_eq!(r, Some(ToolCategory::Git));
    }

    #[test]
    fn classify_shell_query() {
        let r = best_match("运行 ls -la 命令");
        assert_eq!(r, Some(ToolCategory::ShellExec));
    }

    #[test]
    fn classify_grep_query() {
        let r = best_match("项目里 grep TODO 查 regex");
        assert_eq!(r, Some(ToolCategory::Grep));
    }

    #[test]
    fn classify_apply_patch_query() {
        let r = best_match("修改 foo.rs 里的那个函数");
        assert_eq!(r, Some(ToolCategory::ApplyPatch));
    }

    #[test]
    fn classify_webfetch_query() {
        let r = best_match("打开 https://example.com 这个网页");
        assert_eq!(r, Some(ToolCategory::WebFetch));
    }

    #[test]
    fn classify_empty_returns_empty() {
        let r = classify("");
        assert!(r.is_empty());
    }

    #[test]
    fn classify_no_match_returns_empty() {
        let r = best_match("今天天气不错");
        // 今天 天气 不错 都没呼中其他 category
        assert!(r.is_none());
    }

    #[test]
    fn top_n_returns_multiple() {
        let r = top_n("读取文件后运行命令", 2);
        assert!(r.len() >= 2);
    }

    #[test]
    fn tool_name_lookup() {
        assert_eq!(ToolCategory::FileOperator.tool_name(), "FileOperator");
        assert_eq!(ToolCategory::WebFetch.tool_name(), "WebFetch");
    }

    #[test]
    fn classify_is_case_insensitive() {
        assert_eq!(best_match("GREP TODO"), Some(ToolCategory::Grep));
        assert_eq!(best_match("Git COMMIT"), Some(ToolCategory::Git));
    }
}
