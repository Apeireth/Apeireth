//! V5-b 暴露面体检报告 (安全守望 · 守望的产品化): 「帮你看清你暴露了什么」。
//!
//! 防御视角的**本地只读自查面** (自查暴露面, 不做任何枚举/追踪他人的操作):
//!
//! | 体检项 | id | 测什么 |
//! |---|---|---|
//! | 凭据文件分布 | `credential_files` | 本地数据面的凭据/密钥面路径分布 (sensitive_path 族词表) |
//! | 明文风险点 | `plaintext_points` | 可读配置件里密钥字段的明文计数 (值零采集) |
//! | 开放端口面 | `listening_ports` | 本机监听枚举 (受控系统表快照) |
//! | 备份包排除项核对 | `backup_exclusions` | 备份包里「凭据不随行」是否真的成立 |
//! | 客户端令牌表绑定完整性 | `token_bindings` | W3-② 令牌↔用户绑定一致性 (判定真源在装载面) |
//! | 外网暴露面 | `external_surface` | **未测** (需外部探针, 超出本地自查范围) |
//!
//! 三条硬语义: ①**只读零写** (体检不写被测面任何字节, 测试钉死); ②**诚实口径**
//! (只测能测的本地面, 不能测的显式「未测」, 未测不计入总评、不冒充干净);
//! ③**零明文留痕** (凭据值/密钥值永不出现在报告里, 只报路径词面与计数)。
//!
//! 周期巡检接缝: 独立只读工具 [`ExposureAuditTool`] (`tool.exposure_audit`, 只读档)
//! 与 CLI `apeireth exposure report` 共用同一引擎 [`run_exposure_audit`] —— 同一
//! 内核两个出口, 体检可按周期重跑、报告可留档可 diff (暴露面收敛可度量)。

pub mod backup_surface;
pub mod binding_surface;
pub mod credential_surface;
/// V5-c 守望巡检漏斗 (筛查→精查→复核→报告 + 成本闸/断点续查/任务接缝):
/// 本体检面的巡检化编排层, 只读消费本面六项速扫, 不改本面任何口径。
pub mod funnel;
pub mod listener_surface;
pub mod report;
pub mod tool;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backup::{read_backup_ledger, BACKUP_LEDGER_FILE_NAME};

pub use backup_surface::backup_exclusion_item;
pub use binding_surface::{token_binding_item, BindingSurface};
pub use credential_surface::{scan_credential_files, scan_plaintext_points};
pub use listener_surface::{
    listener_item, parse_netstat_snapshot, probe_netstat, Listener, ListenerScope, ListenerSnapshot,
};
pub use report::{
    format_utc_ms, ExposureItem, ExposureReport, Finding, ItemStatus, OverallScore, RiskLevel,
    REPORT_SCHEMA, REPORT_SCHEMA_VERSION,
};
pub use tool::{ExposureAuditTool, EXPOSURE_AUDIT_CAPABILITY_ID, EXPOSURE_AUDIT_TOOL_NAME};

/// 本机监听枚举的探测口 (受控采集; 测试注入快照, 生产注入 `netstat` 受控采集)。
pub type ListenerProbe = Arc<dyn Fn() -> Result<ListenerSnapshot, String> + Send + Sync>;

/// W3-② 绑定一致性的探测口 (判定真源在装载面, 本面只消费判定结果)。
pub type BindingProbe = Arc<dyn Fn() -> BindingSurface + Send + Sync>;

/// 暴露面体检的注入配置 (数据根 + 备份包 + 两个探测口)。
#[derive(Clone)]
pub struct ExposureAuditConfig {
    /// 体检数据根 (本地数据面)。
    pub data_root: PathBuf,
    /// 显式备份包 (`--bundle`); 缺省回落到「最近备份」记录里的包。
    pub bundle: Option<PathBuf>,
    /// 本机监听枚举探测口。
    pub listeners: ListenerProbe,
    /// 绑定一致性探测口。
    pub bindings: BindingProbe,
}

impl std::fmt::Debug for ExposureAuditConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExposureAuditConfig")
            .field("data_root", &self.data_root)
            .field("bundle", &self.bundle)
            .field("listeners", &"configured")
            .field("bindings", &"configured")
            .finish()
    }
}

impl ExposureAuditConfig {
    /// 缺省配置: 生产 `netstat` 受控采集 + 绑定探测口占位 (占位必报: 占位在
    /// 体检项里显式喊「未测」, 不做假旋钮)。
    pub fn new(data_root: impl Into<PathBuf>) -> Self {
        Self {
            data_root: data_root.into(),
            bundle: None,
            listeners: Arc::new(|| probe_netstat().map(|text| parse_netstat_snapshot(&text))),
            bindings: Arc::new(|| BindingSurface::Absent {
                reason: "未注入 W3-② 绑定探测口 (装配面接线归口后才可测)".to_string(),
            }),
        }
    }

    /// 指定备份包。
    pub fn with_bundle(mut self, bundle: impl Into<PathBuf>) -> Self {
        self.bundle = Some(bundle.into());
        self
    }

    /// 注入本机监听枚举探测口。
    pub fn with_listeners(mut self, probe: ListenerProbe) -> Self {
        self.listeners = probe;
        self
    }

    /// 注入绑定一致性探测口。
    pub fn with_bindings(mut self, probe: BindingProbe) -> Self {
        self.bindings = probe;
        self
    }
}

/// 跑一次暴露面体检 (只读零写; 报告是唯一产出)。
pub fn run_exposure_audit(config: &ExposureAuditConfig) -> ExposureReport {
    run_exposure_audit_at(config, now_millis())
}

/// 显式生成时刻的体检 (确定性测试口; 语义与 [`run_exposure_audit`] 逐字一致)。
pub fn run_exposure_audit_at(config: &ExposureAuditConfig, generated_at_ms: i64) -> ExposureReport {
    let bundle = resolve_bundle(config);
    let items = vec![
        scan_credential_files(&config.data_root),
        scan_plaintext_points(&config.data_root),
        listener_item((config.listeners)()),
        backup_exclusion_item(bundle.as_deref()),
        token_binding_item(&(config.bindings)()),
        external_surface_item(),
    ];
    ExposureReport::new(generated_at_ms, &config.data_root, items)
}

/// 项六: 外网暴露面 —— **未测** (诚实口径: 公网入口可达性要外部探针, 本地
/// 只读自查测不了就明说测不了)。
fn external_surface_item() -> ExposureItem {
    ExposureItem::not_measured(
        "external_surface",
        "外网暴露面 (公网入口可达性)",
        "外网暴露面需从外部网络探测 (公网入口/端口可达性), 超出本地只读自查范围, 本报告显式未测",
        "用外部探针自查公网入口; 本地监听面收窄 (默认回环) 是第一道收口",
    )
}

/// 备份包解析: 显式指定优先, 否则取「最近备份」记录里的包 (取不到 = 无包)。
fn resolve_bundle(config: &ExposureAuditConfig) -> Option<PathBuf> {
    if let Some(bundle) = &config.bundle {
        return Some(bundle.clone());
    }
    let ledger_path = config.data_root.join(BACKUP_LEDGER_FILE_NAME);
    let ledger = read_backup_ledger(&ledger_path).ok().flatten()?;
    let bundle = PathBuf::from(&ledger.bundle);
    bundle.is_dir().then_some(bundle)
}

/// 当前时刻 (epoch 毫秒; 回拨钳 0, 不编时间)。
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_config(root: &Path) -> ExposureAuditConfig {
        ExposureAuditConfig::new(root)
            .with_listeners(Arc::new(|| {
                Ok(ListenerSnapshot {
                    listeners: Vec::new(),
                    unparsed_rows: 0,
                    duplicate_rows: 0,
                })
            }))
            .with_bindings(Arc::new(|| BindingSurface::Checked {
                clients: 1,
                users: 1,
                problems: Vec::new(),
            }))
    }

    #[test]
    fn full_report_runs_all_six_items_in_fixed_order() {
        let dir = tempfile::tempdir().expect("temp dir");
        let report = run_exposure_audit_at(&clean_config(dir.path()), 0);
        let ids: Vec<&str> = report.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "credential_files",
                "plaintext_points",
                "listening_ports",
                "backup_exclusions",
                "token_bindings",
                "external_surface",
            ]
        );
        assert_eq!(report.overall.measured_items, 4);
        assert_eq!(
            report.overall.not_measured_items, 2,
            "备份包缺席 + 外网面都显式未测"
        );
    }

    #[test]
    fn clean_surface_scores_100_and_external_stays_unmeasured() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("users.json"), br#"{"users":[]}"#).unwrap();
        let report = run_exposure_audit_at(&clean_config(dir.path()), 0);
        assert_eq!(report.overall.score, 100);
        assert_eq!(report.overall.grade, "优");
        let external = report
            .items
            .iter()
            .find(|item| item.id == "external_surface")
            .expect("external item");
        assert_eq!(external.status, ItemStatus::NotMeasured);
        assert!(external.note.as_deref().unwrap().contains("未测"));
    }
}
