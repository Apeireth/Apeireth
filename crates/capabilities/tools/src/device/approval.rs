//! IM 设备审批卡的两本账 (推送账 + 按钮决定账) —— 推送幂等 / 失败留账 /
//! 一计划一决定的真值账 (K 轴 L3/L4 · 设备审批卡→IM 推送链)。
//!
//! 背景 (设备审批卡接入 IM 推送链): [`super::device_control::DeviceApprovalCard`]
//! 的卡载荷要真的推到 IM 渠道 (irreversible / money 档动作的审批卡到手机)。
//! 推送是**外联动作**, 会失败; 两本账就是这件事的诚实痕迹:
//!
//! - **推送账** (`device-approval-pushes.jsonl`): 推送**幂等键 = 动作 plan_id**
//!   ([`DeviceApprovalPushLedger::pushed_plan_ids`] 是真值幂等集, 同计划不重复
//!   推送); **失败留账** (带真因) 待重试 —— IM 失败不丢卡, 卡载荷 (含执行所需
//!   真参数) 永远在推送账行里, 重试只凭账本即可重建出站;
//! - **决定账** (`device-approval-decisions.jsonl`): 按钮回调的**批准/拒绝**
//!   逐次落账 (`decided_by` = IM 用户名 + 渠道标注, 如实署名) —— **一计划一
//!   决定**: 已拍板的计划再点拒绝即帧 (卡是一次性拍板面, 不是多次开关)。
//!
//! 两本账都 **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写
//! 不删行; 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。本面只记**推送
//! 痕迹与人的决定**, 不记设备动作语义 —— 执行语义全在
//! [`super::device_control::DeviceActionLedger`] (状态账), 零第二套。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use serde::{Deserialize, Serialize};

use super::device_control::DeviceApprovalCard;

// ---------------------------------------------------------------------------
// 推送账 (device-approval-pushes.jsonl)
// ---------------------------------------------------------------------------

/// 推送账文件 (数据目录落点: `<data_dir>/device-approval-pushes.jsonl`)。
pub const DEVICE_APPROVAL_PUSH_LEDGER_FILE: &str = "device-approval-pushes.jsonl";

/// 推送账 StoredDoc 的身份戳。
pub const DEVICE_APPROVAL_PUSH_DOC_NAME: &str = "device-approval-push";

/// 推送账 StoredDoc 的格式版本戳。
pub const DEVICE_APPROVAL_PUSH_DOC_VERSION: u32 = 1;

/// 一次推送的结果 (两态闭合词表: `pushed` / `failed`, 无第三套词表)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceApprovalPushOutcome {
    /// 已送达 (幂等键从此生效: 同计划不再推)。
    Pushed,
    /// 推送失败 (留账待重试; 卡载荷仍在账本行里, 不丢)。
    Failed,
}

impl DeviceApprovalPushOutcome {
    /// 稳定标签 (`pushed` / `failed`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pushed => "pushed",
            Self::Failed => "failed",
        }
    }

    /// 闭合词表解析 (词表外 = `None`, 调用方照实报未知)。
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "pushed" => Some(Self::Pushed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 推送账的一行 (append-only 的行主体: 一次推送尝试的痕迹 + 卡载荷)。
///
/// 卡载荷 ([`DeviceApprovalCard`]) 随行留痕: 重试只凭本账即可重建出站信封,
/// 按钮回调执行所需的动作参数也只认这里的真参数 (卡载荷自足)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApprovalPushRecord {
    /// 被推送的审批卡载荷 (计划身份 = [`DeviceApprovalCard::plan_id`],
    /// **推送幂等键**)。
    pub card: DeviceApprovalCard,
    /// 推送到的 IM 渠道 id。
    pub channel_id: String,
    /// 推送到的 IM 会话 id。
    pub conversation_id: String,
    /// 推送结果 ([`DeviceApprovalPushOutcome`])。
    pub outcome: DeviceApprovalPushOutcome,
    /// 对端回执引用 (`pushed` 时在场; 也是 money 档执行证据的 `im_card_id`
    /// 来源 —— IM 审批卡标识 = 送达回执)。
    pub message_ref: Option<String>,
    /// 失败真因 (`failed` 时在场, 如实)。
    pub reason: Option<String>,
    /// 推送时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

impl DeviceApprovalPushRecord {
    /// 推送幂等键 (动作 plan_id)。
    pub fn plan_id(&self) -> &str {
        &self.card.plan_id
    }
}

/// 设备审批卡推送账 (数据目录 `device-approval-pushes.jsonl`)。
///
/// **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写、不删行;
/// 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceApprovalPushLedger {
    path: PathBuf,
}

impl DeviceApprovalPushLedger {
    /// 数据目录落点 (`<data_dir>/device-approval-pushes.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(DEVICE_APPROVAL_PUSH_LEDGER_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 账本路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, record: &DeviceApprovalPushRecord) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("推送账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            DEVICE_APPROVAL_PUSH_DOC_NAME,
            DEVICE_APPROVAL_PUSH_DOC_VERSION,
            vec![DEVICE_APPROVAL_PUSH_DOC_VERSION],
            record,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("推送账序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("推送账打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("推送账写入失败 {}: {error}", self.path.display()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<DeviceApprovalPushRecord>, String> {
        scan_records::<DeviceApprovalPushRecord>(&self.path, &push_doc_compat())
            .map_err(|error| format!("推送账读取失败 {}: {error}", self.path.display()))
    }

    /// 幂等键集合 (**推送幂等键 = 动作 plan_id**): 已成功送达 (`pushed`) 的计划 id。
    ///
    /// `failed` 行不入集合 —— 失败留账待重试, 重试成功才进幂等集。
    pub fn pushed_plan_ids(&self) -> Result<BTreeSet<String>, String> {
        let scan = self.scan()?;
        Ok(scan
            .records
            .iter()
            .filter(|record| record.body.outcome == DeviceApprovalPushOutcome::Pushed)
            .map(|record| record.body.plan_id().to_string())
            .collect())
    }

    /// 某计划**最近一次成功送达**的推送行 (送达回执 = money 档 `im_card_id` 证据)。
    ///
    /// 只认 `pushed` 行 (卡真的到了手机才有回执); 查无 = `None` (调用方照实
    /// 拒绝即帧, 不冒充送达)。
    pub fn delivered_record_for(
        &self,
        plan_id: &str,
    ) -> Result<Option<DeviceApprovalPushRecord>, String> {
        let scan = self.scan()?;
        Ok(scan
            .records
            .iter()
            .rev()
            .find(|record| {
                record.body.plan_id() == plan_id
                    && record.body.outcome == DeviceApprovalPushOutcome::Pushed
            })
            .map(|record| record.body.clone()))
    }

    /// 失败留账待重试的卡 (每计划取最新一行载荷): 从未送达的计划 = 其最新一行
    /// 是 `failed` (且该计划不在幂等集)。重试凭行内卡载荷重建出站, 不需外部记忆。
    pub fn pending_retry(&self) -> Result<Vec<DeviceApprovalPushRecord>, String> {
        let scan = self.scan()?;
        let pushed = self.pushed_plan_ids()?;
        let mut latest: BTreeMap<String, DeviceApprovalPushRecord> = BTreeMap::new();
        for record in &scan.records {
            latest.insert(record.body.plan_id().to_string(), record.body.clone());
        }
        Ok(latest
            .into_iter()
            .filter(|(plan_id, record)| {
                !pushed.contains(plan_id) && record.outcome == DeviceApprovalPushOutcome::Failed
            })
            .map(|(_, record)| record)
            .collect())
    }
}

/// 推送账读端契约 (仅当前格式版本可读)。
fn push_doc_compat() -> DocCompat {
    DocCompat::exact(
        DEVICE_APPROVAL_PUSH_DOC_NAME,
        DEVICE_APPROVAL_PUSH_DOC_VERSION,
    )
}

// ---------------------------------------------------------------------------
// 决定账 (device-approval-decisions.jsonl)
// ---------------------------------------------------------------------------

/// 决定账文件 (数据目录落点: `<data_dir>/device-approval-decisions.jsonl`)。
pub const DEVICE_APPROVAL_DECISION_LEDGER_FILE: &str = "device-approval-decisions.jsonl";

/// 决定账 StoredDoc 的身份戳。
pub const DEVICE_APPROVAL_DECISION_DOC_NAME: &str = "device-approval-decision";

/// 决定账 StoredDoc 的格式版本戳。
pub const DEVICE_APPROVAL_DECISION_DOC_VERSION: u32 = 1;

/// 一次按钮拍板 (两态闭合词表: `approved` / `rejected`, 无第三套词表)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceActionDecision {
    /// 批准 (批准即执行: 执行语义另落设备动作状态账)。
    Approved,
    /// 拒绝 (不执行; 一计划一决定, 拒绝即关账)。
    Rejected,
}

impl DeviceActionDecision {
    /// 稳定标签 (`approved` / `rejected`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    /// 闭合词表解析 (词表外 = `None`, 调用方照实报未知)。
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// 决定账的一行 (append-only 的行主体: 谁在何时拍了什么板)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApprovalDecisionRecord {
    /// 被拍板的计划 id (与审批卡闭合身份同值)。
    pub plan_id: String,
    /// 拍板结果 ([`DeviceActionDecision`])。
    pub decision: DeviceActionDecision,
    /// 谁批 (IM 用户名 + 渠道标注, 如实署名; 缺人不落账)。
    pub decided_by: String,
    /// IM 审批卡标识 (送达回执; money 档执行证据同源)。
    pub im_card_id: Option<String>,
    /// 拍板时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

/// 设备审批卡按钮决定账 (数据目录 `device-approval-decisions.jsonl`)。
///
/// **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写、不删行;
/// 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。**一计划一决定**:
/// [`DeviceApprovalDecisionLedger::decision_for`] 是真值闭合面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceApprovalDecisionLedger {
    path: PathBuf,
}

impl DeviceApprovalDecisionLedger {
    /// 数据目录落点 (`<data_dir>/device-approval-decisions.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(DEVICE_APPROVAL_DECISION_LEDGER_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 账本路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, record: &DeviceApprovalDecisionRecord) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("决定账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            DEVICE_APPROVAL_DECISION_DOC_NAME,
            DEVICE_APPROVAL_DECISION_DOC_VERSION,
            vec![DEVICE_APPROVAL_DECISION_DOC_VERSION],
            record,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("决定账序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("决定账打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("决定账写入失败 {}: {error}", self.path.display()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<DeviceApprovalDecisionRecord>, String> {
        scan_records::<DeviceApprovalDecisionRecord>(&self.path, &decision_doc_compat())
            .map_err(|error| format!("决定账读取失败 {}: {error}", self.path.display()))
    }

    /// 一计划一决定的真值闭合面: 该计划已拍的板 (首次落账的决定)。
    ///
    /// `None` = 还没人拍板 (卡还开着); 已拍板 = 再点拒绝即帧, 不翻案。
    pub fn decision_for(&self, plan_id: &str) -> Result<Option<DeviceActionDecision>, String> {
        let scan = self.scan()?;
        Ok(scan
            .records
            .iter()
            .find(|record| record.body.plan_id == plan_id)
            .map(|record| record.body.decision))
    }
}

/// 决定账读端契约 (仅当前格式版本可读)。
fn decision_doc_compat() -> DocCompat {
    DocCompat::exact(
        DEVICE_APPROVAL_DECISION_DOC_NAME,
        DEVICE_APPROVAL_DECISION_DOC_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::device_control::ActionGrade;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-approval-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn card(plan_id: &str) -> DeviceApprovalCard {
        DeviceApprovalCard {
            plan_id: plan_id.to_string(),
            device: "shop".to_string(),
            device_name: "商店".to_string(),
            device_action: "purchase".to_string(),
            grade: ActionGrade::Money.as_str().to_string(),
            params: serde_json::json!({"item": "牛奶", "amount": 12.5}),
            effects: vec!["cart: 空 → 牛奶 x1".to_string()],
            requires_im_card: true,
            approval_note: crate::device::device_control::approval_note_for(ActionGrade::Money)
                .to_string(),
        }
    }

    fn push(plan_id: &str, outcome: DeviceApprovalPushOutcome) -> DeviceApprovalPushRecord {
        DeviceApprovalPushRecord {
            card: card(plan_id),
            channel_id: "primary".to_string(),
            conversation_id: "oc_demo_chat".to_string(),
            outcome,
            message_ref: matches!(outcome, DeviceApprovalPushOutcome::Pushed)
                .then(|| "m-out".to_string()),
            reason: matches!(outcome, DeviceApprovalPushOutcome::Failed)
                .then(|| "delivery failed: 503".to_string()),
            at_ms: 1_700_000_000_000,
        }
    }

    /// 推送账 append-only + 幂等键 = 动作 plan_id (只有 `pushed` 行入幂等集)。
    #[test]
    fn the_push_ledger_is_append_only_and_keyed_by_plan_id() {
        let dir = temp_dir("push");
        let ledger = DeviceApprovalPushLedger::under_data_dir(&dir);
        ledger
            .append(&push("plan-1", DeviceApprovalPushOutcome::Failed))
            .unwrap();
        assert!(
            ledger.pushed_plan_ids().unwrap().is_empty(),
            "失败留账不算已送达 (要重试)"
        );
        let prefix = std::fs::read(ledger.path()).unwrap();
        ledger
            .append(&push("plan-1", DeviceApprovalPushOutcome::Pushed))
            .unwrap();
        // 同计划再来一行 pushed 也不重复计数 (幂等键 = plan_id)。
        ledger
            .append(&push("plan-1", DeviceApprovalPushOutcome::Pushed))
            .unwrap();
        ledger
            .append(&push("plan-2", DeviceApprovalPushOutcome::Pushed))
            .unwrap();
        let whole = std::fs::read(ledger.path()).unwrap();
        assert!(
            whole.starts_with(&prefix),
            "append-only: 旧字节前缀逐字节不变"
        );
        let ids = ledger.pushed_plan_ids().unwrap();
        assert_eq!(ids.len(), 2, "去重 plan_id 计数");
        assert!(ids.contains("plan-1") && ids.contains("plan-2"));
        let delivered = ledger
            .delivered_record_for("plan-1")
            .unwrap()
            .expect("送达行");
        assert_eq!(delivered.message_ref.as_deref(), Some("m-out"));
        assert_eq!(delivered.card.params["amount"], serde_json::json!(12.5));
        assert_eq!(ledger.delivered_record_for("plan-9").unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 失败留账待重试: 从未送达的计划凭账本行载荷重推; 已送达的不进重试集。
    #[test]
    fn pending_retry_picks_only_plants_that_never_reached_the_phone() {
        let dir = temp_dir("retry");
        let ledger = DeviceApprovalPushLedger::under_data_dir(&dir);
        ledger
            .append(&push("plan-1", DeviceApprovalPushOutcome::Failed))
            .unwrap();
        let retryable = ledger.pending_retry().unwrap();
        assert_eq!(retryable.len(), 1);
        assert_eq!(retryable[0].plan_id(), "plan-1", "失败留账 = 待重试");
        ledger
            .append(&push("plan-1", DeviceApprovalPushOutcome::Pushed))
            .unwrap();
        ledger
            .append(&push("plan-2", DeviceApprovalPushOutcome::Pushed))
            .unwrap();
        assert!(
            ledger.pending_retry().unwrap().is_empty(),
            "送达过的计划不再进重试集"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 决定账: 一计划一决定 (首次拍板为真值) + append-only + 坏行跳过如实计。
    #[test]
    fn the_decision_ledger_seals_one_decision_per_plan() {
        use std::io::Write as _;
        let dir = temp_dir("decision");
        let ledger = DeviceApprovalDecisionLedger::under_data_dir(&dir);
        ledger
            .append(&DeviceApprovalDecisionRecord {
                plan_id: "plan-1".to_string(),
                decision: DeviceActionDecision::Rejected,
                decided_by: "im:u_demo@primary".to_string(),
                im_card_id: Some("m-out".to_string()),
                at_ms: 1_700_000_000_000,
            })
            .unwrap();
        ledger
            .append(&DeviceApprovalDecisionRecord {
                plan_id: "plan-1".to_string(),
                decision: DeviceActionDecision::Approved,
                decided_by: "im:u_other@primary".to_string(),
                im_card_id: None,
                at_ms: 1_700_000_000_100,
            })
            .unwrap();
        assert_eq!(
            ledger.decision_for("plan-1").unwrap(),
            Some(DeviceActionDecision::Rejected),
            "一计划一决定: 首次拍板为真值, 后行不翻案"
        );
        assert_eq!(ledger.decision_for("plan-2").unwrap(), None);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(ledger.path())
            .unwrap();
        file.write_all(b"{ not json\n").unwrap();
        drop(file);
        let scan = ledger.scan().unwrap();
        assert_eq!(scan.records.len(), 2);
        assert_eq!(scan.skipped, 1, "坏行如实计数, 不砖化整本");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 两本账的词表都闭合 (词表外 = None, 不造第三套词)。
    #[test]
    fn both_outcome_vocabularies_are_closed() {
        assert_eq!(DeviceApprovalPushOutcome::Pushed.as_str(), "pushed");
        assert_eq!(DeviceApprovalPushOutcome::Failed.as_str(), "failed");
        assert_eq!(
            DeviceApprovalPushOutcome::parse("retried"),
            None,
            "推送词表闭合"
        );
        assert_eq!(DeviceActionDecision::Approved.as_str(), "approved");
        assert_eq!(DeviceActionDecision::Rejected.as_str(), "rejected");
        assert_eq!(
            DeviceActionDecision::parse("cancelled"),
            None,
            "决定词表闭合"
        );
    }
}
