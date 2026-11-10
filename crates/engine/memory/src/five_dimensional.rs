//! 五维时空记忆系统 (Five-Dimensional Memory Topology).
//!
//! 吸收 N.E.K.O 与 MemGPT 架构精髓，将智能体的记忆结构解耦为 5 个不同时空尺度的认知层级，
//! 并提供 Memory Browser 可视化校对数据结构，杜绝大模型记忆幻觉.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// 记忆层级枚举.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryDimension {
    /// 工作记忆 (Working Memory): 极速环形内存，最新 K 轮会话
    Working,
    /// 近期记忆 (Recent Memory): 24 小时滑动情境窗口
    Recent,
    /// 事实记忆 (Fact Memory): 实体-属性-值用户画像与知识事实
    Fact,
    /// 反思记忆 (Reflection Memory): 认知自省、矛盾消解与情感感悟
    Reflection,
    /// 人设本体 (Persona Memory): 不可动摇的底层价值观与世界观
    Persona,
}

/// 「近期记忆」的滑动窗口长度：24 小时（与 [`MemoryDimension::Recent`] 的文档一致）。
pub const RECENT_WINDOW_MS: u64 = 24 * 60 * 60 * 1_000;

/// 一条带时间戳的近期记忆。
///
/// # 为什么必须带时间戳
///
/// `Recent` 维度的语义是「**24 小时**滑动情境窗口」——没有时间戳就无法判断
/// 「还在窗口内吗」，只能无限堆积或永不产出。此前该维度的后备字段是
/// `Vec<String>`（无时间戳、且**从未被写入**）⇒ 维度永远为空。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentMemory {
    /// 记忆正文。
    pub content: String,
    /// 写入时刻（Unix epoch 毫秒，**调用方注入**，本模块不读时钟）。
    pub at_ms: u64,
}

/// 单条事实记忆条目.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactItem {
    pub entity: String,
    pub property: String,
    pub value: String,
    pub confidence: f32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// 单条反思条目.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionItem {
    pub id: String,
    pub summary: String,
    pub emotional_insight: String,
    pub source_episodes: Vec<String>,
    pub created_at_ms: u64,
}

/// 供前端 Memory Browser 校对展示的统一视图.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryBrowserEntry {
    pub id: String,
    pub dimension: MemoryDimension,
    pub title: String,
    pub content: String,
    pub timestamp_ms: u64,
    pub is_editable: bool,
}

/// 五维时空记忆管理器.
#[derive(Debug, Clone)]
pub struct FiveDimensionalMemory {
    /// 环形工作记忆 (最大容量 K)
    working_capacity: usize,
    working_buffer: VecDeque<String>,
    /// 近期记忆（24h 滑动窗口，带时间戳）
    recent_memories: Vec<RecentMemory>,
    /// 结构化事实记忆库: entity -> (property -> FactItem)
    facts: HashMap<String, HashMap<String, FactItem>>,
    /// 反思记忆列表
    reflections: Vec<ReflectionItem>,
    /// 核心人设本体
    persona_core: String,
}

impl FiveDimensionalMemory {
    pub fn new(persona_core: &str, working_capacity: usize) -> Self {
        Self {
            working_capacity: if working_capacity == 0 {
                10
            } else {
                working_capacity
            },
            working_buffer: VecDeque::with_capacity(working_capacity),
            recent_memories: Vec::new(),
            facts: HashMap::new(),
            reflections: Vec::new(),
            persona_core: persona_core.to_string(),
        }
    }

    /// 向工作记忆推送最新会话 Raw 消息.
    pub fn push_working(&mut self, message: &str) {
        if self.working_buffer.len() >= self.working_capacity {
            self.working_buffer.pop_front();
        }
        self.working_buffer.push_back(message.to_string());
    }

    /// 获取工作记忆全文.
    pub fn get_working(&self) -> Vec<String> {
        self.working_buffer.iter().cloned().collect()
    }

    /// 写入一条**近期记忆**（24h 滑动窗口），并顺手剪掉出窗的旧条目。
    ///
    /// 时间由调用方注入（本模块不读时钟，与全仓「显式时间戳」口径一致）。
    /// 出窗判定是 `now_ms - at_ms > RECENT_WINDOW_MS`：
    /// **恰好 24h 仍在窗内**（边界含等号，有测试钉死）。
    pub fn push_recent(&mut self, content: &str, now_ms: u64) {
        self.recent_memories.push(RecentMemory {
            content: content.to_string(),
            at_ms: now_ms,
        });
        self.prune_recent(now_ms);
    }

    /// 剪掉已出窗的近期记忆（`now_ms - at_ms > RECENT_WINDOW_MS`）。
    ///
    /// 用饱和减法：`at_ms > now_ms`（时钟回拨 / 未来条目）不会被误剪。
    pub fn prune_recent(&mut self, now_ms: u64) {
        self.recent_memories
            .retain(|item| now_ms.saturating_sub(item.at_ms) <= RECENT_WINDOW_MS);
    }

    /// 取当前仍在 24h 窗口内的近期记忆（按写入顺序，正文）。
    ///
    /// **不改状态**：视图层查询不该顺手改数据；出窗条目由 `push_recent` /
    /// `prune_recent` 清理。
    pub fn get_recent(&self, now_ms: u64) -> Vec<String> {
        self.recent_memories
            .iter()
            .filter(|item| now_ms.saturating_sub(item.at_ms) <= RECENT_WINDOW_MS)
            .map(|item| item.content.clone())
            .collect()
    }

    /// 写入/更新事实记忆 (带置信度与自动时间戳更新).
    pub fn upsert_fact(
        &mut self,
        entity: &str,
        property: &str,
        value: &str,
        confidence: f32,
        now_ms: u64,
    ) {
        let entity_map = self.facts.entry(entity.to_string()).or_default();
        if let Some(existing) = entity_map.get_mut(property) {
            existing.value = value.to_string();
            existing.confidence = confidence;
            existing.updated_at_ms = now_ms;
        } else {
            entity_map.insert(
                property.to_string(),
                FactItem {
                    entity: entity.to_string(),
                    property: property.to_string(),
                    value: value.to_string(),
                    confidence,
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                },
            );
        }
    }

    /// 查询某实体的所有事实属性.
    pub fn get_entity_facts(&self, entity: &str) -> Option<&HashMap<String, FactItem>> {
        self.facts.get(entity)
    }

    /// 记录一条高阶认知反思.
    pub fn add_reflection(
        &mut self,
        summary: &str,
        insight: &str,
        sources: Vec<String>,
        now_ms: u64,
    ) {
        self.reflections.push(ReflectionItem {
            id: format!("refl_{}", self.reflections.len() + 1),
            summary: summary.to_string(),
            emotional_insight: insight.to_string(),
            source_episodes: sources,
            created_at_ms: now_ms,
        });
    }

    /// 获取全部反思列表.
    pub fn get_reflections(&self) -> &[ReflectionItem] {
        &self.reflections
    }

    /// 获取核心人设本体.
    pub fn get_persona(&self) -> &str {
        &self.persona_core
    }

    /// 导出供 Memory Browser 校对的全部条目.
    pub fn export_browser_entries(&self) -> Vec<MemoryBrowserEntry> {
        let mut entries = Vec::new();

        // 1. Persona
        entries.push(MemoryBrowserEntry {
            id: "dim_persona".to_string(),
            dimension: MemoryDimension::Persona,
            title: "核心价值观与人设本体".to_string(),
            content: self.persona_core.clone(),
            timestamp_ms: 0,
            is_editable: false,
        });

        // 2. Working
        for (i, msg) in self.working_buffer.iter().enumerate() {
            entries.push(MemoryBrowserEntry {
                id: format!("working_{}", i),
                dimension: MemoryDimension::Working,
                title: format!("工作记忆 #{}", i + 1),
                content: msg.clone(),
                timestamp_ms: 0,
                is_editable: false,
            });
        }

        // 3. Recent（24h 滑动窗口）
        //
        // 此前这个维度**从不产出**：`recent_memories` 字段声明了却从未被写入，
        // 于是「5 维时空记忆」对 Memory Browser 而言实际只有 4 维。
        for (i, item) in self.recent_memories.iter().enumerate() {
            entries.push(MemoryBrowserEntry {
                id: format!("recent_{}_{}", item.at_ms, i),
                dimension: MemoryDimension::Recent,
                title: "近期记忆".to_string(),
                content: item.content.clone(),
                timestamp_ms: item.at_ms,
                is_editable: false,
            });
        }

        // 4. Facts
        for (entity, props) in &self.facts {
            for (prop, item) in props {
                entries.push(MemoryBrowserEntry {
                    id: format!("fact_{}_{}", entity, prop),
                    dimension: MemoryDimension::Fact,
                    title: format!("{}.{}", entity, prop),
                    content: format!("{} (置信度: {:.2})", item.value, item.confidence),
                    timestamp_ms: item.updated_at_ms,
                    is_editable: true,
                });
            }
        }

        // 5. Reflections
        for refl in &self.reflections {
            entries.push(MemoryBrowserEntry {
                id: refl.id.clone(),
                dimension: MemoryDimension::Reflection,
                title: refl.summary.clone(),
                content: refl.emotional_insight.clone(),
                timestamp_ms: refl.created_at_ms,
                is_editable: true,
            });
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_five_dimensional_memory_flow() {
        let mut mem = FiveDimensionalMemory::new("你是贴心忠诚的伴侣智能体", 3);

        // 1. Working memory ring buffer
        mem.push_working("m1");
        mem.push_working("m2");
        mem.push_working("m3");
        mem.push_working("m4");
        assert_eq!(mem.get_working(), vec!["m2", "m3", "m4"]);

        // 2. Facts
        mem.upsert_fact("user", "favorite_drink", "乌龙茶", 0.95, 1000);
        let fact = &mem.get_entity_facts("user").unwrap()["favorite_drink"];
        assert_eq!(fact.value, "乌龙茶");
        assert_eq!(fact.confidence, 0.95);

        // 3. Reflection
        mem.add_reflection(
            "主人经常熬夜",
            "需要适时主动提醒休息",
            vec!["ep_1".to_string()],
            2000,
        );
        assert_eq!(mem.get_reflections().len(), 1);

        // 4. Browser entries export
        let entries = mem.export_browser_entries();
        assert!(entries
            .iter()
            .any(|e| e.dimension == MemoryDimension::Persona));
        assert!(entries
            .iter()
            .any(|e| e.dimension == MemoryDimension::Fact && e.title == "user.favorite_drink"));
        assert!(entries
            .iter()
            .any(|e| e.dimension == MemoryDimension::Reflection));
    }

    /// **五维必须真的都产得出来**：每个 `MemoryDimension` 变体都要有对应条目。
    ///
    /// 这条是「维度永不空转」的守卫 —— 接线前 `Recent` 从不产出
    /// （字段声明了却从未写入），于是矩阵宣称的「5 维」实际只有 4 维。
    /// 新增维度却忘了覆盖导出时，这条会红。
    #[test]
    fn every_declared_dimension_is_exportable() {
        let mut mem = FiveDimensionalMemory::new("人设", 2);
        mem.push_working("工作");
        mem.push_recent("近期", 1_000);
        mem.upsert_fact("user", "like", "茶", 0.9, 1_000);
        mem.add_reflection("现象", "感悟", vec!["ep".to_string()], 1_000);

        let entries = mem.export_browser_entries();
        for dimension in [
            MemoryDimension::Working,
            MemoryDimension::Recent,
            MemoryDimension::Fact,
            MemoryDimension::Reflection,
            MemoryDimension::Persona,
        ] {
            assert!(
                entries.iter().any(|entry| entry.dimension == dimension),
                "维度 {dimension:?} 从未产出（宣称 N 维就必须真 N 维）"
            );
        }
    }

    /// 近期记忆是**24h 滑动窗口**：出窗即不再产出。
    #[test]
    fn recent_memory_rolls_with_a_24h_window() {
        let mut mem = FiveDimensionalMemory::new("人设", 2);
        let base = 1_700_000_000_000u64;
        mem.push_recent("两小时前", base);
        mem.push_recent("很旧的一条", base);

        // 过 2 小时：两条都还在窗内
        assert_eq!(mem.get_recent(base + 2 * 3_600_000).len(), 2);

        // 过 25 小时：都出窗
        assert!(
            mem.get_recent(base + 25 * 3_600_000).is_empty(),
            "超过 24h 不该再产出"
        );

        // 从游标上届时刻 push ⇒ 会顺手把出窗条目剪掉
        mem.push_recent("新的", base + 25 * 3_600_000);
        let entries = mem.export_browser_entries();
        let recent: Vec<_> = entries
            .iter()
            .filter(|entry| entry.dimension == MemoryDimension::Recent)
            .collect();
        assert_eq!(recent.len(), 1, "出窗条目应已被剪掉: {recent:?}");
        assert_eq!(recent[0].content, "新的");
        assert_eq!(recent[0].timestamp_ms, base + 25 * 3_600_000);
    }

    /// 边界：**恰好 24h 仍在窗内**（含等号），多 1ms 即出窗。
    #[test]
    fn recent_window_boundary_is_inclusive_at_exactly_24h() {
        let mut mem = FiveDimensionalMemory::new("人设", 2);
        let base = 1_000_000u64;
        mem.push_recent("边界条目", base);

        assert_eq!(
            mem.get_recent(base + RECENT_WINDOW_MS),
            vec!["边界条目".to_string()],
            "恰好 24h 必须仍在窗内"
        );
        assert!(
            mem.get_recent(base + RECENT_WINDOW_MS + 1).is_empty(),
            "超过 24h 1ms 即出窗"
        );
    }

    /// 时钟回拨（`now_ms` 早于条目时间）**不得误剪**已有近期记忆。
    #[test]
    fn clock_rollback_does_not_evict_recent_memory() {
        let mut mem = FiveDimensionalMemory::new("人设", 2);
        mem.push_recent("未来条目", 5_000);
        // 回拨到 1_000：饱和减法让差值算作 0，条目仍在
        assert_eq!(mem.get_recent(1_000), vec!["未来条目".to_string()]);
        mem.prune_recent(1_000);
        assert_eq!(mem.get_recent(5_000).len(), 1);
    }
}
