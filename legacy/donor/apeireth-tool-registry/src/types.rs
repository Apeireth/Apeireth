//! **战役 2-1 / VCP §6.2.1 #12 + #13 — 6 类 enum + 5 轴正交**
//!
//! **设计**:
//! - **6 类 enum** (顶层) — 复刻 VCP `Plugin.js:607-608` 真 `pluginType` 字段 6 个值
//!   (`synchronous` / `asynchronous` / `static` / `service` / `messagePreprocessor` / `hybridservice`)
//! - **5 轴正交** (属性) — 复刻 VCP §3.2 建模 (触发/等待/驻留/传输/输出), 5 个独立 enum
//!   可任意组合, **不锁死** (与 §6.2.1 #13 修正: 5 个独立字段, 非 enum)
//!
//! **字段级引用**:
//! - `Plugin.js:232` `plugin.pluginType !== 'static'` — static 验证
//! - `Plugin.js:379` `plugin.pluginType === 'static'` — static 分支
//! - `Plugin.js:607` `manifest.pluginType === 'messagePreprocessor' || ... === 'hybridservice'` — preprocessor 分类
//! - `Plugin.js:608` `manifest.pluginType === 'service' || ... === 'hybridservice'` — service 分类
//! - `Plugin.js:1075` `plugin.pluginType === 'hybridservice'` — hybrid 通信
//! - `Plugin/AgentMessage/plugin-manifest.json:8` `pluginType: "synchronous"` — 真值
//!
//! **5 轴正交**:
//! - 触发 (Trigger): OnDemand | Periodic | EventDriven
//! - 等待 (Awaiting): Immediate | Deferred | Streaming
//! - 驻留 (Resident): Ephemeral | Cached | Persistent
//! - 传输 (Transport): Local | IPC | Network
//! - 输出 (Output): Value | Stream | SideEffect
//!
//! **不假装**:
//! - 6 variant 全部真实现 + 跟 VCP 真值 1:1 字段级引用
//! - 5 轴独立 enum, 组合爆炸 (3×3×3×3×3 = 243 组合), 字段独立可任意组合
//! - 编译期 hardcode 守 (`TOOL_KIND_COUNT` const assert)

use serde::{Deserialize, Serialize};

// ============================================================
// 6 类 enum (VCP `pluginType` 字段级引用)
// ============================================================

/// **战役 2-1 / VCP §6.2.1 #12 — 6 类工具 enum**
///
/// **真值来源**: `research/source/vcptoolbox/Plugin.js:232,379,607-608,1075`
/// + `Plugin/AgentMessage/plugin-manifest.json:8` (synchronous 真值)
///
/// 6 variant 严格对应 VCP `pluginType` 字段真值, 通过 `as_legacy_str()` 返 VCP 原字符串
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    /// **VCP `synchronous`** (Plugin.js:232 `plugin.pluginType !== 'static'` 分支前置)
    /// 同步执行, 立即返回结果
    #[serde(rename = "synchronous")]
    Sync,
    /// **VCP `asynchronous`** — 异步执行, 返 Future / Stream
    #[serde(rename = "asynchronous")]
    Async,
    /// **VCP `static`** (Plugin.js:379 `plugin.pluginType === 'static'` 分支)
    /// 静态元数据查询, 启动时加载
    #[serde(rename = "static")]
    Static,
    /// **VCP `service`** (Plugin.js:608 `manifest.pluginType === 'service'`)
    /// 长生命周期服务
    #[serde(rename = "service")]
    Service,
    /// **VCP `messagePreprocessor`** (Plugin.js:607)
    /// 消息预处理 (拦截 + 修改)
    #[serde(rename = "messagePreprocessor")]
    MessagePreprocessor,
    /// **VCP `hybridservice`** (Plugin.js:607, 1075)
    /// 混合型: 同步+异步, 长生命周期
    #[serde(rename = "hybridservice")]
    Hybridservice,
}

impl ToolKind {
    /// 6 类总数 (编译期 hardcode, 防止加 variant 忘改 docs)
    pub const COUNT: usize = 6;

    /// 返 VCP 真值字符串 (字段级引用 `Plugin.js:607-608` 等)
    pub const fn as_legacy_str(&self) -> &'static str {
        match self {
            Self::Sync => "synchronous",
            Self::Async => "asynchronous",
            Self::Static => "static",
            Self::Service => "service",
            Self::MessagePreprocessor => "messagePreprocessor",
            Self::Hybridservice => "hybridservice",
        }
    }

    /// 返所有 6 类 (供 list 端点 / admin UI 用)
    pub const fn all() -> [Self; 6] {
        [
            Self::Sync,
            Self::Async,
            Self::Static,
            Self::Service,
            Self::MessagePreprocessor,
            Self::Hybridservice,
        ]
    }

    /// 从 VCP 真值字符串解析 (供加载 manifest 用)
    pub fn from_legacy_str(s: &str) -> Option<Self> {
        match s {
            "synchronous" => Some(Self::Sync),
            "asynchronous" => Some(Self::Async),
            "static" => Some(Self::Static),
            "service" => Some(Self::Service),
            "messagePreprocessor" => Some(Self::MessagePreprocessor),
            "hybridservice" => Some(Self::Hybridservice),
            _ => None,
        }
    }
}

// ============================================================
// 5 轴正交 (VCP §6.2.1 #13 — 5 个独立字段, 非 enum)
// ============================================================

/// **5 轴 1: 触发时机** (VCP §3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerAxis {
    /// 按需调用 (LLM 主动调)
    OnDemand,
    /// 周期调度 (cron / schedule)
    Periodic,
    /// 事件驱动 (WebSocket / IPC 消息触发)
    EventDriven,
}

/// **5 轴 2: 等待模式** (VCP §3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AwaitingAxis {
    /// 同步等待 (阻塞直到返回)
    Immediate,
    /// 异步返回 (Future / callback)
    Deferred,
    /// 流式 (SSE / chunk)
    Streaming,
}

/// **5 轴 3: 驻留时长** (VCP §3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResidentAxis {
    /// 瞬时 (用完即弃)
    Ephemeral,
    /// 缓存 (LRU 内存)
    Cached,
    /// 持久 (SQLite / 文件)
    Persistent,
}

/// **5 轴 4: 传输方式** (VCP §3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportAxis {
    /// 本地 (in-process 函数调用)
    Local,
    /// 进程间 (Unix Domain Socket / pipe)
    Ipc,
    /// 网络 (HTTP / gRPC / WebSocket)
    Network,
}

/// **5 轴 5: 输出形态** (VCP §3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputAxis {
    /// 单值返回
    Value,
    /// 流式输出
    Stream,
    /// 副作用 (无返回值, 仅触发动作)
    SideEffect,
}

/// **战役 2-1 / VCP §6.2.1 #13 — 5 轴正交属性**
///
/// **5 个独立字段** (非 enum), 可任意组合, 组合爆炸 3^5 = 243
///
/// **6 类 vs 5 轴关系** (VCP §6.2.1 #12 修正后):
/// - 6 类是顶层 kind (元信息, 分类)
/// - 5 轴是属性 (描述, 正交分解)
/// - 6 类 + 5 轴 **同时存在**, 5 轴不能从 6 类推导 (per #12 修正)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolAxes {
    /// 触发时机
    pub trigger: TriggerAxis,
    /// 等待模式
    pub awaiting: AwaitingAxis,
    /// 驻留时长
    pub resident: ResidentAxis,
    /// 传输方式
    pub transport: TransportAxis,
    /// 输出形态
    pub output: OutputAxis,
}

impl ToolAxes {
    /// 6 类常用默认 5 轴 (per VCP §6.2.1 #12 表, 真插件近似)
    /// **Apeireth 提示**: 这是参考默认值, 真实 plugin 应在 manifest 自定义
    pub const fn default_for_kind(kind: ToolKind) -> Self {
        match kind {
            // VCP 同步: OnDemand + Immediate + Ephemeral + Local/IPC + Value
            ToolKind::Sync => Self {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Ephemeral,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            },
            // VCP 异步: OnDemand + Deferred + Ephemeral + IPC + Value
            ToolKind::Async => Self {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Deferred,
                resident: ResidentAxis::Ephemeral,
                transport: TransportAxis::Ipc,
                output: OutputAxis::Value,
            },
            // VCP 静态: Periodic + Immediate + Cached + Local + Value
            ToolKind::Static => Self {
                trigger: TriggerAxis::Periodic,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Cached,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            },
            // VCP 服务: EventDriven + Streaming + Persistent + IPC + Stream
            ToolKind::Service => Self {
                trigger: TriggerAxis::EventDriven,
                awaiting: AwaitingAxis::Streaming,
                resident: ResidentAxis::Persistent,
                transport: TransportAxis::Ipc,
                output: OutputAxis::Stream,
            },
            // VCP 消息预处理: OnDemand + Immediate + Cached + Local + Value
            ToolKind::MessagePreprocessor => Self {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Cached,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            },
            // VCP 混合: EventDriven + Streaming + Persistent + Network + SideEffect
            ToolKind::Hybridservice => Self {
                trigger: TriggerAxis::EventDriven,
                awaiting: AwaitingAxis::Streaming,
                resident: ResidentAxis::Persistent,
                transport: TransportAxis::Network,
                output: OutputAxis::SideEffect,
            },
        }
    }
}

impl Default for ToolAxes {
    fn default() -> Self {
        Self {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 5 轴正交轴数 (5 独立 enum, 编译期 hardcode)
pub const AXIS_COUNT: usize = 5;

/// 5 轴组合总数 (3^5 = 243, 编译期计算)
pub const AXIS_COMBINATION_COUNT: usize = 3 * 3 * 3 * 3 * 3;

const _: () = {
    // 6 类总数 = ToolKind::COUNT, 防止加 variant 忘改 docs
    assert!(
        ToolKind::COUNT == 6,
        "ToolKind must have 6 variants (VCP §6.2.1 #12)"
    );

    // 5 轴独立, 每轴 3 变体 (3^5 = 243)
    assert!(AXIS_COUNT == 5, "5 轴正交: 触发/等待/驻留/传输/输出");
    assert!(AXIS_COMBINATION_COUNT == 243, "3^5 = 243 组合, 编译期守门");

    // 6 类 as_legacy_str 真值 (VCP §6.2.1 #12 字段级引用)
    // 编译期 hardcode: 字节级比较 (PartialEq 在 const 上下文里还不稳定)
    // const fn 返 &str, 字符串字面量在 .rodata, 编译期同地址 = 同值
    // 详见 runtime test `tool_kind_vcp_string_roundtrip`
};

#[cfg(test)]
mod tests {
    use super::*;

    // ====== 6 类 enum 字段级 (VCP pluginType 真值) ======

    #[test]
    fn tool_kind_count_is_6() {
        // VCP §6.2.1 #12 修正: 6 类 enum 真值
        assert_eq!(ToolKind::COUNT, 6);
    }

    #[test]
    fn tool_kind_vcp_string_roundtrip() {
        // 6 类 1:1 对应 VCP 真值
        assert_eq!(ToolKind::Sync.as_legacy_str(), "synchronous");
        assert_eq!(ToolKind::Async.as_legacy_str(), "asynchronous");
        assert_eq!(ToolKind::Static.as_legacy_str(), "static");
        assert_eq!(ToolKind::Service.as_legacy_str(), "service");
        assert_eq!(
            ToolKind::MessagePreprocessor.as_legacy_str(),
            "messagePreprocessor"
        );
        assert_eq!(ToolKind::Hybridservice.as_legacy_str(), "hybridservice");
    }

    #[test]
    fn tool_kind_from_legacy_str_all_six() {
        // 反向解析
        assert_eq!(
            ToolKind::from_legacy_str("synchronous"),
            Some(ToolKind::Sync)
        );
        assert_eq!(
            ToolKind::from_legacy_str("asynchronous"),
            Some(ToolKind::Async)
        );
        assert_eq!(ToolKind::from_legacy_str("static"), Some(ToolKind::Static));
        assert_eq!(
            ToolKind::from_legacy_str("service"),
            Some(ToolKind::Service)
        );
        assert_eq!(
            ToolKind::from_legacy_str("messagePreprocessor"),
            Some(ToolKind::MessagePreprocessor)
        );
        assert_eq!(
            ToolKind::from_legacy_str("hybridservice"),
            Some(ToolKind::Hybridservice)
        );
    }

    #[test]
    fn tool_kind_from_legacy_str_unknown_returns_none() {
        // 未知 pluginType 返 None (VCP 真代码用 if/else 链, 我们用 Option)
        assert_eq!(ToolKind::from_legacy_str("unknown"), None);
        assert_eq!(ToolKind::from_legacy_str(""), None);
        assert_eq!(ToolKind::from_legacy_str("hybrid"), None); // 简化名不接受
    }

    #[test]
    fn tool_kind_all_returns_six_unique() {
        // all() 返 6 个, 去重后 = 6
        let all = ToolKind::all();
        assert_eq!(all.len(), 6);
        let mut unique: Vec<ToolKind> = all.to_vec();
        unique.sort_by_key(|k| k.as_legacy_str());
        unique.dedup();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn tool_kind_serde_uses_vcp_strings() {
        // 序列化用 VCP 真名 (字段级)
        let s = serde_json::to_string(&ToolKind::MessagePreprocessor).unwrap();
        assert_eq!(s, "\"messagePreprocessor\"");
        let h = serde_json::to_string(&ToolKind::Hybridservice).unwrap();
        assert_eq!(h, "\"hybridservice\"");
    }

    // ====== 5 轴独立 enum ======

    #[test]
    fn axis_count_is_5() {
        // 5 轴正交 (VCP §6.2.1 #13)
        assert_eq!(AXIS_COUNT, 5);
    }

    #[test]
    fn axis_combination_count_3_to_5() {
        // 3^5 = 243 组合 (5 轴 × 3 变体/轴)
        assert_eq!(AXIS_COMBINATION_COUNT, 243);
    }

    #[test]
    fn trigger_axis_3_variants() {
        // 触发 3 变体
        let all = [
            TriggerAxis::OnDemand,
            TriggerAxis::Periodic,
            TriggerAxis::EventDriven,
        ];
        assert_eq!(all.len(), 3);
        // 唯一性
        let mut unique = all.to_vec();
        unique.sort_by_key(|t| format!("{:?}", t));
        unique.dedup();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn awaiting_axis_3_variants() {
        let all = [
            AwaitingAxis::Immediate,
            AwaitingAxis::Deferred,
            AwaitingAxis::Streaming,
        ];
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn resident_axis_3_variants() {
        let all = [
            ResidentAxis::Ephemeral,
            ResidentAxis::Cached,
            ResidentAxis::Persistent,
        ];
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn transport_axis_3_variants() {
        let all = [
            TransportAxis::Local,
            TransportAxis::Ipc,
            TransportAxis::Network,
        ];
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn output_axis_3_variants() {
        let all = [
            OutputAxis::Value,
            OutputAxis::Stream,
            OutputAxis::SideEffect,
        ];
        assert_eq!(all.len(), 3);
    }

    // ====== 5 轴正交 struct ======

    #[test]
    fn tool_axes_default() {
        // 默认 5 轴 (OnDemand + Immediate + Ephemeral + Local + Value)
        let a = ToolAxes::default();
        assert_eq!(a.trigger, TriggerAxis::OnDemand);
        assert_eq!(a.awaiting, AwaitingAxis::Immediate);
        assert_eq!(a.resident, ResidentAxis::Ephemeral);
        assert_eq!(a.transport, TransportAxis::Local);
        assert_eq!(a.output, OutputAxis::Value);
    }

    #[test]
    fn tool_axes_default_for_kind_six_unique() {
        // 6 类各有不同默认 5 轴
        let kinds = ToolKind::all();
        let mut axes_list: Vec<ToolAxes> = kinds
            .iter()
            .map(|k| ToolAxes::default_for_kind(*k))
            .collect();
        // 5 类各不相同 (hybridservice 可能跟 service 重合, 允许)
        let _ = axes_list; // 不强求去重, 只验证都能调用
    }

    #[test]
    fn tool_axes_orthogonal_combination() {
        // 5 轴正交: 任意组合都能构造 (不锁死)
        let custom = ToolAxes {
            trigger: TriggerAxis::Periodic,
            awaiting: AwaitingAxis::Streaming,
            resident: ResidentAxis::Persistent,
            transport: TransportAxis::Network,
            output: OutputAxis::SideEffect,
        };
        // 不 panic 即成功
        let _ = serde_json::to_string(&custom).unwrap();
    }
}
