# NOTICE — 图像核心 vendored 家族（`crates/engine/image/`）

> `Status: 📋 许可落档（2026-10-18 许可搬运线第一批，四关质检之许可关产物）`
> 溯源、版权主体清单与四关报告：[docs/04-internal/license-port/](../../../docs/04-internal/license-port/)

## 1. 本家族代码的版权与许可

本目录下的 `geom/ cms/ color/ raster/ doc/ vector/ compose/ algo/ codecs/ psd/` 十个 crate
是**逐文件搬运（vendored）**的图像核心代码，中性命名 `apeireth-image-*`。

- **版权主体**：Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
- **许可**：MIT OR Apache-2.0（双许可任选）。完整文本见本目录 [LICENSE-MIT](LICENSE-MIT) 与
  [LICENSE-APACHE](LICENSE-APACHE)（自上游原样落档，0 字节改动）。
- 每个搬运源文件的头部带有**原样版权署名行 + SPDX 标识 + 改动声明**（改动文件标
  `Modified 2026-10-18 …`，未改动文件标 `Imported unchanged …`）；逐文件对账清单：
  `artifacts/vendor-image-core-files.txt`。
- `core/`（`apeireth-image-core`，我们的接口层）为本项目原创，许可随主仓
  Apache-2.0 OR MIT。

## 2. 上游 NOTICE 原文（按其条款原样保留）

以下为上游仓库 `NOTICE` 文件的原文（其第三方资产清单中**只有本节 §3 列出的两项随代码
进入本目录**，字体/图标/词典/品牌等资产均未搬运）：

```text
PhotoCraft
Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
Licensed under the MIT License or the Apache License, Version 2.0, at your option
(LICENSE-MIT, LICENSE-APACHE).

The ArtCraft name, wordmark and logos in docs/brand/ are trademarks of the ArtCraft Team. They are
not open source and are not covered by the license above; their terms are in
docs/brand/LICENSE-brand.txt.

Third-party material

This product bundles third-party material under its own licenses, listed below. The per-file
list (author, source, license) is in ATTRIBUTION.md.

Inter font (assets/fonts/Inter-*.ttf) — SIL Open Font License 1.1. See assets/fonts/OFL-Inter.txt.
JetBrains Mono font (assets/fonts/JetBrainsMono-Regular.ttf) — SIL Open Font License 1.1. See assets/fonts/OFL-JetBrainsMono.txt.
Lucide icons (assets/icons/*.svg, except slice-knife.svg) — ISC; icons derived from Feather also MIT. See assets/icons/LICENSE-lucide.txt.
SCOWL English word list (assets/dict/en_US-scowl-50.txt.gz) — SCOWL license. See assets/dict/LICENSE-SCOWL.txt.
libfacedetection face detection weights and network (crates/algo/data/libfacedetection.bin, crates/algo/src/faces/cnn.rs) — BSD-3-Clause, Copyright (c) 2015-2019, Shiqi Yu, all rights reserved. See crates/algo/data/LICENSE-libfacedetection.txt.

Builds made with craft-fonts (CRAFT_FONTS_DIR; all official releases) also embed the fonts listed in
https://github.com/storytold/craft-fonts/blob/main/ATTRIBUTION.md (today BIZ UDPGothic, Shippori Mincho and
BIZ UDMincho; not the web build), under the SIL Open Font License 1.1. Their licence texts
ship with the package as OFL-<family>.txt.
```

## 3. 随代码进入本目录的第三方材料（版权主体清单·二项 + 生成物一项）

| 路径 | 材料 | 版权主体 | 许可 | 随附文本 / 义务 |
|---|---|---|---|---|
| `codecs/src/codecs/xpm/x11_colors.rs` | X11 `rgb.txt` 颜色名查找表（归一化 X.Org RGB 值） | X.Org Foundation and X11 contributors | MIT (X.Org app/rgb) | 致谢即义务；本表即落档 |
| `algo/data/libfacedetection.bin` + `algo/src/faces/`、`algo/src/faces/cnn.rs` | 人脸检测 CNN 权重与推理代码 | Shiqi Yu | BSD-3-Clause | 许可原文随附：[`algo/data/LICENSE-libfacedetection.txt`](algo/data/LICENSE-libfacedetection.txt)；本表保留版权与免责声明 |
| `cms/profiles/coated-cmyk-synth.icc` | 合成 CMYK ICC 剖面（由 `cms/src/synth.rs` 的剖面生成器产出；上游 ATTRIBUTION 将该生成物 CC0-1.0 置公有领域） | 上游贡献者（生成物 CC0-1.0） | CC0-1.0 | 本批已用中性 producer 字符串重新生成（见四关报告·范围适配） |

## 4. 依赖面新增的许可成分

- `jpeg-encoder`（JPEG 编码）声明 `(MIT OR Apache-2.0) AND IJG`：含 Independent JPEG
  Group 许可成分。IJG 许可为 FSF Free/Libre 的宽松非 copyleft 条款，按其惯例在此
  **致谢 Independent JPEG Group**；`deny.toml` 以最小面例外（仅该 crate）放行 IJG 成分。
- 其余新增依赖（tiff/exr/image-webp/rustfft/zune-*/png/half 等）为 MIT/Apache-2.0/BSD/Zlib
  系宽松许可，`cargo deny check licenses bans sources` 全绿（2026-10-18 实测）。

## 5. 商标声明（不使用）

本项目**不使用**上游及其关联社区的商标、字标与品牌资产；`apeireth-image-*` / `image core`
等命名为本项目中性命名。上游 NOTICE 中的商标条款原文见 §2。
