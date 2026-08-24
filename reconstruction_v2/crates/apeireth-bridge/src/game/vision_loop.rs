use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameActionPolicy {
    MinecraftSurvival,
    FactorioAutomation,
    CustomPolicy(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLoopConfig {
    pub target_window_title: String,
    pub frame_rate_fps: u32,
    pub policy: GameActionPolicy,
    pub auto_pause_on_user_input: bool,
}

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            target_window_title: "Minecraft".into(),
            frame_rate_fps: 2,
            policy: GameActionPolicy::MinecraftSurvival,
            auto_pause_on_user_input: true,
        }
    }
}

pub struct GameVisionLoop {
    pub config: GameLoopConfig,
    pub is_running: bool,
    pub frame_count: u64,
}

impl GameVisionLoop {
    pub fn new(config: GameLoopConfig) -> Self {
        Self {
            config,
            is_running: false,
            frame_count: 0,
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Evaluates a single vision frame and returns the next keyboard/mouse action
    pub fn tick_decision(&mut self, screen_brightness: f32) -> Option<String> {
        if !self.is_running {
            return None;
        }
        self.frame_count += 1;

        match self.config.policy {
            GameActionPolicy::MinecraftSurvival => {
                if screen_brightness < 0.2 {
                    Some("place_torch".into()) // Dark night: place torch
                } else {
                    Some("mine_forward".into())
                }
            }
            GameActionPolicy::FactorioAutomation => {
                Some("check_conveyor_belt".into())
            }
            GameActionPolicy::CustomPolicy(_) => {
                Some("observe".into())
            }
        }
    }
}
