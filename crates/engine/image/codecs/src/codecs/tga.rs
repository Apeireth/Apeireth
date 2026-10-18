// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Imported unchanged 2026-10-18 into the Apeireth workspace (no content edits).
// See docs/04-internal/license-port/gate1-license.md.
//! Targa writer. The `image` crate still reads Targa ([`super::via_image`]); writing is ours
//! because its run-length packets ran on from the end of one row into the next. TGA 2.0 says a
//! packet must not cross a scanline, and readers that decode a row at a time (Pillow, texture
//! converters, game engines) misread such files (#6382).
//!
//! Like Photoshop's Targa: RLE (types 10 and 11), rows stored bottom-up (the bottom-left origin
//! every reader assumes, also those that ignore the origin bits), alpha bits in the descriptor,
//! and the TGA 2.0 footer.

use crate::Format;
use crate::error::CodecError;
use crate::image::{ChannelLayout, Image, SampleType};

/// Most pixels one packet holds (a 7-bit count, plus one).
const MAX_PACKET: usize = 128;

/// The TGA 2.0 footer: no extension or developer area, then the signature.
const FOOTER: &[u8; 26] = b"\0\0\0\0\0\0\0\0TRUEVISION-XFILE.\0";

/// Encodes an 8-bit Gray, Gray+alpha, RGB or RGBA image as a run-length encoded Targa.
pub(crate) fn encode(img: &Image) -> Result<Vec<u8>, CodecError> {
    let (w, h) = img.dimensions();
    if img.sample_type() != SampleType::U8 {
        return Err(CodecError::encode(Format::Tga, format!("{:?} samples (Targa stores 8 bits)", img.sample_type())));
    }
    let (bpp, image_type, alpha_bits) = match img.layout() {
        ChannelLayout::Gray => (1, 11, 0),
        ChannelLayout::GrayA => (2, 11, 8),
        ChannelLayout::Rgb => (3, 10, 0),
        ChannelLayout::Rgba => (4, 10, 8),
        l => return Err(CodecError::encode(Format::Tga, format!("no Targa layout for {l:?}"))),
    };
    let (Ok(w16), Ok(h16)) = (u16::try_from(w), u16::try_from(h)) else {
        return Err(CodecError::encode(Format::Tga, format!("dimensions exceed 65535x65535: {w}x{h}")));
    };
    let row = w as usize * bpp;
    let data = img.data();
    if data.len() != row * h as usize {
        return Err(CodecError::InvalidImage("buffer size mismatch".into()));
    }
    let mut out = Vec::with_capacity(18 + data.len() + data.len() / MAX_PACKET + FOOTER.len());
    out.extend_from_slice(&[0, 0, image_type, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&w16.to_le_bytes());
    out.extend_from_slice(&h16.to_le_bytes());
    // Bits per pixel; the descriptor holds the alpha bits, origin bits 0 (bottom-left).
    out.extend_from_slice(&[(bpp * 8) as u8, alpha_bits]);
    let mut px = vec![0u8; row];
    for y in (0..h as usize).rev() {
        px.copy_from_slice(&data[y * row..(y + 1) * row]);
        // Targa stores colour as BGR.
        if bpp >= 3 {
            for p in px.chunks_exact_mut(bpp) {
                p.swap(0, 2);
            }
        }
        rle_row(&px, bpp, &mut out);
    }
    out.extend_from_slice(FOOTER);
    Ok(out)
}

/// One scanline as run-length packets that end with it: a run of two or more equal pixels is a
/// run packet, anything else goes out in raw packets.
fn rle_row(row: &[u8], bpp: usize, out: &mut Vec<u8>) {
    let px: Vec<&[u8]> = row.chunks_exact(bpp).collect();
    let n = px.len();
    let mut i = 0;
    while i < n {
        let mut run = 1;
        while i + run < n && run < MAX_PACKET && px[i + run] == px[i] {
            run += 1;
        }
        if run >= 2 {
            out.push(0x80 | (run - 1) as u8);
            out.extend_from_slice(px[i]);
            i += run;
            continue;
        }
        // Raw until two equal pixels start a run, the packet is full or the row ends. The first
        // pixel always goes in: it differs from the one after it, or it is the row's last.
        let start = i;
        while i < n && i - start < MAX_PACKET && !(i > start && i + 1 < n && px[i] == px[i + 1]) {
            i += 1;
        }
        out.push((i - start - 1) as u8);
        for p in &px[start..i] {
            out.extend_from_slice(p);
        }
    }
}
