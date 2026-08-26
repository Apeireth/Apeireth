//! 关系轨迹 —— 完整关系历史

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::milestone::Milestone;
use crate::partner::PartnerId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub milestone: Milestone,
    pub at: DateTime<Utc>,
}

/// 关系轨迹 —— 一个伙伴的全部里程碑
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    partner_id: PartnerId,
    entries: Vec<TimelineEntry>,
}

impl Timeline {
    pub fn new(partner_id: PartnerId) -> Self {
        Self {
            partner_id,
            entries: Vec::new(),
        }
    }

    pub fn partner_id(&self) -> PartnerId {
        self.partner_id
    }

    pub fn append(&mut self, milestone: Milestone) {
        let at = milestone.at();
        self.entries.push(TimelineEntry { milestone, at });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entries(&self) -> &[TimelineEntry] {
        &self.entries
    }

    pub fn iter(&self) -> std::slice::Iter<'_, TimelineEntry> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::milestone::{MilestoneKind, MilestonePayload};

    #[test]
    fn timeline_starts_empty() {
        let tl = Timeline::new(PartnerId::new());
        assert!(tl.is_empty());
        assert_eq!(tl.len(), 0);
    }

    #[test]
    fn timeline_appends() {
        let mut tl = Timeline::new(PartnerId::new());
        tl.append(Milestone::new(
            MilestoneKind::FirstMeeting,
            MilestonePayload::Text("hi".into()),
        ));
        tl.append(Milestone::new(
            MilestoneKind::Decision,
            MilestonePayload::Text("x".into()),
        ));
        assert_eq!(tl.len(), 2);
    }
}
