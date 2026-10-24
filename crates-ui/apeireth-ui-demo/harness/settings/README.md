# 设置页三页 · 切分方案（**只做切分与骨架，不做迁移**）

> **本目录的定位**：把 `SettingsView.svelte` 的 **3,775 标记行**（script 1,920 +
> markup 1,854）切成 **6 个可独立验收的子面**，并为每一面给出
> **边界 / 依赖 / 对照方式 / 可信度 / 风险**；再把同面的两个独立文件
> （`FirstRunWizard.svelte`、`ThemeSettingsPanel.svelte`）登记为**不需要切分**的
> 独立面。
>
> **本批不做**：一行 Rust 迁移。本目录只有方案、fixture 草案、空骨架。
>
> **本目录是独占的**：`crates-ui/apeireth-ui-demo/harness/settings/**` 里的文件
> 只有本批写。共享文件（`harness/{scripts,shared,tools,fixtures,evidence}/`、
> `harness/vite.config.mjs`、`harness/build.sh`、`crates-ui/apeireth-ui-kit/src/lib.rs`、
> `crates-ui/apeireth-ui-demo/src/{lib,main}.rs`）**零改动**——需要改的写在
> [§8 需要主线改的共享文件](#8-需要主线改的共享文件)（**只报不改**）。
>
> **诚实边界**：本批**没有跑过一次像素对照**（没有迁移 ⇒ 没有引擎侧产物 ⇒
> 对照无对象）。下文所有「对照方式」都是**基于生产源码结构 + 批一已验证的
> 装置形态**得出的**可执行设计**，不是已实测的结论。凡涉及数值的地方，
> 一律给口径与出处，不给未实测的百分比。

---

## 0. 为什么必须先切分（实测口径）

| 口径 | 数值 | 出处 |
|---|--:|---|
| 待迁四面的实测标记行 | 4,345 | owner 2026-10-24 裁定 |
| 其中设置页三页占比 | **75%** | 同上 |
| `SettingsView.svelte` 单文件占本面 | **83%**（3,775 / 4,545 量级） | 本批实测 |
| 批一「标记行 ⇒ Rust 行」倍率 | **2.87×** | 批一实测（试点 152 行） |
| 按 2.87× 直翻的结果 | **≈ 10,800 行 Rust（单文件）** | 3,775 × 2.87 |

⇒ **单文件 10,800 行 Rust 不可接受**（不可并行、不可逐块验收、不可逐块回归）。
⇒ 本批的任务就是把「一个 10,800 行的文件」变成「**6 个 800–2,500 行量级的
可验收单元**」。

---

## 1. 切分判据（三条，缺一不可）

本方案的每个子面都必须同时满足：

| # | 判据 | **可核验的问法** |
|---|---|---|
| **J1 可对照** | 子面能在浏览器里**单独渲染**，并与生产画面对照 | 「在浏览器里怎么把它单独渲染出来？」 |
| **J2 写域独立** | 子面的**可变状态集合**与其它子面**不相交**（或交集是宿主侧共享状态，且各子面只读不改） | 「这个子面会写哪些键？会不会踩到别人的键？」 |
| **J3 依赖明确** | 子面依赖的其它子面**是渲染期依赖还是写域依赖**，可逐条列出来 | 「它要谁先做完才能验？」 |

⚠️ **切分不是「按代码段落好看」**。本方案的切分轴是 **生产的分区（section）边界**
——因为生产 `SettingsView` 的 markup 本身就是 **15 个互斥的 `{#if activeSection === …}`
分支**，每个分支自带一个 `.setting-block` 根 ⇒ **DOM 根天然存在**（J1 的前提）。

---

## 2. 生产结构实测（切分的事实基础）

### 2.1 文件三段

| 段 | 行区间 | 行数 | 内容 |
|---|---|--:|---|
| module script | 1–21 | 21 | `DISPOSITION_PRESETS` / `DISPOSITION_BASELINE_RESET`（**导���**，被源码正则测试镜像校验 ⇒ **不迁**，留在生产） |
| instance script | 23–1920 | 1,898 | 状态、派生、副作用、提交缝 |
| markup | 1922–3775 | 1,854 | 15 个互斥分区分支 + 三个页级覆盖层 |
| style | 3776–5314 | **1,539** | 组件私有样式（**Rust 侧 0 行** ⇒ 抽取器拿，见 §7） |

### 2.2 markup 的分区边界（实测行号）

| 生产分区 id | markup 行区间 | 行数 | 落在本方案的哪个子面 |
|---|---|--:|---|
| （框架：页头 / 侧栏 / 内容列 / 横幅 / 大类说明） | 1922–1993 | 72 | **S1** |
| `appearance`（`ThemeSettingsPanel`） | 1994–2012 | 19 | **T1**（独立文件） |
| `avatar`（`AvatarSettingsPanel`） | 2014–2017 | 4 | ⚠️ **不在本面口径内**（独立文件，279 行，见 §5.3） |
| `models` | 2019–2362 | **344** | **S2** |
| `personality` | 2364–2454 | 91 | **S3** |
| `impression` | 2456–2520 | 65 | **S3** |
| `cognition` | 2522–2662 | 141 | **S3** |
| `disposition` | 2664–2862 | 199 | **S3** |
| `governance` | 2864–3004 | 141 | **S4** |
| `security`（`GovernanceView`） | 3006–3021 | 16 | **S4**（**外部组件**，见 §5.3） |
| `tools` | 3023–3199 | 177 | **S4** |
| `budget` | 3201–3342 | 142 | **S5** |
| `runtime` | 3344–3374 | 31 | **S5** |
| `data` | 3376–3489 | 114 | **S6** |
| `user` | 3491–3565 | 75 | **S6** |
| `developer`（`{:else}` 兜底分支） | 3567–3680 | 114 | **S4** |
| API Key 覆盖层（模态） | 3687–3749 | 63 | **S2** 的覆盖层 |
| 工作区选择覆盖层 | 3751–3758 | 8 | **S6** 的覆盖层（外部组件） |
| 危险动作二次确认覆盖层 | 3760–3774 | 15 | **共享覆盖层**（`ConfirmDialog`，见 §5.3） |

### 2.3 参考侧实测（本批跑出来的数字）

> **本批唯一的实测**：参考侧（生产组件**只读引入** + 固定数据替身）在
> **系统 Chromium / 视口 1440×900** 下逐「面 × 分区」渲染，量出裁剪框尺寸。
> ⇒ **这些数字是实测值，不是推导值**（推导过程见 §4.3）。

| 面 | 分区数 | 裁剪选择器（参考侧） | 实测宽 | 实测高 |
|---|--:|---|--:|--:|
| W1 向导 | 6 | `.wizard-backdrop` | **1440** | **900**（`position: fixed; inset: 0` ⇒ 整视口） |
| T1 主题 | 1 | `.setting-block` | **828** | 1121.69 |
| S1 框架 | 2 | `.settings-subnav` / `.settings-content` | **200** / **900** | 655.2 |
| S5 预算 | 2 | `.setting-block` | **828** | 1477.72 / 114.63 |
| S6 数据 | 2 | `.setting-block` | **828** | 1069.27 / 429.94 |
| S3 人格 | 4 | `.setting-block` | **828** | 964.73 / 484.84 / 1405.48 / 752.22 |
| S4 能力 | 3 | `.setting-block` | **828** | 681.94 / 1042.92 / 583.2 |
| S2 服务商 | 2 | `.setting-block` / `.modal-backdrop` | **828** / — | 797.55 / — |

**实测结论（4 条）**

1. **内容列宽恒为 828px** ⇒ 引擎侧的舞台宽度必须钉成 828px（§4.3）；
2. **侧栏恒为 200px**、**内容列 border-box 恒为 900px**；
3. **向导遮罩是整视口** ⇒ 那一面对照**不裁剪**，比整屏；
4. ⚠️ **API Key 模态默认不出现**（生产里要点「配置 Key」才开）⇒ 那一对
   **需要先触发再裁剪**（对照脚本的动作步骤，见 §9.3）。

⇒ 21 组「面 × 分区」的**参考侧画面全部可达**（本批逐组验过标题确实切到了
目标分区：模型与提供商 / 预算与配额 / 数据与本地缓存 / 伙伴人设与行为 /
用户印象 / 记忆与认知 / 性格与记忆 / 决策与治理 / 工具与安全 / 开发者选项 /
运行时诊断 / 用户中心 / 外观与主题）。

### 2.4 为什么「按分区」是对的（依据，不是偏好）

1. **DOM 根天然存在**：每个分区分支的内容都裹在 `<div class="setting-block">` 里
   ⇒ 圈定区块 = 选择器圈定，不需要拆生产文件。
2. **互斥 ⇒ 无需处理「同时在场」**：生产一次只渲染一个分区 ⇒ 参考侧的像素对照
   不受其它分区干扰。
3. **分区 id 已是深链契约**：15 个 id 在 `settings-nav-groups.ts` 与
   `resolveSettingsSection()` 里 ⇒ 按分区切分不新增契约、不改深链。

⚠️ **但「按分区」不足以直接当子面**：分区粒度太细（15 个）且**写域跨分区重叠**
（例如 `capabilities` 同时被记忆面 / 治理面 / 预算面写）。⇒ 下面把 15 个分区
**归并成 6 个子面**，归并的依据是**写域**与**共用组件**。

---

## 3. 六个子面

### 3.0 子面总表

| 子面 | id | 覆盖生产分区 | markup 行数（实测） | 根 class（CSS 作用域前缀） | Rust 模块（建议） |
|---|---|---|--:|---|---|
| **S1** | `frame` | （全页框架 + 侧栏 + 大类说明 + 横幅位） | 72 | `.settings-view` | `ui_kit::settings::SettingsFrame` |
| **S2** | `provider` | `models` | 344 + 63（覆盖层） | `.sp-provider` | `ui_kit::settings::ProviderGroup` |
| **S3** | `character` | `personality` / `impression` / `cognition` / `disposition` | **496** | `.sp-character` | `ui_kit::settings::CharacterPanels` |
| **S4** | `capability` | `governance` / `security` / `tools` / `developer` | **448** | `.sp-capability` | `ui_kit::settings::CapabilityPanels` |
| **S5** | `budget` | `budget` / `runtime` | 173 | `.sp-budget` | `ui_kit::settings::BudgetPanel` |
| **S6** | `data` | `data` / `user` | 189 | `.sp-data` | `ui_kit::settings::DataPanels` |

> **S3 / S4 是本方案的重心**：两者的 markup 合计 **944 行**（占 3,775 的 **25%**），
> 而它们**共用同一套渲染模式**（§4.1）⇒ 一旦把模式抽成一个 Rust 组件，
> 这 944 行里的大部分变成**数据**而不是**代码**。

---

### 3.1 S1 `frame` —— 页面框架与导航

**边界**

| 项 | 内容 |
|---|---|
| **产出** | `PageHeader` 占位、`.settings-layout` 网格、`.settings-subnav` 侧栏（5 大类 → 15 分区两级导航）、`.settings-content` 内容列、`.category-context` 大类说明条、`.apply-status`（应用进行中）、错误横幅位（`ErrorSolutionBanner`） |
| **script 归属** | 176–240（props / `activeSection` / `expandedGroups` / 落区 effect）、1309–1369（`sections` 表 / `GROUP_ICONS` / `navGroups` / `activeGroup` / `lastSectionByGroup` / `selectSection` / `toggleGroup`）、1968–1993 的 markup 片段 `{#snippet fieldFlag(key)}`（字段旗标：未保存 / ✓） |
| **数据来源** | `SETTINGS_NAV_GROUPS` / `SETTINGS_SECTION_GROUP`（`settings-nav-groups.ts`，65 行）、`sections` 表（生产 1309–1325）、`GROUP_ICONS`（生产 1330–1336） |
| **不产出** | 任何内容区的字段（那是 S2–S6） |

**写域（J2）**

| 状态 | 归属 | 说明 |
|---|---|---|
| `activeSection` | **S1 写** | 侧栏点击改它；内容区**只读**它 |
| `expandedGroups` | **S1 写** | 多开多收、互不顶替 |
| `lastSectionByGroup` | **S1 写** | 每大类记住最近访问的分区页 |
| `liveApplyPendingKey` / `liveApplyError` | **S1 只读**（宿主侧写） | 任何子面发起 apply ⇒ 宿主写这两个 ⇒ S1 渲染 |
| `textDirty` / `textAck` | **S1 只读**（宿主侧写） | 同上；S1 渲染 `fieldFlag` 旗标 |

⇒ **S1 是唯一写「导航类状态」的面**；S2–S6 之间**零写域重叠**。

**依赖（J3）**

- **渲染期依赖**：无（最底层）。
- **写域依赖**：无。
- **被依赖**：S2–S6 的**对照装置**依赖 S1（见 §4.3 的「宽度同源」要求）。

**对照方式（J1）**

| 侧 | 怎么单独渲染 |
|---|---|
| 参考侧 | 生产 `SettingsView`（**只读引入**）+ `initialSection` prop ⇒ 侧栏已渲染；用**选择器** `.settings-subnav` 圈定导航列；用 `.settings-content` 圈定内容列（含大类说明条） |
| 引擎侧 | Rust `SettingsFrame` 独立挂载 ⇒ 产出**同一套 class** 的 `<section class="settings-view">…`；导航列裁剪用 `.settings-subnav` |
| 裁剪 | 导航列宽 **200px**（`.settings-layout` 的 `grid-template-columns: 200px 1fr`）⇒ 两侧裁剪框宽度天然同源；内容列宽 **828px**（`.settings-content` 的 `max-width: 900px` − 左右 `padding: 36px` × 2，**border-box** ⇒ `base.css:251` 的 `* { box-sizing: border-box }`）⇒ 引擎侧舞台宽度必须钉成 828px（对照专用样式表） |

**可信度：高。** 导航列是纯展示 + 纯本地态（展开集合 / 当前分区）⇒ 固定 fixture ⇒
DOM 决定论。⚠️ 唯一风险：`navGroups` 的构造依赖 `SETTINGS_NAV_GROUPS` 的**顺序与
`sectionIds`** ⇒ 迁移时必须**逐字搬运这张表**（fixture `nav-groups.json` 已按
生产顺序登记）。

**风险**

- R1：15 个分区的**图标**散在生产 `sections` 表与 `GROUP_ICONS` ⇒ 迁移时若按
  「重新挑一套图标」就会像素不一致 ⇒ **必须逐个沿用生产导入名**（清单见 §6）。
- R2：`lastSectionByGroup` 是 `Map`（非响应式）⇒ 生产里它**不触发重渲染**；
  迁移成信号时若改成响应式 ⇒ 画面**可能**变化（多一次重排）⇒ 建议**保持非响应式**
  （与生产同构）。

---

### 3.2 S2 `provider` —— 服务商与连接

**边界**

| 项 | 内容 |
|---|---|
| **产出** | `.provider-summary`（协议徽标 / 预设名 / 模型 / 连接态药丸 / 端点）、`.warnings-box`（apply 警告）、`.effective-config`（网关生效配置回显）、服务商组三步（`.protocol-tabs` 协议切换 / `.presets-row` 预设芯片 / 端点·密钥·模型）、`.model-picker-row`（模型选择器 + 重新加载）、`.key-input-wrapper`（密钥框 + 明文切换）、`.model-actions`（测试连接 + 预设推荐芯片）、`.test-result-box`（连接结果 + 远端模型目录）、`.advanced-box`（网关地址 + `.credential-row` 凭据行）、API Key 覆盖层（模态） |
| **script 归属** | 781–842（预设表）、892–948（协议/预设/模型/密钥状态 + 双缓冲）、988–1048（服务商组草稿与整组提交）、1186–1208（网关地址提交）、1210–1233（钥匙串状态 / apply 回显 / 模型发现）、1371–1420（协议切换 / 预设选择）、1422–1439（测试连接）、1441–1455（provider family + 密钥打码）、1482–1542（模型加载 / 打开密钥弹层 / 删除已存密钥）、1620–1704（apply 缝 + 写新密钥） |
| **不产出** | 通用表单件的**实现**（`fieldFlag` / 整组失焦提交 ⇒ S1 与宿主侧共用） |

**写域（J2）**

| 状态 | 归属 |
|---|---|
| `activeProtocol` / `activePreset` / `providerBaseUrl` / `providerApiKey` / `providerModel` / `anthropicVersion` / `showApiKey` | **S2 独占** |
| `openaiBuffer` / `anthropicBuffer` | **S2 独占**（协议切换的双缓冲） |
| `discoveredModels` / `modelsLoading` / `modelsError` / `modelSearchQuery` | **S2 独占**（`modelSearchQuery` 是纯 UI 过滤词） |
| `testResult` / `isTestingConnection` | **S2 独占**（宿主填结果） |
| `applyResult` / `effectiveConfig` | **S2 独占**（宿主填回显） |
| `storedKeyMasked` / `storedKeyExists` / `keychainLoading` / `keychainSaving` / `keychainDeleting` / `keychainActionError` / `showApiKeyModal` / `tempApiKey` | **S2 独占** |
| `editBaseUrl` / `showAdvancedGateway` / `gatewayCommitted` | **S2 独占** |
| `textDirty` / `textAck` 的键 `provider.*` / `gateway.baseUrl` | **S2 独占**（命名空间不与其它面重叠） |
| ⚠️ `config` / `onSave` | **宿主侧**（S2 只发「整组提交」意图） |

⇒ **S2 的写域与 S3–S6 完全不相交**。

**依赖（J3）**

- **渲染期依赖**：S1（页面框架）。
- **写域依赖**：无。
- **被依赖**：无。
- ⚠️ **宿主侧依赖（本面不迁的那一半）**：见 §6「引擎不迁清单」。

**对照方式（J1）**

| 侧 | 怎么单独渲染 |
|---|---|
| 参考侧 | 生产 `SettingsView` + `initialSection="models"` ⇒ 渲染 `models` 分支；裁剪选择器 **`.setting-block`**（`models` 分支的根，`SettingsView.svelte:2020`） |
| 引擎侧 | Rust `ProviderGroup` 挂在 `.sp-provider` 根下，内含 `<div class="setting-block">` ⇒ 裁剪选择器 `.sp-provider .setting-block` |
| 宽度 | 两侧裁剪框宽度必须**相等**（828px）⇒ 对照脚本**先断言宽度相等**，不等即判失败（不静默比较不同宽度） |
| 覆盖层 | API Key 模态是 `position: fixed` ⇒ **按整视口截图**对比（不裁剪），触发方式：参考侧点 `.credential-row` 里的「配置 Key」按钮；引擎侧调同一个 API |

**可信度：高（展示层）/ 不覆盖（数据获取层）。**

| 子项 | 可信度 | 理由 |
|---|---|---|
| 摘要条 / 协议页签 / 预设芯片 / 端点模型输入框 / 密钥框 | **高** | 纯展示 + 受控输入，fixture 固定 ⇒ DOM 决定论 |
| 远端模型目录 / 预设推荐芯片 | **高** | 同上（列表由 fixture 给） |
| 连接测试结果框 | **高** | 结果由 fixture 给；⚠️ **测的是「给定结果的渲染」，不是「测试连接这件事」** |
| 密钥框（`type=password`） | **中高** | 原生控件，**同一浏览器 ⇒ 同一渲染**；⚠️ 引擎侧需产出同样的 `type` / `value` / `autocomplete` 三属性 |
| 密钥打码串（首 3 + `****` + 末 3） | **高** | `maskApiKey`（生产 1451–1455）是**展示口径** ⇒ 可迁；⚠️ 短密钥（≤ 8）整体打码为 `****` 这条分支容易漏 |

**风险**

- R3：**空白折叠**。生产把元素间换行折叠成**恰好一个空格** ⇒ 引擎侧必须
  **逐个复现**这些空白文本节点（批一已踩过：`.tool-body` 的 2px 整卡位移）。
  ⇒ **迁移前先在浏览器 dump 生产 DOM 再照抄，不要推理**。
- R4：**密钥明文切换**是 `{#if showApiKey}` 的**结构分支**（`Eye` ↔ `EyeOff` 两个图标）
  ⇒ 不是 class 切换 ⇒ 引擎侧必须做**真正的条件渲染**（只产出一个图标节点）。
- R5：**整组提交语义**。生产是「端点/模型/密钥/协议头**整组**一个提交原子」
  （避免半截配置）⇒ 引擎侧若按字段各自提交 ⇒ 行为变了 ⇒ **提交时机是宿主侧的
  责任**（引擎只发「提交这组」意图）。

---

### 3.3 S3 `character` —— 人格与记忆（4 个分区）

**边界**

| 分区 | 内容 | markup 行区间 |
|---|---|---|
| `personality` | `.persona-card` 列表（名称 / 固定模型 / 人设文本 / 设为当前 / 删除）+「新增伙伴」+ `.notice-box` | 2364–2454 |
| `impression` | `.notice-box`（当前伙伴）+ `.impression-actions`（立即总结 / 自动总结开关 / 清空）+ 印象全文文本组 | 2456–2520 |
| `cognition` | `.cap-summary` + `.preset-bar`（推荐配置）+ 3 张 `.cap-card`（记忆流 5 行 / 认知增强 6 行 + 检索温度滑杆 / 社区与账本 2 行）+ `.cap-default-card` + `.cap-footnote` | 2522–2662 |
| `disposition` | `.cap-card`（4 个体验参数滑杆）+ `.preset-bar`（4 枚预设芯片）+ `.cap-card`（从使用中学习）+ `.cap-card`（学习日志 + 撤销）+ `.cap-footnote` | 2664–2862 |

**为什么这 4 个分区合成一个子面**

1. **共用同一套模式**：4 个分区里 **3 个**（`cognition` / `disposition` / 部分
   `personality`）的主体都是「**能力开关卡**」；第 4 个（`impression`）是文本编辑区。
2. **共用同一份写域**：`capabilities`（体验参数 4 键 + 记忆/认知/社区 13 键 +
   `selfTuning`）、`personas` / `activePersonaId`、`impression*`。
3. **共用同一个「当前伙伴」上下文**：`impression` 读 `activePersonaName`
   （由 `personas` + `activePersonaId` 派生）⇒ 拆开就要跨子面读状态。

**写域（J2）**

| 状态 | 归属 |
|---|---|
| `personas` / `activePersonaId` / `personasCommitted` | **S3 独占** |
| `impressionEntry` / `impressionDraft` / `impressionError` / `impressionBusy` | **S3 独占** |
| `capabilities` 的键：`memoryInjection` / `consolidation` / `reflexion` / `proactiveRecall` / `typedRecall` / `organs` / `preferenceLearning` / `partnerBond` / `morphologyRecall` / `education` / `absorptionInsight` / `communityTriage` / `oneringLedger` / `memoryFade` / `curiosityStrength` / `toneSaturation` / `consolidationCadence` / `selfTuning` | **S3 独占**（与 S4 / S5 的键集**不相交**） |
| `tuningLog` / `tuningLogLoading` | **S3 独占**（宿主填读数） |
| `textDirty` 键：`persona.*` / `user.impression` | **S3 独占** |
| ⚠️ `capabilities` **容器本身** | **宿主侧共享**（S4 / S5 也写不同的键 ⇒ 容器共享、**键集不相交** ⇒ 并行安全） |

**依赖（J3）**

- **渲染期依赖**：S1。
- **写域依赖**：宿主侧的 `capabilities` 容器 + `textDirty` 容器。
- **被依赖**：无。

**对照方式（J1）**

| 分区 | 参考侧 | 引擎侧 | 裁剪选择器 |
|---|---|---|---|
| `personality` | `initialSection="personality"` | `CharacterPanels::personality` | `.setting-block` |
| `impression` | `initialSection="impression"` | `CharacterPanels::impression` | `.setting-block` |
| `cognition` | `initialSection="cognition"` | `CharacterPanels::cognition` | `.setting-block` |
| `disposition` | `initialSection="disposition"` | `CharacterPanels::disposition` | `.setting-block` |

⇒ **一个子面 ⇒ 四次对照**（四个分区各一次）⇒ 对照脚本按「子面 × 分区」矩阵跑。

**可信度**

| 子项 | 可信度 | 理由 |
|---|---|---|
| 能力开关卡（8 组 `CapDef` 行） | **高** | 数据驱动 ⇒ fixture 给定义表；开关是 `role="switch"` + `aria-checked` + class 组合 ⇒ DOM 决定论 |
| 滑杆（`type=range`） | **中高** | 原生控件，同浏览器一致；⚠️ **滑块位置的像素**由浏览器按 `value`/`min`/`max`/`step` 算 ⇒ 引擎侧必须产出**逐字相同的这四个属性** |
| 4 枚预设芯片 | **高** | 纯展示 |
| 学习日志行 + 「撤销」按钮 | **高（渲染）/ 不覆盖（读数）** | 读数由 fixture 给 |
| ⚠️ 学习日志的**空态分支** | **中** | 生产条件是 `{#if !isDesktop() || tuningLog === null}` ⇒ 浏览器里 `isDesktop()` 恒为 `false` ⇒ **默认永远走「仅桌面版可读」分支** ⇒ 要对照「有日志」的分支，**必须 mock `desktop-bridge`**（见 §8） |
| 人设卡 | **高** | 表单 + 列表；⚠️ `id` 是**模板串**（`persona-name-{p.id}`）⇒ 引擎侧 id 必须逐字相同 |
| 印象文本区 | **高** | textarea；⚠️ `maxlength` 属性、`rows` 属性要逐字 |

**风险**

- R6：**能力开关的「反向旋钮」语义**。`typedRecall` 是 `invert: true`
  （env 名 `APEIRETH_DISABLE_TYPED_RECALL`）⇒ 「开」= `value === false`
  ⇒ `isCapOn()` 的判定是 `def.invert ? value === false : value === true`
  ⇒ 迁移时若按普通布尔处理 ⇒ **开关显示与实际值相反**（静默错误，最危险的一类）。
- R7：**依赖行（`requires`）**。`fileWriteAutoPass` 依赖 `fileWrite`
  ⇒ `capDisabled()` 使其**变暗 + 禁用**。⚠️ 这是**父行状态驱动的子行状态** ⇒
  引擎侧必须保留「父行变化 ⇒ 子行重算」的依赖边。
- R8：**嵌套行的插入位置**。`morphologyRecall` 的滑杆行与 `shell` 的沙箱行是
  **插在父行 `{#each}` 体内**的兄弟节点（不是子节点）⇒ 引擎侧列表渲染必须支持
  「一项产出两个节点」（keyed 列表的插入点）。
- R9：**`module` 段导出的两个常量**（`DISPOSITION_PRESETS` /
  `DISPOSITION_BASELINE_RESET`）被**源码正则测试**镜像校验
  ⇒ 迁移它们进 Rust 会**打断那个测试**（`frontend/companion-desktop/tests/self-tuning-mapping.mjs`）
  ⇒ **不迁**，作为 fixture 数据由生产侧继续持有。

---

### 3.4 S4 `capability` —— 能力与治理（4 个分区）

**边界**

| 分区 | 内容 | markup 行区间 |
|---|---|---|
| `governance` | `.cap-summary` + 评审议会卡（认知深度 `select` + `DECISION_DEFS` 2 行 + 顾问数量滑杆 + 超时数字框）+ 权限子代理卡（`GUARD_DEFS` 2 行）+ `.cap-default-card`（议会语义） | 2864–3004 |
| `security` | 整段委派给生产 `GovernanceView`（16 行）⇒ **外部组件**，见 §5.3 | 3006–3021 |
| `tools` | `.form-group` 允许连接的自建服务器（`.allowed-server-row` 列表 + 添加行）+ `.cap-summary` + 全局权限预设 `select` + 工具授予卡（`TOOL_DEFS` 4 行 + 沙箱嵌套行 + `TOOL_SUB_DEFS` 1 行 + MCP 说明行）+ 两张 `.cap-default-card` | 3023–3199 |
| `developer` | Beta 功能卡（`BETA_DEFS` 1 行 + 思考模式文本旋钮组）+ `.cap-default-card`（运行时契约）+ `.form-group` 客户端配置 JSON（`.code-box-wrap` / `.code-box`） | 3567–3680 |

**写域（J2）**

| 状态 | 归属 |
|---|---|
| `cognitiveDepth` | **S4 独占**（由 `judge`/`council` 派生 ⇒ 只依赖 S4 的键） |
| `allowedServers` / `allowedServerInput` / `allowedServerError` / `allowedServersLoaded` / `allowedServersBusy` | **S4 独占** |
| `permissionPreset` | **S4 独占**（`localStorage` 记录，**不进 config patch**） |
| `configJsonCopied` | **S4 独占** |
| `capabilities` 的键：`judge` / `council` / `councilAdvisors` / `councilTimeoutMs` / `onionLayer` / `worktreeSandbox` / `shell` / `shellSandbox` / `fetch` / `localReadTools` / `fileWrite` / `fileWriteAutoPass` / `reasoningEnabled` / `reasoningModelFilters` / `reasoningTag` | **S4 独占**（与 S3 / S5 的键集**不相交**） |
| `textDirty` 键：`council.timeoutMs` / `reasoning.*` | **S4 独占** |
| ⚠️ `reasoningGroupEl` | **S4 独占**（整组失焦提交的锚点元素） |

**依赖（J3）**

- **渲染期依赖**：S1。
- **写域依赖**：宿主侧 `capabilities` 容器 + `textDirty` 容器。
- **被依赖**：无。
- ⚠️ **`security` 分区不迁**（外部组件 `GovernanceView` ⇒ 属**治理卷宗面**
  ⇒ 见 §5.3 与 §6）。

**对照方式（J1）**

| 分区 | 参考侧 | 引擎侧 | 备注 |
|---|---|---|---|
| `governance` | `initialSection="governance"` | `CapabilityPanels::governance` | ⚠️ `council` 开关**关闭时**，顾问数量/超时两行**不渲染** ⇒ fixture 必须给出 `council: true` 才能对照这两行 |
| `security` | `initialSection="security"` | —（**不迁**） | 对照对象是生产 `GovernanceView` ⇒ 归治理卷宗面 |
| `tools` | `initialSection="tools"` | `CapabilityPanels::tools` | ⚠️ `shell` 关闭 ⇒ 沙箱嵌套行不渲染；`shellSandbox` 关闭 ⇒ 还要多渲染一条 `.cap-warning` ⇒ fixture 至少要两态 |
| `developer` | `initialSection="developer"` | `CapabilityPanels::developer` | ⚠️ `reasoningEnabled` 关闭 ⇒ 思考模式文本旋钮组**不渲染** ⇒ fixture 给 `true` |

**可信度**

| 子项 | 可信度 | 理由 |
|---|---|---|
| 能力开关行（`TOOL_DEFS` / `GUARD_DEFS` / `DECISION_DEFS` / `BETA_DEFS`） | **高** | 同 S3 的开关卡 |
| 沙箱嵌套行 + `.cap-warning` | **高** | 条件渲染的两个分支各对照一次 |
| `select`（认知深度 / 全局权限预设） | **中高** | 原生控件，同浏览器一致；⚠️ 引擎侧要产出**同样的 `option` 列表与逐字相同的 `value`** |
| 数字框（超时） | **高** | `type=number` + `min`/`step` |
| 客户端配置 JSON（`.code-box`） | **高** | ⚠️ 内容是 `JSON.stringify({baseUrl, model, provider, hasApiKey}, null, 2)` ⇒ **键序固定为这四个**、缩进 2 ⇒ 引擎侧必须用 `ui-kit/src/json.rs` 的 `JsonValue`（保插入序）+ `num.rs` 的 ECMAScript 数值语义 ⇒ **不要手拼字符串** |
| 允许服务器列表（`.allowed-server-row`） | **高** | 列表 + 空态 |
| 剪贴板复制（配置 JSON / 运维命令） | **不覆盖** | 剪贴板是宿主能力（§6） |

**风险**

- R10：**`select` 的原生外观**。引擎侧若产出自定义下拉 ⇒ 像素必不一致 ⇒
  **必须产出原生 `<select>` + `<option>`**（不要画）。
- R11：**JSON 预览的键序与缩进**。`JsonValue` 保插入序 ⇒ 四个键的顺序必须按
  生产 `JSON.stringify` 的实参顺序（`baseUrl` → `model` → `provider` → `hasApiKey`）。
- R12：**`permissionPreset` 落 `localStorage`**（键 `apeireth-permission-preset-default`）
  ⇒ 引擎不碰 localStorage（宿主侧）⇒ 引擎只承接「当前值 + 变更意图」。
- R13：**`developer` 是 `{:else}` 兜底分支** ⇒ 生产上「分区 id 未知」也会落这里
  ⇒ 迁移时若把兜底改成 `{#if}`，**深链兜底行为变了** ⇒ 保持兜底语义。

---

### 3.5 S5 `budget` —— 预算与配额 + 运行时诊断

**边界**

| 分区 | 内容 | markup 行区间 |
|---|---|---|
| `budget` | 会话消耗卡（`.meter-grid` 4 格 + `.budget-remaining` 余量行）+ `MetricsPanel`（外部组件）+ 预算旋钮卡（3 行数字框 + `.budget-feedback`）+ 多维配额卡（4 行「暂无接口」）+ 预算耗尽行为卡 | 3201–3342 |
| `runtime` | 「立即执行深度诊断」按钮 + `.diag-results`（总体状态 + 延迟 + 子系统列表 + `StatusBadge`） | 3344–3374 |

**写域（J2）**

| 状态 | 归属 |
|---|---|
| `budgetDraft` / `budgetFeedback` | **S5 独占** |
| `runtimeReport` / `checkingRuntime` | **S5 独占**（宿主填读数） |
| `capabilities` 的键：`maxTurnRounds` / `maxToolCalls` / `contextBudgetChars` | **S5 独占**（与 S3 / S4 的键集**不相交**） |
| `textDirty` 键：`budget.*` | **S5 独占** |
| `sessionUsage` / `sessionTotalsView` / `budgetRemaining` / `budgetBadges` | **派生**（宿主喂 `sessionUsage` + `capabilities`） |

**依赖（J3）**

- **渲染期依赖**：S1。
- **写域依赖**：宿主侧 `capabilities` 容器 + `textDirty` 容器。
- **被依赖**：无。
- ⚠️ **派生函数的宿主依赖**：`budgetRemainingRows` / `effectiveBudget` /
  `formatSessionTotals` / `emptySessionTotals` 都在 `lib/budget.ts`（393 行）与
  `lib/chat-shell/turn-telemetry.ts` ⇒ 这些是**纯函数** ⇒ 可迁（无 I/O）⇒
  建议迁进 `ui_kit::settings::budget`（宿主喂输入，引擎算输出）。
  ⚠️ 但 **`QUOTA_DIMENSIONS` / `BUDGET_EXHAUSTION` / `BUDGET_KNOB_SPECS` 是
  带文案与 env 名的常量表** ⇒ 与 §3.3-R9 同理，**迁移它们会打断生产的源码镜像
  测试** ⇒ 建议**留生产侧、由宿主喂进来**（引擎只渲染）。

**对照方式（J1）**

| 分区 | 参考侧 | 引擎侧 | 裁剪选择器 |
|---|---|---|---|
| `budget` | `initialSection="budget"` | `BudgetPanel::budget` | `.setting-block` |
| `runtime` | `initialSection="runtime"` | `BudgetPanel::runtime` | `.setting-block` |

**可信度**

| 子项 | 可信度 | 理由 |
|---|---|---|
| `.meter-grid` 四格 | **高** | 纯展示 |
| `.budget-remaining` 余量行 + `.remaining-fill` 宽度 | **高** | ⚠️ 宽度是**内联 style**（`style="width: {Math.round(row.ratio*100)}%"`）⇒ 引擎侧**必须**产出同样的内联 style（设计文档 §9.2 C3：**被替换的那一侧本来就有内联样式时才允许写**，且要逐字一致） |
| 预算旋钮数字框 | **高** | `type=number` + `min`/`max`/`step`；⚠️ `max` 为 `null` 时生产写 `max={undefined}` ⇒ **属性不出现**（不是 `max="null"`） |
| `.budget-feedback` | **高** | 宿主填文案 |
| 多维配额行 | **高** | 纯展示（4 行都是「暂无接口」） |
| `MetricsPanel` | **不迁 / 不覆盖** | 外部组件 + 内部 HTTP（§6） |
| 诊断结果 | **高（渲染）** | 宿主填 `RuntimeHealthReport` |

**风险**

- R14：**内联 style 的百分比**。`Math.round(row.ratio * 100)` ⇒ 引擎侧要用
  `num.rs` 的 ECMAScript 数值语义算 round（不是 `f64::round` 的银行家舍入差异）
  ⇒ 例：`ratio = 0.335` ⇒ JS `Math.round` ⇒ `34`（`.5` 向上）⇒ Rust 的
  `f64::round` 同样是 away-from-zero ⇒ 一致；但 `ratio * 100` 本身的浮点误差
  两边要一致 ⇒ **同一份输入 + 同一套算法**才有一致结果 ⇒ 引擎侧要复用
  `budget.ts` 的算式（`budgetRemainingRows`）而不是重写。
- R15：**徽标的两种形态**（`configured` / `constant`）⇒ class 是 `class:on` 与
  `class:note` 两个**互斥** class ⇒ 引擎侧要复现「同时存在/互斥」。

---

### 3.6 S6 `data` —— 数据与用户

**边界**

| 分区 | 内容 | markup 行区间 |
|---|---|---|
| `data` | daemon 状态卡（`.ops-rows` 读数行 + `.ops-guidance` 启停指引与可复制命令）+ `.info-card` 工作区目录（`.workspace-row`）+ `.danger-zone-box`（两档清除 + 未接线说明） | 3376–3489 |
| `user` | `.user-card`（头像槽 + 上传/移除按钮 + 隐藏文件输入）+ 用户信息文本组（昵称 / 签名）+ 错误提示 | 3491–3565 |

**写域（J2）**

| 状态 | 归属 |
|---|---|
| `daemonStatus` / `residentProbe` / `daemonLoading` | **S6 独占**（宿主填读数） |
| `opsCopiedKey` / `opsCopyError` | **S6 独占**（剪贴板回执） |
| `workspaceDir` / `workspaceLoading` / `workspaceError` / `showWorkspacePicker` / `workspaceSuggestions` / `workspaceCommitted` | **S6 独占** |
| `dangerPending` / `dangerPayload` | **S6 独占**（危险动作的待确认态） |
| `userProfile` / `userProfileError` / `avatarFileInput` | **S6 独占** |
| `textDirty` 键：`workspace.dir` / `user.info` | **S6 独占** |

**依赖（J3）**

- **渲染期依赖**：S1。
- **写域依赖**：无。
- **被依赖**：无。
- ⚠️ **覆盖层**：工作区选择覆盖层是外部组件 `WorkspacePickerModal`（297 行）⇒
  **不迁**（§5.3）；危险动作二次确认是 `ConfirmDialog`（共享，§5.3）。

**对照方式（J1）**

| 分区 | 参考侧 | 引擎侧 | 裁剪选择器 |
|---|---|---|---|
| `data` | `initialSection="data"` | `DataPanels::data` | `.setting-block` |
| `user` | `initialSection="user"` | `DataPanels::user` | `.setting-block` |

**可信度**

| 子项 | 可信度 | 理由 |
|---|---|---|
| `.ops-rows` 读数行 | **高（渲染）** | 宿主喂 `BackendStatus` / `ResidentProbeReport`；⚠️ 生产把 `null` 渲染成 `—`（DASH）⇒ 「读不到」的分支也要对照一次 |
| `.ops-guidance` 命令行 | **高** | 纯展示 + 复制按钮 |
| 工作区行 | **高** | 输入 + 按钮 + 错误提示 |
| 危险区 | **高** | 纯展示；⚠️ 「未接线」说明行（记忆遗忘）是**固定文案** ⇒ 逐字 |
| 用户卡 / 头像 | **中高** | ⚠️ 头像走 `localStorage` 里的 data URL ⇒ fixture 要给**一张固定的图**（否则两侧各跑各的压缩 ⇒ 像素不一致）⇒ 对照时用**已知的 data URL**（不跑压缩管线） |

**风险**

- R16：**「读不到」的诚实分支**。`daemonOpsView` 在 `status === null && probe === null`
  时给 4 行 `—` + 一条「仅桌面版可读」注 ⇒ 这条**正是 0 装诚实口径的体现** ⇒
  必须对照，且 fixture 要**两种都给**（有读数 / 无读数）。
- R17：**危险动作**必须走二次确认（`DANGER_ACTION_CONFIRMATIONS` 是唯一文案源）
  ⇒ 引擎侧发「请求危险动作」意图，**宿主**执行 ⇒ 引擎不实现删除。
- R18：**头像压缩**（`downscaleAvatarImage` ⇒ 256px JPEG data URL）在**宿主侧**
  ⇒ 引擎只承接「已有 data URL」⇒ fixture 必须给**确定的** data URL。

---

## 4. 两个「共用模式」（本方案的复用红利）

### 4.1 能力开关卡（`cap-card` 家族）——**一个 Rust 组件吃掉 8 组数据**

生产里 `.cap-card` / `.cap-row` / `.cap-switch` / `.cap-slider-wrap` /
`.cap-env` / `.cap-default-card` / `.cap-summary` 这一整套，被 **8 个注册表**驱动：

| 注册表 | 生产行 | 出现在 | 行数 |
|---|---|---|--:|
| `MEMORY_FLOW_DEFS` | 442–448 | `cognition` | 5 行 |
| `COGNITION_DEFS` | 451–458 | `cognition` | 6 行（+1 嵌套滑杆行） |
| `COMMUNITY_DEFS` | 461–464 | `cognition` | 2 行 |
| `DECISION_DEFS` | 467–470 | `governance` | 2 行 |
| `GUARD_DEFS` | 473–476 | `governance` | 2 行 |
| `TOOL_DEFS` + `TOOL_SUB_DEFS` | 479–497 | `tools` | 4 行 + 1 嵌套行 |
| `BETA_DEFS` | 500–502 | `developer` | 1 行 |

⇒ **8 个注册表 ≈ 74 行数据**，却撑起 **944 行 markup**（S3 + S4）。
⇒ **迁移的正确形态是「一个组件 + 一张定义表」**，不是「8 段模板」。
⇒ ⚠️ 这条是本方案**最大的省量点**，也是**最大的口径风险**（`invert` / `requires` /
  `capDisabled` / 嵌套行插入点，见 §3.3-R6/R7/R8）。

### 4.2 表单输入族（`text-group` / `form-group`）——**提交时机是宿主语义**

生产的表单有**三级即效**语义（改动的生效时机）：

| 级别 | 控件 | 生效时机 |
|---|---|---|
| 1 | 开关 / 滑杆 / 预设 | 点（或松手）即生效 |
| 2 | 文本输入 | **失焦或回车**提交（服务商组**整组**提交，避免半截配置） |
| 3 | 危险动作 | 即点 + 二次确认，确认后即效 |

⇒ 引擎侧承接「**受控输入 + 提交时机意图**」；**提交的执行**（`onSave` /
  钥匙串 / env 注入 / 侧车重启）在**宿主侧**。
⇒ ⚠️ 因此**表单控件本身可以做成 `ui-kit` 的通用件**
（`input::TextField` / `input::Switch` / `input::Select` / `input::Button`
—— 设计文档 §8.2 已列这四个「既有实现里内联、迁移时抽成基础件」）。

---

## 5. 两个独立文件（**不需要切分**）

### 5.1 W1 `wizard` —— 首次运行向导（`FirstRunWizard.svelte`，610 行）

| 项 | 内容 |
|---|---|
| 边界 | 整个文件（script 1–186 + markup 188–338 + style 340–610） |
| DOM 根 | `.wizard-backdrop`（`position: fixed; inset: 0`）⇒ **整视口截图**，不裁剪 |
| 写域 | `tier` / `stepIndex` / `presetId` / `apiKey` / `baseUrl` / `model` / `selfDescription` / `autoTune` / `applying` / `error` —— **全部本地**，零共享 ⇒ 与 S1–S6 **零写域交集** |
| 依赖 | 宿主侧：`setProviderKey`（钥匙串）、`writeSelfDescription`（本地存储）、`buildWizardConfig` / `stepsFor`（纯函数，`lib/onboarding.ts`） |
| 对照 | 参考侧：**直接**挂载生产 `FirstRunWizard`（它本来就是独立组件，不需要 `initialSection` 技巧）⇒ 裁剪 = 整视口；引擎侧：Rust 挂载在同一舞台 ⇒ 整视口 |
| 可信度 | **高**（6 个步骤各对照一次；分流两态（普通 / 专业）各对照一次） |
| 风险 | R19：**遮罩层是 `position: fixed`** ⇒ 引擎页必须**没有** `body { overflow: hidden }` 之外的高度约束差异 ⇒ 对照专用样式表要把 body 高度与滚动钉死。R20：**步骤条 `{#if i < stepIndex}` 的图标分支**（`Check` vs 序号文本）⇒ 结构分支，不是 class 分支。 |

### 5.2 T1 `theme` —— 主题外观面板（`ThemeSettingsPanel.svelte`，447 行）

| 项 | 内容 |
|---|---|
| 边界 | 整个文件（script 1–113 + markup 115–230 + style 232–447） |
| DOM 根 | `.theme-panel`（组件根）；⚠️ **它在生产里不是整页**，而是 `SettingsView` 的 `appearance` 分支里的一块（外面裹着 `.setting-block` + 标题 + 说明）⇒ **参考侧走生产的 `SettingsView`（`initialSection="appearance"`）**，裁剪 `.setting-block` ⇒ 实测宽 **828px**（§2.3） |
| CSS 根 class | **`.sp-theme`**（引擎侧的作用域前缀；组件自身的 `.theme-panel` 规则照旧前缀到 `.sp-theme` 下） |
| 写域 | `bgBusy` / `bgError` / `customPreview` / `fileInput` / `showClearBgConfirm` —— **全部本地** ⇒ 零写域交集 |
| 依赖 | 宿主侧：`applyDocumentTheme` / `applyDocumentAccent`（写 `<html>` 的 `data-theme`/`data-accent` ⇒ **引擎不碰**，设计文档 §8.3 规则 5）、`bg-store`（IndexedDB）、`validateCustomBgFile` |
| 对照 | 参考侧：挂载生产 `SettingsView` + `initialSection="appearance"` ⇒ 裁剪 `.setting-block`（**生产真实宽度 828px**）；引擎侧：Rust 挂载在 `.sp-theme` 根下 ⇒ 裁剪 `.sp-theme .setting-block` |
| 可信度 | **高**（10 个主题卡 + 4 个配色卡各对照；选中态各对照一次） |
| 风险 | R21：**主题卡片的 swatch 是内联 `style:background`**（生产 128 行）⇒ 内联样式必须**逐字一致**（渐变字符串很长 ⇒ 由 fixture 提供**原样字符串**，引擎侧不重算渐变）。R22：**配色样品的 CSS 变量覆写**（`style:--ap-accent-ui` / `style:--ap-accent-ui-ink`）⇒ 引擎侧要产出**同样的自定义属性内联声明**。R23：**背景图预览**是 `URL.createObjectURL(blob)` ⇒ 每次运行的 URL 不同 ⇒ 对照时**两侧都用 fixture 给的同一个 URL**（否则像素不一致）。 |

### 5.3 本面**不迁**的外部组件（登记，避免误吞）

| 组件 | 行数 | 归属 | 理由 |
|---|--:|---|---|
| `GovernanceView.svelte` | 145 | **治理卷宗面** | `security` 分区整段委派给它；本面只登记「S4 有一个 16 行的委派壳」 |
| `AvatarSettingsPanel.svelte` | 279 | **不在本面口径内** | 任务书给本面的三个文件不含它；`avatar` 分区只有 4 行委派壳 |
| `MetricsPanel.svelte` | 205 | **数据面板**（设计文档 §8.2：`data::MetricsPanel`） | 内部拉 `/v1/panel/metrics` ⇒ HTTP 契约 |
| `PageHeader.svelte` | 23 | `shell::PageHeader` | 页头，S1 只放占位 |
| `StatusBadge.svelte` | 62 | `feedback::StatusBadge` | S3 / S5 消费 |
| `ConfirmDialog.svelte` | 190 | `overlay::ConfirmDialog` | **三个子面共用**（S2 / S3 / S6）⇒ 共享组件，不属任一子面 |
| `ErrorSolutionBanner.svelte` | 178 | `feedback::ErrorSolutionBanner` | S1 消费 |
| `SessionModelPicker.svelte` | 241 | `input::SessionModelPicker` | S2 消费 |
| `WorkspacePickerModal.svelte` | 297 | `overlay::WorkspacePickerModal` | S6 消费 |
| `StatusBar.svelte` | — | **状态条面**（另一面） | 不在本面 |

---

## 6. 引擎不迁的部分（**留宿主侧**）

设计文档 §1.3 第 6/7 条：**引擎不碰 HTTP / SSE 契约，不碰宿主能力**。
本面逐条登记（**这一半留在生产侧**）：

| # | 不迁的东西 | 生产位置 | 引擎侧只承接什么 |
|---|---|---|---|
| N1 | `testProviderConnection`（测连接） | `lib/runtime.ts` | 「给定结果的渲染」（`testResult`） |
| N2 | `listModels`（`/v1/models`） | `lib/runtime.ts` | 「给定模型列表的渲染」 |
| N3 | `applyAdminConfig` / `getAdminConfig`（网关热应用 + 回显） | `lib/runtime.ts` | 回显字段（`provider` / `base_url` / `api_key` / `model`） |
| N4 | `checkHealthDetailed`（深度诊断） | `lib/runtime.ts` | `RuntimeHealthReport` 的渲染 |
| N5 | `chatOnce` / `loadConversations` / `summarizeUserImpression`（立即总结） | `lib/runtime.ts` / `lib/user-impression.ts` | 「已总结」的渲染 |
| N6 | `MetricsPanel` 的 `/v1/panel/metrics` | `components/MetricsPanel.svelte` | 不迁 |
| N7 | **钥匙串**：`getProviderKey` / `setProviderKey` / `deleteProviderKey` | `lib/tauri-bridge.ts` | 「已配置 / 未配置」状态位 + 打码串（**打码口径可迁，密钥不迁**） |
| N8 | **工作区**：`getWorkspaceDir` / `setWorkspaceDir` / `listWorkspaceSuggestions` | `lib/tauri-bridge.ts` | 路径文本 + 提交意图 |
| N9 | **允许服务器**：`addAllowedServer` / `listAllowedServers` / `removeAllowedServer` | `lib/desktop-bridge.ts` | 列表渲染 + 增删意图 |
| N10 | **运维读数**：`getBackendStatus` / `getResidentProbe` | `lib/desktop-bridge.ts` | 读数渲染 |
| N11 | **学习日志**：`readTuningLog` | `lib/desktop-bridge.ts` | 日志行渲染 + 撤销意图 |
| N12 | **本地存储**：`user-profile.ts`（localStorage）、`user-impression.ts`（localStorage）、`bg-store.ts`（IndexedDB） | 同名模块 | 表单值 + 提交意图 |
| N13 | **剪贴板**：`navigator.clipboard.writeText`（配置 JSON / 运维命令复制） | 生产内联 | 「已复制」回执态 |
| N14 | **主题写 `<html>`**：`applyDocumentTheme` / `applyDocumentAccent` | `lib/theme.ts` | **零改写**（只渲染选中态） |
| N15 | **头像压缩**：`downscaleAvatarImage`（256px JPEG） | `lib/user-profile.ts` | 已有 data URL 的渲染 |
| N16 | **文件选择**：`<input type=file>` 的取文件 | 生产 | 已有预览的渲染 |
| N17 | **随机 id**：`crypto.randomUUID()`（新增伙伴） | 生产 | 「新增」意图 |
| N18 | **危险动作执行**：删数据 / 删密钥 / 删伙伴 | 生产 | 二次确认弹层 + 意图 |
| N19 | **能力契约**：`CapabilityManifest`（`capabilityManifest` prop → 门控） | 宿主 | `GovernanceView` 的门控（外部组件） |
| N20 | **`module` 段导出常量**：`DISPOSITION_PRESETS` / `DISPOSITION_BASELINE_RESET` | 生产 | 作为 fixture 数据由宿主喂入（**迁它会打断生产的源码镜像测试**） |

⇒ **结论**：引擎侧承接的是「**展示面 + 受控输入 + 提交/意图事件**」；
**数据获取 / 凭据 / 本地存储 / 宿主能力 / 主题写入 / 危险动作执行**全部留在宿主侧。

---

## 7. CSS 抽取（本批**不抽**，但根 class 已定）

### 7.1 规则（本批已实测的事实 + 由此推出的结论）

| 事实 | 出处 |
|---|---|
| CSS 在 Rust 侧是 **0 行** ⇒ 生产 `<style>` 块**原文抽取**注入引擎页 | 批一已实现 |
| 批一已实现「**按组件作用域化**」：前缀根 class，`:global(…)` 保持全局，`@` 规则原样 | 批一 `scripts/extract-assets.mjs` 的 `scopeCss()` |
| 批一已证明**同名 class 会冲突**：`.tool-body` 的 `gap` 一个 8px、一个 10px ⇒ 2px 整卡位移 | 批一实测 |
| 抽取器现版只支持**单组件单根 class**（`COMPONENTS` 表：`{slug, file, css, scope}`） | `harness/scripts/extract-assets.mjs:69-83` |

### 7.2 本面需要的抽取器能力（**共享文件，主线改**）

1. **按子面分片**：`SettingsView.svelte` 的 1,539 行 CSS 要切成 **6 份**
   （每子面一份），而不是一整份。
2. **多根 class 作用域化**：每份用**自己的**根 class 作前缀（§3.0 表）。
3. **公共片段按使用面复制**：`.setting-block` / `.block-title` / `.block-desc` /
   `.text-group` / `.form-group` / `.field-hint` / `.field-flag` / `.cap-*` 等
   被多个子面使用 ⇒ **逐字复制到每个用它的子面文件**（规则文本相同 ⇒ 多份并存
   不冲突）。
4. **框架片段只进 S1**：`.settings-view` / `.settings-layout` / `.settings-subnav` /
   `.settings-content` / `.subnav-*` / `.category-context*` / `.apply-status`。
5. **`@` 规则原样**、**`:global(…)` 去括号保持全局**（批一 `scopeCss()` 已实现 ⇒
   复用即可）。

### 7.3 每个子面的根 class（**作用域化前缀**）

| 子面 | 根 class | 引擎侧 DOM 形态 | ⚠️ 作用域注意点 |
|---|---|---|---|
| S1 | `.settings-view` | `<section class="settings-view">` | 根自身的规则保持原样（批一 `scopeCss` 已有此分支） |
| S2 | `.sp-provider` | `<div class="sp-provider"><div class="setting-block">…</div></div>` | ⚠️ 根 class 必须在 `.setting-block` **之外**（否则 `.sp-provider .setting-block` 匹配不到根自己） |
| S3 | `.sp-character` | 同上 | 同上 |
| S4 | `.sp-capability` | 同上 | 同上 |
| S5 | `.sp-budget` | 同上 | 同上 |
| S6 | `.sp-data` | 同上 | 同上 |
| W1 | `.wizard-backdrop` | `<div class="wizard-backdrop">` | 根自身的规则原样；子规则前缀 |
| T1 | `.sp-theme` | `<div class="sp-theme"><div class="setting-block">…<div class="theme-panel">…</div></div></div>` | ⚠️ 生产 `ThemeSettingsPanel` 有一条**混合**选择器 `.bg-actions :global(.quiet-btn)` ⇒ 实测编译成 `.bg-actions.svelte-x .quiet-btn` ⇒ 抽取器对它**前缀根 class**（`.sp-theme .bg-actions .quiet-btn`）⇒ 特异度等价 |

> **为什么引擎侧要加一层 `.sp-*` 包裹**：生产的 Svelte 作用域 class 是
> **组件级**的（同一个 `svelte-<hash>` 覆盖整个 `SettingsView`）⇒ 抽出后若整份
> 注入 ⇒ 6 个子面的同名规则（`.cap-row` 等）会互相覆盖 ⇒ 必须按子面分片 +
> 各自前缀。**引擎侧加 `.sp-*` 根 class 是分片作用域的锚点** ⇒ 参考侧不需要它
> （Svelte 自己已经作用域化）⇒ 两侧的**计算样式**在各自的作用域内**等价**。

### 7.4 「同一份样式表驱动同一张卡」这条前提的延续

批一的前提是「**同一份样式表驱动同一张卡**」⇒ 本面把它具体化为：

| 面 | 参考侧样式来源 | 引擎侧样式来源 | 是否同一份 |
|---|---|---|---|
| S1–S6 | 生产组件自带 `<style>`（编译期注入，Svelte 作用域化） | 抽取器产出的**按子面分片** CSS（根 class 前缀作用域化） | **规则文本同一份**（逐字）⇒ 只是作用域化方式不同 |
| W1 / T1 | 生产组件自带 `<style>` | 抽取器产出的 `wizard.css` / `theme-panel.css` | 同上 |
| 设计层 | `src/lib/design/*.css`（只读复制） | 同一批文件 | **完全同一份** |

---

## 8. 需要主线改的共享文件（**本批只报不改**）

> 下列文件**本批零改动**。前 4 项是**阻塞项**（不��� ⇒ 设置页对照跑不起来），
> 后 4 项是**接线项**（并入统一 harness / CI）。

| # | 文件 | 需要什么 | 阻塞? |
|---|---|---|:--:|
| 1 | `crates-ui/apeireth-ui-kit/src/lib.rs` | 加 `pub mod settings;`（子面组件的落点） | 迁移期阻塞 |
| 2 | `crates-ui/apeireth-ui-demo/src/lib.rs` | 加 `pub mod settings_fixture;`（fixture 生成物） | 迁移期阻塞 |
| 3 | `crates-ui/apeireth-ui-demo/src/main.rs` | 加 `__APP_MODE__` 的设置页分支（如 `"settings:provider"`）⇒ 挂载对应子面 | 迁移期阻塞 |
| 4 | `crates-ui/apeireth-ui-kit/src/icon_data.rs` | **生成物**：新增本面图标（清单见 §9）⇒ 由抽取器生成 | 迁移期阻塞 |
| 5 | `crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs` | 扩「按子面分片 + 多根 class 作用域化」（现版只支持单组件单根） | 否（本面已用自含抽取器骨架绕开） |
| 6 | `crates-ui/apeireth-ui-demo/harness/scripts/runtime-mock-plugin.mjs` + `scripts/gen-runtime-mock.mjs` + `mock/` | 参考侧要 mock **6 个**生产模块：`runtime` / `tauri-bridge` / `desktop-bridge` / `user-profile` / `user-impression` / `bg-store`（现版只拦 `runtime`） | 否（本面已用自含 mock 插件绕开） |
| 7 | `crates-ui/apeireth-ui-demo/harness/vite.config.mjs` | 若要把设置页并进**统一** harness：加 `rollupOptions.input` 条目（每子面 2 条） | 否 |
| 8 | `crates-ui/apeireth-ui-demo/harness/build.sh` | 若并进统一构建：加「抽取设置页资产 → 对照」两步 | 否 |
| 9 | `crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`（图标段） | 图标清单扩容（§9）⇒ 生成 `icon_data.rs` | 迁移期阻塞 |
| 10 | `.github/workflows/ui-engine-ci.yml` | 若要把设置页对照纳入 CI | 否 |

> **本目录已自带绕开 5/6/7/8 的自含装置**（`vite.config.mjs` / `mock/` / `scripts/`）
> ⇒ 迁移线**不必等主线**就能开工；等主线统一时再合并。

---

## 9. 对照装置（本目录已建的骨架）

```
harness/settings/
├── README.md                  ← 本文件（切分方案全文）
├── SPLIT.md                   ← 可执行迁移序 + 每面元数据表
├── vite.config.mjs            ← 自含 vite 配置（每面 2 条入口 + 2 张总览页）
├── build.sh                   ← 自含构建（入口页 → 替身 → 抽取 → wasm → vite → 对照）
├── package.json               ← 脚本入口（依赖复用上级 harness 的 node_modules）
├── .gitignore                 ← 产物忽略 + `*.py` 放行（与批一 harness 同口径）
├── fixtures/                  ← 固定数据草案（8 份 JSON + 说明）
│   ├── README.md
│   ├── nav-groups.json        （S1）5 大类 / 15 分区 / 图标 / 初始展开态
│   ├── capabilities.json      （S3+S4）8 个开关注册表 + 39 个开关初值
│   ├── provider.json          （S2）协议预设 + 连接态 + 回显 + 钥匙串位
│   ├── character.json         （S3）人设 + 印象 + 预设 + 学习日志
│   ├── budget.json            （S5）消耗 + 徽标 + 余量 + 配额 + 诊断
│   ├── data.json              （S6）运维读数 + 工作区 + 用户资料
│   ├── theme.json             （T1）10 主题 + 4 配色（swatch 原文）
│   └── wizard.json            （W1）6 步骤 + 5 预设 + 介绍文案
├── scripts/
│   ├── gen-entry-pages.mjs        ← 按子面注册表生成两侧入口页（16 页）
│   ├── gen-settings-mock.mjs      ← 按生产导出面生成 6 个数据面替身
│   └── extract-settings-assets.mjs← 按子面分片抽取 scoped CSS + 复制设计层
├── mock/
│   ├── settings-mock-plugin.mjs   ← 拦生产 6 个数据面模块（只读替换）
│   ├── real/<模块>.js             ← 喂固定 fixture 的真实现（手写）
│   └── <模块>.js                  ← **生成物**（真实现 + 显式报错桩）
├── tools/
│   └── compare_settings.py        ← 对照脚本骨架（像素 / DOM / 宽度同源 / JSON）
├── public/
│   ├── settings-test.css          ← 对照专用样式（**手写并入库**，两侧逐字相同）
│   ├── settings-<面>.css          ← **生成物**（按子面分片 + 根 class 作用域化）
│   └── design/*.css               ← **生成物**（只读复制生产的共享设计层）
└── web/
    ├── shared/panels.js           ← **子面注册表（两侧单一真源）**
    ├── shared/settings-api.js     ← 两侧同名同义的 API 骨架
    ├── reference/
    │   ├── index.html + main.js   ← 总览页
    │   ├── ReferenceSettingsApp.svelte ← 只读引入生产组件的宿主
    │   └── <面>/index.html        ← 8 张入口页占位
    └── engine/
        ├── index.html + main.js   ← 总览页
        └── <面>/index.html        ← 8 张入口页占位
```

### 9.0.1 对照脚本的**动作步骤**（还没写，但已定位清楚）

| 面 × 分区 | 触发动作 | 裁剪 |
|---|---|---|
| S2 / `api-key-modal` | 点 `.credential-row` 里的「配置 Key」按钮 ⇒ 生产开 `.modal-backdrop`（`position: fixed`） | **整视口** |
| W1 / 各步骤 | 参考侧是**独立组件** ⇒ 需要在 fixture 侧驱动步骤（`stepIndex` / `tier`）⇒ 引擎侧调同一个 API | **整视口** |
| 其余 18 组 | `window.__SETTINGS__.selectSection(<分区>)` 即可 | 元素框 |

⚠️ 本批**已实测**：参考侧这 22 组**全部可达**（分区切换后标题确实切到了目标分区）。

### 9.0 本目录已跑通的部分（实测）

| # | 事项 | 结果 |
|---|---|---|
| 1 | **参考侧 vite 构建** | ✅ 通过（生产 `SettingsView` / `FirstRunWizard` / `ThemeSettingsPanel` **只读引入**并编译；生产文件零改动） |
| 2 | **数据面替身** | ✅ 6 个生产模块（`runtime` / `tauri-bridge` / `desktop-bridge` / `user-profile` / `user-impression` / `bg-store`）全部拦到替身；导出面**由生产导出面自动派生** ⇒ 链接期不缺名 |
| 3 | **按子面分片的 CSS 抽取** | ✅ 8 份 scoped CSS 产出；**自检通过**（生产每条规则至少归到一个面）；`:global` 语义按**实测编译结果**对齐（整条 global 保持全局 / 混合选择器前缀根 class） |
| 4 | **参考侧逐「面 × 分区」渲染** | ✅ 21 组全部可达，裁剪框宽度实测见 §2.3 |
| 5 | **引擎侧对照** | ❌ **未跑**（没有迁移 ⇒ 没有引擎侧产物 ⇒ 对照无对象） |

### 9.1 对照项与判据（沿用批一口径）

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每（子面 × 分区）差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（D1 结构 / D2 属性 / D3 文本必须为零；文本节点**逐字**） |
| T3 | **宽度同源断言** | 两侧裁剪框**宽度必须相等**（不等 ⇒ **直接判失败**，不比较） |
| T4 | **JSON 差分** | 引擎侧序列化器 vs 浏览器侧 `JSON.stringify`（配置 JSON 预览、运维回显）：**逐字节相同** |
| T5 | **逐帧对照** | 展开/切换动作期间每帧 **≤ 1%**（本面有条件渲染分支：密钥明文切换、模态开合、折叠展开） |

### 9.2 归一化规则（沿用批一口径）

- 剔除 Svelte 作用域 class（`svelte-<hash>`）；
- class token **排序**、属性键**排序**；
- 注释锚点**丢弃**；
- 纯空白文本节点**丢弃**；其余文本节点**逐字保留**；
- ⚠️ **本面追加**：引擎侧多出的 `.sp-*` 包裹根 class 必须在归一化时**剔除**
  （否则 D2 属性差异恒不为零）⇒ 归一化脚本要把 `sp-*` 前缀的 class token 去掉。

---

## 10. 风险总表（本批判断）

| # | 风险 | 影响面 | 严重度 | 处置 |
|---|---|---|---|---|
| R-A | **能力开关的 `invert` 语义**（`typedRecall`） | S3 | **高**（静默反向） | fixture 必须给出**两种** `typedRecall` 取值各对照一次 |
| R-B | **`requires` 依赖行**（`fileWriteAutoPass` ← `fileWrite`） | S4 | **高**（禁用态丢失） | 对照「父开 / 父关」两态 |
| R-C | **嵌套行插入点**（沙箱行 / 检索温度行插在父行旁） | S3 / S4 | 中 | 引擎侧列表渲染要支持「一项两节点」 |
| R-D | **空白折叠**（换行 ⇒ 恰好一个空格） | 全部 | 中 | 迁移前**浏览器 dump 生产 DOM**再照抄 |
| R-E | **内联 style**（余量条宽度 / 主题 swatch / 配色变量） | S5 / T1 | 中 | 逐字一致；渐变字符串由 fixture 提供 |
| R-F | **`max={undefined}` ⇒ 属性不出现** | S5 | 中 | 属性对照要能区分「缺失」与「空串」 |
| R-G | **原生控件外观**（`select` / `range` / `number` / `password`） | S2 / S3 / S4 / S5 | 中 | 必须用**原生元素**，不自绘 |
| R-H | **空态 / 降级态**（`—` 占位、「仅桌面版可读」、空目录、无匹配模型） | S2 / S5 / S6 | 中 | fixture 必须覆盖「有数据」与「无数据」两态 |
| R-I | **危险动作**误实现（引擎侧直接删） | S3 / S6 | **高**（越权） | 引擎只发意图；执行在宿主 |
| R-J | **参考侧的 6 个模块 mock 不全** | 全部 | 中 | 迁移前先跑通 mock（§8-6） |
| R-K | **子面 CSS 分片遗漏公共规则** | 全部 | 中 | 抽取后跑一次「规则覆盖自检」：生产 CSS 的每条规则必须出现在**至少一个**子面文件里 |
| R-L | **`developer` 是 `{:else}` 兜底分支** | S4 | 低 | 保持兜底语义 |
| R-M | **头像 / 背景图的 URL 不确定**（objectURL / 压缩结果） | S6 / T1 | 中 | fixture 给**确定的** data URL |
| R-N | **`module` 段导出常量迁移会打断生产镜像测试** | S3 | 中 | 不迁（§6-N20） |

---

## 11. 诚实边界

- 本批**没有跑过任何像素对照**（无迁移 ⇒ 无引擎侧产物）⇒ §9 的判据是**设计**，不是实测。
  ⇒ 但**参考侧**已逐组实测（§2.3）：8 个面 / 21 组「面 × 分区」的裁剪框尺寸与
  分区切换**全部可达** ⇒ 切分方案的「可对照」这一条（J1）**有实测支撑**。
- 本批**没有改任何生产前端文件**（`frontend/companion-desktop/**` 全程**只读**）。
- 本批**没有碰共享文件**（§8 的清单**只报不改**）。
- 本批**没有 commit**、**没有 `git add -A`**。
- 切分方案的**行号区间**全部来自本批对生产文件的实测（`SettingsView.svelte`
  5,314 行 / `FirstRunWizard.svelte` 610 行 / `ThemeSettingsPanel.svelte` 447 行）。
- 「可信度」评级是**基于渲染确定性的判断**，不是实测分数。
