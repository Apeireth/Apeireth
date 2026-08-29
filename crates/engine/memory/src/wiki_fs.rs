//! Karpathy LLM-Wiki 知识编译与反熵治理引擎 (WikiFsEngine).
//!
//! “知识编译胜于检索 (Compilation over Retrieval)”：
//! 智能体持续读取碎片会话与原始资料 (Raw Sources)，增量“编译”并维护结构化、相互内联的 Markdown 维基库 (`viking://` 或本地 wiki 目录)，
//! 并通过异步反熵 Lint 机制（死链检测、孤岛页面检测与冲突概念消解）保持知识体系高度有序.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// 单个 Wiki 词条页面.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiPage {
    pub slug: String,
    pub title: String,
    pub content: String,
    /// 页面中包含的双链引用集合 (`[[TargetSlug]]`)
    pub outgoing_links: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// 反熵治理 Lint 告警类型.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WikiLintIssue {
    /// 死链: 引用的目标页面不存在
    BrokenLink { from_slug: String, target_slug: String },
    /// 孤岛页面: 无任何其他页面引用该词条
    OrphanPage { slug: String },
    /// 潜在概念重复: 两个页面的标题或关键词高度重合
    PotentialDuplicate { slug_a: String, slug_b: String, similarity: u8 },
}

/// Wiki 反熵健康度报告.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiHealthReport {
    pub total_pages: usize,
    pub total_links: usize,
    pub issues: Vec<WikiLintIssue>,
    pub health_score: u8,
}

/// LLM-Wiki 知识库引擎.
#[derive(Debug, Clone, Default)]
pub struct WikiFsEngine {
    pages: HashMap<String, WikiPage>,
}

impl WikiFsEngine {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
    }

    /// 从 Markdown 文本中提取所有 `[[Target]]` 形式的双向维基链接.
    pub fn extract_wikilinks(markdown: &str) -> Vec<String> {
        let mut links = Vec::new();
        let mut rest = markdown;

        while let Some(start) = rest.find("[[") {
            let body = &rest[start + 2..];
            if let Some(end) = body.find("]]") {
                let target = body[..end].trim().to_string();
                if !target.is_empty() && !links.contains(&target) {
                    links.push(target);
                }
                rest = &body[end + 2..];
            } else {
                break;
            }
        }

        links
    }

    /// 增量“编译”写入或更新一个 Wiki 页面.
    pub fn compile_page(&mut self, slug: &str, title: &str, markdown_content: &str, now_ms: u64) -> WikiPage {
        let links = Self::extract_wikilinks(markdown_content);
        let slug_clean = slug.trim().to_lowercase();

        let page = if let Some(existing) = self.pages.get_mut(&slug_clean) {
            existing.title = title.to_string();
            existing.content = markdown_content.to_string();
            existing.outgoing_links = links;
            existing.updated_at_ms = now_ms;
            existing.clone()
        } else {
            let new_page = WikiPage {
                slug: slug_clean.clone(),
                title: title.to_string(),
                content: markdown_content.to_string(),
                outgoing_links: links,
                created_at_ms: now_ms,
                updated_at_ms: now_ms,
            };
            self.pages.insert(slug_clean, new_page.clone());
            new_page
        };

        page
    }

    /// 获取指定 Wiki 页面.
    pub fn get_page(&self, slug: &str) -> Option<&WikiPage> {
        self.pages.get(&slug.trim().to_lowercase())
    }

    /// 执行反熵 Lint 治理检查.
    pub fn run_lint(&self) -> WikiHealthReport {
        let mut issues = Vec::new();
        let mut total_links = 0;
        let mut incoming_ref_counts: HashMap<String, usize> = HashMap::new();

        for slug in self.pages.keys() {
            incoming_ref_counts.insert(slug.clone(), 0);
        }

        // 1. 扫描死链并统计入度
        for page in self.pages.values() {
            for target in &page.outgoing_links {
                total_links += 1;
                let target_clean = target.trim().to_lowercase();
                if self.pages.contains_key(&target_clean) {
                    *incoming_ref_counts.entry(target_clean).or_insert(0) += 1;
                } else {
                    issues.push(WikiLintIssue::BrokenLink {
                        from_slug: page.slug.clone(),
                        target_slug: target.clone(),
                    });
                }
            }
        }

        // 2. 扫描孤岛页面 (入度为 0 且非 index/home 主页)
        for (slug, in_degree) in &incoming_ref_counts {
            if *in_degree == 0 && slug != "index" && slug != "home" && self.pages.len() > 1 {
                issues.push(WikiLintIssue::OrphanPage {
                    slug: slug.clone(),
                });
            }
        }

        let penalty = (issues.len() * 10).min(100);
        let health_score = (100 - penalty) as u8;

        WikiHealthReport {
            total_pages: self.pages.len(),
            total_links,
            issues,
            health_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wikilinks() {
        let md = "这里涉及到了 [[Rust语言]] 与 [[内存安全]]，详情见 [[Rust语言]].";
        let links = WikiFsEngine::extract_wikilinks(md);
        assert_eq!(links, vec!["Rust语言", "内存安全"]);
    }

    #[test]
    fn test_wiki_compilation_and_anti_entropy_lint() {
        let mut wiki = WikiFsEngine::new();

        // 1. 编译写入 index 页面，引用 rust_lang 与 non_existent
        wiki.compile_page("index", "知识索引", "探索 [[rust_lang]] 与 [[missing_concept]].", 1000);

        // 2. 编译写入 rust_lang 页面
        wiki.compile_page("rust_lang", "Rust 语言核心", "Rust 是一门注重安全与性能的语言.", 1000);

        // 3. 编译写入一个无任何引用的孤岛页面 orphan_page
        wiki.compile_page("orphan_page", "孤立词条", "无人引用的内容.", 1000);

        let report = wiki.run_lint();
        assert_eq!(report.total_pages, 3);
        assert_eq!(report.total_links, 2);

        // 验证捕获到了死链 missing_concept
        assert!(report.issues.iter().any(|i| matches!(i, WikiLintIssue::BrokenLink { target_slug, .. } if target_slug == "missing_concept")));

        // 验证捕获到了孤岛页面 orphan_page
        assert!(report.issues.iter().any(|i| matches!(i, WikiLintIssue::OrphanPage { slug } if slug == "orphan_page")));

        assert!(report.health_score < 100);
    }
}
