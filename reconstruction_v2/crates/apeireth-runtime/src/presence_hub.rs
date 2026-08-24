//! PresenceHub - 聚合 companion/avatar/voice/bridge 实时状态给前端 WebSocket
//!
//! 0 装 PASS: 从 UnifiedRuntimeHost 多个 emotion/companion 字段抽出, 聚合为 PresenceSnapshot,
//! 由 gateway 通过 WebSocket 推给 v2.0 桌面伴侣前端 (替代 v1.0 era 的 SSE presence 流)。
//!
//! 设计动机:
//! - v1.0 时代 EventBus presence 流是字符串 payload, 前端要 parse JSON
//! - v2.0 提供 typed PresenceSnapshot, 前端直接订阅 + apply 到 Live2D params
//!
//! 0 装 PASS: 当前 snapshot 字段用 apeireth-companion::emotion 类型, 不重新定义。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_companion::emotion::{Pad, ResponseStyle};
use apeireth_companion::emergence::BorbelyModel;
use tokio::sync::Mutex;

/// PresenceSnapshot - 单次推送给前端的实时状态 (0 装 PASS: 字段精简, 后续可扩)
#[derive(Debug, Clone)]
pub struct PresenceSnapshot {
    /// Unix 毫秒时间戳
    pub timestamp_ms: i64,
    /// PAD 情感维度 (Pleasure, Arousal, Dominance)
    pub pad: Pad,
    /// 语调风格 (Warm / Reserved / Neutral / Analytical / Empathic / Playful / Assertive)
    pub response_style: ResponseStyle,
    /// 温度 (warmth 0..1, 基于 Borbély 模型 + 互动频率)
    pub drive_warmth: f64,
    /// 静默压力 (silence_pressure 0..1, 越久未互动越高)
    pub silence_pressure: f64,
}

impl PresenceSnapshot {
    /// 序列化为 JSON (gateway 直接 broadcast)
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "timestamp": self.timestamp_ms,
            "pad": {
                "pleasure": self.pad.pleasure,
                "arousal": self.pad.arousal,
                "dominance": self.pad.dominance,
            },
            "response_style": format!("{:?}", self.response_style),
            "drive_warmth": self.drive_warmth,
            "silence_pressure": self.silence_pressure,
        }).to_string()
    }
}

/// PresenceHub - 持有所有"可被前端观察"的子系统引用, 提供 snapshot() 拍快照。
///
/// 0 装 PASS: 通过 Arc 引用 host 已有的 subsystem (Plutchik emotion, Borbély model),
/// 不持所有权, host 仍负责生命周期。
pub struct PresenceHub {
    plutchik: Arc<Mutex<apeireth_companion::emotion::Plutchik>>,
    borbely: Arc<Mutex<BorbelyModel>>,
}

impl PresenceHub {
    pub fn new(
        plutchik: Arc<Mutex<apeireth_companion::emotion::Plutchik>>,
        borbely: Arc<Mutex<BorbelyModel>>,
    ) -> Self {
        Self { plutchik, borbely }
    }

    /// 0 装 PASS: 拍快照 — 不持锁时间过长 (await Mutex lock 一次)。
    /// gateway 每秒 / 每 5 秒调一次, 推到 WebSocket presence 流。
    pub async fn snapshot(&self) -> PresenceSnapshot {
        // Plutchik 是 0..1 情绪维度, Borbély 提供 drive_warmth + silence_pressure
        // Series lock 取避免死锁
        let plutchik = self.plutchik.lock().await;
        let pad: Pad = plutchik.to_pad();
        let style = pad.to_response_style();
        drop(plutchik);

        let borbely = self.borbely.lock().await;
        let drive_warmth = borbely.drive(); // = warmth * w1 + silence_pressure * w2

        // 用 SystemTime 取 timestamp
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        PresenceSnapshot {
            timestamp_ms,
            pad,
            response_style: style,
            drive_warmth,
            silence_pressure: 0.0, // 0 装 PASS: silence_pressure 当前未单独导出, 留 0 placeholder
        }
    }
}

impl std::fmt::Debug for PresenceHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresenceHub").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_companion::emotion::Plutchik;

    #[tokio::test]
    async fn test_presence_snapshot_basic() {
        let pl = Arc::new(Mutex::new(Plutchik::default()));
        let borbely = Arc::new(Mutex::new(BorbelyModel::new(0.6, 0.4)));
        let hub = PresenceHub::new(pl, borbely);
        let snap = hub.snapshot().await;
        assert!(snap.timestamp_ms > 0);
        assert!(snap.drive_warmth >= 0.0 && snap.drive_warmth <= 1.0);
    }
}
