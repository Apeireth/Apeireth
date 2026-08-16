//! `apeireth-companion::context` — 统一注入管线 (ContextAssembler).
//!
//! 背景 (2026-08-16 主人原则: 机制而非补丁, 集成而非分立):
//! 注入链曾是散装 push (状态/记忆/图谱/偏好/今日/成长 各自截断, 无统一预算) —
//! 补丁态。本模块机制化:
//! - 有序块管线: 各块按顺序注册, 统一总预算 (total_budget_chars)
//! - 预算保护: persona/状态块 (核心块) 永不截断; 其余块按注册顺序截断
//! - 截断语义: 超预算块先截长块 (从尾部), 保证核心信息存活
//! - 输出: 一条合并 system 文本 (或逐块列表, 供上层自由组装)
//!
//! 契合点: 与 prompt_cache 的 assemble_tiered (分层组装) 互补 — 本管线管
//! 注入块预算, prompt_cache 管协议层组装.

/// 注入块 (命名 + 内容).
#[derive(Debug, Clone)]
pub struct ContextBlock {
    /// 块名 (调试/预算报告).
    pub name: &'static str,
    /// 块内容 (可能含换行).
    pub content: String,
    /// 是否核心块 (永不截断).
    pub core: bool,
    /// 单块上限 (None = 不限, 受总预算约束).
    pub cap_chars: Option<usize>,
}

impl ContextBlock {
    pub fn new(name: &'static str, content: impl Into<String>) -> Self {
        Self { name, content: content.into(), core: false, cap_chars: None }
    }
    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }
    pub fn with_cap(mut self, cap: usize) -> Self {
        self.cap_chars = Some(cap);
        self
    }
}

/// 统一注入管线: 有序块 + 总预算 + 核心保护 + 截断.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
}

impl ContextAssembler {
    /// 总预算字符数 (默认 6000; 超预算块从尾部截断, 核心块保护).
    pub fn new(total_budget_chars: usize) -> Self {
        Self { blocks: Vec::new(), total_budget_chars: total_budget_chars.max(100) }
    }

    /// 注册块 (保持顺序; 核心块在前更安全).
    pub fn push(mut self, block: ContextBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// 预算报告 (诊断): 各块字符数 + 总占用.
    pub fn budget_report(&self) -> Vec<(String, usize)> {
        self.blocks.iter().map(|b| (b.name.to_string(), b.content.chars().count())).collect()
    }

    /// 预算化组装 (不可变): 返回按预算截断后的块列表.
    pub fn assemble_budgeted(&self) -> Vec<String> {
        self.assemble_budgeted_blocks()
            .into_iter()
            .map(|b| b.content)
            .collect()
    }

    /// 预算化组装 (不可变, 保留块名): 上层可依 name 分流 (如 identity 独立成消息).
    pub fn assemble_budgeted_blocks(&self) -> Vec<ContextBlock> {
        // 1. 单块上限
        let mut capped: Vec<String> = self
            .blocks
            .iter()
            .map(|b| {
                let s: String = b.content.chars().take(b.cap_chars.unwrap_or(usize::MAX)).collect();
                s
            })
            .collect();
        // 2. 总预算超标 → 非核心块按"字符多者优先"截断 (贪心砍大头)
        let mut total: usize = capped.iter().map(|s| s.chars().count()).sum();
        if total > self.total_budget_chars {
            let mut order: Vec<usize> = (0..self.blocks.len()).filter(|&i| !self.blocks[i].core).collect();
            order.sort_by_key(|&i| std::cmp::Reverse(capped[i].chars().count()));
            for i in order {
                if total <= self.total_budget_chars {
                    break;
                }
                let len = capped[i].chars().count();
                if len == 0 {
                    continue;
                }
                let over = total - self.total_budget_chars;
                let cut = len.min(over);
                capped[i] = capped[i].chars().take(len - cut).collect();
                total -= cut;
            }
        }
        self.blocks
            .iter()
            .zip(capped)
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(b, s)| ContextBlock {
                name: b.name,
                content: s,
                core: b.core,
                cap_chars: b.cap_chars,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_blocks_never_truncated() {
        // 总 430 字, 预算 300: 核心 150 保留, 非核心按大头先砍 (mem 200 → 留 70)
        let a = ContextAssembler::new(300)
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", "记忆内容".repeat(50)))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));
        let out = a.assemble_budgeted();
        let total: usize = out.iter().map(|s| s.chars().count()).sum();
        assert!(total <= 300, "总预算应约束 (核心保护下 total=300)");
        assert!(out[0].contains("核心人格内容"), "核心块应完整保留");
        // 记忆块 (非核心, 最大) 应被截断: 460-300=160 → mem 200 砍 160 → 40
        assert!(out[1].chars().count() < "记忆内容".repeat(50).chars().count(), "非核心块应被截断");
        assert_eq!(out[1].chars().count(), 40, "mem 200 → 砍 160 → 留 40");
    }

    #[test]
    fn per_block_cap() {
        let a = ContextAssembler::new(100_000)
            .push(ContextBlock::new("x", "abc".repeat(10)).with_cap(12));
        let out = a.assemble_budgeted();
        assert_eq!(out[0], "abcabcabcabc".to_string());
    }

    #[test]
    fn empty_blocks_filtered() {
        let a = ContextAssembler::new(1000)
            .push(ContextBlock::new("a", "hello"))
            .push(ContextBlock::new("b", "   "));
        let out = a.assemble_budgeted();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "hello");
    }
}
