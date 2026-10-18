# 创作工具面 W1 · 图像操作换创作内核（image_ops core-swap）交付卷宗

```yaml
[Document-Meta]
Document:        docs/04-internal/backend-enhancement/image-ops-core-swap-delivery.md
Version:         1.0
Last-Modified:   2026-10-18
Status:          🟢 交付（换核 + 能力扩展 + 依赖处置 + 能力矩阵随行）
Scope:           crates/capabilities/tools/{Cargo.toml,src/image_ops.rs,src/lib.rs} +
                 tests/image_ops_core_swap.rs + Cargo.lock（依赖边）+ 能力矩阵行
```

## 0. 一句话

`tool.image_ops` 的 resize/crop/rotate/flip/blur/sharpen/format_convert 从外部图像库
实现换为引擎级创作内核 `apeireth-image-core`（接口层 + 图像核心家族，许可与四关
质检见 `docs/04-internal/license-port/`）消费，工具面行为零变化（25 例金标准测试
原样全绿）；借核面新增 `layers_composite` / `psd_read_info` 两个操作（同款参数校验
与测试）；产品代码外部图像库直接依赖清零（测试专用 dev 依赖保留，如实登记核面
内部既有委派）。

## 1. 换核对照表

| 操作 | 换核前实现 | 换核后实现（核面消费口） | 像素级一致性口径 |
|---|---|---|---|
| decode（含炸弹图三层上限） | 外部库 `ImageReader` + 解码器 Limits + 解码后像素复核 | 核面 `engine::codecs::decode_with`（`Limits` 维度/像素/分配三上限显式同值，`keep_orientation: true` 保换核前「按存储像素」语义）+ 解码后像素复核 + `to_rgba8` 归一 | 拒绝面（上限/坏流）帧逐类同款；解码像素全等（同字节流 → 同 RGBA8） |
| 内容嗅探白名单 | 外部库 `guess_format` | 核面 `engine::codecs::detect` | 白名单外拒绝即帧同款（gif 等表外格式拒） |
| resize | `imageops::resize`（三角核） | 核面 `engine::algo::resample::resize_surface_in_canvas`（画布内重采样·线性核档：边像素外扩，不透明图边缘不衰减） | **内核换档，逐像素全等不成立**：尺寸精确 + 纯色逐像素不漂移（金标准①）+ 偏差上界抽样钉（见 §3）；接口层门面 `resized` 的「边缘向默认像素衰减」语义会把不透明边压半透明（实测 alpha 255→234），与工具契约不符，读码定直取 |
| crop | `imageops::crop_imm` | 核面 `RasterImage::cropped` | 逐像素**全等**（抽样断言） |
| rotate（90/180/270） | `imageops::rotate90/180/270` | 核面引擎直取口 `engine::algo::transform::warp_surface`（整数单应 + Nearest = 像素置换） | 逐像素**全等**（抽样断言，含 alpha 渐变样本） |
| flip（horizontal/vertical） | `imageops::flip_horizontal/vertical` | 同上（镜像单应） | 逐像素**全等**（抽样断言） |
| blur | `imageops::blur`（库高斯） | 核面 `engine::algo::apply_in`（引擎高斯 `GaussianBlur`，radius=σ 同语义；显式 extent = 边像素外扩，与换核前邻域语义对齐） | **内核换档**：纯色逐像素不漂移（精确）+ 偏差上界抽样钉（实测 1/255，见 §3）；门面 `blurred` 的邻域读透明档会把不透明边压半透明（实测 alpha 偏差逾百），读码定直取 `apply_in` |
| sharpen | `imageops::unsharpen` | 核面 `engine::algo::apply_in`（引擎 USM `UnsharpMask`，amount 固定 100% 与 unsharpen 同语义，threshold 层级轴同款；显式 extent 同 blur） | **内核换档**：纯色逐像素不漂移（精确）+ 偏差上界抽样钉（实测 0/255，见 §3） |
| format_convert → encode png/webp | 外部库 `PngEncoder` / `WebPEncoder`（仅无损） | 核面门面 `encode`（`OutputFormat::Png/WebP`；webp 无损档 = 核面默认档，契约由金标准⑥无损断言钉住） | 无损往返逐像素**全等** |
| format_convert → encode jpeg | 外部库 `JpegEncoder`（质量 90，白底合成，4:4:4 无色度抽样） | 白底合成（自研，语义不变）+ 核面 `engine::codecs::encode`（`EncodeOptions` 显式钉 `jpeg_quality = 90` + `jpeg_chroma_subsampling = false`——核面默认档是 4:2:0，读码定改为换核前同档 4:4:4） | 有损不冒充无损：换核前后同款编码偏差上界抽样钉（实测 6/255，见 §3）；均匀内容 ±20 容差由金标准⑥钉住 |
| format_convert → encode bmp | 外部库 `BmpEncoder` | 核面 `engine::codecs::encode(Format::Bmp)`（接口层 `OutputFormat` 无 bmp 档，读码定引擎直取口） | 无损往返逐像素**全等** |
| brightness / contrast / grayscale / watermark_text | 自研逐通道算术 + 5×7 点阵字模 | **保留自研**（不在核面；缓冲承载体换为核面 `RasterImage`） | 逐像素**全等**（换核前后同公式独立重算比对） |
| 输出格式裁定（显式 > 扩展名 > 输入） | 外部库 `ImageFormat::from_path` | 核面 `engine::codecs::from_extension` | 白名单外扩展名拒绝即帧同款；两库已知扩展名表的差异仅存在于表外长尾（如实登记） |

### 1.1 新增操作（换核红利，同款参数校验）

| 操作 | 参数（恰好这套，缺/多/越界拒绝即帧） | 核面能力 | 行为 |
|---|---|---|---|
| `layers_composite` | `overlay`（工作区路径）/ `x` / `y`（0..=8192）/ `opacity`（0.0..=1.0 有限） | `engine::color::blend::composite`（W3C/PDF 源叠加通用公式，直通 alpha） | 叠加图走同一受控读面（白名单 + 字节上限 + 敏感面拒 + 观测门禁）解码后贴到工作图 `(x, y)`；整框须落图内（越界拒绝即帧），框外像素一个不动；量化回 u8 用核面同款四舍五入 |
| `psd_read_info` | `path`（工作区路径） | `engine::psd`（分层文档头 + 图层清单解析） | 只读探针（像素零改动）：同一受控读面（路径/敏感面/上限/观测），内容走分层文档解析而非白名单像素解码；产出进汇报探针节 `psd_infos`（相对路径、尺寸、位深、通道数、色彩模式、版本 psd/psb、图层数、图层名≤64 截断显式标记） |

词表 11 → 13（`OPERATION_VOCABULARY` 闭合全集，自述创作节逐操作计数行随行）。

## 2. `image` 依赖处置（读码定，零假装）

- **产品面清零**：`crates/capabilities/tools` 产品代码不再直接引用外部图像库；
  `image` 从 `[dependencies]` 降为 `[dev-dependencies]`（金标准测试原样要用它做
  fixture 生成/回读 + 换核前同款调用的基线重算，测试文件零改动）。
- **核面全接管**：白名单四格式编解码全部经核面（jpeg 有损质量 90 / webp 无损档
  与换核前口径一致）。
- **如实登记（核面内部委派）**：核面 codecs 家族对 bmp/gif/tga/ico/qoi/hdr 等
  格式的编解码内部仍委派外部纯 Rust 图像库（属许可搬运线第一批的依赖闭包既有
  登记，见 `artifacts/licport-dep-closure.txt`）；本层换核不改变该事实，也未新增
  任何外部包。
- `Cargo.lock` 变更 = `apeireth-tools-canonical` 依赖表 +1 条 workspace 内部边
  （`apeireth-image-core`），版本轴零漂移。

## 3. 换核前后像素级一致性抽样（测试证据）

参照口径：换核前实现的同款调用在 dev 依赖图像库上重算，与工具输出逐像素比对。
- 全等面（抽样 7×5/8×8 特征图，逐像素含 alpha 渐变）：crop / rotate90/180/270 /
  flip H/V / brightness / contrast / grayscale —— **逐像素全等断言**（`tests/image_ops_core_swap.rs` ①②）。
- 无损编解码面（8×8 特征图）：png / webp / bmp 往返 **逐像素全等**；jpeg 有损 ±20
  容差如实（⑤）。
- 滤镜内核换档面（上界 = 实测值 + 余量，测试断言钉死，`tests/image_ops_core_swap.rs` ③④⑤）：
  - resize：17×13 → 9×7（缩）与 34×26（放），每通道偏差**实测 27/255 与 1/255**（三角核 vs
    线性核），上界登记 32/255；纯色图缩放逐像素不漂移（精确）。
  - blur σ=1.2（16×16 渐变）：每通道偏差**实测 1/255**（库高斯 vs 引擎高斯，边缘语义已对齐），
    上界登记 4/255；纯色图模糊逐像素不变（精确）。
  - sharpen σ=1.0 / threshold=4（16×16 渐变）：每通道偏差**实测 0/255**（库 USM vs 引擎 USM），
    上界登记 2/255；纯色图锐化逐像素不变（精确）。
  - jpeg（8×8 高频合成图，最坏情况）：换核前后**同款编码**（白底合成 + q90 + 4:4:4）每通道
    偏差**实测 6/255**，上界登记 12/255。读码发现核面默认色度抽样档是 4:2:0、换核前编码是
    4:4:4（逐像素抽样实测偏差 77/255），已显式钉 4:4:4 对齐换核前行为（零假装：这是编码档
    对齐，不是把差异粉饰掉）。

## 4. 测试证据（本批实测，2026-10-18）

见 §6 校验记录。构成：
- **金标准 25 例零回归**：工具面像素级集成 16（`tests/image_ops.rs` 原样）+ 单元 4 +
  自述创作节 1 + 装配级 2（`runtime-assembly/tests/image_ops_face.rs`）+ cli 旋钮 2
  （`adapters/cli/tests/image_ops_knob.rs`）。**唯一演进断言**（如实报备）：
  `operation_vocabulary_is_closed_and_parses` 的词表计数 11 → 13，按该断言自身授权
  条款「增删须显式改表并补测试」执行（能力扩展的既定程序），断言消息同步改写，
  其余 24 例 + 本例其余断言零字面改动。
- **新测试 11 例**（`tests/image_ops_core_swap.rs`）：换核前后一致性 5（几何全等 /
  通道算术全等 / resize 上界+不变量 / blur+sharpen 上界+不变量 / 编解码全等）+
  新操作行为钉 6（layers_composite 钉值 / 参数矩阵+受控读面 / psd_read_info 信息钉 /
  坏文档与参数矩阵 / 探针顺序与截断标记 / 词表与声明闭合）。

## 5. 未收口项（0 假装）

| # | 项 | 状态 | 归口 |
|---|---|---|---|
| 1 | resize/blur/sharpen 滤镜内核换档（线性核/引擎高斯/引擎 USM），非逐像素全等 | 如实登记（§1/§3 偏差上界钉死）；像素级全等不适用于不同内核，不假装 | 如需更贴近旧核，核面可加滤镜档位选择（后续批次候选） |
| 2 | 输出扩展名映射轴换核面 `from_extension`，与换核前库的已知扩展名长尾有差异 | 如实登记；白名单四格式行为一致，表外扩展名两代皆拒绝 | 如需长尾对齐另立批次 |
| 3 | 真文件语料（corpus）未取，滤镜偏差上界为合成图抽样实测 | 如实标注 | 需要时取语料扩样复测 |
| 4 | 共享工作树他线在途红（`doc_ops` 单测在并发编辑下曾偶发失败，隔离复跑绿；本批全量校验时未再现） | 标红不代改（非本批战场） | doc_ops 归口线自检 |

## 6. 校验记录（2026-10-18 实测，`-j 2` 口径）

| 命令 | 结果 |
|---|---|
| `cargo test -p apeireth-tools-canonical --locked -j 2` | 全绿 0 failed（含金标准集成 16 + 单元 4 + 自述创作节 1 + 新增 11 例） |
| `cargo test -p apeireth-runtime-assembly --test image_ops_face --locked -j 2` | 2/2 绿（装配级金标准） |
| `cargo test -p apeireth-cli --test image_ops_knob --locked -j 2` | 2/2 绿（cli 旋钮金标准） |
| `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | **323 套件 / 8509 passed / 0 failed**（cargo exit 0；全量实测日志 `artifacts/w1-core-swap-workspace-test.log`；「套件」= 测试二进制目标计数，本批 +1 = `image_ops_core_swap`） |
| `cargo clippy -p apeireth-tools-canonical --all-targets --locked -j 2 -- -D warnings` | 0 警告 |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | 0 警告（cargo exit 0，日志 `artifacts/w1-core-swap-workspace-clippy.log`） |
| `cargo fmt --check`（全仓）/ `cargo fmt -p apeireth-tools-canonical -- --check` | 0 diff |
| `pwsh -File scripts/check-neutral-terms.ps1` | 0 命中 |
| `pwsh -File scripts/secret-scan.ps1 -Mode scan-all` | 无命中 |

金标准 25 例逐套绿账：`tests/image_ops.rs` 16/16（原样）+ `src/image_ops.rs` 单元 4/4 +
`src/self_status.rs` 自述创作节 1/1 + `runtime-assembly/tests/image_ops_face.rs` 2/2 +
`adapters/cli/tests/image_ops_knob.rs` 2/2。
