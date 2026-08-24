use crate::expression_mapper::Live2dExpression;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Live2dMotion {
    Idle,
    TapBody,
    TapHead,
    Nod,
    ShakeHead,
    Speaking,
    Stretch,
    Sleep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Live2dConfig {
    pub model_path: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub scale: f32,
    pub auto_blink: bool,
    pub auto_breath: bool,
    pub look_at_mouse: bool,
}

impl Default for Live2dConfig {
    fn default() -> Self {
        Self {
            model_path: "assets/live2d/hiyori/hiyori_pro_t10.model3.json".into(),
            canvas_width: 800,
            canvas_height: 1000,
            scale: 0.25,
            auto_blink: true,
            auto_breath: true,
            look_at_mouse: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Live2dParams {
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub eye_l_open: f32,
    pub eye_r_open: f32,
    pub eye_ball_x: f32,
    pub eye_ball_y: f32,
    pub mouth_open_y: f32,
    pub mouth_form: f32,
    pub body_angle_x: f32,
    pub breath: f32,
}

impl Default for Live2dParams {
    fn default() -> Self {
        Self {
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            eye_l_open: 1.0,
            eye_r_open: 1.0,
            eye_ball_x: 0.0,
            eye_ball_y: 0.0,
            mouth_open_y: 0.0,
            mouth_form: 0.0,
            body_angle_x: 0.0,
            breath: 0.0,
        }
    }
}

pub struct Live2dController {
    pub config: Live2dConfig,
    pub current_expression: Live2dExpression,
    pub current_motion: Live2dMotion,
    pub params: Live2dParams,
}

impl Live2dController {
    pub fn new(config: Live2dConfig) -> Self {
        Self {
            config,
            current_expression: Live2dExpression::Neutral,
            current_motion: Live2dMotion::Idle,
            params: Live2dParams::default(),
        }
    }

    pub fn set_expression(&mut self, expr: Live2dExpression) {
        self.current_expression = expr.clone();
        match expr {
            Live2dExpression::Joy | Live2dExpression::Happy => {
                self.params.mouth_form = 1.0;
                self.params.eye_l_open = 0.9;
                self.params.eye_r_open = 0.9;
            }
            Live2dExpression::Shy => {
                self.params.mouth_form = 0.5;
                self.params.angle_z = -5.0;
                self.params.angle_y = -3.0;
            }
            Live2dExpression::Thinking => {
                self.params.angle_y = 10.0;
                self.params.angle_z = 8.0;
                self.params.eye_ball_y = 0.6;
            }
            Live2dExpression::Drowsy => {
                self.params.eye_l_open = 0.3;
                self.params.eye_r_open = 0.3;
                self.params.angle_y = -8.0;
            }
            Live2dExpression::Sleeping => {
                self.params.eye_l_open = 0.0;
                self.params.eye_r_open = 0.0;
                self.params.angle_y = -15.0;
            }
            Live2dExpression::Wink => {
                self.params.eye_l_open = 1.0;
                self.params.eye_r_open = 0.0;
                self.params.mouth_form = 0.8;
            }
            _ => {
                self.params.mouth_form = 0.0;
                self.params.eye_l_open = 1.0;
                self.params.eye_r_open = 1.0;
            }
        }
    }

    pub fn update_lip_sync(&mut self, mouth_open: f32, mouth_form: f32) {
        self.params.mouth_open_y = mouth_open.clamp(0.0, 1.0);
        self.params.mouth_form = mouth_form.clamp(-1.0, 1.0);
    }
}
