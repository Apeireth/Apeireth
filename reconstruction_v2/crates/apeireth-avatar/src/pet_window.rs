use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetWindowConfig {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub always_on_top: bool,
    pub transparent: bool,
    pub click_through: bool,
    pub title: String,
    pub websocket_port: u16,
}

impl Default for PetWindowConfig {
    fn default() -> Self {
        Self {
            width: 400,
            height: 600,
            x: 1300,
            y: 400,
            always_on_top: true,
            transparent: true,
            click_through: false,
            title: "Apeireth Desktop Companion".into(),
            websocket_port: 9002,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetWindowState {
    pub is_visible: bool,
    pub is_dragging: bool,
    pub is_speaking: bool,
    pub current_avatar_type: String, // "live2d" or "vrm"
    pub current_model_name: String,
    pub position_x: i32,
    pub position_y: i32,
}

impl Default for PetWindowState {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_dragging: false,
            is_speaking: false,
            current_avatar_type: "live2d".into(),
            current_model_name: "hiyori".into(),
            position_x: 1300,
            position_y: 400,
        }
    }
}
