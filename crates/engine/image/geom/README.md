# apeireth-image-geom

Geometry primitives shared by every crate of the Apeireth image kernel family: the layer that
everything else measures against. It owns sizes, rectangles, points, affine transforms and the
raster tile grid, plus the non-linear warp descriptions used by the distortion family.

Document space is integer pixels with the origin at the canvas top-left. Layers may extend
beyond the canvas, so negative coordinates are valid.

## What it provides

* `Size` — width / height pair with `area`, `is_empty`.
* `Rect` — integer rectangle, half-open: `[x0, x1) × [y0, y1)`.
* `Point` — integer point.
* `Affine` — 2D affine transform (translate / rotate / scale / shear).
* `TileCoord` and `TILE_SIZE` (256 px) — the tile grid the raster layer allocates on.
* `warp` — `Warp`, `StyleWarp`, `BezierMesh`, `WarpStyle`: the non-linear warp descriptions.

The crate depends on no other crate of the image family, so it sits at the bottom of the
dependency graph and can be published first.

## Example

```rust
use apeireth_image_geom::{Rect, Size};

let size = Size::new(1920, 1080);
let region = Rect::new(0, 0, 640, 480);
assert_eq!(region.width(), 640);
```

## Install

```toml
[dependencies]
apeireth-image-geom = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/geom`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
