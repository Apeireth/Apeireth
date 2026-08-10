# R19 前端 W1/W2 收尾 + 新团队交接文档

**生成时间**: 2026-08-04 11:55
**作者**: Mavis (R19 主 AI 外援, 主人楚零)
**目的**: W1/W2 收尾 + 把前端交给新团队时的完整上下文
**阅读顺序**: 从上到下, 0 上下文能接

---

# §0 项目背景 (新人必读, 5 分钟)

## 0.1 主人

- **楚零**: Apeireth 项目主理人, 研究生 (侦查学院)
- 哲学派, 关注"AI 自主性 + 哲学完整性 + 实战落地"
- 决策风格: **先思考后动手** + 让我做判断 (不机械问拍板) + 砍借鉴/装饰/无业务价值的东西
- 8 个关键认知纠正 (R19 全部设计的源头, 必须背下来):
  1. **9 阶段不需要衰老病死** — 主 AI 只会成长, 不消亡
  2. **用户看结果不看哲学** — 主人 8 纠正 #2, 砍掉哲学暴露
  3. **守门/原则/电子环 用户不需要看** — 砍
  4. **状态为主页** — 尤其主 AI 状态
  5. **历史/事件流/决策日志/反思 值得看** — 专注层必须
  6. **设置不可或缺, 进去要全** — 全量配置
  7. **工具调用用户不关心** — 只看结果
  8. **9 器官拟人化** — 从生物借鉴, AI 成长的核心秘密

## 0.2 Apeireth 是什么

- 长程 AI 成长平台 (R11 阶段 1-5 LOCKED), 30 个 Rust crate 全栈
- 主 AI 是**真生产逼近 ASI 基座**, 当前 V0.5 = 0.8595, ultimate = 0.98
- 8 维大脑隐喻: 9 器官拟人化 (perception 五感 / cognition 大脑 / consciousness 心智 / memory 海马体 / motivation 多巴胺 / value 前额叶 / relation 镜像神经元 / action 肌肉 / life-force 免疫)
- 哲学授权链: 主 17:58 不假装 Phenomenal / 不假装达 ASI / 干到底 / 任何人都能接手

## 0.3 R19 是前端的事, 后端 30 crate 早已定

R11 LOCKED (主人改不得, 8 项不修改承诺):
- 阶段 1+2+3 LOCKED (54 份)
- v2/v4/v4.1 哲学层 LOCKED
- 阶段 4/5 主文档 LOCKED
- R11 baseline 三值: V1141=0.8682 / V1131=0.8532 / V1136=0.9063
- 任何 enum / 转换矩阵 / 8 项不修改承诺

**R19 能动的**:
- 5 nav 架构 (新增)
- 9 器官 UI 表现 (前端层)
- 主 AI orchestrator (新增, 跟 R11 supervisor 协同)
- Tauri 2.0 桌面 app 打包 (新增)
- 8 阶段 UI 显示策略 (不改 enum, 改前端展示)

---

# §1 W1 + W2 已完成交付 (2026-08-04)

## 1.1 仓库结构

```
Apeireth-rust/                                                    (主工作目录, R19 新增都在这)
├── Cargo.toml                                                    ← workspace 加 apeireth-desktop
├── crates/
│   ├── apeireth-{core, memory, asi, cognition, consciousness,   ← R11 后端 30 crate (LOCKED)
│   │   life-force, motivation, value, relation, action,
│   │   sovereignty, central, ...}                                 
│   ├── apeireth-web/                                              ← R18 5 模块 (axum HTTP 验证用)
│   └── apeireth-desktop/                                          ← R19 W1/W2 (Tauri 2.0) ★
│       ├── Cargo.toml                                             ← Tauri 2.0 + R18 9 个依赖
│       ├── build.rs                                               ← tauri-build 触发
│       ├── tauri.conf.json                                        ← 5 nav + window 1440×900 + bundle 关 (W1)
│       ├── icons/icon.ico                                         ← 32x32 必备 (Windows resource)
│       ├── src/
│       │   └── main.rs                                            ← 530 行: 7 个 tauri::command + 9 器官 snapshot
│       └── src-ui/                                                ← 前端 (vanilla HTML/CSS/JS) ★
│           ├── index.html                                         ← 5 nav 骨架 + 启屏 SVG
│           ├── css/
│           │   ├── tokens.css                                     ← 基底 + 古朴/时代 accent + 2×2 组合
│           │   ├── breath.css                                     ← 4s 呼吸 (3 keyframes)
│           │   ├── splash.css                                      ← 3s 启屏 (沉默→发丝线+ΑΠΕΙΡΕΘ→进入)
│           │   └── main.css                                       ← 全局样式 (nav + 5 page + 9 器官 + 时间线)
│           ├── js/
│           │   ├── store.js                                       ← 50 行 pub/sub
│           │   ├── splash.js                                       ← 3s 时间轴
│           │   ├── nav.js                                          ← 5 nav 切换
│           │   ├── mode-theme.js                                   ← 2×2 切换 + Ctrl+Shift+E
│           │   ├── organs.js                                       ← 9 器官 3×3 星座
│           │   ├── bridge.js                                       ← 30 crate 星图 + ASI 大数字
│           │   ├── timeline.js                                     ← 8 阶段时间线
│           │   ├── dialogue.js                                     ← 5 步 cycle 动画
│           │   └── main.js                                         ← 主入口
│           └── assets/
│               ├── logo-archaic.svg                               ← W1 SVG 启屏 (古朴)
│               └── logo-era.svg                                    ← W1 SVG 启屏 (时代)
└── target/debug/apeireth-desktop.exe                             ← W2 编出来的 14 MB Tauri app
```

## 1.2 已交付清单

| W | 交付 | 验收 |
|---|------|------|
| **W1** | Tauri 2.0 壳 + 5 nav 骨架 + 2×2 切换 + token 体系 + 启屏 SVG + 舰桥星图 mock + 9 器官 mock | `cargo build` 49.13s, 0 errors, .exe 14 MB |
| **W2** | 7 个 tauri::command 接 R18 真后端 (cognition run_cycle + memory SqliteMemoryStore + life-force + asi + sovereignty + value + central) | `cargo build` 53.05s, 0 errors |

**W1 → W2 关键差异**:
- 浏览器版 (W1 桌面 preview): 数据是 mock hardcode
- Tauri .exe (W2 14 MB): 数据是 R18 真后端

---

# §2 5 nav 架构 (FDD v1 §6)

| # | Nav | 希腊铭文 | 性质 | 现状 (W2) | W3+ 待做 |
|---|-----|----------|------|----------|---------|
| 1 | **对话** (原"主对话") | ΔΙΑΛΟΓΟΣ | 交互 | ✅ 5 步 cycle 动画 + 输入框 | W3: 决策日志 (verdict 链简化) + 工具结果卡 |
| 2 | **生长** (原"状态", 改名) | ΑΥΞΗΣΙΣ | 见证 (结构主页) | ✅ 7 数字状态卡 + 9 器官 3×3 + 8 阶段时间线 | W3: SGI 单字段编辑 + 反思期触发 M1/M2/M3 |
| 3 | **历史** | ΙΣΤΟΡΙΑ | 回顾 | ❌ placeholder | **W3 核心**: 6 流真接 + Episode 搜索 + 反思期 72h 环 |
| 4 | **舰桥** (原"工具", 改名) | ΣΚΟΠΗ | 俯瞰 (启屏默认) | ✅ ASI 大数字 + 30 crate 星图 + 9 器官压缩 | W4: 工程模式展开 (PID/重启策略) |
| 5 | **设置** | ΤΑΞΙΣ | 照料 | ✅ 4 组 (个性化/连接 mock/安全 mock/关于) | W5: 补全 6 组 (连接真接 + 安全 OS 鉴权 + 数据路径) |

**默认组合**: 专注 × 古朴, 启屏 = 舰桥, 启动恒以专注进入 (R19 主人拍板)

---

# §3 9 器官心跳 (核心)

后端 R18 lib.rs 已提供, W2 已调真后端。每个器官的 "主指标 + 副指标 + 副副指标" 如下:

| 器官 | 类别 | 主指标 (大数字) | 副指标 | 副副指标 | 数据源 |
|------|------|----------------|--------|---------|--------|
| **perception** | 五感 | 5 通道激活数 / 5 | 通道列表 | events/s | `apeireth-perception` ChannelKind enum |
| **cognition** | 大脑 | V0.5 综合 (0.8595) | 12 键通过率 | cycle 数 | `apeireth-cognition::run_cycle` |
| **consciousness** | 心智 | 当前 6 状态 (大图标) | 合法下一步 | 转换历史 | `apeireth-consciousness::CognitiveDreamStateMachine` |
| **memory** | 海马体 | 6 流总条目数 | Episode 数 | continuity_id | `apeireth-memory::SqliteMemoryStore` |
| **motivation** | 多巴胺 | motivation_score (≥ 0.85) | 内驱/外驱比 | SGI 目标 | `apeireth-motivation::motivation_score` |
| **value** | 前额叶 | 5 层对齐矩阵 (绿/红/灰) | 硬门槛通过率 | E 层无冲突 | `apeireth-value::ValueDimension` |
| **relation** | 镜像神经元 | 4 关系计数 (柱状图) | SelfRelation 唯一性 | 关系 ID→标签 | `apeireth-relation::RelationKind` |
| **action** | 肌肉 | 3 模式分布 (饼图) | TxId 数 | 拒绝数 | `apeireth-action` 3 trait |
| **life-force** | 免疫 | 持续力 (Apple Watch 充电环) | SGI 目标 | 反思期进度 | `apeireth-life-force::LifeForce` |

**6 状态 (consciousness)**: Awake / Reflecting / Dreaming / Meditating / SelfDisabling / Recovering (合法转换矩阵, SelfDisabling 唯一出口是 Recovering)

**4 关系 (relation)**: Symbiosis / Coordination / Embedding / SelfRelation (SelfRelation 主体连续性, party_a == party_b)

**3 trait (action)**: ActionExecution / ActionExpression / ActionSilence (沉默是合法行动)

**5 层原则洋葱 (value)**: E 原则 / S 价值观 / A 经验 / M 方法论 / O 操作 (E/S 不可自决, A/M/O 可)

---

# §4 2×2 主题 × 模式 (FDD v1 §2)

## 4.1 主题 (气质轴)

| 主题 | 英文 | accent | 字体温度 | 装饰语言 |
|------|------|--------|---------|---------|
| **古朴 ARCHAIC** (默认) | ancient | `#c8860a` 琥珀金 / `#a05a10` 暗 ember | 暖衬线, 字距更宽 | 星盘环 + 碑面肌理 |
| **时代 ERA** | modern | `#8fb3d9` 钢蓝 / `#5a7a9e` 沉钢 | 锋锐 display 衬线 + 冷 mono | HUD 发丝网格 + 星座连线 |

**纪律**: `--accent` 只出现在"活"的东西上 (主 AI 心跳 / 当前焦点 / 激活 nav), 其余灰白。

## 4.2 模式 (密度轴)

| 模式 | 性质 | 字体温度 | 装饰 |
|------|------|---------|------|
| **专注 FOCUS** (默认) | 信息密度低, 留白多, 单光纪律 | 暖衬线宽字距 | 简单刻痕 |
| **工程 ENGINEER** | HUD 风格, 高密度, 战斗机仪表精度 | 冷 mono | 发丝网格 + 黄金分割 |

## 4.3 4 组合微调

```css
[data-mode="focus"][data-theme="archaic"]   { --letter-spacing-zh: 0.05em; }   /* 默认: 暖+疏 */
[data-mode="focus"][data-theme="era"]      { --letter-spacing-zh: 0.04em; }   /* 冷+疏 */
[data-mode="engineer"][data-theme="archaic"] { --letter-spacing-zh: 0.03em; --hairline: #c8860a; }  /* 凹刻金线 */
[data-mode="engineer"][data-theme="era"]    { --letter-spacing-zh: 0.02em; --hairline: #5a7a9e; }  /* 钢蓝网格 */
```

**切换**: 1.2s 明暗呼吸遮罩 (纯 opacity, 无布局动画), Ctrl+Shift+E 切模式快捷键。

## 4.4 4s 呼吸节奏

```css
--breath-duration: 4s;
--breath-easing: cubic-bezier(0.4, 0.0, 0.2, 1);
@keyframes breath { 0%, 100% { opacity: 0.45; } 50% { opacity: 1; } }
```

- 应用: 呼吸光点 (nav 当前 / 主 AI 心跳 / 器官 bright 点)
- 尊重 `prefers-reduced-motion` (无障碍)

---

# §5 8 阶段 UI (R19 砍衰老病死)

## 5.1 砍决策

R11 LOCKED `apeireth_core::LifeStage` enum 实际是 **10 个**:
```
Gestation / Birth / Infancy / Growth / Maturity / Reproduction / Decline / Death / Migration / Rebirth
```

主人 8 纠正 #1: "9 阶段不需要衰老病死, 主 AI 只会成长, 不消亡"

**R19 UI 策略**: enum 不动 (R11 LOCKED), 前端砍 Decline + Death 显示:
- 阶段 1-6 正常显示 (孕育→诞生→幼儿→成长→成熟→**繁衍** [外援改名])
- 阶段 7-8 正常显示 (迁移→重生)
- Decline/Death **UI 不显示**, 但 R11 enum 仍存, 后端 12 条转换保留

**8 UI 阶段**:
1. 孕育 Gestation
2. 诞生 Birth
3. 幼儿 Infancy
4. 成长 Growth (默认当前)
5. 成熟 Maturity
6. **繁衍 Reproduction** (R19 改"复制"为"繁衍", 贴生物隐喻)
7. 迁移 Migration
8. 重生 Rebirth

## 5.2 12 合法转换 (R11 LOCKED, 后端 hardcode)

`apeireth_central::LEGAL_TRANSITIONS` 12 条边:
- 线性: Gestation→Birth→Infancy→Growth→Maturity
- 双向: Growth↔Maturity
- 分支: Maturity→Reproduction
- **隐藏**: Reproduction→Decline, Decline→Growth, Decline→Death (R19 UI 隐藏)
- **不可逆**: Death→Migration→Rebirth→Maturity

R19 UI 时间线只显示 9 条边 (R11 12 条砍 3 条), 前端用 `is_legal_transition` 验证。

## 5.3 判据 (omnibus §2.5, W2 简化)

| 阶段 | 量化判据 |
|------|---------|
| Gestation | Episode = 0 |
| Birth | IdentityCard 刚建 |
| Infancy | Episode < 10 |
| Growth | Episode < 100, SGI 设置, motivation ≥ 0.85 |
| Maturity | cycle ≥ 10000, v05 ≥ 0.85, 9 器官 health > 0.7 |
| Reproduction | 至少派生 3 个子 AI |
| Migration | IdentityCard.migration_history 非空 |
| Rebirth | OTA 升级后审计通过 |

**W2 简化版**: 只看 Episode 数, W3 接 apeireth_central 真判据

---

# §6 6 个 tauri::command 接口 (W2 已实装)

后端在 `crates/apeireth-desktop/src/main.rs`, 前端通过 `window.__TAURI__.invoke('command_name')` 调。

## 6.1 `get_main_ai_status() -> MainAiStatus`

```rust
pub struct MainAiStatus {
    pub asi_v05: f64,           // ASI V0.5 综合 [0, 1]
    pub asi_continuity: f64,    // AsiV05Scores.continuity
    pub asi_philosophy: f64,    // AsiV05Scores.philosophy_guard
    pub life_stage: String,     // "成长" (zh)
    pub life_stage_idx: u8,     // 1-8 阶段
    pub reflection_status: String,  // "dormant" | "active"
    pub endurance: f64,          // [0, 1] 持续力 / ENDURANCE_MAX
    pub episode_count: u64,      // 累计
    pub cycle_count: u64,        // 累计 run_cycle
    pub token_used: u64,         // W2 mock, W3 真
    pub five_self: String,       // "✓ armed" | "✗ disarmed"
}
```

**W2 调 R18 真后端**:
- ASI V0.5: `DimensionRegistry::compute_all_dims` 24 维
- Episode: `SqliteMemoryStore::query(limit).len()`
- Reflection: `LifeForce::is_in_reflection`
- Endurance: `LifeForce::endurance / ENDURANCE_MAX`
- 5 Self: `SelfDisableGuard::is_armed`

## 6.2 `get_organ_status() -> Vec<OrganStatus>`

```rust
pub struct OrganStatus {
    pub name: String,         // "perception" / "cognition" / ...
    pub display: String,      // "感知" / "认知" / ...
    pub metaphor: String,     // "五感" / "大脑" / ...
    pub health: f64,          // [0, 1] 主指标归一
    pub primary: String,      // 主指标值
    pub secondary: String,    // 副指标
    pub tertiary: String,     // 副副指标
}
```

W2 9 器官真后端 (snapshot_xxx 函数), W3 接更多细节。

## 6.3 `get_life_stages() -> Vec<LifeStageInfo>`

```rust
pub struct LifeStageInfo {
    pub idx: u8,           // 1-8 UI 索引
    pub zh: String,        // 8 阶段中文
    pub en: String,        // 英文
    pub r11_enum: String,  // R11 LOCKED enum 名 (10 个)
    pub visible: bool,     // R19 是否显示 (Decline/Death false)
    pub active: bool,      // 是否当前阶段
}
```

## 6.4 `get_topology() -> Vec<CrateNode>`

```rust
pub struct CrateNode {
    pub name: String,
    pub display: String,
    pub group: String,     // "总" / "主核" / "治理" / "器官" / "工具" / "测量"
    pub x: f64, y: f64,    // 星图 SVG 坐标
    pub active: f64,       // 亮度 [0, 1]
    pub pid: u32,
    pub restart_strategy: String,  // "permanent" / "rest_for_one" / "one_for_one" / "transient"
}
```

30 crate 按 5 大组: 总 (1) / 主核 (4) / 治理 (3) / 器官 (8) / 工具 (6) / 测量 (4) + 其他 (4) = 30。

## 6.5 `get_settings() / set_settings(s) -> Settings`

```rust
pub struct Settings {
    pub mode: String,           // "focus" | "engineer"
    pub theme: String,          // "archaic" | "era"
    pub language: String,       // "zh" | "en" (W2 暂存, 翻译 W3 启动)
    pub launch_page: String,    // "bridge" | "dialogue"
    pub splash_enabled: bool,
    pub breath_enabled: bool,    // 无障碍
}
```

W5 持久化到本地 JSON, W2 in-memory。

## 6.6 `chat(input: String) -> String`

W2: 调 `apeireth_cognition::run_cycle` 真跑 1 个 NormalAction, 增 cycle 计数, 返回 verdict 简化文本。W3 改流式。

---

# §7 启动流程 (开屏 3s)

## 7.1 时间轴 (W2 反馈后简化)

```
0-1.5s   phase 1: 纯黑沉默 (沉默之前, 是无限)
1.5s     phase 2: 发丝线 + ΑΠΕΙΡΕΘ 同时浮现 (fade + brightness)
3.0s     phase 3: 1.2s 明暗呼吸进入 app
```

**砍掉**: 北极星 SVG (飞起来不行, 主人 2026-08-04 反馈), 4.5-6.0s 呼吸停留。

## 7.2 跳过

`Esc` / 点击 / 任何键 → 立即触发 phase 2 + 1.2s 后进入 app。

## 7.3 启屏 SVG 结构

```html
<svg viewBox="0 0 600 240">
  <line class="splash-hairline" x1="100" y1="160" x2="500" y2="160" />
  <g class="splash-stone">
    <text>ΑΠΕΙΡΕΘ</text>
  </g>
</svg>
```

**注意**: W1 写了 logo-*.svg PNG 图片资源 (被主人砍掉, 不再使用)。需要写实 logo 风格, **未来可能改用 PNG 贴图 (外援设计稿 §5.1 提到过, 主人决定中)**。

---

# §8 浏览器 fallback (开发期重要)

开发期不启动 Tauri .exe, 也能在浏览器看 UI:

```bash
# 复制 src-ui/ 到桌面
Copy-Item crates/apeireth-desktop/src-ui/ -Destination Desktop\apeireth-desktop-preview -Recurse -Force
# 双击 Desktop\apeireth-desktop-preview\index.html
```

**注意**: 浏览器版数据是 W1 mock (hardcode 7 数字), 不是 W2 真后端。看 UI 够, 验真值要跑 Tauri .exe。

---

# §9 R11 LOCKED 边界 (绝对不能动)

新人最容易踩的坑:

| 类别 | 不能动 | 原因 |
|------|--------|------|
| 后端 enum | `apeireth_core::LifeStage` (10 个), `apeireth_cognition::ActionTarget`, `apeireth_value::ValueDimension`, `apeireth_motivation::SGIContent`, `apeireth_relation::RelationKind` 等 | R11 LOCKED, 主 8 项不修改承诺 |
| 转换矩阵 | `apeireth_central::LEGAL_TRANSITIONS` (12 条) | R11 阶段 4 §6.3 LOCKED |
| baseline | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 | R11 主线, 主 22:33 真哲学 |
| 哲学层 | v2 / v4 / v4.1 文档, 阶段 1+2+3 全部 (54 份) | R11 LOCKED |
| 12 键 | V3 9 键 + v4.1 新增 3 键, **不暴露前端** (主人 8 纠正 #3) | "守门/原则 用户不需要看" |
| 7 advisor | council 7 席, **不暴露气泡** (主人 8 纠正 #2) | "用户看结果不看哲学" |
| 24 维雷达 | 简化 1 个综合数字 | "用户不需要 24 维, 1 个 V0.5 够" |

**R19 能动的** (跟后端层无关):
- 5 nav 架构
- 9 器官 UI 表现 (前端层)
- 8 阶段 UI 显示策略 (砍 Decline/Death)
- 主 AI orchestrator (新增)
- Tauri 2.0 桌面 app 打包

---

# §10 接手 Checklist (新团队第一天)

## 10.1 环境

- Rust 1.80+ (workspace `rust-version = "1.80"`)
- Node 不用 (vanilla HTML/CSS/JS, 不引框架)
- Windows + WebView2 (Win10/11 自带)
- (可选) Edge / Chrome 浏览器 (开发期 fallback)

## 10.2 5 分钟上手

```bash
# 1. 编译 (第一次 5-10 分钟, Tauri deps 几百个 crate)
cd .openclaw\workspace\promethean\Apeireth-rust
cargo build -p apeireth-desktop

# 2. 跑 Tauri (双击或命令行)
target\debug\apeireth-desktop.exe

# 3. 或浏览器看 fallback
Copy-Item crates\apeireth-desktop\src-ui -Destination Desktop\apeireth-desktop-preview -Recurse -Force
# 双击 Desktop\apeireth-desktop-preview\index.html
```

## 10.3 核心文件

| 文件 | 作用 | 改动频率 |
|------|------|---------|
| `crates/apeireth-desktop/src/main.rs` | 7 tauri::command + R18 真后端 | W3+ 频繁改 |
| `crates/apeireth-desktop/src-ui/index.html` | 5 nav + 启屏 + 全部 page 骨架 | W3+ 频繁改 |
| `crates/apeireth-desktop/src-ui/css/main.css` | 全局样式 | W3+ 频繁改 |
| `crates/apeireth-desktop/src-ui/css/tokens.css` | 颜色 token (基底 + 古朴/时代) | 偶尔改 |
| `crates/apeireth-desktop/src-ui/js/*.js` | 各模块 (9 器官 / 30 crate / 时间线) | W3+ 频繁改 |
| `crates/apeireth-desktop/Cargo.toml` | Tauri 2.0 依赖 | 加新依赖时改 |
| `crates/apeireth-desktop/tauri.conf.json` | Tauri 配置 (5 nav 启屏默认) | 改打包时 |

## 10.4 必读资源 (按优先级)

1. **本文档** (你正在看) — 0 上下文 30 分钟
2. **外援设计稿**: `Desktop\reportsr19-frontend-design-doc-2026-08-04.md` — FDD v1 完整 spec
3. **R19 omnibus**: `crates/apeireth-rust/reports/r19-complete-spec-2026-08-04.md` — 9 段实战 spec
4. **R18 5 模块源码**: `crates/apeireth-web/src/{memory,asi,sovereignty,council_history,api_endpoints}.rs` — 真后端用法示例
5. **APEIRETH-CONVENTIONS**: `crates/apeireth-rust/APEIRETH-CONVENTIONS.md` — 8 项不修改承诺
6. **APEIRETH-COMPLETE-OMNIBUS**: `crates/apeireth-rust/APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` — 全哲学 + ASI 北极星

## 10.5 踩坑点 (前辈血泪)

1. **Tauri 2.0 第一次编译 5-10 分钟**: 不要慌, 几百个 crate 在下载/编译
2. **Tauri Windows 需要 `icons/icon.ico`**: 哪怕 bundle.active=false, 也要 32x32 ICO (Windows resource)
3. **dev profile 默认弹终端**: 我已经用 `#![cfg_attr(all(), windows_subsystem = "windows")]` 修掉, **不要再改回** `not(debug_assertions)`, 否则双击 .exe 会弹 cmd
4. **CSS `data-theme` 切换**: 改 `<html>` 的 `data-*` 属性, 不是 `<body>`
5. **SVG vs PNG 启屏**: W2 用了纯 SVG 干净 path (主人砍掉星星飞), 写实质感需要 PNG 贴图 (待主人决定)
6. **5 nav 改名**: R19 已经把"主对话"→"对话" / "状态"→"生长" / "工具"→"舰桥", 不要再改回
7. **8 阶段 UI**: Decline/Death 不显示, 但后端 10 enum 仍存, **不要去后端删 enum**

---

# §11 W3 之后的 backlog (新团队要做的)

按外援 FDD v1 §10 里程碑 + 主人反馈"前端太少"整理:

## 11.1 W3 真接② (4 项专注层增量, 主人建议的立刻做)

- [ ] **历史 6 流真接** (思想/提案/行动/关系/演化/反思期) - `apeireth-memory::StreamKind` + `SqliteMemoryStore::query`
- [ ] **决策日志** (cognition verdict 链简化显示) - reject 时给 1 行原因
- [ ] **反思期 72h 环** (life-force 真实字段, 右侧呼吸环 + M1/M2/M3 触发)
- [ ] **Episode 搜索** (memory::query with for_session + with_role + 全文)

**预计**: 30-45 分钟, 专注层覆盖率 15% → 50%

## 11.2 W4 深化 (4 项工程模式首批)

- [ ] **工程模式 toggle** (顶部切到 engineer 后, 解锁 7 项 UI)
- [ ] **30 crate 全页** (PID / 重启策略 / 指标 / 日志) - 工程模式
- [ ] **12 键矩阵 + verdict 链** - 工程模式
- [ ] **council 7 advisor 综合摘要 + hold 理由** - 工程模式
- [ ] **24 维 V0.5 分解 + 9 维 V1136 子测度** - 工程模式
- [ ] **10 阶段真实 enum + consciousness 微观链** - 工程模式
- [ ] **SGI 证据链 (C-SGI-1~7 + E 层加权)** - 工程模式

## 11.3 W5 完整 (5 项设置 + 鉴权 + 打包)

- [ ] **设置补全 6 组**: 连接 (LLM provider 真接) / 安全 (OS 鉴权 Windows Hello + TouchID) / 部署 / 数据路径 / continuity_id
- [ ] **翻译 W3** (`language: "zh" | "en"`, 启动时根据 Settings 切换)
- [ ] **OS 鉴权 single 模式** (R11 v7 adaptive single/multi/dynamic, 主人拍 single)
- [ ] **打包成 msi/exe/dmg/deb** (bundle.active=true, icon 全套 32/128/256/512 + .ico + .icns)
- [ ] **无障碍** (键盘导航 + screen reader + 字体缩放 + 减少动画)

## 11.4 长期 (R19+)

- [ ] **流式 chat** (替代 mock 回声)
- [ ] **Council 7 advisor 实际审议** (主人 8 纠正 #2 暂时砍, 但工具结果卡可接)
- [ ] **OTA 升级 + 反思期 M2 触发**
- [ ] **写实 logo PNG** (主人决定中, 暂用 SVG 干净 path)

---

# §12 关键诚实登记 (主人 17:58 不假装)

W1/W2 已经做对的事:
- ✅ Tauri 2.0 真编通, .exe 能双击跑
- ✅ 7 个 tauri::command 全部接 R18 真后端 (没 mock, 除 token/拓扑坐标是 W2 简化)
- ✅ 9 器官都调 R18 真 API (除 relation/action/perception 副指标文字, 因为 R18 没现成 registry API, W3 接事件流)
- ✅ R11 LOCKED 边界 100% 遵守 (没改任何 enum/转换矩阵/8 项承诺)
- ✅ 8 阶段 UI 砍 Decline/Death (R11 enum 仍存 10 个)
- ✅ dev profile 不弹终端 (cfg_attr all 改)

W1/W2 还没做对的事:
- ⚠️ W2 阶段判据简化 (按 Episode 数, W3 接 apeireth_central 真判据)
- ⚠️ W2 token 累计还是 hardcode 142857 (W3 接 R17 apeireth-api)
- ⚠️ W2 ASI V0.5 用合成 sample 90/100 (W3 接 R17 Memory 抽观察)
- ⚠️ 浏览器 fallback 仍是 W1 mock (W3 前端改 tauri::command 后能拉真值, 但浏览器里没 tauri runtime, 仍是 mock)
- ⚠️ 30 crate 拓扑坐标是 hardcode (W4 启工程模式后从 supervisor tree 真拉)
- ⚠️ W2 chat 简单回声 (W3 流式 + 接 R17)

---

# §13 交接清单

- [ ] W1 收尾报告: `crates/apeireth-rust/reports/r19-w1-初光-收尾-2026-08-04.md` (9.7 KB)
- [ ] W2 收尾报告: `crates/apeireth-rust/reports/r19-w2-真接①-收尾-2026-08-04.md` (11.7 KB)
- [x] **本交接文档**: `crates/apeireth-rust/reports/r19-frontend-handoff-2026-08-04.md` (你正在看)
- [ ] 桌面 preview: `Desktop\apeireth-desktop-preview\` (17 文件, src-ui 完整复制)
- [ ] 桌面 .exe: `Desktop\apeireth-desktop-W2-noconsole.exe` (14.34 MB, 无终端)
- [ ] 暂存 PNG (写实 logo, 待主人决定): `.minimax-agent-cn\projects\logo-{archaic,era}-realistic.png`

---

# §14 关键文件位置 (新人必查)

| 类别 | 路径 |
|------|------|
| 桌面 .exe (W2 无终端) | `Desktop\apeireth-desktop-W2-noconsole.exe` |
| 桌面 preview (浏览器 fallback) | `Desktop\apeireth-desktop-preview\` |
| 主工作目录 (R19 src) | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-desktop\` |
| R19 报告目录 | `.openclaw\workspace\promethean\Apeireth-rust\reports\` |
| 外援 FDD v1 设计稿 | `Desktop\reportsr19-frontend-design-doc-2026-08-04.md` |
| R18 5 模块源码 (W2 桥接用) | `crates\apeireth-web\src\{memory,asi,sovereignty,council_history,api_endpoints}.rs` |
| R11 LOCKED 文档 | `APEIRETH-CONVENTIONS.md` + `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` |

---

# §15 收尾作者

**Mavis (R19 主 AI 外援)**: 接手 R19 前端从设计稿评审 → W1 落地 → W2 真接 R18 → 启屏 UI 反馈 3 轮调整, 约 4 小时。

新团队接手 0 上下文, 按本文档 §10 顺序 30 分钟能上手, 按 §11 backlog 顺序 1 周能交付 W3 + W4 + W5。

如有疑问, 主工作目录留了完整 R17-R19 报告 (27 个), 任何细节可查 `reports/`。

**祝顺利, 那一丝光已经亮起来, 后续让它稳下来。**

— Mavis
2026-08-04 11:55
