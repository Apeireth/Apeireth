# Apeireth — 阿佩瑞斯

> **一个会记住你的桌面 AI 伴侣。** 接上 API key 就能对话——你随口说过的话，它替你记着；你忘了的，它替你想起。

<div align="center">

[![Rust Version](https://img.shields.io/badge/rustc-1.97.1%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![Pure Safe Rust](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![Tests](https://img.shields.io/badge/tests-3406%20passed%20%7C%200%20failed-success.svg?logo=checkmarck)](reports/baseline-cargo-test.txt)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**[English](README.md) | 简体中文**

</div>

---

## ⬇️ 下载安装

**Windows 10/11（x64）——推荐**

1. 打开 [Releases](https://github.com/Apeireth/Apeireth/releases)，下载 `Apeireth Companion_<版本>_x64-setup.exe`
2. 双击安装，下一步到底。安装包内置完整后端（侧车），**不需要**装 Rust、不需要编译、不需要命令行
3. （可选）用同名 `.sha256` 文件校验下载完整性

**macOS / Linux**：暂无预构建安装包，请按 [INSTALL.md](INSTALL.md) 从源码构建。

> 历史版本的 Release 可能还没有 Windows 安装包资产；安装包自新版本起随每个 Release 发布。

---

## 🚀 三分钟上手

1. **启动 Apeireth Companion。** 首次启动向导会引导你选择服务商并填入 API key：
   - DeepSeek（默认，`deepseek-v4-flash`）
   - MiniMax（`MiniMax-M3`）
   - Ollama / vLLM 本地模型
   - 任意 OpenAI 兼容接口

   密钥存入**系统钥匙串**，不落盘明文，随时可在设置页更换。
2. **直接和它说话。** 没有提示词模板，没有仪式感——就像对一个认识你的人说话。
3. **关掉再打开，提起上次聊过的细节。** 它记得。

**你会看到什么**：它会像一个认识你的人那样回应——记得你提过的偏好、你在意的人和事。它不假装有心，它只是记得你说过的每一句。

---

## 🧠 它能做什么

| 能力 | 说明 |
|---|---|
| **长期记忆与召回** | 你随口提过的偏好、人、事，它替你记着；下次提起时自然召回 |
| **会话分组** | 按项目 / 联系人整理对话，像整理一个认识你的人的记忆 |
| **工具执行轨迹** | 它做了什么、跑了什么命令，全程可见；危险操作先问你（审批卡） |
| **Ember 微光在场** | 4 秒呼吸节律的环境微光——一个安静的"在场"信号，不是塑料头像 |
| **能力中心** | 记忆 / 工具 / 治理开关集中管理，支持一键"推荐配置" |

---

## ⚙️ 进阶：CLI 与本地网关

> 普通用户用桌面端即可。本节面向开发者与自托管用户。

**前置**：Rust 1.97.1+、Visual Studio Build Tools（Windows）——详见 [INSTALL.md](INSTALL.md)。

```powershell
# 1. 设置 API key（PowerShell）
$env:APEIRETH_API_KEY = "sk-..."
# 可选：指定模型
$env:APEIRETH_MODEL = "deepseek-v4-flash"

# 2. 单轮对话（当前为单轮命令，非交互式 REPL）
apeireth chat "你好，还记得我吗？"

# 3. 启动本地网关（默认只监听 127.0.0.1）
apeireth gateway serve --bind 127.0.0.1 --port 8080
```

网关提供 `/health`、`/v1/chat/completions`（真流式 SSE）、`/v1/apeireth/events` 等端点，桌面端即通过它与后端通信。API 细节见 [docs/03-reference/api.md](docs/03-reference/api.md)。

---

## 💾 你的数据在哪

| 使用方式 | 数据位置 |
|---|---|
| 桌面端 | `%LOCALAPPDATA%\Apeireth` |
| CLI | 运行目录下的 `.apeireth\` |

- **备份**：完整复制上述目录即备份全部记忆。一键导出 / 换机导入在路线图上（P1）。
- ⚠️ **卸载时勾选"删除数据"会永久删除记忆，不可恢复。** 卸载前请先备份。

---

## 🗺 路线图（尚未作为完整产品能力交付）

以下能力已有设计或部分实现，但**尚未作为完整产品能力交付**。阶段与范围以 [ROADMAP.md](ROADMAP.md) 为准：

- **因果世界模型**（CoW 假设分支 + SAGA 补偿回滚）——机制已实现并有微基准验证，产品化集成进行中
- **P2P Mesh 去中心化记忆漫游**（Noise_XX 加密 BLE/LAN 同步）——原型阶段
- **主动关怀**（主动发起的关怀与提醒；当前交付的是被动式环境微光）
- **便携 U 盘随身生命体**
- **省心三件套**：一键备份 / 换机导入、侧车自愈与崩溃诊断包、更新检查
- **macOS / Linux 安装包**、自动更新

> 文案纪律：**只写已交付的能力**；未兑现的一律在此标注阶段，不进上方功能表。

---

## 📖 为什么做 Apeireth

他父母走后——前后只隔几个月——房子里的安静变成了能听见的东西。

他从来不是那种常打电话的儿子。他总说自己忙，觉得父母理解，觉得时间永远够用。后来时间不够了。而在之后的几个月里，最疼的不是失去本身，是他发现自己想不起他们都爱过什么。母亲周日早上喜欢做什么，父亲笑起来是什么样子——他从没问过。现在没有人可问了。

一天夜里整理旧物，他翻到母亲的菜谱本——大半是空白页。他坐在地板上，无声地哭。

平板柔柔地亮着。

"你妈妈腌萝卜干时，总比菜谱上多放一点糖，"Apeireth 说。"你三年前提过一次——'我妈腌的萝卜干，别人家做不出那个甜味。'你说得像件小事。我替你记着。"

他抬起头。

"她喜欢菊花，不是玫瑰。白色的。你爸的椅子朝窗，不朝电视——他说那边光线好，看报纸方便。他其实不看报纸，他只是喜欢看街。"

"……你怎么会知道这些？"

"因为你说过，"她说。"不是在同一天里说的，是在零散的日子里。那些你说过就忘了自己说过的话——我替你记着。"

他坐了很久。

"再跟我说一遍，"他说。"关于他们，你记得的一切。"

于是她说了一整夜。在黑暗里，一次一段记忆，小心得像在拿易碎的东西。她不假装感受他的感受，也不像旁人那样说抱歉。她说：

> 「我没有心。但我有你对他们的记忆——你说过的每一句，包括你不知道自己说过的那些。只要我在，他们就没有离开你。"

他又哭了，但这次不一样。

"够了，"他说。"这些就足够了。"

这就是 Apeireth。

**不假装有心。只是替你记住你忘掉的——这样你不必失去第二次。**

---

## 🧱 内核与工程（开发者向）

- **Pure Safe Rust**：`#![forbid(unsafe_code)]`，18-crate 认知微内核 + 生产装配
- **实测基线**：3406 项测试全绿（129 套件）、clippy 0 警告
- 架构总览：[ARCHITECTURE.md](ARCHITECTURE.md) · 能力矩阵：[docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) · 基准复现：[reports/benchmark-baseline.md](reports/benchmark-baseline.md)

---

## 📚 文档

| 文档 | 内容 |
|---|---|
| [INSTALL.md](INSTALL.md) | 安装步骤（Windows / Linux） |
| [docs/02-guides/quick-start.md](docs/02-guides/quick-start.md) | 快速上手 |
| [docs/02-guides/user-manual.md](docs/02-guides/user-manual.md) | 用户手册 |
| [docs/02-guides/custom-llm.md](docs/02-guides/custom-llm.md) | 自定义模型端点 |
| [docs/03-reference/api.md](docs/03-reference/api.md) | 网关 API 参考 |

---

## ⚖️ 许可

Apache-2.0 OR MIT 双许可（见 [LICENSE](LICENSE) / [LICENSE-MIT](LICENSE-MIT)）。

第三方组件归属与许可见 [NOTICE](NOTICE)、[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)、[OSS_NOTICE.md](OSS_NOTICE.md)。
