//! 技能库只读面 (`GET /v1/panel/skills`) —— 技能书架元数据的 HTTP 数据源。
//!
//! 数据面 = 组合根注入的 [`SkillShelfQuery`] 端口, 其适配实现读
//! `SkillLibrary::scan` (`<data_dir>/skills/*.json` 每卡一 StoredDoc, 拒开语义
//! 原样沿用: 坏 JSON / 串档 / 版本不符一律拒开并报结构化原因)。扫描是**一次
//! 快照读** (W2-③ 面板一致性): 好卡照读, 打不开的卡如实列进 `refused` 侧车 ——
//! 不砖化整个技能库, 也绝不「读坏用默认」。
//!
//! **不回卡体**: 卡体是 L2 封套面 (对话回合内按触发词披露的只读背景材料),
//! 读面只给元数据 (`id` / 卡名 / 摘要 / 触发词 / 来源 / 沉淀时刻 / 信封版本)。
//!
//! 响应信封 (稳定合同, 字段改名须同步 `docs/gateway-api-contract.md` §7d 与
//! `docs/02-guides/frontend-data-contract.md` §10.8):
//! `{ "skills": [SkillMetaDto], "count": n, "refused": [SkillRefusalDto] }`。

use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::panels::GatewayState;

/// 技能卡来源投影 (`{kind, title}`, 冻结词表 `document|book|article`)。
#[derive(Debug, Clone, Serialize)]
pub struct SkillSourceDto {
    /// 来源种类 (内容输入三类, 冻结词表)。
    pub kind: String,
    /// 来源标题。
    pub title: String,
}

/// 技能卡元数据行 (读面只给元数据, **不回卡体**)。
#[derive(Debug, Clone, Serialize)]
pub struct SkillMetaDto {
    /// 稳定身份 (`skill-{FNV-1a 64 hex}`, 卡名确定性派生, 版本演进不变)。
    pub id: String,
    /// 卡名 (技能库内唯一键)。
    pub name: String,
    /// 一句话摘要。
    pub summary: String,
    /// 触发词 (入库时已归一)。
    pub triggers: Vec<String>,
    /// 来源 (种类 + 标题)。
    pub source: SkillSourceDto,
    /// 首次沉淀时刻 (epoch 毫秒, 版本演进不变)。
    pub created_at: u64,
    /// StoredDoc 信封格式版本 (拒开语义的版本戳, 当前恒 `1`)。
    pub version: u32,
}

/// 一张被拒开的卡 (拒开语义的如实回报; 不砖库, 也不静默跳过)。
#[derive(Debug, Clone, Serialize)]
pub struct SkillRefusalDto {
    /// 被拒开的卡文件名 (只回技能库内文件名, 不回磁盘绝对路径)。
    pub file: String,
    /// 拒开原因 (结构化错误的显示文本)。
    pub reason: String,
}

/// 一次扫描的响应信封 (元数据 + 拒开侧车)。
#[derive(Debug, Clone, Serialize)]
pub struct SkillShelfScanDto {
    /// 打开成功的卡元数据 (确定性排序: 卡名键 + id)。
    pub skills: Vec<SkillMetaDto>,
    /// `skills` 行数 (拒开数另计, 不混入)。
    pub count: usize,
    /// 被拒开的卡 (路径序); 坏卡如实列拒, 好卡照常上屏。
    pub refused: Vec<SkillRefusalDto>,
}

/// 技能库只读投影端口 (面板族 bounded-context 端口; 适配实现在组合根,
/// 读 `SkillLibrary::scan` —— 扫描不可失败: 读不到目录 = 空库, 坏卡 = 列拒)。
#[async_trait]
pub trait SkillShelfQuery: Send + Sync {
    /// 一次快照读: 元数据行 + 拒开侧车 (同一刻, 不拼两次读)。
    async fn list_skills(&self) -> Result<SkillShelfScanDto, String>;
}

/// `GET /v1/panel/skills` —— 技能库元数据列表 (只读; 过令牌门, 读面全员可读)。
///
/// 未接线 (端口未注入) = 501 `unsupported` 如实报, 不假装有数据源。
pub async fn list_skills(State(state): State<GatewayState>) -> Response {
    let Some(skills) = &state.services.skills else {
        return crate::panels::unsupported("skills.read");
    };
    match skills.list_skills().await {
        Ok(scan) => (StatusCode::OK, Json(scan)).into_response(),
        Err(message) => crate::panels::panel_error(message),
    }
}
