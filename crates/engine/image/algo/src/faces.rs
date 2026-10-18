// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Face detection: a port of libfacedetection's CNN (Shiqi Yu, BSD-3; `data/LICENSE-libfacedetection.txt`).
//!
//! The network is the YuNet design: a backbone of 1×1 pointwise and 3×3 depthwise convolutions with
//! 2×2 max pooling, a feature pyramid at strides 8, 16 and 32, and per-cell heads for the face score,
//! the box and five landmarks (eyes, nose tip, mouth corners). The weights (`data/libfacedetection.bin`,
//! converted to half precision from the library's `facedetectcnn-data.cpp`) are about 100 KB, and
//! inference is plain Rust: no runtime, so it works offline.
//!
//! [`detect`] takes straight RGBA in `0..=1` and returns faces in pixel coordinates of that image.
//!
//! The browser build (wasm32) leaves the network and its weights out, as the camera-raw decoder does with
//! its JPEG XL decoder: they would push the wasm over the per-file cap of static hosts
//! (packaging/web/package.sh). There [`AVAILABLE`] is false and [`detect`] finds nothing.

#[cfg(not(target_arch = "wasm32"))]
mod cnn;

/// Whether this build can detect faces (not the browser build: see the module docs).
pub const AVAILABLE: bool = cfg!(not(target_arch = "wasm32"));

/// Longest side the network looks at; larger images are scaled down first (faces smaller than
/// about 10 px at this scale are missed).
pub const MAX_SIDE: usize = 1280;

/// One detected face, in pixels of the image passed to [`detect`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// Confidence `0..=1` (the library reports faces above 0.5 as reliable).
    pub score: f32,
    /// Left, top, width and height.
    pub rect: [f32; 4],
    /// Right eye, left eye (as seen in the image: the first is on the image's left), nose tip,
    /// right mouth corner, left mouth corner.
    pub landmarks: [[f32; 2]; 5],
}

/// Faces in `px` (straight RGBA `0..=1`, `w × h`, row-major) scoring at least `min_score`, best
/// first. Transparent pixels count as white. `None` when `px` doesn't match the size.
#[cfg(not(target_arch = "wasm32"))]
pub fn detect(px: &[[f32; 4]], w: usize, h: usize, min_score: f32) -> Option<Vec<Face>> {
    if w == 0 || h == 0 || px.len() != w.checked_mul(h)? {
        return None;
    }
    // Scale down (box filter) so the longer side is at most MAX_SIDE.
    let step = w.max(h).div_ceil(MAX_SIDE).max(1);
    let (sw, sh) = (w.div_ceil(step), h.div_ceil(step));
    let mut bgr = vec![[0.0f32; 3]; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            let (mut acc, mut n) = ([0.0f32; 3], 0.0f32);
            for yy in y * step..((y + 1) * step).min(h) {
                for xx in x * step..((x + 1) * step).min(w) {
                    let [r, g, b, a] = px[yy * w + xx];
                    let a = a.clamp(0.0, 1.0);
                    for (k, v) in [b, g, r].into_iter().enumerate() {
                        acc[k] += v.clamp(0.0, 1.0) * a + (1.0 - a);
                    }
                    n += 1.0;
                }
            }
            bgr[y * sw + x] = acc.map(|v| (v / n.max(1.0) * 255.0).round());
        }
    }
    let s = step as f32;
    let faces = cnn::network(&bgr, sw, sh)?
        .into_iter()
        .filter(|f| f.score >= min_score)
        .map(|f| Face { score: f.score, rect: f.rect.map(|v| v * s), landmarks: f.landmarks.map(|[x, y]| [x * s, y * s]) })
        .collect();
    Some(faces)
}

/// The browser build has no detector (see the module docs): always `None`.
#[cfg(target_arch = "wasm32")]
pub fn detect(_px: &[[f32; 4]], _w: usize, _h: usize, _min_score: f32) -> Option<Vec<Face>> {
    None
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn no_faces_in_flat_or_tiny_images_and_bad_sizes_are_refused() {
        assert_eq!(detect(&vec![[0.5, 0.5, 0.5, 1.0]; 64 * 48], 64, 48, 0.5), Some(vec![]));
        assert_eq!(detect(&[[1.0, 1.0, 1.0, 1.0]], 1, 1, 0.5), Some(vec![]));
        assert_eq!(detect(&[[1.0; 4]; 3], 2, 2, 0.5), None);
        assert_eq!(detect(&[], 0, 0, 0.5), None);
    }
}
