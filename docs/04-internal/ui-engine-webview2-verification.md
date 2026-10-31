# WebView2 验证方案（放行条件 C2）

> **状态**：方案与 CI 配置**已就绪**；**尚未在真实 WebView2 上跑过**（需一次真实 GitHub 手动触发）。
> **owner 决定**：2026-10-31「先泡一泡」⇒ ⇒ 默认**不自动跑**（见下「为什么不自动」）。
> 谁该看：决定是否触发的人 + 将来跑它的人。

---

## §1 这件事关什么

引擎的**切换前提**里，只剩这一条没兑现：

| 条件 | 内容 | 状态 |
|---|---|---|
| C1 | CI 全绿 | ✅ 已闭 |
| C3 | E-54 判据（criterion A） | ✅ 已闭 |
| **C2** | **真实 Windows WebView2 上跑通** | ⬜ **未做**（本文件） |

⇒ ⇒ 现在的浏览器证据**只覆盖 Linux 桌面 Chromium**（headless）。
⇒ ⇒ 「引擎可用」这个结论**还没有**覆盖 Windows 宿主。

---

## §2 怎么验（零 node 依赖的路子）

关键约束：CI 里 harness 的 `node_modules` 是**符号链接**到生产前端
（`frontend/companion-desktop/node_modules`）⇒ ⇒ Windows runner 上**不可靠**
⇒ ⇒ **不跑 vite**。

于是走这条路：

```
cargo build --release --target wasm32-unknown-unknown -p p5b-harness   # crates-ui/target/
wasm-bindgen --target web --out-dir web/engine/pkg --out-name p5b_harness <同上>.wasm
# ⇒ 直接伺服 crates-ui/evidence/p5-b/web （含 l1/ 与 engine/pkg/）
# ⇒ Edge（= WebView2 同一渲染内核）headless + CDP ⇒ 跑同一套 drive_l1.py
```

为什么能绕开 vite：`web/l1/main.js` 用的是**绝对路径** import
`/engine/pkg/p5b_harness.js` ⇒ ⇒ 只要伺服 `web/` 根，它就能加载 wasm ✓。

**判定**：五段全过（`dom` / `upload` / `app` / `binding` / `integration`）
⇒ ⇒ 与 Linux 侧**同一套**探针、**同一套**判定口径 ⇒ ⇒ 结论可比。

---

## §3 为什么不自动跑

1. owner 2026-10-31 决定 C2「先泡一泡」⇒ ⇒ 默认**不自动跑**；
2. Windows runner 比 Linux 慢 ⇒ ⇒ 不进常规路径；
3. 常规 8 个 job 的**结论**不依赖它 ⇒ ⇒ 它是**补充**而非前置；
4. ⭐ 每次自动跑都会给 owner 发一封邮件 ⇒ ⇒ owner 之前明确抱怨过邮件洪水
   ⇒ ⇒ **手动触发**（`workflow_dispatch`）是刻意的设计，不是遗漏。

**怎么触发**：Actions 页 → `ui-engine CI` → **Run workflow**（选分支）⇒ ⇒ 只跑 `webview2` 这一个 job
（其余 job 也会跑，因为 dispatch 触发整个 workflow ⇒ ⇒ **注意**：dispatch 会跑**全部** job，
其中 Linux 的会正常跑；这是 workflow 级触发的固有行为）。

---

## §4 口径（0 装诚实真账）

| 项 | 说明 |
|---|---|
| **覆盖什么** | **引擎兼容性**：同一套探针在 **WebView2 引擎**（Edge 内核）上成立 |
| **不覆盖什么** | ①**不是**「Tauri 应用内嵌 WebView2」的端到端运行 ⇒ ⇒ **宿主集成未覆盖**；②Windows runner **无 GPU** ⇒ ⇒ 走**软件渲染** ⇒ ⇒ **GPU 路径未覆盖**（GPU 路径由 Linux 真实 GPU 的对比证据覆盖） |
| **失败口径** | 任一段红 ⇒ ⇒ job 红 ⇒ ⇒ **C2 仍未闭** ⇒ ⇒ 不许写「已验证」 |
| **证据留存** | 探针报告 `crates-ui/evidence/p5-b/evidence/l1/report.json` ⇒ CI 上传为 artifact `webview2-l1-report` |

---

## §5 真跑通之后要做什么

1. 在 [L1 完成度体检表](ui-engine-l1-readiness.md) 的 C2 行标 ✅ 并附**运行号 + 日期**；
2. 在台账写一条（C2 闭合的证据行，含「哪台 runner / 哪个 WebView2 版本 / 五段结果」）；
3. **不要**把「WebView2 引擎通过」写成「宿主集成通过」⇒ ⇒ 宿主集成是**另一条**验证
   （要真跑 Tauri 应用 ⇒ ⇒ 需要 Windows 装置或 Windows CI 上装 Tauri 工具链）。

---

## §6 相关

- [L1 完成度体检表](ui-engine-l1-readiness.md)（放行条件在哪）
- [引擎产品工程：路径与基线](ui-engine-product-baseline.md)（垂直切片现状）
- 缺陷台账：`docs/04-internal/ui-engine-defect-ledger.md`
