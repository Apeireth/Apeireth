# 治理卷宗面对照 harness（并行面 B · 独占目录）

> 本目录是**治理卷宗面**（`ApprovalsTab` / `GrantsTab` / `AuditTab` / `GuardTab`）
> 的**全部**对照资产 ⇒ 写域独占 ⇒ 不碰共享 harness 的任何文件。

## 两侧是什么

| 侧 | 页面 | 内容 |
|---|---|---|
| **参考侧** | `reference/<面>/index.html` | **生产 Svelte 组件**（只读引入 `frontend/companion-desktop/src/lib/views/governance/*.svelte`）+ 固定 fixture |
| **引擎侧** | `engine/<面>/index.html` | **wasm 引擎**（`engine-host/` ⇒ `apeireth-ui-kit::governance_tab`）渲染同一批组件 + 同一份 fixture |

⇒ 四个卷宗面**各自一页** ⇒ 每个页面只加载**该面**的 CSS ⇒ 四个 Tab 之间同名
class（`.gov-card-title` / `.gov-chip` / `.gov-note` …，生产里**取值不同**）
不会互相覆盖（批一实测过：同名 class 冲突 ⇒ 整卡位移 2px）。

## 资产从哪来（**不手抄**）

| 资产 | 生成器 | 产物 |
|---|---|---|
| 四个 Tab + 六个共享子件的 scoped CSS | `scripts/extract-governance-assets.mjs`（用**生产自己的 Svelte 编译器**产出 ⇒ 与参考侧同源） | `public/<slug>.css` |
| 图标几何 | 同上 | `crates-ui/apeireth-ui-kit/src/icons_governance.rs` |
| 固定数据 | 同上 | `engine-host/src/fixture_generated.rs` |
| `runtime` 替身 | `scripts/gen-runtime-mock.mjs`（读生产 `runtime.ts` 的**导出面** ⇒ 自动派生） | `mock/runtime-mock.js` |

> **四级口径**：本目录的对照资产属于 **IMPLEMENTED**（实现齐备、可复跑）——
> **不是** PRODUCTION WIRED（它是对照用的证据设施，不进产品运行时），
> 更不是 DEFAULT ENABLED / HARDWARE VALIDATED。

### 作用域锚：`.svelte-xxx` ⇒ `:is(根, 根 *)`

生产组件的 `<style>` 里的选择器是**组件私有**的（Svelte 编译器给每个组件生成
一个作用域 class）。引擎页把样式当**普通样式表**注入 ⇒ 没有作用域 class ⇒
同名 class 会互相盖。⇒ 抽取器把作用域 class 换成**作用域锚**：

- `:is(.gov-tab, .gov-tab *)` = 根元素本身 + 它的所有后代；
- `:is()` 的特异度取参数中最高者（一个 class）⇒ **与原 `.svelte-xxx` 完全
  相同** ⇒ 层叠结果一致（设计令牌层里确有 `.amber` / `.on` / `.small` 等同名
  class ⇒ 这一步不是可选优化，是**正确性要求**）。

## `runtime` 替身（mock）

生产的四个 Tab 在 `onMount` 里拉数据（`fetchGrants` / `fetchCanonicalApprovals`
/ `fetchAuditLogs` / `fetchGuardStatus` …）并做能力门控 ⇒ 参考侧要喂固定数据
就得换掉 `../../runtime`。

⇒ `scripts/runtime-mock-plugin.mjs` 在 vite 的 `resolveId` 阶段拦截
**按 importer 路径判断**（只拦 `src/lib/views/governance/**` 指向
`src/lib/runtime` 的导入）⇒ **生产文件零改动**。

⇒ mock 的口径（**IMPLEMENTED**：对照设施的实现齐备）：

- **给了真实现**（喂固定 fixture）：`capabilityAvailable` /
  `capabilityUnavailableReason` / `findCapability` / `friendlyErrorMessage` /
  八个 fetcher；
- **其余一律显式报错桩**（调用即抛错）：其余 54 个导出（含
  `resolveCanonicalApproval` / `revokeGrant` ⇒ **变更调用不迁**）；
- **不碰数据面**：mock 只喂固定数据 ⇒ 引擎不碰 HTTP / SSE 契约。

## 用例模型

固定数据里每个用例带三样东西：

```json
{ "id": "approvals-ledger", "manifest": "full", "data": "default", "actions": [] }
```

| 字段 | 语义 |
|---|---|
| `manifest` | 能力清单键（`null` ⇒ 清单未到 ⇒ 等待态） |
| `data` | 数据行为：`default` / `empty` / `error` / `loading` |
| `actions` | **DOM 点击**序列（选择器 + 第几个匹配项）⇒ 两侧走**同一条交互路径** |

⇒ 动作是 DOM 点击（不是「调用某个 API」）⇒ 参考侧触发的是**生产组件自己的
handler**，引擎侧触发的是 **Rust 组件的 handler** ⇒ 状态可比。

## 时钟冻结

依赖「现在」的渲染（过期判定 / 相对时间 / 同日时刻）⇒ 对照把两侧的
`Date.now()` **冻结到同一基准时刻**（`window.__GOV_BASE__`）⇒ 两侧逐字一致。

## 对照判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | 静态像素 | 差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | DOM 全等 | 归一化后**完全相等**（文本节点**逐字**保留） |
| T2' | 状态快照 | 从 DOM 读出的两侧同义状态**完全相等** |
| T3 | 逐帧 | 配对帧的差异占比 **≤ 1%** |

环境：视口 `1440×900`、DPR 1、时区 `UTC`、同一浏览器实例、同一份 CSS、同一份 fixture。

### DOM 归一（照批一口径，但更严）

- 剔除 Svelte 作用域 class（`svelte-*`）与注释锚点；
- class token **排序**、属性键排序；
- **丢弃纯空白文本节点**；
- **其余文本节点逐字保留**（不折叠空白、不裁剪）⇒ 前导空格等差异会被抓住。

## 复跑

```bash
bash build.sh        # 抽取 → 生成 mock → 构建 wasm → wasm-bindgen → vite build → 对照
```

分步：

```bash
node scripts/extract-governance-assets.mjs          # 写入
node scripts/extract-governance-assets.mjs --check  # 漂移 ⇒ 退出码 1
node scripts/gen-runtime-mock.mjs                   # 写入
node scripts/gen-runtime-mock.mjs --check           # 漂移 ⇒ 退出码 1
python3 tools/compare_governance.py                  # 对照 ⇒ evidence/report.json
```

## 目录

```
governance/
├── face.manifest.json        抽取清单（单一真源）
├── scripts/                  抽取器 / mock 生成器 / vite 插件
├── mock/                     runtime 替身（生成物 + 已接线实现）
├── fixtures/                 固定数据（两侧同源）
├── shared/governance-api.js  两侧同名同义的控制 API
├── reference/<面>/           参考侧（生产组件只读引入）
├── engine/<面>/             引擎侧（wasm）
├── engine-host/              wasm 宿主 crate（**自带 [workspace]** ⇒ 不进依赖图）
├── public/                   CSS 资产 + 设计令牌（只读复制）
├── tools/compare_governance.py
├── evidence/                 对照证据（report.json + 画布 + 控制台）
└── out/                      对照中间产物
```

## 诚实边界

- **变更调用不迁**：签批 / 撤销 / 试算都是**宿主侧**的事 ⇒ 引擎只承接
  「给定数据 ⇒ 渲染出同样的画面」；
- **试算表单的初值**（`bind:value` 的 property 写回）在引擎侧**未接线**
  （`apeireth-ui-macro` 对 `bind:value` 显式报错）⇒ 由**宿主**在挂载后写入
  property（与 Svelte 侧的 `bind:value` 初值等效）；
- **SSE 连接态**：生产的 SSE 能力在对照清单里是 `available: false` ⇒ 参考侧
  恒为「实时不可用」⇒ 引擎侧同一口径（其余三种连接态由 native 单测钉住）；
- **审批面的错误分支**：生产的两个 fetcher 都自带 `.catch(...)` ⇒ 异步拒绝
  会被吞掉 ⇒ mock **同步抛出**才落得到那条分支（对照覆盖的是那条分支）；
- 对照覆盖的是**渲染面**：动作之后的**服务端状态变化**不在对照范围内。
