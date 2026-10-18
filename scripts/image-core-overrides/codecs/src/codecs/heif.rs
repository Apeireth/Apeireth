// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
// This file additionally had the optional decode-layer wiring removed (that layer is
// outside the vendored core's scope); HEIF stays detected-but-not-decoded here.
//! HEIF / HEIC (the iPhone and Mac photo format), read-only: see [`crate::ASYMMETRIC_EXCEPTIONS`].
//!
//! Decoding needs an HEVC still-picture decode layer which is not part of the vendored
//! core, so HEIF files are still detected and opening one is an [`CodecError::Unsupported`]
//! error, never a panic.
//!
//! HEIF records orientation in the container (`irot`/`imir`, plus a `clap` crop), not in
//! EXIF; a decode layer is expected to apply those and rewrite the EXIF Orientation to 1.

use crate::Format;
use crate::error::CodecError;
use crate::image::Image;
use crate::options::Limits;

const F: Format = Format::Heif;

/// The reason given when this build has no HEIF decoder.
pub(crate) const NOT_IN_BUILD: &str = "HEIC/HEIF support isn't included in this build of the image core";

pub(crate) fn decode(_bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    Err(CodecError::unsupported(F, NOT_IN_BUILD))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heif_is_a_clear_unsupported_error() {
        let header = b"\0\0\0\x18ftypheic\0\0\0\0mif1heic";
        assert_eq!(crate::detect(header), Some(F), "still recognised");
        let r = crate::decode(header);
        assert!(matches!(&r, Err(CodecError::Unsupported { format: Format::Heif, reason }) if reason.contains("isn't included in this build")), "{r:?}");
        assert!(!crate::caps(F).read);
    }
}
