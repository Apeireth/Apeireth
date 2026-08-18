//! **战役 1-3 / VCP §6.2.2 #19 — 15s 抑制窗口 (防 OpenAI Responses 偶发 5xx 重试风暴)**
//!
//! **借鉴来源 (字段级)**: `research/source/vcptoolbox/routes/protocolBridge.js:12`
//!
//! **VCP 真代码 (line 11-12, 字段级引用)**:
//! ```js
//! // line 11
//! const RESPONSE_RETRY_SUPPRESSION_WINDOW_MS = parseInt(
//!   process.env.PROTOCOL_BRIDGE_RETRY_SUPPRESSION_MS || '15000', 10
//! );
//! // line 12
//! const recentResponsesRequests = new Map();
//! ```
//!
//! **VCP 语义**:
//! - 同一个 `clientIp + messageId` 在 15s 内的重复请求 → 抑制 (不重跑管线, 返回缓存或拒绝)
//! - 解决: OpenAI Responses API 偶发 5xx, 客户端疯狂重试导致服务过载
//! - 15s 后过期, 正常重试
//!
//! **Apeireth 简化 (工程层借鉴, 不抄 VCP HTTP middleware)**:
//! - 借鉴**15s 时间窗语义** (VCP 真值)
//! - 借鉴**`Map<key, Instant>` 数据结构**
//! - 借鉴**`should_suppress(key)` 判定 API**
//! - 用 Rust `parking_lot::Mutex` 同步 (VCP JS 单线程, 我们多线程要加锁)
//! - **不抄 VCP 跟 HTTP route / clientIp 绑死** — `key` 是泛型 `&str`, 上层决定 key 怎么拼
//!
//! **不假装**:
//! - 15s 窗口真值跟 VCP `RESPONSE_RETRY_SUPPRESSION_WINDOW_MS` 一字不差
//! - `Map<key, Instant>` 真用 `HashMap`
//! - `should_suppress` 真按"first call records, second call within window suppressed, after window cleared"

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// **VCP 借鉴 #19** — 15s 抑制窗口 (ms)
///
/// 字段: `protocolBridge.js:11 RESPONSE_RETRY_SUPPRESSION_WINDOW_MS = parseInt(... || '15000', 10)`
pub const DEFAULT_SUPPRESSION_WINDOW_MS: u64 = 15_000;

/// **VCP 借鉴 #19** — 抑制窗口 Map (key → 首次调用时间)
///
/// 字段级对应 VCP `protocolBridge.js:12 recentResponsesRequests = new Map()`
///
/// **Apeireth 数据结构**: `HashMap<String, Instant>` + `Mutex` (Rust 多线程安全)
/// 用 `Arc<Mutex<...>>` 让 RetrySuppression 可以 Clone + 共享底层 map
#[derive(Debug, Clone)]
pub struct RetrySuppression {
    /// 抑制窗口时长 (ms), 默认 15s (VCP 真值)
    pub window: Duration,
    /// 抑制 key → 首次调用时间 (Arc<Mutex<HashMap>> for Clone)
    recent: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Default for RetrySuppression {
    fn default() -> Self {
        // VCP 默认 15000ms
        Self::new(Duration::from_millis(DEFAULT_SUPPRESSION_WINDOW_MS))
    }
}

impl RetrySuppression {
    /// 创建 (默认 15s VCP 窗口)
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            recent: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// VCP 默认 15s 窗口快速构造
    pub fn with_chat_default() -> Self {
        Self::default()
    }

    /// **VCP `shouldSuppress` 判定语义** — 第一次调用 → false (不抑制, 走真流程),
    /// 之后 15s 内同一 key → true (抑制, 跳过), 15s 后 → false (清掉, 重新计数)
    ///
    /// **副作用**: 第一次调用时, 记录 key → Instant::now()
    pub fn should_suppress(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.recent.lock();

        // 检查是否已有记录
        if let Some(&first_at) = map.get(key) {
            let elapsed = now.duration_since(first_at);
            if elapsed < self.window {
                // 还在窗口内 → 抑制
                return true;
            }
            // 窗口外 → 清掉旧记录, 重新计
            map.remove(key);
        }
        // 第一次调用 / 窗口外重新进入 → 记录, 不抑制
        map.insert(key.to_string(), now);
        false
    }

    /// 清空所有抑制记录 (测试 / 手动 reset)
    pub fn clear(&self) {
        self.recent.lock().clear();
    }

    /// 当前记录的 key 数量 (测试 / 监控)
    pub fn len(&self) -> usize {
        self.recent.lock().len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.recent.lock().is_empty()
    }

    /// 主动清理过期条目 (避免 Map 无限增长)
    ///
    /// VCP 真代码不显式清理, 靠 GC;我们 Rust 没 GC, 显式触发
    pub fn cleanup_expired(&self) -> usize {
        let now = Instant::now();
        let mut map = self.recent.lock();
        let before = map.len();
        map.retain(|_, first_at| now.duration_since(*first_at) < self.window);
        before - map.len()
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

const _: () = {
    // 15s 真值对齐 VCP protocolBridge.js:11
    assert!(DEFAULT_SUPPRESSION_WINDOW_MS == 15_000, "VCP 真值 15000ms");

    // 至少 1s (太小没意义)
    assert!(DEFAULT_SUPPRESSION_WINDOW_MS >= 1_000, "抑制窗口至少 1s");
};

// ============================================================
// 单元测试 — 包含主人要求的边界测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ====== 常量真值 ======

    #[test]
    fn default_window_matches_vcp_15s() {
        // VCP protocolBridge.js:11 RESPONSE_RETRY_SUPPRESSION_WINDOW_MS = 15000
        assert_eq!(DEFAULT_SUPPRESSION_WINDOW_MS, 15_000);
    }

    #[test]
    fn default_constructor_uses_15s() {
        let s = RetrySuppression::default();
        assert_eq!(s.window, Duration::from_millis(15_000));
    }

    #[test]
    fn with_chat_default_uses_15s() {
        let s = RetrySuppression::with_chat_default();
        assert_eq!(s.window, Duration::from_millis(15_000));
    }

    // ====== 15s 窗口边界 ======

    #[test]
    fn should_suppress_first_call_false() {
        let s = RetrySuppression::with_chat_default();
        // 第一次调用 → 不抑制 (VCP 行为: 记录, 不拒)
        assert!(!s.should_suppress("client1:msg-001"));
    }

    #[test]
    fn should_suppress_second_call_within_window_true() {
        let s = RetrySuppression::with_chat_default();
        assert!(!s.should_suppress("client1:msg-001"));
        // 立即第二次 → 抑制
        assert!(s.should_suppress("client1:msg-001"));
    }

    #[test]
    fn should_suppress_short_window_expires() {
        // 100ms 窗口, 测试窗口过期重置
        let s = RetrySuppression::new(Duration::from_millis(100));
        assert!(!s.should_suppress("k"));
        // 100ms 内 → 抑制
        assert!(s.should_suppress("k"));
        // 等 150ms 后, 窗口过期
        std::thread::sleep(Duration::from_millis(150));
        // 又允许
        assert!(!s.should_suppress("k"));
    }

    #[test]
    fn should_suppress_exact_window_boundary() {
        // CI fix 2026-08: 原 200ms 窗口 + 150ms sleep 在 macOS nextest 高并发下
        // 调度延迟可把 150ms 拖到 ≥200ms → 边界断言 flaky. 加大余量:
        // 500ms 窗口, sleep 300ms (余量 200ms), 再 sleep 300ms (总 600ms > 500ms).
        let s = RetrySuppression::new(Duration::from_millis(500));
        assert!(!s.should_suppress("k"));
        std::thread::sleep(Duration::from_millis(300));
        // 300ms < 500ms → 还抑制
        assert!(s.should_suppress("k"));
        std::thread::sleep(Duration::from_millis(300));
        // 总 600ms > 500ms → 过期
        assert!(!s.should_suppress("k"));
    }

    // ====== key 独立 ======

    #[test]
    fn keys_are_independent() {
        let s = RetrySuppression::with_chat_default();
        assert!(!s.should_suppress("client1:msg-001"));
        // 不同 key → 独立
        assert!(!s.should_suppress("client1:msg-002"));
        assert!(!s.should_suppress("client2:msg-001"));
        // 但同 key 第二次 → 抑制
        assert!(s.should_suppress("client1:msg-001"));
    }

    #[test]
    fn key_count_tracks_unique_keys() {
        let s = RetrySuppression::with_chat_default();
        assert_eq!(s.len(), 0);
        let _ = s.should_suppress("a");
        assert_eq!(s.len(), 1);
        let _ = s.should_suppress("b");
        assert_eq!(s.len(), 2);
        // 重复同 key → 不增加
        let _ = s.should_suppress("a");
        assert_eq!(s.len(), 2);
    }

    // ====== cleanup / clear ======

    #[test]
    fn clear_empties_map() {
        let s = RetrySuppression::with_chat_default();
        let _ = s.should_suppress("a");
        let _ = s.should_suppress("b");
        assert_eq!(s.len(), 2);
        s.clear();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn cleanup_expired_removes_old() {
        // 50ms 窗口
        let s = RetrySuppression::new(Duration::from_millis(50));
        let _ = s.should_suppress("a");
        let _ = s.should_suppress("b");
        assert_eq!(s.len(), 2);
        std::thread::sleep(Duration::from_millis(100));
        let removed = s.cleanup_expired();
        assert_eq!(removed, 2);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn cleanup_keeps_fresh() {
        // 5s 窗口 (够长)
        let s = RetrySuppression::new(Duration::from_secs(5));
        let _ = s.should_suppress("a");
        let removed = s.cleanup_expired();
        assert_eq!(removed, 0);
        assert_eq!(s.len(), 1);
    }
}
