# 引擎线 L1 阶段性收尾报告（2026-11-07）

[Document-Meta]
Document: docs/04-internal/ui-engine-l1-stage-closeout-2026-11-07.md
Class: STATUS
Status: 🟢 阶段收口
Last-Modified: 2026-11-07
Author: 前端引擎线（AI 施工）

> **本报告回答三件事**：①**交付了什么**（带可复跑证据）；②**还差什么、为什么差**（难点）；
> ③**下一任从哪接手**。
>
> **口径纪律**：本报告所有数字均可用 §5 的命令复跑；**未实测的一律标「未实测」**，
> 不写「应该没问题」。四级口径定义见
> [system-capabilities.md](../01-architecture/system-capabilities.md)。

---

> ⚠️ **口径范围（先看这一条）**：本报告所有引擎数字都取自 **`ui-engine` 分支**
> （worktree `/home/bai/Projects/apeireth-ui-wt2`，分支 `ui-engine`，10 个 crate）。
> ⚠️ 主仓 `docs/03-reference/test-baseline.md` 里 `crates-ui/` 写的是 **432 passed**
> ⇒ ⇒ 那是 **`main` 分支的旧副本**（7 crate 时期）⇒ ⇒ **与本报告不是同一状态，不可互相覆盖**。
> ⇒ ⇒ 两份数字各自成立，只是**分支不同**。

## 0. 一句话结论

**L1 九项里 6 项完全闭合、1 项引擎侧闭合（产品消费转 G10）、2 项受阻（需外部条件）。**

⇒ ⇒ **「上屏第一屏」早就可开工**（产品首屏已在跑，且本阶段把它的 r10 符合性也拉回合同）；
⇒ ⇒ **「引擎可上线」仍不可写进排期**，但**卡点不是能力，而是三处外部条件**（见 §3）。

---

## 1. 交付清单（本阶段，引擎仓）

### 1.1 提交（`e19823af..3174666b`，**15 个**）

| 提交 | 内容 |
|---|---|
| `3174666b` | **G9 完成**：公开 `IdentitySegment` + `identity_segments_at` / `path_for_identity` / `bounding_box_for_identity` / `scroll_metrics_for_identity` |
| `bed69495` | **E-192 治好**：`asyncLine` 抖动（连跑 10 次 0 败）|
| `c10100b1` | **G8 完成**：真 panic 浏览器证据（「一崩全白」→「一崩可见」）|
| `b0349dc4` | **G5 收口**：接线证据核清 + 1k 行 patch 基准（**3 ms** vs 窗口化 **0.02 ms**）|
| `27d9b5f7` | **G4 完成**（全链闭环）+ 纠正 G2 过期「余下」+ 更正滞后测试数字 |
| `0119a28c` | **G1 收口为完成** + 登记真缺口 **G9** |
| `77395f32` | **G7 CLS 浏览器证据**：真位移>0 / 亚像素=0 / 视口外=0 |
| `f4e36b6f` | **G7 接线完成**：资源契约搬到 `ui-gpu`，`TextureUploader` **零适配器** |
| `95ce4b47` | 修掉 **5 个真 RUSTSEC advisory**（`quinn-proto` 0.11.16→0.11.19）|
| `058ff9d8` | 登记 E-191（advisory 被 CI 解析失败藏了 14 天）|
| `813a1c3f` | **CI「每次 push 一封红邮件」真根因**：`cargo-audit.yml` 重复 `if` 键 ⇒ GitHub 无法解析 |
| `8bbb7b63` | 记录 E-150 的「正解」其实不完整（负结果保留）|
| `d53f74c8` | 远端分支改名 `ui-engine-p0-2026-10-24` → `ui-engine` |
| `b72ac26e` | **撤回 E-184 的「Tauri 产品路线不成立」**（同一壳对照实测：引擎探针页 **5/5 跑通**）+ 登记 **E-188** |
| `ea26cd66` | **E-184 结论降级**（私有仓反例：产品页在 Tauri 2 壳里跑得好好的，帧循环 **91** 帧）|

### 1.2 L1 九项状态（**从交接文档逐行读，不凭记忆**）

| 项 | 状态 | 证据 |
|---|---|---|
| **G1 布局测量** | ✅ **完成** | 产品侧真消费（3 个 API 各承担一个判定）+ E-98 逐值一致 |
| **G2 事件面与载荷** | 🟡 **①完成 / ②未接** | ①浏览器载荷 `value=Some("a");pos=None;checked=Some(false)`；②`hit_test_device` **生产侧 0 消费** |
| **G3 焦点/IME/a11y** | 🟡 **合成事件已验** | `composing_flag: true`；**真实中文输入法硬件实测仍缺** |
| **G4 受控输入闭环** | ✅ **完成** | 全链往返「真实 `input` ⇒ 信号 ⇒ 应用加工 ⇒ 重渲染 ⇒ DOM」，实测 `uplink → uplink-applied` |
| **G5 虚拟列表** | ✅ **引擎侧闭合** | 接线有浏览器证据；1k 行基准 **3 ms**；生产消费转 **G10** |
| **G6 缩放/密度** | ✅ **完成** | E-98：`2.0 × 1.25 = 2.5` 换算正确 |
| **G7 资源/上传/CLS** | ✅ **完成** | `GpuTextureUploader` 零适配器 + CLS 三条对照（手算与实测**逐位一致**）|
| **G8 错误边界/panic** | ✅ **完成** | 真 panic：崩前干净 / 崩后可见 / `RuntimeError: unreachable` 真 trap |
| **G9 稳定寻址** | ✅ **完成** | 重排后按身份测到**同一逻辑元素**（且断言「与下标结果不同」防假绿）|
| **G10 虚拟列表生产消费** | ⏸ **有意不接** | 产品仅 2 条工作项；1k 行全量 patch 仅 3 ms ⇒ 门槛高 |

### 1.3 门禁（引擎仓，实测）

```
cargo test --workspace --locked --offline        ⇒ 1243 passed / 0 failed
cargo clippy --workspace --all-targets -D warnings          ⇒ 0
cargo clippy ... --target wasm32-unknown-unknown -D warnings ⇒ 0
cargo fmt --all -- --check                                  ⇒ 0 diff
python3 scripts/docs_index.py --check                       ⇒ 0
python3 scripts/docs_links.py --check                       ⇒ 0
drive_l1.py（真浏览器）                                      ⇒ EXIT 0
```

`drive_l1.py` 的 12 段**全部 `ok: true`**：
`dom` / `upload` / `asyncLine` / `app` / `perf` / `realLoad` / `w0Window` /
`binding` / `layoutShift` / `keyboardSemantics` / `focusModal` / `integration`。

---

## 2. 交付清单（产品仓，本阶段顺带，**6 个提交**）

| 提交 | 内容 |
|---|---|
| `8aa5012` | **采用引擎 G9 身份寻址**（删掉 30 行重复树遍历）+ **引擎依赖升级** `e19823af` → `3174666b` |
| `3521d4b` | **r10 符合性 P0**：首屏从「**93.4% 规则不生效**」拉回合同（`<body data-interface>` + 9 个自造类归位 + 输入区提为 grid 项）|
| `17d9166` | 清掉误提交的 `__pycache__` |
| `75228e6` | 设计线询问单**重要更正**（首屏并非「看起来对」）|
| `4b47006` | r10 合同符合性询问单（32 块孤儿规则 / 9 个自造类 / 2 个待裁决问题）|
| `ef83bd4` | G1 布局测量真消费（顺带修「三栏不是 grid 项 ⇒ 左栏永不滚」）|

产品仓门禁：`20 passed / 0 failed`；clippy native **0** / wasm32 **0**；fmt **0**；
`drive_first_screen.py` **EXIT 0**。

---

## 3. 难点（**这才是本报告的正文**）

### 难点 1 ⚠️⚠️⚠️ 「能力在、只有测试在用」是**最容易误报完成**的形态（已两次）

**同一形状的病出现过两次**：

| 次 | 症状 | 真相 |
|---|---|---|
| G7（已修） | `TextureUploader` 契约与实现**分居两个平级 crate** ⇒ 跨 crate 适配器**无处可放** | 唯一一份实现躺在 `evidence/p5-b/harness` ⇒ ⇒ **只有测试装置在用** |
| G2 ②（**未修**） | `hit_test_device` 逻辑完备、有 native 测试 | 产品仓**0 处**调用 ⇒ ⇒ 与 G7 修前**同型** |

**教训**：判断一项能力「完成」时，**不能只看「有没有代码 + 有没有测试」**
⇒ ⇒ 必须问「**生产路径上有没有人在用**」
⇒ ⇒ 否则会把「装置自娱自乐」写成「已完成」。

⇒ ⇒ **同型的还有 G10**（虚拟列表）：但**处置不同** —— G7/G2 是「产品**本该用**」，G10 是「产品**当前用不到**」
⇒ ⇒ ⇒ 所以 G10 登记「有意不接 + 判据」，**不是**缺口债。

### 难点 2 ⚠️⚠️⚠️ 「测量装置自己不稳」会被误判成引擎缺陷

**E-192 实例**：`asyncLine` 探针约 **3 跑 1 败**
⇒ ⇒ 而同一份报告里 `resolved=2 / accepted=1 / rejected=1` **全部正确**
⇒ ⇒ ⇒ 说明**仲裁是对的**，坏的是**采样时机**。

**我修它时错了三次，错因完全相同**：

| 尝试 | 判据 | 结果 |
|---|---|---|
| 固定等 900ms | **时间** | 仍 3 跑 1 败（慢请求卡边界）|
| 等「占位文本消失」 | **表象** | **0/10 全败**（第一条回来占位就消失）|
| 等「文本稳定 300ms」 | **表象** | 仍太早（慢请求没回来文本已稳定）|

⇒ ⇒ **正解**：轮询引擎的**真实状态** `resolved >= 2`（只读 `l1_async_line_peek`，不销毁宿主）
⇒ ⇒ ⇒ 连跑 **10 次 0 败**。

**教训**：**用表象推状态**是这类误判的总根源
⇒ ⇒ 「DOM 变了」≠「两条都回来了」；「文本稳定」≠「不会再变」。

### 难点 3 ⚠️⚠️ 「没看真实渲染」让我给出过**错的结论**（已自我更正）

我曾据「类名集合差 + CSS 级联推算」判断
「产品 r10 符合性**看起来是对的**，只是吃旧规则」。
⇒ ⇒ **看真实截图后当场推翻**：r10 的 **821 条选择器只命中 54 条（6.6%）**，
「发送」是 **886×42 的浏览器默认灰按钮**、输入框是**默认 textarea**、**没有顶栏**、窄屏左栏**与画布完全重叠**。

**教训**：**类名对 + 级联对 ≠ 界面对**
⇒ ⇒ 静态审计只能证明「规则算下来该生效」，**不能**证明「界面真的长成那样」
⇒ ⇒ ⇒ 涉及视觉/布局的结论**必须**有真实渲染证据（截图 / `getComputedStyle`）。

### 难点 4 ⚠️⚠️ **结构断言写得像实现，会掩护实现错**（两次）

| 实例 | 掩护了什么 |
|---|---|
| 产品探针断言「`.work-canvas` 里含 `.composer-dock`」 | 而 r10 要求输入区是 **grid 直接子节点** ⇒ ⇒ 那条断言**钉住的正是错的形状**；且它与 `.document-viewport{position:absolute}` 冲突 ⇒ 输入区被盖住、**「发送」点不动** |
| 产品探针断言「找不到 `.workspace-grid`」 | 而 r10 里**没有这个类** ⇒ ⇒ 三栏**不是 grid 项** ⇒ 高度链断开 ⇒ **左栏永不滚** ⇒ 工作多了够不到 |

⇒ ⇒ 两处都是**按实现写的断言**，于是「实现错了」时断言**照样绿**。

**教训**：结构断言应钉**合同**（外部要求），而不是钉**实现细节**
⇒ ⇒ 更好的做法：**读计算样式**验「落在第 2 列」，而不是验「在某个子节点里」。

### 难点 5 ⚠️ `cargo check` 会被 **cfg 门**骗（native 绿、wasm 红）

`host.rs` 是 `#[cfg(target_arch = "wasm32")]`
⇒ ⇒ **native `cargo check` 会整段跳过它** ⇒ 看着绿
⇒ ⇒ ⇒ 而 `shell/build.sh`（wasm32）才报 `unresolved import`。

**教训**：只要改的是 wasm-only 文件，**native 绿不算数** ⇒ ⇒ 必须跑 wasm32 门（本报告 §5 有两道）。

### 难点 6 ⚠️ 引擎依赖**钉 SHA** ⇒ 引擎新增 API 必须**先升 SHA** 才能被产品用

G9 采用时就撞上：私有仓钉 `e19823af`（无 G9 API）
⇒ ⇒ 必须先升到 `3174666b`（**6 处 `Cargo.toml` rev** + `Cargo.lock` 全部引擎包）
⇒ ⇒ 而 `cargo update` 只更新**指定包** ⇒ 分两步（先 `-p apeireth-ui-dom` 再补齐其余）
⇒ ⇒ 且 cargo 要走 **SSH 拉大仓**（实测 `cargo update` 耗时约 10 分钟）。

### 难点 7 ⚠️ 跨批次升 SHA 的风险面**远大于**单个 API

`e19823af → 3174666b` 跨了 13 个提交
⇒ ⇒ 含 **G7 的 `ui-dom::resource` → `ui-gpu::resource` 搬家**（破坏性）
⇒ ⇒ 本次靠「产品仓 `grep 'ui_dom::resource'` = 0 命中」确认不受影响
⇒ ⇒ ⇒ 但**每次**升 SHA 都该做这类**破坏性变更扫描**，否则是运气而非工程。

---

## 4. 受阻项（**我推进不了，需要外部条件**）

| 项 | 卡在哪 | 需要什么 | 我可以先做的准备 |
|---|---|---|---|
| **G3 真实中文输入法** | 只有**合成事件**证据；`compositionstart/end` 是探针造的 | **真实中文输入法 + GUI 会话**（本机无）| 探针骨架已在（`keyboardSemantics` 段），有环境即可跑 |
| **G2 ② 命中测试消费方** | 产品**没有任何**「按指针位置判断元素」的功能 | **真实产品需求**（如弹层外点关闭 / 拖动落点）| `hit_test_device` 已 IMPLEMENTED + native 测；需求到位即可接线 |
| **G10 虚拟列表生产消费** | 产品仅 **2 条**工作项 | **产品长列表**（工作 ≥ 数百 / 段落 ≥ 数百）| 基准已给：1k 行全量 patch **3 ms** ⇒ 到量再接 |
| **桌宠形态** | 无 Live2D 资产；壳与形态未定 | **资产 + owner 定壳** | 引擎侧最小多挂载能力**尚未开工**（owner 已批准做）|

---

## 5. 复跑命令（本报告所有数字的来源）

### 5.1 引擎仓（`/home/bai/Projects/apeireth-ui-wt2`）

```bash
export PATH=/home/bai/.local/node22/node_modules/node-linux-x64/bin:/root/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/home/bai/Projects/apeireth-ui-target RUSTC_WRAPPER=   # RUSTC_WRAPPER= 必须
cd crates-ui

cargo test --workspace --locked --offline                       # 期望 1243 passed / 0 failed
cargo clippy --workspace --all-targets --locked --offline -- -D warnings        # 期望 0
cargo clippy --workspace --all-targets --locked --offline \
  --target wasm32-unknown-unknown -- -D warnings                                # 期望 0
cargo fmt --all -- --check                                                      # 期望 0 diff

cd .. && python3 scripts/docs_index.py --check && python3 scripts/docs_links.py --check

cd crates-ui/evidence/p5-b && bash build.sh && python3 drive_l1.py              # 期望 EXIT 0
```

### 5.2 产品仓（`/home/bai/Projects/apeireth-ui-desktop`）

```bash
export CARGO_TARGET_DIR=/home/bai/Projects/ui-desktop-target RUSTC_WRAPPER=
cargo test --workspace --offline                                                # 期望 20 passed
cargo clippy --workspace --all-targets --offline -- -D warnings                 # 期望 0
cargo clippy --workspace --all-targets --offline \
  --target wasm32-unknown-unknown -- -D warnings                                # 期望 0（⚠️ 必跑）
bash shell/build.sh && python3 evidence/drive_first_screen.py                   # 期望 EXIT 0
```

---

## 6. 台账索引（本阶段新增/更新的条目）

| 条目 | 内容 |
|---|---|
| **E-190** | `cargo-audit.yml` 长期无法被 GitHub 解析（重复 `if` 键）⇒ 14 天审计盲区 |
| **E-191** | CI 修好后立刻暴露 **5 个真 RUSTSEC advisory**（`quinn-proto`）⇒ 已修 |
| **E-192** | `asyncLine` 探针颤动 ⇒ **根因是「用表象推状态」** ⇒ 已修（连跑 10 次 0 败）|

---

## 7. 下一任从哪接手（按优先级）

1. **产品侧采用引擎新能力**（G9 已采用；**CLS / panic 兜底 / 指标门控尚未采用**）
   ⇒ ⇒ 逐项评估收益，避免「引擎有了、产品不用」再次发生。
2. **`body[data-interface]` 相关设计线问题**（询问单已出，见产品仓
   `docs/design-requests/2026-11-07-r10-conformance-inquiry.md`）
   ⇒ ⇒ 待设计线答复 Q1/Q2（`data-interface` 取值档位、`.command-dock` 层级）。
3. **G3 / G2② / G10**：等 §4 的外部条件。
4. **E-192 的同类扫描**：其它探针是否也存在**单次采样竞态** ⇒ ⇒ 建议统一改成「轮询真实状态 + 等两帧」。

---

## 7.5 已知的**既有**红门禁（不是本阶段引入，如实登记）

### `scripts/docs_truth.py --verify --check` 当前 **EXIT 1**（`claims=29 fail=0 stale=13`）

**不是本阶段引入**，证据：

```
$ grep -c "crates-ui" docs/status/claims.yaml     ⇒ 0          # claims 不覆盖 crates-ui
$ docs_truth.py --verify | grep STALE -A2 | grep 'changed:'
⇒ crates/adapters/cli/*、crates/adapters/gateway/*、
  crates/engine/runtime-assembly/*、crates/capabilities/tools/*
⇒ 无一处 crates-ui/                                           # 13 条全在主 workspace
```

⇒ ⇒ 那 13 条 STALE 是**主 workspace 其它线**改过源码而 `verified_at` 未 bump
⇒ ⇒ ⇒ **本阶段一行都没碰它们** ⇒ ⇒ **不代 bump**（那是伪造核验）。

**留待主 workspace 各自的线复核**；本报告的引擎线不受影响
（`fail=0`，且 `docs_index` / `docs_links` 两门都是 **0**）。

## 8. 诚实边界（不夸大）

- ✅ 本报告的**引擎数字**均可复跑（§5.1）。
- ⚠️ **只在 Chromium + Linux 验过**；WebKitGTK / Windows WebView2 / macOS WKWebView **本阶段未跑**。
- ⚠️ **未做逐像素对照**（r10 原型 HTML 未搬入私有仓）⇒ 「符合合同」是**结构 + 计算样式**级，不是像素级。
- ⚠️ 便携性：`drive_l1.py` / `drive_first_screen.py` 依赖**本机 Chromium 路径**，换机需改 `CHROME` 探测。
- ⚠️ 引擎**未跑**真实 GPU 硬件矩阵（只跑过软件后端与 Chromium WebGL/WebGPU 探针）。
