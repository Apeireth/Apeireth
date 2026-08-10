# Apeireth License FAQ (18 常见问题)

> **性质**: 18 常见问题 + 简明答案
> **依据**: Apache-2.0 + Apeireth 实际情况
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-5)

---

## 0. TL;DR

| 类别 | 问题数 | 跳转 |
|------|------:|------|
| 1. 基础问题 | 4 | [§1](#1-基础问题) |
| 2. 商用 / SaaS | 4 | [§2](#2-商用--saas) |
| 3. 商标 / 品牌 | 3 | [§3](#3-商标--品牌) |
| 4. 修改 / fork | 3 | [§4](#4-修改--fork) |
| 5. 专利 / 责任 | 2 | [§5](#5-专利--责任) |
| 6. 合规 / 错误报告 | 2 | [§6](#6-合规--错误报告) |

---

## 1. 基础问题

### Q1.1: Apeireth 是什么 license?

**A**: **Apache License 2.0** (per 根目录 `LICENSE` 文件 180 行). 完整文本见 https://www.apache.org/licenses/LICENSE-2.0.

### Q1.2: 为什么选 Apache-2.0, 不用 MIT 或 GPL?

**A**: 主人 2026-08-04 拍板 (per `docs/adr/0001-apeireth-rust-1.0.md`):
- **MIT 缺点**: 无专利授权 (Apache-2.0 §3 有)
- **GPL 缺点**: 强传染, 商业产品难以集成
- **AGPL 缺点**: SaaS 强传染, 不适合做平台
- **Apache-2.0 优点**: 商用友好 + 专利授权 + 商标边界清晰 + 业界主流 (Kubernetes / TensorFlow / Swift 都在用)

### Q1.3: 我能看到完整 license 文本吗?

**A**: 三处:
1. 根 `LICENSE` (180 行, 编译期 hardcode 引用)
2. 根 `THIRD-PARTY-NOTICES.md` §A (1709 行, 含 12 SPDX 完整文本)
3. [docs/licenses-3rdparty/](../licenses-3rdparty/) (50+ 第三方 LICENSE 副本, D-1)

### Q1.4: 我能为 Apeireth 买商业保险吗 (E&O)?

**A**: ✅ **能**, 但保险**不**包含 Apeireth 本身的担保 (per §7-8 无担保 / 责任限制). 你的保险应**单独**买 (e.g. Hiscox / Chubb / AIG 都卖 tech E&O).

---

## 2. 商用 / SaaS

### Q2.1: 我能用 Apeireth 跑 SaaS 服务卖客户吗?

**A**: ✅ **能** (per §1 "publicly perform" + "distribute").
- 你**不**需要给客户分发 Apeireth 二进制 (SaaS 不算 distribution per §3)
- 你**不**需要公开你的 SaaS 源码 (Apache-2.0 **不**传染)
- 你**必须**在 About 页引 "Powered by Apeireth" + 链接 LICENSE

详见 [02-commercial-use.md §2.1](02-commercial-use.md#21-saas-部署-最常见)

### Q2.2: 我能修改 Apeireth 然后卖服务吗?

**A**: ✅ **能**.
- 你的修改 = 你的版权
- 你的客户**必须**能拿到你修改后的源码 (per §3 Source form)
- 你的客户**不能**再分发你修改的源码给第三方 (per §2 终止条款)

详见 [02-commercial-use.md §2.3](02-commercial-use.md#23-修改--卖服务)

### Q2.3: 我用 Apeireth 训练竞品 AI 模型可以吗?

**A**: ✅ **能** (per §1 "use" 权).
- 你的训练数据 = 你的版权
- 你的模型 = 你的版权
- 0 用 "Apeireth" 商标
- 0 反向工程 Apeireth 私有算法 (商业秘密法, **不** per Apache-2.0)

详见 [02-commercial-use.md §6 Q6](02-commercial-use.md)

### Q2.4: 我能用 Apeireth 卖付费 API 转发吗 (Apeireth-as-a-Service)?

**A**: ✅ **能**, 但:
- 0 用 "Apeireth" 商标 (per §6)
- 你的 SLA 独立负责
- 保留 LICENSE + NOTICE 引用

---

## 3. 商标 / 品牌

### Q3.1: 我能注册 "Apeireth" 商标吗?

**A**: ❌ **不能** (per §6 商标边界). "Apeireth" 名称 + logo 归 Apeireth Team 所有.

### Q3.2: 我能在我的产品名里用 "Apeireth" 吗?

**A**: ⚠️ **看用法**:
- ✅ 描述性: "YourProduct (基于 Apeireth)" / "YourProduct for Apeireth"
- ❌ 营销: "Apeireth Pro" / "Apeireth 企业版" / "Official Apeireth"

详见 [02-commercial-use.md §3](02-commercial-use.md#3-商标边界-per-6) + [03-modification-redistribution.md §3](03-modification-redistribution.md#3-商标边界-再分发场景-per-6)

### Q3.3: 我能用 Apeireth logo 吗?

**A**: ❌ **不能** 营销 / 包装 / 域名. ✅ **能** 在 "About" / "Open source licenses" 页**描述**你的产品包含 Apeireth.

---

## 4. 修改 / fork

### Q4.1: 我 fork Apeireth 然后改 1000 处, license 还是 Apache-2.0 吗?

**A**: 看情况:
- 你的修改 = **你的版权** (你拥有)
- 你的修改部分 = **你决定 license** (可 Apache-2.0 / MIT / Proprietary)
- 原始 Apeireth 部分 = **Apache-2.0** (不可改)
- 你的 fork LICENSE 写 "Apeireth 部分 Apache-2.0, 你的修改部分 <你的 license>"

详见 [03-modification-redistribution.md §1](03-modification-redistribution.md#1-你必须做的-5-件事-per-4)

### Q4.2: 我能去掉 LICENSE / 版权声明再分发吗?

**A**: ❌ **不能** (per §4(a) 红线). 去掉 LICENSE = 违反 Apache-2.0, 你的分发**无效**, 客户**没**法合规使用.

### Q4.3: 我能改 Apeireth 然后闭源卖吗?

**A**: ✅ **能** (Apache-2.0 **不**传染, 不像 GPL/AGPL).
- 你的修改 = 你的版权
- 你**不**需要公开你整个产品源码
- 但你的客户**必须**能拿到**你修改的 Apeireth 部分**的源码 (per §3 Source form)

详见 [03-modification-redistribution.md §2.2](03-modification-redistribution.md#22-fork--闭源仓库)

---

## 5. 专利 / 责任

### Q5.1: 我能用 Apeireth 卖产品, 享受专利保护吗?

**A**: ✅ **能** (per §3 专利授权). Apeireth 贡献者授予你**永久 / 全球 / 免费 / 不可撤销**的专利许可, 制造 / 使用 / 卖 / 进口.

### Q5.2: 我能告 Apeireth 专利侵权吗?

**A**: ⚠️ **不建议** (per §3(b) 专利反诉):
- 你起诉 → 你**自己的** Apache-2.0 专利许可**立即终止**
- 你**失去**用 Apeireth 的权利
- 你**失去**你的客户再许可给他们的权利

实操: 你应**不**起诉, 而是跟 Apeireth Team 谈 cross-license.

### Q5.3: Apeireth 团队为我用 Apeireth 造成的损失负责吗?

**A**: ❌ **不**负责 (per §7-8 无担保 + 责任限制). 你**自己**负商业责任, 应:
- 买 E&O 保险
- 客户合同明确 SLA
- 0 依赖 Apeireth Team 的承诺

---

## 6. 合规 / 错误报告

### Q6.1: 看到 LICENSE 错误 / 缺失 attribution 怎么办?

**A**: 提 GitHub issue:
- 仓库: https://github.com/apeireth/apeireth-rust/issues
- 标题: `[LICENSE] <错误描述>`
- 内容: 出错文件 + 期望 vs 实际
- 标 `D-1 license compliance`

我们会在 **R21+ 估补** (per 整合 #3 D-5). 1.0 release **不**阻塞 tag.

### Q6.2: 我用 cargo-deny 看到一个 license violation, 怎么报告?

**A**: 提 issue:
- 标题: `[RUSTSEC/LICENSE] <CVE / SPDX ID>`
- 内容: `cargo deny check licenses` 输出 + 期望的 deny.toml 配置
- 标 `D-1` + `D-12 security`

我们会在 **R21+ 估补**.

### Q6.3: 我能要求加某个 license 到 allow-list 吗 (deny.toml)?

**A**: 提 issue + 提案, 主人**可能**接受:
- 🟢 双许可 (MIT OR Apache-2.0): **可能接受** (业界惯例)
- 🟡 弱 copyleft (LGPL / MPL): **看情况** (需评估传染)
- 🔴 强 copyleft (GPL / AGPL): **不**接受 (违反主人 2026-08-04 拍板 "不传染")
- 🔴 商业版 (BSL / 商业 source-available): **不**接受 (违反 8 项承诺 #6 "0 依赖 NewAPI")

### Q6.4: THIRD-PARTY-NOTICES.md 怎么生成的? 我能本地重生成吗?

**A**:
- 生成: `cargo install cargo-about --version 0.8.4` + `cargo about generate --output-file THIRD-PARTY-NOTICES.md about.hbs`
- 模板: `about.hbs` (根目录, Handlebars)
- 复现: 详见 [DEPENDENCY §6](../DEPENDENCY) 复现命令

---

## 7. 进阶问题

### Q7.1: 我能把 Apeireth 静态链接到 GPL 项目吗?

**A**: ⚠️ **看 GPL 版本**:
- **GPLv2**: 静态链接 = 整个包变 GPLv2, 你的客户可要求你公开**全部**源码 (强传染)
- **GPLv3**: 静态链接 = 整个包变 GPLv3 + 你的客户可要求你提供安装信息 (per §6)
- **LGPL**: 静态链接**不**传染, 但你的客户必须可替换 Apeireth 库 (per LGPL §6)
- **AGPL**: 静态链接 + 网络服务 = 传染, **不**推荐

**实操建议**: 静态链接**不**推荐, 用**进程间通信** (HTTP / Unix socket / gRPC).

### Q7.2: 我能在多租户 SaaS 里用 Apeireth, 客户之间隔离吗?

**A**: ✅ **能** (Apache-2.0 不限制多租户). 你的隔离策略 (chroot / container / namespace) **自己**实现 + **自己**测试.

### Q7.3: 我能编译 Apeireth 到 WebAssembly (WASM) 然后嵌入网页吗?

**A**: ✅ **能** (per §1 "use" 权), 但:
- 你的网页**不**需要附 LICENSE (网页**不**算 distribution per §3)
- 你的网页 "About" / "Open source" 页可加 "本网页基于 Apeireth (Apache-2.0)" (描述性)
- 你**不**能用 "Apeireth" 商标营销 (per §6)

### Q7.4: 我能改 Apeireth 的 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) 吗?

**A**: ✅ **能** (6 哲学锚是设计哲学, 不**受** license 保护).
- 6 哲学锚**不是** "licensed material"
- 6 哲学锚是 **设计决策** (per `docs/adr/0010-6-philosophy-anchors.md`)
- 你可写你自己的 6 哲学锚 (or 5 哲学锚, or 0 哲学锚), 跟 license 无关
- 但你改了 6 哲学锚, 你的 fork **不**再是 "Apeireth", 你的产品**不**能用 "Apeireth" 名 (per §6)

### Q7.5: 主人 2026-08-04 拍板 "不依赖 NewAPI" 是什么意思?

**A**: 商业版 API 网关 (e.g. `@anthropic-ai/sdk` 商业版) **不**引入. 5 Provider (claude-code / gemini-cli / codex / copilot / opencode) **全部自建 client**, 0 引商业版 SDK. 这是 8 项不修改承诺 #6.

详见 [DEPENDENCY §5 守门](../DEPENDENCY) + `docs/adr/0004-8-promise-audit.md` §3.6.

---

## 8. 实操查询表

| 你的问题 | 查 |
|---------|-----|
| "我能 X 吗?" | [02-commercial-use.md](02-commercial-use.md) §6 (8 商业 FAQ) |
| "我改 + 怎么分发?" | [03-modification-redistribution.md](03-modification-redistribution.md) §2 (8 场景) |
| "license 术语" | https://www.apache.org/licenses/LICENSE-2.0 (官方) |
| "12 SPDX 类别" | [05-spdx-reference.md](05-spdx-reference.md) |
| "错误 / 缺失 attribution" | 提 GitHub issue, 标 D-1 |
| "想贡献" | [01-contribution.md](01-contribution.md) |
| "商标 / 品牌" | 本 FAQ §3 + [02-commercial-use.md §3](02-commercial-use.md#3-商标边界-per-6) |
| "专利" | 本 FAQ §5 + [02-commercial-use.md §4](02-commercial-use.md#4-专利边界-per-3) |

---

## 9. 相关

- 根 `LICENSE` (Apache-2.0 完整, 180 行)
- 根 `NOTICE` (项目声明 + 致谢, 71 行)
- 根 `THIRD-PARTY-NOTICES.md` (1709 行, 561 crate attribution)
- 根 `DEPENDENCY` (170 行 摘要)
- [01-contribution.md](01-contribution.md) (贡献流程)
- [02-commercial-use.md](02-commercial-use.md) (商业使用)
- [03-modification-redistribution.md](03-modification-redistribution.md) (修改 + 再分发)
- [05-spdx-reference.md](05-spdx-reference.md) (12 SPDX 类别详解)
- https://www.apache.org/licenses/LICENSE-2.0 (Apache-2.0 原文)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-5)
