//! SG-25 工具能力策展表 (v0): 给「工具生态健康」内核喂**策展过的能力标签**。
//!
//! ## 为什么需要这张表
//!
//! [`apeireth_tools_canonical::ecosystem_health`] 的 `analyze` 只看
//! [`ToolProfile::capabilities`]；但生产注册面的
//! [`ToolCapability`] 只有 `id` / `declaration`
//! / `invoke` —— **声明里没有能力标签**。从描述文本反推标签属于伪接线（把
//! 「读过描述」当「知道能力」），因此 v0 选择**策展**：由本表显式登记每个工具
//! 的能力标签与来源族；表未收录的工具**不猜测**，一律计为 `untagged`，其读数
//! 不参与分数（覆盖度另行如实上报）。
//!
//! ## 口径 (0 装诚实)
//!
//! - 本表是**策展事实**，不是从代码/描述自动推导的推断；新增工具面必须在这里
//!   显式登记（完整性测试会盯住已声明的工具 id 是否都有条目）。
//! - 标签词汇表 [`TAXONOMY_TAGS`] 是封闭集合：表条目只能引用词汇表内的标签
//!   （单元测试强制）；词汇表扩容 = 改本文件 + 版本号 +1。
//! - 表里**不写**「健康/不健康」结论——结论只由内核公式按标签集合计算。
//! - 目标态（B 类 ADR 前置，见
//!   `docs/04-internal/backend-enhancement/SG-25-tool-capability-taxonomy-proposal-2026-10-20.md`）
//!   是把标签变成**合同声明**（`ToolDeclaration` 带能力标签）；在合同演进之前，
//!   本表是唯一真源，禁止另起一套映射。
//!
//! ## 词汇表 (13 个能力族)
//!
//! 与生物隐喻解耦后的工程槽位：文件/计算/进程/网络/检索/记忆/媒体/文档/设备/
//! 治理/陪伴/自述/MCP。`untagged` **不是**标签，是「本表未收录」的显式分类。

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::ecosystem_health::{ToolProfile, ToolSource};

/// 策展表版本：表内容（条目或词汇表）变更时 +1，冻结进自述读数。
pub const TOOL_TAXONOMY_VERSION: u32 = 1;

/// 封闭标签词汇表（顺序即展示顺序；表条目只能引用此处出现的标签）。
pub const TAXONOMY_TAGS: &[&str] = &[
    "filesystem",
    "compute",
    "process",
    "network",
    "search",
    "memory",
    "media",
    "document",
    "device",
    "governance",
    "companion",
    "self",
    "mcp",
];

/// 策展表的一行：工具身份（声明名 / 能力 id 后缀）+ 能力标签 + 来源族。
///
/// 能力标签为**静态切片**：装配期只读拷贝，运行期不分配、不改写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxonomyEntry {
    /// 工具身份：与 [`NormalizedTool::name`](apeireth_protocol::canonical::NormalizedTool)
    /// 一致（同时兜底匹配能力 id 的最后一段）。
    pub key: &'static str,
    /// 能力标签（必须 ⊆ [`TAXONOMY_TAGS`]；空切片 = 显式「无标签」，不推荐）。
    pub tags: &'static [&'static str],
    /// 来源族：builtin / mcp / local（与内核 `ToolSource` 同值域）。
    pub source: ToolSource,
}

/// 策展表 v0：覆盖当前生产注册面的静态工具（不含运行期动态发现的 MCP 工具，
/// 后者在发现前不在快照里，属如实缺口）。
///
/// 维护规则：新增工具面时在此登记一行；删除工具面时删行并 +版本号。表按 `key`
/// 字典序排列（单元测试强制），便于肉眼比对。
pub const TOOL_TAXONOMY: &[TaxonomyEntry] = &[
    TaxonomyEntry {
        key: "apply_patch",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "backup_export",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "calculator",
        tags: &["compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "care_schedule",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "content_feed",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "device_control",
        tags: &["device"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "device_sense",
        tags: &["device"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "doc_ops",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "education",
        tags: &["document", "companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "exposure_audit",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "fetch",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "filesystem",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "git_write",
        tags: &["filesystem", "process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "guard_watch",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "image_ops",
        tags: &["media"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "mcp",
        tags: &["mcp"],
        source: ToolSource::Mcp,
    },
    TaxonomyEntry {
        key: "play_abort",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "play_finish",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "play_propose",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "play_start",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "process",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "proposal_draft",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "repo",
        tags: &["filesystem", "search"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "ritual_done",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "ritual_skip",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "search",
        tags: &["search"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "self_status",
        tags: &["self"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "sheet_ops",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "shell",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "skill_distill",
        tags: &["memory"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "skill_search",
        tags: &["search", "memory"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "std_sub_supervisor",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "supervisor",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "task_board",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "trait_report",
        tags: &["self", "governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "voice_speak",
        tags: &["media"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "weather",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "web_fetch",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "worktree_apply",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "worktree_plan",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
];

/// 按工具身份查策展条目（精确匹配；未收录 = `None`，**不猜测**）。
///
/// 身份优先用模型可见的工具名（[`NormalizedTool::name`]），其次用能力 id 的
/// 最后一段（`tool.<name>` 的 `<name>`），两者都由调用方显式传入。
#[must_use]
pub fn taxonomy_entry_for(identity: &str) -> Option<&'static TaxonomyEntry> {
    TOOL_TAXONOMY.iter().find(|entry| entry.key == identity)
}

/// 能力 id 的最后一段（如 `tool.play_start` → `play_start`；无点 = 原样）。
#[must_use]
pub fn capability_identity(capability_id: &str) -> &str {
    capability_id.rsplit('.').next().unwrap_or(capability_id)
}

/// 一次策展投影的结果：给内核的画像 + 未收录工具的如实清单。
#[derive(Debug, Clone, PartialEq)]
pub struct TaxonomyProjection {
    /// 有序画像（按能力注册顺序；仅收录命中策展条目的工具）。
    pub profiles: Vec<ToolProfile>,
    /// 参与投影的注册工具总数（含未收录）。
    pub registered: usize,
    /// 未收录的工具身份（`declaration().name`，去重后按字典序）。
    pub untagged: Vec<String>,
}

impl TaxonomyProjection {
    /// 已收录工具数（= `profiles.len()`，语义别名，读自述时更直白）。
    #[must_use]
    pub fn tagged(&self) -> usize {
        self.profiles.len()
    }

    /// 覆盖率读数：`tagged / registered`；零注册 = `None`（不除零、不编 0）。
    #[must_use]
    pub fn coverage(&self) -> Option<f64> {
        if self.registered == 0 {
            return None;
        }
        Some(self.tagged() as f64 / self.registered as f64)
    }
}

/// 把装配面上的能力集合投影成内核画像（纯函数，只读声明，不调用工具）。
///
/// 未命中策展条目的工具**不生成画像**（不猜标签），只记入 `untagged`；调用方
/// 必须把 `untagged` 一并如实上报，禁止把「未收录」洗成「健康」。
#[must_use]
pub fn project_capabilities(
    capabilities: &[std::sync::Arc<dyn ToolCapability>],
) -> TaxonomyProjection {
    let mut profiles = Vec::with_capacity(capabilities.len());
    let mut untagged = Vec::new();
    for capability in capabilities {
        let declaration = capability.declaration();
        let id = capability.id().as_str();
        let identity = if declaration.name.is_empty() {
            capability_identity(id).to_string()
        } else {
            declaration.name.clone()
        };
        match taxonomy_entry_for(&identity).or_else(|| taxonomy_entry_for(capability_identity(id)))
        {
            Some(entry) => profiles.push(ToolProfile::new(
                identity.clone(),
                entry.tags.iter().map(|tag| (*tag).to_string()).collect(),
                entry.source,
            )),
            None => {
                if !untagged.iter().any(|seen| seen == &identity) {
                    untagged.push(identity);
                }
            }
        }
    }
    untagged.sort();
    TaxonomyProjection {
        profiles,
        registered: capabilities.len(),
        untagged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::CapabilityId;
    use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
    use async_trait::async_trait;

    struct StubTool {
        id: CapabilityId,
        name: &'static str,
    }

    #[async_trait]
    impl ToolCapability for StubTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> NormalizedTool {
            NormalizedTool::new(self.name)
        }

        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, serde_json::json!({ "stub": true }))
        }
    }

    fn stub(capability_id: &str, name: &'static str) -> std::sync::Arc<dyn ToolCapability> {
        std::sync::Arc::new(StubTool {
            id: CapabilityId::new(capability_id).expect("static capability id is valid"),
            name,
        })
    }

    #[test]
    fn the_table_is_sorted_unique_and_only_uses_known_tags() {
        let mut keys = Vec::new();
        for entry in TOOL_TAXONOMY {
            keys.push(entry.key);
            assert!(
                !entry.tags.is_empty(),
                "{} 必须有显式标签（空切片是显式无标签，不推荐）",
                entry.key
            );
            for tag in entry.tags {
                assert!(
                    TAXONOMY_TAGS.contains(tag),
                    "{} 引用了词汇表外的标签 {tag}",
                    entry.key
                );
            }
            let mut sorted_tags = entry.tags.to_vec();
            sorted_tags.sort_unstable();
            sorted_tags.dedup();
            assert_eq!(
                sorted_tags.len(),
                entry.tags.len(),
                "{} 的标签不得重复",
                entry.key
            );
        }
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "策展表必须按 key 字典序排列");
        let mut unique = keys.clone();
        unique.dedup();
        assert_eq!(unique.len(), keys.len(), "策展表 key 不得重复");
        assert_eq!(TOOL_TAXONOMY_VERSION, 1, "表内容变更时必须同步 +版本号");
    }

    #[test]
    fn every_declared_production_tool_identity_has_a_curated_entry() {
        // 本清单来自生产注册面（`crates/capabilities/tools` 与
        // `crates/engine/runtime-assembly` 里声明过的工具身份）；新增工具面
        // 必须同步进表，否则本测试失败——这是策展表的完整性闸门。
        let declared = [
            "apply_patch",
            "backup_export",
            "calculator",
            "care_schedule",
            "content_feed",
            "device_control",
            "device_sense",
            "doc_ops",
            "education",
            "exposure_audit",
            "fetch",
            "filesystem",
            "git_write",
            "guard_watch",
            "image_ops",
            "mcp",
            "play_abort",
            "play_finish",
            "play_propose",
            "play_start",
            "process",
            "proposal_draft",
            "repo",
            "ritual_done",
            "ritual_skip",
            "search",
            "self_status",
            "sheet_ops",
            "shell",
            "skill_distill",
            "skill_search",
            "std_sub_supervisor",
            "supervisor",
            "task_board",
            "trait_report",
            "voice_speak",
            "weather",
            "web_fetch",
            "worktree_apply",
            "worktree_plan",
        ];
        for identity in declared {
            assert!(
                taxonomy_entry_for(identity).is_some(),
                "已声明工具面 {identity} 未收录进策展表"
            );
        }
        assert_eq!(
            declared.len(),
            TOOL_TAXONOMY.len(),
            "策展表条目数应与已声明工具面清单一致（漏登或陈旧行）"
        );
    }

    #[test]
    fn projection_tags_known_tools_and_reports_untagged_honestly() {
        let capabilities: Vec<std::sync::Arc<dyn ToolCapability>> = vec![
            stub("tool.filesystem", "filesystem"),
            stub("tool.play_start", "play_start"),
            stub("tool.unknown_face", "mystery_tool"),
        ];
        let projection = project_capabilities(&capabilities);
        assert_eq!(projection.registered, 3);
        assert_eq!(projection.tagged(), 2);
        assert_eq!(projection.untagged, vec!["mystery_tool".to_string()]);
        assert_eq!(projection.coverage(), Some(2.0 / 3.0));
        assert_eq!(projection.profiles[0].name, "filesystem");
        assert_eq!(projection.profiles[0].capabilities, vec!["filesystem"]);
        assert_eq!(projection.profiles[1].source, ToolSource::Builtin);
    }

    #[test]
    fn projection_falls_back_to_capability_id_when_declaration_name_is_blank() {
        let capabilities: Vec<std::sync::Arc<dyn ToolCapability>> =
            vec![stub("tool.sheet_ops", "")];
        let projection = project_capabilities(&capabilities);
        assert_eq!(projection.tagged(), 1);
        assert_eq!(projection.profiles[0].name, "sheet_ops");
    }

    #[test]
    fn empty_assembly_projects_to_empty_and_zero_coverage_is_not_faked() {
        let projection = project_capabilities(&[]);
        assert_eq!(projection.registered, 0);
        assert!(projection.profiles.is_empty());
        assert_eq!(projection.coverage(), None);
    }
}
