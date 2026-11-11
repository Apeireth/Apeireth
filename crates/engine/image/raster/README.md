# apeireth-image-raster

The pixel plane of the Apeireth image kernel family: sparse, copy-on-write tiled surfaces in
any pixel format. It is where the pixels actually live, and it is designed so that undo
snapshots, background jobs and autosave stay cheap.

A `Surface` is an infinite plane of 256×256 tiles. Missing tiles read as the surface's default
pixel (transparent for layers, white for "reveal all" masks). Tiles are `Arc`-shared, so
cloning a surface is O(tiles) and mutation copies only the touched tiles.

## What it provides

* `Surface` — the tiled plane; read, write, sub-surface and tile-level access.
* `Tile` — one 256×256 tile of pixel storage.
* `Rgba8Image` — a plain interleaved RGBA8 buffer, for handing pixels to other libraries.
* `encode_pixel` / `decode_pixel` / `convert_pixel` — sample-level conversion between formats.
* `to_rgba` / `from_rgba` / `from_rgba_into` — RGBA8 conversion helpers.
* `AllocationError` — checked allocation failures (`SizeOverflow`, `NotEnoughMemory`).
* `Interrupt` / `Cancelled` — cooperative cancellation for long pixel operations.

The crate depends on `apeireth-image-geom` (the tile grid) and `apeireth-image-color` (the
pixel formats).

## Example

```rust
use apeireth_image_color::PixelFormat;
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

let mut surface = Surface::new(PixelFormat::RGBA8);
surface.fill_rect(Rect::new(0, 0, 16, 16), &[0.0, 0.0, 0.0, 1.0]);
```

## Install

```toml
[dependencies]
apeireth-image-raster = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/raster`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
