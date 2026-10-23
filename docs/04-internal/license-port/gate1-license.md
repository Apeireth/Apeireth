# 四关质检·第一关：许可关（License Gate）

```yaml
[Document-Meta]
Document:        gate1-license.md
Version:         1.0（许可搬运线第一批：图像核心 vendored 家族）
Last-Modified:   2026-10-18
Status:          🟢 活跃（四关质检报告，逐关一报）
Author:          许可搬运线（子代理施工，主代理验收）
```

> 结论先行：**许可关 PASS**。搬运对象为 MIT OR Apache-2.0 双许可的宽松协议项目；
> 版权头原样保留 + 逐文件改动声明 + NOTICE 落档 + 商标零使用 + 中性表述豁免表
> 新增「许可署名头」类，全部执行完毕且可复核（命令与实测数见各节）。

## 1. 搬运对象与取材溯源（可复核）

| 项 | 值 |
|---|---|
| 上游 | `storytold/photocraft`（github.com/storytold/photocraft） |
| 取材版本 | `main @ 8407d9ccb125af52944fca9d228383459f94aaef`（`git ls-remote` 2026-10-18 实测）；上游 workspace `version = "0.10.0"` |
| 取材物 | `https://codeload.github.com/storytold/photocraft/tar.gz/refs/heads/main`，19,091,279 字节 |
| 取材物校验 | SHA-256 `6EB9A622974398DE8994ADDEA1E2F68763736A832EA5687FD60DD544998EA948` |
| 取材日期/通道 | 2026-10-18。`git clone` 官方通道两次失败（schannel 中断、OpenSSL unexpected eof）后按 `docs/04-internal/github-fetch-guide.md` 换道，codeload tarball 直连成功；官方 `git ls-remote` 取 commit 校验 |
| 取材登记 | 本表 + `crates/engine/image/NOTICE.md` + `OSS_NOTICE.md` §12 |

## 2. 许可核实

- **许可类型**：MIT OR Apache-2.0（双许可任选），上游 `[workspace.package] license = "MIT OR Apache-2.0"`；
  与本仓 `Apache-2.0 OR MIT` 同集合（SPDX 顺序不影响语义）。
- **许可原文**：自上游 `LICENSE-MIT` / `LICENSE-APACHE` **原样落档**至
  [`crates/engine/image/LICENSE-MIT`](../../../crates/engine/image/LICENSE-MIT)、
  [`LICENSE-APACHE`](../../../crates/engine/image/LICENSE-APACHE)（0 字节改动，本批实测 diff 为空）。
- **NOTICE**：上游 `NOTICE` 原文按其条款保留在 [`crates/engine/image/NOTICE.md`](../../../crates/engine/image/NOTICE.md) §2；
  其第三方资产清单逐项判定：**仅两项随代码进入**（X11 颜色表、libfacedetection 权重/推理代码）
  + 一项生成物（合成 CMYK ICC，上游 CC0-1.0 置公有领域），其余（字体/图标/词典/品牌资产）**不搬**。
- **干净房间声明**：上游自述为其格式的 clean-room 实现（依公开规范），其代码为原创并以
  MIT OR Apache-2.0 发放；本批搬运的十 crate 均属该许可覆盖的代码面。
- **商标**：上游声明其社区名称/字标/标识为商标、**不在开源许可覆盖内**。本项目**零使用**：
  模块/crate/接口命名全中性（`apeireth-image-*`、`image core`），生成元数据 producer 用
  `Apeireth`/`apeireth`（§6）。

## 3. 版权主体清单（License Holder Inventory）

| # | 组件 / 路径 | 版权主体 | 许可 | 义务与落点 |
|---|---|---|---|---|
| 1 | `crates/engine/image/{geom,cms,color,raster,doc,vector,compose,algo,codecs,psd}/`（353 个 .rs 等 363 文件） | Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors | MIT OR Apache-2.0 | 每文件署名头（原样）+ 家族 LICENSE-MIT/LICENSE-APACHE + NOTICE.md |
| 2 | `codecs/src/codecs/xpm/x11_colors.rs` | X.Org Foundation and X11 contributors | MIT (X.Org app/rgb) | NOTICE.md §3 致谢行 |
| 3 | `algo/data/libfacedetection.bin`、`algo/src/faces/`、`faces/cnn.rs` | Copyright (c) 2015-2019, Shiqi Yu | BSD-3-Clause | 许可原文随附 `algo/data/LICENSE-libfacedetection.txt` + NOTICE.md §3 免责声明保留 |
| 4 | `cms/profiles/coated-cmyk-synth.icc`（原名 `photocraft-coated-cmyk.icc`，本批重新生成） | 上游贡献者（生成物） | CC0-1.0（上游 ATTRIBUTION 置公有领域） | NOTICE.md §3 登记 |
| 5 | `jpeg-encoder`（新依赖，JPEG 编码） | （依赖版权见其 crate） | (MIT OR Apache-2.0) AND IJG | IJG 成分按条款致谢 Independent JPEG Group（NOTICE.md §4）；`deny.toml` 最小面例外 |
| 6 | `crates/engine/image/core/`（接口层） | The Apeireth Authors | Apache-2.0 OR MIT（随主仓） | 本项目原创，无第三方版权 |

## 4. 落档决定（读码定）

- **落 `OSS_NOTICE.md` 追加节（§12）**：读码核实 `OSS_NOTICE.md` = 手工维护的「第三方来源
  整合致谢 + 决策链」，是 vendored 源码 attribution 的既有归口；`THIRD-PARTY-NOTICES.md`
  为 `cargo-about` 生成的 561-crate 依赖 attribution（工具生成面，手工条目会被再生成覆盖），
  故**不动**它、只维护 OSS_NOTICE.md + 家族级 NOTICE.md（双落档：仓库级 + crate 级）。
- 同步落点：`crates/engine/image/NOTICE.md`（家族级，含上游 NOTICE 原文与版权主体清单）。

## 5. 版权头与改动声明（Apache-2.0 §4(a)(b)）

- **版权头原样保留**：上游源文件无逐文件版权头（仓库级 LICENSE/NOTICE 承载）；本批为每个
  搬运 .rs/.toml 文件**建立署名头**，版权主体行按上游原文措辞保留：
  `Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.` + `SPDX-License-Identifier: MIT OR Apache-2.0`。
- **改动声明**：署名头下按文件状态标注（Apache-2.0 §4(b)「modified files carry prominent notices」）：
  - `Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob names and
    product references renamed to neutral names; generated metadata producer strings neutralised;
    scope adaptations recorded in gate1-license.md. Original license terms unchanged.`
  - 未改动文件标 `Imported unchanged 2026-10-18 …`。
- **逐文件对账**：`artifacts/vendor-image-core-files.txt`（生成式清单，工具产出）。
  实测（2026-10-18）：**Modified 252 + verbatim 111**（+1 整文件覆盖件）。
- **可复跑**：`pwsh -File scripts/vendor-image-core.ps1 -SourceCrates <上游>/crates -Dest crates/engine/image`
  一条命令重放全部改写（规则与范围适配全在脚本内，单一真源）。

## 6. 商标纪律（不用其商标名）

- 我们的模块/crate/命名：`apeireth-image-{geom,cms,color,raster,doc,vector,compose,algo,codecs,psd}`、
  接口层 `apeireth-image-core`、文档散文 `image core`、环境旋钮 `APEIRETH_IMAGE_*`。
- 生成元数据（写入产出文件的 producer/描述串）用 `Apeireth` / `apeireth`：
  ICC 剖面描述括注、DPX creator、FITS Software 标签等（范围适配清单 §8 第 1 组）。
- 上游商标声明原文保留在家族 NOTICE.md §2（法律义务 > 行文风格）。

## 7. 中性表述扫描器豁免表：新增「许可署名头」类

- 落点：`scripts/check-neutral-terms.ps1` 新增 `$licenseRe` 行级豁免类，**只豁免版权/许可/
  商标声明行**（Copyright / SPDX / 许可文本 / 商标声明 / 版权主体行），理由=**法律义务 >
  行文风格**（宪章〇之二原话）；叙述/命名/注释行一律不豁免。
- 禁止面补强：`photocraft | storytold | ArtCraft`（搬运源身份入禁词面，否则「零第三方
  产品名」不可验证）；文件级事实引注豁免扩到 `scripts/vendor-image-core.ps1`（替换规则必须
  含源标识字面量）与四关报告（溯源清单必须点名真实上游——研究引注特权，同既有
  `tui-power-design-research.md` 先例）。
- 实测（2026-10-18）：许可署名头类豁免 **716 行**（全部为搬运文件版权头），本批产物
  **0 命中**；共享扫描面曾有 20 命中在他线在途调研文档（已报备标红，随后归口线自注册
  事实引注豁免收口，全仓复测 `scripts/check-neutral-terms.ps1` = **OK 0 命中**）。

## 8. 范围适配清单（有意改动逐条，全部带 Modified 头 + 脚本内可复核）

| # | 适配 | 理由 | 影响面 |
|---|---|---|---|
| 1 | 生成元数据 producer/描述中性化（`(PhotoCraft)`→`(Apeireth)`、`"PhotoCraft Coated CMYK`→`"Apeireth Coated CMYK`、DPX creator、FITS Software、测试夹具字节串） | 不把第三方商标写进我方产出文件（商标纪律） | `cms/src/{builtin,synth}.rs`、`codecs/src/{dpx,fits}.rs`、测试 |
| 2 | HEIF 解码层不搬（第一批范围外）：`heif` feature/可选依赖/解码代码移除，`Format::Heif` 保持「识别但不解码」并给清晰 unsupported 错误 | 只搬核心四类；HEVC 解码层引入新依赖面超范围 | `codecs/src/codecs/heif.rs`（整文件覆盖件）、`format.rs`、`detect_caps.rs` 测试、`tests/heif.rs` 删除、`tests/malformed.rs` 夹具块移除、fuzz 目标 `decode_heif` 删除 |
| 3 | 剖面名匹配泛化：`…coatedcmyk` 任意厂商前缀 → 合成 CMYK 剖面（原字面商标别名删除） | 去字面商标**且保旧文件互操作**（严格更兼容） | `cms/src/builtin.rs`（+1 语义泛化测试口径） |
| 4 | 基准面收口：`compose/benches` 的笔刷笔触基准删除（依赖未搬的笔刷引擎），保留合成基准 | 范围内基准保留，范围外依赖不引 | `compose/benches/compose.rs`、compose dev-deps |
| 5 | crate/标识符/旋钮中性化（`photocraft-*`→`apeireth-image-*`、`PHOTOCRAFT_*`→`APEIRETH_IMAGE_*`）+ 未搬 crate 引用改中性「层/引擎」称呼（不虚构不存在的 crate 名）+ 散文产品名中性化 | 中性表述纪律 | 全部搬运文件（详见脚本规则表） |
| 6 | 合成 CMYK ICC 以中性 producer 重新生成（`APEIRETH_IMAGE_REGEN_PROFILES=1` 走其自带再生测试） | 元数据中性化后与夹具字节一致性的收口 | `cms/profiles/coated-cmyk-synth.icc` |
| 7 | crate 根 panic 面 `deny(clippy::unwrap_used…)` 收口为 `cfg_attr(not(test), …)` | 属性本意=库码零 panic（保持）；其 CI 只 lint lib 面，本仓门禁含 `--all-targets`（内联测试码的 unwrap 需放行） | 10 个 `lib.rs` 各 1 行 |
| 8 | 金标断言演进（逐位相等 → 逐位或 ULP ≤ 1）+ 改名 | 钉行为不钉 codegen 字面（理由与隔离复跑证据见 gate2 §5） | `algo/src/stack.rs` |
| 9 | lint 姿态沿上游（unsafe_code=forbid + dbg_macro/todo=warn，clippy 全集不施加于 vendored 面） | 1:1 可同步（同步 diff 只含语义改动）+ 与上游 CI 同口径 | 10 份 `Cargo.toml`（理由详 gate2 §6） |

## 9. 不搬面声明（0 假装）

- **UI/平台层不搬**：`ui-egui / engine / io / text / paint / gpu / tablet / ipp / automation /
  plugins / format / affinity / raw / heif / testkit` 及 `apps/*`（第一批范围=编解码/图层/
  滤镜/几何四类核心 + 依赖闭包）。
- **资产不搬**：字体（Inter/JetBrains Mono/craft-fonts）、图标（Lucide/Feather）、词典（SCOWL）、
  品牌/图标资产、Solarized 等第三方素材 0 件进入（上游 NOTICE 清单逐项判定见 §2/§3）。
- **corpus 语料不搬**：真文件回归语料为 opt-in（`corpus` feature + 外部语料仓），本批不取；
  相关测试默认不编译，能力如实标注「未实测（语料未取）」。

## 10. Apache-2.0 §4 条款逐条对照

| 条款 | 执行 | 证据 |
|---|---|---|
| §4(a) 保留版权/许可/NOTICE 声明 | 每文件署名头 + 家族 LICENSE×2 + NOTICE.md（上游 NOTICE 原文） | §3/§5 |
| §4(b) 改动文件显著声明 | 每文件 `Modified …` 头 + 逐文件对账清单 | §5 |
| §4(c) 不用其商标 | 命名/元数据全中性，商标原文仅作法律声明保留 | §6 |
| §4(d) NOTICE 保留 | 上游 NOTICE 原文保留 + 本项目 NOTICE 落档（OSS_NOTICE.md §12 + 家族 NOTICE.md） | §4 |

（MIT 侧义务为版权与许可文本保留，已覆盖。）
