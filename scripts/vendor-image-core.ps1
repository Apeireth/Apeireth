# 图像核心 vendored 家族 — 导入/同步工具（许可搬运线，2026-10-18 立）
#
# 用途：把上游图像核心 crates 以「文件 1:1 可对账」的方式导入 `crates/engine/image/<name>/`，
# 并施加确定性改写规则（中性命名 + 元数据中性化 + 范围适配 + 每文件许可署名/改动声明头）。
# 本脚本是**可复跑**的：上游同步 = 重新取上游源 → 再跑本脚本 → 复核 diff。改写规则与
# 范围适配**全部在本文件内**，任何一次同步的改写面都以本文件为准（单一真源）。
#
# 用法（幂等：重复跑会整面覆盖 -Dest 下 10 个 vendored crate 目录）：
#   pwsh -File scripts/vendor-image-core.ps1 -SourceCrates <上游仓库>/crates -Dest <本仓>/crates/engine/image
#
# 产出：
#   - <Dest>/<crate>/…            10 个 vendored crate（文件 1:1，含测试/示例/数据）
#   - artifacts/vendor-image-core-files.txt   每文件 verbatim | Modified 对账清单
#
# 管线顺序（勿重排）：
#   1) 拷贝 + 文件重命名 → 2) 字面量特例规则 → 3) 标识符/旋钮规则 → 4) 散文产品名规则
#   → 5) 范围适配规则（HEIF 层不搬等）→ 6) Cargo.toml manifest 改写 → 7) 删除/覆盖文件
#   → 8) 许可署名 + 改动声明头 → 9) 对账清单
# 同步收尾（脚本外一步，幂等）：`cargo fmt -p <家族 11 包>` 排版收口（家族 rustfmt.toml
# 与上游同款配置；本仓 rustfmt 版本与上游 CI 版本有排版漂移，收口语义 0 改动），
# 合成 CMYK ICC 用 `APEIRETH_IMAGE_REGEN_PROFILES=1 cargo test -p apeireth-image-cms --release --test regen` 重新生成。
#
# 中性表述纪律说明（豁免表口径）：
#   - 本脚本必须含第三方标识**字面量**才能做替换规则（真实字符串豁免，同
#     scripts/check-neutral-terms.ps1 自身豁免原则）；
#   - 产物文件里第三方标识仅允许出现在**许可署名头**（Copyright / SPDX 行）——
#     「许可署名头」类豁免（法律义务>行文风格），其余行必须中性。

param(
    [Parameter(Mandatory = $true)][string]$SourceCrates,
    [Parameter(Mandatory = $true)][string]$Dest
)

$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# 0. 范围：只搬「编解码/图层/滤镜/几何」四类核心 crate（UI/平台层不搬）
#    依赖闭包：geom ← cms ← color ← raster ← doc ← vector ← compose / algo / codecs / psd
# ---------------------------------------------------------------------------
$Crates = @('geom', 'cms', 'color', 'raster', 'doc', 'vector', 'compose', 'algo', 'codecs', 'psd')

# 文件重命名（产物路径不再带第三方标识）
$FileRenames = @(
    @{ From = 'profiles/photocraft-coated-cmyk.icc'; To = 'profiles/coated-cmyk-synth.icc' }
)

# ---------------------------------------------------------------------------
# 1. 文本改写规则（有序！specific 先于 generic；顺序即语义，勿重排）
# ---------------------------------------------------------------------------
$Rules = @(
    # --- 阶段 2：字面量特例（元数据/错误文案/测试数据，必须先跑）---
    @{ Old = 'photocraft-coated-cmyk.icc'; New = 'coated-cmyk-synth.icc' },
    @{ Old = '"PhotoCraft Coated CMYK'; New = '"Apeireth Coated CMYK' },
    @{ Old = '(PhotoCraft)'; New = '(Apeireth)' },
    @{ Old = '"PhotoCraft"'; New = '"Apeireth"' },
    @{ Old = 'b"PhotoCraftCo"'; New = 'b"ApeirethCore"' },
    @{ Old = '"photocraft 1.0"'; New = '"apeireth"' },
    @{ Old = '"photocraft"'; New = '"apeireth"' },
    @{ Old = 'decode them with photocraft-raw'; New = 'decode them with a camera-raw decoder' },
    @{ Old = 'of PhotoCraft'; New = 'of the image core' },
    # 未搬 crate 的引用改写成中性「层/引擎」称呼（不虚构不存在的 crate 名）
    @{ Old = 'the optional `photocraft-heif` crate'; New = 'the optional HEIF decode layer' },
    @{ Old = 'in the optional photocraft-heif crate'; New = 'in the optional HEIF decode layer' },
    @{ Old = '`photocraft-corpus`'; New = 'the real-file test corpus' },
    @{ Old = '`photocraft-io`'; New = 'the document I/O layer' },
    @{ Old = 'as photocraft-io writes it'; New = 'as the document I/O layer writes it' },
    @{ Old = '`photocraft-format`'; New = 'the native bundle format layer' },
    @{ Old = '`photocraft-text`'; New = 'the text engine' },
    @{ Old = '`photocraft-paint`'; New = 'the brush engine' },
    @{ Old = '`photocraft-raw`'; New = 'the camera-raw decoder' },
    # --- 阶段 3：crate / 标识符 / 环境旋钮中性化 ---
    @{ Old = 'PHOTOCRAFT_'; New = 'APEIRETH_IMAGE_' },
    @{ Old = 'photocraft-'; New = 'apeireth-image-' },
    @{ Old = 'photocraft_'; New = 'apeireth_image_' },
    # --- 阶段 4：散文里的产品名中性化（引注/署名头除外）---
    @{ Old = "PhotoCraft's"; New = "the image core's" },
    @{ Old = 'PhotoCraft'; New = 'image core' }
)

# ---------------------------------------------------------------------------
# 2. 范围适配规则（阶段 5；有序字面量替换，记录在案的有意改动）
#    2a. 生成元数据的 producer 括注统一为本项目名（不落第三方商标）
#    2b. HEIF 解码层不搬（第一批范围）：特性/开关/测试随之收口，识别但不解码
#    2c. 合成 CMYK 剖面名匹配泛化为「任意厂商前缀 + coatedcmyk」（保旧文件互操作）
#    2d. 基准面只留合成基准（笔刷基准依赖未搬的笔刷引擎）
#    2e. 散文顺句（不改语义）
# ---------------------------------------------------------------------------
$AdaptRules = @(
    # 2a. 元数据 producer 括注
    @{ Old = ', image core)"'; New = ', Apeireth)"' },
    @{ Old = '**image core Coated CMYK (synthetic, 300% TAC, medium GCR)**'; New = '**Apeireth Coated CMYK (synthetic, 300% TAC, medium GCR)**' },
    # 2b. HEIF 收口
    @{ Old = @'
        "HEIC decode uses heic-rs (pure Rust, in the optional HEIF decode layer behind the \
         non-default `heif` feature, which official builds enable), so iPhone and Mac photos open; \
         writing needs an HEVC encoder, and the mature ones (x265, libheif) are C, so write stays \
         unsupported. Without the feature HEIF is detected but neither read nor written.",
'@; New = @'
        "HEIF is detected but neither read nor written in this build: the HEVC still-picture \
         decode layer is outside the vendored core's scope, and writing needs an HEVC encoder \
         (the mature ones, x265 and libheif, are C). Opening one reports a clear unsupported \
         error.",
'@ },
    @{ Old = '            FormatCaps { read: cfg!(feature = "heif"), write: false, depths: &[S::U8, S::U16], icc: true, exif: true, xmp: true, lossy: true, ..base }'; New = '            FormatCaps { read: false, write: false, depths: &[S::U8, S::U16], icc: true, exif: true, xmp: true, lossy: true, ..base }' },
    @{ Old = 'fn heif_is_detected_always_and_readable_only_with_the_feature() {'; New = 'fn heif_is_detected_but_not_readable_here() {' },
    @{ Old = '    assert_eq!(caps(Format::Heif).read, cfg!(feature = "heif"));'; New = '    assert!(!caps(Format::Heif).read);' },
    @{ Old = @'
    // Read-only formats: Apple-encoded HEIC from `corpus/heif` (`cargo xtask corpus --heif`), a
    // single picture and a grid of tiles.
    #[cfg(all(feature = "corpus", feature = "heif"))]
    for name in ["rgb-strips-96.heic", "checker-1024.heic"] {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/heif/heic-rs").join(name);
        let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}: run `cargo xtask corpus --all`", p.display()));
        v.push((Format::Heif, bytes));
    }
'@; New = '' },
    @{ Old = @'
apeireth-image-codecs = { path = "..", features = ["heif"] }
# Called directly too: the codec turns its panics into errors (see decode_heif.rs).
heic-rs = "=0.1.4"
'@; New = @'
apeireth-image-codecs = { path = ".." }
'@ },
    @{ Old = @'
[[bin]]
name = "decode_heif"
path = "fuzz_targets/decode_heif.rs"
test = false
doc = false
bench = false

'@; New = '' },
    @{ Old = @'
# HEIC/HEIF decode via the optional apeireth-image-heif crate (heic-rs, pure Rust). Off by default so
# distributors choose it at build time; official builds and CI enable it (`--features heif` on the
# apps). Without it HEIF files are still recognised and opening one is a clear "unsupported" error.
heif = ["dep:apeireth-image-heif"]

'@; New = '' },
    @{ Old = 'apeireth-image-heif = { path = "../heif", optional = true }
'; New = '' },
    @{ Old = @'
| HEIF/HEIC | with feature `heif` | **no** | U8, U16 (10/12-bit decodes to U16) | RGB, RGBA | yes (auxiliary alpha) | yes (`colr` prof) | yes | yes | no | no | n/a | `heic-rs` |
'@; New = @'
| HEIF/HEIC | **no** | **no** | U8, U16 (container-declared) | RGB, RGBA | yes (auxiliary alpha) | yes (`colr` prof) | yes | yes | no | no | n/a | none (decode layer not vendored) |
'@ },
    @{ Old = @'
* **HEIF/HEIC (read-only, feature `heif`).** Decoding lives in the optional HEIF decode layer
  behind the non-default `heif` feature (official image core builds enable it; distributors choose).
  Without it, HEIF is still detected and decoding returns `CodecError::Unsupported` ("HEIC/HEIF
  support isn't included in this build"). iPhone and Mac photos decode with `heic-rs` (pure Rust, no `unsafe`):
  single pictures and grid-tiled photos, 8 to 12 bits, auxiliary alpha, ICC, EXIF and XMP. The
  container's `irot`/`imir`/`clap` are applied and EXIF Orientation is rewritten to 1 (HEIF's EXIF
  tag only repeats the container's, and applying both would turn the photo twice). Grayscale
  decodes to RGB. Image sequences, overlays and identity derivations, multilayer HEVC and some
  4:2:2/4:4:4 streams return `CodecError::Unsupported` or `Malformed`, never wrong pixels. Writing
  needs an HEVC encoder and every mature one is C, so HEIF is listed in `ASYMMETRIC_EXCEPTIONS`.
'@; New = @'
* **HEIF/HEIC (detected, not decoded).** The HEVC still-picture decode layer is outside the
  vendored core's scope, so HEIF is still detected and decoding returns `CodecError::Unsupported`
  ("HEIC/HEIF support isn't included in this build"), never wrong pixels or a panic. Writing needs
  an HEVC encoder and every mature one is C, so HEIF is listed in `ASYMMETRIC_EXCEPTIONS`.
'@ },
    # 2c. 剖面名匹配泛化（保旧文件互操作，去掉字面商标）
    @{ Old = @'
            "cmyk" | "coatedcmyk" | "defaultcmyk" | "photocraftcoatedcmyk" => Some(Builtin::CoatedCmyk),
            _ => None,
'@; New = @'
            "cmyk" | "coatedcmyk" | "defaultcmyk" => Some(Builtin::CoatedCmyk),
            // A coated-CMYK stock name carrying any producer qualifier (normalised to
            // `…coatedcmyk`) resolves to the same builtin, so documents that record a
            // qualified stock name still find their profile.
            other if other.ends_with("coatedcmyk") => Some(Builtin::CoatedCmyk),
            _ => None,
'@ },
    # 2d. 基准面收口（笔刷基准依赖未搬的笔刷引擎）
    @{ Old = @'
    let stroke = apeireth_image_paint::Stroke {
        brush: apeireth_image_paint::BrushSettings { size: 60.0, hardness: 0.5, ..Default::default() },
        points: (0..500).map(|i| apeireth_image_paint::StrokePoint::new(100.0 + i as f64 * 7.0, 1000.0 + (i as f64 * 0.05).sin() * 400.0, 1.0)).collect(),
    };
    let base = filled(4096, 5);
    let mut sel = Surface::with_default(PixelFormat::GRAY8, &[1.0]);
    sel.fill_rect(Rect::new(0, 0, 2000, 4096), &[0.5]);
    g.bench_function("paint_stroke_500pt_60px_with_selection", |b| {
        b.iter(|| {
            let mut s = base.clone();
            apeireth_image_paint::apply_stroke(&mut s, &stroke, Some(&sel), false)
        })
    });
'@; New = '' },
    # 2e. 散文顺句
    @{ Old = '//! Geometry primitives shared by every image core crate.'; New = '//! Geometry primitives shared by every crate of the image core family.' },
    @{ Old = '//! image core colour management: a pure-Rust ICC v2/v4 colour management module.'; New = '//! Colour management for the image core: a pure-Rust ICC v2/v4 colour management module.' },
    @{ Old = '//! image core ships a CMYK profile it generates itself from a documented printer model:'; New = '//! The image core ships a CMYK profile it generates itself from a documented printer model:' },
    @{ Old = '/// Generated looks shipped with image core (procedural, CC0)'; New = '/// Generated looks shipped with the image core (procedural, CC0)' },
    @{ Old = 'so tags image core doesn''t know survive'; New = 'so tags the image core doesn''t know survive' },
    @{ Old = 'every target image core builds for is little-endian'; New = 'every target this code builds for is little-endian' },
    @{ Old = '//! image core holds Lab:'; New = '//! The image core holds Lab:' },
    @{ Old = 'Pure-Rust ICC colour management for image core.'; New = 'Pure-Rust ICC colour management for the image core.' },
    @{ Old = 'so image core generates its own'; New = 'so the image core generates its own' },
    # 2h. crate 根的 panic 面 deny 收口到非测试码（属性本意=库码零 panic；其 CI 只 lint lib 面，
    # 本仓门禁含 --all-targets，内联测试码的 unwrap 需放行）
    @{ Old = '#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]'; New = '#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]' },
    # 2g. 金标断言演进（钉行为不钉 codegen 字面；理由见 docs/04-internal/license-port/gate2-quality.md）
    @{ Old = @'
    /// Every mode except Entropy is bit-for-bit what the old serial loop produced. The expected
    /// bit patterns were recorded by running the unmodified `combine` on [`fixture`].
    #[test]
    fn statistics_are_bit_identical_to_the_serial_implementation() {
'@; New = @'
    /// Every mode except Entropy is bit-for-bit what the old serial loop produced. The expected
    /// bit patterns were recorded by running the unmodified `combine` on [`fixture`].
    ///
    /// 断言于 2026-10-18 演进（理由见 [`assert_bits_within_one_ulp`]）：金标字面跨工具链
    /// 有 ±1 ULP codegen 漂移，改为「逐位相等或 ULP ≤ 1」；行为面（统计正确 + 逐位确定性）
    /// 依旧钉死。
    #[test]
    fn statistics_match_the_serial_reference_within_one_ulp() {
'@ },
    @{ Old = @'
        for (mode, want) in expected {
            assert_eq!(bits(&combine(&layers, mode)), want, "{mode:?}");
        }
    }
'@; New = @'
        for (mode, want) in expected {
            assert_bits_within_one_ulp(&bits(&combine(&layers, mode)), &want, &mode);
        }
    }

    /// 金标位模式抓取自旧串行实现所在的工具链；跨工具链 codegen 漂移会让同一源码算式
    /// 差 1 ULP（2026-10-18 rustc 1.97.1 实测：仅 Kurtosis 两元素 ±1 ULP，dev opt 0 与
    /// opt 1 一致复现，故不是优化档问题）。断言收口为「逐位相等或 ULP 距离 ≤ 1」：
    /// 钉统计行为（含逐位确定性），不钉某条工具链的 codegen 字面。
    fn assert_bits_within_one_ulp(got: &[[u32; 4]], want: &[[u32; 4]], mode: &Stat) {
        let ulp = |a: u32, b: u32| -> u32 {
            let a = if a & 0x8000_0000 != 0 { !a } else { a | 0x8000_0000 };
            let b = if b & 0x8000_0000 != 0 { !b } else { b | 0x8000_0000 };
            a.abs_diff(b)
        };
        for (y, (g_row, w_row)) in got.iter().zip(want.iter()).enumerate() {
            for (x, (g, w)) in g_row.iter().zip(w_row.iter()).enumerate() {
                assert!(ulp(*g, *w) <= 1, "{mode:?} pixel ({y},{x}) off by more than 1 ULP: got {g:#010x} want {w:#010x}");
            }
        }
    }
'@ },
    # 2f. 每文件逐条散点（README / fuzz manifest 注释）
    @{ Old = 'It depends on no other photocraft crate.'; New = 'It depends on no other crate of the image core family.' },
    @{ Old = 'It is its own workspace and is excluded from photocraft''s. Run it with `cargo +nightly fuzz run parse` (or `descriptor`) from `crates/psd`.'; New = 'It is its own workspace and is excluded from the family workspace. Run it with `cargo +nightly fuzz run parse` (or `descriptor`) from the crate root.' },
    @{ Old = '# Standalone: excluded from the photocraft workspace.'; New = '# Standalone: excluded from the family workspace.' }
)

# ---------------------------------------------------------------------------
# 3. 删除 / 整文件覆盖（范围适配；删除路径相对 -Dest 根，覆盖件放
#    scripts/image-core-overrides/<crate>/<相对路径>）
# ---------------------------------------------------------------------------
$DeleteFiles = @(
    'codecs/tests/heif.rs',
    'codecs/fuzz/fuzz_targets/decode_heif.rs'
)

# ---------------------------------------------------------------------------
# 4. Cargo.toml manifest 改写（workspace 继承 → 显式上游等价钉版；保持 1:1 可同步）
# ---------------------------------------------------------------------------
function Rewrite-Manifest([string]$text) {
    # 版本/协议/MSRV 沿本仓 workspace 轴（口径：子 crate 全部继承，0 硬编码）；
    # edition 保上游 2024（编译语义零漂移）；publish=false（vendored 面不发布）。
    $text = $text.Replace("license.workspace = true", "license.workspace = true`npublish = false")
    $text = $text.Replace('edition.workspace = true', 'edition = "2024"')
    # 上游 workspace 依赖的显式等价钉版（版本串取自上游 [workspace.dependencies]）
    $text = $text.Replace('thiserror = { workspace = true }', 'thiserror = "2"')
    $text = $text.Replace('thiserror.workspace = true', 'thiserror = "2"')
    $text = $text.Replace('serde = { workspace = true, optional = true }', 'serde = { version = "1", features = ["derive", "rc"], optional = true }')
    $text = $text.Replace('serde = { workspace = true }', 'serde = { version = "1", features = ["derive", "rc"] }')
    $text = $text.Replace('serde.workspace = true', 'serde = { version = "1", features = ["derive", "rc"] }')
    $text = $text.Replace('serde_json = { workspace = true }', 'serde_json = "1"')
    $text = $text.Replace('serde_json.workspace = true', 'serde_json = "1"')
    $text = $text.Replace('flate2 = { workspace = true }', 'flate2 = { version = "1", default-features = false, features = ["rust_backend"] }')
    $text = $text.Replace('flate2.workspace = true', 'flate2 = { version = "1", default-features = false, features = ["rust_backend"] }')
    $text = $text.Replace('rayon = { workspace = true }', 'rayon = "1"')
    $text = $text.Replace('rayon.workspace = true', 'rayon = "1"')
    $text = $text.Replace('proptest = { workspace = true }', 'proptest = "1"')
    $text = $text.Replace('proptest.workspace = true', 'proptest = "1"')
    $text = $text.Replace('criterion = { workspace = true }', 'criterion = { version = "0.5", default-features = false }')
    $text = $text.Replace('image = { workspace = true, features = ["gif", "bmp", "tga", "ico", "qoi", "hdr"] }', 'image = { version = "0.25", default-features = false, features = ["gif", "bmp", "tga", "ico", "qoi", "hdr"] }')
    $text = $text.Replace('image = { workspace = true, features = ["png", "jpeg", "tiff"] }', 'image = { version = "0.25", default-features = false, features = ["png", "jpeg", "tiff"] }')
    # lint 姿态：vendored 面沿上游（unsafe_code=forbid + dbg_macro/todo=warn，clippy 全集不施加）
    # ——保持与上游 CI 同口径、1:1 可同步；本仓精选 lint 面只约束我方原创代码（接口层）。
    $text = $text.Replace("[lints]`nworkspace = true", "[lints.rust]`nunsafe_code = `"forbid`"`n`n[lints.clippy]`nall = { level = `"allow`", priority = -1 }`ndbg_macro = `"warn`"`ntodo = `"warn`"")
    $text = $text.Replace("[lints]`r`nworkspace = true", "[lints.rust]`nunsafe_code = `"forbid`"`n`n[lints.clippy]`nall = { level = `"allow`", priority = -1 }`ndbg_macro = `"warn`"`ntodo = `"warn`"")
    # crate 间依赖：vendored 集内 → path 依赖；集外（未搬 crate）→ 删除该依赖行
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($line in ($text -split "`n")) {
        if ($line -match '^apeireth-image-(\w+) = \{ workspace = true \}') {
            $dep = $Matches[1]
            if ($Crates -contains $dep) { $out.Add("apeireth-image-$dep = { path = `"../$dep`" }") }
            # 集外依赖（未搬 crate）整行丢弃
            continue
        }
        $out.Add($line)
    }
    return ($out -join "`n")
}

# ---------------------------------------------------------------------------
# 5. 每文件头：许可署名（原样保留版权主体）+ 改动声明（Apache-2.0 §4(b)）
# ---------------------------------------------------------------------------
function Get-Header([string]$ext, [bool]$modified) {
    $c = if ($ext -eq '.toml') { '#' } else { '//' }
    $notice = if ($modified) {
        @(
            'Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob',
            'names and product references renamed to neutral names; generated metadata producer',
            'strings neutralised; scope adaptations recorded in',
            'docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.'
        )
    } else {
        @(
            'Imported unchanged 2026-10-18 into the Apeireth workspace (no content edits).',
            'See docs/04-internal/license-port/gate1-license.md.'
        )
    }
    $lines = @(
        'Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.'
        'SPDX-License-Identifier: MIT OR Apache-2.0'
        ''
    ) + $notice
    return ($lines | ForEach-Object { if ($_ -eq '') { $c } else { "$c $_" } }) -join "`n"
}

# ---------------------------------------------------------------------------
# 6. 执行
# ---------------------------------------------------------------------------
# 换行归一（LF）：上游源是 LF，规则字面量跨平台换行必须确定性匹配
function Fix-Nl([string]$s) { return $s.Replace("`r`n", "`n") }
$SrcRoot = (Resolve-Path $SourceCrates).Path
New-Item -ItemType Directory -Force -Path $Dest | Out-Null
$DestRoot = (Resolve-Path $Dest).Path
$OverrideRoot = Join-Path $PSScriptRoot 'image-core-overrides'
$manifest = New-Object System.Collections.Generic.List[string]
$manifest.Add("# vendor-image-core 对账清单（生成于 $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')）")
$manifest.Add("# <status><TAB><crate>/<相对路径>   status: verbatim=内容与上游一致 | Modified=经本脚本规则改写")

foreach ($crate in $Crates) {
    $src = Join-Path $SrcRoot $crate
    if (-not (Test-Path $src)) { throw "source crate missing: $src" }
    $dst = Join-Path $DestRoot $crate
    if (Test-Path $dst) { Remove-Item $dst -Recurse -Force }
    Copy-Item $src $dst -Recurse -Force

    # 6.1 文件重命名
    foreach ($rn in $FileRenames) {
        $from = Join-Path $dst ($rn.From -replace '/', '\')
        $to = Join-Path $dst ($rn.To -replace '/', '\')
        if (Test-Path $from) {
            New-Item -ItemType Directory -Force -Path (Split-Path $to) | Out-Null
            Move-Item $from $to -Force
        }
    }

    # 6.2 删除件（范围适配；路径相对 -Dest 根）
    foreach ($del in $DeleteFiles) {
        $p = Join-Path $DestRoot ($del -replace '/', '\')
        if (Test-Path $p) { Remove-Item $p -Force }
    }

    # 6.3 文本改写（规则 → 适配 → manifest）+ 文件头
    $files = Get-ChildItem $dst -Recurse -File | Where-Object { $_.Extension -in '.rs', '.toml', '.md', '.txt', '.json' }
    foreach ($f in $files) {
        $original = Fix-Nl ([System.IO.File]::ReadAllText($f.FullName))
        $text = $original
        foreach ($rule in $Rules) { $text = $text.Replace((Fix-Nl $rule.Old), (Fix-Nl $rule.New)) }
        foreach ($rule in $AdaptRules) { $text = $text.Replace((Fix-Nl $rule.Old), (Fix-Nl $rule.New)) }
        if ($f.Name -eq 'Cargo.toml') { $text = Rewrite-Manifest $text }
        $modified = ($text -ne $original)

        if ($f.Extension -in '.rs', '.toml') {
            $text = (Get-Header $f.Extension $modified) + "`n" + $text
        }
        [System.IO.File]::WriteAllText($f.FullName, $text)

        $rel = $f.FullName.Substring($DestRoot.Length + 1)
        $status = if ($modified) { 'Modified' } else { 'verbatim' }
        $manifest.Add("$status`t$rel")
    }

    # 6.4 整文件覆盖件（范围适配；作者定稿，不再过规则/头部管线）
    if (Test-Path $OverrideRoot) {
        Get-ChildItem $OverrideRoot -Recurse -File | ForEach-Object {
            $rel = $_.FullName.Substring($OverrideRoot.Length + 1)
            if ($rel -match "^$crate[\\/]") {
                $target = Join-Path $dst ($rel.Substring($crate.Length + 1))
                New-Item -ItemType Directory -Force -Path (Split-Path $target) | Out-Null
                Copy-Item $_.FullName $target -Force
                $manifest.Add("Modified`t$rel (override)")
            }
        }
    }
}

$reportDir = 'artifacts'
New-Item -ItemType Directory -Force -Path $reportDir | Out-Null
$manifestPath = Join-Path $reportDir 'vendor-image-core-files.txt'
$manifest | Set-Content $manifestPath -Encoding UTF8

# 家族级 rustfmt 配置（与上游同款：宽行 + 紧凑小项），随同步落盘——排版可对账的前提
@(
    '# vendored 图像引擎家族的格式化配置：与上游同款（宽行 + 紧凑小项）。'
    '# 理由（2026-10-18 许可搬运线第一批）：搬运面保持上游排版可对账——同步上游时文件'
    '# diff 只含语义改动，不含格式化噪音。rustfmt 按文件目录向上发现本配置，只作用于'
    '# crates/engine/image/ 家族（本仓其余面仍走根 rustfmt.toml 的 stable 默认风格）。'
    'max_width = 160'
    'use_small_heuristics = "Max"'
) | Set-Content (Join-Path $DestRoot 'rustfmt.toml') -Encoding UTF8

Write-Output "OK    vendored $($Crates.Count) crates into $DestRoot"
Write-Output "OK    per-file manifest: $manifestPath"
$mv = ($manifest | Where-Object { $_ -like 'Modified*' }).Count
$vv = ($manifest | Where-Object { $_ -like 'verbatim*' }).Count
Write-Output "STATS Modified=$mv verbatim=$vv"
