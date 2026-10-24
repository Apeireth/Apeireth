# 四关质检·第四关：集成关（Integration Gate）

```yaml
[Document-Meta]
Document:        gate4-integration.md
Version:         1.0（许可搬运线第一批：图像核心 vendored 家族）
Last-Modified:   2026-10-18
Status:          🟢 活跃（四关质检报告，逐关一报）
Author:          许可搬运线（子代理施工，主代理验收）
```

> 结论先行：**集成关 PASS**（带一项现场红的如实标注：共享工作树存在他线在途编译红，
> 本批用 `--exclude` 口径完成零回归对照，红面归口收口后可全量复验）。

## 1. workspace 成员化（命名中性）

| 项 | 值 |
|---|---|
| 新增成员 | **11 个**：`crates/engine/image/{core,geom,cms,color,raster,doc,vector,compose,algo,codecs,psd}` |
| 命名 | 全中性 `apeireth-image-*`（接口层 `apeireth-image-core`）；crate 口径由 `20 members（19 crates + bench harness）` → **`31 members（30 crates + bench harness）`**（AGENTS.md §3 已同步） |
| 层级合规 | 挂 `crates/engine/`（引擎级）；仅依赖 foundation 允许面以下（本家族只依赖外部 crate + 家族内互依，0 反向依赖）——满足 §8.8 单向依赖 |
| 消费口径 | 接口层 = W1 图像操作工具面的后续消费口（本批只交核心+接口，不触碰 `crates/capabilities/tools` 战场） |

## 2. 构建可复现

- **一条命令重放**：`pwsh -File scripts/vendor-image-core.ps1 -SourceCrates <上游仓库>/crates -Dest crates/engine/image`
  ——拷贝/重命名/改写/适配/署名头全部确定性（规则与适配全在脚本，单一真源），重复执行幂等。
- **重放验证（2026-10-18 实测）**：脚本一键重放 + 两道收尾步（家族 rustfmt 收口、合成 CMYK
  ICC 再生成）后，367 文件中 **366 件逐字节一致**，唯一差异 = ICC 二进制（即「再生成」收尾步
  的产物本身）。收尾步已写进脚本头注（单一真源含后处理说明）。
- **逐文件对账**：`artifacts/vendor-image-core-files.txt`（Modified 252 + verbatim 111 + 1 override）。
- **构建口径**：`edition = "2024"`（编译语义零漂移钉死，逐 crate 显式）；`version/license/rust-version`
  沿本仓 workspace 轴（单轴口径 0 硬编码）；依赖版本串按上游 `[workspace.dependencies]` 显式钉版
  （不走本仓 workspace 继承，避免 thiserror 1.x/2.x 等轴差；理由：vendored manifest 与上游 1:1 可对账）；
  lint 姿态同理沿上游（见 gate2 §6）。
- **`--locked` 可复现**：Cargo.lock 已随本批更新，`cargo test --locked` 口径可复跑（见 §4 命令列）。

## 3. 测试面（收卷数·回填区）

| 项 | 实测值 | 日期 | 命令 |
|---|---|---|---|
| 搬运件自带测试（迁移面，10 crate） | **77 套件 / 2307 passed / 0 failed / 32 ignored** | 2026-10-18 | `cargo test -p apeireth-image-{geom,cms,color,raster,doc,vector,compose,algo,codecs,psd} --all-targets --no-fail-fast` |
| 接口冒烟测试 | 9 passed / 0 failed | 2026-10-18 | `cargo test -p apeireth-image-core` |
| 既有套件零回归（`--exclude apeireth-tools-canonical` 口径） | **287 套件 / 7580 passed / 0 failed / 48 ignored**（含本批新增约 80 套件；既有面 0 failed + 本批对既有 19 crate 源码 0 触碰，diff 可查） | 2026-10-18 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2 --exclude apeireth-tools-canonical`（隔离 target 目录 `target/licport`） |
| 终态复测（收卷后 lint 属性/署名头微调面） | **77 套件 / 2307 passed / 0 failed / 32 ignored**（与调面前数字一致） | 2026-10-18 | `cargo test -p <家族 11 包> --all-targets --no-fail-fast -j 2`（日志 `artifacts/licport-family-final-retest.log`） |
| 全量复验（含红 crate） | **未实测——他线在途编译红阻塞**（见 §6） | — | 待归口线收口后跑全量命令 |

## 4. 体积 / 依赖树前后对照

| 口径 | 搬运前 | 搬运后 | 增量 |
|---|---|---|---|
| workspace 成员 | 20（19 crates + bench harness） | 31（30 crates + bench harness） | +11 |
| 代码体量 | — | vendored 375 文件 / 6,178,084 B（.rs 353 件 / 126,060 行）+ 接口层 7 文件（lib+6 模块+冒烟 9 例） | 一次性引入，后续按脚本同步 |
| `Cargo.lock` 包计数 | 574 | 655 | +81（含他线在途增量；本批闭包净新 35 包 = 11 自有 + 24 外部，逐包清单 `artifacts/licport-dep-closure.txt`） |
| 依赖树深度 | — | 本批闭包 76 包（`cargo tree -p apeireth-image-core`），深度不超既有 reqwest/wasmtime 面 | 无循环、无重复版本新增（cargo deny bans ok） |
| 构建耗时（增量） | — | `cargo check` 全目标 33.7 s；algo 测试面最重（dev 未优化 458 s） | 测试门禁用 `-j 2` 口径不同时占满 |

## 5. 平台兼容

- **Windows MSVC 实测通过**（本批全部构建/测试实测于 Windows，MSVC 14.44 + Win SDK 10.0.26100 口径同 test-baseline）。
- 上游自述支持 wasm32（家族内 `cfg(target_arch = "wasm32")` 分支随源保留）；本仓 wasm 面**未实测**
  （不在本批门禁口径），如实登记。

## 6. 现场红与零回归口径（0 假装）

- 共享工作树存在**他线在途编译红**：`apeireth-tools-canonical` 12 errors（`image` 依赖面未对齐 +
  `FsVersionProbe::version` 缺方法），红点在他线未跟踪新文件（`image_ops.rs` 等），本批 0 触碰
  （施工纪律：他人在途红标红不代改）。该红使全量 `cargo test --workspace` 无法出数（构建失败即
  整体不跑），故零回归对照采用 `--exclude apeireth-tools-canonical` 口径，并在收卷表如实标注。
- 共享 `target/` 与他线并发构建互撞（他线 workspace 测试同样编译本批新成员，algo 测试二进制被
  并发占用 → LNK1104 三次实测）；门禁出数改用**隔离 target 目录** `target/licport`（同命令面、
  同 `--locked`、语义等价），共享面冲突如实登记。
- 待归口线收口后建议全量复验一次（命令见 §3 末行），结果回填本表。

## 7. 口径同步登记

- `AGENTS.md` §3 crate 口径已更新为 31 members（30 crates + bench harness）。
- `docs/03-reference/test-baseline.md` 追加本批复测注记（历史行不追溯改写）。
- 根 `Cargo.toml` 成员表追加本家族 11 行（workspace.version 等 LOCKED 项 0 触碰）。
