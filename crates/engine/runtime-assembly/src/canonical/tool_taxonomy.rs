//! SG-25 工具能力策展表 (v1): 给「工具生态健康」内核喂**策展过的能力标签**。
//!
//! ## 为什么需要这张表
//!
//! [`apeireth_tools_canonical::ecosystem_health`] 的 `analyze` 只看
//! [`ToolProfile::capabilities`]；但生产注册面的
//! [`ToolCapability`] 只有 `id` / `declaration`
//! / `invoke` —— **声明里没有能力标签**。从描述文本反推标签属于伪接线（把
//! 「读过描述」当「知道能力」），因此本表选择**策展**：由本表显式登记每个工具
//! 的能力标签与来源族；表未收录的工具**不猜测**，一律计为 `untagged`，其读数
//! 不参与分数（覆盖度另行如实上报）。
//!
//! ## 口径 (0 装诚实)
//!
//! - 本表是**策展事实**，不是从代码/描述自动推导的推断；新增工具面必须在这里
//!   显式登记（完整性测试会盯住生产注册面是否都有条目）。
//! - **表键 = 能力 id 全量形态**（`tool.<name>` / `tool.<family>.<name>`），
//!   不是模型可见的工具名：能力 id 才是注册面的唯一稳定身份，模型可见名
//!   （如 `re.pack`）会随措辞漂移，能力 id 不会。
//! - 标签词汇表 [`TAXONOMY_TAGS`] 是封闭集合：表条目只能引用词汇表内的标签
//!   （单元测试强制）；词汇表扩容 = 改本文件 + 版本号 +1。
//! - 标签描述工具**作用的能力域**（它读/写/算/连/检索什么），**不描述实现
//!   手段**：外部二进制适配器统一经唯一子进程边界执行，那是手段不是域。
//! - 表里**不写**「健康/不健康」结论——结论只由内核公式按标签集合计算。
//! - 目标态（B 类 ADR 前置，见
//!   `docs/04-internal/backend-enhancement/SG-25-tool-capability-taxonomy-proposal-2026-10-20.md`）
//!   是把标签变成**合同声明**（`ToolDeclaration` 带能力标签）；在合同演进之前，
//!   本表是唯一真源，禁止另起一套映射。
//!
//! ## 表覆盖的三类面 (v1 口径归位)
//!
//! v0 的表把「策展条目」与「生产声明面」混为一谈：登记表收了生产根本没有的
//! 工具（幽灵条目），又漏了生产早已注册的工具（漏登），完整性闸门还是拿**手抄
//! 的清单**跟表比长度 —— 三者互相掩盖，读数因此虚高/虚低。v1 把口径钉死成
//! 三类面，每类都有可复核的出处：
//!
//! 1. **已注册工具面**（表的主体）：生产注册面真实注册的 `tool.*` 能力 id。
//!    完整性测试从**生产源码**扫出这一全集（见 `production_tool_capability_ids`），
//!    两向断言：表里每条都要对上真实注册面；真实注册面的每个 id 都要在表里。
//! 2. **动态能力族**（[`DYNAMIC_FAMILY_ROOTS`]）：`tool.mcp` 是**族根**，代表
//!    运行期随服务器发现而变的 `tool.mcp.<server>.<tool>` 成员。动态 id 无法逐个
//!    静态登记，只能按**族前缀**登记，否则动态能力永远无法被静态覆盖。
//! 3. **库级保留面**（[`LIBRARY_LEVEL_FACES`]）：`tool.process` /
//!    `tool.supervisor` / `tool.std_sub_supervisor` 是**库级面**（进程隔离与
//!    子进程监管），生产策略分类器（只读预设 / 权限洋葱分层）按能力 id 形态
//!    引用它们，但**生产没有注册同名工具**。口径与
//!    `docs/03-reference/capabilities-matrix.md` §1 一致：登记但**显式标注
//!    「未注册为工具」**，不与已注册工具面混为一谈。
//!
//! ## 词汇表 (13 个能力族)
//!
//! 与生物隐喻解耦后的工程槽位：文件/计算/进程/网络/检索/记忆/媒体/文档/设备/
//! 治理/陪伴/自述/MCP。`untagged` **不是**标签，是「本表未收录」的显式分类。

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::ecosystem_health::{ToolProfile, ToolSource};
use apeireth_tools_canonical::mcp_bridge::{is_capability_in_family, MCP_FAMILY_ROOT};

/// 策展表版本：表内容（条目或词汇表）变更时 +1，冻结进自述读数。
pub const TOOL_TAXONOMY_VERSION: u32 = 3;

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

/// 动态能力 id 族的**族根前缀**（成员随外部服务器发现而变，无法逐个静态登记）。
///
/// 族根字面量的**单一真源**在工具侧
/// （`apeireth_tools_canonical::mcp_bridge::MCP_FAMILY_ROOT`），本表只引用，
/// 不留第二份。生产侧的 id 形态由
/// `apeireth_tools_canonical::mcp_bridge::catalog::capability_id_for` 生成
/// （`tool.mcp.<server>.<tool>`）；完整性测试用该**生产函数**反查族根，
/// 防止族根字面量写漂。族根条目按**前缀**匹配动态 id（见 [`taxonomy_entry_for`]）。
pub const DYNAMIC_FAMILY_ROOTS: &[&str] = &[MCP_FAMILY_ROOT];

/// 库级保留面（**未注册为工具**）：生产策略分类器按能力 id 形态引用，但生产
/// 没有注册同名工具。
///
/// - `tool.process` —— 进程隔离面（`crates/capabilities/tools/src/process/`），
///   只读预设与权限洋葱分层把它列为执行面；执行入口是 `tool.shell`，本条
///   **没有** `ToolCapability` 实现。
/// - `tool.std_sub_supervisor` —— 标准子进程监管面
///   （`crates/capabilities/tools/src/std_sub_supervisor.rs`），同上：无工具实现。
/// - `tool.supervisor` —— 子进程监管面
///   （`crates/capabilities/tools/src/supervisor.rs`），同上：无工具实现。
///
/// 口径与 `docs/03-reference/capabilities-matrix.md` §1 的「库级能力（未注册为
/// 工具）」一致。完整性测试会断言这三条**确实不在生产注册面里** —— 一旦哪天
/// 它们被注册成真工具，本清单必须同步删除（否则策展表会拿库级面冒充工具面）。
pub const LIBRARY_LEVEL_FACES: &[&str] =
    &["tool.process", "tool.std_sub_supervisor", "tool.supervisor"];

/// 策展表的一行：工具身份（能力 id）+ 能力标签 + 来源族。
///
/// 能力标签为**静态切片**：装配期只读拷贝，运行期不分配、不改写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxonomyEntry {
    /// 工具身份 = **能力 id 全量形态**（`tool.<name>` / `tool.<family>.<name>`）。
    ///
    /// v0 曾用模型可见的工具名当键；v1 改用能力 id：能力 id 才是注册面的稳定
    /// 身份（模型可见名会随措辞漂移，且动态族成员无固定模型名）。
    pub key: &'static str,
    /// 能力标签（必须 ⊆ [`TAXONOMY_TAGS`]；空切片 = 显式「无标签」，不推荐）。
    pub tags: &'static [&'static str],
    /// 来源族：builtin / mcp / local（与内核 `ToolSource` 同值域）。
    pub source: ToolSource,
}

/// 策展表 v1：覆盖生产注册面的静态工具 + 动态能力族族根 + 库级保留面。
///
/// 维护规则：新增工具面时在此登记一行；删除工具面时删行并 +版本号。表按 `key`
/// 字典序排列（单元测试强制），便于肉眼比对。
pub const TOOL_TAXONOMY: &[TaxonomyEntry] = &[
    TaxonomyEntry {
        key: "tool.apply_patch",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.backup_export",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.care_schedule",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.content_feed",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.device_control",
        tags: &["device"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.device_sense",
        tags: &["device"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.doc_ops",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.education",
        tags: &["document", "companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.exposure_audit",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    // 外部二进制适配族 (`tool.external.*`)：读本地文件 / 连受限网络 / 经唯一
    // 子进程边界调用外部分析器。子进程边界是**手段**，不进标签。
    TaxonomyEntry {
        key: "tool.external.apktool",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.binwalk",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.exiftool",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.ghidra",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.jadx",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.nmap",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.openssl",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.radare2",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.tshark",
        tags: &["filesystem", "network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.external.yara",
        tags: &["filesystem", "search"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.fetch",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.filesystem",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.git_write",
        tags: &["filesystem", "process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.guard_watch",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.image_ops",
        tags: &["media"],
        source: ToolSource::Builtin,
    },
    // 动态能力族族根：代表运行期随服务器发现而变的 `tool.mcp.<server>.<tool>`。
    TaxonomyEntry {
        key: MCP_FAMILY_ROOT,
        tags: &["mcp"],
        source: ToolSource::Mcp,
    },
    TaxonomyEntry {
        key: "tool.play_abort",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.play_finish",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.play_propose",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.play_start",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    // 库级保留面（**未注册为工具**）：见 [`LIBRARY_LEVEL_FACES`]。
    TaxonomyEntry {
        key: "tool.process",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.proposal_draft",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    // 静态侦察族 (`tool.recon.*`)：进程内静态分析，读本地文件；两件带受限网络面。
    TaxonomyEntry {
        key: "tool.recon.binfmt",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.dotnet",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.js",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.net",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.pack",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.pcap",
        tags: &["filesystem", "network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.tls",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.web",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.recon.yara",
        tags: &["filesystem", "search"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.repo",
        tags: &["filesystem", "search"],
        source: ToolSource::Builtin,
    },
    // 代码地图 (`tool.repo_map`)：工作区限域**只读**，扫源文件抽符号签名 → 建跨文件
    // 依赖图 → Personalized PageRank 排序 ⇒ 能力域 = **读本地文件**（filesystem）
    // + **进程内静态分析计算**（compute），与静态侦察族 `tool.recon.*` 同域
    // （纯查询无检索面 ⇒ 不挂 `search` 标签）。
    TaxonomyEntry {
        key: "tool.repo_map",
        tags: &["filesystem", "compute"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.ritual_done",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.ritual_skip",
        tags: &["companion"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.search",
        tags: &["search"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.self_status",
        tags: &["self"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.sheet_ops",
        tags: &["document"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.shell",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.skill_distill",
        tags: &["memory"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.skill_search",
        tags: &["search", "memory"],
        source: ToolSource::Builtin,
    },
    // 库级保留面（**未注册为工具**）：见 [`LIBRARY_LEVEL_FACES`]。
    TaxonomyEntry {
        key: "tool.std_sub_supervisor",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    // 库级保留面（**未注册为工具**）：见 [`LIBRARY_LEVEL_FACES`]。
    TaxonomyEntry {
        key: "tool.supervisor",
        tags: &["process"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.task_board",
        tags: &["governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.trait_report",
        tags: &["self", "governance"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.voice_speak",
        tags: &["media"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.weather",
        tags: &["network"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.worktree_apply",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
    TaxonomyEntry {
        key: "tool.worktree_plan",
        tags: &["filesystem"],
        source: ToolSource::Builtin,
    },
];

/// 按能力 id 查策展条目（精确匹配；动态族按**族根前缀**匹配；未收录 = `None`，
/// **不猜测**）。
///
/// 匹配顺序：
/// 1. **精确匹配**：能力 id 与表键全等（`tool.filesystem`）。
/// 2. **族根前缀匹配**：能力 id 以某个动态族根 + `.` 开头
///    （`tool.mcp.<server>.<tool>` → `tool.mcp`）。多个族根同时命中时取**最长**
///    的那个；同长度不会有两个不同族根同时前缀命中同一 id。
#[must_use]
pub fn taxonomy_entry_for(capability_id: &str) -> Option<&'static TaxonomyEntry> {
    if let Some(entry) = TOOL_TAXONOMY
        .iter()
        .find(|entry| entry.key == capability_id)
    {
        return Some(entry);
    }
    DYNAMIC_FAMILY_ROOTS
        .iter()
        .filter_map(|root| {
            TOOL_TAXONOMY
                .iter()
                .find(|entry| entry.key == *root && is_capability_in_family(capability_id, root))
        })
        .max_by_key(|entry| entry.key.len())
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
        // 主查：能力 id（精确 / 动态族前缀）。兜底：模型可见名恰好是能力 id 形态。
        match taxonomy_entry_for(id).or_else(|| taxonomy_entry_for(&identity)) {
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
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

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
        assert_eq!(TOOL_TAXONOMY_VERSION, 3, "表内容变更时必须同步 +版本号");
    }

    // ---------- 生产注册面扫描（完整性闸门的真源） ----------

    /// 仓根（`crates/engine/runtime-assembly` 上溯三层）。
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
    }

    /// 递归收集 `crates/` 下的生产 `.rs` 源文件（排除夹具目录与夹具文件）。
    fn collect_production_sources(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if matches!(
                    name.as_ref(),
                    "tests" | "benches" | "examples" | "fuzz" | "target"
                ) {
                    continue;
                }
                collect_production_sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !matches!(name.as_ref(), "tests.rs" | "test.rs")
            {
                out.push(path);
            }
        }
    }

    /// 在字节流里找子串（从 `from` 起）。
    fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
        if from >= haystack.len() || needle.is_empty() {
            return None;
        }
        haystack[from..]
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|offset| from + offset)
    }

    /// 跳过空白与注释（行注释 / 块注释）。
    fn skip_ws_and_comments(bytes: &[u8], mut index: usize) -> usize {
        while index < bytes.len() {
            match bytes[index] {
                b' ' | b'\t' | b'\r' | b'\n' => index += 1,
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    while index < bytes.len() && bytes[index] != b'\n' {
                        index += 1;
                    }
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    index += 2;
                    while index + 1 < bytes.len()
                        && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                    {
                        index += 1;
                    }
                    index += 2;
                }
                _ => break,
            }
        }
        index
    }

    /// 字符字面量末尾下标（`'a'` / `'\n'`）；生命周期标记（`'a`）返回 `None`。
    fn char_literal_end(bytes: &[u8], index: usize) -> Option<usize> {
        let next = *bytes.get(index + 1)?;
        if next == b'\\' {
            let mut cursor = index + 2;
            while cursor < bytes.len() && bytes[cursor] != b'\'' {
                cursor += 1;
            }
            return (cursor < bytes.len()).then_some(cursor + 1);
        }
        if bytes.get(index + 2) == Some(&b'\'') {
            Some(index + 3)
        } else {
            None
        }
    }

    /// 跳过字符串字面量，返回结束引号之后的下标。
    fn skip_string_literal(bytes: &[u8], at: usize) -> usize {
        let mut index = at + 1;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index += 2;
                continue;
            }
            if bytes[index] == b'"' {
                return index + 1;
            }
            index += 1;
        }
        index
    }

    /// 读字符串字面量的内容（不含引号）；不是字面量则返回 `None`。
    fn read_string_literal(bytes: &[u8], at: usize) -> Option<String> {
        if bytes.get(at) != Some(&b'"') {
            return None;
        }
        let end = skip_string_literal(bytes, at);
        String::from_utf8(bytes[at + 1..end.saturating_sub(1)].to_vec()).ok()
    }

    /// 匹配 `{` 的闭合 `}`（跳过字符串 / 字符字面量 / 注释）。
    fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
        let mut depth = 0i32;
        let mut index = open;
        while index < bytes.len() {
            match bytes[index] {
                b'/' if matches!(bytes.get(index + 1), Some(&b'/') | Some(&b'*')) => {
                    index = skip_ws_and_comments(bytes, index);
                }
                b'"' => index = skip_string_literal(bytes, index),
                b'\'' => match char_literal_end(bytes, index) {
                    Some(end) => index = end,
                    None => index += 1,
                },
                b'{' => {
                    depth += 1;
                    index += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index);
                    }
                    index += 1;
                }
                _ => index += 1,
            }
        }
        None
    }

    /// 匹配 `(` 的闭合 `)`，返回其后的下标。
    fn matching_paren(bytes: &[u8], open: usize) -> Option<usize> {
        let mut depth = 0i32;
        let mut index = open;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => index = skip_string_literal(bytes, index),
                b'(' => {
                    depth += 1;
                    index += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index + 1);
                    }
                    index += 1;
                }
                _ => index += 1,
            }
        }
        None
    }

    /// 属性结束位置：`)` 之后可能还有 `]`。
    fn skip_attribute_close(bytes: &[u8], index: usize) -> usize {
        if bytes.get(index) == Some(&b']') {
            index + 1
        } else {
            index
        }
    }

    /// cfg 表达式是否表示「仅测试编译」。
    fn is_test_gated(expr: &[u8]) -> bool {
        let compact: Vec<u8> = expr
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        if compact.windows(9).any(|window| window == b"not(test)") {
            return false;
        }
        let mut index = 0usize;
        while index < expr.len() {
            if expr[index].is_ascii_alphabetic() || expr[index] == b'_' {
                let start = index;
                while index < expr.len()
                    && (expr[index].is_ascii_alphanumeric() || expr[index] == b'_')
                {
                    index += 1;
                }
                if &expr[start..index] == b"test" {
                    return true;
                }
            } else {
                index += 1;
            }
        }
        false
    }

    /// 受 cfg 门控项的边界：其后第一个顶层 `{` 或 `;`。
    fn gated_item_extent(bytes: &[u8], from: usize) -> Option<(u8, usize)> {
        let mut depth = 0i32;
        let mut index = from;
        while index < bytes.len() {
            match bytes[index] {
                b'/' if matches!(bytes.get(index + 1), Some(&b'/') | Some(&b'*')) => {
                    index = skip_ws_and_comments(bytes, index);
                }
                b'"' => index = skip_string_literal(bytes, index),
                b'\'' => match char_literal_end(bytes, index) {
                    Some(end) => index = end,
                    None => index += 1,
                },
                b'(' | b'[' => {
                    depth += 1;
                    index += 1;
                }
                b')' | b']' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    index += 1;
                }
                b'{' | b';' if depth == 0 => return Some((bytes[index], index)),
                _ => index += 1,
            }
        }
        None
    }

    /// 把 `#[cfg(test)]`（及 `#[cfg(all(test, …))]`）门控的整段代码抹白。
    ///
    /// 抹白保持字节长度，便于后续偏移对齐；抹白后测试夹具里的能力 id 字面量
    /// 不会进入生产扫描结果（夹具 id 不该进策展表）。
    fn strip_test_gated(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = bytes.to_vec();
        let mut cursor = 0usize;
        while let Some(marker) = find_from(bytes, b"#[cfg(", cursor) {
            let Some(paren) = find_from(bytes, b"(", marker + 5) else {
                cursor = marker + 1;
                continue;
            };
            let Some(after_paren) = matching_paren(bytes, paren) else {
                cursor = marker + 1;
                continue;
            };
            if !is_test_gated(&bytes[paren + 1..after_paren - 1]) {
                cursor = marker + 1;
                continue;
            }
            let mut item_start =
                skip_ws_and_comments(bytes, skip_attribute_close(bytes, after_paren));
            while find_from(bytes, b"#[", item_start) == Some(item_start) {
                let Some(inner_paren) = find_from(bytes, b"(", item_start) else {
                    break;
                };
                let Some(inner_end) = matching_paren(bytes, inner_paren) else {
                    break;
                };
                item_start = skip_ws_and_comments(bytes, skip_attribute_close(bytes, inner_end));
            }
            let Some((kind, at)) = gated_item_extent(bytes, item_start) else {
                cursor = marker + 1;
                continue;
            };
            let end = if kind == b'{' {
                match matching_brace(bytes, at) {
                    Some(close) => close,
                    None => {
                        cursor = marker + 1;
                        continue;
                    }
                }
            } else {
                at
            };
            for slot in &mut out[marker..=end] {
                if *slot != b'\n' {
                    *slot = b' ';
                }
            }
            cursor = end + 1;
        }
        String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
    }

    /// 抽出源码里声明的 `tool.*` 能力 id（两种注册写法）。
    ///
    /// - `CapabilityId::new("tool.…")`（含 `id: CapabilityId::new(…)` 形态）；
    /// - `const …_CAPABILITY_ID: &str = "tool.…";`（常量形态）。
    fn extract_tool_capability_ids(text: &str) -> Vec<String> {
        let bytes = text.as_bytes();
        let mut ids = Vec::new();
        let mut cursor = 0usize;
        while let Some(marker) = find_from(bytes, b"CapabilityId::new(", cursor) {
            let literal_at = skip_ws_and_comments(bytes, marker + "CapabilityId::new(".len());
            if let Some(literal) = read_string_literal(bytes, literal_at) {
                if literal.starts_with("tool.") {
                    ids.push(literal);
                }
            }
            cursor = marker + 1;
        }
        cursor = 0;
        while let Some(marker) = find_from(bytes, b"CAPABILITY_ID", cursor) {
            let mut at = skip_ws_and_comments(bytes, marker + "CAPABILITY_ID".len());
            if bytes.get(at) != Some(&b':') {
                cursor = marker + 1;
                continue;
            }
            at = skip_ws_and_comments(bytes, at + 1);
            if !bytes[at..].starts_with(b"&str") {
                cursor = marker + 1;
                continue;
            }
            at = skip_ws_and_comments(bytes, at + 4);
            if bytes.get(at) != Some(&b'=') {
                cursor = marker + 1;
                continue;
            }
            let literal_at = skip_ws_and_comments(bytes, at + 1);
            if let Some(literal) = read_string_literal(bytes, literal_at) {
                if literal.starts_with("tool.") {
                    ids.push(literal);
                }
            }
            cursor = marker + 1;
        }
        ids
    }

    /// **生产注册面**里真实注册的 `tool.*` 能力 id 全集。
    ///
    /// 判据来自**生产源码的注册点**（`CapabilityId::new("tool.…")` 与
    /// `*_CAPABILITY_ID: &str = "tool.…"` 两��写法），扫描范围覆盖全仓
    /// `crates/`（能力 id 由不同 crate 注册：`tools` / `runtime-assembly` /
    /// `organ` …），并排除夹具面：`tests/` 目录与 `#[cfg(test)]` 门控的代码段。
    fn production_tool_capability_ids() -> BTreeSet<String> {
        let crates_dir = repo_root().join("crates");
        assert!(
            crates_dir.is_dir(),
            "生产注册面扫描失败：{} 不是目录（仓根口径变了？）",
            crates_dir.display()
        );
        let mut sources = Vec::new();
        collect_production_sources(&crates_dir, &mut sources);
        assert!(
            !sources.is_empty(),
            "生产注册面扫描失败：{} 下没找到任何生产源文件",
            crates_dir.display()
        );
        let mut ids = BTreeSet::new();
        for path in sources {
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let production = strip_test_gated(&raw);
            for id in extract_tool_capability_ids(&production) {
                ids.insert(id);
            }
        }
        ids
    }

    /// 策展表键集合。
    fn curated_keys() -> BTreeSet<String> {
        TOOL_TAXONOMY
            .iter()
            .map(|entry| entry.key.to_string())
            .collect()
    }

    // ---------- 完整性闸门（两向，生产注册面派生） ----------

    #[test]
    fn every_curated_entry_maps_to_a_registered_production_capability_id() {
        // 幽灵条目闸门：表里每一条都必须对上一个**真实注册**的能力 id，
        // 或是显式登记的动态族根 / 库级保留面。表里凭空多出来的工具面在这里红。
        let registered = production_tool_capability_ids();
        let ghosts: Vec<&str> = TOOL_TAXONOMY
            .iter()
            .map(|entry| entry.key)
            .filter(|key| {
                !registered.contains(*key)
                    && !DYNAMIC_FAMILY_ROOTS.contains(key)
                    && !LIBRARY_LEVEL_FACES.contains(key)
            })
            .collect();
        assert!(
            ghosts.is_empty(),
            "策展表收了生产未注册的能力 id（幽灵条目）: {ghosts:?}"
        );
    }

    #[test]
    fn every_registered_production_tool_capability_id_has_a_curated_entry() {
        // 漏登闸门：生产注册面里的每个 `tool.*` id 都必须在表里（含动态族成员
        // 由族根覆盖）。新增真实工具面却没登记，在这里红。
        let registered = production_tool_capability_ids();
        assert!(
            !registered.is_empty(),
            "生产注册面扫描为空：扫描口径坏了，不能拿空集当「全绿」"
        );
        let curated = curated_keys();
        let missing: Vec<&String> = registered
            .iter()
            .filter(|id| {
                !curated.contains(*id)
                    && !DYNAMIC_FAMILY_ROOTS
                        .iter()
                        .any(|root| is_capability_in_family(id, root))
            })
            .collect();
        assert!(
            missing.is_empty(),
            "生产已注册但策展表漏登的能力 id: {missing:?}"
        );
    }

    #[test]
    fn the_dynamic_family_root_matches_the_production_id_shape() {
        // 动态族根不是写死的字面量：生产侧 id 由 mcp_bridge 的目录函数生成，
        // 这里用**生产函数**反查族根，防止族根与真实 id 形态漂移。
        let dynamic =
            apeireth_tools_canonical::mcp_bridge::catalog::capability_id_for("server", "probe")
                .expect("动态能力 id 合法");
        let dynamic_id = dynamic.as_str();
        assert!(
            DYNAMIC_FAMILY_ROOTS
                .iter()
                .any(|root| is_capability_in_family(dynamic_id, root)),
            "生产动态能力 id {dynamic_id} 不落在任何已登记族根下: {DYNAMIC_FAMILY_ROOTS:?}"
        );
        // 族根必须自身登记在表里（否则动态能力永远无法静态覆盖）。
        for root in DYNAMIC_FAMILY_ROOTS {
            assert!(
                taxonomy_entry_for(root).is_some(),
                "动态族根 {root} 未登记进策展表"
            );
        }
        // 族成员按前缀命中族根条目。
        let entry = taxonomy_entry_for(dynamic_id).expect("族成员应命中族根条目");
        assert_eq!(entry.key, "tool.mcp");
        assert_eq!(entry.source, ToolSource::Mcp);
    }

    #[test]
    fn library_level_faces_are_declared_and_not_registered_as_tools() {
        // 库级保留面：登记了但**必须**仍未注册为工具；一旦注册成真工具，
        // 本清单要同步删除（否则策展表拿库级面冒充工具面）。
        let registered = production_tool_capability_ids();
        for face in LIBRARY_LEVEL_FACES {
            assert!(
                !registered.contains(*face),
                "{face} 已经是注册工具，必须从库级保留面清单里移出"
            );
            assert!(
                taxonomy_entry_for(face).is_some(),
                "库级保留面 {face} 未登记进策展表"
            );
        }
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
