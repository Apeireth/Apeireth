# 固定数据（fixture）草案

> **两侧同源**：参考侧与引擎侧喂**同一份**这些 JSON ⇒ 渲染出的内容相同 ⇒
> 差异只可能来自渲染实现（批一口径）。
>
> **不手抄**：文案与数值逐字取自生产源码（每个文件的 `_source` 字段给出出处）。
> 迁移时**直接复用本表**，不要重新挑图标 / 改写标题 / 换示例值。

## 文件清单

| 文件 | 归属面 | 内容 | 出处 |
|---|---|---|---|
| `nav-groups.json` | **S1** | 5 个大类 / 15 个分区 / 图标映射 / 初始展开态 | `lib/settings-nav-groups.ts:18-49`、`SettingsView.svelte:1309-1336` |
| `capabilities.json` | **S3 + S4** | 8 个能力开关注册表 + 39 个开关初值 + 认知深度 + 权限预设 | `SettingsView.svelte:429-553` |
| `provider.json` | **S2** | 协议预设表（5 + 3 条）+ 连接态 + 模型列表 + 测试结果 + 网关回显 + 钥匙串回显位 | `SettingsView.svelte:781-948` |
| `character.json` | **S3** | 人设列表 + 用户印象 + 体验参数预设 + 学习日志 | `SettingsView.svelte:555-613、711-779、1050-1107、1821-1919` |
| `budget.json` | **S5** | 会话消耗 + 旋钮徽标 + 余量行 + 配额维度 + 耗尽语义 + 诊断读数 | `lib/budget.ts`、`lib/chat-shell/turn-telemetry.ts` |
| `data.json` | **S6** | 运维读数 + 启停指引 + 工作区 + 用户资料 + 危险动作态 | `SettingsView.svelte:1247-1284、1536-1597、1715-1755、1775-1819` |
| `theme.json` | **T1** | 10 个主题卡（swatch 渐变原文）+ 4 个配色卡（色值原文） | `lib/theme.ts:13-108、132-152` |
| `wizard.json` | **W1** | 6 个步骤标题 + 5 条预设 + 介绍要点 + 预告 + 专业参考卡 | `lib/onboarding.ts:30-76`、`FirstRunWizard.svelte:32-97` |

## 约定

| 约定 | 说明 |
|---|---|
| `_note` / `_source` | 每个文件开头的说明字段（**不渲染**，对照前会被归一化剔除） |
| `_*` 前缀的键 | 说明性键（不渲染） |
| 缺字段 = 「未定义」 | 引擎侧用 `Option` 表达（批一已定的口径）⇒ **不要用空串冒充「未定义」** |
| 空数组 = 「空态」 | 生产的空态分支（如「暂无自动调整记录」「无匹配模型」）**必须**对照 |

## 待评审项（本批只给草案）

| # | 待定 | 谁定 |
|---|---|---|
| 1 | `provider.json` 里 `minimax-M3` 的**大小写**（生产的 `OPENAI_PRESETS` 里写的是 `minimax-M3`，而 `FirstRunWizard` 的 `PRESETS` 里写的是 `minimax-M3` —— **两处不一致**）⇒ 迁移时**按所在面的原文**逐字取，别互相"统一" | 迁移线 |
| 2 | `data.json` 的 `avatar` 空值 ⇒ 像素对照时用**空**（走 fallback 图标分支）还是给一张**确定的** data URL | 迁移线 |
| 3 | `theme.json` 的 `customPreviewUrl` ⇒ `objectURL` 每次不同 ⇒ 对照时两侧都用**同一个固定 URL** | 迁移线 |
| 4 | `capabilities.json` 的 39 个开关初值 ⇒ 取生产的 `DEFAULT_CAPABILITY_TOGGLES` 还是自定义 | 迁移线 |

## 生成物（Rust 侧）

与批一同构：fixture JSON ⇒ **Rust 生成物**（`crates-ui/apeireth-ui-demo/src/settings_fixture_generated.rs`）
⇒ **由抽取器生成**（`scripts/extract-settings-assets.mjs`）⇒ **不手抄进 Rust**。
⇒ 生成物落在**共享 crate** ⇒ 共享文件的改动由**主线统一做**（README §8-2）。
