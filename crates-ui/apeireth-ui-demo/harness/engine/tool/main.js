// 引擎侧（工具卡面）的 wasm 装载 + 控制 API。
//
// ## 装配形态
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
//
// ## 两种应用形态
//
// 同一个 wasm 产物挂两种应用（试点审批卡 / 工具卡面）⇒ 由 `window.__APP_MODE__`
// 决定（**必须在 wasm 初始化之前**设定 ⇒ 初始化即挂载）。
const mode = 'toolcard';
if (typeof window !== 'undefined') {
  window.__APP_MODE__ = mode;
}

const module = await import(/* @vite-ignore */ '/engine/pkg/apeireth-ui-demo.js');
const { createToolCardApi } = await import('../../shared/toolcard-api.js');

// 供对照 harness 调用（与参考侧的 `window.__TOOLCARD__` 同名同义）。
window.__TOOLCARD__ = createToolCardApi({
  caseCount: () => module.case_count(),
  selectCase: (index) => module.select_case(index),
});

await module.default();

// 固定数据里全部 JSON 值的序列化结果（**浏览器差分测试**用）：
// 浏览器侧用同一批值跑脚本侧的序列化 ⇒ 逐字节比对。
window.__TOOLCARD_JSON_PROBE__ = module.json_probe();
