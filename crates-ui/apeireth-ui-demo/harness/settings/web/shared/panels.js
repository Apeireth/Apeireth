// 设置页三页 · **子面注册表**（两侧单一真源）。
//
// ## 这个文件是什么
//
// 每一面的元数据（源文件行号区间 / 对照方式 / CSS 根 class / fixture / 图标清单 /
// 依赖）**只在这里写一遍** ⇒
//
// - 参考侧入口页（`web/reference/<id>/index.html`）由它生成；
// - 引擎侧入口页（`web/engine/<id>/index.html`）由它生成；
// - 对照脚本（`tools/compare_settings.py`）的「面 × 分区」矩阵由它派生；
// - `README.md` / `SPLIT.md` 的元数据表与它**逐条对应**。
//
// ⇒ 改一处 ⇒ 两侧与对照脚本同步 ⇒ 不会出现「文档说的面」与「跑的面」不一致。
//
// ## 纪律
//
// - **不手抄生产文案**：本文件只放**元数据**（id / 行号 / 选择器 / 文件名），
//   不放要渲染的文案与数值 ⇒ 文案与数值在 `fixtures/*.json`（数据）里。
// - **不引入业务名词到引擎**：这里的 id 是**结构名**（`frame` / `provider` / …），
//   引擎侧的 Rust 模块名建议见 `SPLIT.md` §2。
//
// ## ��容
//
// | id | 面 | 源文件 | 根 class | 裁剪 |
//---|---|---|---|---|
// | `wizard` | 首次运行向导 | `components/FirstRunWizard.svelte` | `.wizard-backdrop` | 整视口 |
// | `theme` | 主题外观面板 | `components/ThemeSettingsPanel.svelte` | `.theme-panel` | 元素框 |
// | `frame` | 页面框架与导航 | `views/SettingsView.svelte` | `.settings-view` | 元素框 |
// | `budget` | 预算与配额 | `views/SettingsView.svelte` | `.sp-budget` | 元素框 |
// | `data` | 数据与用户 | `views/SettingsView.svelte` | `.sp-data` | 元素框 |
// | `character` | 人格与记忆 | `views/SettingsView.svelte` | `.sp-character` | 元素框 |
// | `capability` | 能力与治理 | `views/SettingsView.svelte` | `.sp-capability` | 元素框 |
// | `provider` | 服务商与连接 | `views/SettingsView.svelte` | `.sp-provider` | 元素框 |

/** 生产的 `SettingsView.svelte` 相对仓库根的路径（**只读引入**）。 */
export const SETTINGS_VIEW = "frontend/companion-desktop/src/lib/views/SettingsView.svelte";

/** 生产的组件目录相对仓库根的路径（**只读引入**）。 */
export const COMPONENT_DIR = "frontend/companion-desktop/src/lib/components";

/**
 * 侧栏的**内容列宽度**（px）。
 *
 * ## 怎么来的（实测推导，不是猜的）
 *
 * - `.settings-content`：`max-width: 900px` + `padding: 24px 36px 48px`
 *   （`SettingsView.svelte:3834-3841`）；
 * - `* { box-sizing: border-box }`（`src/lib/design/base.css:251`）
 *   ⇒ `max-width` 约束的是 **border box**；
 * - ⇒ 内容盒宽 = 900 − 36 − 36 = **828px**。
 *
 * ⚠️ 引擎侧舞台必须钉成这个宽度 ⇒ 两侧裁剪框宽度才相等 ⇒ 对照脚本的
 * 「宽度同源断言」才可能通过。
 */
export const CONTENT_WIDTH = 828;

/** 侧栏宽度（px）：`.settings-layout` 的 `grid-template-columns: 200px 1fr`。 */
export const SIDEBAR_WIDTH = 200;

/** 视口（沿用批一口径）。 */
export const VIEWPORT = {width: 1440, height: 900};

/**
 * 子面注册表。
 *
 * ## 每条的字段
 *
 * | 字段 | 含义 |
 * |---|---|
 * | `id` | 面的 id（URL 目录名 / CSS 作用域后缀 / 引擎 `__APP_MODE__` 后缀） |
 * | `title` | 面的中文名（**只用于页面标题与说明**，不渲染进画面） |
 * | `source` | 生产源文件（仓库根相对路径） |
 * | `ranges` | 源文件行号区间（给人看的；`README.md` / `SPLIT.md` 逐条对应） |
 * | `scope` | **CSS 作用域前缀**（抽取器按它做前缀作用域化） |
 * | `sections[]` | 本面覆盖的分区；每条含 `id` / `ref`（参考侧裁剪选择器）/ `engine`（引擎侧裁剪选择器）/ `crop?`（覆盖默认裁剪方式） |
 * | `crop` | 默认裁剪方式：`viewport`（整视口）或 `element`（元素框） |
 * | `sections` | 本面覆盖的生产分区（`SettingsView` 的 15 个 id 之一，或独立组件的步骤） |
 * | `extraMarkup` | 额外的 class 线索来源（定义在别的文件、但渲染进本面的片段） |
 * | `cssSources` | 本面 CSS 的来源文件（默认 = `source` 自身） |
 * | `reference` | 参考侧怎么渲染（`component` = 直接挂载生产组件；`settings` = 挂生产 `SettingsView` + `initialSection`） |
 * | `engine` | 引擎侧的 `__APP_MODE__` 取值与建议的 Rust 模块名 |
 * | `fixture` | 固定数据文件（相对 `harness/settings/fixtures/`） |
 * | `icons` | 生产 import 的图标名（真实 class 后缀见 `SPLIT.md` §3） |
 * | `trust` | 对照可信度（`high` / `mid`）+ 一句话理由 |
 * | `deps` | 依赖的其它面（渲染期 / 写域） |
 */
export const PANELS = [
  {
    id: "wizard",
    markupRanges: [[188, 338]],
    cssSources: [{source: `${COMPONENT_DIR}/FirstRunWizard.svelte`}],
    title: "首次运行向导",
    source: `${COMPONENT_DIR}/FirstRunWizard.svelte`,
    ranges: "script 1-186 / markup 188-338 / style 340-610（文件共 610 行）",
    scope: ".wizard-backdrop",
    crop: "viewport",
    // ⚠️ 遮罩是 `position: fixed; inset: 0` ⇒ **整视口**对照，不裁剪。
    // 两侧的选择器相同（都是生产自己的根 class）。
    sections: [
      {id: "welcome", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
      {id: "tier", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
      {id: "api", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
      {id: "identity", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
      {id: "preset", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
      {id: "pro-guide", ref: ".wizard-backdrop", engine: ".wizard-backdrop"},
    ],
    reference: {kind: "component", component: "FirstRunWizard"},
    engine: {mode: "settings:wizard", module: "ui_kit::input::FirstRunWizard"},
    fixture: "wizard.json",
    icons: ["Sparkles", "ChevronLeft", "ChevronRight", "Check"],
    trust: {level: "high", why: "写域 100% 本地；遮罩是整屏 ⇒ 像素对照最直接"},
    deps: [],
  },
  {
    id: "theme",
    title: "主题外观面板",
    source: `${COMPONENT_DIR}/ThemeSettingsPanel.svelte`,
    ranges: "script 1-113 / markup 115-230 / style 232-447（文件共 447 行）",
    markupRanges: [[115, 230]],
    // ⚠️ 主题面板在生产里**不是整页**，而是 `SettingsView` 的 `appearance`
    // 分支里的一块（外面裹着 `.setting-block` + 标题 + 说明）⇒ 参考侧必须走
    // **生产的 `SettingsView`**（`initialSection="appearance"`）⇒ 裁剪到的
    // 才是**生产真实宽度**（828px）的那一块。
    //
    // ⇒ 因此本面还要：① 从 `SettingsView.svelte:1994-2012` 取 class 线索；
    // ② 把 `SettingsView` 的 `<style>` 也纳入本面的 CSS 来源。
    extraMarkup: [{source: SETTINGS_VIEW, ranges: [[1994, 2012]]}],
    cssSources: [
      {source: `${COMPONENT_DIR}/ThemeSettingsPanel.svelte`},
      {source: SETTINGS_VIEW},
    ],
    scope: ".sp-theme",
    crop: "element",
    sections: [
      {id: "appearance", ref: ".setting-block", engine: ".sp-theme .setting-block"},
    ],
    reference: {kind: "settings", initialSection: "appearance"},
    engine: {mode: "settings:theme", module: "ui_kit::settings::ThemeSettingsPanel"},
    fixture: "theme.json",
    icons: ["Check", "Upload", "ImageOff"],
    trust: {level: "high", why: "内联 swatch 渐变与 CSS 变量覆写必须逐字 ⇒ 提前暴露这条硬要求"},
    deps: [],
  },
  {
    id: "frame",
    markupRanges: [[1922, 1993]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "页面框架与导航",
    source: SETTINGS_VIEW,
    ranges: "markup 1922-1993（72 行）/ script 176-240、1309-1369",
    scope: ".settings-view",
    crop: "element",
    // ⚠️ 只对照**侧栏**（它能独立渲染 ⇒ 独立可对照）。
    //    内容列 `.settings-content` 里装的是**各内容面** ⇒ 单独对照它没有意义
    //    （引擎侧的内容列是空的）⇒ 它的职责由**每个内容面对照里的「宽度同源
    //    断言」**兜底（内容块恒 828px ⇒ 内容列恒 900px ⇒ 已实测）。
    sections: [
      {id: "sidebar", ref: ".settings-subnav", engine: ".settings-subnav"},
    ],
    reference: {kind: "settings", initialSection: "appearance"},
    engine: {mode: "settings:frame", module: "ui_kit::settings::SettingsFrame"},
    fixture: "nav-groups.json",
    icons: [
      "Check", "RotateCcw", "ChevronDown",
      "Cpu", "Sparkles", "ShieldCheck", "Palette", "Archive",
      "CircleUserRound", "Smile", "User", "Eye", "Brain", "Scale",
      "Wrench", "Gauge", "Activity", "Trash2", "Code",
    ],
    trust: {level: "high", why: "纯展示 + 纯本地态；给后面每一面提供对照舞台（828px）"},
    deps: [],
  },
  {
    id: "budget",
    markupRanges: [[3201, 3374]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "预算与配额",
    source: SETTINGS_VIEW,
    ranges: "markup 3201-3374（173 行）/ script 615-709",
    scope: ".sp-budget",
    crop: "element",
    sections: [
      {id: "budget", ref: ".setting-block", engine: ".sp-budget .setting-block"},
      {id: "runtime", ref: ".setting-block", engine: ".sp-budget .setting-block"},
    ],
    reference: {kind: "settings", initialSection: "budget"},
    engine: {mode: "settings:budget", module: "ui_kit::settings::BudgetPanel"},
    fixture: "budget.json",
    icons: [
      "Activity", "RotateCcw", "Info", "SlidersHorizontal", "Timer", "Gauge",
      "Wrench", "Layers3",
    ],
    trust: {level: "high", why: "几乎纯展示 + 一个纯函数算式；余量条内联宽度要逐字"},
    deps: ["frame"],
  },
  {
    id: "data",
    markupRanges: [[3376, 3565]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "数据与用户",
    source: SETTINGS_VIEW,
    ranges: "markup 3376-3565（189 行）/ script 1247-1284、1536-1597、1715-1755、1775-1819",
    scope: ".sp-data",
    crop: "element",
    sections: [
      {id: "data", ref: ".setting-block", engine: ".sp-data .setting-block"},
      {id: "user", ref: ".setting-block", engine: ".sp-data .setting-block"},
    ],
    reference: {kind: "settings", initialSection: "data"},
    engine: {mode: "settings:data", module: "ui_kit::settings::DataPanels"},
    fixture: "data.json",
    icons: [
      "Trash2", "Check", "AlertTriangle", "Folder", "RefreshCcw", "Radar",
      "Copy", "UserRound", "ImagePlus",
    ],
    trust: {level: "high", why: "读数渲染 + 空态分支；危险动作只发意图"},
    deps: ["frame"],
  },
  {
    id: "character",
    markupRanges: [[2364, 2862]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "人格与记忆",
    source: SETTINGS_VIEW,
    ranges: "markup 2364-2862（496 行）/ script 555-613、711-779、1050-1107、1176-1184、1821-1919",
    scope: ".sp-character",
    crop: "element",
    sections: [
      {id: "personality", ref: ".setting-block", engine: ".sp-character .setting-block"},
      {id: "impression", ref: ".setting-block", engine: ".sp-character .setting-block"},
      {id: "cognition", ref: ".setting-block", engine: ".sp-character .setting-block"},
      {id: "disposition", ref: ".setting-block", engine: ".sp-character .setting-block"},
    ],
    reference: {kind: "settings", initialSection: "personality"},
    engine: {mode: "settings:character", module: "ui_kit::settings::CharacterPanels"},
    fixture: "character.json",
    icons: [
      "Layers3", "Trash2", "RotateCcw", "Sparkles", "CheckCircle2", "Info",
      "Plus", "Brain", "HeartHandshake", "GitBranch", "SlidersHorizontal",
      "RefreshCcw", "Radar", "Timer", "Archive",
      "MessageSquarePlus", "Network", "Users", "BookOpen", "Dumbbell", "Landmark",
    ],
    trust: {
      level: "high",
      why: "能力开关卡组件在此成型；invert / requires / 嵌套行三处是全批最高危点",
    },
    deps: ["frame"],
  },
  {
    id: "capability",
    markupRanges: [[2864, 3021], [3023, 3199], [3567, 3680]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "能力与治理",
    source: SETTINGS_VIEW,
    ranges: "markup 2864-3021、3023-3199、3567-3680（448 行，可迁 432 行）/ script 256-553、396-423、1234-1238、1757-1773",
    scope: ".sp-capability",
    crop: "element",
    sections: [
      {id: "governance", ref: ".setting-block", engine: ".sp-capability .setting-block"},
      {id: "tools", ref: ".setting-block", engine: ".sp-capability .setting-block"},
      {id: "developer", ref: ".setting-block", engine: ".sp-capability .setting-block"},
    ],
    reference: {kind: "settings", initialSection: "governance"},
    engine: {mode: "settings:capability", module: "ui_kit::settings::CapabilityPanels"},
    fixture: "capabilities.json",
    icons: [
      "Server", "AlertTriangle", "CheckCircle2", "Info", "Brain", "Scale",
      "Wrench", "Users", "ShieldCheck", "ShieldOff", "Terminal", "Timer",
      "Check", "Sparkles", "Filter", "Tag", "Copy",
      "Workflow", "Globe", "FolderSearch", "FilePenLine", "FileCheck2",
    ],
    trust: {level: "high", why: "复用 character 的卡组件 ⇒ 边际成本最低；原生控件不���绘"},
    deps: ["frame", "character（仅 CapabilityCard 组件）"],
  },
  {
    id: "provider",
    markupRanges: [[2019, 2362], [3687, 3749]],
    cssSources: [{source: SETTINGS_VIEW}],
    title: "服务商与连接",
    source: SETTINGS_VIEW,
    ranges: "markup 2019-2362、3687-3749（407 行）/ script 781-948、985-1048、1186-1233、1371-1455、1482-1542、1620-1704",
    scope: ".sp-provider",
    crop: "element",
    sections: [
      {id: "models", ref: ".setting-block", engine: ".sp-provider .setting-block"},
      // API Key 模态是 `position: fixed` ⇒ **整视口**对照。
      {id: "api-key-modal", ref: ".modal-backdrop", engine: ".modal-backdrop", crop: "viewport"},
    ],
    reference: {kind: "settings", initialSection: "models"},
    engine: {mode: "settings:provider", module: "ui_kit::settings::ProviderGroup"},
    fixture: "provider.json",
    icons: [
      "Server", "Globe", "Sparkles", "Eye", "EyeOff", "Lock", "Check",
      "CheckCircle2", "XCircle", "ChevronDown", "ChevronUp", "Search",
      "RotateCcw", "Trash2",
    ],
    trust: {level: "high", why: "展示层高；HTTP / 钥匙串不覆盖（宿主侧）"},
    deps: ["frame"],
  },
];


/**
 * **共享**的 markup 区间（定义在别处、但渲染进本面的片段）。
 *
 * ## 为什么需要
 *
 * `{#snippet fieldFlag(key)}`（字段旗标：未保存 / ✓）定义在**框架区**
 * （`SettingsView.svelte:1968-1974`）⇒ 它的 class（`field-flag` /
 * `field-dirty` / `field-ack`）**不出现在任何内容区的 markup 里** ⇒
 * 抽取器若只按「本面自己的 markup」收集 class 线索 ⇒ 会把
 * `.field-flag` 的规则**漏掉** ⇒ 引擎侧少了「未保存」旗标的样式。
 *
 * ⇒ 抽取器给**每个 SettingsView 系的面**都补上这段区间的 class 线索。
 *
 * ⚠️ 这不是「手抄」：区间号来自实测（本批），文案与规则仍由抽取器**原文拿**。
 */
export const SHARED_MARKUP_RANGES = {
  source: SETTINGS_VIEW,
  ranges: [[1968, 1974]],
};

/** 按 id 取面。 */
export function panelById(id) {
  return PANELS.find((panel) => panel.id === id) ?? null;
}

/**
 * 「面 × 分区」对照矩阵（对照脚本据此跑）。
 *
 * @param {string} id 面 id。
 * @returns {{panel: object, section: string}[]}
 */
export function comparisonMatrix(id) {
  const panel = panelById(id);
  if (!panel) return [];
  return panel.sections.map((section) => ({panel, section}));
}
