# apeireth-image-vector

Rasterization of the vector data held in `apeireth-image-doc`: paths (with the Photoshop path
operations), shape layers (fill + stroke), vector masks, and the inverse direction — tracing a
coverage mask back into a path for "Make Work Path".

Output is either raw coverage (`f32` per pixel) or sparse tiled `Surface`s in any
`PixelFormat`, where only the tiles that differ from the default are allocated.

## What it provides

* `edit` — point-level path editing (direct selection, convert point) and hit testing.
* `flatten` — cubic Bézier flattening with a distance tolerance; `flatten_path`, `flatten_subpath`.
* `map` — non-affine maps of paths (distort, perspective, warp).
* `raster` — exact area-coverage scanline rasterizer (non-zero / even-odd per component,
  boolean combination of components, inversion); `Rasterizer`.
* `stroke` — polygon stroking with miter / round / bevel joins, butt / round / square caps and
  dashes; `StrokeStyle`, `stroke_polygons`.
* `shapes` — live shapes: rectangles with corner radii, ellipses, polygons, stars, lines.
* `trace` — contour tracing and curve fitting (selection → path).

Pure Rust, no platform dependencies; builds for `wasm32` (single-threaded there, rayon on
native targets).

## Example

```rust
use apeireth_image_doc::{FillRule, PathOp};
use apeireth_image_geom::Rect;
use apeireth_image_vector::Rasterizer;

let mut rasterizer = Rasterizer::new(false);
rasterizer.add_component(&polygons, PathOp::Combine, FillRule::NonZero);
let coverage = rasterizer.render(Rect::new(0, 0, 64, 64));
```

## Install

```toml
[dependencies]
apeireth-image-vector = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/vector`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
