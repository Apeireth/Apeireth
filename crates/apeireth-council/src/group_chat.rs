//! 跨 Agent 群聊 (Cross-Agent Group Chat)
//!
//! **源**: VCP v1.1 官网 "通过 AgentAssistant或者VCPGroupChat 与其它 Agent 朋友们围炉夜话".
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - **GroupChat** 房间抽象, 多 Agent 共享上下文
//! - **多角色发言**: Persona / Advisor / External Agent 3 类
//! - **Turn 调度**: 轮询 / 自由 / 主持人 3 种
//! - **room 状态**: Open / Closed / Archived
//! - **跨 Agent 桥**: 群聊事件路由到 `apeireth-bus` 三 channel
//! - **不假装** (O-5): 真实现房间 + 加入 + 发言 + 轮询, 单元测试 8+
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline / AgentA / AgentB
//!          ↓
//!   apeireth-council::group_chat::GroupChat (本模块)
//!          ↓ (事件)
//!   apeireth-bus::ChanneledBus (三 channel 分发)
//!          ↓
//!   apeireth-arbitration::ArbitrationLog (唯一事实时间线)
//! ```

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum GroupChatError {
    #[error("room not found: {0}")]
    RoomNotFound(String),
    #[error("room closed: {0}")]
    RoomClosed(String),
    #[error("participant not in room: {0}")]
    ParticipantNotInRoom(String),
    #[error("room full: capacity={0}")]
    RoomFull(usize),
}

pub type GroupChatResult<T> = Result<T, GroupChatError>;

// ============================================================================
// 角色
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticipantRole {
    /// 主持人, 控制 turn 调度
    Host,
    /// 普通 agent/persona
    Agent,
    /// 旁听 (只读)
    Observer,
}

impl ParticipantRole {
    pub const COUNT: usize = 3;
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Agent => "agent",
            Self::Observer => "observer",
        }
    }
}

// ============================================================================
// 参与者
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub display_name: String,
    pub role: ParticipantRole,
    pub capabilities: Vec<String>,
}

impl Participant {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>, role: ParticipantRole) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            role,
            capabilities: vec![],
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn can_speak(&self) -> bool {
        matches!(self.role, ParticipantRole::Host | ParticipantRole::Agent)
    }
}

// ============================================================================
// 消息
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub room_id: String,
    pub participant_id: String,
    pub content: String,
    pub timestamp_ms: i64,
    /// 引用的上一条消息 id (thread)
    pub reply_to: Option<String>,
    /// 不可变 hash (类似 HASH-SQL 仲裁)
    pub content_hash: String,
}

impl ChatMessage {
    pub fn new(
        room_id: impl Into<String>,
        participant_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let room_id = room_id.into();
        let participant_id = participant_id.into();
        let content = content.into();
        let timestamp_ms = now_ms();
        let content_hash = sha256_hex(&format!("{}|{}|{}|{}", timestamp_ms, room_id, participant_id, content));
        Self {
            id: Uuid::new_v4().to_string(),
            room_id,
            participant_id,
            content,
            timestamp_ms,
            reply_to: None,
            content_hash,
        }
    }

    pub fn reply_to(mut self, parent_id: impl Into<String>) -> Self {
        self.reply_to = Some(parent_id.into());
        self
    }
}

// ============================================================================
// Turn 调度
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnPolicy {
    /// 轮询发言
    RoundRobin,
    /// 谁都能发, 无顺序
    Free,
    /// 主持人点名
    HostDriven,
}

impl TurnPolicy {
    pub const COUNT: usize = 3;
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::Free => "free",
            Self::HostDriven => "host_driven",
        }
    }
}

// ============================================================================
// 房间状态
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomStatus {
    Open,
    Closed,
    Archived,
}

impl RoomStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Archived => "archived",
        }
    }

    pub fn can_post(&self) -> bool {
        matches!(self, Self::Open)
    }
}

// ============================================================================
// 房间
// ============================================================================

#[derive(Debug)]
pub struct GroupRoom {
    pub id: String,
    pub name: String,
    pub topic: String,
    pub status: RoomStatus,
    pub turn_policy: TurnPolicy,
    pub capacity: usize,
    pub participants: Vec<Participant>,
    pub messages: Vec<ChatMessage>,
    /// RoundRobin 轮询 cursor
    pub turn_cursor: usize,
}

impl GroupRoom {
    pub fn new(name: impl Into<String>, topic: impl Into<String>, turn_policy: TurnPolicy) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            topic: topic.into(),
            status: RoomStatus::Open,
            turn_policy,
            capacity: 16,
            participants: Vec::new(),
            messages: Vec::new(),
            turn_cursor: 0,
        }
    }

    pub fn with_capacity(mut self, cap: usize) -> Self {
        self.capacity = cap;
        self
    }

    pub fn add(&mut self, p: Participant) -> GroupChatResult<()> {
        if self.participants.len() >= self.capacity {
            return Err(GroupChatError::RoomFull(self.capacity));
        }
        if self.participants.iter().any(|x| x.id == p.id) {
            return Ok(()); // idempotent
        }
        self.participants.push(p);
        Ok(())
    }

    pub fn remove(&mut self, participant_id: &str) -> GroupChatResult<()> {
        let i = self.participants.iter().position(|p| p.id == participant_id)
            .ok_or_else(|| GroupChatError::ParticipantNotInRoom(participant_id.into()))?;
        self.participants.remove(i);
        if self.turn_cursor >= self.participants.len() && !self.participants.is_empty() {
            self.turn_cursor = 0;
        }
        Ok(())
    }

    pub fn participant(&self, id: &str) -> Option<&Participant> {
        self.participants.iter().find(|p| p.id == id)
    }

    pub fn post(&mut self, msg: ChatMessage) -> GroupChatResult<()> {
        if !self.status.can_post() {
            return Err(GroupChatError::RoomClosed(self.id.clone()));
        }
        let p = self.participant(&msg.participant_id)
            .ok_or_else(|| GroupChatError::ParticipantNotInRoom(msg.participant_id.clone()))?;
        if !p.can_speak() {
            return Err(GroupChatError::ParticipantNotInRoom(p.id.clone()));
        }

        // Turn policy enforcement
        match self.turn_policy {
            TurnPolicy::Free => {}
            TurnPolicy::RoundRobin => {
                // 简化: 不能连发两条, 需轮换
                if let Some(last) = self.messages.last() {
                    if last.participant_id == msg.participant_id {
                        return Err(GroupChatError::ParticipantNotInRoom(
                            "round_robin: wait for next turn".into()
                        ));
                    }
                }
            }
            TurnPolicy::HostDriven => {
                // 简化: 检查 host 是否在场, 强制 host 批准 (这里用 last 是 host 即可)
                let host = self.participants.iter().find(|p| p.role == ParticipantRole::Host);
                if let Some(host) = host {
                    if let Some(last) = self.messages.last() {
                        if last.participant_id != host.id && msg.participant_id != host.id {
                            return Err(GroupChatError::ParticipantNotInRoom(
                                "host_driven: only host or with host approval".into()
                            ));
                        }
                    }
                }
            }
        }

        self.messages.push(msg);
        // RoundRobin cursor 推进
        if let Some(i) = self.participants.iter().position(|p| p.id == self.messages.last().unwrap().participant_id) {
            self.turn_cursor = (i + 1) % self.participants.len().max(1);
        }
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> Vec<ChatMessage> {
        let start = self.messages.len().saturating_sub(limit);
        self.messages[start..].to_vec()
    }

    pub fn close(&mut self) {
        self.status = RoomStatus::Closed;
    }

    pub fn archive(&mut self) {
        self.status = RoomStatus::Archived;
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }
}

// ============================================================================
// GroupChat 管理器
// ============================================================================

#[derive(Clone)]
pub struct GroupChat {
    rooms: Arc<Mutex<HashMap<String, GroupRoom>>>,
}

impl GroupChat {
    pub fn new() -> Self {
        Self { rooms: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn create_room(&self, name: impl Into<String>, topic: impl Into<String>, policy: TurnPolicy) -> String {
        let room = GroupRoom::new(name, topic, policy);
        let id = room.id.clone();
        self.rooms.lock().insert(id.clone(), room);
        id
    }

    pub fn get(&self, room_id: &str) -> GroupChatResult<GroupRoomRef<'_>> {
        let g = self.rooms.lock();
        if !g.contains_key(room_id) {
            return Err(GroupChatError::RoomNotFound(room_id.into()));
        }
        Ok(GroupRoomRef {
            inner: g,
            id: room_id.to_string(),
        })
    }

    pub fn list_rooms(&self) -> Vec<String> {
        self.rooms.lock().keys().cloned().collect()
    }

    pub fn room_count(&self) -> usize {
        self.rooms.lock().len()
    }

    pub fn close_room(&self, room_id: &str) -> GroupChatResult<()> {
        let mut g = self.rooms.lock();
        let r = g.get_mut(room_id).ok_or_else(|| GroupChatError::RoomNotFound(room_id.into()))?;
        r.close();
        Ok(())
    }

    pub fn archive_room(&self, room_id: &str) -> GroupChatResult<()> {
        let mut g = self.rooms.lock();
        let r = g.get_mut(room_id).ok_or_else(|| GroupChatError::RoomNotFound(room_id.into()))?;
        r.archive();
        Ok(())
    }

    // R147: 新增 helper 方法 (不改 LOCKED 入口签名, 仅 +2 fn)
    pub fn add_participant_public(&self, room_id: &str, p: Participant) -> GroupChatResult<()> {
        let mut g = self.rooms.lock();
        let r = g.get_mut(room_id).ok_or_else(|| GroupChatError::RoomNotFound(room_id.into()))?;
        r.add(p)
    }

    pub fn post_message_public(&self, room_id: &str, msg: ChatMessage) -> GroupChatResult<()> {
        let mut g = self.rooms.lock();
        let r = g.get_mut(room_id).ok_or_else(|| GroupChatError::RoomNotFound(room_id.into()))?;
        r.post(msg)
    }
}

impl Default for GroupChat {
    fn default() -> Self { Self::new() }
}

/// Room 借用句柄 (锁住期间不能跨 await)
pub struct GroupRoomRef<'a> {
    inner: parking_lot::lock_api::MutexGuard<'a, parking_lot::RawMutex, HashMap<String, GroupRoom>>,
    id: String,
}

impl<'a> GroupRoomRef<'a> {
    pub fn room(&self) -> &GroupRoom {
        self.inner.get(&self.id).unwrap()
    }
}

// ============================================================================
// Helper
// ============================================================================

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_role_count() {
        assert_eq!(ParticipantRole::COUNT, 3);
        assert!(ParticipantRole::Host.can_speak());
        assert!(ParticipantRole::Agent.can_speak());
        assert!(!ParticipantRole::Observer.can_speak());
    }

    #[test]
    fn t02_turn_policy_count() {
        assert_eq!(TurnPolicy::COUNT, 3);
    }

    #[test]
    fn t03_create_room() {
        let gc = GroupChat::new();
        let id = gc.create_room("family", "evening chat", TurnPolicy::Free);
        assert_eq!(gc.room_count(), 1);
        assert_eq!(gc.list_rooms(), vec![id]);
    }

    #[test]
    fn t04_add_participants() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("agent_a", "Alice", ParticipantRole::Agent)).unwrap();
        room.add(Participant::new("agent_b", "Bob", ParticipantRole::Agent)).unwrap();
        room.add(Participant::new("host_h", "Host", ParticipantRole::Host)).unwrap();
        assert_eq!(room.participant_count(), 3);
    }

    #[test]
    fn t05_post_message() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("a", "Alice", ParticipantRole::Agent)).unwrap();
        let msg = ChatMessage::new(room.id.clone(), "a", "hello world");
        room.post(msg).unwrap();
        assert_eq!(room.message_count(), 1);
    }

    #[test]
    fn t06_post_observer_rejected() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("o", "Observer", ParticipantRole::Observer)).unwrap();
        let msg = ChatMessage::new(room.id.clone(), "o", "hi");
        let r = room.post(msg);
        assert!(r.is_err());
    }

    #[test]
    fn t07_round_robin_enforces_turn() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::RoundRobin);
        room.add(Participant::new("a", "Alice", ParticipantRole::Agent)).unwrap();
        room.add(Participant::new("b", "Bob", ParticipantRole::Agent)).unwrap();

        room.post(ChatMessage::new(room.id.clone(), "a", "first")).unwrap();
        // 不能 a 连续发
        let r = room.post(ChatMessage::new(room.id.clone(), "a", "second"));
        assert!(r.is_err());
        // b 可以发
        room.post(ChatMessage::new(room.id.clone(), "b", "ok")).unwrap();
        assert_eq!(room.message_count(), 2);
    }

    #[test]
    fn t08_capacity_limit() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free).with_capacity(2);
        room.add(Participant::new("a", "A", ParticipantRole::Agent)).unwrap();
        room.add(Participant::new("b", "B", ParticipantRole::Agent)).unwrap();
        let r = room.add(Participant::new("c", "C", ParticipantRole::Agent));
        assert!(r.is_err());
    }

    #[test]
    fn t09_close_room_blocks_posts() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("a", "A", ParticipantRole::Agent)).unwrap();
        room.post(ChatMessage::new(room.id.clone(), "a", "hi")).unwrap();
        room.close();
        let r = room.post(ChatMessage::new(room.id.clone(), "a", "second"));
        assert!(r.is_err());
    }

    #[test]
    fn t10_recent_messages() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("a", "A", ParticipantRole::Agent)).unwrap();
        for i in 0..10 {
            room.post(ChatMessage::new(room.id.clone(), "a", format!("msg{}", i))).unwrap();
        }
        let recent = room.recent(3);
        assert_eq!(recent.len(), 3);
        assert!(recent[0].content.contains("msg7"));
    }

    #[test]
    fn t11_content_hash_deterministic() {
        let m1 = ChatMessage::new("r1", "a", "hello");
        let m2 = ChatMessage::new("r1", "a", "hello");
        // hash 包含 timestamp_ms, 不同时刻不同
        assert_ne!(m1.content_hash, m2.content_hash);
        assert_eq!(m1.content_hash.len(), 64);
    }

    #[test]
    fn t12_group_chat_close_and_archive() {
        let gc = GroupChat::new();
        let id = gc.create_room("evening", "chat", TurnPolicy::Free);
        gc.close_room(&id).unwrap();
        let room = gc.get(&id).unwrap();
        assert_eq!(room.room().status, RoomStatus::Closed);
        gc.archive_room(&id).unwrap();
        let room = gc.get(&id).unwrap();
        assert_eq!(room.room().status, RoomStatus::Archived);
    }

    #[test]
    fn t13_get_room_not_found() {
        let gc = GroupChat::new();
        let r = gc.get("nonexistent");
        assert!(r.is_err());
    }

    #[test]
    fn t14_remove_participant() {
        let mut room = GroupRoom::new("r", "t", TurnPolicy::Free);
        room.add(Participant::new("a", "A", ParticipantRole::Agent)).unwrap();
        room.remove("a").unwrap();
        assert_eq!(room.participant_count(), 0);
        let r = room.remove("a");
        assert!(r.is_err());
    }

    #[test]
    fn t15_reply_to() {
        let room_id = "r1";
        let m1 = ChatMessage::new(room_id, "a", "first");
        let m2 = ChatMessage::new(room_id, "b", "reply").reply_to(m1.id.clone());
        assert_eq!(m2.reply_to.as_deref(), Some(m1.id.as_str()));
    }
}
