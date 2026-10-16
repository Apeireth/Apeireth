# 星舰主题档（科幻 HUD 档）— 变量对照表与效果说明

> 主题层切换，默认主题零变化。本档为可选视觉档：深空黑底 + 青蓝/琥珀发光数据。
> 配色纪律不动：存在金家族（--ap-gold*）整组不染；发光的是数据，正文可读性优先。

## 一、落地位置（纯主题层，组件结构零改动）

| 层 | 文件 | 职责 |
|---|---|---|
| 变量层（--ap-*） | src/lib/design/tokens.css | `html[data-theme='starship']` 块：核心色/档案调覆写 + HUD 数据光令牌 |
| 变量层（旧 Pattern） | src/lib/design/base.css | `html[data-theme='starship']` 块：嵌入视图消费的旧令牌随深空底反色 |
| 氛围层 | src/lib/design/hud.css（新） | 网格/扫描线/发光/角标记；全部规则以星舰主题为域，颜色零字面量（只 var()） |
| 编排 | src/styles.css | `@import './lib/design/hud.css';`（叠加接入） |
| 档位 | src/lib/theme.ts / types.ts | Theme 联合 + 目录条目 + 静态背景判定 |

## 二、变量对照表（普通档 → 星舰档）

### 2.1 核心令牌（--ap-*）

| 令牌 | 普通档（:root 默认） | 星舰档 | 视觉作用 |
|---|---|---|---|
| --ap-space-void | #07070c | #04070d | 深空底更沉一档 |
| --ap-space-horizon | #000000 | #010204 | 事件视界 |
| --ap-bone | #e8e0cc | #e4edf5 | 正文骨白 → 冷白（深空调） |
| --ap-bone-68 | rgba(232,224,204,.68) | rgba(228,237,245,.72) | 次级文字 |
| --ap-bone-42 | rgba(232,224,204,.42) | rgba(228,237,245,.5) | 弱文字（对比度提档） |
| --ap-bone-30 | rgba(232,224,204,.3) | rgba(228,237,245,.34) | 装饰文字 |
| --ap-ink | #26262a | #e8f1f8 | 用户气泡墨字改冷白（内测实锤修复：曾有同名后置深色声明把冷白覆盖成黑字，块内同名令牌只许声明一次） |
| --ap-panel | rgba(11,13,18,.82) | rgba(8,17,28,.88) | 页面面板：深空舱底 |
| --ap-panel-solid | #0b0d12 | #08121e | 专注场景不透明底 |
| --ap-shell-chip | rgba(11,13,18,.52) | rgba(10,24,38,.58) | 次要胶囊底 |
| --ap-shell-elevated | rgba(9,11,16,.96) | rgba(6,14,24,.97) | 弹出层/工作台底 |
| --ap-card | #ece7da | #0c1a2b | 内容卡：哑白 → 深空舱卡 |
| --ap-card-dark | #15181f | #10202f | 深灰卡 → 舱蓝深卡 |
| --ap-line | rgba(232,224,204,.14) | rgba(87,214,255,.16) | 细线分割：骨白 → 青蓝细线 |
| --ap-stamp | #ece7da | #bfe6f7 | 图章/编号 |
| --ap-semantic-success | #7fb894 | #4db380 | 成功（略提饱和） |
| --ap-semantic-warning | #d9a24a | #e8b25a | 警示（琥珀调） |
| --ap-semantic-danger | #c0584e | #e0685c | 危险 |
| --ap-semantic-info | #7d9cc0 | #57d6ff | 信息（青蓝） |
| --ap-register-archive-*（17 项） | 档案调浅底 | 档案调深舱反色 | 记忆/日记档案在深空底整套反色（幽灵编号 #1c3450 青辉，文字级强调沿用亮金） |

### 2.2 旧 Pattern 令牌（嵌入视图消费，base.css）

| 令牌 | 普通档 | 星舰档 |
|---|---|---|
| --bg | #0e0d10 | transparent（静态深空底透出） |
| --surface | #151419 | #081120 |
| --surface-2 | #1b1a20 | #0c1a2b |
| --surface-3 | #242229 | #122334 |
| --line | rgba(255,255,255,.08) | rgba(87,214,255,.12) |
| --line-strong | rgba(255,255,255,.15) | rgba(87,214,255,.22) |
| --text | #ebe8e2 | #dfe9f2 |
| --muted | #96919c | #8fa6b8 |
| --faint | #625f69 | #56728a |
| --amber / --amber-hi | #e7a23b / #f1b95d | #ffb454 / #ffd08a |
| --amber-wash / --amber-line | 骨白琥珀系 | 青档琥珀系（rgba(255,180,84,…)） |
| --green / --green-wash | #4db380 系 | 同值微调（rgba(77,179,128,…)） |
| --blue / --blue-wash | #70a3dc 系 | #57d6ff / rgba(87,214,255,.13) |
| --danger | #e05b50 | #e0685c |
| --shadow | 0 24px 70px rgba(0,0,0,.5) | 0 24px 70px rgba(0,0,0,.6) |
| --bg-input | — | rgba(8,17,32,.88) |

### 2.3 HUD 数据光令牌（新增，只在星舰档定义与消费）

| 令牌 | 值 | 视觉作用 |
|---|---|---|
| --ap-hud-cyan / -soft / -line | #57d6ff / rgba(87,214,255,.45) / rgba(87,214,255,.24) | 青蓝主光（描边/细线/角标） |
| --ap-hud-amber / -soft | #ffb454 / rgba(255,180,84,.5) | 琥珀数据光（遥测/警示） |
| --ap-hud-title-ink / -glow / -glow-hi | #d7ecf9 / 0 0 14px 青辉 / 0 0 22px 青辉 | 面板标题栏发光（呼吸两档） |
| --ap-hud-data-glow | 0 0 8px 青辉 | 状态条/遥测数字光晕 |
| --ap-hud-amber-glow / -glow-hi | 0 0 8px 琥珀辉 / 0 0 15px 琥珀辉 | live 遥测呼吸两档 |
| --ap-hud-danger-glow | 0 0 8px 红辉 | 危险态数据光晕 |
| --ap-hud-corner | rgba(87,214,255,.85) | 角标记括号线 |
| --ap-hud-rule | rgba(87,214,255,.22) | 标题栏细线分割（inset 画线，不占布局） |
| --ap-hud-track / -bar-fill | 青蓝轨 / 琥珀→青蓝渐变 | 数据条（遥测进度） |
| --ap-hud-sky | 网格 + 舱顶冷光 + 底部数据辉光渐变 | 深空底（整条背景值走令牌） |
| --ap-hud-veil | 椭圆压暗渐变 | 可读性托底晕影 |
| --ap-hud-scan | 青蓝单线光渐变 | 扫描线（缓动巡游） |

### 2.4 刻意不动的令牌（纪律）

| 令牌组 | 处置 | 理由 |
|---|---|---|
| --ap-gold*（存在金家族 6 项） | 整组不染 | 金色是他的存在色，任何档位不重染 |
| --ap-accent-ui / --ap-accent-ui-ink / --ap-accent-tab | 不动 | UI 配色方案是独立系统，显式选择优先 |
| 排版（--ap-font-* / --ap-voice-*）、动效时长（--ap-dur-* / --ap-ease） | 不动 | 只换色相与氛围，不动骨架 |

## 三、效果说明

- **深空黑底 + 网格氛围**：静态背景层叠 46px 青蓝网格 + 舱顶冷光 + 底部数据辉光（`--ap-hud-sky` 一条令牌），晕影托底保证正文可读。
- **扫描线**：单线光 30vh 缓动巡游（11s 线性循环）；仅在 `prefers-reduced-motion: no-preference` 启用，reduce 时停为静态高光 + 令牌层全局 `animation: none` 兜底。
- **面板标题栏发光**：页头/抽屉头/面板 h2/数据条头换冷白 + 青辉 text-shadow，标题栏下 inset 细线分割（不占布局）。
- **角标记**：既有 `.panel` / `.ap-bracket` 对角括号线换青辉 + drop-shadow，呼吸 4.6s（55% ↔ 100% 不透明度）。
- **氛围即功能**：发光的是数据——状态条 `.sb-item`（含 warn/danger 语义色阶只加光晕不改色）、遥测 `.mono-note`（等宽 + tabular-nums + 青辉；`.live` 琥珀色 + 琥珀呼吸）、数据条 `.bar i` 琥珀→青蓝热端发光。正文 `.md-body` / `.ap-voice` 显式 `text-shadow: none`。
- **对比度**：正文冷白 #dfe9f5 系对深空面板（rgba(8,17,28,.88)）对比度 ≥ 12:1；弱文字提到 50% 保证 4.5:1 以上；发光只做 text-shadow，不降低字面亮度。
- **用户气泡可读性（内测实锤修复②）**：用户气泡走 `--ap-ink`/`--ap-card` 令牌对（星舰档 = 冷白墨 #e8f1f8 对深空卡 #0c1a2b）；子树里自带墨色的两片叶子（行内代码/代码块）在星舰档经 hud.css 换 `--ap-hud-title-ink`，任何档位用户气泡文字对比度 ≥ 4.5:1（哨兵测试锁定，防同名令牌后置覆盖与叶子墨色回潮）。

## 四、接线与持久（复用既有缝，零新逻辑）

- **全局开关**：设置页「外观」区主题选择器遍历 `THEME_CATALOG`，新档自动出现（无特例分支）。
- **即点即效**：面板 `pick()` 先 `applyDocumentTheme()` 落 DOM，再走 `onSave` apply 缝。
- **持久化**：`config.theme` 在 `persistedConfig` 白名单内 → localStorage `apeireth-config`；重启 `loadConfig()` → `resolveTheme()` → `applyDocumentTheme()` 还原；非法值回落默认。
- **URL 覆写**：`?theme=starship` 仍优先于 config（既有纪律）。
- **命令面板**：`theme.starship` 命令随目录自动生成（别名含拼音）。

## 五、测试（2 套件 / 7 项断言组）

| 套件 | 覆盖 |
|---|---|
| tests/theme-hud-switch.mjs | ① 切换即效接线（apply 缝同步写 DOM + pick 顺序）② 持久化恢复（往返 + 脏值回落）③ 默认零回归（回落/目录首项/night 行为不变） |
| tests/theme-hud-tokens.mjs | ④ 令牌覆盖完整（核心/档案/旧 Pattern/HUD 全落值，存在金不动）⑤ 无硬编码漏网色值（色值只在令牌声明处）⑥ reduced-motion 生效（动效守卫 + reduce 兜底）⑦ 默认零回归（hud.css 全域限定 + 哨兵值字节不动） |
| tests/theme-bubble-contrast.mjs | ⑧ 星舰用户气泡对比度哨兵（fg/bg ≥4.5:1，含子树叶子墨色；同名令牌后置覆盖即失败）⑨ 默认主题气泡零回归（默认/浅色档令牌对与叶子墨色字节不动且 ≥4.5:1） |
