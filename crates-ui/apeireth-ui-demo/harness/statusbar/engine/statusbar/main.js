// 引擎侧（状态条面）的 wasm 装载 + 控制 API。
//
// ## 装配形态
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
//
// ⚠️ 本面**自带 wasm 宿主 crate**（`engine-host/`）⇒ 不碰共享的
// `apeireth-ui-demo`（那是**共享点**：别的面正在施工）。
const module = await import(/* @vite-ignore */ '/engine/pkg/statusbar_harness.js');
const { createStatusBarApi } = await import('../../shared/statusbar-api.js');

// 供对照 harness 调用（与参考侧的 `window.__STATUSBAR__` 同名同义）。
window.__STATUSBAR__ = createStatusBarApi({
  caseCount: () => module.case_count(),
  selectCase: (index) => module.select_case(index),
  clickCount: () => module.click_count(),
});

// ⚠️ wasm-bindgen 的 `--target web` 产物**必须先 await 初始化 promise**
// （默认导出）⇒ 否则调用任何导出函数都会炸（wasm 实例还没建）。
await module.default();

// 挂载（真实 DOM）。
module.start();
