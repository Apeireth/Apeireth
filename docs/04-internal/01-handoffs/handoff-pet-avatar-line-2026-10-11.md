# 交接：桌宠常驻投影 + 角色形象线（2026-10-11 封账）

> `Status: 🤝 已交接（本线暂告段落）`——全部批次落地并验收，余账各有明确等待方。
> **进场必读**：`docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md`（多 AI
> 施工宪章）+ `docs/design/avatar-pack-admission.md`（许可红线，渠道三分）。
> **用词纪律**：本文与本线全部文档中性表述，零第三方产品名（依赖包名仅出现在
> 工程清单与 import，属真实路径豁免）。

---

## ⚠️ 主人裁定与形象重做令（2026-10-11，最高优先级）

**主人裁定：现役前端观感"一言难尽、绝对不行"——形象（含 2D 模式）整体列为
重做项**，后续由**前端能力更强、审美更强的模型**主持形象重做讨论（2D 模式
方向届时专题议）。现役三个形象层实现（光态哨兵/2D 骨骼小人/3D 素体）一律按
**工程验证件**对待：它们证明的是契约与管线能跑，**不代表产品观感**，视觉
语言可整体推翻重写。

**重做边界（换皮不换契约）**：

| 保留不动（底座，重做线的进场即用） | 重做对象（审美线职权） |
|---|---|
| `PetFrame` 形象抽象契约（姿态/光色/呼吸/口型/视线/眨眼） | 全部形象层的**视觉语言**：造型、色彩表现、动态观感 |
| 四姿态机 + 光参数同源镜像 | `avatar/` 三组件可整体重写（只要吃 `PetFrame`） |
| 防唠叨引擎（气泡/抬头/开口预算） | 2D 模式的具体形态（专题讨论） |
| BYO 装载口 + 免责纪律 + 许可红线进代码 | 装载口呈现的观感（面板/菜单视觉） |
| 窗壳/互斥礼让/停帧闩/销毁纪律 | — |
| 素材台账 + CI 门禁 + 全部测试钉 | — |

**给审美重做线的进场清单**：① 读 `docs/design/00-PHILOSOPHY.md`（金色纪律、
"光不长脸"哲学——重设计可以**在哲学内重做，也可经主人拍板修订哲学**，不可
静默漂移）；② 读 `docs/design/01-DESIGN-SYSTEM.md` 令牌层；③ 本文 §3 契约图 +
§4 验证口径（改完跑全套门禁即知底座破没破）；④ 现役三实现仅作管线参考，不作
美学参考。**与主人的 2D 模式专题讨论是第一件事，别在方向定下来前动工。**

---

## 0. 一分钟现状

**presence 第四个投影（桌宠常驻面）已点亮**：透明常驻窗 + 四姿态机 + 主动涌现 +
形象层双形态可插拔（光态哨兵默认 / 开放 3D 角色形象华服 / 微光简模 T0）+
用户自带形象（BYO）装载口 + 素材许可 CI 门禁 + 官方形象委托需求书（待签发）。
**门禁全绿**，随时可继续施工或归并。

## 1. 交付清单（批批可验）

| 批 | 交付 | 验证 |
|---|---|---|
| 首批 | 常驻窗壳（透明/置顶/拖拽/穿透/托盘/互斥礼让/销毁纪律）+ 四姿态形象层 + 点击快捷窗/右键卷宗 | `tests/pet-window-lifecycle.mjs` + Rust 决策表 2 例 |
| 首批 | 主动涌现（事件气泡限频/抬头一瞥/开口 ≤3 次天预算） | `tests/pet-proactive.mjs` 7 组 |
| 形象抽象 | `PetFrame` 契约 + 插拔注册表 + 光参数与后端 HUD 驱动逐值镜像 | `tests/pet-frame.mjs` + `tests/pet-avatar-registry.mjs` |
| 批 A | 准入规范（许可红线**进代码**）+ 选型报告（30+ 实抓证据 URL） | `avatar-pack-admission.md` / `reports/avatar-runtime-selection-2026-10-11.md` |
| 批 B 前置 | 性能探针三线实测（预算口径校准 + 两条纪律固化为验收门禁） | `reports/avatar-perf-probe-2026-10-11.md` + 原始数据 JSON×3 |
| 批 B | 角色形象层（开放 3D 人形格式渲染 + 程序化素体兜底 + WebGL 降级）+ 契约→身体语言映射 | 烟测真渲染 + `tests/pet-character-mapping.mjs` 6 组 |
| 批 B | BYO 装载口（格式校验/免责确认/IndexedDB 持久化/内嵌许可元数据透传） | `tests/pet-user-avatar.mjs` 5 组 |
| 批 C | 素材许可 CI 门禁（台账+扫描脚本+workflow，双向验证）+ 用户指引 + 委托需求书 | `pnpm check:assets` + `docs/02-guides/avatar-byo-guide.md` |
| 批 D | 构建产物烟测 7/7（双形象层真渲染） | `scripts/pet-window-smoke.mjs` |

## 2. 文件图（本线写入面，接手从这里找起）

| 文件 | 职责 |
|---|---|
| `src/lib/pet/pet-contract.ts` | **形象抽象契约**（PetFrame/姿态映射/光参数镜像/层 id）——改契约先改这里 |
| `src/lib/pet/avatar-registry.ts` | 形象层注册表 + 清单 v1/v2 校验（**许可红线在此**） |
| `src/lib/pet/proactive.ts` | 主动涌现引擎（防唠叨数值表即契约） |
| `src/lib/pet/character-mapping.ts` | 契约→口型/眨眼/视线/姿态/呼吸纯映射 |
| `src/lib/pet/user-avatar.ts` | BYO 校验/免责文案/持久化/元数据归一 |
| `src/features/pet/PetWindowView.svelte` | 桌宠窗根（**与伴星状态机/审批快卡合建，改动必 rebase**） |
| `src/features/pet/AvatarHost.svelte` | 形象层宿主（插拔 + 降级回落） |
| `src/features/pet/avatar/*` | 三个形象层：光态哨兵 / 角色形象（3D）/ 微光简模 |
| `src-tauri/src/lib.rs` | 桌宠窗段（构建/开关决策/互斥礼让/卷宗入口/穿透） |
| `tests/pet-*.mjs`（6 套） | 行为钉子（真实模块 + 源码镜像） |
| `scripts/pet-window-smoke.mjs` / `scripts/avatar-perf-probe.mjs` | 烟测 / 性能探针一键采集 |
| `frontend/perf-probe/` | 探针工程 + 实测原始数据 |
| `frontend/companion-desktop/asset-licenses.json` + `scripts/check-asset-licenses.mjs` | 素材许可台账 + 门禁 |
| `docs/design/pet-resident-projection.md` | 施工总账（批次状态） |
| `docs/design/avatar-pack-admission.md` | **准入规范**（渠道三分/白名单/五步流程） |
| `docs/design/avatar-commission-brief.md` | 官方形象委托需求书（待签发） |
| `docs/02-guides/avatar-byo-guide.md` | 用户指引（指路不背书） |

## 3. 契约与纪律地图（改前必看）

1. **`PetFrame` 是长寿契约**——任何形象实现只许吃这个形状；换形象不换契约。
   光参数表与后端 `ember_hud_driver.rs` 逐值镜像，**两处同表改数需同步**。
2. **防唠叨数值表**（`proactive.ts`，数值即契约）：抬头 90s 门槛/2.5s 瞬时/
   5min 限频、开口 ≤3 次天、通报 45s 限频/120s 同文去重、回合事件不出台词。
3. **许可红线**（`avatar-registry.ts`）：白名单 = 公有领域等价 + 署名类（可商用）；
   非商用/禁改/禁二销/许可不明一律拒。**渠道三分**：① 用户自带 = 用户自担
   （免责确认）② 测试/演示 = 白名单 + 不进发布物 ③ 官方 = 全流程准入。
4. **两条渲染纪律（验收门禁）**：隐藏必停渲染；WebGL 不可用必优雅降级回落。
5. **0 装**：气泡只述真实事实；SIM 标注外显；未声明元数据 = null；素体兜底绝不空窗。
6. **金色纪律**：存在金 = 他；角色形象不占用金色语义。

## 4. 验证口径（证明还是绿的）

```powershell
# 前端全套（含 65 套件：本线 11 + 同事线 54）
cd frontend/companion-desktop
pnpm check && pnpm test && pnpm check:assets

# Rust 壳（独立 workspace）
cd src-tauri
cargo test && cargo clippy --all-targets && cargo fmt -- --check

# 构建 + 烟测（双形象层真渲染 7 例）
cd ../.. && node scripts/pet-window-smoke.mjs

# 中性表述纪律
pwsh -File scripts/check-neutral-terms.ps1
```

2026-10-11 封账时点实测：**65/65 套件 · 0/0 类型 · 素材 11/11 · Rust 71 例
0 败（clippy 0 / fmt 0）· 烟测 7/7 · 中性扫描零命中**。

## 5. 挂账与等待方（谁欠谁）

| # | 事项 | 等待方 | 备注 |
|---|---|---|---|
| 1 | ~~真机手感实测~~ **自动化已收**（2026-10-11 五件套批）：真 Tauri 窗 + 真 WebView2 实测 7/7 + 三形象层实拍图；手感残余（拖拽/穿透观感）仍可找 10 分钟人眼 | **主人**（可选） | `node scripts/pet-window-real-smoke.mjs`；实拍 `reports/pet-real-smoke-2026-10-11/` |
| 2 | **官方形象委托需求书三项拍板**（预算/形象设定/小动作批） | **主人** | `avatar-commission-brief.md` §6，签发即开询价 |
| 3 | initiative 生产者（他主动开口的后端产点） | **后端线** | 前端消费端 + 预算器已就位，接线一行点亮 |
| 4 | 语音口型（口型参数通道已在契约） | **后端语音通道先行** | 契约已留 viseme 位 |
| 5 | 便携安装（随身携带形态） | **打包线协同** | 产品想法，见随记 |
| 6 | 2D 社区格式装载口适配器 | 本线后继 | 2D 骨骼华服已落（程序化素体）；社区格式运行时许可核验收口后接适配器（npm 负证在案） |
| 7 | **形象视觉重做（含 2D 模式专题）** | **审美线（主人指派）** | ⚠️ 见本文顶部重做令：现役形象层=工程验证件，视觉语言全部重做；与主人先做 2D 模式方向讨论再动工 |

> **2026-10-11 五件套批增补**（本线封账后追加施工，同口径全绿）：2D 骨骼第二华服、
> 自主行为批（微动作+作息节律）、真窗实测升级、使用条款 BYO 段草案、形象设置面板；
> 真发现入档：`window.hide()` 不翻 `document.visibilityState` → 显式停帧闩
> （`pet-host-visibility` / `pet-render-pause.ts`）双保险。形象层现为**四态**。

## 6. 协作注意事项（给后继施工者）

- **`PetWindowView.svelte` 是合建文件**：本线（形象/装载口/菜单）与同事线（伴星
  状态机 `agent-status.ts`、审批快卡 `approval-quick.ts`）在此汇合——**先读后改、
  后到 rebase、绝不覆盖**（宪章二）。
- 跨范围先例报备：曾顺手修 `starport/apps/notes-store.ts` 两行类型债（归一值被
  丢弃），最小改动 + 注释，见 CHANGELOG 批次注。
- 归并节奏：本线成果由并行归并提交入库（`c0867872` 起），后续照此办理。
- 新形象资产入发布物前必须过 `pnpm check:assets`（台账无行即红）。

## 7. 变更历史索引

详见 `CHANGELOG.md` 两条批次记录（2026-10-11）：
① 桌宠常驻投影首批：形象层双形态可插拔 + 主动涌现；
② 批 B/C/D 收官：角色形象层 + BYO 装载口 + 素材许可门禁。
