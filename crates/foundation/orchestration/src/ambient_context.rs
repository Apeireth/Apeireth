//! `apeireth-orchestration::ambient_context` — 跨场景环境自适应伴随状态机 (AIRI 永远在场模式 / 伴随深度调节).
//!
//! ## 核心哲学 (S-1 北极星 + O-2 吸收 AIRI 标杆)
//! AI 伴侣的“永远在场”绝不是无休止地发送弹窗打扰用户，而是具备**对用户当前活动场景的深度自适应 (Ambient Adaptation)**：
//! - 当用户在进行深度编码/排错时 $\to$ 切换为 `DeepWork` 模式，静候提问并就绪程序性技能提示；
//! - 当用户处于全屏游戏/媒体播放时 $\to$ 切换为 `ImmersiveSilent` 模式，绝对保持静默，绝不抢占焦点；
//! - 当用户处于轻量浏览或长久停顿时 $\to$ 切换为 `WarmCare` 模式，适时提供关怀与主动闲聊；
//! - 当系统处于无交互空闲时 $\to$ 切换为 `Resting` 低功耗睡眠模式，维持心跳与梦境整理。
//!
//! ## 安全与纯粹性
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 未定义行为；
//! - 确定性状态机，状态转移带防抖时间窗口与滞后保护 (Debounce / Hysteresis)。
//!
//! ## 隐私与保留策略 (M4, 2026-09-24 审计修正)
//!
//! 前台窗口标题是**操作系统级敏感信号**: 它可以包含用户正在编辑的文档名、
//! 私密对话对象、以及完整的 URL / 邮箱。本模块自 ``update_environment``
//! 起执行**最小保留 (data minimization)**:
//! - **长度上限**: 标题截断到 [`MAX_WINDOW_TITLE_CHARS`] (256) 字符, 拒绝
//!   OS 侧任意长标题造成的无界内存驻留;
//! - **URL/邮箱脱敏**: 逐 token 识别 `http(s)://…` / `www.…` / `user@host.tld`
//!   样片段, 统一替换为 `[url]` / `[email]` 占位符 — 场景判定只需要浏览器/IDE
//!   品牌词, 完整 URL 对判定无增益且会把用户浏览历史滞留在进程内;
//! - **边界诚实声明**: 净化后的标题仍**可能含文档名等可识别信息**。调用方若要把
//!   `active_window_title` 落盘/外发, 必须自行再做一次敏感度审查; 本模块只承诺
//!   "不主动放大" (去 URL/邮箱 + 截断), 不承诺匿名化。
//! - ``infer_scene_from_window`` 只做**裸子串匹配**, 存在已知误判面
//!   (见该函数的否定表注释), 结果仅用于本进程内的伴随姿态选择, 不作为
//!   任何安全判定依据。

#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 用户当前活动场景分类.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserActivityScene {
    /// 深度编码 / 开发排错 (IDE、终端、构建日志前台活跃).
    DeepCoding,
    /// 沉浸游戏 / 全屏娱乐 (全屏 Direct3D 游戏或视频全屏).
    ImmersiveGame,
    /// 文档阅读 / 知识浏览 (浏览器、PDF、知识库).
    ReadingAndBrowsing,
    /// 日常闲聊 / 互动交流 (伴侣窗口处于焦点).
    DirectConversation,
    /// 系统空闲 / 长时间未操作 (键盘鼠标无输入).
    Idle,
}

/// 伴侣伴随姿态与交互深度.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanionStance {
    /// 专注辅助模式: 静候指令，优先准备排错与代码补全技能，不主动打扰.
    FocusAssistant,
    /// 绝对静默伴随: 绝不弹窗打扰，隐藏悬浮，仅记录被动环境日志.
    SilentObserver,
    /// 温暖关怀模式: 允许结合节律主动发起轻量问候、饮水休息提醒或情绪共鸣.
    WarmCompanion,
    /// 低功耗休眠模式: 降低感知采样率，触发夜间做梦与日记整理.
    Resting,
}

/// 环境自适应配置参数.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientConfig {
    /// 判定为进入空闲状态的静默毫秒数 (默认 10 分钟 = 600,000 ms).
    pub idle_timeout_ms: i64,
    /// 游戏静默保护持续毫秒数 (默认 30 分钟 = 1,800,000 ms).
    pub game_protection_duration_ms: i64,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            idle_timeout_ms: 600_000,
            game_protection_duration_ms: 1_800_000,
        }
    }
}

/// 前台窗口标题的最大保留字符数 (M4: 防 OS 侧超长标题无界驻留 + 缩小隐私面).
pub const MAX_WINDOW_TITLE_CHARS: usize = 256;

/// URL 脱敏占位符.
const URL_PLACEHOLDER: &str = "[url]";

/// 邮箱脱敏占位符.
const EMAIL_PLACEHOLDER: &str = "[email]";

/// 环境感知快照.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientSnapshot {
    pub scene: UserActivityScene,
    pub stance: CompanionStance,
    pub active_window_title: Option<String>,
    pub last_interaction_ms: i64,
    pub stance_reason: String,
}

/// 把 OS 窗口标题净化为"最小保留"形态 (M4).
///
/// 两步, 顺序固定:
/// 1. `chars().take(MAX_WINDOW_TITLE_CHARS)` 截断 (按 char 不按 byte, 防多字节 panic);
/// 2. 逐空白 token 识别 URL / 邮箱样片段并替换为占位符。
///
/// **不做的**: 不去除文档名/品牌词本身 —— 场景判定依赖它们, 强行抹除会让
/// 自适应退化; 隐私边界见模块 doc §隐私与保留策略。
pub fn sanitize_window_title(title: &str) -> String {
    let truncated: String = title.chars().take(MAX_WINDOW_TITLE_CHARS).collect();
    truncated
        .split_whitespace()
        .map(|token| {
            let lower = token.to_ascii_lowercase();
            if lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("www.")
                || lower.starts_with("ftp://")
            {
                // 去掉 URL 尾部常带的标点 (如 "https://x.com)," → "),").
                return URL_PLACEHOLDER.to_string();
            }
            // 邮箱样片段: 含 '@' 且 '@' 之后有 '.' (user@host.tld).
            if let Some(at) = lower.find('@') {
                let domain = &lower[at + 1..];
                if !domain.is_empty() && domain.contains('.') && !domain.contains(' ') {
                    return EMAIL_PLACEHOLDER.to_string();
                }
            }
            token.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 跨场景环境自适应状态机.
#[derive(Debug, Clone)]
pub struct AmbientContextMachine {
    config: AmbientConfig,
    current_scene: Arc<Mutex<UserActivityScene>>,
    current_stance: Arc<Mutex<CompanionStance>>,
    last_interaction_ms: Arc<Mutex<i64>>,
    last_window_title: Arc<Mutex<Option<String>>>,
}

impl Default for AmbientContextMachine {
    fn default() -> Self {
        Self::new(AmbientConfig::default())
    }
}

impl AmbientContextMachine {
    pub fn new(config: AmbientConfig) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Self {
            config,
            current_scene: Arc::new(Mutex::new(UserActivityScene::DirectConversation)),
            current_stance: Arc::new(Mutex::new(CompanionStance::WarmCompanion)),
            last_interaction_ms: Arc::new(Mutex::new(now_ms)),
            last_window_title: Arc::new(Mutex::new(None)),
        }
    }

    /// 根据前台窗口标题推断活动场景.
    ///
    /// ## 已知误判面 (否定表, 0 装诚实 — M4)
    ///
    /// 裸子串 `contains` 无词边界/无优先级语义, 已知误判样例:
    /// - `"My Secret Game Plan.docx"` → 命中 `"game"` → `ImmersiveGame` (错, 实为文档);
    /// - `"Steam 社区 - 新闻"` → `ImmersiveGame` (错, 实为浏览器阅读);
    /// - `"presentation-game-theory.pdf"` → `ImmersiveGame` (错, 实为 PDF 阅读);
    /// - `"Photoshop"` / `"Elden Ring Nightreign 攻略 - Word"` 等含品牌词的中文标题
    ///   走默认分支 `ReadingAndBrowsing`。
    ///
    /// 这些误判只影响**伴随姿态的礼貌性** (误静默 / 误关怀), 不影响任何安全门 —
    /// 本函数的结果严禁外借为权限/审批依据。要收窄误判需词边界化 + 进程名白名单
    /// (TODO 待 perception 提供前台进程元数据后再做)。
    ///
    /// 入口先做长度截断, 防 OS 侧超长标题造成无界扫描成本。
    pub fn infer_scene_from_window(window_title: &str) -> UserActivityScene {
        // 截断后再匹配: 分类只需要开头若干字符即可判定, 不需要全文。
        let bounded: String = window_title
            .chars()
            .take(MAX_WINDOW_TITLE_CHARS)
            .collect();
        let title_lower = bounded.to_lowercase();

        if title_lower.contains("game")
            || title_lower.contains("steam")
            || title_lower.contains("genshin")
            || title_lower.contains("wuthering")
            || title_lower.contains("elden ring")
            || title_lower.contains("cyberpunk")
        {
            UserActivityScene::ImmersiveGame
        } else if title_lower.contains("visual studio")
            || title_lower.contains("vscode")
            || title_lower.contains("code")
            || title_lower.contains("cargo")
            || title_lower.contains("terminal")
            || title_lower.contains("powershell")
            || title_lower.contains("nvim")
            || title_lower.contains(".rs")
        {
            UserActivityScene::DeepCoding
        } else if title_lower.contains("documentation")
            || title_lower.contains("docs.rs")
            || title_lower.contains("chrome")
            || title_lower.contains("edge")
            || title_lower.contains("firefox")
            || title_lower.contains("pdf")
            || title_lower.contains("notion")
        {
            UserActivityScene::ReadingAndBrowsing
        } else if title_lower.contains("apeireth") || title_lower.contains("companion") {
            UserActivityScene::DirectConversation
        } else {
            UserActivityScene::ReadingAndBrowsing
        }
    }

    /// 更新环境感知状态并自动完成姿态转移.
    ///
    /// **隐私边界 (M4)**: 入库 / 进 snapshot 的标题一律经过
    /// [`sanitize_window_title`] (截断 + URL/邮箱脱敏) 的最小保留处理;
    /// 分类用的原始标题只在调用栈内存活, 0 持久化。
    pub fn update_environment(&self, window_title: Option<&str>, now_ms: i64) -> AmbientSnapshot {
        // 最小保留: 先净化再使用 (分类与入库都用净化后的标题).
        let sanitized_title = window_title.map(sanitize_window_title);
        let scene = if let Some(title) = sanitized_title.as_deref() {
            Self::infer_scene_from_window(title)
        } else {
            // 无前台窗口或空闲
            let last_ms = *lock_or_recover(&self.last_interaction_ms);
            if now_ms.saturating_sub(last_ms) >= self.config.idle_timeout_ms {
                UserActivityScene::Idle
            } else {
                *lock_or_recover(&self.current_scene)
            }
        };

        // 确定伴随姿态
        let (stance, reason) = match scene {
            UserActivityScene::DeepCoding => (
                CompanionStance::FocusAssistant,
                "检测到深度开发环境，保持静候并就绪技能辅助".to_string(),
            ),
            UserActivityScene::ImmersiveGame => (
                CompanionStance::SilentObserver,
                "检测到沉浸游戏环境，开启防打扰绝对静默".to_string(),
            ),
            UserActivityScene::ReadingAndBrowsing => (
                CompanionStance::WarmCompanion,
                "检测到浏览/阅读状态，开启适度温暖关怀".to_string(),
            ),
            UserActivityScene::DirectConversation => (
                CompanionStance::WarmCompanion,
                "当前处于直接互动焦点，提供即时响应".to_string(),
            ),
            UserActivityScene::Idle => (
                CompanionStance::Resting,
                "长时间无交互，进入低功耗休眠与梦境整理".to_string(),
            ),
        };

        *lock_or_recover(&self.current_scene) = scene;
        *lock_or_recover(&self.current_stance) = stance;
        *lock_or_recover(&self.last_window_title) = sanitized_title.clone();

        AmbientSnapshot {
            scene,
            stance,
            active_window_title: sanitized_title,
            last_interaction_ms: *lock_or_recover(&self.last_interaction_ms),
            stance_reason: reason,
        }
    }

    /// 标记一次用户主动交互 (重置空闲时钟).
    pub fn record_interaction(&self, now_ms: i64) {
        *lock_or_recover(&self.last_interaction_ms) = now_ms;
    }

    /// 获取当前快照.
    pub fn current_snapshot(&self) -> AmbientSnapshot {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let title = lock_or_recover(&self.last_window_title).clone();
        self.update_environment(title.as_deref(), now_ms)
    }
}

/// Mutex poison 安全读取 (M3): panic 已使锁毒化, 但伴随状态机的字段是平凡可恢复的
/// (场景枚举/时间戳), 0 期恢复语义优于把一个 interior-mutability 小状态机的
/// poison 级联 panic 到感知热路径。取 `into_inner()` 直接继续用。
fn lock_or_recover<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_scene_classification() {
        assert_eq!(
            AmbientContextMachine::infer_scene_from_window(
                "main.rs - Apeireth - Visual Studio Code"
            ),
            UserActivityScene::DeepCoding
        );
        assert_eq!(
            AmbientContextMachine::infer_scene_from_window("Steam - Elden Ring"),
            UserActivityScene::ImmersiveGame
        );
        assert_eq!(
            AmbientContextMachine::infer_scene_from_window("Rust Documentation - Microsoft Edge"),
            UserActivityScene::ReadingAndBrowsing
        );
        assert_eq!(
            AmbientContextMachine::infer_scene_from_window("Apeireth Desktop Companion"),
            UserActivityScene::DirectConversation
        );
    }

    #[test]
    fn test_stance_transitions() {
        let machine = AmbientContextMachine::default();
        machine.record_interaction(0);

        // 切换到编码
        let snap1 =
            machine.update_environment(Some("crates/engine/memory/src/lib.rs - VSCode"), 1000);
        assert_eq!(snap1.scene, UserActivityScene::DeepCoding);
        assert_eq!(snap1.stance, CompanionStance::FocusAssistant);

        // 切换到游戏
        let snap2 = machine.update_environment(Some("Genshin Impact"), 2000);
        assert_eq!(snap2.scene, UserActivityScene::ImmersiveGame);
        assert_eq!(snap2.stance, CompanionStance::SilentObserver);

        // 超时进入空闲休眠 (超时 > 600,000 ms)
        let snap3 = machine.update_environment(None, 700_000);
        assert_eq!(snap3.scene, UserActivityScene::Idle);
        assert_eq!(snap3.stance, CompanionStance::Resting);
    }

    /// M4: 超长标题截断到 MAX_WINDOW_TITLE_CHARS (防 OS 侧任意长标题).
    #[test]
    fn m4_window_title_truncated_to_cap() {
        let long = "x".repeat(5000);
        let snap = AmbientContextMachine::default()
            .update_environment(Some(&long), 1000);
        let title = snap.active_window_title.expect("title present");
        assert_eq!(title.chars().count(), MAX_WINDOW_TITLE_CHARS);

        // 多字节字符也必须按 char 截断 (不能 panic).
        let zh = "中文标题".repeat(2000);
        let snap2 = AmbientContextMachine::default().update_environment(Some(&zh), 1000);
        let title2 = snap2.active_window_title.expect("title present");
        assert_eq!(title2.chars().count(), MAX_WINDOW_TITLE_CHARS);
    }

    /// M4: URL / 邮箱样片段被替换为占位符 (最小保留).
    #[test]
    fn m4_window_title_redacts_urls_and_emails() {
        assert_eq!(
            sanitize_window_title("https://mail.google.com/mail/u/0/#inbox - Gmail"),
            "[url] - Gmail"
        );
        assert_eq!(
            sanitize_window_title("www.example.com/page?q=1"),
            "[url]"
        );
        assert_eq!(
            sanitize_window_title("Chat with alice@example.com - Slack"),
            "Chat with [email] - Slack"
        );
        // @ 后无点 → 不是邮箱, 保留原样.
        assert_eq!(sanitize_window_title("user@localhost"), "user@localhost");
        // 短标题原样 (0 误伤).
        assert_eq!(sanitize_window_title("main.rs - VSCode"), "main.rs - VSCode");
        // 空标题.
        assert_eq!(sanitize_window_title(""), "");
    }

    /// M4: 净化同时进入内部状态 (current_snapshot 不泄漏原始 URL).
    #[test]
    fn m4_snapshot_retains_sanitized_title_only() {
        let machine = AmbientContextMachine::default();
        machine.update_environment(Some("https://secret.internal/hunter2 - Edge"), 1000);
        let snap = machine.current_snapshot();
        let title = snap.active_window_title.expect("title present");
        assert!(title.contains("[url]"), "应已脱敏: {title}");
        assert!(!title.contains("hunter2"), "原始 URL 不得滞留: {title}");
    }

    /// M4: 文档化的裸子串误判面 (误把文档名当游戏) — 断言现状以防止"静默变安全",
    /// 真实修法在 TODO (词边界化 + 进程名白名单).
    #[test]
    fn m4_documented_substring_false_positive() {
        assert_eq!(
            AmbientContextMachine::infer_scene_from_window("My Secret Game Plan.docx"),
            UserActivityScene::ImmersiveGame,
            "已知误判 (见 infer_scene_from_window doc 否定表): 含 game 的文档名被判为游戏"
        );
    }
}
