# 四关质检·第三关：安全关（Security Gate）

```yaml
[Document-Meta]
Document:        docs/04-internal/license-port/gate3-security.md
Version:         1.0（许可搬运线第一批：图像核心 vendored 家族）
Last-Modified:   2026-10-18
Status:          🟢 活跃（四关质检报告，逐关一报）
Author:          许可搬运线（子代理施工，主代理验收）
```

> 结论先行：**安全关 PASS**。搬运面 secret-scan 0 命中；**0 构建脚本**（build.rs 无从谈起）；
> 0 网络 API、0 进程执行 API、0 静默 I/O（文件面仅显式文件 API 与测试调试旋钮）；
> 已知漏洞面闭包内 0 漏洞；两个二进制数据件溯源与许可落档齐备。

## 1. secret-scan（搬运面全覆盖）

- 命令：`pwsh -File scripts/secret-scan.ps1 -Mode scan-all`（18 patterns + .gitleaks.toml 豁免表）
- 实测（2026-10-18）：**[OK] No secrets found**（工作树全量，含 363 个搬运文件 + 11 个新 crate + 报告面）。
- 证据：`artifacts/licport-secret-scan.log`。

## 2. 构建脚本审查（build.rs 无网络无执行）

- 全搬运树检索 `build.rs`：**0 件**（十 crate 均无构建脚本，编译期零代码执行面——比「有 build.rs
  但无网络无执行」更强的结论）。
- 接口层 `crates/engine/image/core` 亦无 build.rs；proc-macro 面 0 件（依赖面的 serde derive 等
  为既有树内组件）。

## 3. 运行时行为面（无外联 / 无执行 / I/O 显式化）

对 `crates/engine/image/**/src/**` 全量检索（2026-10-18）：

| API 面 | 命中 | 判定 |
|---|---|---|
| 网络（`std::net` / TcpStream / UdpSocket / reqwest / hyper） | **0** | ✅ 无外联 |
| 进程执行（`std::process` / `Command::new`） | **0** | ✅ 无执行面 |
| 环境变量 | 14 处 | 全为**测试/基准旋钮**（`APEIRETH_IMAGE_MOTION_*`、`APEIRETH_IMAGE_UNSHARP_*`、`QS_DUMP`/`SUBJECT_DUMP`/`GALLERY_*`/`SEL_BENCH` 等调试转储）+ 1 处运行期合成缓存旋钮 `APEIRETH_IMAGE_FX_NOCACHE`（上游原名已中性化；行为=可选关缓存，非外联） |
| 文件 I/O | 5 处 | `psd/src/file.rs` 的显式文件 API（`from_path`/`write`——「核心 API 走字节切片，文件助手仅 native」上游设计如实保留）+ 测试调试转储（仅 env 旋钮触发） |

## 4. 已知漏洞面（cargo audit / cargo deny）

| 工具 | 命令 | 实测（2026-10-18） |
|---|---|---|
| cargo-audit | `cargo audit --db <advisory-db@8407…>`（本地库，取材走加速池节点） | 5 项漏洞**全部**在 `quinn-proto 0.11.16`（既有 reqwest/QUIC 依赖面，**不在本批闭包**）；4 项 unmaintained 警告（paste/zbus_names/zvariant/zvariant_derive）。本批闭包内漏洞 **0**；闭包含 `paste 1.0.15`（由 exr 传入，unmaintained **警告级**非漏洞，登记待上游/依赖升级收口） |
| cargo-deny | `cargo deny check licenses bans sources` | **licenses ok / bans ok / sources ok**（`bans.multiple-versions = "deny"` 严格档下 0 新增多版本） |
| 许可例外 | `deny.toml` `[[licenses.exceptions]]` | 仅 `jpeg-encoder` 放行 IJG 成分（最小面；理由与致谢见 gate1 §3/§8） |

## 5. 二进制数据件（恶意面审查 + 溯源）

| 文件 | 体量 | 内容 | 审查结论 |
|---|---|---|---|
| `algo/data/libfacedetection.bin` | 106,643 B | CNN 半精度权重（人脸检测），推理代码 `faces/cnn.rs` 为纯 Rust 矩阵运算 | 与上游 ATTRIBUTION 描述一致（BSD-3-Clause，Shiqi Yu）；静态权重数据、无脚本/无嵌入可执行面；许可原文随附 |
| `cms/profiles/coated-cmyk-synth.icc` | 236,848 B | ICC v2/v4 剖面（由 `cms/src/synth.rs` 生成器产出，本批以中性 producer 重新生成） | 纯色彩数据（tag 表 + AToB LUT）；生成器源码在树可审；CC0-1.0 置公有领域（上游 ATTRIBUTION）；重新生成后与生成器一致性由其自带 `regen` 测试校验（ΔE<1.0 模型回归） |

其余资产（字体/图标/词典/品牌）0 件进入（gate1 §9）。

## 6. 恶意面总评

- 无网络、无执行、无构建脚本、无混淆代码、无动态加载、无 unsafe；
- 解析器健壮性声明（上游）：畸形输入不 panic、维度/分配/嵌套深度全限额（psd README 自述 +
  其 malformed 测试面随批迁移运行）；
- 结论：**搬运面无可疑行为面**，附加工具面（W1 后续接线）经接口层消费时仍受本仓
  P0 governance 3 hook 既有治理面覆盖（不在本批改动）。
