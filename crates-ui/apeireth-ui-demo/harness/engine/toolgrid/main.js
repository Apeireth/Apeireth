// 引擎侧（工具卡面批二：工具栏 + 工具网格）的 wasm 装载 + 控制 API。
//
// ## 装配形态
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
//
// ## 应用形态
//
// 同一个 wasm 产物挂多种应用 ⇒ 由 `window.__APP_MODE__` 决定（**必须在 wasm
// 初始化之前**设定 ⇒ 初始化即挂载）。
const mode = 'toolgrid';
if (typeof window !== 'undefined') {
  window.__APP_MODE__ = mode;
}

const module = await import(/* @vite-ignore */ '/engine/pkg/apeireth-ui-demo.js');
const { createToolGridApi } = await import('../../shared/toolgrid-api.js');

// 供对照 harness 调用（与参考侧的 `window.__TOOLGRID__` 同名同义）。
window.__TOOLGRID__ = createToolGridApi({
  caseCount: () => module.toolgrid_case_count(),
  // 引擎侧：搜索词写进状态（信号）⇒ 网格重算。
  applyQuery: (query) => module.set_search(query),
});

await module.default();
