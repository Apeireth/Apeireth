//! `apeireth-companion::daemon` — 总装: 把机制 + 器官 + 渲染 + 送达 + 记忆 接成常驻 daemon.
//!
//! 分层:
//! - `UtteranceGenerator` — 把 Initiative 的诚实事实渲染成「他的话」(`PlainUtterance` = 原文;
//!   真 LLM 渲染见 examples/companion_daemon.rs 的 MiniMaxUtterance).
//! - `Sink` — 通道 (`ConsoleSink` / `LarkSink` 飞书 IM).
//! - `CompanionDelivery` — 渲染 → 通道 组合.
//! - `CompanionDaemon` — 全器官伙伴 + 送达 + 记忆 + 心跳循环.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_lark::{LarkClient, LarkConfig, LarkRealImpl, MessageType};
use apeireth_memory::SqliteMemoryStore;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::emergence::{Boundaries, Delivery, Feedback, Initiative, SelfScore};
use crate::organs::AwakeCompanion;
use crate::proactive::ContextSource;
use crate::Bond;
use crate::dream::DreamScheduler;
use crate::reflection::ReflectionScheduler;
use apeireth_core::RiskLevel;

// ============================================================
// 宪法评审 (Judicator) + 评审策略 (成本与风险成正比)
// ============================================================

/// 宪法评审者: 按原则判案. 只在 Medium+ 风险动作时被调用 (评审成本 ∝ 风险).
#[async_trait]
pub trait Judicator: Send + Sync {
    /// 返回 true=ALLOW / false=BLOCK.
    async fn judge(&self, action: &str) -> Result<bool, String>;
}

/// 没接评审器的默认 (诚实: 全放行 — 本地门禁/洋葱门/审批仍在).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopJudicator;

#[async_trait]
impl Judicator for NoopJudicator {
    async fn judge(&self, _action: &str) -> Result<bool, String> {
        Ok(true)
    }
}

/// 评审策略: Low/Info 动作 0 额外 token (本地 Rust 门禁足够);
/// Medium/High 才请宪法评审者 (1 次 LLM); Critical 由洋葱门直接拦.
/// 这正是 stage1 §20.3 风险分级→席位矩阵的经济学表达: 席位随风险, 成本随席位.
pub fn requires_llm_review(risk: RiskLevel) -> bool {
    matches!(risk, RiskLevel::Medium | RiskLevel::High)
}

// ============================================================
// 渲染: 事实 → 他的话
// ============================================================

#[async_trait]
pub trait UtteranceGenerator: Send + Sync {
    async fn utter(&self, initiative: &Initiative) -> Result<String, String>;
}

/// 无 LLM 的默认渲染: 直接返回机制的诚实原文 (不装「已润色」).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainUtterance;

#[async_trait]
impl UtteranceGenerator for PlainUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        Ok(i.to_message())
    }
}

/// LLM 渲染节流 + 失败退避 (包装任意 `UtteranceGenerator`).
///
/// - **间隔**: 距上次真 LLM 调用不足 `min_interval` → 不调 LLM, 返回机制的诚实原文
///   (`i.to_message()`, 与 [`PlainUtterance`] 同语义, 不装「已润色」).
/// - **退避**: 调用失败 → 下一次等待 `backoff` (从 `min_interval` 起指数翻倍, 封顶
///   `max_backoff`), 防 MiniMax 限流 (suppressed) 下的连续锤打. 成功 → 退避复位.
///
/// 与 `LoopConfig.min_llm_interval` (机制层门禁) 双层防护: 门禁拦「该不该开口」,
/// 这里拦「真 LLM 调用」本身.
pub struct ThrottledUtterance<U: UtteranceGenerator> {
    inner: U,
    min_interval: Duration,
    max_backoff: Duration,
    state: Mutex<ThrottleState>,
}

struct ThrottleState {
    last_call: Option<Instant>,
    backoff: Duration,
}

impl<U: UtteranceGenerator> ThrottledUtterance<U> {
    pub fn new(inner: U, min_interval: Duration) -> Self {
        Self {
            inner,
            min_interval,
            max_backoff: Duration::from_secs(300), // 默认封顶 5min
            state: Mutex::new(ThrottleState {
                last_call: None,
                backoff: Duration::ZERO,
            }),
        }
    }

    /// 覆盖退避封顶 (默认 5min).
    pub fn with_max_backoff(mut self, max_backoff: Duration) -> Self {
        self.max_backoff = max_backoff;
        self
    }

    /// 距下次允许调用还要等多久 (0 = 现在就能调).
    fn wait_remaining(&self, now: Instant) -> Duration {
        let s = self.state.lock().expect("throttle state");
        let Some(last) = s.last_call else {
            return Duration::ZERO;
        };
        let interval = self.min_interval.max(s.backoff);
        interval.saturating_sub(now.saturating_duration_since(last))
    }
}

#[async_trait]
impl<U: UtteranceGenerator + Send + Sync> UtteranceGenerator for ThrottledUtterance<U> {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let now = Instant::now();
        if self.wait_remaining(now) > Duration::ZERO {
            // 节流: 诚实原文兜底 (不装「已润色」)
            return Ok(i.to_message());
        }
        match self.inner.utter(i).await {
            Ok(text) => {
                let mut s = self.state.lock().expect("throttle state");
                s.last_call = Some(now);
                s.backoff = Duration::ZERO;
                Ok(text)
            }
            Err(e) => {
                // 失败退避: 指数翻倍, 封顶 max_backoff
                let mut s = self.state.lock().expect("throttle state");
                s.last_call = Some(now);
                let step = if s.backoff.is_zero() {
                    self.min_interval
                } else {
                    s.backoff
                };
                s.backoff = (step * 2).min(self.max_backoff);
                Err(e)
            }
        }
    }
}

// ============================================================
// 通道: 发出渲染后的文本
// ============================================================

#[async_trait]
pub trait Sink: Send + Sync {
    async fn send(&self, text: &str) -> Result<(), String>;
}

/// 控制台通道 (真输出).
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsoleSink;

#[async_trait]
impl Sink for ConsoleSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        println!("[他说] {}", text);
        Ok(())
    }
}

/// 飞书 IM 通道 (真 HTTP, 需凭据 + receive_id).
#[derive(Debug)]
pub struct LarkSink {
    client: Arc<LarkRealImpl>,
    receive_id: String,
}

impl LarkSink {
    pub fn new(config: LarkConfig, receive_id: impl Into<String>) -> Result<Self, String> {
        let client = LarkRealImpl::new(config).map_err(|e| e.to_string())?;
        Ok(Self {
            client: Arc::new(client),
            receive_id: receive_id.into(),
        })
    }

    /// 从环境变量读飞书凭据构建:
    /// - `APEIRETH_LARK_APP_ID` / `APEIRETH_LARK_APP_SECRET` (必填, 飞书应用凭证)
    /// - `APEIRETH_LARK_RECEIVE_ID` (必填, 会话/用户 open_id)
    /// - `APEIRETH_LARK_BASE_URL` (可选, 默认飞书官方 `https://open.feishu.cn/open-apis`)
    ///
    /// 缺任一必填项 → `Err` (带明确提示, 不装「已接好」).
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// 测试友好的 env 注入版本 (lookup 代替真实环境).
    fn from_env_with<F>(lookup: F) -> Result<Self, String>
    where
        F: Fn(&str) -> Option<String>,
    {
        let missing = |keys: &[&str]| -> String {
            format!("飞书凭据缺失: {} (设 APEIRETH_LARK_* 环境变量)", keys.join(" / "))
        };
        let app_id = lookup("APEIRETH_LARK_APP_ID")
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| missing(&["APEIRETH_LARK_APP_ID"]))?;
        let app_secret = lookup("APEIRETH_LARK_APP_SECRET")
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| missing(&["APEIRETH_LARK_APP_SECRET"]))?;
        let receive_id = lookup("APEIRETH_LARK_RECEIVE_ID")
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| missing(&["APEIRETH_LARK_RECEIVE_ID"]))?;
        let base_url = lookup("APEIRETH_LARK_BASE_URL")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| apeireth_lark::LARK_API_BASE_URL.to_string());
        let config = LarkConfig {
            app_id,
            app_secret,
            base_url,
            token_cache_ttl_seconds: apeireth_lark::LARK_TOKEN_CACHE_TTL_SECONDS,
        };
        Self::new(config, receive_id)
    }
}

#[async_trait]
impl Sink for LarkSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        let content = serde_json::json!({ "text": text }).to_string();
        self.client
            .send_message(&self.receive_id, MessageType::Text, &content)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// SSE 广播通道 (模块 4 主动送达: 涌现/事件 → 前端实时推送).
/// 同时保留控制台输出 (离线可查日志).
#[derive(Debug, Clone)]
pub struct BroadcastSink {
    tx: tokio::sync::broadcast::Sender<String>,
}

impl BroadcastSink {
    pub fn new(tx: tokio::sync::broadcast::Sender<String>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl Sink for BroadcastSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        println!("[他说] {text}");
        let _ = self.tx.send(format!("[他说] {text}"));
        Ok(())
    }
}

/// 多通道送达 (模块 4): 广播 (SSE) + 可选 Lark (离线) 等, 全通道尽力送达.
/// 任一通道失败只记日志, 不阻断其他通道; 全失败才返回 Err.
#[derive(Default)]
pub struct MultiSink {
    sinks: Vec<Box<dyn Sink>>,
}

impl MultiSink {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }
    pub fn push(mut self, s: Box<dyn Sink>) -> Self {
        self.sinks.push(s);
        self
    }
}

#[async_trait]
impl Sink for MultiSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        let mut last_err = String::new();
        for s in &self.sinks {
            if let Err(e) = s.send(text).await {
                last_err = format!("{e}");
                eprintln!("[sink] 送达失败: {e}");
            }
        }
        if last_err.is_empty() {
            Ok(())
        } else {
            Err(last_err)
        }
    }
}

// ============================================================
// 组合送达: 渲染 → 通道
// ============================================================

pub struct CompanionDelivery<U: UtteranceGenerator, S: Sink> {
    utter: U,
    sink: S,
}

impl<U: UtteranceGenerator, S: Sink> CompanionDelivery<U, S> {
    pub fn new(utter: U, sink: S) -> Self {
        Self { utter, sink }
    }
}

#[async_trait]
impl<U: UtteranceGenerator, S: Sink> Delivery for CompanionDelivery<U, S> {
    async fn deliver(&self, i: &Initiative) -> Result<(), String> {
        let text = self.utter.utter(i).await?;
        // 隐私护栏 (guard 真件): 出站前检测 + 脱敏 PII
        let matches = apeireth_guard::detect_pii(&text);
        let safe = if matches.is_empty() {
            text
        } else {
            eprintln!("[guard] 出站消息脱敏 {} 处 PII", matches.len());
            apeireth_guard::redact_text(&text, &matches, apeireth_guard::RedactionStrategy::Mask)
        };
        self.sink.send(&safe).await
    }
}

/// 对象安全适配: `Box<dyn Delivery>` 也是 `Delivery` (运行时切换渲染/通道组合).
#[async_trait]
impl Delivery for Box<dyn Delivery> {
    async fn deliver(&self, initiative: &Initiative) -> Result<(), String> {
        self.as_ref().deliver(initiative).await
    }
}

// ============================================================
// 生产记忆 (真 SQLite)
// ============================================================

/// 生产记忆库路径: `APEIRETH_MEMORY_PATH` 或默认用户数据目录.
/// - Windows: `%APPDATA%\apeireth\memory.sqlite`
/// - 非 Windows: `$XDG_DATA_HOME/apeireth/memory.sqlite`, 无则 `$HOME/.local/share/apeireth/memory.sqlite`
pub fn default_memory_path() -> Result<PathBuf, String> {
    memory_path_from(|k| std::env::var(k).ok())
}

/// 测试友好的路径解析 (lookup 代替真实环境).
fn memory_path_from<F>(lookup: F) -> Result<PathBuf, String>
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(p) = lookup("APEIRETH_MEMORY_PATH") {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p.trim()));
        }
    }
    let nonempty = |k: &str| lookup(k).filter(|v| !v.trim().is_empty());
    let mut p = if let Some(a) = nonempty("APPDATA") {
        PathBuf::from(a) // Windows 用户数据目录
    } else if let Some(x) = nonempty("XDG_DATA_HOME") {
        PathBuf::from(x)
    } else if let Some(h) = nonempty("HOME") {
        PathBuf::from(h).join(".local").join("share") // XDG 默认
    } else {
        return Err("无法定位用户数据目录 (设 APEIRETH_MEMORY_PATH)".to_string());
    };
    p.push("apeireth");
    p.push("memory.sqlite");
    Ok(p)
}

/// 持久 continuity_id: `APEIRETH_CONTINUITY_ID` 或默认值.
/// 哲学锚点 (§18.3 记录+迁移): 跨载体/跨重启的稳定身份, 记忆/日志/目标/反思共用.
pub fn continuity_id_from_env(default: &str) -> String {
    std::env::var("APEIRETH_CONTINUITY_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// 打开生产记忆库 (真路径, 自动建父目录). 空库可正常打开 — 记忆从零开始长.
pub fn open_memory_store() -> Result<Arc<SqliteMemoryStore>, String> {
    let path = default_memory_path()?;
    open_memory_store_at(&path)
}

/// 在给定路径打开记忆库 (内部复用, 测试友好).
fn open_memory_store_at(path: &std::path::Path) -> Result<Arc<SqliteMemoryStore>, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建记忆目录失败 {}: {e}", parent.display()))?;
    }
    let store = SqliteMemoryStore::open(path)
        .map_err(|e| format!("打开记忆库失败 {}: {e}", path.display()))?;
    Ok(Arc::new(store))
}

// ============================================================
// 常驻驱动
// ============================================================

/// 总装: 全器官伙伴 + 送达 + 记忆 + 心跳.
pub struct CompanionDaemon<D: Delivery, C: ContextSource> {
    pub awake: AwakeCompanion,
    pub delivery: D,
    pub context: C,
    pub subject: String,
    pub tick_interval: Duration,
    /// 做梦调度器 (可选): 每 tick 检查, 该做梦 → 合并记忆写回真库.
    pub dream: Option<DreamScheduler>,
    /// 反思周期调度器 (可选): 周期到 → 4 阶段反思 → 写回真库.
    pub reflection: Option<ReflectionScheduler>,
}

impl<D: Delivery, C: ContextSource> CompanionDaemon<D, C> {
    pub fn new(
        bond: Bond,
        boundaries: Boundaries,
        delivery: D,
        context: C,
        subject: impl Into<String>,
        tick_interval: Duration,
    ) -> Self {
        Self {
            awake: AwakeCompanion::new(bond, boundaries),
            delivery,
            context,
            subject: subject.into(),
            tick_interval,
            dream: None,
            reflection: None,
        }
    }

    /// 接做梦调度器 (合并记忆写回真库).
    pub fn with_dream(mut self, dream: DreamScheduler) -> Self {
        self.dream = Some(dream);
        self
    }

    /// 接反思周期调度器 (周期反思写回真库).
    pub fn with_reflection(mut self, reflection: ReflectionScheduler) -> Self {
        self.reflection = Some(reflection);
        self
    }

    /// 一轮心跳: 反思检查 → 做梦检查 → 记忆检索 → 全器官决策 → 渲染送达.
    pub async fn step(&mut self) {
        let now = Utc::now();
        // 反思周期检查 (0 阻塞: 周期未到立即返回; 深度反思可 await)
        if let Some(r) = &mut self.reflection {
            let n = r.tick().await;
            if n > 0 {
                eprintln!("[daemon] 反思周期: 完成 {n} 轮 (累计 {})", r.cycles_completed());
            }
        }
        // 做梦检查 (0 阻塞: 不触发则立即返回)
        if let Some(d) = &self.dream {
            let n = d.tick().await;
            if n > 0 {
                eprintln!("[daemon] 做梦周期: 合并写回 {n} 条记忆");
            }
        }
        let hint = self.context.context_for(&self.subject);
        if let Some(init) = self.awake.tick(now, hint) {
            if let Err(e) = self.delivery.deliver(&init).await {
                eprintln!("[daemon] 送达失败: {e}");
            }
        }
    }

    /// 一次用户交互 (任何消息): 喂节律 + 刷新最后接触 + 重置做梦安静期.
    pub fn on_user_message(&mut self, at: DateTime<Utc>) {
        self.awake.observe_interaction(at);
        if let Some(d) = &self.dream {
            d.record_activity();
        }
    }

    /// 用户对上次主动的明确反馈 (回了 / 没回).
    pub fn on_feedback(&mut self, responded: bool, at: DateTime<Utc>) -> SelfScore {
        let f = if responded {
            Feedback::Responded
        } else {
            Feedback::Ignored
        };
        if let Some(d) = &self.dream {
            d.record_activity();
        }
        self.awake.apply_feedback(f, at)
    }

    /// 常驻主循环 (永不返回, 由外部 kill / shutdown 终止).
    pub async fn run(mut self) {
        let mut interval = tokio::time::interval(self.tick_interval);
        loop {
            interval.tick().await;
            self.step().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emergence::{ConsoleDelivery, NoopDelivery};
    use crate::proactive::EmptyContext;

    #[tokio::test]
    async fn plain_utterance_returns_honest_facts() {
        let u = PlainUtterance;
        let init = Initiative {
            reason: crate::emergence::InitiativeReason::RhythmMatched { minutes_now: 520 },
            action: crate::actions::Action::Greet,
            rhythm: crate::emergence::RhythmEstimate {
                active_probability: 1.0,
                days: 7,
                confidence: 0.5,
            },
            depth: 0.6,
            context_hint: None,
        };
        let t = u.utter(&init).await.unwrap();
        assert!(t.contains("置信度 50%") && t.contains("概率"));
    }

    #[tokio::test]
    async fn console_sink_prints() {
        ConsoleSink.send("你好").await.unwrap();
    }

    fn sample_initiative() -> Initiative {
        Initiative {
            reason: crate::emergence::InitiativeReason::RhythmMatched { minutes_now: 520 },
            action: crate::actions::Action::Greet,
            rhythm: crate::emergence::RhythmEstimate {
                active_probability: 1.0,
                days: 7,
                confidence: 0.5,
            },
            depth: 0.6,
            context_hint: None,
        }
    }

    /// 数调用次数的假 LLM 渲染器.
    struct CountingUtter {
        calls: Mutex<u32>,
    }

    #[async_trait]
    impl UtteranceGenerator for CountingUtter {
        async fn utter(&self, _i: &Initiative) -> Result<String, String> {
            *self.calls.lock().unwrap() += 1;
            Ok("LLM 渲染".to_string())
        }
    }

    #[tokio::test]
    async fn throttled_utterance_respects_min_interval() {
        let inner = CountingUtter {
            calls: Mutex::new(0),
        };
        let t = ThrottledUtterance::new(inner, Duration::from_secs(3600));
        let i = sample_initiative();
        // 第一次: 真调 LLM
        let r1 = t.utter(&i).await.unwrap();
        assert_eq!(r1, "LLM 渲染");
        // 间隔未到: 节流 → 诚实原文兜底 (不装「已润色」)
        let r2 = t.utter(&i).await.unwrap();
        assert!(r2.contains("置信度 50%") && r2.contains("概率"));
        // inner 只被调了 1 次
        assert_eq!(*t.inner.calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn throttled_utterance_backs_off_on_failure() {
        struct FailingUtter;
        #[async_trait]
        impl UtteranceGenerator for FailingUtter {
            async fn utter(&self, _i: &Initiative) -> Result<String, String> {
                Err("MiniMax suppressed".to_string())
            }
        }
        let t = ThrottledUtterance::new(FailingUtter, Duration::from_secs(60));
        let i = sample_initiative();
        // 第一次失败 → 错误如实上抛
        let r1 = t.utter(&i).await;
        assert!(r1.is_err());
        // 退避生效: 立刻再调 → 节流兜底 (不再打 LLM)
        let r2 = t.utter(&i).await.unwrap();
        assert!(r2.contains("置信度"));
        // 退避已增长 (>= min_interval)
        let s = t.state.lock().unwrap();
        assert!(s.backoff >= Duration::from_secs(60));
    }

    #[test]
    fn lark_sink_from_env_missing_credential_errs() {
        let r = LarkSink::from_env_with(|_| None);
        assert!(r.is_err());
        let msg = r.unwrap_err();
        assert!(msg.contains("APEIRETH_LARK_APP_ID"), "提示应点名缺的变量: {msg}");
    }

    #[test]
    fn lark_sink_from_env_complete_ok() {
        let env = |k: &str| match k {
            "APEIRETH_LARK_APP_ID" => Some("cli_test".into()),
            "APEIRETH_LARK_APP_SECRET" => Some("secret".into()),
            "APEIRETH_LARK_RECEIVE_ID" => Some("ou_test".into()),
            _ => None,
        };
        let sink = LarkSink::from_env_with(env).unwrap();
        assert_eq!(sink.receive_id, "ou_test");
        assert_eq!(sink.client.config().app_id, "cli_test");
        assert_eq!(
            sink.client.config().base_url,
            apeireth_lark::LARK_API_BASE_URL
        );
    }

    #[test]
    fn memory_path_resolution() {
        // 显式 APEIRETH_MEMORY_PATH 优先
        let p = memory_path_from(|k| {
            if k == "APEIRETH_MEMORY_PATH" {
                Some("C:/tmp/x.sqlite".into())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(p, PathBuf::from("C:/tmp/x.sqlite"));
        // APPDATA (Windows)
        let p = memory_path_from(|k| {
            if k == "APPDATA" {
                Some("C:/Users/u/AppData/Roaming".into())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("C:/Users/u/AppData/Roaming/apeireth/memory.sqlite")
        );
        // 仅 HOME → XDG 默认 .local/share
        let p = memory_path_from(|k| {
            if k == "HOME" {
                Some("/home/u".into())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(p, PathBuf::from("/home/u/.local/share/apeireth/memory.sqlite"));
        // 全缺 → Err
        assert!(memory_path_from(|_| None).is_err());
    }

    #[test]
    fn open_memory_store_at_creates_parent_and_opens() {
        let dir = std::env::temp_dir().join(format!("apeireth-daemon-test-{}", std::process::id()));
        let path = dir.join("nested").join("memory.sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        let store = open_memory_store_at(&path).unwrap();
        assert!(path.exists(), "真 SQLite 文件应落盘");
        drop(store); // 打开成功即可, migrations 已跑
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn daemon_assembles() {
        let mut bond = Bond::new();
        bond.evolve(crate::BondStage::Trusted, 0.6);
        let d = CompanionDaemon::new(
            bond,
            Boundaries::default(),
            ConsoleDelivery,
            EmptyContext,
            "me",
            Duration::from_secs(60),
        );
        assert_eq!(d.subject, "me");
        // 保留引用: NoopDelivery 也能组装
        let _ = CompanionDaemon::new(
            Bond::new(),
            Boundaries::default(),
            NoopDelivery,
            EmptyContext,
            "x",
            Duration::from_secs(1),
        );
    }

    #[tokio::test]
    async fn daemon_dream_resets_quiet_on_user_activity() {
        use crate::dream::DreamScheduler;
        use apeireth_core::clock::VirtualClock;
        use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
        use chrono::TimeZone;

        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        for (i, c) in ["线代: 特征值卡住", "高数: 换元忘换 dx"].iter().enumerate() {
            store
                .put_episode(&CoreEpisode {
                    id: format!("mem-{i}").into(),
                    timestamp: 1 + i as i64,
                    role: "assistant".into(),
                    content: c.to_string(),
                    session_id: "me".into(),
                })
                .unwrap();
        }
        let dream = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()))
            .with_quiet_threshold(Duration::from_secs(3600)); // 1h 安静才做梦
        let mut d = CompanionDaemon::new(
            Bond::new(),
            Boundaries::default(),
            NoopDelivery,
            EmptyContext,
            "me",
            Duration::from_secs(60),
        )
        .with_dream(dream);

        // 30 分钟后用户活动 → 重置安静期; 再过 30min step → 不做梦 (quiet 只过了 30min < 1h)
        vc.advance(chrono::Duration::minutes(30));
        d.on_user_message(vc.current());
        vc.advance(chrono::Duration::minutes(30));
        d.step().await;
        let eps = store.recent_episodes("me", 100).unwrap();
        assert!(
            !eps.iter().any(|e| e.id.starts_with("mem-dream-")),
            "用户活动应重置做梦安静期, 30min 后不应做梦"
        );

        // 不活动 61min → 做梦 (合并写回 1 条)
        vc.advance(chrono::Duration::minutes(61));
        d.step().await;
        let eps = store.recent_episodes("me", 100).unwrap();
        assert!(
            eps.iter().any(|e| e.id.starts_with("mem-dream-")),
            "长时间无互动应触发做梦"
        );
    }
}
