// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Opt-in merged-image recovery must not weaken strict parsing or lose encoded bytes.

use apeireth_image_psd::{Compression, PsdError, PsdFile, Version, compression, testgen};

#[test]
fn recovery_is_explicit_and_preserves_the_original_file() {
    for version in [Version::Psd, Version::Psb] {
        for compression in Compression::ALL {
            let mut original = testgen::small(version, compression);
            original.image_data.data.pop();
            let bytes = original.to_bytes().unwrap();
            let strict_error = PsdFile::from_bytes(&bytes).unwrap_err();
            for want in [false, true] {
                let rejected = PsdFile::from_bytes_and_merged_recovering(&bytes, |_| want, |_| false).unwrap_err();
                assert_eq!(rejected, strict_error);
                let (file, merged) = PsdFile::from_bytes_and_merged_recovering(&bytes, |_| want, |_| true).unwrap();
                assert_eq!(merged.unwrap().unwrap_err(), strict_error);
                assert_eq!(file, original);
                assert_eq!(file.to_bytes().unwrap(), bytes, "{version:?} {compression:?}");
            }
        }
    }
}

#[test]
fn recovery_does_not_accept_missing_compression_or_incomplete_layers() {
    let original = testgen::small(Version::Psd, Compression::Raw);
    let bytes = original.to_bytes().unwrap();
    let merged_offset = bytes.len() - original.image_data.data.len() - 2;
    for cut in [merged_offset - 1, merged_offset, merged_offset + 1] {
        assert!(PsdFile::from_bytes_and_merged_recovering(&bytes[..cut], |_| true, |_| true).is_err(), "cut {cut}");
    }
}

#[test]
fn recovery_never_suppresses_a_merged_stream_limit_error() {
    let mut original = testgen::small(Version::Psd, Compression::Zip);
    original.image_data.data = compression::zip_compress(&vec![0; (1 << 20) + 4096]);
    let bytes = original.to_bytes().unwrap();
    for want in [false, true] {
        assert!(matches!(PsdFile::from_bytes_and_merged_recovering(&bytes, |_| want, |_| true), Err(PsdError::LimitExceeded(_))));
    }
}
