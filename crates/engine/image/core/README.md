# apeireth-image-core

The neutral, stable facade over the Apeireth image kernel family. It is the crate that tool
surfaces and user interfaces are meant to depend on: a small, hand-picked API in front of the
ten engine crates, plus an `engine` module that re-exports those crates verbatim for consumers
who need a capability the facade does not wrap yet.

This is a **thin facade**. It does not re-implement engine logic: it bridges interleaved bytes
to the engine's tiled surfaces and delegates everything else.

## What it wraps

| Area | Type / function | Underlying crate |
|---|---|---|
| Flat raster decode / encode | `RasterImage`, `decode`, `encode`, `OutputFormat` | `apeireth-image-codecs` |
| Geometry | `crop`, `resize` | `apeireth-image-geom`, `apeireth-image-raster` |
| Filters | `blur`, `sharpen` | `apeireth-image-algo` |
| Layers | `LayerDocument`, `flatten` | `apeireth-image-doc`, `apeireth-image-compose` |
| Errors | `ImageError` | facade-local |

Every other capability is reachable through `apeireth_image_core::engine` (`engine::algo`,
`engine::codecs`, `engine::psd`, `engine::cms`, `engine::compose`, …). Consuming that module
directly means taking on the engine API's evolution yourself.

## Example

```rust
use apeireth_image_core::{OutputFormat, decode, encode};

let image = decode(&bytes)?;
let out = encode(&image, OutputFormat::Png)?;
```

## Install

```toml
[dependencies]
apeireth-image-core = "2.0.0-rc.4"
```

## License

Dual licensed `Apache-2.0 OR MIT`, at your option. The crate header keeps the
`Copyright (c) 2026 The Apeireth Authors` notice.

The engine crates behind the facade are the vendored image family, dual licensed
`MIT OR Apache-2.0`; each keeps its own `ArtCraft Team and the PhotoCraft contributors`
copyright header with the original license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/core`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
