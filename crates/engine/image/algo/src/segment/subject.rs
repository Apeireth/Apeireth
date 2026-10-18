// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Select Subject (classical, heuristic).
//!
//! Photoshop uses a learned model; we approximate it with salient-object detection feeding a
//! GrabCut:
//!
//! 1. SLIC superpixels on a working-resolution copy.
//! 2. Per-superpixel saliency from "Saliency Optimization from Robust Background Detection"
//!    (W. Zhu, S. Liang, Y. Wei and J. Sun, CVPR 2014): boundary connectivity
//!    `BndCon(p) = Len_bnd(p) / sqrt(Area(p))` over geodesic colour distances gives a background
//!    probability; background-weighted contrast gives foreground cues; a least-squares
//!    optimisation with smoothness between neighbours combines them.
//! 3. Blended with frequency-tuned saliency (R. Achanta, S. Hemami, F. Estrada and
//!    S. Süsstrunk, "Frequency-tuned Salient Region Detection", CVPR 2009): distance of the
//!    lightly blurred CIELAB image from its mean colour.
//! 4. Otsu's threshold on the saliency map seeds a GrabCut trimap (low-saliency border pixels
//!    are definite background); the result is cleaned (small islands dropped).
//! 5. A second GrabCut whose background model only knows the backdrop: a colour model fitted
//!    to the image's frame marks every pixel it explains as probable background, and the rest
//!    (the blob, plus whatever the backdrop cannot account for) starts as probable foreground.
//!    Parts of the subject whose colour is close to the backdrop (a cream shirt on a near-white
//!    wall) have low saliency and in step 4 land in the background model, which then explains
//!    them away; here they start as foreground and the cut keeps them (#2788). Only what
//!    touches the first pass's result survives, so the pass cannot pull in unrelated things.
//! 6. Holes are filled only when their colour fits the foreground model better than the
//!    background's, so a patch of backdrop enclosed by the subject (between an arm and the
//!    torso) stays open; specks still fill. Then the mask is re-cut at full resolution.
//!
//! Works well for a distinct object on a comparatively plain background; it is not a semantic
//! segmenter.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use apeireth_image_geom::Rect;
use apeireth_image_raster::{Cancelled, Interrupt};

use super::gmm::Gmm;
use super::grabcut::{self, BG, PR_BG, PR_FG};
use super::slic::{rgb_to_lab, slic_lab};
use super::{Region, Sampler};

/// Working-resolution budget (pixels).
pub const WORK_PX: usize = 110_000;
const SIGMA_CLR: f32 = 10.0;
const SIGMA_SPA: f32 = 0.25;

/// Selects the most salient object on the canvas.
pub fn select_subject(sampler: &dyn Sampler, canvas: Rect) -> Option<Region> {
    select_subject_with(sampler, canvas, &Interrupt::NONE).unwrap_or_default()
}

/// [`select_subject`] that stops with `Err(Cancelled)` once `ctl` is cancelled, asked between
/// stages and inside every min cut (the GrabCut iterations and the full-resolution re-cut).
pub fn select_subject_with(sampler: &dyn Sampler, canvas: Rect, ctl: &Interrupt) -> Result<Option<Region>, Cancelled> {
    if canvas.width() < 4 || canvas.height() < 4 {
        return Ok(None);
    }
    let step = super::scale_for(canvas, WORK_PX);
    let img = sampler.rgb_scaled(canvas, step);
    let (w, h) = (img.w, img.h);
    let sal = saliency(&img);
    ctl.check()?;
    let t = otsu(&sal).clamp(0.15, 0.85);
    let mut trimap = vec![PR_BG; w * h];
    let mut any_fg = false;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let border = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            trimap[i] = if sal[i] >= t {
                any_fg = true;
                PR_FG
            } else if border || sal[i] < 0.4 * t {
                BG
            } else {
                PR_BG
            };
        }
    }
    if !any_fg {
        return Ok(None);
    }
    let Some((fg, bg)) = grabcut::grabcut_with(&img, &mut trimap, 6, ctl)? else { return Ok(None) };
    let low: Vec<bool> = trimap.iter().map(|v| *v == PR_FG || *v == grabcut::FG).collect();
    let low = super::clean_mask(&low, w, h, 0.2, 0.0);
    if !low.iter().any(|v| *v) {
        return Ok(None);
    }
    let (low, fg, bg) = backdrop_pass(&img, &low, ctl)?.unwrap_or((low, fg, bg));
    ctl.check()?;
    let low = super::fill_holes_by_model(&low, &img, &fg, &bg, 0.002, 0.05);
    super::finish_region_with(sampler, canvas, step, &low, w, h, Some((&fg, &bg)), ctl)
}

/// Step 5: GrabCut again with a background model that only knows the backdrop. A colour model
/// is fitted to the image's frame (the outer pixels, minus a margin around `blob` in case the
/// subject runs out of the picture); the frame is definite background, every other pixel it
/// explains well is probable background, and the rest (the blob, and anything the backdrop
/// cannot account for, such as a cream shirt on a near-white wall) starts as probable
/// foreground. `Ok(None)` when the frame cannot be modelled or the cut keeps nothing that
/// touches the blob; `Err(Cancelled)` once `ctl` is cancelled, as in the first pass.
fn backdrop_pass(img: &super::RgbImage, blob: &[bool], ctl: &Interrupt) -> Result<Option<(Vec<bool>, Gmm, Gmm)>, Cancelled> {
    let (w, h) = (img.w, img.h);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0usize, 0usize);
    for (i, on) in blob.iter().enumerate() {
        if *on {
            let (x, y) = (i % w, i / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if x1 < x0 || y1 < y0 {
        return Ok(None);
    }
    let margin = (((x1 - x0 + 1).max(y1 - y0 + 1) as f32 * 0.15).ceil() as usize).max(4);
    let (bx0, by0) = (x0.saturating_sub(margin), y0.saturating_sub(margin));
    let (bx1, by1) = ((x1 + margin).min(w - 1), (y1 + margin).min(h - 1));
    let frame = ((w.min(h) as f32 * 0.06).round() as usize).max(2);
    let in_frame = |x: usize, y: usize| x < frame || y < frame || x + frame >= w || y + frame >= h;
    let near_blob = |x: usize, y: usize| (bx0..=bx1).contains(&x) && (by0..=by1).contains(&y);
    let samples: Vec<[f32; 3]> = (0..w * h).filter(|i| in_frame(i % w, i / w) && !near_blob(i % w, i / w)).map(|i| img.px[i]).collect();
    let Some(backdrop) = Gmm::fit(&super::subsample(&samples, 40_000), grabcut::K, super::gmm::DEFAULT_REG) else { return Ok(None) };
    // What the frame itself costs under its own model bounds "explained well".
    let mut frame_costs: Vec<f32> = samples.iter().map(|p| backdrop.neg_log(*p)).collect();
    frame_costs.sort_by(|a, b| a.total_cmp(b));
    let Some(limit) = frame_costs.get(frame_costs.len() * 99 / 100).map(|c| c + 4.0) else { return Ok(None) };
    let mut trimap: Vec<u8> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            if in_frame(x, y) && !near_blob(x, y) {
                BG
            } else if blob[i] || backdrop.neg_log(img.px[i]) > limit {
                PR_FG
            } else {
                PR_BG
            }
        })
        .collect();
    let Some((fg, bg)) = grabcut::grabcut_with(img, &mut trimap, 4, ctl)? else { return Ok(None) };
    let cut: Vec<bool> = trimap.iter().map(|v| *v == PR_FG || *v == grabcut::FG).collect();
    let low = super::keep_seeded(&cut, blob, w, h);
    Ok(low.iter().any(|v| *v).then_some((low, fg, bg)))
}

/// Per-pixel saliency in `0..=1` (see the module docs).
pub fn saliency(img: &super::RgbImage) -> Vec<f32> {
    let (w, h) = (img.w, img.h);
    let lab: Vec<[f32; 3]> = img.px.iter().map(|p| rgb_to_lab(*p)).collect();
    let target = ((w * h) / 150).clamp(16, 400);
    let sp = slic_lab(&lab, w, h, target, 10.0, 8);
    let n = sp.count;
    // Region statistics.
    let mut mean = vec![[0.0f64; 3]; n];
    let mut pos = vec![[0.0f64; 2]; n];
    let mut cnt = vec![0.0f64; n];
    let mut on_border = vec![false; n];
    for y in 0..h {
        for x in 0..w {
            let l = sp.labels[y * w + x] as usize;
            let c = lab[y * w + x];
            for k in 0..3 {
                mean[l][k] += c[k] as f64;
            }
            pos[l][0] += x as f64 / w as f64;
            pos[l][1] += y as f64 / h as f64;
            cnt[l] += 1.0;
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                on_border[l] = true;
            }
        }
    }
    let mean: Vec<[f32; 3]> = mean.iter().zip(&cnt).map(|(m, c)| m.map(|v| (v / c.max(1.0)) as f32)).collect();
    let pos: Vec<[f32; 2]> = pos.iter().zip(&cnt).map(|(p, c)| p.map(|v| (v / c.max(1.0)) as f32)).collect();
    let dapp = |a: usize, b: usize| super::d2(mean[a], mean[b]).sqrt();
    // Adjacency.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut link = |a: usize, b: usize| {
        if a != b && !adj[a].contains(&b) {
            adj[a].push(b);
            adj[b].push(a);
        }
    };
    for y in 0..h {
        for x in 0..w {
            let l = sp.labels[y * w + x] as usize;
            if x + 1 < w {
                link(l, sp.labels[y * w + x + 1] as usize);
            }
            if y + 1 < h {
                link(l, sp.labels[(y + 1) * w + x] as usize);
            }
        }
    }
    // Boundary connectivity from all-pairs geodesic distances (Dijkstra per region).
    let mut wbg = vec![0.0f32; n];
    let mut geo = vec![f32::INFINITY; n];
    for (p, wb) in wbg.iter_mut().enumerate() {
        dijkstra(p, &adj, &dapp, &mut geo);
        let (mut area, mut len) = (0.0f32, 0.0f32);
        for q in 0..n {
            let s = (-geo[q] * geo[q] / (2.0 * SIGMA_CLR * SIGMA_CLR)).exp();
            area += s;
            if on_border[q] {
                len += s;
            }
        }
        let bnd = len / area.max(1e-6).sqrt();
        *wb = 1.0 - (-bnd * bnd / 2.0).exp();
    }
    // Background-weighted contrast.
    let mut wctr = vec![0.0f32; n];
    for p in 0..n {
        let mut s = 0.0;
        for q in 0..n {
            let (dx, dy) = (pos[p][0] - pos[q][0], pos[p][1] - pos[q][1]);
            let wspa = (-(dx * dx + dy * dy) / (2.0 * SIGMA_SPA * SIGMA_SPA)).exp();
            s += dapp(p, q) * wspa * wbg[q];
        }
        wctr[p] = s;
    }
    let mx = wctr.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let wfg: Vec<f32> = wctr.iter().map(|v| v / mx).collect();
    // Saliency optimisation: min Σ wbg s² + Σ wfg (s−1)² + Σ_{i~j} w_ij (s_i − s_j)² (Gauss–Seidel).
    let wij: Vec<Vec<f32>> = (0..n).map(|p| adj[p].iter().map(|&q| (-dapp(p, q).powi(2) / (2.0 * SIGMA_CLR * SIGMA_CLR)).exp() + 0.1).collect()).collect();
    let mut s = wfg.clone();
    for _ in 0..200 {
        for p in 0..n {
            let (mut num, mut den) = (wfg[p], wbg[p] + wfg[p]);
            for (q, wq) in adj[p].iter().zip(&wij[p]) {
                num += wq * s[*q];
                den += wq;
            }
            s[p] = num / den.max(1e-6);
        }
    }
    // Frequency-tuned saliency.
    let mut mean_all = [0.0f64; 3];
    for c in &lab {
        for k in 0..3 {
            mean_all[k] += c[k] as f64;
        }
    }
    let mean_all = mean_all.map(|v| (v / lab.len() as f64) as f32);
    let blurred = super::RgbImage { w, h, px: lab.clone() };
    let blurred = blur_lab(&blurred);
    let ft: Vec<f32> = blurred.px.iter().map(|c| super::d2(*c, mean_all).sqrt()).collect();
    let ftmax = ft.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let smin = s.iter().copied().fold(f32::MAX, f32::min);
    let smax = s.iter().copied().fold(f32::MIN, f32::max);
    let srange = (smax - smin).max(1e-6);
    let mut out: Vec<f32> = sp.labels.iter().zip(&ft).map(|(l, f)| 0.75 * (s[*l as usize] - smin) / srange + 0.25 * f / ftmax).collect();
    let (lo, hi) = out.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let r = (hi - lo).max(1e-6);
    for v in &mut out {
        *v = (*v - lo) / r;
    }
    out
}

fn blur_lab(img: &super::RgbImage) -> super::RgbImage {
    let (w, h) = (img.w, img.h);
    let mut cur = img.clone();
    for _ in 0..2 {
        let mut tmp = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let (l, c, r) = (cur.at(x.saturating_sub(1), y), cur.at(x, y), cur.at((x + 1).min(w - 1), y));
                tmp.px[y * w + x] = [0, 1, 2].map(|k| (l[k] + 2.0 * c[k] + r[k]) / 4.0);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let (u, c, d) = (tmp.at(x, y.saturating_sub(1)), tmp.at(x, y), tmp.at(x, (y + 1).min(h - 1)));
                cur.px[y * w + x] = [0, 1, 2].map(|k| (u[k] + 2.0 * c[k] + d[k]) / 4.0);
            }
        }
    }
    cur
}

#[derive(PartialEq)]
struct Item(f32, usize);
impl Eq for Item {}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0)
    }
}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn dijkstra(src: usize, adj: &[Vec<usize>], wt: &dyn Fn(usize, usize) -> f32, dist: &mut [f32]) {
    dist.fill(f32::INFINITY);
    dist[src] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(Item(0.0, src));
    while let Some(Item(d, u)) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        for &v in &adj[u] {
            let nd = d + wt(u, v);
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Item(nd, v));
            }
        }
    }
}

/// Otsu's threshold of values in `0..=1` (256 bins).
pub fn otsu(v: &[f32]) -> f32 {
    let mut hist = [0u64; 256];
    for x in v {
        hist[(x.clamp(0.0, 1.0) * 255.0).round() as usize] += 1;
    }
    let total = v.len() as f64;
    let sum: f64 = hist.iter().enumerate().map(|(i, c)| i as f64 * *c as f64).sum();
    let (mut wb, mut sb, mut best, mut bt) = (0.0f64, 0.0f64, -1.0f64, 0usize);
    for (t, c) in hist.iter().enumerate() {
        wb += *c as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sb += t as f64 * *c as f64;
        let (mb, mf) = (sb / wb, (sum - sb) / wf);
        let between = wb * wf * (mb - mf) * (mb - mf);
        if between > best {
            best = between;
            bt = t;
        }
    }
    (bt as f32 + 0.5) / 255.0
}
