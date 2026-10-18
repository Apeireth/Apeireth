// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Mode-native compositing of plain CMYK layer stacks (#3778).
//!
//! The compositor works in display RGB, so flattening a CMYK document through it reads every ink
//! value through the profile and separates it again: even a lone Background comes back with
//! different ink. When every visible layer is a plain raster layer in Normal mode (no masks,
//! effects, clipping or advanced blending), the composite needs no colour conversion at all:
//! Normal mixes ink values linearly, as Photoshop does in CMYK, so it is worked out on the stored
//! values and an opaque layer keeps them exactly. A layer mask (unfeathered) scales the mix, as
//! it does there.
//!
//! Plain adjustment layers that Photoshop evaluates on the inks (Invert, Posterize, Threshold,
//! Brightness/Contrast, Hue/Saturation, Color Balance, and Levels and Curves on CMYK channels)
//! join such a stack too: on the
//! stored inks their rules are exact, where the RGB compositor can only apply them to a separation
//! of its RGB composite, whose black differs from the file's. Such stacks are also rendered for
//! display this way ([`render_cmyk`]); stacks of raster layers alone keep the RGB compositor there.

use apeireth_image_color::blend::BlendMode;
use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
use apeireth_image_doc::adjust::ToneSpace;
use apeireth_image_doc::{Adjustment, Document, Layer, LayerContent, LayerMask};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

use crate::{Buffer, adjust};

/// Process channels of a CMYK pixel.
const INKS: usize = 4;

/// Whether the compositor would only blend `layer` Normally at its opacity through its layer mask:
/// no clipping, vector mask, feathered mask, effects, excluded channels, Blend If or advanced
/// blending.
fn simple(layer: &Layer) -> bool {
    layer.blend == BlendMode::Normal
        && !layer.clipped
        && !crate::masks::has_feather(layer)
        && layer.vector_mask.as_ref().is_none_or(|v| !v.enabled)
        && !crate::effects::has_effects(layer)
        && layer.excluded_channels == 0
        && layer.blend_if.is_default()
        && layer.advanced.is_default()
        && layer.video.is_none()
}

/// A layer the native path can composite: a visible raster layer the compositor would only blend
/// Normally at its opacity.
fn plain(layer: &Layer) -> Option<&Surface> {
    let LayerContent::Raster(s) = &layer.content else { return None };
    (simple(layer) && s.format().mode == ColorMode::Cmyk).then_some(s)
}

/// An adjustment on straight inks, prepared once per composite.
enum InkOp<'a> {
    /// Each ink's stored value (1 - ink) through its channel's table: C, M, Y, K
    /// (`adjust::ink_through`; Levels, Curves and Brightness/Contrast).
    Tables([Vec<f32>; INKS]),
    /// Invert, Posterize or Threshold (`adjust::stored_channels`).
    Rules(&'a Adjustment),
    /// Hue/Saturation or Color Balance on C, M and Y as inverted RGB (`adjust::apply_cmy`).
    Cmy(&'a Adjustment),
}

impl InkOp<'_> {
    /// The adjusted inks of each pixel of `inks` that has coverage (its fifth value).
    fn apply(&self, inks: &[[f32; INKS + 1]], depth: SampleType) -> Vec<[f32; INKS]> {
        let mut out: Vec<[f32; INKS]> = inks.iter().map(|d| [d[0], d[1], d[2], d[3]]).collect();
        match self {
            InkOp::Tables(t) => {
                for ink in &mut out {
                    for (v, table) in ink.iter_mut().zip(t) {
                        *v = adjust::ink_through(table, *v);
                    }
                }
            }
            InkOp::Rules(adj) => out.iter_mut().for_each(|ink| adjust::stored_channels(adj, ColorMode::Cmyk, Some(depth), ink)),
            InkOp::Cmy(adj) => adjust::apply_cmy(adj, &mut out),
        }
        out
    }
}

/// The ink evaluation of an adjustment layer the native path can composite.
fn ink_op(layer: &Layer, depth: SampleType) -> Option<InkOp<'_>> {
    let LayerContent::Adjustment(adj) = &layer.content else { return None };
    if !simple(layer) {
        return None;
    }
    match adj {
        Adjustment::Invert | Adjustment::Posterize { .. } | Adjustment::Threshold { .. } => Some(InkOp::Rules(adj)),
        _ if adjust::cmy_adjustment(adj) => Some(InkOp::Cmy(adj)),
        Adjustment::BrightnessContrast { brightness, contrast, legacy } => {
            let t = adjust::ink_brightness_contrast_lut(*brightness, *contrast, *legacy);
            Some(InkOp::Tables([t.clone(), t.clone(), t.clone(), t]))
        }
        Adjustment::Levels { space: ToneSpace::Cmyk, .. } | Adjustment::Curves { space: ToneSpace::Cmyk, .. } => {
            Some(InkOp::Tables(adjust::tone_luts_depth(adj, Some(depth))))
        }
        _ => None,
    }
}

/// One step of a native stack, at the layer's opacity × fill, through its mask.
enum Item<'a> {
    Layer(&'a Surface, f32, Option<&'a LayerMask>),
    Adjust(InkOp<'a>, f32, Option<&'a LayerMask>),
}

/// Groups nested deeper than this keep the RGB compositor.
const MAX_DEPTH: usize = 64;

/// `layers`' visible steps, appended to `items`; `None` when one needs the RGB compositor. A group
/// at full opacity without a mask is inlined: pass-through groups composite their children straight
/// onto the backdrop, and so do isolated (Normal) groups of raster layers alone, since Normal
/// composites associatively; an adjustment inside an isolated group would only see the group's
/// own content, so such a group keeps the RGB compositor (`adjustments` false).
fn collect<'a>(layers: &'a [Layer], depth: SampleType, adjustments: bool, items: &mut Vec<Item<'a>>, level: usize) -> Option<()> {
    if level > MAX_DEPTH {
        return None;
    }
    for layer in layers.iter().filter(|l| l.visible) {
        let alpha = (layer.opacity * layer.fill_opacity).clamp(0.0, 1.0);
        let mask = layer.mask.as_ref().filter(|m| m.enabled);
        if let LayerContent::Group(g) = &layer.content {
            let open = g.artboard.is_none()
                && alpha >= 1.0
                && mask.is_none()
                && !layer.clipped
                && layer.vector_mask.as_ref().is_none_or(|v| !v.enabled)
                && !crate::effects::has_effects(layer)
                && layer.excluded_channels == 0
                && layer.blend_if.is_default()
                && layer.advanced.is_default();
            match layer.blend {
                BlendMode::PassThrough if open => collect(&g.children, depth, adjustments, items, level + 1)?,
                BlendMode::Normal if open => collect(&g.children, depth, false, items, level + 1)?,
                _ => return None,
            }
            continue;
        }
        let item = match plain(layer) {
            Some(s) => Item::Layer(s, alpha, mask),
            None if adjustments => Item::Adjust(ink_op(layer, depth)?, alpha, mask),
            None => return None,
        };
        items.push(item);
    }
    Some(())
}

/// A CMYK document's visible layers, prepared for compositing on their ink values.
pub struct InkStack<'a> {
    items: Vec<Item<'a>>,
    depth: SampleType,
}

impl<'a> InkStack<'a> {
    /// `None` unless `doc` is CMYK and every visible layer is a plain raster layer, a plain
    /// adjustment layer the inks carry exactly, or a group that can be inlined (see the module
    /// docs and [`collect`]).
    pub fn new(doc: &'a Document) -> Option<Self> {
        if doc.mode != ColorMode::Cmyk {
            return None;
        }
        let mut items = Vec::new();
        collect(&doc.layers, doc.depth, true, &mut items, 0)?;
        Some(InkStack { items, depth: doc.depth })
    }

    /// Whether the stack has an adjustment layer (the RGB compositor can't match it exactly).
    pub fn adjusts(&self) -> bool {
        self.items.iter().any(|i| matches!(i, Item::Adjust(..)))
    }

    /// Straight C, M, Y, K and alpha over `rect`, row-major, into `acc` (`src` and `mv` are
    /// scratch).
    fn composite(&self, rect: Rect, acc: &mut Vec<[f32; INKS + 1]>, src: &mut Vec<f32>, mv: &mut Vec<f32>) {
        let n = rect.width() as usize * rect.height() as usize;
        acc.clear();
        acc.resize(n, [0.0f32; INKS + 1]);
        // The mask's value at pixel `i` (1 without one).
        let masked = |mask: Option<&LayerMask>, mv: &mut Vec<f32>| match mask {
            Some(m) => m.values_into(rect, mv),
            None => {
                mv.clear();
                mv.resize(n, 1.0);
            }
        };
        for item in &self.items {
            match item {
                Item::Layer(surf, opacity, mask) => {
                    let opacity = *opacity;
                    if opacity <= 0.0 || (surf.content_bounds().intersect(&rect).is_empty() && surf.default_pixel().iter().all(|v| *v == 0.0)) {
                        continue;
                    }
                    let sf = surf.format();
                    let sn = sf.channels();
                    surf.read_region_into(rect, src);
                    masked(*mask, mv);
                    for ((d, s), k) in acc.iter_mut().zip(src.chunks_exact(sn)).zip(mv.iter()) {
                        let sa = if sf.alpha { s.get(INKS).copied().unwrap_or(1.0) } else { 1.0 } * opacity * k.clamp(0.0, 1.0);
                        if sa <= 0.0 {
                            continue;
                        }
                        let oa = sa + d[INKS] * (1.0 - sa);
                        let keep = d[INKS] * (1.0 - sa);
                        for (c, v) in s.iter().take(INKS).enumerate() {
                            // An opaque pixel replaces what is below exactly (no rounding through the mix).
                            d[c] = if sa >= 1.0 { *v } else { (v * sa + d[c] * keep) / oa };
                        }
                        d[INKS] = oa;
                    }
                }
                Item::Adjust(op, opacity, mask) => {
                    if *opacity <= 0.0 {
                        continue;
                    }
                    masked(*mask, mv);
                    let adjusted = op.apply(acc, self.depth);
                    for ((d, k), ink) in acc.iter_mut().zip(mv.iter()).zip(&adjusted) {
                        let mix = opacity * k.clamp(0.0, 1.0);
                        if d[INKS] <= 0.0 || mix <= 0.0 {
                            continue;
                        }
                        for (c, v) in ink.iter().enumerate() {
                            d[c] += (v - d[c]) * mix;
                        }
                    }
                }
            }
        }
    }
}

/// The composite of `doc` over `rect` in `fmt`, worked out in ink values, or `None` when the
/// document isn't CMYK or a visible layer needs the full compositor (see the module docs).
/// `paper` flattens onto white paper (no ink) and makes the result opaque.
pub fn flatten_cmyk(doc: &Document, rect: Rect, fmt: PixelFormat, paper: bool) -> Option<Surface> {
    if fmt.mode != ColorMode::Cmyk {
        return None;
    }
    let stack = InkStack::new(doc)?;
    let mut out = Surface::new(fmt);
    if rect.is_empty() {
        return Some(out);
    }
    let n = fmt.channels();
    let rows = crate::band_rows_for(rect.width(), 0);
    let (mut acc, mut src, mut mv, mut data) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut y = rect.y0;
    while y < rect.y1 {
        let band = Rect::new(rect.x0, y, rect.x1, y.saturating_add(rows).min(rect.y1));
        stack.composite(band, &mut acc, &mut src, &mut mv);
        data.clear();
        data.resize(acc.len() * n, 0.0);
        for (p, o) in acc.iter().zip(data.chunks_exact_mut(n)) {
            // White paper carries no ink, so flattening onto it scales the ink by coverage.
            let a = p[INKS];
            let k = if paper && a < 1.0 { a } else { 1.0 };
            for c in 0..INKS {
                o[c] = p[c] * k;
            }
            if fmt.alpha {
                o[INKS] = if paper { 1.0 } else { a };
            }
        }
        out.write_region(band, &data);
        y = band.y1;
    }
    out.prune();
    Some(out)
}

/// The display composite (straight RGBA, through the document's CMYK profile) of a CMYK document
/// whose stack has adjustment layers the inks carry exactly, worked out on the inks; `None` for
/// any other document, which the RGB compositor renders. Rows of `tile` render in parallel.
pub fn render_cmyk(doc: &Document, rect: Rect, tile: i32) -> Option<Buffer> {
    let stack = InkStack::new(doc).filter(InkStack::adjusts)?;
    let cmyk = crate::cmyk_space(doc);
    let w = rect.width() as usize;
    let mut out = Buffer::transparent(rect);
    if w == 0 || rect.is_empty() {
        return Some(out);
    }
    let rows = tile.max(1) as usize;
    let band = |i: usize, px: &mut [[f32; 4]]| {
        let y0 = rect.y0.saturating_add((i * rows) as i32);
        let r = Rect::new(rect.x0, y0, rect.x1, y0.saturating_add((px.len() / w) as i32));
        let (mut acc, mut src, mut mv) = (Vec::new(), Vec::new(), Vec::new());
        stack.composite(r, &mut acc, &mut src, &mut mv);
        apeireth_image_color::convert::with_cmyk_space(cmyk.as_ref(), || {
            for (o, p) in px.iter_mut().zip(&acc) {
                if p[INKS] > 0.0 {
                    let [r, g, b] = apeireth_image_color::convert::cmyk_to_rgb([p[0], p[1], p[2], p[3]]);
                    *o = [r, g, b, p[INKS]];
                }
            }
        });
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        out.px.par_chunks_mut(w * rows).enumerate().for_each(|(i, px)| band(i, px));
    }
    #[cfg(target_arch = "wasm32")]
    for (i, px) in out.px.chunks_mut(w * rows).enumerate() {
        band(i, px);
    }
    Some(out)
}
