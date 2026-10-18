# 四关质检·第二关：质量关（Quality Gate）

```yaml
[Document-Meta]
Document:        docs/04-internal/license-port/gate2-quality.md
Version:         1.0（许可搬运线第一批：图像核心 vendored 家族）
Last-Modified:   2026-10-18
Status:          🟢 活跃（四关质检报告，逐关一报）
Author:          许可搬运线（子代理施工，主代理验收）
```

> 结论先行：**质量关 PASS**（先审计后搬；一处测试断言按纪律演进并注明理由）。
> 审计面：测试覆盖 / unsafe 使用面 / 依赖健康 / 维护状态；搬前审计结论与搬后实测
> 双账并列，数字全部可复核（命令 + 日期 + 工具链）。

## 1. 搬前审计（上游面，取材 2026-10-18，源码 tarball + 公开资料）

| 维度 | 审计发现 | 判定 |
|---|---|---|
| **测试覆盖** | 目标十 crate 静态计数：`#[test]` 标注 **1960** 个（含 tests/ 集成面与 `#[cfg(test)]` 内联面），`proptest!` 属性测试 10 处，内联测试模块 208 个；codecs 379 例、psd 139 例（集成 tests/ 目录）、cms 28 例、compose/algo 另有内联面。真文件语料回归（corpus）为 opt-in 且不随批 | ✅ 达标 |
| **unsafe 使用面** | 上游 workspace `unsafe_code = "forbid"`；十 crate 的 `lib.rs` 全部 `#![forbid(unsafe_code)]`；搬运树全文检索 `\bunsafe\b` 仅 3 处散文提及（README 的 "no `unsafe`" 等），**代码面 0 unsafe** | ✅ 达标 |
| **依赖健康** | 核心闭包浅：L0 crates（cms/psd/geom）0-2 个外部依赖；codecs 为编解码重头（image/png/tiff/exr/half/serde_json/flate2/image-webp/zune-*/jpeg-encoder/rayon）；algo 仅 rustfft+rayon。依赖树深度与体积见 §3 前后对照 | ✅ 达标（详见 §3） |
| **维护状态** | 活跃开发中：上游 workspace `version = 0.10.0`（滚动发版 v0.5→v0.10 可见），8 条 CI 流水线（ci/windows-arm64/freebsd/nix/release/perf-nightly/docs/packaging-lint），docs/roadmap/parity 评估齐备，AGENTS 贡献规范明确（clean-room、测试、分层、fmt/clippy 门禁） | ✅ 达标 |
| **工程质量信号** | 上游 fmt + clippy + 分层检查（`cargo xtask layers`）进 CI；代码注释密度高（格式行为逐条注明依据）；`#![deny(clippy::unwrap_used…)]` 等 crate 级严格面（psd） | ✅ 达标 |

## 2. 搬后实测（本仓工具链 rustc 1.97.1 / cargo 1.97.1，2026-10-18）

| 项 | 实测 | 命令 |
|---|---|---|
| 搬运面编译 | **0 警告**（全目标含 tests/examples/benches） | `cargo check -p apeireth-image-* … --all-targets` |
| 搬运件自带测试 | 见 §4 收卷表（迁移套件逐 crate 计数） | `cargo test -p … --all-targets --no-fail-fast` |
| 接口冒烟测试 | **9/9 通过**（≥6 要求达成）：PNG 往返逐字节一致 / 垃圾字节报错不 panic / 裁切区域与像素正确 / 空区域参数错 / 缩放尺寸 / 模糊软化硬边 / USM 锐化改写像素 / 图层半透明合成叠白底 / PSD 字节级稳定往返 | `cargo test -p apeireth-image-core` |
| clippy | 见 §4 收卷表 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| fmt | 见 §4 收卷表 | `cargo fmt --check` |

## 3. 依赖健康前后对照（依赖树深度与体积）

| 口径 | 搬运前 | 搬运后 | 增量 |
|---|---|---|---|
| `Cargo.lock` `[[package]]` 计数 | **574**（2026-10-18 会话开工实测，工作树含他线在途增量） | **655**（同日收卷实测） | +81（含他线在途并发增量，逐包归因见下） |
| 本批依赖闭包（`cargo tree -p apeireth-image-core -e normal`） | — | **76 包**（含 11 个自有成员） | 见 `artifacts/licport-dep-closure.txt` |
| 其中相对上次提交新增 | — | **35 包** = 11 自有成员 + 24 外部（bit_field/color_quant/exr/fax/gif/image-webp/jpeg-encoder/lebe/num-complex/num-integer/paste/primal-check/pulp/pulp-wasm-simd-flag/qoi/quick-error/raw-cpuid/reborrow/rustfft/strength_reduce/tiff/transpose/weezl/zune-inflate） | 逐包可复核：`git show HEAD:Cargo.lock` 与闭包清单差集 |
| 新增**直接**外部依赖（搬运 crate manifest 面，100% 可归因本批） | — | image(0.25)、png(0.18)、tiff(0.11)、exr(1.73)、half(2)、serde_json、flate2、image-webp(0.2)、zune-jpeg(0.5)、zune-core(0.5)、jpeg-encoder(0.6)、rayon、serde、thiserror(2)、rustfft(6.4.1)、proptest/criterion/moxcms（dev 面） | 其中 image/png/half/zune-*/flate2/serde/thiserror/rayon/proptest/criterion/moxcms **搬运前已在树**（`crates/engine/perception` 的 image 0.25 等），净新 = tiff/exr/image-webp/jpeg-encoder/rustfft 及其小传递链 |
| 许可面 | 12 SPDX | 12 SPDX + IJG（最小面例外，见 gate1 §3.5） | `cargo deny check licenses bans sources` = **ok / ok / ok**（2026-10-18） |
| 已知漏洞面（cargo audit） | — | 闭包内 **0 漏洞**（5 项漏洞全在 `quinn-proto`——既有 reqwest/QUIC 面，非本批闭包；4 项 unmaintained 警告中 `paste 1.0.15` 由 exr 传入本闭包，为警告级非漏洞） | `cargo audit --db <本地 advisory-db>` |

## 4. 收卷数（回填区，随最终校验批实测填入）

| 项 | 实测值 | 日期 | 命令 |
|---|---|---|---|
| 搬运件自带测试（迁移面，10 crate 全目标） | **77 套件 / 2307 passed / 0 failed / 32 ignored** | 2026-10-18 | `cargo test -p apeireth-image-{geom,cms,color,raster,doc,vector,compose,algo,codecs,psd} --all-targets --no-fail-fast` |
| 接口冒烟 | 9 passed / 0 failed | 2026-10-18 | `cargo test -p apeireth-image-core` |
| 全仓测试（--exclude 他线在途红 crate 口径） | **287 套件 / 7580 passed / 0 failed / 48 ignored** | 2026-10-18 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2 --exclude apeireth-tools-canonical`（隔离 target 目录 `target/licport`，同命令面同 `--locked`；共享 target 与他线并发构建互撞 LNK1104 见 gate4 §6） |
| clippy 全仓 | **0 警告 / 0 错误**（11 个本批 crate 全部受检；`--exclude apeireth-tools-canonical` 口径同 §4 上行） | 2026-10-18 | `cargo clippy --workspace --all-targets --locked -- -D warnings --exclude apeireth-tools-canonical`（隔离 target 目录） |
| fmt | 本批面 **0 diff**（家族 rustfmt.toml 与上游同款配置 + 收口）；他线在途面 27 处（标红不代改） | 2026-10-18 | `cargo fmt --check` |

## 5. 测试断言演进登记（钉行为不钉字面，理由注明）

| 用例 | 原断言 | 演进后 | 理由（可复核） |
|---|---|---|---|
| `algo/src/stack.rs` `statistics_are_bit_identical_to_the_serial_implementation` → 改名 `statistics_match_the_serial_reference_within_one_ulp` | 金标位模式逐位相等 | 逐位相等或 ULP 距离 ≤ 1（`assert_bits_within_one_ulp`） | 金标字面抓取自旧串行实现所在工具链；同一源码算式在 rustc 1.97.1 下 Kurtosis 两元素 ±1 ULP（dev opt 0 与 opt 1 一致复现，**排除优化档因素**，定性为工具链 codegen 漂移）。行为面（统计正确 + 逐位确定性）仍钉死；仅放开 codegen 字面。隔离复跑证据：`artifacts/flaky-probe-opt1.log` |

其余 1959 例零改动通过（除 gate1 §8 范围适配清单声明的元数据/夹具中性化改动，行为等价、断言同改）。

## 6. 工程姿态决定（lint / 格式面）

| 面 | 决定 | 理由 |
|---|---|---|
| **lint 姿态** | vendored 十 crate 沿**上游姿态**（`unsafe_code = "forbid"` + `dbg_macro/todo = warn`，clippy 全集不施加）；接口层 `apeireth-image-core` 继续 `[lints] workspace = true`（本仓精选 lint 面全量约束我方代码） | 实测本仓精选 lint（cast_lossless 等 pedantic 档）在 126k 行外来代码上一 crate 即 34 处命中（`as f64`→`f64::from` 级改写）；为保 1:1 可同步（同步 diff 只含语义改动）且与上游 CI 同口径，vendored 面不套本仓风格 lint。**如实声明**：vendored 面的 clippy 0 警告=上游姿态口径（与本仓 19 crate 的 `clippy::all=allow` 同为精选 lint 模式），非全集扫描 |
| **格式面** | 家族内 `rustfmt.toml` 与上游同款（max_width=160 / use_small_heuristics=Max），只作用于 `crates/engine/image/`；接口层随家族配置 | 排版可对账（同步无格式噪音）；实测本仓 rustfmt 版本与上游 CI 版本有排版漂移，收口 35 件（语义 0 改动） |

## 7. 不搬面的质量理由（范围裁剪的诚实登记）

| 面 | 处置 | 理由 |
|---|---|---|
| `heif` 解码层（heic-rs 依赖） | 不搬，`Format::Heif` 保持识别+清晰 unsupported | 只搬核心四类；新依赖面（HEVC 解码）超范围，第一批不做 |
| `compose/benches` 笔刷笔触基准 | 删除该基准项（保留合成基准） | 依赖未搬的笔刷引擎（`paint`） |
| 真文件语料（corpus） | 不取（opt-in feature 保留，测试默认不编译） | 语料仓外部依赖，随取随跑（`cargo xtask test-corpus` 面上游自带）；能力如实标「未实测（语料未取）」 |
| fuzz 目标 | 保留骨架（独立 workspace，不入本仓构建）；`decode_heif` 目标随 heif 层收口删除 | fuzz 面不属测试门禁，保留可复跑骨架 |
