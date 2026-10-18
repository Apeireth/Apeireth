// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
use apeireth_image_algo::transform::{Homography, Interp, warp_surface, warp_surface_clipped};
use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

#[test]
fn clipped_warp_matches_the_existing_kernel_at_every_depth() {
    for sample in [SampleType::U8, SampleType::U16, SampleType::F32] {
        let mut source = Surface::new(PixelFormat::new(ColorMode::Rgb, sample, true));
        source.fill_rect(Rect::new(8, 12, 40, 44), &[0.8, 0.2, 0.4, 0.7]);
        source.fill_rect(Rect::new(20, 24, 32, 36), &[0.1, 0.7, 0.3, 0.4]);
        let clip = Rect::new(10, 10, 52, 52);
        for interpolation in [Interp::Nearest, Interp::Bilinear, Interp::Bicubic] {
            for h in [Homography([1.2, -0.3, 7.0, 0.3, 1.2, -5.0, 0.0, 0.0, 1.0]), Homography([0.125, 0.0, 24.0, 0.0, 0.125, 30.0, 0.0, 0.0, 1.0])] {
                let full = warp_surface(&source, source.content_bounds(), &h, interpolation).unwrap();
                let bounded = warp_surface_clipped(&source, source.content_bounds(), &h, interpolation, clip).unwrap();
                assert_eq!(full.read_region(clip), bounded.read_region(clip));
                assert!(clip.contains_rect(&bounded.content_bounds()));
                assert!(bounded.tiles().count() <= clip.tiles().count());
            }
        }
    }
}

#[test]
fn huge_scaled_warp_allocates_only_the_destination_clip() {
    let mut source = Surface::new(PixelFormat::RGBA8);
    source.fill_rect(Rect::new(8, 8, 12, 12), &[0.2, 0.7, 0.4, 1.0]);
    let clip = Rect::new(0, 0, 64, 64);
    let h = Homography([10_000.0, 0.0, -99_968.0, 0.0, 10_000.0, -99_968.0, 0.0, 0.0, 1.0]);
    let output = warp_surface_clipped(&source, source.content_bounds(), &h, Interp::Bicubic, clip).unwrap();
    assert!(clip.contains_rect(&output.content_bounds()));
    assert!(output.tiles().count() <= clip.tiles().count());
    assert_eq!(output.pixel(32, 32), source.pixel(10, 10));
    assert!(warp_surface_clipped(&source, source.content_bounds(), &h, Interp::Bicubic, Rect::EMPTY).unwrap().tiles().next().is_none());
}

#[test]
fn inverse_footprint_handles_extreme_finite_affines() {
    let mut source = Surface::new(PixelFormat::RGBA8);
    source.fill_rect(Rect::new(0, 0, 8, 8), &[1.0, 0.0, 0.0, 1.0]);
    // A strongly skewed inverse maps a small visible tile beyond i32 limits.
    let h = Homography([1.0, 1e12, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    let clip = Rect::new(0, 0, 32, 32);
    let output = warp_surface_clipped(&source, source.content_bounds(), &h, Interp::Nearest, clip).unwrap();
    assert!(clip.contains_rect(&output.content_bounds()));
    assert!(output.tiles().count() <= clip.tiles().count());
}

#[test]
fn source_center_does_not_add_extreme_coordinates_in_i32() {
    let mut source = Surface::new(PixelFormat::RGBA8);
    let origin = 1_500_000_000;
    let bounds = Rect::new(origin, origin, origin + 4, origin + 4);
    source.fill_rect(bounds, &[0.2, 0.7, 0.4, 1.0]);
    let h = Homography([1.0, 0.0, -f64::from(origin), 0.0, 1.0, -f64::from(origin), 0.0, 0.0, 1.0]);
    let output = warp_surface_clipped(&source, bounds, &h, Interp::Nearest, Rect::new(0, 0, 8, 8)).unwrap();
    assert_eq!(output.pixel(2, 2), source.pixel(origin + 2, origin + 2));
}
