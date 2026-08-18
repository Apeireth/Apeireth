//! **§5.1 记忆域深化包 机制② / VCP `SemanticGroupManager` 精神 — 记忆主题分组 + 主题索引注入**
//!
//! **目标**: 记忆条目注入前先按主题聚簇, 注入链增加"主题索引"块
//! (每主题: 名称 + 条目数 + 代表条目), 让 LLM 一眼看到记忆版图,
//! 与既有反幻觉记忆注入块 (`memory_injection`) 协作不超限.
//!
//! **VCP 对照**: VCP `SemanticGroupManager` 用语义嵌入聚簇; 我们走**确定性分组**
//! (0 嵌入 0 远程 0 随机): token 提取 (CJK 双字组 + 拉丁词) → 贪心聚簇
//! (共享 token 数最多且 ≥1 入簇, 否则新簇) → 主题名 = 簇内最高频 token.
//! 同输入必同输出 (有测试守).
//!
//! **预算协作**: `build_topic_index` 自带字符预算 (`TOPIC_INDEX_MAX_CHARS`),
//! 超预算砍尾部主题行并留"还收纳了 N 组"提示 (VCP foldProtocol 收纳提示精神),
//! 不侵占记忆证据块的既有 120 字/条预算.
//!
//! **挂接点**: `assemble.rs::memory_block` — 主题索引块 + 记忆证据块合并注入,
//! 不另立平行注入系统 (0 装 PASS: 未接真嵌入, 如实标注).

/// **主题索引块字符预算** (与记忆证据块独立, 不超限)
pub const TOPIC_INDEX_MAX_CHARS: usize = 600;

/// 主题收纳提示模板 (VCP foldProtocol "还隐藏收纳了 N 组" 精神)
const COLLAPSE_NOTICE: &str = "…还收纳了 {n} 组主题, 未逐一展开.";

/// **通用词停用表** (确定性聚簇时剔除, 防"主人/明天"类高频通用词把不相关条目并簇)
const STOPWORDS: &[&str] = &[
    "主人", "我们", "今天", "明天", "昨天", "时候", "事情", "需要", "一个", "可以", "现在", "主题",
    "词条",
];

/// **token 提取** — CJK 连续段取双字组 (bigram), 拉丁/数字连续段取整词 (≥2 字符, 小写)
///
/// **停用词是切分点**: CJK 串遇停用词就地切断 (不产生跨停用词的桥接 bigram,
/// 防"主人明天A / 主人明天B"经"人明"桥接误并簇)
///
/// 确定性: 同输入同输出, 无外部依赖
pub fn topic_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut latin = String::new();
    let mut cjk_run: Vec<char> = Vec::new();
    let flush_latin = |latin: &mut String, out: &mut Vec<String>| {
        if latin.chars().count() >= 2 {
            out.push(latin.to_lowercase());
        }
        latin.clear();
    };
    // CJK 段: 停用词切分 + 子段 bigram
    let flush_cjk = |run: &mut Vec<char>, out: &mut Vec<String>| {
        let mut sub: Vec<char> = Vec::new();
        let emit = |sub: &mut Vec<char>, out: &mut Vec<String>| {
            for w in sub.windows(2) {
                out.push(w.iter().collect());
            }
            sub.clear();
        };
        let mut i = 0;
        while i < run.len() {
            // 双字停用词命中 → 切断 (丢当前子段, 跳过 2 字)
            if i + 1 < run.len() {
                let bg: String = run[i..i + 2].iter().collect();
                if STOPWORDS.contains(&bg.as_str()) {
                    emit(&mut sub, out);
                    i += 2;
                    continue;
                }
            }
            sub.push(run[i]);
            i += 1;
        }
        emit(&mut sub, out);
        run.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if !cjk_run.is_empty() {
                flush_cjk(&mut cjk_run, &mut out);
            }
            latin.push(ch);
        } else if is_cjk(ch) {
            if !latin.is_empty() {
                flush_latin(&mut latin, &mut out);
            }
            cjk_run.push(ch);
        } else {
            if !latin.is_empty() {
                flush_latin(&mut latin, &mut out);
            }
            if !cjk_run.is_empty() {
                flush_cjk(&mut cjk_run, &mut out);
            }
        }
    }
    if !latin.is_empty() {
        flush_latin(&mut latin, &mut out);
    }
    if !cjk_run.is_empty() {
        flush_cjk(&mut cjk_run, &mut out);
    }
    out
}

/// CJK 判定 (统一表意 + 扩展A, 覆盖常用中文)
fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

/// **主题分组结果**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicGroup {
    /// 主题名 (簇内最高频 token; 并列取最早出现)
    pub topic: String,
    /// 簇内条目 (按输入顺序)
    pub items: Vec<String>,
}

/// **确定性主题分组** (贪心聚簇, 同输入同输出)
///
/// **算法**: 顺序遍历条目; 每条与已有簇比共享 token 数,
/// 取最高分簇 (并列取先出现的簇), 分数 ≥1 入簇, 否则新开簇.
/// 簇 token 集 = 成员 token 并集 (停用词已在提取时剔除).
pub fn group_topics(entries: &[String]) -> Vec<TopicGroup> {
    let mut groups: Vec<TopicGroup> = Vec::new();
    let mut group_tokens: Vec<Vec<String>> = Vec::new();
    for entry in entries {
        let tokens = topic_tokens(entry);
        // 找共享 token 最多的簇 (并列取先出现)
        let mut best: Option<(usize, usize)> = None; // (score, idx)
        for (idx, gtokens) in group_tokens.iter().enumerate() {
            let score = tokens.iter().filter(|t| gtokens.contains(t)).count();
            if score > 0 && best.map_or(true, |(bs, _)| score > bs) {
                best = Some((score, idx));
            }
        }
        match best {
            Some((_, idx)) => {
                groups[idx].items.push(entry.clone());
                for t in tokens {
                    if !group_tokens[idx].contains(&t) {
                        group_tokens[idx].push(t);
                    }
                }
            }
            None => {
                groups.push(TopicGroup {
                    topic: String::new(), // 下面统一算
                    items: vec![entry.clone()],
                });
                group_tokens.push(tokens);
            }
        }
    }
    // 主题名 = 簇内最高频 token (并列取最早出现), 截 12 字符
    for (group, gtokens) in groups.iter_mut().zip(group_tokens.iter()) {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for item in &group.items {
            for t in topic_tokens(item) {
                match counts.iter_mut().find(|(tok, _)| *tok == t) {
                    Some((_, c)) => *c += 1,
                    None => counts.push((t, 1)),
                }
            }
        }
        let name = counts
            .iter()
            .max_by_key(|(tok, c)| {
                (
                    *c,
                    std::cmp::Reverse(gtokens.iter().position(|g| g == tok).unwrap_or(usize::MAX)),
                )
            })
            .map(|(tok, _)| tok.chars().take(12).collect::<String>())
            .unwrap_or_else(|| "未分类".to_string());
        group.topic = if name.is_empty() {
            "未分类".to_string()
        } else {
            name
        };
    }
    groups
}

/// **主题索引块渲染** (预算感知)
///
/// 每主题一行: `- {主题}: {N} 条 (代表: {首条 ≤40 字})`
/// 超 `max_chars` 从尾部砍主题行, 留收纳提示; 极端小预算硬切.
/// 空条目 → 空串 (0 假装).
pub fn build_topic_index(entries: &[String], max_chars: usize) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let groups = group_topics(entries);
    let header = "[主题索引]";
    let mut lines: Vec<String> = groups
        .iter()
        .map(|g| {
            let rep: String = g
                .items
                .first()
                .map(|s| s.chars().take(40).collect())
                .unwrap_or_default();
            format!("- {}: {} 条 (代表: {})", g.topic, g.items.len(), rep)
        })
        .collect();
    let assemble = |lines: &[String]| -> String {
        let mut s = String::from(header);
        s.push('\n');
        s.push_str(&lines.join("\n"));
        s
    };
    let mut text = assemble(&lines);
    let mut hidden = 0usize;
    // 超预算: 从尾部砍主题行 (留头部 = 先出现的主题), 留收纳提示
    while text.chars().count() > max_chars && lines.len() > 1 {
        lines.pop();
        hidden += 1;
        let notice = COLLAPSE_NOTICE.replace("{n}", &hidden.to_string());
        text = format!("{}\n{}", assemble(&lines), notice);
    }
    // 极端小预算: 硬切保上限
    if text.chars().count() > max_chars {
        text = text.chars().take(max_chars).collect();
    }
    text
}

// ============================================================
// 测试 (分组正常 / 空记忆 / 单主题 / 超预算截断 / 确定性)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_algebra_corpus() -> Vec<String> {
        vec![
            "主人明天要交线代作业, 重点换元法".to_string(),
            "线代作业第二题要用换元积分".to_string(),
            "主人偏好深烘咖啡豆, 不加糖".to_string(),
        ]
    }

    #[test]
    fn empty_entries_empty_index() {
        assert_eq!(build_topic_index(&[], TOPIC_INDEX_MAX_CHARS), "");
        assert!(group_topics(&[]).is_empty());
    }

    #[test]
    fn groups_by_shared_keywords() {
        let groups = group_topics(&linear_algebra_corpus());
        assert_eq!(groups.len(), 2, "线代两条应并簇, 咖啡单独一簇");
        assert_eq!(groups[0].items.len(), 2, "先出现的簇收两条线代");
        assert_eq!(groups[1].items.len(), 1);
        // 主题名应是簇内高频 token (线代相关)
        assert!(!groups[0].topic.is_empty());
        assert_ne!(groups[0].topic, groups[1].topic);
    }

    #[test]
    fn single_topic_single_group() {
        let entries: Vec<String> = vec![
            "git 仓库明天要合并 pull request".into(),
            "git 合并前跑 cargo test 防回归".into(),
            "git 分支命名按任务号走".into(),
        ];
        let groups = group_topics(&entries);
        assert_eq!(groups.len(), 1, "共享 git 应单簇");
        assert_eq!(groups[0].items.len(), 3);
        assert_eq!(groups[0].topic, "git");
    }

    #[test]
    fn stopwords_do_not_merge_unrelated() {
        // 仅共享"主人/明天"类通用词 → 不应并簇 (停用词已剔除)
        let entries: Vec<String> = vec!["主人明天体检要空腹".into(), "主人明天想装新显卡".into()];
        let groups = group_topics(&entries);
        assert_eq!(groups.len(), 2, "通用词不作为聚簇依据");
    }

    #[test]
    fn index_renders_budget_and_notice() {
        // 30 个互不相关主题 (语料 0 共享 bigram), 小预算 → 砍尾行 + 收纳提示 + ≤ 预算
        const CORPUS: &[&str] = &[
            "量子退相干实验",
            "烘焙发酵温度",
            "登山路线规划",
            "钢琴指法练习",
            "河流水质监测",
            "星轨摄影参数",
            "蜜蜂蜂箱检查",
            "陶土窑变釉色",
            "滑雪板打蜡",
            "青铜器除锈",
            "候鸟迁徙环志",
            "咖啡萃取压力",
            "攀岩保护站设置",
            "宣纸帘纹工艺",
            "雷达回波解读",
            "羊毛毡戳刺",
            "潮汐表推算",
            "酱油曲霉培养",
            "滑翔伞气流判断",
            "漆器荫干湿度",
            "地震波走时",
            "竹编经纬起底",
            "云底高度估测",
            "黑胶唱针调校",
            "堆肥碳氮配比",
            "极光指数预报",
            "榫卯燕尾角度",
            "酒花投放时序",
            "冰川裂隙探路",
            "苔藓孢蒴观察",
        ];
        let entries: Vec<String> = CORPUS.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            group_topics(&entries).len(),
            30,
            "语料应互不共享 token → 30 独立簇"
        );
        let idx = build_topic_index(&entries, 300);
        assert!(idx.chars().count() <= 300, "必须 ≤ 预算");
        assert!(idx.starts_with("[主题索引]"));
        assert!(idx.contains("还收纳了"), "超预算必留收纳提示: {idx}");
    }

    #[test]
    fn within_budget_no_notice() {
        let idx = build_topic_index(&linear_algebra_corpus(), TOPIC_INDEX_MAX_CHARS);
        assert!(idx.starts_with("[主题索引]"));
        assert!(idx.contains("2 条 (代表:"), "并簇条目数应渲染");
        assert!(!idx.contains("还收纳了"), "预算内不留收纳提示");
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let entries = linear_algebra_corpus();
        let a = build_topic_index(&entries, TOPIC_INDEX_MAX_CHARS);
        let b = build_topic_index(&entries, TOPIC_INDEX_MAX_CHARS);
        assert_eq!(a, b, "确定性: 同输入同输出");
        let ga = group_topics(&entries);
        let gb = group_topics(&entries);
        assert_eq!(ga, gb);
    }

    #[test]
    fn topic_tokens_cjk_bigram_and_latin() {
        let toks = topic_tokens("线代作业 hello W2");
        assert!(toks.contains(&"线代".to_string()));
        assert!(toks.contains(&"作业".to_string()));
        assert!(toks.contains(&"hello".to_string()));
        assert!(toks.contains(&"w2".to_string()), "拉丁词小写化");
        assert!(!toks.contains(&"主人".to_string()), "停用词剔除");
    }
}
