# Windows 接手须知 —— 前端引擎线（`ui-engine`）（2026-11-09 收尾交接）

[Document-Meta]
Document: docs/status/ui-engine-windows-handoff-2026-11-09.md
Class: STATUS
Status: 🟢 收尾交接
Last-Modified: 2026-11-09
Author: 前端引擎线（AI 施工）

> **用途**：后续开发**搬到 Windows** 时，本页是**引擎线的唯一入口** ⇒ ⇒ 读完本页就能接上，
> 不必翻对话、不必翻 git 历史。
>
> **配套真源（职责不重叠）**：
> * **L1 九项缺口逐项状态** ⇒ [../04-internal/ui-engine-l1-capability-handoff.md](../04-internal/ui-engine-l1-capability-handoff.md)
> * **能力口径汇总** ⇒ [../04-internal/ui-engine-l1-readiness.md](../04-internal/ui-engine-l1-readiness.md)
> * **阶段收尾报告（交付 + 七条难点）** ⇒ [../04-internal/ui-engine-l1-stage-closeout-2026-11-07.md](../04-internal/ui-engine-l1-stage-closeout-2026-11-07.md)
> * **缺陷台账（E-30…E-192）** ⇒ [../04-internal/ui-engine-defect-ledger.md](../04-internal/ui-engine-defect-ledger.md)
> * **私有仓（产品）** ⇒ `Apeireth/ui-desktop`（私有）· 分支 `main`

---

## 1. 一句话现状

**引擎线 L1 九项缺口：6 项完全闭合、1 项引擎侧闭合、2 项受阻（需外部条件）。**
⇒ ⇒ 「**上屏第一屏**」早已可开工（私有仓首屏在跑）；⇒ ⇒ 「**引擎可上线**」仍不可写进排期，
**卡点不是能力，是外部条件**（见 §5）。

⚠️ **口径范围**：本页所有引擎数字取自 **`ui-engine` 分支**（`crates-ui/` **10 crate**）。
主仓 `main` 分支里有一份 **7 crate 旧副本**（owner 已定：**冻结、不维护**）⇒ ⇒
**两者不是同一状态，不可互相覆盖**，各自成立只是分支不同。

---

## 2. 已交付（提交号可逐条对账）

远端分支：**`refs/heads/ui-engine`** · 最新：**`658cd192`**（本地与远端**完全同步**）

| 批次 | 内容 | 提交 |
|---|---|---|
| G9 | **公开「key → 身份 → 反解路径」**：`IdentitySegment` / `identity_segments_at` / `path_for_identity` / `bounding_box_for_identity` / `scroll_metrics_for_identity` | `3174666b` |
| E-192 | `asyncLine` 探针抖动治好（**根因是「用表象推状态」**）⇒ 连跑 **10 次 0 败** | `bed69495` |
| G8 | **真 panic 浏览器证据**（「一崩全白」→「一崩可见」）| `c10100b1` |
| G5 | 接线证据核清 + **1k 行 patch 基准**（全量 **3 ms** vs 窗口化 **0.02 ms**）| `b0349dc4` |
| G4 | **全链闭环**（真实 `input` ⇒ 信号 ⇒ 应用加工 ⇒ 重渲染 ⇒ DOM）| `27d9b5f7` |
| G1 | 收口为完成 + 登记 G9 | `0119a28c` |
| G7 | **CLS 浏览器证据**（真位移>0 / 亚像素=0 / 视口外=0）| `77395f32` |
| G7 | **资源契约搬到 `ui-gpu`**，`TextureUploader` **零适配器**落地 | `f4e36b6f` |
| 依赖 | 修掉 **5 个真 RUSTSEC advisory**（`quinn-proto` 0.11.16→0.11.19）| `95ce4b47` |
| CI | **「每次 push 一封红邮件」真根因**（`cargo-audit.yml` 重复 `if` 键 ⇒ GitHub 无法解析）| `813a1c3f` |
| 分支 | 远端分支改名 `ui-engine-p0-2026-10-24` ⇒ **`ui-engine`**（旧名冻结于 `b72ac26e`）| `d53f74c8` |
| 文档 | **L1 阶段性收尾报告**（交付 + 七条难点 + 四项受阻）| `970991b8` |
| 文档 | 收尾后**文档对齐**（readiness 升到 15 项）| `658cd192` |

### 2.1 私有仓（产品）侧已消费的能力

私有仓 `Apeireth/ui-desktop`（分支 `main`，最新 **`bab4af7`**）：

| 能力 | 状态 |
|---|---|
| **G1 布局测量真消费** | 新建工作后用三个 API 各承担一个判定，把被滚出可视区的列表滚回顶部（实测 `corrections = 1`、`scrollTopAfter = 0`）|
| **G9 身份寻址** | **已采用** ⇒ 删掉原先约 30 行「遍历已挂载树按 `data-*` 找节点」⇒ 改用 `bounding_box_for_identity` 等 |
| **引擎依赖** | 已升到 **`3174666b`**（`Cargo.toml` 6 处 rev + `Cargo.lock` 全部引擎包；旧 SHA 残留 **0**）|

---

## 3. 门禁（三条硬前置，缺一条就别跑）

```bash
# ① 环境（**RUSTC_WRAPPER= 必设**，否则 sccache 找不到 → 编译直接失败）
export RUSTC_WRAPPER=""
export CARGO_TARGET_DIR="<你的盘>:/cargo-target/ui"     # Windows 上用 D:\... 之类
export PATH="/home/bai/.local/node22/node_modules/node-linux-x64/bin:/root/.cargo/bin:$PATH"  # Linux 版；Windows 换成 rustup 的 bin

# ② 引擎门禁（crates-ui 是**独立 workspace**，不进根 workspace ⇒ 必须单独跑）
cd crates-ui
cargo test --workspace --locked --offline                       # 期望 1243 passed / 0 failed
cargo clippy --workspace --all-targets --locked --offline -- -D warnings        # 期望 0
cargo clippy --workspace --all-targets --locked --offline \
  --target wasm32-unknown-unknown -- -D warnings                                # 期望 0
cargo fmt --all -- --check                                                      # 期望 0 diff

# ③ 文档门禁（在**仓库根**，不是 crates-ui/）
cd ../.. && python3 scripts/docs_index.py --check && python3 scripts/docs_links.py --check
```

⚠️ **Windows 上必须额外装**：

```powershell
rustup toolchain install 1.97.1          # 根 rust-toolchain.toml 钉死这个版本
rustup target add wasm32-unknown-unknown  # 本地要手动加（CI 的 setup-rust-toolchain 会自动加）
```

---

## 4. 浏览器证据（`drive_l1.py`）在 Windows 上怎么跑

⚠️ **本节是 Windows 搬迁最容易踩的地方**，逐条照做。

### 4.1 浏览器路径：设环境变量，**不要改代码**

`crates-ui/apeireth-ui-demo/harness/tools/compare.py:63`：

```python
CHROME = os.environ.get("CHROME") or "/usr/bin/chromium"
```

⇒ ⇒ **已经支持 `CHROME` 环境变量** ⇒ ⇒ Windows 上：

```powershell
$env:CHROME = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
```

⭐ **为什么 Edge 而不是 Chrome**：Edge 与 **WebView2 同一渲染内核**
⇒ ⇒ 用 Edge 跑证据 ⇒ 结论**能覆盖**真实 Tauri 宿主（Windows 上 Tauri 用的就是 WebView2）。

⚠️ `drive_intro.py` 的 `find_chrome()` **只认** `google-chrome-stable` / `chromium` /
`chromium-browser` / `/usr/bin/chromium` ⇒ ⇒ **Windows 上要么设 `CHROME_PATH`**
（该脚本支持）**要么改那一行**（推荐前者）。

### 4.2 `--user-data-dir` 已经处理好了

`compare.py:225` 的注释记录了一个**真实踩坑**：

> Windows runner 上跑的是 Edge ⇒ **不给独立 profile ⇒ Edge headless 不监听调试端口**
> ⇒ ⇒ CDP 的 `Page.navigate` **超时**（run 41264856246 的真实失败）。

⇒ ⇒ **代码已带 `--user-data-dir`**（一次性临时目录，不碰用户默认 profile）⇒ ⇒ **不用改**。

### 4.3 两条 Linux 专属腿在 Windows 上**不可用**（如实登记）

| 腿 | 依赖 | Windows 处置 |
|---|---|---|
| `drive_l1.py` | **Chromium/Edge + CDP** | ✅ **可跑**（设 `CHROME`）|
| `drive_webkit.py` | `webkit2gtk-4.1` + `gtk3` + `libsoup-3.0` + `jscore-4.1` + `xvfb-run` + `pkg-config`（Linux **GTK** 后端）| ❌ **不可跑** ⇒ ⇒ **WebKitGTK 侧结论在 Windows 上无法复现**（本来就该由 WebView2 覆盖）|
| `drive_webkit_plain.py` | 同上 | ❌ 同上 |

⚠️ `drive_webkit.py` 有**两处 Linux 硬编码默认值**（都读环境变量，**有覆盖**）：

```python
TARGET_DIR = os.environ.get("APEIRETH_WEBKIT_TARGET", "/home/bai/Projects/webkit-probe-target")
CARGO_TARGET_DIR 默认 "/home/bai/Projects/apeireth-ui-target"
```

⇒ ⇒ Windows 上要么设这两个环境变量，**要么**把默认值改成相对路径（推荐后者，见 §7-①）。

---

## 5. 受阻项（**不是能力缺口，是外部条件**）

| 项 | 卡在哪 | 需要什么 |
|---|---|---|
| **G3 真实中文输入法** | 只有**合成事件**证据（`composing_flag: true`）| **真实中文输入法 + GUI 会话**（本机无）|
| **G2 ② 命中测试消费方** | 产品**没有任何**「按指针位置判断元素」的功能 | **真实产品需求**（弹层外点关闭 / 拖动落点）|
| **G10 虚拟列表生产消费** | 产品仅 **2 条**工作项 | **产品长列表**（工作 ≥ 数百）⇒ 基准已给：1k 行全量 patch **3 ms** |
| **桌宠形态** | 无 Live2D 资产；壳与形态未定 | **资产 + owner 定壳** |

---

## 6. 六条已知残余（**是口径，不是缺陷 —— 别「顺手修掉」**）

1. ⚠️ **`docs_truth.py --verify --check` 当前 EXIT 1**（`claims=29 fail=0 stale=13`）
   ⇒ ⇒ **不是引擎线引入**（`grep -c "crates-ui" docs/status/claims.yaml` = **0**；
   13 条 STALE 的 changed 文件全在 `crates/adapters|engine|capabilities`）⇒ ⇒ **不代 bump**
   ⇒ ⇒ 留待主 workspace 各自的线复核。
2. ⚠️ **G9 只覆盖「重排后仍能找到同一逻辑元素」**，
   **不含**「按 DOM 属性找」（`bounding_box_by_attr`）⇒ ⇒ 产品若按 `data-*` 定位，
   **仍需自己做一次性查找**换身份（但之后每帧可复用身份测量，**不必**再走树）。
3. ⚠️ **产品侧三个 `key` 是视图与宿主的隐式契约**（字符串写在两处）⇒ ⇒ 无编译期保证。
4. ⚠️ **CLS 的累加语义**只验了单元素 3 次观测 ⇒ 「一次会话的总 CLS」**未验**。
5. ⚠️ **真实 panic 在 wasm `panic=abort` 下会 trap 整个实例** ⇒ 触发**必须是最后一个 wasm 调用**
   （引擎仓 `web/l1/main.js` 已按此编排）。
6. ⚠️ **只在 Chromium + Linux 验过** ⇒ WebKitGTK / Windows WebView2 / macOS WKWebView **未跑**
   ⇒ ⇒ **搬 Windows 后第一件事就是把 §4.1 跑起来**，把口径从「Chromium only」升级。

---

## 7. 接手后**能直接做的事**（按优先级）

### ① 跨平台化那两个硬编码默认值 + 浏览器探测（**最该先做**）

- `drive_webkit.py` 两处默认路径 ⇒ 改成**相对仓库根**或读环境变量；
- `drive_intro.py` 的 `find_chrome()` ⇒ 加 `chrome.exe` / `msedge.exe` / `CHROME_PATH`；
- `compare.py` 已支持 `CHROME` ⇒ **不用改**。

### ② 重建推送通道

⚠️ **引擎线的 `push.sh` 不在 git 里**（在 `/home/bai/Projects/apeireth-ops/push.sh`，
走 **SSH-over-443 + SOCKS5** 通道）⇒ ⇒ **Windows 上没有** ⇒ ⇒ 二选一：
① 用普通 `git push`（需确认网络能直连 `github.com:22` 或 443）；
② 把该脚本**收进仓库**（但它含个人网络拓扑 ⇒ ⇒ **先与 owner 确认是否愿意入 git**）。

### ③ 把 §4 的浏览器证据在 Windows 上真跑一遍

⇒ ⇒ 那是「Windows WebView2 引擎跑通」这条**已有历史记录**的现代复现
⇒ ⇒ 跑完把 `ui-engine-l1-readiness.md` 的口径从「Chromium only」升级。

### ④ 产品侧采用引擎其余新能力

**G9 已采用**；**CLS 观测 / panic 可见兜底 / 指标门控** 尚未采用
⇒ ⇒ 逐项评估收益，避免「引擎有了、产品不用」再次发生（G7 的教训，见 §8）。

### ⑤ 私有仓的 r10 符合性还有两个待裁决问题

见私有仓 `docs/design-requests/2026-11-07-r10-conformance-inquiry.md`：
- **Q1**：`data-interface` 取哪档（取 `"simple"` 会**隐藏左栏**）；
- **Q2**：`.command-dock` 该不该是 `main` 的直接 grid 项（**合同与产品现状冲突**）。

---

## 8. 七条难点（**接手前最该知道的坑**）

| # | 难点 | 一句话 |
|---|---|---|
| 1 | **「能力在、只有测试在用」**是最易误报完成的形态 | G7 已修、**G2 ② 未修**，同型 ⇒ 判断完成**必须问「生产路径有没有人在用」** |
| 2 | **测量装置自己不稳**会被误判成引擎缺陷 | E-192：`asyncLine` 3 跑 1 败；修它**错了三次**，错因全是「用表象推状态」 |
| 3 | **没看真实渲染**会给出**错的结论** | 我曾判「产品 r10 看起来是对的」⇒ 看截图后当场推翻（只有 6.6% 规则命中）|
| 4 | **结构断言写得像实现，会掩护实现错** | 两次：断言「composer 在 work-canvas 内」「找 `.workspace-grid`」⇒ 都钉住了**错的形状** |
| 5 | **`cargo check` 会被 cfg 门骗** | `host.rs` 是 wasm-only ⇒ **native 绿、wasm 红** ⇒ 改 wasm-only 文件时 native 绿不算数 |
| 6 | **引擎依赖钉 SHA** ⇒ 新 API 必须**先升 SHA** 才能被产品用 | `cargo update` 只更新指定包 ⇒ 要分两步；且走 SSH 拉大仓**约 10 分钟** |
| 7 | **跨批次升 SHA 的风险面远大于单个 API** | `e19823af→3174666b` 跨 13 提交，含 G7 **破坏性搬家** ⇒ 每次升 SHA 都要做破坏性变更扫描 |

---

## 9. 五分钟自检清单（Windows 上先跑这个）

```powershell
# ① 引擎在不在、分支对不对
git clone --branch ui-engine https://github.com/Apeireth/Apeireth.git
cd Apeireth\crates-ui
# ② 工具链对不对
rustc --version          # 期望 1.97.1
rustup target list --installed | Select-String wasm32
# ③ 门禁绿不绿
$env:RUSTC_WRAPPER=""; $env:CARGO_TARGET_DIR="D:\cargo-target\ui"
cargo test --workspace --locked --offline        # 期望 1243 passed / 0 failed
cargo clippy --workspace --all-targets --locked --offline -- -D warnings   # 期望 0
cargo fmt --all -- --check                        # 期望 0 diff
# ④ 浏览器证据能不能跑
$env:CHROME="C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
cd evidence\p5-b; bash build.sh; python drive_l1.py     # 期望 EXIT 0
```

---

## 10. 诚实边界

- ✅ 本页引擎数字**均可复跑**（§3、§9）。
- ⚠️ **只在 Chromium + Linux 验过**；Windows / macOS **本批未跑**。
- ⚠️ **未做逐像素对照**（r10 原型 HTML 未搬入私有仓）⇒ 符合性是**结构 + 计算样式**级。
- ⚠️ 探针依赖**浏览器路径**；换机需设 `CHROME` / `CHROME_PATH`。
- ⚠️ 引擎**未跑**真实 GPU 硬件矩阵（只跑过软件后端与 Chromium WebGL/WebGPU 探针）。

## 2026-11-11 模态作用域能力（feat/engine-modal-scope-2026-11-11）

**做了什么**：引擎的模态作用域改按「子树身份」打开 —— 应用只声明「这棵子树是模态作用域」，引擎负责枚举该子树内的可聚焦项、独占 Tab 消费、把初始焦点落在指定元素、关闭时归还焦点。

**为什么在引擎层**：宿主侧原先自建了一套捕获阶段的 Tab 陷阱（等于第二套焦点系统，且让引擎的 Tab 通路完全失效）；引擎真缺口是「陷阱内只剩一个可聚焦项时 `FocusOrder::next` 返回 None ⇒ 浏览器原生 Tab 接手 ⇒ 焦点跑出作用域」。

**新增 API**（`crates-ui/apeireth-ui-dom/src/renderer.rs`）：`open_modal_scope` / `open_modal_scope_at_identity` / `focusable_paths_under` / `should_consume_tab`；`ModalLayer` 增 `scope_root`；`focus_next` 每次 Tab 重新枚举子树内可聚焦项且焦点在作用域外时拉回；`focus_at_path` 按作用域判定；`keyboard.rs` 的 `KeyNavigation::install` 改用 `should_consume_tab` ⇒ 模态激活期间独占 Tab。

**0 装口径**：IMPLEMENTED（引擎能力层）。`cargo test -p apeireth-ui-dom --locked --lib` ⇒ 241 passed / 0 failed（236 ⇒ 241，新增 5 条）；`cargo test -p apeireth-ui-dom -p apeireth-ui-kit -p apeireth-ui-app --locked -j 2` 全绿；clippy 三 crate 0 警告；`cargo fmt -p apeireth-ui-dom --check` 0 diff；`scripts/arch_guard.py --strict` VIOLATIONS 0。**未实测**：真实 WebView2 / WKWebView 宿主、真人输入法、macOS、非 Chromium 内核。

**反向验伪**：三处接线各一组红 → 绿（摘「每次 Tab 重新枚举」⇒ 4 passed/1 failed；摘「焦点在作用域外拉回」⇒ 4 passed/1 failed；摘「模态激活时即使没动也消费 Tab」⇒ 4 passed/1 failed ⇒ 装回各 5 passed/0 failed）。

**下一步**：本分支开 PR（base 挂 `ui-engine-p0-2026-10-24`）；合入后宿主侧把引擎 pin 抬到合并后的 SHA。

## 2026-11-11 作用域根按稳定身份跟随（feat/engine-modal-scope-2026-11-11 · c22622e22）

**做了什么**：模态作用域的根除记「子节点下标链」外，另记一份**稳定身份段**（有 key 的层记 key、没有的记下标）；每次用作用域根时**按当前树重新解析**身份 ⇒ ⇒ 元素在树里搬家后，作用域仍跟着同一个元素。

**为什么在引擎层**：`scope_root` 是**位置**（第几个子节点）⇒ ⇒ 树一变（前面插一个兄弟节点、抽屉遮罩撤掉、条件块切换）⇒ ⇒ 下标全变 ⇒ ⇒ 打开那一刻记下的路径会指向**另一个元素** ⇒ ⇒ 作用域判定跑偏 ⇒ ⇒ Tab 在错的子树里走 ⇒ ⇒⇒ 焦点卡住（产品侧的表现是「连续按 Tab 焦点不动，关闭重开才恢复」）。

**新增 API**（`crates-ui/apeireth-ui-dom/src/renderer.rs`）：`ModalLayer.scope_identity`（作用域根的稳定身份段）；私有方法 `Renderer::scope_root_path()`（先按身份在**当前树**重解析，解析失败再回退到 `scope_root`）；`scope_allows_path` / `sync_scope_trap` / `focus_next` 三处统一改走 `scope_root_path()` ⇒ ⇒ 判定口径只有一份。

**0 装口径**：IMPLEMENTED（引擎能力层）。`cargo test -p apeireth-ui-dom --locked --lib` ⇒ **244 passed / 0 failed**（236 ⇒ 241「子树身份打开」+5 ⇒ 243「初始焦点按子树内 key 定位」+2 ⇒ 244「作用域根按身份跟随」+1）；`cargo clippy -p apeireth-ui-dom --all-targets --locked -- -D warnings` ⇒ **0 警告**；`cargo fmt -p apeireth-ui-dom --check` ⇒ **0 diff**；`python scripts/arch_guard.py --strict` ⇒ **VIOLATIONS 0**（crates=31 edges=118 env=150）。**未实测**：真实 WebView2 / WKWebView 宿主、真人输入法、macOS、非 Chromium 内核（与上一节同一批未实测边界）。

**反向验伪**：摘掉 `scope_root_path` 里的身份重解析（退回「打开那一刻的冻结路径」）⇒ ⇒ 新增单测 `modal_scope_follows_the_root_after_the_tree_shifts` **红**（`assertion left == right failed: ⭐ 作用域根必须跟着稳定身份走到新下标 / left: Some([1]) / right: Some([2])`）⇒ ⇒ 装回 **绿**（`1 passed; 0 failed`）。

**下一步**：本分支随 PR #118 合入主线；合入后宿主侧把引擎 pin 抬到合并后的 SHA。

## 2026-11-11 · 吸收 S1 六轮「用引擎」的经验（手册 / 经验手册 / 缺陷台账三处同批更新）

【为什么有这一节】2026-11-10～11-11，私有仓新前端 **S1 工作台**（六轮验收、约 40 项取证断言）
**第一次把本引擎当作「真实产品要用的引擎」用满** ⇒ 暴露了三处引擎级缺口（已修，引擎 PR #118）
与四类**调用方式**的错误。引擎的缺陷修了还不够 —— **调用方下次还会犯同样的错**，
除非经验进到「调用方会读到的那几本」里。

【三处落点（本次同批更新，手册与引擎同批 = 引擎交付的一部分，不是文档债）】
1. **使用手册** `docs/04-internal/ui-engine-w0-onboarding.md`：§2.4 焦点与模态改成 **一等口径**
   （`open_modal_scope` / `open_modal_scope_at_identity` / `close_modal`）+ **调用方须知五条**；
   §4 五个坑 → **加坑 6（宿主自建焦点陷阱 ⇒ 引擎根节点收到 0 次 Tab）**、
   **坑 7（身份路径跳中间容器 ⇒ 模态作用域根本没建立）**；§5 自证清单加两条
   （引擎自己消费 Tab / 焦点请求读回 `activeElement` 对账）；§6「可以依赖」加「模态作用域」段。
2. **经验手册** `docs/04-internal/engineering-experience/02-failure-anatomy.md`：
   新增 **死法 8 · 引擎使用错法**（8a 宿主自建第二套焦点系统 / 8b 把「调用返回成功」当「焦点真的移动」/
   8c 身份路径按「跳中间容器」拼 / 8d 读一个文件就下结论 / 8e 验收探针恒真谓词假绿），
   每条按「症状 → 死因 → 药方 → 防复发断言 + ★ 出处」四段式；速查表补 5 行。
3. **缺陷台账** `docs/04-internal/ui-engine-defect-ledger.md`：新增 **§3.7**，登记
   **E-193**（模态作用域根只记路径、不跟身份 ⇒ 树位移后 Tab 卡死，已修 `c22622e22`）、
   **E-194**（模态激活期间 Tab 未被引擎独占 ⇒ 原生 Tab 逃逸，已修 `662912262`）、
   **E-195**（模态初始焦点不是一等参数 ⇒ 调用方自拼身份且拼错被静默忽略，已修 `662912262`）、
   **E-196**（使用侧反模式登记：四条真实事故 ⇒ 药方指向死法 8 与使用手册）。

【本轮的一条结构性结论（比单条缺陷更重要）】
**「引擎的一等能力」必须被调用方当成一等能力**：三处引擎级缺口（E-193/E-194/E-195）全部是
「本该由引擎承担的事被推给调用方」的产物（路径自己算、初始焦点自己算、Tab 归属自己管）；
四类使用侧反模式（E-196）全部是「调用方没读手册 / 手册落后于引擎」或「调用方自建了一套」的产物
⇒ ⇒ **手册与引擎同批更新，是引擎交付的一部分**；下次引擎交付要带「调用方须知」一起出。

【本节未验证/边界】本节是文档与经验登记，不含代码改动；三条引擎修法本身的红→绿输出见引擎 PR #118
的提交正文与 `docs/04-internal/ui-engine-defect-ledger.md` §3.7 的证据列。
