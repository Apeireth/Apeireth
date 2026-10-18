// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
use apeireth_image_codecs::{ChannelLayout, CodecError, Format, SampleType, decode, decode_as};

const PAM_GRAY: &[u8] = b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 1\nMAXVAL 255\nENDHDR";
const ENDINGS: &[&[u8]] = &[b"\n", b" \n", b"\t \n", b"\r\n", b" \t\r\n"];

#[test]
fn pam_consumes_the_complete_endhdr_line() {
    for ending in ENDINGS {
        let mut bytes = PAM_GRAY.to_vec();
        bytes.extend_from_slice(ending);
        bytes.push(0x80);
        let img = decode(&bytes).unwrap();
        assert_eq!(img.dimensions(), (1, 1));
        assert_eq!(img.layout(), ChannelLayout::Gray);
        assert_eq!(img.data(), &[0x80], "ENDHDR ending {ending:?}");
    }
}

#[test]
fn pam_preserves_every_possible_first_raster_byte() {
    // In particular, LF, CR, space, tab and '#' must not be skipped as header syntax.
    for ending in ENDINGS {
        for first in 0..=255u8 {
            let mut bytes = PAM_GRAY.to_vec();
            bytes.extend_from_slice(ending);
            bytes.push(first);
            assert_eq!(decode(&bytes).unwrap().data(), &[first], "ENDHDR ending {ending:?}, raster byte {first}");
        }
    }
}

#[test]
fn pam_crlf_header_preserves_u16_samples_and_alpha() {
    for ending in ENDINGS {
        let mut bytes = b"P7\r\nWIDTH 1\r\nHEIGHT 1\r\nDEPTH 2\r\nMAXVAL 65535\r\nTUPLTYPE GRAYSCALE_ALPHA\r\nENDHDR".to_vec();
        bytes.extend_from_slice(ending);
        bytes.extend_from_slice(b" \n#\r");
        let img = decode(&bytes).unwrap();
        assert_eq!(img.layout(), ChannelLayout::GrayA);
        assert_eq!(img.sample_type(), SampleType::U16);
        assert_eq!(img.to_u16_samples().unwrap(), vec![0x200a, 0x230d], "ENDHDR ending {ending:?}");
    }
}

#[test]
fn pam_rejects_unterminated_or_invalid_endhdr_lines() {
    for suffix in [b"".as_slice(), b" ", b"\r", b" \t\r", b" \x80", b" \x80\n\x7f", b"#comment\n\x7f"] {
        let mut bytes = PAM_GRAY.to_vec();
        bytes.extend_from_slice(suffix);
        assert!(matches!(decode_as(Format::Pnm, &bytes), Err(CodecError::Malformed { .. })), "ENDHDR suffix {suffix:?}");
    }
}

#[test]
fn pam_truncation_never_uses_header_whitespace_as_pixels() {
    for ending in ENDINGS {
        let mut bytes = PAM_GRAY.to_vec();
        bytes.extend_from_slice(ending);
        bytes.push(0x80);
        for cut in 0..bytes.len() {
            assert!(decode_as(Format::Pnm, &bytes[..cut]).is_err(), "ENDHDR ending {ending:?}, truncated at {cut}");
        }
    }
}

#[test]
fn binary_pnm_keeps_its_single_whitespace_separator() {
    for separator in *b" \t\r\n" {
        for first in *b" \t\r\n#" {
            for (header, channels) in [(b"P5\n1 1\n255".as_slice(), 1), (b"P6\n1 1\n255", 3)] {
                let mut bytes = header.to_vec();
                bytes.push(separator);
                bytes.extend(std::iter::repeat_n(first, channels));
                assert_eq!(decode(&bytes).unwrap().data(), vec![first; channels]);
            }
            let mut bytes = b"P4\n8 1".to_vec();
            bytes.extend_from_slice(&[separator, first]);
            let expected: Vec<u8> = (0..8).map(|bit| if first & (0x80 >> bit) == 0 { 255 } else { 0 }).collect();
            assert_eq!(decode(&bytes).unwrap().data(), expected);
        }
    }
}
