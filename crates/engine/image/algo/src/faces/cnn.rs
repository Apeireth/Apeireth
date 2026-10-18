// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Imported unchanged 2026-10-18 into the Apeireth workspace (no content edits).
// See docs/04-internal/license-port/gate1-license.md.
//! The network itself (native builds only: see the parent module's docs).

use super::Face;
use crate::par;

/// The weights: `PCFACE1\0`, the layer count (u16), then per layer its input channels (u16), filters
/// (u16) and kind (u8: 0 pointwise, 1 depthwise), the weights and the biases, all little-endian f16.
static WEIGHTS: &[u8] = include_bytes!("../../data/libfacedetection.bin");

const MAGIC: &[u8] = b"PCFACE1\0";
const LAYERS: usize = 53;
/// Faces whose boxes overlap more than this (intersection over union) are one face.
const NMS_OVERLAP: f32 = 0.45;
/// Candidates kept before non-maximum suppression, as the library does.
const TOP_K: usize = 1000;

struct Layer {
    channels: usize,
    filters: usize,
    depthwise: bool,
    weights: Vec<f32>,
    biases: Vec<f32>,
}

/// A feature map: `rows × cols` cells of `channels` values each.
#[derive(Clone)]
struct Blob {
    rows: usize,
    cols: usize,
    channels: usize,
    data: Vec<f32>,
}

impl Blob {
    fn new(rows: usize, cols: usize, channels: usize) -> Self {
        Blob { rows, cols, channels, data: vec![0.0; rows * cols * channels] }
    }
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let frac = f32::from(h & 0x3ff);
    match exp {
        0 => sign * frac * (2.0f32).powi(-24),
        31 => {
            if frac == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + frac / 1024.0) * (2.0f32).powi(exp - 15),
    }
}

fn parse(bytes: &[u8]) -> Option<Vec<Layer>> {
    let rest = bytes.strip_prefix(MAGIC)?;
    let mut at = 0usize;
    let u16_at = |n: &mut usize| -> Option<u16> {
        let b = rest.get(*n..*n + 2)?;
        *n += 2;
        Some(u16::from_le_bytes([b[0], b[1]]))
    };
    let count = usize::from(u16_at(&mut at)?);
    if count != LAYERS {
        return None;
    }
    let mut layers = Vec::with_capacity(count);
    for _ in 0..count {
        let channels = usize::from(u16_at(&mut at)?);
        let filters = usize::from(u16_at(&mut at)?);
        let depthwise = *rest.get(at)? == 1;
        at += 1;
        let nw = if depthwise { 9 * channels } else { filters * channels };
        let mut read = |n: usize| -> Option<Vec<f32>> { (0..n).map(|_| u16_at(&mut at).map(f16_to_f32)).collect() };
        let weights = read(nw)?;
        let biases = read(filters)?;
        if depthwise && filters != channels {
            return None;
        }
        layers.push(Layer { channels, filters, depthwise, weights, biases });
    }
    (at == rest.len()).then_some(layers)
}

/// A 1×1 or 3×3 depthwise convolution with bias, then ReLU when `relu`.
fn conv(x: &Blob, l: &Layer, relu: bool) -> Option<Blob> {
    if x.channels != l.channels {
        return None;
    }
    let mut out = Blob::new(x.rows, x.cols, l.filters);
    let (rows, cols, cin, cout) = (x.rows, x.cols, x.channels, l.filters);
    par::chunks_mut(&mut out.data, cols * cout, rows * cols * cout > 1 << 16, |row, line| {
        for col in 0..cols {
            let o = &mut line[col * cout..(col + 1) * cout];
            if l.depthwise {
                o.copy_from_slice(&l.biases);
                for r in row.saturating_sub(1)..(row + 2).min(rows) {
                    for c in col.saturating_sub(1)..(col + 2).min(cols) {
                        let k = (r + 1 - row) * 3 + (c + 1 - col);
                        let input = &x.data[(r * cols + c) * cin..(r * cols + c + 1) * cin];
                        let w = &l.weights[k * cin..(k + 1) * cin];
                        for ((o, i), w) in o.iter_mut().zip(input).zip(w) {
                            *o += i * w;
                        }
                    }
                }
            } else {
                let input = &x.data[(row * cols + col) * cin..(row * cols + col + 1) * cin];
                for (f, o) in o.iter_mut().enumerate() {
                    let w = &l.weights[f * cin..(f + 1) * cin];
                    *o = l.biases[f] + input.iter().zip(w).map(|(i, w)| i * w).sum::<f32>();
                }
            }
            if relu {
                for v in o.iter_mut() {
                    *v = v.max(0.0);
                }
            }
        }
    });
    Some(out)
}

/// Pointwise then depthwise (ReLU after the depthwise one when `relu`).
fn conv_dp(x: &Blob, p: &Layer, d: &Layer, relu: bool) -> Option<Blob> {
    conv(&conv(x, p, false)?, d, relu)
}

fn unit(x: &Blob, l: &[Layer]) -> Option<Blob> {
    let [p1, d1, p2, d2] = l else { return None };
    conv_dp(&conv_dp(x, p1, d1, true)?, p2, d2, true)
}

fn max_pool(x: &Blob) -> Blob {
    let rows = x.rows.saturating_sub(3).div_ceil(2) + 1;
    let cols = x.cols.saturating_sub(3).div_ceil(2) + 1;
    let mut out = Blob::new(rows, cols, x.channels);
    for r in 0..rows {
        for c in 0..cols {
            let o = (r * cols + c) * x.channels;
            for ch in 0..x.channels {
                let mut m = f32::NEG_INFINITY;
                for rr in r * 2..(r * 2 + 2).min(x.rows) {
                    for cc in c * 2..(c * 2 + 2).min(x.cols) {
                        m = m.max(x.data[(rr * x.cols + cc) * x.channels + ch]);
                    }
                }
                out.data[o + ch] = m;
            }
        }
    }
    out
}

/// `small` upsampled ×2 (nearest) plus `big`.
fn up_add(small: &Blob, big: &Blob) -> Option<Blob> {
    if small.rows * 2 != big.rows || small.cols * 2 != big.cols || small.channels != big.channels {
        return None;
    }
    let mut out = big.clone();
    let ch = big.channels;
    for r in 0..big.rows {
        for c in 0..big.cols {
            let s = ((r / 2) * small.cols + c / 2) * ch;
            let o = (r * big.cols + c) * ch;
            for k in 0..ch {
                out.data[o + k] += small.data[s + k];
            }
        }
    }
    Some(out)
}

/// The network's input: each cell holds the 3×3 neighbourhood (stride 2) of BGR values `0..=255`,
/// so the first, 3×3 stride-2 convolution is a pointwise one. The size is padded to 32.
fn input(bgr: &[[f32; 3]], w: usize, h: usize) -> Blob {
    let rows = h.div_ceil(32) * 16;
    let cols = w.div_ceil(32) * 16;
    let mut b = Blob::new(rows, cols, 32);
    for r in 0..rows {
        for c in 0..cols {
            let cell = (r * cols + c) * 32;
            for fy in 0..3 {
                let Some(y) = (r * 2 + fy).checked_sub(1).filter(|&y| y < h) else { continue };
                for fx in 0..3 {
                    let Some(x) = (c * 2 + fx).checked_sub(1).filter(|&x| x < w) else { continue };
                    let o = cell + (fy * 3 + fx) * 3;
                    b.data[o..o + 3].copy_from_slice(&bgr[y * w + x]);
                }
            }
        }
    }
    b
}

/// The heads of one pyramid level, decoded to candidate faces (in network pixels).
fn candidates(feat: &Blob, l: &[Layer], heads: [usize; 4], stride: f32, out: &mut Vec<Face>) -> Option<()> {
    let head = |i: usize| conv_dp(feat, l.get(i)?, l.get(i + 1)?, false);
    let (cls, reg, obj, kps) = (head(heads[0])?, head(heads[1])?, head(heads[2])?, head(heads[3])?);
    if cls.channels != 1 || obj.channels != 1 || reg.channels != 4 || kps.channels != 10 {
        return None;
    }
    let sigmoid = |v: f32| 1.0 / (1.0 + (-v.clamp(-88.0, 88.0)).exp());
    for r in 0..feat.rows {
        for c in 0..feat.cols {
            let i = r * feat.cols + c;
            let score = (sigmoid(cls.data[i]) * sigmoid(obj.data[i])).sqrt();
            if score < 0.2 {
                continue;
            }
            let (px, py) = (c as f32 * stride, r as f32 * stride);
            let b = &reg.data[i * 4..i * 4 + 4];
            let (cx, cy) = (b[0] * stride + px, b[1] * stride + py);
            let (bw, bh) = (b[2].exp() * stride, b[3].exp() * stride);
            let k = &kps.data[i * 10..i * 10 + 10];
            let mut landmarks = [[0.0; 2]; 5];
            for (n, lm) in landmarks.iter_mut().enumerate() {
                *lm = [k[2 * n] * stride + px, k[2 * n + 1] * stride + py];
            }
            out.push(Face { score, rect: [cx - bw / 2.0, cy - bh / 2.0, bw, bh], landmarks });
        }
    }
    Some(())
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let w = ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.0);
    let h = ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.0);
    let i = w * h;
    if i <= 0.0 { 0.0 } else { i / (a[2] * a[3] + b[2] * b[3] - i) }
}

/// The heads (cls, reg, obj, kps: their first layer) of the stride 8, 16 and 32 levels.
const HEADS: [[usize; 4]; 3] = [[29, 35, 41, 47], [31, 37, 43, 49], [33, 39, 45, 51]];

/// The backbone and feature pyramid: the stride 8, 16 and 32 feature maps the heads read.
fn features(bgr: &[[f32; 3]], w: usize, h: usize, l: &[Layer]) -> Option<[Blob; 3]> {
    let x = conv(&input(bgr, w, h), l.first()?, true)?;
    let x = max_pool(&conv_dp(&x, l.get(1)?, l.get(2)?, true)?);
    let x = unit(&unit(&x, l.get(3..7)?)?, l.get(7..11)?)?;
    let f8 = unit(&max_pool(&x), l.get(11..15)?)?;
    let f16 = unit(&max_pool(&f8), l.get(15..19)?)?;
    let f32_ = unit(&max_pool(&f16), l.get(19..23)?)?;
    let f32_ = conv_dp(&f32_, l.get(27)?, l.get(28)?, true)?;
    let f16 = conv_dp(&up_add(&f32_, &f16)?, l.get(25)?, l.get(26)?, true)?;
    let f8 = conv_dp(&up_add(&f16, &f8)?, l.get(23)?, l.get(24)?, true)?;
    Some([f8, f16, f32_])
}

pub(super) fn network(bgr: &[[f32; 3]], w: usize, h: usize) -> Option<Vec<Face>> {
    let l = parse(WEIGHTS)?;
    let levels = features(bgr, w, h, &l)?;
    let mut faces = Vec::new();
    // Heads (cls, reg, obj, kps) per level, as in the library's model.
    for ((feat, heads), stride) in levels.iter().zip(HEADS).zip([8.0, 16.0, 32.0]) {
        candidates(feat, &l, heads, stride, &mut faces)?;
    }
    faces.sort_by(|a, b| b.score.total_cmp(&a.score));
    faces.truncate(TOP_K);
    let mut kept: Vec<Face> = Vec::new();
    for f in faces {
        if kept.iter().all(|k| iou(&k.rect, &f.rect) <= NMS_OVERLAP) {
            kept.push(f);
        }
    }
    Some(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_parse() {
        let l = parse(WEIGHTS).expect("bundled weights");
        assert_eq!(l.len(), LAYERS);
        assert_eq!((l[0].channels, l[0].filters, l[0].depthwise), (32, 16, false));
        assert_eq!((l[2].channels, l[2].filters, l[2].depthwise), (16, 16, true));
        assert!(l.iter().all(|l| l.weights.iter().chain(&l.biases).all(|v| v.is_finite())));
        // Damaged weights are refused, not read past their end.
        assert!(parse(&WEIGHTS[..WEIGHTS.len() - 1]).is_none());
        assert!(parse(b"nope").is_none());
    }

    /// The sum of every head's raw output on a fixed test image matches libfacedetection's own C++
    /// (computed with its f32 weights; ours are f16, hence the tolerance).
    #[test]
    fn heads_match_the_reference_implementation() {
        let (w, h) = (96usize, 72usize);
        let bgr: Vec<[f32; 3]> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let (r, g, b) = ((x * 7 + y * 3) % 256, (x * x + y * 5) % 256, (x * y * 3 + 11) % 256);
                [b as f32, g as f32, r as f32]
            })
            .collect();
        let l = parse(WEIGHTS).unwrap();
        let levels = features(&bgr, w, h, &l).unwrap();
        let reference =
            [27.647039, 549.579923, -1645.299678, 914.784787, -6.149318, 35.705980, -454.742148, 163.847111, 1.267630, 28.415509, -85.662257, 62.294208];
        let mut got = Vec::new();
        for (feat, heads) in levels.iter().zip(HEADS) {
            for first in heads {
                let o = conv_dp(feat, &l[first], &l[first + 1], false).unwrap();
                got.push(o.data.iter().map(|&v| f64::from(v)).sum::<f64>());
            }
        }
        for (g, r) in got.iter().zip(reference) {
            assert!((g - r).abs() <= 0.01 * r.abs().max(10.0), "{got:?}");
        }
    }

    #[test]
    fn f16_decoding() {
        assert_eq!(f16_to_f32(0x3c00), 1.0);
        assert_eq!(f16_to_f32(0xc000), -2.0);
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert_eq!(f16_to_f32(0x3555), 0.333_251_95);
        assert_eq!(f16_to_f32(0x0001), 2.0f32.powi(-24));
    }
}
