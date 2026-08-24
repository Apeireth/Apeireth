use crate::expression_mapper::VrmBlendShapes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrmConfig {
    pub model_path: String,
    pub enable_spring_bone: bool,
    pub enable_look_at: bool,
    pub camera_fov: f32,
    pub light_intensity: f32,
}

impl Default for VrmConfig {
    fn default() -> Self {
        Self {
            model_path: "assets/vrm/avatar.vrm".into(),
            enable_spring_bone: true,
            enable_look_at: true,
            camera_fov: 30.0,
            light_intensity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookAtTarget {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct VrmController {
    pub config: VrmConfig,
    pub blendshapes: VrmBlendShapes,
    pub look_at: LookAtTarget,
}

impl VrmController {
    pub fn new(config: VrmConfig) -> Self {
        Self {
            config,
            blendshapes: VrmBlendShapes::default(),
            look_at: LookAtTarget { x: 0.0, y: 1.4, z: 0.0 },
        }
    }

    pub fn set_blendshapes(&mut self, bs: VrmBlendShapes) {
        self.blendshapes = bs;
    }

    pub fn set_look_at(&mut self, x: f32, y: f32, z: f32) {
        self.look_at = LookAtTarget { x, y, z };
    }
}
