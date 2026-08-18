//! # Lark 日历 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书日历 `calendar/v4/calendars/{calendar_id}/events` API 翻译源.
//! 4 实体之一: `CalendarEvent`.
//!
//! **3 核心 API** (per v0.9.21 商业版):
//! - `list_calendar_events` — 列出 calendar_id 下的 events
//! - `create_calendar_event` — 创建 event (R21 续真接)
//! - `get_freebusy` — 查询用户忙闲
//!
//! **当前 STUB**: 字段保留 1:1 翻译, 走 `list_calendar_events` 返 `NotImplemented`.
//!
//! ## 4 CalendarEvent 字段守门
//!
//! - `event_id` (UUID, R21 真接飞书后才有)
//! - `calendar_id` (per `calendar_id` 字段)
//! - `summary` (per `summary` 字段, 非空)
//! - `start_time` / `end_time` (per `start_time` / `end_time` 字段, `chrono::DateTime<Utc>`)

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;

// ============================================================================
// §1 EventStatus (5 variant, 1:1 翻译 v0.9.21 商业版)
// ============================================================================

/// 日历事件状态 (5 variant, per v0.9.21 商业版 `status` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// 待处理 (per v0.9.21 商业版 `status: "tentative"`).
    #[default]
    Tentative,
    /// 已确认 (per v0.9.21 商业版 `status: "confirmed"`).
    Confirmed,
    /// 已取消 (per v0.9.21 商业版 `status: "cancelled"`).
    Cancelled,
    /// 已完成 (per v0.9.21 商业版 `status: "completed"`, R21 续真接).
    Completed,
    /// 已废弃 (per v0.9.21 商业版 `status: "deprecated"`).
    Deprecated,
}

impl EventStatus {
    /// 5 状态 hardcode 常量.
    pub const COUNT: usize = 5;

    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Tentative => "tentative",
            EventStatus::Confirmed => "confirmed",
            EventStatus::Cancelled => "cancelled",
            EventStatus::Completed => "completed",
            EventStatus::Deprecated => "deprecated",
        }
    }
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 CalendarEvent (per v0.9.21 商业版 calendar event 1:1)
// ============================================================================

/// 日历事件 (per v0.9.21 商业版 `calendar/v4/calendars/{calendar_id}/events/[event_id]` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// 事件 ID (per `event_id` 字段, R21 真接飞书后才有, STUB 模式 None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// 日历 ID (per `calendar_id` 字段, 非空).
    pub calendar_id: String,
    /// 事件标题 (per `summary` 字段, 非空).
    pub summary: String,
    /// 事件描述 (per `description` 字段, 可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 开始时间 (per `start_time` 字段, RFC3339 字符串).
    pub start_time: DateTime<Utc>,
    /// 结束时间 (per `end_time` 字段, RFC3339 字符串, 必须 > start_time).
    pub end_time: DateTime<Utc>,
    /// 时区 (per `timezone` 字段, IANA 格式 e.g. "Asia/Shanghai", 默认 UTC).
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// 全天事件 (per `is_all_day` 字段, 默认 false).
    #[serde(default)]
    pub is_all_day: bool,
    /// 事件状态 (per `status` 字段, 5 variant).
    #[serde(default)]
    pub status: EventStatus,
    /// 参与人 open_id 列表 (per `attendees` 字段).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<String>,
    /// 会议链接 (per `video_conference` 字段, 飞书会中链接).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_conference: Option<String>,
    /// 创建时间 (per `created_at` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<SystemTime>,
    /// 最后修改时间 (per `updated_at` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<SystemTime>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl CalendarEvent {
    /// 创建新日历事件 (STUB 模式不真调飞书 API).
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

    /// 校验 4 字段 (K-1 强校验守门: calendar_id / summary / start_time / end_time).
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.calendar_id.is_empty() {
            return Err(LarkError::Other("calendar_id is empty".to_string()));
        }
        if self.summary.is_empty() {
            return Err(LarkError::Other("summary is empty".to_string()));
        }
        if self.end_time <= self.start_time {
            return Err(LarkError::Other(format!("end_time must be > start_time")));
        }
        for attendee in &self.attendees {
            LarkError::validate_open_id(attendee)?;
        }
        Ok(())
    }
}

// ============================================================================
// §3 CalendarEventQuery (per list_calendar_events 1:1)
// ============================================================================

/// 日历事件查询参数 (per v0.9.21 商业版 list events query 1:1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEventQuery {
    /// 日历 ID (per `calendar_id` 字段).
    pub calendar_id: String,
    /// 起始时间 (per `start_time` 字段).
    pub start_time: DateTime<Utc>,
    /// 结束时间 (per `end_time` 字段).
    pub end_time: DateTime<Utc>,
    /// 最大返回数 (per `page_size` 字段, 默认 50).
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// 分页 token (per `page_token` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

fn default_page_size() -> u32 {
    50
}

impl CalendarEventQuery {
    /// 校验查询参数 (4 字段).
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.calendar_id.is_empty() {
            return Err(LarkError::Other("calendar_id is empty".to_string()));
        }
        if self.end_time <= self.start_time {
            return Err(LarkError::Other(
                "end_time must be > start_time".to_string(),
            ));
        }
        if self.page_size == 0 || self.page_size > 1000 {
            return Err(LarkError::Other(format!(
                "page_size must be 1..=1000, got {}",
                self.page_size
            )));
        }
        Ok(())
    }
}

// ============================================================================
// §4 FreeBusySlot (per freebusy API 1:1)
// ============================================================================

/// 忙闲时间槽 (per v0.9.21 商业版 freebusy 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeBusySlot {
    /// 开始时间 (per `start_time` 字段).
    pub start_time: DateTime<Utc>,
    /// 结束时间 (per `end_time` 字段).
    pub end_time: DateTime<Utc>,
    /// 是否忙碌 (per `is_busy` 字段).
    pub is_busy: bool,
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn event_status_5_variants() {
        assert_eq!(EventStatus::COUNT, 5);
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
    fn calendar_event_reject_empty_calendar_id() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let result = CalendarEvent::new("", "title", start, end);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn calendar_event_reject_empty_summary() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let result = CalendarEvent::new("cal_xxx", "", start, end);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn calendar_event_reject_invalid_time_range() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let result = CalendarEvent::new("cal_xxx", "title", start, end);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn calendar_event_validate_attendees() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let mut event = CalendarEvent::new("cal_xxx", "title", start, end).expect("valid");
        event.attendees = vec!["ou_valid_user_id".to_string()];
        assert!(event.validate().is_ok());
    }

    #[test]
    fn calendar_event_validate_rejects_bad_attendee() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 5, 11, 0, 0).unwrap();
        let mut event = CalendarEvent::new("cal_xxx", "title", start, end).expect("valid");
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
    }

    #[test]
    fn calendar_event_query_reject_invalid_page_size() {
        let start = Utc.with_ymd_and_hms(2026, 8, 5, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 12, 0, 0, 0).unwrap();
        let q = CalendarEventQuery {
            calendar_id: "cal_xxx".to_string(),
            start_time: start,
            end_time: end,
            page_size: 0,
            page_token: None,
        };
        assert!(matches!(q.validate(), Err(LarkError::Other(_))));
    }
}
