// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! CIE L*a*b* TIFFs: CIELab (PhotometricInterpretation 8) as Photoshop writes it, with a* and b*
//! signed, and ICCLab (9) with them offset, both read into the Lab layout (a* and b* as
//! `(a + 128) / 255`).

use apeireth_image_codecs::*;

/// One Lab pixel in the codec's normalized form.
fn lab(l: f32, a: f32, b: f32) -> [f32; 3] {
    [l / 100.0, (a + 128.0) / 255.0, (b + 128.0) / 255.0]
}

fn image(sample: SampleType) -> Image {
    let px: Vec<f32> = [lab(50.0, 50.0, -30.0), lab(100.0, 0.0, 0.0)].concat();
    Image::from_normalized(2, 1, ChannelLayout::Lab, sample, &px).unwrap()
}

fn uncompressed(img: &Image) -> Vec<u8> {
    encode(img, Format::Tiff, &EncodeOptions { tiff_compression: TiffCompression::None, ..Default::default() }).unwrap()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[test]
fn eight_bit_cielab_holds_a_and_b_signed() {
    let file = uncompressed(&image(SampleType::U8));
    // L* 50 → 128, a* 50, b* −30 as a signed byte; then white with neutral a* and b*.
    assert!(find(&file, &[128, 50, (-30i8) as u8, 255, 0, 0]).is_some(), "CIELab samples");
    let back = decode(&file).unwrap();
    assert_eq!((back.layout(), back.sample_type()), (ChannelLayout::Lab, SampleType::U8));
    assert_eq!(back.data(), image(SampleType::U8).data());
}

#[test]
fn sixteen_bit_cielab_holds_256_a() {
    let file = uncompressed(&image(SampleType::U16));
    let le = &file[..2] == b"II";
    let u16b = |v: u16| if le { v.to_le_bytes() } else { v.to_be_bytes() };
    // a* 50 → 12800, b* −30 → −7680, white's a* and b* 0.
    let want: Vec<u8> = [32768u16, 12800, (-7680i16) as u16, 65535, 0, 0].iter().flat_map(|v| u16b(*v)).collect();
    assert!(find(&file, &want).is_some(), "CIELab samples");
    let back = decode(&file).unwrap();
    assert_eq!((back.layout(), back.sample_type()), (ChannelLayout::Lab, SampleType::U16));
    let (a, b) = (image(SampleType::U16).to_normalized(), back.to_normalized());
    assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= 1.5 / 65535.0), "{a:?} {b:?}");
}

#[test]
fn icclab_reads_with_offset_a_and_b() {
    // The CIELab file turned into ICCLab: PhotometricInterpretation 9, a* and b* + 128.
    let mut file = uncompressed(&image(SampleType::U8));
    let at = find(&file, &[128, 50, (-30i8) as u8, 255, 0, 0]).unwrap();
    for i in [1, 2, 4, 5] {
        file[at + i] ^= 0x80;
    }
    let le = &file[..2] == b"II";
    let entry: Vec<u8> = if le { [262u16.to_le_bytes(), 3u16.to_le_bytes()].concat() } else { [262u16.to_be_bytes(), 3u16.to_be_bytes()].concat() };
    let e = find(&file, &entry).expect("PhotometricInterpretation entry");
    let nine = if le { 9u16.to_le_bytes() } else { 9u16.to_be_bytes() };
    file[e + 8..e + 10].copy_from_slice(&nine);
    let back = decode(&file).unwrap();
    assert_eq!(back.layout(), ChannelLayout::Lab);
    assert_eq!(back.data(), image(SampleType::U8).data());
}

#[test]
fn lab_converts_to_rgb_for_other_formats() {
    // White and a saturated orange-red; PNG has no Lab, so the encoder plan converts to RGB.
    let img = image(SampleType::U8);
    let png = decode(&encode(&img, Format::Png, &EncodeOptions::default()).unwrap()).unwrap();
    assert!(png.layout().is_rgb());
    let p = png.to_rgba8();
    assert_eq!(&p[4..7], &[255, 255, 255], "white");
    assert!(p[0] > 180 && p[1] < 120 && p[2] > 140, "L 50, a 50, b −30 is a magenta-ish purple: {:?}", &p[..3]);
}
