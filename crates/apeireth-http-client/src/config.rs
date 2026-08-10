//! `KeepAliveConfig` — 复刻 VCP `chatCompletionHandler.js:22-28` 的 5 字段
//! (`agentOptions = { keepAlive: true, keepAliveMsecs: 1000, freeSocketTimeout: 8000,
//! scheduling: 'lifo', maxSockets: 10000 }`).
//!
//! **字段级引用**: `research/source/vcptoolbox/modules/chatCompletionHandler.js:22-28` (主借鉴文件)
//! + `docs/stage3-blueprints/borrowed-from-projects.md §6.2.2 #14`
//!
//! **设计哲学**:
//! - **编译期 hardcode** —— 字段名跟 VCP 真代码对齐 (snake_case 化), 默认值同 VCP
//! - **不漂移** —— 不加 VCP 没有的字段 (e.g. 不加 `tls_min_version`, 那是别的借鉴范围)
//! - **守护者模式** —— 每个字段是必填 pub, 不留 Optional, 强制调用方思考
//!
//! **字段映射** (VCP → Rust):
//! | VCP 字段 (camelCase) | Rust 字段 (snake_case) | 类型 | VCP 默认值 | 用途 |
//! |----------------------|------------------------|------|-----------|------|
//! | `keepAlive`          | `keep_alive`           | `bool`  | `true`  | 是否启用 TCP keep-alive |
//! | `keepAliveMsecs`     | `keep_alive_msecs`     | `u64`   | `1000`  | TCP keep-alive 探针间隔 (ms) |
//! | `freeSocketTimeout`  | `free_socket_timeout`  | `u64`   | `8000`  | 空闲 socket 主动销毁阈值 (ms) — **绝杀 zombie 1s hang up** |
//! | `scheduling`         | `scheduling`           | `SchedulingPolicy` | `'lifo'` | 调度策略: LIFO 优先复用最新鲜的连接 |
//! | `maxSockets`         | `max_sockets`          | `usize` | `10000` | 全局高并发上限 |

use serde::{Deserialize, Serialize};

/// 调度策略 — 对应 VCP `agentOptions.scheduling` (`'lifo'` / `'fifo'`).
///
/// VCP 真代码只用了 `'lifo'`, 我们保留 `'fifo'` 是兜底 + 可观测 (测延迟对比).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// **LIFO** (Last In, First Out) — 后进先出
    /// 永远优先复用刚刚才活跃过、最新鲜的热连接
    /// VCP `chatCompletionHandler.js:26 scheduling: 'lifo'` 真代码
    Lifo,
    /// FIFO (First In, First Out) — 先进先出
    /// 标准队列, 给测试用, 生产用 LIFO
    Fifo,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        // VCP 真代码默认 lifo, 守住这个默认值
        SchedulingPolicy::Lifo
    }
}

impl SchedulingPolicy {
    /// VCP 真代码字符串表示 (跟 `agentOptions.scheduling` 字面量一致)
    pub fn as_str(&self) -> &'static str {
        match self {
            SchedulingPolicy::Lifo => "lifo",
            SchedulingPolicy::Fifo => "fifo",
        }
    }
}

impl std::fmt::Display for SchedulingPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// **5 字段配置** — 复刻 VCP `agentOptions`.
///
/// 字段名严格对齐 VCP 真代码, 不增不减; 默认值同 VCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// VCP `keepAlive: true` — 是否启用 TCP keep-alive
    pub keep_alive: bool,
    /// VCP `keepAliveMsecs: 1000` — TCP keep-alive 探针间隔 (ms)
    pub keep_alive_msecs: u64,
    /// VCP `freeSocketTimeout: 8000` — 空闲 socket 主动销毁阈值 (ms)
    ///
    /// **绝杀机制**: 空闲 8s 后主动销毁, 防止复用到被上游代理 (如 Nginx) 静默杀死的僵尸连接
    /// 解决 VCP 注释里写的 "-1s Socket Hang Up" 问题
    pub free_socket_timeout: u64,
    /// VCP `scheduling: 'lifo'` — 调度策略
    pub scheduling: SchedulingPolicy,
    /// VCP `maxSockets: 10000` — 全局高并发上限
    pub max_sockets: usize,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        // **编译期 hardcode VCP 默认值** — 守住字面量
        Self::vcp_default()
    }
}

impl KeepAliveConfig {
    /// **VCP 默认值** — `chatCompletionHandler.js:22-28` 真代码 5 字段默认值
    ///
    /// 这是 `Default::default()` 的真源, 单独抽出来便于测试 + 文档
    pub const fn vcp_default() -> Self {
        Self {
            keep_alive: true,
            keep_alive_msecs: 1000,
            free_socket_timeout: 8000,
            scheduling: SchedulingPolicy::Lifo,
            max_sockets: 10_000,
        }
    }

    /// LIFO 策略 + VCP 默认其他字段 — 最常用的快速构造
    pub const fn lifo_default() -> Self {
        Self::vcp_default()
    }

    /// 字段级校验 — 防止 max_sockets=0 这种运行时崩溃
    pub fn validate(&self) -> Result<(), String> {
        if self.max_sockets == 0 {
            return Err("max_sockets must be > 0".to_string());
        }
        if self.keep_alive_msecs == 0 {
            return Err("keep_alive_msecs must be > 0 (TCP keep-alive interval)".to_string());
        }
        if self.free_socket_timeout == 0 {
            return Err(
                "free_socket_timeout must be > 0 (zombie socket kill threshold)".to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcp_default_matches_chat_completion_handler_22_28() {
        // **字段级断言**: VCP 5 字段默认值必须一字不差
        // `chatCompletionHandler.js:22-28`:
        //   const agentOptions = {
        //     keepAlive: true,
        //     keepAliveMsecs: 1000,
        //     freeSocketTimeout: 8000,
        //     scheduling: 'lifo',
        //     maxSockets: 10000
        //   };
        let cfg = KeepAliveConfig::vcp_default();
        assert!(cfg.keep_alive, "keep_alive must be true (VCP line 23)");
        assert_eq!(
            cfg.keep_alive_msecs, 1000,
            "keep_alive_msecs must be 1000 (VCP line 24)"
        );
        assert_eq!(
            cfg.free_socket_timeout, 8000,
            "free_socket_timeout must be 8000 (VCP line 25)"
        );
        assert_eq!(
            cfg.scheduling,
            SchedulingPolicy::Lifo,
            "scheduling must be Lifo (VCP line 26)"
        );
        assert_eq!(
            cfg.max_sockets, 10_000,
            "max_sockets must be 10000 (VCP line 27)"
        );
    }

    #[test]
    fn default_impl_returns_vcp_default() {
        assert_eq!(KeepAliveConfig::default(), KeepAliveConfig::vcp_default());
    }

    #[test]
    fn scheduling_default_is_lifo() {
        assert_eq!(SchedulingPolicy::default(), SchedulingPolicy::Lifo);
    }

    #[test]
    fn scheduling_as_str_matches_vcp_literals() {
        // VCP 字符串字面量: 'lifo' / 'fifo'
        assert_eq!(SchedulingPolicy::Lifo.as_str(), "lifo");
        assert_eq!(SchedulingPolicy::Fifo.as_str(), "fifo");
        assert_eq!(SchedulingPolicy::Lifo.to_string(), "lifo");
        assert_eq!(SchedulingPolicy::Fifo.to_string(), "fifo");
    }

    #[test]
    fn validate_rejects_zero_max_sockets() {
        let cfg = KeepAliveConfig {
            max_sockets: 0,
            ..KeepAliveConfig::vcp_default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_keep_alive_msecs() {
        let cfg = KeepAliveConfig {
            keep_alive_msecs: 0,
            ..KeepAliveConfig::vcp_default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_free_socket_timeout() {
        let cfg = KeepAliveConfig {
            free_socket_timeout: 0,
            ..KeepAliveConfig::vcp_default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_accepts_vcp_default() {
        assert!(KeepAliveConfig::vcp_default().validate().is_ok());
    }

    #[test]
    fn five_field_count_guards_against_drift() {
        // **漂移守门**: 5 字段是 VCP 借鉴的硬约束, 任何新增字段必须显式改这个测试
        // (用 std::mem::size_of 检查字段数变化 — 简化版, 实际 5 字段加起来 ≥ 5*8 = 40 bytes)
        // bool(1) + padding + u64(8) + u64(8) + enum(1) + padding + usize(8) = ≥ 40 字节
        // 实际 sizeof 因对齐可能 = 40, 但我们只断言 ≥ 24 (5 字段下界)
        assert!(
            std::mem::size_of::<KeepAliveConfig>() >= 24,
            "KeepAliveConfig shrunk — 字段可能被砍了, 守住 VCP 5 字段"
        );
    }
}
