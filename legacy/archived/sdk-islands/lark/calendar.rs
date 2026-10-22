//! # lark 日历面 (事件列表 wire 契约 + 分页)
//!
//! 平台日历端点: `GET /calendar/v4/calendars/{calendar_id}/events`。
//!
//! ## wire 契约 (严格形状 + 未知字段容错)
//!
//! - 查询参数: `start_time` / `end_time` (RFC3339, 必填) + `page_size` (1..=1000)
//!   + `page_token` (可选, 翻页)。
//! - 响应 `data` = [`Page<CalendarEventWire>`]:
//!   `{"items": [...], "has_more": bool, "page_token": "..."}`。
//! - 事件条目 (严格必填): `summary` / `start_time` / `end_time` (RFC3339);
//!   其余字段可选, 未知字段忽略。
//!
//! ## 字段校验 (响应映射时完成)
//!
//! - `summary` 非空; `end_time > start_time` (RFC3339 解析失败 = 永久错误);
//! - `status` 未知取值 = 永久错误 (闭合枚举 [`EventStatus`], 缺省 `tentative`);
//! - `attendees` 每项走 K-1 #4 open_id 强校验。
//!
//! ## 分页
//!
//! 调用方 (客户端实现) 按 `has_more` / `page_token` 翻页合并,
//! 页数上限 [`MAX_EVENT_PAGES`] (防服务端 page_token 打转)。

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;
use crate::lark::http::{encode_path_segment, ApiRequest};

/// 单次 list 调用最多跟随的页数 (防 page_token 循环)。
pub const MAX_EVENT_PAGES: usize = 10;

// ============================================================================
// §1 EventStatus (5 variant 闭合枚举)
// ============================================================================

/// 日历事件状态 (5 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// 待定 (`tentative`)。
    #[default]
    Tentative,
    /// 已确认 (`confirmed`)。
    Confirmed,
    /// 已取消 (`cancelled`)。
    Cancelled,
    /// 已完成 (`completed`)。
    Completed,
    /// 已废弃 (`deprecated`)。
    Deprecated,
}

impl EventStatus {
    /// 5 状态 hardcode 常量。
    pub const COUNT: usize = 5;

    /// wire 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Tentative => "tentative",
            EventStatus::Confirmed => "confirmed",
            EventStatus::Cancelled => "cancelled",
            EventStatus::Completed => "completed",
            EventStatus::Deprecated => "deprecated",
        }
    }

    /// 从 wire 字符串解析 (未知值 → `None`, 调用方按永久错误处理)。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "tentative" => Some(EventStatus::Tentative),
            "confirmed" => Some(EventStatus::Confirmed),
            "cancelled" => Some(EventStatus::Cancelled),
            "completed" => Some(EventStatus::Completed),
            "deprecated" => Some(EventStatus::Deprecated),
            _ => None,
        }
    }
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 CalendarEvent (领域实体)
// ============================================================================

/// 日历事件 (领域实体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// 事件 ID (平台颁发后才有)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// 日历 ID (非空)。
    pub calendar_id: String,
    /// 事件标题 (非空)。
    pub summary: String,
    /// 事件描述 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 开始时间 (UTC)。
    pub start_time: DateTime<Utc>,
    /// 结束时间 (UTC, 必须 > start_time)。
    pub end_time: DateTime<Utc>,
    /// 时区 (IANA 名称, 默认 UTC)。
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// 全天事件 (默认 false)。
    #[serde(default)]
    pub is_all_day: bool,
    /// 事件状态 (闭合枚举)。
    #[serde(default)]
    pub status: EventStatus,
    /// 参与人 open_id 列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<String>,
    /// 会议链接 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_conference: Option<String>,
    /// 创建时间 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<SystemTime>,
    /// 最后修改时间 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<SystemTime>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl CalendarEvent {
    /// 创建新日历事件 (字段校验: calendar_id / summary 非空, end > start)。
    pub fn new(
        calendar_id: impl Into<String>,
        summary: impl Into<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Self, LarkError> {
        let calendar_id = calendar_id.into();
        let summary = summary.into();
        if calendar_id.is_empty() {
            return Err(LarkError::Other("calendar_id is empty".to_string()));
        }
        if summary.is_empty() {
            return Err(LarkError::Other("summary is empty".to_string()));
        }
        if end_time <= start_time {
            return Err(LarkError::Other(format!(
                "end_time {end_time} must be > start_time {start_time}"
            )));
        }
        Ok(Self {
            event_id: None,
            calendar_id,
            summary,
            description: None,
            start_time,
            end_time,
            timezone: default_timezone(),
            is_all_day: false,
            status: EventStatus::default(),
            attendees: Vec::new(),
            video_conference: None,
            created_at: None,
            updated_at: None,
        })
    }

    /// 字段校验 (calendar_id / summary / 时间区间 / attendees)。
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.calendar_id.is_empty() {
            return Err(LarkError::Other("calendar_id is empty".to_string()));
        }
        if self.summary.trim().is_empty() {
            return Err(LarkError::Other("summary is empty".to_string()));
        }
        if self.end_time <= self.start_time {
            return Err(LarkError::Other(
                "end_time must be > start_time".to_string(),
            ));
        }
        for attendee in &self.attendees {
            LarkError::validate_open_id(attendee)?;
        }
        Ok(())
    }
}

// ============================================================================
// §3 CalendarEventQuery (列表查询参数)
// ============================================================================

/// 日历事件查询参数。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEventQuery {
    /// 日历 ID。
    pub calendar_id: String,
    /// 起始时间 (UTC)。
    pub start_time: DateTime<Utc>,
    /// 结束时间 (UTC)。
    pub end_time: DateTime<Utc>,
    /// 单页最大返回数 (默认 50)。
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// 起始分页 token (可选; None = 从第一页开始)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

fn default_page_size() -> u32 {
    50
}

impl CalendarEventQuery {
    /// 查询参数校验 (calendar_id 非空 + 时间区间 + page_size 边界)。
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.calendar_id.trim().is_empty() {
            return Err(LarkError::Other("calendar_id is empty".to_string()));
        }
        if self.end_time <= self.start_time {
            return Err(LarkError::Other(
                "end_time must be > start_time".to_string(),
            ));
        }
        if self.page_size == 0 || self.page_size > crate::lark::MAX_CALENDAR_EVENTS_PER_PAGE {
            return Err(LarkError::Other(format!(
                "page_size must be 1..={}, got {}",
                crate::lark::MAX_CALENDAR_EVENTS_PER_PAGE,
                self.page_size
            )));
        }
        Ok(())
    }

    /// 指定翻页 token 的查询 (内部翻页循环用)。
    pub fn with_page_token(&self, page_token: Option<String>) -> Self {
        Self {
            page_token,
            ..self.clone()
        }
    }
}

// ============================================================================
// §4 FreeBusySlot (忙闲查询的领域形态)
// ============================================================================

/// 忙闲时间槽 (忙闲查询面的领域形态; 8 API 客户端不直接调用该查询面)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeBusySlot {
    /// 开始时间 (UTC)。
    pub start_time: DateTime<Utc>,
    /// 结束时间 (UTC)。
    pub end_time: DateTime<Utc>,
    /// 是否忙碌。
    pub is_busy: bool,
}

// ============================================================================
// §5 wire 契约 (请求构造 + 响应映射)
// ============================================================================

/// 列表请求 (`GET /calendar/v4/calendars/{calendar_id}/events`)。
pub fn build_list_request(
    query: &CalendarEventQuery,
    page_token: Option<&str>,
) -> Result<ApiRequest, LarkError> {
    query.validate()?;
    let path = format!(
        "/calendar/v4/calendars/{}/events",
        encode_path_segment(&query.calendar_id)
    );
    let mut req = ApiRequest::get(path)
        .with_query("start_time", to_rfc3339(query.start_time))
        .with_query("end_time", to_rfc3339(query.end_time))
        .with_query("page_size", query.page_size.to_string());
    if let Some(token) = page_token {
        if !token.is_empty() {
            req = req.with_query("page_token", token);
        }
    }
    Ok(req)
}

/// UTC 时间 → RFC3339 (秒精度, `Z` 后缀)。
pub fn to_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 事件条目 wire 形状 (严格必填 + 未知字段容错)。
#[derive(Debug, Clone, Deserialize)]
pub struct CalendarEventWire {
    /// 事件 ID (可选)。
    #[serde(default)]
    pub event_id: Option<String>,
    /// 日历 ID (可选; 缺省回落到查询的 calendar_id)。
    #[serde(default)]
    pub calendar_id: Option<String>,
    /// 标题 (必填)。
    pub summary: String,
    /// 描述 (可选)。
    #[serde(default)]
    pub description: Option<String>,
    /// 开始时间 (必填, RFC3339)。
    pub start_time: String,
    /// 结束时间 (必填, RFC3339)。
    pub end_time: String,
    /// 时区 (可选)。
    #[serde(default)]
    pub timezone: Option<String>,
    /// 全天事件 (可选)。
    #[serde(default)]
    pub is_all_day: Option<bool>,
    /// 状态 (可选; 未知取值 = 永久错误)。
    #[serde(default)]
    pub status: Option<String>,
    /// 参与人 open_id 列表 (可选)。
    #[serde(default)]
    pub attendees: Vec<String>,
    /// 会议链接 (可选)。
    #[serde(default)]
    pub video_conference: Option<String>,
    /// 创建时间 (可选, RFC3339)。
    #[serde(default)]
    pub created_at: Option<String>,
    /// 最后修改时间 (可选, RFC3339)。
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// wire → 领域 (RFC3339 严格解析 + 字段校验 + 未知状态值拒绝)。
pub fn map_event(
    fallback_calendar_id: &str,
    wire: &CalendarEventWire,
) -> Result<CalendarEvent, LarkError> {
    let start_time = parse_rfc3339(&wire.start_time, "start_time")?;
    let end_time = parse_rfc3339(&wire.end_time, "end_time")?;
    if end_time <= start_time {
        return Err(LarkError::Other(format!(
            "malformed event: end_time must be > start_time (summary={})",
            bounded_summary(&wire.summary)
        )));
    }
    let status = match &wire.status {
        Some(s) => EventStatus::parse(s).ok_or_else(|| {
            LarkError::Other(format!("malformed event: unknown status value '{s}'"))
        })?,
        None => EventStatus::default(),
    };
    for attendee in &wire.attendees {
        LarkError::validate_open_id(attendee)?;
    }
    let event = CalendarEvent {
        event_id: wire.event_id.clone(),
        calendar_id: wire
            .calendar_id
            .clone()
            .unwrap_or_else(|| fallback_calendar_id.to_string()),
        summary: wire.summary.clone(),
        description: wire.description.clone(),
        start_time,
        end_time,
        timezone: wire.timezone.clone().unwrap_or_else(default_timezone),
        is_all_day: wire.is_all_day.unwrap_or(false),
        status,
        attendees: wire.attendees.clone(),
        video_conference: wire.video_conference.clone(),
        created_at: wire
            .created_at
            .as_deref()
            .map(|s| parse_rfc3339(s, "created_at"))
            .transpose()?
            .map(|dt| dt.into()),
        updated_at: wire
            .updated_at
            .as_deref()
            .map(|s| parse_rfc3339(s, "updated_at"))
            .transpose()?
            .map(|dt| dt.into()),
    };
    event.validate()?;
    Ok(event)
}

/// RFC3339 严格解析 (失败 = 永久错误, 错误消息只带字段名与长度)。
fn parse_rfc3339(value: &str, field: &'static str) -> Result<DateTime<Utc>, LarkError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| {
            LarkError::Other(format!(
                "malformed event: {field} is not RFC3339 (len={})",
                value.len()
            ))
        })
}

fn bounded_summary(summary: &str) -> String {
    summary.chars().take(32).collect()
}

// ============================================================================
// §6 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn event_status_5_variants_round_trip() {
        assert_eq!(EventStatus::COUNT, 5);
        for status in [
            EventStatus::Tentative,
            EventStatus::Confirmed,
            EventStatus::Cancelled,
            EventStatus::Completed,
            EventStatus::Deprecated,
        ] {
            assert_eq!(EventStatus::parse(status.as_str()), Some(status));
        }
        assert_eq!(EventStatus::parse("bogus"), None);
    }

    #[test]
    fn calendar_event_creation_valid() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let event = CalendarEvent::new("cal_xxx", "团队周会", start, end).expect("valid");
        assert_eq!(event.calendar_id, "cal_xxx");
        assert_eq!(event.summary, "团队周会");
        assert_eq!(event.status, EventStatus::Tentative);
    }

    #[test]
    fn calendar_event_rejects_bad_input() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        assert!(CalendarEvent::new("", "title", start, end).is_err());
        assert!(CalendarEvent::new("cal_xxx", "", start, end).is_err());
        assert!(CalendarEvent::new("cal_xxx", "title", end, start).is_err());
    }

    #[test]
    fn calendar_event_validate_attendees() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let mut event = CalendarEvent::new("cal_xxx", "title", start, end).expect("valid");
        event.attendees = vec!["ou_valid_user_id".to_string()];
        assert!(event.validate().is_ok());
        event.attendees = vec!["invalid".to_string()];
        assert!(matches!(event.validate(), Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn calendar_event_query_validate() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 12, 0, 0, 0).unwrap();
        let q = CalendarEventQuery {
            calendar_id: "cal_xxx".to_string(),
            start_time: start,
            end_time: end,
            page_size: 50,
            page_token: None,
        };
        assert!(q.validate().is_ok());
        // page_size 边界: 0 与 > 上限都拒
        for bad in [0, crate::lark::MAX_CALENDAR_EVENTS_PER_PAGE + 1] {
            let q = CalendarEventQuery {
                page_size: bad,
                ..q.clone()
            };
            assert!(matches!(q.validate(), Err(LarkError::Other(_))));
        }
    }

    // ---- wire 契约 ----

    #[test]
    fn build_list_request_shapes_query_and_page_token() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let q = CalendarEventQuery {
            calendar_id: "cal_x".to_string(),
            start_time: start,
            end_time: end,
            page_size: 10,
            page_token: None,
        };
        let req = build_list_request(&q, Some("p2")).expect("request");
        assert_eq!(req.path, "/calendar/v4/calendars/cal_x/events");
        assert!(req
            .query
            .contains(&("start_time".into(), "2026-08-05T10:00:00Z".into())));
        assert!(req
            .query
            .contains(&("end_time".into(), "2026-08-05T11:00:00Z".into())));
        assert!(req.query.contains(&("page_size".into(), "10".into())));
        assert!(req.query.contains(&("page_token".into(), "p2".into())));

        // 无 token → 不带 page_token 参数
        let req = build_list_request(&q, None).expect("request");
        assert!(!req.query.iter().any(|(k, _)| k == "page_token"));
    }

    #[test]
    fn wire_event_maps_with_unknown_field_tolerance() {
        let wire: CalendarEventWire = serde_json::from_value(serde_json::json!({
            "event_id": "evt_1",
            "summary": "周会",
            "start_time": "2026-08-05T10:00:00Z",
            "end_time": "2026-08-05T11:00:00Z",
            "status": "confirmed",
            "attendees": ["ou_user1234567890abcdef"],
            "future_field": {"a": 1}
        }))
        .expect("parse");
        let event = map_event("cal_fallback", &wire).expect("map");
        assert_eq!(event.calendar_id, "cal_fallback", "缺 calendar_id 时回落");
        assert_eq!(event.event_id.as_deref(), Some("evt_1"));
        assert_eq!(event.status, EventStatus::Confirmed);
        assert_eq!(event.attendees.len(), 1);
    }

    #[test]
    fn wire_event_rejects_missing_required_fields() {
        // 缺 end_time → serde 拒
        let result = serde_json::from_value::<CalendarEventWire>(serde_json::json!({
            "summary": "x",
            "start_time": "2026-08-05T10:00:00Z"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn wire_event_rejects_bad_time_and_unknown_status() {
        let mk = |start: &str, end: &str, status: serde_json::Value| {
            serde_json::json!({
                "summary": "x",
                "start_time": start,
                "end_time": end,
                "status": status
            })
        };
        // 非 RFC3339 → 永久错误
        let wire: CalendarEventWire = serde_json::from_value(mk(
            "yesterday",
            "2026-08-05T11:00:00Z",
            serde_json::Value::Null,
        ))
        .expect("parse");
        assert!(matches!(map_event("cal", &wire), Err(LarkError::Other(_))));
        // end <= start → 永久错误
        let wire: CalendarEventWire = serde_json::from_value(mk(
            "2026-08-05T11:00:00Z",
            "2026-08-05T10:00:00Z",
            serde_json::Value::Null,
        ))
        .expect("parse");
        assert!(matches!(map_event("cal", &wire), Err(LarkError::Other(_))));
        // 未知状态值 → 永久错误 (闭合枚举)
        let wire: CalendarEventWire = serde_json::from_value(mk(
            "2026-08-05T10:00:00Z",
            "2026-08-05T11:00:00Z",
            serde_json::json!("exploded"),
        ))
        .expect("parse");
        assert!(matches!(map_event("cal", &wire), Err(LarkError::Other(_))));
    }

    #[test]
    fn wire_event_rejects_bad_attendee_open_id() {
        let wire: CalendarEventWire = serde_json::from_value(serde_json::json!({
            "summary": "x",
            "start_time": "2026-08-05T10:00:00Z",
            "end_time": "2026-08-05T11:00:00Z",
            "attendees": ["not-an-open-id"]
        }))
        .expect("parse");
        assert!(matches!(
            map_event("cal", &wire),
            Err(LarkError::OpenIdInvalid(_))
        ));
    }
}
