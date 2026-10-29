// 引擎侧的 wasm 装载（wasm-bindgen 生成的胶水 + 启动）。
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
const module = await import(/* @vite-ignore */ '/engine/pkg/apeireth-ui-demo.js');

// 供对照 harness 调用（与参考侧的 `window.__PILOT__` 同名同义）。
window.__PILOT__ = {
  ready: () => module.is_ready(),
  setBusy: (value) => module.set_busy(value),
  setCopied: (value) => module.set_copied(value),
  allow: () => module.allow(),
  reject: () => module.reject(),
  dismiss: () => module.dismiss(),
  decision: () => module.decision(),
  cardPresent: () => module.card_present(),
};

await module.default();
