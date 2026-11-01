# apeireth-image-doc

The document model of the Apeireth image kernel family: pure data, no rendering, no UI. It
describes *what a document is* — its layers, masks, adjustments, fills, effects, artboards and
layer comps — and leaves rendering to `apeireth-image-compose`.

Layer order: inside every group (and the root), `children[0]` is the **bottom** layer, matching
compositing order and the PSD file order. User interfaces display the list reversed.

Non-destructive kinds (adjustment, fill, text, shape, smart object) and colour modes beyond RGB
are part of the model from the start, even where rendering support lands later.

## What it provides

* `Document` — the document root: canvas, layers, groups, artboards, comps, guides, metadata.
* `Layer`, `Group`, `LayerContent`, `LayerMask`, `Locks`, `LabelColor` — the layer tree.
* `Adjustment`, `Fill`, `Pattern`, `Effects`, `AdvancedBlending`, `Knockout` — non-destructive
  content and per-layer blending.
* `TextLayer`, `TextStyles`, `SmartObject`, `SmartSource`, `SmartFilter` — typed layers.
* `LayerComp`, `Artboard`, `Slices`, `Variables`, `Timeline`, `VideoData` — the surrounding
  document furniture (comps, artboards, slices, variable sets, video timelines).
* `Path`, `ShapeLayer`, `VectorMask`, `Gradient`, `GradientStyle` — vector and fill geometry.
* `Color`, `ColorMode`, `PixelFormat`, `SampleType`, `BlendMode`, `Surface`, `Rect`, `Affine`,
  `Size` — re-exported from the lower layers so consumers need one dependency.

## Example

```rust
use apeireth_image_color::{ColorMode, SampleType};
use apeireth_image_doc::Document;
use apeireth_image_geom::Size;

let doc = Document::new("untitled", Size::new(1920, 1080), ColorMode::Rgb, SampleType::U8);
assert!(doc.layers.is_empty());
```

## Install

```toml
[dependencies]
apeireth-image-doc = "2.0.0-rc.4"
```

## License

Dual licensed `MIT OR Apache-2.0`, at your option. The crate header keeps the
`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` notice with the original
license terms unchanged. See `crates/engine/image/NOTICE.md`.

## Links

- Source: `crates/engine/image/doc`
- Family notices: `crates/engine/image/NOTICE.md`, `OSS_NOTICE.md`
