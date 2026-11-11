//! 预算与配额（**设置页 · 预算面**）—— 生产 `SettingsView.svelte` 的
//! `budget` / `runtime` 两个分区的 Rust 重写。
//!
//! ## 本面是什么
//!
//! 生产 `SettingsView.svelte` 的 markup **3201–3342**（`budget` 分区）与
//! **3344–3374**（`runtime` 分区）⇒ 本模块渲染这两块：
//!
//! | 分区 | 内容 |
//! |---|---|
//! | `budget` | 会话消耗卡（`.meter-grid` 四格 + `.budget-remaining` 余量行）+ 预算旋钮卡（三枚数字步进器）+ 多维配额卡（四行「暂无接口」）+ 预算耗尽行为卡 |
//! | `runtime` | 「立即执行深度诊断」按钮 |
//!
//! ## 逐字照抄的**真值口径**（全部来自生产 DOM 的**浏览器实测 dump**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 分区块根 | `<div class="setting-block">` |
//! | 消耗卡标题 | `<span class="cap-card-title">` = 图标 + **一个前导空格** + 文案（**空格与文案同属一个文本节点**） |
//! | 余量行 | `<span class="remaining-label">` = 文案 + `<small>` 作用域（**两者之间无空白**） |
//! | 余量条 | `<span class="remaining-bar" aria-hidden="true">` + `<span class="remaining-fill" style="width: N%;">`（**内联样式带尾分号**） |
//! | 旋钮标题 | `<strong>` = 文案 + `<code class="cap-env">` + 空白 + 旗标位 + 空白 + `<span class="cap-default-badge budget-badge">` |
//! | 徽标 class | `configured` ⇒ `class:on`；`constant` ⇒ `class:note`（**两个互斥**） |
//! | 数字步进器 | `<input type="number" step="1" min aria-label>`；**`max` 为「无上界」⇒ 属性不出现**（不是空串） |
//! | 步进器取值 | 生产用 `bind:value` ⇒ 值是 **property** ⇒ DOM 上**没有 `value` 属性** ⇒ 本组件**不写 `value` 属性**（宿主在挂载后写 property） |
//! | 诊断按钮 | `<button class="quiet-button">`；`checking` 为真 ⇒ `disabled=""`（**属性存在但值为空串**） |
//! | 空白 | 元素之间的换行折叠成**恰好一个空格**；`{#each}` 边界**两侧的空白被裁掉**（⇒ 余量行 / 配额行**之间没有**空白文本节点） |
//!
//! ## ⭐ 余量条的取整口径（**本面最容易错的一处**）
//!
//! 生产是 `Math.round(row.ratio * 100)`：
//!
//! - 脚本侧的 `Math.round` 是**半数向 +∞**（`-2.5 → -2`）；
//! - 原生 `f64::round` 是**半数远离零**（`-2.5 → -3`）⇒ **差 1**。
//!
//! ⇒ 本模块用 [`crate::num::round`]（**半数向 +∞**，与脚本侧同口径）**照抄**
//! ⇒ 见 [`remaining_fill_percent`]。
//!
//! ## 派生值：**宿主喂输入，引擎算输出**
//!
//! `budgetRemainingRows` / `effectiveBudget` / `formatSessionTotals` /
//! `emptySessionTotals` 在生产里是**纯函数**（`lib/budget.ts` +
//! `lib/chat-shell/turn-telemetry.ts`）⇒ **照抄进本模块**（口径逐条对齐，
//! 见各函数的文档）⇒ 宿主只喂**输入**（会话累计 + 三枚旋钮的配置值）。
//!
//! ## 常量表：**宿主喂**（不写死在本模块）
//!
//! `BUDGET_KNOB_SPECS` / `QUOTA_DIMENSIONS` / `BUDGET_EXHAUSTION` 在生产里
//! 是**带文案与环境名的常量表** ⇒ 由**宿主**作为固定数据喂进来
//! （[`KnobRow`] / [`QuotaDimension`] /
//! [`BudgetPanelProps::exhaustion_behaviors`]）⇒ 生产仍是唯一真源。
//!
//! ## 诚实边界
//!
//! - **诊断结果区未迁**：`.diag-results` 用的是**外部组件** `StatusBadge`
//!   （62 行，带**自己的 scoped CSS** ⇒ 未抽取 ⇒ 引擎侧无法逐字复现）
//!   ⇒ 本面**只渲染触发按钮**；`checking` 态（按钮文案 + `disabled`）**已迁**。
//! - **运行指标卡未迁**：`MetricsPanel`（205 行，内部拉 `/v1/panel/metrics`）
//!   是**外部组件 + HTTP 契约** ⇒ 本面**不渲染**它（对照只覆盖本面自己的区块）。
//! - **提交的执行在宿主**：数字步进器是「失焦 / 回车提交」（三级即效第 2 级）
//!   ⇒ 引擎只发**意图**（[`BudgetPanelProps::on_knob_commit`]），**不写配置**。
//! - **数据是固定数据**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约。

use std::rc::Rc;

use crate::icon::{Icon, IconData, IconNode, IconProps};
use crate::num;
use crate::settings_frame::{icons as frame_icons, FieldFlag, FieldFlagProps, FieldFlagState};
use apeireth_ui_core::{ReactiveScope, Signal, UiComponent};
use apeireth_ui_dom::{DomEvent, ElementHandler, EventKind, VNode};
use apeireth_ui_macro::view;

// ── 图标数据（**生成物** —— 勿手改） ───────────────────────────────────────
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
//   （面 manifest 模式 ⇒ 按面输出，避免与其它面共用同一个生成物文件）
// 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`
// ⚠️ 生产 import 名 ⇒ **真实 class 后缀**：`Layers3` ⇒ 别名 `layers-3` ⇒ 真实
//   文件 `layers.svelte` ⇒ class 后缀 `lucide-layers`（**不是** `lucide-layers-3`）。
// ⚠️ `activity` / `gauge` / `rotate-ccw` / `wrench` 已在本 crate 的**框架面**
//   图标数据里（`crate::settings_frame::icons`）⇒ 本模块**直接复用**，
//   **不重复定义**。
//
// 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：
//
// ISC License
//
// Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
//
// Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

/// 图标 `info`（生产 import 名 `Info`）。
#[rustfmt::skip]
pub const ICON_INFO: IconData = IconData {
    name: "info",
    class: "lucide-icon lucide lucide-info",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        IconNode { tag: "path", attrs: &[("d", "M12 16v-4")] },
        IconNode { tag: "path", attrs: &[("d", "M12 8h.01")] },
    ],
};

/// 图标 `layers`（生产 import 名 `Layers3` ⇒ 别名 `layers-3` ⇒ 真实文件
/// `layers.svelte`）。
#[rustfmt::skip]
pub const ICON_LAYERS: IconData = IconData {
    name: "layers",
    class: "lucide-icon lucide lucide-layers",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a2 2 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a2 2 0 0 0 0-1.83z")] },
        IconNode { tag: "path", attrs: &[("d", "M2 12a2 2 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A2 2 0 0 0 22 12")] },
        IconNode { tag: "path", attrs: &[("d", "M2 17a2 2 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A2 2 0 0 0 22 17")] },
    ],
};

/// 图标 `sliders-horizontal`（生产 import 名 `SlidersHorizontal`）。
#[rustfmt::skip]
pub const ICON_SLIDERS_HORIZONTAL: IconData = IconData {
    name: "sliders-horizontal",
    class: "lucide-icon lucide lucide-sliders-horizontal",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "14"), ("y1", "4"), ("y2", "4")] },
        IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "3"), ("y1", "4"), ("y2", "4")] },
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "12"), ("y1", "12"), ("y2", "12")] },
        IconNode { tag: "line", attrs: &[("x1", "8"), ("x2", "3"), ("y1", "12"), ("y2", "12")] },
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "16"), ("y1", "20"), ("y2", "20")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "3"), ("y1", "20"), ("y2", "20")] },
        IconNode { tag: "line", attrs: &[("x1", "14"), ("x2", "14"), ("y1", "2"), ("y2", "6")] },
        IconNode { tag: "line", attrs: &[("x1", "8"), ("x2", "8"), ("y1", "10"), ("y2", "14")] },
        IconNode { tag: "line", attrs: &[("x1", "16"), ("x2", "16"), ("y1", "18"), ("y2", "22")] },
    ],
};

/// 图标 `timer`（生产 import 名 `Timer`）。
#[rustfmt::skip]
pub const ICON_TIMER: IconData = IconData {
    name: "timer",
    class: "lucide-icon lucide lucide-timer",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "14"), ("y1", "2"), ("y2", "2")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "15"), ("y1", "14"), ("y2", "11")] },
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "14"), ("r", "8")] },
    ],
};

// ── 文案常量（**逐字**取自生产模板） ───────────────────────────────────────

/// 分区块标题（`budget` 分区）。
pub const BLOCK_TITLE: &str = "预算与配额";

/// 分区块说明（`budget` 分区）。
///
/// ⚠️ 生产模板里这段文字**跨两行**（换行 + 缩进**留在文本节点里**）⇒ 本常量
/// **逐字**保留那个换行与缩进（否则换行位置不同 ⇒ 像素不一致）。
pub const BLOCK_DESC: &str = "预算旋钮面 + 会话消耗仪表。数字旋钮失焦或回车提交，即效写配置并注入运行时环境；\n            越界钳制到 1..=64，非法值回默认。多维配额没接口的维度如实标注「暂无接口」，不造假旋钮。";

/// 运行时诊断分区的标题。
pub const RUNTIME_BLOCK_TITLE: &str = "运行时诊断";

/// 运行时诊断分区的说明。
pub const RUNTIME_BLOCK_DESC: &str = "实时探测后端网关、模型服务、会话账本与记忆流。";

/// 消耗卡标题（**自带前导空格**：与图标同属一个文本节点）。
pub const SESSION_CARD_TITLE: &str = " 会话消耗（当前对话）";

/// 消耗卡计数前缀。
pub const SESSION_CARD_COUNT_PREFIX: &str = "回合 ";

/// 消耗格 · 输入 / 输出 token 的标签。
pub const METER_TOKENS_LABEL: &str = "本会话 token（入 / 出）";

/// 消耗格 · 提示缓存命中的标签。
pub const METER_CACHE_LABEL: &str = "提示缓存命中（数 · 率）";

/// 消耗格 · 回合数的标签。
pub const METER_TURNS_LABEL: &str = "回合数";

/// 消耗格 · 累计耗时的标签。
pub const METER_DURATION_LABEL: &str = "累计耗时";

/// 余量区标题。
pub const REMAINING_TITLE: &str = "预算余量（对生效上限求余量；无上限维度显「—」）";

/// 旋钮卡标题（**自带前导空格**）。
pub const KNOB_CARD_TITLE: &str = " 预算旋钮";

/// 旋钮卡的提交时机说明。
pub const KNOB_CARD_COUNT: &str = "失焦 / 回车即生效";

/// 配额卡标题（**自带前导空格**）。
pub const QUOTA_CARD_TITLE: &str = " 多维配额（Token / 步数 / 花费 / 深度）";

/// 配额卡的可配维度计数。
pub const QUOTA_CARD_COUNT: &str = "0/4 可配";

/// 「暂无接口」徽标文案（生产 `QUOTA_DIMENSION_STATUS_LABEL`）。
pub const QUOTA_STATUS_LABEL: &str = "暂无接口";

/// 预算耗尽行为卡的徽标（**自带前导空格**）。
pub const EXHAUSTION_BADGE: &str = " 固定语义 · 无选择器";

/// 预算耗尽行为卡的标题。
pub const EXHAUSTION_TITLE: &str = "预算耗尽行为";

/// 预算耗尽行为卡的说明。
pub const EXHAUSTION_LEDE: &str =
    "后端没有可配置的耗尽行为语义，这里不出选择器；实际行为是固定语义：";

/// 诊断按钮的常态文案。
pub const DIAGNOSTICS_LABEL: &str = "立即执行深度诊断";

/// 诊断按钮的进行中文案。
pub const DIAGNOSTICS_BUSY_LABEL: &str = "正在诊断…";

/// 未知值的占位符（生产 `DASH`）。
pub const DASH: &str = "—";

// ── 数据形状（**中性**；宿主注入） ──────────────────────────────────────────

/// 旋钮的取值域处置（与生产 `outOfRange` 同口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutOfRange {
    /// 钳到 `[min, max]`。
    Clamp,
    /// 越界回默认（不钳制）。
    Reject,
}

/// 旋钮的**键**（生产配置键；逐字沿用 ⇒ 与生产的 `capabilities` 键同名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnobKey {
    /// 回合轮数上限。
    MaxTurnRounds,
    /// 单轮工具调用上限。
    MaxToolCalls,
    /// 上下文注入字符预算。
    ContextBudgetChars,
}

impl KnobKey {
    /// 生产配置键（**逐字**）。
    pub fn config_key(self) -> &'static str {
        match self {
            Self::MaxTurnRounds => "maxTurnRounds",
            Self::MaxToolCalls => "maxToolCalls",
            Self::ContextBudgetChars => "contextBudgetChars",
        }
    }

    /// 「未保存 / ✓」旗标的键（生产 `BUDGET_FLAG_KEYS`）。
    pub fn flag_key(self) -> &'static str {
        match self {
            Self::MaxTurnRounds => "budget.maxTurnRounds",
            Self::MaxToolCalls => "budget.maxToolCalls",
            Self::ContextBudgetChars => "budget.contextBudgetChars",
        }
    }
}

/// 一枚旋钮行（**注册表**；文案与环境名由宿主喂 ⇒ 生产仍是唯一真源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnobRow {
    /// 旋钮键。
    pub key: KnobKey,
    /// 显示名。
    pub label: String,
    /// 环境变量名（芯片文案）。
    pub env: String,
    /// 说明（旋钮卡的次行文案）。
    pub desc: String,
    /// 取值域下界。
    pub min: i64,
    /// 取值域上界；`None` = 无上界钳制（⇒ 步进器**不写 `max` 属性**）。
    pub max: Option<i64>,
    /// 非法 / 未配置时的默认值。
    pub fallback: i64,
    /// 越界处置。
    pub out_of_range: OutOfRange,
    /// 行首图标。
    pub icon: &'static IconData,
    /// 行内补充提示（与说明之间**一个空格**）。
    pub hint: String,
}

/// 多维配额的一个维度（**没接口的维度如实标注**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaDimension {
    /// 维度键。
    pub key: String,
    /// 维度名。
    pub label: String,
    /// 诚实原因（为什么不给旋钮）。
    pub reason: String,
    /// 读码出处（悬浮说明）。
    pub evidence: String,
}

/// 会话消耗累计（**纯数据**；缺字段 = 该位未知 ⇒ 显示「—」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUsageTotals {
    /// 回合数（真实计数，0 照显）。
    pub turns: u64,
    /// 输入 token 累计；`None` = 从未上报。
    pub prompt_tokens: Option<i64>,
    /// 输出 token 累计；`None` = 从未上报。
    pub completion_tokens: Option<i64>,
    /// 提示缓存命中 token 累计；`None` = 从未上报。
    pub cache_hit_tokens: Option<i64>,
    /// 累计耗时（毫秒）；`None` = 未测得。
    pub duration_ms: Option<i64>,
    /// 工具调用累计（真实计数）。
    pub tool_calls: u64,
    /// 最近一回合的工具调用数（对单轮上限求余量用）；`None` = 未知。
    pub last_turn_tool_calls: Option<i64>,
}

impl Default for SessionUsageTotals {
    fn default() -> Self {
        Self::empty()
    }
}

impl SessionUsageTotals {
    /// 空累计器（生产 `emptySessionTotals`：本会话还没跑过任何回合）。
    pub fn empty() -> Self {
        Self {
            turns: 0,
            prompt_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            duration_ms: None,
            tool_calls: 0,
            last_turn_tool_calls: None,
        }
    }
}

/// 三枚旋钮的**配置值**（`None` = 未配置 ⇒ 回落编译期常量默认值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KnobValues {
    /// 回合轮数上限。
    pub max_turn_rounds: Option<i64>,
    /// 单轮工具调用上限。
    pub max_tool_calls: Option<i64>,
    /// 上下文注入字符预算。
    pub context_budget_chars: Option<i64>,
}

impl KnobValues {
    /// 取某枚旋钮的配置值。
    pub fn get(&self, key: KnobKey) -> Option<i64> {
        match key {
            KnobKey::MaxTurnRounds => self.max_turn_rounds,
            KnobKey::MaxToolCalls => self.max_tool_calls,
            KnobKey::ContextBudgetChars => self.context_budget_chars,
        }
    }
}

/// 三枚旋钮的**草稿文本**（数字步进器的原始字面量；空串 = 未配置）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnobDraft {
    /// 回合轮数上限的草稿。
    pub max_turn_rounds: String,
    /// 单轮工具调用上限的草稿。
    pub max_tool_calls: String,
    /// 上下文注入字符预算的草稿。
    pub context_budget_chars: String,
}

impl KnobDraft {
    /// 取某枚旋钮的草稿。
    pub fn get(&self, key: KnobKey) -> &str {
        match key {
            KnobKey::MaxTurnRounds => &self.max_turn_rounds,
            KnobKey::MaxToolCalls => &self.max_tool_calls,
            KnobKey::ContextBudgetChars => &self.context_budget_chars,
        }
    }
}

/// 三枚旋钮的**提交反馈**（空 = 无反馈）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnobFeedback {
    /// 回合轮数上限的反馈。
    pub max_turn_rounds: String,
    /// 单轮工具调用上限的反馈。
    pub max_tool_calls: String,
    /// 上下文注入字符预算的反馈。
    pub context_budget_chars: String,
}

impl KnobFeedback {
    /// 取某枚旋钮的反馈。
    pub fn get(&self, key: KnobKey) -> &str {
        match key {
            KnobKey::MaxTurnRounds => &self.max_turn_rounds,
            KnobKey::MaxToolCalls => &self.max_tool_calls,
            KnobKey::ContextBudgetChars => &self.context_budget_chars,
        }
    }
}

/// 三枚旋钮的**字段旗标**（宿主写，组件只渲染）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KnobFlags {
    /// 回合轮数上限的旗标。
    pub max_turn_rounds: FieldFlagState,
    /// 单轮工具调用上限的旗标。
    pub max_tool_calls: FieldFlagState,
    /// 上下文注入字符预算的旗标。
    pub context_budget_chars: FieldFlagState,
}

impl KnobFlags {
    /// 取某枚旋钮的旗标。
    pub fn get(&self, key: KnobKey) -> FieldFlagState {
        match key {
            KnobKey::MaxTurnRounds => self.max_turn_rounds,
            KnobKey::MaxToolCalls => self.max_tool_calls,
            KnobKey::ContextBudgetChars => self.context_budget_chars,
        }
    }
}

/// 生效值徽标的取值口径（生产 `BudgetSource`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetSource {
    /// 读生效中的旋钮（环境可配）。
    Configured,
    /// 读编译期常量（旋钮未配置，取常量默认值）。
    Constant,
}

impl BudgetSource {
    /// 口径串（生产 `budget.source`）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Constant => "constant",
        }
    }
}

/// 一枚旋钮的**生效值徽标**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetBadge {
    /// 实际生效值。
    pub effective: i64,
    /// 取值口径。
    pub source: BudgetSource,
    /// 口径注记（徽标的悬浮说明）。
    pub note: String,
    /// 徽标文案（「生效 8 · configured」）。
    pub label: String,
}

/// 三枚旋钮的生效值徽标集。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBudget {
    /// 回合轮数上限。
    pub max_turn_rounds: BudgetBadge,
    /// 单轮工具调用上限。
    pub max_tool_calls: BudgetBadge,
    /// 上下文注入字符预算。
    pub context_budget_chars: BudgetBadge,
}

impl EffectiveBudget {
    /// 取某枚旋钮的徽标。
    pub fn get(&self, key: KnobKey) -> &BudgetBadge {
        match key {
            KnobKey::MaxTurnRounds => &self.max_turn_rounds,
            KnobKey::MaxToolCalls => &self.max_tool_calls,
            KnobKey::ContextBudgetChars => &self.context_budget_chars,
        }
    }
}

/// 会话消耗卡的显示串（**无数据位一律「—」**，真实 0 照显 0）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTotalsView {
    /// 「入 1,234 / 出 56」；两位都未知 = 「—」。
    pub tokens: String,
    /// 「命中 890 · 72%」；命中未知 = 「—」。
    pub cache: String,
    /// 回合数。
    pub turns: String,
    /// 累计耗时。
    pub duration: String,
    /// 工具调用累计。
    pub tool_calls: String,
}

/// 预算余量的一行。
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetRemainingRow {
    /// 行键。
    pub key: String,
    /// 维度名。
    pub label: String,
    /// 作用域诚实标注（上限管到哪一口径）。
    pub scope: String,
    /// 生效上限（「16 · configured」；无上限 = 「—」）。
    pub cap: String,
    /// 已耗（「—」= 未测得）。
    pub used: String,
    /// 余量（「—」= 无上限或已耗未测得）。
    pub remaining: String,
    /// 用量占比 0..1（有上限且已耗已知才给；否则 `None`，不编百分比）。
    pub ratio: Option<f64>,
}

/// 一次旋钮提交的处置口径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnobResolutionStatus {
    /// 直通。
    Ok,
    /// 越界钳制。
    Clamped,
    /// 非法 / 越界回默认。
    Fallback,
}

/// 一次旋钮提交的**解析结果**（生产 `BudgetKnobResolution`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetKnobResolution {
    /// 归一后写入配置的值；`None` = 未配置（回默认落点）。
    pub value: Option<i64>,
    /// 实际生效值（徽标显示）。
    pub effective: i64,
    /// 处置口径。
    pub status: KnobResolutionStatus,
    /// 人话反馈（空 = 无需提示）。
    pub feedback: String,
}

// ── 纯函数（**照抄生产**：`lib/budget.ts` + `lib/chat-shell/turn-telemetry.ts`）──

/// 千分位展示（**与脚本侧同口径**：只加组分隔，不做单位换算）。
///
/// ## 口径
///
/// 生产的写法是 `String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',')`
/// ⇒ 规则 = **从右往左每三位加一个组分隔符**（负号位不参与分组）。
pub fn format_count(value: Option<i64>) -> String {
    let Some(value) = value else {
        return String::from(DASH);
    };
    let text = value.to_string();
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.as_str()),
    };
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    out.push_str(sign);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (bytes.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(char::from(*byte));
    }
    out
}

/// 回合耗时展示串（未知 / 非法 = 「—」，真实 0 = 「0ms」）。
///
/// ## 与脚本侧的逐条对齐
///
/// - `ms < 1000` ⇒ `Math.round(ms)`ms（**半数向 +∞** ⇒ 用 [`num::round`]）；
/// - `ms < 60_000` ⇒ 秒数 < 10 用 `toFixed(1)`（**半数进位** ⇒ 用
///   [`num::to_fixed`]），否则 `Math.round(seconds)`；
/// - 其余 ⇒ `分` + `秒`（秒数**两位补零**）。
///
/// ## 入口类型
///
/// 本入口是 [`format_duration_f64`] 的**薄封装**（`i64` ⇒ `f64`）⇒ 两个入口
/// **同一实现、同一口径** ⇒ 规则层差分装置（`harness/rules/`）可用整数入口，
/// 也可用 [`format_duration_f64`] 比对**小数毫秒**（生产 `formatDuration`
/// 收 `number` ⇒ 可小数）。
pub fn format_duration(ms: Option<i64>) -> String {
    format_duration_f64(ms.map(|value| value as f64))
}

/// 回合耗时展示串（**小数毫秒**入口；生产 `formatDuration` 的 `number` 面）。
///
/// ## 为什么要有这个入口
///
/// 生产 `formatDuration(ms: number | null | undefined)` 收 **`number`** ⇒ **可小数**；
/// 而 [`format_duration`] 收 `Option<i64>` ⇒ **小数表达不了**。
/// ⇒ 二者**不是同一层 API**：`999.5` 在生产给 `1000ms`（`ms < 1000` 分支里的
/// `Math.round(999.5)`），而整数入口**永远给不出** `1000ms`
/// （`999` ⇒ `999ms`，`1000` ⇒ `1.0s`）⇒ 该形态是**可观测差异**，不是名义差异。
///
/// ## 与脚本侧的逐条对齐
///
/// - 缺省 / 非有限 / 负 ⇒ 「—」（生产 `!Number.isFinite(ms) || ms < 0` ⇒ `DASH`）；
/// - `ms < 1000` ⇒ `Math.round(ms)`ms（**半数向 +∞** ⇒ 用 [`num::round`]）；
/// - `ms < 60_000` ⇒ 秒数 < 10 用 `toFixed(1)`（**半数进位** ⇒ 用
///   [`num::to_fixed`]），否则 `Math.round(seconds)`；
/// - 其余 ⇒ `分` + `秒`（秒数**两位补零**）。
pub fn format_duration_f64(ms: Option<f64>) -> String {
    let Some(ms) = ms else {
        return String::from(DASH);
    };
    if !ms.is_finite() || ms < 0.0 {
        return String::from(DASH);
    }
    if ms < 1000.0 {
        return format!("{}ms", num::number_to_string(num::round(ms)));
    }
    let seconds = ms / 1000.0;
    if ms < 60_000.0 {
        return if seconds < 10.0 {
            format!("{}s", num::to_fixed(seconds, 1))
        } else {
            format!("{}s", num::number_to_string(num::round(seconds)))
        };
    }
    let minutes = (ms / 60_000.0).floor();
    let rest = num::round((ms % 60_000.0) / 1000.0);
    let minutes_text = num::number_to_string(minutes);
    let rest_text = format!("{:02}", rest as i64);
    format!("{minutes_text}m{rest_text}s")
}

/// 命中率串（命中 / 输入；输入为 0 或缺 → 「—」）。
///
/// ⚠️ 百分比的取整用 [`num::round`]（**半数向 +∞**）⇒ 与脚本侧同口径。
pub fn cache_rate(hit: Option<i64>, input: Option<i64>) -> String {
    let (Some(hit), Some(input)) = (hit, input) else {
        return String::from(DASH);
    };
    if input <= 0 {
        return String::from(DASH);
    }
    let ratio = (hit as f64) / (input as f64) * 100.0;
    format!("{}%", num::number_to_string(num::round(ratio)))
}

/// 会话消耗卡显示串（**真值渲染**，未知位诚实「—」）。
pub fn format_session_totals(totals: &SessionUsageTotals) -> SessionTotalsView {
    let prompt = format_count(totals.prompt_tokens);
    let completion = format_count(totals.completion_tokens);
    let tokens = if prompt == DASH && completion == DASH {
        String::from(DASH)
    } else {
        format!("入 {prompt} / 出 {completion}")
    };
    let cache = match totals.cache_hit_tokens {
        None => String::from(DASH),
        Some(hit) => {
            let rate = cache_rate(totals.cache_hit_tokens, totals.prompt_tokens);
            format!("命中 {} · {rate}", format_count(Some(hit)))
        }
    };
    SessionTotalsView {
        tokens,
        cache,
        turns: totals.turns.to_string(),
        duration: format_duration(totals.duration_ms),
        tool_calls: totals.tool_calls.to_string(),
    }
}

/// 整数字面量解析（**镜像后端整数解析**：非整数文本一律非法）。
///
/// 生产口径：`/^[+-]?\d+$/` ⇒ 不接受小数。
pub fn parse_integer_field(raw: &str) -> Option<i64> {
    let text = raw.trim();
    let body = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(text);
    if body.is_empty() || !body.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

/// 解析一次旋钮提交（越界按规格处置；非法一律回默认）。
///
/// ## 与生产逐条对齐
///
/// - 非法 ⇒ `value = None`、`effective = fallback`、反馈「非法值：已回默认 N」；
/// - 区间内 ⇒ 直通，无反馈；
/// - `Reject` 规格越界 ⇒ `value = None`、回默认，反馈「越界（…）：已回默认 N」；
/// - `Clamp` 规格越界 ⇒ 钳到 `[min, max]`，反馈「越界：已钳制到 N（min..=max）」。
pub fn resolve_budget_limit_input(raw: &str, spec: &KnobRow) -> BudgetKnobResolution {
    let Some(parsed) = parse_integer_field(raw) else {
        return BudgetKnobResolution {
            value: None,
            effective: spec.fallback,
            status: KnobResolutionStatus::Fallback,
            feedback: format!("非法值：已回默认 {}", spec.fallback),
        };
    };
    let in_range = parsed >= spec.min && spec.max.is_none_or(|max| parsed <= max);
    if in_range {
        return BudgetKnobResolution {
            value: Some(parsed),
            effective: parsed,
            status: KnobResolutionStatus::Ok,
            feedback: String::new(),
        };
    }
    if spec.out_of_range == OutOfRange::Reject {
        let domain = match spec.max {
            None => format!("{} 起", spec.min),
            Some(max) => format!("{}..={}", spec.min, max),
        };
        return BudgetKnobResolution {
            value: None,
            effective: spec.fallback,
            status: KnobResolutionStatus::Fallback,
            feedback: format!("越界（{domain}）：已回默认 {}", spec.fallback),
        };
    }
    let max = spec.max.unwrap_or(parsed);
    let clamped = max.min(spec.min.max(parsed));
    BudgetKnobResolution {
        value: Some(clamped),
        effective: clamped,
        status: KnobResolutionStatus::Clamped,
        feedback: format!(
            "越界：已钳制到 {clamped}（{}..={}）",
            spec.min,
            spec.max.map(|max| max.to_string()).unwrap_or_default()
        ),
    }
}

/// 生效值徽标（**生产 `budgetKnobBadge`**）。
///
/// - 未配置（或配置值非法）⇒ 回落**编译期常量默认值**，口径如实报 `constant`；
/// - 已配置且合法 ⇒ 口径报 `configured`。
pub fn budget_knob_badge(configured: Option<i64>, spec: &KnobRow) -> BudgetBadge {
    let constant_note = "读编译期常量（旋钮未配置，取常量默认值）";
    let configured_note = "读自生效中的预算旋钮（env 可配；未设值时回落默认）";
    let Some(value) = configured else {
        return badge(spec.fallback, BudgetSource::Constant, constant_note);
    };
    let resolution = resolve_budget_limit_input(&value.to_string(), spec);
    if resolution.status == KnobResolutionStatus::Fallback || resolution.value.is_none() {
        return badge(spec.fallback, BudgetSource::Constant, constant_note);
    }
    badge(
        resolution.effective,
        BudgetSource::Configured,
        configured_note,
    )
}

/// 造一枚徽标（**生产 `badge()`**）。
fn badge(effective: i64, source: BudgetSource, note: &str) -> BudgetBadge {
    BudgetBadge {
        effective,
        source,
        note: note.to_owned(),
        label: format!("生效 {effective} · {}", source.as_str()),
    }
}

/// 三枚旋钮的生效值徽标集（**生产 `effectiveBudget`**）。
pub fn effective_budget(knobs: &KnobValues, rows: &[KnobRow]) -> EffectiveBudget {
    let empty = BudgetBadge {
        effective: 0,
        source: BudgetSource::Constant,
        note: String::new(),
        label: String::new(),
    };
    let mut out = EffectiveBudget {
        max_turn_rounds: empty.clone(),
        max_tool_calls: empty.clone(),
        context_budget_chars: empty,
    };
    for row in rows {
        let badge = budget_knob_badge(knobs.get(row.key), row);
        match row.key {
            KnobKey::MaxTurnRounds => out.max_turn_rounds = badge,
            KnobKey::MaxToolCalls => out.max_tool_calls = badge,
            KnobKey::ContextBudgetChars => out.context_budget_chars = badge,
        }
    }
    out
}

/// 预算余量行（**生产 `budgetRemainingRows`**）。
///
/// ## 逐条对齐
///
/// - 前五行**没有生效上限** ⇒ 上限与余量一律「—」，**不编百分比**；
/// - 「单轮工具调用」对**单轮生效上限**求余量（已耗 = 最近一回合合计数）；
/// - 「回合轮数」前端**未测得轮数** ⇒ 已耗与余量一律「—」。
pub fn budget_remaining_rows(
    totals: &SessionUsageTotals,
    budget: &EffectiveBudget,
) -> Vec<BudgetRemainingRow> {
    let no_cap = |key: &str, label: &str, scope: &str, used: String| BudgetRemainingRow {
        key: key.to_owned(),
        label: label.to_owned(),
        scope: scope.to_owned(),
        cap: String::from(DASH),
        used,
        remaining: String::from(DASH),
        ratio: None,
    };

    let tool_cap = &budget.max_tool_calls;
    let tool_used = totals.last_turn_tool_calls;
    let tool_remaining = tool_used.map(|used| (tool_cap.effective - used).max(0));

    vec![
        no_cap(
            "tokenIn",
            "Token（入）",
            "会话累计",
            format_count(totals.prompt_tokens),
        ),
        no_cap(
            "tokenOut",
            "Token（出）",
            "会话累计",
            format_count(totals.completion_tokens),
        ),
        no_cap(
            "cacheHit",
            "提示缓存命中",
            "会话累计",
            format_count(totals.cache_hit_tokens),
        ),
        no_cap("turns", "回合数", "会话累计", totals.turns.to_string()),
        no_cap(
            "duration",
            "累计耗时",
            "会话累计",
            format_duration(totals.duration_ms),
        ),
        BudgetRemainingRow {
            key: String::from("toolCalls"),
            label: String::from("单轮工具调用"),
            scope: String::from("最近一回合"),
            cap: format!("{} · {}", tool_cap.effective, tool_cap.source.as_str()),
            used: tool_used.map_or_else(|| String::from(DASH), |used| used.to_string()),
            remaining: tool_remaining.map_or_else(|| String::from(DASH), |left| left.to_string()),
            ratio: tool_used.map(|used| {
                let ratio = (used as f64) / (tool_cap.effective as f64);
                ratio.min(1.0)
            }),
        },
        BudgetRemainingRow {
            key: String::from("turnRounds"),
            label: String::from("回合轮数"),
            scope: String::from("每回合（前端未测得轮数）"),
            cap: format!(
                "{} · {}",
                budget.max_turn_rounds.effective,
                budget.max_turn_rounds.source.as_str()
            ),
            used: String::from(DASH),
            remaining: String::from(DASH),
            ratio: None,
        },
    ]
}

/// ⭐ 余量条的**内联宽度百分比**（**生产 `Math.round(row.ratio * 100)`**）。
///
/// ## 为什么不能用 `f64::round`
///
/// | 取整口径 | `-2.5` | `2.5` |
/// |---|---|---|
/// | 脚本侧 `Math.round`（**半数向 +∞**） | `-2` | `3` |
/// | 原生 `f64::round`（**半数远离零**） | `-3` | `3` |
///
/// ⇒ 负数会差 1 ⇒ 这里用 [`num::round`]（**半数向 +∞**，与脚本侧同口径）。
///
/// ## 浮点口径
///
/// `ratio * 100` 的**中间值**也必须一致 ⇒ 两边都用 `f64`（IEEE754 双精度）
/// ⇒ 同一份输入 ⇒ 同一个结果。
pub fn remaining_fill_percent(ratio: f64) -> i64 {
    num::round(ratio * 100.0) as i64
}

/// 余量条的**内联样式串**（**逐字**：`width: N%;` ⇒ **带尾分号**）。
///
/// 生产用样式指令 ⇒ 浏览器把取值**序列化**成自己的口径（**尾分号**）⇒
/// 引擎侧必须写出**同一个串**。
pub fn remaining_fill_style(ratio: f64) -> String {
    format!("width: {}%;", remaining_fill_percent(ratio))
}

// ── 组件 ─────────────────────────────────────────────────────────────────────

/// 渲染哪一块（**分区内互斥**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetSection {
    /// 预算与配额（消耗卡 + 旋钮卡 + 配额卡 + 耗尽行为卡）。
    #[default]
    Budget,
    /// 运行时诊断（触发按钮）。
    Runtime,
}

/// 意图出口：**旋钮草稿变更**（入参 = 旋钮键 + 新的原始字面量）。
pub type KnobInputIntent = Rc<dyn Fn(KnobKey, String)>;

/// 意图出口：**旋钮提交**（失焦 / 回车；入参 = 旋钮键）。
pub type KnobCommitIntent = Rc<dyn Fn(KnobKey)>;

/// 预算面的 props（**状态归宿主** ⇒ 组件是**纯函数**）。
#[derive(Clone)]
pub struct BudgetPanelProps {
    /// 分区。
    pub section: BudgetSection,
    /// 会话消耗累计（**宿主喂输入** ⇒ 引擎算输出）。
    pub totals: SessionUsageTotals,
    /// 三枚旋钮的**配置值**。
    pub knobs: KnobValues,
    /// 旋钮行注册表（**顺序 = 渲染顺序**）。
    pub rows: Vec<KnobRow>,
    /// 旋钮草稿（**原始字面量**）。
    pub draft: KnobDraft,
    /// 旋钮反馈（空 = 无反馈）。
    pub feedback: KnobFeedback,
    /// 字段旗标（宿主写，组件只渲染）。
    pub flags: KnobFlags,
    /// 多维配额维度。
    pub quota_dimensions: Vec<QuotaDimension>,
    /// 预算耗尽行为的固定语义说明。
    pub exhaustion_behaviors: Vec<String>,
    /// 「正在诊断」态（**诊断结果区未迁** ⇒ 见模块文档的诚实边界）。
    pub checking_runtime: bool,
    /// 意图出口：旋钮草稿变更。
    pub on_knob_input: KnobInputIntent,
    /// 意图出口：旋钮提交。
    pub on_knob_commit: KnobCommitIntent,
    /// 意图出口：立即执行深度诊断（**HTTP ⇒ 宿主能力** ⇒ 引擎只发意图）。
    pub on_check_runtime: Rc<dyn Fn()>,
}

impl std::fmt::Debug for BudgetPanelProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BudgetPanelProps")
            .field("section", &self.section)
            .field("totals", &self.totals)
            .field("knobs", &self.knobs)
            .field("rows", &self.rows.len())
            .field("draft", &self.draft)
            .field("feedback", &self.feedback)
            .field("flags", &self.flags)
            .field("quota_dimensions", &self.quota_dimensions.len())
            .field("exhaustion_behaviors", &self.exhaustion_behaviors.len())
            .field("checking_runtime", &self.checking_runtime)
            .finish_non_exhaustive()
    }
}

/// 预算面组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BudgetPanel;

impl UiComponent for BudgetPanel {
    type Props = BudgetPanelProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        match props.section {
            BudgetSection::Budget => render_budget(props),
            BudgetSection::Runtime => render_runtime(props),
        }
    }
}

/// 渲染预算分区（**消耗卡 + 旋钮卡 + 配额卡 + 耗尽行为卡**）。
fn render_budget(props: &BudgetPanelProps) -> VNode {
    let view = format_session_totals(&props.totals);
    let budget = effective_budget(&props.knobs, &props.rows);
    let rows = budget_remaining_rows(&props.totals, &budget);
    // ⚠️ `view!` 的两条**文本**口径（实测踩出来的）：
    //   ① 裸标识符**首字母大写 ⇒ 判成组件** ⇒ 文本常量先落到局部变量；
    //   ② 裸标识符**若是 HTML 标签名**（`title` / `label` / `count`…）⇒ 也判成
    //      元素 ⇒ 局部变量名必须**避开标签名**；
    //   ③ `format!(…)` **不是受支持的表达式**（宏的表达式文法只吃标识符与
    //      `.` 链）⇒ 拼接串**一律在模板外算好**。
    let heading_text = BLOCK_TITLE;
    let desc_text = BLOCK_DESC;

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { heading_text }
            " "
            p(class = "block-desc") { desc_text }
            " "
            { render_session_card(&view, &rows) }
            " "
            { render_knob_card(props, &budget) }
            " "
            { render_quota_card(props) }
            " "
            { render_exhaustion_card(props) }
        }
    }
}

/// 渲染会话消耗卡（**四格 + 余量行**）。
fn render_session_card(view: &SessionTotalsView, rows: &[BudgetRemainingRow]) -> VNode {
    let count = format!("{SESSION_CARD_COUNT_PREFIX}{}", view.turns);
    let remaining: Vec<VNode> = rows.iter().map(render_remaining_row).collect();
    let card_title_text = SESSION_CARD_TITLE;
    let tokens_label = METER_TOKENS_LABEL;
    let cache_label = METER_CACHE_LABEL;
    let turns_label = METER_TURNS_LABEL;
    let duration_label = METER_DURATION_LABEL;
    let remaining_title_text = REMAINING_TITLE;

    view! {
        div(class = "cap-card") {
            div(class = "cap-card-head") {
                span(class = "cap-card-title") {
                    { Icon::render(&IconProps::new(&frame_icons::ICON_ACTIVITY, 13)) }
                    card_title_text
                }
                " "
                span(class = "cap-card-count") { count }
            }
            " "
            div(class = "meter-grid") {
                div(class = "meter-cell") {
                    small { tokens_label }
                    " "
                    b { view.tokens.clone() }
                }
                " "
                div(class = "meter-cell") {
                    small { cache_label }
                    " "
                    b { view.cache.clone() }
                }
                " "
                div(class = "meter-cell") {
                    small { turns_label }
                    " "
                    b { view.turns.clone() }
                }
                " "
                div(class = "meter-cell") {
                    small { duration_label }
                    " "
                    b { view.duration.clone() }
                }
            }
            " "
            div(class = "budget-remaining") {
                div(class = "remaining-title") { remaining_title_text }
                " "
                for row in remaining {
                    { row }
                }
            }
        }
    }
}

/// 渲染一行余量（**空白与结构逐字照抄**）。
fn render_remaining_row(row: &BudgetRemainingRow) -> VNode {
    // 无占比 ⇒ **空片段**（不产出任何节点）⇒ 与生产的 `{#if}` 假分支等价。
    let fill = match row.ratio {
        None => VNode::fragment(Vec::new()),
        Some(ratio) => {
            let style = remaining_fill_style(ratio);
            view! {
                span(class = "remaining-bar", aria-hidden = "true") {
                    span(class = "remaining-fill", style = style) { }
                }
            }
        }
    };

    let cap_text = format!("上限 {}", row.cap);
    let used_text = format!("已耗 {}", row.used);
    let left_text = format!("余 {}", row.remaining);

    view! {
        div(class = "remaining-row") {
            span(class = "remaining-label") {
                row.label.clone()
                small { row.scope.clone() }
            }
            " "
            span(class = "remaining-num") { cap_text }
            " "
            span(class = "remaining-num") { used_text }
            " "
            span(class = "remaining-num remaining-strong") { left_text }
            " "
            { fill }
        }
    }
}

/// 渲染预算旋钮卡（**三行数字步进器**）。
fn render_knob_card(props: &BudgetPanelProps, budget: &EffectiveBudget) -> VNode {
    // ⚠️ **结构分支**：生产里 `{#if budgetFeedback[row.key]}` 在**每一行之后**
    // ⇒ 迭代体 = 「行 + 一个空白 + （有反馈时的）反馈块」。
    let mut items: Vec<VNode> = Vec::new();
    for row in &props.rows {
        items.push(render_knob_row(props, row, budget.get(row.key)));
        items.push(VNode::text(" "));
        let feedback = props.feedback.get(row.key);
        if !feedback.is_empty() {
            items.push(render_feedback(feedback));
        }
    }
    let card_title_text = KNOB_CARD_TITLE;
    let card_count = KNOB_CARD_COUNT;

    view! {
        div(class = "cap-card") {
            div(class = "cap-card-head") {
                span(class = "cap-card-title") {
                    { Icon::render(&IconProps::new(&ICON_TIMER, 13)) }
                    card_title_text
                }
                " "
                span(class = "cap-card-count") { card_count }
            }
            " "
            for item in items {
                { item }
            }
        }
    }
}

/// 渲染提交反馈块（**钳制 / 回默认的人话提示**；无反馈 ⇒ 不渲染）。
fn render_feedback(feedback: &str) -> VNode {
    let text = feedback.to_owned();
    view! {
        div(class = "budget-feedback") {
            { Icon::render(&IconProps::new(&ICON_INFO, 12)) }
            " "
            span { text }
        }
    }
}

/// 渲染一行旋钮（**结构分支**：徽标的两个互斥 class）。
fn render_knob_row(props: &BudgetPanelProps, row: &KnobRow, badge: &BudgetBadge) -> VNode {
    let flag = FieldFlag::render(&FieldFlagProps::new(props.flags.get(row.key)));
    let configured = badge.source == BudgetSource::Configured;
    let note = badge.note.clone();
    let badge_text = badge.label.clone();
    let desc_line = format!("{} {}", row.desc, row.hint);
    let input = render_number_input(props, row);

    view! {
        div(class = "cap-row cap-row-static") {
            span(class = "cap-icon") {
                { Icon::render(&IconProps::new(row.icon, 15)) }
            }
            " "
            span(class = "cap-text") {
                strong {
                    row.label.clone()
                    code(class = "cap-env") { row.env.clone() }
                    " "
                    { flag }
                    " "
                    span(
                        class = "cap-default-badge budget-badge",
                        class:on = configured,
                        class:note = !configured,
                        title = note
                    ) {
                        badge_text
                    }
                }
                " "
                small { desc_line }
            }
            " "
            span(class = "cap-slider-wrap") {
                { input }
            }
        }
    }
}

/// 渲染数字步进器（**原生控件**；`value` 走 **property** ⇒ DOM 上**无该属性**）。
///
/// ## 两条硬口径
///
/// - `max` 为「无上界」⇒ **属性不出现**（**不是空串**）；
/// - `value` 是 **property**（生产用 `bind:value`）⇒ **本组件不写 `value` 属性**
///   ⇒ 由宿主在挂载后写 property（见 `apeireth-ui-demo` 的预算宿主模块）。
fn render_number_input(props: &BudgetPanelProps, row: &KnobRow) -> VNode {
    let mut node = VNode::element("input")
        .attr("type", "number")
        .attr("step", "1")
        .attr("min", row.min.to_string());
    if let Some(max) = row.max {
        node = node.attr("max", max.to_string());
    }
    node = node.attr("aria-label", row.label.clone());

    let key = row.key;
    let on_input = props.on_knob_input.clone();
    let on_commit = props.on_knob_commit.clone();
    // ⭐ **property 通道**（2026-10-25 补）：生产的原生控件是 `bind:value`
    // ⇒ 取值**受控** ⇒ 组件用 `prop("value", …)` **声明**、引擎写入
    // ⇒ ⚠️ 此前本控件**既没有** `prop("value")`、**又**靠事件负载取值
    // ⇒ 而事件负载此前**永远是空的** ⇒ ⇒ **幽灵输入**（实测）：
    // 键入 `87` ⇒ DOM 显示 `87`（浏览器自己画的）⇒ 引擎草稿却是 `""`
    // ⇒ ⇒ 失焦后引擎**不接管**（参考侧会归一化并写回）⇒ ⇒ 用户输入**没进引擎状态**。
    // ⇒ ⇒ 现在两条通道都在：事件负载带**用户刚敲的值**，property 带**引擎的草稿**。
    let draft_value = props.draft.get(key).to_owned();
    node = node.prop("value", draft_value);
    node = node.handler(
        EventKind::Input,
        ElementHandler::new(move |event: &mut DomEvent| {
            let value = event.target_value().unwrap_or_default().to_owned();
            on_input(key, value);
        }),
    );
    let commit_on_blur = on_commit.clone();
    node = node.handler(
        EventKind::FocusOut,
        ElementHandler::new(move |_event: &mut DomEvent| {
            commit_on_blur(key);
        }),
    );
    node = node.handler(
        EventKind::KeyDown,
        ElementHandler::new(move |event: &mut DomEvent| {
            if is_text_commit_key(event.key()) {
                event.prevent_default();
                on_commit(key);
            }
        }),
    );
    node.build()
}

/// 回车是否算「提交」（**生产 `isTextCommitKey`**：回车 ⇒ 提交）。
fn is_text_commit_key(key: Option<&str>) -> bool {
    matches!(key, Some("Enter"))
}

/// 渲染多维配额卡（**四行「暂无接口」**）。
fn render_quota_card(props: &BudgetPanelProps) -> VNode {
    let status_label = QUOTA_STATUS_LABEL;
    let rows: Vec<VNode> = props
        .quota_dimensions
        .iter()
        .map(|dim| {
            let evidence = dim.evidence.clone();
            view! {
                div(class = "cap-row cap-row-static") {
                    span(class = "cap-icon") {
                        { Icon::render(&IconProps::new(&frame_icons::ICON_GAUGE, 15)) }
                    }
                    " "
                    span(class = "cap-text") {
                        strong { dim.label.clone() }
                        " "
                        small { dim.reason.clone() }
                    }
                    " "
                    span(class = "quota-none", title = evidence) { status_label }
                }
            }
        })
        .collect();
    let card_title_text = QUOTA_CARD_TITLE;
    let card_count = QUOTA_CARD_COUNT;

    view! {
        div(class = "cap-card") {
            div(class = "cap-card-head") {
                span(class = "cap-card-title") {
                    { Icon::render(&IconProps::new(&ICON_SLIDERS_HORIZONTAL, 13)) }
                    card_title_text
                }
                " "
                span(class = "cap-card-count") { card_count }
            }
            " "
            for row in rows {
                { row }
            }
        }
    }
}

/// 渲染预算耗尽行为卡（**固定语义说明**；后端无可配置语义 ⇒ 不出选择器）。
fn render_exhaustion_card(props: &BudgetPanelProps) -> VNode {
    let behaviors: Vec<VNode> = props
        .exhaustion_behaviors
        .iter()
        .map(|behavior| {
            view! {
                li { behavior.clone() }
            }
        })
        .collect();
    let badge_text = EXHAUSTION_BADGE;
    let title_text = EXHAUSTION_TITLE;
    let lede_text = EXHAUSTION_LEDE;

    view! {
        div(class = "cap-default-card") {
            span(class = "cap-default-badge note") {
                { Icon::render(&IconProps::new(&ICON_INFO, 11)) }
                badge_text
            }
            " "
            strong(class = "cap-default-title") { title_text }
            " "
            p(class = "cap-default-text") { lede_text }
            " "
            ul(class = "budget-behaviors") {
                for behavior in behaviors {
                    { behavior }
                }
            }
        }
    }
}

/// 渲染运行时诊断分区（**只渲染触发按钮**）。
///
/// ⚠️ **诊断结果区未迁**：`StatusBadge` 是**外部组件**（带自己的 scoped CSS
/// ⇒ 未抽取 ⇒ 引擎侧无法逐字复现）⇒ 本面**不产出** `.diag-results`。
fn render_runtime(props: &BudgetPanelProps) -> VNode {
    let checking = props.checking_runtime;
    let on_check = props.on_check_runtime.clone();
    let heading_text = RUNTIME_BLOCK_TITLE;
    let desc_text = RUNTIME_BLOCK_DESC;

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { heading_text }
            " "
            p(class = "block-desc") { desc_text }
            " "
            { render_diagnostics_button(checking, on_check) }
            " "
        }
    }
}

/// 渲染诊断按钮（**静态 `disabled` ⇒ `disabled=""`** ⇒ **不是属性缺失**）。
fn render_diagnostics_button(checking: bool, on_check: Rc<dyn Fn()>) -> VNode {
    let icon = if checking {
        Icon::render(&IconProps::new(&frame_icons::ICON_ROTATE_CCW, 13).extra_class("spin"))
    } else {
        Icon::render(&IconProps::new(&frame_icons::ICON_ROTATE_CCW, 13))
    };
    let label_text = if checking {
        DIAGNOSTICS_BUSY_LABEL
    } else {
        DIAGNOSTICS_LABEL
    };

    let mut button = VNode::element("button").attr("class", "quiet-button");
    if checking {
        button = button.attr("disabled", "");
    }
    button = button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| {
            on_check();
        }),
    );
    button
        .child(icon)
        .child(VNode::text(" "))
        .child(view! { span { label_text } })
        .build()
}

/// 预算面的**宿主状态**（**信号**；调用方持有 ⇒ 组件是纯函数）。
///
/// ## 为什么状态在宿主
///
/// 数字步进器是**受控输入**（生产 `bind:value` + 失焦 / 回车提交）⇒
///
/// - 草稿值（`Signal<String>`）由宿主持有（输入框的取值是 **property**）；
/// - 反馈文案（`Signal<String>`）由宿主按**迁过来的解析函数**算出。
///
/// ⇒ 组件只渲染 + 发意图。
#[derive(Clone)]
pub struct BudgetState {
    /// 响应式作用域（**使用者形态** ⇒ 本状态**挂不了 effect**）。
    scope: ReactiveScope,
    /// 旋钮草稿（回合轮数上限）。
    draft_max_turn_rounds: Signal<String>,
    /// 旋钮草稿（单轮工具调用上限）。
    draft_max_tool_calls: Signal<String>,
    /// 旋钮草稿（上下文注入字符预算）。
    draft_context_budget_chars: Signal<String>,
    /// 旋钮反馈（回合轮数上限）。
    feedback_max_turn_rounds: Signal<String>,
    /// 旋钮反馈（单轮工具调用上限）。
    feedback_max_tool_calls: Signal<String>,
    /// 旋钮反馈（上下文注入字符预算）。
    feedback_context_budget_chars: Signal<String>,
    /// 「正在诊断」。
    checking_runtime: Signal<bool>,
}

impl BudgetState {
    /// 建一份状态。
    /// ⭐ **接受宿主给的响应式作用域**（**使用者形态**；2026-10-26 升级）。
    ///
    /// ## 为什么不能自建
    ///
    /// 此前本构造**自建**一个 `Runtime` ⇒ ⇒ 草稿 / 反馈信号**不属于**宿主
    /// 挂 effect 的那个 runtime ⇒ ⇒ **effect 根本不依赖这些信号**
    /// ⇒ ⇒ 改草稿**不触发重渲染** ⇒ ⇒ **幽灵输入**（实测）：
    /// 键入 `87` ⇒ DOM 显示 `87`（浏览器自己画的）⇒ 引擎侧草稿/反馈**不更新**
    /// ⇒ ⇒ 失焦后引擎**不接管**（参考侧会钳制并写回）。
    ///
    /// ## 为什么是「作用域」而不是「运行时」（**类型层面的强制**）
    ///
    /// 2026-10-25 的修法只把 `Runtime` 改成**参数** ⇒ ⇒ 但 [`Runtime::new`]
    /// 仍是 `pub` ⇒ ⇒ 「状态对象自建运行时」在**类型层面**仍然写得出来
    /// ⇒ ⇒ 编译器**不可能**发现 ⇒ ⇒ 只能靠脚本近似检查。
    ///
    /// ⇒ ⇒ 现在本状态持有的是 [`ReactiveScope`]（**使用者形态**）：
    /// 它**能**建信号 / 派生，**不能**挂 effect、**不能** flush、**不能**造新的
    /// 运行时 ⇒ ⇒ **状态持有者在类型层面拿不到「渲染链的另一端」**
    /// ⇒ ⇒ 「信号与 effect 分属两个运行时」这个结构条件**被切断**
    /// ⇒ ⇒ 「幽灵输入」不再可能发生（证明见 `ui-core` 的 `compile_fail`
    /// doc-test）。
    ///
    /// ## 唯一正确的用法
    ///
    /// ⭐ **这段是**可运行的 doc-test（不是 `ignore`）⇒ ⇒ 「宿主建运行时 →
    /// 挂 effect → 交作用域」这条链**每次构建都在跑** ⇒ ⇒ 「幽灵输入」的
    /// 回归护栏。
    ///
    /// ```
    /// use apeireth_ui_core::Runtime;
    /// use apeireth_ui_kit::settings_budget::{BudgetState, KnobKey};
    ///
    /// // 宿主（应用根）建运行时 ⇒ ⇒ 挂渲染 effect ⇒ ⇒ 把作用域交给状态持有者。
    /// let runtime = Runtime::new();
    /// let state = BudgetState::new(runtime.reactive_scope());
    ///
    /// let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    /// let sink = seen.clone();
    /// let watched = state.clone();
    /// runtime.effect(move || {
    ///     sink.borrow_mut().push(watched.draft().max_turn_rounds);
    /// });
    /// runtime.flush();
    ///
    /// // 状态持有者改草稿 ⇒ ⇒ 宿主的 effect 必须被驱动（**不是幽灵输入**）。
    /// state.set_draft(KnobKey::MaxTurnRounds, String::from("87"));
    /// runtime.flush();
    /// assert_eq!(
    ///     seen.borrow().as_slice(),
    ///     &[String::from(""), String::from("87")],
    ///     "改草稿必须驱动宿主 effect"
    /// );
    /// ```
    pub fn new(scope: ReactiveScope) -> Self {
        Self {
            draft_max_turn_rounds: scope.signal(String::new()),
            draft_max_tool_calls: scope.signal(String::new()),
            draft_context_budget_chars: scope.signal(String::new()),
            feedback_max_turn_rounds: scope.signal(String::new()),
            feedback_max_tool_calls: scope.signal(String::new()),
            feedback_context_budget_chars: scope.signal(String::new()),
            checking_runtime: scope.signal(false),
            scope,
        }
    }

    /// 响应式作用域（**使用者形态**）。
    ///
    /// ⚠️ **只给宿主做「同一个运行时」的复核用**（[`ReactiveScope::same_runtime`]）
    /// ⇒ ⇒ 它**不是**渲染入口（渲染入口在**应用根**）。
    pub fn scope(&self) -> &ReactiveScope {
        &self.scope
    }

    /// 旋钮草稿快照。
    pub fn draft(&self) -> KnobDraft {
        KnobDraft {
            max_turn_rounds: self.draft_max_turn_rounds.get(),
            max_tool_calls: self.draft_max_tool_calls.get(),
            context_budget_chars: self.draft_context_budget_chars.get(),
        }
    }

    /// 旋钮反馈快照。
    pub fn feedback(&self) -> KnobFeedback {
        KnobFeedback {
            max_turn_rounds: self.feedback_max_turn_rounds.get(),
            max_tool_calls: self.feedback_max_tool_calls.get(),
            context_budget_chars: self.feedback_context_budget_chars.get(),
        }
    }

    /// 「正在诊断」态。
    pub fn checking_runtime(&self) -> bool {
        self.checking_runtime.get()
    }

    /// 写某枚旋钮的草稿（**宿主侧** ⇒ 组件不自己改状态）。
    pub fn set_draft(&self, key: KnobKey, value: String) {
        match key {
            KnobKey::MaxTurnRounds => self.draft_max_turn_rounds.set(value),
            KnobKey::MaxToolCalls => self.draft_max_tool_calls.set(value),
            KnobKey::ContextBudgetChars => self.draft_context_budget_chars.set(value),
        }
    }

    /// 写某枚旋钮的反馈。
    pub fn set_feedback(&self, key: KnobKey, value: String) {
        match key {
            KnobKey::MaxTurnRounds => self.feedback_max_turn_rounds.set(value),
            KnobKey::MaxToolCalls => self.feedback_max_tool_calls.set(value),
            KnobKey::ContextBudgetChars => self.feedback_context_budget_chars.set(value),
        }
    }

    /// 置「正在诊断」。
    pub fn set_checking_runtime(&self, checking: bool) {
        self.checking_runtime.set(checking);
    }
}

// ⚠️ **本类型刻意没有 `Default` 实现**（2026-10-26）。
//
// 原因：`Default` 只能写成「自建一个运行时」⇒ ⇒ 那正是「幽灵输入」的根
// ⇒ ⇒ 有 `Default` ⇒ 就有一条**隐式**的自建路径 ⇒ ⇒ 类型层面的收口被绕过。
// ⇒ ⇒ 想要一份默认状态，调用方**必须**显式给出一个作用域：
// `BudgetState::new(runtime.reactive_scope())`。

#[cfg(test)]
mod tests {
    //! 预算面的**行为**断言（**不读生产源码**；只断言本模块的产物）。

    use super::*;
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::EventKind;

    /// 一枚旋钮行（测试用）。
    fn knob_row(key: KnobKey, min: i64, max: Option<i64>, fallback: i64) -> KnobRow {
        KnobRow {
            key,
            label: String::from("测试旋钮"),
            env: String::from("APEIRETH_TEST_KNOB"),
            desc: String::from("测试说明"),
            min,
            max,
            fallback,
            out_of_range: OutOfRange::Clamp,
            icon: &ICON_TIMER,
            hint: String::from("测试提示"),
        }
    }

    /// 三行注册表（**顺序 = 生产顺序**）。
    ///
    /// ⚠️ 上下文字符预算是 **`reject` 规格**（生产 `outOfRange: 'reject'`）⇒
    /// 越界**回默认**（不钳制）⇒ 与另两枚（`clamp`）不同。
    fn rows() -> Vec<KnobRow> {
        let mut turn = knob_row(KnobKey::MaxTurnRounds, 1, Some(64), 8);
        turn.out_of_range = OutOfRange::Clamp;
        let mut tool = knob_row(KnobKey::MaxToolCalls, 1, Some(64), 16);
        tool.out_of_range = OutOfRange::Clamp;
        let mut context = knob_row(KnobKey::ContextBudgetChars, 1, None, 24000);
        context.out_of_range = OutOfRange::Reject;
        vec![turn, tool, context]
    }

    /// 固定数据（**两侧同源**的口径）。
    fn totals() -> SessionUsageTotals {
        SessionUsageTotals {
            turns: 7,
            prompt_tokens: Some(18420),
            completion_tokens: Some(3120),
            cache_hit_tokens: Some(9120),
            duration_ms: Some(214000),
            tool_calls: 12,
            last_turn_tool_calls: Some(3),
        }
    }

    /// 最小 props（**意图出口是空实现**）。
    fn props() -> BudgetPanelProps {
        BudgetPanelProps {
            section: BudgetSection::Budget,
            totals: totals(),
            knobs: KnobValues {
                max_turn_rounds: Some(8),
                max_tool_calls: Some(16),
                context_budget_chars: Some(24000),
            },
            rows: rows(),
            draft: KnobDraft {
                max_turn_rounds: String::from("8"),
                max_tool_calls: String::from("16"),
                context_budget_chars: String::from("24000"),
            },
            feedback: KnobFeedback::default(),
            flags: KnobFlags::default(),
            quota_dimensions: vec![QuotaDimension {
                key: String::from("tokens"),
                label: String::from("Token 上限"),
                reason: String::from("没有配置/env 接线——暂无接口。"),
                evidence: String::from("cognitive_quota_scheduler.rs :: CognitiveQuota.max_tokens"),
            }],
            exhaustion_behaviors: vec![String::from("轮预算耗尽：回合收束为错误帧。")],
            checking_runtime: false,
            on_knob_input: Rc::new(|_key, _value| {}),
            on_knob_commit: Rc::new(|_key| {}),
            on_check_runtime: Rc::new(|| {}),
        }
    }

    /// 收集全部 class token（去重、保序）。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token);
            }
        }
        for child in node.children() {
            classes(child, out);
        }
    }

    /// 收集文本节点（**逐个**，含空白）。
    fn text_nodes(node: &VNode, out: &mut Vec<String>) {
        match node {
            VNode::Element(element) => {
                for child in &element.children {
                    text_nodes(child, out);
                }
            }
            VNode::Text(text) => out.push(text.data.clone()),
            VNode::Fragment(children) => {
                for child in children {
                    text_nodes(child, out);
                }
            }
        }
    }

    /// 找全部带指定 class 的元素（**保序**）。
    fn find_all_by_class(node: &VNode, class: &str) -> Vec<VNode> {
        let mut found = Vec::new();
        if node.class_tokens().iter().any(|token| token == class) {
            found.push(node.clone());
        }
        for child in node.children() {
            found.extend(find_all_by_class(child, class));
        }
        found
    }

    /// 取属性的文本值。
    fn attr(node: &VNode, name: &str) -> Option<String> {
        node.attr_value(name)
            .and_then(|value| value.to_attr_string())
    }

    #[test]
    fn the_count_formatting_adds_group_separators() {
        assert_eq!(format_count(Some(18420)), "18,420");
        assert_eq!(format_count(Some(3120)), "3,120");
        assert_eq!(format_count(Some(999)), "999");
        assert_eq!(format_count(Some(1000)), "1,000");
        assert_eq!(format_count(Some(0)), "0");
        assert_eq!(format_count(None), "—");
    }

    #[test]
    fn the_duration_formatting_matches_the_script_side() {
        assert_eq!(format_duration(Some(214000)), "3m34s");
        assert_eq!(format_duration(Some(820)), "820ms");
        assert_eq!(format_duration(Some(3200)), "3.2s");
        assert_eq!(format_duration(Some(15000)), "15s");
        assert_eq!(format_duration(Some(0)), "0ms");
        assert_eq!(format_duration(None), "—");
    }

    #[test]
    fn the_cache_rate_rounds_half_towards_positive_infinity() {
        assert_eq!(cache_rate(Some(9120), Some(18420)), "50%");
        assert_eq!(cache_rate(Some(1), Some(200)), "1%");
        assert_eq!(cache_rate(Some(5), Some(0)), "—");
        assert_eq!(cache_rate(None, Some(10)), "—");
    }

    #[test]
    fn the_session_totals_view_reports_every_slot() {
        let view = format_session_totals(&totals());
        assert_eq!(view.tokens, "入 18,420 / 出 3,120");
        assert_eq!(view.cache, "命中 9,120 · 50%");
        assert_eq!(view.turns, "7");
        assert_eq!(view.duration, "3m34s");
        assert_eq!(view.tool_calls, "12");
    }

    #[test]
    fn the_remaining_rows_keep_the_honest_dashes() {
        let budget = effective_budget(
            &KnobValues {
                max_turn_rounds: Some(8),
                max_tool_calls: Some(16),
                context_budget_chars: Some(24000),
            },
            &rows(),
        );
        let remaining = budget_remaining_rows(&totals(), &budget);
        assert_eq!(remaining.len(), 7);
        assert_eq!(remaining[0].cap, "—");
        assert_eq!(remaining[0].used, "18,420");
        assert_eq!(remaining[0].remaining, "—");
        assert_eq!(remaining[0].ratio, None);
        assert_eq!(remaining[5].cap, "16 · configured");
        assert_eq!(remaining[5].used, "3");
        assert_eq!(remaining[5].remaining, "13");
        assert_eq!(remaining[5].ratio, Some(0.1875));
        assert_eq!(remaining[6].cap, "8 · configured");
        assert_eq!(remaining[6].used, "—");
        assert_eq!(remaining[6].ratio, None);
    }

    #[test]
    fn the_badge_reports_the_configured_source() {
        let budget = effective_budget(
            &KnobValues {
                max_turn_rounds: Some(8),
                max_tool_calls: Some(16),
                context_budget_chars: Some(24000),
            },
            &rows(),
        );
        assert_eq!(budget.max_turn_rounds.source, BudgetSource::Configured);
        assert_eq!(budget.max_turn_rounds.label, "生效 8 · configured");
        assert_eq!(budget.max_tool_calls.label, "生效 16 · configured");
        assert_eq!(budget.context_budget_chars.label, "生效 24000 · configured");
    }

    #[test]
    fn an_unconfigured_knob_falls_back_to_the_constant() {
        let budget = effective_budget(&KnobValues::default(), &rows());
        assert_eq!(budget.max_turn_rounds.source, BudgetSource::Constant);
        assert_eq!(budget.max_turn_rounds.label, "生效 8 · constant");
        assert_eq!(budget.max_tool_calls.label, "生效 16 · constant");
        assert_eq!(budget.context_budget_chars.label, "生效 24000 · constant");
    }

    #[test]
    fn the_fill_percent_rounds_half_towards_positive_infinity() {
        // ⭐ 本面最容易错的一处：`Math.round` 是**半数向 +∞**。
        assert_eq!(remaining_fill_percent(0.1875), 19);
        assert_eq!(remaining_fill_percent(0.335), 34);
        assert_eq!(remaining_fill_percent(0.0), 0);
        assert_eq!(remaining_fill_percent(1.0), 100);
        // 负数：脚本侧给 -2，原生 `f64::round` 给 -3 ⇒ 本实现必须给 -2。
        assert_eq!(remaining_fill_percent(-0.025), -2);
        assert_eq!(remaining_fill_percent(-0.015), -1);
    }

    #[test]
    fn the_fill_style_keeps_the_trailing_semicolon() {
        assert_eq!(remaining_fill_style(0.1875), "width: 19%;");
        assert_eq!(remaining_fill_style(0.335), "width: 34%;");
    }

    #[test]
    fn the_knob_resolution_clamps_or_rejects() {
        let rows = rows();
        let clamped = resolve_budget_limit_input("100", &rows[0]);
        assert_eq!(clamped.value, Some(64));
        assert_eq!(clamped.status, KnobResolutionStatus::Clamped);
        assert_eq!(clamped.feedback, "越界：已钳制到 64（1..=64）");

        let rejected = resolve_budget_limit_input("0", &rows[2]);
        assert_eq!(rejected.value, None);
        assert_eq!(rejected.status, KnobResolutionStatus::Fallback);
        assert_eq!(rejected.feedback, "越界（1 起）：已回默认 24000");

        let illegal = resolve_budget_limit_input("abc", &rows[0]);
        assert_eq!(illegal.value, None);
        assert_eq!(illegal.feedback, "非法值：已回默认 8");

        let ok = resolve_budget_limit_input("12", &rows[0]);
        assert_eq!(ok.value, Some(12));
        assert_eq!(ok.feedback, "");
    }

    #[test]
    fn the_budget_section_renders_the_four_cards() {
        let node = BudgetPanel::render(&props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "setting-block",
            "block-title",
            "block-desc",
            "cap-card",
            "cap-card-head",
            "cap-card-title",
            "cap-card-count",
            "meter-grid",
            "meter-cell",
            "budget-remaining",
            "remaining-title",
            "remaining-row",
            "remaining-label",
            "remaining-num",
            "remaining-strong",
            "remaining-bar",
            "remaining-fill",
            "cap-row",
            "cap-row-static",
            "cap-icon",
            "cap-text",
            "cap-env",
            "cap-default-badge",
            "budget-badge",
            "cap-slider-wrap",
            "quota-none",
            "cap-default-card",
            "cap-default-title",
            "cap-default-text",
            "budget-behaviors",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "缺 class {class}：{found:?}"
            );
        }
    }

    #[test]
    fn the_number_input_never_writes_a_value_attribute() {
        let node = BudgetPanel::render(&props());
        let wraps = find_all_by_class(&node, "cap-slider-wrap");
        assert_eq!(wraps.len(), 3, "三行旋钮");
        for wrap in wraps {
            let input = wrap.children()[0].clone();
            assert_eq!(input.tag(), Some("input"));
            assert!(
                attr(&input, "value").is_none(),
                "`value` 是 property ⇒ DOM 上**没有该属性**"
            );
        }
    }

    #[test]
    fn the_max_attribute_is_absent_when_there_is_no_upper_bound() {
        let node = BudgetPanel::render(&props());
        let wraps = find_all_by_class(&node, "cap-slider-wrap");
        let first = wraps[0].children()[0].clone();
        let third = wraps[2].children()[0].clone();
        assert_eq!(
            attr(&first, "max"),
            Some(String::from("64")),
            "有上界 ⇒ 属性存在"
        );
        assert!(
            attr(&third, "max").is_none(),
            "无上界 ⇒ **属性不出现**（不是空串）"
        );
        assert_eq!(attr(&third, "min"), Some(String::from("1")));
        assert_eq!(attr(&third, "step"), Some(String::from("1")));
        assert_eq!(attr(&third, "type"), Some(String::from("number")));
    }

    #[test]
    fn the_fill_style_is_written_verbatim() {
        let node = BudgetPanel::render(&props());
        let fills = find_all_by_class(&node, "remaining-fill");
        assert_eq!(fills.len(), 1, "只有一行有占比");
        assert_eq!(attr(&fills[0], "style"), Some(String::from("width: 19%;")));
    }

    #[test]
    fn the_badge_carries_the_mutually_exclusive_class() {
        let node = BudgetPanel::render(&props());
        let badges = find_all_by_class(&node, "budget-badge");
        assert_eq!(badges.len(), 3, "三枚徽标");
        for badge in &badges {
            let tokens = badge.class_tokens();
            assert!(tokens.iter().any(|token| token == "on"), "{tokens:?}");
            assert!(
                !tokens.iter().any(|token| token == "note"),
                "`on` 与 `note` **互斥**：{tokens:?}"
            );
        }
    }

    #[test]
    fn the_feedback_block_appears_only_when_there_is_feedback() {
        let mut props = props();
        let node = BudgetPanel::render(&props);
        assert!(
            find_all_by_class(&node, "budget-feedback").is_empty(),
            "无反馈 ⇒ 不渲染反馈块"
        );
        props.feedback.max_turn_rounds = String::from("越界：已钳制到 64（1..=64）");
        let node = BudgetPanel::render(&props);
        let blocks = find_all_by_class(&node, "budget-feedback");
        assert_eq!(blocks.len(), 1, "有反馈 ⇒ 渲染反馈块");
        let mut texts = Vec::new();
        text_nodes(&blocks[0], &mut texts);
        assert!(
            texts
                .iter()
                .any(|text| text == "越界：已钳制到 64（1..=64）"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_blank_text_nodes_are_reproduced_one_space_each() {
        let node = BudgetPanel::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        let spaces = texts.iter().filter(|text| *text == " ").count();
        // 逐项（按本测试的固定数据：三枚旋钮 / 一个配额维度 / 一条耗尽语义）：
        //   分区块 5；消耗卡 2；四格之间 3；格内 4；余量区 1；余量行内 4×7；
        //   旋钮卡 4；卡头 3；旋钮行内 2×3；旋钮说明 1×3；旋钮标题 2×3；
        //   配额卡 1；配额行内 2；配额说明 1；耗尽卡 3。
        assert_eq!(
            spaces,
            5 + 2 + 3 + 4 + 1 + 28 + 4 + 3 + 6 + 3 + 6 + 1 + 2 + 1 + 3,
            "空白文本节点逐个复现：{texts:?}"
        );
    }

    #[test]
    fn the_runtime_section_renders_the_button_and_nothing_else() {
        let mut props = props();
        props.section = BudgetSection::Runtime;
        let node = BudgetPanel::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "quiet-button"));
        assert!(
            !found.iter().any(|token| token == "diag-results"),
            "诊断结果区**未迁**（外部组件 + 其 scoped CSS 未抽取）⇒ 不渲染"
        );
        let buttons = find_all_by_class(&node, "quiet-button");
        assert_eq!(buttons.len(), 1);
        assert!(
            attr(&buttons[0], "disabled").is_none(),
            "常态 ⇒ 无 `disabled` 属性"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == "立即执行深度诊断"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_checking_state_writes_an_empty_disabled_attribute() {
        let mut props = props();
        props.section = BudgetSection::Runtime;
        props.checking_runtime = true;
        let node = BudgetPanel::render(&props);
        let buttons = find_all_by_class(&node, "quiet-button");
        assert_eq!(
            attr(&buttons[0], "disabled"),
            Some(String::new()),
            "静态 `disabled` ⇒ `disabled=\"\"`（**不是属性缺失**）"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "正在诊断…"), "{texts:?}");
    }

    #[test]
    fn the_knob_input_emits_an_intent_and_does_not_change_the_state() {
        let seen: Rc<std::cell::RefCell<Vec<(KnobKey, String)>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut props = props();
        let sink = seen.clone();
        props.on_knob_input = Rc::new(move |key, value| {
            sink.borrow_mut().push((key, value));
        });
        let node = BudgetPanel::render(&props);
        let wraps = find_all_by_class(&node, "cap-slider-wrap");
        let input = wraps[0].children()[0].clone();
        let VNode::Element(element) = &input else {
            panic!("旋钮输入框不是元素");
        };
        let mut event = DomEvent::new(EventKind::Input, Vec::new());
        let fired = element
            .handlers
            .iter()
            .find(|(kind, _)| *kind == EventKind::Input)
            .map(|(_, handler)| {
                handler.call(&mut event);
                true
            })
            .unwrap_or(false);
        assert!(fired, "输入框带 `input` 处理器");
        assert_eq!(seen.borrow().len(), 1, "发一次意图");
        assert_eq!(seen.borrow()[0].0, KnobKey::MaxTurnRounds);
    }

    #[test]
    fn the_host_state_owns_the_draft_and_the_feedback() {
        let state = BudgetState::new(Runtime::new().reactive_scope());
        assert_eq!(state.draft().max_turn_rounds, "");
        state.set_draft(KnobKey::MaxTurnRounds, String::from("12"));
        assert_eq!(state.draft().max_turn_rounds, "12");
        state.set_feedback(KnobKey::MaxTurnRounds, String::from("已钳制"));
        assert_eq!(state.feedback().max_turn_rounds, "已钳制");
        assert!(!state.checking_runtime());
        state.set_checking_runtime(true);
        assert!(state.checking_runtime());
    }

    /// ⭐ **「幽灵输入」的行为级反证**：状态持有者拿到的作用域建出来的草稿，
    /// **宿主挂在同一个运行时上的 effect 必须能驱动它**。
    ///
    /// ⇒ ⇒ 这是 2026-10-25 那个缺陷的**回归护栏**（缺陷原文：草稿信号不在
    /// 宿主的 runtime 里 ⇒ ⇒ effect 不依赖它 ⇒ ⇒ 改草稿不重渲染）。
    #[test]
    fn the_host_effect_drives_the_state_draft() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // 宿主（应用根）建运行时 ⇒ ⇒ 挂渲染 effect。
        let runtime = Runtime::new();
        // 宿主把作用域交给状态持有者。
        let state = BudgetState::new(runtime.reactive_scope());

        // 渲染 effect 读草稿 ⇒ ⇒ 依赖边建在**同一个运行时**上。
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = seen.clone();
        let watched = state.clone();
        runtime.effect(move || {
            sink.borrow_mut().push(watched.draft().max_turn_rounds);
        });
        runtime.flush();
        assert_eq!(
            seen.borrow().as_slice(),
            &[String::from("")],
            "首帧读到初值"
        );

        // 状态持有者改草稿 ⇒ ⇒ **宿主的 effect 必须被驱动**（不是幽灵输入）。
        state.set_draft(KnobKey::MaxTurnRounds, String::from("87"));
        runtime.flush();
        assert_eq!(
            seen.borrow().as_slice(),
            &[String::from(""), String::from("87")],
            "改草稿必须驱动宿主 effect（幽灵输入 = 引擎不知道用户敲的值）"
        );
    }

    /// 状态持有者与宿主**同属一个运行时**（⇒ 作用域身份可复核）。
    #[test]
    fn the_state_shares_the_host_runtime() {
        let runtime = Runtime::new();
        let state = BudgetState::new(runtime.reactive_scope());
        assert!(
            state.scope().same_runtime(&runtime.reactive_scope()),
            "状态持有者必须与宿主同属一个运行时"
        );
    }
}
