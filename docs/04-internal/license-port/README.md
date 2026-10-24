# 许可搬运线·第一批：图像核心（四关质检卷宗）

```yaml
[Document-Meta]
Document:        README.md
Version:         1.0（第一批：图像核心 vendored 家族）
Last-Modified:   2026-10-18
Status:          🟢 活跃（逐批卷宗，第一批收卷）
Author:          许可搬运线（子代理施工，主代理验收）
```

## 0. 一句话

按宪章〇之二（许可搬运线四关质检），把一个 MIT OR Apache-2.0 双许可项目的**图像核心能力**
（编解码 / 图层 / 滤镜 / 几何四类 + 依赖闭包）以逐文件可对账方式搬入引擎层，成为
Apeireth 创作内核；四关逐关出报告，任一关不过=不搬该件。

## 1. 卷宗索引

| 文件 | 内容 |
|---|---|
| [gate1-license.md](gate1-license.md) | 许可关：取材溯源 / 许可核实 / 版权主体清单 / NOTICE 落档 / 版权头与改动声明 / 商标纪律 / 扫描器豁免表 / 范围适配清单 |
| [gate2-quality.md](gate2-quality.md) | 质量关：测试覆盖 / unsafe 面 / 依赖健康 / 维护状态（搬前审计 + 搬后实测）/ 断言演进登记 |
| [gate3-security.md](gate3-security.md) | 安全关：secret-scan / 构建脚本审查 / 无外联无执行 / 已知漏洞面 / 二进制数据件 |
| [gate4-integration.md](gate4-integration.md) | 集成关：workspace 成员化 / 构建可复现 / 测试收卷 / 体积与依赖树前后对照 / 平台兼容 |

## 2. 交付物地图

| 交付物 | 位置 |
|---|---|
| vendored 图像核心家族（10 crate） | `crates/engine/image/{geom,cms,color,raster,doc,vector,compose,algo,codecs,psd}/` |
| 接口层（我们的门面） | `crates/engine/image/core/`（`apeireth-image-core`）+ `tests/smoke.rs`（9 例） |
| 许可落档 | `crates/engine/image/{LICENSE-MIT,LICENSE-APACHE,NOTICE.md}` + `OSS_NOTICE.md` §12 |
| 搬运/同步工具（单一真源） | `scripts/vendor-image-core.ps1` + `scripts/image-core-overrides/` |
| 逐文件对账清单 | `artifacts/vendor-image-core-files.txt`（Modified 252 + verbatim 111 + 1 override） |
| 依赖闭包清单 | `artifacts/licport-dep-closure.txt`（76 包） |
| 校验日志 | `artifacts/{vendor-check-1,vendor-test-*,facade-test-*,licport-*}.log` |

## 3. 接口面（给 W1 图像操作工具面的消费口，本批不接线）

`apeireth-image-core` = 稳定中性 API：`decode/encode`（`OutputFormat`）、`RasterImage`
（`cropped/resized/blurred/sharpened`）、`LayerDocument` + `flatten`、`crop/resize/blur/sharpen`
（Surface 面，区域显式）、`engine::*`（原始家族直取口）。冒烟测试钉住主链行为
（PNG 往返 / 错误面 / 几何 / 滤镜 / 图层合成 / PSD 往返）。**接线归 W1 线**（工具面换用
此核心）；本批 0 触碰 `crates/capabilities/tools`。

## 4. 风险与未收口项（0 假装）

| # | 项 | 状态 | 归口 |
|---|---|---|---|
| 1 | 共享工作树他线在途编译红（`apeireth-tools-canonical` 12 errors）阻塞全量测试出数 | 标红报备，`--exclude` 口径完成对照 | W1/归口线自检收口后全量复验 |
| 2 | 中性扫描器补强后，他线两份在途调研文档曾出 20 命中（`docs/04-internal/philosophy-study/05-craft-family.md`、`docs/design/readme-research-notes.md`，性质=研究引注） | **已收口（2026-10-18）**：归口线按报备自注册事实引注豁免（`scripts/check-neutral-terms.ps1` `$exemptFiles`），全仓扫描复测 **OK 0 命中** | 归口线自检收口 ✓ |
| 3 | `paste 1.0.15`（exr 传入）unmaintained 警告 | 登记，非漏洞 | 随依赖升级线收口 |
| 4 | 真文件语料（corpus）未取，相关回归「未实测」 | 如实标注 | 需要时取语料跑 opt-in 面 |
| 5 | wasm 面未实测（上游支持，本仓门禁不含） | 如实标注 | 后续批次 |
| 6 | HEIF 解码层未搬（识别+unsupported） | 范围裁剪登记 | 后续批次候选 |

## 5. 复跑指南

```powershell
# 1) 从上游源一键重放搬运（幂等）
pwsh -File scripts/vendor-image-core.ps1 -SourceCrates <上游仓库>/crates -Dest crates/engine/image
# 2) 校验（-j 2 口径）
cargo test --workspace --all-targets --locked --no-fail-fast -j 2   # 全量（待红面收口）
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --check
pwsh -File scripts/check-neutral-terms.ps1
pwsh -File scripts/secret-scan.ps1 -Mode scan-all
cargo deny check licenses bans sources
```
