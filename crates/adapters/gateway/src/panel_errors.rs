//! 面板族错误帧的分类与渲染。
//!
//! 从 `panels.rs` 拆出（2026-10-25）：尺寸棘轮不给 panels.rs 余量，而错误分类
//! 本身是**独立职责**（谁产生错误 vs 怎么把错误说清楚）—— 拆分是真拆，
//! 不是为凑行数的位移。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// 面板族运行期错误帧; 技能库等旁路面板模块同用。
///
/// # 为什么不是「一律 500」
///
/// 这些端口（`MemoryGovernanceCommand` / `MemoryCommand` / 任务板 …）的
/// `Err(String)` 里混着**两类完全不同的失败**：
///
/// 1. **客户端输入问题**（乐观并发冲突、目标不存在、参数非法）—— 用户改一下
///    就能成功，回 500 + 「重启 Companion 后重试」是**误导**（重启治不了
///    revision 冲突）；
/// 2. **真正的运行期故障** —— 回 500 正确。
///
/// 实测（2026-10-25 真机 HTTP）：过期的 `expected_rev` 提交返回
/// `500 runtime_error` + 「重启 Companion 后重试」，而正确语义是
/// **`409 Conflict`** + 「重新读取最新 revision 后再提交」。
///
/// # 判定依据（窄、稳定、可审计）
///
/// 只认**适配器自己写死的前缀短语**（不是给任意文本做模糊匹配）：
/// `revision conflict`（`memory_governance` / `session_lifecycle` /
/// persona 三处同族文案）。认不出的一律仍走 500 —— **宁严不宽**：
/// 把真故障误报成 4xx 比反过来更危险（用户会以为改参数就能好）。
pub(crate) fn panel_error(message: String) -> Response {
    if message.contains("revision conflict") {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": {
                "code": "revision_conflict",
                "message": message,
                "solution": "这条数据已被改动：重新读取最新 revision 后再提交（不是故障，不需重启）"
            } })),
        )
            .into_response();
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": { "code": "runtime_error", "message": message, "solution": "重启 Companion 后重试, 若复现请提交日志" } })),
    )
        .into_response()
}

#[cfg(test)]
mod panel_error_classification_tests {
    use super::panel_error;
    use axum::http::StatusCode;

    /// 乐观并发冲突 = **客户端可修复**问题 ⇒ 409，不是 500 + 「重启」。
    ///
    /// 真机实测（2026-10-25）：过期 `expected_rev` 提交曾返回
    /// `500 runtime_error` + 「重启 Companion 后重试」。
    #[tokio::test]
    async fn revision_conflict_maps_to_409() {
        for message in [
            "episode `e1` revision conflict: expected 0, actual 1".to_string(),
            "session `s1` revision conflict: expected 2, actual 3".to_string(),
            "persona profile revision conflict: expected 1, current 2".to_string(),
        ] {
            let response = panel_error(message);
            assert_eq!(
                response.status(),
                StatusCode::CONFLICT,
                "revision conflict 必须是 409"
            );
        }
    }

    /// 认不出的错误**仍**走 500 —— 宁严不宽（把真故障报成 4xx 更危险）。
    #[tokio::test]
    async fn unknown_errors_stay_500() {
        for message in [
            "database is locked".to_string(),
            // 只认 `revision conflict` 这个稳定短语；单独出现 conflict 不算
            "writer conflict on channel".to_string(),
        ] {
            let response = panel_error(message);
            assert_eq!(
                response.status(),
                StatusCode::INTERNAL_SERVER_ERROR,
                "非并发冲突类错误不得被降级成 4xx"
            );
        }
    }
}
