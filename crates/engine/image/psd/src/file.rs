// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Imported unchanged 2026-10-18 into the Apeireth workspace (no content edits).
// See docs/04-internal/license-port/gate1-license.md.
//! The top-level [`PsdFile`] model, parser and writer.

use crate::compression::{Compression, MAX_DECODED_BYTES, decode_planes, validate_planes};
use crate::error::{PsdError, Result};
use crate::header::{ColorMode, Header, Version};
use crate::image_data::ImageData;
use crate::io::{Reader, WriteExt};
use crate::layer::{LayerInfo, LayerRecord};
use crate::resources::{self, ImageResource, ResourceData, ids};
use crate::tagged::{TaggedBlock, read_blocks_with, uses_long_length, write_blocks};

/// Where the layer info was stored in the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayerInfoPlacement {
    /// In the layer info field of the layer and mask section (8-bit files).
    #[default]
    Section,
    /// In a global `Lr16` / `Lr32` / `Layr` tagged block (16/32-bit files).
    /// The block is re-inserted at `index` among `global_blocks` on write.
    GlobalBlock {
        /// Position among the global blocks.
        index: usize,
        /// Block signature (`8BIM` / `8B64`).
        signature: [u8; 4],
        /// Block key.
        key: [u8; 4],
        /// Padding after the block (see [`TaggedBlock::padding`]).
        padding: Option<Vec<u8>>,
    },
}

/// Global layer mask info, stored raw with typed accessors.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobalLayerMask {
    /// Raw bytes (without the length field).
    pub data: Vec<u8>,
}

impl GlobalLayerMask {
    fn be16(&self, at: usize) -> Option<u16> {
        self.data.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }
    /// Overlay color space.
    pub fn overlay_color_space(&self) -> Option<u16> {
        self.be16(0)
    }
    /// Four color components.
    pub fn color_components(&self) -> Option<[u16; 4]> {
        Some([self.be16(2)?, self.be16(4)?, self.be16(6)?, self.be16(8)?])
    }
    /// Opacity 0..=100.
    pub fn opacity(&self) -> Option<u16> {
        self.be16(10)
    }
    /// Kind: 0 color selected, 1 color protected, 128 per layer.
    pub fn kind(&self) -> Option<u8> {
        self.data.get(12).copied()
    }
}

/// A complete PSD/PSB file.
///
/// `PsdFile::from_bytes(b)?.to_bytes()? == b` for unmodified well-formed
/// files, and `from_bytes(to_bytes(f)) == f` for any model `f`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsdFile {
    /// Header.
    pub header: Header,
    /// Color mode data (palette for indexed, duotone spec, …).
    pub color_mode_data: Vec<u8>,
    /// Image resources in stored order.
    pub resources: Vec<ImageResource>,
    /// Layer info (`None` when the layer info length is 0 and no
    /// `Lr16`/`Lr32` block holds layers).
    pub layer_info: Option<LayerInfo>,
    /// Where [`Self::layer_info`] is stored.
    pub layer_info_placement: LayerInfoPlacement,
    /// Global layer mask (`None` when the field is absent entirely).
    pub global_layer_mask: Option<GlobalLayerMask>,
    /// Global additional layer information blocks.
    pub global_blocks: Vec<TaggedBlock>,
    /// Bytes at the end of the layer and mask section that do not form a block.
    pub layer_mask_trailing: Vec<u8>,
    /// Merged image data.
    pub image_data: ImageData,
}

impl PsdFile {
    /// Parses a PSD or PSB file.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        Self::from_bytes_and_merged(data, |_| false).map(|(file, _)| file)
    }

    /// Parses like [`Self::from_bytes`] and, when `want` asks for it, decodes the merged image
    /// (as [`Self::decode_merged`] would) in the same pass. `want` sees the file with everything
    /// but the merged image parsed (its `image_data` is still empty). A ZIP merged image is then
    /// inflated once for both the parse's check of the stream and the decode, not twice.
    ///
    /// Errors are those of [`Self::from_bytes`]; a merged image that passes the parse's check but
    /// cannot be decoded is the inner `Err`, as [`Self::decode_merged`] would return it.
    pub fn from_bytes_and_merged(data: &[u8], want: impl FnOnce(&PsdFile) -> bool) -> Result<(Self, Option<Result<Vec<u8>>>)> {
        Self::from_bytes_and_merged_recovering(data, want, |_| false)
    }

    /// Parses like [`Self::from_bytes_and_merged`], but lets `recover` accept damage confined to
    /// the merged image after all layer records and channel data have been read successfully.
    /// Callers should opt in only when they can reconstruct the document from its layers.
    ///
    /// The original encoded image bytes are preserved and its parse error is returned as
    /// `Some(Err(..))`, even when `want` did not request a decode. The caller must report that
    /// error rather than silently discarding the merged image (which may hold extra channels).
    /// Header/layer errors, missing compression fields and parse safety-limit errors stay fatal.
    pub fn from_bytes_and_merged_recovering(
        data: &[u8],
        want: impl FnOnce(&PsdFile) -> bool,
        recover: impl FnOnce(&PsdFile) -> bool,
    ) -> Result<(Self, Option<Result<Vec<u8>>>)> {
        let mut r = Reader::new(data);
        let header = Header::read(&mut r)?;
        let psb = header.version.is_psb();
        let cmd_len = r.u32()?;
        let color_mode_data = r.bytes_u64(u64::from(cmd_len))?.to_vec();
        let resources = resources::read_section(&mut r)?;

        let lm_len = r.len_field(psb)?;
        let mut s = r.sub(lm_len)?;
        let mut layer_info = None;
        let mut global_layer_mask = None;
        let mut global_blocks = Vec::new();
        let mut layer_mask_trailing = Vec::new();
        let mut layer_info_placement = LayerInfoPlacement::Section;
        if !s.is_empty() {
            let li_len = s.len_field(psb)?;
            if li_len > 0 {
                layer_info = Some(LayerInfo::read_body(s.bytes_u64(li_len)?, header.version)?);
            }
            if s.remaining() >= 4 {
                let n = s.u32()?;
                global_layer_mask = Some(GlobalLayerMask { data: s.bytes_u64(u64::from(n))?.to_vec() });
                let target_key = match header.depth {
                    16 => Some(b"Lr16"),
                    32 => Some(b"Lr32"),
                    _ => None,
                };
                let mut candidate_seen = match target_key {
                    Some(_) => false,
                    None => layer_info.as_ref().is_some_and(|i| !i.layers.is_empty()),
                };
                let (blocks, trailing) = read_blocks_with(&mut s, header.version, 4, |index, signature, key, data, padding| {
                    let is_candidate = match target_key {
                        Some(k) => &key == k,
                        None => matches!(&key, b"Layr" | b"Lr16" | b"Lr32"),
                    };
                    if candidate_seen || !is_candidate {
                        return false;
                    }
                    candidate_seen = true;
                    let Ok(info) = LayerInfo::read_body(data, header.version) else {
                        if target_key.is_some() {
                            layer_info = None;
                        }
                        return false;
                    };
                    layer_info = Some(info);
                    layer_info_placement = LayerInfoPlacement::GlobalBlock { index, signature, key, padding: padding.clone() };
                    true
                })?;
                global_blocks = blocks;
                layer_mask_trailing = trailing;
            } else {
                layer_mask_trailing = s.peek_rest().to_vec();
            }
        }
        let image_data = ImageData { compression: Compression::Raw, data: Vec::new() };
        let mut file =
            PsdFile { header, color_mode_data, resources, layer_info, layer_info_placement, global_layer_mask, global_blocks, layer_mask_trailing, image_data };
        let mut encoded = r.clone();
        let merged = if want(&file) {
            file.read_merged_decoded(&mut r).map(Some)
        } else {
            ImageData::read(&mut r, &file.header).map(|image| {
                file.image_data = image;
                None
            })
        };
        let merged = match merged {
            Ok(merged) => merged,
            Err(error) => {
                if matches!(error, PsdError::LimitExceeded(_)) || !recover(&file) {
                    return Err(error);
                }
                let compression = Compression::from_u16(encoded.u16()?);
                let source = encoded.peek_rest();
                let mut data = Vec::new();
                data.try_reserve_exact(source.len()).map_err(|_| PsdError::LimitExceeded("not enough memory to retain the damaged merged image"))?;
                data.extend_from_slice(source);
                file.image_data = ImageData { compression, data };
                Some(Err(error))
            }
        };
        Ok((file, merged))
    }

    /// Reads the merged image into `self.image_data` and decodes it. A ZIP stream within the
    /// decode limit is inflated once: by the decode, whose stream check is the one
    /// [`ImageData::read`] makes. Only when the decode fails is that check run on its own, to
    /// tell a file the parse rejects (the outer `Err`) from one it accepts (the inner `Err`).
    fn read_merged_decoded(&mut self, r: &mut Reader<'_>) -> Result<Result<Vec<u8>>> {
        let layout = ImageData::layout(&self.header);
        let compression = Compression::from_u16(r.clone().u16()?);
        let zip = matches!(compression, Compression::Zip | Compression::ZipPrediction);
        if !zip || layout.total_bytes()? > MAX_DECODED_BYTES {
            // Nothing is inflated twice here: the parse's check reads sizes only (or skips an
            // over-limit stream, which the decode then refuses).
            self.image_data = ImageData::read(r, &self.header)?;
            return Ok(self.decode_merged());
        }
        r.u16()?;
        let data = r.peek_rest();
        let decoded = decode_planes(compression, data, &layout);
        if decoded.is_err() {
            validate_planes(compression, data, &layout)?;
        }
        self.image_data = ImageData { compression, data: data.to_vec() };
        r.skip(data.len())?;
        Ok(decoded)
    }

    /// Serializes the file.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let v = self.header.version;
        let psb = v.is_psb();
        // Reserve the pixel data up front: appending it to an empty Vec reallocates and copies
        // repeatedly. Everything else (headers, resources, records) is small by comparison.
        let hint = self.layer_info.as_ref().map_or(0, LayerInfo::channel_data_len).saturating_add(self.image_data.data.len());
        let mut out = Vec::new();
        out.try_reserve(hint).map_err(|_| PsdError::LimitExceeded("not enough memory to serialize the file"))?;
        self.header.write(&mut out);
        out.put_len(self.color_mode_data.len() as u64, false)?;
        out.put(&self.color_mode_data);
        resources::write_section(&mut out, &self.resources)?;

        let in_block = matches!(self.layer_info_placement, LayerInfoPlacement::GlobalBlock { .. });
        // An entirely empty section is written as a zero length. (A section
        // containing only a zero layer-info length cannot be distinguished in
        // the model and is normalized to this form.)
        let section_empty =
            self.layer_info.is_none() && self.global_layer_mask.is_none() && self.global_blocks.is_empty() && self.layer_mask_trailing.is_empty();
        let at = out.begin_len(psb);
        if !section_empty {
            let li = out.begin_len(psb);
            if let (Some(info), false) = (&self.layer_info, in_block) {
                info.write_body(&mut out, v)?;
            }
            out.end_len(li, psb)?;
            let need_mask_field = self.global_layer_mask.is_some() || !self.global_blocks.is_empty() || (in_block && self.layer_info.is_some());
            if need_mask_field {
                let gm = self.global_layer_mask.as_ref().map_or(&[][..], |g| &g.data[..]);
                out.put_len(gm.len() as u64, false)?;
                out.put(gm);
            }
            match (&self.layer_info, &self.layer_info_placement) {
                (Some(info), LayerInfoPlacement::GlobalBlock { index, signature, key, padding }) => {
                    let (before, after) = self.global_blocks.split_at((*index).min(self.global_blocks.len()));
                    write_blocks(&mut out, before, v, 4)?;
                    // Write into the final buffer: a temporary LayerInfo payload and a clone
                    // of all neighboring opaque blocks otherwise double the save's memory.
                    out.put(signature);
                    out.put(key);
                    let long = uses_long_length(v, key);
                    let at = out.begin_len(long);
                    let start = out.len();
                    info.write_body(&mut out, v)?;
                    let len = out.len() - start;
                    out.end_len(at, long)?;
                    match padding {
                        Some(bytes) => out.put(bytes),
                        None => out.extend(std::iter::repeat_n(0u8, (4 - len % 4) % 4)),
                    }
                    write_blocks(&mut out, after, v, 4)?;
                }
                _ => write_blocks(&mut out, &self.global_blocks, v, 4)?,
            }
            out.put(&self.layer_mask_trailing);
        }
        out.end_len(at, psb)?;

        out.put_u16(self.image_data.compression.as_u16());
        out.put(&self.image_data.data);
        Ok(out)
    }

    /// Layer records in file order (bottom-most first); empty if none.
    pub fn layers(&self) -> &[LayerRecord] {
        self.layer_info.as_ref().map_or(&[], |i| &i.layers)
    }

    /// Mutable layer records (creates an empty layer info if absent).
    pub fn layers_mut(&mut self) -> &mut Vec<LayerRecord> {
        &mut self.layer_info.get_or_insert_with(LayerInfo::default).layers
    }

    /// First resource with the given id.
    pub fn resource(&self, id: u16) -> Option<&ImageResource> {
        self.resources.iter().find(|r| r.id == id)
    }

    /// The global `Lr16` / `Lr32` / `Layr` block that should have held the layers but could not
    /// be parsed, with the parse error. [`Self::from_bytes`] keeps such a block verbatim in
    /// [`Self::global_blocks`] and leaves [`Self::layer_info`] `None`, so the file reads as
    /// flattened; this tells a reader that its layers were lost rather than absent.
    pub fn unreadable_layer_block(&self) -> Option<(&TaggedBlock, PsdError)> {
        if self.layer_info.is_some() {
            return None;
        }
        let block = self.global_blocks.iter().find(|b| matches!(&b.key, b"Lr16" | b"Lr32" | b"Layr"))?;
        LayerInfo::read_body(&block.data, self.header.version).err().map(|e| (block, e))
    }

    /// First global block with the given key.
    pub fn global_block(&self, key: &[u8; 4]) -> Option<&TaggedBlock> {
        self.global_blocks.iter().find(|b| &b.key == key)
    }

    /// ICC profile bytes (resource 1039).
    pub fn icc_profile(&self) -> Option<&[u8]> {
        self.resource(ids::ICC_PROFILE).map(|r| &r.data[..])
    }

    /// Resolution info (resource 1005).
    pub fn resolution(&self) -> Option<crate::resources::ResolutionInfo> {
        match self.resource(ids::RESOLUTION_INFO)?.parsed()? {
            Ok(ResourceData::ResolutionInfo(r)) => Some(r),
            _ => None,
        }
    }

    /// The "has real merged data" flag of resource 1057, if present.
    pub fn has_real_merged_data(&self) -> Option<bool> {
        match self.resource(ids::VERSION_INFO)?.parsed()? {
            Ok(ResourceData::VersionInfo(v)) => Some(v.has_real_merged_data),
            _ => None,
        }
    }

    /// Decodes the merged image into planar big-endian samples.
    pub fn decode_merged(&self) -> Result<Vec<u8>> {
        self.image_data.decode(&self.header)
    }

    /// `true` if the merged image carries a transparency channel: its first extra channel, when
    /// the layer count was stored negative or a "Saving Merged Transparency" global block (`Mtrn`,
    /// `Mt16`, `Mt32`) is present. Photoshop marks 32-bit documents with `Mt32` and a positive
    /// layer count (psd-tools transparentbg.psd).
    pub fn merged_has_alpha(&self) -> bool {
        let Some(color) = self.header.color_mode.color_channels() else {
            return false;
        };
        if self.header.channels <= color {
            return false;
        }
        let marked = self.global_blocks.iter().any(|b| matches!(&b.key, b"Mtrn" | b"Mt16" | b"Mt32"));
        match &self.layer_info {
            Some(li) => li.merged_alpha || marked,
            None => true,
        }
    }

    /// Checks structural invariants that the writer relies on.
    pub fn validate(&self) -> Result<()> {
        self.header.validate()?;
        if self.header.version == Version::Psd && (self.header.width > 30_000 || self.header.height > 30_000) {
            return Err(PsdError::LimitExceeded("PSD dimensions exceed 30000; use PSB"));
        }
        if self.header.color_mode == ColorMode::Indexed && self.color_mode_data.len() != 768 {
            return Err(PsdError::invalid("indexed color mode requires a 768-byte palette"));
        }
        Ok(())
    }

    /// Reads a file from disk (native targets only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Result<Self>> {
        Ok(Self::from_bytes(&std::fs::read(path)?))
    }

    /// Writes the file to disk (native targets only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let b = self.to_bytes().map_err(std::io::Error::other)?;
        std::fs::write(path, b)
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use crate::{Compression, testgen};

    #[test]
    fn global_blocks_default_to_four_byte_padding() {
        for version in [Version::Psd, Version::Psb] {
            for len in 1usize..8 {
                let mut file = testgen::small(version, Compression::Raw);
                file.global_blocks = vec![TaggedBlock::new(*b"TEST", vec![7; len]), TaggedBlock::new(*b"NEXT", vec![8; 4])];
                let bytes = file.to_bytes().unwrap();
                let start = bytes.windows(8).position(|b| b == b"8BIMTEST").unwrap() + 12;
                let next = bytes.windows(8).position(|b| b == b"8BIMNEXT").unwrap();
                assert_eq!(next - start, len.next_multiple_of(4));
                let mut expected = vec![7; len];
                expected.resize(len.next_multiple_of(4), 0);
                assert_eq!(&bytes[start..start + expected.len()], expected);
                assert_eq!(PsdFile::from_bytes(&bytes).unwrap(), file);
            }
        }
    }

    #[test]
    fn streamed_layer_info_equals_a_materialized_tagged_block() {
        for version in [Version::Psd, Version::Psb] {
            for key in [*b"Lr16", *b"Lr32", *b"Layr"] {
                for index in [0, 1, 2, usize::MAX] {
                    for padding in [None, Some(vec![]), Some(vec![42]), Some(vec![1, 2, 3])] {
                        let mut file = testgen::small(version, Compression::Raw);
                        file.global_blocks = vec![TaggedBlock::new(*b"zz01", vec![11]), TaggedBlock::new(*b"zz02", vec![12, 13])];
                        file.layer_info_placement = LayerInfoPlacement::GlobalBlock { index, signature: *b"8B64", key, padding: padding.clone() };
                        // An odd body catches padding calculated against the final file offset.
                        file.layer_info.as_mut().unwrap().padding = Some(vec![77]);
                        let actual = file.to_bytes().unwrap();
                        let mut materialized = file.clone();
                        let mut data = Vec::new();
                        materialized.layer_info.take().unwrap().write_body(&mut data, version).unwrap();
                        materialized.global_blocks.insert(index.min(materialized.global_blocks.len()), TaggedBlock { signature: *b"8B64", key, data, padding });
                        materialized.layer_info_placement = LayerInfoPlacement::Section;
                        assert_eq!(actual, materialized.to_bytes().unwrap(), "{version:?} {key:?} {index}");
                    }
                }
            }
        }
    }

    /// #2926: a PSB whose ZIP merged image declares more than `MAX_DECODED_BYTES` opens without
    /// inflating that stream (its layers are still usable); decoding the composite reports the limit.
    #[test]
    fn over_budget_zip_merged_image_opens_without_inflating() {
        let mut bytes = testgen::small(Version::Psb, Compression::Zip).to_bytes().unwrap();
        // Header: height at 14, width at 18 (big-endian u32).
        bytes[14..18].copy_from_slice(&300_000u32.to_be_bytes());
        bytes[18..22].copy_from_slice(&300_000u32.to_be_bytes());
        let file = PsdFile::from_bytes(&bytes).unwrap();
        assert!(!file.layers().is_empty());
        assert!(matches!(file.decode_merged(), Err(PsdError::LimitExceeded(_))));
        let (same, merged) = PsdFile::from_bytes_and_merged(&bytes, |_| true).unwrap();
        assert_eq!(same, file);
        assert!(matches!(merged, Some(Err(PsdError::LimitExceeded(_)))));
    }

    /// Parsing and decoding a ZIP merged image together inflates it once (the parse's stream
    /// check is the decode's), with the same model, pixels and errors as the two steps apart.
    #[test]
    fn merged_zip_image_is_inflated_once_when_decoded_with_the_parse() {
        use crate::compression::INFLATES;
        let inflates = |f: &dyn Fn()| {
            let before = INFLATES.with(std::cell::Cell::get);
            f();
            INFLATES.with(std::cell::Cell::get) - before
        };
        for compression in [Compression::Zip, Compression::ZipPrediction] {
            let bytes = testgen::small(Version::Psd, compression).to_bytes().unwrap();
            let file = PsdFile::from_bytes(&bytes).unwrap();
            let pixels = file.decode_merged().unwrap();
            let apart = inflates(&|| drop(PsdFile::from_bytes(&bytes).unwrap().decode_merged().unwrap()));
            let together = inflates(&|| {
                let (parsed, merged) = PsdFile::from_bytes_and_merged(&bytes, |f| f.image_data.data.is_empty()).unwrap();
                assert_eq!(parsed, file);
                assert_eq!(merged.unwrap().unwrap(), pixels);
            });
            assert_eq!(together + 1, apart, "{compression:?}");
            let (parsed, merged) = PsdFile::from_bytes_and_merged(&bytes, |_| false).unwrap();
            assert_eq!((parsed, merged), (file, None));
            // A broken stream (its checksum) is still the parse's error.
            let mut broken = bytes.clone();
            *broken.last_mut().unwrap() ^= 0xff;
            let alone = PsdFile::from_bytes(&broken).unwrap_err().to_string();
            assert_eq!(PsdFile::from_bytes_and_merged(&broken, |_| true).unwrap_err().to_string(), alone);
        }
    }

    #[test]
    fn global_layer_info_serialization_propagates_layer_limit_errors() {
        let mut file = testgen::small(Version::Psd, Compression::Raw);
        file.layer_info = Some(LayerInfo { layers: vec![LayerRecord::default(); 32768], ..Default::default() });
        file.layer_info_placement = LayerInfoPlacement::GlobalBlock { index: usize::MAX, signature: *b"8BIM", key: *b"Lr16", padding: None };
        assert!(matches!(file.to_bytes(), Err(PsdError::LimitExceeded(_))));
    }

    #[test]
    fn global_layers_preserve_order_padding_and_first_candidate() {
        for version in [Version::Psd, Version::Psb] {
            for depth in [16, 32] {
                let mut file = testgen::layered(version, ColorMode::Rgb, depth, Compression::Raw);
                let key = if depth == 16 { *b"Lr16" } else { *b"Lr32" };
                file.global_blocks = vec![TaggedBlock::new(*b"BEFR", vec![1]), TaggedBlock::new(*b"AFTR", vec![2, 3])];
                file.layer_info_placement = LayerInfoPlacement::GlobalBlock { index: 1, signature: *b"8B64", key, padding: Some(vec![7, 8]) };
                file.layer_mask_trailing = vec![9; 4];
                let bytes = file.to_bytes().unwrap();
                let parsed = PsdFile::from_bytes(&bytes).unwrap();
                assert_eq!(parsed, file);
                assert_eq!(parsed.to_bytes().unwrap(), bytes);
                let mut duplicate = Vec::new();
                file.layer_info.as_ref().unwrap().write_body(&mut duplicate, version).unwrap();
                file.global_blocks.push(TaggedBlock::new(key, duplicate));
                assert_eq!(PsdFile::from_bytes(&file.to_bytes().unwrap()).unwrap(), file);
                file.global_blocks.insert(0, TaggedBlock::new(key, vec![255]));
                let bytes = file.to_bytes().unwrap();
                let parsed = PsdFile::from_bytes(&bytes).unwrap();
                assert!(parsed.layer_info.is_none());
                assert_eq!(parsed.global_blocks.len(), 5);
                assert_eq!(parsed.to_bytes().unwrap(), bytes);
            }
        }
    }

    #[test]
    fn global_layers_prioritized_over_compatibility_section() {
        for version in [Version::Psd, Version::Psb] {
            for depth in [16, 32] {
                let file = testgen::layered(version, ColorMode::Rgb, depth, Compression::Raw);
                let key = if depth == 16 { *b"Lr16" } else { *b"Lr32" };
                let mut lr_data = Vec::new();
                file.layer_info.as_ref().unwrap().write_body(&mut lr_data, version).unwrap();

                // Photoshop writes an empty (or 8-bit compatibility) Layer Info section
                // alongside the global Lr16/Lr32 block (#4928).
                let mut compat_file = file.clone();
                compat_file.layer_info =
                    Some(LayerInfo { layers: vec![LayerRecord { name: b"CompatibilityLayer".to_vec(), ..Default::default() }], ..Default::default() });
                compat_file.layer_info_placement = LayerInfoPlacement::Section;
                compat_file.global_blocks.insert(0, TaggedBlock::new(key, lr_data));

                let bytes = compat_file.to_bytes().unwrap();
                let parsed = PsdFile::from_bytes(&bytes).unwrap();

                assert_eq!(parsed.layers().len(), file.layers().len());
                assert_ne!(parsed.layers()[0].name(), "CompatibilityLayer");
                assert_eq!(parsed.layers()[0].name(), file.layers()[0].name());
                assert!(matches!(parsed.layer_info_placement, LayerInfoPlacement::GlobalBlock { key: k, .. } if k == key));
            }
        }
    }
}
