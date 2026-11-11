# 设置页三页 · 可执行迁移序 + 每面元数据表

> **配套文件**：[`README.md`](./README.md)（切分方案全文：边界 / 依赖 / 对照方式 /
> 可信度 / 风险 / 引擎不迁清单 / 共享文件需求）。
>
> **本文件是给迁移线直接照着做的清单**：先做哪一面、为什么、每一面动哪些行、
> 要哪些图标、要哪些 fixture 字段、CSS 根 class 是什么。
>
> **口径**：本批**未做任何迁移**（无 Rust 产出、无像素对照结果）。
> 下表的「行号区间」「图标」「fixture 字段」「根 class」全部来自对生产文件的
> **实测**；「建议模块名」是**建议**（落到 `ui-kit` / `ui-demo` 的共享 crate ⇒
> 共享文件的改动**由主线统一做**，见 README §8）。

---

## 1. 迁移序（8 步）

### 1.0 序的形状

```
M0 前置（共享文件，主线）      ┐
  ├─ 抽取器扩「按子面分片」      │
  └─ 参考侧 6 模块 mock 齐      ┘
        │
        ▼
M1  W1 向导      （610 行 / 标记 339）  ← 最小完整闭环
M2  T1 主题面板  （447 行 / 标记 231）  ← 引出「内联样式逐字」硬要求
M3  S1 框架导航  （72 + 导航表）        ← 给后面所有面提供对照舞台
        │
        ├──────────────┬──────────────┐  （M4 / M5 可并行）
        ▼              ▼              │
M4  S5 预算配额   M5  S6 数据用户      │
   （173 行）       （189 行）          │
        │              │              │
        └──────┬───────┘              ▼
               ▼                 M6  S3 人格记忆  （496 行）← 能力开关卡在此成型
        M7  S4 能力治理（448 行）← 复用 M6 的卡组件
               │
               ▼
        M8  S2 服务商连接（344 + 模态 63）← 宿主边界最重 ⇒ 最后
```

### 1.1 为什么是这个序（每一步一条依据）

| 步 | 面 | 行数（标记） | 为什么排这里 |
|---|---|--:|---|
| **M0** | 前置 | — | **不修 mock 与抽取器，后面每一步都跑不起来**：参考侧要喂固定数据，就必须拦 `runtime` / `tauri-bridge` / `desktop-bridge` / `user-profile` / `user-impression` / `bg-store` 六个模块（现版只拦 `runtime`）；抽取器现版只支持「单组件单根 class」（`scripts/extract-assets.mjs:69-83`）⇒ **6 个子面共用一个 `<style>` 块 ⇒ 必须先能分片**。⇒ **M0 是共享文件 ⇒ 主线统一做**（README §8-5/6）。 |
| **M1** | **W1 向导** | 339 | **最小完整闭环**：它是独立组件（不需要 `initialSection` 技巧），DOM 根是 `position: fixed` 的整屏 ⇒ 像素对照最简单；写域 100% 本地（与其它面零交集）⇒ **先把「参考侧只读引入 + 引擎侧独立挂载 + 整视口像素对照」这条链跑通**，再碰复杂的。 |
| **M2** | **T1 主题面板** | 231 | 第二个独立组件。**它把一条硬要求提前暴露出来**：主题 swatch 是**内联渐变字符串**、配色样品是**内联 CSS 变量覆写** ⇒ 「内联样式必须逐字一致」（设计文档 §9.2 C3）在第二个面就撞上 ⇒ 早撞早定规矩（第三条很便宜，第八条很贵）。 |
| **M3** | **S1 框架导航** | 72 + 导航表（~200 script） | **所有内容面的对照都依赖它**：内容列宽 **828px**（`max-width: 900px` − `padding: 36px`×2，border-box）必须两侧同源 ⇒ 引擎侧的舞台宽度从这里钉死。⇒ **S1 不做，后面每一面的像素对照都会因宽度不同而系统性失败。** |
| **M4** | **S5 预算配额** | 173 | 内容面里**风险最低**的一块：几乎全是纯展示 + 一个**纯函数**算式（`budgetRemainingRows` / `effectiveBudget`）⇒ 验证「派生值由宿主喂、引擎只渲染」这条路。⚠️ 唯一技术点是余量条的**内联宽度**（`Math.round(ratio*100)`）。 |
| **M5** | **S6 数据用户** | 189 | 读数渲染 + **空态分支**（`—` 占位 + 「仅桌面版可读」）⇒ 把「0 装诚实口径」的两种形态一起对照上。⚠️ 危险区**只发意图不执行**。⇒ **M4 / M5 可并行**（写域不相交）。 |
| **M6** | **S3 人格记忆** | **496** | **全批技术风险最高点**：能力开关卡组件在这里成型（8 个注册表、`invert` 反向旋钮、`requires` 依赖行、嵌套行插入点）。⇒ 放在框架与两个低风险面都绿了之后 —— **先建通路，再攻难点**。 |
| **M7** | **S4 能力治理** | **448** | **复用 M6 成型的卡组件** ⇒ 边际成本最低（本批最大的一块却最省）。⚠️ `security` 分区**不迁**（外部 `GovernanceView`）⇒ 实际 432 行。⇒ M6 的卡组件 API 冻结后**可与 M6 的面板工作并行**。 |
| **M8** | **S2 服务商连接** | 344 + 63 模态 | **宿主边界最重的一块**：HTTP 测连接 / 模型发现 / 网关热应用回显 + 钥匙串 + 整组提交原子 + API Key 模态 ⇒ 放最后，前面的收获（约定、抽取器、对照脚本）都已就位。 |

### 1.2 并行度

| 时刻 | 可并行的面 | 前提 |
|---|---|---|
| M3 完成后 | M4 ∥ M5 | 写域不相交（`capabilities` 键集不相交、`textDirty` 命名空间不相交） |
| M6 的**卡组件 API**冻结后 | M6 的面板部分 ∥ M7 | M7 的数据模型 = M6 的卡组件 props 形状 |
| M7 完成后 | M8 ∥ （M1/M2 的逐帧对照补测） | M8 需要 M7 的「能力开关 + 嵌套行」已稳定 |

### 1.3 每一步的**完成判据**（不是「看起来对」）

| 步 | 完成判据 |
|---|---|
| M0 | ① 参考侧页面在浏览器里**不产生任何网络请求**（mock 生效）；② 抽取器产出 6+2 份 CSS，`--check` 模式无漂移；③ **规则覆盖自检**：生产 `<style>` 的每条规则**至少出现在一份**子面文件里 |
| M1 | 对照报告：像素差 ≤ 1%（6 个步骤 × 2 条分流分支）；DOM 全等；**逐帧**（步进切换）≤ 1% |
| M2 | 像素差 ≤ 1%（10 主题卡 + 4 配色卡 × 选中/未选）；**内联样式逐字相等**（swatch 渐变串 + CSS 变量覆写） |
| M3 | 侧栏裁剪（200px 宽）与内容列裁剪（828px 宽）两侧**宽度断言相等**；15 分区图标**逐个 class 相同** |
| M4 | 像素 ≤ 1%；**JSON/数值**：余量条的 `Math.round(ratio*100)` 两侧**逐字节相同** |
| M5 | 像素 ≤ 1%（有读数 / 无读数两态）；危险区**只发意图**（断言 DOM 里没有删除副作用） |
| M6 | 像素 ≤ 1%；**`typedRecall` 的 `invert` 两态**对照；**`fileWrite`/`fileWriteAutoPass` 的 `requires` 两态**对照；**嵌套行插入点**的 DOM 结构断言 |
| M7 | 同上 + `select`/`range`/`number` 的**原生控件属性逐字相等**；配置 JSON 预览**逐字节相同** |
| M8 | 像素 ≤ 1%；**密钥明文切换**的结构断言（只产出**一个**图标节点）；整组提交的**原子性**断言；模态按**整视口**对照 |

---

## 2. 每面元数据表

### 2.1 总表

| 面 | 源文件 | 行号区间 | 依赖 | Rust 模块（建议） | CSS 根 class |
|---|---|---|---|---|---|
| **W1** | `frontend/companion-desktop/src/lib/components/FirstRunWizard.svelte` | script 1–186 / markup 188–338 / style 340–610 | 无（宿主：`setProviderKey` / `writeSelfDescription` / `lib/onboarding.ts`） | `ui_kit::input::FirstRunWizard` | `.wizard-backdrop` |
| **T1** | `frontend/companion-desktop/src/lib/components/ThemeSettingsPanel.svelte` | script 1–113 / markup 115–230 / style 232–447（**在生产里裹在 `SettingsView` 的 `appearance` 分支里** ⇒ 参考侧走 `SettingsView`） | 无（宿主：`lib/theme.ts` / `lib/bg-store.ts`） | `ui_kit::settings::ThemeSettingsPanel` | `.sp-theme` |
| **S1** | `frontend/companion-desktop/src/lib/views/SettingsView.svelte` | markup 1922–1993；script 176–240 / 1309–1369 | 无 | `ui_kit::settings::SettingsFrame` | `.settings-view` |
| **S5** | 同上 | markup 3201–3374；script 615–709 | S1 | `ui_kit::settings::BudgetPanel` | `.sp-budget` |
| **S6** | 同上 | markup 3376–3565；script 1247–1284 / 1536–1597 / 1715–1755 / 1775–1819 | S1 | `ui_kit::settings::DataPanels` | `.sp-data` |
| **S3** | 同上 | markup 2364–2862；script 555–613 / 711–779 / 1050–1107 / 1176–1184 / 1821–1919 | S1 | `ui_kit::settings::CharacterPanels` | `.sp-character` |
| **S4** | 同上 | markup 2864–3021 + 3023–3199 + 3567–3680；script 256–553 / 396–423 / 1234–1238 / 1757–1773 | S1 + S3 的卡组件 | `ui_kit::settings::CapabilityPanels` | `.sp-capability` |
| **S2** | 同上 | markup 2019–2362 + 3687–3749；script 781–948 / 985–1048 / 1186–1233 / 1371–1455 / 1482–1542 / 1620–1704 | S1 | `ui_kit::settings::ProviderGroup` | `.sp-provider` |

### 2.2 W1 —— 首次运行向导

| 项 | 内容 |
|---|---|
| **源文件行号** | script 1–186；markup 188–338；style 340–610（文件共 610 行） |
| **依赖的其它子面** | **无**（独立组件） |
| **Rust 模块名（建议）** | `ui_kit::input::FirstRunWizard`（设计文档 §8.2 已列此落点） |
| **图标清单**（生产 import 名 ⇒ 真实 class 后缀） | `Sparkles` ⇒ `lucide-sparkles`；`ChevronLeft` ⇒ `lucide-chevron-left`；`ChevronRight` ⇒ `lucide-chevron-right`；`Check` ⇒ `lucide-check` |
| **fixture 字段** | `steps[]`（步骤 id 序列）、`stepTitles{}`（6 个 id ⇒ `{title, sub}`）、`presets[]`（5 条：`id` / `name` / `baseUrl` / `model` / `hint`）、`introBullets[]`、`introPlan[]`、`proGuideLines[]`、`tier`（`casual` / `pro` / 未选）、`stepIndex`、`presetId`、`apiKey`、`baseUrl`、`model`、`selfDescription`、`autoTune`、`error` |
| **CSS 根 class** | `.wizard-backdrop`（根自身规则原样；子规则前缀） |
| **DOM 根 / 裁剪** | `.wizard-backdrop` 是 `position: fixed; inset: 0` ⇒ **整视口截图**（不裁剪） |
| **写域** | 10 个本地状态，零共享 |
| **⚠️ 注意点** | ① 步骤条 `{#if i < stepIndex}` 是**结构分支**（`Check` 图标 vs 序号文本），不是 class 分支；② `onComplete` / `onSkip` 是**意图出口**（引擎侧不实现 `buildWizardConfig` 的写面）；③ 密钥写入钥匙串 ⇒ 宿主侧 |

### 2.3 T1 —— 主题外观面板

| 项 | 内容 |
|---|---|
| **源文件行号** | script 1–113；markup 115–230；style 232–447（文件共 447 行） |
| **依赖的其它子面** | **无**（但**参考侧**借用生产的 `SettingsView` 的 `appearance` 分支 ⇒ 那一支的壳（`.setting-block` + 标题 + 说明）也是引擎侧要产出的） |
| **Rust 模块名（建议）** | `ui_kit::settings::ThemeSettingsPanel`（设计文档 §8.2：**零改写**，只操作 `html[data-theme]`） |
| **图标清单** | `Check` ⇒ `lucide-check`；`Upload` ⇒ `lucide-upload`；`ImageOff` ⇒ `lucide-image-off` |
| **fixture 字段** | `themeCatalog[]`（10 条：`id` / `label` / `desc` / `swatch`——**swatch 必须是生产那串渐变原文**）、`accentCatalog[]`（4 条：`id` / `label` / `desc` / `accent` / `ink`——色值原文）、`activeTheme` / `activeAccent`、`customBg`（bool）、`customPreviewUrl`（**固定的** URL ⇒ 两侧同一张图）、`bgError`、**`blockTitle` / `blockDesc`**（外壳的标题与说明 ⇒ 见 fixture 待评审项） |
| **CSS 根 class** | **`.sp-theme`**（⚠️ 生产有一条**混合**选择器 `.bg-actions :global(.quiet-btn)` ⇒ 实测编译成 `.bg-actions.svelte-x .quiet-btn` ⇒ 抽取器对它前缀根 class） |
| **DOM 根 / 裁剪** | 参考侧 `.setting-block`（**实测宽 828px**）；引擎侧 `.sp-theme .setting-block` |
| **写域** | `bgBusy` / `bgError` / `customPreview` / `fileInput` / `showClearBgConfirm`，零共享 |
| **⚠️ 注意点** | ① `style:background={item.swatch}` ⇒ **内联样式逐字**；② `style:--ap-accent-ui` / `style:--ap-accent-ui-ink` ⇒ **自定义属性内联声明**要逐字；③ `URL.createObjectURL` 每次不同 ⇒ fixture 给固定 URL；④ 主题/配色切换 = 写 `<html>` ⇒ **引擎不碰**（设计文档 §8.3 规则 5）⇒ 只渲染选中态 |

### 2.4 S1 —— 页面框架与导航

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 1922–1993（72 行）；script 176–240（props / `activeSection` / `expandedGroups`）、1309–1369（`sections` / `GROUP_ICONS` / `navGroups` / `activeGroup` / `lastSectionByGroup` / `selectSection` / `toggleGroup`） |
| **依赖的其它子面** | **无**（最底层） |
| **Rust 模块名（建议）** | `ui_kit::settings::SettingsFrame` |
| **图标清单** | 导航大类（`GROUP_ICONS`）：`Cpu` ⇒ `lucide-cpu`；`Sparkles` ⇒ `lucide-sparkles`；`ShieldCheck` ⇒ `lucide-shield-check`；`Palette` ⇒ `lucide-palette`；`Archive` ⇒ `lucide-archive`。<br>分区（`sections` 表 15 个）：`CircleUserRound` ⇒ `lucide-circle-user-round`；`Palette` ⇒ `lucide-palette`；`Smile` ⇒ `lucide-smile`；`Cpu` ⇒ `lucide-cpu`；`User` ⇒ `lucide-user`；`Eye` ⇒ `lucide-eye`；`Brain` ⇒ `lucide-brain`；`Sparkles` ⇒ `lucide-sparkles`；`Scale` ⇒ `lucide-scale`；`ShieldCheck` ⇒ `lucide-shield-check`；`Wrench` ⇒ `lucide-wrench`；`Gauge` ⇒ `lucide-gauge`；`Activity` ⇒ `lucide-activity`；`Trash2` ⇒ `lucide-trash-2`；`Code` ⇒ `lucide-code`。<br>框架内直接元素：`Check` ⇒ `lucide-check`；`RotateCcw` ⇒ `lucide-rotate-ccw`；`ChevronDown` ⇒ `lucide-chevron-down`。 |
| **fixture 字段** | `navGroups[]`（5 组，**顺序与生产逐字相同**：`account` / `character` / `capability` / `appearance` / `data`；每组 `id` / `title` / `blurb` / `sectionIds[]`）、`sections[]`（15 条：`id` / `label` / `icon`）、`groupIcons{}`（5 条）、`activeSection`、`expandedGroups[]`、`lastSectionByGroup{}` |
| **CSS 根 class** | `.settings-view` |
| **DOM 根 / 裁剪** | 侧栏 `.settings-subnav`（宽 **200px**）；内容列 `.settings-content`（宽 **828px**，含 `.category-context`） |
| **写域** | `activeSection` / `expandedGroups` / `lastSectionByGroup`（**S1 独占写**）；`liveApplyPendingKey` / `liveApplyError` / `textDirty` / `textAck`（**宿主写，S1 只读**） |
| **⚠️ 注意点** | ① `lastSectionByGroup` 在生产是**非响应式 `Map`** ⇒ 迁移时**保持非响应式**（否则多一次重排）；② `fieldFlag` 片段（未保存 / ✓）依赖宿主的 `textDirty`/`textAck` ⇒ **S1 只渲染，不拥有**；③ `PageHeader` 是外部组件（23 行）⇒ S1 只放占位 |

### 2.5 S5 —— 预算与配额 + 运行时诊断

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 3201–3342（`budget`）+ 3344–3374（`runtime`）；script 615–709（旋钮行注册表 / 草稿 / 提交 / 派生） |
| **依赖的其它子面** | **S1**（框架 / 舞台宽度） |
| **Rust 模块名（建议）** | `ui_kit::settings::BudgetPanel` |
| **图标清单** | 直接元素：`Activity` ⇒ `lucide-activity`；`RotateCcw` ⇒ `lucide-rotate-ccw`；`Info` ⇒ `lucide-info`；`SlidersHorizontal` ⇒ `lucide-sliders-horizontal`；`Timer` ⇒ `lucide-timer`；`Gauge` ⇒ `lucide-gauge`。<br>旋钮行（`BUDGET_KNOB_ROWS`）：`Timer` ⇒ `lucide-timer`；`Wrench` ⇒ `lucide-wrench`；`Layers3` ⇒ **`lucide-layers`**（不是 `lucide-layers-3`！生产 import 名 `Layers3` ⇒ 别名 `layers-3` ⇒ 真实文件 `layers.svelte`） |
| **fixture 字段** | `sessionTotals`（`turns` / `tokens` / `cache` / `duration`）、`budgetBadges{}`（3 键 × `{label, source, note}`）、`budgetDraft{}`（3 键的**原始字面量**字符串）、`budgetFeedback{}`（3 键）、`budgetRemaining[]`（每行 `key` / `label` / `scope` / `cap` / `used` / `remaining` / `ratio` / `note`）、`quotaDimensions[]`（4 行 `key` / `label` / `reason` / `evidence`）、`budgetExhaustion{behaviors[]}`、`runtimeReport`（`overall` / `latencyMs` / `subsystems[]`：`name` / `endpoint` / `status`）、`checkingRuntime` |
| **CSS 根 class** | `.sp-budget` |
| **DOM 根 / 裁剪** | `.setting-block`（分区内）⇒ 裁剪 828px 宽 |
| **写域** | `budgetDraft` / `budgetFeedback` / `runtimeReport` / `checkingRuntime`（S5 独占）；`capabilities` 的键 `maxTurnRounds` / `maxToolCalls` / `contextBudgetChars`（S5 独占键集）；`textDirty` 键 `budget.*` |
| **⚠️ 注意点** | ① 余量条是**内联 style**（`width: {Math.round(row.ratio * 100)}%`）⇒ 逐字；② `max={spec.max ?? undefined}` ⇒ `contextBudgetChars` 的 `max` **属性不出现**（不是空串）；③ `MetricsPanel`（205 行，内部拉 `/v1/panel/metrics`）⇒ **不迁 / 不覆盖**；④ `BUDGET_KNOB_SPECS` / `QUOTA_DIMENSIONS` / `BUDGET_EXHAUSTION` 带**文案与 env 名** ⇒ 建议**留生产侧由宿主喂**（迁它会打断生产的源码镜像测试） |

### 2.6 S6 —— 数据与用户

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 3376–3489（`data`）+ 3491–3565（`user`）；script 1247–1284（危险动作）、1536–1597（工作区）、1715–1755（运维读数 + 剪贴板）、1775–1819（用户资料 + 头像） |
| **依赖的其它子面** | **S1** |
| **Rust 模块名（建议）** | `ui_kit::settings::DataPanels` |
| **图标清单** | `Trash2` ⇒ `lucide-trash-2`；`Check` ⇒ `lucide-check`；`AlertTriangle` ⇒ **`lucide-triangle-alert`**（生产 import 名 `AlertTriangle` ⇒ 别名 `alert-triangle` ⇒ 真实文件 `triangle-alert.svelte`）；`Folder` ⇒ `lucide-folder`；`RefreshCcw` ⇒ `lucide-refresh-ccw`；`Radar` ⇒ `lucide-radar`；`Copy` ⇒ `lucide-copy`；`UserRound` ⇒ `lucide-user-round`；`ImagePlus` ⇒ `lucide-image-plus` |
| **fixture 字段** | `daemonStatus`（`state` / `ownership` / `endpoint` / `backend_version` / `pid` / `restart_count`）、`residentProbe`（`health` / `descriptor_present` / `endpoint` / `pid`）、`daemonGuidance[]`（3 条：`key` / `label` / `command` / `desc`）、`workspaceDir` / `workspaceSuggestions[]` / `workspaceError`、`userProfile`（`nickname` / `bio` / `avatar`——**avatar 必须是固定的 data URL**）、`userProfileError`、`dangerPending` / `dangerPayload` |
| **CSS 根 class** | `.sp-data` |
| **DOM 根 / 裁剪** | `.setting-block` ⇒ 裁剪 |
| **写域** | `daemonStatus` / `residentProbe` / `daemonLoading` / `opsCopiedKey` / `opsCopyError` / `workspace*` / `dangerPending` / `dangerPayload` / `userProfile*` / `avatarFileInput`（S6 独占） |
| **⚠️ 注意点** | ① **「读不到」分支**：`status === null && probe === null` ⇒ 4 行 `—` + 「仅桌面版可读」⇒ **必须对照**；② 危险动作**只发意图**（`DANGER_ACTION_CONFIRMATIONS` 的文案是唯一来源 ⇒ 可迁；**执行不迁**）；③ `WorkspacePickerModal`（297 行，外部组件）⇒ **不迁**；④ 头像压缩（256px JPEG）在**宿主侧** ⇒ fixture 给确定 data URL |

### 2.7 S3 —— 人格与记忆

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 2364–2454（`personality`）+ 2456–2520（`impression`）+ 2522–2662（`cognition`）+ 2664–2862（`disposition`）＝ **496 行**；script 555–613（体验参数 4 旋钮 + 预设）、711–779（学习日志）、1050–1107（人设组）、1176–1184（脏标记判定）、1821–1919（用户印象） |
| **依赖的其它子面** | **S1**（框架）；⚠️ 与 **S4** 共享 `capabilities` 容器（**键集不相交** ⇒ 可并行） |
| **Rust 模块名（建议）** | `ui_kit::settings::CharacterPanels` + **`ui_kit::settings::CapabilityCard`**（卡组件在此成型 ⇒ S4 复用） |
| **图标清单** | 直接元素：`Layers3` ⇒ `lucide-layers`；`Trash2` ⇒ `lucide-trash-2`；`RotateCcw` ⇒ `lucide-rotate-ccw`；`Sparkles` ⇒ `lucide-sparkles`；`CheckCircle2` ⇒ **`lucide-circle-check`**（生产 `CheckCircle2` ⇒ 别名 `check-circle-2` ⇒ 真实文件 `circle-check.svelte`）；`Info` ⇒ `lucide-info`；`Plus` ⇒ `lucide-plus`；`Brain` ⇒ `lucide-brain`；`HeartHandshake` ⇒ `lucide-heart-handshake`；`GitBranch` ⇒ `lucide-git-branch`；`SlidersHorizontal` ⇒ `lucide-sliders-horizontal`；`RefreshCcw` ⇒ `lucide-refresh-ccw`；`Radar` ⇒ `lucide-radar`；`Timer` ⇒ `lucide-timer`；`Archive` ⇒ `lucide-archive`。<br>注册表（`<def.icon>`）：`MessageSquarePlus` ⇒ `lucide-message-square-plus`；`Network` ⇒ `lucide-network`；`Users` ⇒ `lucide-users`；`BookOpen` ⇒ `lucide-book-open`；`Dumbbell` ⇒ `lucide-dumbbell`；`Landmark` ⇒ `lucide-landmark`。 |
| **fixture 字段** | `personas[]`（`id` / `name` / `model` / `persona`）、`activePersonaId`、`impression`（`personaId` / `personaName` / `summary` / `updatedAt` / `source` / `autoRefresh`）、`capabilities`（本面键集：`memoryInjection` / `consolidation` / `reflexion` / `proactiveRecall` / `typedRecall` / `organs` / `preferenceLearning` / `partnerBond` / `morphologyRecall` / `education` / `absorptionInsight` / `communityTriage` / `oneringLedger` / `memoryFade` / `curiosityStrength` / `toneSaturation` / `consolidationCadence` / `selfTuning`）、`capDefs`（3 组：`memoryFlow` 5 / `cognition` 6 / `community` 2；每条 `key` / `icon` / `label` / `env` / `desc` / `invert?` / `requires?`）、`dispositionPresets[]`（3 条 + 「恢复基线」）、`tuningLog[]`（`seq` / `param` / `previous` / `next` / `reason` / `at_epoch_ms`）、`tuningParamLabels{}` |
| **CSS 根 class** | `.sp-character` |
| **DOM 根 / 裁剪** | `.setting-block` ⇒ 裁剪（**四个分区各一次对照**） |
| **写域** | `personas` / `activePersonaId` / `personasCommitted` / `impression*` / `tuningLog` / `tuningLogLoading` + `capabilities` 的本面键集 + `textDirty` 键 `persona.*` / `user.impression` |
| **⚠️ 注意点** | ① **`invert`**：`typedRecall` 的 env 是 `APEIRETH_DISABLE_TYPED_RECALL` ⇒ 「开」= `false` ⇒ 判反了就是**静默反向**（本批最高危）；② **`requires`**：`fileWriteAutoPass` ← `fileWrite`（在 S4）⇒ 父行状态驱动子行禁用态；③ **嵌套行插入点**：检索温度行插在 `morphologyRecall` 父行**旁**（同层）⇒ 列表渲染要支持「一项两节点」；④ 人设卡的 `id` 是**模板串**（`persona-name-{p.id}`）⇒ 逐字；⑤ `DISPOSITION_PRESETS` / `DISPOSITION_BASELINE_RESET` 是 **`module` 段导出** ⇒ **不迁**（生产有源码正则镜像测试）⇒ 由宿主喂 |

### 2.8 S4 —— 能力与治理

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 2864–3004（`governance`）+ 3006–3021（`security`，**不迁**）+ 3023–3199（`tools`）+ 3567–3680（`developer`）＝ 448 行（**可迁 432 行**）；script 256–553（认知深度 + 8 个注册表 + 旋钮 setter + 推荐预设）、396–423（文本旋钮提交）、1234–1238（权限预设）、1757–1773（配置 JSON 复制） |
| **依赖的其它子面** | **S1** + **S3 的 `CapabilityCard` 组件**（⇒ M6 冻结后即可开工） |
| **Rust 模块名（建议）** | `ui_kit::settings::CapabilityPanels`（**复用** `ui_kit::settings::CapabilityCard`） |
| **图标清单** | 直接元素：`Server` ⇒ `lucide-server`；`AlertTriangle` ⇒ `lucide-triangle-alert`；`CheckCircle2` ⇒ `lucide-circle-check`；`Info` ⇒ `lucide-info`；`Brain` ⇒ `lucide-brain`；`Scale` ⇒ `lucide-scale`；`Wrench` ⇒ `lucide-wrench`；`Users` ⇒ `lucide-users`；`ShieldCheck` ⇒ `lucide-shield-check`；`ShieldOff` ⇒ `lucide-shield-off`；`Terminal` ⇒ `lucide-terminal`；`Timer` ⇒ `lucide-timer`；`Check` ⇒ `lucide-check`；`Sparkles` ⇒ `lucide-sparkles`；`Filter` ⇒ `lucide-filter`；`Tag` ⇒ `lucide-tag`；`Copy` ⇒ `lucide-copy`。<br>注册表（`<def.icon>` / `<subDef.icon>`）：`Scale` / `Users` / `ShieldCheck` / `Workflow` ⇒ `lucide-workflow` / `Terminal` / `Globe` ⇒ `lucide-globe` / `FolderSearch` ⇒ `lucide-folder-search` / `FilePenLine` ⇒ `lucide-file-pen-line` / `FileCheck2` ⇒ `lucide-file-check-2` / `Brain` |
| **fixture 字段** | `capabilities`（本面键集：`judge` / `council` / `councilAdvisors` / `councilTimeoutMs` / `onionLayer` / `worktreeSandbox` / `shell` / `shellSandbox` / `fetch` / `localReadTools` / `fileWrite` / `fileWriteAutoPass` / `reasoningEnabled` / `reasoningModelFilters` / `reasoningTag`）、`capDefs`（4 组：`decision` 2 / `guard` 2 / `tool` 4 / `toolSub` 1 / `beta` 1）、`cognitiveDepth`（`light` / `balanced` / `deep` / `custom`）、`allowedServers[]` / `allowedServerInput` / `allowedServerError`、`permissionPreset`（`read_only` / `standard` / `full`）、`configJsonPreview`（**由宿主用 `JSON.stringify` 生成** ⇒ 键序 `baseUrl` → `model` → `provider` → `hasApiKey`，缩进 2）、`configJsonCopied` |
| **CSS 根 class** | `.sp-capability` |
| **DOM 根 / 裁剪** | `.setting-block` ⇒ 裁剪（**三个分区各一次对照**；`security` 不对照） |
| **写域** | `cognitiveDepth` / `allowedServers*` / `permissionPreset` / `configJsonCopied` / `reasoningGroupEl` + `capabilities` 的本面键集 + `textDirty` 键 `council.timeoutMs` / `reasoning.*` |
| **⚠️ 注意点** | ① **`select` 用原生元素**（不自绘）；② **配置 JSON 用 `ui-kit/src/json.rs` 的 `JsonValue`**（保插入序）+ `num.rs` 的 ECMAScript 数值语义 ⇒ **不手拼字符串**；③ `developer` 是 **`{:else}` 兜底分支** ⇒ 保持兜底语义；④ `permissionPreset` 落 `localStorage` ⇒ **引擎不碰**（宿主侧）；⑤ 剪贴板复制 ⇒ **不覆盖**；⑥ `security` 分区整段委派 `GovernanceView`（145 行）⇒ **不迁**（治理卷宗面） |

### 2.9 S2 —— 服务商与连接

| 项 | 内容 |
|---|---|
| **源文件行号** | markup 2019–2362（`models`）+ 3687–3749（API Key 模态）＝ 407 行；script 781–842（预设表）、892–948（协议/预设/模型/密钥态 + 双缓冲）、985–1048（整组草稿与提交）、1186–1233（网关地址 + 钥匙串态 + apply 回显 + 模型发现）、1371–1455（协议切换 / 预设 / 测连接 / family / 打码）、1482–1542（模型加载 / 打开弹层 / 删密钥）、1620–1704（apply 缝 + 写新密钥） |
| **依赖的其它子面** | **S1** |
| **Rust 模块名（建议）** | `ui_kit::settings::ProviderGroup` |
| **图标清单** | `Server` ⇒ `lucide-server`；`Globe` ⇒ `lucide-globe`；`Sparkles` ⇒ `lucide-sparkles`；`Eye` ⇒ `lucide-eye`；`EyeOff` ⇒ `lucide-eye-off`；`Lock` ⇒ `lucide-lock`；`Check` ⇒ `lucide-check`；`CheckCircle2` ⇒ `lucide-circle-check`；`XCircle` ⇒ **`lucide-circle-x`**（生产 `XCircle` ⇒ 别名 `x-circle` ⇒ 真实文件 `circle-x.svelte`）；`ChevronDown` ⇒ `lucide-chevron-down`；`ChevronUp` ⇒ `lucide-chevron-up`；`Search` ⇒ `lucide-search`；`RotateCcw` ⇒ `lucide-rotate-ccw`；`Trash2` ⇒ `lucide-trash-2`（模态里的删除按钮） |
| **fixture 字段** | `presets`（`openai` 5 条 + `anthropic` 3 条；每条 `id` / `name` / `baseUrl` / `defaultModel` / `models[]`）、`activeProtocol`（`openai` / `anthropic`）、`activePreset`、`providerBaseUrl` / `providerApiKey` / `providerModel` / `anthropicVersion` / `showApiKey`、`discoveredModels[]`（`id` / `ownedBy`）、`modelsLoading` / `modelsError` / `modelSearchQuery`、`testResult`（`ok` / `message` / `latencyMs?` / `models[]`）、`applyResult`（`ok` / `warnings[]`）、`effectiveConfig`（`provider` / `base_url` / `api_key` / `model`）、`editBaseUrl` / `showAdvancedGateway`、`hasApiKey`、`storedKeyMasked` / `storedKeyExists` / `keychainLoading` / `keychainSaving` / `keychainDeleting` / `keychainActionError` / `tempApiKey` |
| **CSS 根 class** | `.sp-provider` |
| **DOM 根 / 裁剪** | `.setting-block` ⇒ 裁剪；**API Key 模态 ⇒ 整视口截图**（`position: fixed`） |
| **写域** | `activeProtocol` / `activePreset` / `providerBaseUrl` / `providerApiKey` / `providerModel` / `anthropicVersion` / `showApiKey` / `openaiBuffer` / `anthropicBuffer` / `discoveredModels*` / `modelSearchQuery` / `testResult` / `isTestingConnection` / `applyResult` / `effectiveConfig` / `storedKey*` / `keychain*` / `showApiKeyModal` / `tempApiKey` / `editBaseUrl` / `showAdvancedGateway` / `gatewayCommitted`（S2 独占）+ `textDirty` 键 `provider.*` / `gateway.baseUrl` |
| **⚠️ 注意点** | ① **明文切换是结构分支**（`Eye` ↔ `EyeOff`，只产出**一个**图标节点）；② **整组提交原子性**（端点/模型/密钥/协议头一起提交）⇒ 引擎只发「提交这组」意图；③ **密钥打码口径**：`maskApiKey`（首 3 + `****` + 末 3；≤ 8 位整体 `****`）⇒ **展示口径可迁，密钥本身不迁**；④ `SessionModelPicker`（241 行，外部组件）⇒ **不迁**（S2 只放占位）；⑤ 测连接 / 模型发现 / 网关热应用 ⇒ **HTTP，不迁** |

---

## 3. 图标汇总（全面去重 + 真实 class 后缀）

> **抽取器要新增的图标别名**（批一 `ICONS` 清单要扩）：

```
activity, alert-triangle, archive, book-open, brain, check, check-circle-2,
chevron-down, chevron-left, chevron-right, chevron-up, circle-user-round,
code, copy, cpu, dumbbell, eye, eye-off, file-check-2, file-pen-line,
filter, folder, folder-search, gauge, git-branch, globe, group-icons 无,
heart-handshake, image-off, image-plus, info, landmark, layers-3,
loader-2 无, message-square-plus, network, palette, plus, radar,
refresh-ccw, rotate-ccw, scale, search, shield-check, shield-off, smile,
sliders-horizontal, sparkles, tag, terminal, timer, trash-2, triangle-alert 无,
upload, user, user-round, users, workflow, wrench, x-circle
```

⚠️ **别名 → 真实 class 的三条已知改写**（本批实测）：

| 生产 import 名 | 抽取器别名 | 真实 `.svelte` | **渲染出来的 class** |
|---|---|---|---|
| `Layers3` | `layers-3` | `layers.svelte` | `lucide-layers` |
| `AlertTriangle` | `alert-triangle` | `triangle-alert.svelte` | `lucide-triangle-alert` |
| `CheckCircle2` | `check-circle-2` | `circle-check.svelte` | `lucide-circle-check` |
| `XCircle` | `x-circle` | `circle-x.svelte` | `lucide-circle-x` |
| `FileCheck2` | `file-check-2` | `file-check-2.svelte` | `lucide-file-check-2` |
| `Trash2` | `trash-2` | `trash-2.svelte` | `lucide-trash-2` |

⇒ **class 后缀取真实文件名**（批一**已实现**（IMPLEMENTED）的跟随再导出逻辑）⇒ 上表右列就是
`icon_data.rs` 里 `ICON_*` 常量名与 `class` 字段的取值。

---

## 4. 共享的 fixture / 状态容器（**宿主侧持有，引擎只读或按键写**）

| 容器 | 键 | 写者 | 读者 |
|---|---|---|---|
| `capabilities` | 39 键（S3 19 / S4 14 / S5 3 … 见 §2.5–§2.9） | S3 / S4 / S5（**键集不相交**） | S1 / 各面 |
| `textDirty` / `textAck` | `provider.*` / `gateway.baseUrl`（S2）、`persona.*` / `user.impression`（S3）、`council.timeoutMs` / `reasoning.*`（S4）、`budget.*`（S5）、`workspace.dir` / `user.info`（S6） | 各面（**命名空间不相交**） | S1 的 `fieldFlag` |
| `liveApplyPendingKey` / `liveApplyError` | 单值 | 宿主（任何面发起 apply 时） | S1 |
| `activeSection` / `expandedGroups` / `lastSectionByGroup` | — | S1 | S1 |
| `config` / `onSave` | — | 宿主 | S2–S6（只读 + 提交意图） |

⇒ **写域不相交 ⇒ 迁移线可并行**（M4 ∥ M5、M6 ∥ M7）。

---

## 4.5 本批的实测（参考侧）

> **引擎侧未跑**（没有迁移 ⇒ 没有引擎侧产物）。**参考侧已逐组实测**。

| 事项 | 结果 |
|---|---|
| 参考侧 vite 构建 | ✅ 通过（三个生产文件**只读引入**并编译） |
| 数据面替身 | ✅ 6 个生产模块全部拦到替身；导出面自动派生 ⇒ 链接期不缺名 |
| 按子面分片的 CSS 抽取 | ✅ 8 份 scoped CSS；**自检通过**（每条生产规则至少归一个面） |
| 21 组「面 × 分区」渲染 | ✅ 全部可达；内容块恒 **828px**、侧栏 **200px**、内容列 border-box **900px**、向导遮罩 **1440×900** |
| ⚠️ API Key 模态 | 默认不出现 ⇒ 那一对**要先点「配置 Key」再裁剪**（对照脚本的动作步骤） |

---

## 5. 本批的产出边界（诚实声明）

- 本批**只做切分方案 + 骨架** ⇒ **没有一行 Rust 迁移**、**没有一次像素对照**
  （⇒ 引擎侧无产物 ⇒ 对照无对象）。
- 本批**参考侧已实测**：8 个面 / 21 组「面 × 分区」全部可达，裁剪框尺寸已量出
  （见 §4.5）⇒ 切分方案的「可对照」这一条**有实测支撑**。
- 本批**没有改任何共享文件**（清单在 README §8，**只报不改**）。
- 本批**没有改任何生产前端文件**（`frontend/companion-desktop/**` 全程只读）。
- 本批**没有 commit**、**没有 `git add -A`**。
- 本目录里的 `fixtures/*.json` 是**固定数据草案**（字段齐了、取值待评审）。
- 本目录里的 `web/**` 是**空骨架**（入口页占位 + 说明 ⇒ 不是可对照的成品）。
