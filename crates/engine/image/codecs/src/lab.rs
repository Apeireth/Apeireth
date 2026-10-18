// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! CIE L*a*b* (D50) ⇄ encoded sRGB for [`crate::ChannelLayout::Lab`] samples, the same formulas
//! and constants as `apeireth-image-color` (this crate stays standalone). Samples are normalized as
//! The image core holds Lab: L* 0..100 as 0..1, a* and b* −128..127 as `(v + 128) / 255`.

const D50: [f32; 3] = [0.964_22, 1.0, 0.825_21];
/// Linear sRGB → D50 XYZ (Bradford-adapted, as in the ICC sRGB profile).
const SRGB_TO_XYZ_D50: [[f32; 3]; 3] = [[0.436_074, 0.385_064, 0.143_080], [0.222_504, 0.716_878, 0.060_618], [0.013_932, 0.097_104, 0.714_173]];
/// D50 XYZ → linear sRGB.
const XYZ_D50_TO_SRGB: [[f32; 3]; 3] = [[3.133_856, -1.616_867, -0.490_615], [-0.978_768, 1.916_142, 0.033_454], [0.071_945, -0.228_991, 1.405_243]];

fn mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    m.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
}

fn to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_encoded(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

/// Normalized Lab → encoded sRGB in 0..1 (out-of-gamut colours clipped).
pub(crate) fn lab_to_srgb(n: [f32; 3]) -> [f32; 3] {
    let (l, a, b) = (n[0] * 100.0, n[1] * 255.0 - 128.0, n[2] * 255.0 - 128.0);
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let finv = |t: f32| if t > 6.0 / 29.0 { t * t * t } else { 3.0 * (6.0f32 / 29.0).powi(2) * (t - 4.0 / 29.0) };
    let xyz = [finv(fx) * D50[0], finv(fy) * D50[1], finv(fz) * D50[2]];
    mul(&XYZ_D50_TO_SRGB, xyz).map(|v| to_encoded(v.clamp(0.0, 1.0)))
}

/// Encoded sRGB → normalized Lab.
pub(crate) fn srgb_to_lab(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mul(&SRGB_TO_XYZ_D50, rgb.map(|v| to_linear(v.clamp(0.0, 1.0))));
    let f = |t: f32| if t > (6.0f32 / 29.0).powi(3) { t.cbrt() } else { t / (3.0 * (6.0f32 / 29.0).powi(2)) + 4.0 / 29.0 };
    let (fx, fy, fz) = (f(xyz[0] / D50[0]), f(xyz[1] / D50[1]), f(xyz[2] / D50[2]));
    let (l, a, b) = (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz));
    [(l / 100.0).clamp(0.0, 1.0), ((a + 128.0) / 255.0).clamp(0.0, 1.0), ((b + 128.0) / 255.0).clamp(0.0, 1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_black_and_round_trips() {
        let w = srgb_to_lab([1.0; 3]);
        assert!((w[0] - 1.0).abs() < 1e-3 && (w[1] - 128.0 / 255.0).abs() < 1e-3 && (w[2] - 128.0 / 255.0).abs() < 1e-3, "{w:?}");
        assert!(srgb_to_lab([0.0; 3])[0].abs() < 1e-4);
        for rgb in [[0.2, 0.5, 0.8], [0.9, 0.1, 0.3], [0.5; 3]] {
            let back = lab_to_srgb(srgb_to_lab(rgb));
            assert!(rgb.iter().zip(back).all(|(a, b)| (a - b).abs() < 2e-3), "{rgb:?} → {back:?}");
        }
    }
}
