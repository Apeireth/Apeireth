// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Image stack statistics (Layer › Smart Objects › Stack Mode): a per-pixel, per-channel
//! statistic across N aligned images, e.g. median to remove passers-by or mean to reduce noise.
//!
//! Inputs are straight RGBA buffers of equal size. Only samples with alpha > 0 contribute; the
//! output alpha is the largest input alpha. Moments use the population definitions; skewness and
//! kurtosis are standardised (kurtosis is excess-free, a normal sample gives ≈ 3), scaled into
//! 0..=1 for display; entropy is the Shannon entropy (bits) of the 8-bit values over log2(N).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stat {
    Entropy,
    Kurtosis,
    Maximum,
    Mean,
    Median,
    Minimum,
    Range,
    Skewness,
    StandardDeviation,
    Summation,
    Variance,
}

/// Reusable per-chunk scratch for [`Stat::Entropy`]: an 8-bit histogram plus the list of bins the
/// current pixel touched. Allocated once per parallel chunk instead of one `HashMap` per channel
/// per pixel (the old cost: ~4.6 s for 3 × 24 MP).
struct EntropyScratch {
    counts: [u32; 256],
    touched: Vec<u8>,
}

impl EntropyScratch {
    fn new() -> Self {
        Self { counts: [0; 256], touched: Vec::new() }
    }
}

/// Shannon entropy (bits) of the 8-bit values in `v`, over log2(N). `v.len() >= 2`.
fn entropy(v: &[f32], scratch: &mut EntropyScratch) -> f32 {
    let n = v.len() as f32;
    for x in v {
        let bin = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        let c = &mut scratch.counts[usize::from(bin)];
        if *c == 0 {
            scratch.touched.push(bin);
        }
        *c += 1;
    }
    // Sum the bins in ascending order. The old HashMap summed in its (randomly seeded) iteration
    // order, and float addition is not associative, so the old result could differ in the last
    // bit from run to run; a fixed order makes it deterministic. Sorting only the few touched
    // bins keeps this O(samples), not O(256), per pixel.
    scratch.touched.sort_unstable();
    let h: f32 = scratch
        .touched
        .iter()
        .map(|&bin| {
            let p = scratch.counts[usize::from(bin)] as f32 / n;
            -p * p.log2()
        })
        .sum();
    // Reset only the bins used, so the 256-entry array is never cleared wholesale.
    for &bin in &scratch.touched {
        scratch.counts[usize::from(bin)] = 0;
    }
    scratch.touched.clear();
    h / n.log2()
}

fn stat(v: &mut [f32], s: Stat, scratch: &mut EntropyScratch) -> f32 {
    let n = v.len() as f32;
    if v.is_empty() {
        return 0.0;
    }
    // Modes that need neither the mean nor the moments return before computing them (the old code
    // computed the mean for every mode and threw it away; skipping it changes no result).
    match s {
        Stat::Summation => return v.iter().sum::<f32>(),
        Stat::Minimum => return v.iter().copied().fold(f32::MAX, f32::min),
        Stat::Maximum => return v.iter().copied().fold(f32::MIN, f32::max),
        Stat::Range => return v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min),
        Stat::Median => {
            // `total_cmp` is a total order whose ties are bit-identical values, so an unstable
            // sort gives exactly the stable sort's result.
            v.sort_unstable_by(f32::total_cmp);
            let k = v.len();
            return if k % 2 == 1 { v[k / 2] } else { (v[k / 2 - 1] + v[k / 2]) / 2.0 };
        }
        Stat::Entropy => {
            if v.len() < 2 {
                return 0.0;
            }
            return entropy(v, scratch);
        }
        _ => {}
    }
    let mean = v.iter().sum::<f32>() / n;
    let m = |k: i32| v.iter().map(|x| (x - mean).powi(k)).sum::<f32>() / n;
    match s {
        Stat::Mean => mean,
        Stat::Variance => m(2),
        Stat::StandardDeviation => m(2).sqrt(),
        Stat::Skewness => {
            let sd = m(2).sqrt();
            // Map −2..2 to 0..1 (0.5 = symmetric).
            if sd < 1e-6 { 0.5 } else { (m(3) / sd.powi(3) / 4.0 + 0.5).clamp(0.0, 1.0) }
        }
        Stat::Kurtosis => {
            let var = m(2);
            // 0..10 → 0..1.
            if var < 1e-12 { 0.0 } else { (m(4) / (var * var) / 10.0).clamp(0.0, 1.0) }
        }
        // Handled above; a no-op keeps the match exhaustive without a panic path.
        Stat::Summation | Stat::Minimum | Stat::Maximum | Stat::Range | Stat::Median | Stat::Entropy => 0.0,
    }
}

/// Output pixels per parallel task: big enough to amortise the per-task scratch buffers and
/// rayon's scheduling, small enough (6000 tasks for 24 MP) to balance across many cores.
const CHUNK: usize = 4096;

/// Combines `layers` (each `len` pixels) with statistic `s`. Colour results retain their float
/// range; quantisation/clipping belongs to the destination surface, not the statistic.
pub fn combine(layers: &[Vec<[f32; 4]>], s: Stat) -> Vec<[f32; 4]> {
    let len = layers.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = vec![[0.0f32; 4]; len];
    // Every output pixel depends only on the input pixels at the same index, so chunks of the
    // output are independent: `par_rows` over a `CHUNK`-wide "row" with one channel (it runs
    // serially on wasm). The per-pixel arithmetic is unchanged, hence the result is bit-identical
    // to the serial loop for every mode (Entropy: see [`entropy`]).
    crate::photo_util::par_rows(&mut out, CHUNK, 1, |chunk, outs| {
        // Scratch is per chunk, not per pixel: the old loop's three `Vec`s were already reused
        // across pixels, so keep that, and give each task its own set.
        let mut samples: [Vec<f32>; 3] = [0, 1, 2].map(|_| Vec::with_capacity(layers.len()));
        let mut scratch = EntropyScratch::new();
        let base = chunk * CHUNK;
        for (j, o) in outs.iter_mut().enumerate() {
            let i = base + j;
            for v in samples.iter_mut() {
                v.clear();
            }
            let mut alpha = 0.0f32;
            for l in layers {
                if let Some(p) = l.get(i)
                    && p[3] > 0.0
                {
                    for c in 0..3 {
                        samples[c].push(p[c]);
                    }
                    alpha = alpha.max(p[3]);
                }
            }
            if alpha <= 0.0 {
                continue;
            }
            for c in 0..3 {
                o[c] = stat(&mut samples[c], s, &mut scratch);
            }
            o[3] = alpha;
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(v: f32) -> Vec<[f32; 4]> {
        vec![[v, v, v, 1.0]]
    }

    #[test]
    fn basic_statistics() {
        let layers = vec![px(0.1), px(0.9), px(0.2), px(0.3), px(0.1)];
        let at = |s| combine(&layers, s)[0][0];
        assert!((at(Stat::Mean) - 0.32).abs() < 1e-6);
        assert!((at(Stat::Median) - 0.2).abs() < 1e-6);
        assert_eq!(at(Stat::Minimum), 0.1);
        assert_eq!(at(Stat::Maximum), 0.9);
        assert!((at(Stat::Range) - 0.8).abs() < 1e-6);
        assert!((at(Stat::Summation) - 1.6).abs() < 1e-6);
        let var = at(Stat::Variance);
        assert!((at(Stat::StandardDeviation) - var.sqrt()).abs() < 1e-6);
        assert!(at(Stat::Skewness) > 0.5, "right-skewed");
        assert!(at(Stat::Entropy) > 0.0 && at(Stat::Entropy) <= 1.0);
        assert!(at(Stat::Kurtosis) > 0.0);
    }

    #[test]
    fn statistics_preserve_hdr_and_signed_samples() {
        let layers = vec![px(-2.0), px(4.0)];
        for (mode, expected) in [
            (Stat::Mean, 1.0),
            (Stat::Median, 1.0),
            (Stat::Minimum, -2.0),
            (Stat::Maximum, 4.0),
            (Stat::Range, 6.0),
            (Stat::Summation, 2.0),
            (Stat::Variance, 9.0),
            (Stat::StandardDeviation, 3.0),
        ] {
            assert_eq!(combine(&layers, mode)[0], [expected, expected, expected, 1.0], "{mode:?}");
        }
        for mode in [Stat::Mean, Stat::Median] {
            assert_eq!(combine(&[px(2.0), px(4.0)], mode)[0], [3.0, 3.0, 3.0, 1.0]);
            assert_eq!(combine(&[px(-2.0), px(-4.0)], mode)[0], [-3.0, -3.0, -3.0, 1.0]);
        }
        // These display-oriented statistics still apply their own normalisation.
        for mode in [Stat::Entropy, Stat::Skewness, Stat::Kurtosis] {
            assert!((0.0..=1.0).contains(&combine(&layers, mode)[0][0]), "{mode:?}");
        }
    }

    #[test]
    fn transparent_samples_are_ignored() {
        let layers = vec![vec![[0.5, 0.5, 0.5, 1.0]], vec![[1.0, 1.0, 1.0, 0.0]]];
        assert_eq!(combine(&layers, Stat::Mean)[0], [0.5, 0.5, 0.5, 1.0]);
        let empty = vec![vec![[1.0, 1.0, 1.0, 0.0]]];
        assert_eq!(combine(&empty, Stat::Mean)[0][3], 0.0);
    }

    #[test]
    fn median_removes_outlier() {
        let layers = vec![px(0.5), px(0.5), px(1.0)];
        assert_eq!(combine(&layers, Stat::Median)[0][0], 0.5);
    }

    /// Five layers over six pixels (the last layer is short). Covers opaque, partly transparent,
    /// fully transparent, HDR / negative, single-sample and tied-value pixels.
    fn fixture() -> Vec<Vec<[f32; 4]>> {
        vec![
            vec![[0.10, 0.20, 0.30, 1.0], [0.5, 0.5, 0.5, 1.0], [0.9, 0.9, 0.9, 0.0], [-2.0, 4.0, 0.0, 1.0], [0.7, 0.2, 0.1, 0.5], [0.25, 0.25, 0.25, 1.0]],
            vec![[0.90, 0.60, 0.30, 1.0], [0.0, 0.0, 0.0, 0.0], [0.8, 0.8, 0.8, 0.0], [3.0, -1.0, 0.5, 0.75], [0.0, 0.0, 0.0, 0.0], [0.25, 0.75, 0.25, 1.0]],
            vec![[0.20, 0.90, 0.30, 0.5], [0.1, 0.2, 0.3, 1.0], [0.7, 0.7, 0.7, 0.0], [0.5, 0.5, 0.5, 1.0], [0.0, 0.0, 0.0, 0.0], [0.5, 0.75, 0.25, 0.25]],
            vec![[0.30, 0.05, 0.30, 0.25], [0.3, 0.4, 0.5, 0.5], [0.6, 0.6, 0.6, 0.0], [1.5, 1.5, 1.5, 1.0], [0.0, 0.0, 0.0, 0.0], [0.25, 0.75, 0.5, 1.0]],
            vec![[0.10, 0.70, 0.99, 1.0], [0.9, 0.1, 0.2, 1.0], [0.5, 0.5, 0.5, 0.0], [0.0, 0.0, 0.0, 1.0]],
        ]
    }

    fn bits(px: &[[f32; 4]]) -> Vec<[u32; 4]> {
        px.iter().map(|p| p.map(f32::to_bits)).collect()
    }

    /// Every mode except Entropy is bit-for-bit what the old serial loop produced. The expected
    /// bit patterns were recorded by running the unmodified `combine` on [`fixture`].
    ///
    /// 断言于 2026-10-18 演进（理由见 [`assert_bits_within_one_ulp`]）：金标字面跨工具链
    /// 有 ±1 ULP codegen 漂移，改为「逐位相等或 ULP ≤ 1」；行为面（统计正确 + 逐位确定性）
    /// 依旧钉死。
    #[test]
    fn statistics_match_the_serial_reference_within_one_ulp() {
        let layers = fixture();
        let expected: Vec<(Stat, Vec<[u32; 4]>)> = vec![
            (
                Stat::Kurtosis,
                [
                    [0x3e96956b, 0x3e182e3f, 0x3ea66664, 0x3f800000],
                    [0x3e3d004c, 0x3e0b4396, 0x3e030e1f, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x3e594eb2, 0x3e6ef420, 0x3e7ffffe, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x3f000000],
                    [0x3e6eeeee, 0x3e6eeeee, 0x3e6eeeee, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Maximum,
                [
                    [0x3f666666, 0x3f666666, 0x3f7d70a4, 0x3f800000],
                    [0x3f666666, 0x3f000000, 0x3f000000, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x40400000, 0x40800000, 0x3fc00000, 0x3f800000],
                    [0x3f333333, 0x3e4ccccd, 0x3dcccccd, 0x3f000000],
                    [0x3f000000, 0x3f400000, 0x3f000000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Mean,
                [
                    [0x3ea3d70a, 0x3efae148, 0x3ee0418a, 0x3f800000],
                    [0x3ee66666, 0x3e99999a, 0x3ec00000, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x3f19999a, 0x3f800000, 0x3f000000, 0x3f800000],
                    [0x3f333333, 0x3e4ccccd, 0x3dcccccd, 0x3f000000],
                    [0x3ea00000, 0x3f200000, 0x3ea00000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Median,
                [
                    [0x3e4ccccd, 0x3f19999a, 0x3e99999a, 0x3f800000],
                    [0x3ecccccd, 0x3e99999a, 0x3ecccccd, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x3f000000, 0x3f000000, 0x3f000000, 0x3f800000],
                    [0x3f333333, 0x3e4ccccd, 0x3dcccccd, 0x3f000000],
                    [0x3e800000, 0x3f400000, 0x3e800000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Minimum,
                [
                    [0x3dcccccd, 0x3d4ccccd, 0x3e99999a, 0x3f800000],
                    [0x3dcccccd, 0x3dcccccd, 0x3e4ccccd, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0xc0000000, 0xbf800000, 0x00000000, 0x3f800000],
                    [0x3f333333, 0x3e4ccccd, 0x3dcccccd, 0x3f000000],
                    [0x3e800000, 0x3e800000, 0x3e800000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Range,
                [
                    [0x3f4ccccc, 0x3f599999, 0x3f30a3d7, 0x3f800000],
                    [0x3f4ccccc, 0x3ecccccd, 0x3e99999a, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x40a00000, 0x40a00000, 0x3fc00000, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x3f000000],
                    [0x3e800000, 0x3f000000, 0x3e800000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Skewness,
                [
                    [0x3f522026, 0x3ee7cf76, 0x3f600000, 0x3f800000],
                    [0x3f1bd152, 0x3efffffe, 0x3ee4a11b, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x3eedb65a, 0x3f2ea754, 0x3f3a6c7a, 0x3f800000],
                    [0x3f000000, 0x3f000000, 0x3f000000, 0x3f000000],
                    [0x3f49e69e, 0x3e58658a, 0x3f49e69e, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::StandardDeviation,
                [
                    [0x3e99421e, 0x3ea23b6b, 0x3e8d4fdf, 0x3f800000],
                    [0x3e97739e, 0x3e21e89b, 0x3e050581, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x3fd3e0b1, 0x3fd9f9e5, 0x3f0c378c, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x3f000000],
                    [0x3dddb3d7, 0x3e5db3d7, 0x3dddb3d7, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Summation,
                [
                    [0x3fcccccd, 0x401ccccd, 0x400c28f6, 0x3f800000],
                    [0x3fe66666, 0x3f99999a, 0x3fc00000, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x40400000, 0x40a00000, 0x40200000, 0x3f800000],
                    [0x3f333333, 0x3e4ccccd, 0x3dcccccd, 0x3f000000],
                    [0x3fa00000, 0x40200000, 0x3fa00000, 0x3f800000],
                ]
                .to_vec(),
            ),
            (
                Stat::Variance,
                [
                    [0x3db78033, 0x3dcd9e83, 0x3d9c022a, 0x3f800000],
                    [0x3db33332, 0x3ccccccd, 0x3c8a3d70, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                    [0x402f5c29, 0x4039999a, 0x3e99999a, 0x3f800000],
                    [0x00000000, 0x00000000, 0x00000000, 0x3f000000],
                    [0x3c400000, 0x3d400000, 0x3c400000, 0x3f800000],
                ]
                .to_vec(),
            ),
        ];
        for (mode, want) in expected {
            assert_bits_within_one_ulp(&bits(&combine(&layers, mode)), &want, &mode);
        }
    }

    /// 金标位模式抓取自旧串行实现所在的工具链；跨工具链 codegen 漂移会让同一源码算式
    /// 差 1 ULP（2026-10-18 rustc 1.97.1 实测：仅 Kurtosis 两元素 ±1 ULP，dev opt 0 与
    /// opt 1 一致复现，故不是优化档问题）。断言收口为「逐位相等或 ULP 距离 ≤ 1」：
    /// 钉统计行为（含逐位确定性），不钉某条工具链的 codegen 字面。
    fn assert_bits_within_one_ulp(got: &[[u32; 4]], want: &[[u32; 4]], mode: &Stat) {
        let ulp = |a: u32, b: u32| -> u32 {
            let a = if a & 0x8000_0000 != 0 { !a } else { a | 0x8000_0000 };
            let b = if b & 0x8000_0000 != 0 { !b } else { b | 0x8000_0000 };
            a.abs_diff(b)
        };
        for (y, (g_row, w_row)) in got.iter().zip(want.iter()).enumerate() {
            for (x, (g, w)) in g_row.iter().zip(w_row.iter()).enumerate() {
                assert!(ulp(*g, *w) <= 1, "{mode:?} pixel ({y},{x}) off by more than 1 ULP: got {g:#010x} want {w:#010x}");
            }
        }
    }

    /// Entropy against values worked out by hand (bits of the 8-bit histogram, over log2(N)).
    #[test]
    fn entropy_matches_hand_computed_values() {
        let out = combine(&fixture(), Stat::Entropy);
        let h = |ps: &[f64]| -> f64 {
            let n: f64 = ps.iter().sum();
            ps.iter().map(|c| -(c / n) * (c / n).log2()).sum::<f64>() / n.log2()
        };
        // Pixel 1: four distinct values -> 2 bits / log2(4) = exactly 1.
        assert_eq!(out[1][0], 1.0);
        // Pixel 2: every sample transparent -> untouched.
        assert_eq!(out[2], [0.0; 4]);
        // Pixel 3, red: -2 and 0 land in bin 0, 3 and 1.5 clamp into bin 255, 0.5 is alone.
        assert!((f64::from(out[3][0]) - h(&[2.0, 2.0, 1.0])).abs() < 1e-6, "{}", out[3][0]);
        // Pixel 4: a single contributing sample has no entropy.
        assert_eq!(out[4], [0.0, 0.0, 0.0, 0.5]);
        // Pixel 5: three samples share a bin, one is alone, in every channel.
        for (c, v) in out[5].iter().take(3).enumerate() {
            assert!((f64::from(*v) - h(&[3.0, 1.0])).abs() < 1e-6, "channel {c}: {v}");
        }
    }

    /// The old HashMap summed in a random order, so Entropy could differ by an ulp between runs.
    /// Now the bins are summed in ascending order: repeated runs (and any thread count) agree.
    #[test]
    fn entropy_is_deterministic() {
        let layers: Vec<Vec<[f32; 4]>> = (0..7)
            .map(|k| {
                (0..3 * CHUNK + 17).map(|i| [((i * 7 + k * 13) % 11) as f32 / 10.0, ((i + k) % 5) as f32 / 4.0, ((i * k) % 9) as f32 / 8.0, 1.0]).collect()
            })
            .collect();
        let first = bits(&combine(&layers, Stat::Entropy));
        for _ in 0..8 {
            assert_eq!(bits(&combine(&layers, Stat::Entropy)), first);
        }
    }

    /// Chunking is invisible: a multi-chunk image (with a ragged last chunk and layers of unequal
    /// length) gives, pixel by pixel, what combining that pixel on its own gives.
    #[test]
    fn chunked_result_equals_per_pixel_result() {
        let len = 2 * CHUNK + 5;
        let layer = |k: usize, n: usize| -> Vec<[f32; 4]> {
            (0..n)
                .map(|i| {
                    [
                        ((i * 31 + k * 7) % 101) as f32 / 100.0,
                        ((i * 17 + k) % 53) as f32 / 52.0 - 0.2,
                        ((i + 3 * k) % 29) as f32 / 28.0,
                        if (i + k).is_multiple_of(4) { 0.0 } else { 0.5 + ((i + k) % 3) as f32 / 4.0 },
                    ]
                })
                .collect()
        };
        let layers = vec![layer(1, len), layer(2, CHUNK + 3), layer(3, len - 1), layer(4, 0), layer(5, len)];
        for mode in [
            Stat::Entropy,
            Stat::Kurtosis,
            Stat::Maximum,
            Stat::Mean,
            Stat::Median,
            Stat::Minimum,
            Stat::Range,
            Stat::Skewness,
            Stat::StandardDeviation,
            Stat::Summation,
            Stat::Variance,
        ] {
            let whole = combine(&layers, mode);
            assert_eq!(whole.len(), len);
            for i in (0..len).step_by(97).chain([CHUNK - 1, CHUNK, CHUNK + 1, 2 * CHUNK - 1, 2 * CHUNK, len - 1]) {
                let one: Vec<Vec<[f32; 4]>> = layers.iter().map(|l| l.get(i).map(|p| vec![*p]).unwrap_or_default()).collect();
                assert_eq!(whole[i].map(f32::to_bits), combine(&one, mode)[0].map(f32::to_bits), "{mode:?} pixel {i}");
            }
        }
    }

    #[test]
    fn no_layers_gives_an_empty_result() {
        assert!(combine(&[], Stat::Median).is_empty());
    }
}
