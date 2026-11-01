# apeireth-image-color

Pixel formats, colour models and blend-mode math for the Apeireth image kernel family. It is
the layer that decides *what a pixel is*: how many channels it has, how deep each sample is,
which colour model it belongs to and how two layers combine.

Bit depth and colour model are runtime data — nothing in the engine assumes 8-bit or RGB.

## What it provides

* `SampleType` — `U8`, `U16`, `F32` sample storage, with `bytes()` and `bits()`.
* `ColorMode` — Bitmap, Grayscale, Indexed, Rgb, Cmyk, Lab, Multichannel, Duotone.
* `PixelFormat` — colour model + sample type + alpha flag; `RGBA8`, `RGBA16`, `RGBA32F`,
  `GRAY8`, `GRAYA8`, `CMYKA8` and friends, with channel and byte counts.
* `BlendMode` — the blend-mode set used by the document model and the compositor.
* `convert` — colour-space conversion helpers: `cmyk_to_rgb`, `rgb_to_cmyk`, `srgb_to_linear`,
  `linear_to_srgb`, `lab_to_srgb`, `srgb_to_lab`, `rgb_to_gray`, plus a `CmykSpace` context for
  profile-aware CMYK conversion. The `*_naive` variants are the un-profiled conversions.
* `dither_noise` — the per-pixel gradient dither shared by gradient tools and fill layers.

The crate depends only on `apeireth-image-cms` (for colour-managed conversion) and `serde`.

## Example

```rust
use apeireth_image_color::{PixelFormat, SampleType};

let rgba8 = PixelFormat::RGBA8;
assert_eq!(rgba8.channels(), 4);
assert_eq!(rgba8.bytes_per_pixel(), 4);
let _ = SampleType::F32;
```

## Install

```toml
[dependencies]
apeireth-image-color = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/color`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
