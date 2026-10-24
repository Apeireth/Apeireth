# 全栈纯 Rust 前端引擎 —— 施工计划书（分阶段 / 验收门 / write_scope）

```
[Document-Meta]
Document:        docs/04-internal/ui-engine-plan.md
Version:         1.0
Last-Modified:   2026-10-24
Status:          🟡 计划（待实施）
Author:          前端重构项目组 · ui-planner
口径真源:        上位决策 = docs/adr/0015-pure-rust-ui-engine.md；宪章 = docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md
```

> 给谁看：接续本计划的人或 AI——**不读任何对话记录也能接手**。
> 纪律：每个数字随行三件（命令 + 日期 + 工具链）；未实跑的一律写「未实测」，不写估计。
> 数字口径：本文 §1 的全部数字**由本批亲跑实测**（2026-10-24，rustc 1.97.1 / cargo 1.97.1 / node v24.21.0 / pnpm 12.15.0；
> 前端解析链 svelte 5.56.9 / typescript 5.9.3 / vite 6.4.3，版本取自 `pnpm-lock.yaml` 实际解析值），
> 与 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md) 同为「数字单一真源」纪律的适用面；
> 但**本文只报前端仓 `frontend/companion-desktop/` 的实测**，主仓测试/clippy 数字仍以 test-baseline.md 为准，本文不抄写。

---

## §1 背景与问题

### 1.1 现状实测（2026-10-24 亲跑，范围 `frontend/companion-desktop/`）

工作目录 = `frontend/companion-desktop/`。命令即下表「命令」列，可逐条复跑。

| 项 | 实测值 | 命令 |
|---|---:|---|
| `.svelte` 文件 | **62 个 / 28,517 行** | `find src -name '*.svelte' \| wc -l` / `… -exec cat {} + \| wc -l` |
| `.ts` 文件 | **88 个 / 23,225 行** | `find src -name '*.ts' \| wc -l` / `… -exec cat {} + \| wc -l` |
| `.svelte` + `.ts` 合计 | **51,742 行** | 上两行相加 |
| **纯逻辑 TS**（不 import svelte、也不引用 `.svelte`） | **87 文件 / 23,209 行** | `for f in $(find src -name '*.ts'); do grep -qE "from 'svelte\|\.svelte'" "$f" \|\| cat "$f"; done \| wc -l` |
| 与 svelte 有耦合的 `.ts` | **1 文件 / 16 行**（`src/main.ts`，唯一 `import {mount} from 'svelte'`，见 `src/main.ts:1`） | 同上，取补集 |
| `.css` 文件 | 10 个 / 4,567 行 | `find src -name '*.css' …` |
| 巨型文件 | `SettingsView.svelte` **5,314** / `App.svelte` **3,608** / `runtime.ts` **2,829**（合计 11,751） | `wc -l src/lib/views/SettingsView.svelte src/App.svelte src/lib/runtime.ts` |
| Tauri 宿主 | `src-tauri/src/` **17 个 .rs / 13,476 行**；`#[tauri::command]` **45 处** | `find src-tauri/src -name '*.rs' …` / `grep -rn '#\[tauri::command\]' src-tauri/src/ \| wc -l` |
| 测试基座 | `tests/*.mjs` **84 个**；其中 **38 个**用 `readFileSync` 读源码文本断言；**47 个**真 import 生产模块（`from '../src…'`）；两者兼有 14 个；只读源码不 import 24 个；两者皆无 13 个 | `ls tests/*.mjs \| wc -l` / `grep -l 'readFileSync' tests/*.mjs \| wc -l` / `grep -lE "from '\.\./src" tests/*.mjs \| wc -l` |
| 场景层 | `src/lib/scene/blackhole.ts` **1,037 行**，含 **2 条 GLSL 程序**（vertex `:129` / fragment `:138`）；WebGL2 context 取用 3 处（`blackhole.ts:756`、`src/lib/intro/fire.ts:853`、`src/features/pet/avatar/CharacterAvatar.svelte:75`） | `wc -l` / `grep -n 'getContext('` |

**测试基座补充实测**（`tests/run-all.mjs` = 唯一 runner，`package.json` 的 `test` 脚本指向它）：
登记套件 **82 个** = 带 `strip: true`（真 import `.ts` 源码）**66 个** + 无 strip **16 个**；
后 16 个里 **14 个**在 runner 的 `name` 字段里自称 `(mirrored)` 或 `source mirror`
（例：`tests/run-all.mjs` 登记行 `shell-ia.mjs` = "shell wiring source mirrors…"）。
⇒ **源码镜像测试是本仓的既有结构，不是笔误**，它们钉的是字面量与物理位置（典型断言形态：`app.includes('await deleteSession(id, {')`）。

### 1.2 Svelte 特性使用面（口径声明在前，数字在后）

> **统计口径（必须先读）**
> 1. 范围 = `frontend/companion-desktop/src/` 下的 **62 个 `.svelte` 文件**（不含 `src-tauri/`、不含 `tests/`）。
> 2. **SCRIPT 域** = 文件内 `<script …>…</script>` 块的内容。用于 `$state` / `$derived` / `$effect` / `$props` / `$bindable` / `$inspect` / `getContext` / `setContext`。
> 3. **MARKUP 域** = **剔除全部 `<style>…</style>` 块之后**的剩余文本。用于 `{#if}` / `{#each}` / `{#snippet}` / `{#await}` / `{#key}` / `{@html}` / `{@const}` / `{@render}` / `bind:` / `on:` / `in:` / `out:` / `transition:` / `use:` / `class:` / `style:`。
> 4. **指令位置正则**：`bind:` / `on:` / `in:` / `out:` / `transition:` / `use:` / `class:` / `style:` 一律要求 `\b<指令>:[A-Za-z]`（冒号后紧跟字母），以排除 `margin:` / `transition:` 这类 **CSS 属性名误命中**。
> 5. 「处数」= 全文匹配出现次数（含同文件多次出现）；「文件数」= 至少出现 1 次的文件数。
> 6. 复跑方式 = 按上述口径重写脚本即可复跑（本批用一次性测量脚本 `/tmp/measure_svelte.py`，纯标准库，**非仓内常驻件**；口径已完整写在本节，**不依赖脚本留存**）。
>
> **为什么必须剔 `<style>`**：本仓 86 处 `transition:` **全部是 CSS 属性**（例 `src/App.svelte:3295` `transition: opacity 0.8s ease;`），
> 若不剔除 `<style>` 块，`transition:` 指令口径会被 CSS 淹没——**这正是早期口径错误的来源**（见 §1.5）。

| 须覆盖的特性 | 处数 | 文件数 | 域 |
|---|---:|---:|---|
| `{#if}` | **377** | 48 | MARKUP |
| `$state` | **376** | 45 | SCRIPT |
| `$derived` | **162** | 36 | SCRIPT |
| `{#each}` | **111** | 35 | MARKUP |
| `$props` | **55** | 55 | SCRIPT |
| `$effect` | **40** | 19 | SCRIPT |
| `bind:` | **70** | 27 | MARKUP |
| **Svelte 5 事件属性 `on*=`** | **403** | 43 | MARKUP |
| ├ `onclick` | 304 | — | MARKUP |
| ├ `onkeydown` | 41 | — | MARKUP |
| ├ `oninput` | 24 | — | MARKUP |
| ├ `onchange` | 13 | — | MARKUP |
| ├ `onfocusout` | 9 | — | MARKUP |
| └ 其余 12 类（`onmousemove`/`onblur`/`onscroll`/`onpointer*`/`onmousedown`/`onfocus`/`oncontextmenu` 各 1–2） | 12 | — | MARKUP |
| 生命周期（`onMount` / `onDestroy` / `tick` / `untrack` / `mount`） | 29 处 import（`src/` 全域） | 29 | SCRIPT |
| `class:` | 197 | 34 | MARKUP |
| `style:` | 21 | 7 | MARKUP |
| `{@render}` | 20 | 4 | MARKUP |
| `{@const}` | 18 | 12 | MARKUP |
| `{@html}` | 3 | 2 | MARKUP |
| `{#snippet}` | 2 | 2 | MARKUP |
| `{#key}` | 1 | 1 | MARKUP |
| **legacy `on:` 指令**（`on:click`） | **2** | 1 | MARKUP（`src/components/VoiceCallModal.svelte:159,171`） |

**明确 0 用的（自研引擎不实现，列入「不做」清单）**：

| 0 用特性 | 实测 | 备注 |
|---|---:|---|
| `$bindable` | **0** | 0 文件 |
| `{#await}` | **0** | 0 文件 |
| `use:`（action） | **0** | 0 文件 |
| `context=` / `getContext` / `setContext`（框架 context API） | **0** | ⚠️ 裸 `getContext(` 在 `src/` 有 **11 处**命中，**逐条核过全部是 canvas 2d / webgl 上下文**（`Skeletal2dAvatar.svelte:30`、`CharacterAvatar.svelte:75`、`VoiceCallModal.svelte:40`、`blackhole.ts:461,467,756`、`PlanetLayer.svelte:95,100,136`、`fire.ts:853`、`user-profile.ts:115`）⇒ 框架 context API 实为 0 |
| `$inspect` | **0** | 0 文件 |
| **框架级过渡指令 `in:` / `out:` / `transition:`** | **0 / 0 / 0** | 见 §1.3 专节 |
| `animate:` / `{#animate}` | **0 / 0** | 0 |
| `svelte/store` / `svelte/motion` / `svelte/transition` / `svelte/animate` 子路径 import | **0** | `grep "from 'svelte" src/` 全域只 29 处，全部是 `svelte` 裸包（`onMount`/`onDestroy`/`tick`/`untrack`/`mount`/`Component`/`Snippet` 类型） |

### 1.3 关键实测：动画责任 100% 在 CSS，框架过渡指令用量为 0

这是本计划**最容易被误判、且直接决定 P2 形态**的一条，单列。

| 事实 | 实测 | 性质 |
|---|---:|---|
| 框架进入/离场过渡指令 `in:` / `out:` | **0 处** | 框架级离场动画在本仓**不存在** |
| `transition:` 出现在 `.svelte` | **86 处 / 28 文件** | 逐条核为 **CSS 属性**，例 `src/App.svelte:3295` `transition: opacity 0.8s ease;` |
| `transition-*` 变体（`transition-property` / `transition-duration` 等） | 2 处 | 同样是 CSS |
| `@keyframes` | 36 处 / 24 文件（`.css` 内 16 处），**32 个不同关键帧名** | CSS |
| **专属动效层** | `src/lib/design/motion.css` **257 行**，含 5 个 `ap-*` 关键帧：`ap-shell-in`（壳层进入 `:11`）/ `ap-rail-glow`（导轨辉光 `:37`）/ **`ap-card-rise`（卡片浮起 `:63`）** / **`ap-fade-up`（渐显 `:82`）** / **`ap-msg-in`（消息入场 `:197`）** | CSS |
| `animation:` 声明 | 67 处 | CSS |
| `backdrop-filter` | **80 处** | CSS 滤镜层（面板模糊） |
| `<canvas>` 元素 | 5 处 / 5 文件 | 场景层 / 头像 / 语音 / 片头 |
| WebGL2 context | 3 处 | GPU 场景 |
| `requestAnimationFrame` | 21 处 / 12 文件 | rAF 驱动的状态趋近 |
| 动效时长令牌 | `src/lib/scene/tokens.ts:148-169`（`MOTION`：`cameraTweenSec: 2.6` / `uiTransitionMinSec: 2.0` / `stateSmoothingRate: 2.4` / `breatheSec: 2.8` / `timelineTransitionDefaultSec: 90`） | 单一真源 |

⇒ **结论（本计划的第一条硬结论）**：
本仓的「电影感」载体 **不在框架过渡指令里**，而在 **CSS transition/animation + WebGL shader + rAF 指数趋近** 三处，
这三处**全部在 DOM 绑定层之外**（CSS 与 GPU 由 WebView 合成器产出，不由框架调度）。

**逐条点名**（因为「电影感」是本计划最贵的东西，必须钉到具体载体）：
「消息卡浮起」= CSS 关键帧 `ap-card-rise`（`src/lib/design/motion.css:63`，`animation: ap-card-rise 0.5s var(--ap-ease) both` 见 `:49`）；
「面板渐显」= CSS 关键帧 `ap-fade-up`（`motion.css:82`，`animation: … 0.6s var(--ap-ease) both` 见 `:77,80`）；
「他的消息入场」= CSS 关键帧 `ap-msg-in`（`motion.css:197`，`animation: ap-msg-in 0.42s var(--ap-ease) both` 见 `:195`）；
「光环呼吸」= shader 内 `pulseWave` + CSS `ap-halo-breathe`（`src/lib/scene/tokens.ts:162-163`、`motion.css`）；
「胶片颗粒」= shader 内 `grainRefreshMs: 130`（`tokens.ts:165`）。
⇒ **全部 0 处经过框架过渡指令。** 早期口径把「电影感来源」记成「277 处 `in:` 过渡」是**归因错误**（§1.5）。

⇒ **对本计划的三条直接后果**：
1. 自研引擎**不需要**实现任何「过渡时序引擎 / 缓动函数库 / 关键帧求值器」——那是 CSS 的活，引擎重复实现即制造第二套真相。
2. 引擎在「动画」上**唯一的责任**是 **组件挂载/卸载时机** 与 **样式类翻转时机**：
   CSS 过渡要求元素在「旧态」与「新态」之间**跨过一次样式重算边界**；若引擎在同一帧内挂载即卸载、或在同一次任务里把类名从旧值直翻新值，
   CSS 过渡**根本不会开始播放**。这是引擎唯一能弄坏电影感的地方，也是 P2/P3 的验收靶心。
3. 引擎**不得**产生任何无真实来源的动效（设计系统 §0「视觉 0 装」：一切动态必须由后端真实状态驱动，`docs/design/01-DESIGN-SYSTEM.md:23`）。

### 1.4 结论：目标不是实现 Svelte，是覆盖实际用到的那 7 个特性（+ 1 类事件属性面）

> **一句话**：目标不是「实现 Svelte」，是「**覆盖我们实际用到的那 7 个特性**」——
> `$state` / `$derived` / `$effect` / `$props` / `{#if}` / `{#each}` / `bind:`；
> 外加 **1 类事件属性面**（Svelte 5 `on*=` 共 403 处）。
> 两者合计 = **8 类必覆盖面**。

| | 面 | 处数 |
|---|---|---:|
| **必覆盖 · 核心 7 特性** | `$state` 376 / `$derived` 162 / `$effect` 40 / `$props` 55 / `{#if}` 377 / `{#each}` 111 / `bind:` 70 | **1,191 处合计** |
| **必覆盖 · 第 8 类面** | Svelte 5 事件属性 `on*=`（含事件委托与双向绑定回写） | 403 处 |
| 次级必覆盖（量小但不能缺） | `class:` 197 / `style:` 21 / `{@render}` 20 / `{@const}` 18 / `{@html}` 3 / `{#snippet}` 2 / `{#key}` 1 / legacy `on:` 2 / 生命周期 29 | — |
| **明确不做** | `$bindable` / `{#await}` / `use:` / context API / `$inspect` / `in:` / `out:` / `transition:` / `animate:` / `svelte/store` / `svelte/motion` / `svelte/transition` / `svelte/animate` | **0** |

**这把工作量从「重造一个通用前端框架」降到「做一个够用的响应式 + DOM 绑定层」**。

### 1.5 口径更正记录（早期 Svelte 4 语法口径已被推翻）

早先流传的一版特性表用 **Svelte 4 语法口径**统计（`on:` 814 处 / `in:` 277 处 / `transition:` 86 处），
经本批逐条复核，**其中三行是统计假象**：

| 早期数字 | 复核结论 | 复核命令 |
|---|---|---|
| `on:` **814 处 / 102 文件** | **假象**。814 = 裸串 `grep -ro 'on:' src/` 的全文命中数，宿主逐条核对为 `transition:`(186) / `position:`(124) / `animation:`(86) / `reason:` / `resolution:` / `section` / `solidati` 等**英文词尾**。真实的 legacy `on:` 指令只有 **2 处 / 1 文件**；真实的 Svelte 5 事件属性是 **403 处 / 43 文件**。 | `grep -ro 'on:' src/ \| wc -l` → 814；`grep -rhoE '.{0,10}on:' src/ \| …` 拆宿主 |
| `in:` **277 处 / 61 文件** | **假象**。277 = 裸串 `grep -ro 'in:' src/`，宿主为 `margin:`(196) / `margin-origin`(21) / `min:` / `origin:` / `chain:` 等 CSS 属性。**真实的 Svelte `in:` 指令 = 0**。 | `grep -ro 'in:' src/ \| wc -l` → 277；`grep -rhoE '.{0,10}in:' src/ \| …` 拆宿主 |
| `transition:` **86 处 / 28 文件** | **数字对、语义错**。86 处确为 CSS `transition:` 属性，**不是** Svelte 过渡指令。 | `grep -rnoE '\btransition\s*:' --include='*.css' --include='*.svelte' src/` → 86 |

⇒ **早期结论「`in:` 过渡 277 处是框架提供的电影感来源」不成立**，据此设定的 P2「过渡时序等价引擎」随之作废，
按 §1.3 重定为「**CSS 过渡触发时机处理**」（见 §3 P2）。

**被误记成「框架电影感」的那几个效果，逐条归位到 CSS**（§1.3 逐条点名）：
「消息卡浮起」= `ap-card-rise`（`src/lib/design/motion.css:63`）/「面板渐显」= `ap-fade-up`（`motion.css:82`）/
「消息入场」= `ap-msg-in`（`motion.css:197`）/「光环呼吸」= shader `pulseWave`（`src/lib/scene/tokens.ts:162-163`）。

⇒ **早期结论「`in:` 过渡 277 处是框架提供的电影感来源」不成立**，据此设定的 P2「过渡时序等价引擎」随之作废，
按 §1.3 重定为「**CSS 过渡触发时机处理**」（见 §3 P2）。

### 1.6 三个真问题

1. **渲染层与测试层被编译边界割裂**：`.svelte` 无法被 node 直接 import ⇒ 84 个 `.mjs` 里 **38 个**只能读源码文本断言，
   钉的是**字面量与物理位置**。任何重构（哪怕纯搬移、语义等价）都会打碎它们。这是**结构性**债，随功能量线性放大。
2. **契约漂移风险**：前端手写 TS 类型（`runtime.ts` 2,829 行）对上 Rust 后端结构，字段名对不上时运行时才暴露。
3. **双语言工具链**：前端独立 pnpm workspace + 独立 `[workspace]` 的 Tauri crate（`src-tauri/Cargo.toml:1-4` 独立 `[workspace]` 段），
   两套构建、两套依赖、两套 CI job。

---

## §2 目标形态

### 2.1 形态链

```
Rust 源码  ──►  wasm32-unknown-unknown  ──►  Tauri WebView（现有宿主全复用）
   ▲                                                │
   └── crates-ui/ 独立 cargo workspace（不进主依赖图）  └── frontend/companion-desktop/src-tauri/ 13,476 行
```

- **目标平台已具备**：`rustup target list --installed` 实测含 `wasm32-unknown-unknown`（2026-10-24）⇒ 构建目标无需新增工具链安装。
- **宿主全复用**：Tauri 宿主 13,476 行 / 45 个 `#[tauri::command]`（托盘 / 单实例 / 开机自启 / OS keychain / 透明窗 / 后端自拉起 / 快捷窗 / 桌宠穿透）**一行不改**。
- **形态留门**：第二阶段可走原生窗口路径（弃 WebView），但**不在本计划承诺范围**，另出 ADR（见 ADR-0015 §2.5）。

### 2.2 引擎边界（硬约束，写入契约）

> **引擎只做三件事：响应式调度 / DOM 绑定 / CSS 过渡触发时机。**
>
> **不写 CSS 引擎、不写滤镜层、不碰 shader、不写过渡时序引擎（缓动/关键帧求值）、不碰排版、不碰主题。**

**为什么「不写过渡时序引擎」也进硬约束**（§1.3 的直接推论）：
本仓 0 处框架过渡指令，动画 100% 在 CSS。引擎若自建缓动/关键帧求值，等于**制造第二套动画真相**，
与「视觉层零改写」冲突，且会把 CSS 的合成器优势（GPU 加速、浏览器原生时间轴）替换成 JS 逐帧驱动。

**引擎在「动画」上唯一该做的事**：
保证「旧态 → 新态」之间**存在一次样式重算边界**，让 CSS 的 `transition:` / `animation:` / `@keyframes` 正常起播。
即：**管时机，不管曲线**。

### 2.3 视觉层零改写清单（实测载体，引擎是否触碰）

| 层 | 载体 | 实测 | 引擎是否触碰 |
|---|---|---:|:--:|
| GPU 场景 | `src/lib/scene/blackhole.ts` 1,037 行 / 2 条 GLSL（`:129` vertex / `:138` fragment）/ WebGL2 `:756` | WebGL2 | ❌ 零改写 |
| 片头火焰 | `src/lib/intro/fire.ts` / WebGL2 `:853` | WebGL2 | ❌ 零改写 |
| 3D 形象 | `three` 0.186 + `@pixiv/three-vrm` 3.5.5（`package.json`） | WebGL | ❌ 零改写 |
| 场景合成 | `src/lib/scene/SceneLayer.svelte:124` 全屏 canvas `position:absolute; inset:0` | CSS 定位 | ❌ 零改写 |
| 暗角 + 颗粒 | `#vignette` DOM 节点 + `src/lib/design/shell.css:62` | CSS | ❌ 零改写 |
| 面板模糊 | `backdrop-filter` **80 处** | CSS 滤镜层 | ❌ 零改写 |
| 动效曲线 | `src/lib/scene/tokens.ts:148-169`（`MOTION` 令牌）+ `src/lib/design/motion.css` 257 行（5 个 `ap-*` 关键帧） | CSS / shader | ❌ 零改写 |
| 排版 | 衬线栈 / 行高 2.1 / 字距 0.13em / 幽灵编号（`src/lib/design/tokens.css:118-120`） | CSS | ❌ 零改写 |
| 主题 | 多套主题 + 三调性 + `--ap-*` 令牌 + `src/lib/theme.ts` | CSS + TS | ❌ 零改写 |

**引擎替代的只是「谁决定 DOM 长什么样」——上屏像素仍由 WebView 的 GPU + CSS 合成器产出。**
「电影感优先于信息密度」（`docs/design/01-DESIGN-SYSTEM.md:33`）这条总纲因此**零风险**。

---

## §3 分阶段计划（P0 → P5+）

### 3.0 阶段总览

| 阶段 | 目标 | 交付物 | 关键验收门 | 估时（人日） |
|---|---|---|---|---:|
| **P0** | 立项 | 本计划书 + ADR-0015 + 架构文档 + `crates-ui/` 骨架 + 独立 CI | ADR accepted；`crates-ui/` 四门禁绿；主仓 `arch_guard --strict` 0 违规 | 3–5 |
| **P1** | `ui-core` 响应式核心 | 信号 / 依赖图 / 调度 / effect | ≥20 例**真测试**（native 跑，不需浏览器）；`ui-core` 0 依赖 web-sys | 5–9 |
| **P2** | `ui-dom` 绑定层 + **CSS 过渡触发时机处理** | 差分 / 事件委托 / 双向绑定 / 列表 / 挂载卸载时机 | 差分正确性测试；**挂载卸载时机 → CSS 动画是否起播**的机械测试 | 6–11 |
| **P3** | `ui-macro` 过程宏 + 审批卡试点 | `#[component]` 过程宏 + 审批卡试点应用 | **像素对照 ≤1%** + **逐帧对照** + **owner 电影感主观验收** | 8–14 |
| **P4** | 试点复盘 go/no-go | 复盘卷宗 + 决策回执 | **硬闸门**：不过即停，不硬推 | 2–3 |
| **P5+** | 扩面 | 组件库 → 逐面迁移 → GPU 场景 | 每批独立验收；另出 ADR | 未实测 |

- **估时口径**：**主观估时，未经实测**（无脚本不发布数字的纪律在「实测数字」上生效；估时栏是计划栏，不是数字真源）。
  总量级与 ADR-0015 §6.2 登记的「7,000–14,000 行引擎代码量」同源（该数亦为主代理主观加权，ADR §6.2 已如此标注）。
- **每阶段 = 一个独立施工批 = 一个独立 write_scope = 1 commit**（宪章 §二，见 §6）。
- **全程不修改 `frontend/companion-desktop/`**（ADR-0015 §5）：Svelte 前端在任何阶段都是生产在跑的那一份，回滚成本 = 一个 `git revert`。

### 3.1 P0 · 立项（本批）

- **目标**：把「全栈纯 Rust 前端引擎」从口述决策落成可复核的契约与骨架，且**主仓 0 生产改动**。
- **交付物**：
  1. `docs/adr/0015-pure-rust-ui-engine.md`（决策）——由 ADR 子线撰写，本批不碰。
  2. **`docs/04-internal/ui-engine-plan.md`（本文）**——本批唯一产出。
  3. `docs/design/ui-engine-architecture.md`（架构文档）——另线撰写。
  4. `crates-ui/` 骨架（7 crate 目录 + `Cargo.toml` + 各自 `deny.toml`）——后续批。
  5. 契约登记：`docs/architecture/contract.toml` §[workspace] / §[vendored] / §[extension_points]；**例外登记：不适用**（破「0 引新外部 dep」纪律**非架构契约违规**，理由见 §7.2 与 ADR-0015 §6.2）。
- **验收门**：
  - 本计划书落地 + 内部链接可解析（`docs/adr/0015-pure-rust-ui-engine.md` §4 指向本文 §3）；
  - `python scripts/arch_guard.py --strict` 通过且**基线净额 ≤ 0**（主 workspace 30 members / 111 边 0 改动）— **本批未实测**（本批 0 生产改动，命令留 P0 施工批跑）；
  - LOCKED 5 项 0 触碰；
  - 0 触碰 `frontend/companion-desktop/`。
- **估时**：3–5 人日（含骨架与 CI 落地）。
- **write_scope 声明**：
  - 主战场 = `docs/04-internal/ui-engine-plan.md`；
  - 禁区零触碰 = `frontend/companion-desktop/**`、`docs/adr/**`、`docs/design/**`、`crates/**`、根 `Cargo.toml` / `Cargo.lock`。

### 3.2 P1 · `ui-core` 响应式核心

- **目标**：把「状态 → 依赖 → 调度 → effect」这条链做成**不依赖浏览器**的纯逻辑层。
- **交付物**：`crates-ui/apeireth-ui-core/` —— `Signal`（读/写/订阅）、`Derived`（惰性求值 + 版本号）、依赖图（拓扑 + 环检测）、调度器（批处理 + 微任务边界）、`Effect`（依赖收集 + 清理 + 生命周期）。
  **约束**：`ui-core` 的 `Cargo.toml` **0 依赖 `web-sys` / `wasm-bindgen`**（保证能 native 单测）。
- **验收门**：
  - ≥20 例**真测试**，`cargo test -p apeireth-ui-core` 在 **native 目标**直接跑（不需要 headless 浏览器）；
  - 依赖面机械判据：`grep -E 'web-sys|wasm-bindgen' crates-ui/apeireth-ui-core/Cargo.toml` **0 命中**；
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0 警告；`cargo fmt --check` → 0 diff；
  - 环检测 / 重复订阅 / 未清理订阅 三类边界各有断言（钉行为不钉字面）。
- **估时**：5–9 人日。
- **write_scope 声明**：
  - 主战场 = `crates-ui/apeireth-ui-core/**`；
  - 随行 = `crates-ui/Cargo.toml`、`crates-ui/deny.toml`、`.github/workflows/ui-engine-ci.yml`；
  - 禁区零触碰 = `crates/**`（主 workspace）、`frontend/companion-desktop/**`、`Cargo.lock`（主仓）。

### 3.3 P2 · `ui-dom` 绑定层 + CSS 过渡触发时机处理

> **阶段名已按 §1.3 更正**：原「过渡时序等价引擎」作废（本仓 0 处框架过渡指令）。本阶段交付的是**绑定层 + 挂载/卸载时机语义**。

- **目标**：让 Rust 侧能声明式地产生 DOM，并**保证 CSS 动画不被引擎弄丢**。
- **交付物**：`crates-ui/apeireth-ui-dom/` ——
  1. **差分器**：虚拟节点树 → 真实 DOM 补丁（增/删/改/移），幂等、可重入；
  2. **事件绑定**：事件委托（一容器多监听）+ 处理器表；
  3. **双向绑定**：`bind:value` / `bind:checked` / `bind:this` 对应形态；
  4. **列表**：`{#each}` 等价形态（keyed diff + 稳定身份）；
  5. **时机语义（本阶段重点）**：`mount` / `unmount` / 类名翻转 **不落在同一次样式重算内**，保证 CSS `transition:` / `animation:` / `@keyframes` 能起播；
  6. **`ui-core` 0 依赖 web-sys 约束的镜像**：`ui-dom` 是**唯一**接触 DOM 的一层。
- **验收门**：
  - 差分正确性测试：同一状态序列跑两次 → DOM 树字节等价（幂等）；
  - **时机测试（机械判定）**：构造「旧态 → 单帧内翻转 → 新态」场景，断言浏览器侧 `getAnimations()` 非空 / `transitionrun` 事件被触发
    —— **判据：CSS 过渡确实起播**（这是 P2 的核心门）；
  - 事件委托正确性：合成事件 `bubbles: true` 路径上命中正确处理器；
  - `cargo clippy … -D warnings` → 0 警告；`cargo fmt --check` → 0 diff；
  - **wasm 构建通过**：`cargo build --target wasm32-unknown-unknown -p apeireth-ui-dom --release` 成功。
- **估时**：6–11 人日。
- **write_scope 声明**：
  - 主战场 = `crates-ui/apeireth-ui-dom/**`；
  - 随行 = `crates-ui/Cargo.toml`、`crates-ui/deny.toml`、`.github/workflows/ui-engine-ci.yml`；
  - 禁区零触碰 = `crates/**`、`frontend/companion-desktop/**`（**特别是 `src/lib/design/**` 与 `src/lib/scene/**`——视觉层零改写**）。

### 3.4 P3 · `ui-macro` 过程宏 + 审批卡试点

- **目标**：让组件成为 **Rust 函数**（而非模板字符串），并用一个真实业务面做端到端对照。
- **交付物**：
  1. `crates-ui/apeireth-ui-macro/` —— `#[component]` 过程宏（属性解析 → 生成 view 树）；`ui-macro` 是最底层，**零内部依赖**；
  2. `crates-ui/apeireth-ui-kit/` —— 试点所需的最小组件集（消息卡 / 状态条 / 按钮 / 输入）；
  3. `crates-ui/apeireth-ui-demo/` —— **审批卡试点**（对应现有 `src/lib/components/ApprovalRequestCard.svelte`，339 行），
     真后端 = `/v1/apeireth/approval-requests` + SSE `approval_required`（`src/lib/runtime.ts:307,352,1080`）。
- **试点组件选型理由**（沿用 ADR-0015 §4）：输入输出明确、有真后端、有现有 Svelte 版可逐像素对照、交互真实、
  且是「判断即仪式」范式的代表。
- **验收门**（详见 §4）：像素对照 ≤1%、**逐帧对照**（钉住组件挂载/卸载时机 ⇒ CSS 动画是否丢失）、**owner 电影感主观验收**、真测试（禁源码镜像）。
- **估时**：8–14 人日。
- **write_scope 声明**：
  - 主战场 = `crates-ui/apeireth-ui-macro/**`、`crates-ui/apeireth-ui-kit/**`、`crates-ui/apeireth-ui-demo/**`；
  - 随行 = `crates-ui/Cargo.toml`、`crates-ui/deny.toml`、`.github/workflows/ui-engine-ci.yml`；
  - **禁区零触碰 = `frontend/companion-desktop/**`（试点是独立 demo，不动生产前端）、`crates/**`、`docs/design/**`**。

### 3.5 P4 · 试点复盘 go/no-go（硬闸门）

- **目标**：以**可复核的证据**判定「继续扩面」还是「停在试点」。
- **交付物**：复盘卷宗（`docs/04-internal/ui-engine-plan.md` 回写 + 决策回执），
  含：验收门逐条结论、残留清单、**全量迁移成本重估**（把 P5+ 的组件清单与工作量按试点实测速率重算）。
- **验收门（go/no-go 判据，全部机械判定）**：
  - §4 的 12 条验收门**逐条绿**——任一条红 ⇒ **no-go**；
  - **全量迁移成本重估 ≤ 剩余预算**（判据：重估工作量 / 实测速率 ≤ 预算人日）；
  - owner 显式拍板「继续 / 停」（不接受「默认继续」）。
- **不做什么**：no-go 时**不硬推**、不为了赶进度放宽任何一条验收门；引擎停在已交付状态，**前端继续跑 Svelte**。
- **估时**：2–3 人日。
- **write_scope 声明**：
  - 主战场 = `docs/04-internal/ui-engine-plan.md`（回写）、`docs/adr/0015-pure-rust-ui-engine.md`（如需修订，走 ADR 修订流程）；
  - 禁区零触碰 = `frontend/companion-desktop/**`、`crates/**`、LOCKED 5 项。

### 3.6 P5+ · 扩面

- **P5-a 组件库补齐**：`ui-kit` 按实际使用面补齐（消息卡 / 工具卡 / 状态条 / 治理卷宗 / 设置页三页）。
- **P5-b 逐面迁移**：建议序 **审批卡 → 工具卡 → 状态条 → 治理卷宗 → 设置页三页**；
  **每一面 = 一个独立批 = 独立 write_scope = 独立验收**；每面迁移后**对应的源码镜像测试退役一条**（38 条 → 逐面归零）。
- **P5-c GPU 场景**：`apeireth-ui-gpu`（wgpu 桥）。**前置硬条件**：naga 的 GLSL 子集是否覆盖本仓 2 条 GLSL 程序（`blackhole.ts:129,138`）——**本批未实测**，见 §5 风险 R1。
  **未过前置条件不得开工**（不得把「视觉层零改写」偷偷改成「视觉层重写」）。
- **第二阶段（原生窗口路径）**：**不在本计划承诺范围**，P4 复盘时一并评估，另出 ADR。

---

## §4 试点验收门（可机械判定）

> 判据栏凡标「工具形态待定」者，表示**仓内当前无对应工具**，由该阶段立项时落地并把命令写死进 CI；**不豁免判据本身**。
> 全部门 = **AND 关系**，任一条红即 P4 no-go。

| # | 门 | 判据（命令或机械条件） |
|---|---|---|
| **G1** | **像素对照 ≤1%** | 同一视口尺寸 / 同一 DPR / 同一数据快照下，试点组件的 Svelte 版与 Rust 版各截一张图；逐像素 RGB 差分，**差异像素占比 ≤ 1%**。工具形态待定（仓内当前无像素对照工具，2026-10-24 实测 `grep -rl 'pixelmatch\|pngjs\|playwright\|puppeteer'` 于 `frontend/companion-desktop/` **0 命中**） |
| **G2** | **逐帧对照（挂载/卸载时机 ⇒ CSS 动画是否丢失）** | 录制两个版本的逐帧序列（等间隔采样，帧率与时长固定）；逐帧比对**动画是否起播**（`transitionrun` / `animationstart` 事件计数 ≥ 期望值）、**起始帧状态一致**、**结束帧状态一致**；CSS 动画起播帧数差异 **= 0**。⚠️ 理由已按 §1.3 更正：此处钉的是**时机**（元素跨样式重算边界），**不是**框架过渡曲线等价 |
| **G3** | **电影感主观验收（owner 亲审）** | owner 在同一场景下并排观看两版（试点卡浮起 / 面板渐显 / 光环呼吸 / 胶片颗粒），给出**显式通过/不通过**结论并落档；**不通过且无书面理由不重跑**——主观门是**一票否决**，不是可被其他门补偿的软指标 |
| **G4** | **真测试（禁 `readFileSync` 源码镜像）** | 试点组件的行为测试必须**调用生产模块**；机械判据：`grep -rn 'readFileSync' crates-ui/` → **0 命中**；试点测试中**无**「读源码文本断言字面量/物理位置」形态。**判据按「行为」不按「字面」**（宪章 §一.4） |
| **G5** | **wasm 构建通过** | `cd crates-ui && cargo build --target wasm32-unknown-unknown --release` 全 workspace 成功（目标已安装，2026-10-24 实测 `rustup target list --installed` 含 `wasm32-unknown-unknown`） |
| **G6** | **clippy 0** | `cd crates-ui && cargo clippy --workspace --all-targets --locked -- -D warnings` → **0 警告** |
| **G7** | **`cargo deny` 通过** | `cd crates-ui && cargo deny check` 全部通过（许可白名单 + 多版本 skip 登记在 `crates-ui/deny.toml`；形态与内容由 P0 探测结论定，见 §5 R6） |
| **G8** | **0 触碰 LOCKED** | LOCKED 5 项**逐项 0 触碰**：9 哲学锚本体 / **14 键**（`PHL-01..08`）/ 3 不可变脊柱 / `workspace.version` / R11 baseline。判据 = `git diff` 对这些面**0 行改动** |
| **G9** | **0 触碰生产前端** | `git diff --name-only` 中**无** `frontend/companion-desktop/**`；且 `pnpm --dir frontend/companion-desktop check` 与 `pnpm --dir frontend/companion-desktop test` 仍全绿（现有前端不受影响） |
| **G10** | **fmt 0 diff** | `cd crates-ui && cargo fmt --all -- --check` → 0 diff |
| **G11** | **依赖边净额 0（主 workspace）** | 主仓 `cargo metadata` 的 crate 数 / 边数与 P0 基线**逐项相等**（`python scripts/arch_guard.py --strict` 通过，基线净额 ≤ 0） |
| **G12** | **0 引新外部 dep（主 workspace）** | 主仓 `Cargo.lock` **0 行变化**；`crates-ui/` 引入的每个外部依赖在 `crates-ui/deny.toml` 许可白名单**逐个登记**（**主 workspace 不适用任何例外**——破 0-dep 纪律非架构契约违规，见 §7.2） |

---

## §5 风险登记（0 装诚实）

> 概率 / 影响 = 高 / 中 / 低。缓解栏必须给**可执行动作**，不给「注意就好」。

| # | 风险 | 概率 | 影响 | 缓解 |
|---|---|:--:|:--:|---|
| **R1** | **naga 的 GLSL 子集覆盖不了本仓 2 条 GLSL 程序**（`blackhole.ts:129` vertex / `:138` fragment）。`apeireth-ui-gpu` 若走 wgpu（glsl feature 经 naga 转译），转译可能失败或语义不等 | **低**（⚠️ **2026-10-24 P0 探测后下调**：实测 naga **能**吃下本仓 2 条 GLSL 程序 —— 删 `#version 300 es` 行 + 每个 uniform 注入 `layout(binding=N)` 后 **parse_ok=true / validate_ok=true**，16 个 uniform 全保住，6 个函数（`hash21 vnoise fbm diskRadiance starLayer` + `main`）全在；且用 `naga::back::glsl::Writer` 吐出的 17,099 字节 `#version 300 es` **能被 naga 自己重新解析并重新校验通过**） | 中 | **高** | ① **残余风险只剩三点**（不是「能不能转」而是「转得对不对」）：naga 的 GLSL 前端**无 ES profile**（`naga-30.0.1/src/front/glsl/parser.rs:267` 只收 `#version 440\|450\|460` + `core`）⇒ 引擎须内置约 30 行前置变换；`glsl` feature **必须与 `wgsl` 同开**（上游缺陷：`front/mod.rs:5` + `front/glsl/variables.rs:467`，30.0.1 未修）；naga 的 IR 把 uniform 变成独立 binding 且**名字被抹成 `_group_0_binding_N_fs`** ⇒ 引擎须自维护 uniform 名 → binding 索引映射表（`blackhole.ts:337` 的 `UNIFORM_NAMES` 可用）；② 兜底路线 = GPU 场景**完全留在现有 TS/JS 层**，引擎只经 `ui-core` 状态喂数据（保持「视觉层零改写」），**不做 GLSL→WGSL 搬运**；③ **P5-c 开工前硬条件**：在真实浏览器 WebView 里跑一次真画面（探测为编译期/解析期实测，**未在浏览器跑过画面**），未过不得开工 P5-c |
| **R2** | **响应式语义边角多**：依赖图环检测、重复订阅、effect 清理、批处理边界、读取时序（read-after-write）、条件分支下的依赖重收集，任一处与 Svelte 5 语义不一致 ⇒ 试点就出偏差 | **高** | **高** | ① 语义**逐条列出**并对每条写**判别测试**（不是「测 happy path」）；② `ui-core` 0 依赖 web-sys ⇒ 可在 native 上跑大规模随机序列对拍（随机状态序列 → 断言 DOM 树与期望一致）；③ 边角清单进 `crates-ui/apeireth-ui-core/` 的**显式 TODO 台账**，收口前逐条销账，**不留悬空**（宪章 §一.3「见债必还」） |
| **R3** | **过程宏编译期复杂度爆炸**：`#[component]` 属性解析 + 代码生成，编译期报错难读、增量编译变慢、错误信息定位成本高 | 中 | 中 | ① `ui-macro` 独立 crate、**零内部依赖**（最底层），可单独单测；② 过程宏只做**语法 → view 树构造**的翻译，**不做语义分析**（语义全在 `ui-core`）；③ 编译错误必须有**可读 span**（指向用户属性写法），编译期错误质量列为 P3 验收项 |
| **R4** | **试点通过但全量迁移不划算**：审批卡这种「输入输出明确」的组件通过了，但 62 个组件里有大量与 CSS/Canvas 深度耦合的巨型文件（`SettingsView.svelte` 5,314 行 / `App.svelte` 3,608 行），全量重写的收益可能低于成本 | **高** | **高** | ① **P4 硬闸门内含「全量迁移成本重估 ≤ 剩余预算」判据**（§3.5）；② P5-b 采取**逐面迁移**而非全量重写，**每面独立验收**，任何一面不划算就停在那面（保留 Svelte 版继续跑）；③ 明确承认**长期双栈**是可接受终局（ADR §5 已承诺「全程不修改现有前端」）——**不预设「必须全量 Rust 化」** |
| **R5** | **破「0 引新外部 dep」纪律**（`AGENTS.md` §4 提交正文必带项）：`wasm-bindgen` / `web-sys` / `wasm-bindgen-futures` / `js-sys` / `console_error_panic_hook` 等必然要引 | **高**（必然发生） | 中 | ① owner 已拍板豁免（ADR-0015 §6.2）。**不登记 `exceptions.toml`**——实测 `scripts/arch_guard.py` 的违规类别全集为 `direction/structural/size/caliber/duplicate/edge/env/exception`，**不含外部依赖类别**，该纪律是 commit 正文诚实声明而非架构契约约束，登记一个不存在的架构违规是伪造账；② **主 workspace 0 引**（独立 workspace 不进主依赖图）⇒ 纪律实际形态 = 「主仓 0 引新外部 dep」+「`crates-ui/` 内逐个过 `deny.toml` 许可白名单」；③ 每个外部依赖引入须在**同一 commit 正文**登记版本与许可 |
| **R6** | **依赖树撞 `multiple-versions = "deny"`**（`deny.toml:63` 为业界顶尖标准策略）。wasm 生态（`wasm-bindgen` / `js-sys` / `web-sys` / `bumpalo` / `once_cell` / `hashbrown` / `bitflags` …）在独立 workspace 内极易多版本 | **低**（⚠️ **2026-10-24 P0 探测后下调**：实测 wgpu-webgl 68 crate，**唯一多版本包只有 `syn`（2.0.119 + 3.0.9）**；`hashbrown`/`bitflags`/`itertools`/`miniz_oxide`/`ahash`/`crossbeam-*` **实测未进树**；wasm 侧 0 多版本——`jni-sys` 双版本仅出现于 native 目标产物） | 中 | ① 独立 workspace **不进主图** ⇒ 主仓 `deny.toml` 不受影响（主图实测 `naga`/`wgpu` **0 命中**）；② `crates-ui/` 自带 `deny.toml`，多版本 skip **必须带理由逐条登记**，**不接受空理由**（预计仅 `syn` 一条）；③ 收口时**退役不再需要的 skip**（沿用主仓 2026-10-18 D5 对账的纪律：无用的 skip 会掩盖未来的真问题） |
| **R7** | **电影感被引擎悄悄弄坏**：引擎在同一次任务里「挂载即卸载」或「旧态新态同帧翻转」⇒ CSS 过渡**根本不起播**，画面从电影感退化为软件感，且**功能测试全绿不报警** | 中 | **高** | ① 把「旧态 → 新态之间存在样式重算边界」写成**引擎的不变量**，并配机械测试（G2 / P2 时机测试）；② 逐帧对照（G2）+ owner 主观验收（G3）**双保险**，任一红即 no-go；③ 引擎**禁止**产生无真实来源的动效（设计系统 §0 视觉 0 装，`docs/design/01-DESIGN-SYSTEM.md:23`） |
| **R8** | **测试基座迁移期双轨**：38 条源码镜像测试在 Rust 版组件落地后仍留在 `tests/*.mjs`，新旧两套断言同时存在 ⇒ 「同一行为两处断言」必然漂移 | 中 | 中 | ① 迁移**逐面**进行，每迁移一面**同批退役对应的镜像测试**（P5-b 交付物含「退役一条」）；② 退役判据 = 该测试的断言已被 Rust 侧**行为测试**覆盖（不是「删了事」）；③ 镜像测试总数作为**退坡指标**记入 P5 每批交付正文（当前基线 38，实测 2026-10-24） |
| **R9** | **独立 workspace 失主门禁保护**：不进主 `arch_guard` 扫描面 ⇒ 引擎代码可无限膨胀、依赖可无节制增长 | 中 | 中 | ① `crates-ui/` **自带独立门禁 CI**（check / clippy `-D warnings` / fmt `--check` / test 四 job）+ `crates-ui/deny.toml`；② 每次 P5 批的交付正文必报**引擎代码行数**与**依赖边数**增量（防止无节制生长）；③ 主仓门禁**不代管**独立 workspace ⇒ 独立门禁红 = 该批不收 |

---

## §6 批次边界与 write_scope 纪律

### 6.1 规则出处

> **本节规则全部出自** `docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md` **§二「叠加不覆盖（多人并发纪律）」**（`:31-36`）：
>
> 1. **edit 前先读后改**；文件冲突时后到者 rebase，不覆盖他人在途工作；
> 2. **战场不重叠**：批与批的文件范围预先声明（write_scope），同文件区施工必须排队；
> 3. **半成品不弃尸**：中道停机必须回报「已完成/未完清单」（接棒制的遗嘱）；
> 4. **他人文件不乱动**：跨范围的顺手修 = 报备 + 最小改动 + 说明。

配套的**文档头部格式**出自 `docs/04-internal/ENGINEER-MANIFESTO.md` §7.1（`:230-243`），
**交付卷宗骨架**出自 `docs/04-internal/engineering-experience/templates/delivery-doc-skeleton.md:19`（write_scope 栏定义）。

### 6.2 write_scope 声明格式（每批开工前必须声明，交付正文必须复述）

```text
**战场声明（write_scope）**：
  主战场 = <本批新建/修改的目录或文件，逐条列出>；
  随行   = <注册/接线/配置类必须同批改的最小面，逐条列出>；
  禁区零触碰 = <本批明确不碰的范围，含他线在途面>。
```

**三段缺一不可**：只写「主战场」而不写「随行」⇒ 接线面会在下一批变成他线的意外冲突；
只写「主战场」而不写「禁区零触碰」⇒ 无法自证清白（交付骨架要求「他线在途脏文件 = 未碰，列出以示清白」）。

### 6.3 本计划线的批次边界（逐阶段声明已在 §3 各阶段末）

| 批 | 主战场 | 禁区零触碰 |
|---|---|---|
| P0 | `docs/04-internal/ui-engine-plan.md`（本批） | `frontend/companion-desktop/**`、`docs/adr/**`、`docs/design/**`、`crates/**` |
| P1 | `crates-ui/apeireth-ui-core/**` | 主 `crates/**`、`frontend/companion-desktop/**`、主 `Cargo.lock` |
| P2 | `crates-ui/apeireth-ui-dom/**` | 同上 + `frontend/companion-desktop/src/lib/design/**`、`src/lib/scene/**`（视觉层） |
| P3 | `crates-ui/apeireth-ui-macro/**`、`ui-kit/**`、`ui-demo/**` | 同上 + `docs/design/**` |
| P4 | `docs/04-internal/ui-engine-plan.md`（回写） | `frontend/companion-desktop/**`、`crates/**`、LOCKED 5 项 |

### 6.4 不重叠原则（执行规则）

1. **同文件区必须排队**：两个批的 `主战场` 有交集 ⇒ 后到者 rebase，不覆盖在途工作（宪章 §二.1）。
2. **阶段之间必须串行**：P1 → P2 → P3 → P4 有硬依赖（`ui-dom` 依赖 `ui-core`；试点依赖前两层），**不并行**。
3. **与主仓施工线的关系**：本计划线**全程不碰主 workspace**（`crates/**`、根 `Cargo.toml`、`Cargo.lock`）⇒ 与主仓所有并行线**零文件冲突**。
4. **与前端施工线的关系**：本计划线**全程不碰 `frontend/companion-desktop/`** ⇒ 与前端所有并行线**零文件冲突**。
5. **与其他文档子线的关系**：`docs/adr/**`（ADR 子线）、`docs/design/**`（设计子线）由**各子线独占**；
   本计划线只写 `docs/04-internal/ui-engine-plan.md` 一个文件。跨线需要改对方文件 ⇒ **报备 + 最小改动 + 说明**（宪章 §二.4）。
6. **半成品不弃尸**：任何阶段中道停机，必须在本文对应阶段小节追加「已完成 / 未完清单」（落点文件 / 注册条件 / 现成零件 / 验收形状），
   **不留悬空节**（宪章 §二.3）。

---

## §7 诚实边界

### 7.1 本文的「实测」部分（可复核）

- §1.1 全部代码规模 / 文件数 / 行数 / 测试基座计数 / Tauri 宿主计数 —— **2026-10-24 亲跑**，命令写在表内。
- §1.2 特性使用面全部处数与文件数 —— **2026-10-24 亲跑**，口径写在表前（可据此独立重写脚本复跑；本批用的一次性脚本在 `/tmp`，未入库）。
- §1.3 动画责任归属（0 处框架过渡指令 / 86 处 CSS `transition:` / 80 处 `backdrop-filter` / 2 条 GLSL / WebGL2 3 处 / `MOTION` 令牌 / `motion.css` 5 个 `ap-*` 关键帧）—— **亲跑**。
- §1.5 口径更正记录（`on:` 814 / `in:` 277 的宿主拆解）—— **亲跑**。
- §2.1 `wasm32-unknown-unknown` 目标已安装 —— **亲跑**（`rustup target list --installed`）。
- §2.3 视觉层载体清单的行号 —— **亲跑**（`grep -n` 逐条定位）。

### 7.2 本文的「待测」部分（写了但没有实测证据，不得当结论用）

| 项 | 状态 |
|---|---|
| 引擎代码量 **7,000–14,000 行** | **主观估时**，非实测（ADR-0015 §6.2 同样如此标注） |
| 各阶段人日（3–5 / 5–9 / 6–11 / 8–14 / 2–3） | **主观估时**，非实测 |
| P5+ 各子阶段工作量 | **未实测**（本计划未覆盖） |
| naga 对本仓 2 条 GLSL 的覆盖度 | **未实测**（P0 探测子线结论未回写） |
| `crates-ui/` 依赖树实际包数与多版本情形 | **未实测**（P0 探测结论未回写） |
| `wasm-bindgen` / `web-sys` 等外部依赖的最终版本与许可 | ✅ **已实测**（P0 探测子线，2026-10-24）：`wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷ web-sys 0.3.106`（**精确硬钉 1:1**，错配在依赖解析期失败）；wgpu 30.0.1 的 `webgl`/`webgpu` 两后端 wasm32 **均编译通过**；wgpu-webgl 68 crate，唯一多版本包 `syn`（2.0.119 + 3.0.9）。详见 ADR-0015 §2.6 |
| 像素对照 ≤1% 是否可达 | **未实测**（工具尚未落地，见 G1） |
| 逐帧对照的帧序列采集与比对方法 | **未实测**（方法待 P2/P3 立项时定） |
| 现有 38 条源码镜像测试的**逐条覆盖关系**（哪条对应哪个组件） | **未实测**（本批只测了总数与分类，未逐条建映射） |
| 破 0-dep 纪律的**例外登记状态** | ✅ **已定案：不登记 `exceptions.toml`**。实测 `scripts/arch_guard.py` 违规类别全集为 `direction/structural/size/caliber/duplicate/edge/env/exception`，**不含外部依赖类别** ⇒ 该纪律是 commit 正文诚实声明而非架构契约约束，登记不存在的架构违规是伪造账。落点改为 ADR-0015 §6.2 如实登记 + 本文 R5 专项 + 每批 commit 正文声明例外明细 |
| 主仓 `arch_guard --strict` 现行基线（30 crates / 111 边） | ✅ **已实测**（P0 收口时亲跑，`crates-ui/` 不进主图 ⇒ 基线 0 变化）：30 crates / 111 边 / 160 env / VIOLATIONS 0 |
| naga 能否覆盖本仓 2 条 GLSL 程序 | ✅ **已实测**（P0 探测子线）：**能，1,037 行不用重写**；需 2 步机械变换（删 `#version 300 es` 行 + 每个 uniform 注入 `layout(binding=N)`，约 30 行前置编译器）。⇒ **R1 概率由「中」下调为「低」**（详见 ADR-0015 §2.6 与 §8.2） |
| 全仓测试 / clippy 数字 | **不在本文范围**——以 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md) 为准，本文不抄写 |

### 7.3 本文的「提案」部分（尚无人拍板，属待决）

1. **阶段切分 P0–P5+ 与各阶段边界** —— 提案，owner 未逐段拍板（ADR-0015 只对 P0–P5 骨架拍板）。
2. **P3 试点组件 = 审批卡** —— 沿用 ADR-0015 §4 的选型，**owner 未就试点组件本身单独确认**。
3. **G1「像素对照 ≤1%」这个阈值** —— **未与 owner 确认**；1% 是本计划给出的判据提案，owner 可改。
4. **G3「owner 亲审」的通过标准** —— 提案（并排观看 + 显式通过/不通过 + 落档），**具体验收场景清单未定**。
5. **P5-b 逐面迁移的组件序**（审批卡 → 工具卡 → 状态条 → 治理卷宗 → 设置页三页）—— 提案，未排序定稿。
6. **长期双栈为可接受终局** —— ADR-0015 §5 已隐含承诺（「全程不修改现有前端」），但**「长期双栈」这一终局本身未获 owner 显式确认**。
7. **第二阶段走原生窗口路径** —— **不在本计划承诺范围**，P4 复盘时另出 ADR。

### 7.4 本批自身的 0 装真账

- 本批**只写 1 个文件**：`docs/04-internal/ui-engine-plan.md`。
- 本批**0 触碰**：`frontend/companion-desktop/**`、`crates/**`、`docs/adr/**`、`docs/design/**`、根 `Cargo.toml` / `Cargo.lock`、LOCKED 5 项。
- 本批**未 commit**（按上级指示；提交由主代理审后决定）。
- 本批**未跑**主仓全量测试 / clippy / fmt（纯文档批且 0 生产改动；交付前建议由主代理在 wave 收口跑一次
  `pwsh scripts/gate_full.ps1 -FailIfBusy` + `python scripts/docs_truth.py --verify --check`）。
- 本批**未做**：`cargo deny check`、`arch_guard.py --strict`、任何 wasm 构建（`crates-ui/` 尚不存在）—— 均为 P0 待办。

---

## §8 参考与出处

| 事实 | 出处 |
|---|---|
| 引擎边界（视觉层零改写） | 实测 `frontend/companion-desktop/src/lib/scene/blackhole.ts:129,138,756`、`src/lib/intro/fire.ts:853`、`src/features/pet/avatar/CharacterAvatar.svelte:75`、`src/lib/scene/SceneLayer.svelte:124`、`src/lib/design/shell.css:62`、`src/lib/design/tokens.css:118-120` |
| 动效时长与缓动令牌 | `frontend/companion-desktop/src/lib/scene/tokens.ts:148-169`（`MOTION`） |
| 专属 CSS 动效层 | `frontend/companion-desktop/src/lib/design/motion.css:1-257`（`ap-shell-in` / `ap-rail-glow` / `ap-card-rise:63` / `ap-fade-up:82` / `ap-msg-in:197`） |
| 动画责任在 CSS（0 处框架过渡指令） | 实测 `grep -rnoE '\bin:[A-Za-z]' --include='*.svelte' src/` → 0；`grep -rnoE '\btransition\s*:'` → 86（CSS 属性，例 `src/App.svelte:3295`） |
| 电影感定义与载体 | `docs/design/01-DESIGN-SYSTEM.md:27-38`（§1 总纲，`:33` 电影感优先于信息密度）、`:150-156`（§3.3 过渡纪律）、`:344-359`（§7.1 动效时间尺度）、`:361-372`（§7.2 渲染梯队）、`:23`（视觉 0 装） |
| 0 装 PASS 纪律 | `AGENTS.md:8-13` |
| 测试数字单一真源 | `AGENTS.md:22`；`docs/03-reference/test-baseline.md:1-9` |
| write_scope 纪律出处 | `docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md:31-36`（§二）；`docs/04-internal/ENGINEER-MANIFESTO.md:230-243`（§7.1 文档规范） |
| 交付卷宗骨架 | `docs/04-internal/engineering-experience/templates/delivery-doc-skeleton.md:19` |
| 多版本 deny 策略 | `deny.toml:63`（`multiple-versions = "deny"`） |
| 上位决策 | `docs/adr/0015-pure-rust-ui-engine.md`（Accepted，2026-10-24） |
| 契约 / 例外登记位置 | `docs/architecture/contract.toml`、`docs/architecture/exceptions.toml` |
| 独立 workspace 先例 | `frontend/companion-desktop/src-tauri/Cargo.toml:1-4`（独立 `[workspace]` 段） |

---

_本计划书由前端重构项目组（ui-planner）撰写于 2026-10-24。所有 §1 数字为当批亲跑实测，口径逐条写明；
估时与未实测项已按 0 装纪律单独归栏（§7.2 / §7.3）。本批未 commit。_
