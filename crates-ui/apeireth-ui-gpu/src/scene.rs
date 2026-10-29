//! 场景参数与 uniform：状态 → 像素的**纯计算**部分。
//!
//! 职责边界（设计文档 §3.3.4）：只做「状态 → uniform 缓冲布局」，**不改着色器
//! 数学**。P0 骨架阶段这里只落可 native 单测的纯计算（几何量与轨道速度），
//! 真正的绘制待 wgpu 后端接入（P0 探测结论）。
//!
//! 物理常数（SI 口径，见 `docs/adr/0015-pure-rust-ui-engine.md` §8 的实测出处）：
//! - 引力常数 `G = 6.674e-11 m³/(kg·s²)`；
//! - 太阳质量 `M☉ = 1.989e30 kg`；
//! - 光速 `c = 299_792.458 km/s`。

/// 引力常数（m³/(kg·s²)）。
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674e-11;

/// 太阳质量（kg）。
pub const SOLAR_MASS: f64 = 1.989e30;

/// 光速（km/s）。
pub const SPEED_OF_LIGHT: f64 = 299_792.458;

/// 场景**物理量**（P0 留下的场景描述值）。
///
/// ⚠️ **口径更正（2026-10-24）**：本类型**不是**着色器的 uniform 输入。
/// 着色器实际吃的是 **16 个 uniform**（[`crate::bindings::UniformValues`]，
/// 按名字写）。本类型描述的是**场景的物理状态**（相机距离 / 盘半径 / 场景时间），
/// 而「物理量 → 16 个着色器 uniform」这一步**依赖相机 / 主题 / 存在态等宿主侧语义**
/// ⇒ **引擎按设计不碰它**（引擎里不出现业务名词）⇒ 该映射由**宿主**完成。
///
/// ⇒ 本类型保留为**场景描述值**（可被宿主用来算那 16 个 uniform），
/// **不是** [`crate::context::GpuContext`] 的参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneUniforms {
    /// 相机到场景中心的距离（km）。
    pub camera_distance_km: f64,
    /// 吸积盘内半径（km）。
    pub disk_inner_radius_km: f64,
    /// 吸积盘外半径（km）。
    pub disk_outer_radius_km: f64,
    /// 场景时间（秒）。
    pub time_seconds: f64,
}

impl Default for SceneUniforms {
    fn default() -> Self {
        Self {
            camera_distance_km: 1.0e7,
            disk_inner_radius_km: 0.0,
            disk_outer_radius_km: 0.0,
            time_seconds: 0.0,
        }
    }
}

/// 黑洞场景参数（纯计算，不碰着色器）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackHoleParams {
    /// 中心天体质量（太阳质量为单位）。
    pub mass_solar: f64,
    /// 自转参数（无量纲，0 = 不自转）。
    pub spin: f64,
}

impl BlackHoleParams {
    /// 建一组场景参数；质量必须为正。
    ///
    /// # Errors
    ///
    /// 质量非正或自转越界时返回错误（不静默夹取，避免产出无意义的画面）。
    pub fn new(mass_solar: f64, spin: f64) -> Result<Self, SceneError> {
        if !mass_solar.is_finite() || mass_solar <= 0.0 {
            return Err(SceneError::InvalidMass(mass_solar));
        }
        if !spin.is_finite() || spin.abs() > 1.0 {
            return Err(SceneError::InvalidSpin(spin));
        }
        Ok(Self { mass_solar, spin })
    }

    /// 史瓦西半径（km）：`r_s = 2GM/c²`。
    pub fn schwarzschild_radius_km(&self) -> f64 {
        let mass_kg = self.mass_solar * SOLAR_MASS;
        let c_m_per_s = SPEED_OF_LIGHT * 1000.0;
        2.0 * GRAVITATIONAL_CONSTANT * mass_kg / (c_m_per_s * c_m_per_s) / 1000.0
    }

    /// 光子球半径（km）：`1.5 r_s`。
    pub fn photon_sphere_radius_km(&self) -> f64 {
        1.5 * self.schwarzschild_radius_km()
    }

    /// 吸积盘内半径（km）：`3 r_s`（光子球之外的安全下界）。
    pub fn disk_inner_radius_km(&self) -> f64 {
        3.0 * self.schwarzschild_radius_km()
    }

    /// 吸积盘外半径（km）：`12 r_s`。
    pub fn disk_outer_radius_km(&self) -> f64 {
        12.0 * self.schwarzschild_radius_km()
    }

    /// 圆轨道速度（km/s）：`v = sqrt(GM/r)`（牛顿近似；相对论修正不在本批范围）。
    pub fn orbital_speed_km_per_s(&self, radius_km: f64) -> f64 {
        if radius_km <= 0.0 {
            return 0.0;
        }
        let mass_kg = self.mass_solar * SOLAR_MASS;
        let speed_m_per_s = (GRAVITATIONAL_CONSTANT * mass_kg / (radius_km * 1000.0)).sqrt();
        speed_m_per_s / 1000.0
    }

    /// 合成 uniform（状态 → 像素）。
    pub fn to_uniforms(&self, camera_distance_km: f64, time_seconds: f64) -> SceneUniforms {
        SceneUniforms {
            camera_distance_km,
            disk_inner_radius_km: self.disk_inner_radius_km(),
            disk_outer_radius_km: self.disk_outer_radius_km(),
            time_seconds,
        }
    }
}

/// 场景参数错误。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneError {
    /// 质量非法（非正或非有限）。
    InvalidMass(f64),
    /// 自转参数非法（越界或非有限）。
    InvalidSpin(f64),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::InvalidMass(mass) => write!(f, "InvalidMass: 质量必须为正，实测 {mass}"),
            SceneError::InvalidSpin(spin) => {
                write!(f, "InvalidSpin: 自转参数必须落在 [-1, 1]，实测 {spin}")
            }
        }
    }
}

impl std::error::Error for SceneError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_mass_schwarzschild_radius_matches_known_value() {
        // 实测口径：太阳的史瓦西半径 ≈ 2.95 km。
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let radius = params.schwarzschild_radius_km();
        assert!(
            (radius - 2.95).abs() < 0.05,
            "实测半径 {radius} km（期望 ≈2.95 km）"
        );
    }

    #[test]
    fn radius_scales_linearly_with_mass() {
        let one = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let ten = BlackHoleParams::new(10.0, 0.0).expect("参数合法");
        let ratio = ten.schwarzschild_radius_km() / one.schwarzschild_radius_km();
        assert!(
            (ratio - 10.0).abs() < 1e-6,
            "半径必须随质量线性缩放，实测比值 {ratio}"
        );
    }

    #[test]
    fn disk_bounds_sit_outside_the_photon_sphere() {
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        assert!(params.disk_inner_radius_km() > params.photon_sphere_radius_km());
        assert!(params.disk_outer_radius_km() > params.disk_inner_radius_km());
    }

    #[test]
    fn orbital_speed_falls_with_radius() {
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let inner = params.orbital_speed_km_per_s(params.disk_inner_radius_km());
        let outer = params.orbital_speed_km_per_s(params.disk_outer_radius_km());
        assert!(inner > outer, "内圈更快：{inner} > {outer}");
        assert!(outer < SPEED_OF_LIGHT, "轨道速度必须低于光速：{outer}");
        assert_eq!(params.orbital_speed_km_per_s(0.0), 0.0);
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        assert!(BlackHoleParams::new(0.0, 0.0).is_err());
        assert!(BlackHoleParams::new(-1.0, 0.0).is_err());
        assert!(BlackHoleParams::new(1.0, 2.0).is_err());
        assert!(BlackHoleParams::new(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn uniforms_carry_disk_bounds() {
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let uniforms = params.to_uniforms(1.0e7, 3.5);
        assert_eq!(uniforms.camera_distance_km, 1.0e7);
        assert_eq!(uniforms.time_seconds, 3.5);
        assert_eq!(uniforms.disk_inner_radius_km, params.disk_inner_radius_km());
    }
}
