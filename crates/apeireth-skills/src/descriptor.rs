//! R33-x 动态运营层: SkillDescriptor (per VCP `vcptoolbox/modules` 路由 5 字段)
//!
//! **目标**: 跟 Skill (R23 LOCKED 4 字段) 正交, 加扩展 metadata 字段:
//! - `description` — 一句话能力描述
//! - `tags` — 路由标签 (e.g. `["summarize", "text"]`)
//! - `source` — 来源 (`vcptoolbox` / `local` / `user`)
//! - `input_example` / `output_example` — 示例 payload (JSON 字符串)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 Skill / Registry / parse_version / is_valid_id (R23 LOCKED)
//! - 0 改 workspace 1.0.0 / 24 LOCKED crate
//!
//! **借鉴锚 (S-1)**: VCP `vcptoolbox/modules/<name>/index.js` 的 5 字段 metadata 模式
//! (name / description / dependencies / env / handler)

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{Registry, Skill, SkillResult};

/// 动态运营层 metadata 扩展 (跟 Skill 正交, 0 改 Skill 4 字段)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDescriptor {
    /// 跟 Skill.id 1:1
    pub id: String,
    /// 跟 Skill.version 1:1
    pub version: String,
    /// 一句话能力描述 (per VCP `description` 字段)
    pub description: String,
    /// 路由标签 (per VCP `tags` 字段, 用于 select_by_tag)
    pub tags: BTreeSet<String>,
    /// 来源 (`vcptoolbox` / `local` / `user` / `apeireth-eval` / ...)
    pub source: String,
    /// 输入示例 (JSON 字符串, 给 discovery UI 用)
    pub input_example: String,
    /// 输出示例 (JSON 字符串, 给 discovery UI 用)
    pub output_example: String,
}

impl SkillDescriptor {
    /// 便利构造
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        tags: impl IntoIterator<Item = String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            description: description.into(),
            tags: tags.into_iter().collect(),
            source: source.into(),
            input_example: "{}".to_string(),
            output_example: "{}".to_string(),
        }
    }

    /// 绑示例 payload
    pub fn with_examples(mut self, input: impl Into<String>, output: impl Into<String>) -> Self {
        self.input_example = input.into();
        self.output_example = output.into();
        self
    }

    /// 是否命中某个 tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }
}

/// 扩展 Registry: 维护 Skill → SkillDescriptor 映射 (0 改 R23 Registry)
#[derive(Debug, Default, Clone)]
pub struct DescriptorRegistry {
    inner: Registry,
    descriptors: Vec<SkillDescriptor>,
}

impl DescriptorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 (Skill + Descriptor) 对 — Skill 必须先 register, Descriptor 跟它 id 一致
    pub fn register_pair(&mut self, skill: Skill, descriptor: SkillDescriptor) -> SkillResult<()> {
        skill.validate()?;
        self.inner.register(skill)?;
        if descriptor.id
            != self
                .inner
                .ids()
                .last()
                .map(|s| (*s).to_string())
                .unwrap_or_default()
        {
            // 由于 Registry register 内部 sort, 我们要按 inner.get 找 id
            let _ = self.inner.get(&descriptor.id)?;
        }
        self.descriptors.push(descriptor);
        self.descriptors.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(())
    }

    /// 通过 id 查 descriptor
    pub fn descriptor(&self, id: &str) -> Option<&SkillDescriptor> {
        self.descriptors.iter().find(|d| d.id == id)
    }

    /// 通过 tag 路由 — 给动态运营层 select 用
    pub fn select_by_tag(&self, tag: &str) -> Vec<&SkillDescriptor> {
        self.descriptors.iter().filter(|d| d.has_tag(tag)).collect()
    }

    /// 通过 source 路由
    pub fn select_by_source(&self, source: &str) -> Vec<&SkillDescriptor> {
        self.descriptors
            .iter()
            .filter(|d| d.source == source)
            .collect()
    }

    /// 全 descriptors (sorted by id, 给 reporting 用)
    pub fn descriptors(&self) -> &[SkillDescriptor] {
        &self.descriptors
    }

    /// 总数
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// 暴露底层 Registry (R23 兼容)
    pub fn registry(&self) -> &Registry {
        &self.inner
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Skill;

    fn desc(id: &str) -> SkillDescriptor {
        SkillDescriptor::new(
            id,
            "1.0.0",
            format!("{id} skill"),
            vec!["alpha".to_string(), "beta".to_string()],
            "test",
        )
    }

    #[test]
    fn descriptor_new_basic() {
        let d = desc("summarize-text");
        assert_eq!(d.id, "summarize-text");
        assert_eq!(d.version, "1.0.0");
        assert!(d.has_tag("alpha"));
        assert!(d.has_tag("beta"));
        assert!(!d.has_tag("gamma"));
    }

    #[test]
    fn descriptor_with_examples() {
        let d = desc("x").with_examples(r#"{"text":"hello"}"#, r#"{"summary":"hi"}"#);
        assert!(d.input_example.contains("hello"));
        assert!(d.output_example.contains("hi"));
    }

    #[test]
    fn registry_register_pair() {
        let mut r = DescriptorRegistry::new();
        let skill = Skill::new("summarize-text", "1.0.0", "{}", "{}");
        let descriptor = desc("summarize-text");
        r.register_pair(skill, descriptor).unwrap();
        assert_eq!(r.len(), 1);
        let d = r.descriptor("summarize-text").unwrap();
        assert_eq!(d.id, "summarize-text");
    }

    #[test]
    fn registry_select_by_tag() {
        let mut r = DescriptorRegistry::new();
        r.register_pair(
            Skill::new("a", "1.0.0", "{}", "{}"),
            SkillDescriptor::new("a", "1.0.0", "a", vec!["x".to_string()], "local"),
        )
        .unwrap();
        r.register_pair(
            Skill::new("b", "1.0.0", "{}", "{}"),
            SkillDescriptor::new("b", "1.0.0", "b", vec!["y".to_string()], "local"),
        )
        .unwrap();
        r.register_pair(
            Skill::new("c", "1.0.0", "{}", "{}"),
            SkillDescriptor::new(
                "c",
                "1.0.0",
                "c",
                vec!["x".to_string(), "y".to_string()],
                "vcptoolbox",
            ),
        )
        .unwrap();
        assert_eq!(r.select_by_tag("x").len(), 2);
        assert_eq!(r.select_by_tag("y").len(), 2);
        assert_eq!(r.select_by_source("local").len(), 2);
        assert_eq!(r.select_by_source("vcptoolbox").len(), 1);
    }

    #[test]
    fn registry_duplicate_rejected() {
        let mut r = DescriptorRegistry::new();
        r.register_pair(Skill::new("a", "1.0.0", "{}", "{}"), desc("a"))
            .unwrap();
        let result = r.register_pair(Skill::new("a", "1.0.0", "{}", "{}"), desc("a"));
        assert!(result.is_err());
    }
}
