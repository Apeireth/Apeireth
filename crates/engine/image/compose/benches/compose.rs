// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! `cargo bench -p apeireth-image-compose`
use apeireth_image_color::{BlendMode, Color, ColorMode, PixelFormat, SampleType};
use apeireth_image_doc::{Adjustment, Document, Layer, LayerContent, LayerMask, Size};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;
use criterion::{Criterion, criterion_group, criterion_main};

fn filled(n: u32, seed: u8) -> Surface {
    let r = Rect::new(0, 0, n as i32, n as i32);
    let mut s = Surface::new(PixelFormat::RGBA8);
    let row: Vec<u8> = (0..n).flat_map(|x| [(x as u8).wrapping_mul(seed), (x / 3) as u8, seed, 200]).collect();
    let bytes: Vec<u8> = (0..n).flat_map(|_| row.iter().copied()).collect();
    s.write_interleaved(r, &bytes);
    s
}

/// 6016² document, 5 layers: background, masked, multiply, curves, screen (opacity/fill).
fn five_layer_doc(n: u32) -> Document {
    let mut d = Document::with_background("bench", Size::new(n, n), ColorMode::Rgb, SampleType::U8, Color::WHITE);
    let mut a = Layer::new("masked", LayerContent::Raster(filled(n, 3)));
    let mut m = LayerMask::reveal_all();
    m.surface.fill_rect(Rect::new(0, 0, n as i32 / 2, n as i32), &[0.3]);
    a.mask = Some(m);
    d.layers.push(a);
    let mut b = Layer::new("multiply", LayerContent::Raster(filled(n, 7)));
    b.blend = BlendMode::Multiply;
    d.layers.push(b);
    d.layers.push(Layer::new("curves", LayerContent::Adjustment(Adjustment::identity_curves())));
    let mut c = Layer::new("screen", LayerContent::Raster(filled(n, 11)));
    c.blend = BlendMode::Screen;
    c.opacity = 0.7;
    c.fill_opacity = 0.8;
    d.layers.push(c);
    d
}

/// 2048² document with 20 masked layers.
fn mask_heavy_doc() -> Document {
    let n = 2048;
    let mut d = Document::with_background("masks", Size::new(n, n), ColorMode::Rgb, SampleType::U8, Color::WHITE);
    for i in 0..20u8 {
        let mut l = Layer::new(format!("l{i}"), LayerContent::Raster(filled(n, i + 2)));
        let mut m = LayerMask::hide_all();
        m.surface.fill_rect(Rect::new(i as i32 * 50, 0, i as i32 * 50 + 1200, n as i32), &[0.8]);
        l.mask = Some(m);
        l.opacity = 0.6;
        d.layers.push(l);
    }
    d
}

fn benches(c: &mut Criterion) {
    let doc = five_layer_doc(6016);
    let mut g = c.benchmark_group("compose");
    g.sample_size(10);
    g.bench_function("flatten_6016_5_layers", |b| b.iter(|| apeireth_image_compose::flatten(&doc)));
    let masks = mask_heavy_doc();
    g.bench_function("flatten_2048_20_masked_layers", |b| b.iter(|| apeireth_image_compose::flatten(&masks)));

    g.finish();
}

criterion_group!(compose_benches, benches);
criterion_main!(compose_benches);
