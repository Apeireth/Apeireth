# Apeireth — 阿佩瑞斯

<div align="center">

<img src="previews/apeireth-mark.svg" alt="Apeireth 双层菱形标志" width="72">

### 会成长的数字伙伴

**开源免费的个人 AI 助手，让对话、记忆和行动连起来。**

[![Status](https://img.shields.io/badge/status-pre--release-orange.svg)](ROADMAP.md)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

<a id="nav"></a>

**[当前能力](#features) · [如何开始](#start) · [能力与隐私边界](#boundaries) · [社区](#sources) · [工程与验证](#hood) · [项目愿景](#acts)**

> [!IMPORTANT]
> **当前处于开发与预发布阶段，新版桌面前端仍在开发。** 本仓库目前主要提供后端、命令行和集成接口；后端具备某项能力，不代表新版桌面界面已经提供对应体验。
>
> [Releases](https://github.com/Apeireth/Apeireth/releases) 中有历史预发布安装包，它们对应各自的标签和日期，不能代表当前 `main` 或正在开发的新前端。请核对发布说明、平台要求与校验文件。

Apeireth 希望让一次对话能延续成可追溯的记忆，再在明确的权限范围内变成行动。你可以了解它如何工作、检查保存的内容，并参与改进。软件本身开源免费；使用云模型时，服务商可能另行收费。

<a id="features"></a>

## 当前可以从后端做什么

| 你想做的事 | 当前能力 | 使用前需要知道 |
|---|---|---|
| 把对话继续下去 | 模型适配、上下文管理、工具调用循环；HTTP/SSE 聊天支持增量输出 | 需要配置模型服务；不支持流式的适配器会回退为完成后输出 |
| 留住资料与偏好 | 记忆写入、混合检索、偏好召回，以及可检查、可纠正的记忆记录 | 记忆和模型推断都可能出错，重要信息仍需核对 |
| 整理和处理文件 | 本地文件读取、搜索、仓库工具，以及文档工具中的 Markdown → DOCX 导出等能力 | 文档工具与写入能力需要配置；执行仍受工作区和治理策略约束 |
| 让操作可以检查 | 风险判定、审批流程、执行记录与审计 | 能力开关、授权和宿主配置共同决定可执行范围，不是所有工具默认开启 |
| 扩展工作方式 | MCP 桥接、任务板等可选后端能力 | 默认关闭，需要按能力说明启用；不等于已经有对应的新版图形界面 |

完整状态与证据见 [能力矩阵](docs/03-reference/capabilities-matrix.md)、[可核验能力声明](docs/status/STATUS.md)；API 集成见 [网关接入说明](docs/02-guides/v2-gateway-frontend-integration-spec.md)。这些入口区分代码存在、生产装配、默认开启与实机验证，不把它们混为一谈。

<a id="start"></a>

## 如何开始

### 想使用 Apeireth

- 先看 [版本发布页](https://github.com/Apeireth/Apeireth/releases)，确认是否有适合你的平台和阶段的资产。新版前端尚未完成，当前还不是“下载后即可获得官网全部演示体验”的状态。
- 安装或配置有疑问，到 [使用问答](https://github.com/Apeireth/Apeireth/discussions/categories/q-a) 提问。不懂代码也可以参与。
- 使用云模型前，了解 [数据与隐私](docs/02-guides/data-and-privacy.md) 以及服务商的收费和数据政策。

### 想开发或集成

准备 Git、仓库锁定的 Rust 工具链，以及平台所需的编译工具；详见 [源码构建指南](INSTALL.md) 和 [rust-toolchain.toml](rust-toolchain.toml)。从命令行帮助开始：

```bash
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth
cargo run --locked -p apeireth-cli --bin apeireth -- --help
```

该命令会先编译 CLI，再显示帮助；帮助路径不启动模型任务，也不需要 API key。它不是完整产品安装或端到端验证。真正发起对话时，还需要按 [配置说明](INSTALL.md) 设置模型服务与凭据。不要将密钥写进 Issue、截图或提交记录。

参与代码、文档或素材改进，请看 [CONTRIBUTING.md](CONTRIBUTING.md)。版本与 workspace 成员以根 [Cargo.toml](Cargo.toml) 为准。

<a id="boundaries"></a>

## 能力与隐私边界

- **本地运行不等于数据不会离开电脑。** 使用云模型时，相关上下文会发送给你配置的服务商。模型上下文出网闸已有生产装配，但默认关闭（`APEIRETH_ENABLE_EGRESS_GATE=1` 才启用）；它也不能替服务商承诺删除已接收的数据。详见 [数据与隐私](docs/02-guides/data-and-privacy.md)。
- **“会记得”不等于“不会错”。** 检索、提炼和模型回答都有适用范围；证据与历史记录帮助你复核，不构成永久、完整或正确记忆的保证。
- **治理与隔离有明确范围。** 审批、工具白名单与平台隔离用于限制操作，不能承诺软件绝无风险。不同平台和工具的边界见 [安全模型](docs/01-architecture/security.md) 与 [能力矩阵](docs/03-reference/capabilities-matrix.md)。
- **预算需要配置。** 会话的 token、工具步数与费用配额默认不设上限，需要主动配置；费用控制还依赖模型定价信息，不能替代服务商的账单与额度管理。
- **默认开启和可选能力分开。** 记忆召回、偏好等基础能力与 shell、fetch、文件写入、MCP 等可选能力使用不同开关。配置入口和默认值统一查 [能力矩阵 §0.2](docs/03-reference/capabilities-matrix.md)，不要一次打开所有开关来试用。
- **模型适配不是所有协议都已接通。** 当前 provider 适配族为 OpenAI-compatible、MiniMax、Anthropic；Gemini 和 OpenAI Responses 的协议归一器不等于可直接使用的 provider。
- **仍有未完成的接入。** WebSocket 全双工服务端与第三层 DSL 洋葱规则尚未完成生产接入；SDK 的 HTTP 工具调用可用，但 WebSocket 流式传输和配额查询仍返回未实现错误。HTTP/SSE 与这些能力是不同路径；Mesh 的本地 worker 也不能视为带加密与发现机制的公网组网产品。

需要查看具体证据、开关和限制时，以 [能力声明](docs/status/claims.yaml) 与当前源码为准。设计稿、历史交付记录、单元测试和形式化模型证明，都不自动等于当前完整产品已通过实机验收。

<a id="sources"></a>

## 社区与官方入口

无论是第一次提问，还是第一次贡献，都欢迎。完整分流见 [支持指南](SUPPORT.md)，参与时请遵守 [行为准则](CODE_OF_CONDUCT.md)。

| 想做什么 | 入口 |
|---|---|
| 使用求助、配置疑问 | [Q&A](https://github.com/Apeireth/Apeireth/discussions/categories/q-a) |
| 交流想法、分享体验 | [Ideas](https://github.com/Apeireth/Apeireth/discussions/categories/ideas) · [Show and tell](https://github.com/Apeireth/Apeireth/discussions/categories/show-and-tell) |
| 报告缺陷、提交明确的功能请求 | [问题与建议表单](https://github.com/Apeireth/Apeireth/issues/new/choose) |
| 修改代码、文档或素材 | [贡献指南](CONTRIBUTING.md) |
| 报告产品安全漏洞 | [GitHub 私密漏洞报告](https://github.com/Apeireth/Apeireth/security/advisories/new) · [安全政策](SECURITY.md) |
| 行为举报与申诉 | [yintasde@qq.com](mailto:yintasde@qq.com) · [行为准则中的渠道与处理安排](CODE_OF_CONDUCT.md#报告与申诉渠道) |

参与 GitHub 问答和反馈需要账号。公开内容请先移除密钥、私人对话和身份信息；不要在公开 Issue 中发布漏洞或敏感举报材料。

发布资产以 [Apeireth/Apeireth Releases](https://github.com/Apeireth/Apeireth/releases) 为准，校验信息以对应版本提供的文件为准。项目维护者不会要求你在公开反馈或私人支持对话中交出 API key。官网设计预览和历史截图都不是发布渠道。

<a id="hood"></a>

## 工程与验证

同一套后端内核承载对话、记忆、治理和工具执行，命令行与网关提供外部入口。主要用 Rust 编写；平台隔离涉及操作系统接口和受限的 `unsafe`/FFI 实现，因此不把“全仓零 unsafe”作为安全保证。

| 层 | 阅读入口 |
|---|---|
| 总体结构与设计原则 | [架构总览](docs/01-architecture/architecture.md) · [哲学与约束](docs/01-architecture/philosophy.md) |
| 运行时、记忆、模型与治理 | [核心机制讲解](docs/02-guides/core-mechanisms-explained.md) · [crate 索引](docs/03-reference/crates.md) |
| 装配状态、默认值与证据 | [能力矩阵](docs/03-reference/capabilities-matrix.md) · [状态声明](docs/status/STATUS.md) |
| 接下来的工作 | [ROADMAP.md](ROADMAP.md) · [CHANGELOG.md](CHANGELOG.md) |

<a id="trust"></a>

### 可复核的证据

[![Tests](https://img.shields.io/badge/tests-8846%20passed%20%7C%201%20failed%20(2026--10--31)-critical.svg?logo=checkmarx)](docs/03-reference/test-baseline.md)

这枚徽章对应 **2026-10-31 已记录的全量测试基线**，包含失败项；不是当前提交刚刚重跑的结果，也不代表桌面前端或真实模型任务已经端到端通过。

- [测试基线与复现方法](docs/03-reference/test-baseline.md)：命令、工具链、失败说明与历史记录。
- [按改动范围验证](docs/02-guides/fast-feedback-gate.md)：日常迭代的检查入口；全量收口使用 `pwsh scripts/gate_full.ps1 -FailIfBusy`，不承诺几分钟跑完。
- [形式化验证资料](research/verification/)：具体模型、命题与假设；不能从一个命题推导全部操作都安全。
- [历史性能复测](reports/benchmark-reproduction.md)：保留当时的方法和结果，不当作当前设备或当前提交的性能保证。

一次成功聊天不能证明长期记忆正确；一张界面截图也不能证明后端动作已执行。复核时应看相应能力的输入、执行记录、输出和验证范围。

<a id="voice"></a>
<a id="acts"></a>

## 为什么是“数字伙伴”

我们希望 Apeireth 不只回答当前一句话，也能让重要的上下文延续下去：记住你允许它保留的事，在需要行动时说清楚会做什么，让结果有来处、可以检查。

<details>
<summary>愿景三幕：记得、安静、守住边界</summary>

以下是创作与设计愿景，**不是当前功能清单或实机演示**。温牛奶、照顾植物、感知心率等场景不构成已经交付的产品承诺。

### 第一幕：记得零零碎碎的日子

半勺蜂蜜，一盆你曾提起的绿萝，一件没有说完的小事。我们希望“记忆”能让后来的对话少一些重复，让那些重要却零散的背景有机会被重新找到。

### 第二幕：忙的时候不吵你

不是每个时刻都需要一句回应。理想中的伙伴知道何时提供帮助，也尊重安静、专注和拒绝；它不必假装拥有人的内心。

### 第三幕：把决定留给你

能做事，也要说明权限、理由与代价。一个建议成为真实行动之前，应有清楚的边界；失败或不确定的地方，应如实呈现。

愿景作品：[《阿佩瑞斯》——记得一个人](docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt) · [《遗声》——记得一个文明](docs/vision/遗声-未来愿景小说2.txt)。

</details>

<a id="manifesto"></a>

**我们的方向**：把“记得”做成可检查、可纠正的能力，把行动放在明确的治理之下。北极星指向远处，实事求是落在每一次交付里。完整设计见 [创世设计](docs/01-architecture/founding-design-v2.md) 与 [项目哲学](docs/01-architecture/philosophy.md)。

旧版 [桌面窗口截图](previews/mode-companion-v2.png) 与 [开场动画画面](previews/intro-live-flight.png) 保留作设计历史，不代表正在开发的新前端。标志使用与官网一致的双层菱形。

<a id="documentation-map"></a>

## 文档地图

[获取帮助](SUPPORT.md) · [构建环境](INSTALL.md) · [能力矩阵](docs/03-reference/capabilities-matrix.md) · [数据与隐私](docs/02-guides/data-and-privacy.md) · [贡献指南](CONTRIBUTING.md) · [全部文档](docs/README.md)

## 开源协议

Apache-2.0 OR MIT 双许可。许可证见 [LICENSE](LICENSE)，来源与署名见 [NOTICE](NOTICE)。

发现文档和实际行为不一致，欢迎通过 [问题表单](https://github.com/Apeireth/Apeireth/issues/new/choose) 告诉我们。

**[回到开头](#nav)**
