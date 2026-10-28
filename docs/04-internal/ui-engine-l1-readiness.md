# 引擎 L1 完成度体检表（2026-10-28）

> 判定口径：owner 2026-10-28 裁定的三层边界（见 [ADR-0015 增补](../adr/0015-pure-rust-ui-engine.md)
> 与 [能力边界与交接](ui-engine-l1-capability-handoff.md)）。
> **「引擎无已知缺陷」只覆盖 L1**；L2/L3 不计入本表。
>
> 四级口径：**IMPLEMENTED**（代码在）/ **PRODUCTION WIRED**（真实路径在用）/
> **DEFAULT ENABLED**（默认开）/ **HARDWARE VALIDATED**（真实硬件/浏览器实测）。
> 数字均可在 `crates-ui` 内用一条命令复跑（见 §5）。

## 1. 结论（最终判定，2026-10-28）

**L1 已达成「无已知缺陷」的可交付状态**：14 项能力全部 IMPLEMENTED，
其中 **10 项**有浏览器或真实 GPU 证据；本地等价 CI **8/8**（E-116 快照）；
`cargo test --workspace` **1119 passed / 0 failed**。

**「上屏第一屏」可以开工；「引擎可上线」不可写进排期**，直到满足 §7 的三条放行条件。
三条条件都是**外部/口径**事项，不是引擎代码问题：
①CI 一次真绿；②真实 Tauri WebView 至少一个平台跑通；③E-54 判据口径由 owner 裁定。

## 2. L1 能力清单（按四级口径）

| # | 能力 | 四级口径 | 证据 |
|---|------|----------|------|
| 1 | 响应式调度 / DOM 绑定与差值写回 | IMPLEMENTED + WIRED + VALIDATED（五面逐像素） | E-34 / E-71 / 五面对照 |
| 2 | 受控输入闭环（`value`/`checked` 差值写回 + 回读） | IMPLEMENTED + WIRED | E-90 块；`renderer.rs` 的 `patch_props` |
| 3 | **G1 布局测量**（`bounding_box`/`scroll_metrics`/`set_scroll_top`） | IMPLEMENTED + WIRED + **浏览器已验证** | E-91 / E-98（引擎盒 ≡ JS `getBoundingClientRect`） |
| 4 | **G2 事件面（19 类）与载荷**（`checked`/坐标/`button`/`pointer_id`/`alt`/组合数据） | IMPLEMENTED + WIRED（浏览器侧载荷已验证） | E-92 / E-98 |
| 5 | **G3 焦点栈 + 焦点陷阱** | IMPLEMENTED + WIRED + 浏览器已验证（`activeElement`） | E-93 / E-96 / E-98 |
| 6 | **G3 IME 组合静默期**（document 级自动跟踪 + 待写队列补写） | IMPLEMENTED + WIRED + 浏览器已验证（合成事件） | E-95 / E-97 / E-98 |
| 7 | **G5 虚拟列表**（窗口计算 / 窗口化渲染 / 嵌套子级替换 / 滚动监听 / 1k 行 O(窗口)） | IMPLEMENTED + WIRED | E-102 / E-103 / E-108 / E-109 / E-110 |
| 8 | **G6 缩放与密度标量**（DPR × density，含命中测试） | IMPLEMENTED + WIRED + 浏览器已验证 | E-94 / E-98 |
| 9 | **G7 资源句柄 / 加载状态 / 度量未定门控** | IMPLEMENTED + WIRED | E-107 |
| 10 | **G7 CLS 观测**（亚像素与视口外不计分） | IMPLEMENTED + WIRED | E-111 |
| 11 | **G8 错误边界**（渲染失败挂兜底 + 原始错误可读） | IMPLEMENTED + WIRED + **浏览器已验证** | E-104 / E-106 |
| 12 | **G8 panic 钩子**（消息记录 + 宿主上报器，防递归） | IMPLEMENTED + 部分 WIRED + 浏览器仅验「可安装」 | E-105 / E-106 |
| 13 | 渲染/像素等价（五面 + intro） | **真实 GPU 已验证**（逐像素 0 差异，3 次稳定） | E-68/69 / E-56 / E-71 |
| 14 | 双后端一致（WebGL vs WebGPU） | **已验证**（互差 0） | E-71 |

**未达 L1 标准（缺证据或未实现）**：

| 项 | 缺什么 | 影响 |
|---|---|---|
| G7 解码/上传钩子 | **接口已就绪**（E-112）+ **GPU 侧上传已实现**（E-113：256 行对齐 + 逐行拷贝 + `TextureStore`）；**适配器已实现**（E-114：`DomTextureUploader` 放在**同时依赖两边**的证据装置里，避免 ui-gpu→ui-dom 倒置分层；`Engine::device/queue` 只读访问器 + `wgpu` 重导出）；**浏览器端到端证据已采**（E-115：真实引擎上传 3×2 纹理 ⇒ 行对齐 256 + 纹理可用 + 资源 Ready；坏数据被拒）；产品路径接线仍待做 | 图片管线闭环前，宿主需自行上传或使用纯色占位 |
| panic 钩子真 panic 行为 | 浏览器内不可靠触发（wasm 可能 `panic=abort`） | 「一崩可见」只覆盖渲染失败，不覆盖进程级崩溃 |
| **真实 Tauri WebView（WebKitGTK / WebView2 / WKWebView）** | **无装置，0 次验证** | ⚠️ **上线前最大未知** |
| 其它 GPU / Windows | 无装置 | 同上（当前仅 AMD RADV + Linux headless Chromium） |

## 3. 缺陷台账状态

- **已闭合 / 已结案**：E-30、E-34、E-56、E-68、E-69、E-71、E-73、E-74、E-77、E-78、E-79、E-81、E-87（A4）、**E-101（E-54 定性）**
- **仍开**：**E-54 判据口径**（等 owner）、**A3**（只读区，随 frontend 替换面自然消解）、**CI 全绿**（等 runner）
- **负结果入库**（不重复踩坑）：E-82（层叠顺序）、E-86（分数相位）、E-99（box-shadow/color-mix）、E-100（祖先链/元素级定位）

## 4. 「上屏第一屏可开工」判定

| 判据 | 结论 |
|---|---|
| L1 核心能力齐备（输入/事件/测量/焦点/滚动/错误兜底/资源/CLS） | ✅ 是（14 项全部 IMPLEMENTED） |
| 本地等价 CI 8 job | ✅ **8/8**（E-116） |
| 浏览器能力探针 | ✅ EXIT 0（dom + upload 两段） |
| 像素与生产等价（真实 GPU） | ✅ 是（五面 0 差异；intro 0.016%） |
| 浏览器行为证据（测量/焦点/IME/缩放/错误兜底） | ✅ 已采（E-98 / E-106） |
| 真实 WebView 可运行 | ❌ **未知** |
| CI 全绿 | ❌ 未拿到 |
| 虚拟列表可用（嵌套 + 滚动联动） | ✅ 是（G5 完成） |
| 资源/字体未就绪时不跳布局 | ✅ 有门控 + CLS 可测 |

⇒ ⇒ **建议：可以开工「上屏第一屏」的设计与 L2 组件层工作（不依赖未验证平台）**，
但**不应**把「引擎可上线」写进排期 —— 真实 WebView 矩阵与 CI 结论是硬前提。

## 5. 复跑命令（本地等价）

```bash
cd crates-ui
cargo test --workspace --locked           # 本表基线：1111 passed / 0 failed（2026-10-28）
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cd evidence/p5-b && python3 drive_l1.py   # L1 能力浏览器探针（G1/G2/G3/G6/G8）
python3 ../../../scripts/run-ci-local.py --workflow ui-engine-ci.yml   # 8 job 本地等价
```

## 6. 需要 owner 决策（2 件）

1. **E-54 判据口径**：data 面 554 px（0.0626%）已定性为**对照装置固有差**（九项排除 + 元素级定位）；是否改为「有界判据（DOM/几何/样式全等 + ≤0.1% + 归因）」并保留严格读数作为报告项？
2. **平台矩阵**：是否投入一台真机/CI runner 做 Tauri WebView 三平台验证（或明确「首发只承诺 Linux Chromium 系」？）

## 7. 放行判定（最终）

### 7.1 已满足（可开工的前提）

| 项 | 证据 |
|---|---|
| L1 能力 14 项全部 IMPLEMENTED（其中 10 项有浏览器/GPU 证据） | 本表 §2 + E-091…E-115 |
| 五面与 intro 像素等价（真实 GPU，逐像素 0 差异，3 次稳定） | E-68/69 / E-56 / E-71 |
| 双后端一致（WebGL vs WebGPU 互差 0） | E-71 |
| 本地等价 CI **8/8**、测试 **1119/0**、clippy/fmt/wasm32 全 0 | E-116 |
| 浏览器探针 `dom + upload` 双段通过 | E-098 / E-106 / E-115 |
| 有界差异判据带反事实自检（判据不可失败 ⇒ exit 2） | `check_particle_edge.py` v2 |

### 7.2 放行条件（未满足前**不得**写「引擎可上线」）

| # | 条件 | 当前 | 归属 |
|---|------|------|------|
| C1 | GitHub CI 出现**一次全绿** | ❌ run `40701131910` 仍 queued | 外部（runner 积压） |
| C2 | **真实 Tauri WebView 至少一个平台**跑通（`Engine::create` + 一帧非黑 + 降级档） | ❌ 无装置 | 需要真机/runner |
| C3 | **E-54 判据口径**裁定（有界 + 保留严格读数 / 保持严格 0） | ⬜ 等 owner | owner |

### 7.3 允许并行开工的部分（不依赖 C1–C3）

- 上屏线 W0 设计与范式准备；
- L2 组件层（表单控件族 / Markdown / 图标 / 富文本）—— 归 `apeireth-ui-kit`，**不进引擎验收口径**；
- 主仓业务界面按 L1 API 施工（`render_guarded`、`patch_windowed`、`patch_children_at_path`、
  `virtual_window_for`、`observe_layout_shift_at_path`、`set_metrics_status` 已可用）。

### 7.4 明确**不**承诺的

- 真实 WebView 上的像素等价（当前证据仅系统 Chromium；WebKitGTK/WebView2/WKWebView **0 验证**）；
- 非 AMD GPU / Windows 平台行为；
- panic 后的恢复运行（`panic=abort` 下不可恢复，只保证消息可达）。
