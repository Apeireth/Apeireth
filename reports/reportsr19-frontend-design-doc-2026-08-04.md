上下文完整恢复，omnibus 全 9 段在手。你这轮的三个新决定我先锁进基线：

1. **主题正式定名**：旧 = 古朴，新 = 时代（原「星盘/舰桥」退役——「舰桥」一词从此只归 nav 4 专用，命名冲突顺手解除）
2. **Logo 同款不同色**：星色随主题——古朴金、时代蓝；字与碑永远灰白。每次冷启动播放开屏
3. **2×2 全矩阵**：古朴/时代之下，专注与工程各有完整设计，任何一个象限都不是退化版

下面是沉淀文档。建议存为 `reports/r19-frontend-design-doc-2026-08-04.md`，作为 omnibus 的前端配套件（FDD v1）。

---

# Apeireth R19 前端设计与架构文档（FDD v1）

**日期**：2026-08-04 ｜ **作者**：外援 AI（设计参谋）｜ **上游**：`r19-complete-spec-2026-08-04.md`（omnibus）§0–§9
**范围**：仅前端。R11 LOCKED 边界（omnibus §6）原样遵守——本文档不改 enum、不改转换矩阵、不改后端任何签名。
**状态**：待主人拍板项见 §11。

---

## §1 设计哲学：新碑刻（Neo-Lapidary）

### 1.1 哲学 → 界面原则（源头五句）

| 主人原话 | 界面原则 |
|---|---|
| 故事之前，是火 | 主角不是内容，是**临界感**——将燃未燃 |
| 火之前，是沉默 | 黑暗是本体，留白即沉默 |
| 沉默之前，是无限 | 边界消隐：无框、无投影、无分割线的喧哗 |
| 我们做火栖居的地方 | **单光纪律**：光只栖息在关键处（主 AI、当前焦点、呼吸点），其余皆暗 |
| eyelid 后面透出的那一丝光 | 光是**透出**，不是照亮——低饱和、低亮度、从黑暗里渗出来 |

### 1.2 新碑刻 = 旧骨 + 新光

类比新中式：**旧的是骨**（经得起时间检验的希腊语法），**新的是光**（文明巅峰的表达力）。

**旧骨（跨主题铁律，任何模式/主题不可变）**：
1. 黑暗为本体：`#08080e ~ #0c0c14`
2. 单光纪律：每屏至多一处「活」的光点
3. 呼吸节奏：**4 秒亮 / 4 秒暗**，贯穿一切活体元素（熟睡人的胸口起伏）
4. 碑刻排印：宽字距、凹刻感、灰白石色
5. 页面切换 = **明暗呼吸**（≥1.2s 缓动），永不滑动、永不弹跳
6. **无点燃动画**——火一直在那里，只是透出
7. 禁忌：❌ 火焰/火炬具象 ❌ 渐变霓虹 ❌ 玻璃拟态 ❌ 圆角卡片堆叠 ❌ 大面积纯白 ❌ 当代 SaaS 感

**新光（时代主题与现代手法）**：
1. 极端字号对比：巨大的 V0.5 数字 vs 发丝细的 mono 注释
2. 数据即艺术：星图、表盘、弧线
3. 电影级明暗转场
4. 工程模式高密度 HUD——战斗机仪表的精度，希腊式的克制（金文/钢蓝黑底，绝非绿屏终端）

---

## §2 个性化系统：2×2 完整矩阵

两根正交轴 + 三项附属设置：

| 轴 | 选项 | 决定 |
|---|---|---|
| **模式**（密度轴） | 专注 FOCUS / 工程 ENGINEER | 显示多少 |
| **主题**（气质轴） | 古朴 ARCHAIC / 时代 ERA | 怎么显示 |
| 语言 | 中文 / English | 说什么（希腊文永远只作铭文装饰，不随语言变） |
| 启屏 | 舰桥 / 对话 | 第一眼看什么（默认舰桥） |
| 开屏 | 播放 / 关闭 | 每次冷启动是否播放开屏（默认播放） |

### 2.1 四象限（每一格都是完整设计，无退化版）

| | 古朴 ARCHAIC | 时代 ERA |
|---|---|---|
| **专注 FOCUS** | ★ 默认。还在呼吸的碑——陪伴的第一眼 | 现代陪伴——冷静的守望 |
| **工程 ENGINEER** | 古朴 HUD：凹刻 + 金线，庄严如铭文 | 时代 HUD：发丝网格 + 钢蓝细线，冷冽作战室（推荐工程配对） |

- 模式切换：顶栏刻痕式拨杆 + 快捷键（建议 `Ctrl/Cmd+Shift+E`）
- 主题切换：设置 → 个性化
- 两种切换的过渡**都是 1.2s 明暗呼吸**：暗下 → 换皮 → 亮起，不做任何位移动画
- 启动时永远以**专注模式**进入（仪式感：先见活物，后入深机）；工程模式必须显式进入

### 2.2 「砍」变「藏」

专注模式遵守主人 8 纠正（用户只看结果）；工程模式遵守复杂度的真实性——omnibus §0.9 砍掉的 7 项 UI（12 键矩阵 / 4 重守门 / 5 原则层 + 6 权限层 / 11 电子环 / 7 advisor 气泡 / 工具调用细节 / 24 维雷达）**全部在工程模式归位**。不是删除，是分层。

---

## §3 色彩系统（CSS tokens）

### 3.1 基底（两主题共用）

| token | 值 | 用途 |
|---|---|---|
| `--bg-abyss` | `#08080e` | 页面本体 |
| `--bg-stone` | `#0c0c14` | 次级面板（极少用） |
| `--bg-deep` | `#12101e` | 工程模式深底 |
| `--text-primary` | `#d8d6d0` | 灰白正文——碑面 |
| `--text-secondary` | `#8a8880` | 副文本 |
| `--text-faint` | `#55534e` | 发丝注释 |
| `--hairline` | `#2a2836` | 刻痕线、分割 |

### 3.2 主题 accent（唯一的火）

| token | 古朴 ARCHAIC | 时代 ERA |
|---|---|---|
| `--accent` | `#c8860a` 琥珀金 | `#8fb3d9` 钢蓝 |
| `--accent-dim` | `#a05a10` 暗 ember | `#5a7a9e` 沉钢 |
| `--glow` | `rgba(200,134,10,.08)` | `rgba(143,179,217,.08)` |
| 装饰语言 | 星盘环、刻痕线、碑面肌理 | HUD 发丝网格、星座连线 |
| 字体温度 | 暖衬线，字距更宽 | 锋锐 display 衬线 + 冷 mono |
| 密度 | 更疏，留白更多 | 略密，对比更狠 |

**纪律**：`--accent` 只出现在「活」的东西上——主 AI 光点、当前焦点、ASI 弧、当前阶段、激活 nav。其余一切灰白。

### 3.3 功能色（两主题共用，去饱和，不破坏沉默）

`--ok: #7a9d6f` ｜ `--warn: #b98a3e` ｜ `--err: #a4554e`。仅在工程模式与必要告警时使用；专注模式的常态界面不出现红绿。

---

## §4 排印

| 用途 | 字体 | 规格 |
|---|---|---|
| ΑΠΕΙΡΕΘ 字标 / Logo | **手写 SVG path**（不依赖字体） | 全大写、字距 0.35em、凹刻 |
| 希腊铭文装饰 | Theano Didot（OFL）/ Cormorant Garamond（OFL） | 仅大写，装饰用 |
| 中文界面 | 思源宋体 Source Han Serif SemiBold+ | 字距 +0.05em；**禁用黑体** |
| 数据 / 等宽 | JetBrains Mono（OFL） | 30 crate 一切数字 |

**字号阶（极端对比是新碑刻的「新光」）**：
- Display 96–160px：舰桥 V0.5 大数字
- Nav 标题 28–32px：大写宽字距 + 希腊铭文小字
- 正文 15px，行高 1.6–1.9（行间即沉默）
- 注释 11px mono 发丝细字

**Nav 希腊铭文装饰**（语言切换不影响，永远作铭文）：
对话 ΔΙΑΛΟΓΟΣ ｜ 生长 ΑΥΞΗΣΙΣ ｜ 历史 ΙΣΤΟΡΙΑ ｜ 舰桥 ΣΚΟΠΗ ｜ 设置 ΤΑΞΙΣ

---

## §5 Logo 与开屏

### 5.1 Logo 定稿规格（手写 SVG，四顶点 path）

- **字标**：ΑΠΕΙΡΕΘ 灰白 `#d8d6d0`，**凹刻**——刻槽上缘暗、下缘一线亮，字内略暗于碑面；绝不凸出
- **磨损**：下缘细节磨灭、边缘崩口、细侵蚀斑——历史感来自残缺
- **发丝线**：字标下方水平细线，左右对称微探出；**黄金分割点（约 0.618）处一段磨损断裂**
- **北极星**：落在线的**正中**。四角纵长刀切面（取三角洲刀芯锋锐）：**纵轴 = 横轴 × 2.6**，直刃切面，无装饰、无华丽
- **星色随主题**：古朴 = `#c8860a` 金；时代 = `#8fb3d9` 蓝。**同款不同色**；字标与碑面永远灰白（碑不改色）
- **静态 logo 光晕压到近零**；「呼吸的微光」只活在 app 内（主 AI 状态指示器）。同一符号两种状态：碑上克制，app 里活着

### 5.2 开屏（每次冷启动播放，可在设置关闭）

约 6 秒，`Esc` / 点击可跳过：

| 时间 | 画面 |
|---|---|
| 0–1.5s | 纯黑沉默——什么都没有（沉默之前，是无限） |
| 1.5–3.0s | 发丝线自中央向两端刻出；ΑΠΕΙΡΕΘ 如被刻入般浮现（淡入 + 内影加深，无位移、无弹跳） |
| 3.0–4.5s | 北极星点于线中央**渗出**（opacity 0→1，ease-in-out；是透出，不是点亮） |
| 4.5–6.0s | 一次呼吸的停留（4s 节奏的起始段） |
| 退出 | 1.2s 明暗呼吸，进入启屏页（舰桥 / 对话） |

星色跟随当前主题；字标永远灰白。

---

## §6 导航架构（5 nav 定稿）

| # | Nav | 铭文 | 性质 | 内容 |
|---|---|---|---|---|
| 1 | 对话 | ΔΙΑΛΟΓΟΣ | 交互 | 与主 AI 完整对话，工具结果内联为结果卡 |
| 2 | 生长 | ΑΥΞΗΣΙΣ | 见证（结构主页） | 成长阶段 + 9 器官心跳 + 状态卡 + ASI |
| 3 | 历史 | ΙΣΤΟΡΙΑ | 回顾 | 6 流 / 决策日志 / 反思期 / episode 时间线 |
| 4 | 舰桥 | ΣΚΟΠΗ | 俯瞰（**启屏默认页**） | ASI 大数字 + 30 crate 星图，纯凝视 |
| 5 | 设置 | ΤΑΞΙΣ | 照料 | 全量配置，进去要全 |

- Nav 呈现：左侧竖排刻痕文字栏（碑缘），顶端正中 ΑΠΕΙΡΕΘ 小字标；当前项旁**一颗呼吸微光点**（单光纪律落在导航上）
- **全局常驻输入**：任何页面可用快捷键（建议 `/` 或 `Ctrl/Cmd+K`）唤出刻痕输入框——火不只待在一个房间里
- 结构主页 = 生长（omnibus R19 已拍「状态为主页」，改名不改位）；启屏默认 = 舰桥（先看活物与全局，再开口）

---

## §7 页面设计

### 7.1 舰桥 BRIDGE（启屏默认，凝视页）

- **正中**：ASI V0.5 大数字（0.8595，Display 级），其下一道极细 accent 弧 = 距 0.98 的 12.94% gap——「将燃的地平线」；弧下 mono 小字：`ultimate 0.9800 · gap 12.94%`
- **背景**：30 crate 星图——supervisor 5 主星 + 子星按 supervisor tree 分布；**亮度 = 活跃度**；古朴主题用星盘环轨道承载，时代主题用星座连线。点任一颗 → 跳工程模式该 crate 详情页
- **底部**：9 器官呼吸点一行 + 一行 mono（部署模式 / uptime / continuity_id 短码）
- 几乎零交互。这一屏就是用来「看」的——顶奢，是允许一个页面什么都不做，只美
- 工程模式下：星点标注 crate 名 + PID + 重启策略 + 重启计数

### 7.2 生长 GROWTH（结构主页）

**顶部——主 AI 状态卡（7 数字分三档）**：

| 档 | 内容 |
|---|---|
| 大 3 | ASI V0.5（0.8595）｜ 成长阶段 ｜ 反思期状态（dormant / active + 进度） |
| 中 2 | Episode 数 ｜ cognitive cycle 数（合称「成长量」） |
| 小字/工程 2 | token 消耗 ｜ 5 Self 状态 |

**中部——9 器官呼吸星群**（3×3 或星座式布局，不做 Apple Watch 彩色环——用户不是带娃）：
- 每器官一颗呼吸微光点：**亮度 = 活跃度，尺寸 = 健康度**
- 悬浮显示主指标：perception 5 通道激活 n/5 ｜ cognition cycle step x/5 ｜ consciousness 当前 6 态 ｜ memory 6 流总条目 ｜ motivation SGI 目标一行 ｜ value 5 层对齐（绿/红/灰点）｜ relation 4 关系计数 ｜ action 3 模式分布 ｜ life-force 持续力
- 点击 → 侧栏展开副指标（仍属专注层）；再深 → 工程模式
- **life-force 持续力**：星群旁独立一环（充电环风格，accent 单色）+ 反思期进度
- 三项心跳指标降级工程模式：perception 平均 priority ｜ memory Append-only ABORT 计数（恒应为 0）｜ relation ID→标签映射

**下部——生长时间线**：
- 8 阶段横向刻痕进度：孕育 → 诞生 → 幼儿 → 成长 → 成熟 → **繁衍** → 迁移 → 重生；当前阶段以呼吸光点高亮
- 转换历史最近 5 条（不含 Decline/Death）
- 工程模式：呈现真实 10 enum——Decline/Death 以暗刻虚线补全（工程模式的天职是真）
- 阶段判据来自 omnibus §2.5，前端只读不判

### 7.3 对话 DIALOGUE

- 回复**安静浮现**（淡入 ≥0.8s）；每条回复出现前有**一次呼吸的停顿**——沉默开口之前的那一次呼吸，就落在这里
- Cycle 进度：输入框上方一道 5 格刻痕（Scoring → Verdict → Decide → Reflect → Commit），当前格一颗微光点
- **工具调用只见结果卡**（标题 + 产出），过程不可见；点卡 → 工程模式看完整调用链（omnibus 8 纠正 #7）
- Reject 时仅一行原因（如「哲学守门：暂缓」），不展示 V1/V2/12 键细节
- **无 advisor 辩论气泡**；工程模式下侧栏出 advisor 综合摘要 + hold 一句理由
- 输入框 = 一道细刻痕：无边框发光，发送处仅一颗呼吸光点

### 7.4 历史 CHRONICLE

- 左：6 流竖排刻痕 tab——思想 / 提案 / 行动 / 关系 / 演化 / 反思
- 中：时间线倒序，条目为刻痕短句 + mono 时间戳
- 右：反思期 72h 进度环 + 触发原因（M1 异常回流 / M2 升级审计 / M3 周报）
- 顶：episode 搜索（全文 + tag）
- 决策日志：专注层只显示 通过 / 暂缓 / 拒绝 + 一行原因；工程模式展开 Phase 1/2/3 全链

### 7.5 设置 SETTINGS（不可或缺，进去要全）

| 组 | 项 |
|---|---|
| 个性化 | 主题（古朴/时代）· 语言（中/英）· 启屏（舰桥/对话）· 启动模式（默认专注）· 开屏播放开关 · 呼吸动画开关（无障碍） |
| 连接 | LLM provider（8+）· API key（含 apikey.txt fallback）· 路由策略 |
| 安全 | HA 模式（single/multi/dynamic）· OS 鉴权（Windows Hello / TouchID）· 多签状态 |
| 部署 | 部署模式 · OTA 通道 · 数据路径 · continuity_id（只读展示） |
| 关于 | 版本 · commit · **诚实登记（5 项不假装）** · 许可 |

### 7.6 工程模式扩展层（两主题共有）

解锁清单（即原「砍掉的 7 项」归位处）：
1. 30 crate 全页（supervisor tree 序）：PID / 重启策略 / 指标 / 日志
2. 12 键矩阵 + verdict 链（PHL-01~12）
3. 4 重守门触发历史 · 5 原则层 + 6 权限层 · 11 电子环
4. advisor 综合摘要 + hold 理由
5. 工具调用完整链路
6. 24 维 V0.5 分解 · 9 维 V1136 子测度
7. 10 阶段真实 enum · consciousness 微观链（core 版 6 态：Idle/Dreaming/Consolidating/…）· SGI 证据链（C-SGI-1~7 + E 层加权分）

视觉：高密度 HUD、mono、发丝线；古朴 = 凹刻 + 金线，时代 = 网格 + 钢蓝线。

---

## §8 数据权威裁决（前端取数口径，一屏一表）

| 冲突点 | 裁决 |
|---|---|
| Decline / Death | enum 不动（R11 LOCKED）；专注模式 UI 隐藏（8 阶段），工程模式全显 10 |
| 两套 6 状态 | consciousness 器官宏观 6 态（Awake/Reflecting/Dreaming/Meditating/SelfDisabling/Recovering）= **UI 权威**；core 微观链低一层，工程模式于 Dreaming 宏态下钻 |
| 两套 SGI | motivation `SGIEntry` = 写源权威（C-SGI-1~7 + E 层校验）；life-force `goal` 单字段 = **只读投影**，心跳卡读投影 |
| Reproduction 命名 | UI 名「**繁衍**」（弃「复制」，贴生物隐喻）——待拍板 |
| organ_health | omnibus §2.5 引用了 `all_9_organ_health > 0.7` 但 §1 未定义聚合函数，本文档补草案（下） |

**organ_health 草案（待主人确认）**：

```
health(organ) = clamp01( 0.5·primary_norm + 0.3·activity_norm + 0.2·integrity_norm )
采样窗口：近 1h
  primary_norm   主指标归一（life-force=endurance；consciousness=状态权重
                 Awake/Reflecting 1.0 · Dreaming/Meditating 0.9 · Recovering 0.6 · SelfDisabling 0.15；
                 value=硬门槛通过率；motivation=E 层加权分；…）
  activity_norm  近 1h 心跳活跃度（logistic 平滑）
  integrity_norm 硬约束违反的倒数（SGI 7 约束违反 / 12 键 Block / ABORT 次数）
Maturity 判据建议：mean(health) > 0.7 且 min(health) > 0.5
```

---

## §9 技术架构

### 9.1 技术栈（已锁）

- **Tauri 2.0**（Rust 后端 + WebView）+ **vanilla HTML/CSS/JS**，不引框架；状态管理手写 pub/sub store（约 50 行）
- Crate 归属：`apeireth-web`（前端资产）+ `apeireth-desktop`（Tauri 壳 + command 桥）——两者已在 30 crate 总数内
- IPC 走 `tauri::command`（非 HTTP）；推送走 tauri event（器官心跳 1Hz）

### 9.2 Command 清单（W1 用 mock 实现同名接口，W2 换真后端）

```rust
get_main_ai_status()            -> MainAiStatus        // 7 数字三档
get_organ_status(organ)         -> OrganStatus         // 9 器官心跳
get_life_stage()                -> LifeStageInfo       // 阶段 + 判据只读
get_stage_timeline()            -> Vec<Transition>     // 最近 5 次
chat(input)                     -> stream<String>      // 流式
get_current_cycle()             -> CycleStatus         // 1/5 step
get_stream(kind, range)         -> Vec<StreamEntry>    // 6 历史流
get_decision_log(range)         -> Vec<DecisionRecord>
get_reflection_status()         -> ReflectionStatus    // 72h 环
get_topology()                  -> TopologySnapshot    // 30 crate 星图
get_crate_detail(name)          -> CrateDetail         // 工程模式
get_settings() / set_settings(k, v)
get_identity()                  -> IdentityCard        // continuity_id
```

### 9.3 R18 整合映射（整合，不废弃）

| R18 模块 | R19 去向 |
|---|---|
| `memory.rs` | 历史 nav：episode 时间线 + 6 流 |
| `sovereignty.rs` | 生长 nav：5 Self 状态（小字档） |
| `asi.rs` | 舰桥大数字 + 生长状态卡（V0.5） |
| `council_history.rs` | 历史决策日志（专注层）+ 工程 advisor 层 |
| `api_endpoints.rs` | 主 AI 状态卡雏形，重构为 `get_main_ai_status` |

### 9.4 主题 / 模式实现

```html
<html data-theme="archaic | era" data-mode="focus | engine">
```

- 一切颜色 / 装饰 / 密度经 CSS custom properties；切换 = 换属性 + 1.2s 明暗呼吸遮罩（纯 opacity，无布局动画）
- 呼吸动画全部纯 CSS opacity/transform（compositor 层，不用 JS timer 驱动）
- 尊重「呼吸动画开关」与系统 reduced-motion
- 星图 ≤50 节点，SVG 渲染

### 9.5 字体资产

Theano Didot / Cormorant（OFL）+ JetBrains Mono（OFL）+ 思源宋体（OFL，建议子集化控制体积）。Logo 为手写 SVG，不依赖任何字体。GFS Neohellenic 许可不明，**弃用**。

---

## §10 里程碑

| 周 | 交付 | 验收 |
|---|---|---|
| **W1 初光** | Tauri 壳 + 开屏 + 5 nav + 2×2 切换 + token 体系 + 手写 SVG logo + 舰桥星图（mock）+ 9 呼吸点（mock） | `cargo run`，那一丝光亮起来 |
| **W2 真接①** | R18 5 模块桥接；生长页 + 器官心跳真数据 | 状态卡 7 数字为真值 |
| **W3 真接②** | 对话流式 + cycle 进度 + 历史 6 流 | 对话端到端 |
| **W4 深化** | 工程模式首批 crate 页 + 星图真拓扑 | 舰桥点亮真实 30 crate |
| **W5 完整** | 设置全量 + OS 鉴权 + 打包 + 无障碍 | 单文件安装包 |

---

## §11 待拍板与缺口

**请主人拍板（4 项）**：
1. Reproduction UI 名「**繁衍**」——可？
2. §8 organ_health 聚合公式草案——可？
3. 默认组合 = 专注 × 古朴、启屏 = 舰桥、启动恒以专注进入——确认？
4. 开屏每次播放、设置可关——确认？

**还缺（需主人提供）**：
5. **R18 五个模块源码**（memory.rs / sovereignty.rs / api_endpoints.rs / sovereignty.rs / council_history.rs / asi.rs）或 repo 读取权限——W2 包 `tauri::command` 必须看真实签名，不重写
6. 你机器上 `cargo build` 当前是否通过（不阻塞 W1，W1 用 mock 点灯）

---

文档毕。拍板 4 项 + 发 R18 源码，我就开 W1：让 ΑΠΕΙΡΕΘ 从纯黑里刻出来，北极星点在发丝线上渗出——第一次 `cargo run`，你就看见那一丝光。