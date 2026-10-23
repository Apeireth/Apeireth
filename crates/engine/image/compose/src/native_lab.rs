// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Lab-native compositing of plain Lab layer stacks.
//!
//! The compositor works in display RGB, so a Lab document's layers are read through `lab_to_srgb`,
//! which clips colours outside the display gamut, and adjustments Photoshop evaluates on L, a and
//! b ([`adjust::lab_stored_op`]) can then only see those clipped colours: an inverted or levelled
//! saturated colour comes out several levels off. When every visible layer is a plain Lab raster
//! layer in Normal mode or such an adjustment layer, the stack is worked out on the stored values
//! instead: Normal mixes L, a and b linearly, as Photoshop does in Lab, and an opaque layer keeps
//! them exactly. A layer mask (unfeathered) scales the mix.
//!
//! Stacks with such adjustment layers render for display this way ([`render_lab`]); flattening
//! ([`flatten_lab`]) keeps the stored values for any plain stack.

use apeireth_image_color::blend::BlendMode;
use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
use apeireth_image_doc::{Document, Layer, LayerContent, LayerMask};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

use crate::Buffer;
use crate::adjust::{self, LabOp};

/// Colour channels of a Lab pixel.
const LAB: usize = 3;

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

/// One step of a native stack, at the layer's opacity × fill, through its mask.
enum Item<'a> {
    Layer(&'a Surface, f32, Option<&'a LayerMask>),
    Adjust(LabOp<'a>, f32, Option<&'a LayerMask>),
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
        let item = match &layer.content {
            LayerContent::Group(g) => {
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
            LayerContent::Raster(s) if simple(layer) && s.format().mode == ColorMode::Lab => Item::Layer(s, alpha, mask),
            LayerContent::Adjustment(adj) if adjustments && simple(layer) => Item::Adjust(adjust::lab_stored_op(adj, Some(depth))?, alpha, mask),
            _ => return None,
        };
        items.push(item);
    }
    Some(())
}

/// A Lab document's visible layers, prepared for compositing on their stored L, a and b.
pub struct LabStack<'a> {
    items: Vec<Item<'a>>,
}

impl<'a> LabStack<'a> {
    /// `None` unless `doc` is Lab and every visible layer is a plain Lab raster layer, an
    /// adjustment layer [`adjust::lab_stored_op`] evaluates, or a group that can be inlined (see
    /// the module docs and `collect`).
    pub fn new(doc: &'a Document) -> Option<Self> {
        if doc.mode != ColorMode::Lab {
            return None;
        }
        let mut items = Vec::new();
        collect(&doc.layers, doc.depth, true, &mut items, 0)?;
        Some(LabStack { items })
    }

    /// Whether the stack has an adjustment layer (the RGB compositor can't match it exactly).
    pub fn adjusts(&self) -> bool {
        self.items.iter().any(|i| matches!(i, Item::Adjust(..)))
    }

    /// Straight L, a, b and alpha over `rect`, row-major, into `acc` (`src` and `mv` are scratch).
    fn composite(&self, rect: Rect, acc: &mut Vec<[f32; LAB + 1]>, src: &mut Vec<f32>, mv: &mut Vec<f32>) {
        let n = rect.width() as usize * rect.height() as usize;
        acc.clear();
        acc.resize(n, [0.0, adjust::LAB_NEUTRAL, adjust::LAB_NEUTRAL, 0.0]);
        // The mask's value at each pixel (1 without one).
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
                    if opacity <= 0.0 || (surf.content_bounds().intersect(&rect).is_empty() && surf.default_pixel().get(LAB).is_none_or(|a| *a <= 0.0)) {
                        continue;
                    }
                    let sf = surf.format();
                    let sn = sf.channels();
                    surf.read_region_into(rect, src);
                    masked(*mask, mv);
                    for ((d, s), k) in acc.iter_mut().zip(src.chunks_exact(sn)).zip(mv.iter()) {
                        let sa = if sf.alpha { s.get(LAB).copied().unwrap_or(1.0) } else { 1.0 } * opacity * k.clamp(0.0, 1.0);
                        if sa <= 0.0 {
                            continue;
                        }
                        let oa = sa + d[LAB] * (1.0 - sa);
                        let keep = d[LAB] * (1.0 - sa);
                        for (c, v) in s.iter().take(LAB).enumerate() {
                            // An opaque pixel replaces what is below exactly (no rounding through the mix).
                            d[c] = if sa >= 1.0 { *v } else { (v * sa + d[c] * keep) / oa };
                        }
                        d[LAB] = oa;
                    }
                }
                Item::Adjust(op, opacity, mask) => {
                    if *opacity <= 0.0 {
                        continue;
                    }
                    masked(*mask, mv);
                    for (d, k) in acc.iter_mut().zip(mv.iter()) {
                        let mix = opacity * k.clamp(0.0, 1.0);
                        if d[LAB] <= 0.0 || mix <= 0.0 {
                            continue;
                        }
                        let mut lab = [d[0], d[1], d[2]];
                        op(&mut lab);
                        for (c, v) in lab.iter().enumerate() {
                            d[c] += (v - d[c]) * mix;
                        }
                    }
                }
            }
        }
    }
}

/// The composite of `doc` over `rect` in `fmt`, worked out on stored L, a and b, or `None` when
/// the document isn't Lab, `fmt` isn't, or a visible layer needs the full compositor (see the
/// module docs). `white` flattens onto white (L 100, neutral a and b) and makes the result opaque.
pub fn flatten_lab(doc: &Document, rect: Rect, fmt: PixelFormat, white: bool) -> Option<Surface> {
    if fmt.mode != ColorMode::Lab {
        return None;
    }
    let stack = LabStack::new(doc)?;
    let mut out = Surface::new(fmt);
    if rect.is_empty() {
        return Some(out);
    }
    let n = fmt.channels();
    let empty = out.default_pixel();
    let rows = crate::band_rows_for(rect.width(), 0);
    let (mut acc, mut src, mut mv, mut data) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut y = rect.y0;
    while y < rect.y1 {
        let band = Rect::new(rect.x0, y, rect.x1, y.saturating_add(rows).min(rect.y1));
        stack.composite(band, &mut acc, &mut src, &mut mv);
        data.clear();
        data.resize(acc.len() * n, 0.0);
        for (p, o) in acc.iter().zip(data.chunks_exact_mut(n)) {
            let a = p[LAB];
            if white {
                let paper = [1.0, adjust::LAB_NEUTRAL, adjust::LAB_NEUTRAL];
                for c in 0..LAB {
                    o[c] = p[c] * a + paper[c] * (1.0 - a);
                }
                if fmt.alpha {
                    o[LAB] = 1.0;
                }
            } else if a <= 0.0 {
                o.copy_from_slice(&empty[..n]);
            } else {
                o[..LAB].copy_from_slice(&p[..LAB]);
                if fmt.alpha {
                    o[LAB] = a;
                }
            }
        }
        out.write_region(band, &data);
        y = band.y1;
    }
    out.prune();
    Some(out)
}

/// The display composite (straight RGBA) of a Lab document whose stack has adjustment layers that
/// work on L, a and b, worked out on the stored values; `None` for any other document, which the
/// RGB compositor renders. Rows of `tile` render in parallel.
pub fn render_lab(doc: &Document, rect: Rect, tile: i32) -> Option<Buffer> {
    let stack = LabStack::new(doc).filter(LabStack::adjusts)?;
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
        for (o, p) in px.iter_mut().zip(&acc) {
            if p[LAB] > 0.0 {
                let [r, g, b] = apeireth_image_color::convert::lab_to_srgb([p[0] * 100.0, p[1] * 255.0 - 128.0, p[2] * 255.0 - 128.0]);
                *o = [r, g, b, p[LAB]];
            }
        }
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
