# apeireth-image-compose

The CPU compositor of the Apeireth image kernel family. It flattens a document layer tree into
straight-alpha RGBA (`f32`) for any rectangle, and it is the single place where the layer
semantics live.

It encodes:

* blend modes, opacity × fill opacity, visibility;
* layer masks with density;
* clipping groups (clipped layers composite *atop* their base);
* pass-through versus isolated groups;
* adjustment layers (applied to the composite below);
* fill layers, Dissolve.

This compositor is the reference the GPU backend has to match within 1/255. Compositing
happens in display RGB: CMYK layers are read through the document's CMYK profile (the built-in
coated CMYK when untagged) and composited in sRGB; mode-native (CMYK / Lab) compositing is not
implemented here.

## Main entry points

* `render` — composite the whole document over a rectangle, in parallel 256² tiles on native
  targets (single-threaded on wasm). `render_tiled` takes an explicit tile size.
* `render_bands` — the same composite emitted in row bands.
* `flatten` — the composite without transparency. `flatten_to_surface` /
  `composite_to_surface` produce a `Surface` instead of a `Buffer`.
* `render_layer`, `render_reduced`, `render_reduced_rect`, `render_reduced_damage`,
  `thumbnail` — single-layer, downscaled and thumbnail renders.
* `Buffer` — the straight-alpha RGBA result (`rect` + `px`), with `to_rgba8`.

The crate depends on `apeireth-image-geom`, `apeireth-image-color`, `apeireth-image-raster`,
`apeireth-image-doc` and `apeireth-image-vector`.

## Example

```rust
use apeireth_image_compose::render;
use apeireth_image_geom::Rect;

let buffer = render(&doc, Rect::new(0, 0, 1920, 1080));
let rgba8 = buffer.to_rgba8();
```

## Install

```toml
[dependencies]
apeireth-image-compose = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/compose`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
