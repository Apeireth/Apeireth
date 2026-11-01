// 引擎侧（治理卷宗面 · 授权管理）的 wasm 装载 + 控制 API。
//
// ## 装配形态
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
//
// ## 应用形态
//
// 同一个 wasm 产物挂四个卷宗面 ⇒ 由 `window.__GOV_TAB__` 决定（**必须在 wasm
// 初始化之前**设定 ⇒ 初始化即挂载）。
const tab = 'grants';
if (typeof window !== 'undefined') {
  window.__GOV_TAB__ = tab;
}

const module = await import(/* @vite-ignore */ '/engine/pkg/governance-harness.js');
const { createGovernanceApi } = await import('../../shared/governance-api.js');
const fixture = await import('../../fixtures/governance-cases.json');

// 供对照 harness 调用（与参考侧的 `window.__GOVERNANCE__` 同名同义）。
window.__GOVERNANCE__ = createGovernanceApi({ cases: fixture.default.cases[tab] });

// wasm 装载（`default` 是 wasm-bindgen 的 init）⇒ 之后显式启动应用。
await module.default();
module.start();

// 数据面已就位（固定 fixture 已在 wasm 侧装配完毕）⇒ 置位供对照 harness 判定。
window.__GOV_DATA_READY__ = true;
