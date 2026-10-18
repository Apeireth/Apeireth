// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Targa writing (#6382): run-length packets end with their scanline, rows are stored bottom-up,
//! and the file reads back exactly.

use apeireth_image_codecs::{ChannelLayout, EncodeOptions, Format, Image, decode_as, encode};

/// The packets of a run-length encoded Targa body, as (pixels, is a run), from just after the
/// 18-byte header (no image ID or colour map) up to `pixels` pixels.
fn packets(file: &[u8], bpp: usize, pixels: usize) -> Vec<(usize, bool)> {
    let mut at = 18;
    let mut seen = 0;
    let mut out = Vec::new();
    while seen < pixels {
        let head = file[at];
        let count = usize::from(head & 0x7f) + 1;
        let run = head & 0x80 != 0;
        at += 1 + if run { bpp } else { count * bpp };
        seen += count;
        out.push((count, run));
    }
    out
}

/// A `w`×`h` image whose rows mix long runs (wider than a packet), short ones and noise, and
/// whose last pixel equals the next row's first, so a packet would run on if it could.
fn sample(w: u32, h: u32, layout: ChannelLayout) -> Image {
    let n = layout.channels();
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let v = match x {
                _ if x < 150 => 40,
                _ if x % 3 == 0 => (x * 7 + y * 13) as u8,
                _ => 200,
            };
            let px = [v, v.wrapping_add(y as u8), v ^ 0x5a, 255 - v];
            data.extend_from_slice(&px[..n]);
        }
    }
    Image::from_u8(w, h, layout, data).unwrap()
}

#[test]
fn rle_packets_end_with_their_scanline() {
    for layout in [ChannelLayout::Gray, ChannelLayout::GrayA, ChannelLayout::Rgb, ChannelLayout::Rgba] {
        let (w, h) = (300u32, 4u32);
        let file = encode(&sample(w, h, layout), Format::Tga, &EncodeOptions::default()).unwrap();
        let bpp = layout.channels();
        let mut col = 0;
        for (count, _) in packets(&file, bpp, (w * h) as usize) {
            assert!(count <= 128, "{layout:?}: a packet holds at most 128 pixels");
            assert!(col + count <= w as usize, "{layout:?}: a packet runs from column {col} past the row's end");
            col = (col + count) % w as usize;
        }
        // A solid image is one run per row (three packets of 128, 128 and 44), not one long run.
        let solid = Image::from_u8(w, h, layout, vec![90; (w * h) as usize * bpp]).unwrap();
        let file = encode(&solid, Format::Tga, &EncodeOptions::default()).unwrap();
        let p = packets(&file, bpp, (w * h) as usize);
        assert_eq!(p.len(), 3 * h as usize, "{layout:?}");
        assert!(p.iter().all(|&(_, run)| run));
    }
}

#[test]
fn the_file_is_bottom_up_rle_with_its_alpha_bits_and_the_tga2_footer() {
    let mut data = vec![0u8; 2 * 2 * 4];
    // Top row red, bottom row blue, opaque.
    for (i, px) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        *px = if i < 2 { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
    }
    let file = encode(&Image::from_u8(2, 2, ChannelLayout::Rgba, data).unwrap(), Format::Tga, &EncodeOptions::default()).unwrap();
    assert_eq!(file[2], 10, "run-length true colour");
    assert_eq!((file[16], file[17]), (32, 8), "32 bits per pixel, 8 alpha bits, bottom-left origin");
    // The first stored row is the bottom one: a run of two blue pixels, in BGRA.
    assert_eq!(&file[18..23], &[0x81, 255, 0, 0, 255]);
    assert!(file.ends_with(b"TRUEVISION-XFILE.\0"));
    let gray = encode(&Image::from_u8(2, 1, ChannelLayout::Gray, vec![1, 2]).unwrap(), Format::Tga, &EncodeOptions::default()).unwrap();
    assert_eq!((gray[2], gray[16], gray[17]), (11, 8, 0), "run-length grayscale, no alpha bits");
}

#[test]
fn every_layout_reads_back_exactly() {
    for layout in [ChannelLayout::Gray, ChannelLayout::GrayA, ChannelLayout::Rgb, ChannelLayout::Rgba] {
        for (w, h) in [(1, 1), (300, 4), (129, 3), (5, 130)] {
            let img = sample(w, h, layout);
            let file = encode(&img, Format::Tga, &EncodeOptions::default()).unwrap();
            let back = decode_as(Format::Tga, &file).unwrap();
            assert_eq!((back.dimensions(), back.layout()), ((w, h), layout), "{layout:?} {w}x{h}");
            assert_eq!(back.data(), img.data(), "{layout:?} {w}x{h}");
        }
    }
}
