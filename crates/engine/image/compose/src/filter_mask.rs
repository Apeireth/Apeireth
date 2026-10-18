// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! A smart object's filter mask (#215): how the filtered pixels and the unfiltered (placed) ones
//! combine.
//!
//! Photoshop stores a smart object's layer pixels with its filters applied but not its filter
//! mask, and mixes the unfiltered pixels back in through the mask when it composites (photoshop
//! corpus `sf-filter-mask-hard.psd` and `sf-filter-mask-gradient.psd`: the stored pixels are
//! filtered everywhere, the merged image only where the mask shows the filter). Our smart objects
//! keep the mixed result as their pixels, so PSD import mixes ([`mix`]) and export unmixes
//! ([`unmix`]).

use apeireth_image_doc::LayerMask;
use apeireth_image_raster::Surface;

/// `filtered` mixed over `base` (the unfiltered pixels) through `mask`: premultiplied, any
/// channel layout. Pixels the mask hides entirely are `base`'s.
pub fn mix(base: &Surface, filtered: &Surface, mask: &LayerMask) -> Surface {
    let fmt = base.format();
    let area = base.content_bounds().union(&filtered.content_bounds());
    let mut out = filtered.clone();
    if area.is_empty() || filtered.format() != fmt {
        return out;
    }
    let n = fmt.channels();
    let (b, f) = (base.read_region(area), filtered.read_region(area));
    let mut k = Vec::new();
    mask.values_into(area, &mut k);
    let mut o = vec![0.0f32; b.len()];
    for (i, ((bp, fp), op)) in b.chunks_exact(n).zip(f.chunks_exact(n)).zip(o.chunks_exact_mut(n)).enumerate() {
        let m = k.get(i).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        if !fmt.alpha {
            for c in 0..n {
                op[c] = bp[c] + (fp[c] - bp[c]) * m;
            }
            continue;
        }
        let a = n - 1;
        let oa = bp[a] + (fp[a] - bp[a]) * m;
        op[a] = oa;
        for c in 0..a {
            let pm = bp[c] * bp[a] + (fp[c] * fp[a] - bp[c] * bp[a]) * m;
            op[c] = if oa > 0.0 { (pm / oa).clamp(0.0, 1.0) } else { 0.0 };
        }
    }
    out.write_region(area, &o);
    out.prune();
    out
}

/// The mask value below which a pixel's filtered value is taken to be unknown: [`unmix`] gives the
/// unfiltered value there, which the mask shows (almost) none of anyway.
const HIDDEN: f32 = 1.0 / 1024.0;

/// The inverse of [`mix`]: the filtered pixels that mix with `base` through `mask` to `mixed`
/// (clamped to 0..1). Where the mask hides the filter entirely they are `base`'s, which Photoshop
/// doesn't show there.
pub fn unmix(base: &Surface, mixed: &Surface, mask: &LayerMask) -> Surface {
    let fmt = base.format();
    let area = base.content_bounds().union(&mixed.content_bounds());
    let mut out = mixed.clone();
    if area.is_empty() || mixed.format() != fmt {
        return out;
    }
    let n = fmt.channels();
    let (b, x) = (base.read_region(area), mixed.read_region(area));
    let mut k = Vec::new();
    mask.values_into(area, &mut k);
    let mut o = vec![0.0f32; b.len()];
    for (i, ((bp, xp), op)) in b.chunks_exact(n).zip(x.chunks_exact(n)).zip(o.chunks_exact_mut(n)).enumerate() {
        let m = k.get(i).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        if m < HIDDEN {
            // A transparent base pixel (Photoshop's cache keeps white there) stays empty, so the
            // stored pixels don't grow past the content.
            if !fmt.alpha || bp[n - 1] > 0.0 {
                op.copy_from_slice(bp);
            }
            continue;
        }
        if !fmt.alpha {
            for c in 0..n {
                op[c] = (bp[c] + (xp[c] - bp[c]) / m).clamp(0.0, 1.0);
            }
            continue;
        }
        let a = n - 1;
        let fa = (bp[a] + (xp[a] - bp[a]) / m).clamp(0.0, 1.0);
        if fa <= 0.0 {
            continue;
        }
        op[a] = fa;
        for c in 0..a {
            let pm = bp[c] * bp[a] + (xp[c] * xp[a] - bp[c] * bp[a]) / m;
            op[c] = (pm / fa).clamp(0.0, 1.0);
        }
    }
    out.write_region(area, &o);
    out.prune();
    out
}

#[cfg(test)]
mod tests {
    use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
    use apeireth_image_doc::LayerMask;
    use apeireth_image_raster::Surface;

    use super::*;

    fn surface(fmt: PixelFormat, f: impl Fn(i32, i32) -> Vec<f32>) -> Surface {
        let mut s = Surface::new(fmt);
        for y in 0..4 {
            for x in 0..8 {
                s.write_pixel(x, y, &f(x, y));
            }
        }
        s
    }

    /// A mask ramping from hidden (left) to shown (right).
    fn ramp() -> LayerMask {
        let mut m = LayerMask::reveal_all();
        m.surface = surface(PixelFormat::new(ColorMode::Grayscale, SampleType::F32, false), |x, _| vec![x as f32 / 7.0]);
        m
    }

    #[test]
    fn unmix_inverts_mix_wherever_the_mask_shows_the_filter() {
        for fmt in [PixelFormat::new(ColorMode::Rgb, SampleType::F32, true), PixelFormat::new(ColorMode::Rgb, SampleType::F32, false)] {
            let base = surface(fmt, |x, y| {
                let mut p = vec![0.2 + 0.05 * x as f32, 0.6, 0.1 * y as f32];
                if fmt.alpha {
                    p.push(0.5 + 0.06 * x as f32);
                }
                p
            });
            let filtered = surface(fmt, |_, y| {
                let mut p = vec![0.9 - 0.1 * y as f32, 0.3, 0.5];
                if fmt.alpha {
                    p.push(0.8);
                }
                p
            });
            let m = ramp();
            let mixed = mix(&base, &filtered, &m);
            // Hidden column: the unfiltered pixels; shown column: the filtered ones.
            assert!(mixed.pixel(0, 2).iter().zip(base.pixel(0, 2)).all(|(a, b)| (a - b).abs() < 1e-6));
            assert!(mixed.pixel(7, 2).iter().zip(filtered.pixel(7, 2)).all(|(a, b)| (a - b).abs() < 1e-6));
            let back = unmix(&base, &mixed, &m);
            for y in 0..4 {
                for x in 1..8 {
                    let (g, w) = (back.pixel(x, y), filtered.pixel(x, y));
                    assert!(g.iter().zip(&w).all(|(a, b)| (a - b).abs() < 1e-4), "{fmt:?} ({x}, {y}): {g:?} vs {w:?}");
                }
                // Where the mask hides the filter entirely its value is unknown: the base stands in.
                assert_eq!(back.pixel(0, y), base.pixel(0, y));
            }
            // So mixing the unmixed pixels gives the mixed ones back everywhere.
            let again = mix(&base, &back, &m);
            for y in 0..4 {
                for x in 0..8 {
                    assert!(again.pixel(x, y).iter().zip(mixed.pixel(x, y)).all(|(a, b)| (a - b).abs() < 1e-4));
                }
            }
        }
    }

    #[test]
    fn transparent_base_pixels_stay_empty() {
        // Photoshop's unfiltered cache carries white in transparent pixels; where the mask hides
        // the filter those must not become content of the stored pixels.
        let fmt = PixelFormat::new(ColorMode::Rgb, SampleType::U8, true);
        let base = surface(fmt, |x, _| if x < 4 { vec![1.0, 1.0, 1.0, 0.0] } else { vec![0.3, 0.4, 0.5, 1.0] });
        let mut hidden = LayerMask::reveal_all();
        hidden.surface = Surface::with_default(PixelFormat::GRAY8, &[0.0]);
        let mixed = mix(&base, &base, &hidden);
        let back = unmix(&base, &mixed, &hidden);
        assert_eq!(back.pixel(1, 1), vec![0.0; 4]);
        assert_eq!(back.content_bounds().x0, 4);
    }

    #[test]
    fn mismatched_or_empty_surfaces_are_left_alone() {
        let rgba = PixelFormat::new(ColorMode::Rgb, SampleType::U8, true);
        let gray = PixelFormat::new(ColorMode::Grayscale, SampleType::U8, true);
        let a = surface(rgba, |_, _| vec![0.5, 0.5, 0.5, 1.0]);
        let b = surface(gray, |_, _| vec![0.1, 1.0]);
        assert_eq!(mix(&a, &b, &ramp()), b);
        assert_eq!(unmix(&a, &b, &ramp()), b);
        let empty = Surface::new(rgba);
        assert_eq!(unmix(&empty, &empty, &ramp()), empty);
    }
}
