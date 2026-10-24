//! 16 个着色器取值的容器与投递。
//!
//! ## 取值表（顺序与引擎的 uniform 映射表一致）
//!
//! | 取值 | 分量 | 含义 |
//! |---|---|---|
//! | `uRes` | 2 | 画布尺寸（设备像素） |
//! | `uCenter` | 2 | 构图中心（**y 已翻转**） |
//! | `uScale` | 1 | 视区半径（设备像素） |
//! | `uTime` | 1 | 动画时钟（秒） |
//! | `uRot` | 1 | 盘面自转相位 |
//! | `uBright` | 1 | 盘面亮度 |
//! | `uTurb` | 1 | 湍流强度 |
//! | `uPulse` | 1 | 呼吸光强度 |
//! | `uIncl` | 1 | 视角倾角（含鼠标视差） |
//! | `uAzim` | 1 | 视角方位角（含鼠标视差） |
//! | `uDiskAzim` | 1 | 盘面姿态方位角 |
//! | `uDiskIncl` | 1 | 盘面姿态倾角 |
//! | `uAmbStr` | 1 | 行照环境蓝光强度 |
//! | `uEclipse` | 1 | 凌日日食强度 |
//! | `uTlBright` | 1 | 时间线亮度增益 |
//! | `uTlGold` | 1 | 时间线金色占比 |
//!
//! ## 数值口径
//!
//! 本容器里的取值是 **f64**（与生产侧脚本运行时的数值口径相同）；
//! 写进引擎的 uniform 缓冲时**收口**成 f32（着色器取值本来就是单精度）。

use apeireth_ui_gpu::{BindingError, UniformValues};

/// 16 个着色器取值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneUniforms {
    /// 画布尺寸（设备像素）。
    pub res: [f64; 2],
    /// 构图中心（设备像素；**y 已翻转** ⇒ 离底边）。
    pub center: [f64; 2],
    /// 视区半径（设备像素）。
    pub scale: f64,
    /// 动画时钟（秒）。
    pub time: f64,
    /// 盘面自转相位。
    pub rot: f64,
    /// 盘面亮度。
    pub bright: f64,
    /// 湍流强度。
    pub turb: f64,
    /// 呼吸光强度。
    pub pulse: f64,
    /// 视角倾角（含鼠标视差）。
    pub incl: f64,
    /// 视角方位角（含鼠标视差）。
    pub azim: f64,
    /// 盘面姿态方位角。
    pub disk_azim: f64,
    /// 盘面姿态倾角。
    pub disk_incl: f64,
    /// 行照环境蓝光强度。
    pub amb_str: f64,
    /// 凌日日食强度。
    pub eclipse: f64,
    /// 时间线亮度增益。
    pub tl_bright: f64,
    /// 时间线金色占比。
    pub tl_gold: f64,
}

impl SceneUniforms {
    /// 全部取值的（名字, 分量）视图 —— 名字序与引擎的 uniform 映射表一致。
    pub fn entries(&self) -> Vec<(&'static str, Vec<f64>)> {
        vec![
            ("uRes", self.res.to_vec()),
            ("uCenter", self.center.to_vec()),
            ("uScale", vec![self.scale]),
            ("uTime", vec![self.time]),
            ("uRot", vec![self.rot]),
            ("uBright", vec![self.bright]),
            ("uTurb", vec![self.turb]),
            ("uPulse", vec![self.pulse]),
            ("uIncl", vec![self.incl]),
            ("uAzim", vec![self.azim]),
            ("uDiskAzim", vec![self.disk_azim]),
            ("uDiskIncl", vec![self.disk_incl]),
            ("uAmbStr", vec![self.amb_str]),
            ("uEclipse", vec![self.eclipse]),
            ("uTlBright", vec![self.tl_bright]),
            ("uTlGold", vec![self.tl_gold]),
        ]
    }

    /// 单精度视图（写进着色器前的口径）。
    pub fn entries_f32(&self) -> Vec<(&'static str, Vec<f32>)> {
        self.entries()
            .into_iter()
            .map(|(name, components)| {
                (
                    name,
                    components.into_iter().map(|value| value as f32).collect(),
                )
            })
            .collect()
    }

    /// 按名字写进引擎的 uniform 值集合。
    ///
    /// # Errors
    ///
    /// 名字不在引擎的映射表里 → [`BindingError::UnknownUniform`]。
    pub fn write_into(&self, values: &mut UniformValues) -> Result<(), BindingError> {
        for (name, components) in self.entries_f32() {
            values.set_by_name(name, &components)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SceneUniforms {
        SceneUniforms {
            res: [1440.0, 900.0],
            center: [950.4, 495.0],
            scale: 86.4,
            time: 1.2,
            rot: 0.35,
            bright: 0.32,
            turb: 0.12,
            pulse: 0.0,
            incl: 0.3,
            azim: 0.0,
            disk_azim: 0.0,
            disk_incl: 0.3,
            amb_str: 0.0,
            eclipse: 0.0,
            tl_bright: 1.0,
            tl_gold: 1.0,
        }
    }

    #[test]
    fn entries_cover_the_sixteen_names_in_order() {
        let names: Vec<&str> = sample().entries().iter().map(|(name, _)| *name).collect();
        assert_eq!(names.len(), 16);
        assert_eq!(names[0], "uRes");
        assert_eq!(names[1], "uCenter");
        assert_eq!(names[15], "uTlGold");
        assert_eq!(
            names,
            apeireth_ui_gpu::SHADER_UNIFORM_NAMES.to_vec(),
            "名字序必须与引擎的映射表一致"
        );
    }

    #[test]
    fn vector_uniforms_carry_two_components_and_scalars_one() {
        for (name, components) in sample().entries() {
            let expected = if name == "uRes" || name == "uCenter" {
                2
            } else {
                1
            };
            assert_eq!(components.len(), expected, "{name} 的分量数不对");
        }
    }

    #[test]
    fn write_into_binds_every_uniform_by_name() {
        let uniforms = sample();
        let mut values = UniformValues::new();
        uniforms.write_into(&mut values).expect("写入成功");
        assert_eq!(values.assigned_bindings().len(), 16);
        assert_eq!(values.get(0), Some(&[1440.0f32, 900.0][..]));
        assert_eq!(values.get(1), Some(&[950.4f32, 495.0][..]));
        assert_eq!(values.get(15), Some(&[1.0f32][..]));
    }

    #[test]
    fn f64_values_are_narrowed_to_single_precision_at_the_boundary() {
        let mut uniforms = sample();
        uniforms.scale = 59.83743867835388_f64;
        let entries = uniforms.entries_f32();
        let scale = entries
            .iter()
            .find(|(name, _)| *name == "uScale")
            .map(|(_, components)| components[0])
            .expect("uScale");
        assert_eq!(scale, 59.83743867835388_f64 as f32);
        assert_ne!(
            scale as f64, 59.83743867835388_f64,
            "f32 口径与 f64 口径必然不同"
        );
    }
}
