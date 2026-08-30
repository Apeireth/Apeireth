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
    /// 近期记忆列表
    recent_memories: Vec<String>,
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

        // 3. Facts
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

        // 4. Reflections
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
}
