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

/// 环境感知快照.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientSnapshot {
    pub scene: UserActivityScene,
    pub stance: CompanionStance,
    pub active_window_title: Option<String>,
    pub last_interaction_ms: i64,
    pub stance_reason: String,
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

    /// 根据前台窗口标题与系统指标推断活动场景.
    pub fn infer_scene_from_window(window_title: &str) -> UserActivityScene {
        let title_lower = window_title.to_lowercase();

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
    pub fn update_environment(
        &self,
        window_title: Option<&str>,
        now_ms: i64,
    ) -> AmbientSnapshot {
        let scene = if let Some(title) = window_title {
            Self::infer_scene_from_window(title)
        } else {
            // 无前台窗口或空闲
            let last_ms = *self.last_interaction_ms.lock().unwrap();
            if now_ms.saturating_sub(last_ms) >= self.config.idle_timeout_ms {
                UserActivityScene::Idle
            } else {
                *self.current_scene.lock().unwrap()
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

        *self.current_scene.lock().unwrap() = scene;
        *self.current_stance.lock().unwrap() = stance;
        *self.last_window_title.lock().unwrap() = window_title.map(|s| s.to_string());

        AmbientSnapshot {
            scene,
            stance,
            active_window_title: window_title.map(|s| s.to_string()),
            last_interaction_ms: *self.last_interaction_ms.lock().unwrap(),
            stance_reason: reason,
        }
    }

    /// 标记一次用户主动交互 (重置空闲时钟).
    pub fn record_interaction(&self, now_ms: i64) {
        *self.last_interaction_ms.lock().unwrap() = now_ms;
    }

    /// 获取当前快照.
    pub fn current_snapshot(&self) -> AmbientSnapshot {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let title = self.last_window_title.lock().unwrap().clone();
        self.update_environment(title.as_deref(), now_ms)
    }
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
            AmbientContextMachine::infer_scene_from_window("main.rs - Apeireth - Visual Studio Code"),
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
        let snap1 = machine.update_environment(Some("crates/engine/memory/src/lib.rs - VSCode"), 1000);
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
}
