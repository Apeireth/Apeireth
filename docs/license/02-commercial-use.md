# 商业使用 Apeireth (Commercial Use)

> **性质**: 商业使用 Apeireth 的常见场景 + 法律边界
> **依据**: Apache-2.0 §1-3 (商用权) + §4 (再分发) + §6 (商标)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-5)

---

## 0. TL;DR

| 场景 | 允许? | 条件 |
|------|:----:|------|
| **SaaS 部署 Apeireth** | ✅ | 保留版权 + NOTICE, 不用"Apeireth"商标暗示官方 |
| **嵌入 Apeireth 到商业产品** | ✅ | 保留版权 + LICENSE + NOTICE, 改动的源码可选择公开 |
| **修改 Apeireth + 卖服务** | ✅ | 同上, 你的修改 = 你的版权 |
| **用 Apeireth 训练竞品 AI 模型** | ⚠️ | 不允许用 Apeireth 商标 / 名称, 训练权归你 |
| **用 "Apeireth" 名 + logo 做营销** | ❌ | 商标侵权 (per §6) |
| **用 Apeireth 提供付费 API 转发** | ✅ | 保留版权 + NOTICE + 你的 SLA |
| **OEM Apeireth 到硬件** | ✅ | 跟 SaaS 一样条件 |
| **分叉 (fork) Apeireth 改名** | ✅ | 改名前不能用 "Apeireth", 改后用你新名 |

---

## 1. Apache-2.0 商用权 (核心)

### 1.1 你拿到什么 (per §1)

> Subject to the terms and conditions of this License, each Contributor hereby grants You a **perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable** copyright license to **reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute** the Work and such Derivative Works in Source or Object form.

**翻译**:
- **永久** (perpetual) — 不收回
- **全球** (worldwide) — 任何司法管辖区
- **非独占** (non-exclusive) — 项目可同时授别人
- **免费** (no-charge, royalty-free) — 0 授权费
- **不可撤销** (irrevocable) — 一旦授予, 不可收回
- **可再许可** (sublicense) — 你可授你的客户
- **可分发** (distribute) — 卖 / SaaS / OEM 都可
- **可修改** (prepare Derivative Works) — fork / patch / 嵌入 都可

### 1.2 你必须做什么 (per §4)

| 条件 | 怎么做 | 严格度 |
|------|--------|:----:|
| **保留版权声明** | 在 `LICENSE` + 你分发的每个文件保留原 `Copyright 2026 Apeireth Team` | 🔴 必 |
| **附 LICENSE 副本** | 你分发的包里放完整 `LICENSE` (180 行) | 🔴 必 |
| **附 NOTICE** | 你分发的包里放完整 `NOTICE` (71 行) | 🔴 必 |
| **改动说明** | 你改动的文件加 `Modified by <你> on <日期>` (per §4(b)) | 🔴 必 |
| **源可见** (Source form) | 如果你分发二进制, 必须让第三方能拿到对应源代码 (per §3) | 🟡 看你分发方式 |
| **不假背书** | 你的产品不能假借 "Apeireth Team 认证" 之类 (per §6) | 🔴 必 |

---

## 2. 4 商业场景详解

### 2.1 SaaS 部署 (最常见)

**场景**: 你在 AWS 上跑 Apeireth, 按月收客户订阅费.

**许可要点**:
- ✅ **允许** (Apache-2.0 §1 明确授予 "publicly perform" + "distribute")
- ✅ 你**不需要公开**你的 SaaS 源码 (per AGPL 但 **不** per Apache-2.0)
- ✅ 你**不需要**给客户分发 Apeireth 二进制 (SaaS 不算 "distribution" per §3)
- ⚠️ 你**必须**在服务条款 / About 页保留 "Powered by Apeireth" + 链接到 `LICENSE` (per §4)

**实操清单**:
- [ ] About 页加 "Powered by Apeireth" + 链接 `https://github.com/apeireth/apeireth-rust`
- [ ] 服务条款引 "本服务使用 Apache-2.0 许可的 Apeireth (https://www.apache.org/licenses/LICENSE-2.0)"
- [ ] 你的 SLA / 隐私政策 跟 Apeireth 无关 (你独立负责)
- [ ] 0 用 "Apeireth" 名 + logo 做营销 (per §6 商标边界)

### 2.2 嵌入商业产品

**场景**: 你做 IDE, 把 Apeireth 嵌进去当 AI 助手.

**许可要点**:
- ✅ **允许** (per §1 "prepare Derivative Works")
- ✅ 你的产品**可商用**卖
- ⚠️ 你**必须**给客户**附**完整 `LICENSE` + `NOTICE` (per §4(a))
- ⚠️ 你**必须**在"Third-party software"页列 Apeireth + license = Apache-2.0
- 🟡 你的 IDE 源码**不**强制公开 (Apache-2.0 不传染)

**实操清单**:
- [ ] 产品包内附 `THIRD-PARTY-NOTICES.md` (1709 行, Apeireth 全 attribution)
- [ ] 安装目录的 `licenses/` 文件夹放 Apeireth 的 `LICENSE` + `NOTICE`
- [ ] "关于"页列 "本产品包含 Apeireth (Apache-2.0)"

### 2.3 修改 + 卖服务

**场景**: 你 fork Apeireth, 改 100 处, 部署卖客户.

**许可要点**:
- ✅ **允许** (per §1)
- ✅ 你的修改 = 你的版权
- ✅ 你可改用你公司名 + 你的 license (但**不**能禁止 Apeireth 自己的 Apache-2.0)
- ⚠️ 你的客户**必须**能拿到**你修改后的源码** (per §3 Source form)
- ⚠️ 你的客户**必须**能拿到**你修改后的 NOTICE** (per §4)
- ⚠️ 你**不能**用 "Apeireth" 名 (per §6)

**实操清单**:
- [ ] 你的源码仓库公开, 或在客户签约时给客户访问权
- [ ] 你修改的文件加 `Modified by YourCompany on 2026-xx-xx`
- [ ] 你的 `LICENSE` 写 "本产品修改自 Apeireth (Apache-2.0), 修改部分 Copyright 2026 YourCompany"
- [ ] 0 用 "Apeireth" 名, 改用你品牌名

### 2.4 OEM 到硬件

**场景**: 你做路由器, 装 Apeireth 跑本地 AI.

**许可要点**:
- 跟 **2.2 嵌入商业产品** 完全一样
- 额外: 设备的 "About" / "Legal" 页加 "本设备包含 Apeireth (Apache-2.0)"
- 额外: 设备的用户手册附完整 `LICENSE` (或链接到 web)

---

## 3. 商标边界 (per §6)

### 3.1 Apache-2.0 §6 原文

> Unless required by applicable law (such as faithful and necessary reproduction of a NOTICE file in the Work or Derivative Works), **you may not use the trademarks, service marks, or product names of the Licensor** except as required to describe the origin of the Work.

**翻译**: 除非必要描述作品来源, **你不能用许可方的商标 / 服务标记 / 产品名**.

### 3.2 Apeireth 商标

| 商标 | 拥有者 | 允许用 | 不允许用 |
|------|--------|--------|----------|
| **"Apeireth" 名称** | Apeireth Team | 描述来源 (e.g. "基于 Apeireth 修改") | 暗示你 = 官方 |
| **Apeireth logo** (如有) | Apeireth Team | 同上 | 营销 / 包装 / 域名 apeireth-yours.com |
| **"apeireth-rust"** | 社区 | 描述来源 | 注册成你的商标 |
| **Apeireth 标语** | Apeireth Team | 描述 | 改写 / 翻译当你的 |

### 3.3 边界举例

| 用法 | 允许? | 备注 |
|------|:----:|------|
| "本产品基于 Apeireth" | ✅ | 描述来源 |
| "Apeireth 兼容" | ✅ | 描述兼容 |
| "我们 = Apeireth Team" | ❌ | 假背书 |
| "Apeireth 企业版" | ❌ | 暗示官方 |
| 域名 `apeireth-enterprise.com` | ❌ | 商标侵权 |
| 域名 `mycompany.io/apeireth-integration` | ✅ | 描述 |
| 用 Apeireth logo 当你产品 logo | ❌ | 商标侵权 |

---

## 4. 专利边界 (per §3)

### 4.1 Apache-2.0 §3 专利授权

> Each Contributor hereby grants You a **patent license** to make, have made, use, offer to sell, sell, import, and otherwise transfer the Work.

**翻译**: 每个贡献者授予你**专利许可**, 让你**制造 / 使用 / 卖 / 进口** 这份作品.

### 4.2 专利反诉 (per §3(b))

> If You institute **patent litigation** against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that the Work constitutes direct or contributory patent infringement, then any patent licenses granted to You under this License for that Work **shall terminate**.

**翻译**: 你**起诉** Apeireth 专利侵权, 你**自己的**专利许可**立刻终止** (核弹条款).

**实操**:
- 你**可以**用 Apeireth 卖产品, 享受 §3 专利授权
- 你**不可以**反过来告 Apeireth 专利侵权, 否则你的授权就**没了**

### 4.3 实际案例

- 2014 Apache-2.0 vs 专利诉讼: 大公司通常**不**主动起诉 Apache-2.0 项目 (怕 §3(b) 终止自己的授权)
- 实际案例**很少** (Apache-2.0 §3(b) 是非常强的"互不侵犯条约")

---

## 5. 责任边界 (per §7-8)

### 5.1 无担保 (per §7)

> Unless required by applicable law or agreed to in writing, Licensor provides the Work on an **"AS IS" BASIS, WITHOUT WARRANTIES OF CONDITIONS OF ANY KIND**, either express or implied...

**翻译**: 项目**不提供任何担保** (适销性 / 特定用途 / 不侵权 都不担保).

### 5.2 责任限制 (per §8)

> In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise, unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing, shall any Contributor be liable to You for damages...

**翻译**: 贡献者**不对你**的损失负责 (除蓄意 / 重大过失).

**实操**:
- 你**自己**负商业责任 (SLA / 备份 / 灾备)
- Apeireth 团队**不**为你的停机 / 数据丢失 负责
- 你**自己**买保险 (cyber insurance / E&O insurance)

---

## 6. 常见商业 FAQ

### Q1: 我需要给 Apeireth Team 钱吗?

**❌ 不需要** (per §1 "no-charge, royalty-free"). Apache-2.0 **明确免费**.

### Q2: 我能卖 Apeireth 相关服务吗?

**✅ 能**, 但:
- 0 用 "Apeireth" 商标 (per §6)
- 你的服务**独立 SLA**, Apeireth Team 不背书
- 你可卖 "Apeireth 部署 / 培训 / 运维" 服务

### Q3: 我用 Apeireth 跑 AI 助手, 客户告我 AI 输出侵权, 谁负责?

**你负责** (per §7-8 Apeireth 团队不担责). 你应:
- 买 E&O 保险
- 服务条款加 indemnification
- 客户合同明确 AI 输出无担保

### Q4: 我能改 Apeireth 闭源吗?

**✅ 能** (Apache-2.0 **不**传染, 不像 GPL).
- 你的修改 = 你的版权
- 你的客户**必须**能拿到你修改后的源码 (per §3 Source form)
- 但你**不**需要公开你的整个产品源码

### Q5: 我能跟 GPL 项目静态链接 Apeireth 吗?

**⚠️ 看情况**:
- LGPL 项目: **可**, 但遵守 LGPL 条款 (允许动态替换)
- GPL 项目: **可** (GPL 兼容 Apache-2.0 双向)
- AGPL 项目: **可**, 但你的网络服务**也**适用 AGPL (传染)

**实操**: 建议**不**静态链接, 用**进程间通信** (HTTP / Unix socket / gRPC).

### Q6: 我能用 Apeireth 训练竞品 AI 模型吗?

**✅ 能** (Apache-2.0 §1 授予 "use" 权), 但:
- 你的训练数据 = 你的版权
- 你的模型 = 你的版权
- 0 用 "Apeireth" 名
- 0 反向工程 Apeireth 私有算法 (per 商业秘密法, **不** per Apache-2.0)

### Q7: 我能在公司内网部署吗?

**✅ 能**, 跟 SaaS 一样, 不需要分发源码.

### Q8: 我能把 Apeireth 卖给军方 / 政府吗?

**✅ 能** (Apache-2.0 跟政治无关), 但:
- 你**自己**符合当地出口管制法 (EAR / ITAR)
- 你**自己**负合规责任

---

## 7. 实操清单 (商业部署)

| # | 任务 | 工具 | 必须? |
|---|------|------|:----:|
| 1 | About 页加 "Powered by Apeireth" + 链接 | 你产品 UI | ✅ |
| 2 | 服务条款引 Apache-2.0 | 律师 | ✅ |
| 3 | 附完整 `LICENSE` (180 行) | 安装包 | ✅ |
| 4 | 附完整 `NOTICE` (71 行) | 安装包 | ✅ |
| 5 | 附 `THIRD-PARTY-NOTICES.md` (1709 行) | 安装包 | ✅ |
| 6 | 0 用 "Apeireth" 商标做营销 | 品牌策略 | ✅ |
| 7 | 你修改的文件加改动说明 | git | ✅ |
| 8 | 你修改的源码可获取 (Source form) | git / 客户 portal | ✅ |
| 9 | 买 E&O 保险 | 保险公司 | 🟡 推荐 |
| 10 | 独立 SLA (不依赖 Apeireth Team) | 你公司 | ✅ |

---

## 8. 相关

- 根 `LICENSE` (Apache-2.0 完整, 180 行)
- 根 `NOTICE` (项目声明 + 致谢, 71 行)
- 根 `THIRD-PARTY-NOTICES.md` (1709 行, 561 crate attribution)
- [01-contribution.md](01-contribution.md) (贡献流程)
- [03-modification-redistribution.md](03-modification-redistribution.md) (修改 + 再分发)
- [04-faq.md](04-faq.md) (18 常见问题)
- https://www.apache.org/licenses/LICENSE-2.0 (Apache-2.0 原文)
- https://www.apache.org/foundation/marks/ (Apache 商标政策)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-5)
