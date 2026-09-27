# Apeireth — 阿佩瑞斯

> **Apeireth —— 憧憬 AGI 的未来。** 在那之前，先做一个真正记得你的桌面伙伴。

<div align="center">

[![Rust Version](https://img.shields.io/badge/rustc-1.97.1%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![Pure Safe Rust](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![Tests](https://img.shields.io/badge/tests-4638%20passed%20%7C%200%20failed-success.svg?logo=checkmarx)](docs/04-internal/absorption-ledger.md)
[![Clippy](https://img.shields.io/badge/clippy-0%20warnings-brightgreen.svg?logo=rust)](crates)
[![Kani](https://img.shields.io/badge/proof-Kani%20%2B%20TLA%2B%20required%20check-blueviolet.svg)](research/verification)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

---

## 一、这是什么（30 秒）

**Apeireth 是一个住在你桌面上的 AI 伙伴。** 它不是又一个聊天窗口——它**记得**你（几个月前随口提过的事）、**管得住自己**（危险动作要经你批准）、**会长大**（性格从经历里长出来），而且这一切**跑在你自己的机器上**。

四件它真的能做的事：

| | 场景 | 它凭什么做到 |
|---|---|---|
| 🧠 | **记得你**："上次你说想给妈妈腌萝卜干的那个坛子，找到了吗？" | 双时态记忆 + 词法/语义混合检索 + 会呼吸的遗忘曲线 |
| 🔒 | **管得住**：它想删文件、发消息、动配置？先过审批门 | R0-R4 风险矩阵 + 机器证明的审批状态机 |
| 🌱 | **会长大**：聊得越久，语气、偏好、相处方式越像"你们的" | 性格养成引擎（默认开的学习 + 透明可撤销） |
| 🛡 | **靠得住**：历史不可篡改、错误不静默、数字都带脚本 | 追加写死的日志 + 全量测试 + 五分钟可复核 |

**诚实边界**（先说丑话）：它不假装拥有灵魂——它是一个设计精良的智能机器，真诚地扮演伙伴的角色；模型是外接的（DeepSeek / OpenAI 兼容 / 本地皆可），我们的功夫全在**模型外面那圈记忆与信任的机器**。

---

## 二、快速上手

### 安装

从 [Releases](../../releases) 下载 Windows 安装包（NSIS，附 SHA256），或从源码构建：

```bash
# 源码构建（Rust 1.97.1+）
cargo build --release -p apeireth-cli          # 后端
cd frontend/companion-desktop && pnpm install && pnpm tauri build   # 桌面壳
```

### 首次运行

1. 双击桌面图标 → 首启向导（选服务商 → 填密钥 → 开聊）；
2. 密钥存入**系统钥匙串**（不落盘明文，重启自动恢复）；
3. 想调性格？设置页「性格与记忆」：4 个滑杆 + 三档预设（省心 / 均衡 / 深度记忆）。

### 先验货再信任？

直接跑 [五分钟独立验证](#五、验证——不信宣传信命令)——三条命令复核所有核心宣称。

---

## 三、它为什么不一样（给懂一点技术的你）

传统 AI 助手 = 大模型 + 一层提示词。Apeireth = 大模型 + **一整套操作系统级的记忆与信任机器**。18 个 crate 的认知微内核（纯 Safe Rust、零 unsafe），四根支柱：

### 🧠 记忆系统
- **双时态事实流**：每条记忆带"发生时刻"与"记录时刻"——能问"去年三月我们以为的 X 是什么"；
- **追加写死**：历史不可改写（数据库触发器级），Merkle 哈希链防篡改；
- **五层混合检索**：词法 + 语义 + 融合 + 激活度 + 三层渐进披露（不是"向量检索"一层皮）；
- **会淡但不会错**：遗忘按记忆曲线衰减、保护的记忆永不忘（TLA+ 穷尽证明）、纠错走撤回不删历史。

### ⚖️ 治理系统
- **R0-R4 风险矩阵**：不可逆动作必过人工审批，治理规则的修改本身是最高风险级；
- **三洋葱**：原则、权限、行为三层独立把关——价值观正确 ≠ 有权动手；
- **审批状态机**："批准只产生一次副作用"等三条性质经 TLA+ 模型检验 + **运行时在线守卫**（不变量注册表逐事件检查）。

### 🗂 调度与上下文
- **多维配额**（Token/步数/花费/深度）：agent 烧钱跑飞在结构上不可能；
- **压缩检查点**：长对话超窗不再硬截断——摘要替换 + 原文留档 + 确定性可回放，切点永不劈开工具调用对；
- **溢出落盘取回**：超长内容保留头尾预览 + 全文落盘 + 取回指引（宁长勿丢）；
- **溢出自愈**：上下文超限自动降预算重组重发，带进展守卫。

### 🛠 工具与执行
- **五段执行流水线**：策略瀑布 → 单调守卫（拒绝不可翻转）→ 超时/重试 → 纠错通道 → 输出归一；
- **读前观测门禁**：没读过的文件不许覆盖写（版本 CAS 双钥匙）；
- **沙箱升级阶梯**：越权请求需理由、批准只管一次、拒绝时就地引导；
- **统一原子写**：完整性/持久两档 + 跨进程锁，坏配置**拒开**而非静默回退。

> 想看全部 23+ 项机制的来龙去脉？见 [质感吸收台账](docs/04-internal/absorption-ledger.md)。

---

## 四、宣言（灵魂在这儿）

**Apeireth —— 憧憬 AGI 的未来。**

我们不知道通用人工智能何时到来，但我们知道它不该长成"更大的聊天框"。它应该**记得一生**、**守得住规矩**、**经得起复核**。所以我们先把地基打了：

- **日志是唯一权威**——可变状态皆为派生视图，回放永远确定；
- **失败也是帧**——错误走正式通道、词表穷尽，没有半态；
- **单调性即安全**——守卫只许拒绝不许放行，顺序不可逆；
- **损坏可检出优于静默降级**——坏数据拒开或跳过并计数，绝不悄悄用默认。

以及那条贯穿一切的旧誓言：**不假装。** 不假装有灵魂、不假装测试过了、不假装数字是真的——做不到的写"做不到"，未接线的挂"未接线"。

---

## 五、验证（不信宣传信命令）

```bash
# 1. 测试是真的：全工作区回归
cargo test --workspace --locked
# 预期：全部套件绿、0 失败（当前基线 4638 passed / 147 套件）

# 2. 工程红线：纯 Safe Rust + 零警告
cargo clippy --workspace --all-targets --locked -- -D warnings

# 3. LLM 接线是真的（任一支持的服务商 key 即可）
cargo run -p apeireth-cli -- chat "你好，还记得我吗？"
```

### 性能数字（每行带脚本，无脚本不发布）

一键复跑：`pwsh -NoProfile -File scripts/run-benchmarks.ps1`（方法学与两轮原始数据见 [复测报告](reports/benchmark-reproduction.md)）。

| 指标 | 目标 | 实测 P50 | 状态 |
|---|---|---|---|
| 混合记忆检索（1 万节点） | < 10 ms | 3.65 ms | ✅ |
| 认知配额调度 | < 50 µs | 0.81 µs | ✅ |
| 会话事件折叠（千事件） | — | 289 µs | ✅ 记录行 |
| OS 沙箱进程启动 | < 15 ms | 12.8 ms | ✅ |
| 微内核冷启动（真冷） | < 10 ms | 22.0 ms | ❌ 差距 2.2 倍（如实标注） |
| 待机内存 | < 35 MB | 17.5 MB | ✅ |

> 未达标就是未达标——冷启动的剩余差距是真冷口径下的文件创建系统开销，我们不靠改口径粉饰。

### 更多可审计证据

- **机器证明**：25 条 Kani 命题 + TLA/TLC 模型实跑（`kani` 已设为合并必需检查）：[research/verification](research/verification/)
- **逐条读码盘点**：[代码实况](reports/code-implementation-audit-2026-09-26.md)
- **宣称-证据矩阵**（每条对外宣称带状态标）：[claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md)
- **架构标尺**：[founding-design-v2](docs/01-architecture/founding-design-v2.md)

---

## 目录导览

| 想看什么 | 去哪儿 |
|---|---|
| 架构总览 | [docs/01-architecture](docs/01-architecture/) |
| 能力矩阵 | [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) |
| 核心机制讲解（给指导者） | [core-mechanisms-explained](docs/02-guides/core-mechanisms-explained.md) |
| 安装指南 | [INSTALL.md](INSTALL.md) |
| 机制吸收台账 | [absorption-ledger](docs/04-internal/absorption-ledger.md) |
| 贡献与规范 | [CONTRIBUTING.md](CONTRIBUTING.md) |

## 开源协议

Apache-2.0 OR MIT 双许可。来源沿革以 git 历史与 docs 归档为准（见 [NOTICE](NOTICE)）。
