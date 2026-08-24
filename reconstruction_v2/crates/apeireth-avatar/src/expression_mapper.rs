use apeireth_companion::emotion::Pad;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Live2dExpression {
    Neutral,
    Joy,
    Happy,
    Shy,
    Thinking,
    Surprised,
    Drowsy,
    Sleeping,
    Sad,
    Anger,
    Wink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VrmBlendShapes {
    pub happy: f32,
    pub angry: f32,
    pub sad: f32,
    pub relaxed: f32,
    pub surprised: f32,
    pub blink: f32,
    pub blink_l: f32,
    pub blink_r: f32,
    pub look_up: f32,
    pub look_down: f32,
}

impl Default for VrmBlendShapes {
    fn default() -> Self {
        Self {
            happy: 0.0,
            angry: 0.0,
            sad: 0.0,
            relaxed: 0.5,
            surprised: 0.0,
            blink: 0.0,
            blink_l: 0.0,
            blink_r: 0.0,
            look_up: 0.0,
            look_down: 0.0,
        }
    }
}

pub struct ExpressionMapper;

impl ExpressionMapper {
    /// Maps PAD emotional vector and sleep pressure S(t) into Live2D Expression
    pub fn map_pad_to_live2d(pad: &Pad, sleep_pressure: f64) -> Live2dExpression {
        if sleep_pressure > 0.85 {
            return Live2dExpression::Sleeping;
        }
        if sleep_pressure > 0.70 {
            return Live2dExpression::Drowsy;
        }

        let p = pad.pleasure;
        let a = pad.arousal;
        let d = pad.dominance;

        if p > 0.7 && a > 0.6 {
            Live2dExpression::Joy
        } else if p > 0.6 && d < 0.4 {
            Live2dExpression::Shy
        } else if p > 0.5 {
            Live2dExpression::Happy
        } else if a > 0.7 && p < 0.4 {
            Live2dExpression::Surprised
        } else if p < 0.3 && a > 0.6 {
            Live2dExpression::Anger
        } else if p < 0.4 && a < 0.4 {
            Live2dExpression::Sad
        } else if a > 0.5 && d > 0.5 {
            Live2dExpression::Thinking
        } else {
            Live2dExpression::Neutral
        }
    }

    /// Maps PAD emotion vector and sleep pressure into VRM 3D blendshapes weights (0.0..1.0)
    pub fn map_pad_to_vrm(pad: &Pad, sleep_pressure: f64) -> VrmBlendShapes {
        let mut bs = VrmBlendShapes::default();

        if sleep_pressure > 0.85 {
            bs.blink = 1.0;
            bs.relaxed = 1.0;
            return bs;
        } else if sleep_pressure > 0.70 {
            bs.blink = 0.6;
            bs.relaxed = 0.8;
            bs.look_down = 0.4;
        }

        let p = pad.pleasure as f32;
        let a = pad.arousal as f32;

        if p > 0.5 {
            bs.happy = ((p - 0.5) * 2.0).clamp(0.0, 1.0);
            bs.relaxed = (1.0 - (p - 0.5)).clamp(0.0, 1.0);
        } else {
            bs.sad = ((0.5 - p) * 2.0).clamp(0.0, 1.0);
            if a > 0.6 {
                bs.angry = ((a - 0.6) * 2.5).clamp(0.0, 1.0);
            }
        }

        if a > 0.75 {
            bs.surprised = ((a - 0.75) * 4.0).clamp(0.0, 1.0);
        }

        bs
    }
}
