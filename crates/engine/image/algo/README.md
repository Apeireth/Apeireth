# apeireth-image-algo

The filter algorithms of the Apeireth image kernel family: the CPU implementation of the
image-processing menu. It holds blur, sharpen, noise, pixelate, stylize and distort, plus the
specialised families the engine ships alongside them (camera raw, content-aware fill, matting,
panorama, liquify, relight, seam removal, and others).

Every filter is

* **depth-agnostic** — it works on normalised `f32` samples read from any `Surface`
  (8/16-bit integer or 32-bit float) in the surface's own colour model;
* **tile/region aware** — `FilterParams::halo` declares how far outside an output tile the
  filter reads, so the result is identical whatever the tiling;
* **selection aware** — the output is mixed with the original by the selection's coverage.

`apply` runs a filter over an area, tile by tile: in parallel with rayon on native targets,
single-threaded on wasm.

## Main entry points

* `FilterParams` — the parameter set of every filter in the crate.
* `apply`, `apply_in`, `apply_in_with`, `apply_tiled`, `apply_tiled_with` — run a filter over
  an area, with or without a halo/extent and with or without a selection.
* `output_area` — the region a filter will actually touch.
* `Halo`, `Ctx` — the halo declaration and the shared filter context.

The crate depends on `apeireth-image-geom`, `apeireth-image-color` and `apeireth-image-raster`.

## Example

```rust
use apeireth_image_algo::{FilterParams, apply};
use apeireth_image_color::PixelFormat;
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

let surface = Surface::new(PixelFormat::RGBA8);
let region = Rect::new(0, 0, 64, 64);
let blurred = apply(&surface, &FilterParams::GaussianBlur { radius: 2.0 }, region, region, None);
```

## Install

```toml
[dependencies]
apeireth-image-algo = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/algo`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
