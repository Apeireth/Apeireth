// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Cold shape rendering on a synthetic 24 MP canvas, excluding fixture construction.
//! Run in release mode; keep concurrent CPU-heavy work stopped while measuring.
use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
use apeireth_image_doc::{LineCap, ShapeLayer, ShapeStroke};
use apeireth_image_geom::{Rect, Size};
use std::time::Instant;

fn main() {
    let clip = Rect::from_size(Size::new(6000, 4000));
    for (name, dashes, cap) in [("solid", vec![], LineCap::Butt), ("dashed", vec![4.0, 2.0], LineCap::Butt), ("dotted", vec![0.0, 2.0], LineCap::Round)] {
        let shape = ShapeLayer {
            path: apeireth_image_vector::shapes::rounded_rect(32.0, 32.0, 5936.0, 3936.0, [0.0; 4]),
            stroke: Some(ShapeStroke { width: 8.0, dashes, cap, ..Default::default() }),
            ..Default::default()
        };
        for _ in 0..3 {
            let start = Instant::now();
            let surface = apeireth_image_vector::render_shape(&shape, PixelFormat::new(ColorMode::Rgb, SampleType::U8, true), clip);
            println!("{name}: {:.3} ms, bounds {:?}", start.elapsed().as_secs_f64() * 1000.0, surface.content_bounds());
        }
    }
}
